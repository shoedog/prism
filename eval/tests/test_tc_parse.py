import json
from tier_c.parse import parse_claude_json, parse_codex_jsonl

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
