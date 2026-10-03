# Current P2 checkpoint — final targeted fold

[MEASURED] Fold starts at `proto/tsconfig-paths-p2 @92f7d152`; the exact plan patch parent is `2821f1a8`. W1 was fixed by a7c77f4e and fmt by92f7d152. W1b closes the remaining depth exit in the existing artifact. Starting implementation commit is the controller's cumulative squash of `c50de85a..<final proto>` on `proto/tsconfig-paths-p2-final`; controller fills SHA. Cache stays 106/62.

Cap 2/2 reached, converging: owner-authorized targeted W1b/S1b/S7 fold on the existing artifact; no further spec round, controller verifies directly.

[INHERITED — owner/controller aggregate] Post-W1 F re-run completed with **+629 CORRECT_STATIC_BINDING unchanged**, impact0. This closes P2-R1-F. Post-W1b frozen-binary F verification remains assigned to the controller; the worker never opens private inputs or raw receipts.

[MEASURED] The expanded legacy regression selects A1/A4/K1/K2 layouts in JSX/TSX, pkg/no-tsconfig and paths-alias variants, and both star orders:24 cases /48 Call+JSX rows. On unchanged92f7d152 production, K1/K2 each have8 wrong sibling variants while A1/A4 have0 mismatches; after the fold all12 P2 groups pass. All4 scoped W1/W1b mutants are admissibly killed; W04 reverts only depth BlockedClaim and reproduces K1/K2=8/8 with A1/A4=0. `target/p2-fold2/red.log`, `red-binding.json`, `green.log`, `mutants/results.json`.

[MEASURED] Full MCP nextest **5,133 passed /0 failed /1 existing skip**, MCP doctests **2/2**, **fmt PASS**, same-environment Clippy control at 92f7d152 and fold each **371 warning emissions /232 normalized unique /0 new or removed**. Immediate same-worktree release rebuild plus Tier-A matrix **178 OK /0 regression /0 skip**. `nextest-mcp.log`, `doctests.log`, `fmt.log`, `clippy-comparison.json`, `matrix-summary.json`.

[MEASURED] Frozen binary `/Users/wesleyjinks/code/prism-paths-p2-plan/target/p2-fold2/bin/prism-p2-fold2`, SHA256 `e29981a9c89cff8db0e5005ca6ddcd1427814fa3e2ce0089862bf9995162b8a0`; default recorded in the plan's `CONTROLLER-p2.sh`. `binding.json` binds production/vendor/build/test/fixture inputs. Fresh complete public X/installed-X/R/T remains **+8/+8/0/0**, all16 new rows **CORRECT_STATIC_BINDING**, every P1 gain retained, no key/site metadata drift. Full populations19219/19219/953/61712; fact inputs628/628/50/707 and native inputs633/2219/48/606 rehashed. S1b-4:411 controls /639 sites /822 byte-identical comparisons /stderr0. H1 probe:2 grammars /10 states /2 sites per state, all parity/invalidation checks pass. Receipts: `target/p2-fold2/public/summary.json`, `s1b.json`, `h1-cache/summary.json`. S5 cache coverage is the `nonrelative-cache.py` probe runner plus receipts, not a product integration test.

Not verified here: F, Tier-A quick/full multi-corpus, separate default/all-features/opt-in detached-owner-audit sweeps, Linux/case-sensitive/concurrent-tree behavior, quiet-host resource attestation, forced existing skip, independent review, Git/publication or external backup custody. Historical evidence below is source-bound to earlier artifacts.

---

# Historical P2 checkpoint — Opus spec review round 1 fold

Current authority: plan **bb4e4743**, reviewed cumulative prototype **b9fd3775** on main **c50de85a**. Start implementation from the final cumulative prototype the controller squashes; old incremental/gitless adoption instructions are historical. Cache **106/62** is already present. Review **1/2**, FIX: one closed WRONG (W1), S1 documentation refresh, S2–S6 disclosures. Repair gate cap **2 attempts**. No worker Git writes or F reads.

[MEASURED — controller acceptance recorded at bb4e4743; supplied evidence] **F +629**, all CORRECT_STATIC_BINDING, **628 JS_EXPORT_HOP +1 NONRELATIVE_EXPORT_HOP**, **0/0 keys added/removed**, **419 member-written**, **697/697 full-chain span agreement**, **98/98 unresolved retained**. All **350 ABSENT** rows are `NONCONTRIBUTING_STAR_BRANCH_FULL_BARREL_BINDS_TERMINAL`. F total **2,942 = 2,313 +629**. These are measured controller results, not this worker's private execution. OQ2/OQ4/OQ5 are closed for b9fd3775. **[INHERITED — owner/controller aggregate] The post-W1 F re-run is complete: +629 CORRECT_STATIC_BINDING unchanged, impact0.** Final post-W1b binary verification remains controller-owned.

## Round-1 disclosures and reviewer evidence

Evidence: `/Users/wesleyjinks/prism-evidence/paths/reviews-p2/spec-r1-opus.md` and its `spec-r1-opus-probes/RESULTS.txt`; 33 same-environment base/head/TypeScript probes. Base c50de85a / head b9fd3775; TypeScript 5.9.3 SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. These are supplied review receipts unless a fresh command is recorded below.

