import json

from tier_a import cli
from tier_a.model import CallEdge, FunctionDef, Location


def test_run_json_roundtrips_score():
    # P6a item 2b: M2 recomputation and --report-only replay rebuild edges from
    # stored probes[*].prism_sites via _edges() -- capturing score on a live
    # CallEdge alone is not enough; it must survive the _stored_sites -> JSON ->
    # _edges round trip, else every replayed edge classifies as legacy-exact.
    seed = FunctionDef("run", "function", None, Location("main.py", 9, 9), 9)
    edge = CallEdge(
        "caller",
        seed,
        Location("main.py", 4, 6),
        "run",
        Location("main.py", 5, 5),
        "name_only",
        0.6,
    )
    stored = cli._stored_sites([edge], "callers")
    roundtripped = json.loads(json.dumps(stored))
    rebuilt = cli._edges(roundtripped, "callers")
    assert rebuilt[0].score == 0.6
    assert rebuilt[0].resolution_kind == "name_only"


def test_stored_sites_omit_score_key_when_absent():
    # byte-compat: an edge with no score (the legacy/no-candidate-edges case)
    # must not gain a "score" key in the stored site metadata.
    seed = FunctionDef("run", "function", None, Location("main.py", 9, 9), 9)
    edge = CallEdge(
        "caller", seed, Location("main.py", 4, 6), "run", Location("main.py", 5, 5), "exact_owner"
    )
    [site] = cli._stored_sites([edge], "callers")
    assert site == ["main.py", 5, 5, {"resolution_kind": "exact_owner"}]
