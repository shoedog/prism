# eval/tests/test_tc_partc_failure_safe.py
"""Tests for Part-C live run failure-safety and run-id namespacing improvements.

Changes tested:
  1. ArmRunError — structured exception with argv/returncode/stderr/stdout.
  2. Per-arm meta.json written immediately after each arm finishes (try/finally — on FAILURE too).
  3. ArmOutput carries argv/returncode/stderr/cwd on success; ArmRunError carries them on failure.
  4. run-id default → eval/tier_c/runs/partc/ + unique suffix (two calls → distinct ids).
  5. --force-new on an existing dir clears stale files (rmtree + recreate).
  6. status.json written on both success AND failure; records failed_stage when arm raises.
  7. _persist_one_arm writes off or on arm files independently (allows incremental persistence).
  8. Off arm failure: meta.json + prompt file written despite exception; status.json shows failed.
  9. On arm failure: same guarantee; status.json records failed_stage="on".
"""
from __future__ import annotations

import json
import os
import types
from pathlib import Path

import pytest

from tier_c.model import Dose, ArmOutput, Variant, Citation


# ---------------------------------------------------------------------------
# Helper: fake ArmOutput
# ---------------------------------------------------------------------------

def _arm_output(*, prism: bool, text: str = "spec text", raw: str = "",
                prism_calls: int = 0,
                argv: list | None = None,
                returncode: int = 0,
                stderr: str = "",
                cwd: str = "/fake") -> ArmOutput:
    return ArmOutput(
        variant=Variant("opus-4.8", prism),
        text=text,
        citations=[Citation(file="src/a.go", line=1, symbol=None)],
        tokens=5,
        tool_calls=prism_calls,
        wall_s=0.1,
        used_prism=prism_calls > 0,
        prism_calls=prism_calls,
        dose=Dose(count=prism_calls),
        low_dose=(0 < prism_calls <= 1),
        raw_stdout=raw,
        argv=argv or ["claude", "-p"],
        returncode=returncode,
        stderr=stderr,
        cwd=cwd,
    )


def _fake_issue():
    return types.SimpleNamespace(
        key="ruff-1", language="rust", repo="ruff", sha="abc123",
        url="http://example.com", text="issue body", scoped_slice="slice 1",
    )


def _fake_comps_class(off_out, on_out, *, off_prompt="OFF", on_prompt="ON",
                      prewarm=None, call_score=None):
    """Return a comps class that can be used as _LivePartCComps replacement."""
    prewarm_val = prewarm

    class _Comps:
        _last_off = off_out
        _last_on = on_out
        _last_off_prompt = off_prompt
        _last_on_prompt = on_prompt
        _last_prewarm = prewarm_val
        _call = 0

        def run_off_arm(self, cell):
            return off_out

        def run_on_arm(self, cell):
            return on_out

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            self.__class__._call += 1
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    if call_score is not None:
        _Comps.score = call_score
    return _Comps


# ---------------------------------------------------------------------------
# 1. ArmRunError — structured exception with argv/returncode/stderr/stdout
# ---------------------------------------------------------------------------

def test_arm_run_error_is_importable():
    """ArmRunError must be importable from tier_c.arm_runner."""
    from tier_c.arm_runner import ArmRunError
    assert issubclass(ArmRunError, Exception)


def test_arm_run_error_carries_argv():
    """ArmRunError must expose .argv attribute."""
    from tier_c.arm_runner import ArmRunError
    err = ArmRunError(argv=["claude", "-p"], returncode=1, stderr="oops", stdout="")
    assert err.argv == ["claude", "-p"]


def test_arm_run_error_carries_returncode():
    """ArmRunError must expose .returncode attribute."""
    from tier_c.arm_runner import ArmRunError
    err = ArmRunError(argv=["claude"], returncode=137, stderr="killed", stdout="")
    assert err.returncode == 137


