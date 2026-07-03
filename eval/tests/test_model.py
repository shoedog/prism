from tier_a.model import (CallEdge, DefTarget, FunctionDef, Location,
                          edge_tier, from_lsp_line, match_by_selection, tie_break)


def fd(name, file, start, end, sel, kind="function", container=None):
    return FunctionDef(name=name, kind=kind, container=container,
                       location=Location(file, start, end), selection_line=sel)


def test_from_lsp_line_is_zero_to_one_based():
    assert from_lsp_line(0) == 1


def test_tie_break_smallest_span_then_start_then_file():
    a = fd("f", "b.rs", 10, 30, 10)
    b = fd("f", "b.rs", 12, 20, 12)   # smallest span wins
    c = fd("f", "a.rs", 12, 20, 12)   # same span: lower file wins over b
    assert tie_break([a, b]) is b
    assert tie_break([b, c]) is c


def test_match_by_selection_tolerates_doc_comment_offset():
    # LSP DocumentSymbol.range includes the doc comment (starts line 5);
    # tree-sitter's node starts at the fn keyword (line 9). Name-token line (11... )
    # falls inside prism's [9, 20] span -> match (spec §2.4).
    oracle = fd("build", "src/x.rs", 5, 20, 9)
    prism_rec = fd("build", "src/x.rs", 9, 20, 9, kind="function_item")
    assert match_by_selection(oracle, [prism_rec]) is prism_rec


def test_match_by_selection_requires_name_equality():
    oracle = fd("build", "src/x.rs", 5, 20, 9)
    other = fd("rebuild", "src/x.rs", 9, 20, 9)
    assert match_by_selection(oracle, [other]) is None


def _edge(score):
    seed = fd("seed", "src/x.py", 1, 3, 1)
    return CallEdge("caller", seed, None, "run", Location("src/x.py", 2, 2), None, score)


def test_edge_tier_none_score_is_exact():
    # legacy stored sites (recorded before P6a, or replayed from an old run JSON)
    # carry no score -- preserve today's all-together counting.
    assert edge_tier(_edge(None)) == "exact"


def test_edge_tier_full_confidence_is_exact():
    assert edge_tier(_edge(1.0)) == "exact"


def test_edge_tier_name_only_confidence_is_candidate():
    assert edge_tier(_edge(0.6)) == "candidate"


def test_edge_tier_threshold_is_inclusive_of_near_one():
    assert edge_tier(_edge(0.999)) == "exact"
