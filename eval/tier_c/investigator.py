"""Independent citation oracle (spec §6a, codex new-2/new-3): mechanical existence via
neutral file/git primitives — NEVER prism — plus a SECONDARY relevance seam. Scores
citation PRECISION (real & relevant) and RECALL/claim-coverage (under-citing penalized).

resolver-fix-spec.md R1/R3: citations resolve through Checkout.resolve_cite's layered
resolver (RESOLVED | AMBIGUOUS | ABSENT) when available. The load-bearing principle:
a citation to a real line must NEVER be scored as fabrication just because its path is
bare/ambiguous — AMBIGUOUS is a THIRD, separate classification, excluded from both
`is_hallucination` and `is_valid` (and from the precision denominator, see
score_citations). Only ABSENT (line exists in no candidate file) or a resolved file
with the line/symbol actually wrong counts as hallucination.
"""
from __future__ import annotations
from dataclasses import dataclass, field
from .model import Citation
from .interfaces import RelevanceJudge


@dataclass(frozen=True)
class CitationVerdict:
    cite: Citation
    file_ok: bool
    line_ok: bool
    symbol_ok: bool
    relevant: bool
    # R3 three-way classification additions — both default to the pre-fix shape
    # (ambiguous=False, resolve_layer="") so every pre-existing construction of this
    # dataclass (dozens across the test suite) is byte-for-byte unaffected.
    ambiguous: bool = False   # True <=> the resolver returned AMBIGUOUS (real line,
                              # >=2 candidate files, unpinnable) — NEVER a hallucination.
    resolve_layer: str = ""  # R4 resolvability axis: "exact" | "unique_basename" |
                              # "line_range" | "line_symbol" | "line_tokens" |
                              # "llm_disambiguated" | "" (unresolved / unknown legacy path)

    @property
    def is_ambiguous(self) -> bool:
        return self.ambiguous

    @property
    def is_hallucination(self) -> bool:
        """True fabrication ONLY: ABSENT (file/line unresolved) or a resolved file with
        a bad line/symbol. AMBIGUOUS is explicitly excluded — see module docstring."""
        return (not self.ambiguous) and not (self.file_ok and self.line_ok and self.symbol_ok)

    @property
    def is_valid(self) -> bool:
        return not self.ambiguous and not self.is_hallucination and self.relevant


@dataclass(frozen=True)
class InvestigatorReport:
    precision: float        # valid / (cited - ambiguous); ambiguous excluded (spec R3)
    recall: float           # valid / claim_count  (claim-coverage; under-cite -> low)
    hallucinations: int
    verdicts: list[CitationVerdict]
    valid: int = 0          # explicit valid count (R3: "report all three counts")
    ambiguous: int = 0       # explicit ambiguous count, reported SEPARATELY from precision

    @property
    def ambiguous_rate(self) -> float:
        n = len(self.verdicts)
        return (self.ambiguous / n) if n else 0.0


class RelevanceAllTrue:
    def is_relevant(self, cite, issue_text, code: str = ""): return True
class RelevanceNone:
    def is_relevant(self, cite, issue_text, code: str = ""): return False


def _claim_text_for(cite: Citation, text: str) -> str:
    """Best-effort enclosing sentence for a citation — used only as the R1 layer-3
    salient-token tie-break signal and as R2 Q3 disambiguator context. Falls back to ""
    when `text` isn't available (existing callers that don't pass it): the deterministic
    line-range layer alone resolves the large majority of real-world ambiguous-basename
    cases regardless (per the spec's own noqa.rs measurement)."""
    if not text:
        return ""
    from .claims import sentence_spans
    from .citations import iter_citation_occurrences
    spans = sentence_spans(text)
    for start, _end, c in iter_citation_occurrences(text):
        if c == cite:
            return next((s for (a, b, s) in spans if a <= start < b), "")
    return ""


