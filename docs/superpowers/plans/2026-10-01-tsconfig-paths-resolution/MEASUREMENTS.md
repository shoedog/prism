# Lane P P1 r3 PARKED — current verification complete, acceptance failed

[MEASURED] Current-only receipts: target/repair-r3/current. **X/installed-X/R/T = 3,121/0/0/0**; X gains all CORRECT, installed oracle executed and yield FAIL. Installed base/head **33.2905/50.1396 s, 757,202,944/795,099,136 bytes RSS**; four configs decline through two falsely unsafe in-root non-type ordinary-main targets, 74 failing requests. One fresh pre-main control recovers 3,121 all CORRECT on the same tree. Four additional WRONGs reproduce in both grammars. Suites **4,917/5,110/5,133**, zero failures/one existing ignore each; fmt/clippy pass; matrix **170/0/0**; mutants **82/82 integration/unit and 68/68 kernel**; cache **105/61**; S1b-4 **411** identical controls; synthetic **487/503, 0 recovered/0 lost** versus freshly controlled r2d. Serial Nx wall ratios **1.1086/1.0488/1.1195**, RSS ratios **1.2508/1.0659/1.2364**: two RSS FAILs. See [REPAIR-R3-RESULTS.md](REPAIR-R3-RESULTS.md) for warnings, dispositions, exact numbers, provenance and limits; [REPAIR-R3-HANDOFF.md](REPAIR-R3-HANDOFF.md) records the required design stop. Earlier sections and pre-main receipts are historical.

# Historical Lane P P1 r2d verification

[MEASURED] Original comparison base a66b877f49ba858c27b749b0a36bfccf4bc7da7d; current controller custody HEAD 6965da75eb85f32d3ae0dda9e96704ad432375e6. The controller created WIP6965da75 during this repair. No repairer Git writes. Final source is dirty and frozen across1017 build inputs. Immutable binary: target/repair-r2d/head/prism-r2d-boundary, SHA256 **ab4fdc0091c53696dfe38239001c3d689f592fcd3546162030b5ad958fe8222c**. Evidence: target/repair-r2d/boundary-final. Earlier r2d receipts are superseded for current claims.

Cut1 uses the pinned TypeScript5.9.3 Node10 first-pass absence proof. It covers paths candidates and TS/declaration extensions, directory indexes, package types/typings/main/typesVersions, importer ancestors as node_modules files/directories, @types and custom typeRoots. Native source confirms custom roots follow node_modules and use declaration files/directories (typescript.js45322–45325,46573–46594); only node_modules/@types mangles scoped names (44284–44286). Full source ranges and digest are in src/js_paths_first_pass.rs and SPEC.

JS-only absence binds Exact. Any occupied, opaque, unread, outside or skipped first-pass location keeps the full base row; missing uninstalled module directories count absent. Root file stems record opaque sibling candidates. Physical occupancy through captured readable parents catches filesystem aliases without following unknown parents. Every enumerated occupancy probe enters cache dependencies. Metadata order, duplicate-last values, numeric keys, native version ranges, absolute normalization and ECMAScript whitespace follow the pinned source. Cuts2–6 and the r2c type-input classifier remain unchanged (preserved-rules.json). Cache remains **105/61**, with a semantic occupancy discriminator rather than a version bump.

## Counts and controls

X/R/T: **3121 /0 /0 changes**, across19219/953/61712 sites. Every X change is CORRECT_STATIC_BINDING. Final streams exactly match the independently native-classified streams; all1426 public input files were reverified unchanged. The existing native classification was reused after byte parity, rather than rerunning the oracle on identical inputs. public-final-parity.json.

**487 scenarios /503 sites /115 changes**:111 correct bindings,2 correct refusals,2 deliberately parked S6/OQ2 UNPROVEN ownership rows; zero preservation violations. Fifty new negative first-pass/boundary controls preserve the entire base row. Final native certification ran exclusively after earlier producers ended: controls-oracle-exclusive.log, controls-verify-exclusive.log and controls-classified.json.

Recovery versus r2c: **38 certified rows +1 parked =39** (37 bindings,1 refusal,1 parked). Original cut cost:39 correct rows (38 bindings,1 refusal) plus1 parked out of the original80 changed-control census. Restored from that cost:33 bindings,1 refusal,1 parked; four supplemental absence bindings account for the additional recovery. **Five correct bindings remain lost**, all retained cut2 JS export hops: C24/C56/C61/C82/C83 JSX. control-recovery-cost.json preserves the original denominator. [INHERITED] Controller F aggregates report2343→344 correct, a cost of1999; no new F claim is made.

