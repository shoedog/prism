"""R2 — Q3 LLM disambiguator (resolver-fix-spec.md): unit tests with a fake ask().

Invoked ONLY on genuine >=2-candidate ties (see test_tc_checkout.py's resolve_cite
layer-4 tests for the integration point). These tests exercise disambiguate()/
make_disambiguator() standalone, with zero live model calls.
"""
from __future__ import annotations

from tier_c.disambiguate import Q3_MODEL, build_prompt, disambiguate, make_disambiguator


def test_disambiguate_picks_matching_candidate():
    def ask(model, prompt):
        assert model == Q3_MODEL
        return "B, the second candidate mentions the closed handle"
    result = disambiguate(ask, "the fix closes the handle", ["window A", "window B"])
    assert result.index == 1


def test_disambiguate_picks_first_candidate():
    def ask(model, prompt):
        return "A"
    result = disambiguate(ask, "claim", ["window A", "window B", "window C"])
    assert result.index == 0


def test_disambiguate_abstains_on_none_reply():
    def ask(model, prompt):
        return "NONE, cannot tell from the given context"
    result = disambiguate(ask, "claim", ["window A", "window B"])
    assert result.index is None


def test_disambiguate_abstains_on_unparseable_reply():
    def ask(model, prompt):
        return "I'm honestly not sure which one this is."
    result = disambiguate(ask, "claim", ["window A", "window B"])
    assert result.index is None


def test_disambiguate_single_candidate_is_trivially_resolved_without_asking():
    calls = []
    def ask(model, prompt):
        calls.append(prompt)
        return "A"
    result = disambiguate(ask, "claim", ["only window"])
    assert result.index == 0
    assert calls == [], "must not spend a model call when there's only one candidate"


def test_disambiguate_no_candidates_abstains_without_asking():
    calls = []
    def ask(model, prompt):
        calls.append(prompt)
        return "A"
    result = disambiguate(ask, "claim", [])
    assert result.index is None
    assert calls == []


def test_build_prompt_includes_claim_and_labeled_candidates():
    prompt = build_prompt("the fix is here", ["code A", "code B"])
    assert "the fix is here" in prompt
    assert "Candidate A:" in prompt and "code A" in prompt
    assert "Candidate B:" in prompt and "code B" in prompt
    assert "NONE" in prompt


# ---------------------------------------------------------------------------
# make_disambiguator — the (claim_text, windows) -> int|None seam Checkout.resolve_cite
# calls directly.
# ---------------------------------------------------------------------------

def test_make_disambiguator_wraps_ask_and_returns_index():
    fn = make_disambiguator(lambda m, p: "A")
    assert fn("claim", ["w1", "w2"]) == 0


def test_make_disambiguator_abstains_on_none():
    fn = make_disambiguator(lambda m, p: "NONE")
    assert fn("claim", ["w1", "w2"]) is None


def test_make_disambiguator_swallows_ask_exception_as_abstain():
    def boom(m, p):
        raise RuntimeError("judge subprocess failed")
    fn = make_disambiguator(boom)
    assert fn("claim", ["w1", "w2"]) is None, (
        "a judge-call failure must never crash scoring — conservative abstain instead"
    )


def test_make_disambiguator_uses_custom_model_override():
    seen = {}
    def ask(m, p):
        seen["model"] = m
        return "A"
    fn = make_disambiguator(ask, model="sonnet-4.6")
    fn("claim", ["w1", "w2"])
    assert seen["model"] == "sonnet-4.6"
