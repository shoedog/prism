import json
import subprocess
import pytest
from tier_c.model import Variant, Dose
from tier_c.prompts import stage_prompt
from tier_c.arm_runner import build_codex_cmd, build_claude_cmd, FakeArmRunner, ClaudeRunner, CodexRunner

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


def test_build_cmds_map_model_to_cli_flag():
    # claude rejects "opus-4.8" (exit 1); it needs the "opus" alias — the arms MUST map
    # through cli_model_flag, same as the judges (caught by the 2026-06-24 live smoke).
    claude = build_claude_cmd(Variant("opus-4.8", False), mcp_cfg="x")
    assert "opus" in claude and "opus-4.8" not in claude
    codex = build_codex_cmd(Variant("gpt-5.5", False), repo="/r")
    assert "gpt-5.5" in codex  # codex accepts gpt-5.5 as-is


# ---------------------------------------------------------------------------
# Helpers for Task 8: real prism-call gate + dose via stream-json
# ---------------------------------------------------------------------------

def _make_prism_tool_line(tool_name: str) -> str:
    """Build one stream-json line with a single prism mcp__prism__* tool_use."""
    return json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": f"mcp__prism__{tool_name}", "input": {}},
    ]}})


def _make_text_line(text: str) -> str:
    return json.dumps({"type": "assistant", "message": {"content": [
        {"type": "text", "text": text},
    ]}})


def _make_result_line(input_tokens: int = 10, output_tokens: int = 42,
                      cost_usd: float = 0.07) -> str:
    """The final ``type==result`` line that carries usage + cost in stream-json."""
    return json.dumps({
        "type": "result",
        "subtype": "success",
        "is_error": False,
        "result": "summary text",
        "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
        "total_cost_usd": cost_usd,
    })


def stream_with(prism_calls: int, *, input_tokens: int = 10, output_tokens: int = 42,
                cost_usd: float = 0.07) -> str:
    """Build a fake stream-json string with N prism tool calls + result usage event."""
    lines = []
    for i in range(prism_calls):
        lines.append(_make_prism_tool_line(f"nav_callers_{i}"))
    lines.append(_make_text_line("Found issues in src/main.rs"))
    lines.append(_make_result_line(input_tokens=input_tokens, output_tokens=output_tokens,
                                   cost_usd=cost_usd))
    return "\n".join(lines)


class _FakeCompletedProcess:
    def __init__(self, stdout: str, returncode: int = 0, stderr: str = ""):
        self.stdout = stdout
        self.returncode = returncode
        self.stderr = stderr


def _run_fake_claude_arm(stream: str, variant: Variant | None = None,
                         monkeypatch_fn=None) -> "ArmOutput":
    """Run ClaudeRunner with subprocess patched to return the given stream-json text.

    monkeypatch_fn: a callable(target, name, value) to monkeypatch subprocess.run.
    When called from a test with monkeypatch, pass monkeypatch.setattr.
    """
    import tier_c.arm_runner as arm_mod
    if variant is None:
        variant = Variant("opus-4.8", True)
    original_run = subprocess.run

    def fake_run(*args, **kwargs):
        return _FakeCompletedProcess(stdout=stream)

    arm_mod_subprocess = arm_mod.__dict__.get("subprocess") or subprocess
    # Patch subprocess.run directly in the arm_runner module namespace
    import unittest.mock as mock
    with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
        runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
        return runner.run(variant, "spec", "prompt text", "/fake/repo")


# ---------------------------------------------------------------------------
# Task 8 target tests: real mcp__prism__* gate + dose fields on ArmOutput
# ---------------------------------------------------------------------------

def test_arm_output_uses_real_prism_calls_and_flags_low_dose():
    """1 prism call → used_prism=True, prism_calls=1, low_dose=True (≤1 call)."""
    out = _run_fake_claude_arm(stream_with(prism_calls=1))
    assert out.used_prism, "1 real prism call must set used_prism=True"
    assert out.prism_calls == 1
    assert out.low_dose, "exactly 1 prism call is low-dose (≤1)"