def _resolve(co, cite: Citation, *, claim_text: str, ask) -> tuple[str | None, bool, str]:
    """Returns (resolved_path_or_None, ambiguous, resolve_layer)."""
    if hasattr(co, "resolve_cite"):
        disambiguate_fn = None
        if ask is not None:
            from .disambiguate import make_disambiguator
            disambiguate_fn = make_disambiguator(ask)
        rr = co.resolve_cite(cite.file, cite.line, cite.symbol, claim_text,
                             disambiguate=disambiguate_fn)
        resolved = rr.path if rr.status == "RESOLVED" else None
        return resolved, rr.status == "AMBIGUOUS", rr.layer
    if hasattr(co, "resolve_rel"):
        resolved = co.resolve_rel(cite.file)
        if resolved is None:
            return None, False, ""
        layer = "exact" if resolved == cite.file else "unique_basename"
        return resolved, False, layer
    resolved = cite.file if co.file_exists(cite.file) else None
    return resolved, False, ("exact" if resolved is not None else "")


def verify_citation(co, cite: Citation, *, issue_text: str = "",
                    relevance: RelevanceJudge | None = None,
                    read_code=lambda *_: None,
                    text: str = "", ask=None) -> CitationVerdict:
    claim_text = _claim_text_for(cite, text)
    resolved, ambiguous, resolve_layer = _resolve(co, cite, claim_text=claim_text, ask=ask)
    file_ok = resolved is not None
    line_ok = file_ok and (cite.line is None or co.read_line(resolved, cite.line) is not None)
    symbol_ok = True
    if cite.symbol is not None:
        ln = co.read_line(resolved, cite.line) if (file_ok and cite.line) else None
        symbol_ok = bool(ln and cite.symbol in ln)
    relevant = True
    if relevance is not None and file_ok and line_ok and symbol_ok:
        code = read_code(resolved, cite.line) or ""
        relevant = relevance.is_relevant(cite, issue_text, code)
    return CitationVerdict(cite, file_ok, line_ok, symbol_ok, relevant,
                           ambiguous=ambiguous, resolve_layer=resolve_layer)


def score_citations(co, cites: list[Citation], *, claim_count: int,
                    relevance: RelevanceJudge, issue_text: str = "",
                    read_code=lambda *_: None,
                    text: str = "", ask=None) -> InvestigatorReport:
    verdicts = [verify_citation(co, c, issue_text=issue_text, relevance=relevance,
                                read_code=read_code, text=text, ask=ask) for c in cites]
    valid = sum(v.is_valid for v in verdicts)
    halluc = sum(v.is_hallucination for v in verdicts)
    ambiguous = sum(v.is_ambiguous for v in verdicts)
    # R3: ambiguous is EXCLUDED from the precision denominator (reported separately via
    # InvestigatorReport.ambiguous/ambiguous_rate) — everything else (valid, hallucinated,
    # and resolved-but-irrelevant) stays in the denominator exactly as before, so this is
    # a strict backward-compatible generalization of the old valid/len(verdicts) formula
    # (identical when ambiguous == 0, which is every pre-existing code path/fake).
    denom = len(verdicts) - ambiguous
    precision = valid / denom if denom > 0 else 0.0
    recall = valid / claim_count if claim_count > 0 else 0.0
    return InvestigatorReport(precision, recall, halluc, verdicts,
                              valid=valid, ambiguous=ambiguous)


def resolvability_breakdown(verdicts: list[CitationVerdict]) -> dict:
    """R4 — resolvability axis (resolver-fix-spec.md): per-arm fraction of citations
    that were full-path (exact resolve), bare-resolved (needed the basename/line/
    symbol/token/LLM layers), ambiguous, or absent. Kept STRICTLY separate from
    precision/validity (R3) — this is prism's honest "hands the agent exact resolvable
    paths" edge, a resolvability signal, never a truth signal."""
    n = len(verdicts)
    full_path = sum(1 for v in verdicts if v.resolve_layer == "exact")
    ambiguous = sum(1 for v in verdicts if v.is_ambiguous)
    absent = sum(1 for v in verdicts if not v.file_ok and not v.is_ambiguous)
    bare_resolved = n - full_path - ambiguous - absent

    def _rate(k: int) -> float:
        return (k / n) if n else 0.0

    return {
        "n": n,
        "full_path": full_path, "bare_resolved": bare_resolved,
        "ambiguous": ambiguous, "absent": absent,
        "full_path_rate": _rate(full_path), "bare_resolved_rate": _rate(bare_resolved),
        "ambiguous_rate": _rate(ambiguous), "absent_rate": _rate(absent),
    }
