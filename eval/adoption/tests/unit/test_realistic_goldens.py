# eval/adoption/tests/unit/test_realistic_goldens.py
from adoption.goldens import load_realistic_probes

def test_loads_realistic():
    ps = load_realistic_probes()
    assert len(ps) == 5
    assert all(p.kind == "realistic" for p in ps)
    assert all(p.expected_tools == [] for p in ps)   # open-ended: no single expected tool
    assert {p.id for p in ps} >= {"spec-runstage-tiebreak", "analysis-dry-run-flag"}
