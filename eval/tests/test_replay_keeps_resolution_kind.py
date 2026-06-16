import json

from tier_a import cli
from tier_a.model import CallEdge, FunctionDef, Location


def test_run_json_roundtrips_resolution_kind(tmp_path, monkeypatch):
    seed = FunctionDef("Go", "method", None, Location("main.go", 9, 9), 9)
    edge = CallEdge(
        "caller",
        seed,
        Location("main.go", 4, 6),
        "run",
        Location("main.go", 5, 5),
        "interface_dispatch",
    )
    probes = {
        "_corpus": "prism",
        "callers:main.go:9": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": "C-method",
            "seed_def": "main.go:9",
            "prism_sites": cli._stored_sites([edge], "callers"),
            "oracle_sites": [],
        },
    }
    path = tmp_path / "run.json"
    path.write_text(json.dumps({"meta": {"corpus": "prism"}, "probes": probes}))

    loaded = json.loads(path.read_text())
    stored = loaded["probes"]["callers:main.go:9"]["prism_sites"]
    assert cli._edges(stored, "callers")[0].resolution_kind == "interface_dispatch"

    monkeypatch.setattr(cli, "load_records", lambda _path: [])
    replayed = cli.recompute_metrics_from_stored(loaded)
    replayed_sites = replayed["probes"]["callers:main.go:9"]["prism_sites"]
    assert cli._edges(replayed_sites, "callers")[0].resolution_kind == "interface_dispatch"
    assert replayed["pending"][0]["dispatch_kind"] == "interface_dispatch"
