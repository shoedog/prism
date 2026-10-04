# S2 import qualifiers — fail-closed positive admission

## 0. Decisions and dispatch boundary

S2-O6 is the owner's 2026-10-03 decision: **“Ship X gain, fail-closed.”** A qualifier stays base whenever its value identity escapes or its member-write closure is unproved. Use a closed whitelist, not an expanding syntax blacklist. This repair continues the existing prototype; no restart or Git write is authorized.

| ID | Disposition | Design used in this packet |
|---|---|---|
| S2-O6 | Owner decided | Conservative lexical whitelist in every indexed defining/forwarding/importing file. Any unrecognized value use revokes the entire qualifier identity. |
| S2-O1 | Controller interim position, pending owner confirmation for adoption | Separate S2 relative-import selector; captured membership/configuration/occupancy proof. Explicit empty references allowed; nonempty references refused. Normal lane-P alias resolver unchanged. |
| S2-O2 | Controller interim position, pending owner confirmation for adoption | Two qualifier-prefix hops and two callable-export hops, explicitly up to four composed module hops. |
| S2-O3 | Controller interim position; no new work authorized | T remains base. Origin-sensitive default exclusions, inherited ambient inputs and references require a separate ownership increment. |
| S2-O4 | Controller-only execution | Wrapper now compares base/head complete rows and native ownership/full spans on F, returning aggregates only. Its current result is not inferred from the base census. |
| S2-O5 | Disclosed coverage SMELL | Positional-proof mutant survived its older selector. Keep it visible unless a realistic base-dropped negative kills it; no equivalence claim. |

S2-W1 was WRONG at confidence 100: `const Alias=C; Alias.sm=replacement` changed the same exported class object while the prototype added Exact to the original method. The old binaries reproduce it in both grammars; the whitelist must keep base. Static checker agreement alone cannot establish write closure. The owner decision supplies the design authorization that was missing at the previous open-class cap. This repair declared a three-round cap and disclosed bounded extensions for lexical property/destructuring coverage and the remaining namespace receiver carriers. Neither extension restarts the artifact or changes the whitelist policy. Independent cumulative review has cap two, zero rounds dispatched by the planner.

## 1. Admission invariant

Compute landed resolution first. Return any populated base result byte-for-byte. Only an empty base result may gain one Exact `ImportQualified` edge, after every proof succeeds:

1. Direct non-element `X.member()` with positional binding-core Import proof; preserve the landed `CallSite.local_binding` projection. Shadowed, indirect, uncertain and type-only bindings do not qualify.
2. One eligible named/default ESM MemberImport, one indexed module and proven caller ownership. Declaration priority, captured occupancy, ambient inputs and every barrel hop must succeed. No filename-stem fallback supplies admission authority.
3. One qualifier identity, keyed by defining file and local name. Distinct module namespaces remain distinct even when their functions agree. Conflicts, unresolved stars, cycles and depth overflow refuse.
4. One literal supported member with a binding-core Callable body/span and exactly one matching indexed FunctionId by file/name/start/end lines.
5. Closed qualifier-use predicate and member-write closure hold across every visible file. Any failed or unavailable proof returns the whole base outcome, including its drop reason.

## 2. Closed qualifier-use predicate

Traverse the complete parsed file once, including property and destructuring identifier tokens. Unknown identifier contexts are refused by default. Collect lexical refusals even when scope cleanliness fails. Refused local names are applied across every visible file, including dynamic namespace accesses that lack named import bindings. The allowed contexts are:

| Allowed use | Required shape |
|---|---|
| Direct call | Identifier is the object of a literal property `C.member`, and that exact member expression is the call's function. Optional/computed/chained accesses refuse. |
| Construction | Identifier is directly the constructor of `new C(...)`. Uses inside the arguments are checked separately. |
| Type position | Type identifiers or identifiers nested in explicit type annotations, type arguments/parameters, type aliases or interfaces. Class heritage is a runtime value use and refuses. |
| Own declaration/import | Declaration name, parameter declaration, or ESM import binding. Initializers and bodies are separately traversed. Import renaming declares a binding; it does not by itself escape its value. |
| Own export | Same-name named export. A bare `export default C` is allowed only when there is no other direct call/construction/value use of C in that file. `export default class C` is its declaration. |

Every other occurrence refuses: alias initialization/assignment/destructuring; arguments/returns/property/array/map/spread values; member extraction; assignments/updates/deletes; computed access; reflective helpers; renamed exports; default export together with other uses. Once `A=C` refuses C, no positive alias walk is necessary to detect later writes through A. Lexical shadow occurrences may also refuse the module identifier; the owner explicitly permits that conservative cost.

Every `this` occurrence is mapped conservatively to the nearest class, object or declared namespace carrier and checked by the same use whitelist. When its lexical carrier is unavailable, the receiver may be a module namespace: a refused occurrence cuts module-namespace identities via their existing `*namespace*` identity marker. It never silently certifies receiver closure. This closes alias/return/argument/member-write/computed/member-value paths for all supported carriers without assuming the dynamic receiver is known. A direct literal `this.member()` remains allowed. The existing member-write walk remains an independent guard, including destructuring and lexical-this writes. Parse errors or scope-cleanliness failure do not certify closure.

