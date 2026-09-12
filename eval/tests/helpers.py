"""Shared fixtures for the Tier-A quick re-anchor tests."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

from tier_a.adjudication import Adjudication, validate
from tier_a.closure import (
    canonical_path,
    corpus_manifest,
    manifest_digest,
    policy_digest,
    policy_manifest,
    population_manifest,
    raw_digest,
    sut_inputs_manifest,
    write_tsv,
)
from tier_a.corpus import save_snapshot, snapshot_path
from tier_a.lock import Lock, lock_identities, save_lock
from tier_a.lsp_client import LspClient
from tier_a.model import FunctionDef, Location
from tier_a.strata import STRATA


__all__ = [
    "git_repo",
    "git_head",
    "copy_fixture",
    "copy_eval_policy_files",
    "edit",
    "base_ids",
    "fake_run",
    "quick_probes",
    "ADJ_RECORDS",
    "ADJ",
    "ADJ_DIGEST",
    "DECLARED_ADJ",
    "admitted_fake_run",
    "FakeSut",
    "FakeOracle",
    "EchoCapture",
    "stub_binary",
    "anchored_repo",
    "DEFAULTS",
    "ARGS",
]


EVAL_DIR = Path(__file__).parents[1]
FIXTURES_DIR = Path(__file__).parent / "fixtures"


def git_repo(root: Path, files: dict[str, str]) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(
        ["git", "config", "user.email", "t@example.com"], cwd=root, check=True
    )
    subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
    for name, contents in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents)
    subprocess.run(["git", "add", "."], cwd=root, check=True)
    subprocess.run(
        ["git", "commit", "-m", "init"], cwd=root, check=True, capture_output=True
    )
    return root


def git_head(repo: Path) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def copy_fixture(name: str, tmp: Path) -> Path:
    source = FIXTURES_DIR / name
    destination = tmp / source.name
    if source.is_dir():
        shutil.copytree(source, destination)
    else:
        shutil.copy2(source, destination)
    return destination


def copy_eval_policy_files(tmp: Path) -> Path:
    destination = tmp / "eval"
    tier_a = destination / "tier_a"
    tier_a.mkdir(parents=True)
    shutil.copy2(EVAL_DIR / "corpora.toml", destination / "corpora.toml")
    for name in (
        "corpus.py",
        "strata.py",
        "accounting.py",
        "spotcheck.py",
        "pinned.py",
        "cli.py",
    ):
        shutil.copy2(EVAL_DIR / "tier_a" / name, tier_a / name)
    return destination


def edit(path: Path, old: str, new: str) -> None:
    contents = path.read_text()
    assert contents.count(old) == 1
    path.write_text(contents.replace(old, new))


def _raw_digest(contents: str) -> str:
    return "sha256:" + hashlib.sha256(contents.encode()).hexdigest()


ADJ_RECORDS = [
    {
        "corpus": "prism",
        "measurement": "callers",
        "direction": "prism_only",
        "seed_def": "src/U-free.rs:1",
        "site": "src/adjudicated.rs:10",
        "verdict": "oracle_miss",
        "reason": "fixture",
        "adjudicated_by": "test",
        "date": "2026-01-01",
    },
    {
        "corpus": "prism",
        "measurement": "callees",
        "direction": "prism_only",
        "seed_def": "src/U-free.rs:2",
        "site": "src/adjudicated.rs:20",
        "verdict": "prism_fp",
        "reason": "fixture",
        "adjudicated_by": "test",
        "date": "2026-01-01",
    },
]
ADJ = [validate(Adjudication(**record)) for record in ADJ_RECORDS]
ADJ_DIGEST = _raw_digest("".join(json.dumps(record) + "\n" for record in ADJ_RECORDS))
DECLARED_ADJ = (ADJ, ADJ_DIGEST)


def base_ids() -> dict:
    return {
        "corpus_sha_full": "a" * 40,
        "sha_aliases": ["a" * 40],
        "closure_digest": _raw_digest("closure"),
        "sut_sha_full": "a" * 40,
        "sut_digest": _raw_digest("sut"),
        "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
        "oracle_init": {"cargo": {"features": "all"}},
        "policy_digest": _raw_digest("policy"),
        "population_digest": _raw_digest("population"),
        "snapshot_digest": _raw_digest("snapshot"),
        "sample_digest": _raw_digest("sample"),
        "runtime": {"python": "3.12.test", "uv_lock": _raw_digest("uv-lock")},
        "data": {"eval/adjudications.jsonl": {"digest": ADJ_DIGEST}},
        "env_allow": ["PATH", "HOME"],
        "env_values": {"PATH": "/usr/bin", "HOME": "/test/home"},
        "harness_sha": "b" * 40,
        "seed": 42,
    }


def quick_probes(per: int = 3) -> dict:
    return {
        f"{direction}:src/{stratum}.rs:{i}": {
            "outcome": "ok",
            "direction": direction,
            "stratum": stratum,
            "seed_def": f"src/{stratum}.rs:{i}",
            "prism_sites": [["src/a.rs", 3, 3]],
            "oracle_sites": [["src/a.rs", 3, 3]],
            "prism_functions": [],
            "oracle_functions": [],
        }
        for direction in ("callers", "callees")
        for stratum in STRATA
        for i in range(1, per + 1)
    }


def fake_run(
    ids: dict,
    probes: dict | None = None,
    status: dict | None = None,
    pinned: list | None = None,
) -> dict:
    supplied = quick_probes() if probes is None else probes
    normalized = {}
    for pid, fields in supplied.items():
        if pid.startswith("_"):
            continue
        defaults = {
            "outcome": "ok",
            "direction": pid.split(":")[0],
            "stratum": "U-free",
            "seed_def": pid.split(":", 1)[1],
            "prism_sites": [],
            "oracle_sites": [],
            "prism_functions": [],
            "oracle_functions": [],
        }
        normalized[pid] = defaults | fields
    normalized["_corpus"] = "prism"
    normalized["_strata"] = {
        stratum: {"eligible_symbols": 3, "eligible_probes": 6, "target": 3}
        for stratum in STRATA
    }
    return {
        "meta": {**ids, "admitted": True, "baseline_invalid": False},
        "probes": normalized,
        "probe_status": status
        or {pid: probe["outcome"] for pid, probe in normalized.items() if not pid.startswith("_")},
        "site_fingerprints": {},
        "pinned": pinned or [],
        "matrix": [],
    }


def admitted_fake_run() -> dict:
    run = fake_run(base_ids())
    run["meta"].update(
        corpus="prism",
        corpus_sha="a" * 12,
        prism_sha="a" * 12,
        date="2026-01-01",
        oracle_error_rate=0.0,
        sut_error_rate=0.0,
        oracle_not_quiescent=False,
        wall_s={},
    )
    return run


class FakeSut:
    def __init__(self, prism_repo=None, sut_bin=None, allow_stale=False, *, env=None):
        self.sha = "a" * 12
        self.sha_full = "a" * 40
        self.dirty = False
        self.no_cache = False
        self.env = env
        self.bin = "prism"

    def inventory(self, root):
        return []

    def callers(self, root, fd):
        return []

    def callees(self, root, fd):
        return []


DEFAULTS = tomllib.loads((EVAL_DIR / "corpora.toml").read_text())["defaults"]
ARGS = argparse.Namespace(
    corpus="prism",
    quick=True,
    matrix_only=False,
    report_only=None,
    sut_bin=None,
    allow_stale_sut=True,
    allow_drift=False,
    oracle=None,
    date="2026-01-01",
    live=False,
    run_id=None,
    sut_archive=None,
)


class FakeOracle:
    not_quiescent = False

    def start(self):
        pass

    def stop(self):
        pass

    def version(self):
        return "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)"

    def capability_probe(self):
        return True

    def document_symbols(self, f):
        return []

    def callers(self, fd):
        return []

    def callees(self, fd):
        return []


class EchoCapture:
    def __init__(self, init_options=None):
        command = [sys.executable, str(Path(__file__).parent / "echo_server.py")]
        options = {} if init_options is None else {"init_options": init_options}
        self.client = LspClient(
            command,
            cwd=".",
            default_timeout=5.0,
            **options,
        )


def stub_binary(path: Path, sha12: str) -> Path:
    path.write_text(f"#!/bin/sh\nprintf '%s\\n' 'slicing 3.1.2 ({sha12})'\n")
    path.chmod(0o755)
    return path


def _command_version(*command: str) -> str:
    return subprocess.run(
        command,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def anchored_repo(tmp: Path, publish: bool = True) -> tuple[Lock, Path, Path]:
    repo = git_repo(
        tmp / "repo",
        {
            "src/a.rs": "fn a() {}\n",
            "Cargo.toml": "[package]\nname='x'\nversion='0.1.0'\n",
        },
    )
    bootstrap_sha = git_head(repo)
    ev = copy_eval_policy_files(repo)
    edit(ev / "corpora.toml", 'pinned_sha = "20c8490591a3"', f'pinned_sha = "{bootstrap_sha[:12]}"')
    shutil.copy2(EVAL_DIR / ".python-version", ev / ".python-version")
    shutil.copy2(EVAL_DIR / "uv.lock", ev / "uv.lock")
    adjudication_bytes = "".join(
        json.dumps(record) + "\n" for record in ADJ_RECORDS
    ).encode()
    (ev / "adjudications.jsonl").write_bytes(adjudication_bytes)

    # Bootstrap requires corpora.toml to be tracked so finalize_policy can be
    # committed independently of the already-built SUT.
    subprocess.run(
        ["git", "add", "eval/corpora.toml"],
        cwd=repo,
        check=True,
        capture_output=True,
    )
    subprocess.run(
        ["git", "commit", "-m", "add eval policy"],
        cwd=repo,
        check=True,
        capture_output=True,
    )
    head = git_head(repo)

    snapshot = snapshot_path(ev / "snapshots", "prism", bootstrap_sha[:12])
    inventory = [
        FunctionDef("a", "function", None, Location("src/a.rs", 1, 1), 1)
    ]
    save_snapshot(snapshot, inventory)
    lock_snapshot = snapshot_path(ev / "snapshots", "prism", head[:12])
    save_snapshot(lock_snapshot, inventory)

    binary = repo / "target/release/prism"
    binary.parent.mkdir(parents=True)
    stub_binary(binary, head[:12])
    binary.with_suffix(".d").write_text(
        f"{binary}: {repo / 'src/a.rs'} {repo / 'Cargo.toml'}\n"
    )
    bootstrap_binary = stub_binary(repo / "prism", bootstrap_sha[:12])
    bootstrap_binary.with_suffix(".d").write_text(
        f"{bootstrap_binary}: {repo / 'src/a.rs'} {repo / 'Cargo.toml'}\n"
    )
    build_output = repo / "target/release/build/prism-fixture/output"
    build_output.parent.mkdir(parents=True)
    build_output.write_text("cargo:rerun-if-changed=src/a.rs\n")

    suffix = head[:12]
    closure_rows = corpus_manifest(repo, ["Cargo.toml", "src/a.rs"])
    sut_rows = sut_inputs_manifest(repo, binary)
    policy = policy_manifest(ev)
    population = population_manifest(inventory, policy, set(), "rust")
    closure_path = ev / "closure" / f"prism-corpus-{suffix}.tsv"
    sut_inputs_path = ev / "closure" / f"sut-inputs-{suffix}.tsv"
    policy_path = ev / "closure" / f"policy-{suffix}.json"
    population_path = ev / "closure" / f"prism-population-{suffix}.json"

    env_allow = [
        "PATH",
        "HOME",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "TMPDIR",
        "LANG",
        "TIER_A_ROOT",
    ]
    sample_identity = {
        "schema": 1,
        "seed": policy["corpora"]["defaults"]["seed"],
        "quick": 3,
        "full": policy["corpora"]["defaults"]["per_stratum"],
    }
    sample_digests = {
        mode: policy_digest(
            {
                "schema": sample_identity["schema"],
                "seed": sample_identity["seed"],
                "per_stratum": sample_identity[mode],
            }
        )
        for mode in ("quick", "full")
    }
    sut_digest = raw_digest(binary)
    lock = Lock(
        prism={
            "closure": {
                "digest": manifest_digest(closure_rows),
                "manifest": canonical_path(repo, closure_path),
            },
            "sha_aliases": [head],
            "policy": {
                "digest": policy_digest(policy),
                "manifest": canonical_path(repo, policy_path),
            },
            "population": {
                "digest": policy_digest(population),
                "manifest": canonical_path(repo, population_path),
            },
            "snapshot": {
                "path": canonical_path(repo, lock_snapshot),
                "digest": raw_digest(lock_snapshot),
            },
            "sample": {
                "schema": 1,
                "seed": sample_identity["seed"],
                "per_stratum": {
                    "quick": sample_identity["quick"],
                    "full": sample_identity["full"],
                },
                "digest": sample_digests,
            },
            "oracle": {
                "version": _command_version("rust-analyzer", "--version"),
                "init": {"cargo": {"features": "all"}},
            },
            "sut": {
                "sha": head,
                "digest": sut_digest,
                "inputs": canonical_path(repo, sut_inputs_path),
                "archive": str(
                    tmp
                    / "root/sut"
                    / sut_digest.removeprefix("sha256:")
                    / "prism"
                ),
            },
            "runtime": {
                "python": sys.version.split()[0],
                "python_version_file": raw_digest(ev / ".python-version"),
                "uv_lock": raw_digest(ev / "uv.lock"),
                "rustc": _command_version("rustc", "--version"),
                "cargo": _command_version("cargo", "--version"),
                "rust_toolchain": "absent",
                "cargo_config": "absent",
            },
            "data": {
                "adjudications": {
                    "path": "eval/adjudications.jsonl",
                    "digest": raw_digest(ev / "adjudications.jsonl"),
                    "records": len(ADJ_RECORDS),
                },
                "fixtures": manifest_digest([]),
                "pinned_probes": raw_digest(ev / "tier_a/pinned.py"),
            },
            "env": {
                "allow": env_allow,
                "values": {
                    name: os.environ[name] for name in env_allow if name in os.environ
                },
                "forbid_read_outside_allow": True,
            },
            "harness_sha": head,
        }
    )

    report_path = repo / "docs/eval/tier-a/2026-01-01-prism.json"
    report_path.parent.mkdir(parents=True)
    report_path.write_text(
        json.dumps(fake_run(lock_identities(lock)), indent=1, sort_keys=True) + "\n"
    )
    lock.prism["baseline_report"] = {
        "path": canonical_path(repo, report_path),
        "digest": raw_digest(report_path),
    }
    lock.prism["blessed"] = {
        "at": "2026-01-01",
        "by": "bootstrap",
        "adjudication": "fixture",
        "policy_change": "none",
    }

    if publish:
        write_tsv(closure_rows, closure_path)
        write_tsv(sut_rows, sut_inputs_path)
        policy_path.write_text(json.dumps(policy, indent=2, sort_keys=True) + "\n")
        population_path.write_text(
            json.dumps(population, indent=2, sort_keys=True) + "\n"
        )
        save_lock(lock, ev / "tier-a.lock.toml")
    return lock, ev, repo
