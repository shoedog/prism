# Wrapped-export resolution: span-verified React `forwardRef` / `memo` named exports (slice S1)

**Status:** planning complete, revision **r3 (Branch P) with the at-cap round-3 fold**. The owner chose Branch P of
`REPLAN-fable.md` after sol's round 2. Sol's round 3 (final; FIX, 2 WRONG / 1 SMELL, bounded and converging) was
folded at the cap under owner decisions D10–D11 (§11, `REVIEW-r3-fold.md`). This document is normative for
implementation. **Implementation starts directly, with no further spec round.** Sol reviews the implementation, under
its own 2-round cap, which the controller declares before dispatch.

**Base:** `origin/main` `12ca6e8e`.

**Grounding:**
- `PLANNING-PROBES.md` (M-ids for mechanisms, P-ids for measurements);
- `REPLAN-fable.md` (the model diagnosis);
- the replan precedent probes in `~/prism-evidence/wrapped-export/replan/RESULTS.md` (Q1–Q9) and
  `~/prism-evidence/wrapped-export/spec-reviews/precedent-probe/`.

**Predecessor:** the PR #324 readout (`docs/eval/entry-call-proof/readout.md`). It names the wrapped named export as
the dominant refusal lane for the ten `forwardRef` components.

## 0. Owner decisions

| # | Decision | Answer (owner, 2026-09-25) | Basis |
|---|---|---|---|
| D1 | Which wrapper shapes are admitted | **A.** React `forwardRef` / `memo` only (`memo` may take a comparator), with the callee resolved to an **ESM** import of `"react"` | Measured yield 107 (X) and 4 (F), 0 wrong (P6, P7). Library wrappers yield 0 on four corpora. Shape-only admission would bind 27 declarators to the wrong callable (P2, P4) |
| D2 | Confidence of admitted edges | **Exact, as project-standard static binding (Branch P).** The analysis model is §3.2. JSX-only binding (§3.4) stays | `REPLAN-fable.md` §1–§2.4. `import_member` and `import_qualified` on `main` already emit Exact through same-file direct writes, `require` re-acquisition, `Object.defineProperty`, `__defineGetter__` and `eval` (replan Q1–Q6), and so does Python |
| D3 | Slicing | **S1 alone.** S2 (default-object member aliases) and S3 (anonymous wrapper-nested callbacks) are separate slices | S2: 7 X sites, a different R4c route. S3: new function identity, a much larger blast radius (P13, P14) |
| D4 | The pre-existing F4-class false Exact on the export-list path (C06) | **a.** Follow-up slice S1b, reusing `SpannedLocal`. **Widened at r3 (D10):** S1b span-verifies **all** JS/TS export routes (§12) | 0 measured prevalence (P5) |
| D5 | Sequencing | **Ship S1 now.** tsconfig `paths` resolution is the next planning lane | P4 latent projection |
| D6 | r2's closure of the mutable React object (default/namespace custody vs named-only) | **Superseded by Branch P.** Custody K1–K8 is dropped. Runtime mutation and re-acquisition of the React object are out of model (§3.2) | Under sol's standard the class is open for both D6 options (replan Q9: `arguments[1]('react')` and `module.require('react')` pass r2 custody in both modes). Branch S would keep 7 / 0 edges |
| D7 | Budget | **src 350 / tests 680 / combined 1,010** (owner decision, 2026-09-25, after the implementation test-cap stop at 698; originally 350 / 600 / 950 with early stops 315 / 540 / 855; report point 650 tests) | `REPLAN-fable.md` §4 (P row). MEASURED Branch-P prototype: 325 src (P26); with the r3 fold, 328 (P30) |
| D8 | Project-wide statement of the Exact contract | **Ships in this slice's PR.** The implementer adds the §3.2.1 text to `CLAUDE.md` | Without it, S1b, S2 and `paths` would re-litigate the same question (`REPLAN-fable.md` §2.1) |
| D9 | Review | **One final sol round 3**, judged against the Branch-P model. It is done (FIX 2 / 1, bounded). Sol reviews the implementation next (`REVIEWER.md`) | 2-round cap spent; owner-approved exception |
| D10 (r3) | Sol r3 W1: R3 `ImportQualified` and R4 `LocalDef` bypass the span and JSX gates (pre-existing on base) | **Defer to S1b.** S1's contract is narrowed to the new R4c `import_member` route (§2, §3.2, §3.4). S1b's scope is widened (§12), and sol's two inputs are its first RED cases. No R3 or R4 code in S1 | MEASURED P31: both inputs give the same result on base and Branch P (C62, C63) |
| D11 (r3) | Sol r3 W2 and SMELL | **Fold.** R6-P4 walks module-scope `var` hoisted from top-level blocks, loops, `switch` and `try` (T-R6-P4b, M14). T-J4 is a base-green preservation control | P30–P33 |

**History.** r1 set D2 = Exact. r2 made it conditional on JSX-only binding and closed "occurrence custody" (D6 = a).
Sol's round 2 showed that the custody class is open (3 WRONG). `REPLAN-fable.md` showed that it is also open for
the named-only alternative, and that prism's Exact contract has never covered runtime mutation. The owner took
Branch P.

## 1. Problem

