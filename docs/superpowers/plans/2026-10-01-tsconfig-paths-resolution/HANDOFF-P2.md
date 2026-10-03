# Handoff — lane P, P2 Opus spec round-1 fold

**Written:** 2026-10-03 · **By:** Codex planner/repairer · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-paths-p2-plan` · **Measured state:** `[MEASURED]` plan/tsconfig-paths-p2 HEAD bb4e4743, tracked tree CLEAN. Probe `git status --short --branch`; round-1 prepared snapshot under target/p2-spec-r1.
**Predecessor:** owner-supplied plan bb4e4743 / cumulative prototype b9fd3775 / main c50de85a and Opus spec review round1 FIX.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker using installed `/Users/wesleyjinks/.codex/handoff-template.md`. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner assigned planner/repairer for Opus review1 fold. No subagents dispatched. Spec review cap2, used1/2; repair gate cap2 attempts. RESOLVED within the brief.
**(b) Custody exposure** — `[MEASURED]` candidates/sources/baseline rows retained in `target/p2-spec-r1/prepared-owned-snapshot.tar.gz`, `prepared-proto.patch`, `prepared-source-binding.json`. No Git writes. Controller prototype branch switch remains OPEN: this clone is still plan bb4e4743 with no prototype worktree; an async controller-switch request is pending. Code candidates have not been applied to tracked files.
**(c) In flight / irreversible** — `[MEASURED]` baseline RED and H1 reference probes completed; no task process running. RESOLVED. Product gates have not started before correct branch custody.
**(d) Authorization granted but not exercised** — `[INHERITED]` "Edit the prototype on its branch checkout, and the plan docs on the plan branch"; "No git writes"; "Never open F". Controller must switch checkout; worker must not bypass the branch requirement with a gitless implementation.

## 1. Resume order

1. Controller switches the clean clone to `proto/tsconfig-paths-p2 @ b9fd3775`. Worker rebinds status/HEAD and exact base hashes, applies four prepared prototype paths, runs focused regression and gates (minutes). Required branch action is blocked by no-worker-Git-writes instruction; async request pending.
2. Run one full `cargo nextest run --offline --features mcp --no-fail-fast`, scoped `spec-r1-mutants.py`, fmt/clippy, release rebuild immediately followed by Tier-A matrix, `legacy-forward-controls.py`, `nonrelative-cache.py`, S1b-4 and all complete public streams.
3. Freeze repaired binary at `target/p2-spec-r1/bin/prism-p2-spec-r1` with production input/binary hashes and source snapshot; controller preserves prototype bytes and switches back to plan bb4e4743 for prepared plan edits. Worker performs no .git writes.
4. Controller supplies private environment and accepted prototype binary via `ACCEPTED_P2_BIN`; run updated `CONTROLLER-p2.sh P1_BIN FACTS_BIN REPAIRED_P2_BIN PRIOR_GAP_EVIDENCE`. Return aggregate stdout only; expected F repair impact0. Main acceptance +629 is supplied evidence, not a repaired-binary receipt.
5. Controller commits/squashes the final cumulative prototype on main c50de85a and commits the plan set separately; dispatch implementer from that final cumulative artifact. Round2 review remains controller-owned.

**STOP conditions:** wrong branch/base/hash, any uncertified changed row, complete-row loss, nonzero F repair impact, private worker access, open-class defect at the repair cap. No restart.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| W1 diagnosis / same-environment main control | done | `[MEASURED]` legacy-red-corrected/summary.json:8 fixtures/16 sites; main refuses, unmodified proto selects wrong sibling in both grammars/orders. control-source-binding.json:304/304 input hashes match each exact revision. |
| W1 code, both-grammar regressions, baseline rows | pending | `[MEASURED]` four prototype candidates in proto-prepared; tracked application/gates await controller checkout. |
| S1 authority / S2–S6 disclosures | pending | `[MEASURED]` plan-prepared headers replace UNKNOWN350/98/+629 with supplied controller acceptance and cumulative dispatch; not yet applied on plan. |
| S5 H1 cache control | done | `[MEASURED]` cache-reference-corrected/summary.json:2 grammars/10 states, cold/warm x, declaration add refuses/remove restores x, caller paths edit moves y, every cached output equals fresh. Repaired run pending. |
| F acceptance of b9fd3775 | done | `[INHERITED]` P2-MEASUREMENTS controller acceptance at bb4e4743:+629 CORRECT,350 explained,98/98 retained. |
| Fresh repaired gates / F impact | pending | `[UNKNOWN]` no repaired branch build yet; worker has no F observing capability. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| SPEC | Non-relative forwarding facts inert on legacy route | `[MEASURED]` W1 re-probed; draft amendment explicitly preserves unresolved non-relative imported-local claims as BlockedClaim. |
| IMPLEMENTOR/HANDOFF/P2-FILES | Apply old incremental patch to e80fbf54; review not dispatched | `[INHERITED]` cumulative prototype is b9fd3775 on c50de85a; draft dispatch starts from controller's final cumulative squash; spec review1/2 returned FIX. |
| P2-MEASUREMENTS/OQ | +629/350/98 unknown | `[INHERITED]` controller measured +629,350/350 noncontributing branches,98/98 retained; OQ2/4/5 closed for b9fd3775. Repaired F gate stays separate. |
| cpg_cache.rs | v106 mentions only relative hop proof | `[MEASURED]` candidate comment describes caller-config String serde shape too; no version bump. |
| Memory | None | No relevant quick-pass hit; no memory write. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Branch-bound prototype edit/gates | blocked | Controller switch to proto b9fd3775; apply candidates, verify | No-worker-Git-writes rule | prepared-source-binding.json |
| 2 | Branch-bound plan fold | pending | Apply prepared docs/probe runners on plan bb4e4743 after product gates | Controller checkout custody | plan-prepared |
| 3 | Actual repaired F impact | pending | Controller updated wrapper with ACCEPTED_P2_BIN | Never open F; repaired binary not yet built | CONTROLLER-p2.sh |
| 4 | S2 three-row class | parked | Controller may name gate class; until then disclose residual | Private-only rows | review S2 |

## 5. Invariants and traps — do not do these

- Never open F or write .git; branch switching/commits are controller actions.
- Never apply the old e80fbf54 increment onto an integrated cumulative prototype.
- Never reselect the barrel's config; caller program options persist through the chain.
- W1 only blocks unresolved non-relative ImportForward; inherited ReExport/relative/star claim limitations are S3 disclosures, outside this repair.
- Initial runners rejected normal warning/cache stderr: INADMISSIBLE; corrected scripts save diagnostics and inspect full output rows. Receipts stay outside scanned sources.
- +629/350/98 are accepted-checkpoint controller measurements; expected repaired F impact0 is not a fresh measurement.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Plan parent | bb4e47436447b800593cfdc798defca7f14bde6b |
| Reviewed prototype | b9fd3775a91a206c556e05e26656285ee0c66bff |
| Cumulative production parent | c50de85a |
| Main-source-bound P1 binary | /Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism |
| P1 SHA256 | 907d110c70962063d5fde23acd22b6d92b62b117d837b97a81f6d65ed263aa03 |
| Reviewed-source-bound prototype binary | target/p2-nonrelative/bin/prism-p2-nonrelative-verified |
| Prototype binary SHA256 | df0cca2f4a1d79be5ff33a55dff5ac1b71ae53106d47612385f44d5a6d0f1d85 |
| Review | /Users/wesleyjinks/prism-evidence/paths/reviews-p2/spec-r1-opus.md |
| Compiler SHA256 | 3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675 |
| Current repair receipts | target/p2-spec-r1 |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — prepared fix has not been applied/built on its required branch · claim: "the repaired prototype restores complete main refusal on legacy A1/A4 without losing accepted alias yield" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: prepared-proto.patch; fresh baseline RED and H1 receipts in target/p2-spec-r1.

**Questions the owner owes an answer to:** controller prototype checkout switch, then plan checkout for plan fold; repaired F aggregate; round2 review and external custody. No new scope permission requested.
