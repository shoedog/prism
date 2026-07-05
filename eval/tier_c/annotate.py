"""D3 — fact-annotated head-to-head (the direct fix for the pilot's judge miss: a
citation-blind head-to-head rewarded hallucination-backed narration; annotating each
spec with what D0/D1 already proved about its citations lets the SAME judge see that).

Machine-annotates each arm's text with inline tags derived from ALREADY-COMPUTED
verdicts — D0's investigator hallucination flag (CitationVerdict.is_hallucination) and
D1's CONTRADICTED claims (validity.ValidityVerdict) — purely mechanical string surgery,
run IDENTICALLY on both arms via the identical procedure, so the annotation itself
carries no prism tell. The SAME blind SpecQualityJudge ensemble then compares the
annotated pair (see _LivePartCComps.head_to_head_annotated in cli.py) — kept a fully
separate call so the existing blind head_to_head() stays byte-identical (spec cardinal
constraint).

Annotation DENSITY could still be a detectable signal (an arm with more citations gets
more opportunities for a tag) — so run_annotated_detectability() below MUST be run,
pooled across MULTIPLE Part-C cells, before trusting the annotated head-to-head; mark it
INVALID if the pooled test fires (spec D3 / judges.detectable()).
"""
from __future__ import annotations

from .citations import iter_citation_occurrences
from .claims import sentence_spans

_HALLUC_TAG = " [CITED LOCATION DOES NOT EXIST]"
_CONTRA_TAG = " [CODE CONTRADICTS THIS CLAIM]"


def hallucinated_keys(verdicts) -> set[tuple]:
    """(file, line, symbol) keys for citations the D0 investigator flagged as
    hallucinated (CitationVerdict.is_hallucination) — from InvestigatorReport.verdicts."""
    return {(v.cite.file, v.cite.line, v.cite.symbol) for v in verdicts if v.is_hallucination}


def contradicted_keys(validity_verdict_dicts: list[dict] | None) -> set[tuple]:
    """(file, line, symbol, sentence) keys for D1 CONTRADICTED claims, from the
    JSON-safe verdict dicts stored on PartCCell.validity_off / validity_on."""
    return {
        (d["file"], d["line"], d["symbol"], d["sentence"])
        for d in (validity_verdict_dicts or [])
        if d.get("verdict") == "CONTRADICTED"
    }


def annotate_arm_text(text: str, *, hallucinated: set[tuple], contradicted: set[tuple]) -> str:
    """Insert inline tags right after each citation occurrence whose key is flagged.
    Mechanical + deterministic; the SAME procedure runs on both arms with whatever keys
    THAT arm's own verdicts produced (symmetric by construction, not itself a tell)."""
    if not text or (not hallucinated and not contradicted):
        return text
    spans = sentence_spans(text)
    inserts: list[tuple[int, str]] = []
    for start, end, cite in iter_citation_occurrences(text):
        sentence = next((s for (a, b, s) in spans if a <= start < b), "").strip()
        tag = ""
        if (cite.file, cite.line, cite.symbol) in hallucinated:
            tag += _HALLUC_TAG
        if (cite.file, cite.line, cite.symbol, sentence) in contradicted:
            tag += _CONTRA_TAG
        if tag:
            inserts.append((end, tag))
    out = text
    for pos, tag in sorted(inserts, key=lambda x: -x[0]):
        out = out[:pos] + tag + out[pos:]
    return out


def run_annotated_detectability(cells, guesser, alpha: float = 0.05):
    """Pool annotated off/on texts across MULTIPLE Part-C cells and re-run the
    detectability guesser (spec D3). detect.py's own docstring proves a single cell's 2
    outputs cannot reach significance (max separation p=0.0625 > 0.05) — callers MUST
    pass several cells (e.g. the full pilot set) for this to be meaningful. Returns a
    detect.Detectability; mark the annotated head-to-head INVALID when .detectable."""
    from .detect import run_detectability
    from .model import ArmOutput, Variant

    outputs = []
    for c in cells:
        outputs.append(ArmOutput(Variant(c.model, False), c.annotated_off_text, [], 0, 0, 0.0, False))
        outputs.append(ArmOutput(Variant(c.model, True), c.annotated_on_text, [], 0, 0, 0.0, True))
    return run_detectability(outputs, guesser, alpha=alpha)
