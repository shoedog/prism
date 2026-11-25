"""Round-one regressions for the v2 report's evidence and admission rules."""

import ast
from dataclasses import asdict
import json
from pathlib import Path

import pytest

from schema_impact import replay, report_inputs
from schema_impact.report_analysis import build_document
from schema_impact.report_batch import main as batch
from schema_impact.report_evaluation import join_evaluation, render_evaluation
from schema_impact.report_inputs import ReportRequest, build_inputs
from schema_impact.report_markdown import render_json, render_markdown
from test_si_replay import CALL, CALL_SOURCE, CLIENT, CONFIGS, DATE, HANDLER, Repo, SCHEMA, git
from test_si_report import call, report, route_contract, schema, start


JS = "service/routes.js"
OP = "op:GET /fern"


def impacts(doc):
    return {row["operation_id"]: row for row in doc["impacts"]}


def lead(markdown):
    return markdown.split("## Review first", 1)[1].split("**Versions checked**", 1)[0]


@pytest.mark.parametrize("key,value", [("path", '"/oak"'), ("method", '"POST"')])
def test_multiline_entry_with_one_line_edited(tmp_path, key, value):
    entry = 'register([{\n  method: "GET",\n  path: "/fern",\n}]);\n'
    repo, _ = start(tmp_path, **{JS: entry})
    prior = {"path": '"/fern"', "method": '"GET"'}[key]
    head = repo.commit(1, **{JS: entry.replace(f"{key}: {prior}", f"{key}: {value}")})
    md, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    assert impacts(doc)[OP]["change_kinds"] == ["Operation removed"]
    assert "Operation added" in md
    assert doc["summary"]["operation_count"]["total"] == 2


def test_unrelated_gap_keeps_file_removal_and_identifies_actual_operation_gap(tmp_path):
    repo, _ = start(tmp_path)
    head = repo.commit(1, **{"handler.go": "package flora\n", "other.go": "package flora\n// @Router malformed\n"})
    md, doc = report(repo, head, tmp_path)
    group = impacts(doc)[OP]
    assert group["file_changes"][0]["site"]["file"] == "handler.go"
    failure, = group["proof_failures"]
    assert (failure["side"], failure["file"], failure["rev"]) == ("after", "other.go", head)
    assert "Operation removed: Not checked — after other.go" in md
    assert doc["summary"]["operation_count"]["total"] is None


@pytest.mark.parametrize("replacement", ["mount(makeEntries());\n", "mount([\n{method:'GET',path:'/fern'}\n]);\n"])
def test_renamed_registration_is_a_region_gap(tmp_path, replacement):
    repo, _ = start(tmp_path, **{JS: "register([\n{method:'GET',path:'/fern'}\n]);\n"})
    head = repo.commit(1, **{JS: replacement})
    _, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    group = impacts(doc)[OP]
    assert group["after_presence"] == "unknown"
    assert group["proof_failures"][0]["file"] == JS
    assert "outside the configured registration call" in group["proof_failures"][0]["reason"]


def test_hole_preserves_neighbor_and_spread_withholds_its_identity(tmp_path):
    old = "register([]);"
    new = "register([, {method:'GET',path:'/fern'}, {method:'GET',path:'/oak',...override}]);"
    repo, _ = start(tmp_path, **{JS: old})
    _, doc = report(repo, repo.commit(1, **{JS: new}), tmp_path, "producer", *route_contract(tmp_path))
    assert OP in impacts(doc) and "op:GET /oak" not in impacts(doc)
    assert any("array has a hole" in gap["reason"] for gap in doc["warnings"])


def test_owner_side_spread_is_the_failed_check(tmp_path):
    entry = "{method:'GET',path:'/fern'}"
    repo, _ = start(tmp_path, **{JS: f"register([{entry}, {{method:'GET',path:'/oak',...override}}]);"})
    _, doc = report(repo, repo.commit(1, **{JS: "register([]);"}), tmp_path, "producer", *route_contract(tmp_path))
    failure = impacts(doc)[OP]["proof_failures"][0]
    assert failure["side"] == "before" and "spread" in failure["reason"]


def test_same_file_duplicate_removal_retains_sibling(tmp_path):
    declaration = "{method:'GET',path:'/fern'}"
    repo, _ = start(tmp_path, **{JS: f"register([\n{declaration},\n{declaration},\n]);"})
    head = repo.commit(1, **{JS: f"register([\n{declaration},\n]);"})
    _, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    group = impacts(doc)[OP]
    assert len(group["file_changes"]) == 1
    assert group["before_presence"] == group["after_presence"] == "present"
    assert group["change_kinds"] == [f"Declaration removed from {JS}"]


