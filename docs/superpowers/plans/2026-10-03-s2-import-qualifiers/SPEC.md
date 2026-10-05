# S2 import qualifiers — static-binding candidate under S2-O7

**STOP: R4 yield 0/0/0/0; X shortfall 132 from132 exceeds10; selection NONE.**

## 0. Decisions and dispatch boundary

**S2-O8 (2026-10-04): build package resolution first.** Lane PKG is prerequisite; R4 composes03fa9c29 now and dispatch depends on its merge first. Package discovery is not native binding authority.

**S2-O9 (2026-10-04): “PKG + TS-unresolvable out of model.”** A writer import is out of model and disclosed only when neither Prism including PKG nor TS binds it in this checkout. Production has no TS runtime: only PKG `ProvenUnresolved` certifies this exclusion. `Unsupported(reason)` fails closed with scoped unavailable reachability; never an empty absence result. `Bound(module)` remains in model. Accepted cost: false Exact if C is mutated through such an unresolvable import. Every resolved-module forward remains fail-closed, including r2 W2. Coverage gaps where TS binds Unsupported must be enumerated by reason and every gain-revoking writer reported; see REPAIR-R4.

**S2-O7 (2026-10-04): “Static-binding contract.”** `Exact` is a static-binding grade. Runtime re-acquisition, enumeration of runtime namespace objects, and eval/reflection without an expressible static binding are out of model. Static name, import, export and scope channels Prism models are in model: a reachable static alias or member write keeps base, including Object/Reflect use of that binding. See [CLAUDE.md static-binding contract](../../../../CLAUDE.md#navigation). S2-O7 supersedes **S2-O6's runtime-mutation scope**, not the existing conservative lexical whitelist.

A lexically visible member write or alias of C through a static binding in a file the proof sees remains **in model, keeps base**. Do not regress its refusal. The R2 candidate retains the original whitelist, captured own-member calls/class-heritage refusal (F1), every construction refusal (F2), new.target/super carrier refusals (F5), this carrier mapping (F6), and corrected parse-incomplete revocation/legacy opacity (F8). Do not fold F3/F4/F7 whole-project cuts.

| ID | Disposition | Design used in this packet |
|---|---|---|
| S2-O8 | Owner decided | PKG merges first; source03fa9c29; no source redirect from unbuilt workspace inventory. |
| S2-O9 | Owner decided, amended three-way authority | Only direct writer ProvenUnresolved excluded; Unsupported scoped refusal; accepted false Exact mutation cost disclosed; measured gap in REPAIR-R4. |
| S2-O7 | Owner decided | Static-binding contract; runtime F3/F4/F7 out of model, disclosed; static namespace content/forwarding/write closure retained. |
| S2-O6 | Runtime scope superseded by S2-O7 | Original closed whitelist retained; old universal runtime-closure obligations and costly-cut STOPs are historical. |
| S2-O1 | Controller interim, pending adoption confirmation | Separate S2 relative selector; membership/configuration/occupancy proof; empty references allowed, nonempty refused; lane-P unchanged. |
| S2-O2 | Controller interim, pending adoption confirmation | Two qualifier-prefix and two callable-export hops, up to four composed. |
| S2-O3 | Controller interim; no new work authorized | T stays base; ownership expansion requires another increment. |
| S2-O4 | Controller only | Private F comparison with bound tools and native ownership/full spans; planner never opens F. |
| S2-O5 | Disclosed coverage SMELL | Positional-proof mutation survivor remains visible unless a realistic regression kills it. |

The R1 review findings F3, F4 and F7 and R1b reflected-codegen WRONG (`[].filter.constructor(code)()`) remain preserved counterexamples to the superseded runtime-closure claim. They are **out of model by owner decision S2-O7**, not downgraded by inability to reproduce, and not proof of runtime safety. The demonstrated same-C mutation/result 1 is unchanged; it does not refute the newly authorized static-binding grade. S2-W1's statically bound alias/write remains in model.

No Git writes, private F access, source restart, independent review dispatch or adoption is authorized to this repair worker. R4 behavioral cap2 converged; one disclosed exact-union performance extension. Historical independent review cap2 is not restarted. Compilation/setup failures are inadmissible and disclosed separately.

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
| Own export | Same-name named export; bare export default C only without other direct/value uses. Namespace-import bindings cannot use either whitelist arm. |

Aliases, arguments/returns/stored/spread values, member extraction/writes/updates/deletes, computed access, reflective use of a statically bound qualifier, renamed exports and default-export-plus-use refuse. Once A=C refuses C, no positive heap-alias walk is needed. F5 maps super/new.target to plausible lexical carriers. F6 maps this and writes through it to every plausible enclosing class/object/namespace carrier, through nested initializers/arrows. F8 retains incomplete-file refusal facts and revokes possible S2 identities; its legacy-export view preserves missing-star opacity.

Refusal traversal returns `joined`, `proved absent`, `out_of_model_unresolvable`, or `unavailable`. Paths then PKG positive/source-status proof run with the writer's captured ownership; relative S2 proofs retain their scoped candidate ladder. Only a direct writer ProvenUnresolved is out of model under S2-O9. A proved unmatched node: builtin is absent after earlier authority. Unsupported reasons preserve possible path/package modules; when native scope is unknown the caller table is conservatively refused, never treated as an empty absence certificate. A resolved module's complete non-qualifier or proved missing name is absent; unprovable forwarding, syntax, escaped names and depth remain unavailable with its cycle-safe static source closure. Negative closure is independent of positive hop cap; namespace revocation cascades to fixpoint. The two r2 W2 resolved-forward mechanisms keep base. No spelling/depth/config exceptions or native runtime oracle are introduced. Runtime F3/F4/F7 impose no blanket cut. R3b's experimental external isolation is historical and unadopted.

## 2a. Channel dispositions under S2-O7/S2-O9

Every channel has a disposition. The out-of-model rows cite the [CLAUDE.md static-binding clause](../../../../CLAUDE.md#navigation), lines 233–239. Tests run both provider grammars; TS-only import = require uses a TS writer against each provider grammar. R3 reviewer/boundary regressions fail on committed 75a35a5e in the same environment; see REPAIR-R3. Earlier regression REDs cover added in-model cuts against c35719e1; the original whitelist was already present there and its tests remain baseline parity controls. Corrected F8 legacy opacity is a preservation control against c357 and a regression against the pre-correction R1b artifact.

| Channel | Disposition, rule and test |
|---|---|
| Unresolvable writer import | **Out of model (S2-O9), disclosed**, only ProvenUnresolved. Both-grammar `repair_r4_out_of_model_unresolvable_*`; accepted false Exact mutation cost. Measured Unsupported-where-TS-binds gap X/installed-X/R/T: 0/354/0/0; reasons and every gain-revoking writer in REPAIR-R4 and retained gap-revoking-writers.md/json. Unsupported remains in model, keeps base. |
| Direct calls through inherited/unknown or receiver-returning members | **In model, keeps base**: captured own Callable member only; heritage refuses (F1). `repair_r1_non_owned_calls_keep_base`; positive `repair_r1_owned_calls_remain_exact`. |
| Instance back-pointers, constructor/prototype walks through construction | **In model, keeps base**: every construction use refuses (F2). `repair_r1_construction_keep_base`. |
| Static named/default imports, direct namespace binding, named/star forwarding; lexically visible C alias/member write | **In model, keeps base** when a static binding escapes/writes: original whitelist and retained import/forwarder joins. `qualifier_identity_whitelist_refuses_all_value_escapes`, `all_statically_joined_qualifier_writes_keep_base`, `repair_r2_static_alias_and_reflective_writes_keep_base`. Clean forwarding remains eligible. |
| Dynamic import, literal/computed selections, require, TS import = require (F3) | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_dynamic_import`, `repair_r2_out_of_model_require_and_ts_import_require` pin Exact. |
| .default re-acquisition (F3) | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_default_reacquisition` pins Exact. |
| Namespace contents through export * as M, import * as N plus own/default/named forwarding, including Object.values of that static binding (F4 static half) | **In model, keeps base** when the static namespace binding escapes/writes. Refusal cascades to every exposed qualifier to fixpoint. `repair_r3_reviewer_static_refusals_keep_base`, `repair_r3_namespace_own_export_is_refused`, `repair_r3_statically_bound_namespace_enumeration_keeps_base`; clean namespace control remains Exact. |
| Genuinely runtime namespace enumeration without an expressible static binding (F4 runtime half) | **Out of model, disclosed** by S2-O7. This does not exclude static access through a forwarded namespace. |
| Static writer beyond positive hop cap, escaped/unprovable spelling, failed B0, Unsupported authority or resolved-module continuation | **In model, keeps base** on unavailable refusal joins. Positive two-hop cap stays. Both-grammar depth 2/3/5 and escaped/unescaped named/default pairs, writer-project and proved-absence controls in `repair_r3_*`; R3b unresolved failures are scoped, with both-grammar package/relative/Other controls in `repair_r3b_*`. |
| Class/static/inherited this, super, new.target | **In model, keeps base** for unproved carriers: heritage/construction plus explicit super/new.target refusal (F5), this own-call/write proof (F6). `repair_r1_receiver_carriers_keep_base`, `namespace_this_uses_require_the_same_closed_whitelist`. |
| Nested object initializers/arrows carrying surrounding this | **In model, keeps base**: all plausible carriers, including mapped member writes (F6). `repair_r1_nested_this_keep_base`. |
| Object/Reflect, computed properties, prototype walks using a visible static C binding | **In model, keeps base**: lexical alias/argument/extraction/write refusal; construction cut for instances. `repair_r2_static_alias_and_reflective_writes_keep_base`, `repair_r1_construction_keep_base`. |
| globalThis/window and reflective host lookup without a static C binding | **Out of model, disclosed**: CLAUDE static-binding clause. `repair_r2_out_of_model_host_globals_and_reflective_lookup` pins Exact. |
| eval, Function, with, reflected codegen, evaluated re-acquisition (F7) | **Out of model, disclosed**: CLAUDE static-binding clause and S2-O7. `repair_r2_out_of_model_eval_function_and_reflected_codegen` pins Exact, including R1b [].filter.constructor witness. No evaluated-code global cut. |
| Parse-incomplete JS/TS, including files without static imports/exports | **In model, keeps base**: unavailable syntax closure revokes possible S2 identities (corrected F8), while legacy missing-star opacity is preserved. `repair_r1_parse_incomplete_keep_base`, `repair_r1b_error_refusals_preserve_legacy_star_opacity`. |
| Writers outside the indexed-file universe: .mts/.cts, non-JS hosts (.vue/.svelte/HTML), excluded directories | **Out of model, disclosed**: these writers are invisible to Prism's static proof universe. Same text in an indexed .mjs writer is in model. This slice does not expand body indexing. |

The combined X cutoff is **122**: stop if it falls more than ten rows below 132. Installed X, R and T must also be measured; all changed rows must be CORRECT_STATIC_BINDING with module/ownership/full-span agreement. Per-cut R1b yields are not a combined result.

## 3. Mechanisms and resolver reuse

| Mechanism | Positive capture | Refusal boundary |
|---|---|---|
| Class statics | Unique module class, literal unique method/function field, Callable body/span | Instances, accessors, decorators, duplicate/computed/unknown keys, writes/escapes/errors |
| Declared namespaces | Unique namespace with exported Callable functions | Merging, signatures without bodies, noncallable members, writes/escapes |
| Re-exported module namespace | Landed export resolver plus S1b-4 namespace Callable captures | Wrapped/unspanned terminals, unresolved stars, conflicting identities, importer escapes |
| Constant objects | Unique const literal, literal unique method/arrow/named function-expression member | Spreads/computed keys/accessors/duplicates, identifier aliases, call results, writes/escapes |

`src/ast/js_import_qualifiers.rs` reuses B0 scope cleanliness, binding lookup/writes and JsTerminal. `src/js_import_qualifiers.rs` composes identities with the landed export resolver. Raw unproven named claims remain barriers. Base-bound object/namespace rows remain unchanged even if S2 would refuse their qualifier.

`apply_js_paths` retains the captured lane-P resolver and project partitions. S2 relative selection retains default-exclusion refusal for root/output directory options without explicit exclude; rootDirs, moduleSuffixes and noResolve refuse. Explicit `.js` substitution tries `.ts`, `.tsx`, `.d.ts`; `.jsx` tries `.tsx`, `.ts`, `.d.ts`, consistent with the pinned native controls. Declaration winners stay base. `.mts`/`.cts` body indexing is outside this product slice. R4 cache versions are **CPG 121 / navigation 77**, invalidating pre-PKG/S2-O9 derived results; whole-table dependency/config invalidation remains landed behavior.

## 4. Acceptance and measurement

Pinned TypeScript 5.9.3 ProjectService supplies validation evidence, not production admission. Every changed row must have a unique Callable terminal, native module equal to Prism's module proof, caller's native owner equal to Prism's owner, and exact FunctionId file/name/full line span. Complete key populations and source metadata must agree; any changed populated base row fails. Changed UNJOINABLE or unproven rows fail; empty streams and source/config/binary/oracle drift are globally inadmissible.

Unjoinable site/fact/program/position/syntax failures remain in the whole-site denominator. Mixed Rust+JS and BOM/CRLF controls pin the oracle boundary. F is never read by the planner. Controller aggregates report the base census and head comparison separately; zero changed rows does not imply every low row was proven.

Measure X, installed X, R and T against immutable merged-P2 main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. X snapshots are not additive. Partition the old 179 changed rows into survivors and refusals using complete rows and actual qualifier facts, with source locations for refusal causes. Report every new change CORRECT with module/ownership agreement and preserve every lane-P populated row. MEASUREMENTS.md and BUILD-MANIFEST.json carry the current counts and bindings.

## 5. Controls and gates

Both grammars retain original whitelist/base-preservation controls and regressions for F1/F2/F5/F6/F8. The 22-mutant R2 registry hash `7f1d4e2c40882c61768a0f3faf4aac73c45d9a9541bee18d64caf1087904cb22` is folded into the source-parent-bound R3 patch, plus six mutants S2-31..36: namespace cascade, namespace own export, writer project, removal of unavailable fallback, negative source capture, and negative-depth independence. R4 adds S2-37 Unsupported-to-ProvenUnresolved boundary mutant;29 registered,0 executed after yieldSTOP; current execution denominator belongs to VERIFICATION. Runtime F3/F4/F7 blanket cuts remain excluded. Advisory scoped selection may omit new-source anchors; disclose omissions/survivors without equivalence claims.

Required gates: full MCP nextest, MCP doctests, fmt, all-target MCP clippy, advisory mutgate with ancestor --since, immediate-source-rebuild Tier-A matrix, S1b-4 byte parity and all lane-P populated public rows preserved. **Skip Tier-A quick per the R2 brief**: the previous run hung over an hour; report unverified. Full multi-corpus Tier-A remains human-triggered. Binaries are rebuilt from this checkout and BUILD-MANIFEST pins the controller tools. Git writes, private F, independent review/adoption remain controller-only.
