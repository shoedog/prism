"""Phase-IP PR-2 in-scope interface-dispatch manifest reader + precision gate report
(spec §8a/§8b). The manifest is the structural denominator emitted by
`prism nav interface-manifest`; the gate report joins it to the adjudication store and
is REPORTED, never gating. `corrected_fp` is provisional until the Slice-E re-adjudication.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .adjudication import Adjudication


@dataclass(frozen=True)
class ManifestSite:
    file: str
    start_byte: int
    end_byte: int
    line: int
    receiver_class: str
    method: str
    fanout: int

    @property
    def byte_key(self) -> str:
        """Primary identity: byte-span (NOT file:line, which is display only)."""
        return f"{self.file}:{self.start_byte}:{self.end_byte}"

    @property
    def line_key(self) -> str:
        """The adjudication join key (the store is line-keyed: file:line)."""
        return f"{self.file}:{self.line}"


def load_manifest(path: Path) -> list[ManifestSite]:
    """Parse a `prism nav interface-manifest` JSON document into ManifestSite records."""
    doc = json.loads(Path(path).read_text())
    return [ManifestSite(**s) for s in doc.get("sites", [])]


def stratify(sites: list[ManifestSite]) -> dict[str, list[ManifestSite]]:
    """Group in-scope sites by receiver_class (spec §8a stratification)."""
    out: dict[str, list[ManifestSite]] = {}
    for s in sites:
        out.setdefault(s.receiver_class, []).append(s)
    return out


def gate_report(
    sites: list[ManifestSite],
    adjudications: list[Adjudication],
    corpus: str,
    direction: str,  # the manifest measurement: "callers" | "callees"
    prism_only_keys: set[str] | None = None,
) -> list[dict]:
    """Per-receiver-class precision report (spec §8b). REPORTED, never gating;
    `corrected_fp` is provisional until the Slice-E re-adjudication.

    Denominator fields (`dispatch_sites`, `concrete_sites`, `fanout_width`) are computed over
    ALL in-scope sites of the class (review MAJOR 2). The FP numerator is computed over the
    prism-only DISPATCH subset (fanout>0) — `prism_only_keys` is the oracle-derived prism-only
    byte-key set in Slice E; when None (PR-2, no oracle run) every in-scope dispatch site is a
    provisional candidate.

    FP rule (review BLOCKER 1 — positive selection against the adjudication truth table):
      corrected_fp = sites adjudicated `prism_fp` only (oracle_miss=TP, alias_site/ambiguous/
                     oracle_artifact=excused, no-record=pending are all NOT corrected FPs).
      raw_fp       = prism-only dispatch candidates (pre-adjudication upper bound).
      pending      = prism-only dispatch sites with no adjudication record.
      ambiguous    = prism-only dispatch sites adjudicated `ambiguous`.

    The join is line-keyed (`file:line`) and scoped to `direction == "prism_only"` records for
    this corpus + measurement (an `oracle_only` record at the same line must not steal the
    verdict). A byte-keyed store + fingerprint re-anchoring is deferred to Slice E.
    """
    verdict_by_site: dict[str, str] = {
        r.site: r.verdict
        for r in adjudications
        if r.corpus == corpus and r.measurement == direction and r.direction == "prism_only"
    }
    out: list[dict] = []
    for receiver_class, class_sites in sorted(stratify(sites).items()):
        # Denominator over ALL class sites (review MAJOR 2):
        all_dispatch = [s for s in class_sites if s.fanout > 0]
        concrete = [s for s in class_sites if s.fanout == 0]
        # FP numerator over the prism-only dispatch subset:
        prism_only = [
            s for s in all_dispatch
            if prism_only_keys is None or s.byte_key in prism_only_keys
        ]
        corrected_fp = sum(
            1 for s in prism_only if verdict_by_site.get(s.line_key) == "prism_fp"
        )
        ambiguous = sum(
            1 for s in prism_only if verdict_by_site.get(s.line_key) == "ambiguous"
        )
        pending = sum(1 for s in prism_only if verdict_by_site.get(s.line_key) is None)
        raw_fp = len(prism_only)
        fanout_width = (
            sum(s.fanout for s in all_dispatch) / len(all_dispatch) if all_dispatch else 0.0
        )
        out.append({
            "corpus": corpus,
            "direction": direction,
            "receiver_class": receiver_class,
            "dispatch_sites": len(all_dispatch),
            "concrete_sites": len(concrete),
            "raw_fp": raw_fp,
            "corrected_fp": corrected_fp,
            "pending": pending,
            "ambiguous": ambiguous,
            "fanout_width": fanout_width,
        })
    return out
