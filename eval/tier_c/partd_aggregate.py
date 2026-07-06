"""Aggregate a whole Part-D run set (many `run-partd` cells under one run-store
root) into a corpus table + headline summary. Pure I/O over the persisted cell
JSONs (`<root>/<run_id>/<task>-impact-<model>.json`) — no LLM, no prism, no
re-run. The headline is SITE-level Δd-recall (design §5); this module only reads
what `_run_partd_live` already wrote.

Usage:
    uv run python -m tier_c.partd_aggregate <run-store-root> [<run-store-root> ...]

Each positional is a run-store root (e.g. `tier_c/runs/partd/full-codex-2026-07-06`).
Pass two (codex + claude roots) to print both model slates in one table.
"""
from __future__ import annotations

import glob
import json
import os
import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class CellSummary:
    task_id: str
    model: str
    d_recall_off: float
    d_recall_on: float
    d_recall_delta: float
    file_f1_delta: float
    dose: int
    administered: bool
    leaked: bool
    phantom_on: int
    d_gold_size: int
    claimed_off: int
    claimed_on: int
    path: str

    @property
    def off_saturated(self) -> bool:
        """The off arm already recovered the whole D-subset — this cell cannot
        discriminate prism regardless of the on arm (design caveat, 2026-07-06)."""
        return self.d_recall_off >= 1.0

    @property
    def valid_headline(self) -> bool:
        """A cell counts toward the headline only if prism was actually
        administered and the blinding did not break."""
        return self.administered and not self.leaked


def load_cells(root: str) -> list[CellSummary]:
    """Load every persisted Part-D cell JSON under a run-store root.

    A cell JSON is any `*-impact-*.json` (distinguishes cells from the sibling
    manifest.json/status.json). Malformed/partial files are skipped, not fatal.
    """
    cells: list[CellSummary] = []
    for path in sorted(glob.glob(os.path.join(root, "*", "*-impact-*.json"))):
        try:
            d = json.loads(open(path).read())
        except (OSError, json.JSONDecodeError):
            continue
        if "report_off" not in d or "report_on" not in d:
            continue
        off, on = d["report_off"], d["report_on"]
        dose = d.get("dose") or {}
        cells.append(CellSummary(
            task_id=d.get("task_id", "?"),
            model=d.get("model", "?"),
            d_recall_off=off.get("d_recall", 0.0),
            d_recall_on=on.get("d_recall", 0.0),
            d_recall_delta=d.get("d_recall_delta", on.get("d_recall", 0.0) - off.get("d_recall", 0.0)),
            file_f1_delta=d.get("file_f1_delta", 0.0),
            dose=int(dose.get("count", 0)) if isinstance(dose, dict) else 0,
            administered=bool(d.get("administered", False)),
            leaked=bool(d.get("leaked", False)),
            phantom_on=int(on.get("phantom", 0)),
            d_gold_size=int(on.get("d_gold_size", 0)),
            claimed_off=int(off.get("claimed_size", 0)),
            claimed_on=int(on.get("claimed_size", 0)),
            path=path,
        ))
    return cells


def _mean(xs: list[float]) -> float:
    return sum(xs) / len(xs) if xs else 0.0


def render(cells: list[CellSummary]) -> str:
    """Render the corpus table + headline summary. Cells are grouped by model."""
    if not cells:
        return "no Part-D cells found (looked for <root>/*/*-impact-*.json)"

    lines: list[str] = []
    width = 96
    for model in sorted({c.model for c in cells}):
        group = [c for c in cells if c.model == model]
        lines.append("=" * width)
        lines.append(f"Part-D corpus — model={model}  ({len(group)} cells)")
        lines.append("=" * width)
        lines.append(
            f"{'task':40} {'dR off':>7} {'dR on':>7} {'ΔdR':>7} "
            f"{'Δf-F1':>7} {'dose':>5} {'adm':>4} {'leak':>5} {'phan':>5}"
        )
        lines.append("-" * width)
        for c in sorted(group, key=lambda x: x.task_id):
            flags = []
            if not c.administered:
                flags.append("!ADM")
            if c.leaked:
                flags.append("!LEAK")
            if c.off_saturated:
                flags.append("sat")
            tag = ("  " + " ".join(flags)) if flags else ""
            lines.append(
                f"{c.task_id:40} {c.d_recall_off:>7.3f} {c.d_recall_on:>7.3f} "
                f"{c.d_recall_delta:>+7.3f} {c.file_f1_delta:>+7.3f} {c.dose:>5} "
                f"{'yes' if c.administered else 'NO':>4} "
                f"{'YES' if c.leaked else 'no':>5} {c.phantom_on:>5}{tag}"
            )
        lines.append("-" * width)

        valid = [c for c in group if c.valid_headline]
        excluded = [c for c in group if not c.valid_headline]
        saturated = [c for c in valid if c.off_saturated]
        discriminating = [c for c in valid if not c.off_saturated]

        lines.append(f"  cells: {len(group)}  |  valid-headline (administered & no-leak): {len(valid)}"
                     f"  |  excluded: {len(excluded)}")
        if excluded:
            lines.append("  EXCLUDED from headline: "
                         + ", ".join(f"{c.task_id}({'0-dose' if not c.administered else 'LEAK'})"
                                     for c in excluded))
        lines.append(f"  off-saturated (dR_off=1.0, cannot discriminate): {len(saturated)}"
                     + (("  -> " + ", ".join(c.task_id for c in saturated)) if saturated else ""))
        lines.append(f"  headline mean ΔdR (all valid, n={len(valid)}):           "
                     f"{_mean([c.d_recall_delta for c in valid]):+.3f}")
        lines.append(f"  headline mean ΔdR (discriminating only, n={len(discriminating)}): "
                     f"{_mean([c.d_recall_delta for c in discriminating]):+.3f}")
        lines.append(f"  mean Δfile-F1 (all valid): {_mean([c.file_f1_delta for c in valid]):+.3f}")
        lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv:
        print(__doc__)
        return 2
    cells: list[CellSummary] = []
    for root in argv:
        cells.extend(load_cells(root))
    print(render(cells))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
