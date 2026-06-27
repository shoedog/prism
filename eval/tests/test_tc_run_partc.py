# eval/tests/test_tc_run_partc.py
"""Task 11: single-cell Part-C runner + report + CLI (fakes only — NO live spend).

Tests:
  (a) main target: run_partc_cell scores on vs base with fakes.
  (b) 0-prism arm → administered == False.
  (c) leaked text → leaked == True.
  (d) render_partc includes the cell row + "pilot signal" label.
"""
from __future__ import annotations
import pytest
from tier_c.model import Dose, ArmOutput, Variant, Citation
from tier_c.partc import PartCCell, run_partc_cell, render_partc


# ---------------------------------------------------------------------------
# Helpers — fake composites
# ---------------------------------------------------------------------------

def _cell(repo: str, stage: str, model: str):
    """Tiny descriptor triple for run_partc_cell."""
    return (repo, stage, model)


def _fake_arm_output(*, used_prism: bool, prism_calls: int, text: str,
                     citations: list | None = None) -> ArmOutput:
    """Build a minimal ArmOutput for fake comps."""
    dose = Dose(count=prism_calls)
    if citations is None:
        citations = [Citation(file="src/a.go", line=1, symbol=None)]
    return ArmOutput(
        variant=Variant("opus-4.8", True),
        text=text,
        citations=citations,
        tokens=10,
        tool_calls=prism_calls,
        wall_s=0.1,
        used_prism=used_prism,
        prism_calls=prism_calls,
        dose=dose,
        low_dose=(prism_calls > 0 and prism_calls <= 1),
    )


class _FakeComps:
    """Injectable component bundle for run_partc_cell (no live calls)."""

    def __init__(self, *, on_precision: float, base_precision: float,
                 arm_out: ArmOutput, base_text: str = "base spec src/a.go:1"):
        self._on_precision = on_precision
        self._base_precision = base_precision
        self._arm_out = arm_out
        self._base_text = base_text

    def load_base(self, cell):
        return self._base_text

    def extract_citations(self, text: str) -> list:
        from tier_c.citations import parse_citations
        return parse_citations(text)

    def score(self, citations, **kwargs) -> float:
        # Called for on-arm: return on_precision; called for base: return base_precision.
        # We distinguish by injection order (base first, then on).
        if not hasattr(self, "_score_call_count"):
            self._score_call_count = 0
        self._score_call_count += 1
        if self._score_call_count == 1:
            return self._base_precision
        return self._on_precision

    def run_on_arm(self, cell) -> ArmOutput:
        return self._arm_out


def _fake_comps(*, on_precision: float, base_precision: float,
                prism_calls: int = 2, text: str = "clean spec, no tool names",
                citations: list | None = None) -> _FakeComps:
    """Build a FakeComps bundle; mirrors the spec's _fake_comps design."""
    arm_out = _fake_arm_output(
        used_prism=(prism_calls > 0),
        prism_calls=prism_calls,
        text=text,
        citations=citations,
    )
    return _FakeComps(on_precision=on_precision, base_precision=base_precision, arm_out=arm_out)


# ---------------------------------------------------------------------------
# (a) Main target test
# ---------------------------------------------------------------------------

def test_run_partc_cell_scores_on_vs_base_with_fakes():
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(on_precision=0.8, base_precision=0.4),
    )
    assert cell.bundle_delta == pytest.approx(0.4)
    assert cell.dose.count >= 1
    assert not cell.leaked


# ---------------------------------------------------------------------------
# (b) 0-prism arm → administered == False
# ---------------------------------------------------------------------------

def test_run_partc_cell_zero_prism_sets_not_administered():
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(on_precision=0.8, base_precision=0.4, prism_calls=0),
    )
    assert not cell.administered, "0 real prism calls must mark administered=False"
    # bundle_delta is still computed (so caller can see the scores), but administered=False
    # signals the Verify Gate to discard/re-run this cell.


# ---------------------------------------------------------------------------
# (c) Leaked text → leaked == True
# ---------------------------------------------------------------------------

def test_run_partc_cell_leaked_when_on_arm_text_contains_nav_callers():
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=0.7,
            base_precision=0.3,
            prism_calls=2,
            text="I used nav_callers to find the issue in src/a.go:1",
        ),
    )
    assert cell.leaked, "on-arm text naming nav_callers must set leaked=True"


# ---------------------------------------------------------------------------
# (d) render_partc includes row + pilot signal label
# ---------------------------------------------------------------------------

def test_render_partc_includes_row_and_pilot_signal_label():
    cell = PartCCell(
        repo="ruff",
        stage="spec",
        model="opus-4.8",
        precision_on=0.8,
        precision_base=0.4,
        bundle_delta=0.4,
        dose=Dose(count=2),
        low_dose=False,
        administered=True,
        leaked=False,
        recall_on=None,
        recall_base=None,
    )
    report = render_partc([cell])
    # Must contain cell identifier
    assert "ruff/spec/opus-4.8" in report or ("ruff" in report and "spec" in report)
    # Must contain precision values or delta
    assert "0.8" in report or "0.40" in report
    # Must contain "pilot signal" label
    assert "pilot signal" in report.lower()
