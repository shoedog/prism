"""Reporting + investment gate (spec §7, §12). Primary signal = within-model prism
delta on the OBJECTIVE channel; cross-model delta carries the family-bias band; the
GO/NO-GO gate is per role x language and never averaged."""
from __future__ import annotations
from dataclasses import dataclass

def prism_delta(metric_by_id: dict[str, float], model: str) -> float:
    return metric_by_id.get(f"{model}+prism", 0.0) - metric_by_id.get(model, 0.0)

@dataclass(frozen=True)
class Gate:
    decision: str   # GO | NO-GO
    reason: str

_MATERIAL = 0.1            # min objective lift to count
_MAX_ANALYZE_FAIL = 0.34   # above this = coverage debt, fix-first

def gate_decision(*, precision_delta, recall_delta, planted_delta,
                  analyze_failure_rate, cost_ok, detectable_judges) -> Gate:
    if analyze_failure_rate > _MAX_ANALYZE_FAIL:
        return Gate("NO-GO", "high prism analyze-failure / coverage debt — fix maturity first (spec §12)")
    lift = max(precision_delta, recall_delta, planted_delta)
    if lift < _MATERIAL:
        return Gate("NO-GO", "no material objective lift (flat/negative)")
    if not cost_ok:
        return Gate("NO-GO", "lift not net of token/latency cost (non-final, spec §8)")
    note = " (judges detectable -> objective-only)" if detectable_judges else ""
    return Gate("GO", f"material objective lift, acceptable cost & coverage{note}")
