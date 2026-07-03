> **Status: IN FLIGHT (implementer running as of 2026-07-03; branch `p4-js-export-modeling`, worktree /private/tmp/prism-p4-js-exports).** Brief incl. folded codex spec-review corrections: 1 BLOCKER (two default fixtures semantically mismatched — sources rewritten before expectation flips) + typed export facts replacing the flat name set (call_graph.rs:1010), import-side work (default imports = ineligible ModuleImport today; require-bindings only in legacy extractor), confidence semantics preserved (single→Exact, multi→demoted NameOnly).

# Task P4 — Model JS/TS exports: default / export-lists / const-arrow / CJS / barrels

You work in the git worktree `/private/tmp/prism-p4-js-exports` on branch `p4-js-export-modeling` (based on main @ 900adf6). The repo is prism. Follow TDD.

## Problem (verified; the primary driver of JS ~92% unresolved)

`extract_js_ts_exported_functions` (src/ast.rs:1403) deliberately accepts ONLY `export function name(...)` — its own doc comment admits: "Default exports, re-export lists, CommonJS exports, and exported const-arrow functions need separate modeling." Imports of anything else fall through the R4c import-member arm (src/resolution.rs:1744-1790 area, gated by `supports_import_member_resolution` :144-148) to `dropped(UnknownName)`. Tier-A fixtures pin these gaps as executable contracts: `eval/fixtures/javascript/commonjs_export_deferred/` and `eval/fixtures/typescript/{arrow_const_export,default_export_function,default_import}_deferred/` assert `callers = []` + `forbid_resolution_kind = "import_member"`.

## Changes

1. **Export-fact extraction.** Spec-review ground truth: R4c requires an eligible `ImportBinding` (src/call_graph.rs:155, `ImportBindingKind::MemberImport` :170) AND consults `js_ts_exported_functions: BTreeMap<String, BTreeSet<String>>` (src/call_graph.rs:1010) — a flat exported-NAME set that cannot express `default -> local`, renames, or barrels. Replace/extend it with TYPED export facts (exported name -> local function name(s), plus re-export records), extracted starting at ast.rs:1403, so R4c can resolve the imported/exported name to local `FunctionId`s before consulting `self.functions`. Cover, for JavaScript + TypeScript + Tsx:
   a. `export default function name(...)` and `export default name` (identifier referencing an in-file function).
   b. Named export lists incl. renames: `export { a, b as c }` (local declarations only in this sub-item).
   c. Exported const-arrow / function-expression: `export const f = (...) => {...}` and `export const f = function(...) {...}`.
   d. CommonJS: `module.exports = f`, `module.exports = { a, b }`, `module.exports.f = f`, `exports.f = f` (identifier values referencing in-file functions only; arbitrary expressions are skipped and counted).
   e. Re-export chains: `export { x } from './y'` and `export * from './y'` — resolve through the target module's OWN export facts, **depth-bounded at 2** (mirror the Rust `MAX_GLOB_DEPTH` fail-closed pattern, src/name_resolution/engine.rs:107): deeper chains and cycles fail closed (no binding), counted in a telemetry bucket.
   Out of scope (skip + count where cheap): dynamic `require(expr)`/`import(expr)`, `export =` TS-CJS interop, class exports, re-exported renames-of-renames beyond depth 2.