`export const Island = forwardRef<…>((props, ref) => …)` records no export fact, because the declarator arm admits
only arrow and function-expression initializers (M1, the F4 fix of M2). An imported `<Island/>` therefore reaches R4c
with no resolved export and drops as `UnknownName` (M3). On Excalidraw this accounts for 107 dropped call sites
across 41 declarators, 72 of them in the PR #324 E2 ledger (P4, P6). The inner render function already has a stable
callable identity: Pattern 3 names it after the declarator (M4), and same-file uses already bind to it Exact (M7).

## 2. Scope

**In scope.**
- For each top-level `export const X = W(fn…)` declarator in a `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts` or `.tsx` file,
  where `W` is an admitted React wrapper (§3.1), record an export fact for `X` whose target is **the exact function
  node** `fn`. Re-export chains and `export *` barrels carry that target (M3; P6a C14).
- On the new R4c `import_member` route, such a target is bound only from **JSX element** sites, and only to the
  inner function's exact span (§3.4). The existing R3 `ImportQualified` and R4 `LocalDef` routes are unchanged
  (see the non-goals and §12).
- The `CLAUDE.md` statement of the Exact contract (§3.2.1).

**Non-goals.** Each stays exactly as on base, and each is pinned by a test in §7.
- **S2 and S3:** default-object member aliases; the anonymous nested or cast wrappers `memo(forwardRef(fn))`,
  `Object.assign(forwardRef(fn), …)` and `forwardRef(fn) as T`, whose inner function has no identity (M4, M5).
- **Identifier arguments** (`memo(Comp)`, `forwardRef(Inner)`).
- **Admission:** non-React wrappers, shape-only admission, `let`/`var` declarators, and CommonJS acquisition of the
  wrapper (`const { forwardRef } = require("react")`, T-N18).
- **Other routes:** the list and default-identifier `Local` routes (D4, which is S1b); non-relative module
  specifiers (D5); `ImportForward` of a wrapped export (M11).
- **Same-line `memo` comparator collisions.** These are refused and counted (R11). They are a formatting-dependent
  false negative, with 0 occurrences on X and F (P18).
- **Non-JSX uses of an admitted export.** `X(props)` drops as `WrappedExportNonJsx`, and `new X()` is not a call site
  (M13).
- **Runtime mutation or re-acquisition of the React module object.** This is out of model (§3.2), pinned by MB1–MB3.
- **Wrapped targets reached through other rungs** (D10, deferred to S1b §12):
  - namespace JSX `<Lib.Island/>` via R3 `ImportQualified`, which is Exact to every same-stem `Island` including a
    nested decoy (C62);
  - producer-local calls via R4 `LocalDef`, including non-JSX `Island({})` (C63).

  S1 leaves both byte-identical to base.
- Any change to `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`, DFG, or Step 5b.

## 3. Semantics

### 3.1 Admission predicate and reason precedence

The predicate is evaluated inside the existing `lexical_declaration | variable_declaration` arm
(`src/ast.rs:2861-2893`), for an exported declarator `d` whose `name` is an identifier `X`.

- **Arrow and function-expression initializers** keep recording `Local(X)` exactly as on base. No reason is counted.
- **Every other initializer** runs through the checks below **in order**. The first check that fails names the
  reason, and it increments both `skipped_expr_count` (base behavior) and `skipped_decl_reasons[reason]` (§5).
- **R7 is intentionally absent.** It was r2's custody check, removed under Branch P. The gap keeps r2 cross-references
  valid.

| # | Reason | Refuse when |
|---|---|---|
| R1 | `non_call_initializer` | the value is missing, or its kind is not `call_expression` (this includes a cast `forwardRef(fn) as T`) |
| R2 | `not_const` | the declaration is not a `lexical_declaration` with `kind == const` |
| R3 | `parse_recovery` | the enclosing `export_statement` has an ERROR or MISSING node |
| R4 | `import_parse_recovery` | any top-level `import_statement` in the file has an ERROR or MISSING node. This is per statement, not whole-file |
| R5 | `callee_not_admitted` | the callee does not resolve through the **ESM React import table** below: an identifier bound by a named value specifier whose imported name is `forwardRef` or `memo`, or `O.P` where `O` is bound by a default or namespace value clause and `P` is a `property_identifier` spelled `forwardRef` or `memo` |
| R6 | `callee_provenance` | any of P1–P5 fails for the callee local (`L`, or `O` in the member form) |
| R8 | `arity` | the call's `arguments` field is not an `arguments` node, or, after ignoring `comment` children, there are 0 arguments, more than 1 for `forwardRef`, or more than 2 for `memo` |
| R9 | `nested_wrapper` | the first argument is a `call_expression` |
| R10 | `first_arg_not_function` | the first argument is not an `arrow_function` or `function_expression` |
| R11 | `comparator_line_collision` | `memo` has a second argument that is an arrow or function expression whose `function_name` text **and** `node_line_range` equal the inner function's |
| R12 | `inner_unnamed` | `function_name(fn)` is `None`. **Unreachable** given R10 and Pattern 3; kept as a defensive `ok_or`, with no fixture |

