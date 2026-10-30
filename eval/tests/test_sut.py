import json
import subprocess
from pathlib import Path

import pytest

from tier_a import sut as sut_mod
from tier_a.closure import raw_digest
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


def test_from_archive_binds_bytes_and_full_sha_without_head_freshness(
    tmp_path, monkeypatch
):
    binary = tmp_path / "prism"
    binary.write_bytes(b"archived prism")
    binary.chmod(0o755)
    sha_full = "a" * 40
    monkeypatch.setattr(
        subprocess,
        "run",
        lambda *_args, **_kwargs: FakeRun(
            stdout=f"slicing 3.1.2 ({sha_full[:12]})\n"
        ),
    )

    sut = PrismCli.from_archive(
        "/repo", str(binary), sha_full, raw_digest(binary)
    )
    assert (sut.sha, sut.sha_full, sut.dirty) == (sha_full[:12], sha_full, False)
    with pytest.raises(SutStale, match="digest"):
        PrismCli.from_archive("/repo", str(binary), sha_full, "sha256:wrong")
    monkeypatch.setattr(
        subprocess,
        "run",
        lambda *_args, **_kwargs: FakeRun(stdout=f"slicing 3.1.2 ({'b' * 12})\n"),
    )
    with pytest.raises(SutStale, match="SHA"):
        PrismCli.from_archive("/repo", str(binary), sha_full, raw_digest(binary))


def test_lock_bound_instances_use_digest_partitioned_nav_store(tmp_path, monkeypatch):
    binary = tmp_path / "prism"
    binary.write_bytes(b"lock-bound prism")
    binary.chmod(0o755)
    sha_full = "c" * 40
    calls = []
    cache_root = tmp_path / "tier-a-root"
    monkeypatch.setenv("TIER_A_ROOT", str(cache_root))

    def run(cmd, **kwargs):
        calls.append(cmd)
        out = f"slicing 3.1.2 ({sha_full[:12]})\n" if cmd[1:] == ["--version"] else "[]"
        return FakeRun(stdout=out)

    monkeypatch.setattr(subprocess, "run", run)
    monkeypatch.setattr(sut_mod, "verified_binary_identity", lambda *_a: (sha_full[:12], sha_full))
    archived = PrismCli.from_archive("/repo", str(binary), sha_full, raw_digest(binary))
    verified = PrismCli.from_verified(
        "/repo", str(binary), sha_full, raw_digest(binary), env={"PATH": "/usr/bin"}
    )
    for sut in (archived, verified):
        sut._run(["functions"])
        assert "--no-cache" not in calls[-1]
        cache_index = calls[-1].index("--cache-dir")
        assert calls[-1][cache_index + 1] == str(
            cache_root / "nav-cache" / sut.sut_digest.removeprefix("sha256:")
        )
        assert sut.lock_bound  # pinned.py probe-error accounting keys on this, not on the cache


def test_archive_runs_from_authenticated_private_snapshot(tmp_path, monkeypatch):
    binary = tmp_path / "prism"
    binary.write_bytes(b"binary-a")
    binary.chmod(0o755)
    sha_full = "a" * 40
    launched = []

    def run(command, **_kwargs):
        launched.append(command)
        return FakeRun(
            stdout=(
                f"slicing 3.1.2 ({sha_full[:12]})\n"
                if command[1:] == ["--version"]
                else "[]"
            )
        )

    monkeypatch.setattr(subprocess, "run", run)
    sut = PrismCli.from_archive(
        "/repo", str(binary), sha_full, raw_digest(binary), env={"PATH": "/usr/bin"}
    )
    binary.write_bytes(b"binary-b")

    assert sut._run(["functions"]) == []
    assert Path(sut.bin) != binary
    assert Path(sut.bin).read_bytes() == b"binary-a"
    assert launched[-1][0] == sut.bin
    assert sut.sut_digest == raw_digest(Path(sut.bin))