def test_file_move_and_body_only_change_get_distinct_wording(tmp_path):
    repo, _ = start(tmp_path)
    head = repo.commit(1, **{"handler.go": None, "moved.go": HANDLER})
    md, doc = report(repo, head, tmp_path)
    assert "moves from `handler.go` to `moved.go`" in lead(md)
    assert doc["summary"]["operation_count"]["total"] == 1
    edit = repo.commit(2, **{"moved.go": HANDLER.replace("{}", "{ println(1) }")})
    md, _ = report(repo, edit, tmp_path)
    assert "the two source bodies were not compared" in md


@pytest.mark.parametrize("after", ["{invalid", '{"swagger":"2.0","paths":{"/fern":{"$ref":"#/missing"}}}',
                                  '{"swagger":"2.0","paths":{},"paths":{}}'])
def test_schema_gap_qualifies_operation_total_and_coverage(tmp_path, after):
    repo, _ = start(tmp_path, **{SCHEMA: schema({"leaf": {"type": "string"}})})
    md, doc = report(repo, repo.commit(1, **{SCHEMA: after}), tmp_path)
    assert doc["summary"]["operation_count"]["total"] is None
    assert not any(group["fields"] or group["status_removals"] for group in doc["impacts"])
    coverage = md.split("## Coverage", 1)[1].split("## Changes by operation", 1)[0]
    assert "not checked" in coverage and "Compared type" not in coverage


@pytest.mark.parametrize("omitted", [False, True])
def test_schema_coverage_distinguishes_omission_from_absence(tmp_path, omitted):
    repo, _ = start(tmp_path, **({SCHEMA: schema()} if omitted else {}))
    md, doc = report(repo, repo.commit(1), tmp_path, "producer", *(["--no-schema"] if omitted else []))
    coverage = md.split("## Coverage", 1)[1].split("## Changes by operation", 1)[0]
    assert "Compared type" not in coverage
    assert doc["summary"]["operation_count"]["total"] == 0
    assert ("omitted by request" if omitted else "no committed field schema") in md


def test_incomparable_status_move_withholds_children(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema({"leaf": {"type": "string"}})})
    head = repo.commit(1, **{SCHEMA: schema({"leaf": {"type": "string"}}, status="201", media="application/xml")})
    md, doc = report(repo, head, tmp_path)
    group = impacts(doc)[OP]
    assert not group["fields"]
    assert "application/xml" in md
    assert [row["status"] for row in group["status_removals"]] == ["200"]
    assert [row["status"] for row in group["status_additions"]] == ["201"]


def test_openapi_array_item_union_renders_each_type(tmp_path):
    def document(items):
        return json.dumps({"openapi": "3.1.0", "paths": {"/fern": {"get": {"responses": {
            "200": {"content": {"application/json": {"schema": {"type": "object", "properties": {
                "leaf": {"type": "array", "items": {"type": items}}}}}}}
        }}}}})
    repo, _ = start(tmp_path, **{SCHEMA: document("string")})
    md, _ = report(repo, repo.commit(1, **{SCHEMA: document(["integer", "string"])}), tmp_path)
    assert "List item type whole number or text" in md


def test_malformed_route_spec_preserves_client_observations(tmp_path):
    repo, _ = start(tmp_path)
    flags = route_contract(tmp_path)
    Path(flags[1]).write_text("{wrong")
    md, doc = report(repo, repo.commit(1, **{CALL: ""}), tmp_path, "consumer", *flags)
    assert "route specification could not be parsed or validated" in md
    assert doc["call_changes"][0]["before_site_id"]
    assert doc["call_changes"][0]["requested_target"] == ["GET", "/fern"]


def test_alias_and_mjs_calls_are_included(tmp_path):
    repo, _ = start(tmp_path)
    alias = CALL_SOURCE.replace("export const", "const r = request;\nexport const").replace("request.get", "r.get")
    _, doc = report(repo, repo.commit(1, **{"ui/src/other.mjs": alias}), tmp_path, "consumer")
    assert len(doc["call_changes"]) == 1
    after = doc["inputs"][doc["change"]["after_input_id"]["client"]]
    assert any(row["site"]["file"] == "ui/src/other.mjs" and row["mapped_path"] == "/fern" for row in after["calls"])


def test_client_barrel_dependency_affects_role_and_coverage(tmp_path):
    barrel = "ui/bridge.ts"
    source = CALL_SOURCE.replace("@/utils/request", "../bridge")
    repo, _ = start(tmp_path, **{CALL: source, barrel: 'export {default} from "./src/utils/request";\n'})
    head = repo.commit(1, **{barrel: "export {\n", "handler.go": "package flora\n"})
    md, doc = report(repo, head, tmp_path)
    assert doc["change"]["subject_role"] == "both"
    assert doc["summary"]["client_change_count"]["total"] is None
    assert "the imported source could not be parsed" in md
    assert "request paths were unavailable" not in md


