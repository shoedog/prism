# S1b: span-verify every JS/TS export and local route

**Status:** revision **r2** (spec round 1 folded, `REVIEW-r1-fold.md`), with the owner's r1 answers in §0. Spec round
2 of 2 is next (sol + Opus in parallel). Nothing under `src/`, `tests/`, `eval/`, `Cargo.*` or `CLAUDE.md` changes on
this branch.

**Base:** `origin/main` `a6d853f5` (S1 merged). **Grounding:** `PLANNING-PROBES.md` (M-ids for mechanisms, Q-ids for
measurements; r2 measurements are Q24–Q35 on prototype v8). **Predecessor:** the S1 packet's §12, which recorded this
scope.

**Model (binding, `CLAUDE.md`):** Exact is a static-binding grade. Runtime mutation of module objects, `eval`, host
globals and `require`/`import()` re-acquisition are out of model for every rung. A `with` statement is static syntax
and is in model.

## 0. Owner decisions

> **Status after spec round 2 (the cap), 2026-09-26.** Both round-2 reviews
> (`~/prism-evidence/s1b/reviews/spec-r2-{opus,sol}.md`) found new *semantic* misses in kinds the table had
> classified: TS namespace merging and computed method names. Each yields a wrong Exact edge, which the B0 allowlist
> cannot catch. The controller classified the collector (S1b-2..4) as **open-class at the cap**. **Owner decision:**
> ship **S1b-1** (intrinsic guard plus shared collector fixes, §9) now; neither reviewer found a defect in it. The
> collector design for **S1b-2..4 is parked pending a Fable re-plan**, which also owns the bounded round-2 items
> (Opus W2 header errors, sol W2 string-token braces, sol W3 string-literal export names, the B0 leaf-kind SMELL).


Answered by the owner on 2026-09-26 after spec round 1, except where marked **open**.

| # | Decision | Answer | Basis (measured on v8 unless noted) |
|---|---|---|---|
| E1 | Slicing | Accepted: sub-slices, 2-round review cap each. **r2 re-split (open, E13)** | – |
| E2 | Lowercase intrinsic tags | **Include, one guard on every rung** | X 62 rows / 166 edges, F 103 rows, all wrong (Q29) |
| E3 | Drop reason | **`DropReason::JsxIntrinsic`** | relabels X 804, F 387, R 117, T 0 rows with no edge |
| E4 | `useCallback` | **Admit** under S1's ESM-`"react"` provenance, as a plain callable | X 11 / F 98 base edges kept (F 4 more fall to E6) |
| E5 | May-call rows | **Unchanged (base behavior).** A closed syntactic class (§3.1 M1, M2); recorded as a follow-up lane | X 40, F 56, T 163 rows stay base; 0 changed on any corpus (Q29) |
| E5b | **Owner 2026-09-26: (a) uniform**, after first choosing (b) and re-deciding once both reviewers' round-2 views were in. E5 on the export routes | (a) **apply E5 on every route** (C21, C77, C73 stay base); (b) refuse written or call-wrapped *exports* only | S1 §12 listed C21 as a D4 false Exact; one closed rule is simpler. 0 corpus rows either way. **Recommend (a)** |
| E6 | Parse recovery | **The narrower rule**, as a closed predicate (§3.1 B1) | Keeps F 7 and T 12 right rows that the broad rule refused, risks 0 (all 19 audited right); still refuses F 4 and T 41 right rows (Q32) |
| E6b | **Owner 2026-09-26: (a) no**, after first choosing (b) and re-deciding given Opus r2 W2 (header-error hole) and both reviewers' views. Also seal type-space containers (`interface`, `type`, type parameters/arguments/annotations) | (a) no (**recommended**); (b) yes | (b) recovers T's 41 (X, F, R 0). Angle-bracketed type nodes lack the brace guarantee B1's argument rests on (Q32) |
| E7 | Unresolvable namespace modules | **Split:** a caller-relative sibling differing only in a JS→TS extension is **Exact**; any other stem fallback is **NameOnly** | T's 24 rows stay Exact (sibling); F's 4 bare-specifier rows become NameOnly, edges kept (Q30) |
| E8–E12 | S2 qualifiers, imported-local parity, indirect sites, budget, review | Accepted as recommended; E11 superseded by E13 | – |
| E13 | **Owner 2026-09-26: (a) four sub-slices**, caps per §9. r2 split and caps | (a) **four sub-slices**: S1b-1 intrinsic + shared collector fixes; S1b-2 binding core at module scope + D4; S1b-3 nested scopes + lexical `LocalDef` + `useCallback`; S1b-4 R3 namespace; (b) keep three by merging S1b-1 into S1b-2 | v8 prototype 1,079 src lines (about 1,050 design code, Q34) against r1's 760 total. **Recommend (a)**, caps in §9 |

