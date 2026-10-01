# Lane-P measurements — Opus round-2 fold

MEASURED: plan HEAD **71a10b85**, prototype HEAD **88b61108**, initial prototype tree equals reviewed **060092b4**. The fold has three incremental prototype paths and 39 cumulative paths against **e61d52b8c6dfea9ad8db868797122a0bd067d27d**. Current receipts: **target/paths-plan/spec-r2/evidence/**. No Git writes, installs, network/provider use or corpus F reads.

READ: Opus round 2 is FIX (2 WRONG / 4 SMELL). This is the disclosed targeted fold at the two-round cap, with no restart or additional review. **Fix A applies. S6 remains an owner question.**

MEASURED pre-fold cost: the old root-file model selects ancestors for 102 X alias sites in 17 files; the actual reviewed kernel admits modules for **94**, already refusing 8. **89 are changed gains**, so Fix B would remove 89 X gains and exceeds the controller threshold. The 156 original controls have 12 root-model sites, of which the kernel admits **8**, all changed gains (4 already refused). Receipts: X/controls-fallthrough.json, X/controls-production-fallthrough.json and before-kernel/*-resolutions.json. Fix B was halted before any production edit.

MEASURED: W1-r2 refuses wildcard MJS/MTS/declaration-MTS, CJS/CTS/declaration-CTS, minified JS, dot-prefixed basenames and include segments starting with ?. Explicit files retain the existing exemption. Fix A refuses an excluding/not-listed tsconfig only when it has a sibling jsconfig or enables disableSolutionSearching. Including tsconfig still wins. S7 takes ownership from real offline TypeScript 5.9.3 ProjectService.getDefaultProjectForFile; root-file selection is only a cross-check. Any ownership disagreement is UNPROVEN. S9 reports requested import-forward terminal classes, member-write detail codes, and finite membership/star/syntax classes. These are independent explanations, not instrumented production reasons.

| Corpus | Sites | Changed rows and classes | Ownership-disagreement sites / files | Changed disagreements |
|---|---:|---|---:|---:|
| X | 19219 | 3121 / {'CORRECT_STATIC_BINDING': 3121} | 6 / 2 | 0 |
| R | 953 | 0 / none | 144 / 17 | 0 |
| T | 61712 | 0 / none | 15 / 5 | 0 |

MEASURED: all public keys and row metadata are unchanged. X retains ten recoverable refusals: **8 NONRELATIVE_EXPORT_HOP + 2 IMPORT_FORWARD_NOT_FORWARDABLE**, **0 UNCLASSIFIED**. X's six ownership disagreements are unchanged rows; R/T disagreements also keep base. public/FINAL-SUMMARY.json binds expected rows, 649/55/722 input files, hashes and classifications.

MEASURED: **184 controls / 198 sites / 63 changes**: **59 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL + 2 UNPROVEN**. The two UNPROVEN rows are the S6 transitive C80 controls, explicitly pending OQ2 rather than waived. Every declared Option-K preservation control equals the complete base row. All **18 new matcher/Fix-A barrier rows** mint Exact before the fold and preserve base after it (r2-red-population.json, red-test.log, green-test.log). J/C78 independently owns the sibling jsconfig, K/C79 is inferred, N/C80 owns the nearer transitive project in both target grammars.

MEASURED: **28/28 kernel mutants killed**, including one per new matcher barrier and one per Fix-A condition; **11/11 integration mutants killed**, each selecting one test with an actual failing assertion. Kernel dependencies are the matching release artifacts. Setup, missing-file, wrapper/cache and fixture-construction failures are inadmissible and recorded in hypothesis-probe-result.log; no such status is counted as a behavioral result.

| Full suite | Passed | Failed | Ignored | Groups | Log SHA-256 |
|---|---:|---:|---:|---:|---|
| default | 4811 | 0 | 1 | 29 | bcd076cbb00256979aeb610fe24b4289024413499c4102c814b7865a0881f1bf |
| mcp | 5004 | 0 | 1 | 31 | 15d79870f5855dbc80eb9fdd1d1ccb7f0047ddf0039c351d9a6819c6b064966e |

MEASURED: Tier-A matrix **170 ok / 0 regressions** after a release rebuild in the actual prototype. The uv wrapper failed on its protected cache, so installed Python 3.12 ran the same tier_a.cli directly without installs or harness/baseline edits. Formatter and diff checks pass; MCP library clippy passes with **139 warnings**. **411 S1b controls / 639 sites / 1,234 artifacts are byte-identical to e61d52b8 main**, and both summaries equal the committed r5 reference (**b550a2c7466fdbe4d331f93843d44f0bfcfb6c62febf81f115c86a9ab64dd5ca**). Base/facts binaries have inherited rebind-e61 build provenance, were hash-checked and freshly executed in this environment; they were not rebuilt this fold.

MEASURED: controller wrapper defaults to the final immutable spec-r2 head and returns independent ownership-disagreement, refusal and member-write aggregates. Public synthetic smoke emits one JSON object, empty wrapper stderr, **0 UNCLASSIFIED**, and deliberately exits **1** on the two S6 UNPROVEN gains. That is evidence the rejection gate remains active, not private acceptance. actual_F_run=false; never interpret the wrapper's fixed corpus label on this synthetic run as an F receipt.

## Review dispositions

| Item | Classification and disposition | Evidence |
|---|---|---|
| W1-r2 | WRONG; corrected in place | C74–C77, 18-row RED population with C78/C79, M22–M26 |
| W3 | WRONG; Fix A applied | 89 X gain cost rejects Fix B; C78/C79 and M27/M28 |
| S6 | SMELL; owner question remains open | C80 both grammars independently UNPROVEN; OQ2 |
| S7 | SMELL; independent ownership implemented | ProjectService J/K/N controls, X/R/T disagreement counters |
| S8 | SMELL; dispatch corrected | proto/tsconfig-paths-final @ controller fills, parent e61d52b8, one prototype message |
| S9 | SMELL; finite classifier expanded | X 0 UNCLASSIFIED, import-forward controls and member-write detail codes; F pending |

MEASURED source-level discriminator: property writes alone do not block a named function declaration in the existing S1b import-forward model. C83 (written named function) resolves; C81 (written function variable) and C84 (unwritten function variable) refuse. The primary reason therefore combines requested import-forward shape with the terminal declaration class; TERMINAL_MEMBER_WRITTEN is a separate detail, not a standalone causal claim.

Commands and receipts are retained under spec-r2: measure-before.py, check-cost.py, run-controls.py, finalize-public.py, run-s1b.py, package.py and refresh-docs.py. Production source froze before the full suites; the retained progress snapshot's three incremental source hashes match final source, and final frozen 39-path hashes match replay and measured source. Probe/script hashes are recorded in final-checks.json. No supplied or historical receipt is counted as local execution.

READ / UNKNOWN: corpus F and its 119 prior unclassified refusals were never opened or remeasured; the controller must run the updated wrapper. S6's transitive ownership authority remains open under OQ2, and synthetic changed rows remain UNPROVEN. Owner confirmation, independent acceptance, commits/push/merge and P2 are outside this completed fold. All-features Cargo, Tier-A quick/full multi-corpus, dedicated cache acceptance, base full suites, runtime mutation/security/concurrency and large-tree performance were not rerun. Quick has historical pin/probe validity failures; no fresh quick-validity claim or baseline change is made. Requested default/MCP suites, matrix, mutants and S1b identity were freshly executed.

## Controller F acceptance after the spec round-2 fold (private, aggregates only; 2026-10-01)

Run with head `bab21e62` (cumulative on `e61d52b8`) and the independent ProjectService-ownership oracle.

**Result:**
- Changed rows: **2,345**, all `CORRECT_STATIC_BINDING`.
- Keys added or removed: 0.
- Callable-recoverable: 3,106.
- **Changed rows that disagree with tsserver ownership: 0** (19 unchanged rows disagree).

**Refusal histogram:**
- `NONRELATIVE_EXPORT_HOP`: 717.
- `BINDING_OR_SITE_GUARD`: 23.
- `UNCLASSIFIED_P1_PROOF`: 21.

**Fix A applies.** Fix B would remove 89 correct X rows. S6 (transitive project membership) stays an owner question under OQ2. On X and F, 0 changed rows disagree with ProjectService; the two S6 controls are synthetic.