def test_arm_run_error_carries_full_stderr():
    """ArmRunError.stderr must hold the full stderr, not truncated."""
    from tier_c.arm_runner import ArmRunError
    long_stderr = "E" * 5000
    err = ArmRunError(argv=[], returncode=1, stderr=long_stderr, stdout="")
    assert err.stderr == long_stderr, "stderr must not be truncated in ArmRunError"


def test_arm_run_error_carries_stdout():
    """ArmRunError must expose .stdout attribute (may be empty on failure)."""
    from tier_c.arm_runner import ArmRunError
    err = ArmRunError(argv=["c"], returncode=1, stderr="", stdout="partial output")
    assert err.stdout == "partial output"


def test_arm_run_error_has_readable_str():
    """str(ArmRunError) must mention returncode and stderr."""
    from tier_c.arm_runner import ArmRunError
    err = ArmRunError(argv=["claude"], returncode=2, stderr="bad config", stdout="")
    msg = str(err)
    assert "2" in msg or "bad config" in msg, f"str(ArmRunError) uninformative: {msg!r}"


# ---------------------------------------------------------------------------
# 2. Runners raise ArmRunError (not RuntimeError) on non-zero exit
# ---------------------------------------------------------------------------

def test_claude_runner_raises_arm_run_error_on_nonzero(monkeypatch):
    """ClaudeRunner.run must raise ArmRunError when returncode != 0."""
    import tier_c.arm_runner as arm_mod
    from tier_c.arm_runner import ClaudeRunner, ArmRunError

    class _FakeProc:
        returncode = 1
        stdout = ""
        stderr = "claude crashed"

    monkeypatch.setattr(arm_mod.subprocess, "run", lambda *a, **kw: _FakeProc())

    runner = ClaudeRunner()
    with pytest.raises(ArmRunError) as exc_info:
        runner.run(Variant("opus-4.8", False), "spec", "prompt", "/tmp")

    err = exc_info.value
    assert err.returncode == 1
    assert "claude crashed" in err.stderr


def test_codex_runner_raises_arm_run_error_on_nonzero(monkeypatch):
    """CodexRunner.run must raise ArmRunError when returncode != 0."""
    import tier_c.arm_runner as arm_mod
    from tier_c.arm_runner import CodexRunner, ArmRunError

    class _FakeProc:
        returncode = 2
        stdout = ""
        stderr = "codex failed"

    monkeypatch.setattr(arm_mod.subprocess, "run", lambda *a, **kw: _FakeProc())

    runner = CodexRunner()
    with pytest.raises(ArmRunError) as exc_info:
        runner.run(Variant("gpt-5.5", False), "spec", "prompt", "/tmp")

    err = exc_info.value
    assert err.returncode == 2
    assert "codex failed" in err.stderr


def test_arm_run_error_carries_argv_from_runner(monkeypatch):
    """ArmRunError.argv must be the actual command the runner tried to run."""
    import tier_c.arm_runner as arm_mod
    from tier_c.arm_runner import ClaudeRunner, ArmRunError

    captured_argv = []

    class _FakeProc:
        returncode = 1
        stdout = ""
        stderr = "fail"

    def fake_run(cmd, *args, **kwargs):
        captured_argv.extend(cmd)
        return _FakeProc()

    monkeypatch.setattr(arm_mod.subprocess, "run", fake_run)

    runner = ClaudeRunner()
    with pytest.raises(ArmRunError) as exc_info:
        runner.run(Variant("opus-4.8", False), "spec", "prompt", "/tmp")

    err = exc_info.value
    assert isinstance(err.argv, list) and len(err.argv) > 0
    assert err.argv == captured_argv


# ---------------------------------------------------------------------------
# 3. ArmOutput carries argv/returncode/stderr/cwd on success
# ---------------------------------------------------------------------------

def test_arm_output_has_argv_field():
    """ArmOutput must have an argv field (list[str]) for audit persistence."""
    from tier_c.model import ArmOutput
    import dataclasses
    fields = {f.name for f in dataclasses.fields(ArmOutput)}
    assert "argv" in fields, "ArmOutput must have 'argv' field"


