"""Tier-A command dispatch and run preflights."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from .lock import (
    CorpusIdentity,
    Lock,
    corpus_check,
    integrity_check,
    load_lock,
    lock_identities,
    resolve_corpus_identity,
)
from .materialize import materialize, tier_a_root


EVAL_DIR = Path(__file__).resolve().parents[1]


class BoundaryViolation(Exception):
    def __init__(self, reasons: list[str]):
        self.reasons = reasons
        super().__init__(", ".join(reasons))


def _load_optional_lock(eval_dir: Path) -> Lock | None:
    path = eval_dir / "tier-a.lock.toml"
    if not path.exists():
        return None
    try:
        return load_lock(path)
    except Exception as exc:
        raise BoundaryViolation([f"lock_invalid: {exc}"]) from exc


def _version(*command: str) -> str:
    return subprocess.run(
        command,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def _harness_sha(eval_dir: Path) -> str:
    return _version("git", "-C", str(eval_dir.parent), "rev-parse", "HEAD")


def _toolchain() -> dict:
    return {
        "rustc": _version("rustc", "--version"),
        "cargo": _version("cargo", "--version"),
        "python": sys.version.split()[0],
    }


def dispatch(argv: list[str]) -> int | None:
    if not argv or argv[0] not in {"lock-check", "bootstrap", "compare"}:
        return None
    if argv[0] in {"bootstrap", "compare"}:
        print(f"FATAL command_not_implemented: {argv[0]}", file=sys.stderr)
        return 3

    integrity = "--integrity" in argv[1:]
    corpus = None
    if "--corpus" in argv[1:]:
        index = argv.index("--corpus")
        if index + 1 >= len(argv):
            print("FATAL boundary_violation: missing corpus name", file=sys.stderr)
            return 3
        corpus = argv[index + 1]
    if not integrity and corpus is None:
        print(
            "FATAL boundary_violation: lock-check requires --integrity or --corpus",
            file=sys.stderr,
        )
        return 3
    try:
        if integrity:
            preflight_matrix(EVAL_DIR)
        if corpus is not None:
            if corpus != "prism":
                raise BoundaryViolation([f"unknown_corpus: {corpus}"])
            preflight_quick(EVAL_DIR, live=False)
    except BoundaryViolation as exc:
        print(f"FATAL boundary_violation: {exc}", file=sys.stderr)
        return 3
    return 0


def preflight_matrix(eval_dir: Path) -> None:
    lock = _load_optional_lock(eval_dir)
    try:
        reasons = integrity_check(lock, eval_dir)
    except Exception as exc:
        raise BoundaryViolation([f"lock_invalid: {exc}"]) from exc
    if reasons:
        raise BoundaryViolation(reasons)


def preflight_quick(
    eval_dir: Path,
    live: bool,
) -> tuple[Lock | None, Path, dict, CorpusIdentity | None]:
    lock = _load_optional_lock(eval_dir)
    try:
        reasons = integrity_check(lock, eval_dir)
    except Exception as exc:
        raise BoundaryViolation([f"lock_invalid: {exc}"]) from exc
    if reasons:
        raise BoundaryViolation(reasons)
    if lock is not None:
        locked_harness_sha = lock.prism.get("harness_sha")
        computed_harness_sha = _harness_sha(eval_dir)
        if locked_harness_sha != computed_harness_sha:
            raise BoundaryViolation(
                [
                    "harness_sha_drift "
                    f"{locked_harness_sha} != {computed_harness_sha}"
                ]
            )
    if live:
        return (
            lock,
            eval_dir.parent,
            {
                "corpus_mode": "live",
                "baseline_invalid": True,
                "invalid_reasons": ["live_corpus"],
            },
            None,
        )
    if lock is None:
        return None, eval_dir.parent, {}, None

    try:
        checkout = materialize(lock, tier_a_root(), str(eval_dir.parent))
        reasons = corpus_check(
            lock,
            checkout,
            _version("rust-analyzer", "--version"),
            _toolchain(),
        )
        if reasons:
            raise BoundaryViolation(reasons)
        identity = resolve_corpus_identity(lock, checkout)
    except BoundaryViolation:
        raise
    except Exception as exc:
        raise BoundaryViolation([f"corpus_mismatch: {exc}"]) from exc
    identities = lock_identities(lock)
    identities.pop("harness_sha", None)
    return (
        lock,
        checkout,
        {
            **identities,
            "corpus_sha": identity.sha,
            "corpus_dirty": identity.dirty,
            "untracked_sources": identity.untracked,
            "prism_sha": lock.prism["sut"]["sha"][:12],
            "corpus_mode": "pinned",
        },
        identity,
    )
