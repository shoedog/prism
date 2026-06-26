# eval/adoption/model.py
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class Probe:
    id: str
    kind: str                  # "nav" | "negative"
    prompt: str
    repo: str                  # path (relative to eval/) of the small target repo
    expected_tools: list[str]  # bare nav names, e.g. ["nav_callers"]; [] for negatives
    expected_symbol: str | None = None

@dataclass(frozen=True)
class Trajectory:
    final_text: str
    skill_loads: list[str]               # skill names loaded via the Skill tool
    tool_calls: list[tuple[str, dict]]   # (bare_or_builtin_name, input)
    def prism_nav_calls(self) -> list[str]:
        return [n for n, _ in self.tool_calls if n.startswith("nav_")]
    def loaded_prism_skill(self) -> bool:
        return any("prism" in s.lower() for s in self.skill_loads)
