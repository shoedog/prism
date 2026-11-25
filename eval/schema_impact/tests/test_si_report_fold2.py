"""Round-two report regressions."""

import json
from pathlib import Path

import pytest

from schema_impact import replay, report_inputs
from schema_impact._ts_imports import Resolver
from schema_impact.cli import main
from schema_impact.report_batch import main as batch
from schema_impact.report_evaluation import join_evaluation
from test_si_replay import CALL, CALL_SOURCE, CLIENT, CONFIGS, HANDLER, Repo, SCHEMA, git
from test_si_report import call, report, route_contract, schema, start
from test_si_report_fold1 import JS, OP, impacts, lead


def test_import_resolved_by_filesystem_must_exist_in_git_tree(tmp_path, monkeypatch):
    source = 'import request from "@/utils/Request";\nrequest.get("/fern");\n'
    repo, _ = start(tmp_path)
    head = repo.commit(1, **{CALL: source})
    original = Resolver.file

    def insensitive_file(self, importer, module):
        resolved = original(self, importer, module)
        if resolved is None and module == "@/utils/Request":
            resolved = original(self, importer, "@/utils/request").with_name("Request.ts")
        return resolved

    monkeypatch.setattr(Resolver, "file", insensitive_file)
    markdown, doc = report(repo, head, tmp_path, "consumer")
    gap, = [row for row in doc["warnings"] if "absent from the selected tree" in row["reason"]]
    assert gap["file"] == CALL
    assert "@/utils/Request" in gap["reason"]
    after = doc["change"]["after_input_id"]["client"]
    assert CLIENT in doc["inputs"][after]["expected_files"]
    assert "absent from the selected tree" in markdown


@pytest.mark.parametrize(("source", "files"), [
    ('const run = (api) => api.get("/oak");\nrun(request);', {}),
    ('const api = cond ? request : other;\napi.get("/oak");', {}),
    ('class Api { run() { this.api = request; this.api.get("/oak"); } }', {}),
    ('const api = {request};\napi.request.get("/oak");', {}),
    ('const {get} = request;\nget("/oak");', {}),
    ('let api;\napi = request;\napi.get("/oak");', {}),
    ('const api = require("@/utils/request");\napi.get("/oak");', {}),
    ('import api from "./second";\napi.get("/oak");', {
        "ui/src/first.ts": 'export {default} from "@/utils/request";',
        "ui/src/second.ts": 'export {default} from "./first";',
    }),
])
def test_unaccounted_client_reference_qualifies_inventory(tmp_path, source, files):
    repo, _ = start(tmp_path, **files)
    head = repo.commit(1, **{CALL: CALL_SOURCE + source + "\n"})
    markdown, doc = report(repo, head, tmp_path, "consumer")
    gaps = [gap for gap in doc["warnings"] if gap["check"] == "client call inventory"]
    assert gaps and all(gap["file"] == CALL and gap["line"] >= 3 for gap in gaps)
    assert all("recognised callee" in gap["reason"] for gap in gaps)
    assert doc["summary"]["client_change_count"]["total"] is None
    assert "No client calls were added, removed or retargeted" not in markdown


@pytest.mark.parametrize(("hook_call", "method", "fetcher_arg"), [
    ('useSWR<Result>(["/oak", params], request.instance.post,);', "POST", None),
    ('swr("/oak", (request.instance["get"] as Fetcher));', "GET", None),
    ('useSWR("/oak", options, request.instance.post);', "POST", 2),
])
def test_recognised_hook_fetcher_is_accounted(tmp_path, hook_call, method, fetcher_arg):
    cli_args = ()
    if fetcher_arg is not None:
        configs = tmp_path / "configs"
        configs.mkdir()
        name = "apache-answer.toml"
        (configs / name).write_text(
            (CONFIGS / name).read_text().replace("fetcher_arg = 1", f"fetcher_arg = {fetcher_arg}"),
        )
        cli_args = ("--configs", str(configs))
    repo, _ = start(tmp_path)
    source = CALL_SOURCE + 'import swr from "swr";\n' + hook_call + "\n"
    _, doc = report(repo, repo.commit(1, **{CALL: source}), tmp_path, "consumer", *cli_args)
    assert not [gap for gap in doc["warnings"] if gap["check"] == "client call inventory"]
    after = doc["inputs"][doc["change"]["after_input_id"]["client"]]
    fetcher, = [row for row in after["calls"] if row["site"]["line"] == 4]
    assert fetcher["requested_method"] == method
    assert doc["summary"]["client_change_count"]["total"] == 1