def test_arm_output_has_returncode_field():
    """ArmOutput must have a returncode field."""
    from tier_c.model import ArmOutput
    import dataclasses
    fields = {f.name for f in dataclasses.fields(ArmOutput)}
    assert "returncode" in fields, "ArmOutput must have 'returncode' field"


def test_arm_output_has_stderr_field():
    """ArmOutput must have a stderr field."""
    from tier_c.model import ArmOutput
    import dataclasses
    fields = {f.name for f in dataclasses.fields(ArmOutput)}
    assert "stderr" in fields, "ArmOutput must have 'stderr' field"


def test_arm_output_has_cwd_field():
    """ArmOutput must have a cwd field."""
    from tier_c.model import ArmOutput
    import dataclasses
    fields = {f.name for f in dataclasses.fields(ArmOutput)}
    assert "cwd" in fields, "ArmOutput must have 'cwd' field"


def test_arm_output_argv_defaults_to_empty_list():
    """ArmOutput.argv must default to [] (backward-compatible construction)."""
    out = ArmOutput(
        variant=Variant("opus-4.8", False),
        text="ok",
        citations=[],
        tokens=5,
        tool_calls=0,
        wall_s=0.0,
        used_prism=False,
        prism_calls=0,
        dose=Dose(),
        low_dose=False,
    )
    assert out.argv == [], f"argv must default to []; got {out.argv!r}"


def test_arm_output_returncode_defaults_to_zero():
    """ArmOutput.returncode must default to 0."""
    out = ArmOutput(
        variant=Variant("opus-4.8", False),
        text="ok",
        citations=[],
        tokens=5,
        tool_calls=0,
        wall_s=0.0,
        used_prism=False,
        prism_calls=0,
        dose=Dose(),
        low_dose=False,
    )
    assert out.returncode == 0


def test_arm_output_stderr_defaults_to_empty():
    """ArmOutput.stderr must default to ''."""
    out = ArmOutput(
        variant=Variant("opus-4.8", False),
        text="ok",
        citations=[],
        tokens=5,
        tool_calls=0,
        wall_s=0.0,
        used_prism=False,
        prism_calls=0,
        dose=Dose(),
        low_dose=False,
    )
    assert out.stderr == ""


def test_arm_output_cwd_defaults_to_empty():
    """ArmOutput.cwd must default to ''."""
    out = ArmOutput(
        variant=Variant("opus-4.8", False),
        text="ok",
        citations=[],
        tokens=5,
        tool_calls=0,
        wall_s=0.0,
        used_prism=False,
        prism_calls=0,
        dose=Dose(),
        low_dose=False,
    )
    assert out.cwd == ""


# ---------------------------------------------------------------------------
# 4. _persist_one_arm — per-arm writer for independent/incremental persistence
# ---------------------------------------------------------------------------

def test_persist_one_arm_is_importable():
    """_persist_one_arm must be importable from tier_c.cli."""
    from tier_c.cli import _persist_one_arm
    assert callable(_persist_one_arm)


def test_persist_one_arm_writes_off_meta_json(tmp_path):
    """_persist_one_arm writes <base>.off.meta.json with required fields."""
    from tier_c.cli import _persist_one_arm

    out = _arm_output(prism=False, text="off text", raw="raw\n",
                      argv=["claude", "-p"], returncode=0, stderr="", cwd="/repo")
    base = str(tmp_path / "ruff-spec-opus-4.8")
    _persist_one_arm(out, "OFF PROMPT", None, base=base, arm="off")

    meta_path = Path(f"{base}.off.meta.json")
    assert meta_path.exists(), f"off.meta.json must be written: {meta_path}"

    data = json.loads(meta_path.read_text())
    assert "argv" in data, "meta.json must have 'argv'"
    assert "returncode" in data, "meta.json must have 'returncode'"
    assert "stderr" in data, "meta.json must have 'stderr'"
    assert "wall_s" in data, "meta.json must have 'wall_s'"
    assert data["returncode"] == 0


