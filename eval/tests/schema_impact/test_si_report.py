import json
import re

import pytest

from schema_impact.cli import main
from test_si_replay import CALL, CALL_SOURCE, CLIENT, CONFIGS, HANDLER, Repo, SCHEMA, git


def schema(properties=None, *, required=(), status="200", media="application/json", body=True):
    response = {"schema": {"type": "object", "properties": properties or {}, "required": list(required)}} if body else {}
    return json.dumps({
        "swagger": "2.0", "produces": [media],
        "paths": {"/fern": {"get": {"responses": {status: response}}}},
    }, indent=2)


def start(tmp_path, **files):
    repo = Repo(tmp_path / "service")
    base = repo.commit(0, **{
        "handler.go": HANDLER, CLIENT: "export default {};", CALL: CALL_SOURCE,
    } | files)
    return repo, base


def report(repo, head, tmp_path, role="producer", *extra):
    output = tmp_path / "report"
    assert main(["report", "--repo", str(repo.path), "--subject", head, "--role", role,
                 "--configs", str(CONFIGS), "--out-dir", str(output), "--name", "subject", *extra]) == 0
    return (output / "subject.md").read_text(), json.loads((output / "subject.json").read_text())


def call(path="/fern", method="get"):
    return CALL_SOURCE.replace('"/fern"', json.dumps(path)).replace("request.get", f"request.{method}")


@pytest.mark.parametrize(("before", "after", "client", "phrases"), [
    (HANDLER, HANDLER.replace("// @Router /fern [get]\n", ""), CALL_SOURCE,
     ("Operation removed", "Client calls a missing route")),
    ("package flora\n", HANDLER, CALL_SOURCE,
     ("Operation added", "Existing client call gains a matching declaration")),
    ("package flora\n", HANDLER, call("/oak"),
     ("Operation added", "None of the 1 calls compared had this declaration among its closest matches")),
    (HANDLER, HANDLER.replace("[get]", "[post]"), CALL_SOURCE,
     ("Operation removed", "Operation added", "Client calls a missing route")),
    (HANDLER, HANDLER.replace("/fern", "/oak"), CALL_SOURCE,
     ("Operation removed", "Operation added", "Client calls a missing route")),
    ("package flora\n", HANDLER.replace("/fern", "/internal/fern"), call("/edge/fern"),
     ("Possible match under another prefix — both addresses end in /fern.", "1 requests whose method and address were read had no match")),
])
def test_operation_risks(tmp_path, before, after, client, phrases):
    repo, _ = start(tmp_path, **{"handler.go": before, CALL: client})
    head = repo.commit(1, **{"handler.go": after})
    md, doc = report(repo, head, tmp_path)
    for phrase in phrases:
        assert phrase in md
    if all(kind in phrases for kind in ("Operation added", "Operation removed")):
        assert len(doc["impacts"]) == 2
        assert "Different method" not in md


def test_mixed_checks_all_four_pairs(tmp_path):
    repo, _ = start(tmp_path)
    head = repo.commit(1, **{"handler.go": HANDLER.replace("[get]", "[post]"), CALL: call(method="post")})
    md, doc = report(repo, head, tmp_path)
    assert doc["change"]["subject_role"] == "both"
    assert len(doc["comparisons"]) == 4
    assert md.index("Service changes") < md.index("Client changes")
    for phrase in ("Before both edits", "Both edits included", "If the service ships first",
                   "If the client ships first",
                   "Corresponding call: Not checked — the scan recorded line numbers only."):
        assert phrase in md


@pytest.mark.parametrize(("addition", "phrase"), [(True, "Client call added"), (False, "Client call removed")])
def test_client_insertion_deletion(tmp_path, addition, phrase):
    content = CALL_SOURCE + '\nexport const oak = () => request.get("/oak");\n'
    before, after = (CALL_SOURCE, content) if addition else (content, CALL_SOURCE)
    repo, _ = start(tmp_path, **{CALL: before})
    head = repo.commit(1, **{CALL: after})
    md, doc = report(repo, head, tmp_path, "consumer")
    assert phrase in md
    assert "No GET match among" in md
    assert doc["comparisons"][0]["service_input_id"] == doc["comparisons"][1]["service_input_id"]


