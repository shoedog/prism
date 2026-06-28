import json
import subprocess
import pytest
from pathlib import Path
from tier_c.model import Variant, Dose
from tier_c.prompts import stage_prompt
from tier_c.arm_runner import (
    build_codex_cmd, build_claude_cmd, FakeArmRunner, ClaudeRunner, CodexRunner,
    prism_mcp_args, _reset_clean, run_arm_isolated, IsolatedArmResult,
)

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


# ---------------------------------------------------------------------------
# Task 9: per-arm SUT+cache immutability
# ---------------------------------------------------------------------------

def _make_tmp_git_checkout(tmp_path: Path) -> "SimpleNamespace":
    """Create a minimal git repo in tmp_path and return an object with .root (Path)."""
    import types
    root = tmp_path / "repo"
    root.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "README.md").write_text("hello\n")
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=root, check=True)
    ns = types.SimpleNamespace()
    ns.root = root
    return ns


@pytest.fixture
def tmp_git_checkout(tmp_path):
    return _make_tmp_git_checkout(tmp_path)


def test_prism_mcp_args_no_cache_flag():
    """prism_mcp_args(r, no_cache=True) must include --no-cache."""
    args = prism_mcp_args("/some/repo", no_cache=True)
    assert "--no-cache" in args
    assert "--repo" in args
    assert "/some/repo" in args


def test_prism_mcp_args_cached_omits_flag():
    """prism_mcp_args(r, no_cache=False) must NOT include --no-cache."""
    args = prism_mcp_args("/some/repo", no_cache=False)
    assert "--no-cache" not in args
    assert "--repo" in args


def test_reset_clean_reverts_tracked_edit(tmp_git_checkout):
    """_reset_clean must revert an in-place edit to a tracked file (git reset --hard)."""
    root = tmp_git_checkout.root
    readme = root / "README.md"
    readme.write_text("mutated content\n")
    assert readme.read_text() == "mutated content\n"
    _reset_clean(root)
    assert readme.read_text() == "hello\n"


def test_reset_clean_removes_untracked_file(tmp_git_checkout):
    """_reset_clean must remove an untracked new file (git clean -fd)."""
    root = tmp_git_checkout.root
    new_file = root / "x.py"
    new_file.write_text("print('injected')\n")
    assert new_file.exists()
    _reset_clean(root)
    assert not new_file.exists()


def _fake_runner_that_writes(filename: str):
    """Return a runner whose .run() writes *filename* into checkout.root and returns ArmOutput."""
    from tier_c.model import Dose as _Dose, ArmOutput as _ArmOutput

    class _WritingRunner:
        def run(self, variant, stage, prompt, repo_root):
            (Path(repo_root) / filename).write_text("injected\n")
            return _ArmOutput(
                variant=variant, text="done", citations=[], tokens=0,
                tool_calls=0, wall_s=0.0, used_prism=False,
                prism_calls=0, dose=_Dose(), low_dose=False,
            )

    return _WritingRunner()


def test_arm_is_isolated_and_no_cache(tmp_git_checkout):
    """run_arm_isolated: arm writes x.py → reverted after; cache_mode/mcp_args recorded."""
    prism_on = Variant("opus-4.8", True)
    res = run_arm_isolated(
        _fake_runner_that_writes("x.py"),
        checkout=tmp_git_checkout,
        variant=prism_on,
        no_cache=True,
    )
    # x.py was written by the fake arm then cleaned by the AFTER reset
    assert not (tmp_git_checkout.root / "x.py").exists(), "x.py should have been removed by post-arm reset"
    assert res.cache_mode == "no-cache"
    assert "--no-cache" in res.mcp_args


def test_arm_isolated_after_reset_runs_even_if_arm_raises(tmp_git_checkout):
    """If the arm raises, _reset_clean still runs (finally block) → no leaked files."""
    prism_on = Variant("opus-4.8", True)

    class _ErrorRunner:
        def run(self, variant, stage, prompt, repo_root):
            (Path(repo_root) / "leaked.py").write_text("oops\n")
            raise RuntimeError("arm crashed")

    with pytest.raises(RuntimeError, match="arm crashed"):
        run_arm_isolated(
            _ErrorRunner(),
            checkout=tmp_git_checkout,
            variant=prism_on,
            no_cache=True,
        )
    assert not (tmp_git_checkout.root / "leaked.py").exists(), "AFTER reset must run even on arm error"


