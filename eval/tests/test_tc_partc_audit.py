# eval/tests/test_tc_partc_audit.py
"""Tests for Part-C live run auditability and re-run-safety improvements.

Changes tested:
  1. Run-id namespace + collision guard (--run-id / --force-new for run-partc)
  2. manifest.json written per run-id with required fields
  3. Exact prompts persisted (.off.prompt.txt / .on.prompt.txt)
  4. Pre-warm telemetry returned from _prewarm_cpg + surfaced on IsolatedArmResult
  5. Full per-cite citation verdicts on PartCCell + persisted JSON
  6. Explicit gate decision on PartCCell (keep/discard + reason)
  7. Stage-aware file names (.out.md instead of .spec.md for plan stage)
  8. Per-cite judge artifacts (.judge.jsonl with prompt+response+relevant)
  9. Manifest accuracy: real config_summary, full 64-hex sha, dirty_git, thinking_enabled
"""
from __future__ import annotations

import json
import os
import types
from pathlib import Path

import pytest

from tier_c.model import Dose, ArmOutput, Variant, Citation
from tier_c.investigator import InvestigatorReport, CitationVerdict
from tier_c.partc import PartCCell, run_partc_cell


# ---------------------------------------------------------------------------
# Helper: fake ArmOutput with raw_stdout
# ---------------------------------------------------------------------------

def _arm_output(*, prism: bool, text: str = "spec text", raw: str = "",
                prism_calls: int = 0) -> ArmOutput:
    return ArmOutput(
        variant=Variant("opus-4.8", prism),
        text=text,
        citations=[Citation(file="src/a.go", line=1, symbol=None)],
        tokens=5,
        tool_calls=prism_calls,
        wall_s=0.0,
        used_prism=prism_calls > 0,
        prism_calls=prism_calls,
        dose=Dose(count=prism_calls),
        low_dose=(0 < prism_calls <= 1),
        raw_stdout=raw,
    )


def _verdict(*, file_ok=True, line_ok=True, symbol_ok=True, relevant=True) -> CitationVerdict:
    return CitationVerdict(
        cite=Citation(file="src/a.go", line=1, symbol=None),
        file_ok=file_ok, line_ok=line_ok, symbol_ok=symbol_ok, relevant=relevant,
    )


def _report(precision: float, verdicts: list) -> InvestigatorReport:
    return InvestigatorReport(
        precision=precision, recall=precision, hallucinations=0, verdicts=verdicts
    )


class _FakeComps:
    """Fake comps bundle for run_partc_cell."""

    def __init__(self, off_out, on_out, off_report=None, on_report=None,
                 off_prompt="OFF PROMPT", on_prompt="ON PROMPT"):
        self._off_out = off_out
        self._on_out = on_out
        self._off_report = off_report or _report(0.5, [_verdict()])
        self._on_report = on_report or _report(0.7, [_verdict()])
        self._call = 0
        # Prompts that arms would use (for prompt-retention tests)
        self._last_off_prompt = off_prompt
        self._last_on_prompt = on_prompt

    def run_off_arm(self, cell):
        self._last_off = self._off_out
        return self._off_out

    def run_on_arm(self, cell):
        self._last_on = self._on_out
        return self._on_out

    def score(self, citations, **kwargs):
        self._call += 1
        if self._call == 1:
            return self._off_report
        return self._on_report


# ---------------------------------------------------------------------------
# 1. Run-id namespace: --run-id X → artifacts go to runs/partc/<run-id>/
# ---------------------------------------------------------------------------

def test_run_partc_run_id_namespaces_artifacts(tmp_path, monkeypatch):
    """--run-id X makes _persist_partc_cell write under runs/partc/X/."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off text", raw='{"off":1}')
    on_out = _arm_output(prism=True, text="on text", raw='{"on":1}', prism_calls=2)
    comps = _FakeComps(off_out, on_out)

    # Patch _LivePartCComps and Checkout to avoid live IO
    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: comps)

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake/root")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)

    # Patch load_issues
    issue = types.SimpleNamespace(
        key="ruff-1", language="rust", repo="ruff", sha="abc123",
        url="http://example.com", text="issue body", scoped_slice="slice 1",
    )
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [issue])

    # Patch render_partc to avoid print
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")

    # Use a custom runs root
    runs_root = tmp_path / "runs"
    runs_root.mkdir()

    # Patch _persist_partc_cell and _persist_partc_arm_files to write to tmp_path
    written_cells = []
    original_persist_cell = cli_mod._persist_partc_cell

    def fake_persist_cell(cell, repo, stage, model, *, run_id=None, runs_root=None):
        written_cells.append((run_id, runs_root, repo, stage, model))
        path = (Path(runs_root) / run_id / f"{repo}-{stage}-{model}.json") if run_id and runs_root else Path("/dev/null")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"cell": "fake"}))
        return str(path)

    monkeypatch.setattr(cli_mod, "_persist_partc_cell", fake_persist_cell)
    monkeypatch.setattr(cli_mod, "_persist_partc_arm_files", lambda **kw: [])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id="test-run-001",
        runs_root=str(runs_root),
    )

    assert any(r[0] == "test-run-001" for r in written_cells), (
        f"run_id 'test-run-001' must be passed to _persist_partc_cell; got {written_cells}"
    )


def test_run_partc_collision_guard_errors_on_existing_run_id(tmp_path, monkeypatch):
    """If runs/partc/<run-id>/ exists and --force-new is not set, raise FileExistsError."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off")
    on_out = _arm_output(prism=True, text="on", prism_calls=2)
    comps = _FakeComps(off_out, on_out)

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: comps)

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    issue = types.SimpleNamespace(
        key="x", language="rust", repo="ruff", sha="abc", url="u", text="t", scoped_slice="s",
    )
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [issue])

    runs_root = tmp_path / "runs"
    # Pre-create the collision dir
    (runs_root / "test-collision").mkdir(parents=True)

    with pytest.raises(FileExistsError):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id="test-collision",
            runs_root=str(runs_root),
            force_new=False,
        )


