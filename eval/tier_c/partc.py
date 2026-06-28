# eval/tier_c/partc.py
"""Part-C single-cell runner: steered-prism-on arm vs recovered baseline.

``run_partc_cell`` composes Tasks 7/9/10 into one cell and gates on real prism
calls (administered flag).  The live path is wired in ``cli.py``; fakes drive
unit tests with zero live spend.

``PartCCell`` is the frozen dataclass returned to the Verify Gate.
``render_partc`` produces a minimal pilot-signal table.
"""
from __future__ import annotations

from dataclasses import dataclass
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
    cost_off:
        USD cost of the prism-OFF arm run.
    cost_on:
        USD cost of the prism-ON arm run.
    wall_off:
        Wall-clock seconds for the prism-OFF arm run.
    wall_on:
        Wall-clock seconds for the prism-ON arm run.
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
    cost_off: float = 0.0
    cost_on: float = 0.0
    wall_off: float = 0.0
    wall_on: float = 0.0


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

    # Step 2: score off-arm citations through the oracle → baseline precision
    precision_base = comps.score(off_out.citations, cell=cell, arm="base")

    # Step 3: run the steered prism-on arm
    on_out = comps.run_on_arm(cell)

    # Step 4: gate — was prism actually administered?
    administered = on_out.used_prism  # False when prism_calls == 0

    # Step 5: score on-arm citations through the SAME oracle
    precision_on = comps.score(on_out.citations, cell=cell, arm="on")

    # Step 6: leak scan (on-arm text only)
    leak_result = scan_leak(on_out.text)

    # Step 7: compute delta
    bundle_delta = precision_on - precision_base

    # Step 8: token/cost/wall accounting — total tokens = input + output per arm
    tokens_off = off_out.in_tokens + off_out.tokens
    tokens_on  = on_out.in_tokens + on_out.tokens
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
        recall_on=None,
        recall_base=None,
        tokens_off=tokens_off,
        tokens_on=tokens_on,
        cost_off=cost_off,
        cost_on=cost_on,
        wall_off=wall_off,
        wall_on=wall_on,
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


def render_partc(cells: list[PartCCell]) -> str:
    """Render a two-line-per-cell pilot-signal table of Part-C cells.

    Line 1 — precision and gate flags:
        cell (repo/stage/model), precision-off, precision-on, Δprec,
        dose.count, low_dose, administered, leaked.

    Line 2 — token/cost accounting (indented under the cell column):
        tok-off (total tokens off-arm), tok-on, Δtok (signed),
        cost-off ($), cost-on ($), Δcost (signed $).

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
        # Line 2: token/cost accounting
        lines.append(
            f"{'':28} "
            f"{c.tokens_off:>8} "
            f"{c.tokens_on:>8} "
            f"{delta_tok:>+6} "
            f"  ${c.cost_off:>7.4f} "
            f" ${c.cost_on:>7.4f} "
            f" ${delta_cost:>+8.4f}"
        )
    return "\n".join(lines)
