# PR-B repair R1 measurements

Status: authorized source/eval/doc repair and final evidence collection complete. All measured STOP conditions are clear on the admitted population; exclusions and unresolved rows remain explicit. This document supersedes historical b10 completion claims only where fresh evidence is stated. Review cap remains two rounds. No adoption, commit, push or F result is claimed. Clippy warning parity is not achieved (one new SMELL).

Source anchors: main/base `da0604b3bcaa983436f58921ba68386ddf48bb8a`; prototype `0660b3c5c1bb31a90320a74a53e1cd32cf2b3807`; docs HEAD `41de010c23d21f90a906a3cccd653b07b09a2e1a` (see entry.json for the authoritative full hash). Entry verified all 724 src/tests/mutants files, including untracked prototype files. No corpus package was executed; frontend-portal was never opened; no git writes.

Evidence: `/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r1/`. Frozen current production binary `bin/prism-head-r1-final`, SHA256 `f0f314f51de06166e543cf1df835577a39e1c1068b0cc67a1770c2dc0d696a5d`; bytes binary `bin/prism-head-r1-final-bytes`, SHA256 `95c032a792323bfc5f67435cfb5a00406db14627bf399feb8bbe3c0293281a9f`. Intermediate binaries are preserved as superseded. final-source-manifest.json binds source bytes.

## Repairs

| Finding | Fold |
|---|---|
| F1 | Nested same-binding writes enter RD as kill-only inputs, never emitted Defs or rows; named-control label parity and nested rebind controls. |
| F2/F3 | All JS/TS Defs, including assignments, have a binding environment; references are restricted to it. |
| W1/F4 | Reference-aware default/computed-key visibility; body Defs exclude defaults; TypeScript return-type formal references preserved. TS 5.9.3 cannot independently distinguish the parameter-default body-var scope; product AST controls cover it. |
| W2 | Complete member value read-role filter; writes refused, compound/update reads retained, receiver reads preserved. |
| W3 | Runtime enum competing binder and declaration token coverage. |
| W4/S5 | Raw/twin endpoint and alias-target maps keyed by lvalue start byte; two assignments on one line remain distinct. |
| W5 | All declarations must share the actual function/file/static-block/module variable environment before var equivalence. |
| W6 | Validated decoded escaped binder names; unrelated binders retain captures, matching binders fence them. PR-A refusal unchanged. |
| S1/S4/sol S1/O1/O2 | Synthetic owner reasoning/MCP surfaces documented; owner STOP-1(a), standalone O2 and anonymous location O1 resolved; dispatch anchored to prototype plus R1 patches; PB4 reference and b10 probe log reconciled. |

## W5-only re-adjudication

Frozen W5-only oracle snapshot, applied to all historical b10 public/SecBench changed rows: 483 jobs, 1,274,496 rows, zero verdict transitions. Xi missing old detail was recovered by running the exact original docs-HEAD adjudicator and comparing per-row identities/verdicts. Python/Go no-change controls require no adjudication; the nine historical failed SecBench producer pairs remain excluded. No W2 or product change is conflated with this delta. See w5-reaudit/summary.json, its retained per-job details and w5-xi-control.log.

## Current corpus rows

Entries are CORRECT / WRONG / UNDECIDED. Labels and endpoint owners/bytes are included in multiset identity; owner-only replacements are RE-OWNED, not ADDED+LOST.

| Corpus | ADDED | LOST | RELABELLED | RE-OWNED |
|---|---:|---:|---:|---:|
| X | 15,363 / 0 / 0 | 0 / 3,415 / 2 | 0 / 0 / 0 | 0 / 0 / 0 |
| Xi | 15,363 / 0 / 0 | 0 / 3,415 / 2 | 0 / 0 / 0 | 0 / 0 / 0 |
| T | 19,287 / 0 / 0 | 0 / 374,016 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| SecBench (574/583 pairs) | 255,643 / 0 / 16 | 0 / 620,345 / 2 | 1 / 0 / 1 | 0 / 0 / 0 |

X/Xi each: 63,523 base rows → 75,469 head. Byte-to-wire projection matches both sides; call sites identical. R1 interim capture had 50 LOST CORRECT TypeScript return-type rows; all repaired in the current binary. T: 714,977 base rows → 360,248 head; all 374,016 LOST are WRONG. Current X/Xi/T/SecBench LOST CORRECT is zero. SecBench: 2,128,722 base rows → 1,764,034 head, 481 changed packages adjudicated with no checker failure. All admitted call sites are byte-identical. Sixteen ADDED member rows are UNDECIDED due to collapsed mixed bindings; two LOST rows are unresolved (mixed bindings or missing collapsed owner). Both RELABELLED rows change NameOnly(CfgIncomplete) → NameOnly(SameLine), one checker-CORRECT and one UNDECIDED. No RE-OWNED row exists. These are checker-relative guarantees, not promotion of unresolved rows to CORRECT.

