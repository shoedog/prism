# eval/tests/test_tc_harness_hardening.py
"""Unit tests for the F1-F4 Tier-C Part-C harness hardening fixes (harness-fix-spec.md).

F1 — matched-binary preflight (resolve_matched_binaries / PreflightError): kills the
     independent-resolution skew vector between prism and prism-mcp.
F2 — shared explicit --cache-dir threaded through prewarm + both agent MCP configs
     (prism_mcp_args / _prewarm_cpg / _prism_mcp_config / build_codex_cmd).
F3 — MCP_TIMEOUT/MCP_TOOL_TIMEOUT env for the claude on-arm subprocess (codex's
     startup_timeout_sec/tool_timeout_sec are covered in adoption/tests/unit/test_codex_env.py).
F4 — per-cell warm-initialize gate (warm_gate_check) wired into run_arm_isolated between
     _prewarm_cpg and runner.run.

All subprocess interaction is mocked — no real prism/prism-mcp binary is launched here
(one real-binary sanity check for resolve_matched_binaries is skipped when the binaries
are absent, mirroring tests/test_matrix.py's convention).
"""
from __future__ import annotations

import io
import json
import os
import types
from pathlib import Path

import pytest

from tier_c.model import Dose, ArmOutput, Variant
from tier_c.arm_runner import (
    ArmRunError,
    PreflightError,
    build_codex_cmd,
    prism_mcp_args,
    resolve_matched_binaries,
    run_arm_isolated,
    warm_gate_check,
)


# ---------------------------------------------------------------------------
# F2 — prism_mcp_args / _prewarm_cpg / _prism_mcp_config / build_codex_cmd cache_dir
# ---------------------------------------------------------------------------

def test_prism_mcp_args_includes_cache_dir():
    args = prism_mcp_args("/repo", cache_dir="/tmp/prism-cache")
    assert "--cache-dir" in args
    assert "/tmp/prism-cache" in args
    assert "--no-cache" not in args


def test_prism_mcp_args_omits_cache_dir_when_none():
    args = prism_mcp_args("/repo")
    assert "--cache-dir" not in args


def test_prism_mcp_args_no_cache_wins_over_cache_dir():
    """--no-cache and --cache-dir are mutually exclusive on prism-mcp (clap conflicts_with);
    no_cache=True must win so both are never emitted together."""
    args = prism_mcp_args("/repo", no_cache=True, cache_dir="/tmp/prism-cache")
    assert "--no-cache" in args
    assert "--cache-dir" not in args


def test_prewarm_cpg_includes_cache_dir(monkeypatch, tmp_path):
    import tier_c.arm_runner as arm_mod

    calls = []

    def fake_run(cmd, **kwargs):
        calls.append(cmd)
        return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()

    monkeypatch.setattr(arm_mod.subprocess, "run", fake_run)
    telemetry = arm_mod._prewarm_cpg(str(tmp_path), cache_dir="/tmp/shared-cache")

    assert "--cache-dir" in calls[0]
    assert "/tmp/shared-cache" in calls[0]
    assert telemetry["cache_dir"] == "/tmp/shared-cache"
    # `--cache-dir` is a GLOBAL `prism nav` flag: it MUST precede the `repo-map`
    # subcommand, else clap errors "unexpected argument" and the prewarm no-ops
    # (regression: the warm gate then trips on a cold prism-mcp).
    argv = calls[0]
    assert argv.index("--cache-dir") < argv.index("repo-map"), argv


def test_prewarm_cpg_omits_cache_dir_when_none(monkeypatch, tmp_path):
    """Backward-compat: no cache_dir arg -> argv unchanged from pre-F2 shape."""
    import tier_c.arm_runner as arm_mod

    calls = []

    def fake_run(cmd, **kwargs):
        calls.append(cmd)
        return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()

    monkeypatch.setattr(arm_mod.subprocess, "run", fake_run)
    arm_mod._prewarm_cpg(str(tmp_path))

    assert "--cache-dir" not in calls[0]


def test_prism_mcp_config_writes_cache_dir(tmp_path):
    """The claude per-checkout MCP config JSON must carry --cache-dir when given."""
    import tier_c.arm_runner as arm_mod

    path = arm_mod._prism_mcp_config(str(tmp_path), cache_dir="/tmp/shared-cache")
    cfg = json.loads(Path(path).read_text())
    args = cfg["mcpServers"]["prism"]["args"]
    assert "--cache-dir" in args
    assert "/tmp/shared-cache" in args


