import pytest

from tier_a.model import CallEdge, FunctionDef, Location, edge_tier
from tier_a.sut import SutError, extract_callees, extract_callers


def test_extract_callers_reads_score():
    seed = FunctionDef("Go", "method", None, Location("main.go", 9, 9), 9)
    ev = {
        "items": [
            {
                "location": {"file": "main.go", "start_line": 12, "end_line": 12},
                "symbol": {"Function": {"name": "run"}},
                "score": 0.6,
                "why": [
                    {"CalledBy": {"caller": "run", "call_site_line": 12}},
                    {"Resolution": {"kind": "name_only"}},
                ],
            }
        ]
    }
    edges = extract_callers(seed, ev)
    assert edges[0].score == 0.6


def test_extract_callers_missing_score_raises():
    # score is a mandatory Evidence field on live prism JSON (types.rs:87); a
    # missing score means malformed wire output, not a legacy record -- fail
    # fast instead of silently defaulting to None/legacy-exact.
    seed = FunctionDef("Go", "method", None, Location("main.go", 9, 9), 9)
    ev = {
        "items": [
            {
                "location": {"file": "main.go", "start_line": 12, "end_line": 12},
                "symbol": {"Function": {"name": "run"}},
                "why": [{"CalledBy": {"caller": "run", "call_site_line": 12}}],
            }
        ]
    }
    with pytest.raises(SutError, match="missing numeric score"):
        extract_callers(seed, ev)


def test_extract_callers_null_score_raises():
    seed = FunctionDef("Go", "method", None, Location("main.go", 9, 9), 9)
    ev = {
        "items": [
            {
                "location": {"file": "main.go", "start_line": 12, "end_line": 12},
                "symbol": {"Function": {"name": "run"}},
                "score": None,
                "why": [{"CalledBy": {"caller": "run", "call_site_line": 12}}],
            }
        ]
    }
    with pytest.raises(SutError, match="missing numeric score"):
        extract_callers(seed, ev)


def test_extract_callees_non_numeric_score_raises():
    seed = FunctionDef("run", "function", None, Location("main.py", 3, 3), 3)
    ev = {
        "items": [
            {
                "location": {"file": "main.py", "start_line": 20, "end_line": 20},
                "symbol": {"Function": {"name": "helper"}},
                "score": "high",
                "why": [{"Calls": {"callee": "helper", "call_site_line": 5}}],
            }
        ]
    }
    with pytest.raises(SutError, match="missing numeric score"):
        extract_callees(seed, ev)


def test_stored_edge_without_score_still_classifies_exact():
    # Separate from the live-parser fail-fast above: a CallEdge rebuilt from a
    # STORED site that predates score capture (score=None) must still classify
    # as "exact" via model.edge_tier -- unchanged legacy/replay tolerance.
    seed = FunctionDef("run", "function", None, Location("main.go", 9, 9), 9)
    edge = CallEdge(
        "caller", seed, Location("main.go", 12, 12), "run", Location("main.go", 12, 12),
        "name_only", None,
    )
    assert edge.score is None
    assert edge_tier(edge) == "exact"


def test_extract_callees_reads_score():
    seed = FunctionDef("run", "function", None, Location("main.py", 3, 3), 3)
    ev = {
        "items": [
            {
                "location": {"file": "main.py", "start_line": 20, "end_line": 20},
                "symbol": {"Function": {"name": "helper"}},
                "score": 1.0,
                "why": [{"Calls": {"callee": "helper", "call_site_line": 5}}],
            }
        ]
    }
    edges = extract_callees(seed, ev)
    assert edges[0].score == 1.0
