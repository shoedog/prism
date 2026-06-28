"""Tier-C schemas (spec 2026-06-23 rev-3). Frozen dataclasses; files repo-relative POSIX, lines 1-based."""
from __future__ import annotations
from dataclasses import dataclass, field


@dataclass(frozen=True)
class Dose:
    """Prism invocation measurement for one model run."""
    count: int = 0
    distinct_tools: frozenset[str] = field(default_factory=frozenset)
    errors: int = 0

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
    tokens: int          # output tokens
    tool_calls: int
    wall_s: float
    used_prism: bool
    commands: list[str] = field(default_factory=list)
    lsp_leak: bool = False
    compiler_assisted: bool = False
    prism_calls: int = 0
    dose: Dose = field(default_factory=Dose)
    low_dose: bool = False
    in_tokens: int = 0   # input tokens (default 0 so existing constructions keep working)
    cost_usd: float = 0.0
    raw_stdout: str = ""  # full proc.stdout from the model subprocess; "" for fakes/tests
