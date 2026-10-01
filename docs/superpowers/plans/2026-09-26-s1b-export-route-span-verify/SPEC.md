# S1b: span-verify every JS/TS export and local route

**Status:** revision **r4** (2026-09-27; S1b-3 spec round 1 folded 2026-09-29, `REVIEW-s1b3-r1-fold.md`): **S1b-3
re-planned against the landed code** (main `761541c2` with S1b-1, S1b-1b and S1b-2a; S1b-2b approved at `3961cc21`),
prototype **v12** (clean build of scratch `0968ef78`; §3.8, PLANNING-PROBES Q49–Q75; spec round 2 folded at the cap, `REVIEW-s1b3-r2-fold.md`). r3 (`REVIEW-r3-fold.md`)
remains the design of record for everything §3.8 does not change. Earlier folds: `REVIEW-r1-fold.md`,
`REPLAN-fable.md`.

**Base:** `origin/main` `a6d853f5` (S1 merged). **Grounding:** `PLANNING-PROBES.md` (M-ids for mechanisms, Q-ids for
measurements; r2 measurements are Q24–Q35 on prototype v8). **Predecessor:** the S1 packet's §12, which recorded this
scope.

**Model (binding, `CLAUDE.md`):** Exact is a static-binding grade. Runtime mutation of module objects, `eval`, host
globals and `require`/`import()` re-acquisition are out of model for every rung. A `with` statement is static syntax
and is in model.

## 0. Owner decisions

> **READ — S1b-4 planning brief, 2026-10-01 (authoritative constraints):** base is
> main `915fca43d84ea1730959453091fbf8ae97763af8`; no numeric LOC caps; review cap
> **2 rounds**. E5 preserves may-call and every written binding; OQ12's
> conservative cut, accepted parameter value-flow cost, Option K and E6 remain
> in force. S1b-4 cache is **104 / 60**, including the cross-commit row.
> **OPEN:** `OQ-S1b4.md` records alias-export policy and the unavailable Git-write
> capability. The S1b-4 draft is not dispatchable until these are settled and
> the prototype measurements replace the pending §8 row.

> **Owner cap amendments for S1b-3a (authoritative):**
> - **2026-09-29:** after the first implementation measured 828 tests, the owner approved de-duplicating the test
>   scaffolding, adding a C-M41 killing row, and raising **the tests cap to 820** (src stays 700).
> - **2026-09-30:** after the round-1 review found items A–G, the owner raised **the tests cap to 920** for the fold.
> - **2026-09-30 (final fold):** tests cap raised to **950** for the at-cap test-only fold (reverse-direction B rows, C-35 full equality, parameter-decorator row).
> - **2026-09-30 (later): the owner dropped numeric LOC caps entirely (src and tests, every S1b sub-slice).** Slice size
>   is a planning forecast only; split slices that would not converge in 2 review rounds. All cap figures in this
>   SPEC (§0 caps, §9) are historical.

> **Owner decisions 2026-09-29, after S1b-3 spec round 2 of 2 (the cap; disclosed at-cap fold,
> `REVIEW-s1b3-r2-fold.md`). Authoritative.**
> 1. **Caps approved:** 3a src **700** / tests **740**; 3b src **220** / tests **1,320** (report point **1,190**). The
>    C-5/C-6 E5 guards move to 3a as unit rows.
> 2. **Conservative cut, no further precision.** A destructuring declarator whose pattern holds a default
>    (`assignment_pattern`, `object_assignment_pattern`) anywhere is **Alias** (keeps base), whatever its value
>    (sol r2 W1: `const { missing: x = fallback } = {}`); NoFn stays the minimal literal class. The recovered-import
>    check reads the first two **non-comment tokens** (sol r2 W2: `import /* c */ . meta`).
> 3. **Parameters keep dropping, with the real cost recorded (Opus r2 W1).** A parameter-bound callee is dropped
>    **because a parameter's value is not statically bound**, not because it provably holds no function. Measured
>    value-flow cost (a removed row is a right edge lost when the parameter's function has exactly one in-repo call
>    site and that caller passes the base target; `probes/param_valueflow.py`, Q72): **X 6, F 2, T 5** rows (lower
>    bounds); a supplementary reading (several call sites, every one that supplies the argument passes the target)
>    adds X 5, F 2, T 10. These are **owner-accepted costs**, recorded beside the wrong rows removed (§8).
> 4. `js_binding_site.rs` and `js_binding.rs` are split for the 600-line rule (§3.8 (13)).

> **Owner decision 2026-09-29 (S1b-3 spec round 1 fold; authoritative; corrects OQ12 below).** OQ12's "0 right
> lost" was false (Opus r1 W1, confirmed by the controller): v10's R5 drop removed right edges reached through a
> value alias (X `const { t } = useI18n(); t()` and four hook returns; T `const { startLexicalEnvironment, … } =
> context`, `const { addOutput, getOutputs } = createAddOutput()`). **The drop is narrowed to bindings that provably
> hold no function**, at R4 and R5 alike: parameters, `for` heads, classes, enums, namespaces, duplicates,
> recovered imports, and declarators whose unwrapped value is a literal or an object/array literal with no
> identifier or function member (the closed class of §3.8 (11)). Every other declarator or destructuring
> (identifier, member, call, `await`, conditional, logical and any other value) **keeps base**, counted
> `local_binding_may_call: {alias}`. Re-measured on the clean-built v11 (§8 r4): X 64, F 47, T 227 wrong rows removed,
> **0 right lost outside E6**; the rows v10 removed through an alias (X 38, F 424, T 212) keep base.
> **Consequence (disclosed):** carry-forward 4's block-nested `const { f } = o` rows (the S1b-1b `b5` in-block
> rows, F's 2) have an identifier value, so they **keep base** (wrong edges stay); so do `using h = res()` (C114,
> a call value) and the other alias rows at R4 (X 5, F 28, T 24 rows r3 had removed).

> **Owner answers to the S1b-3 re-plan, 2026-09-28. These are authoritative.**
> - **OQ10 = (a):** split S1b-3 into **3a**, the collector at every scope (0 corpus rows; caps src 590 / tests 620),
>   and **3b**, the call-site wiring (caps src 220 / tests 800).
> - **Cache versions:** 3a 102/58, 3b 103/59, S1b-4 104/60.
> - **OQ11 = (a):** destructuring declarators from a call (`const [s, setS] = useState(…)`) are **may-call**; they
>   keep base behavior, per E5.
> - **OQ12 = (a):** accept the R5 reach of carry-forward 4. ~~It removes 781 wrong rows (X 83, F 441, T 257) and
>   loses 0 right ones.~~ **Superseded 2026-09-29** (above): the counts included alias rows, some right.
> - **Earlier (2026-09-27):** any written binding, including a class, a `for (var …)` head or a parameter, keeps base
>   behavior (M2 kinds).


> **Owner decisions for S1b-3, 2026-09-27 (binding).**
> - **M2 kind list:** any written binding keeps base behavior (`MayCall`, Option K), whatever its kind: a written
>   `class f`, a written `for (var f in/of …)` head and a written parameter included (they refused `not_callable`
>   before). §3.8 (3).
> - **Cache:** S1b-3 **102 / 58**, S1b-4 **103 / 59**.
> - **Budget:** plan with honest measurement; tests have run about 1.5–2× the prototype estimates on every S1b
>   slice (both grammars per row, rustfmt stacks tuples). §9 "r4" gives the caps and their basis.
>
> **Open for the owner from the r4 plan (recommendations in §3.8 and §9):**
> - **OQ10, split S1b-3 into 3a (the collector at every scope, 0 corpus rows measured) and 3b (the call-site wiring:
>   every corpus row).** Recommended (a) split. It needs one more cache number: 3a 102 / 58, 3b 103 / 59, S1b-4
>   104 / 60 (3a changes persisted export facts on the CF1/CF8 controls, so it bumps). (b) one slice at 102 / 58.
> - **OQ11, M1 for destructuring declarators.** `const [s, setS] = useState(() => init)`: the landed classifier refuses
>   a pattern declarator `not_callable`, the SPEC's M1 text and the auditor call it may-call. Measured: 3 F rows (X, R,
>   T 0), each a wrong R5 edge. (a) **may-call (keep base; recommended: E5 says may-call rows never change, and it
>   keeps `maycall_changed` at 0)**; (b) refuse (removes the 3 wrong edges). v10 implements (a).
> - **OQ12, the R5 reach of carry-forward 4.** A name bound in-file to a proven non-callable declaration (a parameter,
>   a destructured or other non-callable declarator, a `for` head, a class, an enum, a namespace, a recovered import)
>   binds no repo-wide function either. Measured (v10, **corrected 2026-09-29**: the lexical auditor missed value
>   aliases): X 83, F 441, T 257 R5 rows removed, not all wrong. v9 removed only R4 rows. Recommended: accept (carry-forward 4 asked for the "and similar" rows).

