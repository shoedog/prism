"""Tests for tier_c.rescore — re-derive Part-C scores from SAVED arm artifacts
without re-running the live arms.

Cardinal invariant (feedback_rescore_not_rerun): rescore writes to a NEW run-id
and NEVER mutates the source run dir. If we have the logs/telemetry, we rescore;
we don't re-run the cells.
"""
from __future__ import annotations
import json
import os
import hashlib
from pathlib import Path

from tier_c.model import Variant
from tier_c.rescore import reconstruct_arm_output, rescore_cell, rescore_run_dir


# --------------------------------------------------------------------------
# fixtures: a minimal claude stream-json raw, mirroring what arm_runner saves
# --------------------------------------------------------------------------
def _claude_raw(text: str, *, prism_tools=(), in_tok=1200, out_tok=350, cost=0.03) -> str:
    lines = []
    for t in prism_tools:
        lines.append(json.dumps({"type": "assistant", "message": {"content": [
            {"type": "tool_use", "name": t, "input": {}}]}}))
    lines.append(json.dumps({"type": "assistant", "message": {"content": [
        {"type": "text", "text": text}]}}))
    lines.append(json.dumps({"type": "result", "is_error": False, "result": "done",
        "total_cost_usd": cost, "usage": {"input_tokens": in_tok, "output_tokens": out_tok}}))
    return "\n".join(lines)


class _FakeCo:
    """Stand-in for Checkout: every cite resolves, lines/windows are non-empty."""
    def resolve_rel(self, f): return f
    def read_line(self, f, l): return f"// line {l}"
    def read_window(self, f, l): return f"code window at {f}:{l}"


class _FakeIssue:
    repo = "prometheus"; sha = "505095b"; key = "prometheus-18896"
    language = "go"; url = "u"
    text = "FastRegexMatcher false-positive on a capturing group between literals"
    scoped_slice = "fix the fast matcher"


def _ask_yes(model, prompt):  # judge says relevant
    return "YES"


def _ask_no(model, prompt):
    return "NO"


# --------------------------------------------------------------------------
# reconstruct_arm_output
# --------------------------------------------------------------------------
def test_reconstruct_arm_output_tokens_cost_citations():
    raw = _claude_raw("Root cause in model/labels/regexp.go:42",
                      prism_tools=["mcp__prism__nav_callers"])
    out = reconstruct_arm_output(raw, variant=Variant("opus-4.8", True), model="opus-4.8",
                                 wall_s=12.5, argv=["claude", "-p"], cwd="/repo")
    assert out.in_tokens == 1200
    assert out.tokens == 350                 # output tokens
    assert abs(out.cost_usd - 0.03) < 1e-9
    assert out.prism_calls == 1
    assert out.used_prism is True
    assert out.wall_s == 12.5
    assert out.raw_stdout == raw             # verbatim stream preserved
    assert any("regexp.go" in c.file for c in out.citations)


def test_reconstruct_arm_output_off_arm_no_prism():
    raw = _claude_raw("See model/labels/regexp.go:42")
    out = reconstruct_arm_output(raw, variant=Variant("opus-4.8", False), model="opus-4.8")
    assert out.prism_calls == 0
    assert out.used_prism is False
    assert out.citations                     # citations still parsed from text


def test_reconstruct_arm_output_dose_distinct_tools():
    raw = _claude_raw("x model/labels/regexp.go:42",
                      prism_tools=["mcp__prism__nav_callers", "mcp__prism__nav_repo_map"])
    out = reconstruct_arm_output(raw, variant=Variant("opus-4.8", True), model="opus-4.8")
    assert out.dose.distinct_tools == {"nav_callers", "nav_repo_map"}


# --------------------------------------------------------------------------
# rescore_cell (reuses the real _LivePartCComps.score via a cached-arm wrapper)
# --------------------------------------------------------------------------
def test_rescore_cell_uses_injected_judge_yes():
    off = reconstruct_arm_output(_claude_raw("a model/labels/regexp.go:42"),
                                 variant=Variant("opus-4.8", False), model="opus-4.8")
    on = reconstruct_arm_output(_claude_raw("b model/labels/regexp.go:42",
                                            prism_tools=["mcp__prism__nav_callers"]),
                                variant=Variant("opus-4.8", True), model="opus-4.8")
    cell, off_judge, on_judge = rescore_cell(
        ("prometheus", "spec", "opus-4.8"), off_out=off, on_out=on,
        co=_FakeCo(), issue=_FakeIssue(), base_root="", ask=_ask_yes)
    assert cell.repo == "prometheus"
    assert cell.administered is True         # on-arm used prism
    assert cell.precision_on == 1.0          # judge says YES + fake co resolves all
    assert cell.precision_base == 1.0
    assert len(on_judge) == len(on.citations)