def test_run_partc_force_new_overwrites_existing(tmp_path, monkeypatch):
    """--force-new allows re-using an existing run-id (overwrites)."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off")
    on_out = _arm_output(prism=True, text="on", prism_calls=2)
    comps = _FakeComps(off_out, on_out)

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: comps)

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    issue = types.SimpleNamespace(
        key="x", language="rust", repo="ruff", sha="abc", url="u", text="t", scoped_slice="s",
    )
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [issue])
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")
    monkeypatch.setattr(cli_mod, "_persist_partc_cell",
                        lambda cell, repo, stage, model, **kw: "/dev/null")
    monkeypatch.setattr(cli_mod, "_persist_partc_arm_files", lambda **kw: [])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    runs_root = tmp_path / "runs"
    (runs_root / "existing-run").mkdir(parents=True)

    # Must not raise
    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id="existing-run",
        runs_root=str(runs_root),
        force_new=True,
    )


def test_run_partc_default_run_id_uses_timestamp(monkeypatch):
    """When --run-id not supplied, a timestamp-based id is generated (partc-YYYYmmdd-HHMMSS)."""
    import tier_c.cli as cli_mod
    from datetime import datetime

    # The CLI generates a default id if not given; test the generator function
    # by calling with run_id=None and checking the generated id follows the pattern
    generated = []
    original_live = cli_mod._run_partc_live

    def capturing_live(cell, *, bench_root, base_root, issues_path,
                       run_id=None, runs_root=None, force_new=False):
        generated.append(run_id)

    monkeypatch.setattr(cli_mod, "_run_partc_live", capturing_live)

    # Simulate CLI invocation with no --run-id (None → auto-generate)
    # Call the auto-id generator directly
    id_ = cli_mod._default_partc_run_id()
    assert id_.startswith("partc-"), f"default run-id must start with 'partc-'; got {id_!r}"
    # e.g. partc-20260628-153000
    import re
    assert re.match(r"partc-\d{8}-\d{6}", id_), f"format must be partc-YYYYmmdd-HHMMSS; got {id_!r}"


# ---------------------------------------------------------------------------
# 2. manifest.json written per run with required fields
# ---------------------------------------------------------------------------

def test_write_partc_manifest_creates_file_with_required_fields(tmp_path, monkeypatch):
    """_write_partc_manifest writes manifest.json under <runs_root>/<run_id>/."""
    import tier_c.cli as cli_mod
    import hashlib

    # Fake binary to hash
    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"fake binary content")
    fake_prism_bin = tmp_path / "prism"
    fake_prism_bin.write_bytes(b"fake prism content")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism_bin), raising=False)

    issue = types.SimpleNamespace(
        key="ruff-1", language="rust", repo="ruff", sha="deadbeef123",
        url="http://example.com",
    )
    cell = ("ruff", "spec", "opus-4.8")
    run_id = "manifest-test-run"
    runs_root = tmp_path / "runs"

    cli_mod._write_partc_manifest(
        cell=cell,
        issue=issue,
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id=run_id,
        runs_root=str(runs_root),
    )

    manifest_path = runs_root / run_id / "manifest.json"
    assert manifest_path.exists(), f"manifest.json must be written to {manifest_path}"

    data = json.loads(manifest_path.read_text())

    # Required fields
    assert "timestamp" in data, "manifest must have 'timestamp'"
    assert "cell" in data, "manifest must have 'cell' (repo/stage/model)"
    assert data["cell"]["repo"] == "ruff"
    assert data["cell"]["stage"] == "spec"
    assert data["cell"]["model"] == "opus-4.8"

    assert "issue" in data, "manifest must have 'issue'"
    assert data["issue"]["sha"] == "deadbeef123", "manifest must include issue sha"
    assert data["issue"]["key"] == "ruff-1"

    assert "bench_root" in data
    assert "base_root" in data
    assert "issues_path" in data

    assert "prism_mcp_bin" in data, "manifest must include prism_mcp_bin path"
    assert "prism_mcp_sha" in data, "manifest must include prism_mcp_sha (sha256)"
    assert data["prism_mcp_sha"].startswith("sha256:")

    assert "prism_bin" in data, "manifest must include prism_bin path"
    assert "prism_bin_sha" in data, "manifest must include prism_bin_sha"

    assert "harness_git_sha" in data, "manifest must include harness_git_sha"

    assert "config_summary" in data, "manifest must include config_summary"
    config = data["config_summary"]
    assert "on" in config, "config_summary must have 'on' arm config"
    assert "off" in config, "config_summary must have 'off' arm config"

    assert "steer" in data, "manifest must include steer per arm"
    assert data["steer"]["on"] == "prism_on"
    assert data["steer"]["off"] == ""


# ---------------------------------------------------------------------------
# 3. Exact prompts persisted
# ---------------------------------------------------------------------------

def test_live_partc_comps_retains_off_prompt(monkeypatch):
    """_LivePartCComps must set self._last_off_prompt when run_off_arm builds the prompt."""
    import tier_c.cli as cli_mod
    import tier_c.prompts as prompts_mod

    captured_prompt = []

    def fake_stage_prompt(stage, *, issue_text, scoped_slice, upstream="", steer=""):
        p = f"PROMPT:{stage}:{issue_text}:{steer}"
        captured_prompt.append(("off" if not steer else "on", p))
        return p

    monkeypatch.setattr(prompts_mod, "stage_prompt", fake_stage_prompt)
    monkeypatch.setattr(cli_mod, "stage_prompt", fake_stage_prompt, raising=False)

    class _FakeRunner:
        def run(self, variant, stage, prompt, repo_root):
            return _arm_output(prism=variant.prism, text="arm out", prism_calls=0)

    class _FakeRunIsolated:
        pass

    # Patch run_arm_isolated
    def fake_run_arm_isolated(runner, *, checkout, variant, stage, prompt, no_cache, prewarm):
        from tier_c.arm_runner import IsolatedArmResult
        out = _arm_output(prism=variant.prism, text="out", prism_calls=2 if variant.prism else 0)
        return IsolatedArmResult(out=out, cache_mode="cached", mcp_args=[])

    monkeypatch.setattr(cli_mod, "run_arm_isolated", fake_run_arm_isolated, raising=False)

    import tier_c.arm_runner as arm_mod
    monkeypatch.setattr(arm_mod, "run_arm_isolated", fake_run_arm_isolated)

    issue = types.SimpleNamespace(
        text="ISSUE-TEXT", scoped_slice="slice 1", repo="ruff", sha="abc",
    )

    class _FakeCo:
        root = "/fake"

    comps = cli_mod._LivePartCComps(
        co=_FakeCo(), issue=issue, model="opus-4.8", base_root="/base",
    )
    # Patch _upstream_spec
    comps._upstream_spec = lambda cell: ""

    comps.run_off_arm(("ruff", "spec", "opus-4.8"))
    assert hasattr(comps, "_last_off_prompt"), "_LivePartCComps must set _last_off_prompt"
    assert comps._last_off_prompt is not None
    assert "ISSUE-TEXT" in comps._last_off_prompt or comps._last_off_prompt != ""


def test_live_partc_comps_retains_on_prompt(monkeypatch):
    """_LivePartCComps must set self._last_on_prompt when run_on_arm builds the prompt."""
    import tier_c.cli as cli_mod
    import tier_c.prompts as prompts_mod

    def fake_stage_prompt(stage, *, issue_text, scoped_slice, upstream="", steer=""):
        return f"PROMPT:{stage}:{issue_text}:{steer}"

    monkeypatch.setattr(prompts_mod, "stage_prompt", fake_stage_prompt)
    monkeypatch.setattr(cli_mod, "stage_prompt", fake_stage_prompt, raising=False)

    def fake_run_arm_isolated(runner, *, checkout, variant, stage, prompt, no_cache, prewarm):
        from tier_c.arm_runner import IsolatedArmResult
        out = _arm_output(prism=variant.prism, text="out", prism_calls=2 if variant.prism else 0)
        return IsolatedArmResult(out=out, cache_mode="cached", mcp_args=[])

    import tier_c.arm_runner as arm_mod
    monkeypatch.setattr(arm_mod, "run_arm_isolated", fake_run_arm_isolated)
    monkeypatch.setattr(cli_mod, "run_arm_isolated", fake_run_arm_isolated, raising=False)

    issue = types.SimpleNamespace(
        text="ISSUE-TEXT", scoped_slice="slice 1", repo="ruff", sha="abc",
    )

    class _FakeCo:
        root = "/fake"

    comps = cli_mod._LivePartCComps(
        co=_FakeCo(), issue=issue, model="opus-4.8", base_root="/base",
    )
    comps._upstream_spec = lambda cell: ""

    comps.run_on_arm(("ruff", "spec", "opus-4.8"))
    assert hasattr(comps, "_last_on_prompt"), "_LivePartCComps must set _last_on_prompt"
    assert comps._last_on_prompt is not None


def test_run_partc_live_writes_prompt_files(tmp_path, monkeypatch):
    """_run_partc_live writes .off.prompt.txt and .on.prompt.txt under the run-id dir."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off text", raw="")
    on_out = _arm_output(prism=True, text="on text", raw="", prism_calls=2)

    class _FakeComps2:
        _last_off = off_out
        _last_on = on_out
        _last_off_prompt = "THE OFF PROMPT TEXT"
        _last_on_prompt = "THE ON PROMPT TEXT"

        def run_off_arm(self, cell): return off_out
        def run_on_arm(self, cell): return on_out
        def score(self, citations, **kwargs):
            return _report(0.5, [_verdict()])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FakeComps2())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    issue = types.SimpleNamespace(
        key="ruff-1", language="rust", repo="ruff", sha="abc",
        url="u", text="t", scoped_slice="s",
    )
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [issue])
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")

    runs_root = tmp_path / "runs"
    run_id = "prompt-test-run"

    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id=run_id,
        runs_root=str(runs_root),
    )

    run_dir = runs_root / run_id
    off_prompt = run_dir / "ruff-spec-opus-4.8.off.prompt.txt"
    on_prompt = run_dir / "ruff-spec-opus-4.8.on.prompt.txt"

    assert off_prompt.exists(), f"off.prompt.txt must be written: {off_prompt}"
    assert on_prompt.exists(), f"on.prompt.txt must be written: {on_prompt}"
    assert off_prompt.read_text() == "THE OFF PROMPT TEXT"
    assert on_prompt.read_text() == "THE ON PROMPT TEXT"