> **Owner answers after the round-3 fold, 2026-09-27. These are authoritative.**
> - **OQ8 = (a):** F4 (shorthand-destructuring shadow, routed through `collect_js_ts_binding_pattern_names`) ships
>   as its own small slice, **S1b-1b**, directly after S1b-1.
>   - Caps: 15 src / 120 tests.
>   - Measured: X 88 wrong edges removed and 3 right added; F 16 wrong removed; T 5 NameOnly rows; R 0. 112 of 112
>     audited right.
> - **OQ9 = (a):** re-caps (src / tests): **2a 615 / 555**, **3 590 / 850**, **4 195**; 2b unchanged (140 / 230).
>   These are measured from v9.
> - **Cache schedule with S1b-1b:** S1b-1 99/55, **S1b-1b 100/56**, 2a none, **2b 101/57**, **3 102/58**,
>   **4 103/59**.
> - **S1b-1** (PR #327): both reviewers APPROVE in implementation round 2; awaiting the owner's merge.


> **Owner decision after spec round 3 (2026-09-26): targeted fold, then implement.** Both round-3 reviews were
> converging (Opus FIX 2 / 4, sol FIX 4 / 0). The folds are recorded in `REVIEW-r3-fold.md` and below (§3.1 D1,
> §3.1a, §5, §6, §7, §8, §9), each pinned by RED control rows and mutants that the implementation reviews verify.
> **Open for the owner from this fold:**
> - **OQ8, the F4 collector fix** (S1b-1 implementation review, Opus S1): (a) **a separate small slice S1b-1b**
>   between S1b-1 and S1b-2a (recommended: it changes R4c and R5 rows no other sub-slice owns, and 2a is a 0-row
>   slice); (b) fold into S1b-3; (c) defer. Measured on the S1b-1 head: X 91 rows (88 wrong R5 edges removed, 3 right
>   `import_member` edges added), F 16 (wrong R5 edges removed), R 0, T in PLANNING-PROBES Q45; all audited right.
> - **OQ9, re-caps (measured on v9, §9):** S1b-2a measures **557** src against its 510 cap and S1b-3 **533** against
>   480; 2b (123 / 140) and 4 (177 / 185) fit. Recommended: 2a **615**, 3 **590**, 4 **195**, tests 3 **850** and 2b
>   **230** (§9). Until the owner answers, the dispatch caps stay the re-plan's and the implementer stops at them.

> **Owner answers to the re-plan (`REPLAN-fable.md` §6), 2026-09-26. These are authoritative and supersede every
> "pending owner choice" marker below.**
>
> - **OQ1 = (a):** keep base behavior (`JsLocalBinding::Unchecked`, counted) at unproven positions. This is Option K.
> - **OQ2 = (a):** the leave predicate. Modeling TS merging becomes a follow-up lane.
> - **OQ3 = (a):** five sub-slices, S1b-1 / 2a / 2b / 3 / 4, with the re-plan caps.
> - **OQ4:** one owner-approved **spec round 3** on this packet. There is no round 4; non-convergence goes back to
>   the owner.
> - **OQ5 = (a):** fold string-literal export names (`ModuleExportName`) into 2b.
> - **OQ6:** S1b-1 is unchanged by the re-plan. Its implementation (branch `feat/s1b-1-intrinsic-guard`,
>   `9a26164a`) found that F3 already removes C128's wrong R3 edge, so C128 moves from S1b-4's list to S1b-1's.
> - **OQ7 = (a):** named class expressions with decorators are unproven, which gives base behavior.


> **Status after spec round 2 (the cap), 2026-09-26.** Both round-2 reviews
> (`~/prism-evidence/s1b/reviews/spec-r2-{opus,sol}.md`) found new *semantic* misses in kinds the table had
> classified: TS namespace merging and computed method names. Each yields a wrong Exact edge, which the B0 allowlist
> cannot catch. The controller classified the collector (S1b-2..4) as **open-class at the cap**. **Owner decision:**
> ship **S1b-1** (intrinsic guard plus shared collector fixes, §9) now; neither reviewer found a defect in it. The
> collector design for **S1b-2..4 is parked pending a Fable re-plan**, which also owns the bounded round-2 items
> (Opus W2 header errors, sol W2 string-token braces, sol W3 string-literal export names, the B0 leaf-kind SMELL).
>
> **Re-plan delivered (`REPLAN-fable.md`, 2026-09-26): pending owner choice.** The class is open because the unit of
> proof is the node *kind*; it closes when the unit is *(kind, child position) → environment*, with a positional
> fail-safe that can fire and a leave predicate for TS merging. Measured cost on X, F, R, T: 0 rows under every
> option that keeps v8's yield. Recommended: Option K (keep base at unproven positions), S1b-2 split into 2a/2b,
> S1b-3 re-capped. Sections marked **pending owner choice** below (§3.1a, §7, §9, §11) reflect it; the r2 text stays.
> Owner questions OQ1–OQ7 are in `REPLAN-fable.md` §6. S1b-1 is not affected (OQ6).


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
| **S1b-3** | The core at every scope (site walk, parameter scope, decorators, scoped write scan, memo); lexical `LocalDef` with `(name, span)` matching (§3.6); `useCallback` (§3.7). **r4: §3.8 (against the landed code), recommended as 3a (collector) + 3b (wiring), OQ10** |
| **S1b-4** | R3 namespace qualifiers through the proven qualifier binding and the module's exports; E7 split (§3.4) |

**Non-goals** (each pinned by a test, §7): named-, default- and `require`-import qualifiers on R3, including their
multi-target rows (E8, S2); CommonJS `Local` production; `ImportForward`/`ReExport` mechanics other than the
`(span, wrapped)` barrel key; `IndirectResolution`, `MacroArg` and synthetic sites (E10); module resolution
(`./x.js` for `x.ts` outside S1b-4's sibling rule, tsconfig `paths`, workspaces); the may-call class (E5; a
follow-up lane); **generator function expressions** (not indexed by `function_node_types`; a declarator whose value
is one refuses as `not_callable`, since B3 admits only `arrow_function`/`function_expression` values, and keeps base's
no-edge result, C134; r4 carry-forward 6); runtime mutation; non-JS languages;
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
| D1 | `function_declaration`, `generator_function_declaration` | direct: a declaration of its name. Reached indirectly by a hoisting walk (r3, sol W1): an **Annex B.3.2 marker** (the enclosing statement, non-callable) only for an **ordinary** block `function_declaration` (not `async`, not a generator) in **non-strict** code. Strict: a module (a top-level `import_statement`/`export_statement`, or `.mjs`/`.mts`), a `"use strict"` directive in the program's or any enclosing function's prologue, class code. Sloppy: a `.cjs` script with no directive. **Unknown** (a `.js`/`.ts`/`.jsx`/`.tsx` script without a directive: `package.json` `type` or TS `alwaysStrict` decides): the binding is unproven and keeps base behavior, counted `annex_b_strictness` (Option K). Strict code records no marker (the block declaration is block-scoped) |
| D2 | `class_declaration`, `abstract_class_declaration` | direct: a declaration (non-callable) |
| D3 | `labeled_statement` | transparent (a labelled function declaration is a declaration of its surrounding list) |
| D4 | `lexical_declaration` declarators; TS `using` / `await using` (an `expression_statement` holding an `assignment_expression` with an anonymous `using` token) | direct only |
| D5 | `import_statement` (default, namespace, named, `import x = require()`), `import_alias` (`import x = N.y`: its first named child) | direct; **value space only**: an `import type` statement and `type` specifiers declare nothing |
| D6 | TS: `enum_declaration`, `internal_module`/`module` (identifier name, first segment of a dotted name), `function_signature` inside `ambient_declaration`, ambient `var`/`let`/`const`/`class` | direct (value space). `interface_declaration`, `type_alias_declaration` and an overload `function_signature` outside `ambient_declaration` declare nothing |
| D7 | `var` declarators and `for (var … in/of …)` heads | at any depth while hoisting (T1, T2 bodies, T4 var scopes). A `var` declarator reached through a `catch` clause whose parameter binds the same name, or through a `with` body, records that body as a **marker** (Annex B.3.4: its initializer assigns the catch parameter; under `with`, maybe the object) |
| P1 | binding patterns | BoundNames (any parser-confirmed identifier spelled without an escape, §3.8 (12)): `identifier`, `shorthand_property_identifier_pattern`, `object_pattern`, `array_pattern`, `rest_pattern`, `pair_pattern` (value), `assignment_pattern` (left), **`object_assignment_pattern` (left)**, `required_parameter`/`optional_parameter` (pattern) |
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
nodes is correct; **and a parameter list's parentheses are taken as paired when both edge tokens are present and not
`MISSING`**, r4 carry-forward 5: the brace argument does not cover parentheses, and the landed code states the same
ASSUMPTION at `js_ts_recovery_sealed`). An error inside a sealed node cannot move text across its boundary, and whatever a function,
class or static block declares is invisible outside it. The site is outside every such node, so its walk and every
declaration visible to it are outside the errors.

> **Pending owner choice (re-plan fold of Opus r2 W2 and sol r2 W2; S1b-2a).** (i) counts only anonymous structural
> `{`/`}` tokens under an `ERROR`, or `MISSING` braces; brace *characters* inside string, template, regex or comment
> text never break it. (ii) is sealed only when the error lies inside a **delimited child** of the sealing node (its
> `body` with paired braces, its `parameters` with paired parentheses, a `class_body`); a header error (name, type
> parameters, return type, heritage) refuses, because a node's start is its first token and a leading error can pull
> sibling text in. Measured (re-plan RP4): the 19 rows the narrower rule keeps (F 7, T 12) all stay kept; the 45 it
> refuses stay refused; **0 rows change**, so E6 stands as chosen.

**Why B0 is a fail-safe, not a list of hopes.** `probes/grammar_closure.py` reads the pinned grammars' `node-types.json`
and marks every named kind that can hold a statement, declaration, pattern, parameter list or binding identifier
field as **suspect**; each suspect kind must appear in the table above (as a scope, declaration, wrapper, pattern,
write form, reference-only or type-space kind). It checks this and exits non-zero otherwise (0 unclassified of 76
suspect kinds; 186 concrete kinds). All other non-leaf kinds are mechanically inert. The runtime allowlist is the
union, so a grammar upgrade that adds a kind refuses until the kind is classified. Measured cost: 0 rows on all four
corpora (Q28).

### 3.1a Evaluation context: positions, the leave predicate and the positional fail-safe (S1b-3; owner OQ1/OQ2/OQ7 = (a), r3 fold below; `REPLAN-fable.md` §3)

> Added by the re-plan after spec round 2. It supersedes "Walk jumps" (J1, J2) above and the closure claims "Why B0
> is a fail-safe" for the *site walk*; the declaration rows (D, P, W) and the classification (B0–B3, M1–M2) stand.

**Environment creators (Σ′)** are Σ (T1–T9) plus **T10** `enum_body` (a TS value container declaring the block's
member names, non-callable). The list is cited to ECMA-262's environment creators (§10.2.11, §14.2.3, §14.7.4–5,
§14.11.2, §14.12, §14.15.2, §15.7.14, §16) and TypeScript's namespace and enum bodies.

**Positions.** The site walk consults `E(parent kind, field of the child it came from)` at every step. Kinds outside
Σ′ ∪ {`decorator`} are *Structural* at every position. For Σ′ kinds and decorator holders every field is listed:
`body` positions are *Inside*; function `parameters`/`parameter` are *Parameter* (J1); a `method_definition`'s
computed `name` is *Outside* (**J3**: skip the method's environment, continue at the member's owner); a class's
`decorator` children are *Outside* (J2) for class declarations and **unproven** for named class expressions; the
`with` `object` is *Outside* and its `body` refuses; `for`/`for…in/of`/`catch`/`switch`/static-block positions are
*Inside*; namespace and enum bodies are *Inside* and, on **leaving** the body unbound, apply the **leave predicate**:
unproven iff another namespace/enum declaration in the file shares the first name segment, or the file is a script
(no top-level `import_statement`/`export_statement`). The full table is `REPLAN-fable.md` §3.

