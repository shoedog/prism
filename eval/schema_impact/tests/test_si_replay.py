from collections import Counter
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import random
import subprocess

import pytest

from schema_impact import config, producer_go, render, replay
from schema_impact.cli import main
from schema_impact.producer_route_table import RouteSpec
from schema_impact.schema import Change


SCHEMA = "docs/swagger.json"
ROOT = "ui/src"
CLIENT = f"{ROOT}/utils/request.ts"
CALL = f"{ROOT}/service.ts"
FORM = f"{ROOT}/form.tsx"
OP = "op:GET /fern"
FIELD = "field:resp:GET /fern:200#/leaf"
HANDLER = "package flora\n\n// @Router /fern [get]\nfunc Fern() {}\n"
CALL_SOURCE = 'import request from "@/utils/request";\nexport const fern = () => request.get("/fern");\n'
CONFIGS = Path(config.__file__).parent / "configs"
DATE = datetime(2024, 1, 1, tzinfo=timezone.utc)


def git(repo, *args, env=None):
    return subprocess.check_output(["git", "-C", str(repo), *args], env=env, text=True).strip()


class Repo:
    def __init__(self, path):
        self.path = path
        path.mkdir()
        git(path, "init", "-q", "--template=")

    def commit(self, day, **files):
        for name, contents in files.items():
            path = self.path / name
            if contents is None:
                path.unlink()
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(contents, encoding="utf-8")
        date = (DATE + timedelta(days=day)).isoformat()
        env = os.environ | {"GIT_AUTHOR_DATE": date, "GIT_COMMITTER_DATE": date}
        git(self.path, "add", "-A")
        git(self.path, "-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit",
            "-qm", f"Change on day {day}", "--allow-empty", env=env)
        return git(self.path, "rev-parse", "HEAD")


def swagger(*, name="Fern", fields=None, paths=("/fern",)):
    body = {"type": "object", "properties": {"leaf": {"type": "string"}} if fields is None else fields}
    return json.dumps({
        "swagger": "2.0",
        "paths": {
            path: {"get": {"responses": {"200": {"schema": {"$ref": f"#/definitions/{name}"}}}}}
            for path in paths
        },
        "definitions": {name: body},
    }, indent=2)


def initial(repo, **files):
    return repo.commit(0, **{
        "handler.go": HANDLER, SCHEMA: swagger(), CLIENT: "export default {};", CALL: CALL_SOURCE,
    } | files)


def inventory(repo, pin):
    commits = replay.first_parents(repo, pin)
    ops = replay.operation_sets(repo, commits)
    raw = {commit: replay.changes(repo, commit, ops, SCHEMA) for commit in commits}
    return raw, replay.usable(raw)


def predict(repo, commit, pin, changes):
    p0 = git(repo, "rev-parse", f"{commit}^")
    return replay.replay_one(repo, commit, changes, replay.select_config(repo, p0, CONFIGS), ROOT, pin)


@pytest.mark.parametrize(("change_call", "later_call", "kind", "lag"), [
    (False, "export const fern = () => null;\n", "later", 1),
    (True, None, "same_commit", 0),
    (False, None, "present_at_pin", None),
    (False, "\n// Move the call down.\n" + CALL_SOURCE, "present_at_pin", None),
])
def test_si_replay_follows_call_lines(tmp_path, change_call, later_call, kind, lag):
    repo = Repo(tmp_path / "producer")
    p0 = initial(repo)
    edits = {"handler.go": HANDLER.replace("// @Router /fern [get]\n", "")}
    if change_call:
        edits[CALL] = "export const fern = () => null;\n"
    change = repo.commit(1, **edits)
    pin = repo.commit(3, **({CALL: later_call} if later_call else {}))

    assert replay.first_parents(repo.path, pin) == [p0, change, pin]
    _, selected = inventory(repo.path, pin)
    assert selected == {change: [Change("op_removed", OP)]}
    result = predict(repo.path, change, pin, selected[change])
    assert [(row.call.file, row.call.line, row.field_spelled) for row in result.predicted] == [(CALL, 2, False)]
    expected_touch = {"later": pin, "same_commit": change, "present_at_pin": None}[kind]
    assert result.predicted[0].outcome == replay.Outcome(kind, expected_touch, lag, {"later": 2, "same_commit": 0}.get(kind))
    assert replay.follow(repo.path, p0, [(CALL, 2)], pin) == result.predicted[0].outcome
    assert result.producer_locations[OP][0].file == "handler.go"


