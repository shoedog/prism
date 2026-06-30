# CPG Speed, Recall, and Precision Follow-ups

Status: draft, A2A spec-review and plan-review findings folded
Primary slice: JS/TS Tier-A coverage plus import-member resolution
Date: 2026-06-30

## Context

This document captures five improvement candidates from a repo-level review of
Prism's CPG, call resolution, and navigation paths. The priorities are CPG
speed, recall, and precision across Python, JavaScript/TypeScript, Go, and
Rust.

Two follow-ups from older docs should not be treated as open top-level work:

- Shared CPG construction for multi-algorithm CLI runs is already present:
  `src/main.rs` builds one `CpgContext` before iterating algorithms.
- JS/TS destructuring and optional-chain access-path support are already
  present in `src/ast.rs` and `src/access_path.rs`.

The current higher-leverage gap is narrower. JS/TS import binding extraction is
already present, `file_matches_module` already documents JS/TS-style module
paths, and the call graph stores import bindings for Python/JS/TS. The resolver
still gates the import-member rung to Python files, and the Tier-A fixture
matrix does not yet run JS/TS cases.

## Five-Item Roadmap

| Priority | Theme | Item | Why now | Main files |
|---|---|---|---|---|
| P0 | Recall, precision | Add JS/TS Tier-A coverage, then enable JS/TS import-member resolution. | Tier-A v1 explicitly deferred TS/JS with priority. JS/TS fixture dirs are absent, and the matrix callers hardcode the language allowlist. Import bindings already exist for JS/TS, so the first implementation slice is local if Exact soundness is tightened. | `eval/fixtures/`, `eval/tier_a/matrix.py`, `eval/tier_a/cli.py`, `eval/tests/test_matrix.py`, `src/resolution.rs`, `src/ast.rs`, `src/call_graph.rs`, `tests/integration/import_binding_test.rs` |
| P1 | Recall | Fix Python inherited override resolution. | The current matrix reports `python/inherited_override` as an expected gap. The resolver's inherited-self helper is intentionally depth-1, same-file only. | `src/resolution.rs`, `eval/fixtures/python/` |
| P1 | Speed | Precompute resolved navigation call-edge indexes. | Caller/callee navigation repeatedly scans and resolves call sites per query. Precomputed forward/reverse resolved-edge maps would speed repeated MCP queries without changing semantics. | `src/navigation/mod.rs`, `src/navigation/queries.rs`, `src/navigation/call_resolve.rs` |
| P2 | Speed | Use incremental partial-hit cache refresh for navigation. | The CLI CPG cache supports partial hits, and `NavigationIndex::build_incremental_from_previous` exists, but nav cache partial hits are currently treated as misses. | `src/navigation/cache.rs`, `src/navigation/mod.rs`, `src/cpg/build.rs` |
| P2 | Recall | Model one unmodeled language construct at a time. | The warning table identifies constructs that make `NotReached` non-proving: Python comprehensions, JS/TS async callbacks, Go channels/goroutines, and Rust closures/`?`. | `src/languages/mod.rs`, `src/data_flow.rs`, `src/call_graph.rs`, fixtures |

## Design: JS/TS Tier-A Coverage Plus Import-Member Resolution

### Goal

Add a measured JS/TS call-resolution slice that improves recall without
weakening existing collision precision.

The slice has three stages:

1. Wire JS/TS languages into the Tier-A matrix callers and add fixtures that
   prove the matrix actually executes those cases.
2. Add deterministic JS/TS fixtures that expose the current behavior.
3. Generalize the existing R4c import-member resolver from Python-only to
   JS/TS named imports only when the edge is safe to report as `Exact`.

### Non-Goals

The first slice deliberately does not implement:

- TypeScript path alias resolution from `tsconfig.json`.
- Node package `exports`, npm dependency graph resolution, or package `exports`
  maps.
- Dynamic `import()`, `require()`, CommonJS interop, or re-export chains.
- `tsserver`, SCIP, or LSP-based oracles.
- Full namespace-member resolution such as `utils.process()` from
  `import * as utils from "./util"`.
- Default imports such as `import run from "./util"`.
- Arrow-const exports such as `export const process = () => {}`.
- JS/TS `ImportMember` edges when a function parameter or local binding shadows
  the imported local name.

Those are valid later slices, but they widen the first change beyond the
existing R4c seam.

### Current Evidence

- Tier-A v1 languages are Rust, Go, and Python. The Tier-A design records
  TS/JS oracle and corpora as the highest-priority follow-up.
- `eval/tier_a/matrix.py` iterates the language list supplied by callers; it
  does not auto-discover language directories.
- The current Tier-A CLI and no-regressions test hardcode
  `["rust", "go", "python"]`.
- `src/ast.rs::extract_import_bindings` already handles
  `Language::JavaScript`, `Language::TypeScript`, and `Language::Tsx`.
