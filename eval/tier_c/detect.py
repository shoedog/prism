"""Detectability test (spec §6b): a ConditionGuesser predicts prism on/off from each output; if it
beats chance (exact-binomial p<0.05), the judge prism-delta is INVALID and we lean on objective."""
from __future__ import annotations
from dataclasses import dataclass
from .judges import detectability_pvalue

@dataclass(frozen=True)
class Detectability:
    correct: int
    n: int
    pvalue: float
    detectable: bool

def run_detectability(outputs, guesser, alpha: float = 0.1) -> Detectability:
    n = len(outputs)
    correct = sum(1 for o in outputs if guesser.guess_used_prism(o.text) == o.variant.prism)
    p = detectability_pvalue(correct, n)
    return Detectability(correct=correct, n=n, pvalue=p, detectable=(p < alpha))
