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

def test_live_ask_codex_parses_jsonl(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        assert "mcp_servers.prism" not in " ".join(cmd)   # no prism
        class R: stdout = json.dumps({"item":{"type":"agent_message","text":"YES"}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    assert live_ask("gpt-5.5", "relevant?") == "YES"
