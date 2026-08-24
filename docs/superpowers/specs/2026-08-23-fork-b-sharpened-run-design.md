# Fork B — sharpened Part-C/D measurement design (v1)

**Date:** 2026-08-23. **Owner decision:** B-first (2026-08-23), then A (Python-first) per B's numbers. **Doctrine:** rescore before re-run; no live spend before the phase that needs it is signed off.

## Phase B1 — close Part-D's own open caveats (cheap, mostly zero-spend)

1. **INSTRUMENT-FAIL audit (zero spend, saved data):** the REFUTED verdict stands only if the 6 off-saturated arms *genuinely* reasoned structurally. Audit the saved off-arm transcripts (`eval/tier_c/runs/partd/full-gpt-5.5-2026-08-21/<task>/<task>-impact-gpt-5.5.off.raw.jsonl`) for all 6 saturated tasks: classify each as (a) structural reading through wrappers/dispatch, (b) grep/text luck, (c) leakage/other; extract commands, cited-vs-gold D-sites, token counts. If (b)/(c) dominate, the refutation weakens and the corpus needs harder tasks, not abandonment. **Dispatched to an Opus lane now** (read-only rescore).
2. **TS administration (small live spend, ~4 cells):** the two TS tasks ran 0-dose (codex silently dropped the slow MCP handshake — root-caused, and #178's lazy handshake has since SHIPPED). Re-run `typescript-resolve-alias` + `typescript-resolve-signature`, on+off arms, current harness with: warm gate ≤10s pre-handshake per the readout's rule, absolute paths (#172 lesson), current main SUT. Cost ceiling: ~4 cells × 1–3M input tokens (the Part-D per-cell envelope). **Needs owner spend ack before dispatch.**
3. Fold 1+2 into an updated read-out: the Part-D verdict then rests on audited evidence and a complete corpus.

## Phase B2 (conditional on B1's picture) — Part-C precision row on the CURRENT SUT

Part-D measured pre-#169 prism (main @ 47e21ae). Today's main carries the entire accuracy wave (#149–#189: P1–P14, owner-partition program, #17-narrow, slice-4, R1(b) in flight). The durable Part-C finding — citation-precision ∝ per-language maturity — was measured on the OLD SUT too. B2 re-runs the Part-C spec row (Go/Rust/TS/Python corpora) with the three methodology fixes: **codex-xhigh judge** (~2× steadier than sonnet; run-average for ±0.1 effects), **citation-precision endpoint** (not ΔdR, not head-to-head — the judge-dependent signal is explicitly not banked), tasks screened against off-saturation using B1.1's classifications. Output: per-language Δcitation-precision on current prism — the numbers that decide A-vs-C, and the capstone number for the accuracy wave. Cost: ~4 langs × 2 arms × spec-task envelope (estimate refined after B1.2's cells; Part-C rows historically ~1–2M in-tokens/arm) + judge passes. **Separate owner spend ack with the refined estimate.**

## Non-goals
No new harness construction (both harnesses exist); no claude-slate expansion unless B2's row contradicts Part-C's shape; no head-to-head endpoint.

## Gates / discipline
Pre-registered read-out criteria before any live arm (B1.2's and B2's endpoints + thresholds written down first); rescore-over-rerun whenever saved arms suffice; every cell's admissibility checked (dose>0, handshake timing logged) before its number enters an aggregate — Part-D's two voided/unmeasured failure modes are the checklist.

## B1.1 OUTCOME (2026-08-23) — INSTRUMENT-FAIL confirmed; Part-D verdict downgraded

The audit (Opus lane, saved transcripts, 6/6 auditable): **67/67 recovered D-sites were TEXTUAL, 0 structural** — in 5/6 tasks via a single `rg` whose query token came from the harness prompt itself (`partd.py:165` appends `task.dispatch` to BOTH arms as "Known dispatch path"; for these tasks that names the exact forwarder whose absence defines D1). Part-D's read-out downgrades from "REFUTED — grep suffices" to "**corpus cannot discriminate: D-recall is a text-search metric under the current prompt**" (prism-on's own losses, −0.36 guava / −0.222 prom-matchstring, still stand as prism errors). Four validity defects for the repair: (1) D admission must be a CHAIN property (forwarder token absent/ambiguous too), not per-file; (2) D2 is near-vacuous — drop from numerator or pair with precision; (3) no prompt-side gold-leak scanner exists (`scan_leak` only blinds judge output); (4) `norm_symbol` quantizes the denominator.