# ---------------------------------------------------------------------------
# 4. Pre-warm telemetry
# ---------------------------------------------------------------------------

def test_prewarm_cpg_returns_telemetry_dict(monkeypatch):
    """_prewarm_cpg must return a telemetry dict (not None) with required keys."""
    import tier_c.arm_runner as arm_mod

    def fake_run(cmd, **kwargs):
        class R:
            returncode = 0
            stdout = "warmup ok output"
            stderr = "warmup stderr"
        return R()

    monkeypatch.setattr(arm_mod.subprocess, "run", fake_run)
    result = arm_mod._prewarm_cpg("/tmp/repo")

    assert isinstance(result, dict), f"_prewarm_cpg must return dict; got {type(result)}"
    assert "argv" in result, "prewarm dict must have 'argv'"
    assert "returncode" in result, "prewarm dict must have 'returncode'"
    assert "stdout" in result, "prewarm dict must have 'stdout'"
    assert "stderr" in result, "prewarm dict must have 'stderr'"
    assert "wall_s" in result, "prewarm dict must have 'wall_s'"
    assert "exception" in result, "prewarm dict must have 'exception'"
    assert result["returncode"] == 0
    assert result["exception"] is None


def test_prewarm_cpg_captures_stdout_truncated(monkeypatch):
    """_prewarm_cpg stdout is captured and truncated to 4000 chars."""
    import tier_c.arm_runner as arm_mod

    long_output = "x" * 6000

    def fake_run(cmd, **kwargs):
        class R:
            returncode = 0
            stdout = long_output
            stderr = ""
        return R()

    monkeypatch.setattr(arm_mod.subprocess, "run", fake_run)
    result = arm_mod._prewarm_cpg("/tmp/repo")
    assert len(result["stdout"]) <= 4000, "stdout must be truncated to 4000 chars"
    assert result["stdout"] == long_output[:4000]


def test_prewarm_cpg_captures_exception(monkeypatch):
    """_prewarm_cpg captures exceptions in the telemetry dict (never raises)."""
    import tier_c.arm_runner as arm_mod

    def bang(*a, **kw):
        raise TimeoutError("prism timed out")

    monkeypatch.setattr(arm_mod.subprocess, "run", bang)
    result = arm_mod._prewarm_cpg("/tmp/repo")
    assert result is not None
    assert result["exception"] is not None
    assert "TimeoutError" in result["exception"] or "timed out" in result["exception"]
    assert result["returncode"] == -1 or result["returncode"] is None


