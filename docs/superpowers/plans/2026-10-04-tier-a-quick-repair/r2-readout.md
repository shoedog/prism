# R2 repair readout

Bound to `be7f82662b49d875a9555e0a167bf1c1013e37cf`, branch `feat/tier-a-quick-repair`. The worker transcript records `gpt-6.1-sol` (model-receipt.json). Changes remain uncommitted; controller owns Git writes. Evidence is `/Users/wesleyjinks/prism-evidence/meas/tiera/repair-r2`; R2.patch is relative to this exact base.

N1 and N2 are repaired, both SMELL follow-ups are folded, and the requested repair/eval/matrix/three-fresh-quick gates pass on this host. Both reviewer measurement corrections are confirmed by new live runs. No baseline or adjudication was changed.

## Fixes and evidence

- **N1, WRONG/MATERIAL:** both incoming-call filtering and member concreteness use the same declaration matcher. Native definition starts must match an explicit AST name token, initializer start/name, or contiguous overload-name position within that declaration group. Containment alone cannot admit a nested same-name initializer. Unsupported syntax keeps the existing exact-token fallback. Live tsserver definition fixtures against constructed incoming-family responses cover class/static field arrows, named property function expressions and a second overload, with base/sibling/override negatives; a same-line nested/sibling initializer control also passes. Every incoming drop remains in oracle_filtered.
- **N2, WRONG/IMMATERIAL:** Rust priming retains each non-timeout OracleError and re-raises it through the existing M2 per-probe handler. One error among 32 probes yields VALID 0.03125; four yield INVALID 0.125 through the existing 0.10 floor. Timeout/recovery refusal semantics remain intact.
- **R1, SMELL:** recovery repeats the original overlay/real-symbol capability policy after inventory agreement. Supported and unsupported restart controls pass; Markdown headers record retry/restart counts.
- **R2, SMELL:** environment identity records observed root/ancestor node_modules search paths and present paths. Manifest validation rejects ancestor dependencies as well as in-root dependencies. Both live corpora have no observed parent dependencies.
- diagnosis.md records predictions, alternative mechanisms, results and the two corrected inadmissible probes. This is the brief’s disclosed one-time round-2 extension; no further review round or artifact restart occurred.

## Corrected decision4 tables

`--lang ts,js --sample 12`, seed 42, tsserver 6.0.3; fresh native processes and SUT cache. 60 TS seeds / 120 probes and 55 Node seeds / 110 probes; both VALID, with every selected probe retained.

| Corpus / direction / tier | TP / FP / FN | Precision (95% Wilson CI) | Recall (95% Wilson CI) |
|---|---:|---|---|
| excalidraw-ts callers / raw | 65 / 7 / 112 | 90.3% [81.3, 95.2] | 36.7% [30.0, 44.0] |
| excalidraw-ts callers / exact_tier | 58 / 1 / 119 | 98.3% [91.0, 99.7] | 32.8% [26.3, 40.0] |
| excalidraw-ts callees / raw | 203 / 47 / 14 | 81.2% [75.9, 85.6] | 93.5% [89.5, 96.1] |
| excalidraw-ts callees / exact_tier | 178 / 31 / 39 | 85.2% [79.7, 89.3] | 82.0% [76.4, 86.6] |
| secbench-node callers / raw | 9 / 0 / 56 | 100.0% [70.1, 100.0] | 13.8% [7.5, 24.3] |
| secbench-node callers / exact_tier | 9 / 0 / 56 | 100.0% [70.1, 100.0] | 13.8% [7.5, 24.3] |
| secbench-node callees / raw | 11 / 2 / 35 | 84.6% [57.8, 95.7] | 23.9% [13.9, 37.9] |
| secbench-node callees / exact_tier | 11 / 2 / 35 | 84.6% [57.8, 95.7] | 23.9% [13.9, 37.9] |

The TS caller precision is 90.3% [81.3, 95.2], confirming the reviewer. TS Exact callers are 58/1/119, precision 98.3% [91.0, 99.7]. The TS filter drops 8 rather than 36 sites: 28 genuine class-field callers are restored, including 9 correct SUT edges previously scored as false positives. Node drops zero sites. Callees and Node directional counts are unchanged.

## Corrected 100-site member table

| Corpus / shape | Candidates | Sampled | Concrete | Found / missing | Excluded | Detection recall (95% Wilson CI) |
|---|---:|---:|---:|---:|---:|---|
| excalidraw-ts property_callable | 6118 | 100 | 63 | 10 / 53 | 37 | 15.9% [8.9, 26.8] |
| excalidraw-ts getter | 2282 | 100 | 5 | 0 / 5 | 95 | 0.0% [0.0, 43.4] |
| secbench-node property_callable | 277 | 100 | 84 | 0 / 84 | 16 | 0.0% [0.0, 4.4] |
| secbench-node getter | 37 | 37 | 15 | 0 / 15 | 22 | 0.0% [0.0, 20.4] |

