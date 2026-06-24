# Tier-C Value Measurement — Design (2026-06-23, rev-2)

**Status:** design-of-record, **rev-3** (brainstormed + owner-steered; **two** codex gpt-5.5 xhigh methodology
reviews folded — rev-1 REWORK → 15 findings; rev-2 SHIP-WITH-FIXES → 8 tightenings, all incorporated).
**Companion to** `docs/prism-measurement-strategy-2026-06-23.md` (branch `prism-resolution-gap-analysis`,
`6e8818b`) — the concrete realization of that doc's **Tier C**, extended to the full **6-role SDLC pipeline**.
**Sits beside** `eval/tier_a/` (edge-level P/R, built) and the held Tier-B flow harness.

> **What this is and isn't (codex F2/F9):** a **value spike to gate further investment** — "does prism help
> enough, by role and language, to justify more prism work" — **not** academic causal inference. It reports a
> **directional** signal from a small pilot; per-cell statistical claims wait for the multi-seed scale-up.
> Clean per-stage *causal isolation* is the **deferred ablation** (§9), appropriate later when we optimize
> usage and JS/Python precision/recall are solid.

---

## 1. The value question
Does giving an LLM coding agent **prism** (nav MCP + CLI) make it **better at real software work**, and for
*which role* and *which language*? Tier A proved the nav edges are accurate; it did not prove that accuracy
becomes better end-task output. **Reported per role × per language — never one averaged number.**

## 2. Oracle — real OPEN issues, leakage-controlled
- **Why open/unsolved issues:** the *solution does not exist yet*, so no model trained on it; and real
  multi-repo/multi-language issues escape the single-Rust-repo false signal the strategy doc warns about.
- **Leakage control (codex F10):** the dangerous leak — the *fix* — can't be in training (issue is open). The
  issue *text* being known affects both arms equally and doesn't favor prism. We add only a **cold leakage
  probe** (ask each model, no repo, "do you already know the fix for this issue?") and **prefer
  recently-filed issues**; drop any issue a model can reproduce a fix for. Not "leakage-free" — leakage-bounded.

## 3. Arms — stage-specific 2×2, family axis, **citation parity**
Each stage runs **4 variants = 2 models × {prism OFF, prism ON}**, model pair stage-specific:

| Stage(s) | Model A (Anthropic) | Model B (OpenAI) | Harness |
|---|---|---|---|
| **Spec / Plan / Review** | **Opus 4.8** | **gpt-5.5** | Claude Code / codex |
| **Develop** | **Sonnet 4.6** | **gpt-5.3-spark** | Claude Code / codex |

- **Citation parity (codex F4/F5/F6 — the key fix):** **all** arms, prism-ON and prism-OFF, are instructed to
  **cite the file/line/function** behind every claim. We do **not** strip prism's citations — stripping would
  erase the very thing prism contributes. Parity removes the "only ON cites" detectability tell and converts
  the question from citation *presence* to citation **accuracy** — which *is* prism's value. If judges prefer
  well-cited output and prism makes those citations *correct*, that is value, correctly attributed.
- **Isolation:** both prism conditions keep the agent's normal tools (read/grep/bash/LSP); treatment adds
  **only** prism. Identical prompts/harness within a stage.
- **Family** (Anthropic: Opus/Sonnet; OpenAI: gpt-5.5/spark) is a first-class axis (judge-bias, §6b).
- **Harness confound (codex F12):** Claude Code vs codex differ in built-ins/ergonomics. This confounds the
  **cross-model** delta (§7), not the **prism** delta (on−off within one model+harness is clean). Reported as
  a factor, not hidden.
- **Freshness:** prism pre-warmed at the pinned commit; the develop prism-ON arm uses **fresh CLI/freshness
  probe**, not the frozen MCP snapshot (strategy doc §7).

## 4. Corpus — open issues, Goldilocks selection
- **Languages:** Rust, Go, Python, JS/TS. **Count:** 1–2 open issues/language (~5–8) for the pilot.
- **Selection rubric (all must hold) — the "Goldilocks zone":**
  1. **Not a one-liner / not one-shot** — genuinely needs a spec and a plan before code.
  2. **Spans a few files** — exercises navigation (where prism can plausibly help); a single-file edit can't
     show nav value.
  3. **Tractable** — completable in one agent session. **Multi-slice issues are allowed: scope the task to
     the first slice** and state that scope.
  4. **Clear "done" signal** — failing repro, acceptance criteria, or maintainer-stated expected behavior.
  5. **Buildable/testable** repo at the pinned commit (needed for the develop objective anchor).
