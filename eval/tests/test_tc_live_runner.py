import json
from tier_c.model import Variant
from tier_c.arm_runner import ClaudeRunner, CodexRunner

def test_claude_runner_builds_output(monkeypatch):
    captured = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        captured["cmd"] = cmd
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":2,
                  "result":"spec cites src/a.py:1","total_cost_usd":0.01,
                  "usage":{"input_tokens":5,"output_tokens":7}}); returncode = 0; stderr = ""
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    out = ClaudeRunner(mcp_cfg="/tmp/p.json").run(Variant("opus-4.8", True), "spec", "PROMPT", "/repo")
    assert out.text == "spec cites src/a.py:1"
    assert out.tokens == 7 and out.tool_calls == 1
    assert out.citations[0].file == "src/a.py"
    assert out.used_prism is True  # prism-ON variant + a tool call occurred
    assert "--mcp-config" in captured["cmd"]

def test_codex_runner_off_has_no_prism(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        captured = cmd
        class R:
            stdout = "\n".join([
                json.dumps({"item":{"type":"command_execution"}}),
                json.dumps({"usage":{"input_tokens":3,"output_tokens":9}}),
                json.dumps({"item":{"type":"agent_message","text":"plan src/b.go:2"}})])
            returncode = 0; stderr = ""
        assert "mcp_servers.prism" not in " ".join(cmd)
        assert "--json" in cmd
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    out = CodexRunner().run(Variant("gpt-5.5", False), "plan", "PROMPT", "/repo")
    assert out.text == "plan src/b.go:2" and out.tokens == 9
    assert out.used_prism is False


def test_runner_raises_clear_error_on_subprocess_failure(monkeypatch):
    import pytest
    from tier_c.model import Variant
    from tier_c.arm_runner import ClaudeRunner
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        class R: stdout = ""; returncode = 1; stderr = "auth: missing API key"
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    with pytest.raises(RuntimeError, match="auth: missing API key"):
        ClaudeRunner(mcp_cfg="/tmp/p.json").run(Variant("opus-4.8", True), "spec", "P", "/repo")
