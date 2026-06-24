# eval/tier_c/chain.py
"""Stage orchestration + spec->plan chaining (spec §5). run_stage: 4 variants ->
investigator + planted catch + judge consensus -> sanitized cleaned-best carried
forward. Per-stage prism delta is conditional on the carried frame (provenance logged)."""
from __future__ import annotations
import re
from dataclasses import dataclass, field
from .model import Variant
from .investigator import score_citations, InvestigatorReport
from .planted import PlantedError, score_catch, sanitation_ok, PlantedReport

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

def run_stage(*, stage, variants, runner, co, prompt, repo_root, claim_counts,
              plants, judges, relevance) -> StageResult:
    outputs = {v.id: runner.run(v, stage, prompt, repo_root) for v in variants}
    investigator = {
        vid: score_citations(co, o.citations, claim_count=claim_counts[vid],
                             relevance=relevance)
        for vid, o in outputs.items()
    }
    planted = {vid: score_catch(o.text, plants) for vid, o in outputs.items()}
    # blind candidates: anonymized id -> text
    candidates = {vid: o.text for vid, o in outputs.items()}
    from .judges import borda_consensus
    rankings = {fam: j.rank(stage, "rubric", candidates) for fam, j in judges.items()}
    consensus = borda_consensus(rankings)
    best = consensus[0]
    cleaned = _strip_plants(outputs[best].text, plants)
    assert sanitation_ok(cleaned, plants), "sanitation gate failed (codex new-5)"
    return StageResult(
        stage=stage, investigator=investigator, planted=planted, consensus=consensus,
        best_variant_id=best, cleaned_best_text=cleaned,
        used_prism={vid: o.used_prism for vid, o in outputs.items()},
        tokens={vid: o.tokens for vid, o in outputs.items()},
    )
