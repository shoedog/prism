from tier_a.model import FunctionDef, Location
from tier_a.sut import extract_callers


def test_extract_callers_reads_resolution_kind():
    seed = FunctionDef("Go", "method", None, Location("main.go", 9, 9), 9)
    ev = {
        "items": [
            {
                "location": {"file": "main.go", "start_line": 12, "end_line": 12},
                "symbol": {"Function": {"name": "run"}},
                "score": 1.0,
                "why": [
                    {"CalledBy": {"caller": "run", "call_site_line": 12}},
                    {"Resolution": {"kind": "interface_dispatch"}},
                ],
            }
        ]
    }
    edges = extract_callers(seed, ev)
    assert edges[0].resolution_kind == "interface_dispatch"
