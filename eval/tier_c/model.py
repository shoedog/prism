"""Tier-C schemas (spec 2026-06-23 rev-3). Frozen dataclasses; files repo-relative POSIX, lines 1-based."""
from __future__ import annotations
from dataclasses import dataclass

_ANTHROPIC = {"opus-4.8", "sonnet-4.6"}
_OPENAI = {"gpt-5.5", "gpt-5.3-spark"}

@dataclass(frozen=True, order=True)
class Variant:
    model: str
    prism: bool
    lsp: bool = False
    @property
    def id(self) -> str:
        return f"{self.model}{'+prism' if self.prism else ''}{'+lsp' if self.lsp else ''}"
    @property
    def family(self) -> str:
        if self.model in _ANTHROPIC: return "anthropic"
        if self.model in _OPENAI: return "openai"
        return "unknown"

@dataclass(frozen=True)
class Citation:
    file: str
    line: int | None
    symbol: str | None

@dataclass(frozen=True)
class Issue:
    key: str            # e.g. "ripgrep-12345"
    language: str       # rust|go|python|js|ts
    repo: str           # local path under bench-repos
    sha: str            # pinned commit (issue still OPEN here)
    url: str
    text: str           # issue body snapshot
    scoped_slice: str   # the first-slice scope statement

@dataclass(frozen=True)
class ArmOutput:
    variant: Variant
    text: str
    citations: list[Citation]
    tokens: int
    tool_calls: int
    wall_s: float
    used_prism: bool