**r3 fold (the at-cap round).**
- **J2 split by holder (Opus W1).** A decorator whose holder is the class node itself is *Outside* (the walk resumes
  above the class); on a *named class expression* it is unproven (OQ7, base, counted `decorated_class_expression`).
  A decorator under `class_body`, a class element (`method_definition`, a field) or a parameter is **Inside the
  class** (T8): the walk resumes at the `class_body`, so the class's inner name is visible (member decorators are
  evaluated in the class scope).
- **Dotted namespaces (Opus W2).** On leaving the body of `namespace A.B.C`, each non-first segment (`B`, `C`) is a
  declaration of a namespace, non-callable: a callee with that name refuses `not_callable`. The first segment is
  D6's declaration in the enclosing scope, as before.
- **`for_in_statement.left` is Inside** the loop head environment (Opus S4: a destructuring default in the head).
- **T10 compares member names by StringValue** (Opus S4: `'f' = 1` declares `f`), through the same
  `ModuleExportName` helper as 2b.
- **The table is checked, not asserted (sol W2).** `probes/grammar_closure.py` derives the 23 Σ′ kinds' 74
  positions from `node-types.json`, fails on a missing or extra `E_TABLE` row, derives the 186-kind allowlist
  (leaves included), and with `--rust <src/ast>` fails unless the runtime `CLASSIFIED` and `E_TABLE` equal the
  derived tables exactly. The Rust unit test that asserts the same equality lands in **S1b-3** with the runtime
  table. **S1b-2a** has no site walk: its not-yet-wired items carry `#[allow(dead_code)]` (module-level, with a
  comment naming the wiring slice), which 2b and 3 remove; 2a's tests call the module terminal through the crate's
  normal `pub(crate)` API, so no test-only entry point is added.

**Fail-safe (OQ1 = a, Option K).** A position not listed for a Σ′ kind is **unproven**: the binding is
`JsLocalBinding::Unchecked(reason)` (base behavior; the v9 prototype spells it `BasePosition`), counted in
`local_binding_unchecked_position: {reason: n}`. The reasons are closed: `unproven_position` (an unlisted `E` row),
`namespace_leave` (the leave predicate), `decorated_class_expression` (OQ7) and `annex_b_strictness` (D1, unknown
strictness). `probes/grammar_closure.py` derives every field of every Σ′ kind from `node-types.json` and exits
non-zero when one has no rule; B0 covers leaf kinds (sol r2 S4).

**Measured (re-plan RP3, RP3b; r3 on v9, Q41–Q43):** the pinned grammars allow 319 `(kind, field)` pairs
(`probes/replan/count_pairs.py`); corpus sites cross 103. The unproven set is empty on X, F, R and T (the leave
predicate fires on 0 T sites; decorators, computed member keys, `with` and enum bodies are crossed by 0 sites), and
the r3 fold (J2 split, dotted segments, `for_in.left`, T10 StringValue, the Annex-B predicate) changes 0 rows against
v8 on every corpus, so §5's expected counters and §8's row-diffs are v8's.

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

#### Dated amendment — 2026-10-01, landed-core design; measurements pending

READ: this amendment replaces the old mechanism below. The landed APIs are
`js_ts_site_binding`, `js_ts_binding_walk`, `js_ts_scope_lookup`,
`js_ts_scoped_written` and the B0/B1 checks. Do not recreate Σ, E_TABLE, the
declaration walk, write resolution, recovered-import scanner or wrapper
provenance. The latter is non-classifying specifically to avoid recursion.

ASSUMPTION (implementation design to verify): extend `CallSite.local_binding`
with `JsLocalBinding::NamespaceImport { module_path }`; reuse MayCall, Position,
Unproven and Unchecked for the namespace qualifier. This field denotes the
callee at an unqualified site and the simple qualifier at an eligible namespace
site; it remains serde-defaulted and excluded from cmp_key. No second binding
enum or wide CallSite-literal rewrite is needed. All three source constructors
use the same per-file JsBindingCache. Synthetic/indirect sites stay Unchecked.

ASSUMPTION (qualifier extraction/proof):
1. Locate the exact source node by start/end byte. Calls use their `function`,
   JSX opening/self-closing sites their `name`. Admit only a member_expression
   with an identifier object q and a static member matching the recorded
   callee. A nested receiver, subscript, asserted receiver or unmatched span
   keeps its base behavior. Constructors remain a preservation pin unless
   their existing extraction gives the same eligible member shape.
2. Enter the new route only if the file contains a top-level **value** ESM
   namespace-import spelling q. Cache this syntax inventory once per file.
   A type-only import, named/default import, require declarator or
   `import x = require()` never grants this authority. Do not consult the
   lossy flat imports map as provenance. Existing non-namespace qualifiers
   and their receiver guards retain base behavior.
3. Run the landed core at q. NamespaceImport requires Walk::Found(program,
   None), one declaration in that program index, exactly that namespace
   import_statement, and JsBinding::Import after B0/B1 and the scoped-write
   check. Recheck node identity against the namespace inventory; Import alone
   is insufficient. Retain its syntax-backed specifier (StringValue); an
   undecodable source does not prove a namespace module.
4. JsBinding::MayCall or Alias maps to MayCall and keeps the **whole base
   ladder**, including its existing receiver guards; never use the new export
   projection or demotion for those sites. A written import, written shadow,
   written parameter/class/loop binding has the same rule. Position/Unchecked
   keeps base under Option K. Count qualified MayCall/Position in the existing
   maps once per site, without changing unqualified rows.
5. A clean shadow or duplicate does not prove the imported namespace. Suppress
   this namespace R3 interpretation and continue through the existing receiver
   ladder. Callable q does not prove a namespace object, and not_callable q
   does not prove absence of callable **members** (classes and enums can have
   them). Do not add a blanket member-call drop from that classifier result.
   B0/B1/recovered-import refusals suppress namespace authority; E6 owns any
   parse-recovery cost. A written qualifier is never suppressed merely because
   it is written. Explicitly pin with-body vs with-object positions.
6. Proven namespace authority pre-empts the old syntactic shadow guard, which
   does not model all E_TABLE positions (for example parameter-default sites
   with a body-only declaration). It does not rely on receiver flags to prove
   the import. The proof is never reused at another byte span or file.

ASSUMPTION (export lookup): factor the candidate-file half of
`js_ts_import_member_candidates` into `js_ts_export_candidates(file, member,
site)`, shared by R4c and namespace R3. The R4c relative resolver and outputs
remain identical. Lookup precedes `functions.get(member)` so renamed and
StringValue exports can target a differently registered local name (RP2-c).
Use the landed resolved export table, with its depth/cycle/conflict and class
rules. Match terminal file, registered local name and, when present, exact
start/end lines; exclude methods. A spanned terminal with 0 or 2+ identity
matches has no candidate. Wrapped + non-JSX gives WrappedExportNonJsx. An
authoritatively resolved module is final even when it exports no member;
never use another directory's same stem in that case. A unique candidate is
Exact; multiple unspanned candidates are NameOnly. Empty result is UnknownName
if no registered member name exists, otherwise ImportExternal. Err is final.

READ (E5): an export terminal kept as base Local because it is may-call/written
must keep R3 base behavior too. ASSUMPTION: conservatively return to the base
ladder for an unspanned resolved Local, rather than narrowing an E5 row. This
also preserves opaque CJS Local rows without changing CJS production.

**OPEN OQ-S1b4-1:** Alias exports are deliberately UnprovenLocal in D4. C220
proves that applying that absence literally to R3 can lose a right edge.
ASSUMPTION / recommendation: record a namespace-only opaque terminal for that
Alias outcome, propagate a keep-base signal through the existing bounded
export traversal, and leave R4c results/counters unchanged. This requires
owner approval for the owned-fact/producer extension. The alternative is a
new accepted alias value-flow cost; the planner chooses neither. The signal
must not masquerade as callable authority or let a conflicted/duplicate
export become Exact. Direct, named-forward and star-barrel alias controls
are required before dispatch if preservation is selected.

READ (E7 split); ASSUMPTION (implementation): first use the existing relative
resolver. Only on failure, replace the final suffix `.js` -> `.ts`, then
`.tsx`; `.jsx` -> `.tsx`; `.mjs` -> `.mts`; `.cjs` -> `.cts`, and require the
exact caller-relative replacement file to be indexed. Use base precedence;
do not admit an index directory or append another extension to the replacement.
This extension substitution belongs only to namespace R3, not the shared
barrel resolver or named/default-import paths. Unsupported .mts/.cts parsing
does not create an indexed file. The fixture must assert an indexed sibling,
not assume the filename makes it indexed.

ASSUMPTION (E7 fallback): only when neither relative nor sibling resolution
succeeds, retain the legacy stem/directory match, filter candidates through
their file's exports under the member name and exact terminal identity,
deduplicate FunctionIds and grade every surviving candidate NameOnly. Never
promote a lone stem hit to Exact. Wrapped candidates cannot suppress a valid
plain candidate; WrappedExportNonJsx applies when no eligible candidate remains
and the only otherwise valid hits were wrapped. Empty fallback uses the base
reasons. May-call/opaque outcomes obey the keep-base rule before projection.

READ: the old paragraph below is historical, particularly its proposed
NamespaceImport/Unproven-only state and its treatment of written qualifiers.

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

### 3.8 S1b-3 against the landed code (r4, prototype v12 on `3961cc21`)

The landed S1b-2a collector (`src/ast/js_binding*.rs`) evaluates the core at module scope only; S1b-2b wires it to
D4. S1b-3 extends it to every scope and wires it to call sites. Normative deltas over §3.1, §3.1a and §3.6:

1. **Site walk** (§3.1a, unchanged rules): `js_ts_binding_walk(site, name) -> Walk::{Found(scope, explicit),
   Unbound, Unchecked(reason)}` driven by the 74-row `E_TABLE`, identical to v9's (the closure probe checks it:
   `grammar_closure.py --rust src/ast`, without `--kinds-only` from S1b-3 on). `explicit` is the node that binds the
   name when no index does: the `with_statement` (T9) or a dotted namespace's `internal_module` (a non-first segment).
