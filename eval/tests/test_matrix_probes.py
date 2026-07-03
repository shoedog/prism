"""Matrix fixture-type selector tests (P6bc): `[case] probe = "callers" |
"taint" | "module_deps"`. Mirrors test_matrix.py's load_case/run_matrix
patterns for the two new probe types, plus schema-validation rejections."""
from pathlib import Path

import pytest

from tier_a.matrix import load_case, run_matrix

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"


def _write(tmp_path, lang, capability, body):
    case_dir = tmp_path / "fixtures" / lang / capability
    case_dir.mkdir(parents=True)
    (case_dir / "expected.toml").write_text(body)
    return case_dir / "expected.toml"


def test_load_case_default_probe_is_callers():
    # ALL 73 pre-existing fixtures omit [case] probe entirely -- must still
    # load as "callers" and run byte-identically.
    case = load_case(FIXTURES / "python" / "module_fn" / "expected.toml")
    assert case.probe == "callers"
    assert case.seed_symbol == "helper"


def test_load_case_taint_probe_parses_fields(tmp_path):
    path = _write(tmp_path, "python", "taint_case", """
[case]
language = "python"
capability = "taint_case"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
sinks = ["app.py:4"]
[expect]
reachability = "Reached"
warning_kinds_present = ["Cleansed"]
sanitizers_present = true
""")
    case = load_case(path)
    assert case.probe == "taint"
    assert case.taint_sources == ["app.py:2"]
    assert case.taint_sinks == ["app.py:4"]
    assert case.expect_reachability == "Reached"
    assert case.expect_warning_kinds_present == ["Cleansed"]
    assert case.expect_sanitizers_present is True


def test_load_case_taint_probe_defaults_sinks_and_optional_expect(tmp_path):
    path = _write(tmp_path, "python", "taint_frontier", """
[case]
language = "python"
capability = "taint_frontier"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
""")
    case = load_case(path)
    assert case.taint_sinks == []
    assert case.expect_warning_kinds_present == []
    assert case.expect_sanitizers_present is None


def test_load_case_taint_probe_rejects_seed_section(tmp_path):
    path = _write(tmp_path, "python", "bad_taint_seed", """
[case]
language = "python"
capability = "bad_taint_seed"
status = "pass"
probe = "taint"
[seed]
symbol = "x"
file = "app.py"
line = 1
[taint]
sources = ["app.py:2"]
[expect]
reachability = "Reached"
""")
    with pytest.raises(ValueError, match=r"must not define \[seed\]"):
        load_case(path)


def test_load_case_taint_probe_requires_taint_section(tmp_path):
    path = _write(tmp_path, "python", "bad_taint_missing", """
[case]
language = "python"
capability = "bad_taint_missing"
status = "pass"
probe = "taint"
[expect]
reachability = "Reached"
""")
    with pytest.raises(ValueError, match=r"requires \[taint\]"):
        load_case(path)


def test_load_case_callers_probe_rejects_taint_section(tmp_path):
    path = _write(tmp_path, "python", "bad_callers_taint", """
[case]
language = "python"
capability = "bad_callers_taint"
status = "pass"
[seed]
symbol = "x"
file = "app.py"
line = 1
[taint]
sources = ["app.py:2"]
[[expect.callers]]
file = "app.py"
line = 2
[expect]
exact = true
""")
    with pytest.raises(ValueError, match=r"must not define \[taint\] or \[module\]"):
        load_case(path)


def test_load_case_callers_probe_rejects_module_section(tmp_path):
    path = _write(tmp_path, "python", "bad_callers_module", """
[case]
language = "python"
capability = "bad_callers_module"
status = "pass"
[seed]
symbol = "x"
file = "app.py"
line = 1
[module]
file = "app.py"
[[expect.callers]]
file = "app.py"
line = 2
[expect]
exact = true
""")
    with pytest.raises(ValueError, match=r"must not define \[taint\] or \[module\]"):
        load_case(path)


def test_load_case_module_probe_parses_fields(tmp_path):
    path = _write(tmp_path, "python", "module_case", """
[case]
language = "python"
capability = "module_case"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[[expect.module_edges]]
to = "b.py"
[expect]
forbid_to = ["c.py"]
""")
    case = load_case(path)
    assert case.probe == "module_deps"
    assert case.module_file == "a.py"
    assert case.expect_module_to == {"b.py"}
    assert case.expect_module_forbid_to == {"c.py"}
    assert case.module_exact is False


def test_load_case_module_probe_exact_flag(tmp_path):
    path = _write(tmp_path, "python", "module_case_exact", """
[case]
language = "python"
capability = "module_case_exact"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[[expect.module_edges]]
to = "b.py"
[expect]
exact = true
""")
    case = load_case(path)
    assert case.module_exact is True


def test_load_case_module_probe_rejects_expect_callers(tmp_path):
    path = _write(tmp_path, "python", "bad_module_callers", """
[case]
language = "python"
capability = "bad_module_callers"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[[expect.callers]]
file = "b.py"
line = 1
[expect]
exact = true
""")
    with pytest.raises(ValueError, match=r"must not define \[seed\] or expect.callers"):
        load_case(path)


