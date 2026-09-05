# Rust binary-to-library recall repair

Authority: owner "merged, continue to next increment" and "also should fix the
rust recall defect". Base: merged #241, `1886907`. Three-round self-review cap.

## Diagnosis and contract

WRONG: `src/main.rs:183` calls `prism::navigation::module_graph::module_deps`,
but the prior same-source paired dump reports UnknownName. Minimal current-base
control: `binary_can_call_own_named_library` resolves `crate::api::target()` inside
the library and returns zero targets for `demo::api::target()` in the binary.
The first test probe did not compile (set indexing); it is inadmissible. The
corrected test ran one test and failed the binary assertion, after the library
control passed. Thus ordinary module traversal is not the cause. Source confirms
the extern-prelude dependency map has library consumers only and no own-library
binding. No resolver spelling fallback is authorized.

Capture manifest-backed binary root -> (library crate name, library root path).
Use `[lib].name` or package name with Cargo hyphen normalization; retain exact
per-package library paths, not the legacy global last-lib-path field. Bind only
modeled roots through the existing Rust 2018+ extern-prelude policy. Exclude
missing libraries, disabled auto targets, proc-macro/non-linkable libraries,
conflicting dependency names and target edition overrides. Preserve lexical
shadowing and visibility. No change to leading-colon or 2015 extern-crate policy,
other binary dependencies, test/example dependencies, or Cargo target discovery.

Explicit `[[bin]]` tables contribute explicit paths only; no name-based auto-target
override inference. Mixed anchoring editions contribute no new bindings. The
extern-prelude guard must inspect the skipped lexical ribs: block/callable type
bindings and uncertain glob/macro ribs block fallback. Populate block type aliases,
traits and extern-crate declarations like module items, and carry enclosing
function/impl/trait type-parameter barriers into callable scopes (method bodies
are otherwise re-parented to the module).

Cargo contract source: [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html).

## Implementation and verification plan

1. RED loader-integrated regression and finite positive/negative matrix.
2. Add serialized config fact and exact root binding; invalidate CPG/nav caches.
3. Verify raw resolution, bidirectional served navigation, manifest replacement
   and cache round trips; reproduce the actual main.rs call on the release binary.
4. Full default/MCP suites, fmt/Clippy, immediate-build Tier-A matrix and quick.
   Preserve pins/baselines and report invalid quick baselines honestly.
5. Reconcile #241 merge, publish source/test/spec evidence and open the next PR.

## Python/JS next-increment selection

Direct default named-class receiver imports are a bounded extension of existing
declaration-backed identity. The initial Excalidraw census has no direct
`export default class Name` declarations; `Library` uses `export default Library`.
Do not claim that indirect form is covered or that direct-only support improves
that corpus. Python's prior relative-receiver gains remain measured, with no
sample-proven Python defect. Any added JS lane must retain erased type imports,
constructor/value separation, duplicate/write poison and defining-file identity.

Implementation contract: `export default class Client { ... }` contributes
`default -> Class(Client)` in raw facts, never a callable-function export.
ESM `import Alias` / `import {default as Alias}` can authorize constructors and
typed parameters. TS `import type Alias`, `import type {default as Alias}` and
`import {type default as Alias}` authorize typed parameters only. Namespace types,
anonymous/indirect default exports, reexports, decorators, mutable/duplicate
bindings, shadowed types, static/overridden methods and CJS receivers stay out.
Cold/subset construction, cached incremental transitions and nav sidecars must
preserve the same defining-file identity. Python implementation is unchanged.

## Hypothesis / probe / result log

- Rust initial RED: library control resolves, binary path does not (3 tests RED
  on base before production edits). Crate binding, not general traversal, missing.
- JS initial RED: direct default class yields NameOnly candidates; negative matrix
  passes. Explicit AST default-export/import exclusions, not mutation, deny proof.
- Review 1: explicit name-only binary edition override manufactured an Exact edge;
  target-edition RED reproduced; explicit-path-only metadata closes the bypass.
- Review 2: uncached and serialized nav both contained a block-module shadow edge;
  therefore not a serialization fault. Extern fallback skipped lexical ribs.
  Full negative census then isolated missing type-alias and generic bindings;
  nine initial cases enumerated before production retry. Expanded census also
  covers block traits/extern crates and method/impl generic barriers.
- Full-suite first pass: one failing matrix label, `default`, explicitly encoded
  the old exclusion. Replace it with an indirect-default preservation case; new
  positive tests own the newly admitted direct form. No baseline rewrite.
