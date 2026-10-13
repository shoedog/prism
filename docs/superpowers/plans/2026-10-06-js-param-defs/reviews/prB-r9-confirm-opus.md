# PR-B R9 fold-confirmation review (Opus) — source head `96817370`, docs `5ddbf1d6` (PR #351)

Reviewer: Opus 5.5, read-only, 2026-10-10. One narrow round on the fix delta `2431cb10..96817370`.

**Goal this review serves:** whether plan PR #351 merges and `96817370` lands as PR-B. Contract: zero false Defs or edges; no LOST correct rows against main; Exact stays a static-binding grade; call sites, non-JS output and symbol/call/module navigation byte-identical; E13's extent stated honestly.

**Materiality rule (same as my final review):** MATERIAL = occurs in a measured corpus, or is triggered by a shape idiomatic in the code PR-B targets, or changes a number the owner decides on. IMMATERIAL = reproduced, but no measured and no realistic incidence. IMMATERIAL findings are recorded, never blockers.

## Verdict: FIX (closed, enumerable) — evidence and docs only; no product change is required

- **Product delta:** every fix closes its original reproducer. I found **no MATERIAL product defect** and no new false Exact edge. Four IMMATERIAL product findings are recorded (N1–N4); two of them (N1, N2) are small regressions introduced by the fixes themselves, each with zero incidence on X, T and SecBench.
- **Evidence rules:** two MATERIAL defects (E1, E2) in how the "inside type-annotated JavaScript" numbers are produced. Neither changes the gate result on the measured corpora (outside LOST CORRECT = 0 and ADDED WRONG = 0 both survive my re-score). Both change the numbers the owner re-affirms E13 on, and both are live in the controller's F helper, so they should be folded before the E13 re-affirmation and before the F run. Both are a re-score of saved details plus a wording change, not a re-run.

Corrected SecBench split (my re-score of `final/*/tables/details.jsonl`; it reproduces the reported table exactly before the two corrections):

| | ADDED CORRECT | ADDED WRONG | LOST CORRECT | LOST WRONG | LOST UNDECIDED | LOST INADMISSIBLE |
|---|---:|---:|---:|---:|---:|---:|
| Reported inside | 1,079 | 84 | 6 | 426 | 26 | 5 |
| Corrected inside (files with type-annotation diagnostics only; oracle verdict kept where the override proof is inadmissible) | 885 | 84 | **11** | 395 | 26 | 0 |
| Reported outside | 278,199 | 0 | 0 | 654,296 | 2 | 0 |
| Corrected outside | 278,393 | 0 | 0 | 654,327 | 2 | 0 |

Convergence note: product findings this round are fewer and smaller than last round (0 MATERIAL against 5), but N1 and N2 are new boundary instances created by the fixes. That is the open "syntactic predicate boundary" class. I recommend recording N1–N4 and not opening another source round for them.

## Fix table

