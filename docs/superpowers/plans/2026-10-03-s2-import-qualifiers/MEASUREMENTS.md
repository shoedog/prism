# S2 measurements — provisional prototype; E5 design gate RED

**WRONG S2-W1, confidence 100/100:** a local alias `Alias = C` followed by `Alias.sm = replacement` changes the exported class object, but head still adds Exact for imported `C.sm()`. Actual source-bound base drops the site in both grammars; E5 requires retention. A Node same-object control returns replacement result 1 rather than original 0. The TypeScript checker still certifies the original declaration, demonstrating that static oracle correctness is insufficient to establish the independent E5 requirement. Receipt: `target/s2-plan/e5-alias-boundary/summary.json`. At the declared local cap, repeated new write-identity shapes are open-class: dispatch/adoption is parked for SPEC S2-O6 design. No source restart or further semantic correction was attempted.

[MEASURED] Planning HEAD `bc0fabb39231666a6b2d64faa314eb45585eae14`; immutable merged-P2 product base `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`; prototype is the uncommitted owned source manifest. No Git writes. Final frozen CLI/facts hashes, source inputs and exact receipts are in BUILD-MANIFEST.json.

[MEASURED] Final comparison: `target/s2-plan/verified-public/`. It uses immutable base streams freshly replayed in this continuation and byte-equal to S2-0/P2; the final folded head, head facts and TypeScript checker were freshly run. Before reusing base, the runner checks its binary hash and afterward requires identical source and all native config/input hashes. Every corpus completed; no source/config or site-population drift was admitted.

## Provisional public yield — native correctness, not admission acceptance

Counts are changed source rows, each one new singleton Exact from a dropped base site. Every change is CORRECT_STATIC_BINDING, with native module, caller ProjectService owner and full terminal span agreeing. All 358 measured changed rows across the two X snapshots were `ImportExternal` → Exact `ImportQualified`; no populated base row changed.

| Mechanism | X | Installed X | R | T |
|---|---:|---:|---:|---:|
| Named class static method | 23 | 23 | 0 | 0 |
| Named class static function-valued field | 156 | 156 | 0 | 0 |
| Declared namespace | 0 | 0 | 0 | 0 |
| Re-exported module namespace | 0 | 0 | 0 | 0 |
| Proven literal-object function member | 0 | 0 | 0 | 0 |
| **Provisional native-CORRECT Exact rows** | **179** | **179** | **0** | **0** |
| Lost base edges | **0** | **0** | **0** | **0** |

[READ] X and installed X share the same source snapshot and are not additive. Static method/field counts are call-site rows, not unique declarations. The last three mechanisms have positive native synthetic certification; S2-W1 prevents treating this prototype as a general admission certificate. In particular, public E5 alias/escape closure has not been audited with a sound identity mechanism. This table reports their actual corpus yield rather than their implementation availability.

| Complete-stream check | X | Installed X | R | T |
|---|---:|---:|---:|---:|
| All source call sites | 19,219 | 19,219 | 953 | 61,712 |
| Already-bound base rows, byte-preserved | 10,776 | 10,776 | 216 | 27,452 |
| Low associated S2 rows | 552 | 552 | 30 | 3,167 |
| Native-proven callable opportunities on low rows | 303 | 303 | 0 | 2,086 |
| Prior lane-P gain rows, byte-preserved | 3,129 | 3,129 | 0 | 0 |
| Native-certified changed rows rejected/unproven | 0 | 0 | 0 | 0 |
| UNJOINABLE across the whole call-site stream | 2,967 | 2,967 | 207 | 7,473 |

[MEASURED] The public UNJOINABLE records are `non_direct_or_unmatched_syntax`, not source-bound changed rows. They remain in the complete-site denominator and cannot certify a gain. The mixed-language control separately pins `site_fact_join`. Full per-site candidate/changed records, hashes and refusal buckets remain in the compressed evidence archive.

[MEASURED] Lane-P preservation receipt `lane-p-verified.json` compares every original reference/base/head key and requires all P2 bound rows unchanged. The 3,129 prior gain rows in each X corpus remain byte-identical. P2's published +8/+8/0/0 over P1 are an inherited source-bound baseline; preserving every P2 bound row preserves that subset too. The old supplied reference's exact source revision remains unverified; it is used only to identify the retained prior gain population, not as the S2 base. Fresh base streams are byte-equal to retained S2-0/P2 on all four corpora.

## Yield limits

[MEASURED] The native opportunity census remains X 299 named-class statics plus four call-result members; T 365 named-class statics, 1,485 declared namespace calls, 157 re-exported namespace calls and 79 other callable shapes. These are implementation certificates, not Prism's full-chain admission proofs. Instance calls are not admitted.