- `src/call_graph.rs::ImportBinding` and `ImportBindingKind` are documented as
  Python/JS/TS structures.
- `src/call_graph.rs::file_matches_module` already handles JS/TS slash-based
  module paths, but it also has a stem fallback that is too permissive for
  `Exact` JS/TS import-member resolution.
- `src/resolution.rs` R4c is documented as "Python/JS/TS", but the branch is
  gated by `caller.file.ends_with(".py")`.
- Earlier import-binding design notes deferred JS named imports because export
  soundness needed a separate decision. This design makes that decision explicit:
  first-slice JS/TS `Exact` requires an exported function declaration and no
  lexical shadow.

### Architecture

Reuse the existing call-resolution architecture and add only the facts needed
to make JS/TS `Exact` sound:

1. `ParsedFile` extracts module-level import and module-binding facts.
2. `CallGraph::extract_all_import_bindings` collects those facts for all files
   and runs `mark_import_binding_eligibility`.
3. Add JS/TS export facts for function declarations only. Prefer a separate
   `js_ts_exported_functions: BTreeMap<String, BTreeSet<String>>`-style
   structure over overloading `ModuleBindingKind`; if it is stored in cached
   CPG data, it must be wired through every `CallGraph` constructor and
   incremental path: empty graph construction, skeleton construction, full
   build, direct subset build, `remove_files`, `merge`, serde defaults, and
   cache-version/cache tests.
4. Add conservative JS/TS lexical-shadow facts for function bodies. A
   function-level set of parameter/local names is acceptable for the first
   slice; it may drop true positives, but it prevents false `Exact` edges. If
   this is stored on `CallGraph`, it has the same constructor/cache obligations
   as export facts. If it is transient, say so in the implementation.
5. `CallGraph::resolve_call_site_full` applies the resolution ladder. Python
   R4c remains unchanged. JS/TS R4c runs only after the export and shadow checks
   pass.
6. Navigation and Tier-A callers consume the resolved `FunctionId` identities.

No new resolver subsystem is needed. The implementation belongs in the existing
R4c rung plus the existing import/module fact extraction layer.

### Exact-Soundness Policy

For Python, preserve the current R4c behavior.

For JS/TS, report `ResolutionKind::ImportMember` with `Exact` only when all of
these are true:

- The call is unqualified.
- The caller extension is one of the scoped JS/TS extensions for this slice:
  `.js` and `.ts` initially. Add `.tsx` only with a TSX fixture.
- The binding exists in `self.import_bindings[caller.file]`.
- The binding local matches the call name.
- The binding is `eligible`.
- The binding kind is `MemberImport`.
- The target member name is present.
- The target file is matched by a JS/TS exact relative-module helper, not by
  stem fallback. The helper should accept `./` and `../` candidates plus
  `index.js`, `index.ts`, and `index.tsx` under the resolved directory.
- The target is a module-level exported function declaration populated only
  from `export_statement > function_declaration`, not from bare function
  declarations.
- The target is not a method owner.
- The caller function does not bind the call name as a parameter or local.

If any JS/TS-specific soundness check fails, do not mint an `Exact`
`ImportMember` edge. Fall through unchanged or return a demoted result only if
the implementation deliberately adds and tests a demoted JS/TS policy. The first
slice is simpler if failure falls through.

### Tier-A Matrix Wiring

Adding fixture directories is not enough. The implementation must extend the
Tier-A language allowlist at the three real matrix call sites:

- `eval/tier_a/cli.py` inside the normal `run_corpus` matrix pass.
- `eval/tier_a/cli.py` inside `--matrix-only`.
- `eval/tests/test_matrix.py` in the real-binary no-regressions test.

Add a small helper such as `MATRIX_LANGUAGES = ["rust", "go", "python",
"javascript", "typescript"]` to avoid repeating the list. Include `"tsx"` only
if TSX fixtures are added in the same slice.

Do not blindly replace the synthetic `FakeSut`/`FlipSut` self-test language
lists; those can stay scoped to the existing Rust/Go/Python fake coverage
unless the fake data is expanded.

The matrix-only validation must show JS/TS rows in stdout before any fixture
status is flipped to `pass`.

### Fixture Plan

Add fixtures before changing resolver behavior. Use the existing fixture shape,
but standardize status on `known_fail` for non-passing cases unless the harness
schema is deliberately updated to bless another status value.

Extend `expected.toml` to assert the rung. The Tier-A model already carries
`resolution_kind`, but the matrix comparison currently checks only caller site
sets. Add concrete matrix support for both positive and negative metadata:

```toml
[expect]
exact = true
resolution_kind = "import_member"
```

and, for negative precision cases:

```toml
[expect]
exact = false
forbid_resolution_kind = "import_member"
```

Concrete harness edits:

