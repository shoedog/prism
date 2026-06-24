# eval/tier_c/chain.py
"""Stage orchestration + spec->plan chaining (spec §5). run_stage: 4 variants ->
investigator + planted catch + judge consensus -> sanitized cleaned-best carried
forward. Per-stage prism delta is conditional on the carried frame (provenance logged)."""
from __future__ import annotations
import re
import random
from dataclasses import dataclass
from .investigator import score_citations, InvestigatorReport
from .planted import PlantedError, score_catch, sanitation_ok, PlantedReport, inject
from .judges import borda_consensus

@dataclass(frozen=True)
class StageResult:
    stage: str
    investigator: dict[str, InvestigatorReport]
    planted: dict[str, PlantedReport]
    consensus: list[str]              # best-first variant ids
    best_variant_id: str
    cleaned_best_text: str
    used_prism: dict[str, bool]
    tokens: dict[str, int]

def _strip_plants(text: str, plants: list[PlantedError]) -> str:
    out = text
    for p in plants:
        out = re.sub(re.escape(p.token), "[removed]", out, flags=re.IGNORECASE)
    return out

def run_stage(*, stage, variants, runner, co, prompt, repo_root, claim_counts=None,
              plants, judges, relevance) -> StageResult:
    outputs = {v.id: runner.run(v, stage, prompt, repo_root) for v in variants}
    if claim_counts is None:
        from .claims import count_claims
        claim_counts = {vid: max(1, count_claims(o.text)) for vid, o in outputs.items()}
    investigator = {
        vid: score_citations(co, o.citations, claim_count=claim_counts[vid],
                             relevance=relevance)
        for vid, o in outputs.items()
    }
    planted = {vid: score_catch(o.text, plants) for vid, o in outputs.items()}
    # Blind + shuffle (spec §6b): judges see opaque labels in a condition-independent
    # order, NEVER the variant id (which would leak "+prism"). Seeded by stage+ids so
    # the run is reproducible but the order does not encode the prism condition.
    vids = list(outputs.keys())
    shuffled = vids[:]
    random.Random(f"{stage}|{'|'.join(sorted(vids))}").shuffle(shuffled)
    label_to_vid = {f"cand{i}": vid for i, vid in enumerate(shuffled)}
    candidates = {lbl: outputs[vid].text for lbl, vid in label_to_vid.items()}
    rankings = {fam: j.rank(stage, "rubric", candidates) for fam, j in judges.items()}
    consensus = [label_to_vid[lbl] for lbl in borda_consensus(rankings)]
    best = consensus[0]
    cleaned = _strip_plants(outputs[best].text, plants)
    # Sanitation gate (spec §5 / codex new-5): explicit raise (a bare assert is stripped by python -O).
    if not sanitation_ok(cleaned, plants):
        raise RuntimeError("sanitation gate failed: planted residue in carried frame (spec §5)")
    return StageResult(
        stage=stage, investigator=investigator, planted=planted, consensus=consensus,
        best_variant_id=best, cleaned_best_text=cleaned,
        used_prism={vid: o.used_prism for vid, o in outputs.items()},
        tokens={vid: o.tokens for vid, o in outputs.items()},
    )


@dataclass(frozen=True)
class Provenance:
    spec_best: str
    plan_best: str

@dataclass(frozen=True)
class ChainResult:
    stages: list[StageResult]
    provenance: Provenance

def _salt(frame: str, plants: list[PlantedError]) -> str:
    return inject(frame, plants)[0] if plants else frame

def run_spec_plan_chain(*, issue_text, scoped_slice, variants, runner, co,
                        claim_counts, plants, judges, relevance, prompt_fn) -> ChainResult:
    spec_prompt = prompt_fn("spec", issue_text=_salt(issue_text, plants), scoped_slice=scoped_slice)
    spec = run_stage(stage="spec", variants=variants, runner=runner, co=co,
                     prompt=spec_prompt, repo_root=str(getattr(co, "root", ".")),
                     claim_counts=claim_counts, plants=plants, judges=judges,
                     relevance=relevance)
    plan_prompt = prompt_fn("plan", issue_text=issue_text, scoped_slice=scoped_slice,
                            upstream=_salt(spec.cleaned_best_text, plants))
    plan = run_stage(stage="plan", variants=variants, runner=runner, co=co,
                     prompt=plan_prompt, repo_root=str(getattr(co, "root", ".")),
                     claim_counts=claim_counts, plants=plants, judges=judges,
                     relevance=relevance)
    return ChainResult(stages=[spec, plan],
                       provenance=Provenance(spec.best_variant_id, plan.best_variant_id))