def test_persist_one_arm_writes_on_meta_json(tmp_path):
    """_persist_one_arm writes <base>.on.meta.json for the on arm."""
    from tier_c.cli import _persist_one_arm

    out = _arm_output(prism=True, text="on text", raw='{"prism":1}\n',
                      argv=["claude", "--mcp-config", "/tmp/cfg.json"],
                      returncode=0, stderr="", cwd="/repo", prism_calls=2)
    prewarm = {"argv": ["prism", "nav"], "returncode": 0, "stdout": "ok",
               "stderr": "", "wall_s": 1.5, "exception": None}
    base = str(tmp_path / "ruff-spec-opus-4.8")
    _persist_one_arm(out, "ON PROMPT", prewarm, base=base, arm="on")

    meta_path = Path(f"{base}.on.meta.json")
    assert meta_path.exists(), f"on.meta.json must be written: {meta_path}"


def test_persist_one_arm_writes_prompt_file(tmp_path):
    """_persist_one_arm writes <base>.<arm>.prompt.txt."""
    from tier_c.cli import _persist_one_arm

    out = _arm_output(prism=False, text="off text", raw="")
    base = str(tmp_path / "ruff-spec-opus-4.8")
    _persist_one_arm(out, "MY EXACT PROMPT", None, base=base, arm="off")

    prompt_path = Path(f"{base}.off.prompt.txt")
    assert prompt_path.exists(), "off.prompt.txt must be written"
    assert prompt_path.read_text() == "MY EXACT PROMPT"


def test_persist_one_arm_writes_out_md_and_raw_jsonl_on_success(tmp_path):
    """_persist_one_arm writes .out.md and .raw.jsonl on success."""
    from tier_c.cli import _persist_one_arm

    out = _arm_output(prism=False, text="arm output text", raw='{"line":1}\n',
                      returncode=0)
    base = str(tmp_path / "ruff-spec-opus-4.8")
    _persist_one_arm(out, "PROMPT", None, base=base, arm="off")

    assert Path(f"{base}.off.out.md").exists(), ".off.out.md must be written on success"
    assert Path(f"{base}.off.raw.jsonl").exists(), ".off.raw.jsonl must be written on success"
    assert Path(f"{base}.off.out.md").read_text() == "arm output text"


def test_persist_one_arm_writes_prewarm_json_for_on_arm(tmp_path):
    """_persist_one_arm writes <base>.on.prewarm.json when prewarm is provided."""
    from tier_c.cli import _persist_one_arm

    out = _arm_output(prism=True, text="on", raw="", prism_calls=2)
    prewarm = {"argv": ["prism"], "returncode": 0, "stdout": "ok",
               "stderr": "", "wall_s": 2.3, "exception": None}
    base = str(tmp_path / "ruff-spec-opus-4.8")
    _persist_one_arm(out, "PROMPT", prewarm, base=base, arm="on")

    pw_path = Path(f"{base}.on.prewarm.json")
    assert pw_path.exists(), "on.prewarm.json must be written when prewarm provided"
    data = json.loads(pw_path.read_text())
    assert data["wall_s"] == pytest.approx(2.3)


def test_persist_one_arm_meta_contains_mcp_args(tmp_path):
    """meta.json must contain mcp_args and prism flag."""
    from tier_c.cli import _persist_one_arm

    out = _arm_output(prism=False, text="off", raw="")
    base = str(tmp_path / "ruff-spec-opus-4.8")
    _persist_one_arm(out, "P", None, base=base, arm="off",
                     mcp_args=["--repo", "/fake"], prism=False)

    data = json.loads(Path(f"{base}.off.meta.json").read_text())
    assert "mcp_args" in data, "meta.json must have 'mcp_args'"
    assert "prism" in data, "meta.json must have 'prism'"
    assert data["mcp_args"] == ["--repo", "/fake"]
    assert data["prism"] is False


# ---------------------------------------------------------------------------
# 5. Per-arm meta.json written on FAILURE (try/finally guarantee)
# ---------------------------------------------------------------------------