@pytest.mark.parametrize("role", ["producer", "consumer"])
def test_parse_error_does_not_certify_absence(tmp_path, role):
    repo, _ = start(tmp_path)
    file = "handler.go" if role == "producer" else CALL
    head = repo.commit(1, **{file: "({ invalid [\n"})
    md, doc = report(repo, head, tmp_path, role)
    assert "Not checked" in md and f"`{file}`" in md
    assert "Operation removed" not in md and "Client call removed" not in md
    if role == "producer":
        assert doc["summary"]["operation_count"]["total"] is None


def test_fields_both_directions_and_status_move(tmp_path):
    old = schema({"removed": {"type": "string"}, "typeEdit": {"type": "integer"},
                  "requiredEdit": {"type": "string"}, "optionalEdit": {"type": "string"}},
                 required=("optionalEdit",))
    new = schema({"added": {"type": "string"}, "typeEdit": {"type": "string"},
                  "requiredEdit": {"type": "string"}, "optionalEdit": {"type": "string"}},
                 required=("requiredEdit",), status="201")
    repo, _ = start(tmp_path, **{SCHEMA: old})
    head = repo.commit(1, **{SCHEMA: new})
    md, doc = report(repo, head, tmp_path)
    for phrase in ("Successful response status changed, 200 → 201", "Field declaration added",
                   "Field declaration removed", "Type changed", "whole number", "text",
                   "Now listed as required in its parent", "No longer listed as required in its parent",
                   "Field location: Not checked — only the operation was located",
                   "Client field use: Not checked — the call was matched by method and path."):
        assert phrase in md
    assert doc["summary"]["changed_field_count"]["total"] == 5


def test_bodyless_status_move_and_empty_report(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema(body=False)})
    head = repo.commit(1, **{SCHEMA: schema(body=False, status="204")})
    md, doc = report(repo, head, tmp_path)
    assert "Successful response status removed · 200" in md
    assert "Successful response status added · 204" in md
    assert doc["summary"]["changed_field_count"]["total"] == 0
    md, _ = report(repo, repo.commit(2), tmp_path)
    assert "No method or path was added, removed or changed in the checked declarations." in md
    assert "No difference in the measured field declarations." in md


def test_non_json_body_fields_unchecked(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema({"leaf": {"type": "integer"}}, media="application/xml")})
    head = repo.commit(1, **{SCHEMA: schema({"leaf": {"type": "string"}}, media="application/xml")})
    md, doc = report(repo, head, tmp_path)
    assert "JSON fields: Not checked — this body declares application/xml" in md
    assert doc["summary"]["changed_field_count"]["total"] is None


def test_long_report_and_live_label_contract(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema(body=False)})
    declarations = "".join(f"// @Router /fern{i} [get]\nfunc Fern{i}() {{}}\n" for i in range(7))
    head = repo.commit(1, **{"handler.go": "package flora\n" + declarations})
    md, _ = report(repo, head, tmp_path)
    anchor = re.search(r"Showing 5 of \d+ · \[See all operations\]\(#([^)]*)\)", md)
    assert anchor and f'id="{anchor[1]}"' in md
    assert "An operation is a method and path" in md and "Earlier match: Not checked" in md
    forbidden = ("within_14_days", "later", "present_at_pin", "present_at_head", "same_commit", "coordinated",
                 "fix_after_release", "dead_code", "real_mismatch", "unrelated_edit", "false_prediction",
                 "cannot_tell", "field_spelled", "field spelled", "bound", "unbound", "candidate", "grade",
                 "binding_current", "element_unknown")
    assert not re.search(r"\b(?:" + "|".join(forbidden) + r")\b", md, re.I)