def test_isolated_arm_result_carries_prewarm(tmp_path, monkeypatch):
    """run_arm_isolated returns IsolatedArmResult with .prewarm dict when prewarm=True."""
    import types
    import unittest.mock as mock
    import tier_c.arm_runner as arm_mod
    from tier_c.model import Dose as _Dose, ArmOutput as _ArmOutput

    # Set up minimal git checkout
    root = tmp_path / "repo"
    root.mkdir()
    import subprocess as real_subprocess
    real_subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "README.md").write_text("hello\n")
    real_subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    real_subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                         "commit", "-q", "-m", "init"], cwd=root, check=True)
    co = types.SimpleNamespace(root=root)

    fake_prewarm_result = {
        "argv": ["prism", "nav", "repo-map", "--repo", str(root)],
        "returncode": 0,
        "stdout": "ok",
        "stderr": "",
        "wall_s": 1.23,
        "exception": None,
    }
    monkeypatch.setattr(arm_mod, "_prewarm_cpg",
                        lambda root: fake_prewarm_result)

    class _NoopRunner:
        def run(self, variant, stage, prompt, repo_root):
            return _ArmOutput(
                variant=variant, text="ok", citations=[], tokens=0,
                tool_calls=0, wall_s=0.0, used_prism=False,
                prism_calls=0, dose=_Dose(), low_dose=False,
            )

    variant = Variant("opus-4.8", True)
    result = arm_mod.run_arm_isolated(
        _NoopRunner(),
        checkout=co,
        variant=variant,
        prewarm=True,
    )

    assert hasattr(result, "prewarm"), "IsolatedArmResult must have .prewarm attribute"
    assert result.prewarm is not None, "prewarm must not be None when prewarm=True"
    assert result.prewarm["wall_s"] == pytest.approx(1.23)
    assert result.prewarm["returncode"] == 0


def test_isolated_arm_result_prewarm_none_when_not_prewarmed(tmp_path):
    """run_arm_isolated returns prewarm=None when prewarm=False (default)."""
    import types
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

    class _NoopRunner:
        def run(self, variant, stage, prompt, repo_root):
            return _ArmOutput(
                variant=variant, text="ok", citations=[], tokens=0,
                tool_calls=0, wall_s=0.0, used_prism=False,
                prism_calls=0, dose=_Dose(), low_dose=False,
            )

    result = arm_mod.run_arm_isolated(
        _NoopRunner(), checkout=co, variant=Variant("opus-4.8", True), prewarm=False,
    )
    assert result.prewarm is None, "prewarm must be None when prewarm=False"


def test_run_partc_live_writes_prewarm_json(tmp_path, monkeypatch):
    """_run_partc_live writes <run_id>/<repo>-<stage>-<model>.on.prewarm.json."""
    import tier_c.cli as cli_mod

    prewarm_data = {"argv": ["prism", "nav"], "returncode": 0,
                    "stdout": "ok", "stderr": "", "wall_s": 2.5, "exception": None}

    off_out = _arm_output(prism=False, text="off")
    on_out = _arm_output(prism=True, text="on", prism_calls=2)

    class _FakeComps2:
        _last_off = off_out
        _last_on = on_out
        _last_off_prompt = "off prompt"
        _last_on_prompt = "on prompt"
        _last_prewarm = prewarm_data

        def run_off_arm(self, cell): return off_out
        def run_on_arm(self, cell): return on_out
        def score(self, citations, **kwargs): return _report(0.5, [_verdict()])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FakeComps2())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    issue = types.SimpleNamespace(
        key="ruff-1", language="rust", repo="ruff", sha="abc",
        url="u", text="t", scoped_slice="s",
    )
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [issue])
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")

    runs_root = tmp_path / "runs"
    run_id = "prewarm-test-run"

    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id=run_id,
        runs_root=str(runs_root),
    )

    prewarm_file = runs_root / run_id / "ruff-spec-opus-4.8.on.prewarm.json"
    assert prewarm_file.exists(), f"on.prewarm.json must be written: {prewarm_file}"
    data = json.loads(prewarm_file.read_text())
    assert data["wall_s"] == pytest.approx(2.5)
    assert data["returncode"] == 0


# ---------------------------------------------------------------------------
# 5. Full per-cite citation verdicts on PartCCell
# ---------------------------------------------------------------------------

def test_partc_cell_has_off_verdicts_field():
    """PartCCell must have off_verdicts field (list of per-cite dicts)."""
    v = _verdict()
    off_rep = _report(0.5, [v])
    on_rep = _report(0.7, [_verdict(relevant=False)])

    class _C:
        _call = 0
        _last_off = None
        _last_on = None

        def run_off_arm(self, cell):
            out = _arm_output(prism=False, text="off")
            self._last_off = out
            return out

        def run_on_arm(self, cell):
            out = _arm_output(prism=True, text="on", prism_calls=2)
            self._last_on = out
            return out

        def score(self, citations, **kwargs):
            self._call += 1
            return off_rep if self._call == 1 else on_rep

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _C())
    assert hasattr(cell, "off_verdicts"), "PartCCell must have off_verdicts field"
    assert isinstance(cell.off_verdicts, list), "off_verdicts must be a list"


