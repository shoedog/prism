# eval/tier_c/run.py
"""Run orchestrator (spec §5). run_issue drives one open issue through the spec->plan
chain with whatever ArmRunner/judges/investigator are supplied (fakes in tests, live
runners in a real run). The corpus + live runners are wired by cli.py run --live."""
from __future__ import annotations
from dataclasses import dataclass
from typing import TYPE_CHECKING
from .chain import run_spec_plan_chain, ChainResult
from .prompts import stage_prompt

if TYPE_CHECKING:
    from .store import RunStore


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

    store: a fully-initialised RunStore (ensure_new already called by the CLI).  When None,
    run_live skips all persistence — pure-logic tests pass None here.
    lsp_shim_dir: path to the LSP-deny shim directory; created by the CLI and passed in.
    When None, run_live skips shim setup (tests that don't exercise the shim pass None).
    """
    variants: list
    runner: object
    judges: dict
    relevance: object
    guesser: object
    plants: list
    open_checkout: object = None   # callable(repo, sha) -> ctx-manager; default = Checkout
    store: "RunStore | None" = None
    lsp_shim_dir: str | None = None

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
    before assemble_cell_2x2, so each (stage, language) cell reflects all issues.

    Persistence (spec §4): when comps.store is set (a fully-initialised RunStore whose
    manifest was written by the CLI before calling run_live), per-stage artifacts
    (prompt, seeds, variant outputs, judges, investigator, best) plus detectability +
    report are persisted under runs/<run_id>/.  The CLI is the single owner of the
    RunStore; run_live calls write_manifest() once to flush the CLI's full manifest.

    Deferred (Phase-1d-replay follow-up): raw runner transcript + per-variant prism SHA
    in the arm_<vid> artifact (spec §4 nit).
    """
    import dataclasses
    from .report import StageMetrics, assemble_cell_2x2
    from .detect import run_detectability

    store = comps.store  # may be None (pure-logic tests)

    # Capture per-stage upstream frame so we can persist prompt.json correctly.
    # We replay stage_prompt with the same inputs as run_spec_plan_chain uses,
    # so we need to track what was passed per stage.
    _upstream_for_stage: dict = {}   # stage -> upstream frame fed to prompt_fn

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

        # Re-derive the upstream frames so we can write prompt artifacts.
        # spec upstream = the issue text (salted if plants present).
        from .planted import inject as _inject
        spec_upstream = _inject(issue.text, comps.plants)[0] if comps.plants else issue.text
        spec_prompt_text = stage_prompt("spec", issue_text=spec_upstream,
                                        scoped_slice=issue.scoped_slice)
        _upstream_for_stage["spec"] = (spec_prompt_text, spec_upstream)

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

            # Persist stage artifacts when store is present
            if store is not None:
                s = stage_result.stage
                # prompt.json (Fix 2)
                if s == "spec":
                    prompt_text, upstream = _upstream_for_stage.get("spec", ("", ""))
                else:
                    # plan upstream = cleaned-best spec (may have been salted)
                    # The chain's spec stage carries cleaned_best_text; find it.
                    spec_sr = next((sr for sr in chain.stages if sr.stage == "spec"), None)
                    plan_upstream = (_inject(spec_sr.cleaned_best_text, comps.plants)[0]
                                     if (comps.plants and spec_sr) else
                                     (spec_sr.cleaned_best_text if spec_sr else ""))
                    prompt_text = stage_prompt("plan", issue_text=issue.text,
                                              scoped_slice=issue.scoped_slice,
                                              upstream=plan_upstream)
                    upstream = plan_upstream
                store.write_stage_artifact(s, "prompt", {
                    "prompt": prompt_text,
                    "upstream": upstream,
                })
                # judges.json (Fix 2)
                store.write_stage_artifact(s, "judges", {
                    "rankings": stage_result.rankings,
                    "consensus": stage_result.consensus,
                })
                # seeds.json
                store.write_stage_artifact(s, "seeds", {
                    "shuffle": stage_result.shuffle_seed,
                    "tiebreak": f"{s}|tiebreak",
                    "label_map": stage_result.label_map or {},
                })
                # arm_<vid>.json per variant
                if stage_result.outputs is not None:
                    for arm_out in stage_result.outputs:
                        vid = arm_out.variant.id
                        store.write_stage_artifact(s, f"arm_{vid.replace('+','_')}",
                                                   dataclasses.asdict(arm_out))
                # investigator.json (Fix 2)
                store.write_stage_artifact(s, "investigator", {
                    vid: dataclasses.asdict(rep)
                    for vid, rep in stage_result.investigator.items()
                })
                # best.json
                store.write_stage_artifact(s, "best", {
                    "best_variant_id": stage_result.best_variant_id,
                    "consensus": stage_result.consensus,
                })

    # Single pooled detectability call (n = variants × stages × issues)
    detect = run_detectability(pool, comps.guesser)

    # Average each vid's list of per-issue StageMetrics into one representative value
    cells = {}
    models = sorted({v.model for v in comps.variants})
    for (stage, language), vid_lists in per_cell.items():
        per_id = {vid: _avg(lst) for vid, lst in vid_lists.items()}
        cells[(stage, language)] = assemble_cell_2x2(
            stage=stage,
            language=language,
            per_id=per_id,
            models=models,
            analyze_failure_rate=0.0,   # prism analyze-failure tracking deferred to Phase-2
            detectable=detect.detectable,
        )

    report = Report(cells=cells, detectability=detect)

    # Persist detectability + report at run-dir ROOT + write manifest (Fix 3)
    if store is not None:
        store.write_root_artifact("detectability", dataclasses.asdict(detect))
        store.write_root_artifact("report", {
            "cells": {
                f"{stage}/{language}": {
                    "gate": cell.gate.decision,
                    "prism_at_lsp_on": cell.prism_at_lsp_on,
                    "prism_at_lsp_off": cell.prism_at_lsp_off,
                    "lsp_at_prism_off": cell.lsp_at_prism_off,
                    "lsp_at_prism_on": cell.lsp_at_prism_on,
                    "interaction": cell.interaction,
                }
                for (stage, language), cell in cells.items()
            }
        })
        store.write_manifest()  # CLI's full manifest; single write here

    return report


