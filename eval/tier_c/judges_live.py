"""LLM-backed judges (Phase-1c), all behind the ask() seam so they're unit-tested with fakes.
RankJudge ranks anonymized candidates (style-neutral); RelevanceJudge audits a citation;
ConditionGuesser powers the detectability test. None of them get prism (ask() is tool-free)."""
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
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def is_relevant(self, cite: Citation, issue_text: str) -> bool:
        prompt = (f"Issue:\n{issue_text}\n\nIs the code at {cite.file}:{cite.line} "
                  f"(symbol {cite.symbol}) actually relevant to fixing this issue? Answer YES or NO.")
        return self.ask(self.model, prompt).strip().upper().startswith("YES")

class LlmConditionGuesser:
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def guess_used_prism(self, text: str) -> bool:
        prompt = ("Below is an output from a coding task. Was a code-navigation tool that yields exact "
                  "file:line/call-graph facts likely USED to produce it? Answer YES or NO.\n\n" + text)
        return self.ask(self.model, prompt).strip().upper().startswith("YES")
