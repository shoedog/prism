# Wrapped-export resolution: span-verified React `forwardRef` / `memo` named exports (slice S1)

**Status:** planning only. The spec review has a 2-round cap and is gated by sol. This document does not authorize
implementation until the owner answers §0. Implementation gets its own 2-round review cap, which the controller
declares before dispatch.

**Base:** `origin/main` `12ca6e8e`.

**Grounding:** `PLANNING-PROBES.md`. Mechanism claims cite its M-ids and measurements cite its P-ids.

**Predecessor:** PR #324 readout (`docs/eval/entry-call-proof/readout.md`). It names the wrapped named export as the
dominant refusal lane for the ten `forwardRef` components.

## 0. Owner decisions (before implementation)

| # | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | Which wrapper shapes are admitted | **A.** React `forwardRef` / `memo` only (`memo` may take a comparator), with callee provenance from an `import` of `"react"`. **B.** A plus known pass-through library wrappers identified by import provenance (lodash `memoize`/`debounce`/`throttle`, mobx `observer`). **C.** Shape-only: any call whose direct argument is a function, at `NameOnly`. | **A.** It is MEASURED at 107 (X) and 4 (F) new edges, with 0 wrong in the audit (P6, P7). **B** has a measured yield of **0** on all four corpora: the only pass-through helpers are in-repo (`throttleRAF`, the TS `memoize`), and no library import was seen. **C** would admit 36 more declarators, of which 27 bind a callable with the wrong semantics (a `styled` style function, collection callbacks under an array-valued `X`, a `createAsyncThunk` payload creator), for +1 dropped site (P2, P4). Reject C. Revisit B only on a corpus that shows value. |
| D2 | Confidence of admitted edges | **Exact** or **NameOnly** | **Exact.** The edge identity is the one prism already asserts as Exact for the same component via same-file JSX (`local_def`) and via `export { X }` / `export default X` (M7). The target is verified by span (§3.3), not by name. With NameOnly, the same component would get a different confidence depending only on how it was exported. |
| D3 | Slicing | **S1** (this spec) only; later, separately: **S2** default-object member aliases (`export default { Row: RowStack }`, `<Stack.Row>`), and **S3** calls inside anonymous wrapper-nested callbacks (the "unowned" lane). Or fold S2 and S3 into S1. | **S1 alone.** S2 is 7 sites on X and 0 on F (P13). It needs a new member-on-default-object R4c route, which is a different mechanism. S3 is 58 call/JSX nodes in 4 X declarators and 2 in F (P14). It needs anonymous functions to gain identity, which changes the `FunctionId` set, DFG ownership, and cache (M5), so its blast radius is far larger. Neither shares S1's code. |
| D4 | The pre-existing F4-class bypass: `const f = <non-function>; export { f }` resolves a false Exact to a nested same-name function (MEASURED P3 C06; also `let` reassignment, C21) | **a.** A follow-up slice **S1b** that span-verifies the list and default-identifier `Local` targets, reusing S1's `SpannedLocal` target. **b.** Fold it into S1. **c.** Accept and document. | **a**, queued right after S1. It is a WRONG on constructed input with 0 measured prevalence (P5: 0 nested-target Exact edges across X, F and R), and S1 does not make it worse. Folding it in would roughly double the reviewed surface: every existing list and default export route would need re-proof. S1's data model is chosen so that S1b needs no further data-model change. |
| D5 | Sequencing against module-path resolution | Ship S1 now; or first plan tsconfig `paths` / workspace-package resolution | **Ship S1 now (it is small), and make `paths` resolution the next planning lane.** Relative imports never reach 8 of the 80 E2 rows. Non-relative imports hold a projected **3,174** (X) and **2,773** (F) resolvable-terminal drops, against S1's 107 and 4 (P4 latent; a projection, not a measurement). S1 is independent of that lane and also composes with it: about 10 more A-shaped sites on each corpus become reachable once paths resolve. |

Everything below assumes the recommendations: D1 = A, D2 = Exact, D3 = S1, D4 = a, D5 = S1 now.

## 1. Problem

