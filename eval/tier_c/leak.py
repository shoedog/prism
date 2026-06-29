"""Leak scanner — detects prism/nav tool names in model output (Task 10).

If a model's spec or plan text names prism or nav_* tools, the rank-judge
blinding breaks because the judge can infer which arm was prism-on.
scan_leak() detects and redacts such tokens before the text is judged.
"""
from __future__ import annotations
import re
from dataclasses import dataclass

# Matches "prism" or "nav_<word>" case-insensitively, on word boundaries.
_LEAK_RE = re.compile(r"(?i)\b(prism|nav_[a-z_]+)\b")


@dataclass(frozen=True)
class LeakResult:
    """Result of scanning one text for tool-name leaks."""
    leaked: bool
    redacted: str


def scan_leak(text: str) -> LeakResult:
    """Return a LeakResult indicating whether tool names were found and the redacted text.

    Any occurrence of 'prism' or 'nav_<word>' (case-insensitive, whole-word) is
    replaced with '[tool]' in the redacted output.
    """
    redacted, n = _LEAK_RE.subn("[tool]", text)
    return LeakResult(leaked=n > 0, redacted=redacted)