def test_archive_version_uses_scrubbed_environment_and_bounded_launcher(
    tmp_path, monkeypatch
):
    binary = tmp_path / "prism"
    binary.write_bytes(b"archive")
    binary.chmod(0o755)
    sha_full = "a" * 40
    calls = []

    def run(command, **kwargs):
        calls.append((command, kwargs))
        return FakeRun(stdout=f"slicing 3.1.2 ({sha_full[:12]})\n")

    monkeypatch.setattr(subprocess, "run", run)
    PrismCli.from_archive(
        "/repo", str(binary), sha_full, raw_digest(binary), env={"PATH": "/clean"}
    )

    assert calls[-1][1]["env"] == {"PATH": "/clean"}
    assert calls[-1][1]["timeout"] == 120.0


def test_prism_corpus_budgets_fit_full_corpus_with_all_features():
    import tomllib
    config = tomllib.loads((Path(__file__).parents[1] / "corpora.toml").read_text())
    assert config["corpus"]["prism"]["query_timeout_s"] == 30.0
    assert config["corpus"]["prism"]["startup_timeout_s"] == 300.0
    assert config["corpus"]["prism"]["oracle_budget_s"] == 3600.0
    assert config["defaults"]["query_timeout_s"] == 10.0  # main's default for other corpora


def test_prism_corpus_population_is_only_cargo_target_sources():
    """Every Rust file in the prism corpus population belongs to a cargo target of the
    root package (or its vendored grammar crate); fixture crates and doc probes outside
    the workspace are excluded because rust-analyzer cannot answer for them."""
    import tomllib
    from tier_a.corpus import universe
    repo = Path(__file__).parents[2]
    cfg = tomllib.loads((repo / "eval/corpora.toml").read_text())["corpus"]["prism"]
    population = universe(str(repo), cfg["lang"], cfg["excludes"], tracked_only=True)
    allowed = ("src/", "tests/", "benches/", "examples/", "vendor/")
    stray = sorted(p for p in population if not (p == "build.rs" or p.startswith(allowed)))
    assert stray == []


def test_harness_modules_load_before_the_boundary_observes_reads():
    """make_oracle runs after the boundary starts observing opens; every harness module
    it needs must already be imported, or reading its source is an undeclared read."""
    import subprocess as sp
    import sys
    code = "import sys, tier_a.cli; print('tier_a.tsserver' in sys.modules)"
    out = sp.run([sys.executable, "-B", "-c", code], capture_output=True, text=True,
                 cwd=Path(__file__).parents[1], check=True).stdout.strip()
    assert out == "True"


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
            {"capture_output": True, "text": True, "timeout": 120.0},
        )
    ]


def test_run_passes_the_scrubbed_environment(monkeypatch):
    seen = {}
    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    cli.env = {"PATH": "/usr/bin"}
    monkeypatch.setattr(
        subprocess,
        "run",
        lambda _argv, **kwargs: seen.update(kwargs) or FakeRun(stdout='{"items":[]}\n'),
    )

    cli._run(["functions"])

    assert seen["env"] == {"PATH": "/usr/bin"}


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
    monkeypatch.setattr(sut_mod, "_resolve_executable", lambda command, _env: command)
    monkeypatch.setattr(
        sut_mod,
        "_snapshot_executable",
        lambda path, _digest=None: (None, path, "sha256:" + "a" * 64),
    )
    monkeypatch.setenv("PRISM_BIN", "/env/prism")

    assert PrismCli("/repo", sut_bin="/arg/prism").bin == "/arg/prism"
    assert PrismCli("/repo").bin == "/env/prism"
    assert PrismCli("/repo", env={"PATH": "/clean"}).bin == "/repo/target/release/prism"

    monkeypatch.delenv("PRISM_BIN")
    assert PrismCli("/repo").bin == "/repo/target/release/prism"