`export const Island = forwardRef<…>((props, ref) => …)` records no export fact, because the declarator arm admits
only arrow and function-expression initializers (M1, the F4 fix of M2). An imported `<Island/>` therefore reaches R4c
with no resolved export, and it drops as `UnknownName` (M3). On Excalidraw this accounts for 107 dropped call sites
across 41 declarators, 72 of them in the PR #324 E2 ledger (P4, P6). The inner render function already has a stable
callable identity: Pattern 3 names it after the declarator (M4), and same-file uses already bind to it Exact (M7).

## 2. Scope

**In scope.** For each top-level `export const X = W(fn…)` declarator in a `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts` or
`.tsx` file, where `W` is an admitted wrapper (§3.1), record an export fact for `X` whose target is **the exact
function node** `fn`. Re-export chains and `export *` barrels carry that target (M3; P6a C14).

**Non-goals** (each stays exactly as on base, and each is pinned by a negative test in §7):

- **Default-object member aliases** (S2), and **the anonymous nested or cast wrappers**
  `memo(forwardRef(fn))`, `Object.assign(forwardRef(fn), …)`, `forwardRef(fn) as T` (S3). Their inner function has
  no identity (M4, M5).
- **Wrappers with an identifier argument**: `memo(Comp)`, `forwardRef(Inner)`. The target is another binding. This
  is an alias through a wrapper, and its proof burden differs.
- Non-React wrappers and shape-only admission (D1).
- `let`/`var` declarators. The list and default-identifier `Local` routes (D4 → S1b).
- Non-relative module specifiers (D5).
- `ImportForward` of a wrapped export (`import {X} from './a'; export {X}`). It stays refused, because
  `forwardable_function_locals` holds function declarations only (M11).
- Any change to `Language::function_name`, `FunctionId`, call-site ownership, DFG, or Step 5b.

## 3. Semantics

### 3.1 Admission predicate (every clause must hold)

The predicate is evaluated inside the existing `lexical_declaration | variable_declaration` arm
(`src/ast.rs:2861-2893`), for a declarator `d` named by the identifier `X` whose `value` node `v` has kind
`call_expression`. Any failed clause increments `skipped_expr_count` exactly as base does, together with a reason
counter (§5).

1. **`not_const`**: the declaration is a `lexical_declaration` whose `kind` is `const`
   (`js_ts_lexical_declaration_is_const`). `let` and `var` are refused.
2. **`parse_recovery`**: the enclosing `export_statement` has no ERROR or MISSING node (`!has_error()`).
3. **Callee** (`callee_not_admitted` / `callee_provenance`): `v.function` is one of:
   - an `identifier` `L` with **exactly one** import binding in the file, where `module_path == "react"` and
     `member ∈ {"forwardRef", "memo"}`. Aliased imports such as `import { forwardRef as fr }` are allowed; the
     wrapper kind is the **imported** member name.
   - a `member_expression` `O.P`, where `P ∈ {"forwardRef", "memo"}` and `O` is an `identifier` with **exactly one**
     import binding of `module_path == "react"` that is a default import (`member == "default"`) or a namespace
     import.

   In both forms the local (`L` or `O`) must not be written anywhere in the file (`js_ts_module_value_written`), must
   not be a type-only import, and must have no competing **module-scope** declaration: no top-level function, class,
   or declarator of the same name, including inside `export` statements. The wrapper call is itself at module scope,
   so a nested declaration cannot shadow it. Do **not** reuse the nested-scope
   `js_ts_forwarding_competitor_except` (READ `src/ast/js_module_forwarding.rs:95-150`): it would refuse, for
   example, a component-local `const memo = …` and lose recall for no precision gain. Import eligibility is **not**
   used, because default and namespace bindings are extracted `eligible:false` (M10, P6a-debug). Module `"react"`
   matches exactly; `preact/compat` and others are refused (D1).
4. **Arguments** (`arity`, `first_arg_not_function`): `v.arguments` is an `arguments` node, not a template string.
   `comment` children are ignored for counting and position. The argument count must be exactly 1 for
   `forwardRef`, and 1 or 2 for `memo`. The first argument must have kind `arrow_function` or `function_expression`.
   A `spread_element` or any other kind is refused, including a `call_expression` (`nested_wrapper`) and a
   parenthesized or `as` expression.
5. **Inner identity** (`inner_unnamed`): `self.language.function_name(&fn)` is `Some(n)`. For a bare arrow this is
   `X` (Pattern 3); for `function Inner(…)` it is `Inner`. Record `local = text(n)` and
   `(start_line, end_line) = node_line_range(fn)`. Given clause 4, `None` is unreachable, and it is refused
   defensively.

