import os
from pathlib import Path

import pytest

from tier_a.matrix import MATRIX_LANGUAGES, load_case, run_matrix
from tier_a.model import CallEdge, FunctionDef, Location

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
PRISM_BIN = os.environ.get("PRISM_BIN", str(Path(__file__).resolve().parents[2]
                                            / "target/release/prism"))


class FakeSut:
    """Resolves the rust free_fn_same_file case correctly; everything else empty."""

    def callers(self, root, seed):
        if seed.name == "helper" and root.endswith("free_fn_same_file"):
            return [CallEdge("caller", seed, Location("main.rs", 3, 5), "run",
                             Location("main.rs", 4, 4))]
        return []


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


def test_run_matrix_statuses():
    results = run_matrix(FIXTURES, FakeSut(), languages=["rust", "go", "python"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["free_fn_same_file"].outcome == "ok"
    # a `pass` case the FakeSut can't resolve -> regression
    assert by_cap["free_fn_cross_file_use"].outcome == "regression"
    # a `known_fail` case still failing -> expected_gap. S3 flipped
    # type_method_qualified to pass, so the canonical known_fail exemplar is now a
    # reconciled dispatch gap (spec §2.4 — needs Phase-IP type-confirmed dispatch).
    assert by_cap["inherited_override"].outcome == "expected_gap"


def test_run_matrix_flags_flip_candidates():
    class FlipSut(FakeSut):
        def callers(self, root, seed):
            # a known_fail case the SUT *can* resolve -> flip_candidate
            if root.endswith("inherited_override"):
                # expected.toml: caller at app.py:10
                return [CallEdge("caller", seed, Location("app.py", 9, 11), "run",
                                 Location("app.py", 10, 10))]
            return super().callers(root, seed)

    results = run_matrix(FIXTURES, FlipSut(), languages=["rust", "go", "python"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["inherited_override"].outcome == "flip_candidate"


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