# ---------------------------------------------------------------------------
# P0 fix: CLAUDE_CONFIG_DIR / CODEX_HOME for prism-ON arms (dose=0 fix)
# ---------------------------------------------------------------------------

def _capture_env_from_runner(runner, variant: Variant, stream: str) -> dict:
    """Run *runner* with subprocess.run patched; return the env dict it was called with."""
    import tier_c.arm_runner as arm_mod
    import unittest.mock as mock

    captured_envs = []

    def fake_run(*args, **kwargs):
        captured_envs.append(dict(kwargs.get("env", {})))
        return _FakeCompletedProcess(stdout=stream)

    with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
        runner.run(variant, "spec", "prompt", "/fake/repo")

    # Last call is the arm subprocess (earlier calls may be _prism_mcp_config
    # subprocess.run calls — there are none in the current impl, but be defensive).
    return captured_envs[-1]


def _make_minimal_codex_stream() -> str:
    """Minimal valid codex --json JSONL stream (0 prism calls)."""
    return "\n".join([
        json.dumps({"type": "item.completed",
                    "item": {"type": "agent_message", "text": "ok"}}),
        json.dumps({"type": "turn.completed",
                    "usage": {"input_tokens": 1, "output_tokens": 1}}),
    ])


class TestClaudeCfgDir:
    """CLAUDE_CONFIG_DIR is set for prism-ON claude arms; NOT set for prism-OFF."""

    def test_prism_on_sets_claude_config_dir(self):
        runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
        env = _capture_env_from_runner(
            runner, Variant("opus-4.8", True), stream_with(prism_calls=0))
        assert "CLAUDE_CONFIG_DIR" in env, (
            "prism-ON ClaudeRunner must set CLAUDE_CONFIG_DIR so mcp__prism tools are permitted")

    def test_prism_on_config_dir_has_skill(self):
        runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
        env = _capture_env_from_runner(
            runner, Variant("opus-4.8", True), stream_with(prism_calls=0))
        cfg_dir = env["CLAUDE_CONFIG_DIR"]
        skill_md = Path(cfg_dir) / "skills" / "prism-code-navigation" / "SKILL.md"
        assert skill_md.exists(), (
            f"skills/prism-code-navigation/SKILL.md not found in CLAUDE_CONFIG_DIR {cfg_dir}")

    def test_prism_on_config_dir_allows_mcp_prism(self):
        runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
        env = _capture_env_from_runner(
            runner, Variant("opus-4.8", True), stream_with(prism_calls=0))
        cfg_dir = env["CLAUDE_CONFIG_DIR"]
        settings = json.loads((Path(cfg_dir) / "settings.json").read_text())
        allowed = settings.get("permissions", {}).get("allow", [])
        assert "mcp__prism" in allowed, (
            f"settings.json permissions.allow must include 'mcp__prism'; got {allowed}")

    def test_prism_off_does_not_set_claude_config_dir(self):
        runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
        env = _capture_env_from_runner(
            runner, Variant("opus-4.8", False), stream_with(prism_calls=0))
        assert "CLAUDE_CONFIG_DIR" not in env, (
            "prism-OFF ClaudeRunner must NOT set CLAUDE_CONFIG_DIR (keep baseline vanilla)")

    def test_prism_on_still_passes_mcp_config_arg(self):
        """The per-checkout --mcp-config (with --no-cache) must still be present for prism-ON."""
        import tier_c.arm_runner as arm_mod
        import unittest.mock as mock

        captured_cmds = []

        def fake_run(*args, **kwargs):
            captured_cmds.append(list(args[0]) if args else [])
            return _FakeCompletedProcess(stdout=stream_with(prism_calls=0))

        runner = ClaudeRunner(no_cache=True)  # no mcp_cfg override: uses per-checkout config
        with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
            runner.run(Variant("opus-4.8", True), "spec", "prompt", "/fake/repo")

        cmd = captured_cmds[-1]
        cmd_str = " ".join(cmd)
        assert "--mcp-config" in cmd_str, (
            "prism-ON ClaudeRunner must still pass --mcp-config to the claude command")

    def test_prism_on_config_dir_cached_across_runs(self):
        """_arm_config_dir must return the same dir on repeated calls (lazy cache)."""
        runner = ClaudeRunner(mcp_cfg="/tmp/fake.json")
        # Access the private method directly to verify caching behaviour.
        dir1 = runner._arm_config_dir()
        dir2 = runner._arm_config_dir()
        assert dir1 == dir2, "ClaudeRunner._arm_config_dir() must return the same dir on every call"