- Add `Case.expected_resolution_kind: str | None`.
- Add `Case.forbid_resolution_kind: str | None`.
- Parse both fields in `load_case`.
- In `_run_matrix_inner`, compare `expected_resolution_kind` against returned
  edges at expected call sites.
- In `_run_matrix_inner`, fail a case if any returned edge at the relevant
  caller site has `forbid_resolution_kind`.
- Include kind expectations in `CaseResult`/JSON.
- Add `eval/tests/test_matrix.py` fake-SUT coverage where the right caller site
  with the wrong resolution kind fails.

Minimum fixture set:

| Directory | Language | Purpose | Expected behavior |
|---|---|---|---|
| `eval/fixtures/javascript/named_import_alias` | javascript | `import { process as run } from "./util"; run()` with `export function process`. | Resolves to `util.js::process` via `import_member`. |
| `eval/fixtures/javascript/non_exported_function_deferred` | javascript | Named import, target has only a bare non-exported function. | Forbids exact `import_member`. |
| `eval/fixtures/javascript/commonjs_export_deferred` | javascript | `module.exports = { process }`. | Stays out of this slice; forbids exact `import_member`. |
| `eval/fixtures/typescript/named_import_alias` | typescript | Same as JS, `.ts` extension. | Resolves to `util.ts::process` via `import_member`. |
| `eval/fixtures/typescript/common_name_collision` | typescript | Two files define/export `process`; caller imports one. | Resolves only the imported module member. |
| `eval/fixtures/typescript/rebound_import_local` | typescript | Imported local is rebound at module scope. | Forbids exact `import_member`. |
| `eval/fixtures/typescript/duplicate_import_local` | typescript | Two imports bind the same local name. | Forbids exact `import_member`. |
| `eval/fixtures/typescript/param_shadow_deferred` | typescript | Function parameter shadows imported local. | Forbids exact `import_member`. |
| `eval/fixtures/typescript/local_shadow_deferred` | typescript | Local variable shadows imported local. | Forbids exact `import_member`. |
| `eval/fixtures/typescript/non_exported_function_deferred` | typescript | Imported name exists as non-exported module function. | Forbids exact `import_member`. |
| `eval/fixtures/typescript/arrow_const_export_deferred` | typescript | `export const process = () => {}` plus aliased named import. | Stays `known_fail` in this slice. |
| `eval/fixtures/typescript/default_import_deferred` | typescript | `import run from "./util"; run()`. | Stays `known_fail` in this slice. |
| `eval/fixtures/typescript/index_module` | typescript | `import { process } from "./util"` to `util/index.ts`. | Resolves via exact index candidate if implemented. |
| `eval/fixtures/typescript/wrong_directory_same_stem` | typescript | Caller imports `./util`, caller dir lacks `util.ts`, unrelated `elsewhere/util.ts` exports the symbol. | Forbids exact `import_member`. |

Optional follow-up fixtures:

- `eval/fixtures/tsx/named_import_alias` if `.tsx` is enabled in the helper and
  language allowlist.
- `.mjs`, `.cjs`, and `.jsx` caller/target cases. Prism parses those as
  JavaScript, but they are out of first-slice scope unless fixtured.
- `typescript/namespace_import_deferred` for `import * as util from "./util"`;
  keep it out of R4c because the call is qualified.

### Resolver Change Shape

Refactor the R4c guard from:

```rust
if site.qualifier.is_none() && caller.file.ends_with(".py") {
```

to a helper that scopes first-slice callers:

```rust
fn supports_import_member_resolution(file: &str) -> bool {
    file.ends_with(".py") || file.ends_with(".js") || file.ends_with(".ts")
}
```

Then split the JS/TS path from the Python path inside R4c:

- Python keeps the existing `file_matches_module` plus module-level
  `FunctionDef` filter.
- JS/TS uses a stricter exact-relative module matcher with no stem fallback.
- JS/TS requires exported function declaration facts.
- JS/TS rejects exact import-member resolution when the caller function has a
  parameter or local binding for the imported local name.

Keep the branch before the bare `functions.get(name)` lookup because aliases
make the call-site name differ from the target function name.

### Default and Arrow-Const Decision

Current JS default-import extraction records:

- `kind = ModuleImport`
- `member = Some("default")`
- `eligible = false`

Do not change that in this slice. Default exports require agreeing on how Prism
represents anonymous/default function definitions and assignment exports.

Also do not resolve `export const process = () => {}` in this slice. Current
module binding classification treats JS/TS variable declarations as
`Assignment`, even though the arrow may produce a named `FunctionId`. Named
`export function process()` declarations are the first-slice target.

### Review and Test Gates

Before commit:

```bash
cargo build --release
cd eval && uv run pytest tests/test_matrix.py
cargo test --test import_binding_test
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```

