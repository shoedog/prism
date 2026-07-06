"""P5: `run-partd` cell composition + persistence (spec: clone _run_partc_live's
arm composition + persistence EXACTLY so rescore.py works day one). Fakes only —
NO live agent arms; the wiring (cell tuple shape, persistence shape) is what's
under test here, mirroring test_tc_run_partc.py's fakes-only style.
"""
from __future__ import annotations
import json
import types

import pytest

from tier_c.model import Dose, ArmOutput, Variant
from tier_c.impact import ImpactSite, ImpactParseResult
from tier_c.partd import PartDCell, run_partd_cell, render_partd, _partd_prompt
from tier_c.structural import StructuralReport
from tier_c.structural_corpus import StructuralTask


def _cell(task_id="prometheus-matchstring", model="opus-4.8"):
    return (task_id, model)


def _arm_out(*, prism: bool, prism_calls: int, text: str, in_tokens=10, cost_usd=0.0) -> ArmOutput:
    return ArmOutput(
        variant=Variant("opus-4.8", prism), text=text, citations=[], tokens=5,
        tool_calls=prism_calls, wall_s=0.1, used_prism=prism_calls > 0,
        prism_calls=prism_calls, dose=Dose(count=prism_calls),
        low_dose=(0 < prism_calls <= 1), in_tokens=in_tokens, cost_usd=cost_usd,
    )


def _report(**overrides) -> StructuralReport:
    base = dict(file_precision=1.0, file_recall=1.0, file_f1=1.0,
               symbol_precision=1.0, symbol_recall=1.0, symbol_f1=1.0,
               d_recall=1.0, d_recall_file=1.0, precision=1.0, phantom=0, unmatched_extra=0,
               gold_size=2, d_gold_size=1, d_gold_file_size=1, claimed_size=2)
    base.update(overrides)
    return StructuralReport(**base)


def _parsed(sites) -> ImpactParseResult:
    return ImpactParseResult(ok=True, sites=sites, migration_order=[])


class _FakeComps:
    def __init__(self, *, off_report, on_report, off_out=None, on_out=None,
                off_parsed=None, on_parsed=None):
        self._off_report, self._on_report = off_report, on_report
        self._off_out = off_out or _arm_out(prism=False, prism_calls=0, text="off clean")
        self._on_out = on_out or _arm_out(prism=True, prism_calls=2, text="on clean")
        self._off_parsed = off_parsed or _parsed([ImpactSite("a.go", "Foo")])
        self._on_parsed = on_parsed or _parsed([ImpactSite("a.go", "Foo"), ImpactSite("b.go", "Bar")])
        self._score_calls = 0

    def run_off_arm(self, cell):
        return self._off_out, self._off_parsed

    def run_on_arm(self, cell):
        return self._on_out, self._on_parsed

    def score(self, parsed, *, cell, arm):
        self._score_calls += 1
        return self._off_report if arm == "off" else self._on_report


# ---------------------------------------------------------------------------
# run_partd_cell
# ---------------------------------------------------------------------------

def test_run_partd_cell_computes_deltas():
    off_report = _report(d_recall=0.2, file_f1=0.3)
    on_report = _report(d_recall=0.8, file_f1=0.6)
    cell = run_partd_cell(_cell(), _FakeComps(off_report=off_report, on_report=on_report))
    assert cell.d_recall_delta == pytest.approx(0.6)
    assert cell.file_f1_delta == pytest.approx(0.3)
    assert cell.report_off["d_recall"] == 0.2
    assert cell.report_on["d_recall"] == 0.8


def test_run_partd_cell_administered_reflects_on_arm_used_prism():
    on_out = _arm_out(prism=True, prism_calls=3, text="on")
    cell = run_partd_cell(_cell(), _FakeComps(off_report=_report(), on_report=_report(), on_out=on_out))
    assert cell.administered


def test_run_partd_cell_not_administered_when_zero_prism_calls():
    on_out = _arm_out(prism=True, prism_calls=0, text="on, but zero real prism calls")
    cell = run_partd_cell(_cell(), _FakeComps(off_report=_report(), on_report=_report(), on_out=on_out))
    assert not cell.administered


def test_run_partd_cell_leaked_reflects_on_arm_text_only():
    on_out = _arm_out(prism=True, prism_calls=2, text="I used nav_callers to find sites")
    off_out = _arm_out(prism=False, prism_calls=0, text="clean off text")
    cell = run_partd_cell(_cell(), _FakeComps(off_report=_report(), on_report=_report(),
                                              off_out=off_out, on_out=on_out))
    assert cell.leaked