[MEASURED] T's final 707 facts files contain zero admitted module proofs. Native callable owners are compiler 952, services 531, testRunner 402, server 167, harness 14, typingsInstallerCore 9, tsserver 8, jsTyping 2 and deprecatedCompat 1. Compiler inherits rootDir/outDir without explicit exclude and requests node ambient types; other configurations also carry references and type-input costs. The retained default-exclusion guard is a demonstrated cut, not a complete diagnosis of every T row. A public root/output control pair differs only by explicit `exclude: []`: no exclusion keeps base; explicit exclusion permits a correctly owned gain in both grammars. SPEC S2-O3 asks for a separately designed ownership increment.

[READ] X's remaining native-proven static sites can fail the conservative module, class-key, cleanliness, write or callable capture proof; no complete per-refusal partition or recovery claim is made for those 120 rows. The four call-result opportunities remain outside this slice. The complete public multi-target base population remains unchanged; this prototype adds on drops and does not refine those rows.

## Oracle fix and private F

[MEASURED] Public mixed Rust plus JSX/TSX source reproduces the original fatal join: call-stats emits Rust sites while the facts driver emits JS/TS only. The fixed census records one `UNJOINABLE/site_fact_join` and continues. Source-position controls use UTF-8 BOM plus CRLF. Missing caller program, source position and unmatched/non-direct syntax also fail closed per site; a changed unjoinable row fails comparison. Empty streams, binary/probe/pinned-TS drift and source drift still fail globally.

[INHERITED — controller] F failed at native_oracle with an unavailable site/path join. This result is INADMISSIBLE and proves no yield. [UNKNOWN] The mixed-language explanation is likely, but F was never opened, and neither the repaired F census nor an F head comparison was run here. The corrected command is in README; only the controller may run it and publish aggregates.

## Final gates and custody

| Check | Final result | Receipt under target/s2-plan |
|---|---|---|
| Offline release + exact-lock facts driver | PASS; final source and package identity checked, immutable base tools preserved | verified-release-build.log, verified-facts-build.log, BUILD-MANIFEST |
| Full MCP nextest | **5,150 passed /0 failed /1 existing skipped**, 202.163s | nextest-verified.log |
| MCP doctests | **2 passed** | doctests-verified.log |
| Native synthetic prototype controls | **92 scenarios /92 sites /29 new CORRECT /0 loss /0 unproven** | native-controls-verified/summary.json |
| Additional E5 object-alias admission control | **RED: two WRONG admissions; static checker CORRECT on both, ownership/span agree** | e5-alias-boundary/summary.json, e5-alias-boundary.log |
| Source-bound oracle controls | **JSX 19 /TSX 20 candidates; mixed join, BOM/CRLF, empty/compressed inputs and wrapper negatives PASS** | controls/results.json, oracle-controls-reconciled.log |
| Advisory scoped mutgate, since bc0fabb, scope file | **7/13 selected, 7 admissible, 6 killed, S2-02 SURVIVED** | mutgate-verified/summary.json |
| fmt | PASS | fmt-verified.log |
| Same-environment base/head MCP all-targets Clippy | PASS; **371/371 warning instances, zero new** | clippy-base.jsonl, clippy-verified.jsonl, clippy-verified-comparison.json |
| Immediate-rebuild Tier-A matrix | **180 OK /0 regression /0 skip**, including two new fixtures | matrix-verified-build.log, tier-a-matrix-verified.log |
| New Tier-A fixtures on actual base/head | Base fails both; head passes both, retaining inherited shadow edge | tier-a-new-fixtures-red-green.json |
| S1b-4 controls | **411 controls /639 sites /822 byte-identical outputs /stderr 0** | s1b-verified.json |
| Complete public oracle/owner + P2 preservation | **+179/+179/0/0, no key/metadata drift, no lost or changed populated base row** | verified-public/, lane-p-verified.json |

[MEASURED] Scoped mutgate selected S2-01/02/03/07/10/11/13. S2-04/05/06/08/09/12 remain unselected because their source paths are untracked in this no-Git-write checkout. S2-07 is selected through its extra tracked-file mutation. The S2-02 survivor is a **SMELL: coverage gap**, not demonstrated product incorrectness and not proved equivalent. Authoritative all-13 coverage remains a controller gate after commit.

[MEASURED] One existing ignore is `resolution_test::slice_elem_variant_reserved`, reserved until a future SliceElem increment. No unrelated test failure was re-baselined or repaired. Same-environment controls isolated the new JSX priority defect and the new-fixture expectation mistake; their red/green receipts are retained. No production edit occurred after the final semantic verification build. The later alias boundary probe found S2-W1 without changing that source; the green full suite lacks this negative case and cannot establish admission safety.

[READ] Not established: full qualifier-object alias/escape write closure or admissible adoption; the extra E5 gate fails. Not verified: private F census/head acceptance, authoritative all-13 mutation coverage, independent review or policy adoption, Tier-A quick/full corpora, optional all-feature/detached-owner sweeps, Linux/case-sensitive/concurrent-tree behavior or quiet-host performance. Tier-A quick is required before the controller dispatches review; no review was dispatched here. Full corpus runs remain human-triggered. Final snapshots are local custody, not a commit, remote backup, push or merge.