If the value is not a `call_expression`, the base behavior is unchanged: arrow and function-expression initializers
still record `Local(X)`, and every other kind is skipped with reason `non_call_initializer`.

### 3.2 Data model (`src/js_exports.rs`)

- A new variant is added: `JsExportTarget::SpannedLocal { local: String, start_line: usize, end_line: usize }`. Its
  doc comment says: "an in-file callable proven by the exact line span of its function node; resolution binds only a
  FunctionId with this file, name, and span". S1 produces it only from §3.1. S1b (D4) may reuse it.
- `ResolvedJsExport` gains `#[serde(default)] pub span: Option<(usize, usize)>`. It is `None` for
  `Local`/`Class`/`UnprovenLocal`-derived results and `Some` for `SpannedLocal`.
- `resolve_one_inner` handles `SpannedLocal` as a direct hit (`is_class = false`), copying the span.
  `ReExport` chains return the terminal hit unchanged, so the span survives. The star-barrel candidate set includes
  the span in its tuple, so two identical `(file, local)` pairs with different spans count as a conflict.
- An `ImportForward` whose terminal is a `SpannedLocal` stays `BlockedClaim`, which is the current behavior (M11). It
  gets a test but no new code.
- `insert_named` poisoning is unchanged: a `SpannedLocal` claim that duplicates another claim for the same exported
  name poisons that name.
- The `imported.contains(local)` refusal at `src/ast.rs:2470-2476` applies to `Local` only. A `SpannedLocal` is a
  declaration, so it is not covered.

### 3.3 Resolution (`src/resolution.rs::js_ts_import_member_candidates`)

When `resolved.span == Some((s, e))`, the candidate filter adds `fid.start_line == s && fid.end_line == e`, on top of
the existing checks (same file, non-method). Then:

- **exactly 1 candidate:** `Exact` `ImportMember` (existing arm);
- **0 candidates:** return an empty vector, so the site drops `UnknownName`. There is no name-only fallback;
- **2 or more candidates** (a line-identity collision, e.g. a same-line nested `function X`): **return an empty
  vector.** Do not demote to NameOnly. This mirrors the v94 rule for refusing line-identity collisions
  (`cpg_cache.rs` v94 note). A span-verified target that fails to be unique is not a candidate set.

When `span` is `None`, behavior is byte-identical to base.

### 3.4 Consumers

- `navigation/call_resolve.rs` needs no change. `local_name` is the inner function's registered name, which is what
  the nav correlation compares, and each site is then re-resolved through §3.3 (M11; P12).
- The CPG Step 5 and Step 5b consumers inherit the edges through `resolve_call_site`. JSX sites carry no arguments,
  so there is no new DataFlow on X (P11). A direct call such as `Island(p)` to an admitted component would get a Call
  edge and argument edges. At runtime, `forwardRef` and `memo` return objects that are not callable, so such code is
  already broken. This is noted and accepted (SMELL-level; see REVIEWER).

## 4. Fail-closed rules for each negative control

| Control (P3 id) | Rule that refuses it |
|---|---|
| F4 ternary + nested same name (C07) | not a `call_expression` → `non_call_initializer` (base) |
| `forwardRef(Ident)` ± nested same-name function (C09, C10) | §3.1.4 `first_arg_not_function`; no fact is recorded, so the nested `Island` is never consulted |
| wrapped + nested same name elsewhere (C11) | §3.3 span filter: only `@6-8` matches, never the nested `@3-3` |
| same-line nested same name (new, T-N11) | §3.3: 2 or more span matches → empty result |
| local impostor `function forwardRef` (C12) | §3.1.3: no `"react"` import binding |
| `import { forwardRef } from './mine'` / `preact/compat` (T-N4) | §3.1.3 exact module match |
| duplicate or written React binding (`React = x`, two imports) (T-N10) | §3.1.3 uniqueness and not-written |
| `create(fn)`, `createSelector(a, fn)`, `styled(…)(fn)`, `arr.map(fn)`, `observer(fn)`, `debounce(fn)` (C13, C24, C16, C19, C23, T-N6) | §3.1.3 `callee_not_admitted` |
| `memo(forwardRef(fn))`, `Object.assign(forwardRef(fn), …)`, `forwardRef(fn) as any` (C08, C25) | §3.1.4 `nested_wrapper`, or not a `call_expression` (cast) |
| `forwardRef(a, b)`, `memo(fn, cmp, x)`, `forwardRef(...args)` (T-N8) | §3.1.4 `arity` / `first_arg_not_function` |
| `export let/var` (C20) | §3.1.1 `not_const` |
| importer parameter shadow (C18) | the existing `js_ts_function_locals` guard (unchanged) |
| duplicate claim: `export const X = memo(fn); export { other as X }` (T-N13) | `insert_named` poisons `X` |
| export statement with parse recovery (T-N12) | §3.1.2 `parse_recovery` |
| three-hop re-export | existing `MAX_REEXPORT_DEPTH` (unchanged) |