def test_off_arm_failure_writes_meta_json(tmp_path, monkeypatch):
    """When off arm raises ArmRunError, <base>.off.meta.json IS written with failure info."""
    import tier_c.cli as cli_mod
    from tier_c.arm_runner import ArmRunError

    err = ArmRunError(argv=["claude", "-p"], returncode=1,
                      stderr="HARD FAILURE from claude", stdout="")

    class _FailOffComps:
        _last_off = None
        _last_on = None
        _last_off_prompt = "OFF PROMPT"
        _last_on_prompt = "ON PROMPT"
        _last_prewarm = None

        def run_off_arm(self, cell):
            raise err

        def run_on_arm(self, cell):
            return _arm_output(prism=True, text="on", raw="", prism_calls=2)

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FailOffComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    runs_root = str(tmp_path / "runs")
    run_id = "fail-off-test"

    with pytest.raises((ArmRunError, Exception)):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id=run_id,
            runs_root=runs_root,
        )

    # meta.json MUST exist for the off arm despite the failure
    run_dir = Path(runs_root) / run_id
    meta_files = list(run_dir.glob("*.off.meta.json"))
    assert meta_files, (
        f"<base>.off.meta.json must be written even when off arm fails; "
        f"run_dir contents: {list(run_dir.iterdir())}"
    )
    data = json.loads(meta_files[0].read_text())
    assert data.get("returncode") == 1
    assert "HARD FAILURE" in data.get("stderr", "")
    assert data.get("exception") is not None


def test_off_arm_failure_writes_prompt_file(tmp_path, monkeypatch):
    """When off arm fails, <base>.off.prompt.txt IS written (it was set before the call)."""
    import tier_c.cli as cli_mod
    from tier_c.arm_runner import ArmRunError

    err = ArmRunError(argv=["claude"], returncode=1, stderr="fail", stdout="")

    class _FailOffComps:
        _last_off = None
        _last_on = None
        _last_off_prompt = "THE OFF PROMPT"
        _last_on_prompt = None
        _last_prewarm = None

        def run_off_arm(self, cell):
            raise err

        def run_on_arm(self, cell):
            return _arm_output(prism=True, text="on", raw="", prism_calls=2)

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FailOffComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    runs_root = str(tmp_path / "runs")
    run_id = "fail-off-prompt-test"

    with pytest.raises((ArmRunError, Exception)):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id=run_id,
            runs_root=runs_root,
        )

    run_dir = Path(runs_root) / run_id
    prompt_files = list(run_dir.glob("*.off.prompt.txt"))
    assert prompt_files, (
        f"off.prompt.txt must be written even when off arm fails; "
        f"run_dir: {list(run_dir.iterdir())}"
    )


def test_on_arm_failure_writes_meta_json(tmp_path, monkeypatch):
    """When on arm raises ArmRunError, <base>.on.meta.json IS written with failure info."""
    import tier_c.cli as cli_mod
    from tier_c.arm_runner import ArmRunError

    on_err = ArmRunError(argv=["claude", "--mcp-config", "/tmp/c.json"],
                         returncode=137, stderr="OOM killed", stdout="")

    class _FailOnComps:
        _last_off = _arm_output(prism=False, text="off ok", raw="off-raw")
        _last_on = None
        _last_off_prompt = "OFF PROMPT"
        _last_on_prompt = "ON PROMPT"
        _last_prewarm = None

        def run_off_arm(self, cell):
            return self._last_off

        def run_on_arm(self, cell):
            raise on_err

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FailOnComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    runs_root = str(tmp_path / "runs")
    run_id = "fail-on-test"

    with pytest.raises((ArmRunError, Exception)):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id=run_id,
            runs_root=runs_root,
        )

    run_dir = Path(runs_root) / run_id
    on_meta_files = list(run_dir.glob("*.on.meta.json"))
    assert on_meta_files, (
        f"<base>.on.meta.json must be written when on arm fails; "
        f"run_dir: {list(run_dir.iterdir())}"
    )
    data = json.loads(on_meta_files[0].read_text())
    assert data.get("returncode") == 137
    assert "OOM killed" in data.get("stderr", "")