def test_run_partd_cell_not_leaked_when_on_arm_clean():
    on_out = _arm_out(prism=True, prism_calls=2, text="clean on text")
    off_out = _arm_out(prism=False, prism_calls=0, text="off text mentions nav_callers")
    cell = run_partd_cell(_cell(), _FakeComps(off_report=_report(), on_report=_report(),
                                              off_out=off_out, on_out=on_out))
    assert not cell.leaked


def test_run_partd_cell_threads_tokens_cost_wall():
    off_out = _arm_out(prism=False, prism_calls=0, text="off", in_tokens=30, cost_usd=0.001)
    on_out = _arm_out(prism=True, prism_calls=2, text="on", in_tokens=50, cost_usd=0.002)
    cell = run_partd_cell(_cell(), _FakeComps(off_report=_report(), on_report=_report(),
                                              off_out=off_out, on_out=on_out))
    assert cell.in_tokens_off == 30
    assert cell.in_tokens_on == 50
    assert cell.tokens_off == 35   # 30 in + 5 out
    assert cell.tokens_on == 55
    assert cell.cost_off == pytest.approx(0.001)
    assert cell.cost_on == pytest.approx(0.002)


def test_run_partd_cell_carries_migration_order():
    off_parsed = _parsed([ImpactSite("a.go", "Foo")])
    on_parsed = ImpactParseResult(ok=True, sites=[ImpactSite("a.go", "Foo")],
                                  migration_order=["a.go:Foo", "b.go:Bar"])
    cell = run_partd_cell(_cell(), _FakeComps(off_report=_report(), on_report=_report(),
                                              off_parsed=off_parsed, on_parsed=on_parsed))
    assert cell.on_migration_order == ["a.go:Foo", "b.go:Bar"]
    assert cell.off_migration_order == []


def test_run_partd_cell_task_id_and_model_threaded():
    cell = run_partd_cell(_cell("ruff-typechecker-match-annotation", "gpt-5.5"),
                          _FakeComps(off_report=_report(), on_report=_report()))
    assert cell.task_id == "ruff-typechecker-match-annotation"
    assert cell.model == "gpt-5.5"


# ---------------------------------------------------------------------------
# render_partd
# ---------------------------------------------------------------------------

def test_render_partd_includes_row_and_label():
    cell = run_partd_cell(_cell(), _FakeComps(
        off_report=_report(d_recall=0.2, file_f1=0.3),
        on_report=_report(d_recall=0.8, file_f1=0.6),
    ))
    report = render_partd([cell])
    assert "prometheus-matchstring" in report
    assert "opus-4.8" in report
    assert "d-recall" in report.lower() or "d_recall" in report.lower()


# ---------------------------------------------------------------------------
# _partd_prompt: identical contract, symbol/def_site threaded in
# ---------------------------------------------------------------------------

def _task():
    return StructuralTask(
        id="prometheus-matchstring", repo="prometheus", lang="go", sha="505095b",
        symbol="MatchString", receiver="(*FastRegexMatcher)",
        def_site=("model/labels/regexp.go", 328),
        dispatch="labels.Matcher.Matches -> matchString field",
        prompt_change="We are changing the matching semantics of FastRegexMatcher.MatchString.",
        grep_name_stats="git grep -lw MatchString = 21 files",
        notes="",
    )


def test_partd_prompt_contains_symbol_and_def_site():
    p = _partd_prompt(_task(), steer="")
    assert "MatchString" in p
    assert "model/labels/regexp.go:328" in p
    assert "We are changing the matching semantics" in p


def test_partd_prompt_off_is_prefix_of_on():
    off = _partd_prompt(_task(), steer="")
    on = _partd_prompt(_task(), steer="prism_on")
    assert off in on


# ---------------------------------------------------------------------------
# _run_partd_live persistence wiring (fakes only, no live spend)
# ---------------------------------------------------------------------------