@pytest.mark.parametrize("role", ["producer", "consumer"])
def test_later_evidence_cannot_change_live_bytes(tmp_path, monkeypatch, role):
    from schema_impact import replay

    repo, _ = start(tmp_path, **{SCHEMA: schema(body=False)})
    head = repo.commit(1, **{"handler.go": "package flora\n"} if role == "producer" else {CALL: call("/oak")})
    report(repo, head, tmp_path, role)
    paths = [tmp_path / "report" / f"subject.{ext}" for ext in ("md", "json")]
    frozen = [path.read_bytes() for path in paths]
    future = repo.commit(2, **{"handler.go": HANDLER, CALL: call("/future")})
    git(repo.path, "tag", "release", future)
    git(repo.path, "update-ref", "refs/heads/integration", future)
    (repo.path / CALL).write_text(call("/dirty"))
    (repo.path / "untracked.go").write_text(HANDLER.replace("/fern", "/dirty"))
    def forbidden(*args, **kwargs):
        pytest.fail("Live report accessed replay, line searches, later tags, judgments or controls")
    for name in ("usable", "replay_one", "ReplayResult", "follow"):
        monkeypatch.setattr(replay, name, forbidden)
    monkeypatch.setattr(replay.History, "detail", forbidden)
    report(repo, head, tmp_path, role)
    assert [path.read_bytes() for path in paths] == frozen


def route_contract(tmp_path):
    spec = tmp_path / "routes.json"
    spec.write_text(json.dumps({"callee": "register", "method_key": "method", "path_key": "path",
                                "handler_key": "handler", "validation_key": "validate"}))
    return ("--route-spec", str(spec), "--service-root", "service")


@pytest.mark.parametrize(("entry", "reason"), [
    ('{method: "*", path: "/fern"}', "the matcher compares exact methods only and this declaration lists every method"),
    ('{method: "GET", path: "/{id?}/fern"}', "this path segment may be omitted"),
    ('{method: "GET", path: "/fern"}, {method: "GET", path: "/fern"}', "two declarations have the same method and path"),
    ('{method: [, "GET"], path: "/fern"}', "route method is missing, not literal, or an empty or holed array"),
    ('{method: "GET", path: buildPath()}', "route path is missing or not literal"),
    ('...makeEntries()', "registration item is not an object declaration"),
    ('{method: "GET", path: "/fern", ...override}', "registration entry has a spread or computed key"),
])
def test_route_table_diagnostics_do_not_assert_missing_request(tmp_path, entry, reason):
    previous = 'server.register([{method: "GET", path: "/fern"}]);\n'
    repo, _ = start(tmp_path, **{"service/routes.js": previous})
    head = repo.commit(1, **{"service/routes.js": f"server.register([{entry}]);\n"})
    md, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    assert reason in md
    assert "Client calls a missing route" not in md
    assert doc["summary"]["known_unmatched_count"]["after"]["total"] is None


@pytest.mark.parametrize("kind", ["no-array", "renamed-callee", "renamed-and-edited", "syntax-error"])
def test_route_table_unread_region_is_not_a_removal(tmp_path, kind):
    before = 'server.register([{method: "GET", path: "/fern"}]);\n'
    after = {"no-array": "server.register(getEntries());", "renamed-callee": before.replace("register", "mount"),
             "renamed-and-edited": before.replace("register", "mount").replace("/fern", "/oak"),
             "syntax-error": "server.register([{method: "} [kind]
    repo, _ = start(tmp_path, **{"service/routes.js": before})
    head = repo.commit(1, **{"service/routes.js": after})
    md, doc = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    assert "Removal: Not checked" in md
    assert "Operation removed" not in md
    assert doc["impacts"][0]["after_presence"] == "unknown"
    assert doc["summary"]["known_unmatched_count"]["after"]["total"] is None


def test_missing_inputs_are_explicit(tmp_path):
    repo, _ = start(tmp_path)
    head = repo.commit(1, **{"handler.go": HANDLER.replace("/fern", "/oak")})
    md, doc = report(repo, head, tmp_path, "producer", "--root", "missing", "--configs", str(tmp_path / "missing"),
                     "--route-spec", str(tmp_path / "missing.json"))
    for text in ("client configuration", "route specification", "source root", "is missing",
                 "Changed fields: Not checked — no committed field schema", "Earlier match: Not checked",
                 "the binding inventory is incomplete"):
        assert text in md
    assert doc["summary"]["known_unmatched_count"]["before"]["total"] is None


def test_unknown_target_and_no_enclosing_function(tmp_path):
    repo, _ = start(tmp_path)
    after = CALL_SOURCE + '\nrequest.get(buildAddress());\n'
    head = repo.commit(1, **{CALL: after})
    md, doc = report(repo, head, tmp_path, "consumer")
    for phrase in ("Changed client calls with an unknown target", "The address is built while the program runs",
                   "Enclosing function: Not checked — the scan recorded no function name",
                   "Not checked — the function name is missing"):
        assert phrase in md
    assert doc["summary"]["unknown_target_count"]["after"]["observed"] == 1


