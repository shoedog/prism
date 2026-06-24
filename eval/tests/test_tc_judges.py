from tier_c.judges import borda_consensus, family_bias, detectable

def test_borda_consensus_combines_two_rankings():
    a = ["x", "y", "z", "w"]   # judge A best-first
    b = ["y", "x", "w", "z"]   # judge B
    order = borda_consensus({"A": a, "B": b})
    assert order[0] == "x"  # x and y tie on Borda points; deterministic tie-break by id

def test_family_bias_detects_own_family_inflation():
    # anthropic judge ranks anthropic ids high; openai judge ranks openai high
    fam = {"a1": "anthropic", "a2": "anthropic", "o1": "openai", "o2": "openai"}
    rankings = {"anthropic": ["a1", "a2", "o1", "o2"], "openai": ["o1", "o2", "a1", "a2"]}
    bias = family_bias(rankings, fam)
    assert bias > 0  # each judge favors own family

def test_no_bias_when_judges_agree():
    fam = {"a1": "anthropic", "o1": "openai"}
    rankings = {"anthropic": ["a1", "o1"], "openai": ["a1", "o1"]}
    assert family_bias(rankings, fam) == 0.0

def test_detectable_true_above_chance():
    # classifier guessed condition correctly 9/10 -> detectable
    assert detectable(correct=9, n=10, threshold=0.7)
    assert not detectable(correct=5, n=10, threshold=0.7)

from tier_c.judges import borda_consensus, has_tie

def test_borda_seeded_random_tiebreak_is_deterministic():
    r = {"A": ["x","y"], "B": ["y","x"]}  # x,y tie
    o1 = borda_consensus(r, seed="issue1|spec")
    o2 = borda_consensus(r, seed="issue1|spec")
    assert o1 == o2                      # reproducible
    assert set(o1) == {"x","y"}

def test_has_tie_detects_top_tie():
    assert has_tie({"A": ["x","y"], "B": ["y","x"]})      # x,y both score equal
    assert not has_tie({"A": ["x","y"], "B": ["x","y"]})  # x strictly wins

from tier_c.judges import detectability_pvalue

def test_detectability_pvalue_low_when_guesser_is_accurate():
    # 10 issues, guesser got 9/10 prism-condition guesses right -> low p (detectable)
    p = detectability_pvalue(correct=9, n=10)
    assert p < 0.05
def test_detectability_pvalue_high_at_chance():
    assert detectability_pvalue(correct=5, n=10) > 0.2