2. **Scope index at every scope** (`js_ts_scope_index`, memoized per scope id in `JsBindingCache.decls`): T2–T10 over
   the landed declaration walk (`js_ts_declare_walk`, unchanged), parameters and `arguments` recorded at the
   parameter list (never callable), a named function expression's own name, class inner names, T10 enum members by
   StringValue (the 2b `ModuleExportName` helper; an undecodable member name declares nothing). The module terminal
   (`js_ts_module_binding`, D4) becomes the program-scope case of one `js_ts_scope_binding(scope, explicit, name,
   site)`: unbound → recovered-import marker → B0 → B1 → Annex-B → `js_ts_classify`.
3. **M2, scoped and memoized (carry-forwards 1–3).** The scoped write resolver **replaces** the module write scan and
   2a's predicates (a) and (b) (`js_ts_written_unseen`, deleted): the file's W1 targets are indexed once by name
   (`JsBindingCache.write_targets`), and a binding `(scope, name)` is written iff some target of `name`, walked like a
   site, is `Found` in that scope, or in a `with`, or `Unchecked` (an unproven position counts as a write, toward base).
   The answer is memoized per `(scope id, name)` (`JsBindingCache.written`; C-19). W1 targets: assignment,
   augmented-assignment, update and `delete` targets, and bare `for (x in/of …)` heads (a `using` declaration and a
   declaring head are not writes); **plus every identifier that is a child or a sibling of an `ERROR`** (recovery can
   mangle a write: `f = ;` parses as `f` beside an `ERROR` holding `=`). This closes sol 2a-r2 W1–W3 (C166–C168: a
   `let f` in a default-parameter arrow, an inner `function f` in a sealed-error function, a class-expression inner
   name written from a computed key) and restores C150 to `Callable` while keeping Opus 2a W2's mangled write
   `MayCall`. **Owner M2 ruling:** the check applies to every declaration kind (a written class, `for (var …)` head or
   parameter keeps base, C169).
4. **M1 for destructuring declarators (OQ11 a).** A pattern declarator (or `using`) whose unwrapped value is a call
   with a direct function argument is may-call. Any other call value (identifier or pattern declarator) is an
   **alias** (11), not `not_callable`.
5. **Imports at sites.** A binding whose single declaration is an `import_statement`/`import_alias` is
   `JsBinding::Import`: the import rungs decide (R4c unchanged); a same-file function is never the binding (R4 drops,
   C141). An `unbound` name likewise drops at R4 (every same-file function is out of scope: C88, C93) and keeps base
   everywhere else.
6. **B1 containment (carry-forward 8, Opus 2b S1; narrowed by spec r1 Opus W2).** An error is sealed iff it lies
   in a delimited child of a sealer strictly inside the scope and the site is outside it, where "it" is **the
   delimited child for a `body` / `class_body` error** and **the whole sealer for a `formal_parameters` error**
   (parameters are visible from the body: `export function g(a, f ==) { return f(); }` refuses, C178). The
   declaration-export site (the declaration node, which contains its own body) is therefore outside a sealed body:
   `export function f(){ let x = ; } export { f as g }` now verifies both routes (C172). 0 corpus rows; F's
   `parse_recovery` D4 refusals go 2 → 1.
7. **Recovered imports (carry-forward 9; spec r1 Opus W3, sol W2; r2 sol W2).** A top-level `ERROR` is a recovered
   static import iff its first token is `import` and the next token is neither `.` (`import.meta`) nor `(`
   (`import()`), C179. **Tokens** are leaves that are not comments or other trivia extras (tree-sitter also flags
   recovery `ERROR` nodes as extras; those are descended, not skipped), so `import /* c */ . meta` and `import // c`
   + `.meta` are not imports (C189). It contributes a recovered-import marker to the program index for every `identifier` inside it and for
   every word of its source text, a word being a maximal run of ASCII identifier characters (`A–Z a–z 0–9 _ $`) and
   non-ASCII non-whitespace characters, so `é` (e + U+0301) and ZWNJ/ZWJ names stay whole (C181); the keywords
   `import`, `from`, `as`, `type`, `typeof` and digit-led runs are excepted. Words come from string text too,
   because recovery can put the alias there, as in `import { "\u{GG}" as h }`. A binding with such a marker refuses `import_parse_recovery`, which
   drops at R4 and at R5 (C173). Over-poisoning is bounded to names spelled in a broken import and costs no measured
   row (0 on all four corpora). **In S1b-3, not a separate slice:** the marker is one arm of the program index that
   the site walk already consults; outside the walk it would need its own per-file fact and resolution guard.
8. **Resolution (§3.6 as amended).** `CallSite.local_binding: JsLocalBinding::{Unchecked (default), Callable(JsTerminal),
   MayCall, Unproven(reason), Position(reason)}` (`#[serde(default)]`, excluded from `cmp_key`), set at the three
   source constructors with one `JsBindingCache` per file. Unqualified branch: `Callable` first (§3.6); at R4 with
   same-file candidates, any `Unproven` drops `LocalBindingUnproven`; **at R5, where base would bind, `Unproven` with
   reason `not_callable`, `duplicate_declaration` or `import_parse_recovery` drops `LocalBindingUnproven`**
   (carry-forward 4's class as narrowed 2026-09-29: `not_callable` now means provably no function, (11)).
   `MayCall(reason)`, with `reason` `may_call` (E5) or `alias` (11), keeps base everywhere and counts
   `local_binding_may_call`; `Position` counts `local_binding_unchecked_position` and keeps base; other `Unproven`
   reasons keep base at R5. C170: the block-nested `const { f } = o` and the bare-block `var { f } = o` keep base
   (alias); the bare-block `const` P15 stays Exact (unbound at the site).

   **S1b-3b implementer note (C-44, `import f = M.g`/`require()`; corrected round-1 fold, Opus T-c).**
   A pre-existing JS/TS guard (`is_js_ts_import_member_file` + `js_ts_function_locals`/`import_bindings`/
   `js_ts_exports`, predating S1b) already excludes an **`import_alias`-bound name specifically**
   (`import f = M.g`/`require()`, C141) from R4/R5's same-name fallback before `site.local_binding`'s
   `Unproven("import")` is ever consulted (R4's `local` set is empty: an `import_alias` registers no
   `FunctionId`). The row's qualitative outcome (no edge; C-44's "drop at R4") holds for that form, but the
   recorded `DropReason` is the older guard's `UnknownName`, not `LocalBindingUnproven`. This is correct and
   requires no reordering: SPEC §3.8 (8) only narrows what `Unproven` *adds* at R4/R5; it was never meant to
   pre-empt guards that already produce the same no-edge result earlier.
   `tests/integration/js_binding_scope_resolution_test.rs::c44_import_alias_drops` asserts the actual observed
   reason. **The original note overstated this as a general "same-name JS/TS guard" masking rule — it does
   not.** A namespace import (`import * as f from './m'`) is *not* tracked by that guard at all: an unwritten
   namespace-import shadow reaches this slice's own `LocalBindingUnproven` drop directly (round-1 fold T-c,
   `js_binding_scope_resolution_fold2_test.rs::tc_unwritten_namespace_import_maps_to_unproven_not_maycall`).

   **S1b-3b implementer note (C-M37, ASCII-only word scanner; corrected round-1 fold, Opus S2 / sol SMELL 1 /
   T-g).** The earlier form of this note claimed the mutant was killed at the unit level by 3a's
   `c181_unicode_and_zwnj_names_stay_whole` fixture and by an end-to-end test using the same bare-`identifier`
   shape. **Both claims were wrong: that shape does not kill the mutant at either level**, because
   `js_ts_recovered_import_names` also walks the tree's own `identifier` children directly (not just the text
   scanner), so a clean identifier token independently supplies the correctly joined name regardless of the
   scanner mutant — the survivor is real on that input shape, at both levels. **What actually kills it** is
   the real `C181_solW2_unicode_recovered_alias` control shape from `probes/controls_gen.py`: the Unicode
   aliases spelled *inside the malformed import's string text* (`import { "\u{GG}" as é }`), where parser
   recovery leaves the scanner, not the identifier walk, as the only source of the name. Ported end to end in
   `js_binding_scope_resolution_fold2_test.rs::tg_c181_malformed_string_unicode_alias_drops_and_kills_cm37`
   (both `.js`/`.ts`), confirmed to kill the mutant by direct mutation-and-rerun. The mutant is owned by 3a's
   `js_binding_recovery.rs` (not a S1b-3b file); this is a 3b-owned regression test for a 3a-owned mechanism,
   which SPEC §3.8's ownership split permits since the mutant is only reachable end to end.
9. **`useCallback` (§3.7).** `JsBindingCache::for_sites()` sets the call-site route; S1's predicate takes a
   `local_route` flag. A `useCallback`-admitted binding is `Callable` with `wrapped: false`; forwardRef/memo stay
   `wrapped`. D4 never admits `useCallback`.
10. **Namespace-export restriction (carry-forward 7).** `namespace N { export const { f } = o; }` with a module-level
    `run(){ f() }` stays Exact `free_single` (the walk does not enter the namespace body; the S1b-1b collector's
    `export_statement → program` rule), pinned in both S1b-3 and against MX4 (C171).
11. **What drops, and why (owner 2026-09-29, rounds 1 and 2).** Two different grounds:
    - **Not statically bound: parameters.** A parameter (and a catch parameter) holds whatever a caller passes; the
      static-binding model does not follow values across calls, so a parameter-bound callee binds no function.
      This is **not** a claim that it holds none: the dropped rows include right edges reached by value flow, whose
      measured cost the owner accepted (§0 (3), §8).
    - **Provably holds no function:** `for` heads, classes, enums, namespaces, markers, duplicates, recovered
      imports, and a declarator (or `using`) that has no value (and is unwritten), or whose unwrapped value is in the
      closed class NoFn below, **provided its pattern holds no default** (a destructuring pattern with a default
      anywhere is an alias, round 2).

    `js_ts_classify` answers `not_callable` for a declarator only in that last case. NoFn: `number`, `string`, `template_string`, `true`, `false`, `null`, `undefined`, `regex`; an `array` whose
    every element is in NoFn; an `object` whose every member is a `pair` whose value is in NoFn (a shorthand property,
    a spread, a method or any other member is not). Any other value is `JsBinding::Alias` (`const t = i18n.t`,
    `const { t } = useI18n()`, `const x = await f()`, `a ? f : g`, `f || g`, `new X()`, `-1`,
    `const { missing: x = fallback } = {}`, `const [y = 0] = []`, …). Parameters, `for` heads, catch parameters,
    classes, enums, namespaces and markers stay `not_callable`. **D4 is unchanged:** an alias
    at an export occurrence keeps the landed 2b answer (`UnprovenLocal`, counted `not_callable`).