def test_incomplete_after_schema_preserves_independent_service_evidence(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema({"leaf": {"type": "string"}})})
    head = repo.commit(1, **{SCHEMA: "{incomplete", "handler.go": HANDLER.replace("/fern", "/oak")})
    md, doc = report(repo, head, tmp_path)
    assert "Changed fields: Not checked" in md
    assert "Operation removed" in md and "Client calls a missing route" in md
    assert doc["summary"]["changed_field_count"]["total"] is None
    assert not any(row["fields"] for row in doc["impacts"])


def test_status_inventory_and_unresolved_definition(tmp_path):
    blob = json.loads(schema({"leaf": {"type": "string"}}))
    invalid = json.loads(schema())
    del invalid["paths"]["/fern"]["get"]["responses"]
    repo, _ = start(tmp_path, **{SCHEMA: json.dumps(blob)})
    head = repo.commit(1, **{SCHEMA: json.dumps(invalid)})
    md, _ = report(repo, head, tmp_path)
    assert "Status change: Not checked — after response status list is missing." in md
    assert "Successful response status removed" not in md
    blob["paths"]["/fern"]["get"]["responses"]["200"]["schema"] = {"$ref": "#/definitions/Missing"}
    head = repo.commit(2, **{SCHEMA: json.dumps(blob)})
    md, _ = report(repo, head, tmp_path)
    assert "stops at unreadable definition" in md


def test_caller_name_search_and_retained_declaration(tmp_path):
    before = 'const handler = () => 1;\nserver.register([{method: "GET", path: "/fern", handler}]);\n'
    after = before.replace("handler}]", "handler, validate: validateEntry}]")
    repo, _ = start(tmp_path, **{"service/routes.js": before, CALL: CALL_SOURCE + "\nfern();\n"})
    head = repo.commit(1, **{"service/routes.js": after})
    md, _ = report(repo, head, tmp_path, "producer", *route_contract(tmp_path))
    assert "Route declaration changed" in md
    assert "Validation or handler edits: Not checked — the two source bodies were not compared." in md
    assert "Possible caller found by name" in md and f"{CALL}:4" in md
    assert "Not checked — the search followed direct names only" in md


def test_evaluation_attachment_uses_separate_evidence_and_refuses_same_line_join(tmp_path):
    from schema_impact import replay
    from schema_impact.report_evaluation import join_evaluation, render_evaluation

    repeated = CALL_SOURCE.replace('request.get("/fern")', 'request.get("/fern") || request.get("/fern")')
    repo, base = start(tmp_path, **{CALL: repeated})
    head = repo.commit(1, **{"handler.go": "package flora\n"})
    md, doc = report(repo, head, tmp_path)
    future = repo.commit(2, **{CALL: call("/other")})
    result = replay.replay_one(repo.path, head, [replay.Change("op_removed", "op:GET /fern")],
                               replay.select_config(repo.path, base, CONFIGS), "ui/src", future)
    result.control.append(result.predicted[0])
    end = {"rev": future, "committed_at": "2024-01-03T00:00:00+00:00"}
    labels = {row.location_id: {"class": "unrelated_edit", "reason": "A study-only reason"}
              for row in result.predicted}
    companion = join_evaluation(doc, result, end, labels)
    history = render_evaluation(companion)
    assert len(companion["rows"]) == 2
    for phrase in ("Replay evaluation (uses later history)", "Not checked — more than one call shares this line",
                   "A tracked line changed later.", "Unrelated edit.", "A study-only reason",
                   "Release: Not checked"):
        assert phrase in history and phrase not in md
    replayed_md, replayed_doc = report(repo, head, tmp_path)
    assert (replayed_md, replayed_doc) == (md, doc)


def test_mixed_operation_table_lists_a_proved_shared_call_once(tmp_path):
    repo, _ = start(tmp_path, **{SCHEMA: schema({"leaf": {"type": "integer"}})})
    head = repo.commit(1, **{SCHEMA: schema({"leaf": {"type": "string"}}), CALL: CALL_SOURCE + "\nconst extra = 7;\n"})
    md, doc = report(repo, head, tmp_path)
    assert doc["change"]["subject_role"] == "both"
    assert len(doc["usages"]) == 1 and doc["usages"][0]["correspondence_evidence"]
    service_table = md.split("### Service changes")[1].split("### Client changes")[0]
    assert service_table.count("request.get") == 1
    assert "Corresponding call: Not checked" not in service_table


