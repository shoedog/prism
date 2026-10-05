"""Regression evidence for MEAS-B; no live oracle or network required."""
import json
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import pytest

from tier_a.model import FunctionDef, Location, match_by_selection


def test_indexed_matching_preserves_consumption_and_ties(monkeypatch):
    from tier_a import model
    matcher_type = getattr(model, "SelectionMatcher")
    records = [FunctionDef("f", "function", None, Location(f"{i}.ts", 1, 9), 1)
               for i in range(200)]
    small = FunctionDef("f", "function", None, Location("0.ts", 2, 4), 2)
    records += [small, small]
    seeds = [FunctionDef("f", "function", None, Location("0.ts", 3, 3), 3)] * 3
    seeds += [FunctionDef(None, "function", None, Location("0.ts", 3, 3), 3)]
    expected, used = [], set()
    for fd in seeds:
        m = match_by_selection(fd, [r for r in records if r not in used])
        expected.append(m)
        if m is not None:
            used.add(m)
    sizes = []
    original = model.match_by_selection
    def counted(fd, candidates):
        sizes.append(len(candidates))
        return original(fd, candidates)
    monkeypatch.setattr(model, "match_by_selection", counted)
    matcher = matcher_type(records)
    assert [matcher.match(fd) for fd in seeds] == expected
    assert max(sizes) <= 3  # unrelated files must never enter the matching scan


def test_session_deadline_does_not_reset_per_query():
    from tier_a.lsp_client import LspClient, LspTimeout
    c = LspClient([sys.executable, str(Path(__file__).with_name("echo_server.py"))],
                  ".", default_timeout=2, session_timeout=0.15)
    try:
        c.start()
        assert c.request("test/echo", {}) == {}
        time.sleep(0.18)
        with pytest.raises(LspTimeout, match="budget"):
            c.request("test/echo", {})
    finally:
        c.stop()


def test_native_definitions_convert_columns_enrich_and_exclude_external(tmp_path):
    from tier_a.tsserver import TsserverOracle
    from tier_a.model import DefTarget
    o = TsserverOracle(["fake"], str(tmp_path), "ts")
    o._open = lambda rel: None
    fd = FunctionDef("target", "function", None, Location("target.ts", 4, 6), 4)
    answer = [{"file": str(tmp_path / "target.ts"),
               "start": {"line": 4, "offset": 10}, "end": {"line": 4, "offset": 16}},
              {"file": str(tmp_path.parent / "external.ts"),
               "start": {"line": 1, "offset": 1}, "end": {"line": 1, "offset": 2}}]
    sent = []
    def req(method, args):
        sent.append((method, args))
        return answer
    o._req = req
    assert o.definitions_at([fd], "caller.ts", 2, 3) == [
        DefTarget(Location("target.ts", 4, 4), "target", "function")]
    assert sent[0] == ("definition", {"file": str(tmp_path / "caller.ts"), "line": 2, "offset": 4})
    answer.clear()
    assert o.definitions_at([fd], "caller.ts", 2, 3) == []


def test_native_null_empty_timeout_and_server_error_are_distinct():
    from tier_a.tsserver import TsserverOracle
    from tier_a.oracles import OracleError, OracleTimeout
    from tier_a.lsp_client import LspServerError, LspTimeout
    o = TsserverOracle(["fake"], ".", "js")
    o.client.request = lambda *args: None
    assert o._req("open", {}, allow_null=True) is None
    with pytest.raises(OracleError, match="null result"):
        o._req("calls", {})
    o.client.request = lambda *args: []
    assert o._req("calls", {}) == []
    def timeout(*args): raise LspTimeout("deadline")
    o.client.request = timeout
    with pytest.raises(OracleTimeout, match="calls: deadline"):
        o._req("calls", {})
    def failed(*args): raise LspServerError({"code": "tsserver", "message": "broken"})
    o.client.request = failed
    with pytest.raises(OracleError, match="broken") as caught:
        o._req("calls", {})
    assert not isinstance(caught.value, OracleTimeout)