def test_rescore_cell_judge_no_drops_precision():
    on = reconstruct_arm_output(_claude_raw("b model/labels/regexp.go:42",
                                            prism_tools=["mcp__prism__nav_callers"]),
                                variant=Variant("opus-4.8", True), model="opus-4.8")
    off = reconstruct_arm_output(_claude_raw("a model/labels/regexp.go:42"),
                                 variant=Variant("opus-4.8", False), model="opus-4.8")
    cell, _, _ = rescore_cell(("prometheus", "spec", "opus-4.8"), off_out=off, on_out=on,
                              co=_FakeCo(), issue=_FakeIssue(), base_root="", ask=_ask_no)
    assert cell.precision_on == 0.0          # judge says NO → not relevant → not valid


# --------------------------------------------------------------------------
# rescore_run_dir — writes a NEW dir, NEVER mutates the source
# --------------------------------------------------------------------------
def _seed_src_run_dir(tmp_path: Path) -> Path:
    """Build a fake source run dir as arm_runner/_persist_one_arm would leave it."""
    src = tmp_path / "src-run"
    src.mkdir()
    repo, stage, model = "prometheus", "spec", "opus-4.8"
    # NOTE: filenames are built by direct string (model "opus-4.8" has a dot, so
    # Path.with_suffix would mangle it) — mirrors _persist_one_arm exactly.
    prefix = f"{repo}-{stage}-{model}"
    (src / "manifest.json").write_text(json.dumps({
        "cell": {"repo": repo, "stage": stage, "model": model},
        "issue": {"sha": "505095b", "repo": repo},
        "issues_path": "tier_c/issues/issues.toml",
        "bench_root": "/unused-because-co-injected",
        "base_root": "",
    }))
    for arm in ("off", "on"):
        tools = ["mcp__prism__nav_callers"] if arm == "on" else []
        (src / f"{prefix}.{arm}.raw.jsonl").write_text(
            _claude_raw(f"{arm} model/labels/regexp.go:42", prism_tools=tools))
        (src / f"{prefix}.{arm}.out.md").write_text(f"{arm} spec text")
        (src / f"{prefix}.{arm}.meta.json").write_text(json.dumps(
            {"wall_s": 9.9, "argv": ["claude"], "cwd": "/repo", "returncode": 0}))
    return src


def _hash_dir(d: Path) -> dict:
    out = {}
    for p in sorted(d.rglob("*")):
        if p.is_file():
            out[str(p.relative_to(d))] = hashlib.sha256(p.read_bytes()).hexdigest()
    return out


def test_rescore_run_dir_writes_new_dir(tmp_path):
    src = _seed_src_run_dir(tmp_path)
    runs_root = tmp_path / "out"
    cell, out_dir = rescore_run_dir(
        str(src), out_run_id="src-run-rescore", runs_root=str(runs_root),
        issue=_FakeIssue(), co=_FakeCo(), ask=_ask_yes)
    assert Path(out_dir) != src, "rescore MUST write to a new dir, not the source"
    assert (Path(out_dir) / "prometheus-spec-opus-4.8.json").exists(), "cell json must be written"
    assert cell.precision_on == 1.0


def test_rescore_run_dir_does_not_modify_source(tmp_path):
    """THE LESSON, enforced: rescoring must leave every source file byte-identical."""
    src = _seed_src_run_dir(tmp_path)
    before = _hash_dir(src)
    runs_root = tmp_path / "out"
    rescore_run_dir(str(src), out_run_id="src-run-rescore", runs_root=str(runs_root),
                    issue=_FakeIssue(), co=_FakeCo(), ask=_ask_yes)
    after = _hash_dir(src)
    assert before == after, (
        f"rescore mutated the source run dir!\nbefore={before}\nafter={after}")


