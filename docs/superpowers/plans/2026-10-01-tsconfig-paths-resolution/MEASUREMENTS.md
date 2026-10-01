# Lane-P measurements — W1 extension-priority residual

MEASURED: plan HEAD **2b3b9970**, writable prototype HEAD **32a5e893**; initial prototype tree **099d0e6e9e97cf7c81a2338fb15fe6a34b7b990f** exactly equals **bab21e62**. Both trees were clean before this fold. Three incremental prototype paths; **40 cumulative paths** against **e61d52b8c6dfea9ad8db868797122a0bd067d27d**. Current receipts: **target/paths-plan/extension-priority/evidence/**. No Git writes, installs, network/provider use or F reads.

READ: spec-confirm-opus.md found **1 WRONG / 0 SMELL**, a closed W1 extension-priority residual after the review cap. User authorized this targeted fold. No restart or additional independent review round; implementation/measurement cap was three rounds. The production change passed its first GREEN. The exhaustive table harness required two bounded expectation corrections, fully enumerated in hypothesis-probe-result.log; no production retry resulted from those setup assertions. **Fix A and owner-parked S6/OQ2 remain unchanged.** Previous spec-r2 receipts and the bab21e62 patch are historical.

## Source and binary binding

MEASURED: cumulative P1.diff SHA-256 **92e5c1212915f12049fdf3d02e583e1df572a4c415d58201dc59cf23cbb8da95**. `target/paths-proto/P1.diff`, extension-priority/P1.diff and tracked `prototype/P1.diff.txt` have identical bytes. Fresh Gitless archive replay on e61d52b8 reproduces all **40** owned hashes; **1362 non-owned source/test/eval/build inputs** equal base. source-binding.json records each parent hash or required absence and the resulting hash. Cache versions remain 105/61 on parent 104/60.

| Artifact | SHA-256 |
|---|---|
| P1.diff | 92e5c1212915f12049fdf3d02e583e1df572a4c415d58201dc59cf23cbb8da95 |
| P1-owned-files.tar.gz | b51c277201dbf8e582a1ce86e1e8e26a3c5a35fb2aec9352ff45e50525b23b91 |
| source-hashes-frozen.json | 36a4b0f88ff6b59ca7791a72cb083c0f930163897887d481079e3bd54de34483 |
| source-binding.json | 71b4fbc476c8e209280b1bf444d5c72b2a757c9a6a7fc3bcf9ae9cbec371ef83 |
| Immutable head/prism | 0c8de8336cc05f8678fafa303a468935c6cbc5ec8cfd5022e231b7c92ff5eb12 |
| Offline TypeScript 5.9.3 | 3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675 |

MEASURED: binary version `slicing 3.1.2 (32a5e8933889-dirty)`; the dirty Git identity is supplemented by the source hashes, replay and release-build receipt. CONTROLLER-paths.sh defaults to `target/paths-plan/extension-priority/head/prism`. before/prism was rebuilt here from the pre-change production source at 32a5e893 before editing the barrier. Base prism and import-facts binaries retain inherited rebind-e61 build provenance: their hashes match spec-r2 and both were freshly executed in this environment; they were not rebuilt. binary-binding.json binds all four executables.

MEASURED physical additions against e61d52b8: **914 production / 756 test-code / 2166 regression-data / 70 Tier-A fixture lines**. Integration source has 755 physical lines; size.json has per-path counts. No LOC cap is implied.

## Priority port and RED/GREEN

READ: TypeScript 5.9.3 `typescript.js:22530–22544` defines allSupportedExtensions; `43831–43843` applies wildcard dedupe; `43966–43997` implements higher/lower-priority handling. Production ports the three groups generically, chooses the longest compound suffix and barriers earlier same-stem occupants. `.d.ts` does not suppress `.js`/`.jsx`; exact `files` entries remain authoritative. Existing conservative cross-group MJS/CJS versus TS/TSX barriers remain. Occupants need not themselves be matched, so this is a conservative proof barrier.

MEASURED: **four C85/C86 RED rows**, one missing pair in each target grammar, and **two scenario-O dropped callers** mint Exact on the freshly rebuilt pre-change binary and equal complete base rows after the fix. Both pair tests fail on pre-change production and all **27 paths integration tests** pass after. C87 scenario O retains its included app.ts Exact target `pkg/wrong/util.ts:f`, while app.tsx and b.jsx have inferred ProjectService ownership, undefined alias targets and complete base preservation. C88 covers both literal-file exemptions in both target grammars; C89 covers both declaration/JavaScript exceptions. priority-red-population.json and RED/GREEN logs retain behavioral outputs.

MEASURED: extension_priority_check.cjs extracts the compiler's actual table and checks every ordered pair with wildcard and literal membership in both target grammars: **64 cases; 26 actual TS drops refused; 32 literal exemptions; four declaration/JavaScript exceptions; two conservative declaration barriers; zero errors**. The pre-change source-bound kernel fails **eight rows** across all four changed priority behaviors. TypeScript's suffix stop only special-cases .d.ts: its sorted walk retains .d.cts beside .cts, while later .mts removes .d.mts. The final generic barrier conservatively refuses the retained declaration pair. No exact TS membership-equivalence claim is made.

## Fresh remeasurement

| Corpus | Sites | Changed rows | Classes | Ownership disagreements sites / files | Changed disagreements |
|---|---:|---:|---|---:|---:|
| X | 19219 | 3121 | {'CORRECT_STATIC_BINDING': 3121} | 6 / 2 | 0 |
| R | 953 | 0 | {} | 144 / 17 | 0 |
| T | 61712 | 0 | {} | 15 / 5 | 0 |

MEASURED: **X/R/T = 3121 / 0 / 0**, all public keys and metadata unchanged. X retains **8 NONRELATIVE_EXPORT_HOP + 2 IMPORT_FORWARD_NOT_FORWARDABLE**, **0 UNCLASSIFIED**. Input hashes, positional target identities, expected rows and binary hashes are in public/FINAL-SUMMARY.json and its linked artifacts. Unchanged ownership disagreements remain unchanged rows.

MEASURED: **197 controls / 213 sites / 72 changes**: **68 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL + 2 UNPROVEN**. Every Option-K preservation control equals the complete base row. The two UNPROVEN gains are existing C80/S6, pending OQ2; they are not waived or certified. Scenario O's dropped callers preserve base.

MEASURED: **30/30 kernel mutants killed**, including M29 (.tsx/.ts order) and M30 (.jsx/.js order), each with two target-grammar witnesses. M14/M16/M22/M23 were rebound to the generic implementation. **11/11 integration mutants killed**, each compiling, selecting one test and producing an intended assertion failure. Separate mutation target directories avoid suite interference. Setup assertions and guessed optional-path failures are recorded as inadmissible in the hypothesis log, not counted as behavioral results.

| Full suite | Passed | Failed | Ignored | Groups | Log SHA-256 |
|---|---:|---:|---:|---:|---|
| default | 4814 | 0 | 1 | 29 | 5793b24e4c9d82eff4fd52a34c17584b55525db18252eb747c78f944cebb4eb5 |
| mcp | 5007 | 0 | 1 | 31 | 584a7d20f7d9e3061f09ba0852737e6615f44c49fae4e4886e797390d9ccc507 |

MEASURED: the ignored test in each suite is `resolution_test::slice_elem_variant_reserved` (SliceElem reserved until a future slice). **Tier-A matrix 170 ok / 0 regressions** after an immediately preceding release rebuild in the actual prototype, using installed Python 3.12 `-m tier_a.cli` without uv cache writes or installs. Formatter and both diff checks pass; scoped MCP-library clippy passes with **139 warnings**. **411 S1b-4 controls / 639 sites / 1234 artifacts are byte-identical to e61d52b8**, and both summaries equal the committed r5 reference SHA-256 **b550a2c7466fdbe4d331f93843d44f0bfcfb6c62febf81f115c86a9ab64dd5ca**.

MEASURED: updated controller wrapper smoke uses only these public synthetic controls, emits one JSON object with empty wrapper stderr, 0 UNCLASSIFIED and the fresh head hash, and deliberately exits **1** for the two S6 UNPROVEN gains. **actual_F_run=false**. Its fixed corpus label is not an F receipt.

## Not verified / remaining authority

UNKNOWN: **F was never opened or remeasured**; the controller must privately run the updated wrapper and rebind its aggregates. S6/OQ2 remains owner-parked. No new independent review, owner acceptance, Git commit/push/merge or P2 execution occurred. **All-features/detached-owner-audit Cargo, Tier-A quick/full multi-corpus, dedicated cache acceptance, base full suites, dedicated runtime mutation, security, concurrency stress and large-tree performance checks were not rerun.** Requested default/MCP suites, matrix, mutants, public controls/rows and S1b identity were all freshly executed. The prior quick pin/probe problems are historical, not a fresh failure or validity claim.

## Controller commit files and messages

Prototype incremental files from writable HEAD 32a5e893 (initial tree equals bab21e62):

```text
src/js_paths.rs
tests/integration/js_paths_test.rs
tests/integration/fixtures/js_paths_priority.json
```

Message: **fix(paths): apply TypeScript root-file extension priorities**. Controller must commit/re-squash and fill the new `proto/tsconfig-paths-final` SHA in IMPLEMENTOR. A fresh e61d52b8 replay owns all 40 cumulative paths; never apply it to an integrated prototype.

Plan files from 2b3b9970:

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
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/extension_priority_check.cjs
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/mutants.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/verify_controls.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/prototype/P1.diff.txt
```

Message: **docs(paths): refresh extension-priority evidence and dispatch**. No commits were made. root VERIFICATION.md mirrors this current manifest; local owned-source, plan and evidence archives plus SNAPSHOT-HASHES.json preserve custody. Controller commits/external backup remain pending. Evidence archive omits copied mutant repo trees and executable drivers; source copies, behavioral failure logs, changes and summaries remain, and before/head binaries are retained separately.

## Controller verification of the extension-priority fold (2026-10-01; no further review round, per the reviewer's convergence note)

- **Patch replay:** `prototype/P1.diff.txt` (sha `92e5c121`) applied to `e61d52b8` reproduces cumulative `ed374d97` exactly.
- **Release binary:** the controller built it from `ed374d97` (version `ed374d9752b2`).
- **Scenario O (synthetic, explicit `moduleResolution: node`):** `app.ts` and `m.js` bind Exact to the right target. `app.tsx` and `m.jsx`, which a same-name `.ts` / `.js` displaces, keep base UnknownName. The positive control without `moduleResolution` binds nothing (Node10-only scope); that probe was not counted.
- **Private F with the controller-built head:** 2,345 changed, all `CORRECT_STATIC_BINDING`; 0 changed tsserver disagreements; 0 keys added or removed. Refusals: HOP 717, GUARD 23, UNCLASSIFIED 21.
