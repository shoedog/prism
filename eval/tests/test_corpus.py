import subprocess
from pathlib import Path

from tier_a.corpus import (
    load_snapshot,
    save_snapshot,
    snapshot_path,
    universe,
    untracked_sources,
)
from tier_a.model import FunctionDef, Location


def test_universe_filters_extensions_and_excludes(tmp_path):
    (tmp_path / "src").mkdir()
    (tmp_path / "src/a.rs").write_text("")
    (tmp_path / "src/b.py").write_text("")
    (tmp_path / "vendor").mkdir()
    (tmp_path / "vendor/c.rs").write_text("")
    files = universe(str(tmp_path), "rust", excludes=["vendor/*"])
    assert files == ["src/a.rs"]


def test_universe_tracked_only_and_untracked_sources(tmp_path):
    subprocess.run(["git", "init"], cwd=tmp_path, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "t@example.com"], cwd=tmp_path,
                   check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=tmp_path, check=True)
    (tmp_path / "src").mkdir()
    (tmp_path / "src/tracked.rs").write_text("fn tracked() {}\n")
    subprocess.run(["git", "add", "src/tracked.rs"], cwd=tmp_path, check=True)
    subprocess.run(["git", "commit", "-m", "init"], cwd=tmp_path, check=True,
                   capture_output=True)
    (tmp_path / "src/untracked.rs").write_text("fn untracked() {}\n")
    (tmp_path / "src/my file.rs").write_text("fn spaced_path() {}\n")
    (tmp_path / "newdir").mkdir()
    (tmp_path / "newdir/untracked.rs").write_text("fn new_untracked() {}\n")

    assert universe(str(tmp_path), "rust", excludes=[]) == [
        "newdir/untracked.rs",
        "src/my file.rs",
        "src/tracked.rs",
        "src/untracked.rs",
    ]
    assert universe(str(tmp_path), "rust", excludes=[], tracked_only=True) == [
        "src/tracked.rs",
    ]
    assert untracked_sources(str(tmp_path), "rust") == [
        "newdir/untracked.rs",
        "src/my file.rs",
        "src/untracked.rs",
    ]


def test_snapshot_roundtrip(tmp_path):
    inv = [FunctionDef("f", "function", None, Location("src/a.rs", 1, 3), 1)]
    p = snapshot_path(str(tmp_path), "prism", "abc123def456")
    save_snapshot(p, inv)
    assert load_snapshot(p) == inv
