"""Independent pinned-corpus materialization for Tier-A."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from .lock import Lock, _bind_manifests, _checkout_closure_matches


class CorpusPinViolation(Exception):
    """The extracted corpus does not have the bytes promised by the lock."""


def tier_a_root() -> Path:
    configured = os.environ.get("TIER_A_ROOT")
    if configured:
        return Path(configured).expanduser()
    if sys.platform == "darwin":
        return Path.home() / "Library/Caches/prism/tier-a"
    return Path.home() / ".cache/prism/tier-a"


def ensure_mirror(root: Path, origin_url: str) -> Path:
    mirror = root / "repos/prism.git"
    mirror.parent.mkdir(parents=True, exist_ok=True)
    if mirror.exists():
        subprocess.run(
            ["git", "-C", str(mirror), "fetch", "--prune"],
            check=True,
            capture_output=True,
        )
    else:
        subprocess.run(
            ["git", "clone", "--mirror", origin_url, str(mirror)],
            check=True,
            capture_output=True,
        )
    return mirror


def _bind_source_manifests(lock: Lock, origin_url: str) -> None:
    source = Path(origin_url).expanduser()
    if not source.is_dir():
        raise CorpusPinViolation("manifest source is not a local repository")
    _bind_manifests(lock, source)


def _extract_archive(mirror: Path, sha: str, destination: Path) -> None:
    archive = subprocess.Popen(
        ["git", "-C", str(mirror), "archive", sha],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert archive.stdout is not None
    extracted = subprocess.run(
        ["tar", "-x", "-C", str(destination)],
        stdin=archive.stdout,
        capture_output=True,
    )
    archive.stdout.close()
    archive_stderr = archive.stderr.read() if archive.stderr is not None else b""
    archive_rc = archive.wait()
    if archive_rc or extracted.returncode:
        detail = archive_stderr or extracted.stderr
        raise CorpusPinViolation(detail.decode(errors="replace").strip())


def materialize(
    lock: Lock,
    root: Path,
    origin_url: str,
    name: str = "prism",
) -> Path:
    root = root.expanduser().resolve()
    mirror = ensure_mirror(root, origin_url)
    if not hasattr(lock, "_corpus_rows"):
        _bind_source_manifests(lock, origin_url)

    aliases = lock.prism.get("sha_aliases", [])
    if not aliases:
        raise CorpusPinViolation("lock has no corpus SHA alias")
    sha = aliases[0]
    resolved = subprocess.run(
        ["git", "-C", str(mirror), "rev-parse", f"{sha}^{{commit}}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if resolved != sha:
        raise CorpusPinViolation(f"alias {sha} resolved to {resolved}")

    suffix = lock.prism["closure"]["digest"].removeprefix("sha256:")[:12]
    checkout = root / "corpus" / name / suffix
    if checkout.is_dir() and _checkout_closure_matches(lock, checkout):
        return checkout
    if checkout.exists():
        if checkout.is_dir():
            shutil.rmtree(checkout)
        else:
            checkout.unlink()

    checkout.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=f".{suffix}-", dir=checkout.parent))
    try:
        _extract_archive(mirror, sha, temporary)
        if not _checkout_closure_matches(lock, temporary):
            raise CorpusPinViolation("extracted checkout closure mismatch")
        temporary.replace(checkout)
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    return checkout