| Finding | Disclosure / disposition | Review evidence |
|---|---|---|
| W1 WRONG | Unresolved non-relative ImportForward must be BlockedClaim rather than discarded NoTarget. Restore complete legacy base refusal; retain alias yield. | A1: base UnknownName, head wrong Exact b.ts:k. A4: base UnknownName for call/JSX, head wrong Exact b.jsx:K; TypeScript binds k2.jsx with no diagnostics. D5 alias unresolved package stays refused. |
| S1 SMELL | Current dispatch, amendment, OQ, measurements and handoff now bind the cumulative prototype and measured F acceptance. Historical e80fbf54 instructions do not authorize double application. | Review S1 vs recorded controller acceptance of b9fd3775. |
| S2 SMELL | **3 resolvable non-relative rows remain unexplained and unrecovered.** 697 = 599 resolvable +98 unresolved; 628 recovered JS_EXPORT_HOP minus32 relative =596 non-relative, leaving3. Full749 =628+98+18+2+3. Option K preserves them; controller may supply their gate class later. | Review S2 arithmetic against controller aggregates. No invented gate class or recovery claim. |
| S3 SMELL | Inherited type-only, broken named/forward and absent-supplier star treatment may discard a competing claim. F shows **0 affected changed rows**, as all changed rows are certified. Owner disclosure; separate type-export/claim-without-supplier design required. | D2/D2b type-only: TypeScript UNBOUND/TS2308, head Exact sibling. D3 broken ReExport and D6 broken alias ImportForward: UNBOUND, head Exact sibling. Base E2/E3/E4 and A5/A6 demonstrate inheritance. D1/D4/D5 value/CJS/unresolved package are refused. Types erase in D2; D3/D6 fail linking; TypeScript rejects this class. |
| S4 SMELL | v106 comment now describes both Node10 hop proof and CallGraph serde format change: module values `(String,bool)`→`(String,String)`, export keys `bool`→`String`. No second version bump: shipped main is105, prototype106 unreleased. | Review S4; only unreleased e80fbf54 caches shared106 before the format change. |
| S5 SMELL | Coverage is the `p2-probes/nonrelative-cache.py` probe runner plus receipts at `target/p2-fold2/h1-cache/summary.json`: cold/warm, barrel-ancestor package declaration add/remove, and caller paths edit to a different terminal. This is probe evidence, not a product integration test. | Review H1 real cpg-cache.bin passed all four states; previous production coverage was loader topology unit control only. |
| S6 SMELL | Export tables are recomputed per caller config path, including configs with identical effective options. Monorepo cost scales with admitted config count; F completed, no measured cost problem. Effective-options fingerprint is a separate optimization, not a P2 gate. | Review apply_js_paths table key; options would include paths/baseUrl origin/allowJs/typeRoots/moduleResolution. |

## Repair verification

[MEASURED] Applied prototype **a7c77f4e** over **b9fd3775**, verified in the clean physical prototype checkout. One full MCP nextest **5133 passed /0 failed /1 existing skip**, doctests **2/2**, A1/A4 **8 cases /16 Call+JSX sites per revision RED on b9fd3775 /GREEN on a7c77f4e** in both grammars/orders, scoped mutants **10/10** killed (7 nonrelative +3 W1), matrix **178 OK /0 regression /0 skip**, S1b-4 **411 controls /639 sites /822 byte-identical comparisons**, H1 **2 cases /10 states** all pass.

[INHERITED] The applied-fold verification originally found **fmt FAIL** in new W1 tests; the controller adopted the formatter correction as **92f7d152**, so formatting adoption is done. That older Clippy comparison was **371 warning emissions /229 unique /0 new**, with exact unchanged-source mapping. Final W1b gate receipts below supersede those bytes for current status.

[MEASURED] Fresh complete public X /installed-X /R /T vs P1 final **+8 /+8 /0 /0**, all changed rows **CORRECT_STATIC_BINDING**, no key/site metadata drift; populations **19219 /19219 /953 /61712**. Every new row has fresh native caller checker/ownership certification; facts628/628/50/707 and native inputs633/2219/48/606 rehashed. X/installed-X represent one snapshot and are not additive.

[MEASURED] Frozen release: `/Users/wesleyjinks/code/prism-paths-p2-plan/target/p2-applied-verify/bin/prism-p2-a7c77f4e`, SHA256 `55e7e32b9eaf5e3f85ba2c9a109acde87c622ffe5aaff9459268395766eb0f4b`. Immediately rebuilt release before matrix has identical hash; all1226 source/test/vendor/fixture inputs unchanged. `target/p2-applied-verify/summary.json`, `VERIFICATION.md`, `binding.json`, `owned-snapshot.tar.gz` retain receipts. Only prototype checkout exists, so the plan path/hash and these record updates are prepared against45c6d124 for controller adoption; no Git writes or F reads.

[INHERITED] Post-W1 repaired F impact is measured by the controller: +629 CORRECT unchanged. [UNKNOWN] Final post-W1b F verification remains controller-only and unmeasured here. Not run: broad Tier-A quick/full, separate default/all-features/opt-in detached-owner-audit sweeps, Linux/case-sensitive/concurrent-tree/quiet-host resource behavior, forced existing ignore, independent review or Git/publication. Historical receipts below remain source-bound to earlier artifacts.

---

# Historical P2 checkpoint — non-relative export hops

[MEASURED] Prototype built in the exact gitless e80fbf54 tree at `target/p2-nonrelative/work`. Physical checkout remains plan/tsconfig-paths-p2 @35c481cf; no worker Git writes or F reads. Frozen final binary: `target/p2-nonrelative/bin/prism-p2-nonrelative-verified`, SHA256 `df0cca2f4a1d79be5ff33a55dff5ac1b71ae53106d47612385f44d5a6d0f1d85`. `final-build-binding.json` binds production/vendor/build inputs; final custody refresh is in HANDOFF.

[MEASURED] Public native controls: **82 scenarios /168 sites /72 new CORRECT /96 preserved**, including both grammars, member writes, alias/relative chains, caller-options separation, noncontributing and unresolved star branches, package/declaration/ambient/competition/explicit-substitution refusals, CJS no-span and export= refusal. Receipt: `target/p2-nonrelative/controls-verified/summary.json`. **S1b-4 411 controls /639 sites /822 outputs byte-identical**, stderr0: `s1b-verified.json`.

