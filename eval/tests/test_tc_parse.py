import json
import pytest
from tier_c.parse import parse_claude_json, parse_codex_jsonl, parse_claude_stream_json

CLAUDE = json.dumps({"type":"result","is_error":False,"num_turns":3,"result":"spec sees src/a.py:1",
                     "total_cost_usd":0.05,"usage":{"input_tokens":10,"output_tokens":42}})

def test_parse_claude_extracts_text_tokens_cost_turns():
    r = parse_claude_json(CLAUDE)
    assert r.text == "spec sees src/a.py:1"
    assert r.output_tokens == 42 and r.input_tokens == 10
    assert abs(r.cost_usd - 0.05) < 1e-9
    assert r.tool_calls == 2  # num_turns - 1 (best-effort proxy; stream-json needed for exact)

def test_parse_claude_error_raises():
    import pytest
    with pytest.raises(ValueError, match="claude"):
        parse_claude_json(json.dumps({"type":"result","is_error":True,"result":None,"usage":{}}))

def test_parse_codex_jsonl_picks_last_message_and_sums_tokens():
    lines = "\n".join([
        json.dumps({"type":"item.completed","item":{"type":"reasoning"}}),
        json.dumps({"type":"item.completed","item":{"type":"command_execution"}}),  # a tool call
        json.dumps({"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":20}}),
        json.dumps({"type":"item.completed","item":{"type":"agent_message","text":"plan: src/b.go:9"}}),
    ])
    r = parse_codex_jsonl(lines)
    assert r.text == "plan: src/b.go:9"
    assert r.output_tokens == 20
    assert r.tool_calls == 1


# ---------------------------------------------------------------------------
# parse_claude_stream_json tests
# ---------------------------------------------------------------------------

# Two real prism calls (mcp__prism__*), one non-prism tool call (Bash),
# one tool_result with is_error=True, and a final assistant text block.
SAMPLE = "\n".join([
    # prism call 1: nav_callers
    json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": "mcp__prism__nav_callers", "input": {"symbol": "run"}},
    ]}}),
    # non-prism tool (should NOT be counted as prism)
    json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": "Bash", "input": {"command": "ls"}},
    ]}}),
    # prism call 2: nav_repo_map
    json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": "mcp__prism__nav_repo_map", "input": {}},
    ]}}),
    # tool_result with is_error=True (error from one of the above tool calls)
    json.dumps({"type": "user", "message": {"content": [
        {"type": "tool_result", "is_error": True, "content": "tool failed"},
    ]}}),
    # final assistant text
    json.dumps({"type": "assistant", "message": {"content": [
        {"type": "text", "text": "Found 3 callers in src/main.rs"},
    ]}}),
])

def test_stream_json_counts_real_prism_calls_and_dose():
    r = parse_claude_stream_json(SAMPLE)
    assert r.prism_calls == 2                    # only mcp__prism__* tool_use
    assert r.dose.distinct_tools == {"nav_callers", "nav_repo_map"}
    assert r.dose.errors == 1
    assert r.text.strip()                        # final assistant text captured

def test_stream_json_non_prism_tool_not_counted():
    # Only a Bash call — no prism calls, no errors
    stream = json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": "Bash", "input": {"command": "cargo build"}},
    ]}})
    r = parse_claude_stream_json(stream)
    assert r.prism_calls == 0
    assert r.dose.distinct_tools == set()
    assert r.dose.errors == 0

def test_stream_json_zero_prism_input():
    # Plain text output, no tool calls
    stream = "\n".join([
        json.dumps({"type": "assistant", "message": {"content": [
            {"type": "text", "text": "No prism calls needed."},
        ]}}),
    ])
    r = parse_claude_stream_json(stream)
    assert r.prism_calls == 0
    assert r.dose.count == 0
    assert r.text.strip() == "No prism calls needed."


# ---------------------------------------------------------------------------
# codex double-count gate (FIX 1) — item.started must NOT be counted
# ---------------------------------------------------------------------------

def _codex_double_event(item: dict) -> str:
    """Build a JSONL pair: item.started + item.completed for the same item.
    Real codex --json emits each item twice; only item.completed must count."""
    started = json.dumps({"type": "item.started", "item": item})
    completed = json.dumps({"type": "item.completed", "item": item})
    return started + "\n" + completed


def test_codex_double_event_counts_prism_call_once():
    """Each item appears under item.started AND item.completed; prism_calls must be 1, not 2."""
    prism_item = {"type": "mcp_tool_call", "server": "prism", "tool": "nav_repo_map"}
    agent_msg = {"type": "item.completed", "item": {"type": "agent_message", "text": "done: src/a.go:1"}}
    lines = _codex_double_event(prism_item) + "\n" + json.dumps(agent_msg)
    r = parse_codex_jsonl(lines)
    assert r.prism_calls == 1, f"expected 1 prism_call (not 2); got {r.prism_calls}"
    assert r.dose.count == 1


def test_codex_double_event_counts_command_once():
    """command_execution under item.started + item.completed must count as 1 tool_call."""
    cmd_item = {"type": "command_execution", "command": "cargo build"}
    agent_msg = {"type": "item.completed", "item": {"type": "agent_message", "text": "built: src/b.rs:5"}}
    lines = _codex_double_event(cmd_item) + "\n" + json.dumps(agent_msg)
    r = parse_codex_jsonl(lines)
    assert r.tool_calls == 1, f"expected 1 tool_call (not 2); got {r.tool_calls}"
    assert r.commands == ["cargo build"]
