import os
from pathlib import Path

import pytest

from tier_a.matrix import load_case, run_matrix
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


def test_load_case_parses_expected_toml():
    case = load_case(FIXTURES / "rust" / "free_fn_same_file" / "expected.toml")
    assert case.capability == "free_fn_same_file"
    assert case.seed_symbol == "helper" and case.seed_line == 1
    assert case.expect_callers == {("main.rs", 4)}
    assert case.exact and case.status == "pass"


def test_run_matrix_statuses():
    results = run_matrix(FIXTURES, FakeSut(), languages=["rust"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["free_fn_same_file"].outcome == "ok"
    # a `pass` case the FakeSut can't resolve -> regression
    assert by_cap["free_fn_cross_file_use"].outcome == "regression"
    # a `known_fail` case still failing -> expected_gap
    # (type_method_qualified is the post-reconciliation known_fail — the §2.7
    # step-5 run flipped trait_dyn_dispatch to pass)
    assert by_cap["type_method_qualified"].outcome == "expected_gap"


def test_run_matrix_flags_flip_candidates():
    class FlipSut(FakeSut):
        def callers(self, root, seed):
            if root.endswith("type_method_qualified"):
                # expected.toml: caller at main.rs:4
                return [CallEdge("caller", seed, Location("main.rs", 3, 5), "run",
                                 Location("main.rs", 4, 4))]
            return super().callers(root, seed)

    results = run_matrix(FIXTURES, FlipSut(), languages=["rust"])
    by_cap = {r.capability: r for r in results}
    assert by_cap["type_method_qualified"].outcome == "flip_candidate"


@pytest.mark.skipif(not os.path.exists(PRISM_BIN), reason="release prism binary absent")
def test_matrix_against_real_binary_has_no_regressions():
    from tier_a.sut import PrismCli
    sut = PrismCli(str(Path(__file__).resolve().parents[2]), sut_bin=PRISM_BIN,
                   allow_stale=True)   # self-test: freshness is Task 21's concern
    results = run_matrix(FIXTURES, sut, ["rust", "go", "python"])
    regressions = [r for r in results if r.outcome == "regression"]
    assert not regressions, f"matrix regressions: {regressions}"
