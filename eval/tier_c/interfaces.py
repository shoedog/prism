"""Model-driving + relevance seams (spec §6). Orchestration/scoring depend ONLY on
these, so fakes drive all tests and the live drivers swap in behind them."""
from __future__ import annotations
from typing import Protocol, runtime_checkable
from .model import Citation, Variant, ArmOutput

@runtime_checkable
class RelevanceJudge(Protocol):
    """Secondary, audit-sampled relevance call (spec §6a) — NEVER prism-backed."""
    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool: ...

@runtime_checkable
class ArmRunner(Protocol):
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput: ...

@runtime_checkable
class RankJudge(Protocol):
    """Returns a full ranking (best-first) of anonymized candidate ids."""
    def rank(self, stage: str, rubric: str, candidates: dict[str, str]) -> list[str]: ...
