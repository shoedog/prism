from tier_c.model import Variant
from tier_c.prompts import stage_prompt
from tier_c.arm_runner import build_codex_cmd, build_claude_cmd, FakeArmRunner

def test_stage_prompt_requires_citations():
    p = stage_prompt("spec", issue_text="bug X", scoped_slice="slice 1")
    assert "cite" in p.lower() and "file" in p.lower()  # citation parity

def test_codex_cmd_on_off_toggles_mcp():
    # codex configures MCP via inline `-c mcp_servers.prism...` (not a --mcp-config file)
    on = build_codex_cmd(Variant("gpt-5.5", True), repo="/r")
    off = build_codex_cmd(Variant("gpt-5.5", False), repo="/r")
    assert "mcp_servers.prism" in " ".join(on)
    assert "mcp_servers.prism" not in " ".join(off)
    assert "gpt-5.5" in " ".join(on)

def test_claude_cmd_on_off_toggles_mcp():
    on = build_claude_cmd(Variant("opus-4.8", True), mcp_cfg="/tmp/p.json")
    off = build_claude_cmd(Variant("opus-4.8", False), mcp_cfg="/tmp/p.json")
    assert "/tmp/p.json" in " ".join(on)
    assert "--mcp-config" not in " ".join(off)

def test_fake_runner_is_deterministic():
    r = FakeArmRunner({"gpt-5.5+prism": "spec cites src/a.py:1"})
    out = r.run(Variant("gpt-5.5", True), "spec", "prompt", "/r")
    assert out.text == "spec cites src/a.py:1"
    assert out.citations[0].file == "src/a.py"


def test_prism_mcp_bin_respects_env(monkeypatch):
    from tier_c.arm_runner import _prism_mcp_bin
    monkeypatch.setenv("PRISM_MCP_BIN", "/custom/prism-mcp")
    assert _prism_mcp_bin() == "/custom/prism-mcp"


def test_prism_mcp_bin_resolves_to_a_prism_mcp_path(monkeypatch):
    import os
    from tier_c.arm_runner import _prism_mcp_bin
    monkeypatch.delenv("PRISM_MCP_BIN", raising=False)
    out = _prism_mcp_bin()
    assert out.endswith("prism-mcp")
    # resolved via PATH or the repo's target/release build -> absolute; else bare fallback name
    assert out == "prism-mcp" or os.path.isabs(out)


def test_codex_cmd_prism_on_uses_resolved_bin():
    from tier_c.arm_runner import _prism_mcp_bin
    on = build_codex_cmd(Variant("gpt-5.5", True), repo="/r")
    assert f"mcp_servers.prism.command={_prism_mcp_bin()}" in on
