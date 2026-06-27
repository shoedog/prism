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
        Citation precision of the recovered prism-off baseline (same oracle).
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


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def run_partc_cell(cell: tuple, comps: Any) -> PartCCell:
    """Build a single Part-C cell from a (repo, stage, model) triple + injected comps.

    Flow
    ----
    1. ``comps.load_base(cell)`` → base text (Task 7 header-stripped markdown).
    2. ``comps.extract_citations(base_text)`` → list[Citation] from base text.
    3. ``comps.score(base_cites, ...)`` → precision_base (same oracle as on-arm).
    4. ``comps.run_on_arm(cell)`` → ArmOutput (steered prism-on, Task 9 isolated).
    5. Gate: if on_out.used_prism is False (0 real prism calls) → administered=False.
    6. ``comps.score(on_out.citations, ...)`` → precision_on.
    7. ``scan_leak(on_out.text)`` → leaked flag (Task 10).
    8. bundle_delta = precision_on - precision_base.
    9. Return PartCCell.

    Parameters
    ----------
    cell:
        ``(repo, stage, model)`` descriptor.
    comps:
        Composable component bundle with methods:
        - ``load_base(cell) -> str``
        - ``extract_citations(text) -> list[Citation]``
        - ``score(citations, **kwargs) -> float``
        - ``run_on_arm(cell) -> ArmOutput``
        Fakes in unit tests; live implementations in cli.py.
    """
    from .leak import scan_leak

    repo, stage, model = cell

    # Step 1: load baseline text (prism-off, header-stripped)
    base_text = comps.load_base(cell)

    # Step 2: extract citations from base text
    base_cites = comps.extract_citations(base_text)

    # Step 3: score base citations through the oracle
    precision_base = comps.score(base_cites, cell=cell, arm="base")

    # Step 4: run the steered prism-on arm
    on_out = comps.run_on_arm(cell)

    # Step 5: gate — was prism actually administered?
    administered = on_out.used_prism  # False when prism_calls == 0

    # Step 6: score on-arm citations through the SAME oracle
    precision_on = comps.score(on_out.citations, cell=cell, arm="on")

    # Step 7: leak scan
    leak_result = scan_leak(on_out.text)

    # Step 8: compute delta
    bundle_delta = precision_on - precision_base

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
    )


# ---------------------------------------------------------------------------
# Report renderer
# ---------------------------------------------------------------------------

_HEADER = (
    "directional pilot signal (n=1 per language)\n"
    + "=" * 60 + "\n"
    + f"{'cell':<28} {'base':>6} {'on':>6} {'Δ':>6} {'dose':>5} "
    + f"{'low?':>5} {'adm?':>5} {'leak?':>5}\n"
    + "-" * 60
)


def render_partc(cells: list[PartCCell]) -> str:
    """Render a minimal pilot-signal table of Part-C cells.

    Columns: cell (repo/stage/model), precision base, precision on,
    Δ (bundle_delta), dose.count, low_dose, administered, leaked.

    The report is labelled "directional pilot signal (n=1 per language)".
    """
    lines = [_HEADER]
    for c in cells:
        cell_id = f"{c.repo}/{c.stage}/{c.model}"
        lines.append(
            f"{cell_id:<28} "
            f"{c.precision_base:>6.3f} "
            f"{c.precision_on:>6.3f} "
            f"{c.bundle_delta:>+6.3f} "
            f"{c.dose.count:>5} "
            f"{'yes' if c.low_dose else 'no':>5} "
            f"{'yes' if c.administered else 'NO':>5} "
            f"{'YES' if c.leaked else 'no':>5}"
        )
    return "\n".join(lines)
