# Tier-C Part C — Prism Value via Steered Prism-On vs Recovered Prism-Off (Light Design)

> **For agentic workers:** design-of-record. Next step is `superpowers:writing-plans`. Pairs with the
> Tier-C design-of-record `2026-06-23-tier-c-value-measurement-design.md` (rev-3), the oracle-fix spec
> `2026-06-24-tier-c-investigator-relevance-oracle-fix-design.md`, and the handoff
> `2026-06-24-tier-c-value-measurement-handoff.md`.

**Goal:** Measure prism's deployment value with the lightest *valid* A/B — one fresh **steered prism-on**
arm per cell, scored against the **re-scored recovered prism-off** baseline — after fixing the two defects
that voided the `full-2026-06-24` run (defect #1 prism-never-invoked, defect #2 code-blind relevance oracle).

**Why this shape (settled in dialogue):** The old prism-*on* arms are dead (0 nav calls → behaviorally ==
prism-off), so prism-on must be fresh regardless. But the old prism-*off* arms are real prism-off behavior and
their spec/plan text was recovered — so we **reuse them as the baseline** (re-scored), spending only on the
prism-on half. We do **not** run a steered prism-off: directing a model to use a tool it lacks makes it flail/
hallucinate, producing an unrealistic strawman baseline that would *inflate* prism. The faithful counterfactual
to "agent with prism, told to use it" is "agent without prism, doing its normal grep/Read thing" = the
recovered unsteered prism-off.

**What this measures — and the one caveat:** This is a **deployment** contrast: `(prism + use-prism steer)`
vs `status quo (no prism)`. The steer is *part of the treatment* (prism is worthless un-invoked), not a
confound. **Caveat to state in the report:** the lift is attributable to the *bundle*, not prism-the-tool in
isolation; this design cannot decompose "prism" from "being told to be rigorous." That is fine for a
ship/no-ship gate. (A single capability-steered-prism-off issue would recover the decomposition — out of scope.)

**Two separate numbers — do NOT collapse them into one multiply:**
- **bundle-lift** = the value *when prism is administered* (measured here at ~100% administration, because the
  arm is steered).
- **administration rate** (from the adoption eval) = **72%** skill-only/organic, ~**100%** if the deployment
  also injects a per-task steer.
Realized value then depends on *what you ship*: ship-the-bundle (skill + per-task steer) → ~100% administration
→ ≈ full bundle-lift; ship-skill-only → 72% administration, and per-arm value may be *lighter* than the steered
bundle-lift (organic prism use does fewer nav calls — Arm A vs Arm C). So `bundle-lift × 0.72` is at best a
rough floor for a skill-only deployment, **reported as a caveated estimate, not the headline.**

---

## Design (light)

**Cells:** `model ∈ {opus-4.8, gpt-5.5}` × `issue ∈ {ruff #26287, prometheus #18896, pydantic #13300,
excalidraw #11479}` × `stage ∈ {spec, plan}` = **16 cells** (= 16 (lang × stage × model) combinations; each
issue is a distinct language).

**New arms (the only spend):** exactly **one prism-on-steered arm per cell = 16 fresh arms.**
- **No** LSP variants (inert for spec/plan in the dead run — no shim ever fired; LSP only bites Phase-2
  develop/review).
- **No** fresh prism-off arms, **no** prism-off-steered arm, **no** 4-variant matrix.

**Baseline (reused, 0 spend):** the recovered base arm per cell —
`eval/tier_c/runs/full-2026-06-24/recovered/<model>/<repo>/<stage>/<model>.md` (no suffix = prism-off +
lsp-off). Re-scored with the **fixed** oracle (below). 16 reused baselines.

**The steer (prism-on prompt):** the existing parity spec/plan prompt (`prompts.py`) **+** the Arm-C directive,
verbatim in spirit: *"FIRST use the Skill tool to load both your planning skill and the prism-code-navigation
skill, THEN use prism's nav tools (nav_callers / nav_callees / nav_repo_map / nav_nodes_at) to locate and
ground every file:line you cite."* A tool-specific steer is valid here because prism **exists** in this arm.
Arm-C evidence: this drives ~100% invocation (21/21) and 4–15 nav calls/arm.

---

## Prerequisite fixes (fold into this work)

**1. Relevance-oracle fix (defect #2) — gates everything.** Per the drafted oracle-fix spec: thread a cited
**code window** into the judge. Today `LlmRelevanceJudge.is_relevant(cite, issue_text)` (`judges_live.py:33`)
asks "is `file:line` (symbol None) relevant?" *without showing the code* → always non-YES → `relevant=False` →
`is_valid=0` → `precision≡0`. Fix: add `Checkout.read_window(file, line, ±ctx)`, change the seam to
`is_relevant(cite, issue_text, code)` (`investigator.py:45` call site), put the code in the prompt. Re-scores
**both** the new prism-on arms *and* the recovered prism-off baseline through the identical oracle.

**2. Real-invocation gate (defect #1) — replace the heuristic.** Drop
`used_prism = variant.prism and tool_calls > 0` (`arm_runner.py:85,109,120`). Detect **actual**
`mcp__prism__nav_*` calls:
- **codex:** already exposed — `parse_codex_stream` yields `mcp_tool_call` with `server=="prism"`.
- **claude:** `-p --output-format json` returns only a single result object (no per-tool stream). Run the arm
  with `--output-format stream-json` and parse `tool_use` events for `mcp__prism__*` (reuse the adoption
  eval's `parse_stream_json`). 
- **Policy:** any prism-on arm with **0 real prism calls is discarded** (re-run once) — it is not scored as
  prism-on. This is the gate that makes "prism-on" mean prism was actually administered.

**3. SUT immutability — observed leak.** `deny:[Write,Edit]` does **not** stop a Bash-write: an arm added a
docstring to `eval/tier_c/chain.py` during the adoption work. For a value A/B the per-issue checkout must be
identical across arms. **Fix: `git reset --hard && git clean -fd` the checkout after every arm** (simplest,
robust; the checkout is a throwaway worktree). Optionally also deny Bash-write in the sandbox.

---

## Scoring, reporting, blinding

**Co-primary objective** (the dead run proved precision-only is degenerate): (a) citation **precision/recall**
via the fixed oracle (`investigator.py`); (b) the **prism-blind rank judge** (the one signal that survived —
it put `opus+prism` #1 on plan/ts). Report both per `(stage × language)`.

**Report:** per `(stage × language)`: prism-on-steered vs prism-off-base — precision/recall delta + rank-judge
consensus = the **bundle-lift** (headline); GO/NO-GO. Alongside (not multiplied in): the **administration
rate** (72% skill-only / ~100% steered) from the adoption eval, with `bundle-lift × 0.72` shown only as a
caveated skill-only floor. Reuse `report.py` (collapse `Cell2x2` → a single on-vs-base contrast; drop the
LSP/interaction contrasts).

**Detectability (blinding check, already built):** keep `detect.py` pooled binomial over the rank-judge's
condition guesses — confirms the judge can't tell prism-on from prism-off by style (so the rank delta is valid,
not a tell). Note: prism-on arms now contain real nav-tool traces, but judges score **final spec/plan text**,
not transcripts, so blinding still applies.

**Environment (reuse Phase-1c, unchanged):** prism MCP `--repo` = per-issue checkout (`arm_runner.py:38,48`),
`_prism_mcp_bin()` resolver, `cli_model_flag()` mapping, consolidated `prism-code-navigation` skill present in
the prism-on arm.

---

## Out of scope / deferred
- Fresh prism-off arms; prism-off-steered; the prism-vs-steer **decomposition** control.
- LSP arms (inert for spec/plan); develop + review stages + build sandboxes (Phase 2).
- Scaling beyond 4 issues (2nd-per-language picks held: tokio #8182, prometheus #18972, mypy #21583,
  excalidraw #11313).

## Risks
- **Per-arm timeout:** steered spec generation can exceed the 600s `_TIMEOUT` (crashed Arm C). Bump the
  per-arm budget (the dead run used 1800s/arm) and isolate arm failures so one timeout doesn't abort the run.
- **Reuse validity** rests on: same SHAs/models/corpus (✓), recovered base genuinely prism-off (✓ 0 nav
  calls), identical oracle applied to both arms (✓). The bundle caveat is documented, not hidden.
- **Adoption multiplier (0.72)** was measured on the adoption eval's `tier_c` micro-repo realistic probes, not
  the real Tier-C issues → it is an estimate; real-issue organic invocation may differ. Report it as such.

## Acceptance
- 16 prism-on-steered arms run; the invocation gate records **real** `mcp__prism__*` calls; 0-call arms
  discarded/re-run.
- 16 recovered prism-off baselines re-scored through the fixed oracle.
- ≥1 cell yields a **non-degenerate** (non-zero precision) score — proving the oracle fix worked (the whole
  point; the dead run was uniformly 0).
- Report renders per `(stage × language)` with bundle-lift + adoption multiplier; detectability = not
  detectable (blinding holds).
- SUT checkout is byte-identical before/after each arm (immutability holds).