**The ESM React import table (R5).** It is built by a hand parser over the file's top-level `import_statement` nodes
only (`react_imports()` in `prototype/wrapped-export-prototype-P3.diff.txt`):
- the module string must equal `"react"` exactly;
- type-only statements and specifiers are skipped;
- a named specifier's imported name must be an `identifier`, not a string name.

The parser is ESM-only by construction, so `require("react")` bindings can never enter the table (r2 SMELL 2; T-N18).
**Do not replace it with `extract_import_bindings()`**, which also emits `MemberImport` for a destructured
`const { … } = require(…)` (READ `src/ast.rs:2093-2110`) and does not record the origin. If an implementer does reuse
it, the mandatory origin check is "the binding's local is declared by a top-level `import_statement`", and T-N18
must still pass. The r2 SPEC's claimed 50-line saving from that reuse is retracted.

**Provenance subclauses (R6).** Each has its own test.
- **P1.** Exactly one import binding in the file has `local == L`, across all modules and origins
  (`extract_import_bindings()`).
- **P2.** `L` is not a type-only import (`js_ts_type_only_imports()`). T-R6-P2 uses sol's collision fixture, and
  mutant M12 is MEASURED to flip it (P27).
- **P3.** `L` is not written as a binding anywhere in the file (`js_ts_module_value_written`, `src/ast.rs:4613`).
- **P4.** No module-scope declaration of `L` exists. That is: a top-level or `export`-wrapped function, class,
  declarator, or other named declaration; **or a `var` declarator, or a `for (var … in/of …)` head, hoisted out of
  top-level blocks, loops, `switch` or `try`** (sol r3 W2). Implement it as one recursive walk from the root that
  stops at nested functions, classes, and non-top-level `let`/`const` (which are block-scoped) and skips
  `import_statement`. Block-scoped declarations and declarations inside nested functions do not compete (C60, C61,
  C67). MEASURED P30: 39 honest lines (`prototype/wrapped-export-prototype-P3.diff.txt`,
  `module_scope_declares`).
- **P5.** The spelling of `L` contains no `\`.

These are static-binding facts. They match what `js_ts_forwarded_import` refuses today
(`src/ast/js_module_forwarding.rs:12-18`).

### 3.2 Analysis model (Branch P; replaces r2 §3.2 in full)

> S1 binds by static provenance. The callee must be a named or default/namespace **ESM** import binding of
> `"react"` that is unique, value-typed, unwritten as a binding, and uncompeted at module scope (R5, R6 P1–P5). On
> the new R4c `import_member` route, the export target is the inner function's exact span, and only JSX element sites
> bind. The R3 and R4 routes are unchanged and are deferred to S1b. Runtime mutation of the React module
> object, by any means and from any module, is **out of model**, exactly as it is for every `import_member` and
> `import_qualified` edge prism emits today (replan probes Q1–Q6). Prism grades the evidence path, not the heap.

**The line.** S1 proves what a name denotes. It does not prove what the heap holds.
- **In model (static provenance):** R4, R5, R6 P1–P5, the span filter, R11, the barrel span key and the JSX gate, all
  on the R4c `import_member` route. A defect in any of these on that route is a WRONG. Pre-existing R3 and R4
  behavior on wrapped targets is recorded S1b scope (§12), not an S1 defect.
- **Out of model (runtime mutation of a module object):** direct member writes, `require`/`import()`
  re-acquisition, reflective or inherited-receiver writes, `eval`/`with`, CJS-wrapper handles (`arguments[1]`,
  `module.require`), and host globals. These are pinned as MB1–MB3 asserting Exact, so a future change to the model
  flips a named test on purpose rather than by accident. This is the same treatment as C06.

**Precedent (MEASURED on `main`, `REPLAN-fable.md` §1.2):**
- Same-file `require("./lib").f = g`, `Object.defineProperty(require("./lib"), …)`, `__defineGetter__` and
  `eval` all leave `import_member` Exact.
- `L.f = g` leaves `import_qualified` Exact.
- Python `lib.f = g` leaves Python `import_member` Exact.
- The only same-file mutation prism models is the CJS producer barrier over the file's **own** `module.exports`
  (replan Q8). There, mutation is the construction idiom.

**r2 §3.2 (superseded).** r2 defined a closed "occurrence custody" over `"react"` default and namespace bindings
(K1–K8, write positions w1–w3) and claimed that the same-file model was closed. Sol's round 2 refuted that claim with
three inputs. `REPLAN-fable.md` §1.1 showed that no finite denylist closes it. The text is preserved at commit
`1d4320b1` (SPEC §3.2). Its prototype is `prototype/wrapped-export-prototype-r2.diff.txt`.

#### 3.2.1 Exact text the implementer adds to `CLAUDE.md` (owner decision D8)

Insert this as a new paragraph in the Architecture section, immediately after the sentence "`asserted` grades the
evidence path, not the heuristic's truth.":

> `Exact` is a static-binding grade: the site's name resolves, by the language's declaration, scope, import
> and export syntax as prism models it, to exactly one callable. It does not assert runtime truth. Runtime
> mutation of module objects (monkey-patching, reflective writes, `eval`, host globals, re-acquisition through
> `require`/`import()`), from the same file or another, is out of model for every rung and every language
> (replan probes, 2026-09-25). Producer-side mutation is modeled only where mutation is the construction
> idiom (CJS `module.exports`, `src/ast/js_cjs_export_barriers.rs`). A slice that wants to model runtime
> mutation proposes it as a new contract, not as a precision fix to an existing rung.

This is `REPLAN-fable.md` §2.1's draft, adopted as-is. The planner does **not** edit `CLAUDE.md`; the text lands with
the implementation PR.

### 3.3 Data model (`src/js_exports.rs`)

- **New variant.** `JsExportTarget::SpannedLocal { local: String, start_line: usize, end_line: usize }`. It is
  produced only by §3.1.
- **Resolved span.** `ResolvedJsExport` gains `#[serde(default)] pub span: Option<(usize, usize)>`.
- **Resolution.** `resolve_one_inner` handles `SpannedLocal` as a direct hit and copies the span. **The star-barrel
  candidate key includes the span**, so two claims with the same `(file, local)` but different spans conflict
  (T-N16, M8).