2. **Import-side work (spec-review MAJOR — this is half the task).** (a) Default imports `import X from './y'` are currently extracted as `ImportBindingKind::ModuleImport` with `eligible = false` (src/ast.rs:1292, :1298) and R4c filters to eligible MemberImports (src/resolution.rs:1750) — make a default import a resolvable member binding for `"default"` WHEN the target module has a default export fact. (b) `const { a } = require('./y')` (+ `as`-renames) exists only in the LEGACY `extract_imports` (src/ast.rs:917) — the structured `extract_import_bindings` (src/ast.rs:1075) only walks `import_statement`s; add JS top-level require bindings there with `member=Some(original)`, `local=alias`.
2b. **R4c consumption + confidence (spec-review corrected).** Wire the typed export facts into the R4c arm (resolution.rs:1744+). Existing semantics to PRESERVE: exactly one resolved local target → `Exact` `import_member` (resolution.rs:1811); multiple targets → demoted `NameOnly` `import_member` (resolution.rs:1818). New grounded forms (default, renamed, CJS, barrel-resolved) get the same treatment. Barrel/`export *` conflicts (same name from 2+ chains) fail closed BEFORE target emission, counted.
3. **Telemetry.** Extend call-stats only if a natural existing bucket doesn't already capture the change (unresolved counts dropping is the primary signal); add the depth-exceeded/cycle counter from 1e.
4. **Cache.** Bump CPG `CACHE_VERSION` 33→34 and nav sidecar 3→4 (+ update the two version-pin tests: `cache_version_is_33_...` and `sidecar_version_is_3`) — new serialized export facts + resolution behavior change. NOTE: a parallel Python branch (P7) bumps the same constants; whichever merges second rebases and increments FROM the landed value.
5. **Fixtures (matrix; the done-check centerpiece).**
   - **BLOCKER fix first — two fixtures are semantically mismatched as written and must have their SOURCES corrected before any expectation flip**: `typescript/default_export_function_deferred/` pairs `import { process }` (named) with `export default function process` — a named import of a default-only export must NOT resolve; rewrite its app.ts to `import runProcess from "./util"` (true default-import consumption). `typescript/default_import_deferred/` pairs a default import with only `export function process` (named) — rewrite its util.ts to `export default function process` (or `export { process as default }`). THEN rewrite the `[expect]` blocks of the four capability-gap fixtures to positive contracts: `eval/fixtures/javascript/commonjs_export_deferred/`, `eval/fixtures/typescript/{arrow_const_export,default_export_function,default_import}_deferred/` — caller attributed, `resolution_kind = "import_member"` (TOML key under `[expect]`), correct `exact` set-match flag per each fixture's shape. Consider ALSO adding a negative fixture pinning each mismatch you just fixed (named-import-of-default-only → callers=[]). Optionally rename the dirs to drop `_deferred` — if you rename, update every reference (grep; coverage_test lists don't include eval fixtures, but check).
   - Do NOT touch the negative guards: `typescript/{catch_shadow,param_shadow,local_shadow,type_only_import,non_exported_function}_deferred/` and `javascript/non_exported_function_deferred/` pin correct refusals (shadowing, type-only imports, non-exported) — they must still pass UNMODIFIED. If your change breaks one of them, that is a soundness bug in your change, not a fixture to update.
   - ADD one new fixture: a depth-2 barrel chain (`index.ts` re-exporting from `impl.ts`) resolving, and a depth-3 chain NOT resolving (fail-closed pin).
6. **Skill note.** No skill edits this slice (nav skill is accurate; the change only removes false negatives).

## Tests (TDD)
Rust tests per export form (a-e) + negative guards (shadowed import stays refused; type-only import stays refused; depth-3 barrel refused; `module.exports = someExpression()` skipped). Follow the existing JS/TS resolution test patterns (tests/lang/javascript/, tests/lang/typescript/, tests/integration/import_binding_test.rs if present). Full `cargo test` + `cargo fmt` at the end; keep files under 600 lines.

## Done-checks (run and paste into your report)
```
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # 4 rewritten fixtures pass POSITIVE; all negative guards pass unmodified; 0 regressions
./target/release/prism nav call-stats --repo <a JS or TS corpus path from eval/corpora.toml, e.g. excalidraw or express if present — else pick any JS/TS repo under ~/code/bench-repos; run BASE first then branch>   # paste unresolved/kind_exact deltas
cd eval && uv run pytest tests/ -q                          # 0 failures
```

## Commit style
Small logical commits per export form group. End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
