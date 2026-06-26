from adoption.aggregate import passes_k, summarize

def test_passes_k_requires_all():
    assert passes_k([True, True, True, True, True]) is True
    assert passes_k([True, True, False, True, True]) is False

def test_summarize_pass5_rate():
    # 2 nav probes: one all-5-pass, one not; 1 negative all-pass
    per_probe = {
        "a": {"kind": "nav", "invocation": [True]*5,  "activation": [True]*5},
        "b": {"kind": "nav", "invocation": [True]*4+[False], "activation": [True]*5},
        "n": {"kind": "negative", "invocation": [True]*5, "activation": [False]*5},
    }
    s = summarize(per_probe)
    assert s["nav_invocation_pass5_rate"] == 0.5   # 1 of 2 nav probes pass^5
    assert s["nav_count"] == 2
