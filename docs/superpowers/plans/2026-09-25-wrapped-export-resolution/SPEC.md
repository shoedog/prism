# Wrapped-export resolution: span-verified React `forwardRef` / `memo` named exports (slice S1)

**Status:** planning only, revision r2. This revision folds sol's spec round 1 (FIX, 5 WRONG / 2 SMELL; see
`REVIEW-r1-fold.md`). The spec review has a 2-round cap, and round 2 is the final one. Implementation gets its own
2-round review cap, which the controller declares before dispatch.

**Base:** `origin/main` `12ca6e8e`.

**Grounding:** `PLANNING-PROBES.md`. Mechanism claims cite its M-ids and measurements cite its P-ids.

**Predecessor:** PR #324 readout (`docs/eval/entry-call-proof/readout.md`). It names the wrapped named export as the
dominant refusal lane for the ten `forwardRef` components.

## 0. Owner decisions

| # | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | Which wrapper shapes are admitted | **A.** React `forwardRef` / `memo` only (`memo` may take a comparator), with callee provenance from an `import` of `"react"`. **B.** A plus known pass-through library wrappers identified by import provenance (lodash `memoize`/`debounce`/`throttle`, mobx `observer`). **C.** Shape-only: any call whose direct argument is a function, at `NameOnly`. | **A.** It is MEASURED at 107 (X) and 4 (F) new edges, with 0 wrong in the audit (P6, P7). **B** has a measured yield of **0** on all four corpora: the only pass-through helpers are in-repo (`throttleRAF`, the TS `memoize`), and no library import was seen. **C** would admit 36 more declarators, of which 27 bind a callable with the wrong semantics (a `styled` style function, collection callbacks under an array-valued `X`, a `createAsyncThunk` payload creator), for +1 dropped site (P2, P4). Reject C. Revisit B only on a corpus that shows value. |
| D2 | Confidence of admitted edges | **Exact** or **NameOnly** | **Exact, on two conditions that r2 makes part of the contract.** (i) Only JSX element sites may bind a `SpannedLocal` target (§3.4). (ii) The provenance of the React module object is closed by occurrence custody (§3.2, D6). The edge identity is the one prism already asserts as Exact for the same component, via same-file JSX (`local_def`) and via `export { X }` / `export default X` (M7). |
| D3 | Slicing | **S1** (this spec) only; later, separately: **S2** default-object member aliases (`export default { Row: RowStack }`, `<Stack.Row>`), and **S3** calls inside anonymous wrapper-nested callbacks (the "unowned" lane). Or fold S2 and S3 into S1. | **S1 alone.** S2 is 7 sites on X and 0 on F (P13). It needs a new member-on-default-object R4c route, which is a different mechanism. S3 is 58 call/JSX nodes in 4 X declarators and 2 in F (P14). It needs anonymous functions to gain identity, which changes the `FunctionId` set, DFG ownership, and cache (M5), so its blast radius is far larger. Neither shares S1's code. |
| D4 | The pre-existing F4-class bypass: `const f = <non-function>; export { f }` resolves a false Exact to a nested same-name function (MEASURED P3 C06; also `let` reassignment, C21) | **a.** A follow-up slice **S1b** that span-verifies the list and default-identifier `Local` targets, reusing S1's `SpannedLocal` target. **b.** Fold it into S1. **c.** Accept and document. | **a**, queued right after S1. It is a WRONG on constructed input with 0 measured prevalence (P5: 0 nested-target Exact edges across X, F and R), and S1 does not make it worse. Folding it in would roughly double the reviewed surface: every existing list and default export route would need re-proof. S1's data model is chosen so that S1b needs no further data-model change. |
| D5 | Sequencing against module-path resolution | Ship S1 now; or first plan tsconfig `paths` / workspace-package resolution | **Ship S1 now, and make `paths` resolution the next planning lane.** Relative imports never reach 8 of the 80 E2 rows. Non-relative imports hold a projected **3,174** (X) and **2,773** (F) resolvable-terminal drops, against S1's 107 and 4 (P4 latent; a projection, not a measurement). S1 is independent of that lane and also composes with it: about 10 more A-shaped sites on each corpus become reachable once paths resolve. |
| D6 (r2) | How to close the provenance of the mutable React object (sol W1) | **(a)** Keep the default and namespace member forms (`React.forwardRef`), and apply closed **occurrence custody** (§3.2) to every default or namespace `"react"` binding in the file. **(b)** Admit named imports only (`import { forwardRef }`). Custody still applies (see the rationale). | **(a).** MEASURED P18: **(a)** gives X **107** and F **4** new Exact edges. **(b)** gives X **19** and F **0**. Both have 0 audited wrong edges, and custody refused 0 real declarators on either corpus. **(b) does not remove the custody analysis**: under CommonJS interop, a named import reads the same module object, so a same-file `React.forwardRef = fake` affects `forwardRef` too (control C37). (b) would therefore cost 82% of X yield and all of F for no precision gain. (a)'s refusal set is closed and enumerable (§3.2 K1–K8). |