12. **P1 identifiers (spec r1 sol W1).** `collect_js_ts_binding_pattern_names` trusts parser-confirmed `identifier`
    and `shorthand_property_identifier_pattern` nodes: any spelling without a `\` escape is a BoundName (`é` with
    U+0301, ZWNJ/ZWJ, `$` as S1b-1b), replacing `is_plain_ident`; escaped spellings stay out and B0 refuses their
    scope. Blast radius of the shared helper (about 20 call edges in 3 files, including S1b-1b's F4 and the write
    scans): **0 call-site rows on X, F, R, T** (v11 vs v11 with the old predicate, Q66); preservation tests in 3a.
13. **File split (owner 2026-09-29, Opus r2 S2).** Each file stays under 600 physical lines with docs:
    `js_binding_walk.rs` (`Pos`, `E_TABLE`, `Walk`, the walk, the leave predicate; about 260 in v12);
    `js_binding_site.rs` (`is_scope`, the site binding, scope lookup and index, function names; about 220 of v12's
    292); **`js_binding_writes.rs`** (the scoped write resolver and W1 targets, about 70); **`js_binding_recovery.rs`**
    (the recovered-import predicate and names, about 55; or folded into `js_binding_checks.rs`, 141 lines);
    **`js_binding_values.rs`** (from `js_binding.rs`: `js_ts_classify_call`, M1, the default test and NoFn, about
    90, bringing `js_binding.rs` from 515 to about 425). Tests in new files (`js_binding_site_tests.rs`,
    `js_binding_walk_tests.rs`, `js_binding_values_tests.rs`); `js_binding_tests.rs` (584) only takes 2a's updates.

## 4. Controls (MEASURED, v8; `probes/S1b-controls-*.txt`, expectations in `probes/S1b-controls-expectations-pre-run.md`)

218 scenarios; every S1b scenario runs in the JSX (`.jsx`/`.js`) and TSX (`.tsx`/`.ts`) grammar, except the TS-only
C102–C104, C114, C117, C141–C148. All results match the pre-registered expectations after addenda 4–6 (one prediction
was falsified in detail and recorded before the rerun: C151, addendum 6). **r3 (v9):** 240 scenarios; the fold's
C152–C165 and the reversed C111 match addendum 7, and the v8 → v9 control diff contains only those rows
(`probes/S1b-controls-proto-v9.txt`; §7 C-31–C-34).

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
| C93, C111 block function inside / outside (modules: strict) | D1 | 3 | inside Exact; C93 outside drop; **C111 outside Exact to the outer function (r3, sol W1)** |
| C94 hoisted `var` arrow | D7 | 3 | unchanged |
| C95 same-line collision | span | 3 | → drop |
| C99, C127 recursion, fe self-call, labelled function (sol W5) | D3, T2 | 3 | unchanged |
| C102, C103, C147 (TS) overloads, `declare function`, `declare const` | D6 | 3 | unchanged; drop; drop |
| C104 (TS) block `enum`; `function N` + `namespace N` | D6, B2 | 3 | → drop (the second is a fail-closed recall loss) |
| C105, C108, C109, C151 errors that reach the site or its scope's list | B1 (ii) | 3 | → drop |
| C149, C150 an error sealed in another function | B1 (ii) | 2 | unchanged (Exact) |
| C114 (TS) `using` | D4 | 3 | the using-bound call → drop; the other call unchanged. **r4 2026-09-29: `using h = res()` is an alias (a call value) → keeps base Exact** |
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
| C128 parameter qualifier | §3.4; READ owner OQ6 | 1; preservation in 4 | MEASURED base has no R3 edge; ASSUMPTION unchanged |
| C129 `with` qualifier | §3.4 amended proof | 4 | ASSUMPTION no namespace authority; existing receiver ladder |
| C130 written qualifier | READ E5, §3.4 amendment | 4 | ASSUMPTION whole base row kept |
| C131, C132 (E7) bare / non-sibling stem | §3.4 | 4 | Exact → NameOnly |
| C133 (E7) sibling without the member | §3.4 | 4 | Exact to another directory → drop |
| C84, C85 named/default qualifiers | READ E8 | – | ASSUMPTION unchanged |
| C112 duplicate/competed qualifier | §3.4 amended proof | 4 | ASSUMPTION no namespace authority; preserve existing non-R3 result |

**MEASURED — 2026-10-01 S1b-4 base controls:** 349 scenarios, comprising the
original 287 plus C190–C220 in true .jsx/.tsx twins. All original 287 generated
sources are byte-identical. Both original and extended base runs have empty
stderr. `probes/S1b-4-EXPECTATIONS.md` is the pre-run registration;
`target/plan-s1b4/base/controls-r2/SUMMARY.txt` is the base reference.
ASSUMPTION: direct/directory decoys C190/C191/C205/C207 narrow to exported
identity; C192/C193 add renamed/star-barrel edges; C194 namespace-object and
C195/C196 parameter shapes are preservation pins; C197/C198/C199 and
C218/C219 keep base (written/alias/E5 export); C200/C201 lose namespace authority;
C202/C203/C204 cover recovery refusal/sealing; C206/C208/C209/C210 cover
evaluation positions; C211/C212/C213 preserve type-only/import-equals/named/default
non-goals; C214 gates wrappers; C215 refuses a non-exported decoy; C216 uses
NameOnly fallback. C217's .tsx sibling becomes Exact, while its .jsx target for
a `.js` specifier is a non-sibling NameOnly fallback. C220 is the OPEN owner
policy control, not an accepted-cost assertion. Head summary is pending.

## 5. Counters

- `dropped_jsx_intrinsic`, `dropped_local_binding_unproven` (new drop reasons, exhaustive match);
  `dropped_wrapped_export_non_jsx` now also counts R3 and R4 sites; `js_export_local_refusals` (sorted object).
- **`local_binding_unchecked_position`** (S1b-3; r3, sol W4 / Opus S1): a sorted object `{reason: n}` counting
  bindings kept at base behavior under Option K, by the closed reasons of §3.1a (`unproven_position`,
  `namespace_leave`, `decorated_class_expression`, `annex_b_strictness`). Always emitted, `{}` when empty. Measured on
  v9: **`{}` on X, F, R and T** (C155, C159, C160 and the RP1-c/f controls exercise it).
- An implementation adds `js_export_local_may_call` (a count of export occurrences kept at base by E5), so the E5
  class is observable; the prototype does not have it.
- Expected (v8; **re-measured on v9, identical on all four corpora**, Q43):

| Corpus | `dropped_jsx_intrinsic` (S1b-1) | `js_export_local_refusals` (S1b-2) | `dropped_local_binding_unproven` (S1b-3) | `unresolved_unknown_name` | `multi_target_exact_sites` | `local_def` Exact edges |
|---|---|---|---|---|---|---|
| X | 866 | `{not_callable: 6}` | 19 | 10,922 → 10,118 | 158 → 20 | 2,309 → 2,147 |
| F | 490 | `{not_callable: 28, parse_recovery: 2}` | 34 | 11,399 → 11,019 | 5 → 3 | 794 → 749 |
| R | 117 | `{}` | 0 | 721 → 604 | 0 → 0 | 96 → 96 |
| T | 0 | `{parse_recovery: 26}` | 223 | 28,736 → 28,728 | 680 → 15 | 20,230 → 18,156 |

`multi_target_exact_sites` keeps E8's R3 rows (X 6) and may-call rows (X 14).

**r4 expected for S1b-3 (clean-built v12 `0968ef78`, identical counters to v11 `0982f2fd`, against the S1b-2b head
`3961cc21`, Q64/Q70; the r3 table above and the v10 numbers are superseded):**

| Corpus | `dropped_local_binding_unproven` | `local_binding_may_call` (new) | `local_binding_unchecked_position` | `js_export_local_refusals` | `unresolved_unknown_name` | `multi_target_exact_sites` |
|---|---|---|---|---|---|---|
| X | 0 → **64** | `{alias: 312, may_call: 48}` | `{}` | `{not_callable: 6}` (unchanged) | 10,203 (unchanged) | 158 → 24 |
| F | 0 → **51** | `{alias: 1408, may_call: 89}` | `{}` | `{not_callable: 28, parse_recovery: 2 → 1}` | 11,033 (unchanged) | 5 → 3 |
| R | 0 | `{alias: 26, may_call: 3}` | `{}` | `{}` | 604 | 0 |
| T | 0 → **268** | `{alias: 444, may_call: 185}` | `{}` | `{parse_recovery: 26 → 25}` | 28,743 → 28,733 | 680 → 17 |

`local_binding_may_call` counts call **sites** whose binding keeps base (E5 may-call or a value alias), not changed
rows. The `parse_recovery` decrements are carry-forward 8. Under OQ10 (a), 3a moves only the
`js_export_local_refusals` cells; 3b moves the rest. `js_export_local_may_call` is unchanged (X 1).

## 6. Cache

**Schedule (r3, sol W4 / Opus S1; CPG / sidecar, from the landed 98 / 54):**

| Sub-slice | Bump | Why |
|---|---|---|
| S1b-1 | 99 / 55 (landed on its branch) | a new drop reason changes resolution-derived edges |
| S1b-1b (OQ8 a) | 100 / 56, and every later value shifts by one | R4c/R5 local-binding facts change |
| S1b-2a | **none** | the collector is not wired to any persisted fact or producer; no cached byte changes |
| S1b-2b | 100 / 56 | export facts (`VerifiedLocal`, `ModuleExportName`) |
| S1b-3 | 101 / 57 | `CallSite.local_binding` |
| S1b-4 | 102 / 58 | qualifier binding and R3 edges |

**r4 (owner 2026-09-27; the table above is superseded):** S1b-1 99/55, S1b-1b 100/56, 2a none, 2b 101/57 (landed
pins), **S1b-3 102/58**, **S1b-4 103/59**. If S1b-3 splits (OQ10 a): **3a 102/58** (the export facts of CF1/CF8
shapes change, so a cached D4 fact is stale), **3b 103/59** (`CallSite.local_binding`), **S1b-4 104/60**. Each carries
the B-17-style cross-commit row: a cache written by its parent's binary is rejected and the rebuilt output equals
`--no-cache`.

Each bump updates the pins. **B-17 (2b), cross-commit regression:** a cache written by the parent commit's binary
and read by the sub-slice's binary must be rejected (version mismatch) and the rebuilt output must equal
`--no-cache`; it fails on a mutant that forgets the bump. 2a's review checks that no persisted type changed
(`git diff` over the serde types and the cache writers is empty).

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
| S1b-4 (READ required rows; ASSUMPTION implementation expectations, 2026-10-01) | D-1 C62/C80/C81/C148 + C190/C191: exact exported identity; D-2 C82/C214 wrappers; D-3 C83/C192/C193 + RP2-c (all 4 twins): renamed/StringValue/star exports; D-4 C128/C129/C130/C112 + C195–C210: proof, duplicate/shadow/write/recovery and positional rules; D-5 C131–C133/C215–C217: E7 + authoritative missing-member refusal; D-6 C84/C85/C194/C211–C213: named/default/type-only/import-equals/nested-namespace non-goals; D-7 nav callers/CPG target for C62/C190, with decoy absent; D-8 serde/default/pins/full vs incremental/cross-commit 104/60; D-9 C218/C219 + written-class/parameter E5 pins; D-10 C220 + direct/named/star alias twins conditional on OQ-S1b4-1 | D-M1 stem instead of resolved module; D-M2 skip scope/node-identity proof; D-M3 Exact for a single non-sibling stem; D-M4 route named/default/require as namespace; D-M5 ignore M2/E5; D-M6 span or wrapped omitted from export projection; D-M7 functions.get(member) before export rename; D-M8 shadowed write counted against import; D-M9 no recovery refusal/sealing; D-M10 old shadow guard vetoes a proven E_TABLE position; D-M11 zero export falls to unrelated stem; D-M12 alias-opacity lost in a barrel (conditional OQ); D-M13 opacity grants callable authority at R4c (conditional OQ); D-M14 tuple identity reused at a different site/cache epoch |

**Re-plan additions (owner OQ1–OQ7 = a; `REPLAN-fable.md` §3–§5).** S1b-2 splits into **S1b-2a** (B-4–B-6, B-8,
B-10 with the leaf allowlist, plus **B-14** RP2-a header error → refuse and **B-15** RP2-b string brace → kept, both
quote forms, JSX and TSX; mutants B-M2–B-M5, B-M8, **B-M9** raw-text brace test, **B-M10** header errors sealed) and
**S1b-2b** (B-1–B-3, B-7, B-9, B-11–B-13, plus **B-16** `export { f as "g" }` / `import { "g" as h }` via
`ModuleExportName`, both quote forms and an escape, and **B-17** the cross-commit cache regression (§6); mutants
B-M1, B-M6, B-M7, **B-M11** naive quote trimming). S1b-2a also carries **B-18**, the strictness and Annex-B marker
unit rows (IMPLEMENTOR "S1b-2a dispatch").
S1b-3 adds **C-24** RP1-a/b/g computed keys (key → Outside, body → inner), **C-25** RP1-c merge partner and its
script-file twin (base under OQ1 a), **C-26** RP1-d2 enum member → not callable, **C-27** RP1-e `with` object → Exact,
**C-28** RP1-f dotted partner (base under OQ1 a), **C-29** a named class expression with a decorator (unproven), and
**C-30** a fail-safe mutant (one E-table row removed → the counter appears, the rows go base); mutants **C-M15** no
J3, **C-M16** no leave predicate, **C-M17** partner index ignores dotted names, **C-M18** `with` object treated as
body, **C-M19** enum body not a scope.

**r3 fold additions (at-cap; `REVIEW-r3-fold.md`).** All in S1b-3 and in both grammars unless marked TS-only;
expectations in `probes/S1b-controls-expectations-pre-run.md` addendum 7, measured on v9 (`probes/S1b-controls-proto-v9a.txt`):
- **C-31 (Opus W1, J2 by holder):** C152 member decorator of a named class expression referring to the class's inner
  name (N6) → drop (the inner name is not callable); C153 the same on a class declaration (N6b) → drop; C154 an
  anonymous class expression (N6d) → unchanged; C155 a class-node decorator on a named class expression (OQ7) → base,
  counted `decorated_class_expression`. Mutant **C-M20**: member decorators treated as Outside (killed by C152, C153).
- **C-32 (Opus W2, dotted namespaces; TS-only):** C156 `namespace A.B { B() }` → drop (not callable); C157
  `namespace A.B.C` with `C()` and `A()` → drop both. Mutant **C-M21**: non-first segments ignored (killed by C156).
- **C-33 (sol W1, the Annex-B predicate):** **C111 reversed** (a module: the outer call binds the outer function,
  Exact); C159 (TS script) and C160 (JS script) with no directive → base, counted `annex_b_strictness`; C161 a
  `"use strict"` script and C162 a function-level directive → Exact to the outer function; C163 a `.cjs` sloppy script
  → drop (the marker); C164 a `.cjs` block *generator* → Exact to the outer function (C163, C164 JS-only: there is no provably sloppy TS
  file, since prism does not parse `.cts` and a `.ts` script is Unknown). Mutants **C-M22** unconditional
  marker (killed by C111, C161), **C-M23** marker for generator/async declarations (killed by C164).
- **C-34 (Opus S4):** C165 `for (const { a = f() } of xs)` → Exact to the outer `f`, `for (const { f = f() } of xs)` →
  drop; C158 (TS-only) `enum E { 'f' = 1, g = f() }` → drop. Mutants **C-M24** `for_in_statement.left` unlisted (C165
  goes base and counts), **C-M25** member names compared by raw text (killed by C158).
- **C-35 closure:** the Rust unit test asserting `CLASSIFIED` and `E_TABLE` equal `probes/grammar_closure.py`'s derived
  tables (the probe is the reference; `python3 probes/grammar_closure.py --rust src/ast` must exit 0 in S1b-3's
  acceptance), plus C-30's one-row-removed mutant.
- **RP replay:** `probes/replan/replay_rp.sh <binary> <out>` (46 scenarios, the set asserted against
  `RP-SCENARIOS.txt`) must match the re-plan's "correct" column (Q39) at S1b-3 and S1b-4 heads.

**r4 additions (S1b-3 against the landed code; controls C166–C189 in `probes/controls_gen.py`, 287 scenarios;
reference summaries `probes/S1b-controls-proto-v12.txt` and `probes/S1b-controls-head-3961cc21.txt`).** Both grammars unless TS-only. With OQ10 (a) the
3a rows are unit rows on the collector (`js_ts_site_binding` / `js_ts_module_binding` results, as 2a's), and the 3b
rows are end-to-end resolution rows.
- **C-36 (3a, carry-forward 1, sol 2a-r2 W1–W3):** C166 `let f` in a default-parameter arrow, C167 inner `function f`
  in a sealed-error function, C168 class-expression inner name written from a computed key: the module `f` is
  `Callable` (D4 `VerifiedLocal`; head: `MayCall`, a NameOnly pair at the importer). Also C150 back to `Callable`
  and Opus 2a W2's mangled `f = ;` still `MayCall`. Mutants **C-M26** predicate (a) restored, **C-M27** predicate (b)
  restored, **C-M28** ERROR-adjacent identifiers not write targets (killed by the mangled row).
- **C-37 (3a, C-19 memo, carry-forward 2):** a unit test (inside `src/ast`, where the cache fields are visible)
  binds N sites of one file with one `JsBindingCache` and asserts `write_targets` is filled and `written` holds one
  entry per distinct `(scope, name)` queried; mutant **C-M29** no memo (the entries are missing). No test hook in
  production code.
- **C-38 (3a, owner M2 ruling, carry-forward 3):** C169 written `class f`, written `for (var f in o)` head, written
  parameter → `MayCall` (base Exact kept); unwritten twins → `not_callable` (C92, C136, C87). Mutant **C-M30** M2
  limited to declarators and functions (the r3 list).
- **C-39 (3a, OQ11 a):** `const [s, setS] = useState(() => 0)` → `MayCall`; `const [s] = compute()` →
  `not_callable`. Mutant **C-M31** pattern declarators skip M1.
- **C-40 (3a, carry-forward 8):** C172 declaration export with a sealed body error: both `f` and `{ f as g }` verify.
  Mutant **C-M32** containment tested against the sealer (killed by C172).
- **C-41 (3b, carry-forward 4 as narrowed 2026-09-29):** C170 a block-nested `const { f } = o` and a module
  bare-block `var { f } = o` → **Exact kept** (alias; S1b-1b's `b5` rows are unchanged); the bare-block `const` (P15)
  → Exact kept. Plus one R5 row per dropping reason (a parameter `not_callable`, `duplicate_declaration`,
  `import_parse_recovery`) and one `MayCall` row that keeps base at R5. Mutants **C-M33** R5 drop removed, **C-M34**
  R5 drop for every `Unproven` reason (the `with`/`parse_recovery` rows then drop).
- **C-42 (3b, carry-forward 9):** C173 `import { "\u{GG}" as h }` recovered as a top-level `ERROR` with a later
  `h()` and an `export function h` elsewhere → drop; a clean-import twin keeps R4c; **C179** `import.meta load)` and
  `import('./x') load2)` at top level → Exact kept (spec r1 Opus W3); **C181** Unicode aliases `é` (U+0301) and
  `a‌b` (ZWNJ) → drop (sol W2). Mutants **C-M35** no recovered markers, **C-M36** any `import` token marks (C179),
  **C-M37** ASCII-only word scanner (C181).
- **C-43 (3b, carry-forward 7, TS-only):** C171 `namespace N { export const { f } = o; }` with a module-level
  `run(){ f() }` → Exact `free_single` kept (kills S1b-1b's MX4 as well).
- **C-44 (3b):** `import f = M.g` (C141) and an unbound name with only out-of-scope same-file functions (C88, C93)
  → drop at R4; the same names keep base at R4c/R5.
- **C-45 (3a, owner 2026-09-29, the alias class):** unit rows for NoFn (each literal kind, nested object/array
  literals → `not_callable`; a shorthand, spread, method, identifier element → `Alias`) and for alias values
  (identifier, member, call without a function argument, `await`, conditional, logical, `new`); end to end (3b)
  C174–C177 and C182–C184 (pre-registered, addendum 9/9b): a hook-returned `t`, `ctx.make`, `{ g } = ctx` keep Exact;
  `const f = 0`, `{ k } = { k: 1 }` drop; `[m] = [1, g]` keeps Exact; C177 alias at R4 keeps `local_def`. Mutants
  **C-M38** alias treated as `not_callable` (C182, C177 fail), **C-M39** NoFn admits identifiers (C184 `m` drops),
  **C-M40** D4 maps `Alias` to base `Local` (an existing 2b refusal row changes).
- **C-46 (3a, spec r1 Opus W2):** C178 `export function g(a, f ==) { return f(); }` → refuse (drop at R4); C172 still
  verifies. Mutant **C-M41** containment against the delimited child for a parameter-list error (C178).
- **C-47 (3a, spec r1 sol W1):** C180 `const é = 0; é()` beside `function é` → drop; P1 preservation rows for the
  shared helper in both grammars: ASCII and non-ASCII declarators, parameters, shorthand patterns, writes, `$`
  names (S1b-1b), an escaped spelling left out. Mutant **C-M42** `is_plain_ident` restored (C180).
- **C-48 (3a, owner 2026-09-29 round 2, sol r2 W1):** C185 `const { missing: x = fallback } = {}`, C186 `const [y =
  fallback] = []`, C187 `const { z = fallback } = {}` keep base Exact `local_def`; C188 `const { f = 0 } = {}` keeps
  base (the conservative cut: any default is an alias). Unit rows for `js_ts_has_default` over object, pair, array and
  nested patterns. Mutant **C-M43** defaults ignored (C185–C187 drop).
- **C-49 (3a, sol r2 W2):** C189 `import /* c */ . meta`, `import /* c */ (…)`, `import // c` + `.meta` → Exact kept,
  both grammars; C173/C181 (recovery `ERROR`s are extras) still drop. Mutants **C-M44** raw-leaf scan (C189 drops),
  **C-M45** every extra skipped (C173, C181 keep Exact).