class TestCodexCfgHome:
    """CODEX_HOME is set for prism-ON codex arms; NOT set for prism-OFF.
    Inline -c mcp_servers.prism.* args are dropped when CODEX_HOME is used."""

    def test_prism_on_sets_codex_home(self):
        runner = CodexRunner()
        env = _capture_env_from_runner(
            runner, Variant("gpt-5.5", True), _make_minimal_codex_stream())
        assert "CODEX_HOME" in env, (
            "prism-ON CodexRunner must set CODEX_HOME so codex loads the prism MCP + skill")

    def test_prism_off_does_not_set_codex_home(self):
        runner = CodexRunner()
        env = _capture_env_from_runner(
            runner, Variant("gpt-5.5", False), _make_minimal_codex_stream())
        assert "CODEX_HOME" not in env, (
            "prism-OFF CodexRunner must NOT set CODEX_HOME")

    def test_prism_on_drops_inline_mcp_servers_args(self):
        """When CODEX_HOME defines the MCP server, no -c mcp_servers.prism.* must be injected."""
        import tier_c.arm_runner as arm_mod
        import unittest.mock as mock

        captured_cmds = []

        def fake_run(*args, **kwargs):
            captured_cmds.append(list(args[0]) if args else [])
            return _FakeCompletedProcess(stdout=_make_minimal_codex_stream())

        runner = CodexRunner()
        with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
            runner.run(Variant("gpt-5.5", True), "spec", "prompt", "/fake/repo")

        cmd_str = " ".join(captured_cmds[-1])
        assert "mcp_servers.prism" not in cmd_str, (
            "prism-ON CodexRunner must NOT inject inline -c mcp_servers.prism.* args "
            "(CODEX_HOME/config.toml defines the server instead)")

    def test_prism_off_has_no_inline_mcp_servers_args(self):
        import tier_c.arm_runner as arm_mod
        import unittest.mock as mock

        captured_cmds = []

        def fake_run(*args, **kwargs):
            captured_cmds.append(list(args[0]) if args else [])
            return _FakeCompletedProcess(stdout=_make_minimal_codex_stream())

        runner = CodexRunner()
        with mock.patch.object(arm_mod.subprocess, "run", side_effect=fake_run):
            runner.run(Variant("gpt-5.5", False), "spec", "prompt", "/fake/repo")

        cmd_str = " ".join(captured_cmds[-1])
        assert "mcp_servers.prism" not in cmd_str


# ---------------------------------------------------------------------------
# Part-C prewarm fix: _prism_bin + _prewarm_cpg + run_arm_isolated prewarm param
# ---------------------------------------------------------------------------

def test_prism_bin_respects_env(monkeypatch):
    """`_prism_bin` honours $PRISM_BIN env override."""
    from tier_c.arm_runner import _prism_bin
    monkeypatch.setenv("PRISM_BIN", "/custom/prism")
    assert _prism_bin() == "/custom/prism"


def test_prism_bin_resolves_to_repo_release_prism(monkeypatch):
    """`_prism_bin` falls back to <repo>/target/release/prism (mirrors _prism_mcp_bin test)."""
    import os
    from tier_c.arm_runner import _prism_bin
    monkeypatch.delenv("PRISM_BIN", raising=False)
    result = _prism_bin()
    # Either an absolute path (PATH hit or repo build) or the bare fallback name
    assert result.endswith("prism")
    assert result == "prism" or os.path.isabs(result)


def test_prism_bin_uses_repo_target_when_exists(monkeypatch, tmp_path):
    """`_prism_bin` returns the repo target/release/prism when it exists on disk."""
    import os
    import shutil
    import tier_c.arm_runner as arm_mod

    # Ensure env override is absent and PATH shim is absent
    monkeypatch.delenv("PRISM_BIN", raising=False)
    monkeypatch.setattr(shutil, "which", lambda _name: None)

    # Build the expected path from __file__ (mirrors the production normpath logic)
    expected = os.path.normpath(
        os.path.join(os.path.dirname(os.path.abspath(arm_mod.__file__)),
                     "..", "..", "target", "release", "prism")
    )
    # If the real file already exists, just assert the function returns it
    if os.path.exists(expected):
        assert arm_mod._prism_bin() == expected
    else:
        # Create a fake binary so exists() returns True; patch os.path.exists
        original_exists = os.path.exists

        def patched_exists(p):
            if os.path.normpath(p) == os.path.normpath(expected):
                return True
            return original_exists(p)

        monkeypatch.setattr(os.path, "exists", patched_exists)
        assert arm_mod._prism_bin() == expected


