import subprocess
import sys
from pathlib import Path

import pytest

from tests import test_bootstrap as boot
from tests.helpers import _write_stub_cargo_metadata
from tier_a import cli
from tier_a.closure import raw_digest, read_tsv, write_tsv
from tier_a.model import FunctionDef, Location
from tier_a.sut import PrismCli


GENERATED = "target/release/build/x-fixture/out/generated.rs"
OUTPUT = "target/release/build/x-fixture/output"
DATE = "2099-01-05"


def committed_anchor(tmp_path, monkeypatch, *, bind_output=False):
    original = boot.anchored_repo

    def with_generated(root, publish=False):
        lock, ev, repo = original(root, publish=publish)
        generated = repo / GENERATED
        generated.parent.mkdir(parents=True, exist_ok=True)
        generated.write_text("pub const VALUE: usize = 1;\n")
        output = repo / OUTPUT
        output.write_text("cargo:rerun-if-env-changed=HOME\n")
        for binary in (repo / "prism", repo / "target/release/prism"):
            dep_info = binary.with_suffix(".d")
            additions = [str(generated)]
            if bind_output:
                additions.append(str(output))
            dep_info.write_text(
                dep_info.read_text().rstrip() + " " + " ".join(additions) + "\n"
            )
            _write_stub_cargo_metadata(binary)
        return lock, ev, repo

    monkeypatch.setattr(boot, "anchored_repo", with_generated)
    published, ev, repo = boot._ordinary_bootstrap_end_to_end(tmp_path, monkeypatch)
    subprocess.run(["git", "add", "eval", "docs"], cwd=repo, check=True)
    subprocess.run(
        ["git", "commit", "-m", "publish test anchor"],
        cwd=repo,
        check=True,
        capture_output=True,
    )
    expected = FunctionDef("a", "function", None, Location("src/a.rs", 1, 1), 1)
    monkeypatch.setattr(PrismCli, "inventory", lambda *_args: [expected])
    monkeypatch.setattr(PrismCli, "callers", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(PrismCli, "callees", lambda *_args, **_kwargs: [])
    return published, ev, repo


def run_leg(monkeypatch, capsys, lock, ev, repo, leg, run_id):
    argv = ["tier-a", "--quick", "--run-id", run_id, "--date", DATE]
    if leg == "archive":
        argv += ["--sut-archive", lock.prism["sut"]["archive"]]
    else:
        argv += [
            "--sut-bin",
            str(repo / "prism"),
            "--allow-stale-sut",
        ]
    capsys.readouterr()
    monkeypatch.setattr(sys, "argv", argv)
    rc = cli.main()
    stderr = capsys.readouterr().err
    run_path = ev / f"runs/{DATE}-prism-{run_id}.json"
    return rc, stderr, run_path


@pytest.mark.parametrize("leg", ["archive", "candidate"])
@pytest.mark.parametrize("mutation", ["row_deletion", "digest_replacement"])
def test_committed_anchor_authenticates_complete_sut_input_manifest(
    tmp_path, monkeypatch, capsys, mutation, leg
):
    lock, ev, repo = committed_anchor(tmp_path, monkeypatch)
    manifest = repo / lock.prism["sut"]["inputs"]
    archived = Path(lock.prism["sut"]["archive"]).parent / "inputs" / GENERATED
    rows = read_tsv(manifest)
    assert GENERATED in {row[0] for row in rows}
    if mutation == "row_deletion":
        write_tsv([row for row in rows if row[0] != GENERATED], manifest)
        archived.unlink()
    else:
        archived.write_text("pub const VALUE: usize = 999;\n")
        write_tsv(
            [
                (row[0], raw_digest(archived)) if row[0] == GENERATED else row
                for row in rows
            ],
            manifest,
        )

    rc, stderr, run_path = run_leg(
        monkeypatch, capsys, lock, ev, repo, leg, f"{mutation}-{leg}"
    )

    assert rc == 3
    assert stderr == "FATAL boundary_violation: sut_input_manifest_mismatch\n"
    assert not run_path.exists()


def test_unreadable_archived_build_output_is_named_observation_refusal(
    tmp_path, monkeypatch, capsys
):
    lock, ev, repo = committed_anchor(tmp_path, monkeypatch, bind_output=True)
    output = Path(lock.prism["sut"]["archive"]).parent / "inputs" / OUTPUT
    assert output.is_file()
    output.chmod(0)
    try:
        rc, stderr, run_path = run_leg(
            monkeypatch, capsys, lock, ev, repo, "archive", "unreadable-output"
        )
    finally:
        output.chmod(0o644)

    assert rc == 3
    assert stderr == (
        f"FATAL boundary_violation: sut_observation_unavailable: {output}\n"
    )
    assert not run_path.exists()