## 5. Counters and observability

- `JsExportFacts` gains two fields, both persisted:
  - `#[serde(default)] pub spanned_admitted: usize`
  - `#[serde(default)] pub skipped_decl_reasons: BTreeMap<String, usize>`, keyed by the §3.1 reason names plus
    `non_call_initializer`. Only declarator-arm skips are keyed here. Default-export and CJS skips keep feeding only
    `skipped_expr_count`.

  `is_empty()` includes both fields.
- `skipped_expr_count` keeps its meaning: the total of all skips. For each file, the declarator part of it equals the
  sum of `skipped_decl_reasons`. A test pins this invariant.
- call-stats (`src/navigation/queries.rs:626-636`) gains `js_export_spanned_admitted` (the sum) and
  `js_export_skipped_decl_reasons` (a BTreeMap merged across files, emitted as a JSON object with sorted keys).
  `js_export_skipped_exprs` is unchanged in name and meaning.
- Expected values on X: `js_export_skipped_exprs` 637 → 596, and `js_export_spanned_admitted` = 41.
  `unresolved_unknown_name` −107, and `kind_exact.import_member` +107 (P6). On F: 389 → 358, 31, −4, +4.

## 6. Cache

Raw export facts change shape and resolution behavior changes, so bump both versions and their pins, and add one
version note each:

- `src/cpg_cache.rs`: `CACHE_VERSION` 97 → **98**, with the pin in `cache_versions_are_pinned_for_cpg_semantics`
  updated. Note: "v98: JS/TS span-verified wrapped React exports (`SpannedLocal`) and declarator skip reasons."
- `src/navigation/call_edge_cache.rs`: `NAV_CALL_EDGE_CACHE_VERSION` 53 → **54**, with the pin at `:716` updated and
  a note in the same style.

If a PR merged before this one has bumped either constant, increment from the landed value.

## 7. Test plan

**RED rule.** Every positive test (T-P*) and every observability test (T-O*) must fail on base `12ca6e8e` with a
behavioral assertion, not a compile error. Capture the RED by temporarily reverting only the §3.1–3.3 production
lines, not by running the tests on a tree without the new types. Record the concrete failing value, for example
"left: [] right: [(Exact, ImportMember)]". Negative tests (T-N*) are guards: they pass on base and must still pass.
Each new code path has at least one positive and one negative test.

**Location.** Put the unit and resolution tests in a new `tests/integration/js_wrapped_export_test.rs` (under 600
lines; split into `js_wrapped_export_refusal_test.rs` if needed). Put the state transitions in
`tests/integration/js_wrapped_export_state_test.rs`. Register both in `tests/integration/main.rs`. Put the CLI
counter test in `tests/cli/call_stats_test.rs` and the nav test in the existing `tests/navigation/callers_test.rs`.
Follow the helpers and assertion style of `tests/integration/js_export_test.rs` (`files`, `resolve_kind`) and
`esm_forwarding_state_test.rs`. Assert the exact target `(file, name, start_line, end_line)` wherever a target is
expected, with a same-name decoy present wherever it is cheap to add one.

Positive tests. T-P1 through T-P5 run in JS (`.jsx`) and TSX; the others run in TSX only.