def test_review_first_puts_a_supported_action_before_limits(tmp_path):
    repo, _ = start(tmp_path, **{f"ui/src/extra{i}.tsx": "({ invalid [\n" for i in range(2)})
    head = repo.commit(1, **{"handler.go": "package flora\n"})
    md, _ = report(repo, head, tmp_path)
    review = md.split("## Review first\n\n")[1].split("\n\n", 1)[0]
    plain = re.sub(r"\[`([^`]+)`\]\([^)]*\)|`([^`]+)`", lambda m: m[1] or m[2], review)
    assert not plain.lower().startswith("not checked")
    assert len(re.findall(r"[.!?](?= |$)", plain)) <= 3
    assert "GET /fern" in plain and CALL in plain
    assert plain.lower().index("check whether") < plain.index("could not be checked")


def test_summary_and_shared_limit_references(tmp_path):
    repo, _ = start(tmp_path, **{f"ui/src/extra{i}.tsx": "({ invalid [\n" for i in range(4)})
    md, _ = report(repo, repo.commit(1), tmp_path)
    summary = md.split("## Summary\n\n")[1].split("\n\n", 1)[0]
    for label in ("Operations changed", "Fields changed", "Client calls affected", "Possible matches only",
                  "Requests with no match", "Calls with an unknown target"):
        assert f"| {label}" in summary
    ids = re.findall(r'id="(L\d+)"', md)
    markers = re.findall(r"not checked \[(L\d+)\]\(#(L\d+)\)", md)
    assert ids and len(ids) == len(set(ids))
    assert markers and all(label == target and target in ids for label, target in markers)
    assert md.count("client source could not be parsed") == 1
    assert all(f"ui/src/extra{i}.tsx" in md for i in range(4))


def test_long_preview_and_provenance(tmp_path):
    repo, _ = start(tmp_path)
    routes = "".join(f"// @Router /fern{i} [get]\nfunc Fern{i}() {{}}\n" for i in range(80))
    head = repo.commit(1, **{"handler.go": "package flora\n" + routes})
    md, _ = report(repo, head, tmp_path)
    main, appendix = md.split("## Appendix\n", 1)
    assert len(main.splitlines()) < 140
    assert len(re.findall(r"^#### `GET", main, re.M)) == 5
    anchors = re.findall(r'id="([^"]+)"', md)
    assert len(anchors) == len(set(anchors))
    references = re.findall(r"\]\(#([^)]*)\)|href=\"#([^\"]+)\"", md)
    assert {markdown or html for markdown, html in references} <= set(anchors)
    assert re.findall(r"^## (.+)", md, re.M)[-1] == "Provenance"
    assert "sha256:" not in md.split("## Provenance\n", 1)[0]
    assert "sha256:" in appendix.split("## Provenance\n", 1)[1]


def test_removed_operation_keeps_statuses_fields_and_client_action(tmp_path):
    before = schema({f"leaf{i}": {"type": "string"} for i in range(6)})
    after = json.loads(before)
    after["paths"] = {}
    client = CALL_SOURCE + "\n".join(
        f'export const fern{i} = () => request.get("/fern");' for i in range(5))
    repo, _ = start(tmp_path, **{SCHEMA: before, CALL: client})
    head = repo.commit(1, **{SCHEMA: json.dumps(after), "handler.go": "package flora\n"})
    md, _ = report(repo, head, tmp_path)
    operation = md.split("#### `GET /fern`\n\n")[1].split("## Who calls this code", 1)[0]
    assert operation.count("| Operation removed |") == 1
    assert "Operation fields changed" not in operation and "Successful response statuses changed" not in operation
    assert "<details><summary>Fields and response statuses</summary>" in operation
    assert "Successful response status removed · 200" in operation
    assert "Showing 3 of 6 · <a href=" in operation and "Showing 3 of 6 · [See all calls]" in operation
    assert operation.count("Check whether this client still needs the old declaration") == 1