def test_nonreading_server_write_is_bounded_and_stop_reaps():
    import threading
    from tier_a.lsp_client import LspClient, LspTimeout
    c = LspClient([sys.executable, "-c", "import time; time.sleep(30)"],
                  ".", default_timeout=0.1)
    with pytest.raises(LspTimeout):
        c.start()
    errors = []
    def blocked_write():
        try:
            c.request("blocked", {"big": "x" * 1000000})
        except Exception as exc:
            errors.append(exc)
    worker = threading.Thread(target=blocked_write, daemon=True)
    t = time.monotonic()
    try:
        worker.start()
        worker.join(timeout=0.5)
        assert not worker.is_alive(), "request timeout must cover a blocked pipe write"
        assert len(errors) == 1 and isinstance(errors[0], LspTimeout)
    finally:
        if worker.is_alive():  # bounded cleanup also on the original broken client
            c._proc.terminate()
        c.stop()
        worker.join(timeout=1)
    assert time.monotonic() - t < 3
    assert c._proc.poll() is not None
    assert not c._pending


@pytest.mark.parametrize("server", ["pass", "import sys,time; sys.stdout.write('malformed\\n'); sys.stdout.flush(); time.sleep(30)"])
def test_dead_server_fails_promptly(server):
    from tier_a.lsp_client import LspClient, LspError
    c = LspClient([sys.executable, "-c", server], ".", default_timeout=5)
    t = time.monotonic()
    with pytest.raises(LspError):
        c.start()
    c.stop()
    assert time.monotonic() - t < 2


def test_timeout_is_distinct_from_other_oracle_errors():
    from tier_a.lsp_client import LspTimeout
    from tier_a.oracles import LspOracle, OracleTimeout
    from tier_a.accounting import CorpusAccounting
    o = LspOracle(["fake"], ".", "rust")
    def fail(*args, **kwargs):
        raise LspTimeout("deadline")
    o.client.request = fail
    with pytest.raises(OracleTimeout):
        o._req("query", {})
    acc = CorpusAccounting()
    acc.record("one", "oracle_timeout")
    acc.record("two", "ok")
    assert acc.oracle_error_rate() == 0.5


def test_failed_capability_probe_does_not_swallow_timeout():
    from tier_a.lsp_client import LspTimeout
    from tier_a.oracles import LspOracle, OracleTimeout
    o = LspOracle(["fake"], ".", "rust")
    o.did_open = lambda *args: None
    def fail(*args, **kwargs):
        raise LspTimeout("deadline")
    o.client.request = fail
    with pytest.raises(OracleTimeout):
        o.capability_probe()


