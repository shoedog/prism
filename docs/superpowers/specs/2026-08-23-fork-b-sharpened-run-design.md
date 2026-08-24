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