All 12 historical SecBench alias-twin WRONG rows are eliminated across the complete six-package residue population; each package has zero current ADDED/RE-OWNED WRONG and zero LOST CORRECT. See alias-residue/summary.json and full byte-bound details.

The reviewer’s X legacy cohort reconstructs exactly as 284 rows (20 Exact, 264 NameOnly); all 284 removed. Fourteen additional member rows under the same coarse criterion are also removed. See public-final/X.review-cohort-summary.json and retained source-bound cohort. Independent sampling uses a separate TS-AST implementation with no repaired oracle helpers: 80 LOST rows across four mechanisms; 61 confirmed WRONG, 19 inconclusive. No inconclusive row is promoted to CORRECT or WRONG. See public-final/X.independent-v2-sample-results.jsonl.

## SecBench producer exclusions

All 583 roots were attempted. A failed byte or call-site producer on either side excludes the whole pair. The same nine names were excluded historically; every recorded failure below is a 300-second bounded-producer timeout (exit 124). Partial output supplies no correctness credit.

| Package | Failed producer operations |
|---|---|
| command-injection/total.js_3.4.6 | base.bytes, base.sites, head.bytes |
| path-traversal/atropa-ide_0.2.2-2 | base.bytes, base.sites, head.bytes, head.sites |
| prototype-pollution/total.js_3.4.6 | base.bytes, base.sites |
| redos/cejs_2.0.20170212 | base.bytes, base.sites, head.bytes, head.sites |
| redos/clean-css_4.1.10 | base.bytes, base.sites, head.bytes, head.sites |
| redos/natural_5.1.0 | base.bytes, base.sites, head.bytes, head.sites |
| redos/react-native_0.63.0-rc.0 | base.bytes, base.sites |
| redos/three_0.122.0 | base.bytes, base.sites, head.bytes, head.sites |
| redos/vant_2.12.11 | base.bytes, base.sites |

Exact per-operation timing/status, both-side exclusion and binding hashes are in secbench-final/status.jsonl and final-audit.json. No timeout was re-baselined, and no successful subset is called a full 583-pair result.

## O1 SecBench conversions

Authenticated 97-entry callback-registration cohort; anonymous callable/parameter bytes validated, then location witness/frontier used without anonymous callees. Returned parameter bytes remain necessary for credit.

| Population | Base | Head |
|---|---|---|
| Standalone | 96 prism_error, 1 reached_function_only | 91 prism_error, 4 reached_function_only, 2 partial; 0 traced |
| Joint bare-read counterfactual | 96 prism_error, 1 traced | 90 traced, 2 partial, 5 reached_function_only |

The joint checkpoint inserts only bare reads into source-only package copies and retains anonymous identities. Its historical callable end-byte omission was repaired; failed stale-span runs are inadmissible. The 91 standalone errors have no bare references in their data parameters and lack source roots, not anonymous callee-query failures. O1-summary.json and the final standalone/joint bindings retain exact binary and inspection hashes.

## Navigation

Same seeded sample as the historical probe: X 60 source lines / 241 query pairs; T 20 source lines / 81 pairs. Callers, callees and repo-map outputs are byte-identical. All symbol/call/module payloads and non-DataFlow metadata are identical, including within changed queries. Five DataFlow differences have exact byte/owner/path/access removal proofs against LOST checker-WRONG rows; no additions or unexplained changes:

| Corpus | Query / location | Removal |
|---|---|---|
| X | nodes-at, charts/charts.parse.ts:61 | 1 Variable item |
| X | ego, charts/charts.parse.ts:61 | 2 nodes / 2 edges |
| X | ego, components/ConvertElementTypePopup.tsx:499 | 7 nodes / 7 edges |
| T | ego, services/services.ts:1787 | 14 nodes / 14 edges |
| T | ego, testRunner/unittests/moduleResolution.ts:477 | 3 nodes / 3 edges |

X locations are relative to packages/excalidraw. Saved base/head query outputs, inventories and proofs are in nav-X/ and nav-T/; removal-proof.json is the authoritative explanation. No STOP-1(a) failure in these samples. Unsampled navigation is not verified.

## Performance

Direct wait4 timings of uncached byte-dumper full CPG builds, two alternating base/head repetitions. Table shows minimum wall time and maximum RSS across repetitions. Other bounded static probes were running; these are contended measurements, not isolated benchmarks. Every producer completed with output and exit zero. Cached nav dfg-stats wall times are not used for build ratios.

| Corpus | Base → head seconds | Wall ratio | Base → head RSS MB | RSS ratio |
|---|---:|---:|---:|---:|
| X | 58.276 → 52.489 | 0.901 | 770 → 933 | 1.212 |
| T | 425.830 → 317.550 | 0.746 | 3,888 → 2,860 | 0.736 |
| lodash 4.17.10 | 147.966 → 127.512 | 0.862 | 656 → 280 | 0.427 |