def test_build_codex_cmd_inline_args_include_cache_dir():
    """build_codex_cmd's inline -c mcp_servers.prism.args must carry --cache-dir too
    (symmetry with prism_mcp_args; exercised directly since CodexRunner routes prism-ON
    through CODEX_HOME instead, per its own docstring)."""
    cmd = build_codex_cmd(Variant("gpt-5.5", True), repo="/r", cache_dir="/tmp/shared-cache")
    joined = " ".join(cmd)
    assert "--cache-dir" in joined
    assert "/tmp/shared-cache" in joined


# ---------------------------------------------------------------------------
# F3 — ClaudeRunner sets MCP_TIMEOUT / MCP_TOOL_TIMEOUT for the prism-ON arm
# ---------------------------------------------------------------------------

def _make_minimal_stream_json() -> str:
    return json.dumps({
        "type": "result", "subtype": "success", "is_error": False, "result": "ok",
        "usage": {"input_tokens": 1, "output_tokens": 1}, "total_cost_usd": 0.0,
    })


def test_claude_runner_sets_mcp_timeout_env_for_prism_on(monkeypatch):
    import tier_c.arm_runner as arm_mod
    from tier_c.arm_runner import ClaudeRunner
    import unittest.mock as mock

    captured_envs = []

    def fake_run(*args, **kwargs):
        captured_envs.append(dict(kwargs.get("env", {})))
        return type("P", (), {"returncode": 0, "stdout": _make_minimal_stream_json(),
                              "stderr": ""})()

    runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
    with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
        runner.run(Variant("opus-4.8", True), "spec", "prompt", "/fake/repo")

    env = captured_envs[-1]
    assert env.get("MCP_TIMEOUT") == "600000", f"MCP_TIMEOUT not set for prism-ON: {env.get('MCP_TIMEOUT')!r}"
    assert env.get("MCP_TOOL_TIMEOUT") == "600000"


def test_claude_runner_omits_mcp_timeout_env_for_prism_off(monkeypatch):
    """prism-OFF has no MCP server at all; MCP_TIMEOUT must not be force-set for it."""
    import tier_c.arm_runner as arm_mod
    from tier_c.arm_runner import ClaudeRunner
    import unittest.mock as mock

    captured_envs = []

    def fake_run(*args, **kwargs):
        captured_envs.append(dict(kwargs.get("env", {})))
        return type("P", (), {"returncode": 0, "stdout": _make_minimal_stream_json(),
                              "stderr": ""})()

    runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
    with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
        runner.run(Variant("opus-4.8", False), "spec", "prompt", "/fake/repo")

    env = captured_envs[-1]
    assert "MCP_TIMEOUT" not in env


# ---------------------------------------------------------------------------
# F1 — resolve_matched_binaries / PreflightError
# ---------------------------------------------------------------------------

@pytest.fixture(autouse=True)
def _clear_prism_bin_env(monkeypatch):
    """Ensure ambient PRISM_BIN/PRISM_MCP_BIN/PRISM_BUILD_DIR never leak into a test that
    doesn't explicitly set them (the harness-fix-report run env may export these)."""
    monkeypatch.delenv("PRISM_BIN", raising=False)
    monkeypatch.delenv("PRISM_MCP_BIN", raising=False)
    monkeypatch.delenv("PRISM_BUILD_DIR", raising=False)


def test_resolve_matched_binaries_success(tmp_path):
    build = tmp_path / "release"
    build.mkdir()
    (build / "prism").write_bytes(b"fake-prism-binary")
    (build / "prism-mcp").write_bytes(b"fake-prism-mcp-binary")

    result = resolve_matched_binaries(build_dir=str(build))

    assert result["matched"] is True
    assert result["build_dir"] == str(build)
    assert result["prism_bin"] == str(build / "prism")
    assert result["prism_mcp_bin"] == str(build / "prism-mcp")
    assert result["prism_stat"]["size"] == len(b"fake-prism-binary")
    assert result["prism_mcp_stat"]["size"] == len(b"fake-prism-mcp-binary")
    assert isinstance(result["prism_stat"]["mtime"], float)


