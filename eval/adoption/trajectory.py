# eval/adoption/trajectory.py
"""Parse model-specific JSONL streams into a Trajectory.

- parse_stream_json: claude -p --output-format stream-json (existing; untouched)
- parse_codex_stream: codex exec --json 0.141.0 schema

For codex, mcp_tool_call items with server=="prism" are normalised to bare
nav_X tool names.  command_execution items become ("Bash", {}).
Skill loading is detected by any command_execution whose command references
a path containing "prism-code-navigation/SKILL.md" or "prism-nav/SKILL.md"
(the latter covers the user's knowledge-ref path seen in existing fixtures).
"""
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


# ---------------------------------------------------------------------------
# codex exec --json  (codex-cli 0.141.0)
# ---------------------------------------------------------------------------
# Schema: newline-delimited JSON, each line is an event.
# Relevant events:
#   {"type":"item.completed","item":{"type":"agent_message","text":"..."}}
#   {"type":"item.completed","item":{"type":"command_execution","command":"...","aggregated_output":"..."}}
#   {"type":"item.completed","item":{"type":"mcp_tool_call","server":"prism","tool":"nav_X","arguments":{...},"result":{...}}}
#
# Normalisation:
#   - mcp_tool_call with server=="prism" → (tool, arguments) where tool is already bare nav_X
#   - command_execution → ("Bash", {})
#   - skill detection: command_execution.command contains "prism-code-navigation/SKILL.md"
#     or "prism-nav/SKILL.md" (covers knowledge-ref path in existing fixtures)

_SKILL_SENTINELS = ("prism-code-navigation/SKILL.md", "prism-nav/SKILL.md")


def parse_codex_stream(out: str) -> Trajectory:
    """Parse `codex exec --json` JSONL output into a Trajectory."""
    final_text, skill_loads, calls = "", [], []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue
        if r.get("type") != "item.completed":
            continue
        item = r.get("item") or {}
        kind = item.get("type", "")

        if kind == "agent_message":
            text = item.get("text", "")
            if text.strip():
                final_text = text

        elif kind == "command_execution":
            cmd = item.get("command", "")
            calls.append(("Bash", {}))
            # Detect skill read: any exec that reads a prism SKILL.md
            if any(s in cmd for s in _SKILL_SENTINELS):
                # Extract skill name from path: .../skills/<name>/SKILL.md
                # e.g. ".../prism-code-navigation/SKILL.md" → "prism-code-navigation"
                # or ".../prism-nav/SKILL.md" → "prism-nav"
                for sentinel in _SKILL_SENTINELS:
                    if sentinel in cmd:
                        # basename of the directory before /SKILL.md
                        skill_name = sentinel.split("/")[-2]  # e.g. "prism-code-navigation"
                        if skill_name not in skill_loads:
                            skill_loads.append(skill_name)
                        break

        elif kind == "mcp_tool_call":
            if item.get("server") == "prism":
                tool = item.get("tool", "")
                args = item.get("arguments") or {}
                calls.append((tool, args))

    return Trajectory(final_text=final_text, skill_loads=skill_loads, tool_calls=calls)