- **Unchanged behavior, each with a test:**
  - an `ImportForward` whose terminal is `SpannedLocal` stays `BlockedClaim`;
  - `insert_named` poisoning;
  - the `imported.contains(local)` refusal, which applies to `Local` only.
- **New fields.** `JsExportFacts` gains `spanned_admitted: usize` and `skipped_decl_reasons: BTreeMap<String, usize>`,
  both `#[serde(default)]`. `is_empty()` includes them.

### 3.4 Resolution: span filter and JSX-only binding

No JSX-vs-call distinction exists today (M13), so S1 adds one:

- **`CallSite.jsx_element: bool`.** It is `#[serde(default)]` and excluded from `cmp_key`. It is set by
  `CallGraph::jsx_element_at(parsed, start_byte, end_byte)`, which is true iff the node with exactly those bytes is a
  `jsx_self_closing_element` or `jsx_opening_element`.
  - It is set at the three source constructors (`src/call_graph.rs:1368`, `:1800`, `:5101`).
  - `indirect_call_site` (`:2849`) and every synthetic constructor set `false`.
  - The rejected alternative, a `CallKind::JsxElement` variant, is discussed in r2 and still applies.
- **`js_ts_import_member_candidates(caller, binding, member, site) -> Result<Vec<&FunctionId>, DropReason>`.** When
  `resolved.span == Some((s, e))`:
  1. If `!site.jsx_element`, return `Err(DropReason::WrappedExportNonJsx)`. This is a new variant, counted in
     call-stats.
  2. Otherwise add the filter `fid.start_line == s && fid.end_line == e`.
  3. With exactly 1 match, the result is `Exact` `ImportMember`. With 0 matches, or with 2 or more, return
     `Ok(vec![])`, and the site drops `UnknownName`. A match is never demoted to NameOnly.

  When `span` is `None`, behavior is byte-identical to base.
- **Coverage.** `resolve_call_site_full` is the only production caller of this function, so the nav correlation
  inherits both gates **for the R4c route**. The gates do not reach R3 `ImportQualified` or R4 `LocalDef` (sol r3 W1;
  D10 defers that to S1b).
  - `new X()` produces no call site.
  - `X.call(…)` carries a qualifier.
  - Synthetic and indirect sites have `jsx_element == false`.
- **DFG.** No new DataFlow appears, because JSX sites carry no arguments (P11, P19).

## 4. Rules for each control

| Control | Rule | MEASURED: base / Branch-P prototype (P28) |
|---|---|---|
| F4 ternary + nested same name (C07) | R1 | drop / drop |
| `forwardRef(Ident)` ± nested same-name function (C09, C10) | R10 | drop / drop |
| wrapped export + nested same name elsewhere (C11) | span filter | drop / Exact `@6-8` |
| same-line nested same name (T-N11) | 2 or more span matches | (new) |
| local impostor `forwardRef` (C12); `./react-shim` / `preact/compat` (T-N4) | R5 | drop / drop |
| `const { forwardRef } = require("react")` (T-N18, C56) | R5 (ESM-only table) | drop / drop, `callee_not_admitted` |
| written, duplicate, type-only (C55) or competed callee binding; escaped spelling | R6 P1–P5 | C55: drop / drop, `callee_provenance` |
| top-level `function memo`, `class memo`, `const { memo } = x` (C64–C66) | R6 P4 | drop / drop, `callee_provenance` |
| hoisted `var memo` in a top-level `if` (sol r3 W2, C57), `for (var memo of …)` (C58), `for (var memo = 0; …)` in a `try` (C59) | R6 P4 (hoisted walk) | drop / drop, `callee_provenance` (P31) |
| block-scoped `let memo` (C60); `var memo` inside a nested function or class (C61); component-local `const memo` (C67) | not a competitor | drop / **Exact** |
| **S1b RED characterizations (unchanged by S1, D10):** namespace `<Lib.Island/>` with a nested decoy (C62, sol r3 W1a); producer-local `Island({})` (C63, sol r3 W1b) | R3 / R4 (not S1 routes) | C62: 2× Exact `import_qualified` (decoy `@3-3` and `@6-8`) / same. C63: Exact `local_def` / same |
| import with parse recovery (C29) | R4 | drop / drop |
| one-line `memo` comparator (C30) | R11 | drop / drop |
| span-sensitive star-barrel conflict (C39) | span in the barrel key | drop / drop |
| direct call `X(props)` (C31) | JSX gate | drop / `WrappedExportNonJsx` |
| `new X()` (C32) | no call site | no row / no row |
| non-React wrappers (C13, C16, C19, C23, C24) | R5 | drop / drop |
| nested or cast wrappers (C08, C25) | R9 / R1 | drop / drop |
| `export let` (C20) | R2 | drop / drop |
| importer parameter shadow (C18) | existing guard | not `import_member` / same |
| **Out of model, pinned Exact:** r1 W1 `React.forwardRef = fake` (C28 = MB1); r2 W1 named import + `require("react").forwardRef = fake` (C53 = MB2); r2 W3 `React.__defineGetter__` (C54 = MB3); and the other r2 custody controls: C33–C37 (`Object.assign`, alias, namespace write, `require` handle, named + default write), C40–C43 (`delete`, `||=`, destructuring, subscript), C45–C52 (`eval`, other-member write, `++`, `for…of`, shorthand, `export { React }`, escaped spelling, `with`) | §3.2 model | drop / **Exact** (18 r2-refused controls plus C53 and C54) |

