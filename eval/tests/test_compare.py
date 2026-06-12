from tier_a.compare import caller_fn_sets, collapse_sites, site_compare
from tier_a.metrics import wilson
from tier_a.model import CallEdge, FunctionDef, Location


SEED = FunctionDef("f", "function", None, Location("src/s.rs", 1, 5), 1)


def edge(file, line, direction="caller", name="c", dstart=None, dend=None):
    d = Location(file, dstart or max(1, line - 2), dend or line + 2)
    return CallEdge(direction, SEED, d, name, Location(file, line, line))


def test_collapse_same_line_multicall():
    edges = [edge("a.rs", 10), edge("a.rs", 10), edge("a.rs", 12)]
    sites, collapsed = collapse_sites(edges)
    assert sites == {("a.rs", 10), ("a.rs", 12)}
    assert collapsed == 1


def test_site_compare_line_within_oracle_range_matches():
    # oracle fromRange spans a multi-line call 10..12; prism claims line 11
    prism = [edge("a.rs", 11)]
    oracle = [CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                       Location("a.rs", 10, 12))]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (1, 0, 0)


def test_site_compare_matches_oracle_sites_one_to_one():
    prism = [edge("a.rs", 10), edge("a.rs", 11)]
    oracle = [CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                       Location("a.rs", 9, 12))]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (1, 1, 0)


def test_site_compare_matches_collapsed_oracle_site_once():
    prism = [edge("a.rs", 10), edge("a.rs", 11)]
    oracle = [
        CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                 Location("a.rs", 9, 12)),
        CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                 Location("a.rs", 9, 14)),
    ]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (1, 1, 0)


def test_site_compare_counts_fp_and_fn():
    prism = [edge("a.rs", 30)]
    oracle = [CallEdge("caller", SEED, Location("a.rs", 5, 20), "c",
                       Location("a.rs", 10, 10))]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (0, 1, 1)


def test_caller_fn_sets_uses_module_level_bucket():
    inv = [FunctionDef("caller_fn", "function", None, Location("a.py", 5, 30), 5)]
    in_fn = edge("a.py", 10)
    at_module = edge("a.py", 50)
    fns = caller_fn_sets([in_fn, at_module], inv)
    assert fns == {("a.py", "caller_fn"), ("a.py", "<module_level>")}


def test_wilson_interval_brackets_point_estimate():
    p, lo, hi = wilson(9, 10)
    assert lo < 0.9 < hi and 0 <= lo and hi <= 1


def test_site_compare_chain_tolerance_pairs_receiver_and_name_lines():
    # prism reports the receiver line (132); the oracle reports the method-name
    # line (133) — must pair as one TP, not a phantom FP+FN (2026-06-12 finding)
    prism = [edge("a.rs", 132)]
    oracle = [CallEdge("caller", SEED, Location("a.rs", 99, 140), "c",
                       Location("a.rs", 133, 133))]
    r = site_compare(prism, oracle)
    assert (len(r.tp), len(r.fp), len(r.fn)) == (1, 0, 0)
