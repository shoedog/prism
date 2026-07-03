import os
from pathlib import Path

import pytest

from tier_a.matrix import MATRIX_LANGUAGES, load_case, run_matrix
from tier_a.model import CallEdge, FunctionDef, Location

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
PRISM_BIN = os.environ.get("PRISM_BIN", str(Path(__file__).resolve().parents[2]
                                            / "target/release/prism"))


class FakeSut:
    """Resolves the rust free_fn_same_file case correctly; everything else empty.

    run_matrix() scans the REAL eval/fixtures/<lang> tree (not a tmp copy), so
    this fake must also answer the P6bc taint/module_deps probes now living
    alongside the callers fixtures -- otherwise _run_taint_case/_run_module_case
    raise AttributeError on those fixtures. Neither stub is designed to match
    (this test only asserts specific `callers`-probe capability outcomes), it
    just needs to not blow up.
    """

    def callers(self, root, seed):
        if seed.name == "helper" and root.endswith("free_fn_same_file"):
            return [CallEdge("caller", seed, Location("main.rs", 3, 5), "run",
                             Location("main.rs", 4, 4))]
        return []

    def taint_reaches(self, root, sources, sinks):
        return {"reasoning": None, "warnings": []}

    def module_deps(self, root, file):
        return {"items": []}


class KindSut:
    def __init__(self, kind):
        self.kind = kind

    def callers(self, root, seed):
        return [CallEdge("caller", seed, Location("main.ts", 1, 3), "run",
                         Location("main.ts", 2, 2), self.kind)]


def test_load_case_parses_expected_toml():
    case = load_case(FIXTURES / "rust" / "free_fn_same_file" / "expected.toml")
    assert case.capability == "free_fn_same_file"
    assert case.seed_symbol == "helper" and case.seed_line == 1
    assert case.expect_callers == {("main.rs", 4)}
    assert case.exact and case.status == "pass"


def _write_known_fail_case(tmp_path, capability="synthetic_known_fail"):
    """A minimal standalone known_fail case (P3 review-fix F3): no real fixture
    is currently known_fail (P3 flipped the prior exemplar, inherited_override,
    to pass — see its expected.toml), so synthesize one instead of relying on
    a capability whose status can drift out from under these tests again."""
    case_dir = tmp_path / "fixtures" / "python" / capability
    case_dir.mkdir(parents=True)
    (case_dir / "expected.toml").write_text(f"""
[case]
language = "python"
capability = "{capability}"
status = "known_fail"
[seed]
symbol = "target"
file = "util.py"
line = 1
[[expect.callers]]
file = "main.py"
line = 2
[expect]
exact = true
""")
    return case_dir


def test_run_matrix_statuses(tmp_path):
    results = run_matrix(FIXTURES, FakeSut(), languages=["rust", "go", "python"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["free_fn_same_file"].outcome == "ok"
    # a `pass` case the FakeSut can't resolve -> regression
    assert by_cap["free_fn_cross_file_use"].outcome == "regression"

    # a `known_fail` case still failing -> expected_gap.
    _write_known_fail_case(tmp_path)
    gap_results = run_matrix(tmp_path / "fixtures", FakeSut(), languages=["python"])
    by_cap_gap = {r.capability: r for r in gap_results}
    assert by_cap_gap["synthetic_known_fail"].outcome == "expected_gap"


def test_run_matrix_flags_flip_candidates(tmp_path):
    _write_known_fail_case(tmp_path)

    class FlipSut:
        def callers(self, root, seed):
            # a known_fail case the SUT *can* resolve -> flip_candidate
            return [CallEdge("caller", seed, Location("main.py", 1, 3), "run",
                             Location("main.py", 2, 2))]

    results = run_matrix(tmp_path / "fixtures", FlipSut(), languages=["python"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["synthetic_known_fail"].outcome == "flip_candidate"


def test_run_matrix_checks_expected_resolution_kind(tmp_path):
    case_dir = tmp_path / "fixtures" / "typescript" / "kind_case"
    case_dir.mkdir(parents=True)
    (case_dir / "expected.toml").write_text("""
[case]
language = "typescript"
capability = "kind_case"
status = "pass"
[seed]
symbol = "target"
file = "util.ts"
line = 1
[[expect.callers]]
file = "main.ts"
line = 2
[expect]
exact = true
resolution_kind = "import_member"
""")
    ok = run_matrix(tmp_path / "fixtures", KindSut("import_member"), ["typescript"])
    wrong = run_matrix(tmp_path / "fixtures", KindSut("free_single"), ["typescript"])
    assert ok[0].outcome == "ok"
    assert wrong[0].outcome == "regression"


def test_run_matrix_forbids_resolution_kind(tmp_path):
    case_dir = tmp_path / "fixtures" / "typescript" / "forbid_case"
    case_dir.mkdir(parents=True)
    (case_dir / "expected.toml").write_text("""
[case]
language = "typescript"
capability = "forbid_case"
status = "pass"
[seed]
symbol = "target"
file = "util.ts"
line = 1
[expect]
callers = []
exact = false
forbid_resolution_kind = "import_member"
""")
    ok = run_matrix(tmp_path / "fixtures", KindSut("free_single"), ["typescript"])
    wrong = run_matrix(tmp_path / "fixtures", KindSut("import_member"), ["typescript"])
    assert ok[0].outcome == "ok"
    assert wrong[0].outcome == "regression"


def test_load_case_rejects_resolution_kind_without_expected_callers(tmp_path):
    case_dir = tmp_path / "fixtures" / "typescript" / "empty_kind_case"
    case_dir.mkdir(parents=True)
    expected = case_dir / "expected.toml"
    expected.write_text("""
[case]
language = "typescript"
capability = "empty_kind_case"
status = "pass"
[seed]
symbol = "target"
file = "util.ts"
line = 1
[expect]
callers = []
exact = false
resolution_kind = "import_member"
""")
    with pytest.raises(ValueError, match="resolution_kind requires"):
        load_case(expected)


@pytest.mark.skipif(not os.path.exists(PRISM_BIN), reason="release prism binary absent")
def test_matrix_against_real_binary_has_no_regressions():
    from tier_a.sut import PrismCli
    sut = PrismCli(str(Path(__file__).resolve().parents[2]), sut_bin=PRISM_BIN,
                   allow_stale=True)   # self-test: freshness is Task 21's concern
    results = run_matrix(FIXTURES, sut, MATRIX_LANGUAGES)
    regressions = [r for r in results if r.outcome == "regression"]
    assert not regressions, f"matrix regressions: {regressions}"
