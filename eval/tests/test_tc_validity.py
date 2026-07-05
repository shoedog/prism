"""D1 — citation VALIDITY dimension: claim-sentence extraction + judged support rate.

Mocks the ensemble ask() seam (same fake-ask pattern as test_tc_judges_live.py) and a
fake Checkout (resolve_rel/read_window), following the harness's existing conventions.
"""
from __future__ import annotations

from tier_c.model import Citation
from tier_c.validity import (
    CitationClaim,
    CitationValidityJudge,
    extract_citation_claims,
    score_validity,
)


# ---------------------------------------------------------------------------
# extract_citation_claims — pairs each occurrence with its enclosing sentence
# ---------------------------------------------------------------------------

def test_extract_pairs_citation_with_enclosing_sentence():
    """The sentence splitter (claims.sentence_spans, shared with D0's count_claims) is a
    naive [.!?\\n]+ split, so a period INSIDE the cited path (src/a.py) also ends the
    "sentence" there — the citation's start position falls in the fragment up to that
    inner period, matching count_claims' documented heuristic-FP behavior exactly."""
    text = "The bug is a race. It is fixed in src/a.py:10. A second note follows."
    claims = extract_citation_claims(text)
    assert len(claims) == 1
    assert claims[0].cite == Citation("src/a.py", 10, None)
    assert "fixed in src/a" in claims[0].sentence


def test_extract_counts_repeated_citation_in_different_sentences_separately():
    """Same file:line cited from two different sentences -> two distinct claims (D1
    scores the SENTENCE-level claim, not the deduped citation)."""
    text = "First see src/a.py:10 for the setup. Then also see src/a.py:10 for the fix."
    claims = extract_citation_claims(text)
    assert len(claims) == 2
    assert claims[0].sentence != claims[1].sentence


def test_extract_empty_text_returns_no_claims():
    assert extract_citation_claims("") == []


def test_extract_text_with_no_citations_returns_no_claims():
    assert extract_citation_claims("Just prose, no code references at all.") == []


# ---------------------------------------------------------------------------
# CitationValidityJudge — ensemble-backed SUPPORTED/UNSUPPORTED/CONTRADICTED
# ---------------------------------------------------------------------------

def _claim() -> CitationClaim:
    return CitationClaim(cite=Citation("a.py", 10, "f"), sentence="f() closes the handle.")


def test_validity_judge_parses_supported():
    j = CitationValidityJudge(ask=lambda m, p: "SUPPORTED, the code closes the handle")
    ev = j.validity(_claim(), "def f():\n    fh.close()")
    assert ev.verdict == "SUPPORTED"
    assert ev.escalated is False
    assert len(ev.votes) == 2


def test_validity_judge_parses_contradicted():
    j = CitationValidityJudge(ask=lambda m, p: "CONTRADICTED, the handle is never closed")
    ev = j.validity(_claim(), "def f():\n    pass")
    assert ev.verdict == "CONTRADICTED"


def test_validity_judge_defaults_unsupported_on_unparsed_reply():
    """Conservative default: an unparseable reply (no leading verdict token) must read
    as UNSUPPORTED, mirroring ensemble.py's documented default-on-unparse contract."""
    j = CitationValidityJudge(ask=lambda m, p: "I'm not sure, hard to tell from this")
    ev = j.validity(_claim(), "def f(): ...")
    assert ev.verdict == "UNSUPPORTED"


def test_validity_judge_prompt_includes_sentence_and_code():
    seen = {}
    def ask(m, p):
        seen["p"] = p
        return "SUPPORTED"
    j = CitationValidityJudge(ask=ask)
    j.validity(_claim(), "CODE-WINDOW-XYZ")
    assert "f() closes the handle." in seen["p"]
    assert "CODE-WINDOW-XYZ" in seen["p"]


def test_validity_judge_escalates_on_split():
    seq = iter(["SUPPORTED x", "UNSUPPORTED y"])
    def ask(m, p):
        return "CONTRADICTED final" if m == "opus-4.8" else next(seq)
    ev = CitationValidityJudge(ask).validity(_claim(), "code")
    assert ev.escalated is True and ev.verdict == "CONTRADICTED"


# ---------------------------------------------------------------------------
# score_validity — claim-support RATE over claims (never a count over citations)
# ---------------------------------------------------------------------------

class _FakeCo:
    """Every cite resolves; read_window returns a fixed code string."""
    def resolve_rel(self, rel):
        return rel
    def read_window(self, rel, line):
        return f"code @ {rel}:{line}"


class _PartialCo:
    """Only 'real.py' resolves; everything else is a hallucination."""
    def resolve_rel(self, rel):
        return rel if rel == "real.py" else None
    def read_window(self, rel, line):
        return f"code @ {rel}:{line}"