**Owner answers (2026-09-25):** all recommendations accepted. D1 = A (React `forwardRef`/`memo` only), D2 = Exact,
D3 = S1 alone, D4 = a (S1b follows S1), D5 = ship S1 now, and `paths` resolution is the next planning lane. The owner
also approved publishing corpus F aggregates, with the name, path and commit withheld.

**r2 addendum.** D2 = Exact is kept. The controller directed that it be kept, with conditions (i) and (ii), rather than
downgraded. D6 is new and needs owner confirmation. It is recommended as (a).

## 1. Problem

`export const Island = forwardRef<…>((props, ref) => …)` records no export fact, because the declarator arm admits
only arrow and function-expression initializers (M1, the F4 fix of M2). An imported `<Island/>` therefore reaches R4c
with no resolved export, and it drops as `UnknownName` (M3). On Excalidraw this accounts for 107 dropped call sites
across 41 declarators, 72 of them in the PR #324 E2 ledger (P4, P6). The inner render function already has a stable
callable identity: Pattern 3 names it after the declarator (M4), and same-file uses already bind to it Exact (M7).

## 2. Scope

**In scope.**

- **Export facts.** For each top-level `export const X = W(fn…)` declarator in a `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`
  or `.tsx` file, where `W` is an admitted React wrapper (§3.1), record an export fact for `X` whose target is **the
  exact function node** `fn`. Re-export chains and `export *` barrels carry that target (M3; P6a C14).
- **JSX-only binding.** Such a target is bound only from **JSX element** sites (§3.4).

**Non-goals.** Each stays exactly as on base, and each is pinned by a negative test in §7.

- Default-object member aliases (S2). The anonymous nested or cast wrappers `memo(forwardRef(fn))`,
  `Object.assign(forwardRef(fn), …)` and `forwardRef(fn) as T` (S3), whose inner function has no identity (M4, M5).
- Wrappers with an identifier argument (`memo(Comp)`, `forwardRef(Inner)`). The target is another binding: this is an
  alias through a wrapper, with a different proof burden.
- Non-React wrappers and shape-only admission (D1). `let`/`var` declarators.
- The list and default-identifier `Local` routes (D4 → S1b). Non-relative module specifiers (D5).
- `ImportForward` of a wrapped export (`import {X} from './a'; export {X}`). It stays refused, because
  `forwardable_function_locals` holds function declarations only (M11).
- **Same-line `memo` comparator collisions.** In `memo((p) => …, (a, b) => …)` written on one line, both arrows get
  the same name and span (M4). This is refused and counted (`comparator_line_collision`), not resolved. It is a
  formatting-dependent false negative, accepted because carrying byte identity through `FunctionId` is out of scope.
  MEASURED P18: 0 such declarators on X and F; the 4 X declarators that take a comparator are all multi-line.
- **Non-JSX uses of an admitted export.** `X(props)`, `X.call(…)` and callbacks do not bind (§3.4).
  `new X()` is not a call site in JS/TS at all (M13).
- Any change to `Language::function_name`, `FunctionId`, call-site ownership, DFG, or Step 5b.

## 3. Semantics

### 3.1 Admission predicate and reason precedence

The predicate is evaluated inside the existing `lexical_declaration | variable_declaration` arm
(`src/ast.rs:2861-2893`), for an exported declarator `d` whose `name` is an identifier `X`.

- **Arrow or function-expression initializers** keep recording `Local(X)` exactly as on base, with no reason counted.
- **Every other initializer** is evaluated by the checks below, **in this order**. The first failing check names the
  reason. Every refusal increments `skipped_expr_count` (the base behavior) and `skipped_decl_reasons[reason]`
  (§5).

| # | Reason | Refuse when |
|---|---|---|
| R1 | `non_call_initializer` | the value is missing, or its kind is not `call_expression` (this includes a cast `forwardRef(fn) as T`) |
| R2 | `not_const` | the declaration is not a `lexical_declaration` with `kind == const` |
| R3 | `parse_recovery` | the enclosing `export_statement` has an ERROR or MISSING node (`has_error()`) |
| R4 | `import_parse_recovery` | **any** top-level `import_statement` in the file has an ERROR or MISSING node (sol W2). Per-statement, not whole-file: parse errors elsewhere do not refuse |
| R5 | `callee_not_admitted` | the callee is not (a) an identifier bound by a named value import of `"react"` whose **imported** member is `forwardRef` or `memo`, nor (b) a `member_expression` `O.P` where `O` is an identifier bound by a default or namespace value import of `"react"` and `P` is a `property_identifier` spelled `forwardRef` or `memo`. The module string matches `"react"` exactly. Bindings come from `extract_import_bindings()` (M10): a named import is `MemberImport` with `member == Some(name)`, a default import is `MemberImport` with `member == Some("default")`, and a namespace import is `ModuleImport` (READ `src/ast.rs:2244-2305`). Type-only imports are excluded at extraction |
| R6 | `callee_provenance` | any subclause **P1–P5** below fails for the callee local `L` (named form) or `O` (member form) |
| R7 | `react_object_unaccounted` | the file fails the occurrence custody of §3.2 (subclauses **K1–K8**). This applies to **both** callee forms |
| R8 | `arity` | the call's `arguments` field is not an `arguments` node (for example a template string), or, after ignoring `comment` children, there are 0 arguments, more than 1 for `forwardRef`, or more than 2 for `memo` |
| R9 | `nested_wrapper` | the first argument is a `call_expression` |
| R10 | `first_arg_not_function` | the first argument is not an `arrow_function` or `function_expression`. This covers an identifier, a spread, or a parenthesized or `as` expression |
| R11 | `comparator_line_collision` | `memo` has a second argument that is an arrow or function expression whose `function_name` text equals the inner function's, and whose `node_line_range` equals the inner span (sol W3) |
| R12 | `inner_unnamed` | `function_name(fn)` is `None`. **Unreachable** given R10 and Pattern 3 (M4); kept as a defensive `ok_or`. It needs no fixture, and T-O1 lists it as explicitly uncovered |