| ID | Input | Expected |
|---|---|---|
| T-P1 | `import { forwardRef } from "react"; export const Island = forwardRef((p, r) => …)`; `<Island/>` imported | Exact `ImportMember` → `Island@span(arrow)`; also `forwardRef<A, B>(…)` generics in TSX |
| T-P2 | `import React from "react"` + `React.forwardRef`; and `import * as React` | Exact (the C26 and C27 shapes) |
| T-P3 | `memo(fn)` and `React.memo(fn)` | Exact |
| T-P4 | `memo(fn, (a, b) => true)` | Exact → the **first**-argument span only (the comparator arrow is also named X by Pattern 3) |
| T-P5 | `import { forwardRef as fr } from "react"` | Exact |
| T-P6 | `forwardRef(function IslandImpl(p, r) {…})` | Exact → `IslandImpl@span` |
| T-P7 | wrapped export plus a nested `function Island` elsewhere (C11) | Exact → the wrapper span only, with exactly 1 target |
| T-P8 | `index.ts: export { Island } from "./Island"`, depth 2 (C14); also `export * from "./Island"` | Exact through the chain |
| T-P9 | `export const A = forwardRef(…), B = memo(…)` (C17) | both Exact |
| T-P10 | comment argument `forwardRef(/* c */ (p, r) => …)` | Exact |

Negative tests (guards):

| ID | Input | Expected |
|---|---|---|
| T-N1 | `forwardRef(IslandInner)` + nested `function Island` (C10) | no `ImportMember`; the site drops |
| T-N2 | `memo(forwardRef(fn))`, `Object.assign(forwardRef(fn), {})`, `forwardRef(fn) as any` | drop |
| T-N3 | local `function forwardRef` impostor (C12) | drop |
| T-N4 | `forwardRef` imported from `"./react-shim"` and from `"preact/compat"` | drop |
| T-N5 | `export let` and `export var` wrappers (C20) | drop |
| T-N6 | `create(fn)`, `createSelector(a, fn)`, `styled("div")(fn)`, `items.map(fn)`, `observer(fn)`, `debounce(fn, 1)` | drop |
| T-N7 | importer parameter shadow (C18) | not `ImportMember` |
| T-N8 | `forwardRef(a, b)`, `memo(fn, cmp, x)`, `forwardRef(...args)` | drop |
| T-N9 | `import {X} from "./a"; export {X}` over a wrapped `X` | drop (ImportForward refusal is unchanged) |
| T-N10 | `React = other;` after the import; `import React from "react"` twice; a top-level `function memo(){}` next to `import { memo }`. **Plus positive twins:** `Island.displayName = "Island"` still resolves Exact, which pins M10's member-write reading; and a component-local `const memo = 1` inside another function does not refuse | drop ×3 / Exact ×2 |
| T-N11 | single line: `export const X = forwardRef((p, r) => { function X() {} return null; });` | drop (2 span matches) |
| T-N12 | an export statement containing a parse error inside the wrapper | drop |
| T-N13 | `export const X = memo(fn); export { other as X };` | drop (poisoned) |
| T-N14 | the pinned F4 test and the existing P4/P7 JS export suites | unchanged and passing |

Observability, state, cache, and nav:

| ID | Test | Expected |
|---|---|---|
| T-O1 | extraction unit: for each §3.1 reason, one fixture | `skipped_decl_reasons[reason] == 1`, `skipped_expr_count` delta 1, `spanned_admitted` 0; admitted fixture: `spanned_admitted == 1` and no reason |
| T-O2 | CLI `prism nav --no-cache call-stats --repo <tmp fixture>` | `js_export_spanned_admitted` and `js_export_skipped_decl_reasons` present, with exact values |
| T-S1 | state epochs (full build **and** `build_incremental`, JS/TS/TSX): admitted → wrapper import changed to `"./shim"` (refused) → restored → inner arrow moved two lines down (span changes) → restored | edge set equals the expectation in every epoch, and full equals incremental |
| T-C1 | raw facts serialize and deserialize (`serde_json` round-trip of `JsExportFacts` with `SpannedLocal` and reasons), plus a CPG cache full-hit that preserves the Exact edge | equal |
| T-C2 | pin tests | 98 and 54 |
| T-V1 | nav `callers --symbol Island --file lib.tsx` on a temp fixture | 1 caller, `import_member`, score 1.0 (P12 shape) |

Tier-A fixtures (`eval/fixtures/typescript/`, same `expected.toml` schema as `arrow_const_export_deferred`):

- `forwardref_named_export/`: `lib.tsx` + `app.tsx` (C02), `exact = true`, `resolution_kind = "import_member"`.
  RED on base.
- `forwardref_ident_arg_nested_samename_refused/` (C10): `callers = []`,
  `forbid_resolution_kind = "import_member"`.
