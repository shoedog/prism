"""Tests for stage_prompt steer parameter (Task 10)."""
from tier_c.prompts import stage_prompt


def _base(steer="") -> str:
    return stage_prompt("spec", issue_text="i", scoped_slice="s", steer=steer)


def test_no_steer_is_byte_identical_to_old_prompt():
    """steer='' must be byte-identical to old stage_prompt (no steer appended)."""
    old = stage_prompt("spec", issue_text="i", scoped_slice="s")
    new = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="")
    assert old == new


def test_prism_on_steer_directs_nav_and_forbids_naming_tools():
    p = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="prism_on")
    assert "nav_callers" in p and "do not name the tools" in p.lower()


def test_prism_on_steer_contains_all_nav_tools():
    p = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="prism_on")
    for tool in ("nav_callers", "nav_callees", "nav_repo_map", "nav_nodes_at"):
        assert tool in p, f"Missing nav tool: {tool}"


def test_capability_steer_names_task_not_prism():
    p = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="capability")
    assert "who calls" in p.lower() and "prism" not in p.lower() and "nav_" not in p.lower()


def test_capability_steer_also_forbids_naming_tools():
    p = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="capability")
    assert "do not name the tools" in p.lower()


def test_unknown_steer_appends_nothing():
    base = _base("")
    other = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="bogus")
    assert other == base


def test_steer_does_not_change_upstream_behavior():
    with_up = stage_prompt("spec", issue_text="i", scoped_slice="s", upstream="U")
    with_up_steer = stage_prompt("spec", issue_text="i", scoped_slice="s", upstream="U", steer="")
    assert with_up == with_up_steer
