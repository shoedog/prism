# eval/adoption/tests/unit/test_invocation_metric.py
from adoption.model import Trajectory
from adoption.aggregate import prism_invoked, summarize_cells

def test_prism_invoked():
    assert prism_invoked(Trajectory("ans", ["prism-code-navigation"], [("nav_callers", {})])) is True
    assert prism_invoked(Trajectory("ans", [], [("Bash", {})])) is False

def test_summarize_cells_rate_and_attribution():
    # cell -> probe -> list of (invoked, skill_loaded_name_or_None) per trial
    cells = {
      "cell4": {
        "s1": [(True, "prism-code-navigation"), (True, "prism-nav"), (False, None), (True, "prism-code-navigation"), (True, "prism-code-navigation")],
        "s2": [(False, None)] * 5,
      }
    }
    out = summarize_cells(cells)
    c = out["cell4"]
    assert c["invocation_rate"] == 4/10          # 4 of 10 sample×trial runs invoked
    assert c["pass5_rate"] == 0.0                # neither sample hit 5/5
    assert c["skill_attribution"]["prism-code-navigation"] == 3
    assert c["skill_attribution"]["prism-nav"] == 1
