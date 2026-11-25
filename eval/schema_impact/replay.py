"""First-parent schema impact replay."""

from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
import random
import re

from . import config as client_config
from . import consumer_ts, producer_go, producer_route_table, schema
from ._replay_client import ClientSources
from ._replay_git import (
    Commit, History, LineFollower, SourceLine, blob, changed_files, first_parents as first_parents,
    materialize, parent, regular_files, revision, touch_hunk,
)
from .config import Config
from .consumer_ts import ClientCall
from .ids import op_id
from .match import bind
from .schema import Change, SchemaIndex


EARLY_ERA_START = "8e6c49a5"
CURRENT_ERA_START = "2b1ad947"
SEED = 20261120
REVERT_WINDOW = 10
DEFAULT_SCHEMA = "docs/swagger.json"
USABLE_KINDS = frozenset({
    "op_removed", "field_removed", "field_type_changed", "field_required_added", "enum_values_changed",
})
_EMPTY_SCHEMA = b'{"swagger":"2.0","paths":{}}'
_DAYS = 86400

type ElementState = str | bool | None
type Transitions = dict[str, tuple[ElementState, ElementState]]


@dataclass(frozen=True)
class Outcome:
    kind: str
    touched_in: str | None = None
    lag_commits: int | None = None
    lag_days: float | None = None


@dataclass
class Location:
    location_id: str
    element: str
    call: ClientCall
    outcome: Outcome
    field_spelled: bool
    predicted_line: SourceLine
    followed_lines: list[SourceLine]
    current_line: SourceLine | None = None
    current_property_lines: list[SourceLine] = field(default_factory=list)
    touching_commit: Commit | None = None
    hunks: list[str] = field(default_factory=list)
    callers: list[SourceLine] | None = None
    element_served_at_pin: bool | None = None


@dataclass
class ReplayResult:
    commit: str
    parent: str
    changes: list[Change]
    predicted: list[Location] = field(default_factory=list)
    control: list[Location] = field(default_factory=list)
    skipped_reason: str | None = None
    producer_locations: dict[str, list[SourceLine]] = field(default_factory=dict)
    change_commit: Commit | None = None
    consumer_revision: str | None = None
    config: str | None = None


class _ElementChanges(list[Change]):
    """Reportable rows plus state transitions such as a field reappearing."""

    def __init__(self, rows: list[Change], transitions: Transitions):
        super().__init__(rows)
        self.transitions = transitions


def _go_files(repo: Path, commit: str, paths: list[str] | None = None) -> list[str]:
    if paths == []:
        return []
    selectors = [f":(literal){file}" for file in paths] if paths else []
    return [file for file in regular_files(repo, commit, *selectors)
            if file.endswith(".go") and not file.endswith("_test.go")]


def operation_sets(repo: Path, commits: list[str]) -> dict[str, set[str]]:
    """Update annotations over contiguous, oldest-first first-parent commits."""
    by_file: dict[str, list[str]] = defaultdict(list)
    count: Counter[str] = Counter()
    result: dict[str, set[str]] = {}
    previous = None
    for commit in commits:
        touched = None
        if previous is not None:
            touched = changed_files(repo, previous, commit, "*.go")
            for file in touched:
                count.subtract(by_file.pop(file, ()))
        sources = {file: blob(repo, commit, file).decode() for file in _go_files(repo, commit, touched)}
        for row in producer_go.annotations_from_sources(sources) if sources else ():
            by_file[row.file].append(row.element)
            count[row.element] += 1
        count = +count
        result[commit] = set(count)
        previous = commit
    return result


def _schema_at(repo: Path, commit: str, file: str) -> SchemaIndex:
    if regular_files(repo, commit, f":(literal){file}"):
        return schema.load(blob(repo, commit, file))
    return schema.load(_EMPTY_SCHEMA)


def _owner(element: str) -> str:
    if element.startswith("op:"):
        return element
    root = element.split("#/", 1)[0].removeprefix("field:")
    if root.startswith("resp:"):
        return "op:" + root.removeprefix("resp:").rsplit(":", 1)[0]
    return "op:" + root.removeprefix("req:")