**B1.3 (pending owner spend ack):** the audit's cheapest discriminating probe — re-run guava (27 sites on one grep, sharpest case) with the SCOPE dispatch line stripped, separating the hint-vs-reverse-grep confound (~1 cell, ~1.3M in-tokens). Protocol repairs (a) strip `task.dispatch` from arm prompts, (b) chain-property D admission, (c) D2 disposition — required before ANY Part-D number is cited as a fork-B verdict.

**TS cells (B1.2, in flight at audit time):** launched under the OLD prompt — their dR inherits the non-discrimination defect. Their admissible yield: administration/dose evidence for the #178 lazy handshake (the instrument fix under test) + transcripts for the repaired protocol; their dR does NOT enter any verdict. Disclosed, not killed (spend already committed; dose evidence stands).

## B1 COMPLETE (2026-08-23) — consolidated read
- **B1.3 guava probe (hint stripped): off-arm dR = 1.000 unchanged** — the leaked hint was NOT load-bearing; a reverse-grep on the target name reaches the forwarder because D1 is a per-file property (validity defect 1). Part-D's dR endpoint is non-discriminating on this corpus BY CONSTRUCTION; the repair is a chain-property gold rebuild (a new corpus), not a prompt fix. (On-arm 0.960/−0.040, 1 phantom — consistent with the prior guava cell's prism-side noise.)
- **B1.2 TS cells (first ever administered; #178 + 240s gate validated):** alias +0.176 dR / +0.243 file-F1 (WIN), signature −0.143 / 0.0 (LOSS, 1 phantom) — both DISCRIMINATING (off-arms 0.765/0.714, unsaturated). n=2, split.
- **Conclusion:** dR can discriminate only where off-arms don't saturate; this corpus mostly saturates for structural reasons independent of the hint. **B2 (Part-C citation-precision row on the current SUT, codex-xhigh judge) is the right next instrument** — its endpoint is orthogonal to grep-hardness. Refined B2 cost from the administered cells: ~1–2.5M input tokens per arm ⇒ 4 languages × 2 arms ≈ 8–20M input tokens + judge passes. Pre-registered criteria before launch.

## B2 PRE-REGISTRATION (2026-08-23, owner spend acked — written BEFORE any live arm)

**Cells:** the four June Part-C corpus rows at their pinned SHAs (ruff-26287/rust, prometheus-18896/go, pydantic-13300/python, excalidraw-11479/ts), stage **spec**, arm model **opus-4.8** (AMENDED pre-launch, disclosed: only the opus-4.8 off arms were preserved as raw streams — `runs/partc/isorow-2026-06-28-<repo>/<repo>-spec-opus-4.8.off.raw.jsonl` — which the admissibility rule and `_reconstruct_arm_from_files` require; the June gpt-5.5 off arms survive only as rendered .md transcripts. gpt-5.5 would force 8 live cells; opus-4.8 honors rescore-over-rerun). SUT = current main (post-#193, CPG 50), matched-binary preflight (F1) + warm gate with `--warm-gate-timeout-s 240` (B1.2 TS lesson).

**Arms & spend shape:** OFF arms are NOT re-run — the isorow-2026-06-28 opus-4.8 off-arm raw streams are saved and the off arm has no SUT dependence (same model/prompts/checkout SHA); they are RESCORED with the B2 judge (rescore-over-rerun rule). ON arms: 4 live cells. Estimated ≤ half the acked 8–20M envelope.

**Endpoint (sole banked number):** per-language Δcitation-precision = precision(on) − precision(off), judged by **codex-xhigh**, identical judge config both arms, run-average. NOT ΔdR, NOT head-to-head; judge-dependent signals beyond citation-precision are reported but not banked.

**Read-out criteria (pre-registered):** (1) |Δ| ≥ 0.10 counts as signal; |Δ| < 0.10 = wash for that language. (2) The decision object is the SHAPE across languages vs the old-SUT Part-C row (Go/Rust strong, TS strong, Python wash): same-shape ⇒ maturity finding re-confirmed on the current SUT and the accuracy wave's value is banked at the new magnitudes; shape inversion or Python turning strongly positive ⇒ the accuracy wave changed the value distribution — triggers the A-vs-C re-read, no silent reinterpretation. (3) No claude-slate expansion unless the row contradicts the old shape.

**Per-cell admissibility (all required before a number enters the aggregate):** dose > 0 (on-arm prism tool calls observed in the transcript); warm-gate handshake logged with timing; prompt-leakage check — the Part-C prompt for the cell is inspected for task-token leakage of the B1.1 class before the arm runs; off-arm rescore uses the SAME saved outputs the June run banked (no regeneration). Inadmissible cells are named and excluded, never imputed.

**Custody:** run dirs under `eval/tier_c/runs/partc/` with run-id `b2-<date>`; all cell JSONs + judge outputs durable-copied to `~/code/prism-lane-artifacts/2026-08-23-next10/fork-b/b2/`; read-out appended to this document.

## B2 READ-OUT (2026-08-24) — 3 of 4 cells admissible; pre-registered shape-change trigger FIRES, but attribution is confounded

**Run:** 4 cells `<repo>:spec:opus-4.8`, SUT main@`f423401` (post-#193), off arms reused verbatim from `isorow-2026-06-28` (verified **byte-identical** to the June raw streams), codex-judge rescore per cell. Artifacts: `~/code/prism-lane-artifacts/2026-08-23-next10/fork-b/b2/` (prism-caches stripped).

### Admissibility (pre-registered gate)
| cell | lang | administered | dose | leaked | verdict |
|---|---|---|---|---|---|
| ruff | rust | ✓ | 3 calls (callers/callees), 1 tool error | no | **admissible** |
| prometheus | go | ✓ | 3 calls (callers/callees/repo_map), 0 errors | no | **admissible** (recovered — see below) |
| excalidraw | ts | ✓ | 4 calls (callers/callees/repo_map), 1 tool error | no | **admissible** |
| pydantic | python | ✗ | — | — | **INADMISSIBLE — excluded, not imputed** |

- **pydantic**: the ON arm produced no output at all (`status.json`: `failed_stage: "on"`, `arm exited 1`); no `.on.out.md`/`.on.raw.jsonl` exist and the rescore correctly failed. Nothing to score. Python is **unmeasured** in this row.
- **prometheus**: the live cell exited 1, but the crash was in `head_to_head_annotated` (the ensemble opus judge, `claude judge exited 1` — plausibly the same API-overload window that hit pydantic). Head-to-head is explicitly **not** the banked endpoint. Both arms' raw streams were saved, and the rescore reconstructed and scored the cell successfully — so the banked number exists and is admissible. This is the rescore path working as designed.

### Banked endpoint — Δcitation-precision (codex judge; |Δ| ≥ 0.10 = signal)
| lang | precision off | precision on | **Δ** | vs old Part-C row |
|---|---|---|---|---|
| Go (prometheus) | 0.364 | 0.769 | **+0.406** | same sign, **larger** (was +0.18..0.26) |
| Rust (ruff) | 0.778 | 0.634 | **−0.144** | **SIGN INVERSION** (was +0.18..0.26) |
| TS (excalidraw) | 0.750 | 0.750 | **0.000** | collapsed to wash (was +0.23) |
| Python (pydantic) | — | — | **no data** | (was ~0) |

**Pre-registered criterion (3) FIRES:** the shape changed vs the old-SUT row (Rust inverted; TS → wash), which by the pre-registration triggers an **owner A-vs-C re-read** — recorded here rather than silently reinterpreted.

### ⚠ Attribution caveat — the trigger must NOT be banked as a prism finding yet
The off arms are **June (2026-06-29)**; the on arms are **August (2026-08-23)**. They differ in prism-availability **and** in run date, i.e. two months of model drift plus sampling noise. The pre-registration justified reuse as "no SUT dependence," which is true of the SUT but **not** of the arm model — so a same-window control was never run. Per the attribution rule (a control must come from the same environment that produced the effect), **the Rust inversion and the TS collapse are hypotheses, not findings**: model drift alone would produce the same output. The Go gain survives this caveat better only in the weak sense that its sign matches the old row.

Also note: `recall_*` values in these cells are not usable (prometheus `recall_on` = 1.05, excalidraw `recall` 15.0 → 3.0 — a known claim-count normalization defect noted in `cli.py`). Precision is computed separately and is unaffected; recall is not banked and should not be quoted.

### Recommended next step (owner decision, spend)
Re-run the **off** arms in the current window for ruff + excalidraw (2 cells, prism-off, no warm gate needed) and rescore all three cells together. That converts the shape question from confounded to attributable for the cost of two off arms. Until then the honest statement is: **Go shows a large positive Δ; Rust and TS are unresolved pending a same-window control; Python is unmeasured (harness failure, re-run needed).**
