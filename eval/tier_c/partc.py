# eval/tier_c/partc.py
"""Part-C single-cell runner: steered-prism-on arm vs recovered baseline.

``run_partc_cell`` composes Tasks 7/9/10 into one cell and gates on real prism
calls (administered flag).  The live path is wired in ``cli.py``; fakes drive
unit tests with zero live spend.

``PartCCell`` is the frozen dataclass returned to the Verify Gate.
``render_partc`` produces a minimal pilot-signal table.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .model import Dose


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class PartCCell:
    """Result of one Part-C (repo, stage, model) cell.

    Fields
    ------
    repo, stage, model:
        Cell identifier triple.
    precision_on:
        Citation precision of the steered prism-on arm (after oracle scoring).
    precision_base:
        Citation precision of the de-specified prism-OFF arm (status-quo fresh run, same
        oracle).  This is NOT a recovered historical baseline — it is a fresh live run
        with prism disabled, providing the cost-benefit denominator.
    bundle_delta:
        precision_on - precision_base (the primary Part-C signal).
    dose:
        Prism invocation measurement from the on-arm (count, distinct_tools, errors).
    low_dose:
        True when prism was called but only once (≤1 call — weak signal).
    administered:
        False when the on-arm made ZERO real prism calls (the treatment was not
        delivered).  The Verify Gate should discard or re-run non-administered cells.
    leaked:
        True when the on-arm text contains prism/nav_* tool names (blinding break).
    recall_on, recall_base:
        Optional recall scores if the scoring oracle returns them cheaply; None otherwise.
    tokens_off:
        Total tokens (input + output) consumed by the prism-OFF arm.
    tokens_on:
        Total tokens (input + output) consumed by the prism-ON arm.
    in_tokens_off:
        Input tokens for the prism-OFF arm (split from total).
    out_tokens_off:
        Output tokens for the prism-OFF arm (split from total).
    in_tokens_on:
        Input tokens for the prism-ON arm (split from total).
    out_tokens_on:
        Output tokens for the prism-ON arm (split from total).
    cost_off:
        USD cost of the prism-OFF arm run.
    cost_on:
        USD cost of the prism-ON arm run.
    wall_off:
        Wall-clock seconds for the prism-OFF arm run.
    wall_on:
        Wall-clock seconds for the prism-ON arm run.
    off_breakdown:
        Per-arm citation failure breakdown dict for the OFF arm:
        {n, valid, halluc, irrelevant, fails: [str, ...]}.
    on_breakdown:
        Per-arm citation failure breakdown dict for the ON arm:
        {n, valid, halluc, irrelevant, fails: [str, ...]}.
    """
    repo: str
    stage: str
    model: str
    precision_on: float
    precision_base: float
    bundle_delta: float
    dose: Dose
    low_dose: bool
    administered: bool
    leaked: bool
    recall_on: float | None
    recall_base: float | None
    tokens_off: int = 0
    tokens_on: int = 0
    in_tokens_off: int = 0
    out_tokens_off: int = 0
    in_tokens_on: int = 0
    out_tokens_on: int = 0
    cost_off: float = 0.0
    cost_on: float = 0.0
    wall_off: float = 0.0
    wall_on: float = 0.0
    off_breakdown: dict = None  # type: ignore[assignment]
    on_breakdown: dict = None   # type: ignore[assignment]
    off_verdicts: list = None   # type: ignore[assignment]
    on_verdicts: list = None    # type: ignore[assignment]
    gate: dict = None           # type: ignore[assignment]
    head_to_head: dict = field(default_factory=dict)

    def __post_init__(self):
        # Supply empty-breakdown dicts for callers that don't provide them
        if self.off_breakdown is None:
            object.__setattr__(self, "off_breakdown",
                               {"n": 0, "valid": 0, "halluc": 0, "irrelevant": 0, "fails": []})
        if self.on_breakdown is None:
            object.__setattr__(self, "on_breakdown",
                               {"n": 0, "valid": 0, "halluc": 0, "irrelevant": 0, "fails": []})
        if self.off_verdicts is None:
            object.__setattr__(self, "off_verdicts", [])
        if self.on_verdicts is None:
            object.__setattr__(self, "on_verdicts", [])
        if self.gate is None:
            object.__setattr__(self, "gate",
                               {"decision": "keep", "reason": "", "prism_calls": 0,
                                "distinct_tools": []})


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def run_partc_cell(cell: tuple, comps: Any) -> PartCCell:
    """Build a single Part-C cell from a (repo, stage, model) triple + injected comps.

    Both arms are now fresh live runs using the SAME de-specified prompt so the
    comparison is fair.  The prism-OFF arm acts as the baseline.

    Flow
    ----
    1. ``comps.run_off_arm(cell)`` → ArmOutput (fresh prism-OFF status-quo run, no steer).
    2. ``comps.score(off_out.citations, ...)`` → precision_base.
    3. ``comps.run_on_arm(cell)`` → ArmOutput (steered prism-ON run, Task 9 isolated).
    4. Gate: if on_out.used_prism is False (0 real prism calls) → administered=False.
    5. ``comps.score(on_out.citations, ...)`` → precision_on.
    6. ``scan_leak(on_out.text)`` → leaked flag (Task 10).
    7. bundle_delta = precision_on - precision_base.
    8. Return PartCCell.

    Parameters
    ----------
    cell:
        ``(repo, stage, model)`` descriptor.
    comps:
        Composable component bundle with methods:
        - ``run_off_arm(cell) -> ArmOutput``
        - ``score(citations, **kwargs) -> float``
        - ``run_on_arm(cell) -> ArmOutput``
        Fakes in unit tests; live implementations in cli.py.
    """
    from .leak import scan_leak

    repo, stage, model = cell

    # Step 1: run the fresh prism-OFF status-quo arm (no steer, no prism)
    off_out = comps.run_off_arm(cell)

    # Step 2: score off-arm citations through the oracle → InvestigatorReport
    off_rep = comps.score(off_out.citations, cell=cell, arm="base")
    precision_base = off_rep.precision

    # Step 3: run the steered prism-on arm
    on_out = comps.run_on_arm(cell)

    # Step 4: gate — was prism actually administered?
    administered = on_out.used_prism  # False when prism_calls == 0

    # Step 5: score on-arm citations through the SAME oracle → InvestigatorReport
    on_rep = comps.score(on_out.citations, cell=cell, arm="on")
    precision_on = on_rep.precision

    # Step 6: extract per-arm citation failure breakdowns + full verdict lists
    def _breakdown(rep) -> dict:
        verdicts = rep.verdicts
        halluc = sum(v.is_hallucination for v in verdicts)
        irrel  = sum(1 for v in verdicts if not v.is_hallucination and not v.relevant)
        valid  = sum(v.is_valid for v in verdicts)
        fails  = [
            f"{v.cite.file}:{v.cite.line} ({'halluc' if v.is_hallucination else 'irrelevant'})"
            for v in verdicts if not v.is_valid
        ]
        return {"n": len(verdicts), "valid": valid, "halluc": halluc,
                "irrelevant": irrel, "fails": fails}

    def _verdict_list(rep) -> list:
        """Convert InvestigatorReport.verdicts → list of plain dicts for JSON serialisation."""
        result = []
        for v in rep.verdicts:
            result.append({
                "file": v.cite.file,
                "line": v.cite.line,
                "symbol": v.cite.symbol,
                "file_ok": v.file_ok,
                "line_ok": v.line_ok,
                "symbol_ok": v.symbol_ok,
                "relevant": v.relevant,
                "is_hallucination": v.is_hallucination,
                "is_valid": v.is_valid,
            })
        return result

    off_breakdown = _breakdown(off_rep)
    on_breakdown  = _breakdown(on_rep)
    off_verdicts  = _verdict_list(off_rep)
    on_verdicts   = _verdict_list(on_rep)

    # Step 6b: gate decision
    prism_calls_on = on_out.prism_calls
    distinct_tools = sorted(on_out.dose.distinct_tools) if on_out.dose.distinct_tools else []
    if prism_calls_on == 0:
        gate: dict = {
            "decision": "discard",
            "reason": "zero real prism calls",
            "prism_calls": 0,
            "distinct_tools": distinct_tools,
        }
    else:
        gate = {
            "decision": "keep",
            "reason": f"prism used ({prism_calls_on} calls)",
            "prism_calls": prism_calls_on,
            "distinct_tools": distinct_tools,
        }

    # Step 7: leak scan (on-arm text only)
    leak_result = scan_leak(on_out.text)

    # Step 8: compute delta
    bundle_delta = precision_on - precision_base

    # Head-to-head spec quality (optional on the comps protocol; back-compat for fakes).
    head_to_head = (comps.head_to_head(off_out, on_out, cell)
                    if hasattr(comps, "head_to_head") else {})

    # Step 9: token/cost/wall accounting — total tokens = input + output per arm
    in_tokens_off  = off_out.in_tokens
    out_tokens_off = off_out.tokens
    in_tokens_on   = on_out.in_tokens
    out_tokens_on  = on_out.tokens
    tokens_off = in_tokens_off + out_tokens_off
    tokens_on  = in_tokens_on  + out_tokens_on
    cost_off   = off_out.cost_usd
    cost_on    = on_out.cost_usd
    wall_off   = off_out.wall_s
    wall_on    = on_out.wall_s

    return PartCCell(
        repo=repo,
        stage=stage,
        model=model,
        precision_on=precision_on,
        precision_base=precision_base,
        bundle_delta=bundle_delta,
        dose=on_out.dose,
        low_dose=on_out.low_dose,
        administered=administered,
        leaked=leak_result.leaked,
        # D0: recall as a SEPARATE axis (owner rubric-a: never collapse into precision).
        # Previously hardcoded to None even though InvestigatorReport.recall was already
        # computed by comps.score() — the value was silently discarded.
        recall_on=on_rep.recall,
        recall_base=off_rep.recall,
        tokens_off=tokens_off,
        tokens_on=tokens_on,
        in_tokens_off=in_tokens_off,
        out_tokens_off=out_tokens_off,
        in_tokens_on=in_tokens_on,
        out_tokens_on=out_tokens_on,
        cost_off=cost_off,
        cost_on=cost_on,
        wall_off=wall_off,
        wall_on=wall_on,
        off_breakdown=off_breakdown,
        on_breakdown=on_breakdown,
        off_verdicts=off_verdicts,
        on_verdicts=on_verdicts,
        gate=gate,
        head_to_head=head_to_head,
    )


# ---------------------------------------------------------------------------
# Report renderer
# ---------------------------------------------------------------------------

_WIDTH = 76
_HEADER = (
    "directional pilot signal (n=1 per language)\n"
    + "=" * _WIDTH + "\n"
    + f"{'cell':<28} {'prec-off':>8} {'prec-on':>8} {'Δprec':>6} "
    + f"{'dose':>5} {'low?':>4} {'adm?':>4} {'leak?':>5}\n"
    + f"{'':28} {'tok-off':>8} {'tok-on':>8} {'Δtok':>6} "
    + f"{'cost-off':>9} {'cost-on':>8} {'Δcost':>8}\n"
    + "-" * _WIDTH
)


def _fmt_breakdown(label: str, bd: dict) -> str:
    """Render one cite-breakdown line: 'off cites: N = V valid / H halluc / I irrelevant'."""
    return (
        f"  {label} cites: {bd['n']} = {bd['valid']} valid "
        f"/ {bd['halluc']} halluc / {bd['irrelevant']} irrelevant"
    )


def render_partc(cells: list[PartCCell]) -> str:
    """Render a multi-line-per-cell pilot-signal table of Part-C cells.

    Per cell:
      Line 1 — precision and gate flags.
      Line 2 — token/cost accounting (totals).
      Line 3 — in/out token split per arm.
      Lines 4-5 — per-arm cite breakdowns (valid / halluc / irrelevant counts).

    The report is labelled "directional pilot signal (n=1 per language)".
    """
    lines = [_HEADER]
    for c in cells:
        cell_id = f"{c.repo}/{c.stage}/{c.model}"
        delta_tok = c.tokens_on - c.tokens_off
        delta_cost = c.cost_on - c.cost_off
        # Line 1: precision + gate flags
        lines.append(
            f"{cell_id:<28} "
            f"{c.precision_base:>8.3f} "
            f"{c.precision_on:>8.3f} "
            f"{c.bundle_delta:>+6.3f} "
            f"{c.dose.count:>5} "
            f"{'yes' if c.low_dose else 'no':>4} "
            f"{'yes' if c.administered else 'NO':>4} "
            f"{'YES' if c.leaked else 'no':>5}"
        )
        # Line 2: token/cost accounting (totals)
        lines.append(
            f"{'':28} "
            f"{c.tokens_off:>8} "
            f"{c.tokens_on:>8} "
            f"{delta_tok:>+6} "
            f"  ${c.cost_off:>7.4f} "
            f" ${c.cost_on:>7.4f} "
            f" ${delta_cost:>+8.4f}"
        )
        # Line 3: in/out token split
        lines.append(
            f"  tok off {c.in_tokens_off}in/{c.out_tokens_off}out  "
            f"on {c.in_tokens_on}in/{c.out_tokens_on}out  "
            f"Dtot {delta_tok:+}  D${delta_cost:+.2f}"
        )
        # Lines 4-5: per-arm cite breakdowns
        lines.append(_fmt_breakdown("off", c.off_breakdown))
        lines.append(_fmt_breakdown("on ", c.on_breakdown))
        h2h = getattr(c, "head_to_head", {}) or {}
        if h2h:
            esc = " (opus tiebreak)" if h2h.get("escalated") else ""
            lines.append(f"  head-to-head spec quality: {h2h.get('winner','?')}{esc}")
    return "\n".join(lines)
