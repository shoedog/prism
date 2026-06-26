# eval/adoption/metrics.py
"""Deterministic metrics carry the pass^5 signal (no LLM cost). SkillActivation reads the
skill-load flag stashed in metadata; ToolCorrectness (deepeval) compares the
nav tools fired vs expected. ArgumentCorrectness/TaskCompletion (LLM-judge) are quality-only
and added later, not part of the v1 gate."""
from __future__ import annotations
from deepeval.metrics import BaseMetric, ToolCorrectnessMetric
from deepeval.test_case import LLMTestCase

class SkillActivationMetric(BaseMetric):
    """1.0 iff the prism-nav skill loaded in this trajectory (deterministic)."""
    def __init__(self, threshold: float = 1.0):
        self.threshold = threshold
        self.score = 0.0
        self.reason = ""
        self.success = False
    def measure(self, test_case: LLMTestCase) -> float:
        loaded = bool((test_case.metadata or {}).get("prism_skill_loaded"))
        self.score = 1.0 if loaded else 0.0
        self.reason = "prism-nav skill loaded" if loaded else "prism-nav skill did not load"
        self.success = self.score >= self.threshold
        return self.score
    async def a_measure(self, test_case: LLMTestCase, *args, **kwargs) -> float:
        return self.measure(test_case)
    def is_successful(self) -> bool:
        return self.success
    @property
    def __name__(self):
        return "SkillActivation"

# The two deterministic gate metrics (no model credentials needed).
GATE_METRICS = [SkillActivationMetric(), ToolCorrectnessMetric()]
