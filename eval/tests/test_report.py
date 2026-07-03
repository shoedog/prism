from types import SimpleNamespace

from tier_a.report import render_markdown

RUN = {
    "meta": {
        "corpus": "prism",
        "corpus_sha": "abc123def456",
        "corpus_dirty": False,
        "prism_sha": "abc123def456",
        "oracle": "rust-analyzer 1.94.0",
        "seed": 42,
        "harness_sha": "deadbeef",
        "date": "2026-06-11",
        "wall_s": {"m1": 1.0, "m2": 2.0, "m3": 0.5, "matrix": 0.2},
        "oracle_error_rate": 0.0,
        "sut_error_rate": 0.0,
        "baseline_invalid": False,
        "oracle_not_quiescent": False,
    },
    "m2": {
        "callers": {
            "U-free": {
                "raw": {
                    "precision": [1.0, 0.7, 1.0],
                    "recall": [1.0, 0.7, 1.0],
                    "tp": 10,
                    "fp": 0,
                    "fn": 0,
                },
                "corrected": {
                    "precision": [1.0, 0.7, 1.0],
                    "recall": [1.0, 0.7, 1.0],
                    "tp": 10,
                    "fp": 0,
                    "fn": 0,
                },
                "pending": 0,
                "shortfall": 0,
            }
        }
    },
}


def test_render_markdown_shows_wilson_and_metadata():
    md = render_markdown(RUN)
    assert "rust-analyzer 1.94.0" in md
    assert "1.00 [0.70–1.00]" in md
    assert "abc123def456" in md


def test_report_only_replay_recomputes_metrics_from_probes():
    import json

    from tier_a.cli import compute_m2_from_probes, recompute_metrics_from_stored

    probes = {
        "_corpus": "prism",
        "callers:src/s.rs:5": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": "U-free",
            "seed_def": "src/s.rs:5",
            "prism_sites": [["src/a.rs", 10, 10], ["src/b.rs", 99, 99]],
            "oracle_sites": [["src/a.rs", 9, 11]],
        },
    }
    stored = {"meta": {"corpus": "prism"}, "probes": probes,
              "m2": compute_m2_from_probes(probes, [])}
    roundtrip = json.loads(json.dumps(stored, default=str))
    again = recompute_metrics_from_stored(roundtrip)
    m = again["m2"]["callers"]["U-free"]
    assert m["raw"]["tp"] == 1
    assert m["raw"]["fp"] == 1
    assert m["pending"] == 1
    assert recompute_metrics_from_stored(roundtrip)["m2"] == again["m2"]


def test_report_only_without_probes_is_identity():
    from tier_a.cli import recompute_metrics_from_stored

    stored = {"meta": {"corpus": "x"}}
    assert recompute_metrics_from_stored(stored) == stored


def test_check_pinned_sha_guard():
    from tier_a.cli import check_pinned

    assert check_pinned({"pinned_sha": "abc"}, "abc", allow=False) == (True, None)
    assert check_pinned({}, "abc", allow=False) == (True, None)
    assert check_pinned({"pinned_sha": "old"}, "new", allow=True) == (True, None)
    assert check_pinned({"pinned_sha": "old"}, "new", allow=False) == (
        False,
        "corpus_sha_drift: new != pinned old",
    )


def test_inventory_miss_replay_scores_all_oracle_sites_as_fn():
    from tier_a.cli import compute_m2_from_probes

    probes = {
        "_corpus": "prism",
        "callers:src/s.rs:5": {
            "outcome": "inventory_miss",
            "direction": "callers",
            "stratum": "C-method",
            "seed_def": "src/s.rs:5",
            "prism_sites": [],
            "oracle_sites": [["src/a.rs", 9, 9], ["src/b.rs", 12, 12]],
        },
    }
    m = compute_m2_from_probes(probes, [])["callers"]["C-method"]
    assert m["raw"]["tp"] == 0
    assert m["raw"]["fp"] == 0
    assert m["raw"]["fn"] == 2
    assert m["pending"] == 2


def test_replay_counts_same_diff_site_once_per_probe():
    from tier_a.cli import compute_m2_from_probes

    probes = {
        "_corpus": "prism",
        "callers:src/a.rs:5": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": "C-name",
            "seed_def": "src/a.rs:5",
            "prism_sites": [["src/shared.rs", 10, 10]],
            "oracle_sites": [],
        },
        "callers:src/b.rs:8": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": "C-name",
            "seed_def": "src/b.rs:8",
            "prism_sites": [["src/shared.rs", 10, 10]],
            "oracle_sites": [],
        },
    }
    m = compute_m2_from_probes(probes, [])["callers"]["C-name"]
    assert m["raw"]["fp"] == 2
    assert m["pending"] == 2