def test_snapshot_reports_content_and_manifest_drift(tmp_path):
    import hashlib
    from tier_a.corpus import verify_source_manifest
    root = tmp_path / "source"
    root.mkdir()
    (root / "a.ts").write_text("function a() {}")
    manifest = tmp_path / "files.sha256"
    manifest.write_text(hashlib.sha256((root / "a.ts").read_bytes()).hexdigest() + "  a.ts\n")
    cfg = {"path": str(root), "source_manifest": str(manifest),
           "source_manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest()}
    assert verify_source_manifest(cfg) == (["a.ts"], [])
    (root / "a.ts").write_text("changed")
    (root / "extra.js").write_text("function surprise() {}")
    files, drift = verify_source_manifest(cfg)
    assert "source_content:a.ts" in drift
    assert "source_extra:extra.js" in drift
    manifest.write_text(manifest.read_text() + "junk\n")
    assert "source_manifest_sha256" in verify_source_manifest(cfg)[1]


def test_js_ts_extensions_and_strata(tmp_path):
    from tier_a.corpus import universe
    from tier_a.strata import is_nested
    for file in ("a.js", "b.jsx", "c.ts", "d.tsx", "ignore.py"):
        (tmp_path / file).write_text("")
    assert universe(str(tmp_path), "js", []) == ["a.js", "b.jsx"]
    assert universe(str(tmp_path), "ts", []) == ["c.ts", "d.tsx"]
    fd = FunctionDef("f", "function", None, Location("src/a.ts", 1, 2), 1)
    assert is_nested(fd, "ts")


def test_quick_honors_explicit_corpus_and_language_filter():
    from tier_a.cli import select_corpora
    cfg = {"quick": {"corpora": ["stable", "x-ts", "x-js"]}, "corpus": {
        "prism": {"lang": "rust"}, "stable": {"lang": "rust"},
        "x-ts": {"lang": "ts"}, "x-js": {"lang": "js"}}}
    args = SimpleNamespace(quick=True, corpus=None, lang="ts,js")
    assert select_corpora(cfg, args) == ["x-ts", "x-js"]
    args.corpus = "prism"
    with pytest.raises(ValueError, match="no corpora"):
        select_corpora(cfg, args)
    args.lang = None
    assert select_corpora(cfg, args) == ["prism"]


def test_tsserver_mapping_columns_external_and_multiline():
    from tier_a.tsserver import map_ts_calls
    fd = FunctionDef("f", "function", None, Location("a.ts", 2, 10), 2)
    span = {"start": {"line": 4, "offset": 3}, "end": {"line": 5, "offset": 1}}
    item = {"name": "g", "file": "/root/b.ts", "span": span,
            "selectionSpan": span, "kind": "function"}
    calls = [{"to": item, "fromSpans": [span]},
             {"to": {**item, "file": "/outside/b.ts"}, "fromSpans": [span]}]
    edges = map_ts_calls(fd, calls, "/root", "callees")
    assert len(edges) == 1
    assert edges[0].call_site == Location("a.ts", 4, 4)
    assert edges[0].other_def == Location("b.ts", 4, 4)


def test_tsserver_response_errors_and_no_content_are_not_empty_calls():
    from tier_a.tsserver import TsserverClient
    from tier_a.lsp_client import LspServerError
    c = TsserverClient(["fake"], ".")
    with pytest.raises(LspServerError):
        c._response({"success": False, "message": "No content available."})
    assert c._response({"success": True, "body": []}) == []


def test_sut_subprocess_timeout_is_classified(monkeypatch):
    import subprocess
    from tier_a.sut import PrismCli, SutTimeout
    sut = PrismCli.__new__(PrismCli)
    sut.bin = "fake"
    sut.query_timeout_s = 0.1
    def hung(cmd, **kwargs):
        assert kwargs["timeout"] == 0.1
        raise subprocess.TimeoutExpired(cmd, kwargs["timeout"])
    monkeypatch.setattr(subprocess, "run", hung)
    with pytest.raises(SutTimeout, match="sut_timeout"):
        sut._run(["functions"])


def test_sut_uses_workspace_cache_and_matrix_bypasses_it(monkeypatch):
    import subprocess
    from tier_a.sut import PrismCli
    sut = PrismCli.__new__(PrismCli)
    sut.bin = "fake"
    sut.cache_dir = "/workspace/target/tier-a-nav-cache"
    calls = []
    def run(cmd, **kwargs):
        calls.append(cmd)
        return SimpleNamespace(returncode=0, stdout="[]")
    monkeypatch.setattr(subprocess, "run", run)
    sut._run(["functions"])
    assert calls[-1][2:4] == ["--cache-dir", sut.cache_dir]
    sut.no_cache = True
    sut._run(["functions"])
    assert "--no-cache" in calls[-1]
    assert "--cache-dir" not in calls[-1]


def test_content_modified_retry_is_bounded_and_visible():
    from tier_a.lsp_client import LspServerError
    from tier_a.oracles import LspOracle, OracleError
    o = LspOracle(["fake"], ".", "rust")
    calls = []
    def transient(method, params, timeout=None):
        calls.append(timeout)
        if len(calls) == 1:
            raise LspServerError({"code": -32801, "message": "content modified"})
        return []
    o.client.request = transient
    assert o._req("query", {}) == []
    assert len(calls) == 2 and calls[1] < calls[0]
    assert len(o.retries) == 1
    def permanent(*args, **kwargs):
        raise LspServerError({"code": -32801, "message": "content modified"})
    o.client.request = permanent
    with pytest.raises(OracleError):
        o._req("query", {})
    assert len(o.retries) == 3  # at most two retries per request


@pytest.mark.parametrize("stage", ["start", "inventory", "query"])
def test_runner_retains_timeout_failure_and_partial_inventory_is_not_saved(monkeypatch, tmp_path, capsys, stage):
    from tier_a import cli
    from tier_a.oracles import OracleTimeout
    seed = FunctionDef("f", "function", None, Location("a.rs", 1, 2), 1)
    class Oracle:
        not_quiescent = False
        stopped = False
        def start(self):
            if stage == "start":
                raise OracleTimeout("initialize timed out")
        def version(self): return "fake"
        def capability_probe(self): return True
        def document_symbols(self, f):
            if stage == "inventory":
                raise OracleTimeout("documentSymbol timed out")
            return [seed]
        def callers(self, fd): raise OracleTimeout("incomingCalls timed out")
        def callees(self, fd): raise OracleTimeout("outgoingCalls timed out")
        def stop(self): self.stopped = True
    oracle = Oracle()
    sut = SimpleNamespace(sha="abc", dirty=False, inventory=lambda root: [seed])
    monkeypatch.setattr(cli, "PrismCli", lambda *args, **kwargs: sut)
    monkeypatch.setattr(cli, "make_oracle", lambda cfg: oracle)
    monkeypatch.setattr(cli, "corpus_sha", lambda path: "abc")
    monkeypatch.setattr(cli, "corpus_dirty", lambda path: False)
    monkeypatch.setattr(cli, "untracked_sources", lambda *args: [])
    monkeypatch.setattr(cli, "universe", lambda *args, **kwargs: ["a.rs"])
    snap = tmp_path / "snapshot.json"
    monkeypatch.setattr(cli, "snapshot_path", lambda *args: snap)
    run = cli.run_corpus("tiny", {"path": str(tmp_path), "lang": "rust", "oracle": "rust-analyzer"},
        {"seed": 42, "per_stratum": 8, "oracle_error_floor": {"rust": .1}, "sut_error_floor": .05},
        SimpleNamespace(sut_bin=None, allow_stale_sut=True, quick=True, date="test"))
    assert run["meta"]["baseline_invalid"]
    assert "oracle_timeout" in run["meta"]["invalid_reasons"]
    assert any(v["outcome"] == "oracle_timeout" for v in run["failures"].values())
    assert oracle.stopped
    assert "oracle_timeout" in capsys.readouterr().err
    if stage != "query":
        assert not snap.exists()


def test_tsserver_inventory_includes_arrow_and_excludes_scalar_and_alias(tmp_path):
    from tier_a.tsserver import TsserverOracle
    from tier_a.oracles import OracleError
    span = {"start": {"line": 2, "offset": 7}, "end": {"line": 4, "offset": 1}}
    tree = {"kind": "module", "text": "m", "childItems": [
        {"kind": "const", "text": name, "nameSpan": span} for name in ("arrow", "scalar", "alias")]}
    o = TsserverOracle(["fake"], str(tmp_path), "ts")
    o._open = lambda path: None
    counter = []
    def req(method, args, **kwargs):
        if method == "navtree": return tree
        counter.append(method)
        if len(counter) == 2: raise OracleError("No content available.")
        return {"name": "arrow", "file": str(tmp_path / "a.ts"), "span": span, "selectionSpan": span}
    o._req = req
    [fd] = o.document_symbols("a.ts")
    assert fd.name == "arrow" and fd.kind == "function" and fd.selection_char == 6
    assert fd.location == Location("a.ts", 2, 3)


def test_pinned_timeout_remains_explicit():
    from tier_a import pinned
    from tier_a.oracles import OracleTimeout
    seeds = [FunctionDef(p["symbol"], "function", None, Location(p.get("file") or "a.rs", 1, 2), 1)
             for p in pinned.PINNED]
    def timeout(fd): raise OracleTimeout("query timed out")
    sut = SimpleNamespace(callers_by_symbol=lambda *args: [])
    rows = pinned.run_pinned(SimpleNamespace(callers=timeout), sut, seeds, ".")
    assert [p["outcome"] for p in rows[:3]] == ["oracle_timeout"] * 3


def test_startup_quiescence_cap_is_a_timeout(monkeypatch):
    from tier_a.oracles import LspOracle, OracleTimeout
    o = LspOracle(["fake"], ".", "rust", settle_s=1, startup_timeout_s=0.01)
    o.client = SimpleNamespace(start=lambda: None, deadline=time.monotonic()+30,
        drain_notifications=lambda: [{"method": "$/progress", "params": {
            "token": "busy", "value": {"kind": "begin"}}}])
    with pytest.raises(OracleTimeout, match="quiescent"):
        o.start()
    assert o.not_quiescent


@pytest.mark.parametrize("features", [None, ["mcp"]])
def test_oracle_cargo_features_are_sent_only_when_configured(features):
    from tier_a.oracles import LspOracle
    o = LspOracle(["fake"], ".", "rust", cargo_features=features)
    c = o.client
    c._spawn = lambda: None
    sent = []
    def request(method, params):
        sent.append(params)
        return {}
    c.request = request
    c.notify = lambda *args: None
    c.start()
    if features is None:
        assert "initializationOptions" not in sent[0]
    else:
        assert sent[0]["initializationOptions"] == {"cargo": {"features": ["mcp"]}}


def test_markdown_exposes_timeout_and_oracle_configuration():
    from tier_a.report import render_markdown
    run = {"meta": {"corpus": "tiny", "date": "test", "corpus_sha": "abc",
        "prism_sha": "def", "oracle": "fake", "seed": 42, "harness_sha": "ghi",
        "oracle_error_rate": 1.0, "sut_error_rate": 0.0, "baseline_invalid": True,
        "oracle_not_quiescent": False, "wall_s": {}, "invalid_reasons": ["oracle_timeout"],
        "oracle_configuration": {"cargo_features": ["mcp"]}},
        "failures": {"callers:a.rs:1": {"outcome": "oracle_timeout", "error": "deadline"}}}
    md = render_markdown(run)
    assert "oracle_timeout" in md and "callers:a.rs:1" in md
    assert "oracle configuration" in md and "mcp" in md


@pytest.fixture
def snapshot_runner(monkeypatch, tmp_path):
    """Real pinned files; fake query answers isolate runner validity decisions."""
    import hashlib
    from tier_a import cli
    source = tmp_path / "source"
    source.mkdir()
    file = source / "a.rs"
    file.write_text("fn f() {}\n")
    manifest = tmp_path / "files.sha256"
    manifest.write_text(hashlib.sha256(file.read_bytes()).hexdigest() + "  a.rs\n")
    seed = FunctionDef("f", "function", None, Location("a.rs", 1, 1), 1)
    cfg = {"path": str(source), "lang": "rust", "oracle": "rust-analyzer",
           "pinned_sha": "abc", "source_manifest": str(manifest),
           "source_manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest()}
    sut = SimpleNamespace(sha="abc", dirty=False, inventory=lambda root: [seed],
                          callers=lambda *args: [], callees=lambda *args: [])
    monkeypatch.setattr(cli, "PrismCli", lambda *args, **kwargs: sut)
    monkeypatch.setattr(cli, "corpus_sha", lambda path: "abc")
    monkeypatch.setattr(cli, "corpus_dirty", lambda path: False)
    monkeypatch.setattr(cli, "untracked_sources", lambda *args: [])
    monkeypatch.setattr(cli, "universe", lambda *args, **kwargs: ["a.rs"])
    monkeypatch.setattr(cli, "snapshot_path", lambda *args: tmp_path / "snapshot.json")
    class Oracle:
        not_quiescent = False
        stopped = False
        inventory = [seed]
        on_stop = lambda self: None
        def start(self): pass
        def version(self): return "fake"
        def capability_probe(self): return True
        def document_symbols(self, file): return self.inventory
        def callers(self, fd): return []
        def callees(self, fd): return []
        def stop(self):
            self.stopped = True
            self.on_stop()
    oracle = Oracle()
    monkeypatch.setattr(cli, "make_oracle", lambda cfg: oracle)
    def run():
        return cli.run_corpus("tiny", cfg,
            {"seed": 42, "per_stratum": 8, "oracle_error_floor": {"rust": .1}, "sut_error_floor": .05},
            SimpleNamespace(sut_bin=None, allow_stale_sut=True, quick=True, date="test"))
    return run, oracle, cfg, file, manifest


@pytest.mark.parametrize("mutation", ["unchanged", "changed_source", "missing_manifest"])
def test_source_bookend_drift_invalidates_after_cleanup(snapshot_runner, mutation):
    run, oracle, cfg, file, manifest = snapshot_runner
    if mutation == "changed_source":
        oracle.on_stop = lambda: file.write_text("fn changed() {}\n")
    elif mutation == "missing_manifest":
        oracle.on_stop = manifest.unlink
    result = run()
    assert oracle.stopped
    assert result["meta"]["baseline_invalid"] == (mutation != "unchanged")
    reasons = result["meta"]["invalid_reasons"]
    if mutation == "changed_source":
        assert "final:source_content:a.rs" in reasons
    elif mutation == "missing_manifest":
        assert any(r.startswith("final:source_manifest_unavailable:") for r in reasons)


def test_empty_successful_inventory_never_claims_valid_accuracy(snapshot_runner):
    run, oracle, cfg, file, manifest = snapshot_runner
    oracle.inventory = []
    result = run()
    assert result["meta"]["baseline_invalid"]
    assert "no_scored_probes" in result["meta"]["invalid_reasons"]


@pytest.mark.parametrize("mutation", ["missing", "symlink", "malformed", "extra_config"])
def test_manifest_missing_symlink_malformed_and_extra_config(snapshot_runner, mutation):
    import hashlib
    from tier_a.corpus import verify_source_manifest
    run, oracle, cfg, file, manifest = snapshot_runner
    if mutation == "malformed":
        manifest.write_text("a" * 64 + "  ../outside.rs\n")
        cfg["source_manifest_sha256"] = hashlib.sha256(manifest.read_bytes()).hexdigest()
        with pytest.raises(ValueError, match="malformed"):
            verify_source_manifest(cfg)
        return
    if mutation in ("missing", "symlink"):
        file.unlink()
        if mutation == "symlink":
            file.symlink_to(manifest)
    else:
        (file.parent / "tsconfig.json").write_text("{}")
    _, drift = verify_source_manifest(cfg)
    expected = {"missing": "source_missing:a.rs", "symlink": "source_symlink:a.rs",
                "extra_config": "source_extra:tsconfig.json"}[mutation]
    assert expected in drift


def test_sut_expired_corpus_budget_does_not_launch_another_command(monkeypatch):
    import subprocess
    from tier_a.sut import PrismCli, SutError
    sut = PrismCli.__new__(PrismCli)
    sut.bin = "fake"
    sut.deadline = time.monotonic() - 1
    def forbidden(*args, **kwargs):
        raise AssertionError("expired budget launched a subprocess")
    monkeypatch.setattr(subprocess, "run", forbidden)
    with pytest.raises(SutError, match="corpus budget exhausted"):
        sut._run(["functions"])


@pytest.mark.parametrize("limit", ["0", "-1", "nan", "inf"])
def test_invalid_timeout_cli_rejected_before_work(monkeypatch, capsys, limit):
    from tier_a import cli
    monkeypatch.setattr(sys, "argv", ["tier-a", "--oracle-budget-s", limit])
    with pytest.raises(SystemExit) as exc:
        cli.main()
    assert exc.value.code == 2
    assert "timeouts must be positive" in capsys.readouterr().err


def test_out_dir_honored_without_writing_baseline_reports(monkeypatch, tmp_path):
    from tier_a import cli
    out = tmp_path / "separate"
    monkeypatch.setattr(cli, "EVAL_DIR", tmp_path)
    monkeypatch.setattr(cli, "load_corpora", lambda: {"corpus": {"tiny": {"lang": "ts"}}, "defaults": {}})
    monkeypatch.setattr(cli, "run_corpus", lambda *args: {
        "meta": {"corpus": "tiny", "date": "test", "baseline_invalid": False}})
    destinations = []
    monkeypatch.setattr(cli, "write_reports", lambda run, path: destinations.append(path))
    monkeypatch.setattr(sys, "argv", ["tier-a", "--corpus", "tiny", "--out-dir", str(out)])
    assert cli.main() == 0
    assert destinations == [out]
    assert not (tmp_path.parent / "docs/eval/tier-a").exists()


def test_aggregate_counts_and_empty_denominators():
    from tier_a.cli import summarize_m2
    result = summarize_m2({"callers": {
        "one": {"raw": {"tp": 10, "fp": 1, "fn": 2}, "exact_tier": {"raw": {"tp": 3, "fp": 0, "fn": 2}}},
        "two": {"raw": {"tp": 4, "fp": 3, "fn": 4}, "exact_tier": {"raw": {"tp": 0, "fp": 0, "fn": 4}}}}})
    raw = result["callers"]["raw"]
    assert (raw["tp"], raw["fp"], raw["fn"]) == (14, 4, 6)
    assert raw["precision"][0] == 14 / 18 and raw["recall"][0] == 14 / 20
    assert result["callers"]["exact_tier"]["recall"][0] == 1 / 3
    empty = summarize_m2({"callees": {}})
    assert empty["callees"]["raw"]["tp"] == 0
    json.dumps(empty, allow_nan=False)


def test_native_incoming_calls_use_caller_file_and_exclude_external():
    from tier_a.tsserver import map_ts_calls
    seed = FunctionDef("f", "function", None, Location("seed.ts", 1, 3), 1)
    span = {"start": {"line": 7, "offset": 2}, "end": {"line": 7, "offset": 9}}
    item = {"file": "/root/caller.js", "name": "g", "span": span}
    calls = [{"from": item, "fromSpans": [span]},
             {"from": {**item, "file": "/external/g.js"}, "fromSpans": [span]}]
    [edge] = map_ts_calls(seed, calls, "/root", "callers")
    assert edge.call_site == Location("caller.js", 7, 7)
    assert edge.other_name == "g"
    assert map_ts_calls(seed, [], "/root", "callers") == []


def test_native_protocol_roundtrip_error_and_late_response_correlation(tmp_path):
    from tier_a.tsserver import TsserverClient
    from tier_a.lsp_client import LspServerError
    server = '''import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg['command'] == 'exit': break
    reply = dict(seq=0, type='response', command=msg['command'], request_seq=msg['seq'], success=msg['command'] != 'bad')
    if not reply['success']: reply['message'] = 'server failure'
    else: reply['body'] = msg['arguments'] if msg['command'] == 'echo' else None
    body = json.dumps(reply).encode()
    sys.stdout.buffer.write(('Content-Length: %d\\r\\n\\r\\n' % len(body)).encode() + body + b'\\n')
    sys.stdout.buffer.flush()
'''
    c = TsserverClient([sys.executable, "-u", "-c", server], str(tmp_path), default_timeout=1)
    try:
        c.start()
        assert c.request("echo", {"x": 1}) == {"x": 1}
        with pytest.raises(LspServerError, match="server failure"):
            c.request("bad", {})
        # A response for a timed-out/unknown request must not answer this request.
        c._dispatch({"type": "response", "request_seq": 9999, "success": True, "body": "late"})
        assert c.request("echo", {"after": True}) == {"after": True}
    finally:
        c.stop()
