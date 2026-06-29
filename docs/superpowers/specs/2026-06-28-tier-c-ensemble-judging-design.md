# Tier-C Part-C — Ensemble Judging + Head-to-Head Spec Quality

**Date:** 2026-06-28
**Status:** design (coverage metric deferred)

## Problem

The Part-C oracle currently scores citation **precision** with a *single* sonnet
relevance judge, one `YES`/`NO` per citation. Confirmation runs on prometheus
proved this is **noise-dominated**: re-scoring the *identical* off-arm citations
gave precision **0.364 three times and 0.545 once** — two citations flipped
`NO→YES` purely from judge nondeterminism (±0.18 on identical input). The
per-cell Δprecision swung +0.199 / +0.182 / −0.188 across three on-arm runs, so a
~0.2 effect is indistinguishable from noise. We also only measure one narrow
thing (citation relevance) and capture no *reason* for any verdict.

## Goals

1. **Cut judge noise** on the relevance metric with a small ensemble.
2. **Capture reasons** for every verdict (auditability + understanding *why*).
3. **Add a head-to-head spec-quality** judgment (a second, complementary signal).
4. Keep it **cheap and rescore-able** — validate on saved arms with no re-runs.

Non-goal (deferred): **fix-location coverage** (recall vs the actual fix). The
gold-source decision (real merged fix commit vs opus-derived expected set) is
parked; the design leaves a clean seam for it.

## Design

### 1. Ensemble judge layer (shared by all LLM judges)

**Output contract.** Every judge prompt instructs the model to **start its reply
with the verdict token**, then give a one-sentence reason:
- relevance: `YES` or `NO`
- head-to-head: `A`, `B`, or `TIE`

**Parsing.** Regex the leading token (case-insensitive, anchored):
`^\s*(YES|NO)\b` / `^\s*(A|B|TIE)\b`. The full reply text is kept as the reason.
A reply that does not start with a valid token is treated conservatively
(relevance → `NO`; head-to-head → `TIE`) **and flagged** `unparsed: true`.

**Escalation.** Run **2 sonnet** judges independently on the same prompt:
- agree → that verdict (no opus call)
- disagree → **1 opus** tiebreaker → opus's verdict is final

**Data model.**
```python
@dataclass(frozen=True)
class EnsembleVerdict:
    verdict: str            # "YES"/"NO"  or  "A"/"B"/"TIE"
    escalated: bool         # were the two sonnet judges split?
    votes: list[dict]       # [{model, verdict, reason, unparsed}, ...] sonnet,sonnet,(opus)
```
`ensemble(ask, prompt, choices, *, sonnet="sonnet-4.6", opus="opus-4.8")` returns
an `EnsembleVerdict`. `ask(model, prompt)` is the existing `live_ask` seam (already
MCP/tool-isolated), injectable for tests.

Cost: 2 sonnet per item, +1 opus only on disagreement. Sonnet ≈ 9s/call; rescore
makes re-running free (no live arms).

### 2. Metric 1 — relevance precision via the ensemble

`LlmRelevanceJudge.is_relevant(cite, issue, code)` switches from a single
`ask(...)` to `ensemble(...)` over the *same* relevance prompt (now YES/NO-first +
reason). `verdict == "YES"` → relevant. Precision/hallucination math in
`investigator.py` is unchanged — only the relevance source gets quieter.

Per-cite judge artifacts (`*.judge.jsonl`) gain: `votes` (each model's
verdict+reason), `escalated`, `final` — replacing the single `prompt`/`response`.

### 3. Metric 2 — head-to-head spec quality

A new `SpecQualityJudge`. Inputs: the issue, off-spec text, on-spec text. It:
- **anonymizes** both specs (strip any tool names — the on-spec is already told
  not to name tools, but strip defensively) and labels them `A`/`B`,
- **randomizes A/B assignment deterministically** from
  `sha256(off_text + on_text)` parity, so position bias can't favor an arm and
  **rescores reproduce** the same assignment (no `random`),
- asks: *"Which spec better identifies the root cause and a correct, complete
  fix? Answer A, B, or TIE, then one sentence why."*
- runs through the **same ensemble** (2 sonnet → opus tiebreaker),
- maps the winning label back to `off` / `on` / `tie`.

Output on the cell: `head_to_head = {winner: "off"|"on"|"tie", escalated, votes}`.

This is **per-cell** (one comparison), not per-cite.

### 4. PartCCell additions

```
head_to_head: dict           # {winner, escalated, votes}
# off_verdicts / on_verdicts entries gain: votes[], escalated, final
```
`render_partc` prints the head-to-head winner + escalation, and precision as now.

### 5. Coverage — deferred seam

No code now. When resumed, coverage is a **mechanical** set-overlap of cited
locations vs a gold fix-location set (no LLM, no judge noise) — the only open
decision is the gold source (real merged fix commit vs opus-derived). The cell
will gain `coverage = {gold_n, hit_n, recall, source}`.

## Validation plan

1. Unit: ensemble agreement/escalation/parse paths (fake `ask`); YES/NO-first and
   A/B/TIE parsing; deterministic head-to-head anonymization/assignment.
2. **Rescore the saved prometheus v1–v4** with the ensemble (no arm re-runs):
   - the identical off-arm citations should now score the **same** precision every
     run (the 0.364↔0.545 swing should collapse, or escalate-to-opus should make
     the contested cites consistent);
   - record per-cell head-to-head winners + reasons.
3. Report residual variance: is a ±0.1 effect now readable?

## Cost / risk

- More judge calls (2–3× per cite + 2–3 per cell), but sonnet-cheap and free to
  re-run via rescore. No arm re-runs in validation.
- Risk: 2 sonnet judges may *agree on the wrong answer* (correlated error) — opus
  only adjudicates disagreements, not agreed-but-wrong. Accepted; head-to-head +
  reasons give a cross-check. Revisit if reasons reveal systematic sonnet bias.

## Out of scope

- Fix-location coverage implementation (seam only).
- Changing the arms, the steer, prism, or the corpus.
- Replacing the judge model globally (rank/condition judges in the full chain are
  untouched unless they share the ensemble helper).
