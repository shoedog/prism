from tier_a.model import FunctionDef, Location
from tier_a.sut import extract_callees, extract_callers


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


def test_extract_callers_missing_score_is_none():
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
    edges = extract_callers(seed, ev)
    assert edges[0].score is None


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