def test_resolve_matched_binaries_missing_prism_mcp_raises(tmp_path):
    build = tmp_path / "release"
    build.mkdir()
    (build / "prism").write_bytes(b"x")
    # prism-mcp deliberately absent

    with pytest.raises(PreflightError, match="prism-mcp"):
        resolve_matched_binaries(build_dir=str(build))


def test_resolve_matched_binaries_missing_prism_raises(tmp_path):
    build = tmp_path / "release"
    build.mkdir()
    (build / "prism-mcp").write_bytes(b"x")
    # prism deliberately absent

    with pytest.raises(PreflightError, match="prism binary not found"):
        resolve_matched_binaries(build_dir=str(build))


def test_resolve_matched_binaries_skew_between_env_vars_raises(tmp_path, monkeypatch):
    """PRISM_BIN and PRISM_MCP_BIN pointing at DIFFERENT parent dirs is the exact skew
    vector F1 exists to catch — must FAIL LOUD even before checking existence."""
    d1 = tmp_path / "build1"
    d2 = tmp_path / "build2"
    d1.mkdir()
    d2.mkdir()
    (d1 / "prism").write_bytes(b"x")
    (d2 / "prism-mcp").write_bytes(b"y")
    monkeypatch.setenv("PRISM_BIN", str(d1 / "prism"))
    monkeypatch.setenv("PRISM_MCP_BIN", str(d2 / "prism-mcp"))

    with pytest.raises(PreflightError, match="DIFFERENT build directories"):
        resolve_matched_binaries()


def test_resolve_matched_binaries_uses_build_dir_env(tmp_path, monkeypatch):
    build = tmp_path / "release"
    build.mkdir()
    (build / "prism").write_bytes(b"x")
    (build / "prism-mcp").write_bytes(b"y")
    monkeypatch.setenv("PRISM_BUILD_DIR", str(build))

    result = resolve_matched_binaries()
    assert result["build_dir"] == str(build)


def test_resolve_matched_binaries_derives_build_dir_from_prism_bin(tmp_path, monkeypatch):
    """When only $PRISM_BIN is set (no explicit build_dir / PRISM_BUILD_DIR), its parent
    dir becomes the resolved build_dir, and prism-mcp is looked up there too."""
    build = tmp_path / "release"
    build.mkdir()
    (build / "prism").write_bytes(b"x")
    (build / "prism-mcp").write_bytes(b"y")
    monkeypatch.setenv("PRISM_BIN", str(build / "prism"))

    result = resolve_matched_binaries()
    assert result["build_dir"] == str(build)
    assert result["prism_mcp_bin"] == str(build / "prism-mcp")


# Cached at module-collection time (BEFORE the _clear_prism_bin_env autouse fixture ever
# runs and deletes these from os.environ for the duration of each test).
_REAL_PRISM_BIN = os.environ.get("PRISM_BIN", "")
_REAL_PRISM_MCP_BIN = os.environ.get("PRISM_MCP_BIN", "")


@pytest.mark.skipif(
    not (_REAL_PRISM_BIN and os.path.exists(_REAL_PRISM_BIN) and
         _REAL_PRISM_MCP_BIN and os.path.exists(_REAL_PRISM_MCP_BIN)),
    reason="PRISM_BIN/PRISM_MCP_BIN not set to real binaries on disk",
)
def test_resolve_matched_binaries_against_real_binaries(monkeypatch):
    """Sanity check against the ACTUAL built binaries when PRISM_BIN/PRISM_MCP_BIN are
    exported (no subprocess launch — just a stat/existence check)."""
    monkeypatch.setenv("PRISM_BIN", _REAL_PRISM_BIN)
    monkeypatch.setenv("PRISM_MCP_BIN", _REAL_PRISM_MCP_BIN)
    result = resolve_matched_binaries()
    assert result["matched"] is True
    assert result["prism_stat"]["size"] > 0
    assert result["prism_mcp_stat"]["size"] > 0


# ---------------------------------------------------------------------------
# F4 — warm_gate_check (mocked subprocess.Popen; never launches a real prism-mcp)
# ---------------------------------------------------------------------------

class _FakeStdout:
    """Iterable standing in for a text-mode Popen.stdout: yields *lines* one at a time,
    optionally sleeping *delay* seconds before each yield (to simulate a slow/cold server
    without actually blocking the test suite for the full real cold-build duration)."""

    def __init__(self, lines: list[str], delay: float = 0.0):
        self._lines = lines
        self._delay = delay

    def __iter__(self):
        import time as _time
        for line in self._lines:
            if self._delay:
                _time.sleep(self._delay)
            yield line


