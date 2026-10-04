# S2 import qualifiers — bounded positive admission

## 0. Owner decisions and blocking design gate

**DISPATCH PARKED — WRONG S2-W1 (confidence 100/100):** `export class C { static sm() { return 0; } } const Alias = C; function replacement() { return 1; } Alias.sm = replacement;` followed by an imported `C.sm()` is a base drop. Head adds Exact to the original `sm` despite the qualifier object being written through its statically known alias. Both JSX/TSX and the same-object runtime control reproduce it. The static checker still certifies the declaration; that does not prove E5. Repeated new write-identity forms after the local cap are open-class: park for design, preserve the artifact, and do not adopt these positive edges yet.

The corrected owner brief authorizes new Exact edges on base-dropped sites after a complete static proof. S1b-4's no-new-identity rule still governs refinement of an existing R3 set. Option K and qualifier-object member-write E5 are binding requirements, not open questions.

The following new policy choices are explicit placeholders. The working prototype is evidence for these choices; it does not constitute their adoption.

| ID | New policy | Proposed boundary | Owner decision |
|---|---|---|---|
| S2-O1 | S2 relative-import ownership envelope | Reuse the captured lane-P membership/configuration/occupancy proof. Only the S2 selector admits an explicit empty `references: []`; nonempty references remain refused. Simple relative imports may use NodeNext with the retained option fences. Bare paths aliases retain P1/P2's unchanged resolver. | `__OWNER_RELATIVE_PROOF_ENVELOPE__` |
| S2-O2 | Combined namespace-chain depth | Qualifier identity uses the landed depth bound of two. A re-exported module namespace separately uses the landed callable export-chain bound of two; the composed path can contain four module hops. Adopt this explicit bound, or require a single shared two-hop budget and remeasure. | `__OWNER_COMPOSED_DEPTH_BOUND__` |
| S2-O6 | Qualifier object write/escape identity design | E5 is already mandatory; do not authorize this wrong edge. Prefer a bounded fail-closed escape/alias cut first: any qualifier whose value identity or member-write closure is unproved stays base. A later positive alias analysis must compose local aliases, imported aliases, namespace-member paths and unknown escapes without changing the S1b projection. Decide and review this design before another implementation retry. | `__OWNER_QUALIFIER_IDENTITY_DESIGN__` |
| S2-O3 | Further T ownership work | Keep this packet's T rows at base. Origin-bound default output-directory exclusions and inherited ambient type inputs require a separately specified ownership increment, with inside/outside exclusion and inherited-origin controls. Do not remove the current guard merely to recover the native census. | `__OWNER_T_OWNERSHIP_INCREMENT__` |

Exact remains a static binding grade; this does not excuse known writes to the qualifier object. The prototype handles direct and statically joined importing writes but lacks object-alias/escape closure. The current implementation therefore does not satisfy the admission invariant below on all inputs. S2-O6 chooses the repair design, not whether to waive E5.

## 1. Admission invariant

Compute the complete landed resolution first. If it contains any target, return that result byte-for-byte: S2 neither widens nor replaces it. This part passes complete-row controls; the positive branch remains blocked on S2-W1. Any future narrowing must obey S1b-4's positive subset proof and is outside this prototype.

For an empty base result, return one Exact `ImportQualified` edge only after every condition below succeeds:

1. A direct, non-JSX-element `X.member()` source call has a positional binding-core Import proof. Shadowed, uncertain, indirect and type-only bindings do not qualify. Preserve the existing `CallSite.local_binding` projection.
2. Exactly one eligible named/default ESM MemberImport binds `X`. Its specifier resolves to one indexed source module under the caller's proven owning configuration. Captured filesystem occupancy, declaration priority, ambient inputs and every barrel hop must succeed. No filename-stem fallback grants a new edge.
3. The exported qualifier has one identity, including its defining file and local identity. Two different module namespaces that export the same function are different qualifier identities. Conflicting claims, unresolved star branches, cycles and exceeded depth keep base.
4. The requested member has one supported, unwritten Callable implementation capture with an exact span. The graph contains exactly one matching FunctionId by file, indexed name, start and end lines.
5. Binding writes or qualifier-object member writes make the row may-call. Writes in the provider, caller, forwarding module or a statically joined importing module invalidate that qualifier identity. Assignment, computed assignment, update, delete, destructuring member targets and lexical `this` writes are covered. A shadowed local object's write does not poison the import.

Every failed proof must keep the whole base outcome, including its drop reason. Native checker absence is never a product proof of non-callability. The oracle is validation evidence; it is not used by production admission.

## 2. Mechanisms, in yield order