- **Existing tests to update (r4, measured on clean v11 and v12: 5 fail, all by design):** 2a's
  `b6_parse_recovery_refuses_unless_sealed` (C150 `MayCall` → `Callable`), `table_declarations_at_module_scope` and
  `table_typescript_value_space` (a D5 import → `JsBinding::Import`), and the two cache pins (§6). S1b-1b's
  `b5_nested_declarators_keep_base` now **passes unchanged** (its in-block rows are aliases).

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
| S1b-1b (F4, OQ8 a) | 91 (88 wrong R5 edges removed, 3 right `import_member` added) | 16 (wrong removed) | 0 | 5 (wrong NameOnly removed) |
| S1b-2a, S1b-2b | 0 | 0 | 0 | 0 |
| S1b-3 | 153 (134 re-targeted, 19 wrong removed) | 35 (1, 30 wrong, 4 parse-recovery right refused) | 0 | 868 (635, 182 wrong, 41 parse-recovery right refused, 10 right added) |
| S1b-4 | 0 | 4 (Exact → NameOnly, edge kept) | 0 | 0 |
| **S1b-3, r4 (clean v12 `0968ef78` vs `3961cc21`, audited, Q70–Q72)** | **198**: 134 re-targeted right; 64 removed: 58 wrong (2 `for` heads, 56 parameter rows) + **6 parameter rows right lost (accepted value-flow cost)**; 0 right lost otherwise | **52**: 1 re-targeted right; 47 removed: 45 wrong (1 `for` head, 44 parameter rows) + **2 parameter rows right lost (NameOnly)**; 4 right lost (E6) | **0** | **913**: 635 re-targeted right; 227 removed: 222 wrong (7 `for` heads, 7 unbound-at-site R4, 208 parameter rows) + **5 parameter rows right lost**; 10 right added; 41 right lost (E6) |
| S1b-3a alone (OQ10 a) | 0 | 0 | 0 | 0 |
| S1b-3b (OQ10 a) | the S1b-3 r4 row | the S1b-3 r4 row | 0 | the S1b-3 r4 row |
| S1b-4 r2, 2026-10-01 | **PENDING head measurement**; MEASURED base 19,219 sites / 15 R3 rows | **PENDING controller**; READ old forecast 4 demotions, no lost IDs | **PENDING head measurement**; MEASURED base 953 sites / 0 R3 | **PENDING head measurement**; MEASURED base 61,712 sites / 26 R3 rows (24 namespace, 2 named IO non-goals) |