**Provenance subclauses (R6).** Each has its own test (§7, T-R6-P1…P5).

- **P1.** Exactly one import binding in the file has `local == L`, across all modules, via `extract_import_bindings()`.
- **P2.** `L` is not a type-only import (`js_ts_type_only_imports()`).
- **P3.** `L` is not written anywhere in the file (`js_ts_module_value_written(L)`, READ `src/ast.rs:4613`).
- **P4.** No module-scope declaration of `L` exists: no top-level (or `export`-wrapped) function, class or declarator
  binding name. The wrapper call is itself at module scope, so a nested declaration cannot shadow it. Do not reuse the
  nested-scope `js_ts_forwarding_competitor_except`, which would refuse a component-local `const memo = …` for no
  precision gain.
- **P5.** The spelling of `L` contains no `\` (escaped identifier). This mirrors `js_cjs_export_barriers.rs:125-135`.

### 3.2 React-object occurrence custody (R7; D6 = a)

**Analysis model (stated, not proven).** A third-party module object is assumed not to be mutated from **another**
module, and not to be reachable in this file through a host or global alias (for example a bundler external mapping
`"react"` to `window.React`). This is the same assumption prism makes today for module objects:

- the CommonJS producer barrier states "Cross-module mutation and require-time consumer snapshots remain open"
  (READ `docs/superpowers/plans/2026-09-11-cjs-export-barriers.md:24`);
- it is implemented as refusal-only occurrence custody, "does not prove … require-time snapshots"
  (READ `src/ast/js_cjs_export_barriers.rs:1-2`);
- R4c binds `import_member` Exact on in-repo CJS exports with only this producer-side custody (M3, M14).

**Within the file, the model is closed.** Let `OBJ` be the set of locals bound by default or namespace value imports
of `"react"`. The file **fails custody** if any of the following holds.

- **K1.** A `with_statement` appears anywhere.
- **K2.** An identifier spelled `eval` appears anywhere.
- **K3.** `OBJ` is non-empty and some `identifier`, `shorthand_property_identifier` or
  `shorthand_property_identifier_pattern` contains `\`.
- **K4.** `OBJ` is non-empty and a `call_expression` whose callee text is `require` or `import` has an argument whose
  text contains `react` (another handle to the same object).
- **K5–K8.** An occurrence of a name in `OBJ` is **unaccounted**. The check is spelling-based, so it is conservative
  about shadowing. An occurrence is any `identifier`, `shorthand_property_identifier` or
  `shorthand_property_identifier_pattern` node whose text is in `OBJ`. It is accounted **only** if it is one of:
  - **(O1)** an `identifier` inside an `import_statement`;
  - **(O2)** an `identifier` that is the `object` field of a `member_expression` whose `property` is a
    `property_identifier`, where that member expression is **not in a write position** (below);
  - **(O3)** an `identifier` whose parent is `nested_type_identifier` or `type_query` (type positions such as
    `React.FC` and `typeof React`).

  Everything else is unaccounted:
  - **K5** a bare value use: an argument (`Object.assign(React, …)`, `Object.defineProperty(React, …)`), an
    initializer (`const R = React`), a return value, a spread, or `export { React }`;
  - **K6** a shorthand object property (`{ React }`);
  - **K7** a subscript object (`React["forwardRef"]`), or a parenthesized or cast object (`(React as any).x`);
  - **K8** an O2-shaped member that **is** in a write position.

**Write position.** For a member expression `m`, climb from `m` through parents while the parent is a
`member_expression` or `subscript_expression` whose `object` is the current node, or a `parenthesized_expression` or
`non_null_expression`. Call the result `root`. Then `m` is in a write position iff one of these holds:

- **(w1)** `root`'s parent is an `update_expression` (`++`/`--`);
- **(w2)** `root`'s parent is a `unary_expression` whose operator is `delete`;
- **(w3)** some ancestor of `root`, or `root` itself, is the `left` field of an `assignment_expression`,
  `augmented_assignment_expression` (every compound and logical assignment: `+=`, `||=`, `&&=`, `??=`) or
  `for_in_statement` (`for … in` and `for … of`). This also covers destructuring assignment targets.

These are all the property-write syntaxes in the JS/TS grammar. Every reflective or heap write
(`Object.assign`/`defineProperty`/`setPrototypeOf`, `Reflect.*`, `Proxy`) needs the object as a bare value, which K5
already refuses. Writes through a *different* member of `React` (`React.customThing = 1`) are refused too: this is
conservative, and control C46 pins it as a recall cost.

Named bindings (`forwardRef`) cannot be rebound: ESM import bindings are immutable, and P3 refuses a visible write.
Custody applies to named-form wrappers as well, because under CJS interop a named import reads the same module object
(C37).

### 3.3 Data model (`src/js_exports.rs`)

- **New variant.** `JsExportTarget::SpannedLocal { local: String, start_line: usize, end_line: usize }`. It is produced
  only by §3.1, and S1b may reuse it.
- **Resolved span.** `ResolvedJsExport` gains `#[serde(default)] pub span: Option<(usize, usize)>`. It is `None` for
  `Local`/`Class`-derived results.
