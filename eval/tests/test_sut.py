import json
import subprocess
from pathlib import Path

import pytest

from tier_a.model import FunctionDef, Location
from tier_a.sut import (PrismCli, SutAmbiguous, SutError, SutSeedMiss, SutStale,
                        extract_callees, extract_callers, extract_functions,
                        parse_version)

FIX = Path(__file__).parent / "fixtures"
SEED = FunctionDef(
    "load_repo",
    "function",
    None,
    Location("src/repo_loader.rs", 1, 1),
    1,
)


def test_parse_version_sha_and_dirty():
    assert parse_version("slicing 3.1.2 (abcdef123456)\n") == ("abcdef123456", False)
    assert parse_version("slicing 3.1.2 (abcdef123456-dirty)\n") == (
        "abcdef123456",
        True,
    )
    assert parse_version("slicing 3.1.2 (abcdef1234567)\n") == ("abcdef1234567", False)
    with pytest.raises(SutStale, match="gitless build"):
        parse_version("slicing 3.1.2 (unknown)\n")


def test_extract_functions_inventory_records():
    recs = extract_functions(json.loads((FIX / "wire_functions_sample.json").read_text()))
    assert recs, "sample must be non-empty"
    r = recs[0]
    assert r.location.start_line >= 1 and r.location.end_line >= r.location.start_line
    assert r.selection_line == r.location.start_line


def test_extract_callers_per_site_edges():
    ev = json.loads((FIX / "wire_callers_sample.json").read_text())
    edges = extract_callers(SEED, ev)
    assert edges, "load_repo has known callers"
    e = edges[0]
    assert e.direction == "caller"
    assert e.other_def is not None
    assert e.other_def.start_line <= e.call_site.start_line <= e.other_def.end_line
    assert e.call_site.file == e.other_def.file


def test_extract_callees_unresolved_have_none_def():
    ev = json.loads((FIX / "wire_callees_sample.json").read_text())
    edges = extract_callees(SEED, ev)
    assert edges
    for e in edges:
        assert e.direction == "callee"
        assert e.call_site.file == SEED.location.file
        if e.other_def is None:
            assert e.other_name


class FakeRun:
    def __init__(self, returncode=0, stdout="", stderr=""):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr


def test_run_classifies_ambiguous_symbol(monkeypatch):
    def fake_run(*_args, **_kwargs):
        return FakeRun(3, '{"error":{"AmbiguousSymbol":{"candidates":[]}}}\n')

    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    monkeypatch.setattr(subprocess, "run", fake_run)
    with pytest.raises(SutAmbiguous):
        cli._run(["callers", "--repo", "/repo", "--symbol", "dup"])


def test_run_classifies_seed_miss(monkeypatch):
    def fake_run(*_args, **_kwargs):
        return FakeRun(3, '{"error":{"SymbolNotFound":{"seed":"loc:x.py:1"}}}\n')

    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    monkeypatch.setattr(subprocess, "run", fake_run)
    with pytest.raises(SutSeedMiss):
        cli._run(["callers", "--repo", "/repo", "--location", "x.py:1"])


def test_run_happy_path_json_decode_and_argv(monkeypatch):
    seen = []

    def fake_run(argv, **kwargs):
        seen.append((argv, kwargs))
        return FakeRun(stdout='{"items":[]}\n')

    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    monkeypatch.setattr(subprocess, "run", fake_run)
    assert cli._run(["callers", "--repo", "/repo"]) == {"items": []}
    assert seen == [
        (
            ["/tmp/prism", "nav", "callers", "--repo", "/repo", "--format", "json"],
            {"capture_output": True, "text": True},
        )
    ]


def test_run_invalid_json_and_non_ambiguous_error(monkeypatch):
    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"

    monkeypatch.setattr(subprocess, "run", lambda *_args, **_kwargs: FakeRun(stdout="{"))
    with pytest.raises(SutError, match="invalid JSON"):
        cli._run(["functions"])

    monkeypatch.setattr(
        subprocess,
        "run",
        lambda *_args, **_kwargs: FakeRun(3, "", "plain failure"),
    )
    with pytest.raises(SutError, match="plain failure"):
        cli._run(["functions"])


