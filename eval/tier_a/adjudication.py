"""Adjudication store (spec §2.8): keyed records, legal-combo validation, the
metric-contribution truth table, stale/pending/budget accounting."""
from __future__ import annotations

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
    stale: int = 0


def _key(file: str, line: int) -> str:
    return f"{file}:{line}"


def apply_verdicts(
    tp: int,
    fp_sites: set,
    fn_sites: set,
    records: list[Adjudication],
    corpus: str,
    measurement: str,
    seed_def: str,
) -> Corrected:
    """The §2.8 truth table. fp_sites/fn_sites are (file, line) raw-diff sets."""
    out = Corrected(tp=tp)
    rel = {
        (r.direction, r.site): r
        for r in records
        if r.corpus == corpus and r.measurement == measurement and r.seed_def == seed_def
    }
    live_sites = (
        {("prism_only", _key(f, line)) for f, line in fp_sites}
        | {("oracle_only", _key(f, line)) for f, line in fn_sites}
    )
    out.stale = sum(1 for k in rel if k not in live_sites)
    for f, line in fp_sites:
        r = rel.get(("prism_only", _key(f, line)))
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
        r = rel.get(("oracle_only", _key(f, line)))
        if r is None:
            out.pending += 1
        elif r.verdict == "prism_fn":
            out.fn += 1
        else:
            out.excluded += 1
    return out