def test_run_partd_live_writes_manifest_status_and_arm_files(tmp_path, monkeypatch):
    import tier_c.partd as partd_mod

    off_out = _arm_out(prism=False, prism_calls=0, text="## Off impact\nsrc/a.go:1")
    on_out = _arm_out(prism=True, prism_calls=2, text="## On impact\nsrc/a.go:1")
    off_parsed = _parsed([ImpactSite("src/a.go", "Foo")])
    on_parsed = _parsed([ImpactSite("src/a.go", "Foo"), ImpactSite("src/b.go", "Bar")])

    class _FakeLiveComps:
        def __init__(self, **kwargs):
            self._last_off = off_out
            self._last_on = on_out
            self._last_off_parsed = off_parsed
            self._last_on_parsed = on_parsed
            self._last_off_prompt = "OFF PROMPT"
            self._last_on_prompt = "ON PROMPT"
            self._last_prewarm = None

        def run_off_arm(self, cell):
            return off_out, off_parsed

        def run_on_arm(self, cell):
            return on_out, on_parsed

        def score(self, parsed, **kwargs):
            return _report()

    monkeypatch.setattr(partd_mod, "_LivePartDComps", _FakeLiveComps)

    # Avoid touching real git: patch Checkout with a trivial context manager.
    class _FakeCheckout:
        def __init__(self, repo, sha):
            self.root = tmp_path / "checkout-root"
            self.root.mkdir(exist_ok=True)

        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

    monkeypatch.setattr(partd_mod, "Checkout", _FakeCheckout)

    task = _task()
    monkeypatch.setattr(partd_mod, "load_structural_tasks", lambda path: [task])

    runs_root = str(tmp_path / "runs")
    partd_mod._run_partd_live(
        ("prometheus-matchstring", "opus-4.8"),
        bench_root=str(tmp_path / "bench"),
        structural_issues_path="unused.toml",
        gold_root=str(tmp_path / "gold"),
        run_id="test-run-1",
        runs_root=runs_root,
    )

    run_dir = tmp_path / "runs" / "test-run-1"
    assert (run_dir / "manifest.json").exists()
    manifest = json.loads((run_dir / "manifest.json").read_text())
    assert manifest["cell"]["task_id"] == "prometheus-matchstring"
    assert manifest["cell"]["model"] == "opus-4.8"

    status = json.loads((run_dir / "status.json").read_text())
    assert status["status"] == "success"

    base_name = "prometheus-matchstring-impact-opus-4.8"
    assert (run_dir / f"{base_name}.off.out.md").exists()
    assert (run_dir / f"{base_name}.on.out.md").exists()

    assert (run_dir / f"{base_name}.json").exists(), \
        f"expected a persisted cell JSON in {list(run_dir.iterdir())}"


def test_run_partd_live_collision_guard_without_force_new(tmp_path, monkeypatch):
    import tier_c.partd as partd_mod

    run_dir = tmp_path / "runs" / "dup-run"
    run_dir.mkdir(parents=True)

    with pytest.raises(FileExistsError):
        partd_mod._run_partd_live(
            ("prometheus-matchstring", "opus-4.8"),
            bench_root=str(tmp_path / "bench"),
            structural_issues_path="unused.toml",
            gold_root=str(tmp_path / "gold"),
            run_id="dup-run",
            runs_root=str(tmp_path / "runs"),
            force_new=False,
        )


def test_run_partd_live_arm_failure_writes_failed_status(tmp_path, monkeypatch):
    import tier_c.partd as partd_mod
    from tier_c.arm_runner import ArmRunError

    class _FailingComps:
        def __init__(self, **kwargs):
            self._last_off = None
            self._last_off_prompt = "OFF PROMPT"

        def run_off_arm(self, cell):
            raise ArmRunError(argv=["fake"], returncode=1, stderr="boom", stdout="")

        def run_on_arm(self, cell):
            raise AssertionError("must not run on-arm when off-arm failed")

        def score(self, parsed, **kwargs):
            raise AssertionError("must not score when off-arm failed")

    monkeypatch.setattr(partd_mod, "_LivePartDComps", _FailingComps)

    class _FakeCheckout:
        def __init__(self, repo, sha):
            self.root = tmp_path / "checkout-root"
            self.root.mkdir(exist_ok=True)

        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

    monkeypatch.setattr(partd_mod, "Checkout", _FakeCheckout)
    monkeypatch.setattr(partd_mod, "load_structural_tasks", lambda path: [_task()])

    runs_root = str(tmp_path / "runs")
    with pytest.raises(ArmRunError):
        partd_mod._run_partd_live(
            ("prometheus-matchstring", "opus-4.8"),
            bench_root=str(tmp_path / "bench"),
            structural_issues_path="unused.toml",
            gold_root=str(tmp_path / "gold"),
            run_id="fail-run",
            runs_root=runs_root,
        )

    status = json.loads((tmp_path / "runs" / "fail-run" / "status.json").read_text())
    assert status["status"] == "failed"
    assert status["failed_stage"] == "off"