- **Resolution.** `resolve_one_inner` handles `SpannedLocal` as a direct hit (`is_class = false`) and copies the span.
  `ReExport` chains return the terminal hit unchanged. **The star-barrel candidate key includes the span**, so two
  claims with the same `(file, local)` and different spans conflict (sol W5; T-N16; mutant M8).
- **Unchanged behavior** (each still gets a test):
  - an `ImportForward` whose terminal is `SpannedLocal` stays `BlockedClaim` (M11);
  - `insert_named` poisoning is unchanged;
  - the `imported.contains(local)` refusal (`src/ast.rs:2470-2476`) applies to `Local` only.
- **New fields** on `JsExportFacts`, both `#[serde(default)]`: `spanned_admitted: usize` and
  `skipped_decl_reasons: BTreeMap<String, usize>`. `is_empty()` includes both.

### 3.4 Resolution: span filter and JSX-only binding (`src/resolution.rs`, `src/call_graph.rs`)

**No JSX-vs-call distinction exists today** (M13):

- `CallKind` has only `Call` and `MacroInvocation` (`src/call_graph.rs:390-394`);
- `call_kind_at` returns `Call` for every non-Rust site (`:5504-5507`);
- `call-stats --dump-sites` reports JSX rows as `"call_kind":"Call"` (P1).

So S1 adds one.

- **`CallSite.jsx_element: bool`** (`#[serde(default)]`, **excluded from `cmp_key`**, placed after
  `pre_resolved_target` at `src/call_graph.rs:495`). It is set by a new
  `CallGraph::jsx_element_at(parsed, start_byte, end_byte)`, which is true iff
  `descendant_for_byte_range(start, end)` has exactly those bytes and kind `jsx_self_closing_element` or
  `jsx_opening_element`. It is set at the three source constructors (`:1368` skeleton, `:1800` full, `:5101` subset).
  `indirect_call_site` (`:2849`) and every synthetic or derived constructor set `false`. Test and fixture literals
  gain `jsx_element: false`.
- **Rejected alternative:** a `CallKind::JsxElement` variant. It would change `call_kind` on every JSX dump row. It
  would flip the `kind != Call` filters (`src/call_graph.rs:2705`, `src/go_callback.rs:537,655`) and the
  namespace match (`src/name_resolution/consumer.rs:67-70`), so its blast radius is larger.
- **`js_ts_import_member_candidates(caller, binding, member, site) -> Result<Vec<&FunctionId>, DropReason>`.** When
  `resolved.span == Some((s, e))`:
  1. if `!site.jsx_element`, return `Err(DropReason::WrappedExportNonJsx)`, a new variant counted in call-stats
     (§5). The site drops with that reason. There is no fallback to other rungs: the JS arm already returns before R5
     (M3);
  2. otherwise add the filter `fid.start_line == s && fid.end_line == e` to the existing file and non-method filter;
  3. with exactly 1 match, the result is `Exact` `ImportMember`. With 0 matches, return `Ok(vec![])`, so the site
     drops `UnknownName`. With **2 or more matches, return `Ok(vec![])`**: never demote a span-verified collision to
     NameOnly, mirroring the v94 line-identity refusal.

  When `span` is `None`, behavior is byte-identical to base.
- **Sole caller.** `resolve_call_site_full` is the only production caller of `js_ts_import_member_candidates`
  (sol-verified). The nav correlation (`src/navigation/call_resolve.rs:30-75`) compares `local_name` and then
  re-resolves through `resolve_call_site_full`, so it inherits both gates.
- **What the gate covers.**
  - `new X()`: `new_expression` is not captured by the JS, TS or TSX call queries (`src/queries.rs:150-163`), so
    there is no site to bind (MEASURED C32).
  - `X.call(…)`, `X.render(…)`: these carry a qualifier, so the unqualified R4c arm is not entered.
  - `(X as any)(…)`: the callee text is not `X`.
  - Level-3, indirect and synthetic sites have `jsx_element == false` and are refused.

### 3.5 Consumers

CPG Step 5 and Step 5b inherit the edges through `resolve_call_site`. JSX sites carry no arguments, so no new
DataFlow appears. MEASURED P11 and P19: the X `dfg-stats --edges` dump is byte-identical to base.

## 4. Fail-closed rules for each negative control

