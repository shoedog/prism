"""Corpus file universe (§2.4) + oracle-inventory snapshots (§2.5/G3)."""
from __future__ import annotations

import dataclasses
import fnmatch
import json
import os
import subprocess
from pathlib import Path

from .model import FunctionDef, Location

EXTENSIONS = {"rust": [".rs"], "go": [".go"], "python": [".py"]}


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
        dirnames[:] = [d for d in dirnames if d != ".git"]
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
