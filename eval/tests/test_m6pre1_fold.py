"""Astra R2/2 W2-remainder (controller fold): archive-provenance stat failures are named refusals."""
from pathlib import Path

import pytest

from tests import test_bootstrap as boot
from tests import test_m6pre1_fix1 as fix1
from tier_a.closure import read_tsv

MISSING = "target/release/build/x-fixture/out/absent-only/missing.rs"


def committed_anchor_with_absent(tmp_path, monkeypatch):
    original = boot.anchored_repo

    def with_absent(root, publish=False):
        lock, ev, repo = original(root, publish=publish)
        for binary in (repo / "prism", repo / "target/release/prism"):
            dep_info = binary.with_suffix(".d")
            dep_info.write_text(
                dep_info.read_text().rstrip() + " " + str(repo / MISSING) + "\n"
            )
        return lock, ev, repo

    monkeypatch.setattr(boot, "anchored_repo", with_absent)
    return fix1.committed_anchor(tmp_path, monkeypatch, bind_output=True)


def _refuses_then_admits(monkeypatch, capsys, lock, ev, repo, locked_dir, named, run_id):
    locked_dir.chmod(0)
    try:
        rc, stderr, run_path = fix1.run_leg(monkeypatch, capsys, lock, ev, repo, "archive", run_id)
    finally:
        locked_dir.chmod(0o755)
    assert rc == 3, stderr
    assert stderr == f"FATAL boundary_violation: sut_observation_unavailable: {named}\n"
    assert not run_path.exists()
    rc, stderr, _ = fix1.run_leg(monkeypatch, capsys, lock, ev, repo, "archive", run_id + "-restored")
    assert rc == 0, stderr


def test_inaccessible_archived_output_parent_is_named(tmp_path, monkeypatch, capsys):
    lock, ev, repo = fix1.committed_anchor(tmp_path, monkeypatch, bind_output=True)
    output = Path(lock.prism["sut"]["archive"]).parent / "inputs" / fix1.OUTPUT
    _refuses_then_admits(monkeypatch, capsys, lock, ev, repo, output.parent, output, "output-parent")


def test_inaccessible_present_generated_parent_is_named(tmp_path, monkeypatch, capsys):
    lock, ev, repo = fix1.committed_anchor(tmp_path, monkeypatch)
    generated = Path(lock.prism["sut"]["archive"]).parent / "inputs" / fix1.GENERATED
    _refuses_then_admits(monkeypatch, capsys, lock, ev, repo, generated.parent, generated, "generated-parent")


@pytest.mark.parametrize("where", ["archive", "worktree"])
def test_inaccessible_absent_input_parent_is_named(tmp_path, monkeypatch, capsys, where):
    lock, ev, repo = committed_anchor_with_absent(tmp_path, monkeypatch)
    rows = {row[0]: row[1] for row in read_tsv(repo / lock.prism["sut"]["inputs"])}
    assert rows.get(MISSING) == "absent", rows.get(MISSING)
    archived = Path(lock.prism["sut"]["archive"]).parent / "inputs" / MISSING
    target = archived if where == "archive" else repo / MISSING
    target.parent.mkdir(parents=True, exist_ok=True)
    _refuses_then_admits(monkeypatch, capsys, lock, ev, repo, target.parent, target, f"absent-{where}")
