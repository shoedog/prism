"""Corpus file universe (§2.4) + oracle-inventory snapshots (§2.5/G3)."""
from __future__ import annotations

import dataclasses
import fnmatch
import hashlib
import json
import os
import subprocess
from pathlib import Path

from .model import FunctionDef, Location

EXTENSIONS = {"rust": [".rs"], "go": [".go"], "python": [".py"],
              "ts": [".ts", ".tsx"], "js": [".js", ".jsx"]}


def _tracked_files(root: str) -> set[str]:
    p = subprocess.run(["git", "-C", root, "ls-files", "-z"],
                       capture_output=True, check=True)
    return {
        path.decode("utf-8", errors="replace").replace(os.sep, "/")
        for path in p.stdout.split(b"\0")
        if path
    }


def universe(root: str, lang: str, excludes: list[str],
             tracked_only: bool = False) -> list[str]:
    tracked = _tracked_files(root) if tracked_only else None
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in (".git", "target", "node_modules", ".venv")]
        for fn in filenames:
            rel = os.path.relpath(os.path.join(dirpath, fn), root).replace(os.sep, "/")
            if tracked is not None and rel not in tracked:
                continue
            if not any(fn.endswith(e) for e in EXTENSIONS[lang]):
                continue
            if any(fnmatch.fnmatch(rel, g) for g in excludes):
                continue
            out.append(rel)
    return sorted(set(out))


def corpus_sha(root: str) -> str:
    return subprocess.run(["git", "-C", root, "rev-parse", "--short=12", "HEAD"],
                          capture_output=True, text=True, check=True).stdout.strip()


def corpus_dirty(root: str) -> bool:
    p = subprocess.run(["git", "-C", root, "status", "--porcelain", "-uno"],
                       capture_output=True, text=True, check=True)
    return bool(p.stdout.strip())


def untracked_sources(root: str, lang: str) -> list[str]:
    p = subprocess.run(["git", "-C", root, "status", "--porcelain=v1", "-z"],
                       capture_output=True, check=True)
    exts = EXTENSIONS[lang]
    out = []
    for entry in p.stdout.split(b"\0"):
        if not entry.startswith(b"?? "):
            continue
        rel = entry[3:].decode("utf-8", errors="surrogateescape")
        abs_path = os.path.join(root, rel)
        if os.path.isdir(abs_path):
            for dirpath, _dirnames, filenames in os.walk(abs_path):
                for fn in filenames:
                    path = os.path.join(dirpath, fn)
                    rel_file = os.path.relpath(path, root).replace(os.sep, "/")
                    if any(rel_file.endswith(ext) for ext in exts):
                        out.append(rel_file)
        elif any(rel.endswith(ext) for ext in exts):
            out.append(rel.replace(os.sep, "/"))
    return sorted(set(out))


def snapshot_path(snap_dir: str, corpus: str, sha: str) -> Path:
    return Path(snap_dir) / f"{corpus}-{sha}.json"


def save_snapshot(path: Path, inventory: list[FunctionDef]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps([dataclasses.asdict(f) for f in inventory],
                               indent=1, sort_keys=True))


def load_snapshot(path: Path) -> list[FunctionDef]:
    return [FunctionDef(name=r["name"], kind=r["kind"], container=r["container"],
                        location=Location(**r["location"]),
                        selection_line=r["selection_line"],
                        selection_char=r.get("selection_char", 0))
            for r in json.loads(path.read_text())]


def verify_source_manifest(cfg: dict) -> tuple[list[str], list[str]]:
    """Verify a Git-free public archive, naming each drifting component.

    The complete SHA256 manifest is itself pinned in corpora.toml. Generated
    target/node_modules/.venv directories are outside the source universe.
    """
    root = Path(cfg["path"])
    body = Path(cfg["source_manifest"]).read_bytes()
    if hashlib.sha256(body).hexdigest() != cfg["source_manifest_sha256"]:
        return [], ["source_manifest_sha256"]
    files, drift = [], []
    for line in body.decode().splitlines():
        digest, sep, rel = line.partition("  ")
        path = Path(rel)
        if (not sep or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest)
                or path.is_absolute() or ".." in path.parts or str(path) != rel
                or rel in files):
            raise ValueError(f"malformed source manifest entry: {line!r}")
        files.append(rel)
        p = root / rel
        if any((root / parent).is_symlink() for parent in [path, *path.parents]):
            drift.append(f"source_symlink:{rel}")
        elif not p.is_file():
            drift.append(f"source_missing:{rel}")
        elif hashlib.sha256(p.read_bytes()).hexdigest() != digest:
            drift.append(f"source_content:{rel}")
    known = set(files)
    for dirpath, dirnames, filenames in os.walk(root):
        if cfg.get("reject_node_modules") and "node_modules" in dirnames:
            drift.append("oracle_node_modules_present:" + (Path(dirpath) / "node_modules").relative_to(root).as_posix())
        dirnames[:] = [d for d in dirnames if d not in (".git", "target", "node_modules", ".venv")]
        for filename in filenames:
            rel = (Path(dirpath) / filename).relative_to(root).as_posix()
            if rel not in known:
                drift.append(f"source_extra:{rel}")
    return files, sorted(set(drift))
