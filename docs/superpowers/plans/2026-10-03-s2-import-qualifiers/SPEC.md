# S2 import qualifiers — static-binding candidate under S2-O7

**Current R2 state:** selected source candidate assembled from c35719e1 + F1/F2/F5/F6/corrected F8. Combined yield132/132/0/0; final gates recorded in VERIFICATION; controller review/adoption pending. R1/R1b are historical runtime-contract experiments. The owner decision below supersedes their current-state design STOPs.

## 0. Decisions and dispatch boundary

**S2-O7 (2026-10-04): “Static-binding contract.”** `Exact` is a static-binding grade. Runtime mutation of module objects is out of model for every rung, including monkey-patching, reflective writes, eval/Function/host globals and require/import() re-acquisition. See [CLAUDE.md static-binding contract](../../../../CLAUDE.md#navigation). S2-O7 supersedes **S2-O6's runtime-mutation scope**, not the existing conservative lexical whitelist.

A lexically visible member write or alias of C through a static binding in a file the proof sees remains **in model, keeps base**. Do not regress its refusal. The R2 candidate retains the original whitelist, captured own-member calls/class-heritage refusal (F1), every construction refusal (F2), new.target/super carrier refusals (F5), this carrier mapping (F6), and corrected parse-incomplete revocation/legacy opacity (F8). Do not fold F3/F4/F7 whole-project cuts.

| ID | Disposition | Design used in this packet |
|---|---|---|
| S2-O7 | Owner decided | Static-binding contract; F3/F4/F7 out of model, disclosed; static lexical write/alias exception retained. |
| S2-O6 | Runtime scope superseded by S2-O7 | Original closed whitelist retained; old universal runtime-closure obligations and costly-cut STOPs are historical. |
| S2-O1 | Controller interim, pending adoption confirmation | Separate S2 relative selector; membership/configuration/occupancy proof; empty references allowed, nonempty refused; lane-P unchanged. |
| S2-O2 | Controller interim, pending adoption confirmation | Two qualifier-prefix and two callable-export hops, up to four composed. |
| S2-O3 | Controller interim; no new work authorized | T stays base; ownership expansion requires another increment. |
| S2-O4 | Controller only | Private F comparison with bound tools and native ownership/full spans; planner never opens F. |
| S2-O5 | Disclosed coverage SMELL | Positional-proof mutation survivor remains visible unless a realistic regression kills it. |

The R1 review findings F3, F4 and F7 and R1b reflected-codegen WRONG (`[].filter.constructor(code)()`) remain preserved counterexamples to the superseded runtime-closure claim. They are **out of model by owner decision S2-O7**, not downgraded by inability to reproduce, and not proof of runtime safety. The demonstrated same-C mutation/result 1 is unchanged; it does not refute the newly authorized static-binding grade. S2-W1's statically bound alias/write remains in model.

No Git writes, private F access, restart, independent review dispatch or adoption is authorized to this repair worker. Gate repair cap two; the first targeted assembly failure was enumerated and corrected in place. Cumulative independent review cap remains two: R1 used, R2 controller-owned.

## 1. Admission invariant

Compute landed resolution first. Return any populated base result byte-for-byte. Only an empty base result may gain one Exact `ImportQualified` edge, after every proof succeeds:

1. Direct non-element `X.member()` with positional binding-core Import proof; preserve the landed `CallSite.local_binding` projection. Shadowed, indirect, uncertain and type-only bindings do not qualify.
2. One eligible named/default ESM MemberImport, one indexed module and proven caller ownership. Declaration priority, captured occupancy, ambient inputs and every barrel hop must succeed. No filename-stem fallback supplies admission authority.
3. One qualifier identity, keyed by defining file and local name. Distinct module namespaces remain distinct even when their functions agree. Conflicts, unresolved stars, cycles and depth overflow refuse.
4. One literal supported member with a binding-core Callable body/span and exactly one matching indexed FunctionId by file/name/start/end lines.
5. Closed qualifier-use predicate and member-write closure hold across every visible file. Any failed or unavailable proof returns the whole base outcome, including its drop reason.

## 2. Closed qualifier-use predicate

Traverse complete parsed files, including property/destructuring identifier tokens. Unknown lexical contexts refuse by default; collect refusals even when scope cleanliness fails. Refusals suppress local capture, caller admission and statically joined imports/forwarders across visible files. Conservative lexical shadow refusals are allowed.

| Allowed use | Required shape |
|---|---|
| Direct call | Identifier is the object of a literal nonoptional member used directly as a call function; the joined identity captures that own Callable member. Unknown/inherited members and heritage refuse (F1). |
| Construction | Outside the whitelist: every new C(...) refuses C (F2). |
| Type position | Explicit type syntax; class heritage is a value use and refuses. |
| Own declaration/import | Declaration/parameter name or ESM import binding; initializer/body separately traversed. |
| Own export | Same-name named export; bare export default C only without other direct/value uses. |

Aliases, arguments/returns/stored/spread values, member extraction/writes/updates/deletes, computed access, reflective use of a statically bound qualifier, renamed exports and default-export-plus-use refuse. Once A=C refuses C, no positive heap-alias walk is needed. F5 maps super/new.target to plausible lexical carriers. F6 maps this and writes through it to every plausible enclosing class/object/namespace carrier, through nested initializers/arrows. F8 retains incomplete-file refusal facts and revokes possible S2 identities; its legacy-export view preserves missing-star opacity.

The existing named/default/static import and forwarding joins remain. F3 runtime re-acquisition, F4 namespace enumeration and F7 evaluated-code capability impose no new whole-project revocation. They grant no positive binding authority either. Incidental lexical refusals may still keep base; out of model does not promise Exact for every shape. Pinned tests below document selected current shapes.

## 2a. Channel dispositions under S2-O7

Every channel has a disposition. The out-of-model rows cite the [CLAUDE.md static-binding clause](../../../../CLAUDE.md#navigation), lines 233–239. Tests run both provider grammars; TS-only import = require uses a TS writer against each provider grammar. Regression REDs cover newly added in-model cuts against c35719e1; the original whitelist was already present there and its tests remain baseline parity controls. Corrected F8 legacy opacity is a preservation control against c357 and a regression against the pre-correction R1b artifact.

| Channel | Disposition, rule and test |
|---|---|
| Direct calls through inherited/unknown or receiver-returning members | **In model, keeps base**: captured own Callable member only; heritage refuses (F1). `repair_r1_non_owned_calls_keep_base`; positive `repair_r1_owned_calls_remain_exact`. |
| Instance back-pointers, constructor/prototype walks through construction | **In model, keeps base**: every construction use refuses (F2). `repair_r1_construction_keep_base`. |
| Static named/default imports, direct namespace binding, named/star forwarding; lexically visible C alias/member write | **In model, keeps base** when a static binding escapes/writes: original whitelist and retained import/forwarder joins. `qualifier_identity_whitelist_refuses_all_value_escapes`, `all_statically_joined_qualifier_writes_keep_base`, `repair_r2_static_alias_and_reflective_writes_keep_base`. Clean forwarding remains eligible. |
| Dynamic import, literal/computed selections, require, TS import = require (F3) | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_dynamic_import`, `repair_r2_out_of_model_require_and_ts_import_require` pin Exact. |
| .default re-acquisition (F3) | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_default_reacquisition` pins Exact. |
| Namespace enumeration through export * as M / Object.values, including forwarding (F4) | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_namespace_enumeration` pins Exact. No strict F4 blanket cascade. |
| Class/static/inherited this, super, new.target | **In model, keeps base** for unproved carriers: heritage/construction plus explicit super/new.target refusal (F5), this own-call/write proof (F6). `repair_r1_receiver_carriers_keep_base`, `namespace_this_uses_require_the_same_closed_whitelist`. |
| Nested object initializers/arrows carrying surrounding this | **In model, keeps base**: all plausible carriers, including mapped member writes (F6). `repair_r1_nested_this_keep_base`. |
| Object/Reflect, computed properties, prototype walks using a visible static C binding | **In model, keeps base**: lexical alias/argument/extraction/write refusal; construction cut for instances. `repair_r2_static_alias_and_reflective_writes_keep_base`, `repair_r1_construction_keep_base`. |
| globalThis/window and reflective host lookup without a static C binding | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_host_globals_and_reflective_lookup` pins Exact. |
| eval, Function, with, reflected codegen, evaluated re-acquisition (F7) | **Out of model, disclosed**: CLAUDE static-binding clause and S2-O7. `repair_r2_out_of_model_eval_function_and_reflected_codegen` pins Exact, including R1b [].filter.constructor witness. No evaluated-code global cut. |
| Parse-incomplete JS/TS, including files without static imports/exports | **In model, keeps base**: unavailable syntax closure revokes possible S2 identities (corrected F8), while legacy missing-star opacity is preserved. `repair_r1_parse_incomplete_keep_base`, `repair_r1b_error_refusals_preserve_legacy_star_opacity`. |

The combined X cutoff is **122**: stop if it falls more than ten rows below 132. Installed X, R and T must also be measured; all changed rows must be CORRECT_STATIC_BINDING with module/ownership/full-span agreement. Per-cut R1b yields are not a combined result.

## 3. Mechanisms and resolver reuse

| Mechanism | Positive capture | Refusal boundary |
|---|---|---|
| Class statics | Unique module class, literal unique method/function field, Callable body/span | Instances, accessors, decorators, duplicate/computed/unknown keys, writes/escapes/errors |
| Declared namespaces | Unique namespace with exported Callable functions | Merging, signatures without bodies, noncallable members, writes/escapes |
| Re-exported module namespace | Landed export resolver plus S1b-4 namespace Callable captures | Wrapped/unspanned terminals, unresolved stars, conflicting identities, importer escapes |
| Constant objects | Unique const literal, literal unique method/arrow/named function-expression member | Spreads/computed keys/accessors/duplicates, identifier aliases, call results, writes/escapes |

`src/ast/js_import_qualifiers.rs` reuses B0 scope cleanliness, binding lookup/writes and JsTerminal. `src/js_import_qualifiers.rs` composes identities with the landed export resolver. Raw unproven named claims remain barriers. Base-bound object/namespace rows remain unchanged even if S2 would refuse their qualifier.

`apply_js_paths` retains the captured lane-P resolver and project partitions. S2 relative selection retains default-exclusion refusal for root/output directory options without explicit exclude; rootDirs, moduleSuffixes and noResolve refuse. Explicit `.js` substitution tries `.ts`, `.tsx`, `.d.ts`; `.jsx` tries `.tsx`, `.ts`, `.d.ts`, consistent with the pinned native controls. Declaration winners stay base. `.mts`/`.cts` body indexing is outside this product slice. R2 cache versions are **CPG 118 / navigation 74**, invalidating the c35719e1 derived results; whole-table dependency/config invalidation remains landed behavior.

## 4. Acceptance and measurement

Pinned TypeScript 5.9.3 ProjectService supplies validation evidence, not production admission. Every changed row must have a unique Callable terminal, native module equal to Prism's module proof, caller's native owner equal to Prism's owner, and exact FunctionId file/name/full line span. Complete key populations and source metadata must agree; any changed populated base row fails. Changed UNJOINABLE or unproven rows fail; empty streams and source/config/binary/oracle drift are globally inadmissible.

Unjoinable site/fact/program/position/syntax failures remain in the whole-site denominator. Mixed Rust+JS and BOM/CRLF controls pin the oracle boundary. F is never read by the planner. Controller aggregates report the base census and head comparison separately; zero changed rows does not imply every low row was proven.

Measure X, installed X, R and T against immutable merged-P2 main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. X snapshots are not additive. Partition the old 179 changed rows into survivors and refusals using complete rows and actual qualifier facts, with source locations for refusal causes. Report every new change CORRECT with module/ownership agreement and preserve every lane-P populated row. MEASUREMENTS.md and BUILD-MANIFEST.json carry the current counts and bindings.

## 5. Controls and gates

Both grammars retain original whitelist/base-preservation controls and regressions for F1/F2/F5/F6/F8. Each in-model rule has registered mutants: original S2-01 through S2-14; F1 S2-15/16/17; F2 S2-18; F5 S2-26; F6 S2-27; F8 S2-29/30. The registry omits out-of-model F3/F4/F7 cuts; their historical patches/evidence remain in R1/R1b custody. Advisory scoped mutation selection may omit new-source anchors; record its full denominator and survivors, without an equivalence claim.

Required gates: full MCP nextest, MCP doctests, fmt, all-target MCP clippy, advisory mutgate with ancestor --since, immediate-source-rebuild Tier-A matrix, S1b-4 byte parity and all lane-P populated public rows preserved. **Skip Tier-A quick per the R2 brief**: the previous run hung over an hour; report unverified. Full multi-corpus Tier-A remains human-triggered. Binaries are rebuilt from this checkout and BUILD-MANIFEST pins the controller tools. Git writes, private F, independent review/adoption remain controller-only.
