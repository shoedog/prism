# eval/adoption/tests/unit/test_trajectory.py
from adoption.trajectory import parse_stream_json
F = "adoption/tests/fixtures"  # cwd = eval/ when running pytest

def test_detects_prism_calls():
    t = parse_stream_json(open(f"{F}/with_prism.jsonl").read())
    assert t.prism_nav_calls()              # e.g. ['nav_nodes_at','nav_callers',...]
    assert "nav_callers" in t.prism_nav_calls()

def test_no_prism_calls_in_baseline():
    t = parse_stream_json(open(f"{F}/without_prism.jsonl").read())
    assert t.prism_nav_calls() == []

def test_detects_skill_load_and_args():
    t = parse_stream_json(open(f"{F}/with_skill.jsonl").read())
    assert t.loaded_prism_skill() is True
    assert ("nav_callers", {"symbol": "foo"}) in t.tool_calls
    assert t.final_text == "foo is called at a.py:1"
