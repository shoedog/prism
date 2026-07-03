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
    assert case.frontier_count_min is None


def test_load_case_taint_probe_rejects_boolean_frontier_count_min(tmp_path):
    """Item 1(b) (codex MAJOR): `bool` is a subclass of `int` in Python, so a
    fixture author's typo `frontier_count_min = true` would otherwise load
    silently and then (per 1(a)) coerce into a meaningless comparison. Reject
    it at load time as a schema error, same tier as the "asserts nothing"
    checks below."""
    path = _write(tmp_path, "python", "bad_taint_frontier_bool", """
[case]
language = "python"
capability = "bad_taint_frontier_bool"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
frontier_count_min = true
""")
    with pytest.raises(ValueError, match=r"frontier_count_min must be an int"):
        load_case(path)


def test_load_case_taint_probe_rejects_zero_frontier_count_min(tmp_path):
    """Item 1(b): a floor of 0 asserts nothing (`frontier_count >= 0` is
    always true for a present int field) -- consistent with the harness's
    existing >=1-assertion doctrine, reject at load time."""
    path = _write(tmp_path, "python", "bad_taint_frontier_zero", """
[case]
language = "python"
capability = "bad_taint_frontier_zero"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
frontier_count_min = 0
""")
    with pytest.raises(ValueError, match=r"frontier_count_min must be >= 1"):
        load_case(path)


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


# F2 (codex BLOCKER 2 + opus m2 + controller finding): schema must reject
# assert-nothing and typo'd fixtures instead of silently loading them
# unconstrained (or with an empty/no-op expectation).


def test_load_case_taint_probe_rejects_module_section(tmp_path):
    """Both-directions coverage: a taint probe must not define [module] either
    (the pre-existing check only forbade [seed]/expect.callers)."""
    path = _write(tmp_path, "python", "bad_taint_module", """
[case]
language = "python"
capability = "bad_taint_module"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[module]
file = "app.py"
[expect]
reachability = "Reached"
""")
    with pytest.raises(ValueError, match=r"must not define \[module\]"):
        load_case(path)


def test_load_case_module_probe_rejects_taint_section(tmp_path):
    """Both-directions coverage: a module_deps probe must not define [taint]
    either (the pre-existing check only forbade [seed]/expect.callers)."""
    path = _write(tmp_path, "python", "bad_module_taint", """
[case]
language = "python"
capability = "bad_module_taint"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[taint]
sources = ["app.py:2"]
[[expect.module_edges]]
to = "b.py"
""")
    with pytest.raises(ValueError, match=r"must not define \[taint\]"):
        load_case(path)


def test_load_case_rejects_unknown_top_level_section(tmp_path):
    path = _write(tmp_path, "python", "bogus_section", """
[case]
language = "python"
capability = "bogus_section"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[bogus]
whatever = 1
[expect]
reachability = "Reached"
""")
    with pytest.raises(ValueError, match=r"unknown top-level section"):
        load_case(path)


def test_load_case_taint_probe_rejects_unknown_expect_key(tmp_path):
    """Repro of the reported typo: `reachabilty` (missing an 'i') silently
    dropped into an ignored key, leaving the fixture fully unconstrained."""
    path = _write(tmp_path, "python", "typo_reachabilty", """
[case]
language = "python"
capability = "typo_reachabilty"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachabilty = "Reached"
""")
    with pytest.raises(ValueError, match=r"unknown key.*reachabilty"):
        load_case(path)


def test_load_case_module_probe_rejects_unknown_expect_key(tmp_path):
    """Repro of the reported typo: `module_edge` (missing the trailing 's')
    left the module expectation empty, and an empty-subset check passes
    against zero items."""
    path = _write(tmp_path, "python", "typo_module_edge", """
[case]
language = "python"
capability = "typo_module_edge"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[expect]
module_edge = "b.py"
""")
    with pytest.raises(ValueError, match=r"unknown key.*module_edge"):
        load_case(path)