def test_arm_output_zero_prism_calls_not_administered():
    """0 prism calls → used_prism=False (true zero, not a heuristic)."""
    out = _run_fake_claude_arm(stream_with(prism_calls=0))
    assert not out.used_prism, "0 real prism calls must set used_prism=False"
    assert out.prism_calls == 0
    assert not out.low_dose


def test_arm_output_two_or_more_prism_calls_not_low_dose():
    """≥2 prism calls → used_prism=True, low_dose=False."""
    out = _run_fake_claude_arm(stream_with(prism_calls=2))
    assert out.used_prism
    assert out.prism_calls == 2
    assert not out.low_dose, "2 prism calls is NOT low-dose"


def test_arm_output_usage_cost_preserved_from_result_event():
    """Switching to stream-json must NOT lose input_tokens/output_tokens/cost_usd.
    The final type==result line carries them; parse_claude_stream_json must read it."""
    out = _run_fake_claude_arm(stream_with(prism_calls=1, input_tokens=55,
                                           output_tokens=88, cost_usd=0.13))
    # ArmOutput.tokens maps to output_tokens from the ModelResult
    assert out.tokens == 88, f"output_tokens not captured; got {out.tokens}"


def test_claude_cmd_uses_stream_json_format():
    """build_claude_cmd must use --output-format stream-json (not json)."""
    cmd = build_claude_cmd(Variant("opus-4.8", False), mcp_cfg="x")
    cmd_str = " ".join(cmd)
    assert "stream-json" in cmd_str, f"expected stream-json in command: {cmd_str}"
    assert "--output-format json" not in cmd_str


def test_arm_output_dose_populated():
    """ArmOutput.dose must reflect the prism call count from the stream."""
    out = _run_fake_claude_arm(stream_with(prism_calls=3))
    assert isinstance(out.dose, Dose)
    assert out.dose.count == 3


# ---------------------------------------------------------------------------
# Codex runner: prism_calls derived from mcp_tool_call events
# ---------------------------------------------------------------------------

def _make_codex_stream_with_prism(prism_calls: int) -> str:
    """Build a fake codex --json JSONL stream with N prism mcp_tool_call events."""
    lines = []
    for i in range(prism_calls):
        lines.append(json.dumps({
            "type": "item.completed",
            "item": {
                "type": "mcp_tool_call",
                "server": "prism",
                "tool": f"nav_callers_{i}",
            }
        }))
    lines.append(json.dumps({
        "type": "item.completed",
        "item": {"type": "agent_message", "text": "analysis done src/main.rs:42"},
    }))
    lines.append(json.dumps({
        "type": "turn.completed",
        "usage": {"input_tokens": 10, "output_tokens": 20},
    }))
    return "\n".join(lines)


def test_codex_arm_output_real_prism_calls():
    """CodexRunner: prism_calls from mcp_tool_call items with server=='prism'."""
    import tier_c.arm_runner as arm_mod
    import unittest.mock as mock

    stream = _make_codex_stream_with_prism(prism_calls=2)

    def fake_run(*args, **kwargs):
        return _FakeCompletedProcess(stdout=stream)

    with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
        runner = CodexRunner()
        out = runner.run(Variant("gpt-5.5", True), "spec", "prompt", "/fake/repo")

    assert out.prism_calls == 2
    assert out.used_prism
    assert not out.low_dose  # 2 calls → not low-dose


def test_codex_arm_output_zero_prism_calls():
    """CodexRunner: 0 prism mcp_tool_call events → used_prism=False."""
    import tier_c.arm_runner as arm_mod
    import unittest.mock as mock

    stream = _make_codex_stream_with_prism(prism_calls=0)

    def fake_run(*args, **kwargs):
        return _FakeCompletedProcess(stdout=stream)

    with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
        runner = CodexRunner()
        out = runner.run(Variant("gpt-5.5", True), "spec", "prompt", "/fake/repo")

    assert out.prism_calls == 0
    assert not out.used_prism