def test_parameter_alias_stays_unaccounted_alongside_hook_fetcher(tmp_path):
    repo, _ = start(tmp_path)
    source = CALL_SOURCE + '''
const run = (api) => api.get("/oak");
run(request);
useSWR("/oak", request.instance.get);
useSWR("/oak", fetch,
  request.instance.post);
'''
    _, doc = report(repo, repo.commit(1, **{CALL: source}), tmp_path, "consumer")
    gaps = [gap for gap in doc["warnings"] if gap["check"] == "client call inventory"]
    assert [(gap["file"], gap["line"]) for gap in gaps] == [(CALL, 5), (CALL, 8)]


def test_recognised_bind_receiver_is_accounted(tmp_path):
    repo, _ = start(tmp_path)
    source = CALL_SOURCE + '''
const bound = request.instance.post.bind(
  request.instance,
  request);
const other = request.instance.post.bind(
  request);
useSWR("/oak", () => request.instance.post.bind(
  request.instance));
'''
    _, doc = report(repo, repo.commit(1, **{CALL: source}), tmp_path, "consumer")
    gaps = [gap for gap in doc["warnings"] if gap["check"] == "client call inventory"]
    assert [(gap["file"], gap["line"]) for gap in gaps] == [(CALL, 6), (CALL, 8)]


@pytest.mark.parametrize("replacement", [
    "register([]); mount(makeEntries());\n",
    "/* comment */ mount(makeEntries());\n",
    "const keep = 1;\nmount(makeEntries());\n",
])
def test_registration_replacement_requires_byte_accounting(tmp_path, replacement):
    prior = "register([{method:'GET',path:'/fern'}]);\n"
    if replacement.startswith("const"):
        prior += "const keep = 1;\n"
    repo, _ = start(tmp_path, **{JS: prior})
    md, doc = report(repo, repo.commit(1, **{JS: replacement}), tmp_path, "producer", *route_contract(tmp_path))
    assert "Operation removed" not in impacts(doc)[OP]["change_kinds"]
    assert "outside the configured registration call" in md
    assert any(gap["check"] == "registration region" for gap in doc["warnings"])


@pytest.mark.parametrize("add", [False, True])
def test_one_sided_schema_withholds_confirmed_zero(tmp_path, add):
    shape = schema(body=False)
    repo, _ = start(tmp_path, **({} if add else {SCHEMA: shape}))
    md, doc = report(repo, repo.commit(1, **{SCHEMA: shape if add else None}), tmp_path)
    count = doc["summary"]["operation_count"]
    assert count["total"] is None and count["observed"] == 0
    assert ("before" if add else "after") + " " + SCHEMA in count["reason"]
    assert "no measured change" not in lead(md)


def test_unresolved_response_is_not_a_bodyless_success(tmp_path):
    unresolved = json.loads(schema())
    unresolved["paths"]["/fern"]["get"]["responses"]["200"] = {"$ref": "#/responses/missing"}
    repo, _ = start(tmp_path, **{SCHEMA: json.dumps(unresolved)})
    head = repo.commit(1, **{SCHEMA: schema(body=False, status="204")})
    md, doc = report(repo, head, tmp_path)
    assert "Successful response status removed · 200" in md and "Successful response status added · 204" in md
    assert doc["summary"]["changed_field_count"]["total"] is None
    assert "response body (200)" in doc["summary"]["changed_field_count"]["reason"]
    assert "Compared type" not in md.split("## Coverage", 1)[1].split("## Changes by operation", 1)[0]


def test_route_spec_does_not_imply_schema_path_absence(tmp_path):
    repo, _ = start(tmp_path, **{JS: "register([]);", SCHEMA: schema()})
    md, doc = report(repo, repo.commit(1), tmp_path, "producer", *route_contract(tmp_path))
    assert "no field schema path was selected" in md
    assert "no committed field schema" not in md
    assert all(row["schema"]["status"] == "unselected" for row in doc["inputs"].values() if row["schema"])


def test_nonchange_route_never_displaces_addition_or_inflates_call_count(tmp_path):
    entry = "{method:'GET',path:'/edge/literal/fern'}"
    source = CALL_SOURCE.replace('"/fern"', '`/edge/${key}/fern`')
    repo, _ = start(tmp_path, **{JS: f"register([{entry}]);", CALL: source})
    head = repo.commit(1, **{JS: f"register([{entry}, {{method:'GET',path:'/internal/oak'}}]);"})
    md, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    assert "The service adds " + "`GET /internal/oak`" in lead(md)
    assert "op:GET /edge/literal/fern" not in impacts(doc)
    assert doc["summary"]["listed_usage_count"]["total"] == 0


