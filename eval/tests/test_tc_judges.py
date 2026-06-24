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
