import subprocess

import pytest

from tests.helpers import anchored_repo, git_repo
from tier_a.closure import corpus_manifest, manifest_digest, raw_digest
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


def _rust_checkout(tmp_path):
    checkout = tmp_path / "checkout"
    source = checkout / "src/a.rs"
    source.parent.mkdir(parents=True)
    source.write_text("fn a() {}\n")
    cargo = checkout / "Cargo.toml"
    cargo.write_text("[package]\nname='x'\nversion='0.1.0'\n")
    rows = [("Cargo.toml", raw_digest(cargo)), ("src/a.rs", raw_digest(source))]
    lock = Lock(
        prism={
            "sha_aliases": ["a" * 40],
            "closure": {"digest": manifest_digest(rows)},
        }
    )
    lock._corpus_rows = rows
    lock._policy_data = {
        "corpora": {
            "corpus": {
                "prism": {
                    "lang": "rust",
                    "excludes": ["tests/fixtures/*"],
                }
            }
        }
    }
    return lock, checkout


def test_other_language_source_is_outside_rust_checkout_closure(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    (checkout / "extra.py").write_text("def extra(): pass\n")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is False
    assert identity.untracked == []


def test_other_language_target_does_not_hide_undeclared_rust_symlink(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    (checkout / "extra.py").write_text("def extra(): pass\n")
    (checkout / "src/extra.rs").symlink_to("../extra.py")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is True
    assert identity.untracked == ["extra.py"]


def test_undeclared_rust_source_is_inside_rust_checkout_closure(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    (checkout / "src/extra.rs").write_text("fn extra() {}\n")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is True
    assert identity.untracked == ["src/extra.rs"]


def test_declared_source_digest_drift_is_dirty(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    (checkout / "src/a.rs").write_text("fn changed() {}\n")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is True
    assert identity.untracked == []


def test_excluded_same_language_source_is_outside_checkout_closure(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    excluded = checkout / "tests/fixtures/extra.rs"
    excluded.parent.mkdir(parents=True)
    excluded.write_text("fn excluded() {}\n")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is False
    assert identity.untracked == []


def test_excluded_target_does_not_hide_undeclared_rust_symlink(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    excluded = checkout / "tests/fixtures/extra.rs"
    excluded.parent.mkdir(parents=True)
    excluded.write_text("fn excluded() {}\n")
    (checkout / "src/extra.rs").symlink_to("../tests/fixtures/extra.rs")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is True
    assert identity.untracked == ["tests/fixtures/extra.rs"]


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
