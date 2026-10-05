> **R1 supersession (2026-10-05):** Historical prototype record. Its line-based all-CORRECT verdict is refuted by both spec reviewers. Active repair custody and byte-span binding evidence are in `HANDOFF-repair-r1.md` and `MEASUREMENTS-prA.md`; use the committed prototype `1b2dfdc9` plus R1 patches. Prior D11/D12 isolation and zero-WRONG claims do not apply to the repaired artifact.

# Handoff: lane js-param-defs, planner (step-0 census + PR-A spec/prototype/measurement)

**Written:** 2026-10-05 · **By:** planner subagent (Opus-5.5), controller session `0ec85e7b` · **Provider:** claude
**Workspace:** `~/code/prism-pd-plan` · `plan/js-param-defs` · **Measured state:** `[MEASURED]` HEAD `006573d9` · Tree DIRTY (prototype + packet, uncommitted by design) · Probe `git status --short` · Output: final report
**Predecessor:** none (first planner in this lane)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. Every `[MEASURED]` claim was probed this session; evidence is under `~/prism-evidence/js-param-defs/`.

## 0. Gating facts — settle these before starting anything below
**(a) Lane ownership** — `[INHERITED]` the controller owns lane 1 and `CACHE_VERSION` (EVALUATION §2.1). This planner has finished. — **RESOLVED** at handback.
**(b) Custody exposure** — `[MEASURED]` everything is uncommitted: 10 modified tracked files, plus `src/ast_js_param_defs_tests.rs`, `src/cpg/js_param_defs_tests.rs`, `mutants/js-param-defs.json` and the packet docs/probes. The controller commits. Evidence lives outside the repo in `~/prism-evidence/js-param-defs/`. — **OPEN** until the controller commits.
**(c) In flight / irreversible** — `[MEASURED]` none. No background jobs remain.
**(d) Authorization granted but not exercised** — "No git commits/pushes (controller commits)." "Never open any directory named frontend-portal."

## 1. Resume order
1. Controller: commit the prototype (src/tests/mutants) and the packet as two commits (messages in the final report).
2. Controller: F census and F diff:
   - `CORPUS_F_ROOT=~/code/frontend-portal PRIVATE_EVIDENCE_ROOT=~/prism-evidence/js-param-defs/f-<new> bash docs/superpowers/plans/2026-10-06-js-param-defs/CONTROLLER-pd.sh diff <TS_JS> <BASE_BIN> <HEAD_BIN>`.
   - Use the base binary `prism-base-006573d9`, and a head binary rebuilt from the committed tree.
   - Each run takes about one T-sized build per binary.
3. Spec review: SPEC-prA.md, Opus ∥ sol, cap 2.
4. Sonnet implementation per IMPLEMENTOR-prA.md.

**STOP conditions:**
- F `status: STOP`;
- any LOST row;
- any ADDED WRONG row;
- any non-JS byte difference;
- an owner decision needed on E5/E6.

## 2. State ledger
| Item | State | Evidence / correction |
|---|---|---|
| Step-0 census (X, Xi, T, SB) | done | `[MEASURED]` CENSUS.md; `census/*.json` |
| PR-A prototype (D1–D13) | done, uncommitted | `[MEASURED]` nextest 5,152/5,152; full mutgate 139/139 |
| PR-A row delta + adjudication (X, Xi, T, SB) | done | `[MEASURED]` MEASUREMENTS-prA.md §1; 0 LOST, 0 WRONG |
| SecBench targets + eligible sweep | done | `[MEASURED]` §4/§4b; +7 traced, 0 regressions |
| Non-JS controls | done | `[MEASURED]` byte-identical (Python, Go, Rust) |
| Tier-A matrix + quick | done | `[MEASURED]` 178/178; quick identical to r2 |
| F census / diff | next (controller) | CONTROLLER-pd.sh |

## 3. Corrections to standing documents and memory
| Location | Stale or false assertion | Correction |
|---|---|---|
| EVALUATION §3.2 Gap 3+4 | "`find_parameters_node`: `.or_else(parameter)`" | Unsafe: 16 callers (SPEC D2). Use the new `parameter_binding_region` helper at 3 consumers. |
| EVALUATION §3.2 Gap 3+4 | "TS: admit `rest_parameter`" | No such node in the pinned grammar. It is `required_parameter` + `rest_pattern` (D3). |
| EVALUATION §3.2 Gap 3+4 | "`slots()` already handles it — port it" (implies Step-5b binding) | Binding the bare formal minted false Exact rows through E5. It stays a hole (D12). |
| EVALUATION §3.1 / brief | "arrow ≥3 should now trace" | Measured 1/3 trace. All 3 gain the Def; the other 2 have downstream barriers. |
| `survey/synthesis.md` (as quoted in EVALUATION §3.6) | "Excalidraw rest 13 / bare arrows 2" | This census counts 46 identifier rest formals (41 in named owners, 33 bare-used) and 2 bare arrows. The definitions differ and were not reconciled `[ASSUMPTION]`. |

## 4. Open work
| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | F run | next | CONTROLLER-pd.sh diff | controller | — |
| 2 | E5 call-ladder false Exacts through name-inferred callables | owner question | decide the lane | owner | `debug.ts:1057` |
| 3 | PR-B callback identity | pending | plan; census in CENSUS.md | PR-A merge | — |

## 5. Invariants and traps — do not do these
- Never add a `parameter` fallback to `find_parameters_node`. Its callers iterate children, and the S1b F3 binding would be lost.
- Never bind call arguments to the bare formal until E5 is fixed. That mints false Exact rows.
- Never apply the D11 guard to pre-existing plain formals. That changes existing rows; it is PR-B's job.
- Do not compare SecBench outcomes against R1 numbers from a different binary. Use the same-harness base/head sweep (`probes/secbench_subset.py`).
- Rebuilding the SUT during a Tier-A or SecBench run invalidates that run (lesson 12). One Tier-A run was discarded for this reason.

## 6. Identifiers
| Item | Verbatim |
|---|---|
| base binary | `~/prism-evidence/js-param-defs/bin/prism-base-006573d9` sha256 `d2babb6d6e415866c6896fad7cad17f545a246d7939b20444cde32542447b7af` |
| head binary (measured) | `~/prism-evidence/js-param-defs/bin/prism-head-prA3` sha256 `b8838d2ce9c663bc7ed9c80b30c25d7c21f329f840854789ab8b1c009e21e553` |
| head after fmt | `~/prism-evidence/js-param-defs/bin/prism-head-prA3fmt` sha256 `cdc22b31f37b8f2758561adc58f82e776ac80eab5b2435aa992ee4aa924c6b5a` |
| TS checker | `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` |
| SecBench inspection | `~/prism-evidence/meas/secbench/repair-r1/measured/inspection.jsonl` sha256 `ba2f57c6…` |

## 7. Refutation verdict and owner questions
**§2c verdict:** NOT RUN — the planner's handback goes to an independent two-reviewer spec round next · claim: "PR-A adds 1,920 checker-correct DFG rows and 0 LOST/WRONG rows, with byte-identical non-JS and call-site output" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: MEASUREMENTS-prA.md

**Questions the owner owes an answer to:**
1. E5: base already resolves bare calls **Exact** through `free_single` to name-inferred callables (`{ Array: t => … }`). Which lane owns that fix? PR-A avoids amplifying it (D12).
2. E6: should Exact mean "only reaching definition" rather than "reaches on an unflagged route"? This affects every formal, not just PR-A's.
3. Confirm the cross-lane rebind of `P2-M11-cpg-cache-version` in `mutants/lane-p-tsconfig-paths.json`.