def test_unrequested_side_is_reviewed_when_it_changed(tmp_path):
    repo, _ = start(tmp_path)
    md, doc = report(repo, repo.commit(1, **{"handler.go": "package flora\n"}), tmp_path, "consumer")
    assert doc["change"]["subject_role"] == "service"
    assert "removes" in lead(md)


def test_unread_client_inventory_produces_no_confirmed_zero(tmp_path):
    repo, _ = start(tmp_path, **{CALL: "const [ invalid"})
    md, doc = report(repo, repo.commit(1, **{CALL: "const [ still-invalid"}), tmp_path, "consumer")
    assert doc["summary"]["client_change_count"]["total"] is None
    assert "could not be parsed" in lead(md)


def test_function_moves_do_not_pair_and_same_line_sites_are_distinct(tmp_path):
    first = CALL_SOURCE.splitlines()[0]
    before = first + '\nclass A { load() {\n request.get("/fern");\n} }\nclass B { load() {\n} }\n'
    after = first + '\nclass A { load() {\n} }\nclass B { load() {\n request.get("/fern");\n} }\n'
    repo, _ = start(tmp_path, **{CALL: before})
    _, doc = report(repo, repo.commit(1, **{CALL: after}), tmp_path, "consumer")
    assert {row["kind"] for row in doc["call_changes"]} == {"Observed before", "Observed after"}
    assert all(not row["usage_id"] for row in doc["usages"])
    repeated = CALL_SOURCE.replace('request.get("/fern")', 'request.get("/fern") || request.get("/fern")')
    repo.commit(2, **{CALL: repeated})
    md, _ = report(repo, repo.commit(3, **{"handler.go": "package flora\n"}), tmp_path)
    assert "occurrence 1" in md and "occurrence 2" in md


def test_mixed_deletion_keeps_combined_after_distinct(tmp_path):
    repo, _ = start(tmp_path)
    md, doc = report(repo, repo.commit(1, **{CALL: CALL_SOURCE.splitlines()[0], "handler.go": "package flora\n"}), tmp_path)
    assert doc["change"]["subject_role"] == "both"
    service = md.split("### Service changes", 1)[1].split("### Client changes", 1)[0]
    assert "No call at this site" in service
    assert "If the service ships first · No matching declaration among those read" in service


def test_standing_comparison_reports_deficient_side_and_links_client_change(tmp_path):
    repo, _ = start(tmp_path, **{JS: "register([{method:'*',path:'/fern'}]);"})
    head = repo.commit(1, **{JS: "register([]);"})
    _, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    row, = doc["standing_mismatches"]
    assert row["timing_failures"][0]["side"] == "before"
    assert "exact methods only" in row["timing_failures"][0]["reason"]
    head = repo.commit(2, **{CALL: CALL_SOURCE + 'export const absent = () => request.get("/absent");\n'})
    _, doc = report(repo, head, tmp_path, "consumer", *route_contract(tmp_path))
    assert any(row["linked_to_change"] and row["after_site_id"] for row in doc["standing_mismatches"])


def test_schema_selection_and_repository_are_fingerprinted(tmp_path):
    repo, _ = start(tmp_path, **{"a.json": schema(), "b.json": schema()})
    head = repo.commit(1)
    _, a = report(repo, head, tmp_path, "producer", "--schema", "a.json")
    _, b = report(repo, head, tmp_path, "producer", "--schema", "b.json")
    assert a["report_id"] != b["report_id"]
    assert a["input_manifest"][0]["repository_id"]


def test_wrong_operation_join_and_backdated_assessment_are_qualified(tmp_path):
    extra = HANDLER.replace("/fern", "/oak").replace("Fern", "Oak").replace("package flora\n", "")
    repo, base = start(tmp_path, **{"handler.go": HANDLER + extra})
    head = repo.commit(4, **{"handler.go": "package flora\n"})
    _, live = report(repo, head, tmp_path)
    early = repo.commit(1, **{CALL: call("/other")})
    result = asdict(replay.replay_one(repo.path, head, [replay.Change("op_removed", OP)],
                                    replay.select_config(repo.path, base, CONFIGS), "ui/src", early))
    row = result["predicted"][0]
    row["element"] = "op:GET /oak"
    doc = join_evaluation(live, result, {"rev": early, "committed_at": None},
                          {row["location_id"]: {"class": "dead_code"}})
    observed, = doc["rows"]
    assert observed["site_id"] is None and "element’s operation" in observed["join_reason"]
    md = render_evaluation(doc)
    assert "dated before the service change" in md and "within 14 days" not in md
    assert "lacks searched revision, searched scope, supporting record" in md
    assert "found no direct call by name" not in md


