import json
from tier_c.model import Variant
from tier_c.arm_runner import ClaudeRunner, CodexRunner

def test_lsp_off_prepends_deny_shim_to_path(monkeypatch, tmp_path):
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None, env=None):
        seen["path"] = (env or {}).get("PATH", "")
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":1,
                  "result":"ok","usage":{"output_tokens":1}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    deny = str(tmp_path / "deny")
    r = ClaudeRunner(lsp_deny_dir=deny)
    r.run(Variant("opus-4.8", False, lsp=False), "spec", "p", "/repo")   # lsp OFF -> deny on PATH
    assert seen["path"].startswith(deny)
    r.run(Variant("opus-4.8", False, lsp=True), "spec", "p", "/repo")    # lsp ON -> no deny
    assert not seen["path"].startswith(deny)


def test_codex_lsp_off_prepends_deny_shim_to_path(monkeypatch, tmp_path):
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None, env=None):
        seen["path"] = (env or {}).get("PATH", "")
        class R:
            # Real codex --json: items arrive under {"type":"item.completed","item":{...}}
            stdout = json.dumps({"type": "item.completed", "item": {"type": "agent_message", "text": "ok"}}); returncode = 0; stderr = ""
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    deny = str(tmp_path / "deny")
    r = CodexRunner(lsp_deny_dir=deny)
    r.run(Variant("gpt-5.5", False, lsp=False), "spec", "p", "/repo")
    assert seen["path"].startswith(deny)
    r.run(Variant("gpt-5.5", False, lsp=True), "spec", "p", "/repo")
    assert not seen["path"].startswith(deny)
