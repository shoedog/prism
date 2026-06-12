from tier_a.model import CallEdge, FunctionDef, Location
from tier_a.pinned import evaluate_pinned, run_pinned

SEED = FunctionDef("target", "method", None, Location("src/seed.rs", 1, 3), 1)


def edge(file: str, line: int) -> CallEdge:
    return CallEdge("caller", SEED, None, None, Location(file, line, line))


def test_known_fail_reproduces_when_precision_and_recall_are_low():
    got = evaluate_pinned(
        {"id": "target-c-method", "expected": "known_fail"},
        prism_edges=[edge("src/p.rs", 1)],
        oracle_edges=[edge("src/o1.rs", 10), edge("src/o2.rs", 20)],
        ambiguous_raised=False,
    )
    assert got["outcome"] == "known_fail"
    assert got["known_fail"]


def test_known_fail_becomes_flip_candidate_when_metrics_improve():
    got = evaluate_pinned(
        {"id": "target-c-method", "expected": "known_fail"},
        prism_edges=[edge("src/a.rs", 10)],
        oracle_edges=[edge("src/a.rs", 10)],
        ambiguous_raised=False,
    )
    assert got["outcome"] == "flip_candidate"
    assert not got["known_fail"]


def test_oracle_miss_site_found_in_prism_only_diff():
    got = evaluate_pinned(
        {
            "id": "module-deps-feature-gated",
            "expected": "oracle_miss_site",
            "oracle_miss_site": "src/mcp/tools.rs:162",
        },
        prism_edges=[edge("src/mcp/tools.rs", 162)],
        oracle_edges=[],
        ambiguous_raised=False,
    )
    assert got["outcome"] == "ok"
    assert got["oracle_miss_site_found"]


def test_oracle_miss_site_missing_from_prism_only_diff():
    got = evaluate_pinned(
        {
            "id": "module-deps-feature-gated",
            "expected": "oracle_miss_site",
            "oracle_miss_site": "src/mcp/tools.rs:162",
        },
        prism_edges=[edge("src/other.rs", 9)],
        oracle_edges=[],
        ambiguous_raised=False,
    )
    assert got["outcome"] == "missing"
    assert not got["oracle_miss_site_found"]


def test_ambiguous_symbol_contract_records_safe_fail():
    got = evaluate_pinned(
        {"id": "ambiguous-symbol-contract", "expected": "ambiguous_symbol_error"},
        prism_edges=[],
        oracle_edges=[],
        ambiguous_raised=True,
    )
    assert got["outcome"] == "ok"
    assert got["ambiguous_ok"]


def test_ambiguous_symbol_unexpected_error_is_recorded_not_raised(monkeypatch):
    import tier_a.pinned as pinned

    class BadSut:
        def callers_by_symbol(self, _root, _symbol, file=None):
            raise RuntimeError("nav failed")

    monkeypatch.setattr(
        pinned,
        "PINNED",
        [{
            "id": "ambiguous-symbol-contract",
            "symbol": "slice",
            "file": None,
            "expected": "ambiguous_symbol_error",
        }],
    )
    [got] = run_pinned(oracle=None, sut=BadSut(), snapshot=[], corpus_root="/repo")
    assert got["outcome"] == "regression"
    assert not got["ambiguous_ok"]
    assert got["error"] == "nav failed"
