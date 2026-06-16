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
    direction: str,  # "callers" | "callees" (the manifest measurement)
) -> list[dict]:
    """Per-receiver-class precision report (spec §8b). FP rule:

      raw_fp       = in-scope sites adjudicated as prism-only false positives (verdict "prism_fp")
      corrected_fp = raw_fp MINUS sites whose verdict is "ambiguous" or "oracle_artifact"
                     (i.e. prism_fp survivors only) — provisional until Slice E
      pending      = in-scope sites with NO adjudication record
      ambiguous    = in-scope sites adjudicated "ambiguous"
      fanout_width = mean `fanout` over the class's sites (0.0 if none)

    The join is line-keyed (`file:line`, the store's key); each site is matched to an
    adjudication by site line-key. Emit one dict per receiver_class, sorted by class name.

    Note: oracle_artifact contributes to neither raw_fp nor corrected_fp. Only prism_fp
    verdict sites contribute to raw_fp (and also to corrected_fp, since oracle_artifact
    and ambiguous are already excluded from corrected_fp's increment path).
    """
    # Index verdicts by their line-key site string for this corpus/measurement.
    verdict_by_site: dict[str, str] = {
        r.site: r.verdict
        for r in adjudications
        if r.corpus == corpus and r.measurement == direction
    }
    out: list[dict] = []
    for receiver_class, class_sites in sorted(stratify(sites).items()):
        raw_fp = corrected_fp = pending = ambiguous = 0
        for s in class_sites:
            v = verdict_by_site.get(s.line_key)
            if v is None:
                pending += 1
            elif v == "prism_fp":
                raw_fp += 1
                corrected_fp += 1
            elif v == "ambiguous":
                ambiguous += 1
            elif v == "oracle_artifact":
                pass  # excluded from corrected_fp; not a real prism FP
            # other verdicts (oracle_miss, alias_site, prism_fn, …) are not FPs
        fanout_width = (
            sum(s.fanout for s in class_sites) / len(class_sites) if class_sites else 0.0
        )
        out.append({
            "corpus": corpus,
            "direction": direction,
            "receiver_class": receiver_class,
            "raw_fp": raw_fp,
            "corrected_fp": corrected_fp,
            "pending": pending,
            "ambiguous": ambiguous,
            "fanout_width": fanout_width,
        })
    return out
