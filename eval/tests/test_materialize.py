import subprocess

import pytest

from tests.helpers import anchored_repo, git_repo
from tier_a.closure import corpus_manifest, manifest_digest
from tier_a.lock import Lock, resolve_corpus_identity
from tier_a.materialize import CorpusPinViolation, materialize


def test_materialize_uses_an_independent_mirror_and_registers_no_worktree(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    co = materialize(lock, root, origin_url=str(repo))
    assert (root / "repos/prism.git/HEAD").exists() and not (co / ".git").exists()
    assert (
        subprocess.run(
            ["git", "-C", str(repo), "worktree", "list", "--porcelain"],
            capture_output=True,
            text=True,
        ).stdout.count("worktree ")
        == 1
    )
    assert not any(p.name == "worktrees" for p in (repo / ".git").iterdir())


def test_materialize_refuses_dirty_or_closure_mismatch(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    co = materialize(lock, root, str(repo))
    (co / "src/a.rs").write_text("fn a() { }\n")
    co2 = materialize(lock, root, str(repo))
    assert (co2 / "src/a.rs").read_text() == "fn a() {}\n"
    bad = Lock(
        prism={
            **lock.prism,
            "closure": {**lock.prism["closure"], "digest": "sha256:0000"},
        }
    )
    with pytest.raises(CorpusPinViolation):
        materialize(bad, root, str(repo))


def test_extra_closure_source_is_dirty_untracked_and_reextracted(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    co = materialize(lock, root, str(repo))
    (co / "src/extra.rs").write_text("fn extra() {}\n")

    identity = resolve_corpus_identity(lock, co)
    assert identity.dirty is True
    assert identity.untracked == ["src/extra.rs"]

    clean = materialize(lock, root, str(repo))
    assert not (clean / "src/extra.rs").exists()
    assert resolve_corpus_identity(lock, clean).dirty is False


def test_missing_closure_path_is_dirty_and_reextracted(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    co = materialize(lock, root, str(repo))
    (co / "src/a.rs").unlink()

    identity = resolve_corpus_identity(lock, co)
    assert identity.dirty is True
    assert identity.untracked == []

    clean = materialize(lock, root, str(repo))
    assert (clean / "src/a.rs").read_text() == "fn a() {}\n"


def test_stray_git_directory_is_dirty_untracked_and_reextracted(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    co = materialize(lock, root, str(repo))
    (co / ".git").mkdir()

    identity = resolve_corpus_identity(lock, co)
    assert identity.dirty is True
    assert identity.untracked == [".git"]

    clean = materialize(lock, root, str(repo))
    assert not (clean / ".git").exists()


def test_feature_topology_change_is_not_benign_docs_only_is(tmp_path):
    repo = git_repo(
        tmp_path,
        {
            "src/a.rs": "fn a() {}\n",
            "Cargo.toml": "[package]\nname='x'\nversion='0.1.0'\n[features]\n",
            "README.md": "x\n",
        },
    )
    files = ["Cargo.toml", "src/a.rs"]
    d1 = manifest_digest(corpus_manifest(repo, files))
    (repo / "README.md").write_text("y\n")
    assert manifest_digest(corpus_manifest(repo, files)) == d1
    (repo / "Cargo.toml").write_text(
        "[package]\nname='x'\nversion='0.1.0'\n[features]\nmcp = []\n"
    )
    assert manifest_digest(corpus_manifest(repo, files)) != d1