def test_si_replay_reads_only_changed_go_and_keeps_duplicate_annotations(tmp_path, monkeypatch):
    repo = Repo(tmp_path / "producer")
    base = initial(repo, **{"backup.go": HANDLER, "stable.go": "package flora\n", "note.txt": "old"})
    duplicate_removed = repo.commit(1, **{"backup.go": None, "new.go": "package flora\n", "note.txt": "new"})
    method_changed = repo.commit(2, **{"handler.go": HANDLER.replace("[get]", "[post]")})
    calls = []
    extract = producer_go.annotations_from_sources

    def record(files):
        calls.append(sorted(files))
        return extract(files)

    monkeypatch.setattr(producer_go, "annotations_from_sources", record)
    result = replay.operation_sets(repo.path, [base, duplicate_removed, method_changed])
    assert result == {base: {OP}, duplicate_removed: {OP}, method_changed: {"op:POST /fern"}}
    assert [files for files in calls if files] == [
        ["backup.go", "handler.go", "stable.go"], ["new.go"], ["handler.go"],
    ]


def test_si_replay_operation_restore_at_ten_and_after_ten(tmp_path):
    repo = Repo(tmp_path / "producer")
    initial(repo)
    removal = repo.commit(1, **{"handler.go": "package flora\n"})
    for day in range(2, 11):
        repo.commit(day)
    restored = repo.commit(11, **{"handler.go": HANDLER})
    assert inventory(repo.path, restored)[1] == {}

    removal_again = repo.commit(12, **{"handler.go": "package flora\n"})
    for day in range(13, 23):
        repo.commit(day)
    late = repo.commit(23, **{"handler.go": HANDLER})
    assert inventory(repo.path, late)[1] == {removal_again: [Change("op_removed", OP)]}
    assert removal != removal_again


def test_si_replay_wire_rename_and_schema_regress_restore(tmp_path):
    repo = Repo(tmp_path / "producer")
    paths = ("/fern", "/hedge")
    initial(repo, **{SCHEMA: swagger(paths=paths)})
    renamed = swagger(name="Sprig", paths=paths)
    rename = repo.commit(1, **{SCHEMA: renamed})
    regressed = swagger(paths=("/hedge", "/ash"), fields={
        "leaf": {"type": "string"}, "stale": {"type": "boolean"},
    })
    regress = repo.commit(2, **{SCHEMA: regressed})
    restore = repo.commit(3, **{SCHEMA: renamed})
    raw, selected = inventory(repo.path, restore)

    assert raw[rename] == []
    assert [(row.kind, row.element) for row in raw[regress]] == [("field_removed", FIELD)]
    assert {row.element for row in raw[restore]} == {
        "field:resp:GET /ash:200#/leaf", "field:resp:GET /ash:200#/stale",
        "field:resp:GET /hedge:200#/stale",
    }
    assert selected == {}


def test_si_replay_field_restoration_is_matched_by_shape(tmp_path):
    repo = Repo(tmp_path / "producer")
    initial(repo)
    gone = repo.commit(1, **{SCHEMA: swagger(fields={})})
    repo.commit(2, **{SCHEMA: swagger(fields={"leaf": {"type": "integer"}})})
    pin = repo.commit(3, **{SCHEMA: swagger()})
    raw, selected = inventory(repo.path, pin)
    assert raw[pin][0].kind == "field_type_changed"
    assert selected == {}
    assert raw[gone][0].element == FIELD


