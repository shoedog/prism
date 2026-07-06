"""D3 — fact-annotated head-to-head: mechanical inline tagging + the mandatory pooled
detectability re-check over annotated texts (a single cell can't reach significance —
mirrors test_tc_detect.py's own underpowered-single-stage case)."""
from __future__ import annotations

from tier_c.annotate import (
    annotate_arm_text,
    contradicted_keys,
    hallucinated_keys,
    run_annotated_detectability,
)
from tier_c.investigator import CitationVerdict
from tier_c.model import Citation


def _verdict(file, line, symbol=None, *, file_ok=True, line_ok=True, symbol_ok=True,
            relevant=True) -> CitationVerdict:
    return CitationVerdict(cite=Citation(file, line, symbol), file_ok=file_ok,
                           line_ok=line_ok, symbol_ok=symbol_ok, relevant=relevant)


# ---------------------------------------------------------------------------
# hallucinated_keys / contradicted_keys
# ---------------------------------------------------------------------------

def test_hallucinated_keys_only_flags_hallucinations():
    verdicts = [
        _verdict("real.py", 1),                       # not a hallucination
        _verdict("ghost.py", 99, file_ok=False),       # hallucination
    ]
    keys = hallucinated_keys(verdicts)
    assert keys == {("ghost.py", 99, None)}


def test_contradicted_keys_only_flags_contradicted_verdict():
    dicts = [
        {"file": "a.py", "line": 1, "symbol": None, "sentence": "s1", "verdict": "SUPPORTED"},
        {"file": "b.py", "line": 2, "symbol": None, "sentence": "s2", "verdict": "CONTRADICTED"},
    ]
    assert contradicted_keys(dicts) == {("b.py", 2, None, "s2")}


def test_contradicted_keys_handles_none_and_empty():
    assert contradicted_keys(None) == set()
    assert contradicted_keys([]) == set()


# ---------------------------------------------------------------------------
# annotate_arm_text
# ---------------------------------------------------------------------------

def test_annotate_tags_hallucinated_citation():
    text = "The fix is at ghost.py:99 in the handler."
    out = annotate_arm_text(text, hallucinated={("ghost.py", 99, None)}, contradicted=set())
    assert "ghost.py:99 [CITED LOCATION DOES NOT EXIST]" in out


def test_annotate_tags_contradicted_claim_by_sentence():
    text = "The fix is at real.py:1 in the handler."
    # sentence text must match exactly what sentence_spans would extract (stripped)
    from tier_c.claims import sentence_spans
    spans = sentence_spans(text)
    sentence = spans[0][2].strip()
    out = annotate_arm_text(text, hallucinated=set(),
                            contradicted={("real.py", 1, None, sentence)})
    assert "[CODE CONTRADICTS THIS CLAIM]" in out


def test_annotate_leaves_unflagged_citations_untouched():
    text = "See real.py:1 for context."
    out = annotate_arm_text(text, hallucinated=set(), contradicted=set())
    assert out == text


def test_annotate_can_apply_both_tags_to_the_same_occurrence():
    text = "See real.py:1 here."
    from tier_c.claims import sentence_spans
    sentence = sentence_spans(text)[0][2].strip()
    out = annotate_arm_text(
        text,
        hallucinated={("real.py", 1, None)},
        contradicted={("real.py", 1, None, sentence)},
    )
    assert "[CITED LOCATION DOES NOT EXIST]" in out
    assert "[CODE CONTRADICTS THIS CLAIM]" in out


def test_annotate_only_tags_matching_occurrence_not_all_citations():
    """Two DIFFERENT citations in one text; only the flagged one gets tagged."""
    text = "First see a.py:1. Then see b.py:2."
    out = annotate_arm_text(text, hallucinated={("a.py", 1, None)}, contradicted=set())
    assert "a.py:1 [CITED LOCATION DOES NOT EXIST]" in out
    assert "b.py:2 [CITED LOCATION DOES NOT EXIST]" not in out
    assert "b.py:2." in out  # untouched


def test_annotate_empty_flags_returns_text_unchanged():
    text = "Nothing flagged here at all.py:1."
    assert annotate_arm_text(text, hallucinated=set(), contradicted=set()) == text


def test_annotate_empty_text_returns_empty():
    assert annotate_arm_text("", hallucinated={("a.py", 1, None)}, contradicted=set()) == ""


# ---------------------------------------------------------------------------
# resolver-fix-spec.md R5 — ambiguous citations get a NEUTRAL tag, never the
# "does not exist" hallucination tag.
# ---------------------------------------------------------------------------

def test_ambiguous_keys_only_flags_ambiguous_verdicts():
    verdicts = [
        _verdict("real.py", 1),                                      # neither
        _verdict("ghost.py", 99, file_ok=False),                     # hallucinated
    ]
    from tier_c.investigator import CitationVerdict
    ambiguous_verdict = CitationVerdict(
        cite=Citation("noqa.rs", 10, None), file_ok=False, line_ok=False,
        symbol_ok=True, relevant=True, ambiguous=True,
    )
    from tier_c.annotate import ambiguous_keys
    keys = ambiguous_keys(verdicts + [ambiguous_verdict])
    assert keys == {("noqa.rs", 10, None)}


def test_annotate_tags_ambiguous_citation_neutrally_not_as_hallucinated():
    text = "The fix likely touches noqa.rs:10 based on context."
    out = annotate_arm_text(text, hallucinated=set(), contradicted=set(),
                            ambiguous={("noqa.rs", 10, None)})
    assert "[AMBIGUOUS PATH]" in out
    assert "[CITED LOCATION DOES NOT EXIST]" not in out, (
        "R5: a real-but-unpinnable citation must NEVER read as fabrication"
    )


def test_annotate_ambiguous_default_empty_set_is_backward_compatible():
    """Existing 2-kwarg callers (hallucinated=, contradicted=) must be unaffected."""
    text = "See real.py:1 for context."
    out = annotate_arm_text(text, hallucinated=set(), contradicted=set())
    assert out == text


# ---------------------------------------------------------------------------
# run_annotated_detectability — pooled, mirrors test_tc_detect.py's own guarantees
# ---------------------------------------------------------------------------

class _Cell:
    """Minimal stand-in for PartCCell — only the fields run_annotated_detectability uses."""
    def __init__(self, model, annotated_off_text, annotated_on_text):
        self.model = model
        self.annotated_off_text = annotated_off_text
        self.annotated_on_text = annotated_on_text


class _TagGuesser:
    """Guesses prism-on iff the annotated text contains a density tell ('MANYTAGS')."""
    def guess_used_prism(self, text):
        return "MANYTAGS" in text


def test_pooled_annotated_detectability_flags_when_separable():
    cells = [_Cell(f"m{i}", "plain off", "MANYTAGS on") for i in range(4)]
    r = run_annotated_detectability(cells, _TagGuesser())
    assert r.n == 8 and r.correct == 8
    assert r.detectable is True


def test_pooled_annotated_detectability_not_flagged_at_chance():
    cells = [_Cell(f"m{i}", "plain off", "plain on") for i in range(4)]
    r = run_annotated_detectability(cells, _TagGuesser())
    assert r.detectable is False


def test_single_cell_annotated_detectability_cannot_fire():
    """A single cell (2 outputs) can never reach significance — mirrors detect.py's own
    documented underpowered-single-stage guarantee; callers must pool multiple cells."""
    cells = [_Cell("m0", "plain off", "MANYTAGS on")]
    r = run_annotated_detectability(cells, _TagGuesser())
    assert r.n == 2
    assert r.detectable is False
