# eval/adoption/tests/unit/test_codex_trajectory.py
"""TDD tests for parse_codex_stream against the codex_with_prism.jsonl fixture."""
from adoption.trajectory import parse_codex_stream

F = "adoption/tests/fixtures"  # cwd = eval/ when running pytest


def test_codex_fixture_has_prism_nav_call():
    t = parse_codex_stream(open(f"{F}/codex_with_prism.jsonl").read())
    assert t.prism_nav_calls(), "Expected at least one prism nav call"
    assert "nav_repo_map" in t.prism_nav_calls()


def test_codex_fixture_loaded_prism_skill():
    """A command_execution referencing prism-code-navigation/SKILL.md should count as loaded."""
    # The fixture reads from knowledge-ref/skills/prism-nav/SKILL.md (old path);
    # our parser must detect any SKILL.md read that mentions 'prism' in a skills path.
    # The fixture has a command_execution with command containing "prism-nav/SKILL.md".
    # skill_loads should include the detected skill name so loaded_prism_skill() returns True.
    t = parse_codex_stream(open(f"{F}/codex_with_prism.jsonl").read())
    # The fixture DID load a prism skill (just from knowledge-ref), so loaded_prism_skill() True.
    assert t.loaded_prism_skill() is True


def test_codex_final_text_is_last_agent_message():
    t = parse_codex_stream(open(f"{F}/codex_with_prism.jsonl").read())
    # The last agent_message text in the fixture
    assert "nav_repo_map" in t.final_text or len(t.final_text) > 10


def test_codex_tool_calls_include_bash_for_command_exec():
    """command_execution items should appear as ("Bash", {}) in tool_calls."""
    t = parse_codex_stream(open(f"{F}/codex_with_prism.jsonl").read())
    bash_calls = [n for n, _ in t.tool_calls if n == "Bash"]
    assert bash_calls, "Expected at least one Bash call from command_execution"


def test_codex_stream_empty_input():
    t = parse_codex_stream("")
    assert t.final_text == ""
    assert t.skill_loads == []
    assert t.tool_calls == []
    assert not t.prism_nav_calls()
    assert not t.loaded_prism_skill()


def test_codex_stream_no_prism_events():
    """A stream with only an agent_message and no mcp_tool_call should have no prism calls."""
    no_prism = '{"type":"item.completed","item":{"id":"x","type":"agent_message","text":"hello"}}\n'
    t = parse_codex_stream(no_prism)
    assert t.prism_nav_calls() == []
    assert t.final_text == "hello"


def test_codex_stream_mcp_tool_call_non_prism_server_ignored():
    """mcp_tool_call from a server other than 'prism' should NOT appear in prism_nav_calls."""
    line = '{"type":"item.completed","item":{"id":"x","type":"mcp_tool_call","server":"other","tool":"some_tool","arguments":{},"result":null,"error":null,"status":"completed"}}\n'
    t = parse_codex_stream(line)
    assert t.prism_nav_calls() == []