def test_check_freshness_accepts_sha_prefix_and_clean_repo(monkeypatch):
    calls = iter([
        FakeRun(stdout="slicing 3.1.2 (abcdef1234567)\n"),
        FakeRun(stdout="abcdef1234567890abcdef1234567890abcdef12\n"),
        FakeRun(stdout="abcdef1234567890abcdef1234567890abcdef12\n"),
        FakeRun(stdout=""),
    ])

    monkeypatch.setattr(subprocess, "run", lambda *_args, **_kwargs: next(calls))
    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    cli.repo = "/repo"
    cli.allow_stale = False
    assert cli._check_freshness() == ("abcdef1234567", False)


def test_allow_stale_records_unresolvable_binary_commit_without_raising(monkeypatch):
    monkeypatch.setattr(
        sut_mod,
        "_resolve_binary_commit",
        lambda *_args: (_ for _ in ()).throw(SutStale("binary_commit_unresolvable")),
    )
    calls = iter([FakeRun(stdout="f" * 40 + "\n"), FakeRun(stdout="")])
    monkeypatch.setattr(subprocess, "run", lambda *_a, **_k: next(calls))
    cli = PrismCli.__new__(PrismCli)
    cli.bin = "/tmp/prism"
    cli.repo = "/repo"
    cli.allow_stale = True
    cli._run = lambda _args: "slicing 3.1.2 (abcdef123456)\n"

    assert cli._check_freshness() == ("abcdef123456", False)
    assert cli.sha_full == "abcdef123456"
    assert cli.identity_note == "binary_commit_unresolvable"


def test_check_freshness_rejects_dirty_repo_even_if_binary_clean(monkeypatch):
    calls = iter([
        FakeRun(stdout="slicing 3.1.2 (abcdef123456)\n"),
        FakeRun(stdout="abcdef1234567890abcdef1234567890abcdef12\n"),
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


def test_callers_callees_thread_confidence_flag(monkeypatch):
    seen = []

    def fake_run(_self, args):
        seen.append(args)
        return {"items": []}

    monkeypatch.setattr(PrismCli, "_run", fake_run)
    cli = PrismCli.__new__(PrismCli)

    cli.callers("/repo", SEED, confidence="exact")
    cli.callees("/repo", SEED, confidence="exact")
    assert all("--confidence" in a and "exact" in a for a in seen), seen

    seen.clear()
    cli.callers("/repo", SEED)  # default "all" omits the flag (byte-for-byte)
    cli.callees("/repo", SEED)
    assert all("--confidence" not in a for a in seen), seen


def test_public_edge_methods_map_seed_miss_to_empty(monkeypatch):
    def fake_run(_self, _args):
        raise SutSeedMiss("no seed")

    monkeypatch.setattr(PrismCli, "_run", fake_run)
    cli = PrismCli.__new__(PrismCli)

    assert cli.callers("/repo", SEED) == []
    assert cli.callees("/repo", SEED) == []
    assert cli.callers_by_symbol("/repo", "missing") == []


def test_taint_reaches_and_module_deps_wire_flags_to_run(monkeypatch):
    calls = []

    def fake_run(self, args):
        calls.append(args)
        return {"reasoning": None} if args[0] == "taint-reaches" else {"items": []}

    monkeypatch.setattr(PrismCli, "_run", fake_run)
    cli = PrismCli.__new__(PrismCli)

    cli.taint_reaches("/repo", ["app.py:2"], ["app.py:4"])
    cli.taint_reaches("/repo", ["app.py:2"])  # frontier mode: no --sink
    cli.module_deps("/repo", "a.py")

    assert calls == [
        ["taint-reaches", "--repo", "/repo", "--source", "app.py:2", "--sink", "app.py:4"],
        ["taint-reaches", "--repo", "/repo", "--source", "app.py:2"],
        ["module-deps", "--repo", "/repo", "--file", "a.py"],
    ]
