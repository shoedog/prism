import gzip
import json
import os
from datetime import timedelta

import pytest

from schema_impact.report_batch import main as batch
from test_si_replay import CALL, CALL_SOURCE, CLIENT, DATE, OP, Repo, git
from test_si_report import report, route_contract


ROUTES = "service/routes.js"
SERVICE_URL = "https://example.test/study/service"
CLIENT_URL = "https://example.test/study/client"


def route(method=None):
    entry = f'  {{method: "{method}", path: "/fern"}}\n' if method else ""
    return f"server.register([\n{entry}]);\n"


def config(tmp_path):
    source = tmp_path / "configs"
    source.mkdir()
    (source / "synthetic.toml").write_text('''[aliases]
"@/" = "src/"

[[client]]
file = "src/utils/request.ts"
export = "default"
methods = { get = "GET", post = "POST" }
''')
    return source


def settings(tmp_path, counterpart):
    return ("--counterpart-repo", str(counterpart.path), "--configs", str(config(tmp_path)),
            "--selection-time", "2024-02-01T00:00:00+00:00", *route_contract(tmp_path))


def snapshot(doc, ref):
    return doc["inputs"][ref["reference_input_id"]]


@pytest.mark.parametrize(("old", "new", "risks"), [
    ("GET", None, {"Client calls a missing route"}),
    (None, "GET", {"Existing client call gains a matching declaration"}),
    ("GET", "POST", {"Client calls a missing route"}),
])
def test_service_compares_one_fixed_client(tmp_path, old, new, risks):
    service, client = Repo(tmp_path / "service"), Repo(tmp_path / "client")
    # Even a matching path in the subject repo belongs to the service side only.
    base = service.commit(0, **{ROUTES: route(old), CALL: CALL_SOURCE})
    head = service.commit(4, **{ROUTES: route(new), CALL: CALL_SOURCE.replace("/fern", "/other")})
    client.commit(1, **{CLIENT: "export default {};", CALL: CALL_SOURCE.replace("/fern", "/early")})
    selected = client.commit(3, **{CALL: CALL_SOURCE})
    later = client.commit(4.00002, **{CALL: CALL_SOURCE.replace("/fern", "/later")})
    args = (*settings(tmp_path, client), "--source-url", SERVICE_URL, "--counterpart-source-url", CLIENT_URL)
    md, doc = report(service, head, tmp_path, "producer", *args)

    ref = doc["reference"]
    assert doc["mode"] == "version_comparison"
    assert doc["change"]["subject_role"] == "service"
    assert doc["change"]["before"] == base
    assert ref["selected_by"] == "commit_date"
    assert ref["duration_seconds"] == -86400
    fixed = snapshot(doc, ref)
    assert (fixed["rev"], fixed["repo"], fixed["tree"]) == (
        selected, "client", git(client.path, "rev-parse", f"{selected}^{{tree}}"))
    assert fixed["calls"][0]["mapped_path"] == "/fern"
    assert {p["purpose"] for p in doc["comparisons"]} == {"before", "after"}
    assert {p["client_input_id"] for p in doc["comparisons"]} == {fixed["input_id"]}
    assert {risk["kind"] for group in doc["impacts"] for risk in group["risks"]} == risks
    assert ("Operation removed" in md) == bool(old)
    assert ("Operation added" in md) == bool(new)
    if new == "POST":
        match = next(iter(doc["comparisons"][1]["matches"].values()))
        assert match["status"] == "no_match" and match["alternates"][0]["method"] == "POST"
    for line in ("# Service change checked against client code · Comparison of selected versions",
                 doc["reference"]["selection_reason"],
                 "2024-01-04T00:00:00+00:00", "2024-01-05T00:00:00+00:00",
                 "Checked revision is 1 day earlier than the change",
                 "Changed fields: Not checked — no committed field schema",
                 "**Client configuration** · `synthetic.toml`", "**Route specification** · `routes.json`",
                 "supplied by the study", f"{CLIENT_URL}/blob/{selected}/{CALL}#L2",
                 f"{SERVICE_URL}/blob/{base if old else head}/{ROUTES}#L2"):
        assert line in md
    assert later in md and ref["selection_history_tip"] == later
    assert later not in json.dumps(doc["inputs"])
    assert "/later" not in md and "/later" not in json.dumps(doc)
    assert "Both sides in one change" not in md

    files = [tmp_path / "report" / f"subject.{ext}" for ext in ("md", "json")]
    original = [file.read_bytes() for file in files]
    future = client.commit(7, **{CALL: CALL_SOURCE.replace("/fern", "/future")})
    (client.path / CALL).write_text(CALL_SOURCE.replace("/fern", "/dirty"))
    git(client.path, "tag", "later", "HEAD")
    revised, moved = report(service, head, tmp_path, "producer", *args)
    assert [file.read_bytes() for file in files] != original
    assert moved["reference"]["rev"] == selected
    assert moved["reference"]["selection_history_tip"] == future
    assert moved["comparisons"] == doc["comparisons"]
    assert "/future" not in revised and "/dirty" not in revised


