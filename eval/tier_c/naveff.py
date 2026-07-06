"""D4 — navigation efficiency (mechanical, free; no judge, no new live calls). Pure
aggregation over already-saved ArmOutput fields (tool_calls, commands, wall_s,
cost_usd) plus the citation set citations.py already parsed from the arm text.

KNOWN DEGRADATION (documented, not silently approximated): the harness does not record
a timestamp per tool call, so "calls/wall-seconds to the FIRST valid citation" (as
literally specified) cannot be computed exactly. We report the arm-level AGGREGATE
tool_calls/wall_s instead — labeled as totals, never mislabeled as first-citation
timing — plus a wasted-exploration rate and cost-per-valid-citation, which ARE exactly
computable from saved fields.

wasted_exploration_rate additionally degrades to None (not 0.0) when ArmOutput.commands
is empty — which is the case for EVERY claude/opus arm today (parse_claude_stream_json
does not populate ModelResult.commands; only parse_codex_jsonl does, from
command_execution items). Reporting None (rather than fabricating 0.0) keeps the
"coverage" honest per the spec's fail-open-and-report-degradation posture.
"""
from __future__ import annotations

import re
from dataclasses import dataclass

from .model import ArmOutput

_FILE_TOKEN = re.compile(
    r"[\w./-]+\.(?:rs|go|py|js|jsx|ts|tsx|c|cc|cpp|h|hpp|java|lua)\b"
)


def _files_touched(commands: list[str]) -> set[str]:
    touched: set[str] = set()
    for cmd in commands:
        touched.update(_FILE_TOKEN.findall(cmd))
    return touched


@dataclass(frozen=True)
class NavEfficiency:
    tool_calls: int                          # arm-level total (ArmOutput.tool_calls)
    wall_s: float                            # arm-level total wall-clock
    cost_usd: float
    valid_citations: int
    cost_per_valid_citation: float | None    # None when valid_citations == 0
    wasted_exploration_rate: float | None    # None when commands unavailable (degrade, don't fabricate)
    files_touched: int
    files_cited: int


def nav_efficiency(arm_out: ArmOutput, *, valid_citations: int) -> NavEfficiency:
    touched = _files_touched(arm_out.commands)
    cited_files = {c.file for c in arm_out.citations}
    wasted = (len(touched - cited_files) / len(touched)) if touched else None
    cost_per_valid = (arm_out.cost_usd / valid_citations) if valid_citations > 0 else None
    return NavEfficiency(
        tool_calls=arm_out.tool_calls,
        wall_s=arm_out.wall_s,
        cost_usd=arm_out.cost_usd,
        valid_citations=valid_citations,
        cost_per_valid_citation=cost_per_valid,
        wasted_exploration_rate=wasted,
        files_touched=len(touched),
        files_cited=len(cited_files),
    )


def nav_efficiency_to_dict(ne: NavEfficiency) -> dict:
    return {
        "tool_calls": ne.tool_calls,
        "wall_s": ne.wall_s,
        "cost_usd": ne.cost_usd,
        "valid_citations": ne.valid_citations,
        "cost_per_valid_citation": ne.cost_per_valid_citation,
        "wasted_exploration_rate": ne.wasted_exploration_rate,
        "files_touched": ne.files_touched,
        "files_cited": ne.files_cited,
    }
