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

    The FP metric is computed over INTERFACE-DISPATCH sites only (``fanout > 0``); concrete
    owner-resolved receivers (``fanout == 0`` — e.g. a typed-param `*Foo` calling a method
    that merely shares a name with some interface method) are reported separately as
    ``concrete_sites`` and excluded from the FP numerator/denominator (review MAJOR 4 —
    otherwise the per-class FP rate is uninterpretable).

    FP rule (review BLOCKER 1 — ``raw_fp`` is raw prism-only membership, NOT verdict-derived):
      raw_fp       = dispatch sites in the prism-only set (raw FP candidates, pre-adjudication).
                     ``prism_only_keys`` (byte-keys) is the oracle-derived prism-only set when
                     available (Slice E); when None (PR-2, no oracle run) every in-scope
                     dispatch site is provisionally a candidate.
      corrected_fp = raw_fp MINUS sites adjudicated ``ambiguous`` / ``oracle_artifact``
                     (excused — not real prism FPs).
      pending      = raw dispatch sites with NO adjudication record.
      ambiguous    = dispatch sites adjudicated ``ambiguous``.
      fanout_width = mean fanout over dispatch sites (0.0 if none).

    The adjudication join is line-keyed (``file:line``) and scoped to ``direction ==
    "prism_only"`` records for this corpus + measurement (review MAJOR 3 — an ``oracle_only``
    record at the same line must not steal the verdict). A byte-keyed store + fingerprint
    re-anchoring is deferred to Slice E (see the PR-2 deferred doc); byte-span remains the
    manifest's own identity (``ManifestSite.byte_key``).
    """
    # Prism-only-direction verdicts for this corpus + measurement, keyed by line (MAJOR 3).
    verdict_by_site: dict[str, str] = {
        r.site: r.verdict
        for r in adjudications
        if r.corpus == corpus and r.measurement == direction and r.direction == "prism_only"
    }
    out: list[dict] = []
    for receiver_class, class_sites in sorted(stratify(sites).items()):
        # Prism-only membership (BLOCKER 1): the supplied oracle set, else provisional (all).
        prism_only = [
            s for s in class_sites
            if prism_only_keys is None or s.byte_key in prism_only_keys
        ]
        # Interface-dispatch vs concrete (MAJOR 4): the FP metric is dispatch-only.
        dispatch = [s for s in prism_only if s.fanout > 0]
        concrete = [s for s in prism_only if s.fanout == 0]
        ambiguous = sum(1 for s in dispatch if verdict_by_site.get(s.line_key) == "ambiguous")
        oracle_artifact = sum(
            1 for s in dispatch if verdict_by_site.get(s.line_key) == "oracle_artifact"
        )
        pending = sum(1 for s in dispatch if verdict_by_site.get(s.line_key) is None)
        raw_fp = len(dispatch)
        corrected_fp = raw_fp - ambiguous - oracle_artifact
        fanout_width = (
            sum(s.fanout for s in dispatch) / len(dispatch) if dispatch else 0.0
        )
        out.append({
            "corpus": corpus,
            "direction": direction,
            "receiver_class": receiver_class,
            "dispatch_sites": len(dispatch),
            "concrete_sites": len(concrete),
            "raw_fp": raw_fp,
            "corrected_fp": corrected_fp,
            "pending": pending,
            "ambiguous": ambiguous,
            "fanout_width": fanout_width,
        })
    return out