def test_client_compares_one_fixed_service_and_can_pin_newer_service(tmp_path):
    service, client = Repo(tmp_path / "service"), Repo(tmp_path / "client")
    ref = service.commit(2, **{ROUTES: route("GET")})
    later = service.commit(5, **{ROUTES: route("POST")})
    base = client.commit(1, **{CLIENT: "export default {};", CALL: CALL_SOURCE})
    head = client.commit(4, **{CALL: CALL_SOURCE + 'export const other = () => request.post("/fern");\n'})
    args = (*settings(tmp_path, service), "--source-url", CLIENT_URL, "--counterpart-source-url", SERVICE_URL)
    md, doc = report(client, head, tmp_path, "consumer", *args)
    assert doc["change"]["subject_role"] == "client"
    assert doc["change"]["before"] == base
    assert f"**Before** · `{base}`" in md and "**Repository** · `client`" in md
    assert doc["reference"]["duration_seconds"] == -172800
    fixed = snapshot(doc, doc["reference"])
    assert fixed["rev"] == ref
    assert {pair["service_input_id"] for pair in doc["comparisons"]} == {fixed["input_id"]}
    assert doc["call_changes"][0]["risk"] == "Client calls a missing route"
    assert f"{CLIENT_URL}/blob/{head}/{CALL}#L3" in md
    assert fixed["operations"][OP]["sites"][0]["url"] == f"{SERVICE_URL}/blob/{ref}/{ROUTES}#L2"
    assert "Changed fields: Not checked — no committed field schema" in md

    md, pinned = report(client, head, tmp_path, "consumer", *args, "--counterpart-rev", later)
    assert "Revision supplied for this review" in md
    assert "This comparison uses code from after the change." in md
    assert "Checked revision is 1 day later than the change" in md
    assert pinned["reference"]["requested_rev"] == later
    assert pinned["call_changes"][0]["risk"] is None
    assert f"{SERVICE_URL}/blob/{later}/{ROUTES}#L2" in md


@pytest.mark.parametrize("date", [3, 4])
def test_date_selection_follows_first_parents_instead_of_sorting_dates(tmp_path, date):
    service, client = Repo(tmp_path / "service"), Repo(tmp_path / "client")
    service.commit(0, **{ROUTES: route("GET")})
    head = service.commit(4, **{ROUTES: route()})
    earlier_in_line = client.commit(4, **{CLIENT: "export default {};", CALL: CALL_SOURCE.replace("/fern", "/other")})
    selected = client.commit(date, **{CALL: CALL_SOURCE})
    later = client.commit(6)
    tree = git(client.path, "rev-parse", f"{later}^{{tree}}")

    def commit_tree(day, *parents):
        stamp = (DATE + timedelta(days=day)).isoformat()
        env = os.environ | {"GIT_AUTHOR_DATE": stamp, "GIT_COMMITTER_DATE": stamp}
        return git(client.path, "-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit-tree", tree,
                   *(option for parent in parents for option in ("-p", parent)), "-m", f"Graph day {day}", env=env)

    topic = commit_tree(4, earlier_in_line)
    merge = commit_tree(7, later, topic)
    git(client.path, "update-ref", "HEAD", merge)

    args = (*settings(tmp_path, client), "--configs", str(tmp_path / "configs" / "synthetic.toml"),
            "--counterpart-source-url", CLIENT_URL)
    md, doc = report(service, head, tmp_path, "producer", *args)
    assert doc["reference"]["rev"] == selected
    assert doc["reference"]["selection_history_tip"] == merge
    assert doc["reference"]["selection_history_tip_committed_at"] == (DATE + timedelta(days=7)).isoformat()
    assert "**Searched history tip**" in md and merge in md
    assert ("Both commits have the same time" in md) == (date == 4)
    assert f"{CLIENT_URL}/blob/{selected}/{CALL}#L2" in md
    assert f"`{ROUTES}:2`" in md and f"[`{ROUTES}:2`]" not in md


