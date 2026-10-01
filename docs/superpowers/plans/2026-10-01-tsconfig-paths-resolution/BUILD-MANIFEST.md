# Lane-P build manifest — capped Opus round-2 targeted fold

MEASURED: plan HEAD **71a10b85**, prototype HEAD **88b61108**, initial prototype tree equals reviewed **060092b4**. The fold has three incremental prototype paths and 39 cumulative paths against **e61d52b8c6dfea9ad8db868797122a0bd067d27d**. Current receipts: **target/paths-plan/spec-r2/evidence/**. No Git writes, installs, network/provider use or corpus F reads.

READ: Opus round 2 is FIX (2 WRONG / 4 SMELL). This is the disclosed targeted fold at the two-round cap, with no restart or additional review. **Fix A applies. S6 remains an owner question.**

MEASURED pre-fold cost: the old root-file model selects ancestors for 102 X alias sites in 17 files; the actual reviewed kernel admits modules for **94**, already refusing 8. **89 are changed gains**, so Fix B would remove 89 X gains and exceeds the controller threshold. The 156 original controls have 12 root-model sites, of which the kernel admits **8**, all changed gains (4 already refused). Receipts: X/controls-fallthrough.json, X/controls-production-fallthrough.json and before-kernel/*-resolutions.json. Fix B was halted before any production edit.

MEASURED: W1-r2 refuses wildcard MJS/MTS/declaration-MTS, CJS/CTS/declaration-CTS, minified JS, dot-prefixed basenames and include segments starting with ?. Explicit files retain the existing exemption. Fix A refuses an excluding/not-listed tsconfig only when it has a sibling jsconfig or enables disableSolutionSearching. Including tsconfig still wins. S7 takes ownership from real offline TypeScript 5.9.3 ProjectService.getDefaultProjectForFile; root-file selection is only a cross-check. Any ownership disagreement is UNPROVEN. S9 reports requested import-forward terminal classes, member-write detail codes, and finite membership/star/syntax classes. These are independent explanations, not instrumented production reasons.

## Source and binaries

MEASURED: cumulative patch SHA-256 **56345833cc6fc5ff22806507347fbe29998419f6e988a96a5763cf1056b8e0dd**. target/paths-proto/P1.diff and tracked prototype/P1.diff.txt contain identical bytes. A fresh Gitless archive replay on e61d52b8 reproduces all **39** owned file hashes, and **1362 non-owned source/test/eval/build inputs** match that base. source-binding.json records the exact parent hash or required absence and resulting hash per owned path. Cache versions remain **105/61** on parent **104/60**.

| Artifact | SHA-256 |
|---|---|
| Cumulative P1.diff | 56345833cc6fc5ff22806507347fbe29998419f6e988a96a5763cf1056b8e0dd |
| P1-owned-files.tar.gz | ff174ef83e7b3a14e65d088a9c41e0adf0934f7319d2faaeb2ee83a0bd8ba18d |
| source-hashes-frozen.json | 48db87f0b46c41117e09a533e0e00d4566bfcffd6aa08d9b73fa4936ccc46128 |
| source-binding.json | 4c97c49473bd83db5d9d2edb792a5ce216c9e341dcd53d6fae35dbeda06c615d |
| Final head/prism | a8456c670f1cbbf72891e3772cb6087ffbbe077e2e849b4761e7bb50c35dde03 |
| Offline TypeScript 5.9.3 | 3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675 |

MEASURED: final embedded binary version is `slicing 3.1.2 (88b61108f5b3-dirty)`; a dirty Git identity alone is not a source binding. Binary paths and hashes are in binary-binding.json. Physical additions from Git numstat are **900 production / 714 test-code / 1916 regression-data / 70 Tier-A fixture lines**; integration source is 713 lines. This physical-line method replaces earlier code-only size prose; no LOC cap exists.

## Remeasurement

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

## Not verified / remaining authority

READ / UNKNOWN: corpus F and its 119 prior unclassified refusals were never opened or remeasured; the controller must run the updated wrapper. S6's transitive ownership authority remains open under OQ2, and synthetic changed rows remain UNPROVEN. Owner confirmation, independent acceptance, commits/push/merge and P2 are outside this completed fold. All-features Cargo, Tier-A quick/full multi-corpus, dedicated cache acceptance, base full suites, runtime mutation/security/concurrency and large-tree performance were not rerun. Quick has historical pin/probe validity failures; no fresh quick-validity claim or baseline change is made. Requested default/MCP suites, matrix, mutants and S1b identity were freshly executed.

## Controller commit files and messages

Prototype incremental files from 88b61108 (initially tree-equal to reviewed 060092b4):

```text
src/js_paths.rs
tests/integration/js_paths_test.rs
tests/integration/fixtures/js_paths_r2.json
```

Message: **fix(paths): refuse matcher doubts and targeted ownership ambiguity**. Controller binds the resulting artifact as **proto/tsconfig-paths-final @ <controller fills>**. A fresh e61d52b8 replay commits all 39 cumulative paths; do not apply the cumulative patch on an integrated prototype.

Plan files from 71a10b85:

```text
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/BUILD-MANIFEST.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/IMPLEMENTOR.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/MEASUREMENTS.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/OQ-paths.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REVIEWER.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/controls_gen.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/final_receipts.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/mutants.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/oracle.cjs
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/ownership_cost.cjs
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/verify_controls.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/prototype/P1.diff.txt
```

Message: **docs(paths): fold capped Opus review and independent ownership evidence**. No commits were made. Root VERIFICATION.md is ignored local custody and mirrors this manifest; tracked patch, source/archive and plan snapshots retain the fold. SNAPSHOT-HASHES.json indexes local custody; controller commits/external backup remain pending.