def test_constructor_discovers_binary_in_contract_order(monkeypatch):
    monkeypatch.setattr(PrismCli, "_check_freshness", lambda self: ("abcdef123456", False))
    monkeypatch.setenv("PRISM_BIN", "/env/prism")

    assert PrismCli("/repo", sut_bin="/arg/prism").bin == "/arg/prism"
    assert PrismCli("/repo").bin == "/env/prism"

    monkeypatch.delenv("PRISM_BIN")
    assert PrismCli("/repo").bin == "/repo/target/release/prism"


def test_check_freshness_accepts_sha_prefix_and_clean_repo(monkeypatch):
    calls = iter([
        FakeRun(stdout="slicing 3.1.2 (abcdef1234567)\n"),
        FakeRun(stdout="abcdef1234567890abcdef1234567890abcdef12\n"),
        FakeRun(stdout=""),
    ])

    monkeypatch.setattr(subprocess, "run", lambda *_args, **_kwargs: next(calls))
    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    cli.repo = "/repo"
    cli.allow_stale = False
    assert cli._check_freshness() == ("abcdef1234567", False)


def test_check_freshness_rejects_dirty_repo_even_if_binary_clean(monkeypatch):
    calls = iter([
        FakeRun(stdout="slicing 3.1.2 (abcdef123456)\n"),
        FakeRun(stdout="abcdef1234567890abcdef1234567890abcdef12\n"),
        FakeRun(stdout=" M src/main.rs\n"),
    ])

    monkeypatch.setattr(subprocess, "run", lambda *_args, **_kwargs: next(calls))
    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    cli.repo = "/repo"
    cli.allow_stale = False
    with pytest.raises(SutStale, match="repo has uncommitted changes"):
        cli._check_freshness()


def test_public_methods_wire_flags_to_run_and_extract(monkeypatch):
    calls = []
    functions = json.loads((FIX / "wire_functions_sample.json").read_text())
    callers = json.loads((FIX / "wire_callers_sample.json").read_text())
    callees = json.loads((FIX / "wire_callees_sample.json").read_text())

    def fake_run(self, args):
        calls.append(args)
        if args[0] == "functions":
            return functions
        if args[0] == "callers":
            return callers
        if args[0] == "callees":
            return callees
        raise AssertionError(args)

    monkeypatch.setattr(PrismCli, "_run", fake_run)
    cli = PrismCli.__new__(PrismCli)

    assert cli.inventory("/repo")
    assert cli.callers("/repo", SEED)
    assert cli.callees("/repo", SEED)
    assert cli.callers_by_symbol("/repo", "load_repo", "src/repo_loader.rs")
    assert calls == [
        ["functions", "--repo", "/repo"],
        [
            "callers",
            "--repo",
            "/repo",
            "--location",
            "src/repo_loader.rs:1",
            "--depth",
            "1",
        ],
        [
            "callees",
            "--repo",
            "/repo",
            "--location",
            "src/repo_loader.rs:1",
            "--depth",
            "1",
        ],
        [
            "callers",
            "--repo",
            "/repo",
            "--symbol",
            "load_repo",
            "--depth",
            "1",
            "--file",
            "src/repo_loader.rs",
        ],
    ]


def test_public_edge_methods_map_seed_miss_to_empty(monkeypatch):
    def fake_run(_self, _args):
        raise SutSeedMiss("no seed")

    monkeypatch.setattr(PrismCli, "_run", fake_run)
    cli = PrismCli.__new__(PrismCli)

    assert cli.callers("/repo", SEED) == []
    assert cli.callees("/repo", SEED) == []
    assert cli.callers_by_symbol("/repo", "missing") == []