S1b-4: **411 controls /639 sites /1234 artifacts byte-identical** to r2c, summary SHA256 b550a2c7466fdbe4d331f93843d44f0bfcfb6c62febf81f115c86a9ab64dd5ca. s1b-byte-identity.json.

## Suites, mutants and cache

Full default/MCP/all-features: **4897/5090/5113 passed**, zero failures, one existing ignored test each; no suite excluded. The existing ignore is resolution_test::slice_elem_variant_reserved. Default includes101 paths tests and4 first-pass units; native range fixture53 cases and the complete Unicode scalar population checked against25 native whitespace characters. suite-summary.json, focused-tests-from-full-suite.json.

**68/68 kernel and57/57 integration/library mutants killed by behavioral mismatches/assertion failures**; no compiler/setup/zero-test failure counts as a kill. Kernel uses exact immutable Cargo extern filenames; integration uses isolated targets. Nine first-pass location classes have direct both-grammar kernel witnesses, plus killed dependency, blanket-cut, metadata and boundary variants. mutants-summary.json, integration-mutants-summary.json, mutant-location-witnesses.json.

Cache105/61:34 native location cases,16 native boundary cases,8 lexical comparisons and12 normalization comparisons pass; same-environment preceding artifacts establish RED/GREEN. Old-cache rebuild, stable next hits and fresh/cached parity are checked. Retained r2c type-input cache8 and scanner8 cases/64 states pass. location-complete-cache/cache-summary.json, boundary-native-cache.json, lexical-native-cache.json, normalization-parity.json, type-input-cache/cache-summary.json and scan-cache/summary.json.

Historical invalid probes remain labeled in hypothesis-probe-result.log: mixed extern setup, one-edit paths omission protected by an independent gate, aliased-parent deletion expectation, and overlapping superseded producers. Corrected paired paths mutants disable both enforcement points. The alias probe now removes the opaque parent before expecting Exact. No production edit followed a probe-only error.

## Accuracy and serial performance

Immediate release rebuild then existing Tier-A matrix: **170/170 OK**. The uv launcher refused its default cache outside writable roots; the same installed CLI ran via Python without installation/network. Paired quick is **baseline-invalid on both** at C-method4/6, oracle error1/15, SUT error0. All28 successful SUT caller/callee outputs and pinned values match. No quick-green claim. tier-a-quick-comparison.json and raw paired receipts.

| Scenario | Base/final wall s | Wall ratio | Base/final RSS MB | RSS ratio |
|---|---:|---:|---:|---:|
| nx | 6.723/7.067 | 1.051 | 764.7/904.8 | 1.183 |
| nx_bundler | 7.273/7.348 | 1.010 | 768.4/770.4 | 1.003 |
| nx_wild | 7.247/7.651 | 1.056 | 769.1/891.9 | 1.160 |

All18 measurement children ran serially after own verification jobs ended; each produced64000 sites, expected Exact counts and pinned row hashes. All pass the unchanged1.20 wall/RSS bound. Quiet-host certification is unverified, so no exclusive attribution is made. perf/perf-summary.json.

## Limits and custody

Not verified: private F, independent post-repair review, quiet-host attestation, human-triggered full multi-corpus Tier-A, Linux case behavior or concurrent-tree security. The existing ignored test was not forced; clippy was not rerun. S6/OQ2 stays parked. No repairer commit/push, installation/network, baseline/threshold/cache-version change or external-backup claim.

CONTROLLER-paths.sh defaults to the corrected immutable binary and was checked with bash -n only. Controller must run F before acceptance. Files/messages: REPAIR-R2D-FILES.md. Source, binary and lean receipts are locally bound by production-binding.json, owned-files.tar.gz, essential-receipts.tar.gz and custody-final.json. Earlier settled/lexical receipts remain historical and cannot certify this binary.

## Controller F acceptance after repair r2 to r2d (private, aggregates only; 2026-10-02)

The controller built the head from `f5ef9c1e`.

**Result:**
- Changed rows: **2,313**, all `CORRECT_STATIC_BINDING`.
- 0 changed rows disagree with tsserver ownership.
- 0 keys added or removed.
- Refusals: JS_EXPORT_HOP 749, GUARD 22, UNCLASSIFIED 21, HOP 1.

**Cost:** 30 rows below round 1 (2,343). That is the disclosed price of the ambient-declaration and Node10 first-pass fences.

**Interim regressions, now superseded:** a blanket JS-target refusal measured 344 (r2c), and a type-input decline measured X 0 (r2). The controller ordered both cuts; both were replaced by faithful ports of the TypeScript rules.
