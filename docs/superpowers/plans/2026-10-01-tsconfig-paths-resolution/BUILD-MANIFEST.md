# Lane-P source and evidence manifest — main e61d52b8

MEASURED: plan worktree plan/tsconfig-paths at b26ed7152adb84a5ed6e50da513f7087e902b1a1; prototype proto/tsconfig-paths-rebased at 9c52a382466286aaa9d32fc6ee2225b495706e36 plus the three integration files below; exact parent **e61d52b8c6dfea9ad8db868797122a0bd067d27d**. Both trees were initially clean. No Git writes, installs, network requests or private F reads were performed. Current receipts: **target/paths-plan/rebind-e61/evidence/**. The 5048f443 / 95195882 / P1.diff 33f8fc33 packet and R1 receipts are historical, preserved under rebind-e61/historical-r1 and r1/final-evidence.

## Source and cumulative patch

MEASURED: **38 owned paths**, exactly target/paths-proto/P1-owned-files.txt. Current source, frozen source/archive and cumulative patch replay agree. Parent build inputs match git archive e61d52b8 for 1402 files; **1391 non-owned tracked build/source/test/eval inputs equal parent**. Native patch replay writes only a Gitless archive working tree. No Git store was written. source-binding.json and source-hashes.json bind these claims.

| Artifact | SHA-256 |
|---|---|
| target/paths-proto/P1.diff (against e61d52b8) | 192820a5a424342909341cac804ecb899f9bdcaf3fa01763fd8e2be199950420 |
| P1-owned-files.tar.gz | dd44ec4d5360fdd5819920a9d8e7dceb0bf2f261658e33a7221e2b4bcd42ab34 |
| source-hashes.json | e5090aa599764e2e77f49b52d74986f450d9558bdfaf43a05b8fff756c4c9d9a |
| P1-owned-files.txt | 4fbeb991abebece3fdd480a22dd1610c01039f4c7739e07849bd41379e3a9ecf |

MEASURED: honest added production source **832**, test code **631**, declarative test-data physical lines **1,538**, Tier-A fixtures **70**; integration source **633 physical lines**. Detailed physical additions/removals and test-module attribution: size.json. No numeric LOC cap is asserted.

## Integration decisions

MEASURED: removed two duplicate test field initializers, supplied two missing test initializers and the S1b-4 local namespace Callable initializer in src/ast.rs. Local namespace terminals have **via_unresolved_star=false**. The existing **namespace_identity rejects unresolved stars before any identity is inserted**, so namespace calls retain base. Explicit named exports override stars. The added regression exercises all three routes in both JSX/TSX and asserts complete relative namespace-row equality; I11 proves the terminal initializer assertion is causal.

MEASURED: resolve_js_namespace_exports and namespace_identity are byte-identical to landed main. Paths uses the landed extractor, module and positional binding classification, relative-module resolver, R4c span/wrapper/unique callable gates and existing export projections. No duplicated namespace or wrapper resolver was added. Landed JsExportFacts::is_empty retains complete namespace facts already; extra P1 empty facts have namespace_proof_complete=false. Alias-only skipped-star provenance remains necessary for the legacy named-export closure's different relative-row policy. Parent cache versions **104/60** become **105/61**, both pinned with changelog entries.

## Built binaries and compiler

MEASURED: base and head were freshly built offline in this environment. Base is the exact e61d52b8 source archive; head is the actual rebased Git worktree. Logs base-build.log and head-build.log name their source roots. Executables were copied before use and never replaced during measurement. Base embeds **b26ed7152adb-dirty** because the nested archive inherits enclosing plan Git metadata; this string is not parent provenance. Exact archive/build-input hashes bind base to e61d52b8. Head embeds **9c52a3824662-dirty** and is bound to the frozen source hashes. binary-binding.json records both.

| Executable | SHA-256 |
|---|---|
| /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/rebind-e61/base/prism | d033ed8be086d2c6fd42b3721eed1e203bbfd8bc2be1d6dabe5436753fe0dd57 |
| /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/rebind-e61/base/dump_imports | 1122fab7ca1177a645df61081ff868cda35bad06acb1ffd896bacb089cda9e36 |
| /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/rebind-e61/head/prism | 5194aedd270d539e1afa36d81c575a255b0bd9c9958e440653d08efd3f266e95 |

MEASURED: offline TypeScript 5.9.3 compiler SHA-256 **3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675**. BUILD-RECEIPTS.json binds probes/logs/results and the cumulative source bundle. Compiler version/bytes are asserted by the controller script. The TS2308/checkJs blind spot and conservative P1 cuts remain documented in SPEC/MEASUREMENTS.

## Fresh corpus and control results

| Corpus | Total sites | Callable alias candidates | Changed correct bindings | Added/removed keys | Metadata changes |
|---|---:|---:|---:|---|---:|
| X | 19219 | 3131 | 3121 | 0 / 0 | 0 |
| R | 953 | 0 | 0 | 0 / 0 | 0 |
| T | 61712 | 0 | 0 | 0 / 0 | 0 |

MEASURED: X/R/T **3,121 / 0 / 0**, delta **0 / 0 / 0** from R1. Every changed row independently TS-certified CORRECT_STATIC_BINDING. Fresh expected rows and 640/51/722 source-input hashes are in public/FINAL-SUMMARY.json. X retains eight NONRELATIVE_EXPORT_HOP and two UNCLASSIFIED_P1_PROOF candidate reasons; no cause is invented for the latter.

MEASURED: S1b-4 **411 scenarios / 639 sites**, all **1,234 dump/inventory/stderr/summary artifacts byte-identical head versus main**. Base/head summary also matches S1b-controls-s1b4-r5-impl.txt, SHA-256 b550a2c7466fdbe4d331f93843d44f0bfcfb6c62febf81f115c86a9ab64dd5ca. P1 **152 scenarios / 166 sites / 55 changes** = **53 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL**, zero complete-row preservation violations. Complete control and mutant tables are in MEASUREMENTS.

MEASURED: **20/20 kernel + 11/11 integration mutants killed**; each integration mutant compiled, selected one test and produced an assertion panic. Mutated source and logs are retained; setup errors/zero selections are inadmissible. Fresh base standalone harness **8 behavioral RED / 9 preservation passes**; same harness head **17 GREEN**. Cache probe passes cross-binary rejection; cold/full-hit/sidecar/no-cache equality; config/extends/candidate/package edits; unrelated .txt add/remove hit preservation.

## Fresh full suites

MEASURED: mutation builds precede suites serially. Every suite hashes all 38 files before/after and matches frozen source. No unrelated failure was rebaselined.

| Suite | Passed | Failed | Ignored | Groups | Log SHA-256 |
|---|---:|---:|---:|---:|---|
| default | 4808 | 0 | 1 | 29 | 417c80bf4c1a554b00fd2808f1ff857d13eac9079fe626a3c1a8982b22bc914c |
| mcp | 5001 | 0 | 1 | 31 | b09762564f868529fb285c41d29751987316351af3b6f4d438c8e9885060b47f |
| all-features | 5024 | 0 | 1 | 31 | 22c25d0d6fd70183b8f3697e31a9077b4a438a3742d4d7f5343ac8cced3220ca |

MEASURED: existing ignored integration::resolution_test::slice_elem_variant_reserved remains ignored. Matrix **170 / 0** after immediate head release build; fresh base matrix **166 / 0**. The new parent adds one matrix check versus the prior parent (165 + four P1 checks = historical 169; now 166 + four = 170). Installed Python 3.12 ran the harness directly with the bound immutable executable; no uv install or denied retry. Formatter passes; scoped MCP clippy passes with **139 library warnings**.

MEASURED: first actual-worktree quick reached the 300-second deadline without a complete result. MEASURED: second actual-worktree Tier-A quick **completed in 474.24 seconds** within its 900-second bound, return code 2. Its accuracy baseline is **invalid** for **corpus pin drift (9c52a3824662 != 20c8490591a3)** and **C-method stratum only 4/6 successful probes**. Oracle error rate 0.0667, SUT error rate 0; all 170 matrix outcomes are ok. No pin, query, oracle, grade or baseline override. quick-summary.json and quick-second/generated retain the actual report, raw run and inventory snapshot. No regression attribution is made without a same-environment base quick control; that control was not run. Base full Cargo suites were not rerun; fresh base corpus/control/cache/matrix/harness execution supplies the requested pre-change controls.

## Controller boundary and exclusions

MEASURED: CONTROLLER-paths.sh defaults to the new base/head/facts paths above, passes bash syntax and synthetic-public aggregate smoke: 152 scenarios, 55 correct changes, one JSON object, empty stderr. controller-public-receipt.json explicitly says **actual_F_run=false**. The wrapper's fixed corpus F label in that smoke is not private F evidence. READ: prior F aggregates 2,345/3,102 (75.60%) are controller-supplied on the old parent/body/oracle; fresh F is not certified. No private source/raw data was opened.

READ: fresh F, independent spec round 2, owner OQ1–OQ4 confirmation, Git commits/push/merge and P2 remain controller/owner work. Full multi-corpus Tier-A is human-triggered and was not run. Runtime mutation, snapshot concurrency/security and large-tree overhead remain unverified. Quick accuracy baseline acceptance is excluded for the two stated validity reasons. No buildable implementation work remains locally.

## Files and controller commits

MEASURED: prototype incremental commit from **9c52a382** contains exactly:

```text
src/ast.rs
src/js_exports.rs
tests/integration/js_paths_test.rs
```

ASSUMPTION commit message: **fix(paths): integrate P1 with S1b-4 namespace proof**. A fresh replay from e61d52b8 instead commits all 38 cumulative owned paths; do not apply cumulative P1.diff on top of 9c52a382.

MEASURED: plan commit contains these nine tracked paths:

```text
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/BUILD-MANIFEST.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/IMPLEMENTOR.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/MEASUREMENTS.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/OQ-paths.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REVIEWER.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/integration_mutants.py
```

ASSUMPTION commit message: **docs(paths): rebind P1 evidence to main e61d52b8**. Root VERIFICATION.md is excluded by .git/info/exclude:7 and is a snapshotted local report, not a tracked commit path. target/paths-proto/P1-plan-owned-files.txt lists the nine tracked plan paths. target/paths-proto/P1.diff, the source bundle/list/hashes and rebind-e61 evidence/snapshot archives are ignored local artifacts for controller external custody, not extra tracked plan/prototype changes. Final local snapshots: target/paths-proto/P1-evidence-rebind-e61.tar.gz and P1-plan-rebind-e61.tar.gz; post-write hashes in snapshot-rebind-e61-hashes.json. They include current binaries, source bundle, raw public rows/expected data, controls, mutation source/logs, full-suite logs and final quick outputs. Runtime caches, whole mutant build trees and compiled mutant drivers are excluded. Local snapshots do not replace controller commits or external backup; no Git write was performed.
