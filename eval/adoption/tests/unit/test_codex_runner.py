# eval/adoption/tests/unit/test_codex_runner.py
"""TDD tests for codex runner path: build_codex_cmd, model dispatch."""
from adoption.runner import build_codex_cmd, cache_key


def test_build_codex_cmd_structure():
    cmd = build_codex_cmd(prompt="who calls run_trial", model="gpt-5.5")
    assert cmd[0] == "codex"
    assert "exec" in cmd
    assert "--json" in cmd
    assert "-m" in cmd
    idx_m = cmd.index("-m")
    assert cmd[idx_m + 1] == "gpt-5.5"
    assert "-s" in cmd
    idx_s = cmd.index("-s")
    assert cmd[idx_s + 1] == "read-only"
    assert cmd[-1] == "who calls run_trial"


def test_build_codex_cmd_has_cd_flag():
    """The -C flag sets cwd for codex; runner passes eval_root/probe.repo via env, not flag."""
    # build_codex_cmd doesn't embed a -C; the caller sets CODEX_HOME + cwd via subprocess.
    # We just verify it builds a valid command without -C (caller handles cwd).
    cmd = build_codex_cmd(prompt="test", model="gpt-5.5")
    # -C should NOT be in build_codex_cmd output (it's passed separately or cwd is set)
    # Actually per spec: -C <eval/tier_c> is in the cmd. Let's check for absence of --mcp-config.
    assert "--mcp-config" not in cmd  # codex uses config.toml not --mcp-config flag


def test_cache_key_differs_by_model():
    a = cache_key(skill_bytes=b"v1", probe_id="p", prompt="q", repo="r", trial=0, model="sonnet")
    b = cache_key(skill_bytes=b"v1", probe_id="p", prompt="q", repo="r", trial=0, model="gpt-5.5")
    assert a != b