def test_multiplicity_does_not_support_declaration_move(tmp_path):
    duplicated = HANDLER + HANDLER.replace("package flora\n\n", "").replace("Fern()", "FernAgain()")
    repo, _ = start(tmp_path, **{"handler.go": duplicated})
    head = repo.commit(1, **{"handler.go": None, "other.go": HANDLER})
    md, doc = report(repo, head, tmp_path)
    assert "moves from" not in lead(md)
    assert len(impacts(doc)[OP]["file_changes"]) == 3


def test_object_literal_owner_prevents_false_correspondence(tmp_path):
    body = '  load() {\n    return request.get("/fern");\n  },\n'
    prefix = CALL_SOURCE.splitlines()[0] + "\n"
    old = prefix + f"const a = {{\n{body}}};\nconst b = {{\n}};\n"
    new = prefix + f"const a = {{\n}};\nconst b = {{\n{body}}};\n"
    repo, _ = start(tmp_path, **{CALL: old})
    _, doc = report(repo, repo.commit(1, **{CALL: new}), tmp_path, "consumer")
    assert {row["kind"] for row in doc["call_changes"]} == {"Observed before", "Observed after"}
    assert all(row["usage_id"] is None for row in doc["usages"])


def test_service_first_gain_names_the_checked_deployment_pair(tmp_path):
    source = CALL_SOURCE.splitlines()[0] + '\nexport const fern = () => request.get("/fern");\n'
    repo, _ = start(tmp_path, **{"handler.go": "package flora\n", CALL: source})
    head = repo.commit(1, **{"handler.go": HANDLER, CALL: source.splitlines()[0] + "\n"})
    md, doc = report(repo, head, tmp_path)
    assert "gains a match if the service ships first" in lead(md)
    risk, = impacts(doc)[OP]["risks"]
    assert risk["comparison_purposes"] == ["before", "service_first"]
    assert [p["matches"].get(risk["site_id"], {}).get("status") for p in doc["comparisons"]] == [
        "no_match", None, "match", None]


def test_standing_row_names_unproved_correspondence(tmp_path):
    old = CALL_SOURCE.replace('"/fern"', '"/oak"')
    repo, _ = start(tmp_path, **{CALL: old})
    md, doc = report(repo, repo.commit(1, **{CALL: old.replace("const fern", "const oak")}), tmp_path, "consumer")
    rows = [row for row in doc["standing_mismatches"] if row["before_site_id"] or row["after_site_id"]]
    assert len(rows) == 2
    assert all("correspondence is unproved" in row["timing_failures"][0]["reason"] for row in rows)
    assert "there is no corresponding request" not in md


def test_observed_operation_does_not_join_replay_change(tmp_path):
    repo, base = start(tmp_path, **{SCHEMA: schema()})
    head = repo.commit(1, **{"handler.go": "package [\n"})
    pin = repo.commit(2, **{CALL: call("/oak")})
    _, doc = report(repo, head, tmp_path)
    result = replay.replay_one(repo.path, head, [replay.Change("op_removed", OP)],
                               replay.select_config(repo.path, base, CONFIGS), "ui/src", pin)
    end = {"rev": pin, "committed_at": git(repo.path, "show", "-s", "--format=%cI", pin)}
    row, = join_evaluation(doc, result, end)["rows"]
    assert row["site_id"] is None
    assert "no measured change" in row["join_reason"]


def test_batch_indexes_inferred_role_and_keeps_companion_index_separate(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema()})
    head = repo.commit(1, **{CALL: CALL_SOURCE + 'request.get("/oak");\n'})
    subjects = tmp_path / "subjects.tsv"
    subjects.write_text(f"01\tunused\t{head}\tunused\n")
    out = tmp_path / "batch"
    assert batch(["--repo", str(repo.path), "--pin", head, "--subjects", str(subjects),
                  "--out-dir", str(out), "--configs", str(CONFIGS)]) == 0
    index = (out / "index.md").read_text()
    assert "Client change" in index and "Service change" not in index
    assert ".evaluation." not in index
    assert ".evaluation." in (out / "evaluation-index.md").read_text()


