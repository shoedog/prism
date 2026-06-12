from tier_a.model import DefTarget, FunctionDef, Location
from tier_a.spotcheck import classify_site, find_call_position

SEED = FunctionDef(
    "target",
    "method",
    "TaintSeed",
    Location("src/algorithms/taint.rs", 1276, 1278),
    1276,
)


def test_call_position_preferred_over_binding():
    # bare-first-occurrence would hit the LHS local and mint a false FP (§2.6)
    line = "    let target = edge.target();"
    assert find_call_position(line, "target") == line.index("edge.target") + len("edge.")


def test_name_absent_is_alias_site_not_fp():
    assert classify_site("    g()", "target", [], SEED) == "alias_site"


def test_any_matching_definition_confirms_tp():
    defs = [DefTarget(Location("src/algorithms/taint.rs", 1276, 1278), "target", "method")]
    assert classify_site("    seed.target()", "target", defs, SEED) == "confirmed_tp"


def test_all_other_named_definitions_confirm_fp():
    defs = [DefTarget(Location("src/petgraph_shim.rs", 40, 44), "edge_endpoint", "method")]
    assert classify_site("    e.target()", "target", defs, SEED) == "confirmed_fp"


def test_same_name_different_def_is_ambiguous():
    # oracle returned a trait/interface DECLARATION named like the seed (§2.6)
    defs = [DefTarget(Location("src/traits.rs", 10, 12), "target", "method")]
    assert classify_site("    s.target()", "target", defs, SEED) == "ambiguous"
