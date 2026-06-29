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


def test_live_ask_claude_isolates_mcp_and_tools(monkeypatch):
    """The claude judge MUST be fully isolated — two independent failure modes seen live:
      1. --strict-mcp-config so it ignores the user's default MCP servers (the prism-dev
         plugin). Without it, every judge `claude -p` launches prism-mcp, which eagerly
         builds the CPG before answering — minutes per call. Absence of --mcp-config is
         NOT isolation; default MCP servers still load unless --strict-mcp-config is set.
      2. --disallowed-tools <all> so the judge cannot use built-in tools. After MCP was
         dropped, opus still went AGENTIC (Read/Grep the repo, ~5min, prose not YES/NO).
    The prompt goes on STDIN (input=), NOT a positional, so the variadic --disallowed-tools
    cannot swallow it."""
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        seen["cmd"] = cmd
        seen["input"] = input
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":1,
                  "result":"YES","usage":{"output_tokens":1}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    live_ask("sonnet-4.6", "relevant?")
    assert "--strict-mcp-config" in seen["cmd"], f"must pass --strict-mcp-config; got {seen['cmd']}"
    assert "--disallowed-tools" in seen["cmd"], f"judge must disable built-in tools; got {seen['cmd']}"
    for t in ("Read", "Grep", "Bash"):
        assert t in seen["cmd"], f"--disallowed-tools must include {t}; got {seen['cmd']}"
    # prompt must be on stdin, never a positional (else the variadic eats it)
    assert seen["input"] == "relevant?", f"prompt must go via stdin; got input={seen['input']!r}"
    assert "relevant?" not in seen["cmd"], f"prompt must NOT be a positional arg; got {seen['cmd']}"


def test_judge_model_is_sonnet_claude():
    """The relevance/rank oracle runs on sonnet (a claude model), per owner testing that
    the smaller models follow the YES/NO instruction where opus went agentic."""
    from tier_c.llm import JUDGE_MODEL, MODEL_CLI
    assert JUDGE_MODEL in MODEL_CLI, f"JUDGE_MODEL must be a known model; got {JUDGE_MODEL}"
    assert MODEL_CLI[JUDGE_MODEL][0] == "claude"
    assert MODEL_CLI[JUDGE_MODEL][1] == "sonnet", (
        f"JUDGE_MODEL must map to the claude 'sonnet' alias; got {MODEL_CLI[JUDGE_MODEL]}")


def test_model_cli_has_sonnet_and_haiku_aliases():
    assert MODEL_CLI["sonnet-4.6"] == ("claude", "sonnet")
    assert MODEL_CLI["haiku-4.5"] == ("claude", "haiku")

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

def test_judge_tiebreaker_is_opus():
    from tier_c.llm import JUDGE_TIEBREAKER, MODEL_CLI
    assert JUDGE_TIEBREAKER in MODEL_CLI
    assert MODEL_CLI[JUDGE_TIEBREAKER] == ("claude", "opus")
