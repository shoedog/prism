"""Exercise every registered requirement and simultaneous failures."""

from importlib import import_module
from itertools import combinations
import json
import os
from pathlib import Path
from types import SimpleNamespace

import pytest

from schema_impact import (
    report_analysis, report_inputs, report_layout, report_markdown, report_matches, report_proof, report_schema,
)
from schema_impact.report_layout import Limits, table
from schema_impact.report_markdown import render_markdown
from test_si_replay import CALL, CALL_SOURCE, CONFIGS, HANDLER, SCHEMA
from test_si_report import route_contract, schema, start
from test_si_report_fold1 import lead


def case_registry():
    """An archived baseline can use the same case IDs without adding the gate to its code."""
    if reference := os.environ.get("SCHEMA_IMPACT_TEST_CASES"):
        return {kind: [SimpleNamespace(**record) for record in records]
                for kind, records in json.loads(Path(reference).read_text()).items()}
    from schema_impact.report_claims import CLAIMS

    return CLAIMS


CLAIMS = case_registry()
report_summary = "schema_impact.report_summary"


def cells(kinds):
    return [
        pytest.param(kind, missing, id=f"{kind}-" + "+".join(
            f"{record.name}-{record.side or 'pair'}" for record in missing))
        for kind in kinds for count in (1, 2) for missing in combinations(CLAIMS[kind], count)
    ]


@pytest.fixture(scope="module")
def witnesses(tmp_path_factory):
    folder = tmp_path_factory.mktemp("claim-evidence")
    repo, _ = start(folder, **{SCHEMA: schema()})
    head = repo.commit(1)
    frozen = report_inputs.build_inputs(report_inputs.ReportRequest(repo.path, head, "producer", CONFIGS))
    return {"before": frozen.before["service"], "after": frozen.after["service"], None: frozen.after["service"]}


def label(record):
    return f"withheld {record.side or 'pair'} {record.name}"


@pytest.mark.parametrize("kind,missing", cells(CLAIMS))
def test_registry_withholds_each_record_and_every_pair(kind, missing, witnesses):
    from schema_impact.report_claims import decide, failed, wording

    records = {record: True for record in CLAIMS[kind]}
    claim = f"Confirmed evidence for {kind}"
    assert wording(decide(kind, records), claim) == claim
    for record in missing:
        witnesses_for_side = witnesses[record.side]
        records[record] = failed(kind, "handler.go", witnesses_for_side["rev"], label(record), side=record.side)
    proof = decide(kind, records)
    rendered = wording(proof, claim)
    limits = Limits()
    cell = table(["Result"], [(limits.add(proof),)])
    appendix = table(["Limit", "Reason"], limits.render(lambda _name, entries: "<br>".join(entries)))
    assert not proof["confirmed"] and "Not checked" in rendered
    assert claim not in rendered and claim not in cell
    for record in missing:
        assert label(record) in rendered and label(record) in appendix
    assert {row["record"] for row in proof["failures"]} == {record.name for record in missing}


# The extracted repository inputs feed the production decision functions and the real Markdown
# renderer here. The registry-only matrix above also covers records for first-cut detail cells.
_INTEGRATION = {
    "Operation added": ("operation", report_proof, "The service adds"),
    "Operation removed": ("operation", report_proof, "The service removes"),
    "Operation presence": ("operation", report_analysis, "| Operation added | Absent from the checked operations | Listed |"),
    "Declaration moved": ("move", report_proof, "moves from"),
    "Declaration removed from file": ("move", report_proof, "| Declaration removed from handler.go |"),
    "Declaration added to file": ("move", report_proof, "| Declaration added to other.go |"),
    "Route declaration changed": ("retained", report_proof, "| Route declaration changed |"),
    "Successful response status added": ("schema", report_schema, "Successful response status added · 201"),
    "Successful response status removed": ("schema", report_schema, "Successful response status removed · 200"),
    "Field declaration changed": ("schema", report_schema, "Type changed"),
    "Response bodies correspond": ("schema", report_schema, "Successful response status changed, 200 → 201"),
    "Client call added": ("client", report_analysis, "Client call added"),
    "Client call removed": ("client", report_analysis, "Client call removed"),
    "Client target changed": ("retarget", report_analysis, "The client changes the target of a call"),
    "Corresponding call": ("retarget", report_analysis, "The client changes the target of a call"),
    "No call at this site": ("client", report_markdown, "No call at this site"),
    "Operation total": ("empty", report_summary, "| Operations changed | 0 |"),
    "Field total": ("empty", report_summary, "| Fields changed | 0 |"),
    "Client change total": ("client_empty", report_summary, "| Client calls changed | 0 |"),
    "Empty service report": ("empty", report_summary, "No method or path was added"),
    "Empty client report": ("client_empty", report_summary, "No client calls were added"),
    "Empty mixed report": ("mixed_empty", report_summary, "The checked source shows no measured change"),
    "Schema coverage": ("empty", report_summary, "Compared type, format"),
    "No matching declaration": ("client", report_matches, "No GET match among declarations read"),
    "Matches declaration": ("operation", report_matches, "Matches GET /oak"),
    "Possible match": ("possible", report_matches, "Possible match — the value"),
    "Possible match under another prefix": ("prefix", report_matches, "Possible match under another prefix — both addresses"),
    "Closest-match population": ("operation", report_summary, "None of the 1 calls compared"),
    "Request count": ("operation", report_summary, "Before [1](#requests-before) · After [0](#requests-after)"),
    "Unknown target count": ("operation", report_summary, "| Calls with an unknown target | Before [0](#requests-before) · After [0](#requests-after)"),
    "Possible-only count": ("possible", report_summary, "| Possible matches only | 1 |"),
    "Existing client call gains a matching declaration": ("operation", report_analysis, "which gains a match"),
    "Client calls a missing route": ("removed", report_analysis, "which loses its match"),
    "Client target needs confirmation": ("possible", report_analysis, "whose target needs confirmation"),
    "Standing difference timing": ("prefix", report_analysis, "Already present"),
    "Standing schema difference": ("schema_standing", report_analysis, "The committed schema lists this operation"),
    "Caller search": ("operation", report_markdown, "No direct call found by this name in the searched files"),
    "Address mapping": ("operation", report_markdown, "entry 1 · path unchanged"),
    "Source location": ("operation", report_layout, "Before · Route annotation · `handler.go:3`"),
    "Source coverage": ("operation", report_layout, "Test files are excluded.\n\n"),
    "Enclosing function": ("operation", report_markdown, " · `fern`"),
}
assert set(CLAIMS) == _INTEGRATION.keys()