def test_partc_cell_off_verdicts_contain_per_cite_fields():
    """off_verdicts items must be dicts with file/line/symbol/file_ok/line_ok/symbol_ok/relevant."""
    v1 = CitationVerdict(
        cite=Citation(file="src/a.go", line=5, symbol="Foo"),
        file_ok=True, line_ok=True, symbol_ok=True, relevant=True,
    )
    v2 = CitationVerdict(
        cite=Citation(file="src/b.go", line=10, symbol=None),
        file_ok=False, line_ok=False, symbol_ok=True, relevant=True,
    )
    off_rep = InvestigatorReport(precision=0.5, recall=0.5, hallucinations=1, verdicts=[v1, v2])
    on_rep = _report(0.8, [_verdict()])

    class _C:
        _call = 0
        _last_off = None
        _last_on = None

        def run_off_arm(self, cell):
            out = _arm_output(prism=False, text="off")
            self._last_off = out
            return out

        def run_on_arm(self, cell):
            out = _arm_output(prism=True, text="on", prism_calls=2)
            self._last_on = out
            return out

        def score(self, citations, **kwargs):
            self._call += 1
            return off_rep if self._call == 1 else on_rep

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _C())

    assert len(cell.off_verdicts) == 2, f"expected 2 off_verdicts, got {len(cell.off_verdicts)}"
    v_dict = cell.off_verdicts[0]
    assert "file" in v_dict, "verdict dict must have 'file'"
    assert "line" in v_dict, "verdict dict must have 'line'"
    assert "symbol" in v_dict, "verdict dict must have 'symbol'"
    assert "file_ok" in v_dict, "verdict dict must have 'file_ok'"
    assert "line_ok" in v_dict, "verdict dict must have 'line_ok'"
    assert "symbol_ok" in v_dict, "verdict dict must have 'symbol_ok'"
    assert "relevant" in v_dict, "verdict dict must have 'relevant'"
    assert "is_hallucination" in v_dict, "verdict dict must have 'is_hallucination'"
    assert "is_valid" in v_dict, "verdict dict must have 'is_valid'"

    # Check values
    assert v_dict["file"] == "src/a.go"
    assert v_dict["line"] == 5
    assert v_dict["file_ok"] is True
    assert v_dict["is_valid"] is True

    # Second verdict: hallucination
    v2_dict = cell.off_verdicts[1]
    assert v2_dict["file"] == "src/b.go"
    assert v2_dict["file_ok"] is False
    assert v2_dict["is_hallucination"] is True


def test_partc_cell_on_verdicts_present():
    """PartCCell must have on_verdicts field matching on-arm score() verdicts."""
    on_v = CitationVerdict(
        cite=Citation(file="src/c.go", line=99, symbol="Bar"),
        file_ok=True, line_ok=True, symbol_ok=True, relevant=False,
    )
    on_rep = InvestigatorReport(precision=0.0, recall=0.0, hallucinations=0, verdicts=[on_v])
    off_rep = _report(0.5, [_verdict()])

    class _C:
        _call = 0
        _last_off = None
        _last_on = None

        def run_off_arm(self, cell):
            out = _arm_output(prism=False, text="off")
            self._last_off = out
            return out

        def run_on_arm(self, cell):
            out = _arm_output(prism=True, text="on", prism_calls=2)
            self._last_on = out
            return out

        def score(self, citations, **kwargs):
            self._call += 1
            return off_rep if self._call == 1 else on_rep

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _C())
    assert len(cell.on_verdicts) == 1
    v = cell.on_verdicts[0]
    assert v["file"] == "src/c.go"
    assert v["relevant"] is False
    assert v["is_valid"] is False


def test_partc_cell_verdicts_in_persisted_json(tmp_path):
    """_persist_partc_cell must include off_verdicts/on_verdicts in the JSON output."""
    import tier_c.cli as cli_mod
    from tier_c.partc import PartCCell

    on_v = CitationVerdict(
        cite=Citation(file="a.go", line=1, symbol=None),
        file_ok=True, line_ok=True, symbol_ok=True, relevant=True,
    )
    cell = PartCCell(
        repo="ruff", stage="spec", model="opus-4.8",
        precision_on=0.8, precision_base=0.5, bundle_delta=0.3,
        dose=Dose(count=2), low_dose=False, administered=True, leaked=False,
        recall_on=None, recall_base=None,
        off_verdicts=[{"file": "b.go", "line": 2, "symbol": None,
                       "file_ok": True, "line_ok": True, "symbol_ok": True,
                       "relevant": True, "is_hallucination": False, "is_valid": True}],
        on_verdicts=[{"file": "a.go", "line": 1, "symbol": None,
                      "file_ok": True, "line_ok": True, "symbol_ok": True,
                      "relevant": True, "is_hallucination": False, "is_valid": True}],
    )

    runs_root = str(tmp_path / "runs")
    path = cli_mod._persist_partc_cell(
        cell, "ruff", "spec", "opus-4.8", run_id="v-test", runs_root=runs_root
    )
    data = json.loads(Path(path).read_text())
    assert "off_verdicts" in data, "persisted JSON must include off_verdicts"
    assert "on_verdicts" in data, "persisted JSON must include on_verdicts"
    assert isinstance(data["off_verdicts"], list)
    assert len(data["off_verdicts"]) == 1
    assert data["off_verdicts"][0]["file"] == "b.go"


# ---------------------------------------------------------------------------
# 6. Explicit gate decision
# ---------------------------------------------------------------------------

def test_partc_cell_gate_keep_when_prism_calls_gt_zero():
    """PartCCell gate.decision == 'keep' when on-arm prism_calls > 0."""
    class _C:
        _call = 0
        _last_off = None
        _last_on = None

        def run_off_arm(self, cell):
            out = _arm_output(prism=False, text="off")
            self._last_off = out
            return out

        def run_on_arm(self, cell):
            out = _arm_output(prism=True, text="on", prism_calls=3)
            self._last_on = out
            return out

        def score(self, citations, **kwargs):
            self._call += 1
            rep = _report(0.5, [_verdict()]) if self._call == 1 else _report(0.8, [_verdict()])
            return rep

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _C())
    assert hasattr(cell, "gate"), "PartCCell must have .gate attribute"
    assert cell.gate["decision"] == "keep", (
        f"gate.decision must be 'keep' when prism_calls > 0; got {cell.gate}"
    )
    assert "prism_calls" in cell.gate
    assert cell.gate["prism_calls"] == 3


def test_partc_cell_gate_discard_when_zero_prism_calls():
    """PartCCell gate.decision == 'discard' when on-arm prism_calls == 0."""
    class _C:
        _call = 0
        _last_off = None
        _last_on = None

        def run_off_arm(self, cell):
            out = _arm_output(prism=False, text="off")
            self._last_off = out
            return out

        def run_on_arm(self, cell):
            out = _arm_output(prism=True, text="on", prism_calls=0)  # no prism!
            self._last_on = out
            return out

        def score(self, citations, **kwargs):
            self._call += 1
            return _report(0.5, [_verdict()])

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _C())
    assert cell.gate["decision"] == "discard", (
        f"gate.decision must be 'discard' when prism_calls == 0; got {cell.gate}"
    )
    assert "zero real prism calls" in cell.gate["reason"].lower()