def test_each_absence_cell_uses_its_own_side_and_retains_both_failures(tmp_path):
    entry = "{method:'GET',path:'/fern'}"
    other = "{method:'GET',path:'/oak',...override}"
    repo, _ = start(tmp_path, **{JS: f"register([{entry}, {other}]);"})
    head = repo.commit(1, **{JS: "register([ ({ broken\n"})
    md, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    group = impacts(doc)[OP]
    assert {row["side"] for row in group["proof_failures"]} == {"before", "after"}
    after = group["presence_claims"]["after"]["failures"]
    assert after and all(row["side"] == "after" and "spread" not in row["reason"] for row in after)
    assert "spread" in md and "could not be parsed" in md
    assert "spread" in lead(md) and "could not be parsed" in lead(md)


def test_proved_deletion_has_no_unknown_presence_reason(tmp_path):
    before = CALL_SOURCE + 'export const oak = () => request.get("/oak");\n'
    repo, _ = start(tmp_path, **{CALL: before})
    _, doc = report(repo, repo.commit(1, **{CALL: CALL_SOURCE}), tmp_path, "consumer")
    usage, = doc["usages"]
    assert usage["after_presence"] == "absent" and usage["presence_reason"] is None


@pytest.mark.parametrize("mode", ["live", "historical_as_of"])
def test_actual_cli_selection_uses_frozen_requests_and_ignores_later_driver_checkout(tmp_path, monkeypatch, mode):
    old_shape = schema({"leaf": {"type": "string"}})
    new_shape = schema({"leaf": {"type": "integer"}})
    repo, _ = start(tmp_path, **{SCHEMA: old_shape})
    changed_field = repo.commit(1, **{SCHEMA: new_shape})
    client_only = repo.commit(2, **{CALL: CALL_SOURCE + 'export const oak = () => request.get("/oak");\n'})
    joint = repo.commit(3, **{"handler.go": HANDLER.replace("[get]", "[post]"), CALL: call(method="post")})
    empty = repo.commit(4)
    partial = repo.commit(5, **{"handler.go": "package [\n"})
    study = Repo(tmp_path / "study")
    config = "client.toml"
    old_study = study.commit(0, **{config: (CONFIGS / "apache-answer.toml").read_text()})
    requests = []
    for name, head, role in [
        ("field", changed_field, "producer"), ("client", client_only, "consumer"), ("joint", joint, "producer"),
        ("empty", empty, "producer"), ("partial", partial, "producer"),
    ]:
        reference = f"refs/heads/{name}"
        git(repo.path, "update-ref", reference, head)
        stamp = git(repo.path, "show", "-s", "--format=%cI", head)
        receipt = tmp_path / f"{name}.json"
        receipt.write_text(json.dumps({"subject": head, "subject_query": reference, "observed_at": stamp,
                                       "study_rev": old_study}))
        requests.append(["--subject", reference, "--role", role, "--selection-time", stamp,
                         "--cutoff-receipt", str(receipt)])

    def deliver(directory, selected):
        for request in selected:
            assert main(["report", "--repo", str(repo.path), "--out-dir", str(directory), "--mode", mode,
                         "--study-repo", str(study.path), "--study-rev", old_study, "--configs", config, *request]) == 0
        return {path.name: path.read_bytes() for path in directory.iterdir()}

    original = report_inputs._metadata
    revision = report_inputs.revision
    driver_directory = Path(report_inputs.__file__).parent
    meta = {"subject": "First driver checkout", "committed_at": "2024-01-02T00:00:00+00:00"}

    def metadata(path, revision):
        observed = original(repo.path if path == driver_directory else path, revision)
        return observed | meta if path == driver_directory else observed

    monkeypatch.setattr(report_inputs, "_metadata", metadata)
    monkeypatch.setattr(report_inputs, "revision", lambda path, ref:
                        partial if path == driver_directory else revision(path, ref))
    before = deliver(tmp_path / "before", requests)
    assert len(before) == 10
    assert all(b"field use" in md.lower() for name, md in before.items() if name == changed_field[:8] + ".md")
    later = repo.commit(2, **{"handler.go": HANDLER, CALL: call("/later"), SCHEMA: old_shape})
    git(repo.path, "tag", "future-release", later)
    for request in requests:
        git(repo.path, "update-ref", request[1], later)
    (repo.path / CALL).write_text(call("/dirty"))
    (repo.path / "untracked.go").write_text(HANDLER)
    study.commit(6, **{config: "[corrupt", "saved-bindings.json": json.dumps(
        [{"element_id": "field:resp:GET /fern:200#/leaf", "line": 2, "target": "/later"}])})
    meta.update(subject="Later driver checkout", committed_at="2024-11-20T00:00:00+00:00")
    after = deliver(tmp_path / "after", requests)
    assert before == after

    (repo.path / CALL).write_text(call("/later"))
    (repo.path / "untracked.go").unlink()
    assert git(repo.path, "status", "--porcelain") == ""
    control = repo.commit(7, **{"handler.go": HANDLER.replace("/fern", "/new")})
    stamp = git(repo.path, "show", "-s", "--format=%cI", control)
    receipt = tmp_path / "control.json"
    receipt.write_text(json.dumps({"subject": control, "observed_at": stamp, "study_rev": old_study}))
    files = deliver(tmp_path / "control", [["--subject", control, "--role", "producer",
                                           "--selection-time", stamp, "--cutoff-receipt", str(receipt)]])
    doc = json.loads(files[control[:8] + ".json"])
    row = impacts(doc)["op:GET /new"]
    assert (row["change_kinds"], row["before_presence"], row["after_presence"]) == (
        ["Operation added"], "absent", "present")


