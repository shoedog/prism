"""Markdown rendering for Tier-A base-vs-candidate comparisons."""
from __future__ import annotations

import json


def _json_section(lines: list[str], title: str, value) -> None:
    lines.extend([
        "",
        f"## {title}",
        "",
        "```json",
        json.dumps(value, indent=2, sort_keys=True),
        "```",
    ])


def _metric_tier(m2: dict, tier: str) -> dict:
    rendered = {}
    for leg, directions in m2.items():
        rendered[leg] = {}
        for direction, strata in directions.items():
            rendered[leg][direction] = {
                stratum: row[tier]
                for stratum, row in strata.items()
                if tier in row
            }
    return rendered


def render_compare_md(cmp: dict) -> str:
    """Render every populated field in the comparative report contract."""
    lines = [
        "# Tier-A base-vs-candidate comparison",
        "",
        f"**Verdict:** `{cmp['verdict']}`",
    ]
    _json_section(lines, "Runs", cmp.get("runs", {}))
    _json_section(lines, "Generation metadata", cmp.get("meta", {}))
    _json_section(lines, "Identities", cmp.get("identities", {}))
    _json_section(lines, "Identity legs", cmp.get("identity_legs", {}))
    _json_section(lines, "Comparison policy", cmp.get("policy", {}))

    lines.extend(["", "## Criteria", ""])
    lines.extend(
        f"- [{'x' if passed else ' '}] `{name}`"
        for name, passed in cmp.get("criteria", {}).items()
    )
    _json_section(lines, "Probe classes", cmp.get("classes", {}))
    _json_section(lines, "Class counts", cmp.get("counts", {}))
    _json_section(lines, "Oracle changed summary (excluded)", cmp.get("oracle_changed", {}))
    _json_section(lines, "Oracle changed answers", cmp.get("oracle_changed_answers", {}))
    _json_section(lines, "Pinned oracle exclusions", cmp.get("pinned_oracle_changed", []))
    _json_section(lines, "Pinned outcome differences", cmp.get("pinned_diffs", {}))
    _json_section(lines, "Matrix differences", cmp.get("matrix_diffs", {}))
    _json_section(lines, "Status transitions", cmp.get("status_transitions", {}))
    _json_section(
        lines,
        "Lost matched oracle sites",
        cmp.get("lost_matched_oracle_sites", {}),
    )

    measured = cmp.get("m2", {})
    _json_section(lines, "Corrected measured metrics", _metric_tier(measured, "corrected"))
    _json_section(lines, "Raw measured metrics", _metric_tier(measured, "raw"))
    _json_section(lines, "Measured metric rows", measured)
    _json_section(lines, "Full-sample metrics (appendix)", cmp.get("full_sample_m2", {}))
    _json_section(lines, "Wilson flags (descriptive)", cmp.get("wilson_flags", []))
    _json_section(lines, "Recall regressions", cmp.get("recall_regressions", []))
    lines.append("")
    return "\n".join(lines)