**MEASURED / READ — S1b-4 r2 reconciliation:** the old S1b-4 row is an older-base
forecast, not current acceptance evidence. All three old expected JSON files are
empty and remain unchanged. No head binary exists yet; do not copy those empty
files to `S1b-4-r2-{X,R,T}.json` or call a base self-comparison a head result.
After the prototype build, use rowdiff.py on X/R/T and retain the complete base
and head dumps. Reconcile every deviation from the old 0 / 4-demotions / 0 / 0
row before dispatch. F is controller-only, with aggregates returned to this row.

READ (audit requirement): `audit_s1b4.py` inventories every changed row and
annotates lexical qualifier declarations; it cannot follow alias initializers,
function/hook returns, destructuring/defaults, parameter suppliers, callable
object members, merged namespaces or runtime writes. It therefore never calls
a removal wrong just because the qualifier is not callable. Run
`valueflow_guard_s1b4.py` on **complete dumps** to inventory every missing target
identity, including entire missing site keys; duplicate keys make comparison
inadmissible. Hand-audit each lost identity, plus every gained/retargeted row,
using the qualifier's actual value origin and the producer's exported value
flow. Record base/head tuple, source slices, flow path, alternative mechanism
ruled out, class and owner-cost citation. A zero-loss identity result is proof
that no target was removed independent of lexical assumptions; it is not proof
that retained edges are right. For nonempty losses, a zero-right-loss claim
requires this hand audit and cannot come from the lexical annotation alone.

   **r4 (2026-09-29, rounds 1 and 2):** the S1b-3 row supersedes r3's, v10's and v11's (v11 and v12 differ on 0
   corpus rows). Removals are parameters (X 62, F 46, T 213), `for` heads (X 2, F 1, T 7) and unbound-at-site R4
   rows (T 7); **no removed row has a declarator binding** (`removed_alias` 0, now including default-bearing
   patterns, Q71). **Parameter value-flow cost (owner-accepted, §0 (3)):** under the owner's rule (the parameter's
   function has exactly one in-repo call site, found syntactically by name, and it passes a base target defined in
   the caller's own file, bound lexically) X 6, F 2, T 5 rows are right edges lost; the supplementary all-suppliers
   reading adds X 5, F 2, T 10. The rest of the parameter rows are `passed_other` (X 22, F 9, T 43) or undetermined
   (several callers without the all-suppliers pattern, or unreadable: X 29, F 33, T 155), so the counts are
   **lower bounds**. The rows v10 removed
   through an alias keep base (X 38, F 424, T 212). `maycall_changed` 0. Expected files
   `probes/expected/S1b-3-r4-{X,R,T}.json` (regenerated from v12; F by hash in the evidence root).
   The S1b-1b row is measured against the S1b-1 head (Q45); with S1b-1b landed, S1b-3's expected row-diff is
   re-derived on the S1b-1b base before dispatch. Measured overlap of the F4 row keys with v9's S1b-3 row keys
   (upto1 → upto2): **0 on every corpus** (X 91 / 153, F 16 / 35, T 5 / 868), so the counts above are expected to
   hold; the controller still re-derives them rather than assuming.
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

**Re-plan caps (owner OQ3 = a; `REPLAN-fable.md` §5).** S1b-2 splits and S1b-3 carries the evaluation-context
table:

| Sub-slice | src cap / early stop | tests cap / report point | combined |
|---|---|---|---|
| S1b-1 | **60** / 54 (unchanged) | **200** / 180 | **260** |
| S1b-2a (collector, module terminal, B0 leaves, B1 folds) | **510** / 465 | **560** / 500 | **1,070** |
| S1b-2b (D4 wiring, R4c gate, `ModuleExportName`) | **140** / 125 | **200** / 180 | **340** |
| S1b-3 (site walk with the E-table, leave predicate, fail-safe, scoped writes, `local_binding`, `useCallback`) | **480** / 430 | **700** / 630 | **1,180** |
| S1b-4 | **185** / 165 (unchanged) | **330** / 300 | **515** |

**r3 measurement (v9, Q46; OQ9 open).** Prototype v9 is **1,448 src** lines: 26 S1b-1 (landed as 32 on its
branch), **2a 557**, **2b 123**, **3 533**, **4 177**, and 32 measurement-only switches that are not ported. The
attribution is per function and reproducible (`~/prism-evidence/s1b/planning/budget/fn_attrib.py` then
`slice_attrib.py`; mixed functions are split by hand-counted shares listed in the script). What the r3 fold added:
the Annex-B predicate (`js_ts_strictness` and the declare-walk plumbing, about 40, in 2a), the B1 structural-token and
delimited-child folds (about 25, 2a), the leaf allowlist (about 6, 2a), and the 74-row `E_TABLE` (88 lines, one row
per line after `rustfmt`, in 3). **2a and 3 no longer fit their caps; nothing is compressed to fit.** A design option
for S1b-3 (not compression): share one set of `E_TABLE` rows among the nine function-like kinds, which would save
about 40 lines; the closure probe would then compare the expanded table.

| Sub-slice | measured src | cap / early stop (re-plan) | **proposed (OQ9)** src cap / early stop | tests cap / report point |
|---|---|---|---|---|
| S1b-1 | 32 (landed) | 60 / 54 | unchanged | 200 / 180 |
| S1b-1b (F4, OQ8 a) | 1 (plus tests) | – | **15** / 12 | **120** / 100 |
| S1b-2a | 557 | 510 / 465 | **615** / 555 | 560 / 500 (unchanged) |
| S1b-2b | 123 | 140 / 125 | unchanged | **230** / 205 (B-17) |
| S1b-3 | 533 | 480 / 430 | **590** / 530 | **850** / 765 (C-31–C-35) |
| S1b-4 | 177 | 185 / 165 | **195** / 175 | 330 / 300 |