[MEASURED] Complete same-environment public base/P1/P2 streams: X **19,219 sites /+8 new CORRECT**, installed-X **19,219 /+8 new CORRECT**, R **953 /0 byte-identical**, T **61,712 /0 byte-identical**. Site population and metadata unchanged; all original P1 gains retained, X total gain3129 against original base. X and installed-X are one source snapshot, not additive. Every changed row was freshly certified by the actual caller ProjectService/checker. Fact hashes rechecked628/628/50/707; native inputs633/2219/48/606. Receipts:`public/summary.json`. Post-test rebuild replays complete head rows and rehashes native inputs before reusing those exact certificates:`public-verified/summary.json`; all four rebuilt streams are byte-identical to their certified predecessors.

[MEASURED] Product RED on exact e80fbf54 with final controls: **7 groups pass /3 new positive groups fail**; added library dependency-prime control **0 pass /1 fail**. Same-environment b11 old assertion **1 pass**. Initial library-only integration probe was INADMISSIBLE and corrected. Final **10 regression groups GREEN**; **full unfiltered MCP nextest 5,131 passed /0 failed /1 existing skip**, plus **2 MCP doctests passed**. Original skip is `resolution_test::slice_elem_variant_reserved`. Three verification rounds converged after two bounded corrections; cap3 reached GREEN, no extension. No failure outside this task's expectation change was found.

[MEASURED] **7/7 admissible scoped mutants killed**, all behavioral: alias-hop refusal, shared project options, omitted Node10 priority pass, ambient fence, alias-as-relative lookup, restored extraction literal gate, omitted alias priming. Incremental builds in isolated copies; unmutated10-group reference GREEN. `mutants/manifest.json`, `results.json`, per-mutant logs. **fmt PASS /MCP all-targets clippy PASS** (existing warnings; no new resolver/test warning). Immediate same-tree release rebuild + **Tier-A matrix178 OK /0 regressions /0 skips**, including four new fixtures. `gates.json`, `tier-a-matrix.log`, `matrix-rebuild.log`, `nextest-mcp-verified.log`, `doctests.log`.

[MEASURED] Public diagnostic-partition seam controls: **8 ABSENT = noncontributing star branch**, complete barrel/site binds the exact callable through the other alias branch; **8 unresolved /8 preserved at P1**. All16 actual site import symbols agree with native callable spans. `controls-verified/audit-seam-summary.json`; origin explicitly synthetic, no F claim. CommonJS no-span and export= controls remain at base. ESM alias import-forward functions bind; imported arrows/written bindings remain refused.

[INHERITED] F's prior +32 relative gains and 697 nonrelative population are supplied aggregates, preserved in historical sections below. [UNKNOWN] Final F recovery, explanation of the exact350 ABSENT and retention of the98 unresolved rows await controller `CONTROLLER-p2.sh ... NEW_P2_BIN PRIOR_GAP_EVIDENCE_DIR`. Public controls prove a noncontributing star branch can be ABSENT while the full barrel/site binds the other branch's exact callable; they do not prove this is all350. The aggregate audit distinguishes that mechanism from CommonJS/export= and unresolved explanations.

Size forecast: **80–140 source /200–280 tests /70–100 fixtures LOC** plus bounded measurement runners. Cache stays106/62. Private F, independent review, Linux/case-sensitive/concurrent-tree behavior, full multi-corpus Tier-A and quiet-host resource attestation have not been verified; owner-tier work uses matrix, not another broad quick/full sweep.

---

# Historical P2 result — diagnosis only

INHERITED, controller aggregate supplied 2026-10-03: **32** changed rows, all `CORRECT_STATIC_BINDING`, all `JS_EXPORT_HOP`; **24** member-written; **0** keys added/removed. Native relative JS-resolution ceiling **33**. Against **749** native callable, owner-agreeing bucket rows this is **717** actual unrecovered rows (roughly **716** outside the ceiling). P2 as built is **not material**. The earlier opportunity-based materiality decision below is superseded.

MEASURED: this clone is now `proto/tsconfig-paths-p2` at `e80fbf541d53ace5c547d5f26c9135e46e7be12b`; predecessor plan docs are `plan/tsconfig-paths-p2` at `1cb46b80`. This round changes diagnostics/docs only. [P2-GAP-DIAGNOSIS.md](P2-GAP-DIAGNOSIS.md) specifies the controller-only exact-gate partition, public shapes and ASSUMPTION forecasts. Private per-class counts remain UNKNOWN pending that run. No production or Git writes and no private reads.

---

# Historical P2 prototype checkpoint — relative JS export hops

MEASURED: Working-tree prototype on `plan/tsconfig-paths-p2` HEAD `8bd3c2dad641bf209f82073017b1549a8f377dff`. No worker Git writes or private F reads. The owner-supplied F materiality decision supersedes the P2-0 public-only stop recommendation below. The prototype reuses P1's local Node10 priority-pass proof, admits member-written terminals consistently with existing E5 behavior, and keeps all other export/binding/site guards. Actual private prototype recovery remains **OPEN** pending controller aggregate; no ceiling is reported as a measured gain.

## Current yield

All public streams were rerun with original base, P1 final and the immutable P2 binary in the same environment. P2 and P1 are byte-identical, including the complete site population and metadata. Native certificates for the retained P1 gains were reused only after full row parity and live native/fact input rehash; no fresh public gain-oracle certification is claimed where no row changed.

| Corpus | Complete sites | P2 changed vs original base | P2 changed vs P1 final | Evidence |
|---|---:|---:|---:|---|
| X | 19,219 | 3,121 | 0 | MEASURED, full byte parity |
| installed-X | 19,219 | 3,121 | 0 | MEASURED, full byte parity |
| R | 953 | 0 | 0 | MEASURED, full byte parity |
| T | 61,712 | 0 | 0 | MEASURED, full byte parity |
| F | controller-only | P1 supplied 2,313 + P2 pending | P2 pending | READ supplied ceiling; actual wrapper run OPEN |