| Control | Rule that refuses it | MEASURED (P3 base / P18 r2) |
|---|---|---|
| F4 ternary + nested same name (C07) | R1 | drop / drop |
| `forwardRef(Ident)` ± nested same-name function (C09, C10) | R10; no fact, so the nested function is never consulted | drop / drop |
| wrapped + nested same name elsewhere (C11) | §3.4 span filter (never the nested `@3-3`) | drop / Exact `@6-8` |
| same-line nested same name (T-N11) | §3.4, 2 or more matches | (new) |
| local impostor `forwardRef` (C12) | R5 | drop / drop |
| `./react-shim` or `preact/compat` (T-N4) | R5 exact module | (new) |
| written, duplicate or type-only callee binding; module-scope competitor; escaped spelling | R6 P1–P5 | (new; T-R6-*) |
| `React.forwardRef = fake` (sol W1, C28) | R7 K8 (w3) | drop / **drop** |
| `Object.assign(React, …)`, `const R = React`, `(React as any).x = …`, `require("react").x = …` (C33–C36) | R7 K5, K5, K7, K4 | drop / drop |
| default-object write with a **named** callee (C37) | R7 K7 | drop / drop |
| `delete`, `||=`/`??=`, destructuring, subscript writes (C40–C43); `React.version++` (C47); `for (React.version of …)` (C48) | R7 K7/K8 (w1, w2, w3) | drop / drop |
| `{ React }`, `export { React }`, escaped `Re\u0061ct` (C49–C51) | R7 K6 / K5 / K3 | drop / drop |
| `eval(…)` in the file (C45); `with` in a `.jsx` file (C52) | R7 K2 / K1 | drop / drop |
| write to an unrelated member, `React.customThing = 1` (C46) | R7 K7 (conservative; recall cost) | drop / drop |
| import with parse recovery (sol W2, C29) | R4 | drop / **drop** |
| same-line `memo` comparator (sol W3, C30) | R11 | drop / **drop** |
| span-sensitive star-barrel conflict (sol W5, C39) | §3.3 span in the candidate key | drop / **drop** (`barrel_conflicts` +1) |
| direct call `X(props)` (C31) | §3.4 `WrappedExportNonJsx` | drop / drop (`WrappedExportNonJsx`) |
| `new X()` (C32) | no call site (M13) | no row / no row |
| `create(fn)`, `createSelector(a, fn)`, `styled(…)(fn)`, `arr.map(fn)`, `observer(fn)`, `debounce(fn)` | R5 | drop / drop |
| `memo(forwardRef(fn))`, `Object.assign(forwardRef(fn), …)` (C08) / `forwardRef(fn) as any` (C25) | R5 or R9 / R1 | drop / drop |
| `forwardRef(a, b)`, `memo(fn, cmp, x)`, `forwardRef(...args)` | R8 / R10 | (new) |
| `export let` / `export var` (C20) | R2 | drop / drop |
| importer parameter shadow (C18) | existing `js_ts_function_locals` guard | not `import_member` / same |
| duplicate claim `export { other as X }` (T-N13) | `insert_named` poison | (new) |
| export statement with parse recovery (T-N12) | R3 | (new) |

## 5. Counters and observability

- **Per file.** `spanned_admitted` counts admitted declarators. `skipped_decl_reasons` holds R1–R12 counts for
  declarator-arm skips only. **Invariant, per file:** `skipped_expr_count` equals `sum(skipped_decl_reasons)` plus
  the default-export and CJS skips. T-O3 pins it on one mixed file.
- **call-stats** (`src/navigation/queries.rs`):
  - `js_export_spanned_admitted` (the sum);
  - `js_export_skipped_decl_reasons` (a BTreeMap merged across files, emitted as a JSON object with sorted keys);
  - `dropped_wrapped_export_non_jsx` (from the new `DropReason`, added to the exhaustive match at `:385-397`).
  - `js_export_skipped_exprs` is unchanged.
- **Expected values (MEASURED P18, prototype r2):**
  - **X:** `js_export_skipped_exprs` 637 → 596; `spanned_admitted` 41; reasons `{callee_not_admitted: 331,
    first_arg_not_function: 1, non_call_initializer: 247}`; `dropped_wrapped_export_non_jsx` 0;
    `unresolved_unknown_name` −107; `kind_exact.import_member` +107.
  - **F:** 389 → 358; 31; `{callee_not_admitted: 168, non_call_initializer: 167}`; 0; −4; +4.
  - **R:** 28 → 28; 0.

  The prototype evaluates in the R1–R12 order: non-call initializers take R1 before any other check, and R12 is
  unreachable. So these distributions are the expected values, and acceptance re-measures them.

## 6. Cache

Raw export facts, `ResolvedJsExport`, and `CallSite` (which gains `jsx_element`) are all persisted. Bump both
versions, update their pins, and add one version note each.

- `src/cpg_cache.rs`: `CACHE_VERSION` 97 → **98**. Note: "v98: JS/TS span-verified wrapped React exports
  (`SpannedLocal`), JSX-element call-site flag, declarator skip reasons."
- `src/navigation/call_edge_cache.rs`: `NAV_CALL_EDGE_CACHE_VERSION` 53 → **54**.

