import json
from tier_c.llm import live_ask, MODEL_CLI

def test_model_cli_maps_families():
    assert MODEL_CLI["opus-4.8"][0] == "claude"
    assert MODEL_CLI["gpt-5.5"][0] == "codex"

def test_live_ask_claude_parses_result(monkeypatch):
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        seen["cmd"] = cmd
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":1,
                  "result":"ranked: cand1,cand0","usage":{"output_tokens":5}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    out = live_ask("opus-4.8", "rank these")
    assert out == "ranked: cand1,cand0"
    assert "--mcp-config" not in " ".join(seen["cmd"])   # judges get NO prism


def test_live_ask_claude_passes_strict_mcp_config(monkeypatch):
    """The claude judge MUST pass --strict-mcp-config so it ignores the user's default
    MCP servers (the prism-dev plugin). Without it, every judge `claude -p` launches
    prism-mcp, which eagerly builds the CPG before answering — turning a ~3s relevance
    call into minutes (observed: one citation hung >4min). Absence of --mcp-config is
    NOT isolation: the default config's MCP servers still load unless --strict-mcp-config
    is set. The judge is designed tool-free (judges_live.py docstring)."""
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        seen["cmd"] = cmd
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":1,
                  "result":"YES","usage":{"output_tokens":1}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    live_ask("opus-4.8", "relevant?")
    assert "--strict-mcp-config" in seen["cmd"], (
        f"claude judge must pass --strict-mcp-config to avoid loading default MCP servers; "
        f"got {seen['cmd']}")
    # the prompt must remain the trailing positional (the strict flag is boolean, takes no value)
    assert seen["cmd"][-1] == "relevant?", (
        f"prompt must stay the last positional arg; got {seen['cmd']}")

def test_live_ask_codex_parses_jsonl(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        assert "mcp_servers.prism" not in " ".join(cmd)   # no prism
        # Real codex --json: items arrive under {"type":"item.completed","item":{...}}
        class R: stdout = json.dumps({"type":"item.completed","item":{"type":"agent_message","text":"YES"}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    assert live_ask("gpt-5.5", "relevant?") == "YES"

def test_live_ask_claude_raises_runtime_error_on_bad_json(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        class R: stdout = json.dumps({"is_error": True, "subtype": "api_error"}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    import pytest
    with pytest.raises(RuntimeError, match="claude judge output unparseable"):
        live_ask("opus-4.8", "rank these")

def test_live_ask_codex_raises_runtime_error_on_bad_jsonl(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        # JSONL with no agent_message -> ValueError -> RuntimeError
        class R: stdout = json.dumps({"item":{"type":"other","text":""}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    import pytest
    with pytest.raises(RuntimeError, match="codex judge output unparseable"):
        live_ask("gpt-5.5", "relevant?")
