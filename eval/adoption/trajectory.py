# eval/adoption/trajectory.py
"""Parse `claude -p --output-format stream-json` JSONL into a Trajectory. mcp__prism__nav_X
tool names are normalised to bare `nav_X` so probes match on the nav verb."""
from __future__ import annotations
import json
from .model import Trajectory

def _norm(name: str) -> str:
    # mcp__prism__nav_callers -> nav_callers ; leave builtins (Bash/Read/Skill) as-is
    return name.split("__")[-1] if name.startswith("mcp__prism__") else name

def parse_stream_json(out: str) -> Trajectory:
    final_text, skill_loads, calls = "", [], []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue
        if r.get("type") != "assistant":
            continue
        for c in r.get("message", {}).get("content", []) or []:
            if not isinstance(c, dict):
                continue
            if c.get("type") == "text" and c.get("text", "").strip():
                final_text = c["text"]
            elif c.get("type") == "tool_use":
                name = c.get("name", "")
                inp = c.get("input", {}) or {}
                if name == "Skill":
                    skill_loads.append(str(inp.get("skill", "")))
                else:
                    calls.append((_norm(name), inp))
    return Trajectory(final_text=final_text, skill_loads=skill_loads, tool_calls=calls)
