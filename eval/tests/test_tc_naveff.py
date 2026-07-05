"""D4 — navigation efficiency: mechanical, free, no judge. Pure aggregation over
already-saved ArmOutput fields; degrades to None (never fabricates 0.0) when the
harness didn't capture per-arm commands (true for every claude/opus arm today)."""
from __future__ import annotations

from tier_c.model import ArmOutput, Citation, Variant
from tier_c.naveff import nav_efficiency, nav_efficiency_to_dict


def _arm(*, commands=None, citations=None, tool_calls=0, wall_s=0.0, cost_usd=0.0) -> ArmOutput:
    return ArmOutput(
        variant=Variant("opus-4.8", True), text="x",
        citations=citations or [], tokens=0, tool_calls=tool_calls, wall_s=wall_s,
        used_prism=True, commands=commands or [], cost_usd=cost_usd,
    )


def test_cost_per_valid_citation_computed_when_valid_citations_present():
    arm = _arm(cost_usd=0.10)
    ne = nav_efficiency(arm, valid_citations=2)
    assert ne.cost_per_valid_citation == 0.05


def test_cost_per_valid_citation_none_when_no_valid_citations():
    arm = _arm(cost_usd=0.10)
    ne = nav_efficiency(arm, valid_citations=0)
    assert ne.cost_per_valid_citation is None


def test_wasted_exploration_rate_none_when_commands_unavailable():
    """Claude/opus arms today never populate ArmOutput.commands (parse_claude_stream_json
    doesn't capture it) -> must degrade to None, not fabricate 0.0."""
    arm = _arm(commands=[], citations=[Citation("a.py", 1, None)])
    ne = nav_efficiency(arm, valid_citations=1)
    assert ne.wasted_exploration_rate is None


def test_wasted_exploration_rate_computed_from_commands_vs_citations():
    """3 distinct files touched by commands, only 1 appears in citations -> 2/3 wasted."""
    commands = [
        "cat src/a.py",
        "grep -n foo src/b.py",
        "cat src/c.py",
    ]
    arm = _arm(commands=commands, citations=[Citation("src/a.py", 10, None)])
    ne = nav_efficiency(arm, valid_citations=1)
    assert ne.files_touched == 3
    assert ne.files_cited == 1
    assert ne.wasted_exploration_rate == 2 / 3


def test_wasted_exploration_rate_zero_when_all_touched_files_cited():
    commands = ["cat src/a.py"]
    arm = _arm(commands=commands, citations=[Citation("src/a.py", 1, None)])
    ne = nav_efficiency(arm, valid_citations=1)
    assert ne.wasted_exploration_rate == 0.0


def test_tool_calls_and_wall_s_pass_through_as_arm_level_totals():
    """These are TOTALS (spec's exact 'to first valid citation' timing is not tracked by
    the harness — documented degradation), not per-citation timings."""
    arm = _arm(tool_calls=7, wall_s=42.5)
    ne = nav_efficiency(arm, valid_citations=1)
    assert ne.tool_calls == 7
    assert ne.wall_s == 42.5


def test_nav_efficiency_to_dict_is_json_safe():
    arm = _arm(commands=["cat src/a.py"], citations=[Citation("src/a.py", 1, None)],
              tool_calls=3, wall_s=1.5, cost_usd=0.02)
    d = nav_efficiency_to_dict(nav_efficiency(arm, valid_citations=1))
    assert d["tool_calls"] == 3
    assert d["wall_s"] == 1.5
    assert d["cost_per_valid_citation"] == 0.02
    assert d["wasted_exploration_rate"] == 0.0
    assert d["files_touched"] == 1
    assert d["files_cited"] == 1