RD functions_over_cap: X 0 → 0; T 0 → 0; lodash 2 → 2. Functions_without_cfg: X 1,751 → 3,708; T 7,398 → 10,081; lodash 766 → 956. All repetitions and counters are retained in perf/perf.json; own-child timer controls are recorded in PROBE-LOG P8. No measured wall ratio exceeds the historical 1.25 threshold; this does not establish isolated performance parity.

## Gates and limits

- Seven reviewer regression groups: 0/7 pass on unchanged prototype; 29/29 callback tests pass repaired head.
- Full cargo nextest --features mcp: 5,203 passed, 1 pre-existing ignored test (`resolution_test::slice_elem_variant_reserved`); doctests 2/2.
- SecBench Python 52/52; independent oracle controls 17/17 plus checkpoint custody tests 3/3 (combined suite 20/20). The original checkpoint fails two physical-span regressions and passes the refusal control.
- Authoritative new Rust guard mutants 10/10 killed; external eval/oracle/checkpoint guard mutants 6/6 killed; final advisory scoped mutgate 8/8 killed. Advisory diff selection excludes untracked prototype source files; authoritative text mode covers their guards.
- fmt passes. Clippy --all-targets --features mcp has no errors on either side: 235 warning occurrences on same-environment main/base, 236 on repaired head. One new nonminimal_bool warning at member-filter condition is a SMELL; warning parity is not claimed.
- Tier-A matrix 178/178. Quick: Excalidraw and SecBench VALID; Prism VALID on the single capped retry (query timeout 120s). Initial head and same-environment main/base controls timed out in incomingCalls before comparison, both retained as INVALID oracle evidence. No baseline changed.
- Initial uv launch failed outside-root cache initialization; initial time-l measurements failed in the timing wrapper's sysctl call. These failures are inadmissible product/performance evidence. Direct wait4 timer passed success/nonzero-exit/timeout controls. Superseded time-l exec ended with exit 130; its measurements remain excluded.

All six public producer pairs succeed, all call sites are identical, and byte-to-wire projections match. Black (20,539 rows) and Caddy (72,681) whole DFG outputs are byte-identical. Prism's 54,129 non-JS rows have identical raw-byte SHA256 on both sides; 15 added JS-fixture rows are independently adjudicated CORRECT. See public-final/non-js-control.json. No non-JS change or unexplained navigation delta is present in completed checks.

S3 filter sensitivity on X: zero CORRECT/WRONG/UNDECIDED transitions; one full-verdict metadata difference on a collapsed Module line mixing a plain write and two reads (still CORRECT). S2 multi-function warnings in the sampled O1 frontier population: one on base and one on head for standalone and joint; whole-X degradation was not measured. S6 introduces no further mandatory change; S7 remains inherited, unconstructed uncertainty. Legacy E4/E7/E11 defects outside the authorized repair remain, so zero ADDED WRONG is not a claim that every existing graph row is correct. TS 5.9.3's default-initializer scope blind spot remains disclosed; syntax-derived product regressions supply the independent control there.

STOP: none in completed admitted checker/control evidence. Private F, exhaustive grammar/navigation coverage, the nine excluded producer pairs, isolated performance, corpus runtime behavior, whole-X degraded-seed counts and independent review/adoption were not verified. Clippy warning parity remains an explicit unmet gate; tests are not re-baselined and no review-cap extension is claimed.

## Patch custody and controller commits

`repair-r1/R1-src.patch` is relative to committed prototype `0660b3c5c1bb31a90320a74a53e1cd32cf2b3807`: six files, 53,325 bytes, SHA256 `4f91af469142ab920695ecb561ac8a896ccffd4075d2f3cc924ddffcaeb0b354`. It changes ast_callback_identity.rs, data_flow.rs, callback_identity_tests.rs, optional_parameter_tests.rs, dfg_label_store_test.rs and mutants/js-param-defs.json. Suggested commit: `fix(js): preserve callback binding scopes and lvalue identity`.

`repair-r1/R1-docs.patch` is relative to docs HEAD `41de010c23d21f90a906a3cccd653b07b09a2e1a`, including eval/secbench O1 implementation/tests, corrected oracle/helper probes/tests, MCP documentation, SPEC/IMPLEMENTOR/historical supersession notices and this report. Suggested commit: `docs(eval): fold PR-B R1 review and seed anonymous SecBench sources`. Its authoritative final file list, size and SHA256 belong to external patch-manifest.json (the document cannot contain its own enclosing patch hash).

Both patches passed read-only apply checks against pristine git-archive controls. final-custody-check.json rechecks all 724 frozen source hashes and both frozen binary hashes, plus git diff --check. repaired-source-docs-checkpoint.tgz retains source/docs, frozen binaries, gate logs, exact patches and key evidence summaries/removal proofs; snapshot-manifest.json binds its bytes. Final regeneration includes the completed SecBench audit and handoff. No Git writes or commits were performed; controller commits are still owed.
