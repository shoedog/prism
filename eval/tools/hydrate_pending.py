#!/usr/bin/env python3
"""Hydrate Tier-A pending diffs into adjudicator-input JSON.

WHY THIS EXISTS
---------------
A Tier-A run records each unresolved prism/oracle disagreement as a *minimal*
pending record -- ``{corpus, measurement, direction, seed_def, site}`` (see
``tier_a.cli._pending_for_probe`` and the per-corpus reports under
``docs/eval/tier-a/``). An adjudicator (human, or an LLM running the
``adjudicate-sample.md`` prompt) cannot judge a bare ``file:line``; it needs the
surrounding source. The harness deliberately stores only the minimal key (it is
SHA- and tool-agnostic), so turning those keys back into reviewable evidence is a
separate, repo-local step. This tool is that step. It is intentionally NOT part
of ``tier_a`` proper: it reads corpus *source*, which the metric pipeline never
touches.

WHAT IT DOES
------------
For every ``pending`` record in one or more per-corpus report JSONs, it reads the
corpus source at the seed definition and the call site and emits the rich shape
``adjudicate-sample.md`` consumes:

    {id, corpus, measurement, direction, seed_def, seed_context, site, site_context}

- ``seed_context``  = the definition line +/- ``--seed-context`` lines (default 2).
- ``site_context``  = the call-site line +/- ``--site-context`` lines (default 3),
  with ``>`` marking the exact line.
- ``id`` is the 0-based index *within that corpus's output file*; adjudicator
  verdicts join back on ``(corpus, id)``.

Corpus roots come from ``eval/corpora.toml`` (the single source of truth). Output
is one ``<outdir>/<corpus>.json`` array per corpus seen across the inputs.

SHA SAFETY
----------
Pending line numbers are only valid at the corpus SHA the run was taken against.
The tool compares each report's ``meta.corpus_sha`` to the corpus checkout's
current ``HEAD`` and refuses to proceed on a mismatch unless ``--allow-sha-drift``
is given (hydrating against a drifted tree silently produces wrong contexts).

WHEN TO USE
-----------
After a ``uv run tier-a`` run, before dispatching adjudicators, to produce the
evidence packages. The downstream loop: hydrate -> adjudicate (per
``adjudicate-sample.md``) -> append verdicts to ``eval/adjudications.jsonl`` ->
``uv run tier-a --report-only`` to fold corrected metrics. See ``eval/README.md``
(Adjudication) and ``docs/eval/tier-a/re-anchor-adjudication-2026-06-14.md`` for a
worked example.

HOW TO USE
----------
Run under the harness env (needs Python 3.11+ for ``tomllib``):

    uv run python eval/tools/hydrate_pending.py \
        --report docs/eval/tier-a/2026-06-13-prism.json \
        --report docs/eval/tier-a/2026-06-13-caddy.json \
        --out /tmp/tier-a-adj

Then feed each ``/tmp/tier-a-adj/<corpus>.json`` to the adjudicator as the
``{{input}}`` of ``prompts/adjudicate-sample.md``.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

EVAL = Path(__file__).resolve().parents[1]   # eval/
REPO = EVAL.parent


def corpus_roots() -> dict[str, str]:
    cfg = tomllib.loads((EVAL / "corpora.toml").read_text())
    roots = {}
    for name, c in cfg["corpus"].items():
        p = os.path.expanduser(c["path"])
        if not os.path.isabs(p):
            p = os.path.join(REPO, p)
        roots[name] = os.path.abspath(p)
    return roots


def head_sha(root: str) -> str | None:
    try:
        out = subprocess.run(
            ["git", "-C", root, "rev-parse", "HEAD"],
            capture_output=True, text=True, check=True,
        )
        return out.stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


def read_lines(root: str, relfile: str) -> list[str] | None:
    try:
        return (Path(root) / relfile).read_text(
            encoding="utf-8", errors="replace"
        ).splitlines()
    except (FileNotFoundError, IsADirectoryError):
        return None


def ctx(lines: list[str] | None, line: int, before: int, after: int, mark: bool) -> str:
    if lines is None:
        return "<file not found in corpus checkout>"
    lo = max(1, line - before)
    hi = min(len(lines), line + after)
    rows = []
    for n in range(lo, hi + 1):
        pfx = ">" if (mark and n == line) else " "
        rows.append(f"{pfx}{n}: {lines[n - 1]}")
    return "\n".join(rows)


def hydrate_record(r: dict, idx: int, root: str, seed_ctx: int, site_ctx: int) -> dict:
    sfile, sline = r["seed_def"].rsplit(":", 1)
    cfile, cline = r["site"].rsplit(":", 1)
    return {
        "id": idx,
        "corpus": r["corpus"],
        "measurement": r["measurement"],
        "direction": r["direction"],
        "seed_def": r["seed_def"],
        "seed_context": ctx(read_lines(root, sfile), int(sline), seed_ctx, seed_ctx, mark=False),
        "site": r["site"],
        "site_context": ctx(read_lines(root, cfile), int(cline), site_ctx, site_ctx, mark=True),
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="Hydrate Tier-A pending diffs for adjudication.")
    ap.add_argument("--report", action="append", required=True, metavar="REPORT.json",
                    help="per-corpus report JSON with a `pending` list (repeatable)")
    ap.add_argument("--out", required=True, metavar="DIR", help="output directory")
    ap.add_argument("--seed-context", type=int, default=2)
    ap.add_argument("--site-context", type=int, default=3)
    ap.add_argument("--allow-sha-drift", action="store_true",
                    help="hydrate even if the corpus checkout HEAD != report corpus_sha")
    args = ap.parse_args()

    roots = corpus_roots()
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    by_corpus: dict[str, list[dict]] = {}
    for report_path in args.report:
        report = json.loads(Path(report_path).read_text())
        corpus_sha = report.get("meta", {}).get("corpus_sha")
        for rec in report.get("pending", []):
            by_corpus.setdefault(rec["corpus"], []).append(rec)
        # SHA-drift guard, per distinct corpus in this report
        for corpus in {rec["corpus"] for rec in report.get("pending", [])}:
            root = roots.get(corpus)
            if root is None:
                sys.exit(f"unknown corpus {corpus!r} (not in corpora.toml)")
            head = head_sha(root)
            if corpus_sha and head and not head.startswith(corpus_sha) and not args.allow_sha_drift:
                sys.exit(
                    f"SHA drift for {corpus}: checkout {head[:12]} != report {corpus_sha} "
                    f"({root}). Line numbers would be wrong; pass --allow-sha-drift to override."
                )

    total = 0
    for corpus, recs in sorted(by_corpus.items()):
        root = roots[corpus]
        items = [
            hydrate_record(r, i, root, args.seed_context, args.site_context)
            for i, r in enumerate(recs)
        ]
        missing = sum(1 for it in items
                      if "<file not found" in it["seed_context"]
                      or "<file not found" in it["site_context"])
        (out_dir / f"{corpus}.json").write_text(json.dumps(items, indent=1))
        total += len(items)
        flag = f"  (!! {missing} missing-context)" if missing else ""
        print(f"{corpus:8} {len(items):4} items -> {out_dir / f'{corpus}.json'}{flag}")
    print(f"total = {total} items")
    return 0


if __name__ == "__main__":
    sys.exit(main())