## 1. Problem

Prism turns a JS/TS name into callables by name lookup, and four routes do so without checking what the name
statically denotes (PLANNING-PROBES M1–M8):

1. **D4, the ESM local export routes** (export lists, `export default <ident>`, `export function`, function-valued
   exported declarators) record `Local(name)`, and R4c binds any same-file function of that name (C06, C68).
2. **R3 `ImportQualified`** returns every same-stem function of the member name, all Exact, and its qualifier guard
   misses shadowing forms (C62, C80, C128–C130).
3. **R4 `LocalDef`** binds every same-file free function of the callee name, all Exact: shadowing parameters,
   functions in other scopes, over-named object properties (C63, C86–C88; T 635 multi-target rows).
4. **JSX intrinsic tags** (`<input>`, `<label>`) bind on every rung (M8).

## 2. Scope

| Sub-slice | Contents |
|---|---|
| **S1b-1** | The JSX intrinsic guard (§3.5); `JsxIntrinsic` and `LocalBindingUnproven` drop reasons and counters; the three shared-collector fixes (§3.1 F1–F3) |
| **S1b-2** | The binding core (§3.1) evaluated at module scope; D4 export terminals (§3.2); R4c keyed on `wrapped` (§3.3) |
| **S1b-3** | The core at every scope (site walk, parameter scope, decorators, scoped write scan, memo); lexical `LocalDef` with `(name, span)` matching (§3.6); `useCallback` (§3.7) |
| **S1b-4** | R3 namespace qualifiers through the proven qualifier binding and the module's exports; E7 split (§3.4) |