def test_load_case_taint_probe_rejects_unknown_taint_key(tmp_path):
    path = _write(tmp_path, "python", "typo_sources", """
[case]
language = "python"
capability = "typo_sources"
status = "pass"
probe = "taint"
[taint]
soruces = ["app.py:2"]
[expect]
reachability = "Reached"
""")
    with pytest.raises(ValueError, match=r"unknown key.*soruces"):
        load_case(path)


def test_load_case_module_probe_rejects_unknown_module_key(tmp_path):
    path = _write(tmp_path, "python", "typo_file", """
[case]
language = "python"
capability = "typo_file"
status = "pass"
probe = "module_deps"
[module]
filee = "a.py"
[[expect.module_edges]]
to = "b.py"
""")
    with pytest.raises(ValueError, match=r"unknown key.*filee"):
        load_case(path)


def test_load_case_taint_probe_requires_at_least_one_assertion(tmp_path):
    path = _write(tmp_path, "python", "assert_nothing_taint", """
[case]
language = "python"
capability = "assert_nothing_taint"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
""")
    with pytest.raises(ValueError, match=r"asserts nothing"):
        load_case(path)


def test_load_case_taint_probe_requires_at_least_one_assertion_missing_expect(tmp_path):
    """No `[expect]` table at all is the same as an empty one."""
    path = _write(tmp_path, "python", "assert_nothing_taint_no_expect", """
[case]
language = "python"
capability = "assert_nothing_taint_no_expect"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
""")
    with pytest.raises(ValueError, match=r"asserts nothing"):
        load_case(path)


def test_load_case_module_probe_requires_at_least_one_assertion(tmp_path):
    path = _write(tmp_path, "python", "assert_nothing_module", """
[case]
language = "python"
capability = "assert_nothing_module"
status = "pass"
probe = "module_deps"
[module]
file = "a.py"
[expect]
""")
    with pytest.raises(ValueError, match=r"asserts nothing"):
        load_case(path)


def test_load_case_taint_probe_rejects_bad_reachability_value(tmp_path):
    """Controller adjudication (e): validate `expect.reachability` at load
    time against the known sentinel set -- kills the typo'd-sentinel class
    (e.g. "Reachd", "none" lowercase) instead of letting it silently fail to
    match at run time with no clear diagnosis."""
    path = _write(tmp_path, "python", "bad_reachability_value", """
[case]
language = "python"
capability = "bad_reachability_value"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "Reachd"
""")
    with pytest.raises(ValueError, match=r"reachability.*Reachd"):
        load_case(path)


def test_load_case_taint_probe_rejects_empty_callers_list(tmp_path):
    """(d): key PRESENCE, not `bool(...)`, must gate the expect.callers forbid
    check -- an empty `callers = []` on a taint probe is still `expect.callers`
    defined where it must not be."""
    path = _write(tmp_path, "python", "empty_callers_on_taint", """
[case]
language = "python"
capability = "empty_callers_on_taint"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "Reached"
callers = []
""")
    with pytest.raises(ValueError, match=r"must not define \[seed\] or expect.callers"):
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
    sut = FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": "Reached", "per_sink": []},
        "warnings": [],
    })
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
    sut = FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": "NotReached", "per_sink": []},
        "warnings": [],
    })
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
        "query": "taint_reaches",
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
        "query": "taint_reaches",
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
    sut = FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": None, "per_sink": []},
        "warnings": [],
    })
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "ok"


def _frontier_case_toml() -> str:
    return """
[case]
language = "python"
capability = "taint_frontier_case"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
frontier_count_min = 1
"""


def test_run_matrix_taint_probe_missing_reasoning_key_is_regression(tmp_path):
    """F1 (codex BLOCKER 1): `reasoning` entirely absent from the wire JSON --
    e.g. a bug that drops the field, or a query that never ran -- must be a
    hard mismatch (regression on a `pass` fixture), never null-equals-None.
    This is the literal reported bug: `{"items":[],"warnings":[]}` with no
    `reasoning` key was reported `ok` against the frontier fixture's
    `reachability = "None"` sentinel."""
    _write(tmp_path, "python", "taint_frontier_case", _frontier_case_toml())
    sut = FakeTaintSut({"query": "taint_reaches", "items": [], "warnings": []})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "regression"
    # Item 2 (codex MINOR): the missing-evidence branch must format the
    # frontier floor the same way as the normal mismatch branch (">=N"), not
    # the bare int -- so a triage reader sees "frontier=>=1" consistently.
    assert "frontier=>=1" in results[0].expected


