# eval/tier_c/run.py
"""Run orchestrator (spec §5). run_issue drives one open issue through the spec->plan
chain with whatever ArmRunner/judges/investigator are supplied (fakes in tests, live
runners in a real run). The corpus + live runners are wired by cli.py run --live."""
from __future__ import annotations
from dataclasses import dataclass
from .chain import run_spec_plan_chain, ChainResult
from .prompts import stage_prompt


def run_issue(issue, *, variants, runner, co, judges, relevance, plants,
              claim_counts=None) -> ChainResult:
    """Drive one issue through the spec->plan chain.

    claim_counts=None (default) -> run_stage derives per-output via count_claims;
    pass an explicit dict to override.
    """
    return run_spec_plan_chain(
        issue_text=issue.text,
        scoped_slice=issue.scoped_slice,
        variants=variants,
        runner=runner,
        co=co,
        claim_counts=claim_counts,
        plants=plants,
        judges=judges,
        relevance=relevance,
        prompt_fn=stage_prompt,
    )


# ---------------------------------------------------------------------------
# Live run loop (Task 5) — wires run_issue per issue into a pooled Report
# ---------------------------------------------------------------------------

@dataclass
class LiveComponents:
    """All injectable components for a live run (variants, runners, judges, checkout factory).

    open_checkout: callable(repo: str, sha: str) -> context-manager returning a Checkout-like
    object.  Defaults to tier_c.checkout.Checkout if not supplied; callers may override with
    a fake (FakeCo) for unit tests.
    """
    variants: list
    runner: object
    judges: dict
    relevance: object
    guesser: object
    plants: list
    open_checkout: object = None   # callable(repo, sha) -> ctx-manager; default = Checkout

    def __post_init__(self):
        if self.open_checkout is None:
            from .checkout import Checkout
            self.open_checkout = Checkout


@dataclass(frozen=True)
class Report:
    """Aggregated per-(stage x language) cells + pooled detectability."""
    cells: dict       # dict[(stage, language), Cell]
    detectability: object   # Detectability


def _avg(ms: list) -> "StageMetrics":
    """Average a list of StageMetrics into a single representative StageMetrics.

    Numeric fields (precision, recall, planted, tokens) are arithmetic means.
    used_prism uses majority vote: True when the True count >= (n+1)//2.
    """
    from .report import StageMetrics
    n = len(ms)
    return StageMetrics(
        precision=sum(m.precision for m in ms) / n,
        recall=sum(m.recall for m in ms) / n,
        planted=sum(m.planted for m in ms) / n,
        used_prism=(sum(1 for m in ms if m.used_prism) >= (n + 1) // 2),
        tokens=sum(m.tokens for m in ms) // n,
    )


def run_live(issues, comps: LiveComponents) -> "Report":
    """Drive all issues through the spec->plan chain and assemble a Report.

    Pooling strategy (spec §6b): a single stage's 4 outputs cannot reach p<0.05
    (min p=0.0625 > 0.05), so ArmOutputs are pooled across ALL issues × stages before
    the single detectability call at the end.

    per_cell accumulates per-variant StageMetrics keyed by (stage, language).  Multiple
    issues of the same language are APPENDED (not overwritten) and averaged via _avg
    before assemble_cell, so each (stage, language) cell reflects all issues.
    """
    from .report import StageMetrics, assemble_cell
    from .detect import run_detectability

    pool = []                      # all ArmOutputs across issues × stages
    per_cell: dict = {}            # (stage, language) -> {vid: list[StageMetrics]}

    for issue in issues:
        with comps.open_checkout(issue.repo, issue.sha) as co:
            chain = run_issue(
                issue,
                variants=comps.variants,
                runner=comps.runner,
                co=co,
                judges=comps.judges,
                relevance=comps.relevance,
                plants=comps.plants,
            )

        for stage_result in chain.stages:
            # Pool all ArmOutputs from this stage for detectability
            if stage_result.outputs is not None:
                pool.extend(stage_result.outputs)

            key = (stage_result.stage, issue.language)
            vid_lists = per_cell.setdefault(key, {})

            for vid in stage_result.investigator:
                vid_lists.setdefault(vid, []).append(StageMetrics(
                    precision=stage_result.investigator[vid].precision,
                    recall=stage_result.investigator[vid].recall,
                    planted=stage_result.planted[vid].recall,
                    used_prism=stage_result.used_prism[vid],
                    tokens=stage_result.tokens[vid],
                ))

    # Single pooled detectability call (n = variants × stages × issues)
    detect = run_detectability(pool, comps.guesser)

    # Average each vid's list of per-issue StageMetrics into one representative value
    cells = {}
    models = sorted({v.model for v in comps.variants})
    for (stage, language), vid_lists in per_cell.items():
        per_id = {vid: _avg(lst) for vid, lst in vid_lists.items()}
        cells[(stage, language)] = assemble_cell(
            stage=stage,
            language=language,
            per_id=per_id,
            models=models,
            analyze_failure_rate=0.0,   # prism analyze-failure tracking deferred to Phase-2
            detectable=detect.detectable,
        )

    return Report(cells=cells, detectability=detect)