**Non-goals** (each pinned by a test, §7): named-, default- and `require`-import qualifiers on R3, including their
multi-target rows (E8, S2); CommonJS `Local` production; `ImportForward`/`ReExport` mechanics other than the
`(span, wrapped)` barrel key; `IndirectResolution`, `MacroArg` and synthetic sites (E10); module resolution
(`./x.js` for `x.ts` outside S1b-4's sibling rule, tsconfig `paths`, workspaces); the may-call class (E5; a
follow-up lane); **generator function expressions** (not indexed by `function_node_types`, so a binding to one
refuses as `unindexed` and keeps base's no-edge result, C134); runtime mutation; non-JS languages;
`Language::function_name`, `FunctionId`, call-site ownership, `CallKind`, DFG construction.

## 3. Semantics

### 3.1 The binding core: an enumerated table

The core answers: what does name `n` denote at site `s`? It finds the **binding scope** (the nearest environment
enclosing `s` that declares `n`), its **declarations** of `n`, and classifies them. The table is keyed to ECMA-262
static semantics (VarScopedDeclarations, LexicallyScopedDeclarations, BoundNames; §10.2.11
FunctionDeclarationInstantiation; §14.2 blocks; §14.11 `with`; §14.15 `catch`; §15.7 classes and static blocks;
Annex B.3.3–B.3.4) and to TypeScript's declaration spaces (value vs type). The same table holds in the JavaScript,
TypeScript and TSX grammars; each row has a JSX and a TSX control (§4).

**Environments (Σ).** A node opens an environment iff its kind is in this list.

| # | Kind | Declares (ECMA-262) |
|---|---|---|
| T1 | `program` | Script/Module TopLevel{Var,Lexically}ScopedDeclarations: the statement list (D-rows), `var` hoisted from any depth (D7), imports (D5) |
| T2 | function-like: `function_declaration`, `generator_function_declaration`, `function_expression`, `generator_function`, `arrow_function`, `method_definition` | FunctionDeclarationInstantiation: BoundNames of the formal parameters (both the `parameters` list and the single-arrow `parameter` field), the implicit `arguments` of a non-arrow function (non-callable), a named function expression's own name (funcEnv), and the body's var- and lexically scoped declarations |
| T3 | `formal_parameters` | the parameter environment when parameter expressions exist: the parameters, `arguments` (non-arrow), a named function expression's own name; **not** the body's declarations. The walk resumes above the function (J1) |
| T4 | `statement_block` not directly a function body | BlockDeclarationInstantiation: lexical declarations. When its parent is `class_static_block` (ClassStaticBlockBody) or `internal_module`/`module` (a TS namespace body, emitted as a function), it is **also a var scope**: `var`s hoist to it and no further |
| T5 | `for_statement`, `for_in_statement` (covers `for…of`) | a `let`/`const` head |
| T6 | `catch_clause` | BoundNames of the catch parameter |
| T7 | `switch_body` | CaseBlock lexical declarations |
| T8 | `class_declaration`, `class`, `abstract_class_declaration` | the class's inner name binding |
| T9 | `with_statement` | an object environment: **every** name (refuse) |

**Walk jumps.** J1: from `formal_parameters`, continue above its function. J2: from a `decorator`, continue above
the enclosing class (decorator expressions are evaluated where the class is defined).

**Declarations in a scope** are collected by one walk over its statement list. The walk stays **direct** through the
transparent wrappers `export_statement`, `expression_statement`, `labeled_statement`, `switch_case`,
`switch_default`, `ambient_declaration`, `lexical_declaration` and `variable_declaration`; any other statement makes
its subtree indirect. It stops at function-like nodes, classes, `class_static_block`, `internal_module`/`module`,
`enum_declaration` and `decorator`.

| # | Form | Rule |
|---|---|---|
| D1 | `function_declaration`, `generator_function_declaration` | direct: a declaration of its name. Reached indirectly by a hoisting walk: an **Annex B.3.3 marker** (the enclosing statement), non-callable, because sloppy code may var-hoist a block function |
| D2 | `class_declaration`, `abstract_class_declaration` | direct: a declaration (non-callable) |
| D3 | `labeled_statement` | transparent (a labelled function declaration is a declaration of its surrounding list) |
| D4 | `lexical_declaration` declarators; TS `using` / `await using` (an `expression_statement` holding an `assignment_expression` with an anonymous `using` token) | direct only |
| D5 | `import_statement` (default, namespace, named, `import x = require()`), `import_alias` (`import x = N.y`: its first named child) | direct; **value space only**: an `import type` statement and `type` specifiers declare nothing |
| D6 | TS: `enum_declaration`, `internal_module`/`module` (identifier name, first segment of a dotted name), `function_signature` inside `ambient_declaration`, ambient `var`/`let`/`const`/`class` | direct (value space). `interface_declaration`, `type_alias_declaration` and an overload `function_signature` outside `ambient_declaration` declare nothing |
| D7 | `var` declarators and `for (var … in/of …)` heads | at any depth while hoisting (T1, T2 bodies, T4 var scopes). A `var` declarator reached through a `catch` clause whose parameter binds the same name, or through a `with` body, records that body as a **marker** (Annex B.3.4: its initializer assigns the catch parameter; under `with`, maybe the object) |
| P1 | binding patterns | BoundNames: `identifier`, `shorthand_property_identifier_pattern`, `object_pattern`, `array_pattern`, `rest_pattern`, `pair_pattern` (value), `assignment_pattern` (left), **`object_assignment_pattern` (left)**, `required_parameter`/`optional_parameter` (pattern) |
| W1 | writes | `assignment_expression` (not a `using` declaration), `augmented_assignment_expression`, `update_expression`, and a `for … in/of` head with no declaration keyword. Target names come from P1 **plus** `parenthesized_expression`, `as_expression`, `satisfies_expression`, `non_null_expression` and `type_assertion`. A write belongs to the scope the site walk finds from the target; a write that resolves to a `with` counts for every scope |

**Shared-collector fixes** (S1b-1; base helpers used by S1, the CJS proof and the shadow guards): **F1** the write-target
unwraps in W1; **F2** `object_assignment_pattern` in P1; **F3** the single-arrow `parameter` in the parameter
collector. Blast radius on base (Q27): X 0, F 7, R 0, T 2 rows, each an R5 edge from a site whose callee is its own
enclosing arrow's parameter, removed (wrong).

**Classification**, in order; the first rule that applies decides:

| # | Outcome | Rule |
|---|---|---|
| B0 | refuse `unclassified_kind` / `escaped_identifier` | **fail-safe:** the binding scope's subtree holds a non-leaf named node whose kind is not in the classified list (the table's kinds plus the grammar-derived inert kinds; `probes/grammar_closure.py`), or an identifier spelled with an escape (`\u…`), which raw-spelling comparison cannot match |
| B1 | refuse `parse_recovery` | **E6, narrower rule.** The binding scope has an `ERROR`/`MISSING` node and it is **not** the case that (i) the file has no `MISSING` brace and no `ERROR` node containing a brace, and (ii) every error node in the scope lies inside a braced function, a class or a static block that is strictly inside the scope and does not contain the site |
| – | refuse `with` | the binding scope is a `with_statement` |
| B2 | refuse `duplicate_declaration` | two or more declarations (markers count) |
| M2 | **may-call (base behavior, E5)** | the single declaration is a function declaration, the scope's named function expression, a declarator or a `using` declaration, and some write (W1) resolves to this scope |
| B3 | callable | a function declaration or named function expression (the node itself); a declarator or `using` whose value, after unwrapping parentheses, TS assertions and assignment chains (`(M.f = function(){})`), is an `arrow_function` or `function_expression`; a `const` declarator whose value is an admitted React wrapper (S1's predicate: `forwardRef`/`memo`, wrapped; `useCallback` under E4, plain) |
| M1 | **may-call (base behavior, E5)** | a declarator or `using` whose unwrapped value is a `call_expression` with a direct `arrow_function`/`function_expression` argument that S1's predicate does not admit (a Pattern-3 over-named argument: `throttle(fn)`, `lazy(fn)`, `styled(…)(fn)`, `memoize(fn)`) |
| – | refuse `not_callable` | anything else (a parameter, catch parameter, class, import, enum, namespace, marker, a call with no function argument, any other value) |
| – | refuse `unindexed` | the callable has no registered name (for example a generator expression) |
| – | refuse `unbound` | no scope declares the name |

The terminal is `(local, start_line, end_line, wrapped)`: `local` is the callable's registered name
(`Language::function_name`), the lines are its `FunctionId` span, and `wrapped` is true for `forwardRef`/`memo`.

**Why B1 is closed.** Brace tokens outside `ERROR` nodes are paired by the parse, and (i) excludes a missing brace or a
brace inside an error, so every braced node's extent is its true extent (ASSUMPTION: tokenization outside error
nodes is correct). An error inside a sealed node cannot move text across its boundary, and whatever a function,
class or static block declares is invisible outside it. The site is outside every such node, so its walk and every
declaration visible to it are outside the errors.