If a PR merged before this one has bumped either constant, increment from the landed value.

## 7. Test plan

**RED rule.** Every positive (T-P*), observability (T-O*) and JSX-gate (T-J*) test must fail on base `12ca6e8e` with
a behavioral assertion. Capture that by reverting only the §3.1–§3.4 production lines, with the types still present.
Record the concrete failing value. Negative tests (T-N*, T-R*) are guards: they pass on base and must still pass.
Every new code path has at least one positive and one negative test.

**Location and style.**

- New `tests/integration/js_wrapped_export_test.rs`, `js_wrapped_export_refusal_test.rs` (each under 600 lines) and
  `js_wrapped_export_state_test.rs`, registered in `tests/integration/main.rs`.
- The CLI counters go in `tests/cli/call_stats_test.rs`, and the nav test in `tests/navigation/callers_test.rs`.
- Use table-driven inputs where a clause has several forms.
- Assert the exact target `(file, name, start_line, end_line)`, with a same-name decoy where cheap.

**Positive tests.** T-P1 through T-P5 run in JS (`.jsx`) and TSX.

| ID | Input | Expected |
|---|---|---|
| T-P1 | named `forwardRef((p, r) => …)`, plus `forwardRef<A, B>(…)` generics; `<Island/>` imported | Exact `ImportMember` → the arrow's span |
| T-P2 | `import React from "react"` and `import * as React from "react"` with `React.forwardRef` (C26, C27) | Exact |
| T-P3 | `memo(fn)` and `React.memo(fn)` | Exact |
| T-P4 | multi-line `memo(fn, cmp)`, where the comparator arrow starts on a **different line** (so its span differs; Pattern 3 still names it `X`) | Exact → the **first**-argument span only; the comparator span never appears as a target |
| T-P5 | `import { forwardRef as fr } from "react"` | Exact |
| T-P6 | `forwardRef(function IslandImpl(p, r) {…})` | Exact → `IslandImpl@span` |
| T-P7 | wrapped export + nested `function Island` elsewhere (C11) | Exact → the wrapper span only, exactly 1 target |
| T-P8 | depth-2 `export { Island } from "./Island"` and `export * from "./Island"` (C14) | Exact through the chain |
| T-P9 | `export const A = forwardRef(…), B = memo(…)` (C17) | both Exact |
| T-P10 | comment argument `forwardRef(/* c */ (p, r) => …)` | Exact |
| T-P11 | benign React uses in the same file (C38): `React.useState`, `<React.Fragment>`, `type P = React.ComponentProps<…>`, `typeof React`, `Island.displayName = …` | Exact (O2, O3; and the M10 member-write reading on `Island`) |

**JSX gate:**

| ID | Input | Expected |
|---|---|---|
| T-J1 | `Island(props)` from an importer (C31 `App2`) | drop, `DropReason::WrappedExportNonJsx`; call-stats `dropped_wrapped_export_non_jsx == 1` |
| T-J2 | `new (Island as any)()` next to `<Island/>` (C32) | no call-site row for `new`; the JSX row is Exact |
| T-J3 | a site with `jsx_element == false` constructed through the resolution unit-test helper (`src/resolution.rs` test `site()`) against a SpannedLocal fact | `WrappedExportNonJsx`. Covers synthetic and indirect origins |
| T-J4 | a plain `Local` export (`export const A = () => …`) called directly `A()` | unchanged Exact; the gate applies only to spans |

**Negative tests and guards.** Each of R4, R6 P1–P5, R7 K1–K8 and R8–R11 has at least one row.