def test_score_validity_rate_is_over_claims_not_citations():
    """3 claims (from 3 sentences), 2 SUPPORTED, 1 CONTRADICTED -> support_rate = 2/3,
    NOT a raw count. This is the anti-volume guarantee spec D1 requires."""
    text = ("Fix one is in a.py:1. Fix two is in a.py:1. "  # SAME cite, different sentences
            "The third detail is in b.py:2.")

    # Deterministic per-sentence verdict (both sonnet votes see the SAME prompt content
    # and so always agree -> no opus escalation, and no StopIteration risk from a shared
    # iterator being consumed more than once per claim).
    def ask(model, prompt):
        if "Fix one" in prompt:
            return "SUPPORTED one"
        if "Fix two" in prompt:
            return "SUPPORTED two"
        assert "third detail" in prompt
        return "CONTRADICTED three"

    judge = CitationValidityJudge(ask=ask)
    report = score_validity(_FakeCo(), text, judge)
    assert report.total == 3
    assert report.support_rate == 2 / 3, f"expected 2/3, got {report.support_rate}"
    assert report.contradicted == 1


def test_score_validity_unresolvable_citation_is_unsupported_without_asking():
    """A hallucinated citation has no code window to judge -> conservative UNSUPPORTED,
    and the ensemble must NOT be spent on it (no code exists to show it)."""
    calls = []
    def ask(m, p):
        calls.append(p)
        return "SUPPORTED"
    judge = CitationValidityJudge(ask=ask)
    text = "The fix touches ghost.py:99 directly."
    report = score_validity(_PartialCo(), text, judge)
    assert report.total == 1
    assert report.verdicts[0].verdict == "UNSUPPORTED"
    assert calls == [], "ensemble must not be called for an unresolvable citation"


def test_score_validity_empty_text_gives_zero_rate_not_crash():
    judge = CitationValidityJudge(ask=lambda m, p: "SUPPORTED")
    report = score_validity(_FakeCo(), "", judge)
    assert report.total == 0
    assert report.support_rate == 0.0
    assert report.contradicted == 0


def test_score_validity_all_contradicted_gives_zero_support_rate():
    judge = CitationValidityJudge(ask=lambda m, p: "CONTRADICTED nope")
    text = "The claim about a.py:1 is wrong."
    report = score_validity(_FakeCo(), text, judge)
    assert report.support_rate == 0.0
    assert report.contradicted == 1


# ---------------------------------------------------------------------------
# resolver-fix-spec.md R5 — D1 rewire onto the fixed resolver: a bare-but-real
# citation must RESOLVE (via resolve_cite) so its window actually reaches the judge,
# instead of auto-UNSUPPORTED just because the path was ambiguous.
# ---------------------------------------------------------------------------

class _LayeredResolveCo:
    """Fake exposing resolve_cite (not resolve_rel) — simulates the noqa.rs shape:
    a bare `noqa.rs:1014` citation resolves via the line-range layer; a bare citation
    to a line that's real in NO candidate is ABSENT."""

    def resolve_cite(self, file, line=None, symbol=None, claim_text="", *, disambiguate=None):
        from tier_c.checkout import ABSENT, RESOLVED, ResolveResult
        if file == "noqa.rs" and line == 1014:
            return ResolveResult(status=RESOLVED, path="real/noqa.rs", layer="line_range")
        return ResolveResult(status=ABSENT, path=None)

    def read_window(self, rel, line):
        return f"code @ {rel}:{line}"


def test_score_validity_bare_but_real_citation_resolves_and_reads_window():
    """THE R5 FIX: a bare ambiguous-basename citation to a REAL line must resolve and
    have its window read by the judge — NOT auto-UNSUPPORTED (the old bug: ANY
    ambiguous basename skipped straight to UNSUPPORTED regardless of the line)."""
    seen = {}
    def ask(m, p):
        seen["prompt"] = p
        return "SUPPORTED"
    judge = CitationValidityJudge(ask=ask)
    text = "The fix is in noqa.rs:1014 exactly."
    report = score_validity(_LayeredResolveCo(), text, judge)
    assert report.total == 1
    assert report.verdicts[0].verdict == "SUPPORTED"
    assert "code @ real/noqa.rs:1014" in seen["prompt"], (
        "the RESOLVED path's window must reach the judge prompt"
    )


def test_score_validity_absent_citation_via_resolve_cite_stays_unsupported():
    calls = []
    def ask(m, p):
        calls.append(p)
        return "SUPPORTED"
    judge = CitationValidityJudge(ask=ask)
    text = "The fix is in noqa.rs:1 exactly."
    report = score_validity(_LayeredResolveCo(), text, judge)
    assert report.verdicts[0].verdict == "UNSUPPORTED"
    assert calls == [], "a truly-ABSENT citation must never spend an ensemble call"