def test_replay_reports_function_level_metrics_and_pending_records():
    from tier_a.cli import recompute_metrics_from_stored

    probes = {
        "_corpus": "prism",
        "callers:src/s.rs:5": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": "U-free",
            "seed_def": "src/s.rs:5",
            "prism_sites": [["src/a.rs", 10, 10]],
            "oracle_sites": [],
            "prism_functions": [["src/a.rs", "caller"]],
            "oracle_functions": [],
        },
    }
    run = recompute_metrics_from_stored({"meta": {"corpus": "prism"}, "probes": probes})
    m = run["m2"]["callers"]["U-free"]
    assert m["function"]["raw"]["fp"] == 1
    assert run["pending"] == [{
        "corpus": "prism",
        "measurement": "callers",
        "direction": "prism_only",
        "seed_def": "src/s.rs:5",
        "site": "src/a.rs:10",
        "site_fingerprint": None,
    }]


def test_replay_reports_stale_adjudication_count(monkeypatch):
    from tier_a import cli
    from tier_a.adjudication import Adjudication

    probes = {
        "_corpus": "prism",
        "callers:src/s.rs:5": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": "U-free",
            "seed_def": "src/s.rs:5",
            "prism_sites": [],
            "oracle_sites": [],
        },
    }
    stale = Adjudication(
        corpus="prism",
        measurement="callers",
        direction="prism_only",
        seed_def="src/s.rs:5",
        site="src/gone.rs:99",
        verdict="prism_fp",
        reason="r",
        adjudicated_by="t",
        date="2026-06-11",
    )
    monkeypatch.setattr(cli, "load_records", lambda _path: [stale])

    run = cli.recompute_metrics_from_stored({"meta": {"corpus": "prism"}, "probes": probes})
    assert run["meta"]["stale_adjudications"] == 1


def test_matrix_result_json_uses_sorted_arrays():
    from tier_a.cli import matrix_result_to_json
    from tier_a.matrix import CaseResult

    got = matrix_result_to_json(
        CaseResult(
            capability="cap",
            language="rust",
            outcome="regression",
            got={("b.rs", 2), ("a.rs", 1)},
            expected={("c.rs", 3)},
            got_kinds={("b.rs", 2): "exact", ("a.rs", 1): "name_only"},
            expected_resolution_kind="exact",
            forbid_resolution_kind=None,
        )
    )
    assert got["got"] == [["a.rs", 1], ["b.rs", 2]]
    assert got["expected"] == [["c.rs", 3]]
    assert got["probe"] == "callers"


def test_matrix_result_json_passes_through_taint_and_module_strings():
    """taint/module CaseResults carry deterministic triage-useful strings
    (not caller-site tuples), so matrix_result_to_json must pass them through
    unchanged rather than applying the callers-probe tuple-sorting transform."""
    from tier_a.cli import matrix_result_to_json
    from tier_a.matrix import CaseResult

    taint_result = CaseResult(
        capability="taint_case",
        language="python",
        outcome="regression",
        got="NotReached|warnings=none|sanitizers=false",
        expected="Reached|warnings=none|sanitizers=any",
        got_kinds={},
        expected_resolution_kind=None,
        forbid_resolution_kind=None,
        probe="taint",
    )
    got = matrix_result_to_json(taint_result)
    assert got["got"] == "NotReached|warnings=none|sanitizers=false"
    assert got["expected"] == "Reached|warnings=none|sanitizers=any"
    assert got["probe"] == "taint"
    assert got["got_kinds"] == {}
    # deterministic: repeated calls on the same result produce identical JSON
    assert matrix_result_to_json(taint_result) == got

    module_result = CaseResult(
        capability="module_case",
        language="python",
        outcome="ok",
        got="b.py",
        expected="b.py",
        got_kinds={},
        expected_resolution_kind=None,
        forbid_resolution_kind=None,
        probe="module_deps",
    )
    got2 = matrix_result_to_json(module_result)
    assert got2["got"] == "b.py"
    assert got2["probe"] == "module_deps"


