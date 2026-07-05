"""D1 — citation VALIDITY dimension (judged; owner rubric-b).

Distinct from the existing relevance judge (judges_live.LlmRelevanceJudge = "is this
location relevant to fixing the issue?"). Validity asks a narrower, independent
question: "does the code AT the cited location support the sentence that cites it?"

- Extract (claim-sentence, Citation) pairs: for EVERY citation occurrence in the arm's
  text, take the enclosing sentence via claims.sentence_spans (the same sentence
  splitter D0 uses for the recall denominator).
- Verify (judged, scoped): show the ensemble the claim sentence + the code window
  (Checkout.read_window) and ask for a first-token SUPPORTED / UNSUPPORTED /
  CONTRADICTED verdict — same contract as ensemble.parse_verdict; default UNSUPPORTED
  on an unparsed reply (conservative).
- Score: claim-support RATE = SUPPORTED / (claims-with-citations) — a rate over claims,
  never a count over citations (anti-volume, spec D1). CONTRADICTED is reported
  separately (it feeds D3's annotated head-to-head).

Do NOT use prism as the validity oracle here (circularity/doctrine) — this is an LLM
reading the raw code window, independent of prism.
"""
from __future__ import annotations

from dataclasses import dataclass, field

from .citations import iter_citation_occurrences
from .claims import sentence_spans
from .model import Citation

_VALIDITY_CHOICES = ("SUPPORTED", "UNSUPPORTED", "CONTRADICTED")


@dataclass(frozen=True)
class CitationClaim:
    """One citation OCCURRENCE paired with its enclosing sentence."""
    cite: Citation
    sentence: str


def extract_citation_claims(text: str) -> list[CitationClaim]:
    """Pair every citation occurrence in `text` with its enclosing sentence."""
    if not text:
        return []
    spans = sentence_spans(text)
    out: list[CitationClaim] = []
    for start, _end, cite in iter_citation_occurrences(text):
        sentence = next((s for (a, b, s) in spans if a <= start < b), "").strip()
        out.append(CitationClaim(cite=cite, sentence=sentence))
    return out


@dataclass(frozen=True)
class ValidityVerdict:
    claim: CitationClaim
    verdict: str  # SUPPORTED | UNSUPPORTED | CONTRADICTED
    escalated: bool = False
    votes: list = field(default_factory=list)


@dataclass(frozen=True)
class ValidityReport:
    support_rate: float   # SUPPORTED / (claims-with-citations); 0.0 when there are none
    contradicted: int     # count, feeds D3's annotated head-to-head
    total: int
    verdicts: list[ValidityVerdict]


class CitationValidityJudge:
    """Per-citation-claim VALIDITY via the 2-sonnet/opus ensemble (ensemble.py) — same
    infra + guards as LlmRelevanceJudge (judges_live.py), an independent code-support
    question ("does the code say this?" vs relevance's "is this location relevant?")."""

    def __init__(self, ask, model: str | None = None, *, opus: str | None = None):
        from .llm import JUDGE_MODEL, JUDGE_TIEBREAKER
        self.ask = ask
        self.sonnet = model or JUDGE_MODEL
        self.opus = opus or JUDGE_TIEBREAKER

    def validity(self, claim: CitationClaim, code: str):
        from .ensemble import ensemble
        prompt = (
            f"A written claim cites code at {claim.cite.file}:{claim.cite.line}"
            f"{' (symbol ' + claim.cite.symbol + ')' if claim.cite.symbol else ''}.\n\n"
            f"Claim sentence:\n{claim.sentence}\n\n"
            f"Code at that location:\n{code}\n\n"
            "Does the code SUPPORT the claim sentence, CONTRADICT it, or is it "
            "UNSUPPORTED (the code doesn't confirm it either way)? "
            "Start your reply with EXACTLY one of SUPPORTED, UNSUPPORTED, or CONTRADICTED, "
            "then one sentence why."
        )
        return ensemble(self.ask, prompt, _VALIDITY_CHOICES,
                        sonnet=self.sonnet, opus=self.opus, default="UNSUPPORTED")


def _resolve_for_validity(co, claim: CitationClaim, *, ask) -> tuple[str | None, bool]:
    """Returns (resolved_path_or_None, ambiguous). resolver-fix-spec.md R5: prefer the
    layered resolve_cite (R1) so a bare-but-real citation RESOLVES and its window is
    actually read by the validity judge — the old resolve_rel-or-file_exists fallback
    marked ANY ambiguous-basename citation unresolvable regardless of whether the line
    was real, which is exactly the measurement artifact this fix removes. The claim's
    own sentence doubles as the R1 layer-3 tie-break / R2 Q3 disambiguator context."""
    cite = claim.cite
    if hasattr(co, "resolve_cite"):
        disambiguate_fn = None
        if ask is not None:
            from .disambiguate import make_disambiguator
            disambiguate_fn = make_disambiguator(ask)
        rr = co.resolve_cite(cite.file, cite.line, cite.symbol, claim.sentence,
                             disambiguate=disambiguate_fn)
        return (rr.path if rr.status == "RESOLVED" else None), (rr.status == "AMBIGUOUS")
    if hasattr(co, "resolve_rel"):
        return co.resolve_rel(cite.file), False
    return (cite.file if co.file_exists(cite.file) else None), False


def score_validity(co, text: str, judge: "CitationValidityJudge", *, ask=None) -> ValidityReport:
    """Score D1 claim-support rate for one arm's text against the pinned checkout `co`.

    `ask` (optional) wires the R2 Q3 disambiguator into genuinely-tied bare citations,
    same seam CitationValidityJudge itself uses (judge.ask) — pass the SAME ask when
    available so a real-but-tied citation gets one more chance to resolve before D1
    gives up on it.
    """
    claims = extract_citation_claims(text)
    verdicts: list[ValidityVerdict] = []
    for claim in claims:
        resolved, ambiguous = _resolve_for_validity(co, claim, ask=ask)
        if resolved is None or claim.cite.line is None:
            # Truly unresolvable (ABSENT, or still-AMBIGUOUS after the R2 disambiguator
            # had its shot): no definitive code window to judge -> conservative
            # UNSUPPORTED, and we never spend an ensemble call on it. A bare-but-REAL
            # citation the R1 layered resolver pinned to exactly one candidate reaches
            # the `code = ...` branch below instead (the R5 fix — never auto-UNSUPPORTED
            # just because the path was bare).
            verdicts.append(ValidityVerdict(claim=claim, verdict="UNSUPPORTED"))
            continue
        code = co.read_window(resolved, claim.cite.line) or ""
        ev = judge.validity(claim, code)
        verdicts.append(ValidityVerdict(claim=claim, verdict=ev.verdict,
                                        escalated=ev.escalated, votes=ev.votes))
    total = len(verdicts)
    supported = sum(1 for v in verdicts if v.verdict == "SUPPORTED")
    contradicted = sum(1 for v in verdicts if v.verdict == "CONTRADICTED")
    rate = supported / total if total else 0.0
    return ValidityReport(support_rate=rate, contradicted=contradicted, total=total,
                          verdicts=verdicts)