def test_single_repo_writes_live_set_even_with_unreadable_replay_pin(tmp_path):
    repo, _ = start(tmp_path)
    service = repo.commit(1, **{"handler.go": "package flora\n"})
    client = repo.commit(2, **{CALL: call("/oak")})
    subjects = tmp_path / "subjects.tsv"
    subjects.write_text(f"01\tunused\t{service}\tunused\n")
    out = tmp_path / "batch"
    code = batch(["--repo", str(repo.path), "--pin", "missing-replay-pin", "--subjects", str(subjects),
                  "--client-subject", client, "--out-dir", str(out), "--configs", str(CONFIGS)])
    assert code == 1
    assert sorted(path.name for path in out.glob("*.md")) == [
        f"01-{service[:8]}.md", f"client-1-{client[:8]}.md", "index.md"]
    assert "github.com/apache/answer" not in (out / f"01-{service[:8]}.md").read_text()
    index = (out / "index.md").read_text()
    assert index.index("## Live reports") < index.index("## Evaluation companions")


def test_live_import_graph_forbids_replay_bypasses():
    source = Path(report_inputs.__file__).parent
    visited = set()

    def visit(module):
        if module in visited or not (source / f"{module}.py").is_file():
            return
        visited.add(module)
        tree = ast.parse((source / f"{module}.py").read_text())
        # Only the call after serializing the live index crosses into evaluation.
        excluded = {node for node in ast.walk(tree) if module == "report_batch" and isinstance(node, ast.ImportFrom)
                    and node.module == "report_batch_evaluation" and node.col_offset > 0}
        for node in ast.walk(tree):
            if node in excluded:
                continue
            if isinstance(node, ast.ImportFrom) and node.level == 1:
                names = {name.name for name in node.names}
                if node.module == "replay":
                    assert names <= {"DEFAULT_SCHEMA", "select_config"}, (module, names)
                elif node.module:
                    visit(node.module)
                else:
                    assert "replay" not in names, module
                    for name in names:
                        visit(name)
            if isinstance(node, ast.Import):
                assert all(not name.name.endswith(".replay") for name in node.names)

    for root in ("report_batch", "report_inputs", "report_analysis", "report_markdown"):
        visit(root)


@pytest.mark.parametrize("mode", ["live", "historical_as_of"])
def test_frozen_report_set_excludes_later_artifacts_in_both_modes(tmp_path, mode):
    repo, _ = start(tmp_path, **{SCHEMA: schema({"leaf": {"type": "string"}})})
    joint = repo.commit(1, **{"handler.go": HANDLER.replace("[get]", "[post]"), CALL: call(method="post")})
    empty = repo.commit(2)
    partial = repo.commit(3, **{"handler.go": "package [\n"})
    study = Repo(tmp_path / "study")
    config = "client.toml"
    study_rev = study.commit(0, **{config: (CONFIGS / "apache-answer.toml").read_text()})
    subjects = {"joint": joint, "empty": empty, "partial": partial}
    requests = {}
    for name, head in subjects.items():
        stamp = git(repo.path, "show", "-s", "--format=%cI", head)
        requests[name] = ReportRequest(repo.path, head, "producer", Path(config), mode=mode,
                                       selection_time=stamp, study_repo=study.path, study_rev=study_rev,
                                       cutoff_receipt={"subject": head, "observed_at": stamp, "study_rev": study_rev})

    def render_all(inputs):
        reports = {}
        for name, frozen in inputs.items():
            doc = build_document(frozen)
            assert doc["mode"] == mode
            reports[name] = render_json(doc).encode(), render_markdown(doc).encode()
        return reports

    frozen = {name: build_inputs(request) for name, request in requests.items()}
    before = render_all(frozen)
    assert json.loads(before["joint"][0])["change"]["subject_role"] == "both"
    assert json.loads(before["empty"][0])["summary"]["operation_count"]["total"] == 0
    assert json.loads(before["partial"][0])["summary"]["operation_count"]["total"] is None
    # Move the refs, restore the original service, and vary working bytes and unadmitted study files.
    later = repo.commit(1, **{"handler.go": HANDLER, CALL: call("/later")})
    git(repo.path, "tag", "later-release", later)
    git(repo.path, "update-ref", "refs/heads/integration", later)
    (repo.path / CALL).write_text(call("/dirty"))
    (repo.path / "untracked.go").write_text(HANDLER)
    study.commit(4, **{config: "[corrupt", "saved-bindings.json": "later"})
    for artifact in ("run.json", "followed-lines.json", "judgments.json", "subjects.tsv", "restorations.json"):
        (tmp_path / artifact).write_text("later evidence")
    joint_live = json.loads(before["joint"][0])
    base = joint_live["change"]["before"]
    evidence = replay.replay_one(repo.path, joint, [replay.Change("op_removed", OP)],
                                 replay.select_config(repo.path, base, CONFIGS), "ui/src", later)
    assert evidence.predicted
    end = {"rev": later, "committed_at": git(repo.path, "show", "-s", "--format=%cI", later)}
    companion = render_evaluation(join_evaluation(joint_live, evidence, end))
    evidence.control.append(evidence.predicted[0])
    later_labels = {evidence.predicted[0].location_id: {"class": "unrelated_edit", "reason": "Later study judgment"}}
    revised_companion = render_evaluation(join_evaluation(joint_live, evidence, end, later_labels))
    assert companion != revised_companion and "Later study judgment" in revised_companion
    assert render_all(frozen) == before
    assert render_all({name: build_inputs(request) for name, request in requests.items()}) == before
    # A different admitted declaration changes the result even when the same study tree is used.
    control = repo.commit(5, **{"handler.go": HANDLER.replace("/fern", "/new")})
    stamp = git(repo.path, "show", "-s", "--format=%cI", control)
    request = ReportRequest(repo.path, control, "producer", Path(config), mode=mode, selection_time=stamp,
                            study_repo=study.path, study_rev=study_rev,
                            cutoff_receipt={"subject": control, "observed_at": stamp, "study_rev": study_rev})
    assert b"op:GET /new" in render_all({"control": build_inputs(request)})["control"][0]