def test_driver_provenance_and_selected_invocation_are_fingerprinted(tmp_path, monkeypatch):
    repo, _ = start(tmp_path)
    head = repo.commit(1, **{"handler.go": HANDLER.replace("/fern", "/oak")})
    flags = ["--selection-time", "2024-01-02T00:00:00+00:00", "--source-url", "https://example.invalid/test"]
    _, first = report(repo, head, tmp_path, "producer", *flags)
    original = report_inputs._metadata
    revision = report_inputs.revision
    driver_directory = Path(report_inputs.__file__).parent

    def metadata(path, revision):
        observed = original(repo.path if path == driver_directory else path, revision)
        return observed | {"subject": "Changed driver commit"} if path == driver_directory else observed

    monkeypatch.setattr(report_inputs, "_metadata", metadata)
    monkeypatch.setattr(report_inputs, "revision", lambda path, ref:
                        head if path == driver_directory else revision(path, ref))
    changed, second = report(repo, head, tmp_path, "producer", *flags)
    assert first["report_id"] != second["report_id"]
    assert "Changed driver commit" in changed
    assert second["study_invocation"]["source_url"] == flags[-1]
    assert second["study_invocation"]["schema"] == SCHEMA
    for snap in second["inputs"].values():
        for source in snap["config_sources"]:
            assert source["captured_at"] == flags[1]


def test_empty_mixed_lead_requires_the_client_population(tmp_path):
    repo, _ = start(tmp_path, **{CALL: CALL_SOURCE.splitlines()[0] + "\nexport const value = 1;\n"})
    head = repo.commit(1, **{"other.go": "package flora\nfunc New() {}\n", CALL: "const [\n"})
    md, doc = report(repo, head, tmp_path)
    assert doc["change"]["subject_role"] == "both"
    assert not doc["impacts"] and not doc["call_changes"]
    assert doc["summary"]["operation_count"]["total"] == 0
    assert "no measured change" not in lead(md)
    assert CALL in lead(md) and "could not be parsed" in lead(md)


def test_same_line_occurrences_are_not_file_ordinals(tmp_path):
    extra = 'export const others = () => request.get("/fern") || request.get("/fern");\n'
    repo, _ = start(tmp_path, **{CALL: CALL_SOURCE + extra})
    md, doc = report(repo, repo.commit(1, **{"handler.go": "package flora\n"}), tmp_path)
    client = doc["inputs"][doc["change"]["before_input_id"]["client"]]
    assert [call["site"]["occurrence"] for call in client["calls"]] == [1, 1, 2]
    assert "(occurrence 1)" in md and "(occurrence 2)" in md and "(occurrence 3)" not in md


def test_explicit_schema_omission_has_one_limit(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema()})
    md, doc = report(repo, repo.commit(1), tmp_path, "producer", "--no-schema")
    assert len(doc["field_limits"]) == 1
    limits = md.split("## Limits", 1)[1].split("## Coverage", 1)[0]
    assert limits.count("omitted by request") == 1


def test_nondefault_historical_invocation_requires_matching_receipt(tmp_path):
    repo, _ = start(tmp_path)
    head = repo.commit(1)
    stamp = git(repo.path, "show", "-s", "--format=%cI", head)
    receipt = {"subject": head, "observed_at": stamp}
    request = report_inputs.ReportRequest(
        repo.path, head, "producer", CONFIGS, source_url="https://example.invalid/test",
        mode="historical_as_of", cutoff_receipt=receipt)
    with pytest.raises(ValueError, match="study invocation"):
        report_inputs.build_inputs(request)
    receipt["invocation"] = report_inputs.effective_invocation(request)
    frozen = report_inputs.build_inputs(request)
    assert frozen.after["service"]["source_url"] == request.source_url