READ: Expected opportunity ceiling from supplied aggregates is **749** in JS_EXPORT_HOP (775 bucket rows, 26 without callable/owner agreement), and **up to792** if all21 UNCLASSIFIED_P1_PROOF and22 BINDING_OR_SITE_GUARD rows share the admitted mechanism. This body changes no binding/site or forwardability guard to force those43. Without the457 member-written top-bucket rows, the top ceiling would be292; including both extra buckets and their18 member-written rows gives792 with475 /317 without. Those ceilings are not an admissible claim of realized F recovery.

The updated `p2-probes/CONTROLLER-p2.sh P1_BIN FACTS_BIN P2_BIN` now runs the P2 binary, validates identical complete keys/site metadata and certifies **every changed row** against the actual native caller ProjectService/checker terminal and ownership. Aggregate stdout reports actual gains by P1 reason, recovered member-written counts and the independent resolution ceiling; raw private rows/paths remain private. The bound head run was requested asynchronously. Its P2 branch was exercised on public synthetic inputs: **80 scenarios /160 sites /64 gains**, all64 CORRECT_STATIC_BINDING, all96 negative rows retained, zero uncertified changes. No private run was executed by this planner.

Receipts: `target/p2-plan/p2-public/summary.json`, lossless complete dumps, `p2-controls/summary.json`, `p2-controls/aggregate.json`. Native inputs rehashed: X633 /installed-X2219 /R48 /T606. Retained fact source files rehashed:628 /628 /50 /707. X and installed-X are copies of one snapshot; their gains are not additive.

## E5 and the native port

READ: E5/E5b keeps **written bindings** at base on every route. Both scoped writes (`src/ast/js_binding_writes.rs:49–71`) and module writes (`src/ast.rs:4730–4775`) collect names through `collect_js_ts_binding_pattern_names` (`src/ast.rs:5098–5148`). Member/subscript targets produce no binding names. The existing export resolver retains the same forwardability/span gates (`src/js_exports.rs:502–570`). No extraction/write/binding code changed.

MEASURED: Unchanged P1, built from this HEAD's archive in the same environment, passes local and legacy relative **Call and JSX** probes with dot/subscript member writes in both grammars:16 observations, `p2-e5-base-corrected.log`. Prototype positive controls and native synthetic rows admit the member-written alias-hop terminals; binding writes still equal the complete base row. **Bind the457** when the remaining proof succeeds. No new E5 owner ruling is needed; OQ-paths-p2 records the resolved determination and both ceilings.