def test_partc_cell_gate_keep_includes_distinct_tools():
    """gate must include 'distinct_tools' list."""
    class _C:
        _call = 0
        _last_off = None
        _last_on = None

        def run_off_arm(self, cell):
            out = _arm_output(prism=False, text="off")
            self._last_off = out
            return out

        def run_on_arm(self, cell):
            from tier_c.model import Dose as _Dose
            out = ArmOutput(
                variant=Variant("opus-4.8", True),
                text="on", citations=[], tokens=5, tool_calls=2, wall_s=0.0,
                used_prism=True, prism_calls=2,
                dose=_Dose(count=2, distinct_tools=frozenset({"nav_callers", "nav_callees"})),
                low_dose=False,
            )
            self._last_on = out
            return out

        def score(self, citations, **kwargs):
            self._call += 1
            return _report(0.7, [_verdict()])

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _C())
    assert "distinct_tools" in cell.gate
    assert isinstance(cell.gate["distinct_tools"], list)


def test_partc_cell_gate_in_persisted_json(tmp_path):
    """_persist_partc_cell must include 'gate' in the persisted JSON."""
    import tier_c.cli as cli_mod
    from tier_c.partc import PartCCell

    cell = PartCCell(
        repo="ruff", stage="spec", model="opus-4.8",
        precision_on=0.8, precision_base=0.5, bundle_delta=0.3,
        dose=Dose(count=2), low_dose=False, administered=True, leaked=False,
        recall_on=None, recall_base=None,
        gate={"decision": "keep", "reason": "prism used", "prism_calls": 2,
              "distinct_tools": ["nav_callers"]},
    )

    runs_root = str(tmp_path / "runs")
    path = cli_mod._persist_partc_cell(
        cell, "ruff", "spec", "opus-4.8", run_id="gate-test", runs_root=runs_root
    )
    data = json.loads(Path(path).read_text())
    assert "gate" in data, "persisted JSON must include 'gate'"
    assert data["gate"]["decision"] == "keep"


# ---------------------------------------------------------------------------
# 7. Stage-aware file names (.out.md instead of .spec.md)
# ---------------------------------------------------------------------------

def test_persist_arm_files_uses_out_md_extension(tmp_path):
    """_persist_partc_arm_files uses <repo>-<stage>-<model>.off.out.md (not .spec.md)."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off spec text", raw="offraw\n")
    on_out = _arm_output(prism=True, text="on spec text", raw="onraw\n", prism_calls=2)

    runs_dir = str(tmp_path / "runs")
    paths = cli_mod._persist_partc_arm_files(
        off_out=off_out, on_out=on_out,
        repo="ruff", stage="spec", model="opus-4.8",
        runs_dir=runs_dir,
    )
    names = [Path(p).name for p in paths]
    # Must NOT use .spec.md
    assert not any(n.endswith(".spec.md") for n in names), (
        f"arm files must not use .spec.md; got {names}"
    )
    # Must use .out.md
    assert any(n.endswith(".off.out.md") for n in names), (
        f"arm files must use .off.out.md; got {names}"
    )
    assert any(n.endswith(".on.out.md") for n in names), (
        f"arm files must use .on.out.md; got {names}"
    )


def test_persist_arm_files_plan_stage_uses_out_md(tmp_path):
    """_persist_partc_arm_files uses .out.md for plan stage too."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off plan text", raw="")
    on_out = _arm_output(prism=True, text="on plan text", raw="", prism_calls=2)

    runs_dir = str(tmp_path / "runs")
    paths = cli_mod._persist_partc_arm_files(
        off_out=off_out, on_out=on_out,
        repo="ruff", stage="plan", model="opus-4.8",
        runs_dir=runs_dir,
    )
    names = [Path(p).name for p in paths]
    assert any(n.endswith(".off.out.md") for n in names), f"plan stage must use .off.out.md; {names}"
    assert any(n.endswith(".on.out.md") for n in names), f"plan stage must use .on.out.md; {names}"
    # Content must be correct
    for p in paths:
        name = Path(p).name
        if name.endswith(".off.out.md"):
            assert Path(p).read_text() == "off plan text"
        elif name.endswith(".on.out.md"):
            assert Path(p).read_text() == "on plan text"


# ---------------------------------------------------------------------------
# 8. Per-cite judge artifacts (.judge.jsonl with prompt+response+relevant)
# ---------------------------------------------------------------------------

def test_recording_relevance_judge_captures_prompt_and_response():
    """_RecordingRelevanceJudge must record (cite, prompt, response, relevant) per call."""
    from tier_c.judges_live import LlmRelevanceJudge, _RecordingRelevanceJudge
    from tier_c.model import Citation as _Cit

    calls: list[tuple] = []

    def fake_ask(model, prompt):
        calls.append(("ask", model, prompt))
        return "YES"

    inner = LlmRelevanceJudge(fake_ask, "opus-4.8")
    records: list[dict] = []
    cite = _Cit(file="src/a.go", line=10, symbol="Foo")
    recorder = _RecordingRelevanceJudge(inner, records)

    result = recorder.is_relevant(cite, "issue text", "def foo(): ...")
    assert result is True
    assert len(records) == 1
    r = records[0]
    assert r["file"] == "src/a.go"
    assert r["line"] == 10
    assert r["symbol"] == "Foo"
    assert "issue text" in r["prompt"]
    assert r["response"] == "YES"
    assert r["relevant"] is True


def test_recording_relevance_judge_captures_no_response():
    """NO response → relevant=False, still recorded."""
    from tier_c.judges_live import LlmRelevanceJudge, _RecordingRelevanceJudge
    from tier_c.model import Citation as _Cit

    inner = LlmRelevanceJudge(lambda m, p: "NO, unrelated", "opus-4.8")
    records: list[dict] = []
    cite = _Cit(file="x.py", line=5, symbol=None)
    recorder = _RecordingRelevanceJudge(inner, records)

    result = recorder.is_relevant(cite, "issue", "")
    assert result is False
    assert len(records) == 1
    assert records[0]["relevant"] is False
    assert records[0]["symbol"] is None