def test_si_replay_annotation_removal_suppresses_its_fields(tmp_path):
    repo = Repo(tmp_path / "producer")
    initial(repo)
    change = repo.commit(1, **{"handler.go": "package flora\n", SCHEMA: swagger(paths=())})
    assert inventory(repo.path, change)[1] == {change: [Change("op_removed", OP)]}


def test_si_replay_era_config_and_distinct_skip_reasons(tmp_path, monkeypatch):
    repo = Repo(tmp_path / "producer")
    old = initial(repo, **{CALL: CALL_SOURCE.replace("@/utils/", "@answer/utils/")})
    change = repo.commit(1, **{"handler.go": "package flora\n"})
    monkeypatch.setattr(replay, "EARLY_ERA_START", old)
    monkeypatch.setattr(replay, "CURRENT_ERA_START", change)

    assert replay.select_config(repo.path, old, CONFIGS).name == "apache-answer-2022.toml"
    assert replay.select_config(repo.path, change, CONFIGS).name == "apache-answer.toml"
    assert len(predict(repo.path, change, change, [Change("op_removed", OP)]).predicted) == 1

    unresolvable = predict(repo.path, repo.commit(2), git(repo.path, "rev-parse", "HEAD"),
                           [Change("op_removed", OP)])
    assert unresolvable.skipped_reason == "no_client_import"
    no_client = repo.commit(3, **{CLIENT: None})
    pin = repo.commit(4)
    assert predict(repo.path, pin, pin, [Change("op_removed", OP)]).skipped_reason == "config_missing"
    assert no_client != pin


def test_si_replay_resolvable_import_without_call_is_an_observed_zero(tmp_path):
    repo = Repo(tmp_path / "producer")
    initial(repo, **{CALL: CALL_SOURCE.splitlines()[0] + "\n"})
    change = repo.commit(1, **{"handler.go": "package flora\n"})
    result = predict(repo.path, change, change, [Change("op_removed", OP)])
    assert result.skipped_reason is None
    assert result.predicted == []


def test_si_replay_control_sample_is_seeded_excludes_predictions_and_follows(tmp_path):
    repo = Repo(tmp_path / "producer")
    other_calls = [f'export const plot{i} = () => request.get("/plot{i}");' for i in range(8)]
    initial(repo, **{CALL: CALL_SOURCE + "\n".join(other_calls) + "\n",
                     SCHEMA: swagger(paths=("/fern", *(f"/plot{i}" for i in range(8))))})
    change = repo.commit(1, **{"handler.go": "package flora\n"})
    pin = repo.commit(2, **{CALL: CALL_SOURCE})
    result = predict(repo.path, change, pin, [Change("op_removed", OP)])
    assert [row.call.line for row in result.control] == random.Random(20261120).sample(list(range(3, 11)), 5)
    assert all(row.outcome == replay.Outcome("later", pin, 1, 1) for row in result.control)
    assert result.predicted[0].outcome.kind == "present_at_pin"


@pytest.mark.parametrize("through_barrels", [False, True])
def test_si_replay_follows_properties_in_named_importer(tmp_path, through_barrels):
    repo = Repo(tmp_path / "producer")
    form = 'import { fern as getFern } from "./service";\nexport const render = async () => (await getFern()).leaf;\n'
    decoy = f"{ROOT}/decoy.ts"
    barrels = {}
    if through_barrels:
        form = form.replace("./service", "./garden")
        barrels = {
            f"{ROOT}/garden.ts": 'export * from "./flowers";\n',
            f"{ROOT}/flowers.ts": 'export * from "./service";\nexport * from "./garden";\n',
        }
    initial(repo, **{FORM: form, decoy: "const unrelated = {leaf: 1};\n"} | barrels)
    change = repo.commit(1, **{SCHEMA: swagger(fields={})})
    repo.commit(2, **{decoy: "const unrelated = {};\n"})
    pin = repo.commit(3, **{FORM: form.replace(".leaf", "")})
    result = predict(repo.path, change, pin, inventory(repo.path, pin)[1][change])

    assert len(result.predicted) == 1
    row = result.predicted[0]
    assert row.field_spelled is True
    assert [(line.file, line.line) for line in row.followed_lines] == [(CALL, 2), (FORM, 2)]
    assert row.outcome == replay.Outcome("later", pin, 2, 2)
    assert row.touching_commit.commit == pin
    assert FORM in row.hunks[0] and "-export const render" in row.hunks[0]


