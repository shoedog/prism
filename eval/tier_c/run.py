# eval/tier_c/run.py
"""Run orchestrator (spec §5). run_issue drives one open issue through the spec->plan
chain with whatever ArmRunner/judges/investigator are supplied (fakes in tests, live
runners in a real run). The corpus + live runners are wired by cli.py run --live."""
from __future__ import annotations
from .chain import run_spec_plan_chain, ChainResult
from .prompts import stage_prompt


def run_issue(issue, *, variants, runner, co, judges, relevance, plants,
              claim_counts=None) -> ChainResult:
    """Drive one issue through the spec->plan chain.

    claim_counts defaults to {v.id: 1 for v in variants} as a placeholder.
    Per-output claim counting (via claims.count_claims on each output.text) is a
    documented follow-up wired in run_stage — see Phase-1b plan Task 7 Step 3 note.
    """
    cc = claim_counts or {v.id: 1 for v in variants}
    return run_spec_plan_chain(
        issue_text=issue.text,
        scoped_slice=issue.scoped_slice,
        variants=variants,
        runner=runner,
        co=co,
        claim_counts=cc,
        plants=plants,
        judges=judges,
        relevance=relevance,
        prompt_fn=stage_prompt,
    )