@pytest.mark.parametrize("role", ["producer", "consumer"])
def test_no_eligible_counterpart_retains_subject_evidence(tmp_path, role):
    service, client = Repo(tmp_path / "service"), Repo(tmp_path / "client")
    if role == "producer":
        service.commit(0, **{ROUTES: route("GET")})
        head = service.commit(1, **{ROUTES: route()})
        client.commit(2, **{CLIENT: "export default {};", CALL: CALL_SOURCE})
        subject, counterpart = service, client
    else:
        client.commit(0, **{CLIENT: "export default {};", CALL: CALL_SOURCE})
        head = client.commit(1, **{CALL: CALL_SOURCE + 'export const other = () => request.get("/oak");\n'})
        service.commit(2, **{ROUTES: route("GET")})
        subject, counterpart = client, service
    md, doc = report(subject, head, tmp_path, role, *settings(tmp_path, counterpart))
    assert "Not checked — no first-parent commit" in md
    assert doc["reference"]["availability_status"] == "unavailable"
    assert doc["reference"]["duration_seconds"] is None
    assert doc["summary"]["possible_only_usage_count"]["total"] is None
    assert all(pair["known_unmatched_count"]["total"] is None for pair in doc["request_counts"].values())
    assert "Client calls a missing route" not in md
    assert f"**After** · `{head}`" in md
    assert "https://" not in md
    if role == "producer":
        assert "Operation removed" in md and f"`{ROUTES}:2`" in md
        assert doc["summary"]["operation_count"]["total"] == 1
    else:
        assert "Client call added" in md and f"`{CALL}:3`" in md
        assert doc["comparisons"][1]["matches"][doc["call_changes"][0]["after_site_id"]]["status"] == "not_checked"
        assert "no committed field schema" not in md


def test_batch_companion_follows_explicit_client_pin_after_live_serialization(tmp_path):
    service, client = Repo(tmp_path / "service"), Repo(tmp_path / "client")
    service.commit(0, **{ROUTES: route("GET")})
    head = service.commit(2, **{ROUTES: route()})
    earlier = client.commit(1, **{CLIENT: "export default {};", CALL: CALL_SOURCE.replace("/fern", "/earlier")})
    selected = client.commit(3, **{CALL: CALL_SOURCE})
    later = client.commit(4, **{CALL: CALL_SOURCE.replace("/fern", "/later")})
    subjects = tmp_path / "subjects.tsv"
    subjects.write_text(f"01\tunused\t{head}\tunused\n")
    output = tmp_path / "batch"
    args = ["--repo", str(service.path), "--counterpart-repo", str(client.path), "--counterpart-rev", selected,
            "--subjects", str(subjects), "--out-dir", str(output), "--configs", str(config(tmp_path)),
            *route_contract(tmp_path)]
    assert batch([*args, "--pin", later]) == 0
    name = f"01-{head[:8]}"
    md = (output / f"{name}.md").read_text()
    with gzip.open(output / f"{name}.json.gz", "rt") as stream:
        live = json.load(stream)
    with gzip.open(output / f"{name}.evaluation.json.gz", "rt") as stream:
        companion = json.load(stream)
    assert "Replay evaluation" not in md and later not in md
    assert companion["live_report_id"] == live["report_id"]
    assert companion["observation_end"]["rev"] == later
    row, = companion["rows"]
    assert (row["tracked_client_revision"], row["element_id"], row["lag_days"]) == (selected, OP, 2.0)
    assert row["join_reason"] is None and row["site_id"] == live["impacts"][0]["client_site_ids"][0]
    assert row["touching_commit"]["commit"] == later and row["lag_commits"] is None
    assert "A tracked line changed within 14 days." in (output / f"{name}.evaluation.md").read_text()

    live_paths = [output / f"{name}.{ext}" for ext in ("md", "json.gz")]
    before = [path.read_bytes() for path in live_paths]
    assert batch([*args, "--pin", earlier]) == 0
    assert [path.read_bytes() for path in live_paths] == before
    assert "Not checked — the checked client revision is outside" in (output / f"{name}.evaluation.md").read_text()


def test_unreadable_pin_does_not_admit_unused_route_spec(tmp_path):
    service, client = Repo(tmp_path / "service"), Repo(tmp_path / "client")
    service.commit(0, **{ROUTES: route("GET")})
    client.commit(1, **{CLIENT: "export default {};", CALL: CALL_SOURCE})
    head = client.commit(2, **{CALL: CALL_SOURCE + 'export const other = () => request.get("/oak");\n'})
    args = settings(tmp_path, service)
    (tmp_path / "routes.json").write_text("This specification cannot be parsed")
    md, doc = report(client, head, tmp_path, "consumer", *args, "--counterpart-rev", "absent-revision")
    assert "Not checked — the supplied counterpart revision absent-revision could not be read" in md
    assert doc["reference"]["requested_rev"] == "absent-revision"
    assert doc["reference"]["rev"] is None
    assert "Client call added" in md and "Route specification" not in md