## 5. Counters and observability

- **Per file.** `spanned_admitted` and `skipped_decl_reasons` (reasons R1–R6 and R8–R12). Invariant:
  `skipped_expr_count == sum(skipped_decl_reasons)` plus the default-export and CJS skips. T-O3 pins it.
- **call-stats.**
  - `js_export_spanned_admitted`;
  - `js_export_skipped_decl_reasons` (sorted object);
  - `dropped_wrapped_export_non_jsx`, added to the exhaustive drop match (`src/navigation/queries.rs:385-397`);
  - `js_export_skipped_exprs` is unchanged.
- **Expected (MEASURED P26, and again with the r3 fold in P33; identical to r2):**

| Corpus | `skipped_exprs` | `spanned_admitted` | reasons | `non_jsx` | `unresolved_unknown_name` | `kind_exact.import_member` |
|---|---|---|---|---|---|---|
| X | 637 → 596 | 41 | `{callee_not_admitted: 331, first_arg_not_function: 1, non_call_initializer: 247}` | 0 | −107 | +107 |
| F | 389 → 358 | 31 | `{callee_not_admitted: 168, non_call_initializer: 167}` | 0 | −4 | +4 |
| R | 28 → 28 | 0 | `{callee_not_admitted: 1, non_call_initializer: 23}` | 0 | 0 | 0 |

## 6. Cache

Raw export facts, `ResolvedJsExport` and `CallSite` are all persisted, so bump both caches:
- `CACHE_VERSION` 97 → **98**, with the note "v98: JS/TS span-verified wrapped React exports (`SpannedLocal`),
  JSX-element call-site flag, declarator skip reasons";
- `NAV_CALL_EDGE_CACHE_VERSION` 53 → **54**;
- update both pins.

If either constant has landed higher since, increment from the landed value.

## 7. Test plan

**RED rule.**
- Every T-P*, **T-J1–T-J3** and T-O* test must fail on base `12ca6e8e` with a behavioral assertion (revert only the
  §3.1 and §3.3–§3.4 production lines), and the concrete value is recorded. Mandatory JSX-gate RED evidence covers
  T-J1–T-J3 only.
- **T-J4 is a base-green preservation control** (sol r3 SMELL). It must pass before and after, and it is never cited
  as failing-first evidence.
- T-N* and T-R6-* tests are guards: they pass on base and after the change.
- MB* tests are model-boundary characterizations. They assert Exact on the new route, so they cannot pass on base,
  where there is no fact. They are not RED evidence for any refusal.

**Location.**
- `tests/integration/js_wrapped_export_test.rs`, `js_wrapped_export_refusal_test.rs` and
  `js_wrapped_export_state_test.rs` (each under 600 lines), registered in `tests/integration/main.rs`;
- the CLI counters in `tests/cli/call_stats_test.rs`;
- nav in `tests/navigation/callers_test.rs`.

Use table-driven inputs, and assert exact `(file, name, start_line, end_line)` targets, with decoys.

**Positive tests.** T-P1–T-P5 run in JS (`.jsx`) and TSX.

| ID | Input | Expected |
|---|---|---|
| T-P1 | named `forwardRef((p, r) => …)`, plus `forwardRef<A, B>(…)` | Exact → the arrow span |
| T-P2 | `import React` / `import * as React` + `React.forwardRef` (C26, C27) | Exact |
| T-P3 | `memo(fn)`, `React.memo(fn)` | Exact |
| T-P4 | multi-line `memo(fn, cmp)`, comparator on a **different line** | Exact → the first-argument span only |
| T-P5 | `import { forwardRef as fr } from "react"` | Exact |
| T-P6 | `forwardRef(function IslandImpl(p, r) {…})` | Exact → `IslandImpl@span` |
| T-P7 | wrapped export + nested `function Island` elsewhere (C11) | Exact → the wrapper span only, exactly 1 target |
| T-P8 | depth-2 `export { Island } from` and `export *` (C14) | Exact through the chain |
| T-P9 | `export const A = forwardRef(…), B = memo(…)` (C17) | both Exact |
| T-P10 | comment argument `forwardRef(/* c */ (p, r) => …)` | Exact |
| T-P11 | benign React uses (C38): `React.useState`, `<React.Fragment>`, `React.ComponentProps`, `typeof React`, `Island.displayName = …` | Exact |