def _op_changes(old: set[str], new: set[str]) -> tuple[list[Change], Transitions]:
    return (
        [Change("op_removed", element) for element in sorted(old - new)],
        {element: (element in old, element in new) for element in old ^ new},
    )


def _changes(repo: Path, commit: str, p0: str | None, op_sets: dict[str, set[str]],
             schema_path: str) -> list[Change]:
    if p0 is None:
        return []
    rows, states = _op_changes(op_sets[p0], op_sets[commit])
    if not changed_files(repo, p0, commit, f":(literal){schema_path}"):
        return _ElementChanges(rows, states)
    old = _schema_at(repo, p0, schema_path)
    new = _schema_at(repo, commit, schema_path)
    removed_ops = op_sets[p0] - op_sets[commit]
    states.update({
        element: (old.fields[element].shape if element in old.fields else None,
                  new.fields[element].shape if element in new.fields else None)
        for element in old.fields.keys() | new.fields.keys()
        if element not in old.fields or element not in new.fields
        or old.fields[element].shape != new.fields[element].shape
    })
    rows.extend(row for row in schema.diff(old, new)
                if row.kind in USABLE_KINDS and row.kind != "op_removed"
                and _owner(row.element) not in removed_ops)
    # The structural differ reports schema operation removals as a single row.
    # Replay uses annotations for those rows, so keep their orphaned field changes.
    schema_only_removals = old.operations.keys() - new.operations.keys() - removed_ops
    rows.extend(
        Change("field_removed", element, old_shape=old.fields[element].shape)
        for element in sorted(old.fields.keys() - new.fields.keys())
        if _owner(element) in schema_only_removals
    )
    return _ElementChanges(rows, states)


def changes(repo: Path, commit: str, op_sets: dict[str, set[str]], schema_path: str | Path) -> list[Change]:
    commit = revision(repo, commit) if commit not in op_sets else commit
    return _changes(repo, commit, parent(repo, commit), op_sets, str(schema_path))


def usable(changes_by_commit: dict[str, list[Change]]) -> dict[str, list[Change]]:
    """Filter reportable changes in oldest-first order using the next ten revisions."""
    commits = list(changes_by_commit)
    transitions: list[Transitions] = []
    for rows in changes_by_commit.values():
        if isinstance(rows, _ElementChanges):
            transitions.append(rows.transitions)
        else:
            transitions.append({
                row.element: ((row.kind == "op_removed", row.kind == "op_added")
                              if row.element.startswith("op:") else (row.old_shape, row.new_shape))
                for row in rows
            })
    excluded = set()
    for index, rows in enumerate(changes_by_commit.values()):
        reportable = {row.element for row in rows if row.kind in USABLE_KINDS}
        # A schema regression can temporarily introduce fields as well as drop
        # them. Removing those additions restores the preceding absence.
        introduced = {element for element, (before, _) in transitions[index].items()
                      if element.startswith("field:") and before is None}
        for element in reportable | introduced:
            before, _ = transitions[index][element]
            for following in range(index + 1, min(len(commits), index + REVERT_WINDOW + 1)):
                transition = transitions[following].get(element)
                if transition is not None and transition[1] == before:
                    excluded.update(((index, element), (following, element)))
                    break
    result = {}
    for index, (commit, rows) in enumerate(changes_by_commit.items()):
        eligible = [row for row in rows if row.kind in USABLE_KINDS and (index, row.element) not in excluded]
        if eligible:
            result[commit] = eligible
    return result


def select_config(repo: Path, p0: str, configs_dir: Path) -> Path:
    # Resolve the era by first-parent ancestry, independent of clock skew.
    for commit in reversed(first_parents(repo, p0)):
        if commit.startswith(CURRENT_ERA_START):
            break
        if commit.startswith(EARLY_ERA_START):
            return Path(configs_dir) / "apache-answer-2022.toml"
    return Path(configs_dir) / "apache-answer.toml"


def _outcome(history: History, change: str, touched: str | None) -> Outcome:
    if touched is None:
        return Outcome("present_at_pin")
    lag = history.position[touched] - history.position[change]
    days = (history.seconds[touched] - history.seconds[change]) / _DAYS
    return Outcome("same_commit" if lag == 0 else "later", touched, lag, days)


