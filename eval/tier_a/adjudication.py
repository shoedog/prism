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


def reanchor_map(
    fp_sites: set,
    fn_sites: set,
    scoped: list[Adjudication],
    site_fps: dict[str, str],
) -> dict[tuple, Adjudication]:
    """Map a live ``(direction, "file:line")`` -> the stale verdict it re-anchors to, via a
    UNIQUE 1:1 fingerprint match: exactly ONE live site AND exactly ONE stale record share
    the fingerprint. A 2:1 (two live sites, one record) or 1:2 match falls through (no
    guess) so a verdict is never applied to more than one site. ``scoped`` is pre-filtered
    to one ``(corpus, measurement, seed_def)``. Shared by ``apply_verdicts`` (counts) and
    pending generation so the two agree on what re-anchored.
    """
    live = [("prism_only", f, line) for f, line in fp_sites] + [
        ("oracle_only", f, line) for f, line in fn_sites
    ]
    live_keys = {(d, _key(f, line)) for d, f, line in live}
    rel = {(r.direction, r.site): r for r in scoped}
    live_by_fp: dict[tuple, list] = {}
    for d, f, line in live:
        if (d, _key(f, line)) in rel:  # exact-matched; cannot re-anchor
            continue
        fp = site_fps.get(_key(f, line))
        if fp:
            live_by_fp.setdefault((d, fp), []).append((d, _key(f, line)))
    rec_by_fp: dict[tuple, list] = {}
    for r in scoped:
        if r.site_fingerprint and (r.direction, r.site) not in live_keys:
            rec_by_fp.setdefault((r.direction, r.site_fingerprint), []).append(r)
    out: dict[tuple, Adjudication] = {}
    for key_fp, live_ks in live_by_fp.items():
        recs = rec_by_fp.get(key_fp, [])
        if len(live_ks) == 1 and len(recs) == 1:  # unique 1:1 only
            out[live_ks[0]] = recs[0]
    return out


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
    corpus source). A verdict matches a live site by exact ``(direction, line)`` first;
    else it re-anchors to a moved site via a UNIQUE 1:1 fingerprint match (``reanchor_map``)
    instead of being counted stale + re-pended — durable across churn, never double-applied.
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
    ra = reanchor_map(fp_sites, fn_sites, scoped, site_fps)
    out.reanchored = len(ra)

    def resolve(direction: str, f: str, line: int):
        return rel.get((direction, _key(f, line))) or ra.get((direction, _key(f, line)))

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
    # A record is stale only if its own site is not a live diff-site AND it didn't re-anchor.
    reanchored_keys = {(r.direction, r.site) for r in ra.values()}
    out.stale = sum(
        1
        for r in scoped
        if (r.direction, r.site) not in live_sites
        and (r.direction, r.site) not in reanchored_keys
    )
    return out
