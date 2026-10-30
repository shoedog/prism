# JS/TS workstream value: status, surveys, measurements and decision (2026-10-05)

**Decision (owner, 2026-10-05):** the next JS/TS product slice is **"JS/TS parameter-Def gaps"**. A small, independent `dist/`/`build/` admission slice and a measurement lane for the still-unmeasured areas are proposed alongside it; their orchestration is under independent evaluation. This document records how the decision was reached.

## 1. The seven JS/TS/React workstreams: coverage as of 2026-10-04

| # | Workstream | Status | Landed in |
|---|---|---|---|
| 1 | Exact parameter-token binding repair | **Done** | #312 (removed exactly 3 public / 61 private body-target flows) |
| 2 | Additional parameter forms | Partial | #313, #317 optional/inert defaults; #319 frequency census; #322–#324 positional gap measured and deferred |
| 3 | Occurrence and member data-flow limits | Partial | #316 rvalue owner scoping; #317 same-line occurrences; #318 exact caller reads; #314 callback flow |
| 4 | Real-project compiler ownership and admission | Not covered | — |
| 5 | React.FC and hook/context provenance | Not covered | — |
| 6 | Node-specific value checkpoint | Not covered as a measurement | Some mechanisms touched by #314, S1b, P2 and PKG (parked) |
| 7 | Production readiness and accuracy measurement | Measurement part **done** | #346 Tier-A quick repair; #347 SecBench harness |

Most work after #320 went to import and call-resolution lanes outside this list: S1/S1b (#325–#336), tsconfig paths P1/P2 (#337–#342), and S2/PKG (parked; see `docs/features/language-coverage/jsx-tsx-react-plan.md`).

## 2. Literature surveys (prioritisation prior only)
- **Survey A:** Sonnet-5.5, general OSS defects. Files: `surveys/survey-defects-sonnet.md`.
- **Survey B:** gpt-6-luna, npm/Node/React advisories. Files: `surveys/survey-cve-luna.md`.

Both used the shared capability taxonomy in `surveys/taxonomy.md`. They **independently ranked the workstreams 3 > 6 > 2 > 5 > 4**. Evidence gaps: there is no peer-reviewed prevalence data for React hooks or for destructuring bugs, and the BugsJS table could not be extracted. Neither survey's citations were verified by the controller. Synthesis: `surveys/synthesis.md`.

## 3. Tier-A quick: accuracy measurement repaired (#346, `75fbe1f7`)
Quick is now bounded, pinned and VALID. There were three fresh VALID runs, one of which exercised the bounded Rust recovery. The rust-analyzer ContentModified root cause is still unknown, and the harness fails closed. The tsserver oracle binds incoming callers by **declaration identity** (base, sibling, override and `super` callers are excluded).

| Measurement (Excalidraw TS; Node sample) | Result (95% Wilson CI) |
|---|---|
| TS caller precision | 90.3% [81.3, 95.2]; Exact 98.3% |
| TS caller recall | ≈37% |
| TS callee precision / recall | 81% / 94% |
| TS member (object-property function) recall | 15.9% [8.9, 26.8] |
| **Node member recall** | **0/84 [0, 4.4]** |
| Node caller / callee recall | 14% / 24% |

Of TS callables, 29.7% are outside the call-hierarchy frame, as are 66% of Node callables (mostly object-property functions).

## 4. SecBench.js ground truth (#347, `2777599b`)
SecBench.js `5d362353` has 600 npm vulnerabilities; 583 packages were acquired with `npm pack` and sha256-pinned. The harness `eval/secbench` (`uv run secbench`) derives the exported-API or HTTP-handler source plus the recorded sink, runs prism, classifies each outcome, and **demonstrates conversions** with semantics-preserving rewrites. No corpus code is executed.

- **Eligible:** 373 (from 192 after review round 1). The gain comes from HTTP-handler sources (136; 75 near-clone clusters) and TS-checker callee resolution (44).
- **Traced end to end:** 113.

| Workstream | Demonstrated single conversions (severity-weighted) | Joint |
|---|---|---|
| ws3: member-only formal Def | **20 (35)** | ws3+ws6: **92 raw / 34 handler clusters (184)** |
| ws6: callback-argument parameter registration | 1 (2) | (same joint) |
| ws4: dist/build admission | 7 (14) | — |
| ws2: rest parameter | 6 (12) | — |

### Prism mechanisms found (fixtures in the packet's evidence; to become product slices)

| Mechanism | Affected entries | Workstream |
|---|---:|---|
| A function or arrow passed as a call argument gets no parameter Defs (any statement context) | 97 | ws6 label, but a syntactic gap |
| A formal used only as `x.member` gets no Def (intentional field-isolation guard) | 133 incl. stacked | ws3 |
| A single unparenthesised arrow parameter `x => …` gets no Def (review r2 F2) | ≥3 | ws2/ws3 |
| A rest parameter gets no Def | 9 | ws2 |
| Legacy `arguments` | 6 | ws2 |
| `dist/` / `build/` skipped by `repo_loader` | 19 | ws4 |
| Nested-callback capture | 5 | ws6 |

**Server-side destructuring** sits on 0 eligible payload paths; rest/spread on 9. SecBench skews to old, small packages, so this axis is **unmeasured**, not absent.

**Review:**
- Round 1: Opus and gpt-6.1-sol both returned FIX.
- Round 2: Opus returned APPROVE. It sampled 17 conversion patches across every stratum and found them semantics-preserving and single-mechanism, and all 126 credited traces reach the sink from the payload parameter.
- Follow-ups F1–F6 are listed on PR #347.

## 5. Independent blind labels (`blind-labels/`)
Sonnet labelled a frozen, failure-enriched sample of 40 entries blind (`blind-label-brief.md`). It was scored against the harness labels (`harness-reference.jsonl`) with `eval.secbench.blind`; the output is in `kappa.json`.

| Stage | Cohen's κ | Raw agreement |
|---|---:|---:|
| GT availability | 0.14 | 77.5% |
| Outcome | 0.36 | 45% |
| Mechanism | 0.26 | 35% |
| Traced vs not traced (both GT-available) | — | **28/30** |

Interpretation:
- **Label-based attribution is unreliable.** The labeller lacked the harness's function-level and frontier queries, and the mechanism disagreements cluster between callback registration and member flow.
- The decision therefore rests on **demonstrated conversions**, which are re-run evidence, not labels.
- The sample is failure-enriched, so it estimates agreement, not prevalence.

## 6. Decision and next lanes
1. **JS/TS parameter-Def gaps:** callback-argument parameter registration, member-only formal root Def, unparenthesised arrow parameter, and rest parameter Def. This covers about 20 single plus 92 joint plus 6 demonstrated conversions, cross-checked against Tier-A Node member recall of 0/84.
2. **dist/build admission:** an independent small slice (7 conversions).
3. **Measurement lane:** Node runtime and async beyond callbacks (SecBench bypasses ingress), React props and hooks, and server-side destructuring.

**Row-change correctness method:** every changed or lost row is classified by a native TS symbol-identity oracle plus CFG reachability, SecBench traces, Tier-A, and blinded adjudication where no oracle decides. Changes are gated to JS/TS, with byte-identical controls for the other languages. Details are in the parameter-Def lane plan.
