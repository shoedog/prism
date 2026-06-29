# Tier-C Part C — Prism Value via Steered Prism-On vs Recovered Prism-Off (Light Design)

> **For agentic workers:** design-of-record. Next step is `superpowers:writing-plans`. Pairs with the
> Tier-C design-of-record `2026-06-23-tier-c-value-measurement-design.md` (rev-3), the oracle-fix spec
> `2026-06-24-tier-c-investigator-relevance-oracle-fix-design.md`, and the handoff
> `2026-06-24-tier-c-value-measurement-handoff.md`.
>
> **Review status:** codex gpt-5.5 xhigh methodology review = **SHIP-WITH-FIXES** (it validated the
> reuse-baseline + no-fake-steer + bundle-framing decisions as sound). All 9 findings folded — see the
> "Review folded" section at the end for the mapping.

**Goal:** Measure prism's deployment value with the lightest *valid* A/B — one fresh **steered prism-on**
arm per cell, scored against the **re-scored recovered prism-off** baseline — after fixing the two defects
that voided the `full-2026-06-24` run (defect #1 prism-never-invoked, defect #2 code-blind relevance oracle).

**Why this shape (settled in dialogue, codex-validated):** The old prism-*on* arms are dead (0 nav calls →
behaviorally == prism-off), so prism-on must be fresh regardless. But the old prism-*off* arms are real
prism-off behavior and their spec/plan text was recovered — so we **reuse them as the baseline** (re-scored),
spending only on the prism-on half. We do **not** run a steered prism-off: directing a model to use a tool it
lacks makes it flail/hallucinate, producing an unrealistic strawman baseline that would *inflate* prism. The
faithful counterfactual to "agent with prism, told to use it" is "agent without prism, doing its normal
grep/Read thing" = the recovered unsteered prism-off.

**What this measures — and the one caveat:** This is a **deployment** contrast: `(prism + use-prism steer)`
vs `status quo (no prism)`. The steer is *part of the treatment* (prism is worthless un-invoked), not a
confound. **Caveat to state in the report:** the headline lift is attributable to the *bundle*, not
prism-the-tool in isolation. The **GO-only decomposition sentinel** (below) recovers the prism-vs-steer split
for winning cells, so a GO is not blindly "ship a better prompt."

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
excalidraw #11479}` × `stage ∈ {spec, plan}` = **16 cells** (each issue is a distinct language).

**New arms (the only spend):** exactly **one prism-on-steered arm per cell = 16 fresh arms.**
- **No** LSP variants (inert for spec/plan in the dead run — no shim ever fired; LSP only bites Phase-2
  develop/review).
- **No** fresh prism-off arms, **no** prism-off-steered arm, **no** 4-variant matrix (the sentinel below is a
  conditional second pass, not part of the base 16).

**Baseline (reused, 0 spend):** the recovered base arm per cell —
`eval/tier_c/runs/full-2026-06-24/recovered/<model>/<repo>/<stage>/<model>.md` (no suffix = prism-off + lsp-off;
genuinely prism-off — `prism_called=False`, 0 nav calls). **The loader MUST strip the leaking metadata header:**
these files begin with a block (`prism=False`, `prism_called`, `session:` path, `group_size`, …) terminated by
a `---` separator on its own line; score/judge **only the assistant text after the first `---`**. Scoring the
raw file would leak the condition and break blinding + detectability. Add a regression test that the loader
drops everything through the first `---`. Re-scored with the **fixed** oracle (below); 16 reused baselines.

**The steer (prism-on prompt):** the existing parity spec/plan prompt (`prompts.py`) **+** the Arm-C directive,
verbatim in spirit: *"FIRST use the Skill tool to load both your planning skill and the prism-code-navigation
skill, THEN use prism's nav tools (nav_callers / nav_callees / nav_repo_map / nav_nodes_at) to locate and
ground every file:line you cite."* A tool-specific steer is valid here because prism **exists** in this arm.
Arm-C evidence: this drives ~100% invocation (21/21) and 4–15 nav calls/arm. **Plus a no-leak instruction** (see
Scoring/blinding): "do not name the tools you used in the spec/plan."

**Harness (NEW path — not the old 8-variant loop):** the current `tier-c run --live` (`cli.py:135,154`) still
builds the 8-variant 2×2 matrix and reports `prism_at_lsp_on` precision-only — a worker running it would run the
*wrong* experiment. This work adds a **dedicated Part-C runner/rescore path** (e.g. `tier-c run-partc`): load
the 16 cleaned base texts, run only the 16 steered-on arms (+ conditional sentinels), report on-vs-base
precision/recall + rank. The 8-variant loop is not reused.

---

## Decomposition sentinel (conditional, GO-only)

**The threat it closes.** A positive bundle-lift answers "should we deploy the bundle?" — but leaves one
alternative open: maybe the *steer alone* (telling the model to ground every claim by tracing code structure)
produces the lift and prism contributes little. If so, a GO really means "ship a better prompt," not "ship
prism" — a materially different and far cheaper conclusion. We don't want to green-light prism engineering on a
lift the steer alone would deliver.

**Why conditional (GO-only), not always-on.** A decomposition arm on *every* cell doubles the steered spend and
is wasted where there's no bundle-lift (nothing to decompose — if the bundle didn't win, "was it prism or the
steer?" is moot). So the sentinel fires **only on cells whose bundle-lift is positive** (candidate GOs),
bounding extra spend to ≤(#winning cells) (realistically 1–4) and spending it exactly where it matters.

**The sentinel arm.** For each GO cell `(model, issue, stage)`, run ONE extra arm: **capability-steered,
prism-OFF**. The steer here is **capability-level, NOT tool-level** — it names the *task*, never prism:
*"Before writing, ground every file:line by tracing the code structure — who calls the symbols you touch, what
they call, what a change would touch — then cite exact file:line."* It must NOT mention prism or `nav_*` (that
is the strawman codex correctly rejected — directing a model at a tool it lacks makes it flail). The model
satisfies it with grep/Read/its normal tools. Same issue / SHA / model / stage / checkout; re-scored through
the identical fixed oracle.

**Interpretation (per GO cell).**
- `capability-steered-off ≈ steered-prism-on` → the lift was the **steer, not prism** → downgrade to "ship the
  prompt, not prism" for that cell.
- `steered-prism-on >> capability-steered-off > base` → prism adds value **on top of** the steer → **confirmed
  prism GO**. The split: `on − capability-off` = prism-attributable lift; `capability-off − base` =
  steer-attributable lift.
- `capability-steered-off ≈ base` (steer alone does nothing) → prism is doing the work → **strongest prism GO**.

**Cost / placement.** A *second pass*, run only after the base 16 land and bundle-lift is known. The light first
pass is unchanged; the sentinel is pure upside insurance on any win (≤#GO extra arms, ever).

---

## Prerequisite fixes (fold into this work)

**1. Relevance-oracle fix (defect #2) — gates everything.** Today `score_citations` is called WITHOUT issue
context (`chain.py:42` → `issue_text=""` default at `investigator.py:49`) AND the judge never sees the code
(`is_relevant(cite, issue_text)`, `judges_live.py:33`) → "is `file:line` (symbol None) relevant?" → always
non-YES → `relevant=False` → `precision≡0`. The fix needs **both** halves:
- **Thread issue context** into `score_citations` from `chain.py`: the `issue_text`, the scoped slice, and for
  the **plan** stage the upstream **spec** the plan was built from.
- **Thread the cited code**: add `Checkout.read_window(file, line, ±ctx)`, change the seam to
  `is_relevant(cite, issue_text, code)` (`investigator.py:45` call site), put both issue + code in the prompt.
Re-scores **both** the new prism-on arms *and* the recovered prism-off baseline through the identical oracle.

**2. Real-invocation gate (defect #1) — replace the heuristic, record dose.** Drop
`used_prism = variant.prism and tool_calls > 0` (`arm_runner.py:85,109,120`). Detect **actual**
`mcp__prism__nav_*` calls:
- **codex:** already exposed — `parse_codex_stream` yields `mcp_tool_call` with `server=="prism"`.
- **claude:** `-p --output-format json` returns only a single result object (no per-tool stream). Run the arm
  with `--output-format stream-json` and parse `tool_use` events for `mcp__prism__*` (reuse the adoption eval's
  `parse_stream_json`).
- **Record dose** per arm: successful nav-call count, distinct tools used, and error count.
- **Policy:** discard/re-run only a **true 0-call** prism-on arm (prism never administered). Do **not** hard-gate
  on a dose threshold — a single well-placed `nav_callers` can be legitimately sufficient. Instead **flag
  low-dose arms** (e.g. 1 call / all-error) in the report so a diluted contrast is visible, not silently a NO-GO.

**3. SUT + environment immutability per arm.** Two leaks to close:
- **File mutation:** `deny:[Write,Edit]` does not stop a Bash-write (an arm added a docstring to
  `eval/tier_c/chain.py` during the adoption work). **`git reset --hard && git clean -fd` the checkout BEFORE
  and AFTER every arm** (throwaway worktree); optionally also deny Bash-write in the sandbox.
- **prism cache/server:** prism-mcp defaults to a cached nav store unless given `--no-cache`/a run cache dir
  (`prism-mcp.rs:15`, `session.rs:356`). Use a **fresh MCP server per arm** with `--no-cache` (or a run-scoped
  cache dir) so a stale CPG can't survive the `git reset`, and **log the cache mode/hit status** per arm.

---

## Scoring, reporting, blinding

**Co-primary objective** (the dead run proved precision-only is degenerate): (a) citation **precision/recall**
via the fixed oracle (`investigator.py`); (b) the **prism-blind rank judge** (the one signal that survived — it
put `opus+prism` #1 on plan/ts). Report both per `(stage × language)`.

**Report:** per `(stage × language)`: prism-on-steered vs prism-off-base — precision/recall delta + rank-judge
consensus = the **bundle-lift** (headline). **Label cells as directional "pilot signal," show `n=1` issue per
language and exact counts** — at one issue/language a per-language GO/NO-GO over-claims (the design-of-record
calls pilot cells directional only); reserve a true language-level GO/NO-GO for scale-up. Alongside (not
multiplied in): the **administration rate** (72% skill-only / ~100% steered), with `bundle-lift × 0.72` shown
only as a caveated skill-only floor. Build a **dedicated Part-C report** (single on-vs-base contrast; drop the
LSP/interaction contrasts of `Cell2x2`).

**Blinding + leak prevention** (the steer makes this load-bearing): the rank judge scores **final spec/plan
text**, not transcripts — but a steered arm may write "nav_callers shows…" / "Prism found…" into the text and
leak the condition. So: (a) the prism-on prompt instructs "do not name the tools you used"; (b) **scan final
text for `prism|nav_`** and flag/redact those judge inputs before ranking; (c) keep `detect.py` pooled-binomial
detectability over the rank-judge's condition guesses as the check that blinding actually held.

**Environment (reuse Phase-1c, unchanged):** prism MCP `--repo` = per-issue checkout (`arm_runner.py:38,48`),
`_prism_mcp_bin()` resolver, `cli_model_flag()` mapping, consolidated `prism-code-navigation` skill present in
the prism-on arm.

---

## Out of scope / deferred
- Fresh *unconditional* prism-off arms; always-on decomposition (the **GO-only sentinel** covers winning cells).
- LSP arms (inert for spec/plan); develop + review stages + build sandboxes (Phase 2).
- Scaling beyond 4 issues (2nd-per-language picks held: tokio #8182, prometheus #18972, mypy #21583,
  excalidraw #11313).

## Risks
- **Per-arm timeout:** steered spec generation can exceed the 600s `_TIMEOUT` (crashed Arm C). Bump the per-arm
  budget (the dead run used 1800s/arm) and isolate arm failures so one timeout doesn't abort the run.
- **Reuse validity** rests on: same SHAs/models/corpus (✓), recovered base genuinely prism-off (✓ 0 nav calls),
  header stripped before scoring (fix #2 baseline), identical oracle applied to both arms (✓). The bundle caveat
  is documented and the sentinel decomposes winners.
- **Adoption multiplier (0.72)** was measured on the adoption eval's `tier_c` micro-repo realistic probes, not
  the real Tier-C issues → it is an estimate; real-issue organic invocation may differ. Report it as such.

## Acceptance
- 16 prism-on-steered arms run; the invocation gate records **real** `mcp__prism__*` calls + dose; true 0-call
  arms discarded/re-run; low-dose arms flagged.
- 16 recovered prism-off baselines re-scored through the fixed oracle, header-stripped (regression test passes).
- **Oracle calibration:** a small fixed set of known-relevant / known-irrelevant / known-hallucinated citations
  is classified correctly by the fixed oracle, and the run **fails on all-YES or all-NO saturation** (guards
  against the oracle flipping from "always 0" to "always 1" — "≥1 non-zero precision cell" alone only proves the
  old bug moved).
- Sentinel runs on every GO cell; report shows the prism-vs-steer split for winners.
- Report renders per `(stage × language)` as directional pilot signal (n=1, counts shown) with bundle-lift +
  administration rate; detectability = not detectable and no `prism|nav_` leak survived into judged text.
- SUT checkout byte-identical before/after each arm; prism cache mode logged per arm.

---

## Review folded (codex gpt-5.5 xhigh, SHIP-WITH-FIXES)
Validated sound as-is: reuse recovered base, no fake-steer prism-off, bundle framing, claude stream-json,
score-text-not-transcripts. Findings folded: **[B1]** dedicated Part-C runner (not the 8-variant loop) →
Design/Harness; **[B2]** strip leaking header through `---` + regression test → Baseline; **[B3]** oracle needs
issue text (+ slice, + plan-stage spec), not just code → fix #1; **[S4]** record dose + flag low-dose (not
hard-gate) → fix #2; **[S5]** GO-only capability-steer decomposition sentinel → its own section; **[S6]** prism
cache/server isolation per arm → fix #3; **[S7]** `prism|nav_` leak scan + "don't name tools" → blinding;
**[S8]** relabel per-(stage×lang) as directional pilot signal at n=1 → Report; **[N9]** oracle calibration set +
all-YES/all-NO saturation guard → Acceptance.