| ID | Input | Expected |
|---|---|---|
| T-N1 | `forwardRef(IslandInner)` + nested `function Island` (C10) | drop (R10) |
| T-N2 | `memo(forwardRef(fn))`, `Object.assign(forwardRef(fn), {})`, `forwardRef(fn) as any` | drop (R9, R5, R1) |
| T-N3 | local impostor `function forwardRef` (C12) | drop (R5) |
| T-N4 | `forwardRef` from `"./react-shim"` and from `"preact/compat"` | drop (R5) |
| T-N5 | `export let` / `export var` (C20) | drop (R2) |
| T-N6 | `create(fn)`, `createSelector(a, fn)`, `styled("div")(fn)`, `items.map(fn)`, `observer(fn)`, `debounce(fn, 1)` | drop (R5) |
| T-N7 | importer parameter shadow (C18) | not `ImportMember` |
| T-N8 | `forwardRef(a, b)`, `memo(fn, cmp, x)`, `forwardRef(...args)`, zero args | drop (R8, R8, R10, R8) |
| T-N9 | `import {X} from "./a"; export {X}` over a wrapped `X` | drop (ImportForward refusal unchanged) |
| T-N11 | single line: `export const X = forwardRef((p, r) => { function X() {} return null; });` | drop (§3.4, 2 or more matches) |
| T-N12 | a parse error inside the export statement | drop (R3) |
| T-N13 | `export const X = memo(fn); export { other as X };` | drop (poisoned) |
| T-N14 | the pinned F4 test and the existing P4/P7 JS export suites | unchanged and passing |
| T-N15 | one-line `memo((p) => …, (a, b) => true)` (C30) | drop (R11); `skipped_decl_reasons.comparator_line_collision == 1` |
| T-N16 | star-barrel span conflict (C39): `impl` has `A = memo(function Same…)` and `B = memo(function Same…)`; `a`/`b` re-export them as `X`; `index` has `export *` of both | drop; `js_export_barrel_conflicts` +1 |
| T-N17 | `import { forwardRef as fr ??? } from "react"` (C29), plus a clean-import twin where a parse error is **outside** any import (which must still resolve) | drop (R4) / Exact |
| T-R6-P1 | `import { memo } from "react"; import { memo } from "./other"` (duplicate local) | drop |
| T-R6-P2 | `import type { memo } from "react"` + value use | drop (R5; type-only never enters bindings) |
| T-R6-P3 | `memo = other;` after the import | drop |
| T-R6-P4 | top-level `function memo(){}`, `class memo {}`, `const { memo } = x` (each alone) next to `import { memo }`; **positive twin:** a component-local `const memo = 1` | drop ×3 / Exact |
| T-R6-P5 | an escaped callee spelling `forw\u0061rdRef` | drop |
| T-R7-K1…K8 | one row each (table-driven): `with` (K1, JS only, C52); `eval('1')` (K2, C45); an escaped `Re\u0061ct` occurrence (K3, C51); `require("react").forwardRef = f` (K4, C36); `Object.assign(React, …)` and `const R = React` (K5, C33/C34); `export { React }` and `({ React })` (K5, K6; C50, C49); `React["forwardRef"] = f` and `(React as any).x = 1` (K7, C43/C35); `React.forwardRef = fake` (w3, C28), `React.forwardRef ||= f` (w3, C41), `delete React.memo` (w2, C40), `React.version++` (w1, C47), `[React.forwardRef] = [f]` (w3, C42), `for (React.version of xs)` (w3, C48) (K8); a named-form callee with a default-object write (C37) | each drop (R7) |

**Observability, state, cache and nav:**

| ID | Test | Expected |
|---|---|---|
| T-O1 | one extraction fixture per reachable reason R1–R11 | `skipped_decl_reasons[reason] == 1` and `skipped_expr_count` +1; the admitted fixture gives `spanned_admitted == 1`. R12 is listed as unreachable |
| T-O2 | CLI `prism nav --no-cache call-stats --repo <tmp>` | `js_export_spanned_admitted`, `js_export_skipped_decl_reasons` and `dropped_wrapped_export_non_jsx` present with exact values |
| T-O3 | **mixed-declarator sum invariant:** one file with an admitted `forwardRef`, a `create(fn)` (R5), a ternary (R1), an `export let` wrapper (R2), a one-line comparator (R11), and an `export default someCall()` | `skipped_expr_count == sum(skipped_decl_reasons) + 1` (the default-export skip); each reason count is exact |
| T-S1 | state epochs (full build **and** `build_incremental`, JS/TS/TSX): admitted → wrapper import changed to `"./shim"` (refused) → restored → `React.forwardRef = fake` added (refused, member form) → restored → inner arrow moved two lines (span changes) → restored | the edge set equals the expectation in every epoch, and full equals incremental |
| T-C1 | `serde_json` round-trip of `JsExportFacts` (with `SpannedLocal` and reasons) and of a `CallSite` with `jsx_element = true`; a CPG cache full hit that preserves the Exact edge | equal |
| T-C2 | pin tests | 98 and 54 |
| T-V1 | nav `callers --symbol Island --file lib.tsx` on a temp fixture | 1 caller, `import_member`, score 1.0 |

**Mutants.** The implementer applies each alone and records the test that kills it.

| # | Mutant | Must fail |
|---|---|---|
| M1 | drop the span filter | T-P7, T-N11 |
| M2 | accept any callee | T-N6, T-N3 |
| M3 | accept `let` | T-N5 |
| M4 | pick the last argument | T-P4 |
| M5 | demote a collision to NameOnly | T-N11 |
| M6 | stop counting a reason | T-O1 |
| M7 | skip the cache bump | T-C2 |
| M8 | drop the span from the star-barrel candidate key | T-N16. MEASURED P21: the mutant yields NameOnly to both `Same` spans instead of dropping |
| M9 | skip the JSX gate | T-J1, T-J3 |
| M10 | treat member writes as reads (remove w1–w3) | T-R7-K8, C28 |
| M11 | skip the import error guard | T-N17 |

**Tier-A fixtures** (`eval/fixtures/typescript/`, same schema as `arrow_const_export_deferred`; at most 3 new
directories):

- `forwardref_named_export/` (C02): `exact = true`, `resolution_kind = "import_member"`. RED on base.
- `forwardref_ident_arg_nested_samename_refused/` (C10): `callers = []`, `forbid_resolution_kind = "import_member"`.
- `react_default_member_write_refused/` (C28): `callers = []`, `forbid_resolution_kind = "import_member"`.

## 8. Acceptance

