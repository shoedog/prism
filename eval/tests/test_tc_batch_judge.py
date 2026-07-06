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
