import copy
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

from tests.helpers import (
    ARGS,
    _write_stub_cargo_metadata,
    admitted_fake_run,
    anchored_repo,
    base_ids,
    fake_run,
    git_head,
    quick_probes,
    stub_binary,
    FakeOracle,
)
from tier_a import cli, commands
from tier_a.closure import raw_digest
from tier_a.compare_report import classify_probe, compare_runs
from tier_a.sut import PrismCli, SutStale


def _commit(repo: Path, name: str, contents: str) -> str:
    (repo / name).write_text(contents)
    subprocess.run(["git", "add", name], cwd=repo, check=True)
    subprocess.run(
        ["git", "commit", "-m", name], cwd=repo, check=True, capture_output=True
    )
    return git_head(repo)


def _run_quick(
    ev: Path,
    tmp_path: Path,
    monkeypatch,
    sut_bin: str,
    *,
    run_id: str,
) -> dict:
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    monkeypatch.setattr(PrismCli, "inventory", lambda *_args: [])
    monkeypatch.setattr(PrismCli, "callers", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(PrismCli, "callees", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(cli, "make_oracle", lambda *_args, **_kwargs: FakeOracle())
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "tier-a",
            "--quick",
            "--allow-stale-sut",
            "--sut-bin",
            sut_bin,
            "--run-id",
            run_id,
            "--date",
            "2099-01-05",
        ],
    )
    assert cli.main() == 0
    return json.loads(
        (ev / f"runs/2099-01-05-prism-{run_id}.json").read_text()
    )


def test_real_adapter_records_the_binary_ancestor_not_checkout_head(
    tmp_path, monkeypatch
):
    _lock, ev, repo = anchored_repo(tmp_path)
    ancestor = git_head(repo)
    descendant = _commit(repo, "DESCENDANT", "later harness\n")
    assert descendant != ancestor
    binary = stub_binary(repo / "target/release/prism", ancestor[:12])
    _write_stub_cargo_metadata(binary)

    run = _run_quick(
        ev, tmp_path, monkeypatch, str(binary), run_id="ancestor-binary"
    )

    assert run["meta"]["sut_sha_full"] == ancestor
    assert run["meta"]["sut_sha_full"] != descendant
    assert run["meta"]["sut_digest"] == raw_digest(binary)


def test_bare_path_symlink_records_the_executable_target_digest(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path)
    binary_dir = tmp_path / "path-bin"
    binary_dir.mkdir()
    target = binary_dir / "prism-real"
    target.write_text(
        f"#!/bin/sh\nprintf '%s\\n' 'slicing 3.1.2 ({git_head(repo)[:12]})'\n"
        "# distinct PATH-selected bytes\n"
    )
    target.chmod(0o755)
    target.with_suffix(".d").write_text(
        f"{target}: {repo / 'src/a.rs'} {repo / 'Cargo.toml'}\n"
    )
    _write_stub_cargo_metadata(target)
    (binary_dir / "prism").symlink_to(target)
    assert raw_digest(target) != lock.prism["sut"]["digest"]
    monkeypatch.setenv("PATH", f"{binary_dir}{os.pathsep}{os.environ['PATH']}")

    run = _run_quick(ev, tmp_path, monkeypatch, "prism", run_id="path-symlink")

    assert run["meta"]["sut_digest"] == raw_digest(target)
    assert run["meta"]["sut_sha_full"] == git_head(repo)


def test_missing_executable_is_a_named_observation_refusal(tmp_path):
    _lock, _ev, repo = anchored_repo(tmp_path)
    with pytest.raises(
        SutStale, match="sut_observation_unavailable: executable_not_found"
    ):
        PrismCli(
            str(repo),
            sut_bin="does-not-exist",
            allow_stale=True,
            env={"PATH": str(tmp_path / "empty-path")},
        )


def test_absent_observed_identities_refuse_shared_record_schema(tmp_path, monkeypatch):
    lock, ev, repo = anchored_repo(tmp_path)
    monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    missing = {"sut_sha_full", "oracle", "oracle_init", "harness_sha"}

    def run_without_observed_identities(*_args, **_kwargs):
        run = admitted_fake_run()
        for key in missing:
            run["meta"].pop(key, None)
        run["_sut_bin"] = repo / "target/release/prism"
        return run

    monkeypatch.setattr(cli, "run_corpus", run_without_observed_identities)
    config = tomllib.loads((ev / "corpora.toml").read_text())
    args = copy.copy(ARGS)
    with pytest.raises(commands.BoundaryViolation) as caught:
        commands.run_quick_admitted(
            "prism", config["corpus"]["prism"], config["defaults"], args, ev
        )
    assert caught.value.reasons == [
        f"run_record_schema_invalid: meta.{key}" for key in sorted(missing)
    ]


def test_sut_error_transitions_use_closure_then_digest_then_unexplained():
    ids = base_ids()
    pid = "callees:src/C-name.rs:2"
    answered = fake_run(ids)
    error_probes = quick_probes()
    error_probes[pid] = {**error_probes[pid], "outcome": "sut_error"}
    errored = fake_run(ids, probes=error_probes)

    recovery = compare_runs(errored, answered, ([], ids["data"]["eval/adjudications.jsonl"]["digest"]))
    assert recovery["classes"][pid] == "unexplained"
    assert recovery["verdict"] == "REGRESSION(unexplained)"

    new_error = compare_runs(answered, errored, ([], ids["data"]["eval/adjudications.jsonl"]["digest"]))
    assert new_error["classes"][pid] == "unexplained"
    assert new_error["verdict"] == "REGRESSION(no_new_sut_errors; unexplained)"

    changed = {
        **ids,
        "closure_digest": "sha256:different-content",
        "sut_digest": "sha256:different-binary",
    }
    assert (
        classify_probe(pid, "ok", quick_probes()[pid], "sut_error", None, ids, changed)
        == "carried_changed/content_change"
    )
