"""Integration tests for the Part-C scorecard-v2 wiring (partc.py): run_partc_cell
threading D1-D4 through the comps protocol, and render_partc showing every dimension
as a SEPARATE line (never collapsed — owner rubric-a).

Mirrors the existing hasattr-gated optionality pattern already used for head_to_head:
a comps object that doesn't implement score_validity/score_relational/
head_to_head_annotated must still produce a valid (if D1-D3-empty) PartCCell — this is
the regression safety net for every pre-existing minimal test fake in the suite.
"""
from __future__ import annotations

from tier_c.investigator import CitationVerdict, InvestigatorReport
from tier_c.model import ArmOutput, Citation, Dose, Variant
from tier_c.partc import run_partc_cell, render_partc


def _arm(prism: bool, text: str, citations=None, *, tool_calls=0, wall_s=0.0,
        cost_usd=0.0, commands=None) -> ArmOutput:
    return ArmOutput(
        variant=Variant("opus-4.8", prism), text=text, citations=citations or [],
        tokens=10, tool_calls=tool_calls, wall_s=wall_s, used_prism=prism,
        prism_calls=1 if prism else 0, dose=Dose(count=1 if prism else 0),
        low_dose=prism, in_tokens=5, cost_usd=cost_usd, raw_stdout="",
        commands=commands or [],
    )


def _cv(file, line, *, ok=True) -> CitationVerdict:
    return CitationVerdict(cite=Citation(file, line, None), file_ok=ok, line_ok=ok,
                           symbol_ok=True, relevant=True)


# ---------------------------------------------------------------------------
# Full comps: D1/D2/D3 hooks implemented -> PartCCell gets populated fields
# ---------------------------------------------------------------------------

class _FullComps:
    """Comps implementing every scorecard-v2 hook, with deterministic canned results —
    exercises the WIRING (run_partc_cell -> PartCCell), not the judging logic itself
    (already covered by test_tc_validity.py / test_tc_relational.py / test_tc_annotate.py)."""

    def run_off_arm(self, cell):
        return _arm(False, "off text a.py:1", [Citation("a.py", 1, None)],
                   tool_calls=2, wall_s=1.0, cost_usd=0.01)

    def run_on_arm(self, cell):
        return _arm(True, "on text a.py:1 and ghost.py:99",
                   [Citation("a.py", 1, None), Citation("ghost.py", 99, None)],
                   tool_calls=5, wall_s=2.0, cost_usd=0.02)

    def score(self, citations, *, cell, arm):
        if arm == "base":
            return InvestigatorReport(precision=1.0, recall=0.5, hallucinations=0,
                                      verdicts=[_cv("a.py", 1)])
        return InvestigatorReport(
            precision=0.5, recall=0.5, hallucinations=1,
            verdicts=[_cv("a.py", 1), _cv("ghost.py", 99, ok=False)],
        )

    def head_to_head(self, off, on, cell):
        return {"winner": "off", "escalated": False, "votes": []}

    def score_validity(self, text, *, arm):
        if arm == "base":
            return {"support_rate": 1.0, "contradicted": 0, "total": 1, "verdicts": []}
        return {"support_rate": 0.5, "contradicted": 1, "total": 2, "verdicts": [
            {"file": "a.py", "line": 1, "symbol": None, "sentence": "on text a",
             "verdict": "CONTRADICTED", "escalated": False},
        ]}

    def score_relational(self, off_text, on_text, *, cell):
        return {
            "off": {"precision": 1.0, "unknown_rate": 0.0, "contradicted": 0, "total": 1, "verdicts": []},
            "on": {"precision": 0.5, "unknown_rate": 0.25, "contradicted": 1, "total": 4, "verdicts": []},
            "delta": -0.5,
        }

    def head_to_head_annotated(self, off_annotated, on_annotated, cell):
        return {"winner": "on", "escalated": True, "votes": []}


