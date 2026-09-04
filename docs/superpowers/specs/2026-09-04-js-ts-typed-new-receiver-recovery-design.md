# JS/TS typed-parameter and new-constructor receiver recovery

**Status:** design of record; implementation authorized
**Recorded:** 2026-09-04
**Exact base:** `deca1669947cd42d94d358ae80cb13cde0982750` (verified lexical-binding prerequisite)
**Scope:** caller-file class recovery for TypeScript/TSX typed parameters and JavaScript/TypeScript/TSX direct `new` locals

## 1. Decision and slice boundary

Implement roadmap item 4 without reopening the prerequisite. Recover a receiver only from (a) a simple TypeScript/TSX parameter whose annotation is one bare `type_identifier`, or (b) a simple JS/TS/TSX variable initialized before the call by direct `new Foo(...)`, where `Foo` is one bare identifier. The resolver may emit `TypedParam` or `ConstructorLocal` only when `Foo` is an occurrence-clean module-scope class in the caller's own file and that exact class directly defines the called method.

Out of scope: annotated locals, imported or cross-file receiver types, interfaces and structural dispatch, unions/intersections/generics/qualified annotations, qualified/dynamic constructors, factory calls, constructor-origin capture into nested callables, inherited methods, arbitrary assignment flow, fields/returns, and a general JS/TS type graph. Unsupported evidence remains materialized residue; it does not authorize generic owner lookup.

## 2. Demonstrated gap and alternatives

The predecessor tests prove `function run(x: Foo) { x.m(); }` and `const x = new Foo(); x.m();` retain `receiver_type=None` and do not resolve when the method-name fanout exceeds the candidate cap. The source mechanism is explicit: `classify_simple_ident_mode` and `receiver_type_evidence_in_fn_mode` admit only Rust, Go, and Python, while `constructor_type` has no `new_expression` arm.

The alternative that recovery is computed and discarded later is false: call-site construction stores the classifier result directly. Merely opening the gates is unsafe, however: generic `owner_lookup("Foo", "m")` cannot distinguish a local class from an imported/external/interface type with the same bare name. Resolution therefore needs caller-file clean-class proof rather than a bare-owner hit.

## 3. Binding evidence

Add a JS/TS-specific call-position evidence query; do not reuse the legacy recursive receiver walk. It uses the actual receiver AST node and the prerequisite's lexical boundaries.

- Search enclosing callable scopes from inner to outer. The nearest reaching value binding wins.
- A direct simple TS/TSX parameter with a single bare `type_identifier` yields `TypedParam`. Untyped, destructured, rest, generic, union, qualified, or malformed parameters are materialized-only.
- A simple variable declarator in the call's innermost function yields `ConstructorLocal` only when its direct initializer is `new Foo(...)`, `Foo` is a bare identifier, the initializer precedes the call, and no competing reaching declaration or same-name write makes the origin ambiguous.
- `let`/`const` scope follows the prerequisite's block/loop/switch/catch containment; `var` is function-scoped. Ended/sibling lexical declarations do not poison a call. A declaration after the call binds (TDZ/hoisting) but cannot donate its initializer.
- Constructor locals captured by nested callables are materialized-only in this increment. A typed outer parameter may donate its static type to a nested callable.
- Parse-recovery uncertainty in the relevant callable is materialized-only. A class name remains an owner-like value; function/enum/namespace names and named-function self bindings materialize so R3b cannot reinterpret them as an unrelated class owner.

The classifier opens JS/TS/TSX only through this evidence query. A recovered or unsupported value binding is materialized so R3/R3b cannot reinterpret the receiver variable as a module or class name. Imported receiver qualifiers retain the prerequisite behavior.

## 4. Resolution authority

For a JS/TS/TSX recovered type, require `clean_class_spans[(caller.file, type)]`. Then use the existing `recovered_receiver_direct_method` lookup, which requires the method to be a non-static method belonging directly and unambiguously to that exact class span. `Hit` preserves `TypedParam`/`ConstructorLocal` and Exact confidence. `Miss`, `Blocked`, missing class proof, static-only method, or unsupported type falls through to the existing R6 residue; it never enters generic `owner_lookup(type, method)` and never becomes `ExternalReceiver` solely because this syntax was observed.

This same-file proof prevents imported/external `Foo` annotations and constructors from binding to an unrelated in-repo `Foo`. Module-level occurrence ambiguity (duplicate/rebound/import-colliding class names) already removes the clean-class fact.

## 5. Persistence and caches

`receiver_type`, `receiver_recovery`, and `receiver_materialized` persist receiver evidence. Add a serialized `js_ts_static_methods` `FunctionId` set so cache-loaded resolution preserves the instance/static boundary. Because JS/TS receiver population and resolved topology change, advance CPG cache 58 to 59 and navigation call-edge sidecar 27 to 28. Full, direct-subset, incremental, CPG, and sidecar paths must agree.

## 6. RED/GREEN acceptance

- TypeScript and TSX required/optional/default bare typed parameters recover the caller-file class and resolve its direct method as Exact `TypedParam`; JavaScript stays untyped.
- JavaScript, TypeScript, and TSX `const`/`let`/`var x = new Foo()` recover as Exact `ConstructorLocal` when the declaration reaches the call.
- Bare factory calls, annotated locals, qualified or locally shadowed constructors, interface/union/generic/imported types, cross-file same-name classes, missing direct methods, and static-only methods do not mint Exact recovered edges.
- Reassignment, a competing same-scope binding, constructor capture, declaration-after-call, and parse uncertainty fail closed. Ended/sibling-block shadows do not suppress a valid outer origin; an active inner binding does.
- A receiver variable whose name collides with another class owner proves recovered materialization pre-empts R3b.
- Full and direct-subset builds, both incremental transitions, CPG/navigation round trips, and JS/TS/TSX language targets agree. Python/Rust/Go behavior remains unchanged.

Every recovery branch needs a pre-change RED and a negative/edge pole. Cache tests must prove changed behavior, not only version constants.

## 7. Verification and review

Review cap: two rounds. At the cap, recurring/open-class scope or mutation findings park the artifact; closed non-repeating findings get targeted fixes on the same branch.

Required gates: focused RED/GREEN; full JavaScript, TypeScript, TSX, resolution, direct-subset/incremental, and cache targets; format/diff/check/configured Clippy; full default and `mcp` suites with totals; release build; Tier-A matrix-only; immediate second release build; Tier-A quick. Tier-A pin drift is reported with an exact-base same-environment control and never re-baselined.