def test_python_package_dirs_feed_q_scoped_classification(tmp_path):
    from tier_a.cli import package_dirs
    from tier_a.model import FunctionDef, Location
    from tier_a.strata import sample_strata

    pkg = tmp_path / "pkg"
    pkg.mkdir()
    (pkg / "__init__.py").write_text("")
    fd = FunctionDef("helper", "function", None, Location("pkg/mod.py", 1, 2), 1)

    sample = sample_strata(
        [fd],
        {"helper": 1},
        "python",
        seed=42,
        per_stratum=8,
        package_dirs=package_dirs(str(tmp_path)),
    )
    assert sample["Q-scoped"] == [fd]


def test_main_returns_nonzero_for_embedded_matrix_regression(monkeypatch, tmp_path):
    from tier_a import cli

    run = {
        "meta": {
            "corpus": "prism",
            "date": "2026-06-11",
            "baseline_invalid": False,
        },
        "matrix": [{"outcome": "regression"}],
    }
    monkeypatch.setattr(cli, "EVAL_DIR", tmp_path)
    monkeypatch.setattr(
        cli,
        "load_corpora",
        lambda: {"corpus": {"prism": {}}, "defaults": {}},
    )
    monkeypatch.setattr(cli, "run_corpus", lambda *_args: run)
    monkeypatch.setattr(cli, "write_reports", lambda *_args: None)
    monkeypatch.setattr("sys.argv", ["tier-a", "--date", "2026-06-11"])
    assert cli.main() == 1


def test_run_corpus_counts_inventory_miss_as_floor_success(monkeypatch, tmp_path):
    from tier_a import cli
    from tier_a.model import CallEdge, FunctionDef, Location

    seed = FunctionDef("seed", "function", None, Location("src/lib.rs", 1, 3), 1)

    class FakeOracle:
        not_quiescent = False

        def start(self):
            pass

        def version(self):
            return "fake-oracle 1"

        def capability_probe(self):
            return True

        def document_symbols(self, _file):
            return [seed]

        def callers(self, fd):
            return [CallEdge("caller", fd, None, None, Location("src/lib.rs", 2, 2))]

        def callees(self, fd):
            return [CallEdge("callee", fd, None, None, Location("src/lib.rs", 2, 2))]

    class FakeSut:
        sha = "abcdef123456"
        dirty = False

        def inventory(self, _root):
            return []

    monkeypatch.setattr(cli, "PrismCli", lambda *_args, **_kwargs: FakeSut())
    monkeypatch.setattr(cli, "make_oracle", lambda _cfg: FakeOracle())
    monkeypatch.setattr(cli, "corpus_sha", lambda _root: "abc123def456")
    monkeypatch.setattr(cli, "corpus_dirty", lambda _root: False)
    monkeypatch.setattr(cli, "untracked_sources", lambda *_args: [])
    monkeypatch.setattr(cli, "universe", lambda *_args, **_kwargs: ["src/lib.rs"])
    monkeypatch.setattr(cli, "snapshot_path", lambda *_args: tmp_path / "snap.json")

    run = cli.run_corpus(
        "tiny",
        {"path": str(tmp_path), "lang": "rust", "oracle": "rust-analyzer", "excludes": []},
        {
            "seed": 42,
            "per_stratum": 8,
            "settle_s": 2.0,
            "quiescence_cap_s": 300,
            "oracle_error_floor": {"rust": 0.10},
            "sut_error_floor": 0.05,
        },
        SimpleNamespace(
            sut_bin=None,
            allow_stale_sut=True,
            date="2026-06-11",
            quick=True,
        ),
    )

    assert not run["meta"]["baseline_invalid"]
    outcomes = [
        p["outcome"]
        for pid, p in run["probes"].items()
        if not pid.startswith("_")
    ]
    assert outcomes == ["inventory_miss", "inventory_miss"]


