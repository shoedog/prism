"""Adjudication store (spec §2.8): keyed records, legal-combo validation, the
metric-contribution truth table, stale/pending/budget accounting."""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

LEGAL = {
    "prism_only": {
        "oracle_miss",
        "prism_fp",
        "oracle_artifact",
        "ambiguous",
        "alias_site",
    },
    "oracle_only": {"prism_fn", "oracle_artifact", "ambiguous"},
}


class IllegalAdjudication(ValueError):
    pass


@dataclass(frozen=True)
class Adjudication:
    """A human verdict for one raw diff site.

    `site_fingerprint` is optional and backward-compatible. Line-keyed records can
    go stale after corpus drift; fingerprint-based re-anchoring is the planned
    migration path for durable verdicts.
    """

    corpus: str
    measurement: str        # "callers" | "callees" | "m3"
    direction: str          # "prism_only" | "oracle_only"
    seed_def: str           # "file:selection_line" of the sampled symbol
    site: str               # "file:line" of the call site
    verdict: str
    reason: str
    adjudicated_by: str
    date: str
    site_fingerprint: str | None = None


def validate(r: Adjudication) -> Adjudication:
    if r.direction not in LEGAL or r.verdict not in LEGAL[r.direction]:
        raise IllegalAdjudication(f"{r.direction} x {r.verdict} is not a legal combination")
    return r


def load_records(path: Path) -> list[Adjudication]:
    if not path.exists():
        return []
    return [
        validate(Adjudication(**json.loads(line)))
        for line in path.read_text().splitlines()
        if line.strip()
    ]


@dataclass
class Corrected:
    tp: int = 0
    fp: int = 0
    fn: int = 0
    pending: int = 0
    excluded: int = 0
    oracle_miss_count: int = 0
    reanchored: int = 0
    stale: int = 0


def _key(file: str, line: int) -> str:
    return f"{file}:{line}"


def fingerprint(window_lines: list[str]) -> str:
    """Drift-stable site key: a hash of the normalized site window (call line +/- 1).

    Survives line-number shifts so a verdict re-anchors when resolution churn moves a
    call site to a new line; changes only when the call code itself changes. Used both
    when stamping live diff-sites (harness, with corpus source) and adjudication records
    (backfill / future adjudication), so the two match across drift.
    """
    norm = "\n".join(line.strip() for line in window_lines)
    return hashlib.sha256(norm.encode("utf-8")).hexdigest()[:16]


def apply_verdicts(
    tp: int,
    fp_sites: set,
    fn_sites: set,
    records: list[Adjudication],
    corpus: str,
    measurement: str,
    seed_def: str,
    site_fps: dict[str, str] | None = None,
) -> Corrected:
    """The §2.8 truth table. fp_sites/fn_sites are (file, line) raw-diff sets.

    ``site_fps`` maps live ``"file:line"`` -> fingerprint (stamped by the harness from
    corpus source). A verdict is matched to a live site by exact ``(direction, line)``
    first; if its line is stale but a stale record's ``site_fingerprint`` uniquely
    matches a live site's fingerprint, the verdict is RE-ANCHORED to that site instead
    of being counted stale + re-pended (durable across resolution churn).
    """
    site_fps = site_fps or {}
    out = Corrected(tp=tp)
    scoped = [
        r
        for r in records
        if r.corpus == corpus and r.measurement == measurement and r.seed_def == seed_def
    ]
    rel = {(r.direction, r.site): r for r in scoped}
    live_sites = (
        {("prism_only", _key(f, line)) for f, line in fp_sites}
        | {("oracle_only", _key(f, line)) for f, line in fn_sites}
    )
    # Re-anchor index: stale records (line not live) keyed by (direction, fingerprint).
    rel_fp: dict[tuple, list] = {}
    for r in scoped:
        if r.site_fingerprint and (r.direction, r.site) not in live_sites:
            rel_fp.setdefault((r.direction, r.site_fingerprint), []).append(r)
    reanchored: set = set()

    def resolve(direction: str, f: str, line: int):
        r = rel.get((direction, _key(f, line)))
        if r is not None:
            return r
        lfp = site_fps.get(_key(f, line))
        if lfp:
            cands = rel_fp.get((direction, lfp), [])
            if len(cands) == 1:  # unique fingerprint match → safe re-anchor
                r = cands[0]
                reanchored.add((r.direction, r.site))
                out.reanchored += 1
                return r
        return None

    for f, line in fp_sites:
        r = resolve("prism_only", f, line)
        if r is None:
            out.pending += 1
        elif r.verdict == "oracle_miss":
            out.tp += 1
            out.oracle_miss_count += 1
        elif r.verdict == "prism_fp":
            out.fp += 1
        else:                       # oracle_artifact | ambiguous | alias_site
            out.excluded += 1
    for f, line in fn_sites:
        r = resolve("oracle_only", f, line)
        if r is None:
            out.pending += 1
        elif r.verdict == "prism_fn":
            out.fn += 1
        else:
            out.excluded += 1
    out.stale = sum(1 for k in rel if k not in live_sites and k not in reanchored)
    return out