- **Sampling honesty (codex F13):** sample from the *target* repo distribution — do **not** pre-exclude repos
  prism analyzes poorly. A repo prism can't analyze (or analyzes with high unresolved %) is a **prism
  outcome**, recorded as such, not an exclusion — otherwise we inflate dynamic-stack value.
- **Pinning:** record `(repo, commit SHA, issue URL, issue text snapshot, scoped-slice statement)`. Rubric is
  frozen **before** any arm runs; no issue swapped after seeing results.

## 5. The 4-stage chain — "same frame per stage" (not a tournament)
One chain **per issue**. At each stage all 4 variants run on a **common input**; outputs are scored; the
**cleaned best output is carried forward** as the next stage's common input. Purpose (codex F2): **ensure
every variant starts each stage from the identical frame**, so the per-stage prism comparison is
apples-to-apples — *not* a competition. Because all 4 variants get the *same* frame, the per-stage prism delta
is **valid conditional on that carried frame** (codex new-6) — **provenance is logged**: if an ON arm produced
the frame, the next stage's OFF arms inherit prism-derived context, which can distort that stage's delta, so
deltas are reported *conditional on the frame*, never as the unconditional isolate. That provenance-free
isolation is the **deferred ablation** (§9); end-to-end path-effects are out of scope here.

| Stage | Common input | Each variant produces | Objective anchors | Carry forward |
|---|---|---|---|---|
| **1. Spec** (architect/spec writer) | issue (+ planted refs) | a spec (cited) | investigator; planted-ref catch-rate | cleaned best spec |
| **2. Plan** (plan writer) | best spec (+ planted refs) | a plan (cited) | investigator; planted-ref catch-rate | cleaned best plan |
| **3. Develop** (developer) | best plan | a diff (cited) | **build+test+lint+repro**; investigator | cleaned best diff |
| **4. Review** (code reviewer) | best diff **+ seeded bugs** | a review | **seeded-bug recall**; investigator | — |

- **6-role mapping:** producers = the 4 variants; **reviewers = the dual judges + investigator** judging each
  stage (judging the spec *is* spec-review; the plan *is* plan-review); stage 4 is the explicit code-reviewer.
- **Carry-forward is the *cleaned* best**, gated by a **sanitation step** (codex new-5): before a frame
  advances, the investigator runs a **zero-survival check** — no planted reference remains, *and* no planted
  falsehood survives as a corrected-looking-but-still-misleading claim; a clean-frame diff is recorded and the
  stage is **re-run if planted residue remains**. This keeps the probe from corrupting the chain.
- **Ties (codex F3):** broken by a **pre-registered random** seed (not "carry OFF," which would systematically
  disadvantage prism downstream); tied cells flagged non-identifiable.