def test_rescore_run_dir_persists_judge_jsonl(tmp_path):
    src = _seed_src_run_dir(tmp_path)
    runs_root = tmp_path / "out"
    _, out_dir = rescore_run_dir(
        str(src), out_run_id="src-run-rescore", runs_root=str(runs_root),
        issue=_FakeIssue(), co=_FakeCo(), ask=_ask_yes)
    on_judge = Path(out_dir) / "prometheus-spec-opus-4.8.on.judge.jsonl"
    assert on_judge.exists(), "per-cite judge artifacts must be persisted in the rescore dir"
    recs = [json.loads(l) for l in on_judge.read_text().splitlines() if l.strip()]
    assert recs and all("relevant" in r for r in recs)


# --------------------------------------------------------------------------
# CLI dispatch
# --------------------------------------------------------------------------
def test_cli_rescore_dispatches(monkeypatch, capsys):
    import tier_c.cli as cli_mod
    import tier_c.rescore as rescore_mod
    seen = {}

    def fake_rescore(src, **kw):
        seen["src"] = src
        seen["kw"] = kw
        return ("CELL", "/out/dir")

    render_seen = {}

    def fake_render(cells):
        render_seen["cells"] = cells
        return "rendered"

    monkeypatch.setattr(rescore_mod, "rescore_run_dir", fake_rescore)
    monkeypatch.setattr(cli_mod, "render_partc", fake_render)
    rc = cli_mod.main(["rescore", "--run-dir", "/some/run", "--out-run-id", "x"])
    assert rc == 0
    assert seen["src"].endswith("/some/run")
    assert seen["kw"]["out_run_id"] == "x"
    # render_partc takes a LIST of cells — the single cell_result must be wrapped
    assert render_seen["cells"] == ["CELL"], (
        f"render_partc must receive [cell_result]; got {render_seen['cells']!r}")
    out = capsys.readouterr().out
    assert "rescored" in out and "/out/dir" in out


# --------------------------------------------------------------------------
# --reuse-off-from: re-run the on-arm only, reconstruct the off-arm baseline
# --------------------------------------------------------------------------
def test_live_comps_reuse_off_from_reconstructs_without_rerun(tmp_path, monkeypatch):
    """When reuse_off_from is set, run_off_arm reconstructs the off-arm from a saved run
    dir and MUST NOT run a live arm — holds the baseline constant when only the steer/on-arm
    changed, and avoids re-spending on an unchanged arm (feedback_rescore_not_rerun)."""
    from tier_c.cli import _LivePartCComps
    import tier_c.arm_runner as arm_mod
    src = _seed_src_run_dir(tmp_path)

    def _boom(*a, **k):
        raise AssertionError("run_off_arm must NOT run a live arm when reuse_off_from is set")

    monkeypatch.setattr(arm_mod, "run_arm_isolated", _boom)
    comps = _LivePartCComps(co=_FakeCo(), issue=_FakeIssue(), model="opus-4.8",
                            base_root="", reuse_off_from=str(src))
    out = comps.run_off_arm(("prometheus", "spec", "opus-4.8"))
    assert out is not None
    assert any("regexp.go" in c.file for c in out.citations), (
        "reused off-arm must carry the citations reconstructed from the saved off.raw.jsonl")
    assert out.used_prism is False, "off-arm baseline must remain prism-free"


def test_live_comps_head_to_head_uses_spec_quality_judge():
    """_LivePartCComps.head_to_head compares the two specs via SpecQualityJudge over self._ask."""
    from tier_c.cli import _LivePartCComps
    from tier_c.model import ArmOutput, Variant, Dose

    def _arm(text):
        return ArmOutput(variant=Variant("opus-4.8", False), text=text, citations=[], tokens=0,
                         tool_calls=0, wall_s=0.0, used_prism=False, prism_calls=0, dose=Dose(count=0),
                         low_dose=False, in_tokens=0, cost_usd=0.0, raw_stdout="")

    comps = _LivePartCComps(co=_FakeCo(), issue=_FakeIssue(), model="opus-4.8", base_root="",
                            ask=lambda m, p: "A clearer root cause")
    r = comps.head_to_head(_arm("OFF spec"), _arm("ON spec"), ("prometheus", "spec", "opus-4.8"))
    assert r["winner"] in ("off", "on", "tie") and "votes" in r