# ---------------------------------------------------------------------------
# 6. status.json written on success AND failure
# ---------------------------------------------------------------------------

def test_status_json_written_on_success(tmp_path, monkeypatch):
    """status.json must be written with status='success' when the cell completes."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off", raw="")
    on_out = _arm_output(prism=True, text="on", raw="", prism_calls=2)

    class _SuccessComps:
        _last_off = off_out
        _last_on = on_out
        _last_off_prompt = "OFF"
        _last_on_prompt = "ON"
        _last_prewarm = None

        def run_off_arm(self, cell):
            return off_out

        def run_on_arm(self, cell):
            return on_out

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.6, recall=0.6, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _SuccessComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")

    runs_root = str(tmp_path / "runs")
    run_id = "status-success-test"

    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id=run_id,
        runs_root=runs_root,
    )

    status_path = Path(runs_root) / run_id / "status.json"
    assert status_path.exists(), f"status.json must be written on success; run_dir: {list((Path(runs_root) / run_id).iterdir())}"
    data = json.loads(status_path.read_text())
    assert data["status"] == "success"
    assert data.get("failed_stage") is None
    assert data.get("error") is None


def test_status_json_written_on_off_arm_failure(tmp_path, monkeypatch):
    """status.json must record status='failed' + failed_stage='off' when off arm raises."""
    import tier_c.cli as cli_mod
    from tier_c.arm_runner import ArmRunError

    err = ArmRunError(argv=["claude"], returncode=1, stderr="off arm died", stdout="")

    class _FailOffComps:
        _last_off = None
        _last_on = None
        _last_off_prompt = "OFF"
        _last_on_prompt = None
        _last_prewarm = None

        def run_off_arm(self, cell):
            raise err

        def run_on_arm(self, cell):
            return _arm_output(prism=True, text="on", raw="", prism_calls=2)

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FailOffComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    runs_root = str(tmp_path / "runs")
    run_id = "status-fail-off-test"

    with pytest.raises((ArmRunError, Exception)):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id=run_id,
            runs_root=runs_root,
        )

    status_path = Path(runs_root) / run_id / "status.json"
    assert status_path.exists(), "status.json must be written even when off arm fails"
    data = json.loads(status_path.read_text())
    assert data["status"] == "failed"
    assert data.get("failed_stage") == "off"
    assert data.get("error") is not None


def test_status_json_written_on_on_arm_failure(tmp_path, monkeypatch):
    """status.json must record status='failed' + failed_stage='on' when on arm raises."""
    import tier_c.cli as cli_mod
    from tier_c.arm_runner import ArmRunError

    on_err = ArmRunError(argv=["claude", "--mcp-config", "x"],
                         returncode=2, stderr="on arm OOM", stdout="")

    class _FailOnComps:
        _last_off = _arm_output(prism=False, text="off", raw="")
        _last_on = None
        _last_off_prompt = "OFF"
        _last_on_prompt = "ON"
        _last_prewarm = None

        def run_off_arm(self, cell):
            return self._last_off

        def run_on_arm(self, cell):
            raise on_err

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _FailOnComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)

    runs_root = str(tmp_path / "runs")
    run_id = "status-fail-on-test"

    with pytest.raises((ArmRunError, Exception)):
        cli_mod._run_partc_live(
            ("ruff", "spec", "opus-4.8"),
            bench_root="/fake/bench",
            base_root="base_root",
            issues_path="issues.toml",
            run_id=run_id,
            runs_root=runs_root,
        )

    status_path = Path(runs_root) / run_id / "status.json"
    assert status_path.exists(), "status.json must be written even when on arm fails"
    data = json.loads(status_path.read_text())
    assert data["status"] == "failed"
    assert data.get("failed_stage") == "on"


# ---------------------------------------------------------------------------
# 7. run-id default → runs/partc/ + unique suffix
# ---------------------------------------------------------------------------

def test_default_partc_run_id_is_under_runs_partc():
    """_default_partc_run_id must be under runs/partc/ (the default runs_root subdir)."""
    import tier_c.cli as cli_mod
    # The run-id itself is just the id string; the runs_root is what determines the directory.
    # The TEST is that the DEFAULT runs_root ends in runs/partc/ (not runs/).
    runs_root = cli_mod._default_partc_run_store_root()
    assert runs_root.endswith("runs/partc") or runs_root.endswith("runs/partc/"), (
        f"default partc runs root must end in runs/partc; got {runs_root!r}"
    )


def test_default_partc_run_id_has_unique_suffix():
    """Two calls to _default_partc_run_id() produce distinct ids."""
    import tier_c.cli as cli_mod
    import time
    id1 = cli_mod._default_partc_run_id()
    # Ensure same second is possible; the suffix must break the tie
    id2 = cli_mod._default_partc_run_id()
    # Even within the same second, the ids must differ
    assert id1 != id2, (
        f"Two calls to _default_partc_run_id must produce distinct ids; both got {id1!r}"
    )


def test_default_partc_run_id_format():
    """_default_partc_run_id must match partc-YYYYmmdd-HHMMSS-XXXX (4-hex suffix)."""
    import tier_c.cli as cli_mod
    import re
    id_ = cli_mod._default_partc_run_id()
    # Allow any suffix format (hex, pid-derived, etc) as long as something is after the timestamp
    assert re.match(r"partc-\d{8}-\d{6}-[0-9a-f]+", id_), (
        f"run-id format must be partc-YYYYmmdd-HHMMSS-<suffix>; got {id_!r}"
    )


# ---------------------------------------------------------------------------
# 8. --force-new clears stale files
# ---------------------------------------------------------------------------

def test_force_new_clears_stale_files(tmp_path, monkeypatch):
    """--force-new on an existing run dir removes stale files before writing new ones."""
    import tier_c.cli as cli_mod

    off_out = _arm_output(prism=False, text="off", raw="")
    on_out = _arm_output(prism=True, text="on", raw="", prism_calls=2)

    class _SuccessComps:
        _last_off = off_out
        _last_on = on_out
        _last_off_prompt = "OFF"
        _last_on_prompt = "ON"
        _last_prewarm = None

        def run_off_arm(self, cell):
            return off_out

        def run_on_arm(self, cell):
            return on_out

        def score(self, citations, **kwargs):
            from tier_c.investigator import InvestigatorReport
            return InvestigatorReport(precision=0.5, recall=0.5, hallucinations=0, verdicts=[])

    monkeypatch.setattr(cli_mod, "_LivePartCComps", lambda **kw: _SuccessComps())

    class _FakeCheckout:
        def __init__(self, *a, **kw): pass
        def __enter__(self): return types.SimpleNamespace(root="/fake")
        def __exit__(self, *a): pass

    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout, raising=False)
    monkeypatch.setattr(cli_mod, "load_issues", lambda path: [_fake_issue()])
    monkeypatch.setattr(cli_mod, "_write_partc_manifest", lambda *a, **kw: None, raising=False)
    monkeypatch.setattr(cli_mod, "render_partc", lambda cells: "REPORT")

    runs_root = str(tmp_path / "runs")
    run_id = "stale-test"
    run_dir = Path(runs_root) / run_id
    run_dir.mkdir(parents=True)

    # Plant a stale file
    stale_file = run_dir / "stale-artifact.txt"
    stale_file.write_text("OLD STALE CONTENT")
    assert stale_file.exists()

    # Run with force_new=True
    cli_mod._run_partc_live(
        ("ruff", "spec", "opus-4.8"),
        bench_root="/fake/bench",
        base_root="base_root",
        issues_path="issues.toml",
        run_id=run_id,
        runs_root=runs_root,
        force_new=True,
    )

    # Stale file must be gone
    assert not stale_file.exists(), (
        f"--force-new must clear stale files from the run dir; "
        f"stale-artifact.txt still exists"
    )
