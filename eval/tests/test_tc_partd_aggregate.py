"""Unit tests for tier_c.partd_aggregate — corpus roll-up over persisted Part-D
cell JSONs. Pure I/O; no live run. Mirrors the persisted cell schema written by
partd._run_partd_live via cli._persist_partc_cell."""
from __future__ import annotations

import json
import os

from tier_c.partd_aggregate import CellSummary, load_cells, render


def _cell(*, task, model, dr_off, dr_on, dose, administered=True, leaked=False,
          phantom=0, file_f1_delta=0.0, d_gold_size=6):
    return {
        "task_id": task, "model": model,
        "report_off": {"d_recall": dr_off, "phantom": 0, "claimed_size": 20,
                       "d_gold_size": d_gold_size},
        "report_on": {"d_recall": dr_on, "phantom": phantom, "claimed_size": 30,
                      "d_gold_size": d_gold_size},
        "d_recall_delta": dr_on - dr_off,
        "file_f1_delta": file_f1_delta,
        "dose": {"count": dose, "distinct_tools": [], "errors": 0},
        "administered": administered, "leaked": leaked,
    }


def _write_cell(root, run_id, cell):
    run_dir = os.path.join(root, run_id)
    os.makedirs(run_dir, exist_ok=True)
    safe_model = cell["model"].replace("/", "_").replace(":", "_")
    path = os.path.join(run_dir, f"{cell['task_id']}-impact-{safe_model}.json")
    with open(path, "w") as f:
        json.dump(cell, f)
    # sibling non-cell files must be ignored by the glob
    with open(os.path.join(run_dir, "status.json"), "w") as f:
        json.dump({"status": "success"}, f)
    with open(os.path.join(run_dir, "manifest.json"), "w") as f:
        json.dump({"cell": {}}, f)
    return path


def test_load_cells_ignores_manifest_and_status(tmp_path):
    root = str(tmp_path)
    _write_cell(root, "t1", _cell(task="alpha", model="gpt-5.5", dr_off=0.3, dr_on=0.8, dose=7))
    cells = load_cells(root)
    assert len(cells) == 1
    c = cells[0]
    assert isinstance(c, CellSummary)
    assert c.task_id == "alpha" and c.model == "gpt-5.5"
    assert c.d_recall_off == 0.3 and c.d_recall_on == 0.8
    assert abs(c.d_recall_delta - 0.5) < 1e-9
    assert c.dose == 7 and c.administered and not c.leaked


def test_off_saturated_and_valid_headline_flags():
    sat = CellSummary("s", "m", 1.0, 1.0, 0.0, 0.0, 5, True, False, 0, 6, 20, 30, "p")
    assert sat.off_saturated and sat.valid_headline
    zero_dose = CellSummary("z", "m", 0.2, 0.6, 0.4, 0.0, 0, False, False, 0, 6, 20, 30, "p")
    assert not zero_dose.valid_headline  # 0-dose -> excluded from headline
    leaked = CellSummary("l", "m", 0.2, 0.6, 0.4, 0.0, 5, True, True, 0, 6, 20, 30, "p")
    assert not leaked.valid_headline     # blinding break -> excluded


def test_render_summary_separates_saturated_from_discriminating(tmp_path):
    root = str(tmp_path)
    # discriminating cell: off < 1.0, prism lifts d-recall
    _write_cell(root, "t1", _cell(task="disc", model="gpt-5.5", dr_off=0.4, dr_on=0.9, dose=8))
    # saturated cell: off already 1.0
    _write_cell(root, "t2", _cell(task="sat", model="gpt-5.5", dr_off=1.0, dr_on=1.0, dose=5))
    # 0-dose cell: excluded from headline entirely
    _write_cell(root, "t3", _cell(task="dud", model="gpt-5.5", dr_off=0.5, dr_on=0.5,
                                  dose=0, administered=False))
    out = render(load_cells(root))
    assert "model=gpt-5.5" in out
    assert "off-saturated" in out and "sat" in out
    # valid-headline excludes the 0-dose cell (2 of 3 valid)
    assert "valid-headline (administered & no-leak): 2" in out
    # discriminating-only mean = the single disc cell's +0.5
    assert "n=1): +0.500" in out


def test_render_empty():
    assert "no Part-D cells found" in render([])