def test_run_partc_cell_populates_all_v2_dimensions():
    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _FullComps())

    # D0
    assert cell.recall_base == 0.5
    assert cell.recall_on == 0.5

    # D1
    assert cell.validity_off == {"support_rate": 1.0, "contradicted": 0, "total": 1, "verdicts": []}
    assert cell.validity_on["support_rate"] == 0.5
    assert cell.validity_on["contradicted"] == 1

    # D2
    assert cell.relational["delta"] == -0.5
    assert cell.relational["on"]["unknown_rate"] == 0.25

    # D3
    assert cell.head_to_head_annotated == {"winner": "on", "escalated": True, "votes": []}
    assert "ghost.py:99" in cell.annotated_on_text
    assert "[CITED LOCATION DOES NOT EXIST]" in cell.annotated_on_text, (
        "the on-arm's hallucinated ghost.py:99 citation must be annotated inline"
    )
    assert "[CODE CONTRADICTS THIS CLAIM]" in cell.annotated_on_text, (
        "the on-arm's D1-CONTRADICTED a.py:1 claim must be annotated inline"
    )
    assert cell.annotated_off_text == "off text a.py:1", "off-arm has no flagged citations -> unchanged"

    # D4
    assert cell.nav_eff_off["tool_calls"] == 2
    assert cell.nav_eff_on["tool_calls"] == 5
    assert cell.nav_eff_off["cost_per_valid_citation"] == 0.01  # 1 valid citation
    assert cell.nav_eff_on["cost_per_valid_citation"] == 0.02   # 1 valid citation (ghost.py is hallucinated)


# ---------------------------------------------------------------------------
# Minimal comps (no v2 hooks): regression safety net for every pre-existing fake
# ---------------------------------------------------------------------------

class _MinimalComps:
    """Implements ONLY the pre-v2 protocol (run_off_arm/run_on_arm/score) — mirrors
    every minimal test fake already in the suite. Must not crash, and D1-D3 fields
    must degrade to empty dicts (D4 is still always computed — mechanical, no hook)."""

    def run_off_arm(self, cell):
        return _arm(False, "off text", [])

    def run_on_arm(self, cell):
        return _arm(True, "on text", [])

    def score(self, citations, *, cell, arm):
        return InvestigatorReport(precision=0.0, recall=0.0, hallucinations=0, verdicts=[])


def test_run_partc_cell_degrades_gracefully_without_v2_hooks():
    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _MinimalComps())
    assert cell.validity_off == {} and cell.validity_on == {}
    assert cell.relational == {}
    assert cell.head_to_head_annotated == {}
    assert cell.head_to_head == {}
    # D4 has no comps hook -> always computed, even for a minimal comps
    assert cell.nav_eff_off["tool_calls"] == 0
    assert cell.nav_eff_on["tool_calls"] == 0
    # D3 annotation is pure/mechanical -> still runs; with no hallucinations/contradictions
    # to flag, the annotated text equals the raw text.
    assert cell.annotated_off_text == "off text"
    assert cell.annotated_on_text == "on text"


# ---------------------------------------------------------------------------
# render_partc — every dimension its OWN line
# ---------------------------------------------------------------------------

def test_render_partc_shows_every_v2_dimension_on_its_own_line():
    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _FullComps())
    report = render_partc([cell])

    assert "D0 recall" in report
    assert "D1 validity" in report
    assert "D2 relational" in report
    assert "D3 annotated head-to-head" in report
    assert "D4 nav-eff" in report
    # Divergence note: blind H2H picked "off", annotated H2H picked "on" -> must flag it.
    assert "DIVERGES from blind head-to-head" in report


def test_render_partc_omits_v2_lines_when_not_populated():
    """A cell with no D1-D3 data (only D4, which is always mechanical) must not print
    empty/placeholder D1-D3 lines — the render is additive, not padded."""
    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _MinimalComps())
    report = render_partc([cell])
    assert "D1 validity" not in report
    assert "D2 relational" not in report
    assert "D3 annotated head-to-head" not in report
    # D0 recall IS populated here (0.0/0.0, not None) since _MinimalComps.score always
    # returns a real InvestigatorReport -> the line renders even though the value is 0.
    assert "D0 recall" in report
    assert "D4 nav-eff" in report
