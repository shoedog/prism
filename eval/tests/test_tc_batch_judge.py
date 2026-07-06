"""Batch judging (perf fix): tier_c.batch_judge.classify_batch collapses the N x
(2-sonnet+opus ensemble) per-citation fanout into ONE single-model call per arm.

Root problem this replaces (see rubric-v2-report.md "Batch judging (perf fix)"):
D1 validity and relevance each judged PER CITATION via ensemble.py's 2-sonnet+opus
vote -> an arm with N citations cost N x (2-3) cold `claude -p` subprocess calls.
classify_batch asks the model to classify every item in ONE prompt/ONE call.
"""
from __future__ import annotations

from tier_c.batch_judge import classify_batch

CHOICES = ("SUPPORTED", "UNSUPPORTED", "CONTRADICTED")


# ---------------------------------------------------------------------------
# R1 (a)-(d): classify_batch parsing/defaults/empty-no-call
# ---------------------------------------------------------------------------

def test_classify_batch_three_well_formed_lines_parse_correctly():
    def ask(model, prompt):
        return "#1 SUPPORTED yes\n#2 UNSUPPORTED no\n#3 CONTRADICTED nope"
    out = classify_batch(ask, "sonnet-4.6", "intro", ["a", "b", "c"], CHOICES,
                         default="UNSUPPORTED")
    assert out == ["SUPPORTED", "UNSUPPORTED", "CONTRADICTED"]


def test_classify_batch_missing_line_defaults_that_item_only():
    def ask(model, prompt):
        return "#1 SUPPORTED yes\n#3 CONTRADICTED nope"  # #2 missing
    out = classify_batch(ask, "sonnet-4.6", "intro", ["a", "b", "c"], CHOICES,
                         default="UNSUPPORTED")
    assert out == ["SUPPORTED", "UNSUPPORTED", "CONTRADICTED"]


def test_classify_batch_unparseable_token_defaults_that_item_only():
    def ask(model, prompt):
        return "#1 SUPPORTED yes\n#2 MAYBE not sure\n#3 CONTRADICTED nope"
    out = classify_batch(ask, "sonnet-4.6", "intro", ["a", "b", "c"], CHOICES,
                         default="UNSUPPORTED")
    assert out == ["SUPPORTED", "UNSUPPORTED", "CONTRADICTED"]


def test_classify_batch_empty_items_returns_empty_and_never_calls_ask():
    def boom(model, prompt):
        raise AssertionError("ask must NEVER be called for an empty item list")
    assert classify_batch(boom, "sonnet-4.6", "intro", [], CHOICES,
                          default="UNSUPPORTED") == []


# ---------------------------------------------------------------------------
# Singleton fallback: a batch of exactly one item where the model ignores the
# "#1 VERDICT" instruction and answers with a bare token is still parsed —
# unambiguous when there's only one item. This keeps every pre-existing
# single-primitive-era fake (`ask=lambda m, p: "SUPPORTED"`) green without
# requiring every such test to be rewritten (resolver-fix/rubric-v2 test suite).
# ---------------------------------------------------------------------------

def test_classify_batch_singleton_accepts_bare_unprefixed_reply():
    def ask(model, prompt):
        return "CONTRADICTED, plain reply with no #1 prefix"
    out = classify_batch(ask, "sonnet-4.6", "intro", ["only item"], CHOICES,
                         default="UNSUPPORTED")
    assert out == ["CONTRADICTED"]


def test_classify_batch_singleton_bare_reply_defaults_when_unparseable():
    def ask(model, prompt):
        return "not sure, hard to tell"
    out = classify_batch(ask, "sonnet-4.6", "intro", ["only item"], CHOICES,
                         default="UNSUPPORTED")
    assert out == ["UNSUPPORTED"]


def test_classify_batch_prompt_numbers_items_and_carries_intro():
    seen = {}
    def ask(model, prompt):
        seen["prompt"] = prompt
        return "#1 SUPPORTED x\n#2 SUPPORTED y"
    classify_batch(ask, "sonnet-4.6", "INTRO-TEXT", ["ITEM-ONE", "ITEM-TWO"], CHOICES,
                  default="UNSUPPORTED")
    p = seen["prompt"]
    assert "INTRO-TEXT" in p
    assert "ITEM-ONE" in p and "ITEM-TWO" in p
    assert "#1" in p and "#2" in p


# ---------------------------------------------------------------------------
# R4 — anti-fanout perf guard: call count independent of citation count.
# An arm with >=4 citations across >=2 sentences must cost AT MOST ONE ask()
# call for validity and AT MOST ONE for relevance — this is the test that
# would have caught the original O(N) per-citation ensemble design
# (~140-220 calls/cell, >58 min/cell).
# ---------------------------------------------------------------------------

def test_batch_judging_call_count_is_independent_of_citation_count():
    import types
    from tier_c.cli import _LivePartCComps
    from tier_c.validity import CitationValidityJudge, score_validity
    from tier_c.model import Citation

    calls = {"n": 0}

    def counting_ask(model, prompt):
        calls["n"] += 1
        n_items = prompt.count("--- Item #")
        return "\n".join(f"#{i} YES" for i in range(1, n_items + 1))

    class _CountingCo:
        root = "/fake"
        def resolve_rel(self, f):
            return f
        def file_exists(self, f):
            return True
        def read_line(self, f, ln):
            return "func Foo() Bar { }"
        def read_window(self, f, ln):
            return "def foo(): ..."

    co = _CountingCo()
    text = (
        "Claim one touches src/a.go:1. Claim two touches src/b.go:2. "
        "Claim three touches src/c.go:3. Claim four touches src/d.go:4."
    )

    # --- D1 validity: 4 claims across 4 sentences -> <=1 call ---
    judge = CitationValidityJudge(ask=counting_ask)
    vreport = score_validity(co, text, judge)
    assert vreport.total == 4
    validity_calls = calls["n"]
    assert validity_calls <= 1, f"validity must cost <=1 ask call, got {validity_calls}"

    # --- relevance: 4 citations -> <=1 ADDITIONAL call ---
    issue = types.SimpleNamespace(text="ISSUE", scoped_slice="s", repo="ruff", sha="abc")
    comps = _LivePartCComps(co=co, issue=issue, model="opus-4.8", base_root="x",
                            ask=counting_ask)
    comps._upstream_spec = lambda cell: ""
    cites = [
        Citation(file="src/a.go", line=1, symbol=None),
        Citation(file="src/b.go", line=2, symbol=None),
        Citation(file="src/c.go", line=3, symbol=None),
        Citation(file="src/d.go", line=4, symbol=None),
    ]
    rep = comps.score(cites, cell=("ruff", "spec", "opus-4.8"), arm="on")
    assert len(rep.verdicts) == 4
    relevance_calls = calls["n"] - validity_calls
    assert relevance_calls <= 1, f"relevance must cost <=1 ask call, got {relevance_calls}"

    assert calls["n"] <= 2, (
        f"TOTAL validity+relevance calls over a 4-citation arm must be <=2 "
        f"(pre-fix this was ~4 x 2-3 = 8-12 ensemble calls), got {calls['n']}"
    )