TS property-callable recall is 10/63, confirming the reviewer; 60 rows are recovered and 37 remain excluded. Node property-callable recall is 0/84, also confirming the reviewer; 45 rows are recovered and 16 remain excluded. Getter results are unchanged. The former “most TS sampled members resolve to nonconcrete definitions” explanation is refuted. These measurements show low concrete TS member recall and zero detected Node members in this selected frame.

This is conditional site-detection recall, not incoming-set precision. Unsupported, abstract and external definitions remain explicit exclusions. Wilson intervals condition on sampled native answers; they do not adjust for clustering, stratified sampling or population weighting. The five-package SecBench sample does not represent all Node/JavaScript runtime behavior. Alias, dynamic and async/runtime truth remains outside native calibration.

## Three consecutive fresh quick runs

Each run immediately rebuilds release in this worktree, starts new native oracle processes, uses a new SUT cache and a manifest-exact external Rust source copy, and removes that copy’s generated target before the next run. Source manifests are checked before/after. All measurement receipts have identical harness/configuration hashes before/after and across runs. Whole wall excludes the preceding build.

| Run | Whole wall | Rust wall | TS wall | Node wall | Validity | Retries / restarts |
|---|---:|---:|---:|---:|---|---|
| quick1 | 318.840s | 165.532s | 149.099s | 4.178s | all 3 VALID | 0 / 0 |
| quick2 | 229.795s | 103.514s | 122.059s | 4.191s | all 3 VALID | 0 / 0 |
| quick3 | 257.851s | 126.805s | 126.383s | 4.616s | all 3 VALID | 15 / 1 |

## Gates and wider checks

| Check | Result | Evidence |
|---|---|---|
| Repair tests | 98 passed | repair-final2.log |
| Final exact-base RED | 15 failed; 14 behavioral/contract failures, 1 environment-schema failure; no new helper import failures | red-final2.log |
| Full eval suite | 1038 passed / 1 explicit opt-in live-adoption skip | eval-suite-final.log |
| Matrix, after immediate rebuild | 178/178 ok | matrix-build.log, matrix.log |
| Full Rust, all features / no-fail-fast | 5162 passed / 0 failed / 1 ignored, 31 targets | rust-suite-final.log |
| Full Node scripts, 44 files / concurrency 2 | 778 passed / 18 failed / 1 skipped, 797 tests | node-suite-final.log |
| Script Python | 57 passed + 82 subtests | scripts-python.log |
| Baselines, snapshots, adjudications, manifests and corpus configuration | 218 files byte-identical to base | anchors-unchanged.json |

The first wider-suite launches mistakenly supplied parent cache directories. Their missing-input failures are inadmissible for regression attribution. suite-environment.json derives corrected pinned installation paths from scripts/gate-inputs/pins.json; compiler and archive hashes were checked before corrected reruns. The full Node suite remains non-green with the same 18 native-membership failures reported in R1, all in membership.test.mjs; no out-of-scope repair or rebaseline was made. Rust, Node, examples and scripts source bytes are unchanged from be7f8266: these broader runs execute the same base artifact in this environment. No regression attribution to the eval-only repair is asserted.

## Files and controller commit

`fix(tier-a): match declaration bindings and preserve priming validity`

- Source: eval/tier_a/{ts_syntax.cjs,tsserver.py,member_sample.py,oracles.py,cli.py,corpus.py,report.py}.
- Tests: eval/tests/{test_r2_repair.py,test_r1_repair.py}.
- Documentation: r2-readout.md, the refreshed handoff.md, and supersession headers in this plan’s r1-readout.md/readout.md; VERIFICATION.md has current totals. Evidence delivery: repair-r2/R2.patch, delivery-source-snapshot/, delivery-receipt.json, handoff.md and reproducible run/readout scripts.

## Not verified / still open

- quick3 exercised live Rust recovery after 15 ContentModified retries and one incoming-call timeout; the new session passed inventory agreement and the capability recheck, then re-queried the complete sample. All three corpus summaries match quick1/2. Supported/unsupported recovery also has fixture controls. The upstream rust-analyzer ContentModified mechanism and other-host reproduction remain unsettled.
- Full human-triggered all-corpus evaluation, live Go/Python oracles, model/provider evaluations and mutation campaigns.
- The 18 broader Node native-membership failures and the reader-thread write-lock stall SMELL remain open; no whole-project-green claim.
- Statistical power, cluster/design correction and runtime/async truth beyond calibrated static native answers.
- Commit, push, independent acceptance and publication remain controller-owned. No Git writes, network acquisition or frontend-portal access occurred.
