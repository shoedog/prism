"""Pure tests for the exhaustive promoted-selector resolver-delta audit."""

import importlib.util
import json
import sys
from pathlib import Path


_EVAL = Path(__file__).resolve().parents[1]
_TOOL = _EVAL / "tools" / "promoted_delta_audit.py"
_spec = importlib.util.spec_from_file_location("promoted_delta_audit", _TOOL)
audit = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(audit)

_FIXTURES = Path(__file__).parent / "fixtures"


def _delta():
    control = audit.load_sites(_FIXTURES / "promoted_delta_control.jsonl")
    candidate = audit.load_sites(_FIXTURES / "promoted_delta_candidate.jsonl")
    return audit.select_changed_sites(control, candidate)


def test_selects_entire_exact_embedded_promotion_delta_only():
    delta = _delta()
    assert [site[0][0] for site in delta] == ["app/a.go", "app/d.go"]
    assert sum(len(site[2]) for site in delta) == 3
    assert all(
        target["kind"] == "embedded_promotion" and target["confidence"] == "exact"
        for _, _, targets in delta
        for target in targets
    )


def test_gate_pass_requires_every_site_and_every_target_to_hit():
    summary = audit.summarize(
        new_sites=2,
        sampled=2,
        target_verdicts=["hit", "hit", "hit"],
    )
    assert summary["verdict"] == "PASS"
    assert summary["sampled"] == summary["new_sites"] == 2
    assert summary["hits"] == summary["targets"] == 3


def test_gate_fails_when_the_entire_delta_was_not_processed():
    summary = audit.summarize(
        new_sites=2,
        sampled=1,
        target_verdicts=["hit"],
    )
    assert summary["verdict"] == "FAIL"


def test_gate_fails_on_unknown_definition():
    summary = audit.summarize(
        new_sites=1,
        sampled=1,
        target_verdicts=["unknown:timeout"],
    )
    assert summary["unknown"] == 1
    assert summary["verdict"] == "FAIL"


def test_gate_fails_when_any_individual_target_misses():
    summary = audit.summarize(
        new_sites=1,
        sampled=1,
        target_verdicts=["hit", "MISS"],
    )
    assert summary["misses"] == 1
    assert summary["verdict"] == "FAIL"


def test_zero_site_delta_is_no_data_pass():
    summary = audit.summarize(new_sites=0, sampled=0, target_verdicts=[])
    assert summary["verdict"] == "NO-DATA"
    assert audit.exit_code(summary) == 0


def test_definition_must_land_inside_each_prism_target_span():
    target = {
        "function_id": {
            "file": "q/a.go",
            "name": "M",
            "start_line": 5,
            "end_line": 7,
        }
    }
    assert audit.definition_in_target(target, {"kind": "concrete", "file": "q/a.go", "line": 4})
    assert audit.definition_in_target(target, {"kind": "concrete", "file": "q/a.go", "line": 6})
    assert not audit.definition_in_target(
        target, {"kind": "concrete", "file": "q/a.go", "line": 7}
    )
    assert not audit.definition_in_target(
        target, {"kind": "concrete", "file": "other/a.go", "line": 4}
    )


def test_empty_callee_name_is_unknown_instead_of_querying_span_end(tmp_path):
    source = "package app\nfunc f() {}\n"
    (tmp_path / "use.go").write_text(source)
    assert audit.token_position(tmp_path, "use.go", 0, len(source), "") is None


def test_audit_checks_one_gopls_definition_against_every_target(tmp_path):
    source = "package app\nfunc f(s S) { s.M() }\n"
    (tmp_path / "app").mkdir()
    (tmp_path / "app" / "use.go").write_text(source)
    start = source.index("s.M")
    key = ("app/use.go", start, start + len("s.M()"), "s.M")
    targets = [
        {
            "function_id": {
                "file": "q/a.go",
                "name": "M",
                "start_line": 5,
                "end_line": 7,
            },
            "kind": "embedded_promotion",
            "confidence": "exact",
        },
        {
            "function_id": {
                "file": "q/b.go",
                "name": "M",
                "start_line": 5,
                "end_line": 7,
            },
            "kind": "embedded_promotion",
            "confidence": "exact",
        },
    ]

    class FakeOracle:
        def method_decl(self, relative_file, line, character):
            assert relative_file == "app/use.go"
            assert (line, character) == (1, 16)
            return {"kind": "concrete", "file": "q/a.go", "line": 4}

    records, verdicts = audit.audit_sites([(key, {}, targets)], tmp_path, FakeOracle())
    assert verdicts == ["hit", "MISS"]
    assert [result["verdict"] for result in records[0]["target_results"]] == [
        "hit",
        "MISS",
    ]


def test_zero_delta_cli_never_starts_gopls(tmp_path, monkeypatch):
    class ForbiddenOracle:
        def __init__(self, *_args, **_kwargs):
            raise AssertionError("zero-site delta must not construct gopls")

    fixture = _FIXTURES / "promoted_delta_control.jsonl"
    output = tmp_path / "audit.json"
    monkeypatch.setattr(audit.do, "GoplsSatisfiers", ForbiddenOracle)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "promoted_delta_audit.py",
            "--control-sites",
            str(fixture),
            "--candidate-sites",
            str(fixture),
            "--repo",
            str(tmp_path),
            "--out",
            str(output),
        ],
    )
    assert audit.main() == 0
    assert json.loads(output.read_text())["summary"]["verdict"] == "NO-DATA"