**r4 measurement (clean-built v12 `0968ef78` on `3961cc21`, Q73).** v12 adds **757 src** honest lines (4 of them
the two measurement switches, not ported): **3a 596**, **3b 161** (`budget/fn_attrib.py`, split by the §3.8
ownership). Round 2 added 14 to 3a (the default test 12, the trivia skip 2) and 2 to 3b (formatting). Against the
owner-approved caps (§0): 3a 596 × 1.10–1.20 landing growth = 656–715 against **700** (the upper end would stop at
the cap: an implementer reports at the early stop, 630); 3b 161 × 1.33 = 214 against **220**. The r4-round-1 text below
(v11, 582 / 159) is kept for its test-row basis.

**Calibration from the landed slices** (prototype → landed src; test estimate → landed tests): S1b-2a 557 → 612 src
(×1.10), 560 → 518 tests; S1b-2b 123 → 163 src (×1.33), 200 → 474 tests (×2.4); S1b-1b 1 → 16 src, 120 → 149 tests.
Tests are estimated from rows: 2a's unit tests cost 5.2 lines per row with both grammars (516 / 99, harness
included); end-to-end scenarios cost 25–45 lines (Opus r1 S1 uses 30).
- **3a (unit rows, new files; `js_binding_tests.rs` is 584 lines, spec r1 Opus S3):** about 120 rows — the walk,
  scope, write, M2-kind, M1-pattern, alias-class (C174–C177, C182–C184 at unit level and the NoFn table), B1 (C172,
  C178), recovered-import (C173, C179, C181), P1 (C180 and the shared-helper preservation rows: declarators,
  parameters, shorthand patterns, writes, `$`), E-table rows, **plus the C-5/C-6 E5 guards moved here as unit rows
  (Opus r1 S1's alternative)** — × 5.2 ≈ 625, plus the `E_TABLE` equality test and mutant support ≈ 45 → **≈ 670**.
- **3b (end to end), enumerated:** C-1 (2), one representative per `JsLocalBinding` outcome (Callable, MayCall
  `may_call`, MayCall `alias`, Unproven at R4, Unproven at R5, Position: 6), C-8 (1), C-18 (1), C-20 (2), C-41 (C170
  and one R5 row per dropping reason: 4), C-42 (C173 and a clean twin, C179 ×2, C181: 5), C-43 (1), C-44 (3), the
  alias pins C174–C177 and C182–C184 end to end (7), C-5's `useCallback` impostor twins (3), C114 (1) → **36
  scenarios × 30 ≈ 1,080**, plus counters (2 new maps), serde, epochs, pins and the cross-commit cache row ≈ 120 →
  **≈ 1,200**. Keeping the C-5/C-6 guards end to end instead adds 9 scenarios (≈ 270).

| Sub-slice (OQ10 a) | measured src | **proposed src cap / early stop** | estimated tests | **proposed tests cap / report point** | basis |
|---|---|---|---|---|---|
| S1b-3a (collector at every scope) | 582 (v12: 596) | **700** / 630 (**approved**) | ≈ 670 (+ round 2's C-48/C-49 unit rows ≈ 40) | **740** / 670 (**approved**) | src ×1.2; tests 120 rows × 5.2 + 45, +10 % |
| S1b-3b (call-site wiring) | 159 (v12: 161) | **220** / 200 (**approved**) | ≈ 1,200 (+ C185–C189 end to end ≈ 150) | **1,320** / 1,190 (**approved**) | src ×1.33 of 165; tests 36 × 30 + 120, +10 % |

Round 2 adds about 40 unit-test lines to 3a (defaults, trivia: 3a ≈ 710 of 740). In 3b, pin the round-2 behavior
end to end as **2 scenarios** (one default, one trivia; ≈ 60), giving 3b ≈ 1,260 of 1,320; pinning all of C185–C189
end to end (≈ 150) would reach ≈ 1,350 and cross the cap, so the rest stay unit rows in 3a.

The 2026-09-28 caps (3a 590 / 620, 3b 220 / 800) are superseded: 3a's src exceeds 590 by measurement (582 before the
×1.1–1.3 landing growth), and 3b's tests are re-forecast by enumeration (Opus r1 S1).

The E-table row-sharing option (r3, about 40 lines) is still open to the implementer as design, never as compression.

**ASSUMPTION — S1b-4 forecast, 2026-10-01, against landed 915fca43 (not a cap):**
src **260–340** without the OQ alias-preservation extension; **320–440** if that
extension is approved. Basis: namespace inventory + core proof 55–75;
extraction/enum/memo/counter integration 45–60; R3 routing + fallback 70–95;
shared export projection + E7 helper 55–75; cache/docs/pins 15–25; namespace-only
alias fact/traversal 60–100 if approved. Ranges include formatting/landing growth;
the earlier ~167 / 177 forecasts are historical and predate the landed APIs.
Tests **1,000–1,400**: 31 new control scenarios × roughly 25–35 honest lines
with a both-grammar harness = 775–1,085; missing older wrapper/nav representatives,
serde/epoch/cache/counter checks and E7 extension precedence add 150–250; the
conditional alias-barrel guards add 100–150. This is an enumerated forecast,
not a prototype recount. Recount after rustfmt on the prototype and replace
these ranges with measured attribution before the two-round review dispatch.
ASSUMPTION: one slice can converge because the scope/write grammar is already
landed and the new surface is this finite outcome/export/E7 matrix. If the
prototype reveals another mechanism or the forecast expands beyond that matrix,
record a recommended split in OQ-S1b4; do not independently authorize the split
or silently extend the two-round review cap. Numeric cap/checkpoint stop rules
in the historical budgets do not apply.

## 10. Risks

- **The collector's closure** remains the review surface. The table is keyed to the specification, every suspect
  grammar kind is classified by a checked probe, and B0 turns any unclassified kind into a refusal; the residual
  assumption is B1's (correct tokenization outside error nodes). **Re-plan (owner OQ1 = a):** the unit of
  proof becomes the position (§3.1a); an unlisted position preserves base under OQ1 (a), so a new position can no
  longer yield a wrong Exact from S1b. Base's own wrong Exacts at unproven positions remain (0 measured).
- **E5 leaves known wrong Exact edges** (subscription returns, loaders, styled callbacks, and multi-target may-call
  rows: X 14 multi-target rows stay). They are recorded for the may-call follow-up lane.
- **Grammar gaps** still cost right edges under E6 (F 4, T 41; E6b would recover T's 41).
- **Nav correlation for R3 additions** (C83) as r1 §10.

## 11. Review history

| Round | Verdict | Disposition |
|---|---|---|
| spec r1 (Opus) | FIX 9 WRONG / 4 SMELL | all folded (`REVIEW-r1-fold.md`); structural fold per both reviewers' convergence advice |
| spec r1 (sol) | FIX 6 WRONG / 2 SMELL | all folded; W6 in S1b-4; S8 a pinned non-goal |
| spec r2 (Opus) | FIX 2 WRONG / 3 SMELL | r1 all CLOSED. W1 namespace merging and S1 closure claim → the re-plan's E-table and leave predicate (§3.1a); W2 header errors → delimited-child sealing (§3.1 B1 note, 0 rows); S2 auditor tuple folded in `probes/jsscope.py`; S3 2a/2b split adopted (OQ3) |
| spec r2 (sol) | FIX 3 WRONG / 1 SMELL | r1 all CLOSED. W1 computed keys → J3 (§3.1a); W2 string-token braces → B1 note (auditor folded); W3 string-literal names → `ModuleExportName` in S1b-2b (OQ5); S4 B0 leaf kinds → §3.1a fail-safe |
| **cap** | controller: collector open-class | S1b-1 ships; S1b-2..4 re-planned (`REPLAN-fable.md`); one owner-approved round 3 on the re-planned packet (OQ4) |
| spec r3 (Opus) | FIX 2 WRONG / 4 SMELL, converging (collector findings 15 → 5 → 2) | W1 J2 by holder, W2 dotted segments, S1 cache schedule, S2 probe custody, S3 closure probe, S4 `for_in.left` + T10 StringValue: all folded |
| spec r3 (sol) | FIX 4 WRONG / 0 SMELL | W1 Annex-B predicate (C111 reversed), W2 closure probe second table, W3 probe custody, W4 cache schedule + counter: all folded |
| **cap (round 3 of 3)** | **disclosed at-cap targeted fold** (owner: "targeted fold, then implement") | Both reviews were converging, so per the convergence rule the valid fixes were folded without a round 4 (`REVIEW-r3-fold.md`): each fix is a closed, enumerable row with RED controls (C-31–C-35, C111 reversed) and mutants (C-M20–C-M25) that the sub-slice implementation reviews verify. Measured corpus change of the whole fold: 0 rows (Q42). The fold pushes 2a and 3 over their caps (OQ9, §9); S1b-1b (F4) is proposed (OQ8) |
| **r4 re-plan (2026-09-27)** | S1b-3 against the landed 2a/2b | prototype v10, 4-corpus audited row-diffs, 255 controls, carry-forwards 1–9, owner's M2/cache/budget rulings; OQ10–OQ12 (§0, §3.8, §9) |
| **S1b-3 spec r1 (Opus FIX 3/3, sol FIX 2/1) → fold (2026-09-29)** | round 1 of 2 | Owner narrowed OQ12 to provably-no-function bindings (alias class, §3.8 (11)); Opus W2/W3 and sol W1/W2 fixed; S1–S3 and sol S1 addressed; clean-built v11 re-measured, 0 right lost outside E6; `REVIEW-s1b3-r1-fold.md` |
| **S1b-3 spec r2 of 2 (the cap): Opus FIX 1/2, sol FIX 2/0 (open-class) → owner conservative cut (2026-09-29)** | disclosed at-cap fold | NoFn closed by the cut (defaults → Alias), trivia-skipping import check, parameters dropped on the not-statically-bound ground with the value-flow cost measured and owner-accepted (X 6, F 1, T 5, lower bounds), caps approved, file split planned; clean v12; `REVIEW-s1b3-r2-fold.md` |