def test_live_partc_comps_score_produces_judge_records(monkeypatch):
    """_LivePartCComps.score populates self._last_off_judge / _last_on_judge with cite records."""
    import tier_c.cli as cli_mod
    import tier_c.investigator as inv_mod

    ask_log: list[tuple] = []

    def fake_ask(model, prompt):
        ask_log.append((model, prompt))
        return "YES"

    # Build a checkout fake with resolve_rel + read_window.
    # read_line must return a string that CONTAINS the cite's symbol so symbol_ok=True
    # and relevance.is_relevant is always called (otherwise no record is appended).
    class _FakeCo:
        root = "/fake"

        def resolve_rel(self, f):
            return f

        def file_exists(self, f):
            return True

        def read_line(self, f, ln):
            # Return a line that contains every possible symbol so symbol_ok is always True
            return "func Foo() Bar { }"

        def read_window(self, f, ln):
            return "def foo(): ..."

    issue = types.SimpleNamespace(
        text="ISSUE BODY", scoped_slice="slice 1", repo="ruff", sha="abc",
    )

    comps = cli_mod._LivePartCComps(
        co=_FakeCo(), issue=issue, model="opus-4.8",
        base_root="/base", ask=fake_ask,
    )
    # Force _upstream_spec to return "" (spec cell)
    comps._upstream_spec = lambda cell: ""

    from tier_c.model import Citation as _Cit
    cites = [
        _Cit(file="src/a.go", line=10, symbol="Foo"),
        _Cit(file="src/b.go", line=20, symbol=None),
    ]

    cell = ("ruff", "spec", "opus-4.8")
    comps.score(cites, cell=cell, arm="base")

    # After scoring off-arm, _last_off_judge should have 2 records
    assert hasattr(comps, "_last_off_judge"), (
        "_LivePartCComps.score must set _last_off_judge after arm='base' call"
    )
    assert len(comps._last_off_judge) == 2, (
        f"expected 2 judge records (one per cite), got {len(comps._last_off_judge)}"
    )
    rec = comps._last_off_judge[0]
    assert "file" in rec
    assert "line" in rec
    assert "prompt" in rec
    assert "response" in rec
    assert "relevant" in rec

    # Score the on-arm
    comps.score(cites, cell=cell, arm="on")
    assert hasattr(comps, "_last_on_judge"), (
        "_LivePartCComps.score must set _last_on_judge after arm='on' call"
    )
    assert len(comps._last_on_judge) == 2


def test_run_partc_live_writes_judge_jsonl(tmp_path, monkeypatch):
    """_run_partc_live writes <base>.off.judge.jsonl and <base>.on.judge.jsonl."""
    import tier_c.cli as cli_mod

    off_judge_records = [
        {"file": "src/a.go", "line": 1, "symbol": None,
         "prompt": "Is src/a.go:1 relevant?", "response": "YES", "relevant": True},
    ]
    on_judge_records = [
        {"file": "src/a.go", "line": 1, "symbol": None,
         "prompt": "Is src/a.go:1 relevant?", "response": "NO", "relevant": False},
    ]

    off_out = _arm_output(prism=False, text="off text", raw="")
    on_out = _arm_output(prism=True, text="on text", raw="", prism_calls=2)

    class _FakeComps3:
        _last_off = off_out
        _last_on = on_out
        _last_off_prompt = "off prompt"
        _last_on_prompt = "on prompt"
        _last_off_judge = off_judge_records
        _last_on_judge = on_judge_records

        def run_off_arm(self, cell):
            return off_out

        def run_on_arm(self, cell):
            return on_out

        def score(self, citations, **kwargs):
            return _report(0.5, [_verdict()])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FakeComps3())

    class _FakeCheckout:
        def __init__(self, *a, **kw):
            pass
        def __enter__(self):
            return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a):
            pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    issue = types.SimpleNamespace(
        key="ruff-1", language="rust", repo="ruff", sha="abc",
        url="u", text="t", scoped_slice="s",
    )
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [issue])
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")

    runs_root = tmp_path / "runs"
    run_id = "judge-test-run"

    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id=run_id,
        runs_root=str(runs_root),
    )

    run_dir = runs_root / run_id
    off_judge = run_dir / "ruff-spec-opus-4.8.off.judge.jsonl"
    on_judge = run_dir / "ruff-spec-opus-4.8.on.judge.jsonl"

    assert off_judge.exists(), f"off.judge.jsonl must be written: {off_judge}"
    assert on_judge.exists(), f"on.judge.jsonl must be written: {on_judge}"

    # Each line is a JSON object with required fields
    off_lines = [json.loads(l) for l in off_judge.read_text().splitlines() if l.strip()]
    on_lines = [json.loads(l) for l in on_judge.read_text().splitlines() if l.strip()]
    assert len(off_lines) == 1
    assert len(on_lines) == 1
    assert off_lines[0]["relevant"] is True
    assert on_lines[0]["relevant"] is False
    assert "prompt" in off_lines[0]
    assert "response" in off_lines[0]
    assert "file" in off_lines[0]
    assert "line" in off_lines[0]


def test_judge_jsonl_one_record_per_cite():
    """Each cited location produces exactly one judge record (no duplicates/omissions)."""
    from tier_c.judges_live import LlmRelevanceJudge, _RecordingRelevanceJudge
    from tier_c.model import Citation as _Cit

    responses = ["YES", "NO", "YES"]
    idx = 0

    def fake_ask(model, prompt):
        nonlocal idx
        r = responses[idx]
        idx += 1
        return r

    inner = LlmRelevanceJudge(fake_ask, "opus-4.8")
    records: list[dict] = []
    recorder = _RecordingRelevanceJudge(inner, records)

    cites = [
        _Cit("a.go", 1, None),
        _Cit("b.go", 2, "Bar"),
        _Cit("c.go", 3, None),
    ]
    results = [recorder.is_relevant(c, "issue", "") for c in cites]

    assert len(records) == 3
    assert [r["relevant"] for r in records] == [True, False, True]
    assert records[1]["file"] == "b.go"
    assert records[1]["symbol"] == "Bar"
    assert results == [True, False, True]


# ---------------------------------------------------------------------------
# 9. Manifest accuracy
# ---------------------------------------------------------------------------