1. **Suites green.**
   - `cargo fmt --check`;
   - `cargo test --offline --no-fail-fast`: base 4,559 / 0 / 1 (P9); the r2 prototype, with only literal fix-ups,
     also gives 4,559 / 0 / 1 (P16); report the new total;
   - `cargo test --offline --features mcp`;
   - `cargo clippy --offline --all-targets --features mcp`;
   - Tier-A `--matrix-only`: base 159/159, r2 prototype 159/159 (P10), plus the new fixtures, with 0 regressions;
   - the Node gate;
   - `tier-a --quick` if rust-analyzer is available; otherwise report it as not run.
2. **Yield re-measurement (the acceptance criterion; unchanged by r2, MEASURED P18).** Run the implementation's
   release binary on each corpus with `nav --no-cache call-stats --dump-sites`, then `probes/rowdiff.py` against the
   base dumps.
   - **X:** exactly the 107 rows of `probes/P6-excalidraw-expected-rowdiff.json` (SHA-256 `265790a3…`), every one
     `UnknownName → Exact import_member`, with 0 other changed rows. The r2 prototype row-diff is **byte-identical**
     to the v2 one. `probes/audit.py` 107/107. `dfg-stats --edges` byte-identical to base.
   - **F:** 4 rows, all `UnknownName → Exact`, audited 4/4, and 0 other changes.
   - **R and T:** dumps byte-identical to base (MEASURED again for r2: P18, P20).
   - Counters as in §5, including `dropped_wrapped_export_non_jsx == 0` on X and F.

   Any deviation is reported row by row with its mechanism, and it is a blocker until the owner accepts it.
3. **Custody.** The acceptance outputs go under `~/prism-evidence/wrapped-export/acceptance/` with a
   `MANIFEST.sha256`.

## 9. Budget (honest lines: after `rustfmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` count as tests)

**Analogues (P8)**, all on this `JsExportFacts`/R4c seam:

| Commit | Slice | src | tests |
|---|---|---|---|
| `c3d110ef` | ESM forwarding | 160 | 491 |
| `032e1824` | CJS producer export-object fences | 157 | 303 |
| `88af6511` | CJS terminal proof | 231 | 457 |
| `4ffe55bf` | module-binding audit | 94 | 670 |

**MEASURED P17: the r2 prototype is 431 src / 17 test honest lines.** It implements all of §3 and §5, but no tests,
no cache bumps, and a mode-(b) switch. By function in `js_wrapped_export.rs`:

| Function | Lines |
|---|---|
| main predicate | 103 |
| React import parsing | 64 |
| write position | 41 |
| module-scope competitor | 35 |
| custody walk | 34 |
| occurrence accounting | 26 |

Plus `ast.rs` 28, `js_exports.rs` 47, `resolution.rs` 25, `call_graph.rs` 17, `queries.rs` 15, and 2 literal lines.
Demonstrable reduction: React import parsing can reuse `extract_import_bindings()` (M10: namespace is `ModuleImport`
with `member: None`; default is `member == Some("default")`), which saves about 50 lines. Removing the mode switch
saves 3. That gives about **380**.

**The r1 cap (200 src) no longer fits.** r2 adds the JSX gate (+17), custody (+101) and the reason table (+30), all
required by sol's findings. Re-capped with the evidence above:

| Bucket | Cap | Early stop |
|---|---|---|
| src (non-test Rust) | **420** | 380 |
| tests (Rust) | **650** | 585 |
| combined | **1,070** | 960 |
| Tier-A fixture directories | 3 | – |

**Test estimate:** about 55 table rows at 8–10 lines each, plus about 60 lines of helpers, plus the state test
(about 90), which lands near 600.

**Size and convergence.** At 1,070 combined, the slice is larger than every analogue except P4 (705 src / 1,279
tests) and `c3d110ef` (651). It is still a single-mechanism slice: one predicate module, one data-model variant, one
resolution gate and one call-site flag. It stays within the range that converged in 2 rounds on this seam.

**Split fallback,** if the controller prefers two smaller reviews: **S1-i** (the JSX flag, `CallSite` field,
`DropReason`, cache: about 60 src / 120 tests, with no behavior change on base because no span targets exist yet)
lands first. Then **S1-ii** (predicate, custody, data model, span filter: about 320 / 530). The planner does not
recommend the split, because S1-i alone cannot be yield-tested.

New production logic lives in `src/ast/js_wrapped_export.rs`, following `src/ast/js_module_forwarding.rs`. It must
stay under 600 lines. `ast.rs` gains only the call-site arm.

## 10. Risks and open questions for review

- **The analysis model** (§3.2) excludes cross-module and host-global mutation of the React object. It is the same
  model as the CJS producer barrier. A reviewer who rejects it should argue D6 or D2, not the mechanism.
- **The React rendering contract** (rendering `<X/>` invokes the wrapped render function) is an ASSUMPTION, with the
  M7 precedent. The JSX gate confines Exact to exactly the sites where that contract applies.
- **Pattern 3 over-naming** (M4) is pre-existing. The span filter and R11 make S1 immune to it.
- **Longer term** (sol): a distinct framework "renders" edge kind would separate JSX-to-render from ordinary call
  edges. That is out of scope here, and the `jsx_element` flag is the seam for it.