**JSX gate.**

| ID | Input | Expected |
|---|---|---|
| T-J1 | `Island(props)` from an importer (C31 `App2`) | `WrappedExportNonJsx`; `dropped_wrapped_export_non_jsx == 1` |
| T-J2 | `new (Island as any)()` next to `<Island/>` (C32) | no `new` row; the JSX row is Exact |
| T-J3 | a `jsx_element == false` site built through the `src/resolution.rs` test `site()` helper, against a `SpannedLocal` fact | `WrappedExportNonJsx` |
| T-J4 | a plain `Local` export called directly `A()` | unchanged Exact. **A base-green preservation control, not RED evidence** (sol r3 SMELL) |

**Negative tests (guards).**

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
| T-N9 | `import {X} from "./a"; export {X}` over a wrapped `X` | drop |
| T-N11 | single line: `export const X = forwardRef((p, r) => { function X() {} return null; });` | drop (2 or more span matches) |
| T-N12 | a parse error inside the export statement | drop (R3) |
| T-N13 | `export const X = memo(fn); export { other as X };` | drop (poisoned) |
| T-N14 | the pinned F4 test and the existing P4/P7 JS export suites | unchanged |
| T-N15 | one-line `memo((p) => …, (a, b) => true)` (C30) | drop (R11) |
| T-N16 | star-barrel span conflict (C39) | drop; `js_export_barrel_conflicts` +1 |
| T-N17 | `import { forwardRef as fr ??? } from "react"` (C29), plus a twin whose parse error is outside any import (which must stay Exact) | drop (R4) / Exact |
| **T-N18** | `const { forwardRef } = require("react"); export const Island = forwardRef(…)` (C56) | drop (R5 `callee_not_admitted`: the ESM-only table) |
| T-R6-P1 | `import { memo } from "react"; import { memo } from "./other"` | drop (R6) |
| **T-R6-P2** | sol's fixture `import { memo } from "react"; import type { T as memo } from "./types";` + `memo(fn)` (C55) | drop, **`callee_provenance`**. A second row keeps the old type-only-only input, which drops as R5 |
| T-R6-P3 | `memo = other;` after the import | drop (R6) |
| T-R6-P4 | top-level `function memo(){}`, `class memo {}`, `const { memo } = x` (C64–C66); positive twins: a component-local `const memo = 1` (C67), block `let memo` (C60), `var memo` inside a nested function or class (C61) | drop ×3 / Exact ×3 |
| **T-R6-P4b** | sol r3 W2: `if (flag) { var memo = fake; }` (C57); also `for (var memo of …)` (C58) and `try { for (var memo = 0; …) }` (C59) | exactly one `callee_provenance` each; drop |
| T-R6-P5 | escaped callee spelling `forw\u0061rdRef` | drop (R6) |

**Model-boundary characterization tests.** These assert **Exact**, and each carries a comment naming SPEC §3.2.

| ID | Input | Expected |
|---|---|---|
| MB1 | r1 W1: `import React`; `React.forwardRef = fake`; `React.forwardRef(fn)` (C28) | Exact → the wrapper span |
| MB2 | r2 W1: `import { forwardRef }`; `require("react").forwardRef = fake`; `forwardRef(fn)` (C53) | Exact → the wrapper span |
| MB3 | r2 W3: `import React`; `React.__defineGetter__("forwardRef", () => fake)`; `React.forwardRef(fn)` (C54) | Exact → the wrapper span |

**Observability, state, cache and nav.**

| ID | Test | Expected |
|---|---|---|
| T-O1 | one fixture per reachable reason (R1–R6, R8–R11) | each reason count is 1; `skipped_expr_count` +1; the admitted fixture gives `spanned_admitted == 1`; R12 is listed as unreachable |
| T-O2 | CLI `call-stats` on a temp fixture | the three new keys are present with exact values |
| T-O3 | a mixed-declarator file: admitted, R5, R1, R2, R11, plus `export default someCall()` | `skipped_expr_count == sum(reasons) + 1`, with exact per-reason counts |
| T-S1 | epochs (full **and** incremental, JS/TS/TSX): admitted → import changed to `"./shim"` (refused) → restored → inner arrow moved two lines (span changes) → restored. r2's `React.forwardRef = fake` epoch is removed, because it is no longer a state change under §3.2 | expected edges every epoch; full == incremental |
| T-C1 | `serde_json` round-trip of `JsExportFacts` (with `SpannedLocal` and reasons) and of a `CallSite` with `jsx_element = true`; a CPG cache full hit that keeps the edge | equal |
| T-C2 | pin tests | 98 and 54 |
| T-V1 | nav `callers --symbol Island --file lib.tsx` | 1 caller, `import_member`, score 1.0 |

**Mutants.** The implementer applies each alone and records the test that kills it. M10 (r2 custody) is dropped.

