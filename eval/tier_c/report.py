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


# ---------------------------------------------------------------------------
# Per stage×language report cells (Task 6, Phase-1b)
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class StageMetrics:
    precision: float
    recall: float
    planted: float
    used_prism: bool
    tokens: int

@dataclass(frozen=True)
class Cell:
    stage: str
    language: str
    prism_precision_delta: dict        # model -> ON-OFF
    prism_recall_delta: dict
    prism_planted_delta: dict
    itt_available_rate: float        # fraction of variants that HAD prism (intent-to-treat)
    per_protocol_used_rate: float    # fraction that actually used prism (tool_calls>0)
    gate: Gate

def assemble_cell(*, stage, language, per_id, models, analyze_failure_rate, detectable,
                  family_bias_band: float = 0.0) -> Cell:
    """Assemble a per-(stage x language) Cell from per-variant StageMetrics.

    For each model in ``models``, computes the within-model prism delta across
    precision / recall / planted (ON variant keyed as ``{model}+prism``, OFF as
    ``{model}``).  ITT used-prism rate is the fraction of all supplied variants
    whose ``used_prism`` flag is True.  The GO/NO-GO gate is driven by the
    maximum delta across all models and all three objectives.
    """
    def dlt(attr):
        return {
            m: getattr(per_id.get(f"{m}+prism"), attr, 0.0) - getattr(per_id.get(m), attr, 0.0)
            for m in models
            if (f"{m}+prism" in per_id and m in per_id)
        }

    pd, rd, ld = dlt("precision"), dlt("recall"), dlt("planted")
    n = len(per_id)
    itt_available_rate = sum(1 for vid in per_id if vid.endswith("+prism")) / n if n else 0.0
    per_protocol_used_rate = sum(v.used_prism for v in per_id.values()) / n if n else 0.0
    gate = gate_decision(
        precision_delta=max(pd.values(), default=0.0),
        recall_delta=max(rd.values(), default=0.0),
        planted_delta=max(ld.values(), default=0.0),
        analyze_failure_rate=analyze_failure_rate,
        cost_ok=True,
        detectable_judges=detectable,
    )
    return Cell(stage, language, pd, rd, ld, itt_available_rate, per_protocol_used_rate, gate)
