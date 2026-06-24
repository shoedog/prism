"""Detectability test (spec §6b): a ConditionGuesser predicts prism on/off from each output; if it
beats chance (exact-binomial p<0.05), the judge prism-delta is INVALID and we lean on objective.

Note: detectability is meaningful only over **pooled** outputs (across issues × stages). A single
stage's 4 outputs cannot reach p<0.05 (max separation p=0.0625 > 0.05), so callers must pool
across multiple issues and stages before interpreting the result. The --live loop (Task 5) is
responsible for collecting and pooling outputs before calling this function."""
from __future__ import annotations
from dataclasses import dataclass
from .judges import detectability_pvalue

@dataclass(frozen=True)
class Detectability:
    correct: int
    n: int
    pvalue: float
    detectable: bool

def run_detectability(outputs, guesser, alpha: float = 0.05) -> Detectability:
    n = len(outputs)
    correct = sum(1 for o in outputs if guesser.guess_used_prism(o.text) == o.variant.prism)
    p = detectability_pvalue(correct, n)
    return Detectability(correct=correct, n=n, pvalue=p, detectable=(p < alpha))
