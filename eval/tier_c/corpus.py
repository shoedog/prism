"""Open-issue registry (spec §4). Enforces the Goldilocks rubric at load time so a
bad corpus can't silently weaken the study. Selection is frozen before any run."""
from __future__ import annotations
import tomllib
from pathlib import Path
from .model import Issue

_LANGS = {"rust", "go", "python", "js", "ts"}

class CorpusError(Exception): ...

def load_issues(path: str | Path) -> list[Issue]:
    raw = tomllib.loads(Path(path).read_text())
    out: list[Issue] = []
    for d in raw.get("issue", []):
        key = d.get("key", "<no key>")
        for req in ("key", "language", "repo", "sha", "url", "text", "scoped_slice"):
            if not d.get(req):
                raise CorpusError(f"{key}: missing required field {req!r}")
        if d["language"] not in _LANGS:
            raise CorpusError(f"{key}: language {d['language']!r} not in {_LANGS}")
        if int(d.get("files_touched_hint", 0)) < 2:
            raise CorpusError(f"{key}: must be multi-file (files_touched_hint >= 2), "
                              "not a one-liner (spec §4 Goldilocks)")
        out.append(Issue(key=d["key"], language=d["language"], repo=d["repo"],
                         sha=d["sha"], url=d["url"], text=d["text"],
                         scoped_slice=d["scoped_slice"]))
    if not out:
        raise CorpusError("no [[issue]] entries found")
    return out