class _FakeProc:
    def __init__(self, lines: list[str], delay: float = 0.0):
        self.stdout = _FakeStdout(lines, delay=delay)
        self.stdin = io.StringIO()
        self.stderr = io.StringIO()
        self.killed = False

    def kill(self):
        self.killed = True

    def wait(self, timeout=None):
        return 0


def _init_ok_line() -> str:
    return json.dumps({
        "jsonrpc": "2.0", "id": 1,
        "result": {"protocolVersion": "2025-11-25", "capabilities": {"tools": {}},
                  "serverInfo": {"name": "prism-mcp", "version": "0.0.0-test"}},
    }) + "\n"


def _tools_line(names: list[str]) -> str:
    return json.dumps({
        "jsonrpc": "2.0", "id": 2,
        "result": {"tools": [{"name": n} for n in names]},
    }) + "\n"


def _make_fake_popen_factory(lines: list[str], *, delay: float = 0.0, raises: Exception | None = None,
                             captured_argv: list | None = None):
    def _factory(argv, **kwargs):
        if captured_argv is not None:
            captured_argv.append(list(argv))
        if raises is not None:
            raise raises
        return _FakeProc(lines, delay=delay)
    return _factory


def test_warm_gate_check_ok_on_good_handshake(monkeypatch):
    import tier_c.arm_runner as arm_mod

    lines = [_init_ok_line(), _tools_line(["nav_callers", "nav_callees"])]
    monkeypatch.setattr(arm_mod.subprocess, "Popen", _make_fake_popen_factory(lines))

    result = warm_gate_check("/repo", timeout_s=5.0)

    assert result["ok"] is True
    assert result["tools_count"] == 2
    assert result["error"] is None
    assert result["wall_s"] >= 0.0


def test_warm_gate_check_fails_on_empty_tools_list(monkeypatch):
    import tier_c.arm_runner as arm_mod

    lines = [_init_ok_line(), _tools_line([])]
    monkeypatch.setattr(arm_mod.subprocess, "Popen", _make_fake_popen_factory(lines))

    result = warm_gate_check("/repo", timeout_s=5.0)

    assert result["ok"] is False
    assert result["tools_count"] == 0
    assert "empty" in (result["error"] or "").lower()


def test_warm_gate_check_fails_on_timeout(monkeypatch):
    """A slow/never-responding server must fail fast (bounded by timeout_s), not hang."""
    import tier_c.arm_runner as arm_mod

    lines = [_init_ok_line(), _tools_line(["x"])]
    monkeypatch.setattr(arm_mod.subprocess, "Popen",
                        _make_fake_popen_factory(lines, delay=2.0))

    result = warm_gate_check("/repo", timeout_s=0.05)

    assert result["ok"] is False
    assert "timed out" in (result["error"] or "").lower()
    assert result["wall_s"] < 2.0, "must fail fast, bounded by timeout_s, not wait for the slow reader"


def test_warm_gate_check_fails_on_spawn_error(monkeypatch):
    import tier_c.arm_runner as arm_mod

    monkeypatch.setattr(
        arm_mod.subprocess, "Popen",
        _make_fake_popen_factory([], raises=OSError("no such file or directory")),
    )

    result = warm_gate_check("/repo", timeout_s=1.0)

    assert result["ok"] is False
    assert "spawn failed" in (result["error"] or "").lower()


def test_warm_gate_check_includes_cache_dir_in_argv(monkeypatch):
    import tier_c.arm_runner as arm_mod

    captured: list = []
    lines = [_init_ok_line(), _tools_line(["x"])]
    monkeypatch.setattr(
        arm_mod.subprocess, "Popen",
        _make_fake_popen_factory(lines, captured_argv=captured),
    )

    warm_gate_check("/repo", cache_dir="/tmp/shared-cache", timeout_s=5.0)

    assert "--cache-dir" in captured[0]
    assert "/tmp/shared-cache" in captured[0]


