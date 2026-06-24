"""Model-output parsers (Phase-1b). Pure functions over the CLIs' JSON so they are
fully unit-tested; the subprocess spawn (arm_runner) is the only un-tested seam.
claude: `--output-format json` single object. codex: `--json` JSONL events."""
from __future__ import annotations
import json
from dataclasses import dataclass

@dataclass(frozen=True)
class ModelResult:
    text: str
    input_tokens: int
    output_tokens: int
    tool_calls: int
    cost_usd: float

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
    )

# codex --json event item types that count as a tool call (verify against live output, Task 2 Step 5):
_CODEX_TOOL_ITEMS = {"command_execution", "mcp_tool_call", "file_change", "web_search"}

def parse_codex_jsonl(out: str) -> ModelResult:
    text, inp, outp, tools = "", 0, 0, 0
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
        u = ev.get("usage") or {}
        if u:
            inp = int(u.get("input_tokens", inp))
            outp = int(u.get("output_tokens", outp))
    if not text:
        raise ValueError("codex run produced no agent_message")
    return ModelResult(text=text, input_tokens=inp, output_tokens=outp, tool_calls=tools, cost_usd=0.0)