**Why B0 is a fail-safe, not a list of hopes.** `probes/grammar_closure.py` reads the pinned grammars' `node-types.json`
and marks every named kind that can hold a statement, declaration, pattern, parameter list or binding identifier
field as **suspect**; each suspect kind must appear in the table above (as a scope, declaration, wrapper, pattern,
write form, reference-only or type-space kind). It checks this and exits non-zero otherwise (0 unclassified of 76
suspect kinds; 186 concrete kinds). All other non-leaf kinds are mechanically inert. The runtime allowlist is the
union, so a grammar upgrade that adds a kind refuses until the kind is classified. Measured cost: 0 rows on all four
corpora (Q28).

### 3.2 D4: module-scope terminals for the ESM export routes (S1b-2)

For each ESM local export occurrence (list without `source`, default identifier, `export [default] function`,
exported arrow or function-expression declarator), in order:
1. `js_ts_forwarded_import` as on base.
2. An imported name keeps base's `Local(name)`, which base then poisons (E9).
3. Otherwise classify the module-scope binding (site = the export occurrence), with the base module write scan after
   F1–F3 (identical to the scoped scan on all four corpora and all controls, Q28):
   - callable, not wrapped → `JsExportTarget::VerifiedLocal { local, start_line, end_line }`;
   - callable, wrapped → S1's `SpannedLocal`;
   - may-call → base `Local(name)` (E5, E5b a);
   - refused → `UnprovenLocal(name)` and `local_export_refusals[reason] += 1`.

S1's declarator arm is unchanged. Data model as r1: `VerifiedLocal`; `ResolvedJsExport.wrapped`
(`#[serde(default)]`); barrel candidate key `(file, local, is_class, span, wrapped)`;
`JsExportFacts.local_export_refusals`.

### 3.3 R4c (S1b-2)

`WrappedExportNonJsx` iff `resolved.wrapped && !site.jsx_element`; filter by span whenever it is `Some`; 0 or 2+
span matches yield no candidate. S1's R4c-only lowercase check is deleted (§3.5 covers every rung).

### 3.4 R3 namespace qualifiers (S1b-4)

Applies when the caller is JS/TS and the qualifier `q` has exactly one import binding in the file, a `ModuleImport`
(an ESM `import * as q`).
- **Qualifier proof.** At extraction, the site's walk from the qualifier identifier must find the program scope with
  exactly one declaration of `q`, that `import_statement`, and B0, B1 and "no write to `q`" must hold. The result is
  stored on the call site (`JsLocalBinding::NamespaceImport` / `Unproven`). An unproven qualifier (a parameter,
  `with`, a written namespace, a shadowing declaration) takes **no** R3 route and falls to the later rungs.
