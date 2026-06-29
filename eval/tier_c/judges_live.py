"""LLM-backed judges (Phase-1c), all behind the ask() seam so they're unit-tested with fakes.
RankJudge ranks anonymized candidates (style-neutral); RelevanceJudge audits a citation;
ConditionGuesser powers the detectability test. None of them get prism (ask() is tool-free).

_RecordingRelevanceJudge wraps LlmRelevanceJudge and appends a record per cite to a caller-
supplied list — {file, line, symbol, verdict, escalated, votes, relevant} — for audit persistence."""
from __future__ import annotations
import re
from .model import Citation

_RANK_INSTR = ("Rank the candidates best-to-worst for this {stage} task on the rubric: {rubric}. "
               "IGNORE citation formatting/volume — judge substance only. "
               "Respond with ONLY the candidate ids in order, best first, comma-separated.")

class LlmRankJudge:
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def rank(self, stage: str, rubric: str, candidates: dict[str, str]) -> list[str]:
        body = "\n\n".join(f"[{lbl}]\n{txt}" for lbl, txt in candidates.items())
        prompt = _RANK_INSTR.format(stage=stage, rubric=rubric) + "\n\n" + body
        raw = self.ask(self.model, prompt)
        # candidate labels are chain.py's opaque cand{i} scheme; unknown/extra labels are filtered, omitted ones appended -> always a full permutation
        found = [t for t in re.findall(r"cand\d+", raw)]
        seen, order = set(), []
        for c in found:
            if c in candidates and c not in seen:
                seen.add(c); order.append(c)
        for c in candidates:                       # repair: append any omitted in input order
            if c not in seen:
                order.append(c)
        return order

class LlmRelevanceJudge:
    """Per-citation relevance via the 2-sonnet/opus ensemble (ensemble.py)."""
    def __init__(self, ask, model: str | None = None, *, opus: str | None = None):
        from .llm import JUDGE_MODEL, JUDGE_TIEBREAKER
        self.ask = ask
        self.sonnet = model or JUDGE_MODEL
        self.opus = opus or JUDGE_TIEBREAKER

    def relevance(self, cite: Citation, issue_text: str, code: str = ""):
        from .ensemble import ensemble
        code_section = f"\n\nCode at {cite.file}:{cite.line}:\n{code}" if code else ""
        prompt = (f"Issue:\n{issue_text}{code_section}\n\nIs the code at {cite.file}:{cite.line} "
                  f"(symbol {cite.symbol}) actually relevant to fixing this issue? "
                  f"Start your reply with YES or NO, then one sentence why.")
        return ensemble(self.ask, prompt, ("YES", "NO"),
                        sonnet=self.sonnet, opus=self.opus, default="NO")

    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        return self.relevance(cite, issue_text, code).verdict == "YES"

class LlmConditionGuesser:
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def guess_used_prism(self, text: str) -> bool:
        prompt = ("Below is an output from a coding task. Was a code-navigation tool that yields exact "
                  "file:line/call-graph facts likely USED to produce it? Answer with exactly YES or NO and nothing else.\n\n" + text)
        # conservative: any non-YES (incl. hedged) reads False
        return self.ask(self.model, prompt).strip().upper().startswith("YES")


class _RecordingRelevanceJudge:
    """Wraps LlmRelevanceJudge; appends one record per cite to *records*:
    {file, line, symbol, verdict, escalated, votes, relevant}."""
    def __init__(self, inner: "LlmRelevanceJudge", records: list):
        self.inner, self.records = inner, records

    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        ev = self.inner.relevance(cite, issue_text, code)
        self.records.append({
            "file": cite.file, "line": cite.line, "symbol": cite.symbol,
            "verdict": ev.verdict, "escalated": ev.escalated, "votes": ev.votes,
            "relevant": ev.verdict == "YES",
        })
        return ev.verdict == "YES"
