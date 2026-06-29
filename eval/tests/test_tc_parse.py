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


# ---------------------------------------------------------------------------
# Tool telemetry: all_tools histogram + prism gate semantics intact
# ---------------------------------------------------------------------------

def _stream_with_tools(*tool_names, text="result text") -> str:
    """Build a minimal claude stream-json fixture with the given tool_use names."""
    lines = []
    for name in tool_names:
        lines.append(json.dumps({"type": "assistant", "message": {"content": [
            {"type": "tool_use", "name": name, "input": {}},
        ]}}))
    lines.append(json.dumps({"type": "assistant", "message": {"content": [
        {"type": "text", "text": text},
    ]}}))
    return "\n".join(lines)


def test_stream_json_all_tools_histogram_includes_all_tool_names():
    """parse_claude_stream_json all_tools must include ALL tool names (prism + non-prism)."""
    stream = _stream_with_tools(
        "mcp__prism__nav_callers",
        "Bash",
        "Read",
        "mcp__prism__nav_callees",
    )
    r = parse_claude_stream_json(stream)
    assert hasattr(r, "all_tools"), "ModelResult must have all_tools attribute"
    assert isinstance(r.all_tools, dict), "all_tools must be a dict (name→count)"
    # All four tool names must be counted
    assert r.all_tools.get("mcp__prism__nav_callers", 0) >= 1
    assert r.all_tools.get("Bash", 0) >= 1
    assert r.all_tools.get("Read", 0) >= 1
    assert r.all_tools.get("mcp__prism__nav_callees", 0) >= 1


def test_stream_json_prism_calls_still_counts_only_prism():
    """prism_calls must remain prism-only; all_tools captures the full histogram."""
    stream = _stream_with_tools(
        "mcp__prism__nav_callers",   # prism
        "Bash",                       # non-prism
        "Read",                       # non-prism
    )
    r = parse_claude_stream_json(stream)
    assert r.prism_calls == 1, f"prism_calls must be 1 (only mcp__prism__*); got {r.prism_calls}"
    assert r.dose.count == 1, "dose.count must equal prism_calls"
    # tool_calls is now total
    assert r.tool_calls == 3, f"tool_calls must be total (3); got {r.tool_calls}"
    # all_tools histogram
    assert sum(r.all_tools.values()) == 3


def test_stream_json_tool_calls_total_when_no_prism():
    """Off-arm (no prism): tool_calls counts all non-prism tools; prism_calls stays 0."""
    stream = _stream_with_tools("Read", "Bash", "Grep")
    r = parse_claude_stream_json(stream)
    assert r.prism_calls == 0
    assert r.dose.count == 0
    assert r.tool_calls == 3, f"tool_calls must be 3 (total non-prism); got {r.tool_calls}"
    assert r.all_tools == {"Read": 1, "Bash": 1, "Grep": 1}


def test_stream_json_dose_gate_semantics_unchanged():
    """Gate semantics: dose.count == prism_calls (unchanged by all_tools addition)."""
    stream = _stream_with_tools(
        "mcp__prism__nav_callers",
        "mcp__prism__nav_repo_map",
        "Bash",
    )
    r = parse_claude_stream_json(stream)
    assert r.dose.count == 2   # prism only
    assert r.prism_calls == 2  # prism only
    assert r.tool_calls == 3   # total
    assert "nav_callers" in r.dose.distinct_tools
    assert "nav_repo_map" in r.dose.distinct_tools


def test_stream_json_repeated_tool_name_counted_in_histogram():
    """A tool called twice must appear with count=2 in all_tools."""
    stream = _stream_with_tools("Bash", "Bash", "Read")
    r = parse_claude_stream_json(stream)
    assert r.all_tools.get("Bash") == 2
    assert r.all_tools.get("Read") == 1
    assert r.tool_calls == 3


def test_codex_all_tools_histogram():
    """parse_codex_jsonl must produce an all_tools histogram for codex events."""
    lines = "\n".join([
        json.dumps({"type": "item.completed", "item": {"type": "command_execution", "command": "ls"}}),
        json.dumps({"type": "item.completed", "item": {"type": "mcp_tool_call", "server": "prism", "tool": "nav_callers"}}),
        json.dumps({"type": "item.completed", "item": {"type": "mcp_tool_call", "server": "other_mcp", "tool": "search"}}),
        json.dumps({"type": "item.completed", "item": {"type": "agent_message", "text": "done: src/a.go:1"}}),
    ])
    r = parse_codex_jsonl(lines)
    assert hasattr(r, "all_tools"), "ModelResult must have all_tools for codex"
    # command_execution counted as "command_execution"
    assert r.all_tools.get("command_execution", 0) >= 1
    # prism mcp_tool_call
    assert r.all_tools.get("mcp_tool_call:prism", 0) >= 1 or \
           r.all_tools.get("nav_callers", 0) >= 1 or \
           "prism" in str(r.all_tools), (
               f"prism MCP call must appear in all_tools; got {r.all_tools}"
           )
    # prism_calls gate unchanged
    assert r.prism_calls == 1
    assert r.dose.count == 1
