# eval/adoption/tests/unit/test_goldens.py
from adoption.goldens import load_probes

def test_loads_all_probes():
    ps = load_probes()
    assert len(ps) == 12
    assert {p.kind for p in ps} == {"nav", "negative"}
    by = {p.id: p for p in ps}
    assert by["callers-count-claims"].expected_tools == ["nav_callers"]
    assert by["neg-docstring-salt"].expected_tools == []
