from tier_c.claims import count_claims

def test_counts_sentences_asserting_code_facts():
    text = ("The matcher lives in globset. It calls compile(). The sky is blue. "
            "We must update the Glob struct.")
    # 3 code-claim sentences (globset, compile(), Glob struct); 'sky is blue' excluded
    assert count_claims(text) == 3

def test_minimum_one_when_any_code_token():
    assert count_claims("uses Foo") >= 1
    assert count_claims("hello world") == 0