READ: Pinned TypeScript 5.9.3 SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`: priority passes45240–45243; relative branch45327–45338; file-before-directory/full-candidate lookup45341–45394; suffix replacement/appending45423–45503; package/index45745–45813. P2 calls the exact existing local `Pass::relative(target,true,0)` through a six-line adapter. Non-relative ancestor/@types/custom-root searches are not added. Explicit JS-family literals get that proof; missing literals and other secondary/package redirects remain refused. P1 alias entries and explicit TS hops remain unchanged. SPEC's P2 amendment is normative.

## Controls, mutations and suites

MEASURED: Six product groups in JSX/TSX cover named/star/eligible function import-forward, intermediate JS barrels, explicit `.js/.jsx/.mjs/.cjs`, unknown dotted suffixes, TS/declaration competition, member writes, binding writes, shadow/allowJs/non-relative/star/package refusal and local-only roots. P1 RED has three behaviorally failing positive groups and three green preservation/E5 groups; initial invalid fixture setup is excluded. The final negative suffix competitors cover each JS family. Two pre-existing tests encoded the old Cut2 refusal. Same-environment P1 controls both pass; only the authorized newly admitted states' expectations were updated. No unrelated suite failure was found.

MEASURED: **12/12 admissible mutants killed** against an isolated source copy; six-group unmutated baseline green. M01 refuses JS; M02 drops local priority proof; M03 ignores allowJs; M04 drops explicit-JS support; M05 imports non-relative searches; M06 drops suffix replacement; M07 drops appending; M08 drops declarations; M09/M10 drop .d.mts/.d.cts; M11/M12 revert CPG/navigation cache versions. First ten are behavioral kills; last two are version-pin kills. Manifest, source hashes and failing assertions: `p2-mutant-manifest.json`, `p2-mutants.json`, `P2-M*.log`.

MEASURED: Cache106/62 from105/61. **8 cases /40 states** pass old-P1 cache rebuild, warm/unrelated hits and intermediate/terminal TS/declaration add/remove parity against fresh output (`p2-cache/summary.json`). **S1b-4:411 controls /639 sites /822 outputs byte-identical**, zero stderr (`p2-s1b.json`). Four Tier-A fixtures expose the private-shaped multi-hop member-written pattern and declaration refusal in both grammars.

| Verification | Current result |
|---|---|
| Full default | 4,935 passed /0 failed /1 existing ignore |
| Full mcp | 5,128 passed /0 failed /1 existing ignore |
| Full all-features | 5,151 passed /0 failed /1 existing ignore |
| fmt | PASS |
| all-targets/all-features clippy | PASS;372 warning emissions, none in changed resolver/new test file |
| release + Tier-A matrix | 174 OK /0 regressions /0 skips |
| Tier-A quick | INCOMPLETE: interrupted after1,642s in a SUT callers subprocess; no completed accuracy receipt |

MEASURED: All three Rust suites are unfiltered, with no environment exclusions; existing ignore is `resolution_test::slice_elem_variant_reserved`. Initial matrix setup was inadmissible because empty caller expectations cannot specify resolution_kind. Enumerating all fixture schemas found exactly both new refusal fixtures; both corrected, then immediate rebuild and174/174. Exact commands, RED/GREEN coverage and exclusions are in root `VERIFICATION.md`. Logs and hypothesis/probe/result record remain lean under target/p2-plan.

## Size, custody and remaining gates

MEASURED: Four production paths:31 added /7 removed lines. Five test paths:302-line new regression file plus small helper/expectation/module updates. Twenty-six fixture paths:78 lines. The shared local port keeps the source delta smaller than the P2-0 forecast; there is no new resolver table or global guard. ASSUMPTION forecast for the dispatch remains40–80 source /300–450 tests /70–120 fixtures; one bounded kernel/cache slice, no numeric cap. Independent controller plan review cap2, used0/2.

MEASURED: Immutable head `target/p2-plan/bin/prism-p2` SHA256 `3310ec8621dc82d4a93bd1b51f1697d0e3bde925a84f1553e4f523ba1f344ca7`, matching304 frozen production/vendor/build inputs. P1 final `907d110c…`; original base `8722d1af…`. Build binding, exact file split, owned snapshot and controller commit recommendations are in P2-FILES/HANDOFF and target/p2-plan. Controller commits the prototype; no push or merge.

READ/UNKNOWN: Not verified: actual F recovery and classification of its43 extra rows; independent review; human-triggered full multi-corpus Tier-A; Linux/case-sensitive and concurrent-tree behavior; quiet-host/resource gate attestation. Public input/fact bytes were freshly rehashed, not freshly re-extracted or gain-oracle reclassified; all complete public rows equal P1. Quick has no completed accuracy result; it was interrupted after1,642s waiting in a SUT callers subprocess (`p2-tier-a-quick.log`). No same-environment P1 quick control was run, so no regression attribution is made. Its generated oracle snapshot was retained under target, outside committed baselines.

---

## Historical P2-0 measurement checkpoint (superseded decision boundary)

# Lane P, P2-0 measurement checkpoint

ASSUMPTION: **Stop implementation dispatch at this checkpoint.** The measured public opportunity is ten distinct sites targeting six callable spans: eight non-relative export hops and two import-forwarded arrows. That is 0.052% of X's 19,219 sites, or 0.32% of P1's 3,121 gains. I recommend treating that public-only yield as immaterial. The historical 749-row F JS-hop bucket could change the decision substantially; its current resolution-admissible yield is still unmeasured. This is a public measurement result and an F gate, not a completed all-corpus P2-0 or a global recommendation to abandon P2.

READ: Owner authorization is measure-first, build only for material precision-safe yield. S6 keeps refusing; no program-graph ownership computation is authorized. The accepted tolerant ambient-scan cost remains. Plan review cap: **2 rounds**; controller's Opus-5.5 review has not run (**0/2**). No production prototype, SPEC amendment, implementation dispatch, product mutants or Tier-A fixture has been created before the gate.

MEASURED: Checkout is `plan/tsconfig-paths-p2` at `1811d2fed149cd8b5794f8a7e5557f2c915b247b`, directly after main `c50de85a`. The retained r5 executable SHA256 is `907d110c70962063d5fde23acd22b6d92b62b117d837b97a81f6d65ed263aa03`; all **304 production/vendor/build inputs** in its frozen manifest equal this checkout. `target/p2-plan/entry-binding.json` records that comparison. The executable was rerun, not rebuilt here; its embedded version identifies the historical dirty build.

## Public measurement and denominator

MEASURED: `p2-probes/public.py R X installed-X T` reran complete no-cache call-site streams, then the pinned P1 oracle. The final `p2-probes/native.cjs` supplement reran against every remaining associated low row using the **actual caller ProjectService program and checker**, rather than the P1 oracle's independent combined program. `p2-probes/summarize.py` rehashed native inputs and bound all certificates to the final instrument hash. Evidence: `target/p2-plan/summary.json`, `public-callable-sites.json`, `public/*/receipt.json`, complete native rows/traces and native input hashes.

READ: Import/site facts were extracted previously by the retained P0 instrument. They were not freshly extracted for the public corpora. MEASURED: all fact-source bytes were rehashed: X **628**, installed-X **628**, R **50**, T **707** files. Complete fresh P1 streams equal their r5 streams byte-for-byte, including keys and metadata. This admits reuse of those facts for this unchanged input and source population; it is not a claim of fresh extraction. The F wrapper does fresh extraction in the controller's private environment.

MEASURED: Native input rehashes cover **633 / 2,219 / 48 / 606** files for X / installed-X / R / T. Oracle version is **5.9.3**, SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. All ten remaining public candidates have identical native terminal `(file, name, start_line, end_line)` and agreeing ProjectService/root-file ownership. No candidate has been upgraded in Prism.

MEASURED: Counts below concern **remaining non-relative import-associated low call sites and their export hops**, including ineligible guards to preserve the denominator. They do not claim absence of these mechanisms throughout unrelated relative, class, CommonJS or dynamic call sites. X and installed-X are copies of the same source snapshot; their ten remaining sites are not additive.

| Corpus | Complete sites | Associated remaining low rows | Native callable candidates | Distinct spans |
|---|---:|---:|---:|---:|
| X | 19,219 | 648 | 10 | 6 |
| installed-X | 19,219 | 648 | 10 | 6 |
| R | 953 | 168 | 0 | 0 |
| T | 61,712 | 15 | 0 | 0 |

## Complete refusal census

MEASURED: These are the P1 oracle's **ordered independent explanations**, not production refusal telemetry. An explanation can preempt another mechanism. In particular, `BARE_BASEURL_OUTSIDE_P1` is a generic `via != paths` fallback and does **not** demonstrate a successful bare-baseUrl resolution. Counts sum to the associated-low denominator in each public corpus. Callable counts are separated in the next table.

READ: F numbers are the owner's supplied final-P1 aggregates (also retained in `MEASUREMENTS.md` under controller acceptance). They were not rerun here and are not bound to this turn's executable. `pending` means no fresh controller aggregate, not zero.

| Ordered refusal reason | X MEASURED | installed-X MEASURED | R MEASURED | T MEASURED | F READ / pending |
|---|---:|---:|---:|---:|---:|
| JS_EXPORT_HOP | 0 | 0 | 0 | 0 | 749 READ |
| NONRELATIVE_EXPORT_HOP | 8 | 9 | 0 | 0 | 1 READ |
| IMPORT_FORWARD_NOT_FORWARDABLE | 2 | 2 | 0 | 0 | pending |
| UNCLASSIFIED_P1_PROOF | 0 | 0 | 0 | 0 | 21 READ |
| BINDING_OR_SITE_GUARD | 0 | 0 | 0 | 0 | 22 READ |
| INVALID_CONFIG_BARRIER | 2 | 2 | 0 | 0 | pending |
| TSSERVER_OWNERSHIP_DISAGREEMENT | 6 | 6 | 144 | 15 | pending |
| BARE_BASEURL_OUTSIDE_P1, generic explanation | 568 | 568 | 0 | 0 | pending |
| MODULE_RESOLUTION_OUTSIDE_P1 | 23 | 23 | 0 | 0 | pending |
| TERMINAL_VALUE_ALIAS_OR_NONCALLABLE | 24 | 24 | 0 | 0 | pending |
| UNPROVEN_STAR_BRANCH | 1 | 0 | 0 | 0 | pending |
| NO_OWNING_CONFIG | 14 | 14 | 0 | 0 | pending |
| DELEGATED_CONFIG_BARRIER | 0 | 0 | 24 | 0 | pending |
| Public denominator | **648** | **648** | **168** | **15** | not measured |

MEASURED: Mechanisms and actual native results for every nonzero public bucket:

| Bucket | TypeScript binding / remaining yield |
|---|---|
| NONRELATIVE_EXPORT_HOP | X: eight callable rows; installed-X: the same eight plus one noncallable value. Four callable identities: `getSceneVersion`, `getSelectedElements`, `convertToExcalidrawElements`, `getCommonBounds`. Entry aliases resolve to TS barrels; named non-relative `@excalidraw/element` forwarding reaches TS arrows, sometimes through a relative star. |
| IMPORT_FORWARD_NOT_FORWARDABLE | Two rows bind `FooterCenter` and `WelcomeScreen` arrow spans in both X copies. The hops are **relative** import forwarding. READ: `js_exports.rs:562–566` requires `forwardable_function_locals`, populated by the declaration model; these arrows do not pass it. This corrects the older P0 list, which labeled all ten rows non-relative. |
| BARE_BASEURL_OUTSIDE_P1 | X: 568 unavailable native terminals. Installed-X: 466 unavailable + 102 noncallable values. Actual successful `via=baseUrl` rows: **0** in both copies. Installing packages resolves many declarations/value aliases; it adds no callable implementation span in this bucket. |
| TERMINAL_VALUE_ALIAS_OR_NONCALLABLE | 24 values in each X copy; native checker does not supply a function/arrow/wrapper implementation span. No P2 callable yield. |
| INVALID_CONFIG_BARRIER | X: two unavailable terminals; installed-X: one value and one unavailable terminal. No callable yield. Membership/config doubt keeps base. |
| TSSERVER_OWNERSHIP_DISAGREEMENT | X: six unavailable; installed-X: two values + four unavailable; R: 144 unavailable; T: 15 unavailable. No callable implementation span. Ownership disagreement independently keeps base under S6. |
| MODULE_RESOLUTION_OUTSIDE_P1 | 23 unavailable terminals in each X copy; native mode is Bundler. No callable yield. |
| UNPROVEN_STAR_BRANCH | One X row has no native callable terminal. Installed dependencies turn its ordered explanation into the ninth non-relative-hop row, which remains a value. It is not the ninth callable gain. |
| NO_OWNING_CONFIG | 14 unavailable terminals in each X copy. No callable yield. |
| DELEGATED_CONFIG_BARRIER | 24 R rows without native callable terminals. Solution/inferred ownership is not an implementation opportunity. |

## Re-measured parked mechanisms

MEASURED: Cells are **remaining mechanism rows / native callable rows**, with overlap allowed. A reference/config/package attribute is not an additive gain. `package_or_other` is the oracle's broad routing classification; it is not proof that package exports or workspaces caused the result.

| Mechanism | X | installed-X | R | T | F |
|---|---:|---:|---:|---:|---|
| Relative JS-family export hops | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 749 historical reason rows; actual hops pending |
| Non-relative callable export paths | 8 / 8 | 8 / 8 | 0 / 0 | 0 / 0 | 1 historical reason row; actual paths pending |
| Import-forwarded callable arrows | 2 / 2 | 2 / 2 | 0 / 0 | 0 / 0 | pending |
| Successfully resolved bare baseUrl | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| `.js` to `.ts`, entry or requested export hop | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| Project-reference attribute | 0 / 0 | 0 / 0 | 139 / 0 | 15 / 0 | pending |
| Package/other routing | 0 / 0 | 592 / 0 | 0 / 0 | 0 / 0 | pending |
| Multiple paths substitutions | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| Tied patterns / empty captures | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| Explicit substitution extension refusal | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| NodeNext/Node16/Bundler | 23 / 0 | 23 / 0 | 139 / 0 | 15 / 0 | pending |
| Package/array extends attribute | 6 / 0 | 6 / 0 | 0 / 0 | 0 / 0 | pending |

READ: Native rules for any later port, all in the pinned `typescript.js`:

| Rule | Source lines |
|---|---|
| Node10 priority TS/declaration pass, then secondary pass | 45240–45243 |
| Relative hop: local file/directory lookup; non-relative: optional settings, ancestors/@types/custom roots | 45287–45336 |
| Relative normalization and file-before-directory lookup | 45341–45394 |
| Known suffix replacement and implicit extensions, including `.js` to `.ts` | 45423–45503 |
| Directory/package types, typings, main, typesVersions and index | 45745–45813 |
| Selected paths pattern suppresses baseUrl fallback, including a miss | 44970–45001 |
| Bare baseUrl lookup | 45062–45074 |
| Ordered paths substitutions; explicit known-extension `tryFile` shortcut | 46417–46440 |
| Pattern precedence | 3638–3649, 22753–22762 |
| Extends arrays / ordered option merge | 43455–43549 |
| Real default project ownership | 190331–190337, followed by `ensureDefaultProjectForFile` |

## What blocks JS_EXPORT_HOP

READ: `js_paths.rs` has two separate blockers. `relative()` ignores `_allow_js` and finally rejects every JS-family result. Earlier, `prove_path()` rejects explicit `.js/.jsx/.mjs/.cjs` spellings. Therefore removing the final rejection alone cannot recover explicit-JS hops. `call_graph.rs:2257–2259` routes the alias export projection through that relative resolver, and `repo_loader.rs:282` primes the same route. The alias entry resolver already has the non-relative Node10 first-pass proof; the relative export path does not use its local counterpart.

READ: A faithful **relative** hop uses the same Node10 two-pass rule at that hop's containing file, but its first pass searches local relative file/directory candidates. It does **not** search importing ancestors, node_modules/@types or custom typeRoots. Reusing the entire non-relative `absent()` helper for a relative hop would introduce unrelated refusals. Reuse/port the local `Pass::relative` semantics and suffix/package rules, retaining captured no-follow readability, indexed-source identity, caller-project allowJs, complete export provenance and every existing binding/span guard. Native module resolution alone does not authorize Exact.

MEASURED: Both grammars' extensionless/named/star/import-forward and explicit-relative-JS probe cases bind the JS implementation after the priority lookup misses. A sibling declaration binds first and preserves refusal; explicit relative `.js` with a sibling `.ts` binds that TS implementation. A missing star branch prevents a complete-hop proof. The probe checks cover these outcomes without editing production.

MEASURED: A separate witness shows that **non-relative explicit paths substitutions are different**: `@p -> ./leaf.js` resolves directly to `leaf.js` with an occupied `leaf.d.ts` and **no secondary-pass trace**. This follows `typescript.js:46431–46435`. The instrument records JS-family and observed secondary-pass separately. Do not assert that every JS winner implies first-pass absence. Receipt: `target/p2-plan/explicit-path-witness/result.json`.

READ: The controller script measures F's actual named/star/import-forward paths, JS spellings, allowJs-off population, non-relative overlaps, native terminal/ownership disagreements, physical/closure doubt and unclassified detail codes. Its `relative_only_resolution_ceiling` is a **resolution ceiling**, not a certified Prism gain: the requested-member traversal does not reproduce all Prism star-opacity, write, wrapper, depth, cache or extraction guards. Those require the conditional prototype and changed-row oracle. Until F returns, the exact mechanisms within its 749/21/22/1 buckets remain OPEN.

## Ranked scope and forecast

ASSUMPTION: Ranking uses measured public marginal yield and READ historical F potential. The first row's realized F yield is not established; it cannot yet authorize a build. Forecasts are review sizing estimates, not LOC caps.

| Rank | Mechanism / decision | Yield evidence | Precision risk and forecast |
|---:|---|---|---|
| 1 | Relative JS export-hop proof, only if fresh F shows material resolution ceiling | MEASURED: public 0. READ: F reason bucket 749. | ASSUMPTION: bounded local two-pass port; explicit-JS suffix mapping may be inseparable. Preserve allowJs, physical/declared occupancy and all export guards. About 150–300 source lines, 300–500 control/test lines, 30–70 cache/fixture lines. Start with the measured spelling population; do not add an unmeasured blanket refusal. |
| 2 | Non-relative export hops | MEASURED: eight public sites, four spans. READ: F reason bucket 1. | ASSUMPTION: project option/context identity must accompany the projection; current allowJs-only partition is insufficient when two importing projects map the same barrel specifier differently. About 250–450 source lines plus 350–550 tests/cache controls. Public-only yield does not justify it. |
| 3 | Import-forwarded arrows | MEASURED: two public sites, two spans. | ASSUMPTION: changing shared forwardability risks legacy consumers. Prefer an alias-only span/provenance addition if ever material. About 60–120 source lines plus 150–250 tests. Park at two rows. |
| 4 | Bare baseUrl, general `.js` mapping, ordered substitutions, package/array extends, other modes | MEASURED: zero public marginal callable yield. | ASSUMPTION: no implementation now; each has precedence/config risks and would enlarge review without measured return. |
| 5 | References / program ownership, package exports/workspaces | MEASURED: zero public callable yield; ownership disagreements explicitly present. | READ: S6 remains refused. ASSUMPTION: no scope expansion; do not add program ownership or modern package machinery. |

ASSUMPTION: If the controller certifies a substantial portion of the 749 F rows as resolution-admissible, relative JS-hop proof is a plausible bounded P2. That would be roughly 32% of the historical 2,313 F P1 gains at the full ceiling. The actual gain must be measured with a prototype; neither 749 nor that ratio is a promised recovery. Otherwise recommend stopping. The owner supplied no numeric materiality threshold; [OQ-paths-p2.md](OQ-paths-p2.md) records the decision.

## Verification, custody and next boundary

MEASURED: Complete public no-cache dumps and both oracles ran; **22/22 synthetic instrument checks pass**, **0 failures**, across TS/TSX. Wrapper stdout was checked for absence of corpus, caller and terminal paths. Shell/Node syntax checks pass. Source/input/instrument hashes and bucket denominators pass. `hypothesis-probe-result.log` records the inadmissible first setup attempt and subsequent instrument corrections; unavailable terminal fields are no longer counted as agreement. No Git writes, private F access, network/install or cargo build directory was created.

READ: Not verified: fresh F aggregates, every F changed row, a P2 production prototype, product RED/GREEN, product cache behavior/resource gates, product mutants, full Rust test suites, Tier-A matrix/quick/full, independent Opus review, Linux/case-sensitive behavior or concurrent-tree semantics. Rust suites/Tier-A were not rerun because this checkpoint changes only measurement probes and documentation and makes no production implementation done-claim. P1 test totals remain historical, not this turn's execution.

ASSUMPTION: No buildable production work is awaiting completion. The next authorized boundary depends on missing F information. If material yield is established, retain the prototype as a patch plus exact source/binary hashes in an isolated copy or obtain a controller worktree; finish all four public head/base comparisons, every changed-row oracle, both-grammar controls, scoped incremental mutants, full suites and Tier-A, SPEC amendment and P2 implementation dispatch before handing off a buildable slice. Do not reopen any P1 refusal population without X/installed-X/F yield measurements.

READ: Controller command, with `CORPUS_F_ROOT`, a new private `PRIVATE_EVIDENCE_ROOT`, and pinned `TS_JS` already set:

```bash
bash docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/CONTROLLER-p2.sh \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r1/base/dump_imports
```

READ: Return stdout JSON only. Raw rows, paths, sources, traces, diagnostics and the actual ProjectService program remain private. A setup failure yields no measurement; its diagnostics are private. Proposed controller commits and the exact file inventory are in [P2-FILES.md](P2-FILES.md).

## Controller F aggregates (private, aggregates only; 2026-10-03)

Run with `CONTROLLER-p2.sh` on the P1 final binary (sha `907d110c`).

| Bucket | Rows | Natively callable, ownership agrees | Rows with `TERMINAL_MEMBER_WRITTEN` |
|---|---:|---:|---:|
| `JS_EXPORT_HOP` | 775 | **749** | 457 |
| `UNCLASSIFIED_P1_PROOF` | 21 | 21 | 8 |
| `BINDING_OR_SITE_GUARD` | 22 | 22 | 10 |
| `NONRELATIVE_EXPORT_HOP` | 6 | 1 | — |
| `TSSERVER_OWNERSHIP_DISAGREEMENT` | 19 | 3 | — |

Rows with no callable candidate and no recovery: `CANDIDATE_COMPETITION_OR_ABSENCE` 2,785 (0 callable), `TERMINAL_VALUE_ALIAS_OR_NONCALLABLE` 375 (0 callable), `UNPROVEN_STAR_BRANCH` 36 (0 callable).

**Verdict:** P2's yield is material on F (749 against P1's 2,313) and about zero on the public corpora. The relative JS export-hop proof ranks first.

## Controller F gap diagnosis (aggregates only; 2026-10-03)

Run with `CONTROLLER-p2-gap.sh`. Population: 749 natively callable `JS_EXPORT_HOP` rows. The prototype recovers 32; 717 remain unrecovered.

| Class | Rows | Breakdown |
|---|---:|---|
| `nonrelative_hop` | **697** (398 member-written) | TypeScript resolution: 599 via JS secondary pass, 98 unresolved. Export symbol at the hop: present 249, absent 350, unresolved module 98. |
| `binding_or_site_guard` | 18 | |
| `directory_literal` | 2 | |

**Verdict:** the dominant blocker is barrel export hops whose module specifier is non-relative (an alias or bare specifier). Applying P1's alias resolver at the hop, with the caller project's options, is the candidate P2 mechanism. Its ceiling on F is about 249 to 599 rows.

## Controller F acceptance of the P2 prototype `b9fd3775` (relative and non-relative hops; aggregates only; 2026-10-03)

| Measure | Result |
|---|---|
| Changed rows vs P1 final | **629**, all `CORRECT_STATIC_BINDING` |
| Keys added / removed | 0 / 0 |
| Recovered by P1 reason | `JS_EXPORT_HOP` 628, `NONRELATIVE_EXPORT_HOP` 1 |
| Member-written terminals among the recovered rows | 419 |
| Full-chain span agreement with TypeScript (non-relative hops) | 697 / 697 |
| Unresolved rows that stayed at P1 | 98 / 98 |

**The 350 ABSENT rows** are all `NONCONTRIBUTING_STAR_BRANCH_FULL_BARREL_BINDS_TERMINAL`: the hop's own module lacks the name, but the full barrel binds the exact terminal through another star branch.

F total: 2,313 (P1) + 629 = **2,942**, about +27%.

Public corpora: X +8 CORRECT, installed X +8 CORRECT, R 0, T 0, with every P1 gain preserved.

## Controller verification of the final fold (2026-10-03; disclosed extension past the spec-review cap, no third round)

- **Final prototype:** `826df3b3`. The cumulative squash is `proto/tsconfig-paths-p2-final` @ `c0a97207`, on `c50de85a`.
- **Reviewer probe cases, on the controller's build** (`826df3b3`) vs P1/main:
  - K1–K4 and A1–A4 match base: the legacy-star refusal is preserved.
  - D2, D2b, D3 and D6 differ. These are the disclosed S3 class: new alias-route bindings through type-only or broken star branches, only in programs that TypeScript reports as errors. F has 0 instances.
- **F:** +629 changed rows, all CORRECT_STATIC_BINDING; 0 keys added, 0 removed.
