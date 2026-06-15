#!/usr/bin/env python3
"""Backfill `site_fingerprint` onto existing adjudication records (one-time, idempotent).

Gives line-keyed verdicts a drift-stable fingerprint so they re-anchor across resolution
churn (e.g. Phase-IP) instead of going stale + re-pending. For each record it reads the
call-site source at its corpus's pinned SHA (prism via `git show <sha>:<file>`; bench
repos at their pinned SHA) and computes `adjudication.fingerprint(site line +/- 1)` — the
same window the harness stamps on live diff-sites, so the two match across line drift.

Records whose file/line no longer exists at the pinned SHA are left `site_fingerprint=null`
(they stay line-keyed; already stale). Re-runnable: recomputes + overwrites.

Run from eval/:  uv run python tools/backfill_fingerprints.py [--dry-run]
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

EVAL = Path(__file__).resolve().parents[1]
REPO = EVAL.parent
STORE = EVAL / "adjudications.jsonl"
sys.path.insert(0, str(EVAL))
from tier_a.adjudication import fingerprint  # noqa: E402


def corpora() -> dict[str, tuple[str, str]]:
    cfg = tomllib.loads((EVAL / "corpora.toml").read_text())
    out = {}
    for name, c in cfg["corpus"].items():
        p = os.path.expanduser(c["path"])
        if not os.path.isabs(p):
            p = os.path.join(REPO, p)
        out[name] = (os.path.abspath(p), c["pinned_sha"])
    return out


_cache: dict[tuple, list | None] = {}


def source_at(repo: str, sha: str, relpath: str) -> list[str] | None:
    key = (repo, sha, relpath)
    if key not in _cache:
        try:
            out = subprocess.run(
                ["git", "-C", repo, "show", f"{sha}:{relpath}"],
                capture_output=True, text=True, check=True,
            )
            _cache[key] = out.stdout.splitlines()
        except subprocess.CalledProcessError:
            _cache[key] = None
    return _cache[key]


def window(lines: list[str], line: int) -> list[str]:
    lo = max(1, line - 1)
    hi = min(len(lines), line + 1)
    return lines[lo - 1 : hi]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    corp = corpora()
    records = [json.loads(line) for line in STORE.read_text().splitlines() if line.strip()]
    done = skipped = 0
    by_corpus: dict[str, int] = {}
    for r in records:
        root_sha = corp.get(r["corpus"])
        if root_sha is None:
            skipped += 1
            continue
        root, sha = root_sha
        file, line_s = r["site"].rsplit(":", 1)
        line = int(line_s)
        src = source_at(root, sha, file)
        if src is None or not (1 <= line <= len(src)):
            skipped += 1
            continue
        r["site_fingerprint"] = fingerprint(window(src, line))
        done += 1
        by_corpus[r["corpus"]] = by_corpus.get(r["corpus"], 0) + 1
    print(f"backfilled={done} skipped(missing file/line)={skipped} total={len(records)}")
    print(f"  by corpus: {dict(sorted(by_corpus.items()))}")
    if args.dry_run:
        print("(dry-run — store not written)")
    else:
        STORE.write_text("".join(json.dumps(r) + "\n" for r in records))
        print(f"wrote {STORE}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