| Mechanism | Required capture | Kept-base boundary |
|---|---|---|
| Named/default class static methods and function-valued fields | Unique module-scope class binding; literal unique member name; static method or function initializer; clean binding-core Callable body/span | Instance methods, accessors, decorators, duplicate keys, unknown/computed members, writes, parse errors and unindexed bodies |
| Declared namespaces | Unique namespace identity; exported function declaration proven Callable inside its body | Merged/duplicate namespaces, signatures without a body, non-callable members and writes |
| Re-exported module namespaces | `export * as N`; landed export resolution plus the S1b-4 binding-core callable capture; unique namespace identity | Wrapped terminals, missing spans, unresolved stars and identity conflicts |
| Constant object literals | Unique `const` object binding; literal unique method, arrow or named function-expression member | Spreads/computed keys, accessors, duplicates, identifier/shorthand alias members, call-result objects and writes |

The class extractor deliberately refuses the complete class when a member key cannot be proved; it can undercount otherwise safe static members. Object and namespace cases already bound by base are preserved even if this table could prove a different singleton.

## 3. Resolver and core reuse

`src/ast/js_import_qualifiers.rs` derives separate qualifier facts using the existing B0 scope-cleanliness checks, scope lookup, binding/write walk and JsBinding/JsTerminal types. Member syntax has no identifier binding; its capture checks the function body, indexed name and exact span before constructing Callable. Declared namespace functions use the existing scoped binding operation directly.

`src/js_import_qualifiers.rs` composes positive qualifier identities. It reuses `resolve_js_exports_for` and the S1b-4 `namespace_callable_locals` captures for module-namespace functions. Raw named claims that are unproven remain barriers rather than disappearing.

`apply_js_paths` uses the captured lane-P resolver and partitions tables by caller project. The normal P1/P2 selector remains unchanged. S2's separate relative selector retains the default-exclusion refusal when output/root directory options occur without explicit exclude; it also refuses rootDirs, moduleSuffixes and noResolve. Alias imports use existing P1/P2 module proofs.

Explicit `.js` substitution checks `.ts`, `.tsx`, then `.d.ts`; `.jsx` checks `.tsx`, `.ts`, then `.d.ts`, following pinned TypeScript 5.9.3 at lines 45480–45487. Declaration winners keep base. `.mjs`/`.cjs` typed counterparts are checked for priority, but `.mts`/`.cts` bodies are not indexed by this product and therefore cannot supply new Callable edges. Unblocked occupancy and indexed source identity remain mandatory.

Derived facts and project tables are serialized, with CPG cache version 107 and navigation sidecar version 63. Dependency/config invalidation remains the existing lane-P whole-table rebuild. Do not reset the already-merged P2 cache versions or patch derived tables incrementally.

## 4. Oracle and complete-row acceptance

The pinned TypeScript 5.9.3 ProjectService uses each caller's actual default project. A changed row must have a unique callable terminal; native import module and owner configuration must equal Prism's module and owner proof; its exact FunctionId file/name/full line span must match. Full base/head key populations and source metadata must agree. No already-bound row may differ, and target membership loss must be zero.

Join failures are per-site `UNJOINABLE`: missing site/fact join, missing caller program, unsupported/non-direct syntax or unusable source position. They count in the whole-site denominator and never provide a certificate. A changed unjoinable row fails head comparison. Empty streams, binary/probe drift, pinned-oracle drift and source-hash drift remain whole-run INADMISSIBLE errors.

The public mixed Rust + JSX/TSX fixture reproduces the original fatal site/fact join: call-stats exposes a Rust site while the facts driver emits JS/TS facts only. BOM+CRLF controls independently certify valid positions. This is a likely explanation of the supplied F error, not a claim about F's contents.

## 5. Controls and evidence

The existing green integration controls cover all four mechanisms, base-bound preservation, caller ownership, import shadows, callable uniqueness/body spans, export conflicts, unresolved stars, E5 writes and explicit suffix priority/declaration refusals. The additional `probes/e5-alias-boundary.py` admission control is RED in both grammars and must become GREEN before adoption. Existing native synthetic controls run in both grammars; declared namespaces intentionally keep base in the JavaScript grammar.

`mutants/lane-s2-import-qualifiers.json` records behavioral obligations for the scoped and authoritative mutgate. Scoped selection sees tracked diff paths, so several new-source obligations are omitted until the controller commits them; an extra mutation in a tracked file can still select a new-source obligation. A surviving mutant remains a coverage SMELL unless a concrete product failure is demonstrated; do not call it equivalent merely because its current selector stays green.

Tier-A fixtures for JavaScript and TypeScript expect the new imported static edge while allowing the inherited R6SingleOwner shadow edge. Both new fixtures fail on the source-bound base and pass on head. Existing Tier-A baselines are unchanged.

Public yield, complete denominators, preserved P2 rows, source/binary bindings and gate receipts are in MEASUREMENTS.md and BUILD-MANIFEST.json. F remains controller-only. Independent review cap is two; zero rounds have been dispatched. Preserve the existing artifact for targeted corrections at the cap; do not restart it.