def test_run_corpus_matches_snapshot_symbols_not_drifting_live_inventory(
    monkeypatch,
    tmp_path,
):
    from tier_a import cli
    from tier_a.corpus import save_snapshot
    from tier_a.model import CallEdge, FunctionDef, Location

    seed = FunctionDef("seed", "function", None, Location("src/lib.rs", 1, 3), 1)
    snap = tmp_path / "snap.json"
    save_snapshot(snap, [seed])

    class DriftOracle:
        not_quiescent = False

        def start(self):
            pass

        def version(self):
            return "fake-oracle 1"

        def capability_probe(self):
            return True

        def document_symbols(self, _file):
            return []

        def callers(self, fd):
            return [CallEdge("caller", fd, None, None, Location("src/lib.rs", 2, 2))]

        def callees(self, fd):
            return [CallEdge("callee", fd, None, None, Location("src/lib.rs", 2, 2))]

    class MatchingSut:
        sha = "abcdef123456"
        dirty = False

        def inventory(self, _root):
            return [seed]

        def callers(self, _root, fd):
            return [CallEdge("caller", fd, None, None, Location("src/lib.rs", 2, 2))]

        def callees(self, _root, fd):
            return [CallEdge("callee", fd, None, None, Location("src/lib.rs", 2, 2))]

    monkeypatch.setattr(cli, "PrismCli", lambda *_args, **_kwargs: MatchingSut())
    monkeypatch.setattr(cli, "make_oracle", lambda _cfg: DriftOracle())
    monkeypatch.setattr(cli, "corpus_sha", lambda _root: "abc123def456")
    monkeypatch.setattr(cli, "corpus_dirty", lambda _root: False)
    monkeypatch.setattr(cli, "untracked_sources", lambda *_args: [])
    monkeypatch.setattr(cli, "universe", lambda *_args, **_kwargs: ["src/lib.rs"])
    monkeypatch.setattr(cli, "snapshot_path", lambda *_args: snap)

    run = cli.run_corpus(
        "tiny",
        {"path": str(tmp_path), "lang": "rust", "oracle": "rust-analyzer", "excludes": []},
        {
            "seed": 42,
            "per_stratum": 8,
            "settle_s": 2.0,
            "quiescence_cap_s": 300,
            "oracle_error_floor": {"rust": 0.10},
            "sut_error_floor": 0.05,
        },
        SimpleNamespace(
            sut_bin=None,
            allow_stale_sut=True,
            date="2026-06-11",
            quick=True,
        ),
    )

    outcomes = [
        p["outcome"]
        for pid, p in run["probes"].items()
        if not pid.startswith("_")
    ]
    assert outcomes == ["ok", "ok"]
    assert run["m1"]["matched"] == 0
    assert run["m1"]["snapshot_prism_missing"] == 0


def test_main_continues_after_corpus_error(monkeypatch, tmp_path):
    from tier_a import cli

    def fail_corpus(*_args):
        raise RuntimeError("missing gopls")

    monkeypatch.setattr(cli, "EVAL_DIR", tmp_path)
    monkeypatch.setattr(
        cli,
        "load_corpora",
        lambda: {"corpus": {"bad": {"path": str(tmp_path), "oracle": "gopls"}},
                 "defaults": {"seed": 42}},
    )
    monkeypatch.setattr(cli, "run_corpus", fail_corpus)
    monkeypatch.setattr(cli, "write_reports", lambda *_args: None)
    monkeypatch.setattr(cli, "corpus_sha", lambda _root: "abc123def456")
    monkeypatch.setattr(cli, "corpus_dirty", lambda _root: False)
    monkeypatch.setattr("sys.argv", ["tier-a", "--corpus", "all", "--date", "2026-06-11"])

    assert cli.main() == 2


def test_zero_denominator_metrics_are_strict_json_safe():
    import json

    from tier_a.metrics import precision_recall

    metric = precision_recall(0, 0, 0)
    assert metric["precision"] == (0.0, 0.0, 1.0)
    json.dumps(metric, allow_nan=False)


def test_resolve_capability_two_stage():
    # §2.2: overlay probe short-circuits; otherwise one retry against a real
    # inventory symbol (rust-analyzer ignores overlay files outside the workspace).
    from tier_a.cli import resolve_capability
    from tier_a.model import FunctionDef, Location
    from tier_a.oracles import OracleError

    fd = FunctionDef("f", "function", None, Location("src/a.rs", 1, 2), 1)

    class OkOracle:
        def callers(self, _fd):
            return []

    class DeadOracle:
        def callers(self, _fd):
            raise OracleError("no call hierarchy")

    assert resolve_capability(DeadOracle(), True, [])           # overlay won
    assert resolve_capability(OkOracle(), False, [fd])          # real-symbol retry
    assert not resolve_capability(DeadOracle(), False, [fd])    # both stages fail
    assert not resolve_capability(OkOracle(), False, [])        # nothing to retry on


def test_pending_skips_inventory_miss_probes():
    from tier_a.cli import _pending_for_probe
    probe = {"outcome": "inventory_miss", "direction": "callers",
             "seed_def": "a.go:107", "prism_sites": [],
             "oracle_sites": [["b.go", 40, 40]]}
    assert _pending_for_probe(probe, "caddy", []) == []
