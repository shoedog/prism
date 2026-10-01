# Lane-P current build manifest — jsconfig precedence correction

MEASURED: plan `38519a0413d92882d9e20b884e9e01fc04fdb50d`; integrated prototype `304f24b2d2b0ab5780262a973429258bc5d2b2db` plus exactly two dirty paths below. Cumulative base **e61d52b8c6dfea9ad8db868797122a0bd067d27d**. Current receipts: **target/paths-plan/jsconfig-precedence/evidence/**. No Git writes, installs or private corpus reads. Prior rebind-e61 receipts and tracked `prototype/P1-3113e357-on-e61d52b8.diff.txt` are historical; they do not certify the corrected source.

READ controller-found defect: **a same-directory jsconfig barrier removed all F yield**. MEASURED public controls construct the same failure: both C72 grammars refuse before the fix and resolve afterwards. The old oracle also refuses both; new oracle admits both. No restoration of private yield is claimed.

MEASURED: resolver and oracle ignore jsconfig when tsconfig exists in the same directory. A visited directory with jsconfig and no tsconfig still refuses, including a strictly nearer config or no tsconfig anywhere. References/files-empty barriers retain their original code. TypeScript lookup-order citations are in oracle.cjs.

## Source and cumulative patch

MEASURED: 38 cumulative owned paths; patch replay from a fresh Gitless e61d52b8 archive equals every source hash. 1391 non-owned tracked build/source/test/eval inputs equal base. All 38 source hashes match before and after full suites. Integrated HEAD bytes equal the retained historical pre-correction source hashes, binding the old-binary control to the pre-change body. Source and binary receipts distinguish inherited parent-binary provenance from execution in this turn.

| Artifact | SHA-256 |
|---|---|
| target/paths-proto/P1.diff | 1b238c4d2cc96760c7e4db59b8cbe536835e08d8d383104cfbccd4488aebbc3b |
| prototype/P1.diff.txt (tracked current patch) | 1b238c4d2cc96760c7e4db59b8cbe536835e08d8d383104cfbccd4488aebbc3b |
| jsconfig-precedence/P1-owned-files.tar.gz | b2c9eb2ba1373a45b63d86853d61503803d949d131dd661bc7e17f2497faeba4 |
| evidence/source-hashes-frozen.json | a527fd4c81378abab7abdf012ff30f6dfbe0152d7fe17b1c9d083005d802309c |
| evidence/source-binding.json | 4d8b6080bb60f988ee82682eb21f89361a766280b887bc149da334dba456d584 |
| new immutable head/prism | c56c364608b44bb1d6c8ad4660870c257c41c2580f2f88d6868002620b757bbc |

MEASURED: cumulative additions: 836 production source lines, 691 test-code lines, 1538 declarative regression-data lines, 70 Tier-A fixture lines. Integration source: 693 physical lines. Per-path physical additions/removals: size.json. Cache versions remain 105/61 on base 104/60.

## Executables and compiler

MEASURED: fresh offline release build from the actual integrated prototype; copied immutable binary used for every new head measurement. Immediate release rebuild before matrix/quick uses identical source and identical binary bytes. Embedded version: `slicing 3.1.2 (304f24b2d2b0-dirty)`. Build logs and hashes bind actual source; the embedded dirty Git identity is not a clean-source claim.

| Role | Path | SHA-256 |
|---|---|---|
| retained base/facts | /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/rebind-e61/base/prism | d033ed8be086d2c6fd42b3721eed1e203bbfd8bc2be1d6dabe5436753fe0dd57 |
| retained base/facts | /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/rebind-e61/base/dump_imports | 1122fab7ca1177a645df61081ff868cda35bad06acb1ffd896bacb089cda9e36 |
| new head | /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/jsconfig-precedence/head/prism | c56c364608b44bb1d6c8ad4660870c257c41c2580f2f88d6868002620b757bbc |
| retained before | /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/rebind-e61/head/prism | 5194aedd270d539e1afa36d81c575a255b0bd9c9958e440653d08efd3f266e95 |

MEASURED: pinned offline TypeScript 5.9.3 bytes **3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675**. Parent/before binaries were retained, hash-checked and freshly executed in the same environment; not rebuilt this turn. The oracle comment-only citation-anchor change is bound in oracle-script-binding.json; public and control oracle certification were rerun against the final script bytes.

## Remeasurement

| Corpus | Sites | Correct changed bindings | Delta | Key / metadata changes |
|---|---:|---:|---:|---|
| X | 19219 | 3121 | 0 | 0 / 0 |
| R | 953 | 0 | 0 | 0 / 0 |
| T | 61712 | 0 | 0 | 0 / 0 |

MEASURED: every changed row is independently TypeScript-certified CORRECT_STATIC_BINDING. public/FINAL-SUMMARY.json, expected rows and input hashes retain proof. X retains eight NONRELATIVE_EXPORT_HOP and two UNCLASSIFIED_P1_PROOF candidates.

MEASURED: **156 controls / 170 sites / 57 changes**, **55 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL**, zero preservation violations. Old-head vs new-head complete-row comparison changes exactly two C72 rows; all other rows, including C73 strictly-nearer and existing solution barriers, are identical. Both grammars are pinned in verify_controls.py. Rust tests add malformed sibling and no-tsconfig edges; same-environment pre-change assertion RED, all 23 paths tests GREEN.

MEASURED: **21/21 kernel mutants killed**. M21 restores the same-directory barrier; both C72 requests change from lib/real to None. Reference and all mutants compile using release dependencies. Initial mixed debug serde variants were a probe setup error, excluded from evidence. Diagnostic retry cap three reached with three initial setup errors; a disclosed bounded extension repaired dependency selection. The first optional quick then failed on conflicting wrapper cache flags; a second disclosed bounded extension corrected flag composition and allowed one 900-second retry. All setup failures are inadmissible. No production-source correction retry was needed.

| Full suite | Passed | Failed | Ignored | Groups | Log SHA-256 |
|---|---:|---:|---:|---:|---|
| default | 4810 | 0 | 1 | 29 | 21bb03162498912b674b25048d5c197a1d730cbdc8e593c94b72e16f2b24a46b |
| mcp | 5003 | 0 | 1 | 31 | 484297d9eb9e69b17ea3df34da1010f7cd3405c4e05048e705ec8dc044a4b674 |

MEASURED: Tier-A matrix **170 ok / 0 regressions** after fresh release build. Formatter and diff checks pass; scoped MCP library clippy passes with 139 warnings. Existing ignored `slice_elem_variant_reserved` remains ignored. Tier-A quick completed in 311.49 seconds. Baseline validity: False; invalid reasons: ['corpus_sha_drift: 304f24b2d2b0 != pinned 20c8490591a3', 'stratum C-method: 4/6 successful probes']. Matrix 170 checks, 0 regressions. No pin, query, grade, oracle or baseline override.

MEASURED: controller script defaults point at the new immutable head; bash syntax and synthetic-public smoke pass: 170 sites / 57 correct changes, one aggregate JSON object, empty wrapper stderr. controller-public-receipt.json says actual_F_run=false. No private execution is represented by this smoke.

## Scope not reverified

READ: private F restoration, independent review round two, owner OQ1–OQ4, Git commits/push/merge and P2 remain controller/owner work. All-features Cargo, eleven integration mutants, cache acceptance, S1b-4 artifact equivalence, parent full Cargo suites and parent quick were not rerun this turn; their earlier rebind receipts are historical. Current default/MCP suites include existing integration/cache tests. Full multi-corpus Tier-A is human-triggered and not run. Runtime mutation/security/concurrency and large-tree overhead remain unverified. Quick validity failures are recorded without regression attribution; no same-environment base quick control was run.

## Controller files and commit messages

Prototype incremental commit from 304f24b2:

```text
src/js_paths.rs
tests/integration/js_paths_test.rs
```

Recommended message: **fix(paths): prefer tsconfig over same-directory jsconfig**.

Plan commit from 38519a04:

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
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/mutants.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/oracle.cjs
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/verify_controls.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/prototype/P1.diff.txt
```

Recommended message: **docs(paths): correct jsconfig precedence and refresh P1 evidence**. No commits were made. A fresh e61d52b8 replay commits all 38 cumulative paths; never apply the cumulative patch to the integrated prototype. target/paths-proto/P1.diff and source/evidence snapshots are ignored local custody artifacts. Current tracked patch is prototype/P1.diff.txt; the versioned patch is retained historical evidence. Snapshot hashes are indexed in target/paths-plan/jsconfig-precedence/SNAPSHOT-HASHES.json. Local snapshots do not replace controller commits or external backup.