def test_warm_gate_check_never_raises_on_malformed_response(monkeypatch):
    """A garbage first line (not JSON, or missing 'result') must be reported via the
    telemetry dict, never propagate as an exception."""
    import tier_c.arm_runner as arm_mod

    lines = ["not json at all\n"]
    monkeypatch.setattr(arm_mod.subprocess, "Popen", _make_fake_popen_factory(lines))

    result = warm_gate_check("/repo", timeout_s=5.0)
    assert result["ok"] is False
    assert result["error"] is not None


# ---------------------------------------------------------------------------
# F4 — run_arm_isolated wiring: the gate runs after prewarm, before runner.run,
# and a failing gate blocks the agent entirely (skip_warm_gate=False).
# ---------------------------------------------------------------------------

def _make_tmp_git_repo(tmp_path: Path) -> "types.SimpleNamespace":
    import subprocess as real_subprocess
    root = tmp_path / "repo"
    root.mkdir()
    real_subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "README.md").write_text("hello\n")
    real_subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    real_subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                         "commit", "-q", "-m", "init"], cwd=root, check=True)
    return types.SimpleNamespace(root=root)


def _recording_runner(calls: list):
    class _R:
        def run(self, variant, stage, prompt, repo_root):
            calls.append("runner.run")
            return ArmOutput(variant=variant, text="ok", citations=[], tokens=0,
                             tool_calls=0, wall_s=0.0, used_prism=False,
                             prism_calls=0, dose=Dose(), low_dose=False)
    return _R()


def _fake_git_ok(cmd, **kwargs):
    """Stand-in for subprocess.run that no-ops any command (git reset/clean, prism nav).
    None of these tests assert on real filesystem reset state (that's covered by
    test_arm_is_isolated_and_no_cache et al. in test_tc_arm_runner.py) — only on call
    order/gating between prewarm, the F4 gate, and runner.run."""
    return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()


def test_run_arm_isolated_skips_gate_by_default(tmp_path, monkeypatch):
    """Backward compat: skip_warm_gate defaults to True, so warm_gate_check is never
    even called unless a caller opts in."""
    import tier_c.arm_runner as arm_mod

    co = _make_tmp_git_repo(tmp_path)
    monkeypatch.setattr(arm_mod.subprocess, "run", _fake_git_ok)

    gate_calls = []
    monkeypatch.setattr(arm_mod, "warm_gate_check", lambda *a, **kw: gate_calls.append(1))

    calls: list = []
    arm_mod.run_arm_isolated(_recording_runner(calls), checkout=co,
                             variant=Variant("opus-4.8", True), prewarm=True)

    assert gate_calls == [], "warm_gate_check must not be called when skip_warm_gate=True (default)"
    assert calls == ["runner.run"]


def test_run_arm_isolated_gate_blocks_agent_on_failure(tmp_path, monkeypatch):
    import tier_c.arm_runner as arm_mod

    co = _make_tmp_git_repo(tmp_path)
    monkeypatch.setattr(arm_mod.subprocess, "run", _fake_git_ok)
    monkeypatch.setattr(
        arm_mod, "warm_gate_check",
        lambda *a, **kw: {"ok": False, "wall_s": 20.0, "tools_count": 0,
                          "argv": ["prism-mcp"], "error": "timed out after 15.0s"},
    )

    calls: list = []
    with pytest.raises(ArmRunError, match="not warm"):
        arm_mod.run_arm_isolated(
            _recording_runner(calls), checkout=co, variant=Variant("opus-4.8", True),
            prewarm=True, skip_warm_gate=False,
        )

    assert calls == [], "runner.run must NEVER be called when the F4 warm gate fails"


def test_run_arm_isolated_gate_allows_agent_on_success(tmp_path, monkeypatch):
    import tier_c.arm_runner as arm_mod

    co = _make_tmp_git_repo(tmp_path)
    monkeypatch.setattr(arm_mod.subprocess, "run", _fake_git_ok)
    monkeypatch.setattr(
        arm_mod, "warm_gate_check",
        lambda *a, **kw: {"ok": True, "wall_s": 1.2, "tools_count": 8,
                          "argv": ["prism-mcp"], "error": None},
    )

    calls: list = []
    result = arm_mod.run_arm_isolated(
        _recording_runner(calls), checkout=co, variant=Variant("opus-4.8", True),
        prewarm=True, skip_warm_gate=False,
    )

    assert calls == ["runner.run"]
    assert result.prewarm["warm_gate"]["ok"] is True
    assert result.prewarm["warm_gate"]["tools_count"] == 8