def test_configuration_read_error_is_distinct_from_missing(tmp_path, monkeypatch):
    repo, _ = start(tmp_path)
    subject = repo.commit(1)
    cfg = tmp_path / "study.toml"
    cfg.write_text((CONFIGS / "apache-answer.toml").read_text())
    read_bytes = Path.read_bytes

    def denied(path):
        if path == cfg:
            raise PermissionError("denied")
        return read_bytes(path)

    monkeypatch.setattr(Path, "read_bytes", denied)
    md, doc = report(repo, subject, tmp_path, "consumer", "--configs", str(cfg))
    assert "client configuration could not be read" in md
    assert "client configuration is missing" not in md
    assert any(row["file"] == cfg.name for row in doc["warnings"])


def test_prefix_hint_does_not_count_possible_match_to_another_operation(tmp_path):
    before = 'register([{method:"GET",path:"/edge/literal/fern"}]);\n'
    after = before + 'register([{method:"GET",path:"/internal/fern"}]);\n'
    client = CALL_SOURCE.replace('() => request.get("/fern")', '(key) => request.get(`/edge/${key}/fern`)')
    repo, _ = start(tmp_path, **{JS: before, CALL: client})
    md, doc = report(repo, repo.commit(1, **{JS: after}), tmp_path, "producer", *route_contract(tmp_path))
    assert impacts(doc)["op:GET /internal/fern"]["prefix_hints"]
    assert all(row["match_after"]["status"] == "possible" for row in doc["usages"])
    assert doc["summary"]["possible_only_usage_count"]["total"] == 0
    assert "Distinct client calls shown" in md


def test_single_repo_batch_route_spec_matches_cli_schema_default(tmp_path):
    repo, _ = start(tmp_path, **{JS: "register([]);", SCHEMA: schema({"x": {"type": "integer"}})})
    head = repo.commit(1, **{JS: 'register([{method:"GET",path:"/fern"}]);'})
    subjects = tmp_path / "subjects.tsv"
    subjects.write_text(f"01\tunused\t{head}\tunused\n")
    flags = route_contract(tmp_path)
    _, direct = report(repo, head, tmp_path, "producer", *flags)
    out = tmp_path / "batch"
    batch(["--repo", str(repo.path), "--pin", head, "--subjects", str(subjects),
           "--out-dir", str(out), "--configs", str(CONFIGS), *flags])
    import gzip
    batched = json.loads(gzip.decompress((out / f"01-{head[:8]}.json.gz").read_bytes()))
    assert direct["summary"]["changed_field_count"]["total"] == batched["summary"]["changed_field_count"]["total"]
    assert batched["summary"]["changed_field_count"]["total"] is None
    assert not any(s["source_url"] for s in batched["inputs"].values())