## 6. Scoring — objective backbone + secondary judges
**Objective channels are primary** (the spike's backbone); the LLM judges are a secondary quality read. Judges
**never** see the objective metrics, the family tag, or the prism condition.

### 6a. Objective (primary) — investigator + planted-error probe + build/test
- **The investigator (codex F1/F7 — the independent oracle):** a separate verification pass over each
  variant's output. **Mechanical** existence-check via **neutral repo primitives — compiler/parser/grep, NOT
  prism** (codex new-3 — using prism would make the oracle favor exactly the references prism can resolve):
  does each cited file/line/symbol exist at the pinned commit? Plus a **secondary, audit-sampled + adjudicated
  LLM relevance/validity** judgment (not sole authority), **blind to arm/model**. It checks against the
  **repo**, not any arm's best-diff — **independent and non-circular**. Output per variant: **citation
  precision** (cited refs that are real *and* relevant) **and citation recall / claim-coverage** — every
  substantive repo claim must be linked, so *missing* citations count against the arm and **under-citing
  cannot game the metric** (codex new-2); plus hallucinated-reference count (normalized by citation volume)
  and claim-validity rate. Primary prism-value signal: prism-OFF cites from grep/memory and hallucinates /
  under-grounds more; prism-ON cites from real nav.
- **Planted-error sensitivity probe (per stage):** each stage's input frame is salted with a known, balanced
  taxonomy of errors — **invalid file paths, nonexistent functions, wrong variable/symbol references,
  false claims about the code** (for review: **intentional bugs**, balanced taxonomy, seeded **blind to arm
  outputs**, including **non-graph** bugs — codex F15). Score per variant: **planted-error recall** (did it
  flag/correct/avoid the planted error). This is a **diagnostic / grounding-sensitivity** metric — objective,
  independent, every stage — **not a standalone value metric** (codex new-4): catching planted refs can
  co-occur with a still-wrong spec/fix, so it is reported **alongside real task correctness** (build/test,
  citation precision/recall) and we **check whether it correlates with downstream success**. It also
  **calibrates the investigator** (the oracle must catch the planted errors too).
- **Develop only:** diff applies; **build + test + lint pass**; issue repro resolves (the primary develop
  metric — codex F8).
- **Effort/efficiency:** file reads, wrong-file reads, tool-calls, tokens, wall-time. **prism usage logged**
  (codex F14): report **intent-to-treat** (prism available) **and per-protocol** (prism actually used);
  forced-use sensitivity run if ON arms underuse it.

### 6b. Subjective (secondary) — dual judges, full rankings, measured bias
- **Judges = one per family:** **Opus 4.8** + **gpt-5.5**, each returning a **full 1–4 ranking** (the
  instrument that makes self-preference visible). Same-as-arms is fine by design (bias is measured).
- **Blind + style-neutral:** variants arm-anonymized + order-shuffled; **citation parity** (§3) removes the
  citation-*presence* tell; judges are **instructed to ignore citation polish/volume/formatting** — the
  investigator owns citation accuracy, judges grade substance (codex new-1). A **pre-registered detectability
  test** runs (pooled across issues; permutation test at a fixed threshold — codex new-7): **if detectability
  > chance, the judge-based prism delta is treated as INVALID** (not merely downgraded) and the prism signal
  rests entirely on the objective channel (§6a).
- **Rubric per stage:** right APIs/files/invariants/risks/tests (spec); right files/order/test-plan (plan);
  diff correctness/scope (develop); real-issue find-rate, low noise (review).
- **Consensus + adjudication:** combine the two rankings (rank-average/Borda) → consensus; **you adjudicate**
  disagreements (κ-style). **Family bias is reported** (own- vs other-family rank inflation = the
  "codex-picks-codex / claude-picks-claude" number); the consensus cancels symmetric bias; residual is a
  caveat band on the cross-model delta (§7). Asymmetric/shared bias is **not** assumed away — it's why the
  objective channel is primary.

## 7. Reporting
Per **(stage × language)**:
- **prism effect (primary):** prism-ON − prism-OFF *within each model+harness*, **reported conditional on the
  carried frame** (provenance logged — §5; codex new-6) — led by the objective channel (citation
  precision/recall, build/test; planted-recall as a diagnostic) with the judge read as support **only if the
  detectability test passes** (§6b). Least exposed to confounds (same model, same harness, parity-cited,
  objective-anchored), but *conditional*, not the unconditional isolate the deferred ablation would give.
- **model effect (caveated):** the stage's cross-model delta (Opus 4.8 − gpt-5.5; Sonnet 4.6 − gpt-5.3-spark)
  — reported **with the family-bias caveat band AND the harness-confound flag** (§3, §6b). **No cross-stage
  prism comparison** (codex F11): stage-specific pairs make "prism helps spec more than develop" uninterpretable.
- **evidence-availability split:** bucket by "prism had high-confidence evidence" vs "prism mostly
  unresolved/warned" — separates product value from language-coverage debt.
- **Honesty (codex F9):** with ~5–8 issues / 1 seed, cells are **directional only**; the deliverable is a
  *populated harness + a direction*, not powered per-cell effects. Powered claims need the multi-seed scale-up.

## 8. Confounders — how each is controlled
- **Blinding / detectability:** **citation parity** (not stripping) + detectability check + objective channel
  primary (codex F4/5/6).
- **No independent oracle / circularity:** **investigator** checks vs the repo, not vs arm outputs; planted-
  error recall; build/test (codex F1/F7).
- **Judge self-preference:** *measured, not eliminated* — one judge/family, full rankings, consensus cancels
  symmetric bias, residual reported; objective channel is primary anyway.
- **Selection/coverage bias:** target-distribution sampling; analyze-failures counted as prism outcomes (F13).
- **Availability ≠ usage:** usage logged; ITT + per-protocol; forced-use sensitivity (F14).
- **Harness:** confound on the cross-model delta only; prism delta within model+harness is clean; reported as
  a factor (F12). (Optional same-model cross-harness control later.)
- **Staleness / token bloat:** fresh CLI in develop; efficiency reported as a real cost, not hidden; a
  *negative* prism result attributable to known repo-map/callers token bloat is flagged "non-final."
- **Ties:** pre-registered random (F3).

## 9. Scope & non-goals
- **In scope:** 2×2 × ~5–8 open issues × 4-stage chain; objective (investigator + planted-error + build/test) +
  secondary judges; per stage×language directional report.
- **Deferred (deliberate):** **ablation** (fixed-input causal isolation) — later, when optimizing usage with
  solid JS/Python P/R; **Tier-B flow/taint value** (held Plan B); **multi-seed / powered N**; **C/C++** (prism
  doesn't complete).

## 10. Cost & key build risks
- **Cheap:** spec/plan stages; the investigator's mechanical existence-check; effort metrics.
- **Most expensive (codex-confirmed):** the **develop** stage — a **buildable+testable sandbox per repo at the
  pinned commit** (per-language build/test/lint), plus the **planted-error injector** (per-stage salting +
  cleaned carry-forward) and the **seeded-bug** taxonomy.
- **Harness pieces:** (1) arm runner (Claude Code / codex driver, prism on/off toggle, citation-parity prompt
  templates, transcript+token+tool-call capture); (2) pinned-checkout + per-repo build/test sandbox;
  (3) **investigator** (mechanical symbol-existence + LLM relevance/validity); (4) **planted-error injector +
  scorer**; (5) dual-judge runner (blind, full-ranking, detectability check) + adjudication capture;
  (6) report generator (per stage×language deltas, family-bias band, evidence split, ITT/per-protocol).
- **De-risking option:** ship **spec+plan stages first** (no build sandboxes; investigator + planted-error +
  judges fully exercise the pipeline), then add **develop+review**.

## 11. Worked example (one issue)
`ripgrep` (Rust), an open multi-file issue scoped to its first slice, pinned at SHA `X`:
1. **Spec:** 4 variants = {Opus 4.8, gpt-5.5} × {prism on/off}, **all required to cite file/line/fn**, write a
   spec from the issue **salted with 2 invalid refs** (a nonexistent fn, a wrong file path). Investigator
   checks every citation vs the repo (accuracy, hallucinations); planted-ref **catch-rate** scored; dual
   judges rank blind (+ detectability check); consensus + your adjudication → **cleaned** best spec.
2. **Plan:** same 4 variants plan from the best spec (re-salted) → cleaned best plan.
3. **Develop:** 4 variants = {Sonnet 4.6, gpt-5.3-spark} × {prism on/off} implement the best plan; harness runs
   `cargo build && cargo test` at SHA `X`; investigator audits diff citations; cleaned best diff.
4. **Review:** {Opus 4.8, gpt-5.5} × {prism on/off} review the best diff with **N seeded bugs** (balanced,
   blind); **seeded-bug recall** + investigator + judged finding quality.
→ per-stage **prism-ON−OFF** (primary, objective-led) and the caveated cross-model delta for this Rust issue.

## 12. Success criteria + investment gate
**Harness success:** for ≥5 issues across ≥4 languages, a populated per-stage×language table with objective
prism deltas (citation precision/recall, build/test; planted-recall diagnostic), the family-bias band,
ITT/per-protocol usage, the detectability-test outcome, and adjudicated rankings — a **repeatable** way to
answer "where does prism help, by role and language."

**Go/no-go for further prism investment (codex new-8 — the point of the spike).** Pre-registered, decided
**per role × language** (directional for the pilot, confirmed at multi-seed):
- **GO (invest)** where prism-ON shows a **material objective lift** — citation precision/recall and
  planted-error recall up, and (develop) build/test pass-rate up — **net of cost** (tokens/latency within an
  agreed budget) **and** with an acceptable prism **analyze-failure rate** on that language's repos.
- **NO-GO / fix-first** where lift is flat/negative, OR is attributable to token-bloat (the §8 "non-final"
  flag), OR where prism's **analyze-failure / unresolved %** is high — a coverage-debt signal: fix maturity
  before claiming value (the strategy doc's dynamic-stack thesis).
- **Per-language is decisive:** a GO on Rust/Go with NO-GO on Python/JS is the *expected* shape and must **not**
  be averaged into a single verdict.

A directional finding (e.g. "prism lifts spec/plan citation-accuracy + planted-error catch on Python/JS, flat
on Rust") is the first deliverable; statistical strength is the multi-seed follow-up.
