import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from tests.helpers import edit
from tier_a.boundary import load_declared_inputs
from tier_a.closure import (
    _POLICY_EXCLUSIONS,
    policy_digest,
    policy_manifest,
)
from tier_a.lock import load_lock


EVAL_DIR = Path(__file__).resolve().parents[1]
REPO = EVAL_DIR.parent


def _copy_policy_tree(tmp_path: Path) -> Path:
    root = tmp_path / "repo"
    shutil.copytree(EVAL_DIR / "tier_a", root / "eval/tier_a")
    shutil.copy2(EVAL_DIR / "corpora.toml", root / "eval/corpora.toml")
    (root / "src").mkdir()
    shutil.copy2(REPO / "src/repo_loader.rs", root / "src/repo_loader.rs")
    return root / "eval"


def test_policy_manifest_hashes_every_tracked_tier_a_file_by_derivation():
    tracked = subprocess.run(
        ["git", "ls-files", "-z", "--", "eval/tier_a"],
        cwd=REPO,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    expected = {
        entry.decode().removeprefix("eval/")
        for entry in tracked
        if entry and Path(entry.decode()).name not in _POLICY_EXCLUSIONS
    }

    assert set(policy_manifest(EVAL_DIR)["modules"]) == expected
    assert "tier_a/ts_syntax.cjs" in expected


def test_untracked_policy_input_refuses_but_pycache_is_ignored(tmp_path):
    eval_dir = _copy_policy_tree(tmp_path)
    repo = eval_dir.parent
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True)
    subprocess.run(["git", "add", "eval", "src/repo_loader.rs"], cwd=repo, check=True)
    (eval_dir / "tier_a/__pycache__").mkdir(exist_ok=True)
    (eval_dir / "tier_a/__pycache__/ignored.pyc").write_bytes(b"cache")
    policy_manifest(eval_dir)

    (eval_dir / "tier_a/measurement_helper.txt").write_text("untracked input\n")
    with pytest.raises(ValueError, match="untracked_policy_input"):
        policy_manifest(eval_dir)


def test_ts_syntax_and_native_predicate_mutations_move_policy_digest(tmp_path):
    eval_dir = _copy_policy_tree(tmp_path)
    before = policy_digest(policy_manifest(eval_dir))
    edit(eval_dir / "tier_a/ts_syntax.cjs", "shape='getter'", "shape='method'")
    cjs_changed = policy_digest(policy_manifest(eval_dir))
    assert cjs_changed != before

    loader = eval_dir.parent / "src/repo_loader.rs"
    edit(loader, '| "go.mod" |', '| "go.modd" |')
    assert policy_digest(policy_manifest(eval_dir)) not in {before, cjs_changed}


@pytest.mark.parametrize(
    "source, expected",
    [
        ("from tier_a.cli import main", {"cli"}),
        ("import tier_a.cli", {"cli"}),
        ('import importlib\nimportlib.import_module(".cli", "tier_a")', {"cli"}),
        ('from . import cli, policy', {"cli", "policy"}),
    ],
)
def test_import_derivation_covers_supported_python_import_forms(source, expected):
    from tier_a.closure import in_package_imports_from_source

    assert in_package_imports_from_source(source) == expected


def test_only_justified_plumbing_modules_are_outside_measurement_import_closure():
    from tier_a.closure import measurement_import_closure

    imported = measurement_import_closure(EVAL_DIR / "tier_a")
    assert not imported & set(_POLICY_EXCLUSIONS)
    assert set(_POLICY_EXCLUSIONS) == {
        "__init__.py",
        "cli.py",
        "commands.py",
        "interfaces.py",
        "manifest.py",
    }


def test_audit_hook_every_repo_read_or_execution_is_bound_or_justified():
    allowlist = {
        "eval/tier-a.lock.toml": "anchor authority record cannot digest itself",
    }
    active = True
    opens = set()
    processes = []

    def audit_hook(event, args):
        if not active:
            return
        if event == "open" and args and isinstance(args[0], (str, bytes, os.PathLike)):
            try:
                opens.add(
                    Path(os.fsdecode(args[0])).resolve().relative_to(REPO).as_posix()
                )
            except (OSError, ValueError):
                pass
        elif event == "subprocess.Popen" and len(args) >= 2:
            processes.append(tuple(os.fsdecode(item) for item in args[1]))

    sys.addaudithook(audit_hook)
    try:
        policy = policy_manifest(EVAL_DIR)
        lock = load_lock(EVAL_DIR / "tier-a.lock.toml")
        declared = load_declared_inputs(lock, EVAL_DIR)
    finally:
        active = False

    policy_paths = {
        f"eval/{path}" for path in policy["modules"]
    } | {
        "eval/corpora.toml",
        "src/repo_loader.rs",
    }
    data_paths = set(declared.digests)
    assert opens - policy_paths - data_paths == set(allowlist)
    assert allowlist == {
        "eval/tier-a.lock.toml": "anchor authority record cannot digest itself",
    }

    repo_argv = {
        argument
        for argv in processes
        for argument in argv
        if str(REPO) in argument
    }
    assert repo_argv == set()
    assert processes
    assert {argv[0] for argv in processes} == {"git"}
    assert {argv[1] for argv in processes} == {"rev-parse", "ls-files"}
