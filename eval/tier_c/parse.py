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
    prism_count = 0
    distinct: set[str] = set()
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        # Usage lives on outer turn events (e.g. turn.completed); read it regardless of event type.
        u = ev.get("usage") or {}
        if u:
            inp = int(u.get("input_tokens", inp))
            outp = int(u.get("output_tokens", outp))
        # Each item is emitted twice by codex --json: once under item.started and once under
        # item.completed.  Gate on item.completed to count every item exactly once.
        if ev.get("type") != "item.completed":
            continue
        item = ev.get("item") or {}
        if item.get("type") == "agent_message" and item.get("text"):
            text = item["text"]              # last agent message wins
        if item.get("type") in _CODEX_TOOL_ITEMS:
            tools += 1
        if item.get("type") == "mcp_tool_call" and item.get("server") == "prism":
            prism_count += 1
            tool_name = item.get("tool") or item.get("name") or ""
            if tool_name:
                distinct.add(tool_name)
        if item.get("type") == "command_execution":
            cmd_str = item.get("command") or item.get("cmd") or ""
            if cmd_str:
                commands.append(cmd_str)
    if not text:
        raise ValueError("codex run produced no agent_message")
    dose = Dose(count=prism_count, distinct_tools=frozenset(distinct), errors=0)
    return ModelResult(text=text, input_tokens=inp, output_tokens=outp, tool_calls=tools,
                       cost_usd=0.0, commands=commands, prism_calls=prism_count, dose=dose)


def _norm_prism(name: str) -> str:
    """mcp__prism__nav_callers -> nav_callers; non-prism names returned unchanged."""
    return name.split("__")[-1] if name.startswith("mcp__prism__") else name


def parse_claude_stream_json(out: str) -> ModelResult:
    """Parse ``claude -p --output-format stream-json`` newline-delimited output.

    Counts only real prism tool calls (``tool_use`` entries whose ``name``
    starts with ``mcp__prism__``), collects distinct bare nav tool names, counts
    ``tool_result`` entries with ``is_error: true``, and captures the final
    assistant text.  Also reads the final ``type=="result"`` event to capture
    ``input_tokens``, ``output_tokens``, and ``total_cost_usd`` — mirroring the
    fields that ``parse_claude_json`` provides so switching parsers loses nothing.
    Returns a :class:`ModelResult` with ``prism_calls`` and ``dose`` populated.
    """
    final_text = ""
    prism_count = 0
    distinct: set[str] = set()
    errors = 0
    input_tokens = 0
    output_tokens = 0
    cost_usd = 0.0

    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue

        msg_type = r.get("type")

        # Read usage + cost from the final result event (same fields as parse_claude_json).
        if msg_type == "result":
            u = r.get("usage") or {}
            input_tokens = int(u.get("input_tokens", input_tokens))
            output_tokens = int(u.get("output_tokens", output_tokens))
            cost_usd = float(r.get("total_cost_usd", cost_usd))
            # The result event may also carry the final text; only use it if we have
            # not already captured an assistant text block (assistant text blocks win).
            if not final_text:
                result_text = r.get("result") or ""
                if result_text.strip():
                    final_text = result_text
            continue

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
        input_tokens=input_tokens,
        output_tokens=output_tokens,
        tool_calls=prism_count,  # for stream-json we have exact prism counts
        cost_usd=cost_usd,
        prism_calls=prism_count,
        dose=dose,
    )