def test_si_replay_cli_receipts_and_rendering(tmp_path):
    repo = Repo(tmp_path / "producer")
    p0 = initial(repo, **{FORM: 'import { fern as getFern } from "./service";\ngetFern();\n'})
    change = repo.commit(1, **{"handler.go": "package flora\n"})
    git(repo.path, "tag", "v1.0.0", change)
    pin = repo.commit(2, **{CALL: "// Line shifted\n" + CALL_SOURCE})
    out = tmp_path / "reports"

    assert main(["replay", "--repo", str(repo.path), "--schema", SCHEMA, "--configs", str(CONFIGS),
                 "--root", ROOT, "--pin", pin, "--out", str(out)]) == 0
    rows = [json.loads(line) for line in (out / "replay-results.jsonl").read_text().splitlines()]
    assert len(rows) == 1 and rows[0]["commit"] == change and rows[0]["parent"] == p0
    evidence = json.loads((out / "f4-locations.jsonl").read_text())
    assert evidence["change"]["element"] == OP and evidence["change_commit"]["earliest_release_tag"] == "v1.0.0"
    assert evidence["predicted_line"]["line"] == 2 and evidence["current_line"]["line"] == 3
    assert evidence["callers"][0]["file"] == FORM and evidence["element_served_at_pin"] is False
    assert (out / "F4-summary.md").read_text().index("Usable commits: 1") < (
        out / "F4-summary.md").read_text().index("Replay wall time")
    markdown = render.report(rows[0], {evidence["location_id"]: {"class": "real_mismatch"}})
    for required in (change, "handler.go:3", f"{CALL}:2", "present_at_pin", "real_mismatch", "What happened next"):
        assert required in markdown


def test_si_replay_cross_uses_committer_dates_and_route_restorations(tmp_path):
    producer = Repo(tmp_path / "producer")
    consumer = Repo(tmp_path / "consumer")
    routes = '''register([
    {verb: "get", pattern: "/fern"}, {verb: "get", pattern: "/moss"},
    {verb: "get", pattern: "/bud"}, {verb: "get", pattern: "/stem"},
]);'''
    producer.commit(0, **{"routes.js": routes})
    producer.commit(1, **{"routes.js": routes.replace('{verb: "get", pattern: "/fern"},', "")})
    producer.commit(2, **{"routes.js": routes})
    change = producer.commit(5, **{"routes.js": 'register([{verb: "post", pattern: "/moss"}]);'})
    calls = ['fetch("/garden/moss");', 'fetch("/garden/bud");', 'fetch("/garden/stem");']
    consumer.commit(1, **{"src/service.ts": "\n".join(calls) + "\n"})
    current = consumer.commit(4)
    within = consumer.commit(19, **{"src/service.ts": "\n".join(calls[1:]) + "\n"})
    later = consumer.commit(20, **{"src/service.ts": calls[2] + "\n"})
    spec = RouteSpec("register", "verb", "pattern", "handler", "validation", mount="/garden")
    results = replay.replay_cross(producer.path, consumer.path, spec, config.Config(), "src")

    assert [(row.commit, row.consumer_revision) for row in results] == [(change, current)]
    assert Counter(row.outcome.kind for row in results[0].predicted) == {
        "within_14_days": 1, "later": 1, "present_at_head": 1,
    }
    observed = {row.element: row.outcome for row in results[0].predicted}
    assert observed["op:GET /garden/moss"].touched_in == within
    assert observed["op:GET /garden/moss"].lag_days == 14
    assert observed["op:GET /garden/bud"].touched_in == later
    assert observed["op:GET /garden/bud"].lag_days == 15
    assert observed["op:GET /garden/stem"].touched_in is None
