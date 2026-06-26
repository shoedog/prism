# eval/adoption/tests/unit/test_testcase.py
from adoption.model import Probe, Trajectory
from adoption.testcase import build_test_case

def test_build_test_case_maps_tools_and_skill():
    probe = Probe(id="x", kind="nav", prompt="who calls foo", repo="tier_c",
                  expected_tools=["nav_callers"], expected_symbol="foo")
    traj = Trajectory(final_text="foo at a.py:1", skill_loads=["prism-code-navigation"],
                      tool_calls=[("nav_callers", {"symbol": "foo"})])
    tc = build_test_case(traj, probe)
    assert tc.input == "who calls foo"
    assert tc.actual_output == "foo at a.py:1"
    assert [t.name for t in tc.tools_called] == ["nav_callers"]
    assert [t.name for t in tc.expected_tools] == ["nav_callers"]
    assert tc.metadata["prism_skill_loaded"] is True