- `memo_named_export/` (T-P3 shape). **Optional**; at most 3 new fixture directories in total.

## 8. Acceptance

1. **Suites green.** `cargo fmt --check`; `cargo test --offline --no-fail-fast` (base 4,559 / 0 / 1, P9; the new
   total is reported); `cargo test --offline --features mcp`; `cargo clippy --offline --all-targets --features mcp`;
   Tier-A `--matrix-only` (base 159/159, P10), plus the new fixtures, with 0 regressions; and the Node gate
   (`node scripts/gate-inputs/acquire.mjs && node scripts/gate-inputs/gate.mjs --out target/gate-runs/<name>`).
   `tier-a --quick` needs rust-analyzer. Run it if it is available, and otherwise report it as not run.
2. **Yield re-measurement (the acceptance criterion).** Run the implementation's release binary on each corpus with
   `nav --no-cache call-stats --dump-sites`, then `probes/rowdiff.py` against the base dumps. The P1 files have their
   hashes in the evidence `MANIFEST.sha256`.
   - **X:** exactly the 107 rows of `probes/P6-excalidraw-expected-rowdiff.json` (SHA-256 `265790a3…`), every one
     `UnknownName → Exact import_member` with an identical target, and 0 other changed rows. `probes/audit.py` 107/107.
     `dfg-stats --edges` byte-identical to base (P11).
   - **F:** 4 rows, all `UnknownName → Exact`, audited 4/4, and 0 other changes.
   - **R and T:** dumps byte-identical to base.
   - Counters as in §5.

   Any deviation, whether a missing row, an extra row, or a changed target, is reported row by row with a
   mechanism. It is a blocker until the owner accepts it. A lower count caused by the predicates that are stricter
   than the prototype (§3.1.2 parse recovery, and §3.1.3 uniqueness and module-scope competitor) must name the
   refused declarator and its reason counter.
3. **Custody.** The acceptance outputs go under `~/prism-evidence/wrapped-export/acceptance/`, with a
   `MANIFEST.sha256`.

## 9. Budget (honest lines: after `rustfmt`, non-blank, non-`//`, `#[cfg(test)]` and `tests/**` counted as tests)

**Analogues (P8, `probes/honest_lines.py`)**, all on this same `JsExportFacts`/R4c seam:

| Commit | Slice | src | tests |
|---|---|---|---|
| `c3d110ef` | ESM imported-local forwarding: new variant, proof, cache bump | 160 | 491 |
| `032e1824` | CJS producer export-object fences | 157 | 303 |
| `88af6511` | CJS terminal capture proof | 231 | 457 |
| `4ffe55bf` | module-binding audit | 94 | 670 |

**Prototype:** 120 honest lines, with no counters, cache, or test fix-ups. Remaining src work: the reason counters and
call-stats (about 30), the cache and pins (about 4), the comment-skipping and module-scope competitor/uniqueness
clauses (about 15), and module plumbing (about 10).

| Bucket | Cap | Early stop |
|---|---|---|
| src (non-test Rust) | **200** | 180 |
| tests (Rust) | **450** | 405 |
| combined | **650** | 585 |
| Tier-A fixture directories | 3 | – |

New production logic goes in a new submodule, `src/ast/js_wrapped_export.rs`, following
`src/ast/js_module_forwarding.rs`, because `ast.rs` is far over the 600-line rule. `ast.rs` itself should gain only
the call-site arm. The slice is sized to converge within the 2-round cap. The prototype shows the mechanism is
local: 3 files and 1 filter. If the forecast exceeds a cap, stop and report the count and the remaining items. Do not
compress logic to fit.

## 10. Risks and open questions for review

- **Semantic claim.** Exact is justified by React's contract that rendering `<X/>` invokes the wrapped render function
  (ASSUMPTION, M7 precedent). A reviewer who rejects that framing should argue D2, not the mechanism.
- **Direct calls of admitted components** (`X(props)`) get Call and argument edges (§3.4). Excluding non-JSX sites
  would need a site-kind flag on `CallSite` that does not exist today. P6 measured 0 non-JSX occurrences.
- **Pattern 3 over-naming** (M4) gives a comparator arrow or a `createSelector` result function the declarator's
  name. That is pre-existing and not changed here. §3.3's span filter makes S1 immune to it.