def follow(repo: Path, p0: str, lines: list[tuple[str, int]], pin: str) -> Outcome:
    history = History(repo, pin)
    p0 = revision(repo, p0)
    follower = LineFollower(history, p0)
    touched = follower.first_touch([follower.trace(file, line) for file, line in lines])
    change = history.successor(p0)
    return Outcome("present_at_pin") if change is None else _outcome(history, change, touched)


def _match_index(index: SchemaIndex, operations: set[str]) -> SchemaIndex:
    for element in sorted(operations - index.operations.keys()):
        index.operations[element] = schema.Operation(element.split(" ", 1)[1], None, "")
    return index


def _locations(result: ReplayResult, bound: list[tuple[ClientCall, str]], sources: ClientSources,
               follower: LineFollower, outcome, *, controls: bool) -> None:
    changes_by_op: dict[str, list[str]] = defaultdict(list)
    for element in sorted({row.element for row in result.changes}):
        changes_by_op[_owner(element)].append(element)
    available = []
    for call, operation in bound:
        if operation not in changes_by_op:
            available.append((call, operation))
            continue
        for element in changes_by_op[operation]:
            result.predicted.append(_location(
                f"{result.commit}:pred:{len(result.predicted) + 1}", element, call, sources,
                follower, outcome, include_evidence=True,
            ))
    if controls:
        count = min(len(available), max(5, 3 * len(result.predicted)))
        sample = random.Random(SEED).sample(available, count) if count < len(available) else available
        for number, (call, element) in enumerate(sample, 1):
            result.control.append(_location(
                f"{result.commit}:control:{number}", element, call, sources, follower, outcome,
                include_evidence=False,
            ))


def _location(location_id: str, element: str, call: ClientCall, sources: ClientSources,
              follower: LineFollower, outcome, *, include_evidence: bool) -> Location:
    properties = (sources.property_lines(call, element.rsplit("/", 1)[1])
                  if element.startswith("field:") else [])
    call_trace = follower.trace(call.file, call.line)
    property_traces = [follower.trace(site.file, site.line) for site in properties]
    traces = list(dict.fromkeys([call_trace, *property_traces]))
    touched = follower.first_touch(traces)
    row = Location(location_id, element, call, outcome(touched), bool(properties),
                   call_trace.initial, [trace.initial for trace in traces])
    history = follower.history
    if touched is None:
        row.current_line = call_trace.last_line
        row.current_property_lines = [trace.last_line for trace in property_traces]
    elif include_evidence:
        row.touching_commit = history.detail(touched)
        row.hunks = list(dict.fromkeys(
            patch for trace in traces
            if history.successor(trace.last_commit) == touched
            if (patch := touch_hunk(history.repo, trace, touched))
        ))
    return row


def _bound(snapshot: Path, config: Config, root: str, index: SchemaIndex) -> list[tuple[ClientCall, str]]:
    found = []
    for call in consumer_ts.calls(snapshot, config, root):
        grade, elements, _ = bind(call, index)
        if grade == "bound":
            found.append((call, elements[0]))
    return sorted(found, key=lambda entry: (entry[0].file, entry[0].line))


def _producer_locations(snapshot: Path, rows: list[Change], index: SchemaIndex,
                        annotations: list[producer_go.Binding], schema_path: str) -> dict[str, list[SourceLine]]:
    result: dict[str, list[SourceLine]] = {row.element: [] for row in rows}
    source = {}
    for binding in annotations:
        if binding.element in result:
            if binding.file not in source:
                source[binding.file] = (snapshot / binding.file).read_text().split("\n")
            result[binding.element].append(SourceLine(
                binding.file, binding.line, source[binding.file][binding.line - 1],
            ))
    path = snapshot / schema_path
    if not path.is_file() or not any(element.startswith("field:") for element in result):
        return result
    schema_sites = producer_go._schema_sites(index, path.read_bytes())
    for element, locations in result.items():
        if element.startswith("field:") and (site := schema_sites.get(_owner(element))):
            locations.append(SourceLine(schema_path, *site))
    return result


