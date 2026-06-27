"""Tests for scan_leak (Task 10)."""
from tier_c.leak import scan_leak


def test_leak_scanner_flags_and_redacts():
    f = scan_leak("We used nav_callers and Prism to find foo().")
    assert f.leaked and "nav_callers" not in f.redacted and "prism" not in f.redacted.lower()


def test_clean_text_is_not_leaked():
    f = scan_leak("We traced the call graph and found the bug at src/main.rs:42.")
    assert not f.leaked
    assert f.redacted == "We traced the call graph and found the bug at src/main.rs:42."


def test_all_nav_variants_flagged():
    for token in ("nav_callers", "nav_callees", "nav_repo_map", "nav_nodes_at", "nav_ego_graph"):
        f = scan_leak(f"Used {token} here.")
        assert f.leaked, f"Expected leak for {token}"
        assert token not in f.redacted


def test_prism_case_insensitive():
    for variant in ("Prism", "prism", "PRISM", "pRiSm"):
        f = scan_leak(f"Used {variant} to navigate.")
        assert f.leaked, f"Expected leak for {variant!r}"
        assert "prism" not in f.redacted.lower()


def test_nav_arbitrary_suffix_flagged():
    f = scan_leak("called nav_something_new here")
    assert f.leaked
    assert "nav_something_new" not in f.redacted


def test_all_matches_redacted_not_just_first():
    f = scan_leak("nav_callers then nav_callees then prism again.")
    assert f.leaked
    assert "nav_callers" not in f.redacted
    assert "nav_callees" not in f.redacted
    assert "prism" not in f.redacted.lower()


def test_prism_on_steer_text_itself_would_flag():
    """Sanity: the prism_on steer mentions nav tools → scanner WOULD flag it.
    That's fine; the steer is in the PROMPT not the model's output."""
    from tier_c.prompts import stage_prompt
    steer_prompt = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="prism_on")
    f = scan_leak(steer_prompt)
    assert f.leaked  # nav_* tokens appear in the steer directive


def test_leak_result_is_frozen():
    f = scan_leak("clean text")
    try:
        f.leaked = True  # type: ignore[misc]
        assert False, "Should have raised FrozenInstanceError"
    except Exception:
        pass