def test_manifest_config_summary_on_arm_has_skill_and_mcp_prism(tmp_path, monkeypatch):
    """config_summary.on reflects real prism-ON config: skill_present=True,
    allow_includes_mcp_prism=True, deny=['Write','Edit']."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"bin")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"prism")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="cfg-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "cfg-test" / "manifest.json").read_text())
    on_cfg = data["config_summary"]["on"]
    assert on_cfg["skill_present"] is True
    assert on_cfg["allow_includes_mcp_prism"] is True
    assert "Write" in on_cfg["deny"] and "Edit" in on_cfg["deny"]


def test_manifest_config_summary_off_arm_no_skill_no_mcp_prism(tmp_path, monkeypatch):
    """config_summary.off reflects real prism-OFF config: no skill, no mcp__prism,
    allow=[Read,Grep,Glob,Bash], deny=[Write,Edit]."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"bin")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"prism")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="cfg-off-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "cfg-off-test" / "manifest.json").read_text())
    off_cfg = data["config_summary"]["off"]
    assert off_cfg["skill_present"] is False
    assert off_cfg["allow_includes_mcp_prism"] is False
    assert "mcp__prism" not in off_cfg.get("allow", [])
    assert "Read" in off_cfg["allow"]
    assert "Grep" in off_cfg["allow"]
    assert "Write" in off_cfg["deny"] and "Edit" in off_cfg["deny"]


def test_manifest_codex_sandbox_flag_is_workspace_write(tmp_path, monkeypatch):
    """config_summary must expose the TRUE codex sandbox flag (-s workspace-write),
    not claim 'read-only' or 'deny Write/Edit'."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"bin")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"prism")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="codex-flag-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "codex-flag-test" / "manifest.json").read_text())
    # Must have a codex section in config_summary
    assert "codex" in data["config_summary"], (
        "config_summary must have a 'codex' key describing the codex arm config"
    )
    codex_cfg = data["config_summary"]["codex"]
    assert codex_cfg["sandbox"] == "workspace-write", (
        f"codex sandbox must be 'workspace-write' (the TRUE value from build_codex_cmd); "
        f"got {codex_cfg.get('sandbox')!r}"
    )


def test_manifest_binary_sha_is_full_64_hex(tmp_path, monkeypatch):
    """prism_mcp_sha and prism_bin_sha must be full 64-hex sha256 (not 16-char truncated)."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"fake binary content for sha256 test")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"fake prism binary")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="sha-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "sha-test" / "manifest.json").read_text())
    mcp_sha = data["prism_mcp_sha"]
    bin_sha = data["prism_bin_sha"]

    assert mcp_sha.startswith("sha256:"), f"must start with sha256:; got {mcp_sha!r}"
    hex_part = mcp_sha[len("sha256:"):]
    assert len(hex_part) == 64, (
        f"sha256 hex must be 64 chars (full digest), not truncated; got {len(hex_part)}: {hex_part!r}"
    )

    bin_hex = bin_sha[len("sha256:"):]
    assert len(bin_hex) == 64, (
        f"prism_bin_sha hex must be 64 chars; got {len(bin_hex)}: {bin_hex!r}"
    )


def test_manifest_has_dirty_git_key(tmp_path, monkeypatch):
    """manifest must include 'dirty_git' key (git status --porcelain output or error string)."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"bin")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"prism")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="dirty-git-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "dirty-git-test" / "manifest.json").read_text())
    assert "dirty_git" in data, "manifest must include 'dirty_git' key"
    # Value must be a string (either git output or an error message)
    assert isinstance(data["dirty_git"], str), (
        f"dirty_git must be a string; got {type(data['dirty_git'])}"
    )


def test_manifest_has_thinking_enabled_key(tmp_path, monkeypatch):
    """manifest must include 'thinking_enabled' boolean key."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"bin")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"prism")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="thinking-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "thinking-test" / "manifest.json").read_text())
    assert "thinking_enabled" in data, "manifest must include 'thinking_enabled' key"
    assert isinstance(data["thinking_enabled"], bool), (
        f"thinking_enabled must be bool; got {type(data['thinking_enabled'])}"
    )


def test_manifest_records_codex_reasoning_exposure(tmp_path, monkeypatch):
    """manifest must record codex raw-reasoning exposure (hide=false / show_raw=true)
    in BOTH config_summary.codex and the top-level reasoning_exposure block — these
    mirror the codex config.toml keys so the run's auditability settings are pinned."""
    import tier_c.cli as cli_mod

    fake_bin = tmp_path / "prism-mcp"
    fake_bin.write_bytes(b"bin")
    fake_prism = tmp_path / "prism"
    fake_prism.write_bytes(b"prism")

    monkeypatch.setattr(cli_mod, "_prism_mcp_bin", lambda: str(fake_bin))
    monkeypatch.setattr(cli_mod, "_prism_bin", lambda: str(fake_prism), raising=False)

    issue = types.SimpleNamespace(
        key="r1", language="rust", repo="ruff", sha="abc", url="u",
    )
    runs_root = tmp_path / "runs"
    cli_mod._write_partc_manifest(
        cell=("ruff", "spec", "opus-4.8"), issue=issue,
        bench_root="/bench", base_root="/base", issues_path="i.toml",
        run_id="reasoning-test", runs_root=str(runs_root),
    )

    data = json.loads((runs_root / "reasoning-test" / "manifest.json").read_text())

    codex_cfg = data["config_summary"]["codex"]
    assert codex_cfg["hide_agent_reasoning"] is False, (
        f"config_summary.codex must record hide_agent_reasoning=False; got {codex_cfg!r}")
    assert codex_cfg["show_raw_agent_reasoning"] is True, (
        f"config_summary.codex must record show_raw_agent_reasoning=True; got {codex_cfg!r}")

    assert "reasoning_exposure" in data, "manifest must include a 'reasoning_exposure' block"
    re_block = data["reasoning_exposure"]
    assert re_block["claude"]["show_thinking_summaries"] is True, (
        f"reasoning_exposure.claude must record show_thinking_summaries=True; got {re_block!r}")
    assert re_block["codex"]["hide_agent_reasoning"] is False, (
        f"reasoning_exposure.codex must record hide_agent_reasoning=False; got {re_block!r}")
    assert re_block["codex"]["show_raw_agent_reasoning"] is True, (
        f"reasoning_exposure.codex must record show_raw_agent_reasoning=True; got {re_block!r}")