def test_run_matrix_taint_probe_null_reasoning_is_regression(tmp_path):
    """`"reasoning": null` (explicit JSON null, not just an absent key) must
    also be treated as a hard mismatch, not `got_reachability is None`."""
    _write(tmp_path, "python", "taint_frontier_case", _frontier_case_toml())
    sut = FakeTaintSut({"query": "taint_reaches", "reasoning": None, "warnings": []})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "regression"


def test_run_matrix_taint_probe_reasoning_without_reachability_key_is_regression(tmp_path):
    """`reasoning` present as a dict but missing the `reachability` key
    entirely (distinct from `reachability: null`) must also be a hard
    mismatch."""
    _write(tmp_path, "python", "taint_frontier_case", _frontier_case_toml())
    sut = FakeTaintSut({"query": "taint_reaches", "reasoning": {"frontier_count": 2}, "warnings": []})
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "regression"


def test_run_matrix_taint_probe_wrong_query_is_regression(tmp_path):
    """F1(a): evidence from the wrong query (or with `query` missing) must not
    be accepted as a taint result, even if it happens to carry a
    reasoning-shaped payload."""
    _write(tmp_path, "python", "taint_frontier_case", _frontier_case_toml())
    sut = FakeTaintSut({
        "query": "callers",
        "reasoning": {"reachability": None, "per_sink": []},
        "warnings": [],
    })
    results = run_matrix(tmp_path / "fixtures", sut, languages=["python"])
    assert results[0].outcome == "regression"


def test_run_matrix_taint_probe_frontier_count_min_enforced(tmp_path):
    """F1(c): `frontier_count_min` asserts `reasoning["frontier_count"] >= N`.
    Mirrors the by-construction `taint_frontier_only` fixture semantics
    (observed frontier_count == 2 against the real binary)."""
    path = _write(tmp_path, "python", "taint_frontier_min", """
[case]
language = "python"
capability = "taint_frontier_min"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
frontier_count_min = 2
""")
    case = load_case(path)
    assert case.frontier_count_min == 2

    below = run_matrix(tmp_path / "fixtures", FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": None, "frontier_count": 1, "per_sink": []},
        "warnings": [],
    }), languages=["python"])
    assert below[0].outcome == "regression"

    at_min = run_matrix(tmp_path / "fixtures", FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": None, "frontier_count": 2, "per_sink": []},
        "warnings": [],
    }), languages=["python"])
    assert at_min[0].outcome == "ok"

    missing_key = run_matrix(tmp_path / "fixtures", FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": None, "per_sink": []},
        "warnings": [],
    }), languages=["python"])
    assert missing_key[0].outcome == "regression"


def test_run_matrix_taint_probe_boolean_frontier_count_is_regression(tmp_path):
    """Item 1(a) (codex MAJOR): `bool` is a subclass of `int` in Python, so
    `isinstance(got_frontier_count, int)` would let a malformed wire payload
    `reasoning.frontier_count = true` satisfy `frontier_count_min = 1` (since
    `True >= 1`). Must be a hard mismatch -- the runtime check requires
    `type(got_frontier_count) is int`, not merely `isinstance`."""
    _write(tmp_path, "python", "taint_frontier_min", """
[case]
language = "python"
capability = "taint_frontier_min"
status = "pass"
probe = "taint"
[taint]
sources = ["app.py:2"]
[expect]
reachability = "None"
frontier_count_min = 1
""")
    results = run_matrix(tmp_path / "fixtures", FakeTaintSut({
        "query": "taint_reaches",
        "reasoning": {"reachability": None, "frontier_count": True, "per_sink": []},
        "warnings": [],
    }), languages=["python"])
    assert results[0].outcome == "regression"


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