| # | Mutant | Must fail |
|---|---|---|
| M1 | drop the span filter | T-P7, T-P4 (corrected at implementation: T-N11's two same-line targets both pass the span filter, so it cannot kill M1) |
| M2 | accept any callee | T-N6, T-N3 |
| M3 | accept `let` | T-N5 |
| M4 | pick the last argument | T-P4 |
| M5 | demote a collision to NameOnly | T-N11 |
| M6 | stop counting a reason | T-O1 |
| M7 | skip the cache bump | T-C2 |
| M8 | drop the span from the barrel key | T-N16 (MEASURED P21: NameOnly ×2) |
| M9 | skip the JSX gate | T-J1, T-J3 |
| M11 | skip the import error guard | T-N17 |
| M12 | remove P2 | T-R6-P2 (MEASURED P27: C55 flips to Exact) |
| M13 | feed R5 from `extract_import_bindings()` without an origin check | T-N18 |
| M14 | restore a root-only competitor walk (no descent below root children) | T-R6-P4b on C57 and C59. MEASURED P32: both flip to Exact. C58 stays refused, because its `for (var …)` head is itself a root child |

**Tier-A fixtures** (`eval/fixtures/typescript/`, at most 3 new directories):
- `forwardref_named_export/` (C02): `exact = true`, `resolution_kind = "import_member"`. RED on base.
- `forwardref_ident_arg_nested_samename_refused/` (C10): `callers = []`, `forbid_resolution_kind = "import_member"`.
- `react_default_member_write_out_of_model/` (C28 = MB1): `exact = true`, `resolution_kind = "import_member"`. It
  replaces r2's `react_default_member_write_refused/`, and it is RED on base.

## 8. Acceptance

1. **Suites green.**
   - `cargo fmt --check`;
   - `cargo test --offline --no-fail-fast`: base 4,559 / 0 / 1 (P9); the Branch-P prototype with the r3 fold gives
     4,559 / 0 / 1 (P34); report the new total;
   - `cargo test --offline --features mcp`;
   - `cargo clippy --offline --all-targets --features mcp`;
   - Tier-A `--matrix-only`: base 159/159, Branch-P prototype with the r3 fold 159/159 (P33), plus the new fixtures,
     with 0 regressions;
   - the Node gate;
   - `tier-a --quick` if rust-analyzer is available; otherwise report it as not run.
2. **Yield (the acceptance criterion; unchanged across r1, r2 and P).** Run the release binary with
   `nav --no-cache call-stats --dump-sites` on each corpus, then `probes/rowdiff.py` against the base dumps.
   - **X:** exactly the 107 rows of `probes/P6-excalidraw-expected-rowdiff.json` (SHA-256 `265790a3…`), all
     `UnknownName → Exact import_member`, with 0 other changes. The Branch-P prototype row-diff, with and without the
     r3 fold, is **byte-identical** to it (P26, P33). Audit 107/107. `dfg-stats --edges` byte-identical to base.
   - **F:** 4 rows, audited 4/4.
   - **R and T:** byte-identical to base (P26, P33).
   - Counters as in §5.

   Any deviation is a blocker until the owner accepts it, reported row by row.
3. **Custody.** Acceptance outputs go under `~/prism-evidence/wrapped-export/acceptance/` with a `MANIFEST.sha256`.

## 9. Budget (honest lines: after `rustfmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` are tests)

**MEASURED (P30), Branch-P prototype with the r3 W2 fold: 328 src / 17 test honest lines**
(`prototype/wrapped-export-prototype-P3.diff.txt`). Before the fold it was 325 (P26).

- **The fold's first version was over the cap.** A separate hoisted-`var` walk added 32 lines, for 357. That breaches
  the 350 cap before the cache bumps.
- **The fix is one walk, not compressed logic.** It was replaced by a single recursive `module_scope_declares` that
  covers both the top-level declarations and the hoisted `var`s (sol's "share one walk" alternative). The one walk is
  39 lines, replacing 36, and gives an identical result on all 67 controls and the corpora (P31, P33).

| File or function | Lines |
|---|---|
| `js_wrapped_export.rs`: predicate | 97 |
| `js_wrapped_export.rs`: React import table | 64 |
| `js_wrapped_export.rs`: module-scope competitor (one recursive walk) | 39 |
| `js_wrapped_export.rs`: header | 9 |
| `ast.rs` | 28 |
| `js_exports.rs` | 47 |
| `resolution.rs` | 25 |
| `call_graph.rs` | 17 |
| `queries.rs` | 15 |
| literal fix-ups | 2 |

Still to add: the cache bumps and notes (about 4 src). The `CLAUDE.md` paragraph is documentation, not honest code.
The forecast is **about 332 src**, which leaves 18 lines of headroom under 350.

| Bucket | Cap (D7) | Early stop (about 90%) |
|---|---|---|
| src | **350** | 315 |
| tests | **680** (was 600) | 650 (was 540) |
| combined | **1,010** (was 950) | – (was 855) |
| Tier-A fixture directories | 3 | – |

**Owner decision, 2026-09-25:** the test and combined caps were raised to 680 and 1,010 after the implementation
stopped at 698 test lines (D7).

**The early-stop semantics are a checkpoint, not a halt.** The measured prototype (328) already sits above the
src early stop (315). At an early stop the implementer reports the current count and the forecast to the controller,
and continues only while the forecast is within the cap. A forecast above a cap is a stop, with the enumerated
remaining items. Logic is never compressed to fit.

**Test estimate:** about 45 table rows at about 9 lines each (T-R6-P4b is table-driven), plus about 180 for helpers,
T-S1 and cache/nav: about 585 against the 600 cap. This is the tightest bucket, so report it at the 540 checkpoint.

## 10. Risks and open questions

- **The model (§3.2)** is owner-decided. A reviewer who disagrees argues D2 in the convergence view, with the edge
  cost, rather than raising findings.
- **A real same-file monkey-patch** of `React.forwardRef` or `React.memo` yields Exact to the render function. The r2
  custody, which refused far more than monkey-patches, fired on 0 of the 72 admitted declarators' files on X and F
  (P18).
- **The React rendering contract** is an ASSUMPTION, with the M7 precedent. The JSX gate confines Exact to the sites
  where it applies.
- **Longer term:** a distinct framework "renders" edge kind could separate JSX-to-render edges from ordinary calls.
  `jsx_element` is the seam for it.
- **Optional, not in this slice:** `REPLAN-fable.md` §2.1 also suggests a `docs/superpowers/pipeline-lessons.md`
  doctrine entry ("state the analysis model before the reviewer does"). It lands only if the controller directs it.

## 11. Review history

| Round | Verdict | Disposition |
|---|---|---|
| r1 (sol) | FIX 5 WRONG / 2 SMELL | all folded (`REVIEW-r1-fold.md`) |
| r2 (sol) | FIX 3 WRONG / 2 SMELL, open-class | **Branch P** (owner): W1–W3 are out of model and pinned as MB2, MB2's class, and MB3 (sol's W2 computed specifier belongs to MB2's class); SMELL 1 is folded as T-R6-P2 plus M12; SMELL 2 is folded as the ESM-only table plus T-N18 and M13. See `REPLAN-fable.md` §3 |
| r3 (sol, final) | FIX 2 WRONG / 1 SMELL, bounded and converging | **Folded at the cap (disclosed extension; owner D10–D11):** W1 (R3/R4 bypass the gates; pre-existing) is deferred to S1b, with S1's contract narrowed to the R4c route and S1b widened (§12). W2 (hoisted `var`) is folded as the P4 recursive walk, T-R6-P4b and M14, re-measured (P30–P33). The SMELL is folded (T-J4 is a preservation control). See `REVIEW-r3-fold.md` |
| impl r1 (sol + opus) | FIX: sol 1 WRONG; opus 3 WRONG / 2 SMELL | all folded: sibling `ERROR` imports (R4), lowercase JSX tags, opening-element coverage, R3 comment, RED values |
| impl r2 (opus + sol, final) | FIX 1 WRONG / 0 SMELL each | **Folded at the cap (disclosed):** converging, 4 → 2 WRONG; targeted fix (R4 import-token containment; exact R5 module token); narrow confirmation to follow |
| impl r2 follow-up (controller) | V6 in JSX open after the r2 fold | **Folded in the same finding:** the JS grammar recovers `import` as `identifier:"import"`; R4's token search also matches that spelling (reserved word, so erroneous parses only) |

## 12. S1b recorded scope: span-verify all JS/TS export routes (owner D4 + D10)

S1b is the next slice on this seam. It reuses `SpannedLocal`, `ResolvedJsExport.span` and `CallSite.jsx_element`
from S1, so it needs no new data model. Its scope:

1. **The export-list and default-identifier `Local` routes (D4).** `const f = <non-function>; export { f }` resolves
   a false Exact to a nested same-name function on base (C06). The same holds for `export default f`, and for a
   `let` export reassigned in the file (C21).
2. **R3 `ImportQualified` through a namespace import (sol r3 W1a).** `import * as Lib from "./lib"; <Lib.Island/>`
   returns every same-stem `Island` as Exact, including a nested decoy. For wrapped targets it should consult
   `js_ts_resolved_exports` and keep only the `SpannedLocal` span; non-JSX qualified calls should drop as
   `WrappedExportNonJsx`.
3. **R4 `LocalDef` in the producer file (sol r3 W1b).** A non-JSX `Island({})` binds Exact `local_def` to the wrapper's
   inner function. The same-file multi-target Exact `local_def` (C62 `helper`) belongs here too.

**First RED cases** (sol r3's concrete inputs, recorded as controls; S1 leaves both byte-identical to base, P31):

| Case | Input | Base and S1 result | S1b expectation |
|---|---|---|---|
| S1b-RED-1 (C62) | `lib.tsx`: `function helper() { function Island() {…} … }` + `export const Island = forwardRef(…)` (`@6-8`); `app.tsx`: `import * as Lib from './lib'; <Lib.Island/>` | 2× Exact `import_qualified`: `Island@3-3` (decoy) and `Island@6-8` | exactly 1 Exact → `Island@6-8` |
| S1b-RED-2 (C63) | `lib.tsx`: `export const Island = forwardRef(…)` (`@2-4`); `export function Host() { … Island({} as any, null as any); }` | Exact `local_def` → `Island@2-4` from a non-JSX site | drop `WrappedExportNonJsx` |

S1b must re-measure the yield on X and F (S1b's behavior changes are refusals and re-targeting). It plans its own
budget.