| Finding | Status | Evidence |
|---|---|---|
| Opus F2 (early-error refusal on recovered parameter lists) | **PARTIAL** | All three original reproducers now equal main (`j(a: string, b: string)`, `t(ary: string[])`, typed constructor). Saved tables: 590 of 601 raw oracle-CORRECT losses restored; the independent census's 26 refused real callables are all gone. Residual, same family, inside type-annotated JS: N4. |
| Opus F3 (evidence rule) | **PARTIAL** | The allowlist is sound and conservative; the TS-AST census shares no product code; zero rows on any corpus were credited through the allowlist. Two defects remain: E1 (boundary is "any TS diagnostic", not "type annotations") and E2 (an inadmissible proof erases the oracle's CORRECT). |
| Opus F1 (JSX tag names) | **CLOSED**, with regression N2 | `<img>`, `<option>`…`</option>`, `<a>`, `<svg:path>`, `<t<string>>`: tag-name Uses gone in `.jsx` and `.tsx`, opening, closing and self-closing. `<Item>`, `<_c>`, `<$d>`, `<ctx.Provider>` (base identifier) kept. Named owner unchanged from main. |
| Opus F5 / sol W3 (wrapped `eval`) | **CLOSED** | 11 direct forms refused (`as`, `!`, `satisfies`, `<any>`, nested and composed, `eval<string>(…)`), wrapper inside a nested arrow refused; `(0, eval as any)`, `(eval!)?.()`, `(eval as any).call`, `window.eval!`, `new (eval as any)()` stay admitted. Named owner unchanged from main. |
| sol W1 (class receiver writes) | **CLOSED** | Field, static field, `accessor`, `#private`, static block, class declaration, arrow owner: the false Exact edge and the false kill are gone and the outer row is Exact. Computed keys, decorators, heritage and object-literal values keep the outer receiver (Node control: computed key writes outer `this`, field initializer does not). Named owner unchanged from main. |
| sol W2 (class self-name) | **CLOSED** | Declaration and abstract declaration, JS/TS; static field and static block reads fenced; heritage, class-expression, same-name formal in a nested block and unrelated captures unchanged. See S3 for an oracle blind spot this creates. |
| Opus F6 (labelled function, SEAM) | **CLOSED**, with regression N1 | The seam reproducer is refused; named seam owner equals main. |
| Opus F7 (duplicate formals) | **CLOSED** | Sloppy `async function`, `function*`, `async function*` equal main again; arrows, methods, strict, module and non-simple lists still refused. Node control on 14 shapes agrees. |
| Opus F8 (enum members) | **PARTIAL** | Identifier-keyed members (bare and assigned), `const enum`, nested arrow in an initializer: fenced. String-literal keys and merged declarations are not: N3. |
| Opus F9 (type predicates) | **CLOSED** | `v is T`, `asserts v is T`, `asserts v`, same-line and multi-line; named functions and methods keep main's rows. All 62 certificate rows are F9 changes (checked independently, below). |
| Opus F10, F11 | recorded, unchanged | As the report states. |

**The 62 certificate rows.** I re-derived them from source bytes without the worker's classifier: all 34 removed endpoints sit at `): NAME is …` (a type-predicate parameter name); all 28 added endpoints share Def and line with a removed row and are not followed by `is`. The three removed-only rows on X are multi-line predicates with no value read on that line. None is a non-F9 change.

---

## MATERIAL findings (evidence)

### E1 — WRONG / MATERIAL — "inside type-annotated JavaScript" is decided by any TypeScript diagnostic, so it includes files with no type annotations

**State → incorrect result (static; re-scored from saved evidence):** `independent-census.cjs` and the controller's `probes/r9-admissibility.cjs` put a `.js`/`.jsx` file inside the boundary when `parseDiagnostics` or `getSyntacticDiagnostics` is non-empty. That is "TypeScript cannot parse this file cleanly", not "this file has type annotations".

- 88 of the 541 SecBench files inside the boundary have no "can only be used in TypeScript files" diagnostic (TS8xxx) at all.
- **225 of the reported inside rows (194 ADDED CORRECT, 31 LOST WRONG) sit in 17 such files across 11 roots.** Examples: `pdfinfojs lib/pdfinfo.js` (octal escape `"\033[31m"`, valid sloppy JavaScript), `crud-file-server.js` (octal literal), `jquery_1.11.0 src/intro.js` (a file fragment), `buns` (`#!namespace` preprocessor lines), `htmlparser utils_example.js` (`var class`).
- None of the 225 is gate-relevant. All 84 ADDED WRONG, all raw-CORRECT LOST and all UNDECIDED inside rows are in React Native files that do carry TS8xxx diagnostics; aurelia's 10 LOST WRONG are in a TS8010 file. I also checked the census: only 2 disagreements (both TS-only callables in file fragments) were waved through as "E13" in a file without annotations.

So nothing was hidden, but the label overstates the type-annotated population, and the exemption is wider than the acceptance rule in SPEC §8 E13 ("outside Flow-annotated files"). On F, an ADDED WRONG or LOST CORRECT row in a plain JavaScript file with one octal escape would be exempted.

The boundary is independent of the product (TypeScript parser, not tree-sitter). `.mjs`/`.cjs` files are never inside; that errs toward gating.

**Fix (recommended):** inside = the file has at least one TS8xxx "can only be used in TypeScript files" diagnostic. Re-score from saved details (table above) and correct SPEC §8 E13 line 106 and MEASUREMENTS-prB-R9. **Alternative:** keep the rule and rename the class "files TypeScript cannot parse cleanly", stating the 225 rows and 17 files, and have the owner accept that wider exemption explicitly.

**Self-critique — when it would not apply:** if the owner's intent for E13 is "any file the oracle cannot parse is out of scope", the current rule matches that intent and only the name is wrong. Three of the 17 files are valid JavaScript that Node runs, so I do not think that reading holds for them.

**Classification:** CLOSED INSTANCE of F3 (evidence boundary definition).

### E2 — WRONG / MATERIAL — an inadmissible override proof replaces the oracle's verdict, and an outside INADMISSIBLE row does not STOP

**State → incorrect result (static: `measure.py` `tables()`, `probes/r9-admissibility.cjs`, `final/redos__react-native_0.63.0-rc.0/tables/overrides.json`):** for a LOST row in a product-refused owner, if Node cannot compile the owner's text and the message is not allowlisted, the row's verdict becomes INADMISSIBLE. The oracle's own verdict is discarded. The STOP rule fires only on `outside|LOST|CORRECT`.

- Measured effect: 5 rows, all inside. They are `onHoverOut` and `delayHoverOut` in `Libraries/Pressability/Pressability.js:583-593`: real local value flows that main has and head loses, oracle-CORRECT. The honest inside LOST CORRECT count is **11, not 6**. REPORT.md says this in prose; the tables and the brief's summary line do not.
- These 5 are not the misparse class E13 describes. The 6 counted rows are (type names as Defs in `splitLayoutProps.js`, an owner named `string`, a `type` alias as a Def). The 5 are lost because PR-B's own early-error refusal fires on a parser artefact (N4b).
- A failed compile of `if (this._isHovered) {…}` as an expression says nothing about whether main's row was right, so it cannot move the row out of CORRECT.
- Gate hole on F: every typed callable in a `.ts` file fails `new Function` with `Unexpected token ':'`. A LOST CORRECT row in a refused owner in TypeScript would be counted INADMISSIBLE and would not STOP. Outside INADMISSIBLE is 0 on X, T and SecBench, so no measured number is affected.

**Fix (recommended):** an inadmissible proof leaves the oracle verdict in place; report the count of failed proofs beside it; STOP on outside raw-CORRECT LOST unless the proof is EARLY_ERROR. **Alternative:** keep the INADMISSIBLE column but STOP on any outside INADMISSIBLE row, and state "11 raw oracle-CORRECT" wherever "6 CORRECT" appears.

**Self-critique — when it would not apply:** nothing was concealed, and on the measured corpora the gate result is identical under either rule. If N4b is fixed in the product the 5 rows disappear and only the F-side rule matters.

**Classification:** CLOSED INSTANCE of F3 (an override that does not discriminate its claimed mechanism).

---

## IMMATERIAL findings (product) — recorded, not blockers

### N1 — WRONG / IMMATERIAL — the F6 label fix makes a labelled body-level function declaration fence its same-name formal in non-seam callables; a named owner loses rows main has

**Input → incorrect result (reproduced: head, `2431cb10` binary, main):**

```js
function N(y) {
  lbl: function y() {}
  use(y);
}
```

Main and `2431cb10`: `N| y (param) -> y@L3 exact` plus the zero-width L2 row. Head: **0 rows**. The unlabelled twin (`function y() {}`) keeps both rows on all three. The `.ts` form and the synthetic form (`reg(function (y) { lbl: function y() {} use(y); })`) behave the same. Node control: `(function K(y){ lbl: function y(){}; y = 5; return arguments[0] })(1)` returns 5, so the formal and the function share one binding.

**Mechanism (static):** `js_ts_block_statement_binds` gained a `labeled_statement` arm (`src/ast_callback_identity.rs:725-727`). `js_ts_scope_binds`' `statement_block` arm (`:701-718`) skips body-level function declarations for a callable body by kind, and a labelled one has kind `labeled_statement`, so the body block now looks like a nested block scope that binds `y`. The formal's scope is the callable, the Use's scope is the block, and the fence fires.

**Fix (recommended):** peel label chains before the `callable_body` kind test in the `statement_block` arm, and add a rows assertion to the non-seam half of `r9_labelled_body_function_is_a_seam_binding` (it asserts the predicate only). **Alternative:** record and disclose.

**Self-critique:** labelled function declarations are sloppy-only Annex B syntax. The certificate shows no changed row outside its classes other than the 62 F9 rows, so incidence on X, T and SecBench is zero. It is, though, a LOST binding-correct row against main in a named owner in valid JavaScript.

**Classification:** CLOSED INSTANCE of the SEAM predicate boundary (regression introduced by the F6 fix).

### N2 — WRONG / IMMATERIAL — the F1 fix treats a member-expression JSX tag as a non-reference when its text starts with a lowercase letter

**Input → incorrect result (reproduced, `.jsx`/`.tsx`; head against the `2431cb10` binary):**

```jsx
register(function () {
  this.Comp = pick();
  return (
    <this.Comp />
  );
});
```

`2431cb10`: `this.Comp@L2 -> this.Comp@L4 exact` (correct; TypeScript emits `React.createElement(this.Comp, null)`). Head: 0 rows. Same for `ui.box = pick(); … <ui.box />` and `props.icon`. Base-identifier Uses are unaffected (`const ctx = make(); … <ctx.Provider>` keeps its rows).

**Mechanism (static):** `js_ts_node_is_non_reference` (`:898-924`) is also called with the `member_expression` node from `collect_fenced_path_refs` and `js_ts_span_is_non_reference`. That node is the element's `name` field and its text (`this.Comp`) starts with a lowercase letter, so the intrinsic-tag arm matches.

**Fix (recommended):** require `node.kind() == "identifier"` in the JSX arm; add a path-level control (the test's `<a.B />` control checks only the identifier span). **Alternative:** record.

**Self-critique:** this drops a correct row that `2431cb10` added. It is not a false edge and not a loss against main (main has no synthetic pass). It needs a Def of the member path in the same callback, which is not idiomatic. X shows no such change.

**Classification:** CLOSED INSTANCE of E7 / F1 (over-correction).

### N3 — WRONG / IMMATERIAL — enum fence misses string-literal member keys and merged enum declarations

**Input → incorrect result (reproduced, `.ts`):** `reg(function (y: number) {\n enum E { 'y' = 1,\n z = y }\n use(y);\n});` and `… enum E { y = 1 }\n enum E {\n z = y } …` → head still emits `y (param) -> y` at the initializer, NameOnly. TypeScript 5.9.3 emits `E["z"] = 1` for both, and running the emit with the formal set to 7 gives `[1, 7]`: the initializer reads the member.

**Mechanism (static):** the `enum_body` arm (`:668-682`) compares decoded identifier text, so a quoted key never matches, and it looks only at the enclosing body.

**Fix:** strip quotes from string keys; collect members across same-name sibling enum declarations. **Alternative:** record. **Self-critique:** a local enum inside a callback that shadows a formal through a quoted key or a merge; NameOnly; no measured incidence. **Classification:** CLOSED INSTANCE of the E3 fence / F8.

### N4 — WRONG / IMMATERIAL (inside E13) — the early-error refusal still fires without positive proof in recovered regions the parameter-list guard does not cover

**(a) Recovered body (reproduced; head and `2431cb10` both drop it, main has rows):**

```js
function f(key, value) {
  const map: {[key: string]: value} = {};
  map[key] = value;
  return map;
}
```

Main: 6 rows. Head: 0 rows, owner refused. The Flow annotation is recovered as an `object_pattern` that "declares" `value`, and the lexical-redeclaration clause (`:106-120`) fires. `function f(type) { const x: Map<string, type> = make(type); … }` behaves the same. The guard at `:93-102` looks at the parameter list and at ERROR ancestors only.

**(b) Parser-artefact owners (static, from saved evidence plus a parse probe):** React Native still has 22 refused owners on head, 20 named `if` and 2 named `Error`. `Pressability.js:580` `if (this._isHovered) {` is parsed as a `method_definition` whose "formal" is a member expression, with no ERROR in its parameter list or ancestors. Its 5 lost rows are the E2 rows.

**Fix (recommended), with measured yield:** require an error-free file (`!root.has_error()`) before any early-error refusal; the R7 premise "tree-sitter accepts semantic early errors" only holds where the parse is otherwise trustworthy. Yield on the measured corpora: exactly the 5 rows of (b) come back; no other LOST row sits in a refused owner. `owner.has_error()` is a narrower alternative that covers (a) only. **Alternative:** disclose under E13.

**Self-critique:** both are inside type-annotated JavaScript. (a) needs a formal whose name equals a type name in a body-level annotation, and the independent census finds no real refused callable left. I rate the product side IMMATERIAL; the disclosure side is E2.

**Classification:** CLOSED INSTANCE of the R7 family / F2.

## SMELL findings

- **S1 — SMELL / IMMATERIAL — test gaps on new code paths (static).** (i) The ERROR-ancestor walk at `:96-102` has no test and no mutant; `r9_recovered_parameters…` returns at `params.has_error()` first, and `node.is_missing()` on an ancestor can never be true. (ii) `r9_jsx_intrinsic_tags…` uses the formal `img` for the `<svg:path />` case, so the `jsx_namespace_name` arm is not exercised (my probe with a formal named `svg` shows the arm works). (iii) The bare enum member arm and `asserts v` are untested (both work). (iv) The negative controls behind N1 and N2 check a predicate or an identifier span, not rows. CLOSED INSTANCE (test adequacy).
- **S2 — SMELL / IMMATERIAL — the allowlist omits two specific genuine messages (reproduced, Node 26).** `((a.b)=>1)` → `Invalid destructuring assignment target`; `function q([a.b]){}` → `Illegal property in declaration context`. Both are what the product's invalid-binding-target clause refuses, and both land INADMISSIBLE. That is the safe direction, and no corpus row is affected (0 EARLY_ERROR rows anywhere). Add them if E2's stricter STOP is adopted, to avoid false STOPs. CLOSED INSTANCE of F3.
- **S3 — SMELL / IMMATERIAL — the oracle cannot see the class inner binding, so a W2 removal reads as LOST CORRECT (reproduced).** `export function F() { class C { static make() { return new C(); } } let c = C; c = C.make(); return c; }`: main has two alias-twin rows `C@L4[c] -> C@L3` and `C@L5[c] -> C@L3`, NameOnly; head removes them. TypeScript gives a class one symbol, so `adjudicate.cjs` would score the removed rows CORRECT. Zero such rows on X, T and SecBench. If F reports LOST CORRECT in a named owner at a class declaration's own name, this is the cause, and it needs a Node identity control, not a product change. NEW FAMILY (oracle blind spot), evidence only.

Unchanged and pre-existing, for the record: `(/* c */ eval)("f = 2")` is still admitted (Node: direct eval, formal becomes 2); same on `2431cb10`. CLOSED INSTANCE of the EVAL predicate boundary, IMMATERIAL.

---

## Priorities, answered

1. **Each fix closes and nothing adjacent regresses.** Closed as tabled. Adjacent regressions: N1, N2. Nothing else moved in about 140 fixtures run against head, the `2431cb10` binary and main.
2. **F2 guard over-reach.** In error-free plain JavaScript every genuine early error I tried is still refused (`(a, a) =>`, `let a` over a formal, `'use strict'` with a default, `function (a.b)`), also when a parse error sits elsewhere in the file. The guard only fires where tree-sitter failed inside the parameter list or an ancestor, which means the file is not valid JavaScript or is type-annotated. There, named owners return to main's rows and synthetic owners are admitted: `reg(function (a: string, b: string) { … use(string) })` now gets a pass with `string` as a formal. That is the cost the report states (React Native ADDED WRONG 82 → 84).
3. **New false Exact or LOST-correct rows outside type-annotated JS from this delta.** No new false Exact found. LOST against main: N1 only. The new rows the delta adds (computed method keys, decorators, anonymous sloppy `async function (a, a)`, F9 same-line reads) are correct.
4. **Evidence rules.** Allowlist: strict enough (a typed callable fails at `:` first; the redeclaration pattern is anchored; either wrapper compiling defeats the claim) and incomplete only in the safe direction (S2). Boundary: independent of the product, too broad (E1). Split honesty: E1 and E2. Sample: 16 of the 84 ADDED WRONG rows and all 6 LOST CORRECT rows are the E13 misparse class (type names as Defs or formals; real flows under an artefact arrow owner). The 5 INADMISSIBLE rows are not (E2, N4b).
5. **Tests and mutants.** Each of the nine groups has a negative control. `red-corrected.log` shows 0/9 on `2431cb10` and `green-final.log` 9/9 on R9; my probes show the matching behaviour difference between the two binaries for every group. The ten mutants each switch one fix off and are killed by that fix's group. They are meaningful as regression pins and do not pin secondary arms (S1).

## Custody and probe log

- `STOP-source-manifest.json` (727 files) equals `git archive origin/wip/js-param-defs-prB-r3` byte for byte. I built that archive (`cargo build --release` plus `byte_dump.rs` and a parse probe as examples); the frozen `prism-head-r9-bytes` and my build agree on all 143 fixture directories.
- Inadmissible probes, fixed before any conclusion: `nav dfg-stats --edges --no-cache` (the flag is not accepted there; I used the byte dumper); my first navigation comparison passed each query as one unsplit argument and both binaries printed usage (re-run under bash).
- Navigation on a fixture repo covering the delta's shapes: `callers`, `callees`, `symbol-spans`, `ego`, `repo-map`, `module-deps`, `functions`, `nodes-at` byte-identical between head and main with no `<cb@`; `call-stats` differs only in its nested `dfg_labels` counters.
- Gate logs read, not re-run: nextest 5,241 passed / 1 skipped; mutants 103/103 KILLED; clippy 235/235 equal multiset; Tier-A matrix cells `ok`. `final/summary.json` matches every number in the brief (578 admitted).
- Scratch build, `target/` and fixtures deleted; the clone is clean on `review-r9`. No git writes. No `frontend-portal` access. Node ran only my own snippets and the pinned TypeScript library; no corpus package was executed.

## What I did not check

- **F** — never opened.
- Gates not re-run: nextest, the nine R9 tests, mutants, clippy, fmt, Tier-A matrix and quick, O1, identity and nav on X/T, perf, MCP.
- Byte tables not re-captured or re-adjudicated. Everything about X, T and SecBench rows comes from the worker's saved `final/` evidence, which I re-scored and sampled but did not regenerate.
- The 84 ADDED WRONG rows beyond a sample of 16; the 426 LOST WRONG and 26 UNDECIDED inside rows; the 2 outside UNDECIDED rows.
- The census disagreement classification beyond its script, its totals and the files without annotations; `strict_context.cjs` beyond a read; the 763 + 200 matrix cells and the Node control files.
- The three items the brief lists as open bookkeeping (the 62-row class, apart from confirming every row is F9; the escaped-directive helper; the `cejs` re-measure).
- The seven excluded roots and `cejs`.
- Cache-version coupling (PD11 / P2-M11) beyond reading the diff; `mutants/lane-p-tsconfig-paths.json` beyond its one-line change.
- F10 and F11 (unchanged follow-ups).