def test_prewarm_calls_nav_repo_map(monkeypatch, tmp_path):
    """`_prewarm_cpg` calls `prism nav repo-map --repo <root>` via subprocess.run."""
    import tier_c.arm_runner as arm_mod

    calls = []

    def fake_run(cmd, **kwargs):
        calls.append(cmd)
        return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()

    monkeypatch.setattr(arm_mod.subprocess, "run", fake_run)
    arm_mod._prewarm_cpg(str(tmp_path))

    assert len(calls) == 1
    cmd = calls[0]
    assert "nav" in cmd
    assert "repo-map" in cmd
    assert "--repo" in cmd
    assert str(tmp_path) in cmd


def test_prewarm_suppresses_exceptions(monkeypatch, tmp_path):
    """`_prewarm_cpg` swallows all exceptions (best-effort)."""
    import tier_c.arm_runner as arm_mod

    def bang(*a, **kw):
        raise RuntimeError("prism exploded")

    monkeypatch.setattr(arm_mod.subprocess, "run", bang)
    # Must not raise:
    arm_mod._prewarm_cpg(str(tmp_path))


def test_run_arm_isolated_prewarm_order(tmp_path):
    """prewarm=True: warm call runs AFTER the before-reset and BEFORE runner.run.

    Patching order: monkeypatch subprocess.run so we can record call sequence.
    We use a real tmp git checkout so _reset_clean is real (not faked).
    The fake runner just returns an ArmOutput; we record its call position.
    """
    import types
    import unittest.mock as mock
    import tier_c.arm_runner as arm_mod
    from tier_c.model import Dose as _Dose, ArmOutput as _ArmOutput

    # --- set up minimal git checkout ---
    root = tmp_path / "repo"
    root.mkdir()
    import subprocess as real_subprocess
    real_subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "README.md").write_text("hello\n")
    real_subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    real_subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                         "commit", "-q", "-m", "init"], cwd=root, check=True)
    co = types.SimpleNamespace(root=root)

    # --- record all subprocess.run calls ---
    call_log = []  # list of (kind, argv_or_label)

    original_run = real_subprocess.run

    def recording_run(cmd, *args, **kwargs):
        # Classify: git commands vs prism nav
        if isinstance(cmd, (list, tuple)):
            first = cmd[0]
            if first == "git":
                call_log.append(("git", list(cmd)))
                return original_run(cmd, *args, **kwargs)
            elif str(first).endswith("prism") or first == "prism":
                call_log.append(("prewarm", list(cmd)))
                # Return a no-op result (don't actually run prism)
                return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()
        call_log.append(("other", cmd))
        return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()

    # --- fake runner records when it runs ---
    runner_calls = []

    class _RecordingRunner:
        def run(self, variant, stage, prompt, repo_root):
            runner_calls.append("runner.run")
            return _ArmOutput(
                variant=variant, text="ok", citations=[], tokens=0,
                tool_calls=0, wall_s=0.0, used_prism=False,
                prism_calls=0, dose=_Dose(), low_dose=False,
            )

    variant = Variant("opus-4.8", True)
    with mock.patch.object(arm_mod.subprocess, "run", side_effect=recording_run):
        arm_mod.run_arm_isolated(
            _RecordingRunner(),
            checkout=co,
            variant=variant,
            prewarm=True,
        )

    # Extract the sequence of event kinds
    kinds = [k for k, _ in call_log]
    # Must have: git (before-reset x2) → prewarm → ... runner.run ... → git (after-reset x2)
    # combined with runner_calls
    # Build a merged ordered sequence
    events = []
    git_idx = 0
    prewarm_idx = 0
    runner_idx = 0
    for kind, argv in call_log:
        events.append(kind)
    # Now splice in runner.run calls in position (they don't go through subprocess.run)
    # The runner runs between the prewarm and the after-reset git calls.
    # We detect before-reset as the first two git calls, after-reset as the last two.
    assert "prewarm" in kinds, "Expected a prewarm call in subprocess.run"
    prewarm_pos = kinds.index("prewarm")

    # Count git calls before and after prewarm
    git_calls_before_prewarm = [i for i, k in enumerate(kinds) if k == "git" and i < prewarm_pos]
    git_calls_after_prewarm = [i for i, k in enumerate(kinds) if k == "git" and i > prewarm_pos]

    assert len(git_calls_before_prewarm) >= 2, (
        "before-reset (2 git calls) must precede prewarm")
    assert len(git_calls_after_prewarm) >= 2, (
        "after-reset (2 git calls) must follow prewarm")
    assert runner_calls == ["runner.run"], "runner.run must have been called exactly once"
    # Verify the prewarm subprocess call contains repo-map + --repo + root
    prewarm_argv = [argv for kind, argv in call_log if kind == "prewarm"][0]
    assert "repo-map" in prewarm_argv
    assert "--repo" in prewarm_argv
    assert str(root) in prewarm_argv


