"""P4: `impact` stage prompt + deterministic JSON-block parser (design §4 output
contract): a fenced ```json block {"impact":[...], "migration_order":[...]}
followed by free-text discussion. Deterministic parse; ONE auto re-prompt on
parse failure; failure-after-retry = ARM FAILURE (not a zero score)."""
from __future__ import annotations
import pytest

from tier_c.impact import (
    ImpactSite, parse_impact_block, run_impact_with_retry, RETRY_PROMPT,
)
from tier_c.prompts import stage_prompt, _STAGE
from tier_c.arm_runner import ArmRunError


# ---------------------------------------------------------------------------
# _STAGE["impact"] + stage_prompt identical-contract wiring
# ---------------------------------------------------------------------------

def test_impact_stage_registered():
    assert "impact" in _STAGE


def test_impact_stage_prompt_mentions_json_contract():
    p = stage_prompt("impact", issue_text="i", scoped_slice="s")
    assert "```json" in p
    assert '"impact"' in p
    assert '"migration_order"' in p


def test_impact_stage_prompt_identical_shape_both_arms():
    """The base impact instruction must be IDENTICAL regardless of steer — only
    the steer block differs (design §4: no condition tell)."""
    off = stage_prompt("impact", issue_text="i", scoped_slice="s", steer="")
    on = stage_prompt("impact", issue_text="i", scoped_slice="s", steer="prism_on")
    assert off in on  # 'on' == off + the appended steer block


# ---------------------------------------------------------------------------
# parse_impact_block — well-formed
# ---------------------------------------------------------------------------

WELL_FORMED = '''
Some preamble text.

```json
{"impact": [{"file": "a.go", "symbol": "Foo", "reason": "calls target"},
            {"file": "b.go", "symbol": "Bar", "reason": "dispatch"}],
 "migration_order": ["a.go:Foo", "b.go:Bar"]}
```

Design discussion follows here.
'''


def test_parse_well_formed_block():
    r = parse_impact_block(WELL_FORMED)
    assert r.ok
    assert r.sites == [
        ImpactSite(file="a.go", symbol="Foo", reason="calls target"),
        ImpactSite(file="b.go", symbol="Bar", reason="dispatch"),
    ]
    assert r.migration_order == ["a.go:Foo", "b.go:Bar"]


def test_parse_fenced_with_prose_around_it():
    text = "Let me think.\n\n```json\n" + \
        '{"impact": [{"file": "x.py", "symbol": "y"}], "migration_order": []}' + \
        "\n```\n\nAnd here is my reasoning about risk...\n"
    r = parse_impact_block(text)
    assert r.ok
    assert r.sites == [ImpactSite(file="x.py", symbol="y", reason="")]


def test_parse_uses_last_fenced_block_when_multiple():
    """A draft block earlier in the text must not shadow the final answer."""
    text = (
        '```json\n{"impact": [{"file": "draft.go", "symbol": "Draft"}], "migration_order": []}\n```\n'
        "Actually let me reconsider...\n"
        '```json\n{"impact": [{"file": "final.go", "symbol": "Final"}], "migration_order": []}\n```\n'
    )
    r = parse_impact_block(text)
    assert r.ok
    assert r.sites == [ImpactSite(file="final.go", symbol="Final", reason="")]


# ---------------------------------------------------------------------------
# parse_impact_block — malformed -> retry signal (ok=False, never raises)
# ---------------------------------------------------------------------------

def test_parse_no_fenced_block_returns_not_ok():
    r = parse_impact_block("just prose, no json block at all")
    assert not r.ok
    assert r.error


def test_parse_invalid_json_returns_not_ok():
    r = parse_impact_block("```json\n{not valid json,,,\n```\n")
    assert not r.ok
    assert r.error


def test_parse_missing_impact_key_returns_not_ok():
    r = parse_impact_block('```json\n{"migration_order": []}\n```\n')
    assert not r.ok


def test_parse_impact_not_a_list_returns_not_ok():
    r = parse_impact_block('```json\n{"impact": "not-a-list"}\n```\n')
    assert not r.ok


def test_parse_impact_item_missing_symbol_returns_not_ok():
    r = parse_impact_block('```json\n{"impact": [{"file": "a.go"}]}\n```\n')
    assert not r.ok


def test_parse_migration_order_optional_defaults_empty():
    r = parse_impact_block('```json\n{"impact": [{"file": "a.go", "symbol": "F"}]}\n```\n')
    assert r.ok
    assert r.migration_order == []


# ---------------------------------------------------------------------------
# run_impact_with_retry — ONE auto re-prompt; failure-after-retry = arm failure
# ---------------------------------------------------------------------------

class _FakeOut:
    def __init__(self, text, argv=None):
        self.text = text
        self.argv = argv or ["fake"]


def test_retry_not_needed_when_first_call_parses():
    calls = []

    def run_once(prompt):
        calls.append(prompt)
        return _FakeOut('```json\n{"impact": [{"file": "a.go", "symbol": "F"}]}\n```\n')

    out, parsed = run_impact_with_retry(run_once, "ORIGINAL PROMPT")
    assert len(calls) == 1
    assert parsed.ok
    assert calls[0] == "ORIGINAL PROMPT"


def test_retry_fires_once_on_first_parse_failure_then_succeeds():
    calls = []

    def run_once(prompt):
        calls.append(prompt)
        if len(calls) == 1:
            return _FakeOut("no json block here")
        return _FakeOut('```json\n{"impact": [{"file": "a.go", "symbol": "F"}]}\n```\n')

    out, parsed = run_impact_with_retry(run_once, "ORIGINAL PROMPT")
    assert len(calls) == 2
    assert parsed.ok
    assert "ORIGINAL PROMPT" in calls[1]
    assert RETRY_PROMPT in calls[1]


def test_failure_after_retry_raises_arm_run_error_not_zero_score():
    calls = []

    def run_once(prompt):
        calls.append(prompt)
        return _FakeOut("still no json block")

    with pytest.raises(ArmRunError):
        run_impact_with_retry(run_once, "ORIGINAL PROMPT")
    assert len(calls) == 2, "must call exactly ONCE + ONE retry, never more"