def replay_one(repo: Path, commit: str, usable_changes: list[Change], config: Path | Config,
               root: str, pin: str, *, schema_path: str = DEFAULT_SCHEMA,
               history: History | None = None) -> ReplayResult:
    repo = Path(repo)
    history = history or History(repo, pin)
    commit = commit if commit in history.position else revision(repo, commit)
    position = history.position[commit]
    if position == 0:
        raise ValueError("A replayed change must have a first parent")
    p0 = history.commits[position - 1]
    result = ReplayResult(commit, p0, list(usable_changes), change_commit=history.detail(commit),
                          consumer_revision=p0, config=None if isinstance(config, Config) else Path(config).name)
    cfg = config if isinstance(config, Config) else client_config.load(config)
    with materialize(repo, p0) as snapshot:
        index = _schema_at(repo, p0, schema_path)
        annotations = producer_go.annotations(snapshot)
        result.producer_locations = _producer_locations(snapshot, usable_changes, index, annotations, schema_path)
        sources = ClientSources(snapshot, cfg, root)
        client_paths = {path for path, _ in sources.resolver.clients}
        # P's primary client is identical in both eras. A supplemental client
        # cannot turn a missing primary client into an observed zero.
        primary = [path for path in client_paths if path.name == "request.ts"]
        required_clients = set(primary) or client_paths
        present = {path for path in required_clients if path.is_file()}
        if required_clients and not present or not (snapshot / root).is_dir():
            result.skipped_reason = "config_missing"
            return result
        if present and not sources.has_client_import(present):
            result.skipped_reason = "no_client_import"
            return result
        _match_index(index, {row.element for row in annotations})
        follower = LineFollower(history, p0, snapshot)
        _locations(result, _bound(snapshot, cfg, root, index), sources, follower,
                   lambda touch: _outcome(history, commit, touch), controls=True)
    return result


def replay_cross(producer: Path, consumer: Path, route_spec: producer_route_table.RouteSpec,
                 config: Path | Config, root: str) -> list[ReplayResult]:
    """Replay route removals with producer and consumer aligned by committer dates."""
    producer, consumer = Path(producer), Path(consumer)
    prod, client = History(producer, "HEAD"), History(consumer, "HEAD")
    cfg = config if isinstance(config, Config) else client_config.load(config)
    all_changes = {}
    producer_sites = {}
    old_ops: set[str] = set()
    previous_sites = {}
    for commit in prod.commits:
        with materialize(producer, commit) as snapshot:
            routes = producer_route_table.routes(snapshot, route_spec)
            sites: dict[str, list[SourceLine]] = defaultdict(list)
            for route in routes:
                source = (snapshot / route.file).read_text().split("\n")[route.line - 1]
                sites[op_id(route.method, route.path)].append(SourceLine(route.file, route.line, source))
        rows, states = _op_changes(old_ops, set(sites))
        all_changes[commit] = _ElementChanges(rows, states)
        producer_sites[commit] = previous_sites
        old_ops, previous_sites = set(sites), sites

    results = []
    for commit, rows in usable(all_changes).items():
        p0 = prod.commits[prod.position[commit] - 1]
        current = next((candidate for candidate in reversed(client.commits)
                        if client.seconds[candidate] <= prod.seconds[commit]), None)
        result = ReplayResult(commit, p0, rows, change_commit=prod.detail(commit), consumer_revision=current)
        sites = producer_sites[commit]
        result.producer_locations = {row.element: sites[row.element] for row in rows}
        results.append(result)
        if current is None:
            result.skipped_reason = "no_consumer_revision"
            continue

        def outcome(touched: str | None) -> Outcome:
            if touched is None:
                return Outcome("present_at_head")
            days = (client.seconds[touched] - prod.seconds[commit]) / _DAYS
            return Outcome("within_14_days" if days <= 14 else "later", touched, lag_days=days)

        with materialize(consumer, current) as snapshot:
            if not (snapshot / root).is_dir():
                result.skipped_reason = "config_missing"
                continue
            sources = ClientSources(snapshot, cfg, root)
            index = _match_index(schema.load(_EMPTY_SCHEMA), set(sites))
            follower = LineFollower(client, current, snapshot)
            _locations(result, _bound(snapshot, cfg, root, index), sources, follower, outcome, controls=False)
    return results