def test_run_arm_isolated_gate_skipped_for_prism_off_variant(tmp_path, monkeypatch):
    """prewarm (and therefore the gate) never runs for a prism-OFF variant, even with
    skip_warm_gate=False — preserves the prism-OFF arm staying prism-free."""
    import tier_c.arm_runner as arm_mod

    co = _make_tmp_git_repo(tmp_path)
    monkeypatch.setattr(arm_mod.subprocess, "run", _fake_git_ok)
    gate_calls = []
    monkeypatch.setattr(arm_mod, "warm_gate_check", lambda *a, **kw: gate_calls.append(1))

    calls: list = []
    arm_mod.run_arm_isolated(
        _recording_runner(calls), checkout=co, variant=Variant("opus-4.8", False),
        prewarm=True, skip_warm_gate=False,
    )

    assert gate_calls == [], "prism-OFF variant must never trigger the warm gate"
    assert calls == ["runner.run"]


# ---------------------------------------------------------------------------
# CLI wiring: run-partc --live threads --prism-build-dir/--skip-warm-gate/
# --warm-gate-timeout-s through to _run_partc_live (no live components touched).
# ---------------------------------------------------------------------------

def test_cli_run_partc_live_threads_hardening_flags(monkeypatch):
    import tier_c.cli as cli_mod

    captured: dict = {}

    def fake_run_partc_live(cell, **kwargs):
        captured["cell"] = cell
        captured.update(kwargs)

    monkeypatch.setattr(cli_mod, "_run_partc_live", fake_run_partc_live)

    rc = cli_mod.main([
        "run-partc", "--cell", "ruff:spec:opus-4.8", "--live",
        "--prism-build-dir", "/tmp/build",
        "--skip-warm-gate",
        "--warm-gate-timeout-s", "5",
    ])

    assert rc == 0
    assert captured["cell"] == ("ruff", "spec", "opus-4.8")
    assert captured["prism_build_dir"] == "/tmp/build"
    assert captured["skip_binary_preflight"] is False, (
        "the CLI --live path must always run the F1 preflight (no user-facing skip flag)")
    assert captured["skip_warm_gate"] is True
    assert captured["warm_gate_timeout_s"] == 5.0


def test_cli_run_partc_live_default_does_not_skip_warm_gate(monkeypatch):
    """Without --skip-warm-gate, the CLI must pass skip_warm_gate=False (gate ON by default)."""
    import tier_c.cli as cli_mod

    captured: dict = {}

    def fake_run_partc_live(cell, **kwargs):
        captured.update(kwargs)

    monkeypatch.setattr(cli_mod, "_run_partc_live", fake_run_partc_live)

    cli_mod.main(["run-partc", "--cell", "ruff:spec:opus-4.8", "--live"])

    assert captured["skip_warm_gate"] is False
    assert captured["skip_binary_preflight"] is False


# ---------------------------------------------------------------------------
# _run_partc_live: F1 preflight failure is recorded via status.json like other cell
# failures (failed_stage="preflight"), and never reaches the Checkout/agent stage.
# ---------------------------------------------------------------------------

def test_run_partc_live_binary_preflight_failure_writes_status_json(tmp_path, monkeypatch):
    import tier_c.cli as cli_mod

    # No _LivePartCComps/Checkout monkeypatching needed — preflight must fail BEFORE
    # either is ever touched. Fail fast: point --prism-build-dir at an empty tmp dir.
    empty_build_dir = tmp_path / "empty-build"
    empty_build_dir.mkdir()

    checkout_calls = []
    monkeypatch.setattr(cli_mod, "Checkout",
                        lambda *a, **kw: checkout_calls.append(1), raising=False)

    runs_root = str(tmp_path / "runs")
    run_id = "preflight-fail-test"

    with pytest.raises(Exception, match="matched-binary preflight FAILED"):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id=run_id,
            runs_root=runs_root,
            skip_binary_preflight=False,
            prism_build_dir=str(empty_build_dir),
        )

    assert checkout_calls == [], "Checkout must never be opened when the preflight fails"

    status_path = Path(runs_root) / run_id / "status.json"
    assert status_path.exists(), "status.json must be written even when the F1 preflight fails"
    data = json.loads(status_path.read_text())
    assert data["status"] == "failed"
    assert data["failed_stage"] == "preflight"
    assert "matched-binary preflight FAILED" in data["error"]