Raw `QualifierFacts.written` contains both write and whitelist refusals. Provider refusals suppress local qualifier capture. Caller refusals suppress admission. Importer refusals and renamed source re-exports join to the qualifier identity and revoke it from every project's table. Namespace importer paths conservatively revoke all qualifiers from that module. Refusal joins can use retained resolver hops and indexed relative candidates even when the writer's owning project is unproved; those candidates never grant a positive edge. Unjoined named imports conservatively revoke matching exported names. An unjoined namespace has no captured identity equal to an arbitrary class; it does not blanket-revoke unrelated objects. Its spelled property/destructuring names still participate in the global lexical refusal set. This can reduce yield and must be measured rather than relaxed without proof.

The predicate proves only the visible indexed source universe. It does not claim closure over files excluded from the repository input or arbitrary external runtime code.

## 3. Mechanisms and resolver reuse

| Mechanism | Positive capture | Refusal boundary |
|---|---|---|
| Class statics | Unique module class, literal unique method/function field, Callable body/span | Instances, accessors, decorators, duplicate/computed/unknown keys, writes/escapes/errors |
| Declared namespaces | Unique namespace with exported Callable functions | Merging, signatures without bodies, noncallable members, writes/escapes |
| Re-exported module namespace | Landed export resolver plus S1b-4 namespace Callable captures | Wrapped/unspanned terminals, unresolved stars, conflicting identities, importer escapes |
| Constant objects | Unique const literal, literal unique method/arrow/named function-expression member | Spreads/computed keys/accessors/duplicates, identifier aliases, call results, writes/escapes |

`src/ast/js_import_qualifiers.rs` reuses B0 scope cleanliness, binding lookup/writes and JsTerminal. `src/js_import_qualifiers.rs` composes identities with the landed export resolver. Raw unproven named claims remain barriers. Base-bound object/namespace rows remain unchanged even if S2 would refuse their qualifier.

`apply_js_paths` retains the captured lane-P resolver and project partitions. S2 relative selection retains default-exclusion refusal for root/output directory options without explicit exclude; rootDirs, moduleSuffixes and noResolve refuse. Explicit `.js` substitution tries `.ts`, `.tsx`, `.d.ts`; `.jsx` tries `.tsx`, `.ts`, `.d.ts`, consistent with the pinned native controls. Declaration winners stay base. `.mts`/`.cts` body indexing is outside this product slice. Cache versions are **CPG 108 / navigation 64**, invalidating pre-whitelist derived results; whole-table dependency/config invalidation remains landed behavior.

## 4. Acceptance and measurement

Pinned TypeScript 5.9.3 ProjectService supplies validation evidence, not production admission. Every changed row must have a unique Callable terminal, native module equal to Prism's module proof, caller's native owner equal to Prism's owner, and exact FunctionId file/name/full line span. Complete key populations and source metadata must agree; any changed populated base row fails. Changed UNJOINABLE or unproven rows fail; empty streams and source/config/binary/oracle drift are globally inadmissible.

Unjoinable site/fact/program/position/syntax failures remain in the whole-site denominator. Mixed Rust+JS and BOM/CRLF controls pin the oracle boundary. F is never read by the planner. Controller aggregates report the base census and head comparison separately; zero changed rows does not imply every low row was proven.

Measure X, installed X, R and T against immutable merged-P2 main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. X snapshots are not additive. Partition the old 179 changed rows into survivors and refusals using complete rows and actual qualifier facts, with source locations for refusal causes. Report every new change CORRECT with module/ownership agreement and preserve every lane-P populated row. MEASUREMENTS.md and BUILD-MANIFEST.json carry the current counts and bindings.

## 5. Controls and gates

Both grammars cover every whitelist refusal family in the provider, caller, distinct importer and forwarding importer; source-renamed re-exports, namespace paths, unproved writer ownership, object aliases and `this` escapes have additional controls. Positive declaration/call/new/type/same-name/default-only exports prevent a blanket refusal implementation. The expanded `e5-alias-boundary.py` compares immutable base/old prototype/repaired head. S2-W1 must keep base in both grammars.

Mutants registry `mutants/lane-s2-import-qualifiers.json` includes S2-14 removing the whitelist. Its extra tracked-file comment is only a selection marker so the scoped driver includes the new untracked source obligation; it changes no behavior. The scoped run is advisory, and omitted new-source anchors still need the controller's authoritative registry run after commit. Surviving mutants are disclosed SMELLs without an invented failure or equivalence claim.

Tier-A positive and alias-write refusal fixtures exist for both JavaScript and TypeScript. Older baselines remain unchanged. The required tiered gates are one full MCP nextest after final semantic edits, MCP doctests, advisory scoped mutgate, fmt/clippy, immediate-rebuild Tier-A matrix, all retained S1b-4 byte parity and complete lane-P public preservation. Tier-A quick is required before independent review; full multi-corpus Tier-A remains human-triggered. The controller owns Git writes, F, model-bound dispatch/review and adoption; no auto-merge.