def test_load_case_module_probe_requires_module_section(tmp_path):
    path = _write(tmp_path, "python", "bad_module_missing", """
[case]
language = "python"
capability = "bad_module_missing"
status = "pass"
probe = "module_deps"
[expect]
exact = true
""")
    with pytest.raises(ValueError, match=r"requires \[module\]"):
        load_case(path)


def test_load_case_unknown_probe_rejects(tmp_path):
    path = _write(tmp_path, "python", "bogus_probe", """
[case]
language = "python"
capability = "bogus_probe"
status = "pass"
probe = "bogus"
[seed]
symbol = "x"
file = "app.py"
line = 1
[expect]
callers = []
""")
    with pytest.raises(ValueError, match=r"unknown \[case\] probe"):
        load_case(path)


class FakeTaintSut:
    def __init__(self, evidence):
        self.evidence = evidence

    def taint_reaches(self, root, sources, sinks):
        return self.evidence


class FakeModuleSut:
    def __init__(self, evidence):
        self.evidence = evidence

    def module_deps(self, root, file):
        return self.evidence


def test_run_matrix_dispatches_taint_probe_ok(tmp_path):
    _write(tmp_path, "python", "taint_ok", """
[case]
language = "python"
capability = "taint_ok"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
sinks = ["app.py:4"]
[expect]
reachability = "Reached"
""")
    sut = FakeTaintSut({"reasoning": {"reachability": "Reached", "per_sink": []}, "warnings": []})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "ok"
    assert results[0].probe == "taint"
    assert results[0].got == "Reached|warnings=none|sanitizers=false"


def test_run_matrix_dispatches_taint_probe_regression(tmp_path):
    _write(tmp_path, "python", "taint_bad", """
[case]
language = "python"
capability = "taint_bad"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
sinks = ["app.py:4"]
[expect]
reachability = "Reached"
""")
    sut = FakeTaintSut({"reasoning": {"reachability": "NotReached", "per_sink": []}, "warnings": []})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "regression"
    assert "NotReached" in results[0].got


def test_run_matrix_taint_probe_checks_warnings_and_sanitizers(tmp_path):
    _write(tmp_path, "python", "taint_sanitized", """
[case]
language = "python"
capability = "taint_sanitized"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
sinks = ["app.py:4"]
[expect]
reachability = "Reached"
warning_kinds_present = ["Cleansed"]
sanitizers_present = true
""")
    evidence = {
        "reasoning": {
            "reachability": "Reached",
            "per_sink": [
                {"sources": [{"sanitizers_present_in_source_fn": ["xss"]}]},
            ],
        },
        "warnings": [{"kind": {"Reasoning": {"Cleansed": {"source_function": "f"}}}}],
    }
    ok = run_matrix(tmp_path / "fixtures", FakeTaintSut(evidence), languages=["python"])
    assert ok[0].outcome == "ok"

    evidence_no_warning = {
        "reasoning": {
            "reachability": "Reached",
            "per_sink": [{"sources": [{"sanitizers_present_in_source_fn": []}]}],
        },
        "warnings": [],
    }
    wrong = run_matrix(tmp_path / "fixtures", FakeTaintSut(evidence_no_warning), languages=["python"])
    assert wrong[0].outcome == "regression"


def test_run_matrix_taint_probe_frontier_none_reachability(tmp_path):
    _write(tmp_path, "python", "taint_frontier_case", """
[case]
language = "python"
capability = "taint_frontier_case"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
""")
    sut = FakeTaintSut({"reasoning": {"reachability": None, "per_sink": []}, "warnings": []})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "ok"


def test_run_matrix_dispatches_module_probe_subset_default(tmp_path):
    _write(tmp_path, "python", "module_ok", """
[case]
language = "python"
capability = "module_ok"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[[expect.module_edges]]
to = "b.py"
""")
    sut = FakeModuleSut({"items": [{"location": {"file": "b.py"}}, {"location": {"file": "a.py"}}]})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "ok"
    assert results[0].probe == "module_deps"


def test_run_matrix_module_probe_exact_flag_rejects_extra_edges(tmp_path):
    _write(tmp_path, "python", "module_exact", """
[case]
language = "python"
capability = "module_exact"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[[expect.module_edges]]
to = "b.py"
[expect]
exact = true
""")
    sut = FakeModuleSut({"items": [{"location": {"file": "b.py"}}, {"location": {"file": "a.py"}}]})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "regression"  # extra a.py breaks exact set equality


def test_run_matrix_module_probe_forbid_to(tmp_path):
    _write(tmp_path, "python", "module_forbid", """
[case]
language = "python"
capability = "module_forbid"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[[expect.module_edges]]
to = "b.py"
[expect]
forbid_to = ["c.py"]
""")
    ok = run_matrix(tmp_path / "fixtures", FakeModuleSut({"items": [{"location": {"file": "b.py"}}]}),
                    languages=["python"])
    assert ok[0].outcome == "ok"
    bad = run_matrix(
        tmp_path / "fixtures",
        FakeModuleSut({"items": [{"location": {"file": "b.py"}}, {"location": {"file": "c.py"}}]}),
        languages=["python"],
    )
    assert bad[0].outcome == "regression"