Because this touches call-resolution behavior, the full pre-review gate is:

```bash
cargo build --release
cd eval && uv run pytest tests/test_matrix.py
cargo test --test import_binding_test
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

Use `--allow-stale-sut` only with the immediate preceding rebuild in the same
worktree.

The matrix-only output must include `javascript/...` and `typescript/...` rows.

### Implementation Tasks

1. Add a shared Tier-A matrix language list and include `javascript` and
   `typescript` at the three real matrix call sites only.
2. Extend the matrix comparison with `resolution_kind` and
   `forbid_resolution_kind`, plus fake-SUT tests for wrong-kind failure.
3. Add JS/TS fixtures and mark current unresolved positives as `known_fail`;
   use `forbid_resolution_kind = "import_member"` for negative precision cases.
4. Run matrix-only and confirm JS/TS rows appear as gaps before resolver changes.
5. Move Rust integration expectations before implementation:
   - Flip the ESM `export function` named-import test to expect `ImportMember`.
   - Keep CommonJS `module.exports` as a retained negative.
   - Add tests for named-import alias, wrong-file collision, default import,
     namespace import, duplicate local, module-scope rebound, parameter shadow,
     local shadow, non-exported function, arrow-const export, exact relative
     matching, index matching, and wrong-directory same-stem rejection.
6. Add JS/TS exported-function facts for `export function` declarations only.
   If stored on `CallGraph`, wire every constructor/incremental/cache path and
   serde default/cache-version policy in the same task.
7. Add conservative JS/TS lexical-shadow facts for function parameters and local
   bindings. State whether these facts are stored or transient; if stored, wire
   the same constructor/cache paths.
8. Add a strict JS/TS relative module matcher for R4c that does not use stem
   fallback. Include acceptance tests for `./util`, `../util`,
   `./util/index.ts`, no wrong-directory fallback, no bare/package specifier,
   and the chosen extension scope.
9. Add `supports_import_member_resolution` and replace the Python-only R4c guard
   with Python/JS/TS branching that preserves current Python behavior.
10. Flip only the fixtures that now pass with the intended `import_member` rung.
11. Run focused tests, `cargo build --release`, and Tier-A matrix-only.
12. Run Tier-A quick before review.

### Risks

- JS/TS export detection can become a partial type/module system. Mitigation:
  only `export function name` declarations are in scope.
- Function-level shadow facts can be conservative. Mitigation: false negatives
  are acceptable in the first slice; false `Exact` positives are not.
- `file_matches_module` stem fallback is too broad for JS/TS `Exact`.
  Mitigation: use a JS/TS R4c-specific exact relative matcher.
- The matrix can pass by site set through another resolver rung. Mitigation:
  assert `resolution_kind` for the R4c fixtures.
- JS/TS corpus precision remains unmeasured. Mitigation: state that this slice
  is fixture-measured only; tsserver/SCIP oracle work is a later slice.

## A2A Review Record

Architecture/spec review command:

```bash
/Users/wesleyjinks/code/a2a-bridge/target/release/a2a-bridge run-workflow spec-review \
  --input /private/tmp/jsts-tier-a-import-resolution-spec-review.md \
  --session-cwd /Users/wesleyjinks/code/slicing \
  --config /Users/wesleyjinks/code/a2a-bridge/examples/a2a-bridge.slicing-spec-review.toml \
  --out /private/tmp/jsts-tier-a-import-resolution-spec-review.out.md
```

Spec review result: completed. Key findings folded here:

- JS/TS fixtures must be wired into Tier-A language allowlists, not just added
  as directories.
- JS/TS `Exact` import-member needs explicit export and lexical-shadow
  soundness decisions.
- R4c fixtures should assert `resolution_kind`, not just caller site sets.
- Arrow-const exports, default imports, and broad package/module matching stay
  deferred.

Plan review command:

```bash
/Users/wesleyjinks/code/a2a-bridge/target/release/a2a-bridge run-workflow plan-review \
  --input /private/tmp/jsts-tier-a-import-resolution-plan-review.md \
  --session-cwd /Users/wesleyjinks/code/slicing \
  --config /Users/wesleyjinks/code/a2a-bridge/examples/a2a-bridge.slicing-plan-review.toml \
  --out /private/tmp/jsts-tier-a-import-resolution-plan-review.out.md
```

Plan review result: completed. Key findings folded here:

- `resolution_kind` assertions need concrete `matrix.py` fields and tests.
- Negative fixtures need a `forbid_resolution_kind` contract, not ambiguous
  "known_fail or assert no edge" wording.
- Any stored `CallGraph` fact needs explicit constructor, incremental, serde,
  and cache-version wiring.
- Existing ESM tests should flip to `ImportMember`; CommonJS tests should stay
  retained negatives.
- Focused `pytest` and Rust import-binding tests should run before Tier-A.
