"""Independent citation oracle (spec §6a, codex new-2/new-3): mechanical existence via
neutral file/git primitives — NEVER prism — plus a SECONDARY relevance seam. Scores
citation PRECISION (real & relevant) and RECALL/claim-coverage (under-citing penalized)."""
from __future__ import annotations
from dataclasses import dataclass
from .model import Citation
from .interfaces import RelevanceJudge

@dataclass(frozen=True)
class CitationVerdict:
    cite: Citation
    file_ok: bool
    line_ok: bool
    symbol_ok: bool
    relevant: bool
    @property
    def is_hallucination(self) -> bool:
        return not (self.file_ok and self.line_ok and self.symbol_ok)
    @property
    def is_valid(self) -> bool:
        return not self.is_hallucination and self.relevant

@dataclass(frozen=True)
class InvestigatorReport:
    precision: float        # valid / cited
    recall: float           # valid / claim_count  (claim-coverage; under-cite -> low)
    hallucinations: int
    verdicts: list[CitationVerdict]

class RelevanceAllTrue:
    def is_relevant(self, cite, issue_text): return True
class RelevanceNone:
    def is_relevant(self, cite, issue_text): return False

def verify_citation(co, cite: Citation, *, issue_text: str = "",
                    relevance: RelevanceJudge | None = None) -> CitationVerdict:
    file_ok = co.file_exists(cite.file)
    line_ok = file_ok and (cite.line is None or co.read_line(cite.file, cite.line) is not None)
    symbol_ok = True
    if cite.symbol is not None:
        ln = co.read_line(cite.file, cite.line) if (file_ok and cite.line) else None
        symbol_ok = bool(ln and cite.symbol in ln)
    relevant = True
    if relevance is not None and file_ok and line_ok and symbol_ok:
        relevant = relevance.is_relevant(cite, issue_text)
    return CitationVerdict(cite, file_ok, line_ok, symbol_ok, relevant)

def score_citations(co, cites: list[Citation], *, claim_count: int,
                    relevance: RelevanceJudge, issue_text: str = "") -> InvestigatorReport:
    verdicts = [verify_citation(co, c, issue_text=issue_text, relevance=relevance) for c in cites]
    valid = sum(v.is_valid for v in verdicts)
    halluc = sum(v.is_hallucination for v in verdicts)
    precision = valid / len(verdicts) if verdicts else 0.0
    recall = valid / claim_count if claim_count > 0 else 0.0
    return InvestigatorReport(precision, recall, halluc, verdicts)