@pytest.fixture(scope="module")
def comparisons(tmp_path_factory):
    scenarios = {
        "operation": ({CALL: CALL_SOURCE.replace('"/fern"', '"/oak"')},
                      {"handler.go": HANDLER.replace("/fern", "/oak")}, "producer"),
        "schema": ({SCHEMA: schema({"leaf": {"type": "string"}})},
                   {SCHEMA: schema({"leaf": {"type": "integer"}}, status="201")}, "producer"),
        "client": ({CALL: CALL_SOURCE + 'export const old = () => request.get("/old");\nconst keep = 1;\n'},
                   {CALL: CALL_SOURCE + 'const keep = 1;\nexport const added = () => request.get("/new");\n'}, "consumer"),
        "empty": ({SCHEMA: schema()}, {}, "producer"),
        "client_empty": ({SCHEMA: schema()}, {}, "consumer"),
        "mixed_empty": ({SCHEMA: schema()}, {"handler.go": HANDLER + "\n", CALL: CALL_SOURCE + "\n"}, "producer"),
        "move": ({}, {"handler.go": None, "other.go": HANDLER}, "producer"),
        "removed": ({}, {"handler.go": "package flora\n"}, "producer"),
        "retained": ({"service/routes.js": "register([{method:'GET',path:'/fern'}]);"},
                     {"service/routes.js": "register([{method:'GET',path:'/fern',handler:load}]);"}, "producer"),
        "possible": ({"handler.go": "package flora\n",
                      CALL: CALL_SOURCE.replace('"/fern"', '`/edge/${name}/fern`')},
                     {"handler.go": HANDLER.replace("/fern", "/edge/literal/fern")}, "producer"),
        "prefix": ({"handler.go": "package flora\n", CALL: CALL_SOURCE.replace("/fern", "/edge/fern")},
                   {"handler.go": HANDLER.replace("/fern", "/internal/fern")}, "producer"),
        "schema_standing": ({SCHEMA: schema().replace("/fern", "/unlisted")}, {}, "producer"),
        "retarget": ({}, {"ui/src/version.ts": "export const version = 2;"}, "consumer"),
    }
    reports = {}
    for name, (before, after, role) in scenarios.items():
        folder = tmp_path_factory.mktemp(name)
        repo, base = start(folder, **before)
        head = repo.commit(1, **after)
        settings = {}
        if name == "retained":
            flags = route_contract(folder)
            settings = {"route_spec": Path(flags[1]), "service_root": flags[3], "schema": None}
        configs = CONFIGS
        with pytest.MonkeyPatch.context() as patch:
            if name == "retarget":
                configs = folder / "configs"
                configs.mkdir()
                config = (CONFIGS / "apache-answer.toml").read_text()
                previous = configs / "before.toml"
                previous.write_text(config)
                following = configs / "after.toml"
                following.write_text(config.replace('response_root = "data"', 'add_prefix = "/api"\nresponse_root = "data"'))
                patch.setattr(report_inputs, "select_config", lambda _repo, rev, _configs:
                              previous if rev == base else following)
            frozen = report_inputs.build_inputs(report_inputs.ReportRequest(repo.path, head, role, configs, **settings))
        reports[name] = frozen, render_markdown(report_analysis.build_document(frozen))
    return reports


@pytest.mark.parametrize("kind,missing", cells(_INTEGRATION))
def test_registered_requirements_drive_live_markdown(kind, missing, comparisons, monkeypatch):
    from schema_impact.report_claims import failed

    scenario, module, claim = _INTEGRATION[kind]
    if isinstance(module, str):
        module = import_module(module)
    # The same repository can show a different operation first. Check the removed operation
    # in its confirmed table row, which is present independently of lead ordering.
    if kind == "Operation removed":
        claim = "| Operation removed |"
    frozen, positive = comparisons[scenario]
    assert claim in positive
    actual = module.decide

    def withhold(candidate, records):
        if candidate == kind:
            records = records | {record: failed(kind, None, None, label(record), side=record.side)
                                 for record in missing}
        return actual(candidate, records)

    monkeypatch.setattr(module, "decide", withhold)
    document = report_analysis.build_document(frozen)
    negative = render_markdown(document)
    if kind in {"Client call added", "Client call removed"}:
        assert kind not in {row["kind"] for row in document["call_changes"]}
        assert (claim + " ·") not in negative and (claim + " |") not in negative
    else:
        assert claim not in negative
    for record in missing:
        assert label(record) in negative
    if kind in {"Empty service report", "Empty client report", "Empty mixed report"}:
        for record in missing:
            assert label(record) in lead(negative)