- **Module.** The specifier resolves relatively (base resolver), or, when it ends in `.js`/`.jsx`/`.mjs`/`.cjs`, the
  caller-relative sibling with the TypeScript extension (`.ts`/`.tsx`, `.tsx`, `.mts`, `.cts`) is indexed (E7).
  Then the member resolves through the module's export facts (`js_ts_export_candidates`, shared with R4c): 1 →
  Exact; 2+ → NameOnly; `Err` → that drop; 0 → `UnknownName` if no function of that name exists, else
  `ImportExternal` (base's reasons).
- **Otherwise (E7):** base's stem candidates, filtered to functions their own file exports under the member name;
  a wrapped one from a non-JSX site → `WrappedExportNonJsx`; 1+ → **NameOnly**; 0 → base's reasons.

### 3.5 JSX intrinsic guard (S1b-1)

First statement of `resolve_call_site_full`: a JSX element site with no qualifier whose callee starts with an ASCII
lowercase letter or contains `-` or `:` drops `DropReason::JsxIntrinsic` (TypeScript `isIntrinsicJsxName`, Babel
`isCompatTag`). Member tags and `_x`/`$x` tags are not intrinsic.

### 3.6 R4 by lexical binding (S1b-3)

`CallSite.local_binding: JsLocalBinding` (`#[serde(default)]`, excluded from `cmp_key`), set at the three source
constructors for JS/TS sites whose callee is a plain identifier (call `function` field or JSX `name`):
`Callable(JsTerminal)`, `MayCall`, `Unproven`, or `Unchecked` (default: other languages, synthetic and indirect
sites, non-identifier callees). Memoized per file by scope (declaration index, write index, hygiene).

Resolution, first thing in the unqualified branch: `Callable(t)` → if `t.wrapped` and the site is not JSX,
`WrappedExportNonJsx`; else the same-file non-method functions named `t.local` with `t`'s span: exactly 1 → Exact
`LocalDef` (this can bind a callable whose name differs from the callee, W4 option a); otherwise drop
(`LocalBindingUnproven` if a same-file function has the callee's name, else `UnknownName`). At base's R4 point, with
same-file candidates: `Unproven` → `LocalBindingUnproven` (never falls to R5); `MayCall`/`Unchecked` → base.

### 3.7 `useCallback` (S1b-3, E4)

S1's predicate gains a `local_route` flag; on the binding route (not the export arm) it also admits `useCallback`
(arity 1–2, same ESM-`"react"` provenance and R6 checks), as a plain callable.

## 4. Controls (MEASURED, v8; `probes/S1b-controls-*.txt`, expectations in `probes/S1b-controls-expectations-pre-run.md`)

218 scenarios; every S1b scenario runs in the JSX (`.jsx`/`.js`) and TSX (`.tsx`/`.ts`) grammar, except the TS-only
C102–C104, C114, C117, C141–C148. All results match the pre-registered expectations after addenda 4–6 (one prediction
was falsified in detail and recorded before the rerun: C151, addendum 6).

| Control (table row) | Rule | Sub-slice | base → S1b |
|---|---|---|---|
| C06, C68 ternary list/default export + nested `f` | B3 | 2 | Exact to the decoy → drop |
| C21, C77, C139 written export/function; C73 list-exported `create(fn)` | M2 / M1 (E5, E5b a) | 2 | unchanged |
| C69–C71 function/arrow/list export + nested decoy | span | 2 | NameOnly ×2 → Exact ×1 |
| C72 list-exported `forwardRef`: `Island({})`, `<Island/>` | §3.3 | 2 | → `WrappedExportNonJsx`, Exact |
| C74, C75 duplicate `var`; hoisted competitor | B2 | 2 | → drop |
| C78 `const f = function g(){}` exported | span | 2 | drop → Exact `g` |
| C79 error with braces elsewhere in the exporting file | B1 (i) | 2 | Exact → drop |
| C126 (sol W2) exported `f` plus `const g = f => { f = 2 }` | F3 | 1 | Exact, unchanged |
| C146, C143, C144 (TS) interface/type/`import type` next to a value | D5, D6 | 2/3 | unchanged (Exact) |
| C96, C97 intrinsic tags | §3.5 | 1 | → `JsxIntrinsic` |
| C98 `<lib.island/>` | §3.5 | 1 | unchanged |
| C63, C106 non-JSX call of a same-file wrapped component | §3.6 | 3 | → `WrappedExportNonJsx` |
| C11/C62/C69–C71/C80/C81/C85 producers, C86, C110 | T2, T3 | 3 | Exact ×2 → Exact ×1 |
| C87, C101 shadowing parameter (list and single arrow) | T2 | 3 | → drop |
| C88, C113, C121 (Opus W7, sol W4) global / implicit `arguments` | unbound / T2, T3 | 3 | → drop (the module-scope arrow keeps Exact) |
| C89 `useCallback` | §3.7 | 3 | unchanged (Exact) |
| C90, C107, C138 `throttle`/`lazy`/decoy | M1 | 3 | unchanged |
| C91, C125, C142 written bindings (including destructuring and TS-assertion writes) | M2 | 3 | unchanged |
| C92 catch / for-of / class / `with` shadows | T5, T6, T8, T9 | 3 | → drop (the try-block call keeps Exact) |
| C93, C111 block function inside / outside; Annex B | D1 | 3 | inside Exact; outside drop |
| C94 hoisted `var` arrow | D7 | 3 | unchanged |
| C95 same-line collision | span | 3 | → drop |
| C99, C127 recursion, fe self-call, labelled function (sol W5) | D3, T2 | 3 | unchanged |
| C102, C103, C147 (TS) overloads, `declare function`, `declare const` | D6 | 3 | unchanged; drop; drop |
| C104 (TS) block `enum`; `function N` + `namespace N` | D6, B2 | 3 | → drop (the second is a fail-closed recall loss) |
| C105, C108, C109, C151 errors that reach the site or its scope's list | B1 (ii) | 3 | → drop |
| C149, C150 an error sealed in another function | B1 (ii) | 2 | unchanged (Exact) |
| C114 (TS) `using` | D4 | 3 | the using-bound call → drop; the other call unchanged |
| C115, C117 (Opus W1) static-block and namespace `var` arrows | T4 | 3 | Exact ×2 → Exact → the inner arrow |
| C116 (sol W3) static-block non-callable `var` | T4 | 3 | → drop |
| C118 (Opus W4) `const f = function g(){}` next to `o.f` on one line | §3.6 | 3 | Exact → `o.f` → Exact → `g` |
| C119, C120 (Opus W5) `var` under `catch (f)` / `with` | D7 | 3 | → drop |
| C122 (Opus W8) parameter visible in a sibling default | T3 | 3 | → drop |
| C123 (Opus W9) assignment-chain value | B3 | 3 | unchanged (Exact) |
| C124 (sol W1) destructuring-default parameter | P1 | 3 | → drop |
| C134 (sol S8) generator expression | non-goal | 3 | unchanged (no edge) |
| C135 switch-case lexical | T7 | 3 | Exact ×2 → Exact ×1 each |
| C136 `for (var f in o)` | D7 | 3 | → drop |
| C137 escaped identifier | B0 | 3 | → drop |
| C140 method parameter vs sibling method | T2 | 3 | unchanged |
| C141 (Opus W2) `import f = M.g` | D5 | 3 | → drop |
| C145 (sol W4) parameter decorator | J2 | 3 | unchanged (Exact) |
| C62, C80, C148 namespace + decoy (relative, `.js` sibling) | §3.4 | 4 | Exact ×2 → Exact ×1 |
| C81 `./lib.js` + decoy | §3.4 | 4 | Exact ×2 → Exact ×1 |
| C82 namespace wrapped call / JSX | §3.4 | 4 | → `WrappedExportNonJsx`, Exact |
| C83 namespace re-export rename | §3.4 | 4 | drop → Exact |
| C128, C129, C130 (sol W6) parameter / `with` / written qualifier | §3.4 proof | 4 | Exact → no R3 edge |
| C131, C132 (E7) bare / non-sibling stem | §3.4 | 4 | Exact → NameOnly |
| C133 (E7) sibling without the member | §3.4 | 4 | Exact to another directory → drop |
| C84, C85, C112 non-goal qualifiers | E8 | – | unchanged |

## 5. Counters

- `dropped_jsx_intrinsic`, `dropped_local_binding_unproven` (new drop reasons, exhaustive match);
  `dropped_wrapped_export_non_jsx` now also counts R3 and R4 sites; `js_export_local_refusals` (sorted object).
- An implementation adds `js_export_local_may_call` (a count of export occurrences kept at base by E5), so the E5
  class is observable; the prototype does not have it.
- Expected (v8):

| Corpus | `dropped_jsx_intrinsic` (S1b-1) | `js_export_local_refusals` (S1b-2) | `dropped_local_binding_unproven` (S1b-3) | `unresolved_unknown_name` | `multi_target_exact_sites` | `local_def` Exact edges |
|---|---|---|---|---|---|---|
| X | 866 | `{not_callable: 6}` | 19 | 10,922 → 10,118 | 158 → 20 | 2,309 → 2,147 |
| F | 490 | `{not_callable: 28, parse_recovery: 2}` | 34 | 11,399 → 11,019 | 5 → 3 | 794 → 749 |
| R | 117 | `{}` | 0 | 721 → 604 | 0 → 0 | 96 → 96 |
| T | 0 | `{parse_recovery: 26}` | 223 | 28,736 → 28,728 | 680 → 15 | 20,230 → 18,156 |

`multi_target_exact_sites` keeps E8's R3 rows (X 6) and may-call rows (X 14).

## 6. Cache

Each sub-slice bumps both caches from the landed values: S1b-1 99 / 55 (a new drop reason changes resolution-derived
edges); S1b-2 100 / 56 (export facts); S1b-3 101 / 57 (`CallSite.local_binding`); S1b-4 102 / 58 (qualifier binding
and R3 edges). Update the pins.

## 7. Test plan

**RED rule.** Every new path has a behavioral test that fails on the sub-slice's base with a recorded concrete value.
Compile, setup and zero-test failures are inadmissible. Guards are marked and never cited as RED.

**Location.** `tests/integration/js_binding_*_test.rs` (table-driven, both grammars per row, exact
`(file, name, start, end)` targets with decoys), each under 600 lines; counters in `tests/cli/call_stats_test.rs`;
nav in `tests/navigation/callers_test.rs`. **Every row of the §3.1 table has at least one test row per grammar**
(the auditor shares the planner's model, so these pinned rows are the check on the rules).

**Existing tests to update** (Q35; representation or reason changes only): S1b-1 `t_j2_t_j4_jsx_gate_scope`
(`UnknownName` → `JsxIntrinsic`); S1b-2 the five `js_export_test::extract_*` tests, `indirect_default_class_facts_…`
and `cjs_refusal_raw_serde_and_esm_custody` (`Local` → `VerifiedLocal`); S1b-4 `module_binding_audit_test::
esm_namespace_import` (Gap → Supported).

| Sub-slice | Test rows (control ids) | Mutants (each alone; record the killing test) |
|---|---|---|
| S1b-1 | A-1 C96/C97 (self-closing, opening, `-`, `:`); A-2 C98, `_x`, `$x` (guard); A-3 C126, C124-param and a TS assertion write through the base scan (F1–F3); A-4 counters; A-5 pins | A-M1 no guard; A-M2 guard on member tags; A-M3 each of F1, F2, F3 reverted |
| S1b-2 | B-1 C06, C68; B-2 C69–C71; B-3 C72; B-4 C74, C75; B-5 C78; B-6 C79, C149, C150; B-7 C21, C77, C73, C139 (E5 guards); B-8 C143, C144, C146; B-9 C126; B-10 grammar-closure unit test (every kind the probe lists is in the allowlist); B-11 serde + barrel key; B-12 full vs incremental epochs; B-13 pins | B-M1 record `Local`; B-M2 skip B2; B-M3 broad B1; B-M4 drop B1 (i); B-M5 treat `interface`/`type` as declarations; B-M6 key R4c's gate on `span`; B-M7 drop `wrapped` from the barrel key; B-M8 remove one allowlist kind (refusal appears) |
| S1b-3 | C-1 C63, C106; C-2 C86, C62/C11 producers, C135; C-3 C87, C101, C124, C92, C136, C104, C103, C147; C-4 C88, C113, C121; C-5 C89 + impostor twins; C-6 C90, C107, C138, C91, C125, C142 (E5 guards); C-7 C93, C111, C127, C94, C99, C102; C-8 C95; C-9 C105, C108, C109, C151; C-10 C110, **C122 (B-10b)**; C-11 C115, C116, C117; C-12 C118; C-13 C119, C120; C-14 C114; C-15 C137; C-16 C141; C-17 C145, C140; C-18 C134 (non-goal pin); C-19 memo keyed by scope and name; C-20 indirect/qualified sites unchanged; C-21 epochs; C-22 serde; C-23 pins | C-M1 ignore `local_binding`; **C-M2a** no `formal_parameters` scope (killed by C122); **C-M2b** no J1 jump (killed by C110); C-M3 no Annex-B marker; C-M4 no single-arrow `parameter`; C-M5 `with` not a scope; C-M6 no catch parameter; C-M7 no implicit `arguments`; C-M8 no D7 taint; C-M9 static blocks not var scopes; C-M10 no J2; C-M11 span-only match; C-M12 fall to R5 on `Unproven`; C-M13 admit `useCallback` without provenance; C-M14 M1 without the function-argument requirement |
| S1b-4 | D-1 C62, C80, C81, C148; D-2 C82; D-3 C83; D-4 C128–C130; D-5 C131–C133; D-6 C84, C85, C112 guards; D-7 nav callers for C62; D-8 pins | D-M1 stem lookup for resolving modules; D-M2 skip the qualifier proof; D-M3 Exact for non-sibling stems; D-M4 namespace rule on named imports |

**Tier-A fixtures** (`eval/fixtures/typescript/`, each RED on base and green on v8, Q33): S1b-1
`s1b_jsx_intrinsic_tag_refused`; S1b-2 `s1b_list_export_nested_decoy_refused`; S1b-3
`s1b_param_shadow_local_def_refused`; S1b-4 `s1b_namespace_nested_decoy_refused`.

## 8. Acceptance

1. **Suites.** `cargo fmt --check`; `cargo test --offline --no-fail-fast` (base 4,583 / 0 / 1); `--features mcp`;
   clippy; Tier-A `--matrix-only` (base 162 / 162; v8 166 / 166 with the four fixtures) with 0 regressions; the Node
   gate; `tier-a --quick` if rust-analyzer is available.
2. **Exact audited row-diff per sub-slice** (`probes/rowdiff.py` against the previous sub-slice's dumps; the expected
   files are `probes/expected/S1b-{1,3,4}-{X,R,T}.json`; F's are private, by hash in the evidence root):

| Sub-slice | X | F | R | T |
|---|---|---|---|---|
| S1b-1 | 866 (804 relabels, 62 wrong intrinsic removals) | 497 (387, 103, 7 shared-fix removals) | 117 relabels | 2 shared-fix removals |
| S1b-2 | 0 | 0 | 0 | 0 |
| S1b-3 | 153 (134 re-targeted, 19 wrong removed) | 35 (1, 30 wrong, 4 parse-recovery right refused) | 0 | 868 (635, 182 wrong, 41 parse-recovery right refused, 10 right added) |
| S1b-4 | 0 | 4 (Exact → NameOnly, edge kept) | 0 | 0 |

   No row may change in the may-call class (E5): `audit_rowdiff.py` reports it as `maycall_changed`, which must be 0.
   Any deviation is a blocker until the owner accepts it, reported row by row.
3. **Custody.** `~/prism-evidence/s1b/acceptance/<sub-slice>/` with a `MANIFEST.sha256`.

## 9. Budget (honest lines: after `rustfmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` are tests)

**MEASURED (Q34):** prototype v8 is **1,079 src** lines, of which about 30 are measurement-only switches, so about
**1,050** of design code. r1's caps (300 / 340 / 120 src) cannot hold it: the r1 fold added the enumerated table,
the fail-safe allowlist (32 lines of grammar-derived data), the sealed-error rule, may-call classification,
`(name, span)` matching and the qualifier proof. Attribution by function:

| Sub-slice | Prototype src (design) | Main parts |
|---|---|---|
| S1b-1 | about 37 | intrinsic guard 10, drop reasons and counters 12, F1–F3 15 |
| S1b-2 | about 560 | allowlist and types 60, declaration walk 127, imports and `using` 48, classification 93, hygiene 29, sealed-error rule 48, module index 20, export target 48, `js_exports` 30, producers and R4c 15, sealing and helpers 15 |
| S1b-3 | about 297 | scope walk, jumps and remaining scope arms 125, scoped write index 44, call-site field and extraction 70, R4 resolution 40, `useCallback` 10 |
| S1b-4 | about 167 | R3 routing, stem refactor, E7 and export identity 120, qualifier proof and extraction 47 |

**Proposed caps (E13, open):** about the prototype plus 10% for src; tests from S1's measured ratio and the table
rows.

| Sub-slice | src cap / early stop | tests cap / report point | combined |
|---|---|---|---|
| S1b-1 | **60** / 54 | **200** / 180 | **260** |
| S1b-2 | **620** / 560 | **750** / 675 | **1,370** |
| S1b-3 | **330** / 300 | **600** / 540 | **930** |
| S1b-4 | **185** / 165 | **330** / 300 | **515** |

S1b-2 is about 1.8 times S1's src. It is the collector itself; it has no measured corpus row change (0 on all four),
so its review surface is the table, the controls and the closure probe. **`src/ast/js_binding.rs` is about 900
physical lines in the prototype:** the implementation splits it (for example the declaration walk and the checks)
to stay under 600 lines per file.

## 10. Risks

- **The collector's closure** remains the review surface. The table is keyed to the specification, every suspect
  grammar kind is classified by a checked probe, and B0 turns any unclassified kind into a refusal; the residual
  assumption is B1's (correct tokenization outside error nodes).
- **E5 leaves known wrong Exact edges** (subscription returns, loaders, styled callbacks, and multi-target may-call
  rows: X 14 multi-target rows stay). They are recorded for the may-call follow-up lane.
- **Grammar gaps** still cost right edges under E6 (F 4, T 41; E6b would recover T's 41).
- **Nav correlation for R3 additions** (C83) as r1 §10.

## 11. Review history

| Round | Verdict | Disposition |
|---|---|---|
| spec r1 (Opus) | FIX 9 WRONG / 4 SMELL | all folded (`REVIEW-r1-fold.md`); structural fold per both reviewers' convergence advice |
| spec r1 (sol) | FIX 6 WRONG / 2 SMELL | all folded; W6 in S1b-4; S8 a pinned non-goal |
| spec r2 | – | – |
