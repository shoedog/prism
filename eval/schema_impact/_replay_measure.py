"""Run public replay serially and write the F4 ledger and reader packet."""

from collections import Counter
from dataclasses import asdict
from datetime import datetime, timezone
import json
from pathlib import Path
import random
from time import perf_counter

from . import config, producer_go, replay
from ._replay_client import ClientSources
from ._replay_git import History, changed_files, materialize


def _stamp() -> str:
    return datetime.now(timezone.utc).isoformat()


def _write_jsonl(path: Path, rows) -> None:
    with path.open("w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, ensure_ascii=False) + "\n")


def _hits(rows: list[replay.Location]) -> int:
    return sum(row.outcome.lag_commits in range(1, 11) for row in rows)


def _lift(pred: int, pred_n: int, ctrl: int, ctrl_n: int) -> str:
    lift = f"{(pred / pred_n) / (ctrl / ctrl_n):.3f}" if pred_n and ctrl and ctrl_n else "undefined"
    return f"{pred}/{pred_n} vs {ctrl}/{ctrl_n}, lift {lift}"


def _sample(results: list[replay.ReplayResult]) -> list[tuple[replay.ReplayResult, replay.Location]]:
    operations, fields = [], []
    for result in results:
        for row in result.predicted:
            if row.outcome.kind in {"later", "present_at_pin"}:
                group = operations if row.element.startswith("op:") else fields
                group.append((result, row))
    capacity = max(0, 150 - len(operations))
    if len(fields) > capacity:
        fields = random.Random(replay.SEED).sample(fields, capacity)
    return operations[:150] + fields


def _reader_rows(repo: Path, history: History, results: list[replay.ReplayResult],
                 configs: Path, root: str, schema_path: str) -> list[dict]:
    chosen = _sample(results)
    waiting = [(result, row) for result, row in chosen if row.outcome.kind == "present_at_pin"]
    if waiting:
        pin = history.pin
        fields = replay._schema_at(repo, pin, schema_path).fields
        cfg = config.load(replay.select_config(repo, pin, configs))
        with materialize(repo, pin) as snapshot:
            sources = ClientSources(snapshot, cfg, root)
            needs_operations = any(row.element.startswith("op:") for _, row in waiting)
            annotations = {row.element for row in producer_go.annotations(snapshot)} if needs_operations else set()
            registrations = producer_go.registrations(snapshot) if needs_operations else []
            for _, row in waiting:
                site = row.current_line
                assert site is not None
                row.callers = sources.callers(site.file, row.call.enclosing_function)
                if row.element.startswith("op:"):
                    method, path = row.element.removeprefix("op:").split(" ", 1)
                    row.element_served_at_pin = row.element in annotations or any(
                        route.method == method and path.endswith(route.path_suffix)
                        for route in registrations
                    )
                else:
                    row.element_served_at_pin = row.element in fields
    return [
        {
            "location_id": row.location_id,
            "change": [asdict(change) for change in result.changes if change.element == row.element][0],
            "element_changes": [asdict(change) for change in result.changes if change.element == row.element],
            "change_commit": asdict(result.change_commit),
            "parent": result.parent,
            "pin": history.pin,
            **asdict(row),
        }
        for result, row in chosen
    ]


def _inventory_text(history: History, selected: dict, raw: dict) -> str:
    ops = {sha for sha, rows in selected.items() if any(row.kind == "op_removed" for row in rows)}
    fields = {sha for sha, rows in selected.items() if any(row.kind != "op_removed" for row in rows)}
    counts = Counter(row.kind for rows in selected.values() for row in rows)
    return "\n".join([
        "# F4 history replay",
        "",
        f"Pin: `{history.pin}`",
        f"Inventory recorded before consumer extraction: {_stamp()}",
        f"First-parent commits examined: {len(history.commits)}",
        f"Commits with element changes before reverts: {sum(bool(rows) for rows in raw.values())}",
        f"Usable commits: {len(selected)}",
        f"Operation-only commits: {len(ops - fields)}",
        f"Field-only commits: {len(fields - ops)}",
        f"Commits with both: {len(ops & fields)}",
        f"Commits with operation changes including overlap: {len(ops)}",
        f"Commits with field changes including overlap: {len(fields)}",
        "Usable element changes by kind:",
        *(f"- {kind}: {counts[kind]}" for kind in sorted(replay.USABLE_KINDS)),
        "",
    ])


def _replay_text(results: list[replay.ReplayResult], reader_count: int, seconds: float) -> str:
    skips = Counter(result.skipped_reason for result in results if result.skipped_reason)
    predicted = [row for result in results for row in result.predicted]
    control = [row for result in results for row in result.control]
    outcomes = Counter(row.outcome.kind for row in predicted)
    eligible = outcomes["later"] + outcomes["present_at_pin"]
    with_predictions = sum(len({row.element for row in result.predicted}) for result in results)
    lines = [
        "",
        f"Replay wall time: {seconds:.3f} s (parent materialization through replay, excluding inventory and reader packet)",
        f"Replay completed: {_stamp()}",
        f"Replayed usable commits: {len(results)}",
        f"Skipped config_missing: {skips['config_missing']}",
        f"Skipped no_client_import: {skips['no_client_import']}",
        f"Element occurrences with predictions: {with_predictions}",
        f"Predicted locations: {len(predicted)}",
        f"Control locations: {len(control)}",
        f"Predicted outcomes: {dict(sorted(outcomes.items()))}",
        f"R3 packet: {reader_count} selected of {eligible} eligible, {eligible - reader_count} outside the cap",
        "",
        "Lift uses touches in first-parent successors +1 through +10 after C.",
        f"Pooled lift: {_lift(_hits(predicted), len(predicted), _hits(control), len(control))}",
        "",
        "## Per-commit lift",
        "",
    ]
    for result in results:
        statement = result.skipped_reason or _lift(
            _hits(result.predicted), len(result.predicted), _hits(result.control), len(result.control),
        )
        lines.append(f"- `{result.commit}` {statement}")
    lines.extend(["", "## Late touches", ""])
    for result in results:
        for row in result.predicted:
            if row.outcome.kind == "later":
                lines.append(
                    f"- `{result.commit}` → `{row.outcome.touched_in}` "
                    f"`{row.element}` at `{row.call.file}:{row.call.line}` "
                    f"(+{row.outcome.lag_commits} commits)"
                )
    return "\n".join(lines) + "\n"


def run(repo: Path, schema: Path, configs: Path, root: str, pin: str, out: Path) -> None:
    repo = repo.resolve()
    schema_path = schema.relative_to(repo).as_posix() if schema.is_absolute() else schema.as_posix()
    history = History(repo, pin)
    ops = replay.operation_sets(repo, history.commits)
    raw = {
        commit: replay._changes(repo, commit, history.commits[index - 1] if index else None, ops, schema_path)
        for index, commit in enumerate(history.commits)
    }
    selected = replay.usable(raw)
    out.mkdir(parents=True, exist_ok=True)
    summary = out / "F4-summary.md"
    summary.write_text(_inventory_text(history, selected, raw), encoding="utf-8")
    print(f"Counted {len(selected)} usable commits across {len(history.commits)} first-parent revisions.", flush=True)

    started = perf_counter()
    results = []
    for number, (commit, changes) in enumerate(selected.items(), 1):
        p0 = history.commits[history.position[commit] - 1]
        cfg = replay.select_config(repo, p0, configs)
        results.append(replay.replay_one(repo, commit, changes, cfg, root, history.pin,
                                         schema_path=schema_path, history=history))
        if number % 10 == 0:
            print(f"Processed {number} of {len(selected)} usable commits.", flush=True)
    elapsed = perf_counter() - started

    rows = _reader_rows(repo, history, results, configs, root, schema_path)
    _write_jsonl(out / "replay-results.jsonl", (asdict(result) for result in results))
    _write_jsonl(out / "f4-locations.jsonl", rows)
    with summary.open("a", encoding="utf-8") as stream:
        stream.write(_replay_text(results, len(rows), elapsed))
