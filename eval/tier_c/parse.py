"""Model-output parsers (Phase-1b). Pure functions over the CLIs' JSON so they are
fully unit-tested; the subprocess spawn (arm_runner) is the only un-tested seam.
claude: `--output-format json` single object. codex: `--json` JSONL events.
claude: `--output-format stream-json` newline-delimited JSON (parse_claude_stream_json)."""
from __future__ import annotations
import json
from dataclasses import dataclass, field
from .model import Dose

@dataclass(frozen=True)
class ModelResult:
    text: str
    input_tokens: int
    output_tokens: int
    tool_calls: int
    cost_usd: float
    commands: list[str] = field(default_factory=list)
    prism_calls: int = 0
    dose: Dose = field(default_factory=Dose)

def parse_claude_json(out: str) -> ModelResult:
    d = json.loads(out)
    if d.get("is_error") or not d.get("result"):
        raise ValueError(f"claude run failed: {d.get('subtype') or d.get('api_error_status')}")
    u = d.get("usage", {})
    return ModelResult(
        text=d["result"],
        input_tokens=int(u.get("input_tokens", 0)),
        output_tokens=int(u.get("output_tokens", 0)),
        tool_calls=max(0, int(d.get("num_turns", 1)) - 1),  # best-effort; exact needs stream-json
        cost_usd=float(d.get("total_cost_usd", 0.0)),
        commands=[],  # claude full per-command capture = follow-up (stream-json needed)
    )

# codex --json event item types that count as a tool call (verify against live output, Task 2 Step 5):
_CODEX_TOOL_ITEMS = {"command_execution", "mcp_tool_call", "file_change", "web_search"}

def parse_codex_jsonl(out: str) -> ModelResult:
    text, inp, outp, tools = "", 0, 0, 0
    commands: list[str] = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        item = ev.get("item") or {}
        if item.get("type") == "agent_message" and item.get("text"):
            text = item["text"]              # last agent message wins
        if item.get("type") in _CODEX_TOOL_ITEMS:
            tools += 1
        if item.get("type") == "command_execution":
            cmd_str = item.get("command") or item.get("cmd") or ""
            if cmd_str:
                commands.append(cmd_str)
        u = ev.get("usage") or {}
        if u:
            inp = int(u.get("input_tokens", inp))
            outp = int(u.get("output_tokens", outp))
    if not text:
        raise ValueError("codex run produced no agent_message")
    return ModelResult(text=text, input_tokens=inp, output_tokens=outp, tool_calls=tools,
                       cost_usd=0.0, commands=commands)


def _norm_prism(name: str) -> str:
    """mcp__prism__nav_callers -> nav_callers; non-prism names returned unchanged."""
    return name.split("__")[-1] if name.startswith("mcp__prism__") else name


def parse_claude_stream_json(out: str) -> ModelResult:
    """Parse ``claude -p --output-format stream-json`` newline-delimited output.

    Counts only real prism tool calls (``tool_use`` entries whose ``name``
    starts with ``mcp__prism__``), collects distinct bare nav tool names, counts
    ``tool_result`` entries with ``is_error: true``, and captures the final
    assistant text.  Returns a :class:`ModelResult` with ``prism_calls`` and
    ``dose`` populated.
    """
    final_text = ""
    prism_count = 0
    distinct: set[str] = set()
    errors = 0

    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue

        msg_type = r.get("type")
        content = r.get("message", {}).get("content") or []

        for c in content:
            if not isinstance(c, dict):
                continue
            c_type = c.get("type")

            if c_type == "tool_use":
                name = c.get("name", "")
                if name.startswith("mcp__prism__"):
                    prism_count += 1
                    distinct.add(_norm_prism(name))

            elif c_type == "text":
                text = c.get("text", "")
                if text.strip():
                    final_text = text  # last non-empty assistant text wins

            elif c_type == "tool_result":
                if c.get("is_error"):
                    errors += 1

    dose = Dose(count=prism_count, distinct_tools=frozenset(distinct), errors=errors)
    return ModelResult(
        text=final_text,
        input_tokens=0,
        output_tokens=0,
        tool_calls=prism_count,  # for stream-json we have exact prism counts
        cost_usd=0.0,
        prism_calls=prism_count,
        dose=dose,
    )