def test_run_arm_isolated_no_prewarm_by_default(tmp_path):
    """prewarm=False (default): no prism nav subprocess call is made."""
    import types
    import unittest.mock as mock
    import tier_c.arm_runner as arm_mod
    from tier_c.model import Dose as _Dose, ArmOutput as _ArmOutput

    root = tmp_path / "repo"
    root.mkdir()
    import subprocess as real_subprocess
    real_subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "README.md").write_text("hello\n")
    real_subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    real_subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                         "commit", "-q", "-m", "init"], cwd=root, check=True)
    co = types.SimpleNamespace(root=root)

    call_log = []
    original_run = real_subprocess.run

    def recording_run(cmd, *args, **kwargs):
        if isinstance(cmd, (list, tuple)):
            first = cmd[0]
            if first == "git":
                call_log.append(("git", list(cmd)))
                return original_run(cmd, *args, **kwargs)
            elif str(first).endswith("prism") or first == "prism":
                call_log.append(("prewarm", list(cmd)))
                return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()
        call_log.append(("other", cmd))
        return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()

    class _NoopRunner:
        def run(self, variant, stage, prompt, repo_root):
            return _ArmOutput(
                variant=variant, text="ok", citations=[], tokens=0,
                tool_calls=0, wall_s=0.0, used_prism=False,
                prism_calls=0, dose=_Dose(), low_dose=False,
            )

    variant = Variant("opus-4.8", True)
    with mock.patch.object(arm_mod.subprocess, "run", side_effect=recording_run):
        arm_mod.run_arm_isolated(
            _NoopRunner(),
            checkout=co,
            variant=variant,
            # prewarm defaults to False
        )

    kinds = [k for k, _ in call_log]
    assert "prewarm" not in kinds, "prewarm=False (default) must not call prism nav"


def test_run_arm_isolated_prewarm_skipped_for_prism_off(tmp_path):
    """prewarm=True with prism-OFF variant must NOT call the pre-warm."""
    import types
    import unittest.mock as mock
    import tier_c.arm_runner as arm_mod
    from tier_c.model import Dose as _Dose, ArmOutput as _ArmOutput

    root = tmp_path / "repo"
    root.mkdir()
    import subprocess as real_subprocess
    real_subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "README.md").write_text("hello\n")
    real_subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    real_subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                         "commit", "-q", "-m", "init"], cwd=root, check=True)
    co = types.SimpleNamespace(root=root)

    call_log = []
    original_run = real_subprocess.run

    def recording_run(cmd, *args, **kwargs):
        if isinstance(cmd, (list, tuple)):
            first = cmd[0]
            if first == "git":
                call_log.append(("git", list(cmd)))
                return original_run(cmd, *args, **kwargs)
            elif str(first).endswith("prism") or first == "prism":
                call_log.append(("prewarm", list(cmd)))
                return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()
        call_log.append(("other", cmd))
        return type("R", (), {"returncode": 0, "stdout": "", "stderr": ""})()

    class _NoopRunner:
        def run(self, variant, stage, prompt, repo_root):
            return _ArmOutput(
                variant=variant, text="ok", citations=[], tokens=0,
                tool_calls=0, wall_s=0.0, used_prism=False,
                prism_calls=0, dose=_Dose(), low_dose=False,
            )

    variant = Variant("opus-4.8", False)  # prism OFF
    with mock.patch.object(arm_mod.subprocess, "run", side_effect=recording_run):
        arm_mod.run_arm_isolated(
            _NoopRunner(),
            checkout=co,
            variant=variant,
            prewarm=True,  # requested but variant.prism=False → must be skipped
        )

    kinds = [k for k, _ in call_log]
    assert "prewarm" not in kinds, "prism-OFF variant must not trigger prewarm even when prewarm=True"
