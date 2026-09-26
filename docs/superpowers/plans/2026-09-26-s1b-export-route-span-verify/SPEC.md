# S1b: span-verify every JS/TS export and local route

**Status:** planning complete, revision r1, awaiting owner decisions (§0) and the spec review (sol + Opus in
parallel, 2-round cap). Nothing under `src/`, `tests/`, `eval/`, `Cargo.*` or `CLAUDE.md` changes on this branch.

**Base:** `origin/main` `a6d853f5` (S1 merged, #325/#326). **Grounding:** `PLANNING-PROBES.md` (M-ids for
mechanisms, Q-ids for measurements). **Predecessor:** S1 packet
`docs/superpowers/plans/2026-09-25-wrapped-export-resolution/` §12, which recorded this scope (owner D4 + D10).

**Model (binding, `CLAUDE.md`):** Exact is a static-binding grade. Runtime mutation of module objects, `eval`, host
globals and `require`/`import()` re-acquisition are out of model for every rung. A `with` statement is static
syntax and is in model.

## 0. Owner decisions

Each row gives the options, the measured cost, and the planner's recommendation. None is decided here.

| # | Decision | Options | Measured basis | Recommendation |
|---|---|---|---|---|
| E1 | **Slicing** | (a) three sub-slices in order: **S1b-1** binding core + D4 export routes + JSX intrinsic guard; **S1b-2** R4 lexical `LocalDef`; **S1b-3** R3 namespace. (b) two: S1b-1 with R3 folded in, then S1b-2. (c) one slice | Prototype about 666 src honest lines of design code against S1's 342 (Q16). Per sub-slice forecasts in §9: about 275 / 310 / 95 src | **(a)**. Each sub-slice stays at or below S1's size, so each can converge in 2 rounds. S1b-2 carries almost all the measured wrong edges; S1b-1 validates the core at module scope first; S1b-3 is small and latent. (b) is acceptable if the owner prefers fewer PRs (about 370 src, above S1's landed size) |
| E2 | **Lowercase intrinsic tags** (Z02b, S1 impl-r1 W1 alt 2) | (a) **include**, as one guard at the top of resolution that covers every rung; (b) include on the plain `Local` route only, as the handoff framed it; (c) defer | The class is not route-specific. On base, intrinsic tags bind on `free_single`, `free_multi` and `local_def`, not only `import_member`: X 62 rows (166 edges: `<input>` → a test helper, `<label>` → action `label:` arrows), F 103, R 0, T 0 (Q3, Q12). 0 right edges among them | **(a)** in S1b-1. It is a closed predicate (a tag that is a plain identifier starting with an ASCII lowercase letter, or containing `-` or `:`), about 12 src lines, and (b) would leave 62 X rows wrong. S1's R4c-only lowercase check becomes redundant and is removed |
| E3 | **Drop reason for intrinsic tags** | (a) new `DropReason::JsxIntrinsic`; (b) reuse `UnknownName` | (a) relabels `UnknownName` → `JsxIntrinsic` on rows that had no edge: X 804, F 387, R 117, T 0 (Q14). `unresolved_unknown_name` then stops counting `<div>`s | **(a)**. Honest telemetry; the unresolved-name count is the recall lanes' denominator. (b) keeps the row-diff small but hides the class |
| E4 | **`useCallback` pass-through** (S1b-2) | (a) **admit** React `useCallback` under S1's ESM-`"react"` provenance (R5/R6) as a plain callable, bindable from any site; (b) refuse it like any other call-wrapped declarator | React documents that `useCallback(fn, deps)` returns `fn` itself (cached across renders), so the call site invokes that function's code (ASSUMPTION, React API). Refusing drops X 11 and F 98 base edges that are right under that contract; admitting keeps X 11 / 11 and F 94 / 98 (4 fail the parse-recovery rule) (Q14) | **(a)**. About 12 src lines on top of S1's table; same provenance checks and mutants as S1 |
| E5 | **May-call wrappers** (S1b-2): `const t = throttle(fn)`, `memoize(fn)`, `useStableCallback(fn)`, rendering HOCs, test mocks, and bindings written on some paths | (a) **drop** (the binding holds a different callable); (b) demote the base target to NameOnly; (c) keep base Exact | Under static binding these Exact edges are wrong. Under a may-call reading they are edges to code that may run later. Removed edges: X 68, F 4, T 172 (T: `memoize`/`memoizeOne` 157) (Q12). The same syntactic class also holds plainly wrong targets that (b) would keep as NameOnly: subscription returns (`const unsub = api.onChange(listener); unsub()`), `lazy`/`dynamic` loaders, `styled(…)` style callbacks: X 9, F 54, R 1 | **(a)** in S1b-2, with a follow-up lane recorded (a library-semantics "may-call" NameOnly edge kind). (b) keeps wrong edges; telling them apart needs per-library knowledge, an open-ended list |
| E6 | **Parse recovery** | (a) **refuse** when the binding's scope contains any `ERROR` or `MISSING` node; (b) ignore errors confined to a nested function that does not contain the site | (a) refuses right edges in files with tree-sitter grammar gaps: T 53 (2 files: `in out` variance, a `symbol:` parameter), F 7 (+4 `useCallback` under E4a), X 0, R 0; module-scope export refusals F 6 with 0 edge impact (Q7a, Q13). (b) would recover 12 of T's 53 and 3 of F's 7, and is not closed (a `MISSING` brace can move later declarations into the erroneous function) | **(a)**. Closed, and it matches the CJS terminal proof (M10). The remaining loss belongs to a grammar upgrade, not to S1b |
| E7 | **Namespace qualifiers whose module does not resolve** (non-relative, or `./x.js` for `x.ts`, M5) | (a) **keep the stem lookup, filtered to callables their own file exports under that member name** (rule R3-2); (b) refuse; (c) leave base | Base stem lookups are right on T (24 rows) and F (4 rows) (Q3a). (b) drops those 28 right rows. (c) leaves the C62 decoy class open for the `./lib.js` spelling (C81) | **(a)**, in S1b-3 |
| E8 | **Non-namespace R3 qualifiers** (`import { obj } …; obj.f()`, default-import members), including multi-target Exact rows | (a) **leave to S2** (member-of-exported-object semantics); (b) demote R3 rows with 2+ targets to NameOnly now | X 15 rows (9 single-target, plausibly right; 6 multi-target with fan-out 2 to 49), F 59, T 2 (Q3a). (b) changes X 6, F 1, T 1 rows from Exact to NameOnly without removing an edge | **(a)**. The single-target rows are mostly right, and the correct fix (resolve the exported object's member) is S2's. Record the multi-target rows as S2's first cases |
| E9 | **Imported locals in export lists** | (a) **keep the base path** (`Local(name)`, then poisoned into `conflicted`); (b) route them through the terminal (refused as `not_callable`) | (b) moved X's `js_export_barrel_conflicts` from 13 to 0 and failed one existing test (Q14, Q15). No edge differs | **(a)**. Parity; the refusal is already correct on base |
| E10 | **`IndirectResolution` sites** that resolve through `LocalDef` | (a) **unchanged** (non-goal); (b) include | X 9, T 6 rows (Q3). Their callee is a function-pointer target name, not a name written at the site, so the lexical rule does not apply | **(a)** |
| E11 | **Budget** (honest lines, per sub-slice) | §9 | Prototype about 666 src; S1 landed 342 / 660 / 1,002 | S1b-1 **300 / 600 / 900**; S1b-2 **340 / 680 / 1,020**; S1b-3 **120 / 300 / 420** (early stops at about 90%) |
| E12 | **Review** | sol + Opus-5.5 in parallel, 2-round cap per sub-slice; Fable re-plans on a rejection or an open-class cap | S1 process | as stated; `REVIEWER.md` |

## 1. Problem

Prism turns a JS/TS name into callables by **name lookup**, and three routes do so without checking what the name
statically denotes:

1. **D4, the ESM export routes** record `Local(name)` for export lists, `export default <ident>`,
   `export function` and exported function-valued declarators (M1). R4c then binds any same-file function of that
   name (M2). `const f = <non-function>; export { f }` binds a nested `function f` (C06); `export let A = …; A = Other`
   stays Exact (C21); `const useStore = create(fn); export { useStore }` binds the factory's argument (C73).
2. **R3 `ImportQualified`** returns every same-stem function of that name, all Exact (M3):
   `import * as Lib from './lib'; <Lib.Island/>` binds the export and a nested decoy (C62, S1b-RED-1).
3. **R4 `LocalDef`** binds every same-file free function of that name, all Exact (M6): a parameter that shadows it,
   a nested function in another scope, an object-literal `key:` arrow, a call-wrapped declarator's argument. On T,
   635 `local_def` rows carry 2 to 21 Exact targets (Q12). S1's non-JSX `Island({})` binds a `forwardRef` render
   function (C63, S1b-RED-2).
4. **JSX intrinsic tags** (`<input>`, `<label>`, `<line>`) bind on every rung (M8).

Prevalence (Q3, Q12): the export routes are latent (0 wrong `import_member` rows on four corpora); R3's namespace
decoy is latent; R4 and intrinsic tags are not (X 282 rows, F 295, R 1, T 1,033 change).

## 2. Scope

**S1b-1 (binding core, D4, intrinsic).**
- The binding terminal (§3.1), evaluated at module scope.
- Every ESM local export producer records a span-verified target or a counted refusal (§3.2).
- R4c keys the JSX gate on the new `wrapped` flag and filters any span (§3.3).
- The JSX intrinsic guard (§3.5).

**S1b-2 (lexical `LocalDef`).**
- The terminal evaluated at every scope, per call site, memoized per file (§3.6).
- R4 binds only the site's proven callable (§3.6).
- `useCallback` admission under E4 (§3.7).

**S1b-3 (R3 namespace).** Namespace qualifiers name the resolved module's export (§3.4).

**Non-goals.** Each stays exactly as on base and is pinned by a test (§7).
- Named- and default-import qualifiers on R3, including their multi-target rows (E8; S2).
- CommonJS `Local` targets (`js_ts_cjs_terminal_proven` already proves them, M10), `ImportForward`, `ReExport` and
  star-barrel mechanics other than the span/`wrapped` key.
- `IndirectResolution`, `MacroArg` and synthetic sites (E10).
- Module resolution: `./x.js` → `x.ts`, tsconfig `paths`, workspace packages (the next lane).
- Runtime mutation, `eval`, host globals (out of model).
- The may-call class as a new edge kind (E5, a follow-up lane).
- Non-JS languages. `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`, DFG construction.

## 3. Semantics

### 3.1 The binding terminal (S1b-1 builds it; S1b-2 uses it at every scope)

**The question.** Given a name `n` and either a site node (S1b-2) or a scope (S1b-1, the program), which callable
does `n` statically denote? The answer is a **terminal** `(local, start_line, end_line, wrapped)`, where `local` is
the callable's registered name (`Language::function_name`) and the lines are its `FunctionId` span; or a refusal
reason.

**Scope set Σ (closed).** A node opens a scope iff its kind is one of:

| Scope kind | Declares |
|---|---|
| `program` | its statements (below), `var` hoisted from any depth, imports |
| function-like: `function_declaration`, `generator_function_declaration`, `function_expression`, `generator_function`, `arrow_function`, `method_definition` (`is_js_ts_function_like`) | its parameters (`parameters` **and** the single-arrow `parameter` field), the implicit `arguments` binding of every non-arrow function (not a callable), a named function expression's own name, and its body as a program-like list with `var` hoisting |
| `formal_parameters` | its function's parameters and a function expression's own name only. A parameter default cannot see body declarations, so the walk continues **above** that function |
| `statement_block` not directly a function body | its own lexical declarations (includes TS namespace bodies and class static blocks) |
| `for_statement`, `for_in_statement` (covers `for…of`) | their `let`/`const` heads |
| `catch_clause` | its parameter pattern |
| `switch_body` | lexical declarations in its cases |
| `class_declaration`, `class`, `abstract_class_declaration` | the class's own name (inner binding) |
| `with_statement` | **every name** (a `with` object may supply any identifier) |

The nearest scope in Σ (walking parents from the site) that declares `n` holds the binding. No scope declares it:
the name is global, and S1b refuses (`unbound`).

**Declarations of `n` in a scope (closed; walked over the scope's own statement list).**
- Transparent wrappers (the walk stays "direct"): `export_statement`, `expression_statement`, `switch_case`,
  `switch_default`, `lexical_declaration`, `variable_declaration`.
- Direct only: `function_declaration`, `generator_function_declaration`, `class_declaration`,
  `abstract_class_declaration` (own name); `lexical_declaration` declarators (pattern names); `import_statement`
  (default, namespace, named with or without alias, `import x = require()`; value and type-only alike); TS
  `ambient_declaration` (`declare function|const|…`), `enum_declaration`, `internal_module`/`module`,
  `import_alias`, `interface_declaration`, `type_alias_declaration` (fail-closed: type names shadow).
- Hoisted (`var`), at any depth that does not cross a function-like or class node: `variable_declaration`
  declarators and `for (var … in/of …)` heads.
- **Annex B marker.** A `function_declaration` nested in a block below the scope, whose name is `n`, adds a marker
  (the enclosing block). Sloppy-mode code var-hoists such a function (ECMA-262 B.3.3), so outside its block the name
  may denote it or an outer binding. The marker is not a callable, so the terminal refuses. Inside the block, the
  block scope finds the declaration first.
- TS overload signatures (`function_signature` outside an `ambient_declaration`) are **not** declarations. The
  implementation `function_declaration` is.
- The walk stops at nested function-like and class nodes (their own names bind only when direct).

**Why this is closed, and where the argument is weakest.** The rules map each binding source of ECMA-262 (the
var-scoped and lexically scoped declarations, formal parameters and the implicit `arguments`, catch parameters,
`for` heads, function and class expression names, import bindings) and each TS declaration form to a node kind
above. A construct the grammar does not know parses as `ERROR`, which B1 refuses. The weak point is a construct the
grammar **mis-parses without an error**: MEASURED (Q23), tree-sitter-typescript 0.23 reads `using h = e;` as the
assignment `h = e`. That case still refuses, because B5 sees a write to the outer binding (C114), but the argument
rests on each such mis-parse surfacing as a write or an error. Reviewers should probe this list directly; §7 has one
test row per form, in both the JSX and TSX grammars.

**Terminal classification**, in order (the first failing check names the reason):

| # | Reason | Refuse when |
|---|---|---|
| B1 | `parse_recovery` | the binding scope's subtree has an `ERROR` or `MISSING` node (`has_error()`; E6). The site lies inside that scope, so every recovery that could hide a closer declaration lies inside it too |
| B2 | `duplicate_declaration` | the scope holds 2 or more declarations of `n` (an Annex-B marker counts as one) |
| B3 | `not_callable` | the declaration is not one of: a `function_declaration`/`generator_function_declaration`; the scope's own named `function_expression`; a `variable_declarator` whose name is a plain identifier and whose value is an `arrow_function` or `function_expression` (no parentheses or casts) |
| B4 | `call_not_admitted` | a `variable_declarator` whose value is a `call_expression` that S1's `js_ts_wrapped_export` does not admit (`forwardRef`/`memo`, and `useCallback` under E4 in S1b-2 only) |
| B5 | `written` | an assignment, augmented assignment, update, or head-less `for…in/of` whose target pattern names `n`, anywhere in the scope, resolves (by this same walk, from the target) to this scope |

The terminal's span is the declaration (function declaration), the expression (named function expression), the value
(declarator), or the admitted wrapper's inner function. `wrapped` is true for `forwardRef`/`memo` and false
otherwise. `local` is the registered name of that function node.

### 3.2 D4: module-scope terminals for the ESM export routes (S1b-1)

For each ESM local export occurrence (M1: list without `source`, default identifier, `export [default] function`,
exported arrow or function-expression declarator), in this order:
1. `js_ts_forwarded_import(name)` as on base (`ImportForward`).
2. If `name` is an import binding (value or type-only): keep `Local(name)`, which the existing loop poisons (E9a).
3. Otherwise evaluate the terminal at the program scope with the base write scan
   (`js_ts_module_value_written`, READ `src/ast.rs:4637`; its arrow-parameter gap (M9) can only over-report writes,
   which refuses; MEASURED identical to the prototype's scoped scan on X, F, R and every control, Q22):
   - `Ok`, not wrapped → **`JsExportTarget::VerifiedLocal { local, start_line, end_line }`** (new);
   - `Ok`, wrapped → S1's `SpannedLocal { … }`;
   - `Err(reason)` → `UnprovenLocal(name)` (retained, so duplicate and barrel conflicts still see the claim) and
     `local_export_refusals[reason] += 1`. A name with no module-scope declaration is `undeclared`.

S1's declarator arm (`export const X = forwardRef(fn)`) is unchanged. CJS `Local` is unchanged.

**Data model (`src/js_exports.rs`).**
- `JsExportTarget::VerifiedLocal { local, start_line, end_line }`; `resolve_one_inner` resolves it like
  `SpannedLocal` with `wrapped: false`.
- `ResolvedJsExport.wrapped: bool` (`#[serde(default)]`): true only for `SpannedLocal`. The star-barrel candidate key
  gains it: `(file, local, is_class, span, wrapped)`.
- `JsExportFacts.local_export_refusals: BTreeMap<String, usize>` (`#[serde(default)]`), included in `is_empty()`.

### 3.3 R4c (S1b-1)

In `js_ts_import_member_candidates`: return `WrappedExportNonJsx` iff `resolved.wrapped && !site.jsx_element` (was
`span.is_some()`); filter by span whenever `span` is `Some`; with a span, 0 or 2+ matches yield no candidate. S1's
lowercase check is deleted (§3.5 subsumes it on every route). With `span == None` (CJS), behavior is base.

### 3.4 R3 namespace qualifiers (S1b-3)

Applies only when the caller is JS/TS and the qualifier `q` has exactly one import binding in the file, of kind
`ModuleImport` (an ESM `import * as q`, M4). The existing `receiver_lexically_bound` guard still runs first.
- **R3-1 (the module resolves relatively).** Resolve the member through `js_ts_import_member_candidates(caller,
  binding, member, site)`, exactly as R4c does: 1 → Exact `ImportQualified`; 0 → drop `UnknownName`; 2+ (only
  possible for span-less CJS targets) → NameOnly; `Err` → that drop. This follows renames and re-export chains
  (C83), so it can add a correct edge.
- **R3-2 (it does not resolve: non-relative, or `./x.js` for `x.ts`; E7).** Take base's stem candidates, keep only
  functions `fid` whose own file's resolved export under `member` is `(fid.file, fid.name)` with `fid`'s span (or no
  span, for CJS). If any kept candidate is `wrapped` and the site is not JSX → `WrappedExportNonJsx`. 0 → base's
  `ImportExternal`; 1 → Exact; 2+ (same-stem files in different directories) → NameOnly.
- Every other qualifier (named import, default import, `require`) is base (E8).

### 3.5 JSX intrinsic guard (S1b-1)

First statement of `resolve_call_site_full`: if `site.jsx_element && site.qualifier.is_none()` and the callee's
first character is an ASCII lowercase letter, or the callee contains `-` or `:`, drop `DropReason::JsxIntrinsic`.
This is the React/TypeScript/Babel rule for intrinsic elements (TS `isIntrinsicJsxName`; Babel `isCompatTag`).
Member tags (`<lib.island/>`) carry a qualifier and are never intrinsic (C98). `_x` and `$x` tags are component
references and stay on the normal rungs.

### 3.6 R4 `LocalDef` by lexical binding (S1b-2)

**Extraction.** `CallSite.local_binding: JsLocalBinding` (`#[serde(default)]`, excluded from `cmp_key`) is set at the
three source constructors (`src/call_graph.rs` build skeleton, full build, incremental build), from the site node
with exactly the site's bytes:
- `Unchecked` (default): not JS/TS; qualified; synthetic or `IndirectResolution`; the callee is not a plain
  `identifier` (call `function` field, or JSX `name` field); or no function in the file has that registered name.
- `Callable(JsTerminal { start_line, end_line, wrapped })`: §3.1 at the site.
- `Unproven`: §3.1 refused, or the name is unbound.

The terminal is memoized per file by `(scope node id, name)` (Q8: an unmemoized walk did not finish T in the time
base needs).

**Resolution**, where R4 has at least one same-file free candidate (`local`):
- `Unchecked` → base (`exact(local, LocalDef)`).
- `Unproven` → drop `DropReason::LocalBindingUnproven`. It never falls through to R5 (the name's in-file candidates
  are exactly what the binding refuses; R5 would re-bind the same functions as `FreeSingle`).
- `Callable(t)`: if `t.wrapped && !site.jsx_element` → `WrappedExportNonJsx` (C63, C106). Keep the candidates with
  `t`'s span: exactly 1 → Exact `LocalDef`; otherwise → `LocalBindingUnproven` (same-line collisions, C95).

### 3.7 `useCallback` (S1b-2, under E4a)

`js_ts_wrapped_export`'s wrapper table gains `useCallback` for the local route only (not the export arm, which keeps
`forwardRef`/`memo`): same ESM-`"react"` table, same R6 provenance, arity 1–2 (the second argument is the deps
array, so R11 never applies). The terminal is `wrapped: false`: any site may bind it.

## 4. Controls (MEASURED, P7; `probes/S1b-controls-expectations-pre-run.md`, `probes/S1b-controls-proto-v7*.txt`)

Every S1b scenario runs in the JSX (`_jsx`: `.jsx`/`.js`) and TSX (`_tsx`: `.tsx`/`.ts`) grammar; C102–C104 and C114
are TS-only; 153 scenarios in all. All base and P7 results match the pre-registered expectations, except the three
recorded deviations (Q5) and C112 (Q9).

| Control | Rule | Sub-slice | base → S1b |
|---|---|---|---|
| C06, C68 list/default of a ternary + nested `f` | B3 | 1 | Exact to the decoy → drop |
| C21, C77 exported binding written | B5 | 1 | Exact → drop |
| C69, C70, C71 function, arrow and list exports + nested decoy | span | 1 | NameOnly ×2 → Exact ×1 (re-target) |
| C72 list-exported `forwardRef`: `Island({})`, `<Island/>` | §3.3 | 1 | Exact, Exact → `WrappedExportNonJsx`, Exact |
| C73 list-exported `create(fn)` | B4 | 1 | Exact to the argument → drop |
| C74, C75 duplicate `var`; hoisted `var` competitor | B2 | 1 | NameOnly ×2 / Exact → drop |
| C78 `const f = function g(){}; export { f }` | span | 1 | drop → Exact `g` (addition) |
| C79 unrelated parse error in the exporting file | B1 | 1 | Exact → drop (E6 cost) |
| C96 `<div>`, `<island>…</island>`, `<my-el/>` | §3.5 | 1 | Exact `local_def` → `JsxIntrinsic` |
| C97 (Z02b) `export const island`; `island()`, `<island/>` | §3.5 | 1 | Exact, Exact → Exact, `JsxIntrinsic` |
| C98 `<lib.island/>` | §3.5 (not intrinsic) | 1 | Exact, unchanged |
| C63 (S1b-RED-2) producer `Island({})` | §3.6 | 2 | Exact → `WrappedExportNonJsx` |
| C62 producer `helper`'s `Island()`, C11, C69–C71/C80/C81/C85 producers | §3.6 | 2 | Exact ×2 → Exact ×1 (its own nested function) |
| C86 two nested `const g` | §3.6 | 2 | Exact ×2 each → Exact ×1 each |
| C87 shadowing parameter; C101 single arrow parameter | Σ | 2 | Exact → drop |
| C88 Pattern-2 `require:` + global `require()` | `unbound` | 2 | Exact → drop |
| C89 `useCallback` | §3.7 | 2 | Exact → drop (E4b) / Exact (E4a) |
| C90 `throttle(fn)`; C107 `lazy(loader)` | B4 | 2 | Exact → drop |
| C91 `let g; g = () => 1` | B3 | 2 | Exact ×2 → drop |
| C92 catch / for-of / class / `with` shadows | Σ | 2 | 4× Exact → drop; the try-block call stays Exact |
| C93 block function: call inside / outside the block | Σ + Annex B | 2 | Exact / Exact → Exact / drop |
| C94 `var` hoisted from a nested block | hoist | 2 | Exact, unchanged |
| C95 same-line `const f` in two functions | 2 span matches | 2 | Exact ×2 → drop |
| C99 recursion; named function expression self-call | Σ | 2 | Exact, unchanged |
| C102 TS overloads | not a declaration | 2 | Exact, unchanged |
| C103 `declare function f` + nested decoy | B3 | 2 | Exact → drop |
| C104 block `enum E`; `function N` + `namespace N` | Σ / B2 | 2 | Exact → drop (the second is a fail-closed loss; TS merges them) |
| C105, C108, C109 parse recovery in the binding scope (generic, JSX, hides a closer declaration) | B1 | 2 | Exact → drop |
| C106 same-file `memo`: `Card({})`, `<Card/>` | §3.6 | 2 | Exact, Exact → `WrappedExportNonJsx`, Exact |
| C110 parameter default `g()` vs body `function g` | `formal_parameters` | 2 | Exact ×2 → Exact → top-level `g` |
| C111 Annex B: `inner()` after `if (…) { function inner(){} }` | Annex B | 2 | Exact ×2 → drop |
| C113 `arguments()` inside a function / inside a top-level arrow, with a top-level `function arguments` | implicit `arguments` | 2 | Exact / Exact → drop / Exact |
| C114 (TS) `using h = res(); h()` in one function, `h()` in another, top-level `function h` | B5 (the mis-parse is a write) | 2 | Exact / Exact → drop / drop (fail-closed; the second is a right edge lost) |
| C62 (S1b-RED-1) `<Lib.Island/>` + decoy | R3-1 | 3 | Exact ×2 → Exact `Island@6-8` |
| C80 namespace + decoy | R3-1 | 3 | Exact ×2 → Exact ×1 |
| C81 `./lib.js` + decoy | R3-1 (`_jsx`) / R3-2 (`_tsx`) | 3 | Exact ×2 → Exact ×1 |
| C82 `Lib.Island({})`, `<Lib.Island/>` | R3 + §3.3 | 3 | Exact, Exact → `WrappedExportNonJsx`, Exact |
| C83 namespace re-export rename | R3-1 | 3 | drop → Exact (addition) |
| C84 named-object qualifier; C85 default-import qualifier + decoy | non-goal (E8) | – | unchanged |
| C112 `import * as Lib` + `var Lib` | existing guard (M4) | – | drop, unchanged |

## 5. Counters and observability

- **New drop reasons** (exhaustive match, `src/navigation/queries.rs`): `JsxIntrinsic` → `dropped_jsx_intrinsic`;
  `LocalBindingUnproven` → `dropped_local_binding_unproven`. `WrappedExportNonJsx` now also counts R3 and R4 sites.
- **New key** `js_export_local_refusals` (sorted object, summed over files).
- **Expected values** (P5; S1b-1 alone is the `ONLY_A` run; S1b-3 adds 0 on the corpora):

| Corpus | `dropped_jsx_intrinsic` (S1b-1) | `js_export_local_refusals` (S1b-1) | `dropped_local_binding_unproven` (S1b-2, E4a / E4b) | `unresolved_unknown_name` |
|---|---|---|---|---|
| X | 866 | `{call_not_admitted: 4, not_callable: 3}` | 75 / 86 | 10,922 → 10,118 |
| F | 490 | `{call_not_admitted: 9, not_callable: 19, parse_recovery: 6}` | 97 / 191 | 11,399 → 11,012 |
| R | 117 | `{}` | 1 / 1 | 721 → 604 |
| T | 0 | `{parse_recovery: 32}` (no edge impact) | 398 / 398 | 28,736, unchanged |

`js_export_barrel_conflicts` is unchanged (E9a). `multi_target_exact_sites` (S1b-2): X 158 → 6 (the 6 are E8's R3
rows), F 5 → 1, T 680 → 6.

## 6. Cache

Each sub-slice bumps both caches, incrementing from the landed value:
- S1b-1: `CACHE_VERSION` 98 → 99 ("v99: JS/TS export terminals (`VerifiedLocal`, `ResolvedJsExport.wrapped`,
  local export refusals), JSX intrinsic drop"); `NAV_CALL_EDGE_CACHE_VERSION` 54 → 55.
- S1b-2: 99 → 100 ("v100: `CallSite.local_binding`"); 55 → 56.
- S1b-3: 100 → 101 and 56 → 57. It changes no persisted fact, but both caches hold resolution-derived edges (CPG
  Call edges, nav call edges), so a full cache hit would otherwise serve the old R3 edges.
- Update both pins. If either constant has landed higher, increment from it.

## 7. Test plan

**RED rule.** Every new path has a behavioral test that fails on the sub-slice's base with a concrete recorded
value (revert only that sub-slice's production lines). Compile, setup and zero-test failures are inadmissible.
Guards (base-green preservation rows) are marked as such and are never cited as RED evidence.

**Location.** `tests/integration/js_binding_terminal_test.rs` (S1b-1 export rows), `js_local_binding_test.rs` and
`js_local_binding_shadow_test.rs` (S1b-2), `js_namespace_export_test.rs` (S1b-3), each under 600 lines and
registered in `tests/integration/main.rs`; counters in `tests/cli/call_stats_test.rs`; one nav row in
`tests/navigation/callers_test.rs`. Table-driven, both grammars per row, exact `(file, name, start, end)` targets with
decoys.

**Existing tests to update** (Q15; each is a representation or reason change, not a behavior regression):
`js_export_test::{extract_const_arrow_export, extract_const_function_expression_export,
extract_default_export_identifier, extract_default_export_named_function, extract_named_export_list_with_rename}`,
`type_relative_receiver_test::indirect_default_class_facts_do_not_fall_back_to_callable_exports`,
`cjs_refusal_state_test::cjs_refusal_raw_serde_and_esm_custody` (`Local` → `VerifiedLocal`, S1b-1);
`js_wrapped_export_test::t_j2_t_j4_jsx_gate_scope` (`UnknownName` → `JsxIntrinsic`, S1b-1);
`module_binding_audit_test::esm_namespace_import` (Gap → Supported, S1b-3).

### S1b-1

| ID | Input | Expected |
|---|---|---|
| A-1 | C06, C68 | drop; `local_export_refusals.not_callable` |
| A-2 | C21, C77 | drop; `written` |
| A-3 | C69, C70, C71 | Exact ×1 → the top-level span (RED: NameOnly ×2) |
| A-4 | C72 | call `WrappedExportNonJsx`; JSX Exact |
| A-5 | C73 | drop; `call_not_admitted` |
| A-6 | C74, C75 | drop; `duplicate_declaration` |
| A-7 | C78 | Exact `g` |
| A-8 | C79, and a twin with the error in the importer | drop / Exact |
| A-9 | TS overload export (`export function f(a: string): void; export function f(a: any) {}`) | Exact (guard) |
| A-10 | imported local re-listed (`import { f } …; export { f }` without forwarding) | `conflicted` holds `f`; `barrel_conflicts` unchanged (guard, E9a) |
| A-11 | C96, C97 (self-closing and opening elements, `-` and `:` names) | `JsxIntrinsic`; `dropped_jsx_intrinsic` |
| A-12 | C98, `<_x/>`, `<$x/>` | not intrinsic (guard) |
| A-13 | serde round-trip of `VerifiedLocal`, `ResolvedJsExport.wrapped`, refusals; star-barrel key with `wrapped` | equal; conflicting claims still conflict |
| A-14 | full vs incremental epochs: export arrow → rewritten to a ternary → restored → a nested decoy added | expected edges every epoch; full == incremental |
| A-15 | CLI `call-stats` keys | present with exact values |
| A-16 | pins | 99 / 55 |
| A-17 | module-scope Annex B: `if (x) { function f(){} } export { f }`, and the same plus a top-level `function f` | drop; drop (`duplicate_declaration`) |

Mutants (each applied alone; the killing test recorded): **A-M1** remove the intrinsic guard (A-11); **A-M2** treat
qualified tags as intrinsic (A-12); **A-M3** record `Local` instead of the terminal (A-1, A-5); **A-M4** skip B5
(A-2); **A-M5** skip B2 (A-6); **A-M6** skip B1 (A-8); **A-M7** gate R4c on `span.is_some()` instead of `wrapped`
(A-3: a plain call drops); **A-M8** drop `wrapped` from the barrel key (A-13).

### S1b-2

| ID | Input | Expected |
|---|---|---|
| B-1 | C63, C106 | `WrappedExportNonJsx`; JSX Exact |
| B-2 | C62/C11 producer `helper`, C86 | Exact ×1 → the site's own function (RED: Exact ×2) |
| B-3 | C87, C101, C92 (catch, for-of, class, `with`), C104, C103 | drop `LocalBindingUnproven` |
| B-4 | C88 | drop, and **no** `free_single` edge |
| B-5 | C89 under E4a, plus impostor twins: a local `function useCallback`, `useCallback` from `"./hooks"`, `require("react").useCallback` | Exact / drop ×3 |
| B-6 | C90, C107, C91 | drop |
| B-7 | C93, C94, C99, C102 | inside Exact, outside drop; Exact; Exact; Exact (guards except C93-outside) |
| B-8 | C95 | drop |
| B-9 | C105, C108, C109 (both grammars) | drop |
| B-10 | C110 | Exact → top-level `g` |
| B-11 | C111 | drop |
| B-11a | C113, C114 | drop, Exact / drop, drop |
| B-12 | two names bound in one scope, each with a nested decoy, called from one function | each call binds its own function (kills a memo keyed without the name) |
| B-13 | `IndirectResolution` and qualified sites | `Unchecked`; base result (guard) |
| B-14 | full vs incremental epochs: add a shadowing parameter → remove it → move the callee two lines | expected edges; full == incremental |
| B-15 | serde of `CallSite.local_binding`; CPG cache full hit keeps the edge | equal |
| B-16 | pins | 100 / 56 |

Mutants: **B-M1** ignore `local_binding` (B-2, B-3); **B-M2** no `formal_parameters` scope (B-10); **B-M3** no
Annex-B marker (B-11); **B-M4** drop the arrow `parameter` field (C101 in B-3); **B-M5** `with` not a scope (B-3);
**B-M6** skip the catch parameter (B-3); **B-M7** skip B1 (B-9); **B-M8** no JSX gate on wrapped (B-1); **B-M9** memo
key without the name (B-12); **B-M10** demote a same-line collision to NameOnly (B-8); **B-M11** fall through to R5
on `Unproven` (B-4); **B-M12** admit `useCallback` without the React provenance (B-5).

### S1b-3

| ID | Input | Expected |
|---|---|---|
| C-1 | C62, C80 | Exact ×1 (RED: Exact ×2) |
| C-2 | C81 in both grammars (R3-1 and R3-2) | Exact ×1 |
| C-3 | C82 | `WrappedExportNonJsx`; JSX Exact |
| C-4 | C83 | Exact `impl:g` |
| C-5 | a non-relative namespace (`import * as v from 'pkg/validators'`) over a module-scope export, and over a nested-only name | Exact / `ImportExternal` |
| C-6 | C84, C85, a `require` qualifier | unchanged (guards, E8) |
| C-7 | C112 | drop (guard, M4) |
| C-8 | nav `callers` for C62's `Island@6-8` | 1 caller, `import_qualified`, score 1.0 |

Mutants: **C-M1** stem lookup for relative namespaces (C-1); **C-M2** R3-2 without the export-identity filter
(C-2 `_tsx`); **C-M3** apply the namespace rule to named imports (C-6).

**Tier-A fixtures** (`eval/fixtures/typescript/`; Q11 shows each is RED on base and green on the prototype):
- S1b-1: `s1b_list_export_nested_decoy_refused` (C06 without the decoy's local call; `callers = []`,
  `forbid_resolution_kind = "import_member"`); `s1b_jsx_intrinsic_tag_refused` (`<input/>` vs a helper `input`).
- S1b-2: `s1b_param_shadow_local_def_refused` (C87; `forbid_resolution_kind = "local_def"`).
- S1b-3: `s1b_namespace_nested_decoy_refused` (C80 without the decoy's local call; forbid `import_qualified`).

## 8. Acceptance

1. **Suites.** `cargo fmt --check`; `cargo test --offline --no-fail-fast` (base 4,583 / 0 / 1, Q15; report the new
   total); `--features mcp`; `cargo clippy --offline --all-targets --features mcp`; Tier-A `--matrix-only` (base
   162 / 162; the prototype 162 / 162 plus the new fixtures, Q10–Q11) with 0 regressions; the Node gate;
   `tier-a --quick` if rust-analyzer is available, else reported as not run.
2. **Yield: the exact audited row-diff per corpus.** Run the release binary with
   `nav --no-cache call-stats --dump-sites` on X, F, R and T, then `probes/rowdiff.py` against the base dumps.
   - **S1b-1** must equal the P5/P7 `ONLY_A` row-diff (`probes/expected/S1b-1-*.json`): X 866 rows (804 relabels, 62
     intrinsic removals); F 490 (387, 103); R 117 relabels; T 0.
   - **S1b-2** (on top of S1b-1) must equal the P5 row-diff minus S1b-1 under the owner's E4 answer
     (`probes/expected/S1b-2-{E4a,E4b}-*.json`): X 220 rows (134 re-targeted, 86 removed; E4a: 75 removed);
     F 192 (1, 191; E4a: 97); R 1; T 1,033 (635, 398).
   - **S1b-3**: row-diff empty on all four corpora (latent), controls C62/C80–C83 as in §4.
   - `dfg-stats --edges` on X: the diff is exactly the Step-5b edges of the changed `local_def` rows (Q18).
   - F appears only as counts; its row-diffs and SHA-256 stay in the private evidence root.

   Any deviation is a blocker until the owner accepts it, reported row by row with the audit class.
3. **Custody.** Acceptance outputs go under `~/prism-evidence/s1b/acceptance/<sub-slice>/` with a `MANIFEST.sha256`.

## 9. Budget (honest lines: after `rustfmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` are tests)

**MEASURED (Q16):** the prototype is **681 src** (v7), of which about 14 lines are measurement-only switches, so about
667 of design code. It is feasibility code with some duplication: an uncached and a cached scope walk, and two write
scans. Per-function attribution (Q17):

| Part | Prototype lines | Sub-slice |
|---|---|---|
| declaration collector (`scope_walk`, import/ambient/pattern helpers) | 112 | 1 |
| terminal classification + module terminal + export target | 106 | 1 |
| `js_exports.rs` model | 30 | 1 |
| `ast.rs` producers | 9 | 1 |
| intrinsic guard, drop reasons, call-stats | about 25 | 1 |
| nested scope arms, `formal_parameters`, visible-binding walk, memo | about 140 | 2 |
| scoped write scan | 35 | 2 |
| `CallSite.local_binding` + extraction at three constructors | 79 | 2 |
| R4 filter | about 25 | 2 |
| `useCallback` | 14 | 2 |
| R3 namespace rules and helpers | about 85 | 3 |

**Forecasts** (after removing the prototype's duplication, plus cache bumps): S1b-1 about **275** src, S1b-2 about
**310**, S1b-3 about **95**. Tests, from S1's measured ratio (660 / 342 ≈ 1.9) and the control tables above:
about 520 / 600 / 250.

| Sub-slice | src cap / early stop | tests cap / report point | combined |
|---|---|---|---|
| S1b-1 | **300** / 270 | **600** / 540 | **900** |
| S1b-2 | **340** / 305 | **680** / 610 | **1,020** |
| S1b-3 | **120** / 108 | **300** / 270 | **420** |

Early stops are checkpoints: report the count and the forecast; continue only while the forecast is within the cap.
A forecast above a cap is a stop with the remaining items enumerated. Logic is never compressed to fit. The
implementer measures after **every** test batch (S1 lesson).

## 10. Risks and open questions

- **The declaration collector's closure** is the review surface. §3.1 argues it from ECMA-262's BoundNames; the
  auditor shares the planner's model (PLANNING-PROBES, custody), so reviewers should probe binding forms directly.
- **Grammar gaps** cost right edges under E6 (T 53, F 7 + 4). A tree-sitter-typescript upgrade is the remedy.
- **React semantics** (E4) and **may-call semantics** (E5) are assumptions or owner choices, not static facts.
- **Performance:** memoization is required (Q8); Q19 records the clean T timing.
- **Nav correlation for R3-1 additions:** `scoped_caller_site_match_count` (READ
  `src/navigation/call_resolve.rs:34-40`) returns early for qualified sites, so `callers` of a function reached only
  through a renamed namespace export (C83) may not list that caller. 0 such rows on the corpora; C-8 pins the
  common case.

## 11. Review history

| Round | Verdict | Disposition |
|---|---|---|
| spec r1 | – | – |
