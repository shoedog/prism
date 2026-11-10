# JS/TS, React and Node: known gaps and edge cases

**Status:** living list, last updated 2026-10-31 (coverage pass against the defect/CVE taxonomy; plan and order in #604).

Each entry says:
- what prism does today, with a small example;
- whether the result is a **wrong** edge (an edge, often Exact, to a function the call does not reach) or a **missing** one (no edge, i.e. prism fails closed);
- where the entry is tracked.

`Exact` is a static-binding grade, not a runtime claim; see the call-resolution section of `CLAUDE.md`.

The variant matrix in `tests/fixtures/js_alias_matrix/` is the executable form of many entries. It holds one row per binding shape, each with an authored truth and today's verdict. Its `WRONG`, `OVERCLAIM` and `MISSING` rows are known gaps of this kind, and its README explains the verdicts.

Bare calls cannot gain Exact from an object/class method property name (#443),
including accessors, function-valued properties, member assignments, and class
fields (also in anonymous/default-exported classes). For example,
`var f = make(); const o = { f() { return f(); } };` declines the method self-edge.
The outer value may remain unproven for `var`/`let`; proven `const` returned values,
imports and outer function declarations retain their binding routes. Existing
NameOnly candidate pools remain unchanged. Inferred-name origins come from the
same classifier as `Language::function_name`. A same-named declarator whose value
is the member assignment, such as `var f = (Module.f = function() {})`, retains
its lexical target; only parentheses are transparent for this exemption.
Regression shapes are in
`tests/integration/js_method_name_binding_test.rs` for JS, TS and TSX.

An explicit function-expression name is visible only within its own parameters and body (#450).
For example, `var f = make(); const o = { p: function f() {}, n() { f(); } };`
cannot gain Exact to the expression from the outer call. Recursive calls and
nested callables within the expression retain the self binding; a proven
outer callable declaration or import keeps its existing resolution route.
Spanless import/re-export and namespace spelling fallbacks exclude private expression names; span-verified exports still expose their callable value.
`let t7; t7 = function t7() {}; t7()` drops, while its anonymous twin stays Exact via the inferred name.
Inferred names are unchanged. Parameter/body spans survive full and partial CPG cache hits; regressions
are in `tests/integration/js_named_function_scope_test.rs` for JS, TS and TSX.

## Anonymous callers (lane A2 S2a)

Calls inside unnamed arrows, function expressions and generators now have positional
`<anon@LINE:COL>` owners. Calls already claimed by a named owner at the same byte
span retain that owner, including class callbacks. Default enclosing-function seeds
still select named owners. A callable with a recognized parameter early error keeps
its call-graph node and has no DFG rows. Explicitly named generator expressions
retain their earlier inventory behavior; generator declarations keep their existing
owners. The new anonymous generator inventory does not change default enclosing
seeds.

Ordinary module-level calls (#437), including initializers, export assignments
and top-level control flow, use the file-bound `<anon@module>` identity. The same
named byte claims, innermost callable ownership and A2 Exact barrier apply.
Navigation presents module execution as a `module-level` statement; function
seeds and default visibility still hide it. A top-level class's decorators and
`extends` expression run during module evaluation and are module-owned, matching
how a class inside a function credits them to that function. Class fields, static
blocks, member decorators and computed keys await an evaluation-region policy. Constructors, `.mts`/`.cts`
admission and the source file-size cap remain unchanged.

Anonymous Exacts require source-backed binding and environment proof. Unsupported
receiver/class, CommonJS, import-equals and script-global authority routes lose
Exact; the named-owner resolution behavior described below is unchanged. Arrow `this`
inheritance is S2b; anonymous invocation edges (including IIFEs), metadata and display
labels are S3. The S1 duplicate named-claim extractor and S5 block-blind inherited
locals remain unchanged; the latter can lose recall.

Receiver/class certificates are deferred (owner decision, 2026-10-16). The census
(`~/prism-evidence/js-anon/census-recv-certs/`) measured how many sites would become
Exact if the barrier admitted these routes on B0 alone:
- typed parameters, including local destructured Props: X 17 product sites; T 1 test site;
- constructor locals: X 18 test sites; T 629 test sites, all calls to four methods of one
  test class (`TestSession`);
- class qualifiers, `this` and typed fields: none on either corpus.

The class-qualifier route would also re-admit known-wrong fixture Exacts: static vs instance
slots, and class-expression self-shadowing. These sites stay NameOnly, or are dropped when
there are several candidates.

Build cost (S4, #419 and #420): on a quiet-ish host, a cold `call-stats` build on X is
about 10–14% slower than before anonymous owners existed, and T is at or below its old
time. Peak RSS is unchanged by S4. The remaining known per-owner cost is that call-graph
uses of `node_for_id` re-scan the file's candidates (`source_before`, receiver refresh).
This is a recorded follow-up, not scheduled.

The B0 eval veto folds whole computed names in subscripts and destructuring keys:
a proven `eval` name vetoes on any receiver; an unknown name vetoes on a host or
proven const host alias. A pattern's direct initializer or assignment RHS supplies
its receiver; nested property values, parameters and loop heads have unknown
receivers. Plain `{eval: e}` and shorthand `{eval}` already trigger the existing
identifier-spelling veto.
Receivers given only as a pattern or parameter default are not followed, like conditional receivers.

## 1. What a called name is bound to

prism resolves a bare call `f()` or element `<F />` by asking what the nearest binding of the name is (`src/ast/js_binding*.rs`).

### 1.1 Shapes that are and are not followed

| Binding of the called name | Today | Tracked |
|---|---|---|
| `function f() {}`, `const f = () => {}`, `const f = useCallback(() => {}, [])` | Bound to that function | — |
| `const x = call(() => …)` with any other call taking an inline function | Bound by name to the inline function (`local_def`, Exact). Right for `debounce`-like wrappers; **wrong** when the call returns something else: `const H = React.lazy(() => import('./H'))` binds `<H />` to the loader arrow, and `useMemo(() => () => …)` binds the factory | not tracked |
| `import { f } from './x'`, `import f from './x'`, `import * as N`; `N.f()` | Bound through the module's export facts | §2 |
| `const h = f`, `const h = N.f`, `const { f: h } = N` (flat pattern), chains of these up to 8 links | Followed to what they name (#372); a longer chain is not followed | — |
| `let`/`var` aliases, a binding written later, `c ? f : g`, object-literal and array members, patterns with a rest, nested, computed-key or default entry | Not followed; falls back to name search (§1.2) | refusal slice, §1.2 |
| `const { save } = useEditor()`, `const [v, toggle] = useToggle()`, `const h = makeHandler()` | Followed one step when `useEditor` is exactly one in-repo function whose every value-returning `return` gives an object or array literal (or a `const` bound to one) holding a provable callable at that key or slot; otherwise the spelling fallback of §1.2 | — |
| `const { f } = require('./x')` at the top level of a CommonJS file | Bound through the module's CommonJS exports (`import_member`) | — |
| `const m = require('./x'); m(…)`, `m.f(…)`, `require('./x').f(…)`, `const f = require('./x').f; f(…)` with the `require` bound by a top-level `const`, `let` or `var` | Bound Exact (`import_member`) when the producer's CommonJS export is a declared function or a `{ f }` / `exports.f = f` member; a named caller takes `const`, `let` or `var`; an anonymous or module-level caller (#597) takes only a direct top-level `const` or `var` (and `const`/`var { f } = require('./x')`, `{ k: f }`) declared once in the resolved program scope (hoisted declarations included), with no write resolving to that binding and no shadowing between the site and the program scope (#598, #597) | `let` bindings and inline `require('./x').f(…)` from anonymous callers keep the earlier outcome |
| `const { f } = require(…)` inside a function, `await import('./x')` | Not followed (§1.2) | #436 |
| `import x = N.y` (TypeScript) | Not followed; resolved by the repository-wide name search as if unbound (§1.3), so a same-named function elsewhere can get a **wrong** Exact edge | not tracked |
| A parameter: `function g(cb) { cb() }` | Dropped (`UnknownName` or `LocalBindingUnproven`) | argument following, planned at NameOnly |

### 1.2 Name-search fallback for a binding whose value is not proven (wrong edges)

When the nearest binding is a declaration in the file whose value prism cannot prove, resolution falls back to spelling:
1. a function of that name in the same file (`local_def`, Exact), at any scope;
2. otherwise, only when the binding is declared in an *enclosing* function (the call is in a nested function), the one function of that name anywhere in the repository (`free_single`, Exact), or several of them (`free_multi`, NameOnly);
3. otherwise, when the binding is in the calling function itself or at module scope, the call is dropped (`UnknownName`) by the lexical guard.

The result is often right, because a value usually carries the name of the function it holds, but not always:

```js
export function a() { let shared = () => 0; /* … */ }
export function b() {
  const shared = thing();                  // value unknown
  return function inner() { return shared(); };   // Exact → the arrow `shared` inside a()
}
```

The older textual alias path (`h = f` read from source text, "Level 1" in `compute_indirect_call_sites`) has the same weakness: it does not look at scopes.

**Plan (owner, 2026-10-13):** after the value-following slices, such sites are demoted to NameOnly instead of Exact (JS, JSX, TS, TSX only; other languages unchanged). A follow-on slice then measures which demoted edges value following can prove, and restores those to Exact. Tracked: #376 item 8; branch `feat/js-bound-name-refusal`.

### 1.3 Names with no binding in the file (script globals)

A name with no binding in its file is resolved by a repository-wide name search, even from an ES module to a function another module does not export:

```js
// app.js (a module)
export function f() { return helper(); }   // Exact → helper() in other.js
// other.js
function helper() { return 1; }
```

That is right for classic `<script>` globals and wrong for modules. **Left as is for now (owner, 2026-10-13).** The fix is a declared-globals model, so that only real script-file declarations bind across files; estimated at 250–450 lines plus tests. Tracked: #376 item 7.

### 1.4 Declarations a JS engine rejects (early errors), deferred

JavaScript has early errors: rules the engine checks before running anything; if one fails, the file never runs. Several concern declarations:
- a name declared twice in one scope;
- `eval`, `arguments` or a reserved word (`let`, `static`, `package`, `enum`, …) used as a variable name in strict code (every ES module and class body is strict);
- a parameter or `catch` variable redeclared with `let`/`const` in its own body.

Tree-sitter checks only the grammar, so prism treats such a file as normal and binds through these declarations:

| Code | main before #372 | after #372 |
|---|---|---|
| `const g = () => 1, eval = 2; g()` | Exact → `g` | same |
| `catch (h) { const h = () => 1; h() }` | Exact → `h` | same |
| `function p(h) { const h = () => 1; h() }` | dropped (two declarations of `h`) | same |
| `const { f, x: eval } = N; f()` | dropped | Exact → the module's `f` |
| `const k = imp, eval = 2; k()` | dropped | Exact → the imported function |

The edge names what the code says the name is. A called name declared twice in the same scope (including a parameter redeclared in the function body) is refused; a `catch` variable redeclared in its block binds to the block's declaration (row 2). Code that builds never contains these errors, so they appear only in code being edited (for example the MCP server on a working tree) or in a broken change under review.

**Options, if this comes back** (decision deferred by the owner on 2026-10-13):

The rules, taken from the ECMAScript early errors for declarations (a closed list, not a list of probes):
1. no name bound twice in one `let`/`const` declaration, across declarators and patterns;
2. no `let`/`const`/`class` name declared again in the same scope by `let`, `const`, `class`, `function`, `var` or an import;
3. no top-level `let`/`const` in a function body that redeclares a parameter, and none in a `catch` block that redeclares the caught variable;
4. no `let` as a `let`/`const` name; in strict code no `eval`, `arguments` or strict reserved word (`implements`, `interface`, `package`, `private`, `protected`, `public`, `static`, `yield`), and in modules no `await`.

TypeScript reports the same errors. The three places the rules could apply:

| Option | Applies the rules to | Effect | Rough size |
|---|---|---|---|
| (a) | every declaration the alias and returned-value routes follow | such a declaration is not followed; the call site keeps the base result; existing behavior unchanged | 200–350 lines plus a table test (one negative per rule, valid controls) |
| (b) | (a), plus the base binder | the existing Exact edges in such files (rows 1–2 above) also stop; needs measuring | (a) plus a corpus re-measure |
| (c) | the whole file | a file with any such error is treated like a file with a parse error, so findings that rely on it drop from `asserted` to `candidate` through the existing parse-quality mechanism; the only option that tells users the file cannot run | a pass over every declaration of every file |

A note in `CLAUDE.md` saying such code is out of model was considered and rejected: it informs agents working on prism, not prism's users or its output. Tracked: #376 item 6.

## 2. Imports and modules

- **Packages.** Bare specifiers resolve only through tsconfig `paths`. Workspace packages (`package.json` `exports`/`main`/`types`), npm packages and `node:` builtins are external: no edge. Lane PKG, parked: #352.
- **Imported classes and objects as qualifiers.** `import { C } from './m'; C.m()` and `new C()` get no Exact edge. Lane S2, parked: #353.
- **Relative import resolution** (all recorded in #371):
  - wrong: when the namespace export proof fails, the fallback matches files by stem and grades eligible spelling matches Exact (private function-expression names excluded), so `N.f()` can bind `other/x.ts:f`;
  - missing:
    - a barrel conflict check that counts a forwarded type-only import;
    - an inherited method reached through an imported props type;
    - bare-dot forms other than named imports and re-exports;
    - `.js`-suffixed hops inside the `paths` lane;
    - `.jsx` → `.tsx`, `.mjs` → `.mts` and `.cjs` → `.cts` for named imports;
  - model limits:
    - a directory specifier always resolves to `index.*`, not to `package.json` `main`;
    - with both `x.js` and `x.ts` present, `./x.js` binds `x.js`, whereas TypeScript binds `x.ts`.
- **CommonJS.**
  - `module.exports = …` and `exports.x = …` are modeled as the construction idiom (`src/ast/js_cjs_export_barriers.rs`). Most CommonJS exports remain spanless, so an alias of a CommonJS import generally keeps the base result. Named function expressions with a proved same-name declarator retain their callable span (#450); since #627 a named function expression written in place (`module.exports = function g(){}`, `exports.f = function g(){}`, `{ f: function g(){} }`, any `g`) does too, in a file with no ESM export (beside an ESM export no CommonJS export form is a claim: `exports`/`module` do not exist in an ES module, #664). ESM importers of such a CommonJS producer (`import m from './m'`, `import { f } from './m'`) get these facts like those of declared exports. Tracked: #376 item 2.
  - The direct routes (`w()`, `N.w()`) bind a span-less export to any same-named function in the exporting file except private function-expression names, including nested declarations. Tracked: #376 item 5.
  - A private function-expression name never proves an export by spelling alone. Direct same-name declarator values and `exports.N = function N(){}` / `module.exports.N = function N(){}` values retain source-backed identity; array-element and identity-wrapper values remain refused pending a separate policy decision (#450).
  - A direct call to an import is dropped when the caller declares another binding of that name in a nested scope. Tracked: #376 item 4.
  - A module-scope `const` or `var` CommonJS `require` binding (`const m = require('./x')`, `var f = require('./x').f`, `const { f } = require('./x')`, `var { k: f } = require('./x')` with a flat object pattern) called inside an anonymous callable or from module-level code is Exact when the producer's CommonJS export proof holds (#597). A module-level call that runs before the declaration (`var a = m(1), m = require('./x')`) is Exact under the static-binding contract (the binding never holds another callable). Not admitted from anonymous callers: `let` bindings, a name declared more than once in the resolved program scope (a second `var`, a later `function m(){}`, a hoisted `var m` in a nested block, loop or `try`), a binding with a write resolving to it (writes in nested functions included), one shadowed between the site and the program scope (parameter, inner declaration, loop or `catch` head, `switch` case, same-name function or class), a declaration inside a block or function (#436), inline `require('./x').f(…)`, a destructuring pattern with a rest, default or nested pattern, and files with `with`, `eval`, escaped identifiers or parse errors.
  - module-as-function (`const merge = require('./merge'); merge(a, b)` with `module.exports = merge`) and member calls through a required module (`m.f()`, `require('./m').f()`, `const f = require('./m').f`) bind Exact when the export is a declared function proven by the terminal proof (#598) or a named function expression written in place (#627: `module.exports = function g() {}`, `exports.f = function g() {}`, `module.exports.f = function g() {}`, `{ f: function g() {} }`, `g` may differ from `f`; the target is the expression's own span, never a same-spelled declaration; async is admitted, a generator expression is not indexed as a callable and refuses). Still `UnknownName` or `LocalBindingUnproven`: anonymous exports (`module.exports = function () {}`, `exports.f = () => …`, which need anonymous node identity); a function used as the module value with static members (`module.exports = f; f.g = g`: `m.g()`); the inline `require('./m')(…)` call; a producer with an ESM export beside it; `let` or inline `require` called from anonymous callables (#597 admits only a top-level `const`/`var`); `require` inside functions (#436); a bare package or `paths` specifier; a member site `m.f()` whose consumer file writes `m.f`, `m[..]`, or hands `m` to an `Object`/`Reflect` mutator (a consumer-side stub, refused file-wide by spelling); an inline `require` inside a `with` body. A proven site replaces earlier NameOnly or indirect records at that site (slice L's rule).
- **Runtime mutation is out of model** for every route: monkey-patching, reflective writes, `eval`, host globals, re-acquisition through `require`/`import()`. See `CLAUDE.md`.

## 3. Consumers that still look calls up by spelling

A call through a followed alias is filed under the alias's spelling. The resolver, the navigation callers index (`nav callers`, MCP `nav_callers`) and `--format callers` all handle it. Two kinds of consumer do not:

- **Taint descent** (`nav taint-reaches`, MCP `taint_reaches`) stops at `const h = sink; h(x)`, where the direct call `sink(x)` reaches. `--algorithm taint` does cross it. Tracked: #376 item 1.
- **Review `taint` scope.** On small fixtures, review-CLI `--algorithm taint` findings stayed in the diff file where `nav taint-reaches` from the same seed found a cross-file witness. Mechanism not yet investigated (sink scoping or descent). Tracked: #602.
- **Name-keyed lookups.** `CallGraph::callers_of`, `callers_of_in_file`, `resolve_callers` and the diff-scope skeleton do not see followed-alias callers. Tracked: #376 item 3.

## 4. React

- The resolver treats a lowercase JSX tag (`<div />`, and `<h />` where `h` aliases a component) as an intrinsic element and gives it no target. The older textual alias path (§1.2), which scans source line by line, can still emit an edge for `const h = Comp;` on its own line followed by `<h />` in the same function. That edge is wrong, since `<h>` is a DOM element; it exists on main and is kept unchanged.
- Missing: a JSX attribute value has no data-flow edge into the component's parameter, destructured or not (`<Html html={text} />` → `Html({ html })`), so a `dangerouslySetInnerHTML` sink is not reached from the caller's value. The element → component call edge exists. Tracked: #596.
- A wrapped component export (`memo`, `forwardRef`, …) binds from JSX element sites only. Called as a function, it drops with `WrappedExportNonJsx`.
- Hook results: see §1.1. These are not followed:
  - an object built inside `useMemo(() => ({ … }))`;
  - `useContext`;
  - a value returned through two hooks;
  - an awaited result;
  - a value that is the hook's own parameter;
  - a `const` object or array that the hook uses anywhere but as the whole value of a `return` (a property write, `Object.assign`, an argument, a spread, a closure capture);
  - a hook with an explicit `return undefined` (`return;` and `return null` are fine), or one that spells any name with a unicode escape;
  - a hook that returns an object or array literal of more than 512 members.

## 5. Parsing

- `.js`/`.jsx` files are parsed with tree-sitter's JavaScript grammar, which has no type syntax. Flow-annotated or otherwise type-annotated JavaScript is misparsed, and wrong data-flow rows have been measured in it. Tracked: #359. TypeScript and TSX use the TypeScript grammars and are unaffected.
- Early errors are not detected (§1.4).

## 6. Data flow

- Object-literal arguments (#594, cache v136): `run({ cmd: x })` → `run(opts) { exec(opts.cmd) }` now binds `x` to the reads of `opts.cmd` (and its deeper paths and bare reads of `opts`), never to `opts.other`. It applies only when the call resolves to an Exact singleton and the formal is a plain identifier, a member-only formal or a parenthesised-arrow formal. Left unbound: the source of a spread (`{ ...o }`); keys written before a spread or a computed key (those can be overwritten; keys after them bind); all but the last of duplicate keys; accessors, methods and `__proto__`; values that are not an identifier or member path (calls, arithmetic, arrays, function values, `await`, conditionals); a literal after a spread argument (`f(...xs, { a })`); NameOnly and multi-target calls; destructured formals (`({ k })`, tracked separately), rest formals and bare-arrow formals (#595). A callee's member writes to a mixed formal follow the member-only lane's flow-insensitive suppression (a plain, compound or logical assignment, `++`/`--`, `delete`, a destructuring or `for-in/of` target, also inside nested callables): a read whose path is written, has a written prefix or a deeper write below it is not fed, and a leaf at or below a written path feeds no read of that formal, so a read that ran before the write loses its edge. A computed-member write to the formal itself (`p[k] = v`, `p["a"] = v`, `p[k]++`, `delete p[k]`, an array/object/rest pattern or `for-in/of` target, TS wrappers such as `(p as any)[k] = v`, also inside nested callables) may replace any key, so it is a wildcard write: no leaf of the literal feeds any read of that formal (fail closed; a shadowed inner `p` and a write to another base do not count, and a computed read `p[k]` does not suppress). The member-only lane keeps its own limit and does not see `p[k] = v`. Not modelled, as in the member-only lane: a write below a key (`p.a[k] = v`, `p[k].a = v`) and aliases of the formal. Zero-width reads: a mixed formal's reads in a declarator initialiser (`const t = o.a`, `const { a } = o`), a condition (`if (o.a)`), a `for-in` head and an `in` test carry a zero-width base Use, so the member chain cannot be read back and the edge is not made (it fails closed; bare reads and call-argument reads are fed). Estimated on the two corpora (review round 1): about 39 of 289 (Excalidraw) and 41 of 310 (TypeScript compiler) textually bare reads among mixed formals that received any literal edge. A fallback that reads the chain from the enclosing statement is the follow-up; the exact set is pinned by `real_shape_options_object_with_bare_and_member_reads` and `zero_width_base_reads_of_a_mixed_formal_are_unfed_today`. The label can be `NameOnly(CfgIncomplete)` or `NameOnly(Killed)` when the callee's own definition edge is; a `Killed` edge's `kill_line` is a line of the callee (the `to` endpoint's function), not of the argument's function. A TypeScript angle-bracket assertion around the literal (`f(<any>{ a })`) is admitted; TSX has none.
- Missing: rest formals (`function f(...args)`) and unparenthesized arrow formals (`x => …`) get a Def but no call-site binding; `(x) => …` binds. Tracked: #595.
- Label precision (#599, closed for parameter-only owners): a JS/TS/TSX owner whose statement universe is one line (one statement, several statements on a line, an expression-bodied arrow) has no CFG edge to build, and reaching definitions used to refuse it, labelling every edge `NameOnly(CfgIncomplete)`. A single node has no internal control flow at line granularity, so `src/cfg/js_single.rs` now lets the pass run on it (an expression-bodied arrow gets one synthetic whole-body statement, RD only). Edges are unchanged; labels move to `Exact` (parameter defined on an earlier line than the body) or `NameOnly(SameLine)` (parameter and body share a line). Still `CfgIncomplete`: an owner that defines one name twice (any alias def such as `const d = c`, or `c = c + 1`), because one node cannot order two defs of a name; an empty body; reads in nested callables (captures). The one-line `{ const d = c; ... }` shape still emits no param-to-use edge (declarator initialiser is no rvalue span and its use sits on the signature line): an edge-emission gap, not a label gap. The enumerated one-node guard (`first_hazard`) is deleted (#660 S1d-2): the writes and deferred reads the producer records no def for (update, pattern default, hoisted declaration, `arguments`, direct `eval(..)`, a nested callable's parameter default) are labelled by the write census (kill-only defs, D3 order, the refusal classes and the capture regions) in one-node and multi-statement owners alike. A one-node owner with a formal and a kill-only write of the same name still takes the label refusal `owner-refuse:one-node` (the distinct-bases belt); strict or non-simple `arguments` and updates of other names are `Exact`.
- Label precision (#759, closed): an expression-bodied arrow whose body is itself a callable (`const outer = (k) =>\n  () => { var t = src(); t++; sink(t); }`, also a function or generator expression, async or not) or a ternary used to take the located statements of that body as its universe: a callable's block statements (so the outer owner labelled the inner `t@3->t@5` `exact` beside the inner owner's `killed`; the census adds no kill-only defs for writes inside a nested callable), or the calls of a ternary's mutually exclusive arms (`c(k)\n ? f(k = 1)\n : sink(k)` gave `k@3->k@4` `exact`). Both now take the one synthetic body node (`cfg::js_single::arrow_body_takes_one_node`): a nested body is a deferred region of the outer owner and the outer label is `NameOnly(SameLine)`, or `NameOnly(CfgIncomplete)` where the distinct-bases belt refuses the owner (two recorded defs of one name, a ternary arm def of a formal); pinned by `tests/fixtures/js_write_census/arrow-body-matrix.tsv`, `reaching/arrow_body_tests.rs`. The inner owner keeps its own correct label as a separate edge. Costs: a belt-refused owner is RD-unavailable, so the fallback edge cap applies to it (it did not on main; `data_flow/build/arrow_body_cap_tests.rs`), and a correct cross-line `exact` inside a ternary body becomes `sameline`. A ternary body also costs correct `exact`s when no arm writes: a formal on the body's first line (`(k) => c(k)\n  ? f(k)\n  : g(k)`) becomes `sameline`, and a def in the condition (`c(k = 1) ? f(k) : g(k)`) trips the belt (`cfg_incomplete`, and the fallback cap then applies); both are pinned (rows `h01`, `h02`) and occur 0 times on excalidraw, express and TypeScript src. Opus round 2 offered narrowing the rule to ternaries whose arms write; it was left as a disclosed cost. On TypeScript src the ternary rule promotes 8 edges `cfg_incomplete -> exact`, all hand-checked correct: a formal read on a condition or arm line that had no statement node on base. Residual: an expression body holding two or more located statements that are neither a callable nor a ternary (`c(k) ||\n  sink(k)`: calls on several lines of one operator chain) keeps its partial multi-line universe; `||`, `&&` and `??` run left to right, so a conditionally skipped def can only over-kill (`killed`), and no wrong `exact` was found (matrix rows f08-f10, c03, c07). The belt is skipped when the file CFG admits any edge on the node's line (the lookup is keyed by line, not owner), so an owner with a same-line loop is not refused where its twin is; labels stay NameOnly. The anonymous middle arrow of `a => b => () => {..}` is an RD owner that carries none of the nested callable's edges, and `export default (k) =>\n  () => {..}` has no labelled edge, so the rule changes no label there.
- Try/catch/finally joins (#802, cache v171): the generic try edges (header -> catch, header -> finally, first catch statement -> finally), their duplicate line pairs, and the exits that a nested compound statement (`if`, loop, `switch`, `try`) ending a try body or catch draws into a handler or finalizer entry are explicit incomplete joins for JS/TS/TSX, so a route through them is never `exact` (`catch (e) { if ((x = a())) { x = b(); } else { x = c(); } } finally { f(); } sink(x);` no longer labels `x@(first catch line)->sink` `exact`; likewise a one-line `try { x = a(); } catch (e) {\n sink(x);`). This contains the demonstrated catch -> finally witnesses only. Cost: a correct exact whose only modelled route crosses such an edge becomes `cfg_incomplete` (including through header -> finally), and a `killed` label can become `cfg_incomplete` where the kill was on the unflagged route; excalidraw's cost is in `textWysiwyg.tsx` (a `for..of` exit lands on the catch's `return null` line, then catch -> finally). Retagging matches by line pair, so a loop exit or a header's legitimate false edge sharing the pair with an arm exit (an entry-line collision) is retagged too. Residuals (open): a nested-if exit not entering a handler or finalizer (#796 family; the no-try `x@3->12` is wrong on main too); a loop or do-while completing into the finalizer (#693/#697, same wrong `exact` without a try; #785 fixes admitted loops); and no pending-completion model (#802), so a def made in a finalizer still reaches the statement after the try as `exact` (`try { return; } finally { x = 1; } sink(x);`), and the try-header bypass geometry of #673 is unchanged.
- Array expression bodies (#784, open, pre-existing, design issue): an arrow whose expression body is an array holding a nested callable keeps the located direct call elements as its universe whenever two or more are located, so `const outer = (k) =>\n  [a(), () => { var t = src();\n    t++;\n    sink(t); }, b()]` labels `t@2->t@4` `exact` on both owners although `t++` kills it. The #759 rule only covers a body that is itself a callable or a ternary; widening the predicate one shape at a time is open-class, so the shape is left to the expression-body universe design in #784 and pinned as a known-wrong matrix row (`g01_array_body_nested_callable_784`), so a fix flips it deliberately. An array body with 0 or 1 located statements is one node already; a parenthesised, object or call body locates no element.
- Missing: a dynamic-key write `t[k] = s[k]` is recorded as Uses only, not as a path Def, so the prototype-pollution write in a recursive merge is invisible to data flow; there is no prototype-pollution sink. Tracked: #603.
- Spread field provenance (`{ ...a, ...b }`) is not tracked per field: [`language-analysis-gaps.md`](language-analysis-gaps.md), gap 6.
- Reads of a captured variable inside a deferred or nested callable are graded `NameOnly(CfgIncomplete)`. This rule is provisional pending callable-timing work; see `CLAUDE.md`.
- A parameter used only through member reads (`p.x`, `p?.x`) gets a base Def and projection edges to each evaluated read (cache v118). The walk covers the callable body by bytes: declaration-line body reads (`function h(p) { sink(p.x); }`, `p => p.x`) are projected; the parameter list is not, so a default that reads another formal (`q = p.x`) gets no edge, on any layout. Erased TS type positions (`typeof p.x` in a type, interface members) are not reads and never make a formal member-only; a callable containing any `with` statement (nested callables included) is never member-only and keeps the old behaviour (no Def), fail-closed, so its other member reads lose their edges too; reads of a callable nested inside a `with` body are also refused. A bare read inside a member object (`wrap(p).x`, `(p as any).x`, `p!.x`) makes the formal Bare, and a callable containing any `\`-escaped identifier keeps the old behaviour (no Def), fail-closed. Narrowings: a member write bound to the formal (assignment, compound or logical assignment, `++`/`--`, `delete`, array/object/rest pattern targets, `for (p.x of/in ...)`, also inside nested callables) suppresses that path, its prefixes and its extensions for the whole callable, so reads before the write lose the edge (flow-insensitive); a write is recognised through transparent wrappers anywhere in its receiver chain (`(p.x).y = 0`, `p.x!.y = 0`, `(p.x as any).y = 0`, `delete p.x!.y`), using the same normalizer as reads; a write through a subscript (`p.x[k] = 1`) or a mutating call (`Object.assign(p.x, ...)`) is not seen; `this.x`, destructured formals (`{ a }`) and `arguments` are not handled; a callable containing a parse error gets no Def. When one line holds several reads of the same path, the first admitted read carries the edge. The classifier is syntactic, so a formal whose only member accesses are writes (`p.x = 1`) gets a Def with no outgoing edges. A seed on the whole parameter reaches every `p.<field>` read (accepted cost), and a diff on the parameter's own line fans out through FullFlow and LeftFlow; a diff on a field read does not reach siblings in FullFlow by any route (directly, through an upstream argument Use/Def, or through a same-file caller): its forward leg seeds only from locations reached before the base Def, while the upstream lines stay included. A diff on a field condition (`if (p.x)`) in LeftFlow, FullFlow and ConditionedSlice also never forward-expands out of the base Def, so their condition walk does not reach a local derived from a sibling field (`const v = p.y; ... sink(v)`); RelevantSlice and SpiralSlice still include such a line through their own branch and ring expansion, as before #390. A condition variable whose every Use on the diff's condition lines is a projection read of its own callable's member-only formal (`if (p.x)`, `!p.x`, `p.x === 1`; such Uses did not exist before #390) keeps the pre-projection name-based inclusion of every textual `p` line, siblings included, and a local derived from the condition's own field is no longer reached either (the pre-#390 behaviour); a condition with any other Use of the variable (a call argument `if (ok(p.x))`, an assignment `if (w = p.x)`, which existed before #390), or on an ordinary formal fed by a field argument (`g(p.x)`), keeps its bounded DFG answer, as before #390. Wrong caller-to-callee argument edges inherited from call resolution now also reach these parameters at the same base rate. A named-owner formal that newly counts as Bare through a bare read inside a member object (`file: isString(file) ? file : file.path`) can emit a Use at an object-literal key span on its line (the legacy bare pass picks the line's first identifier span, main's existing E7 class, now reached for more formals; the line still holds a real read).
- Parameter-list alias (#761, fixed in CPG cache v166): a default `u = o` in the own parameter list is the body alias `const u = o` placed at the parameter, through the body's own alias machinery. It reads `o` bare (so `o` is a mixed formal, not member-only: `function f(o, u = o, v = u.x++) { return sink(o.x); }` no longer has the projection edge `o@1->o.x` exact; any bare read of a formal in a default does the same, `u = g(o)` included) and it mints the alias binding's def and the alias twin of `o`, so a write through `u` (in a later default or the body) is an `o.<path>` def. The alias binding's def and every twin of the list sit on the owner's FIRST line with the identifier's bytes, where main puts every formal's def, however many rows the signature spans (a row of the signature other than the first maps to no CFG node, so a twin there would neither kill nor collapse with the formal's def). A destructuring default with an identifier right-hand side is the body's destructuring alias (`[u] = o` and `{ a: u } = o`, `{ a } = o`; named owners take the body's array and object extractor, anonymous owners only a flat top-level object pattern, as the body does; `{ a: u = 1 } = o` is no alias in either). Label cost: a read-only alias (`u = o`, no write through `u`) turns the formal's edges from `exact` projection edges into mixed-formal edges, with the twin def sharing the signature node (`nameonly/sameline`, or `nameonly/cfg_incomplete` in a one-node owner), where the body's later-line twin gives `killed`. Residuals, all `exact` across a write through the alias unless noted:
  - Not aliases in the body either: a literal right-hand side (`{ a: u } = { a: o }`, `[u] = [o]`), `u = o.x`, `u = g(o)`, `u = c ? o : p`, and wrapped aliases `u = (o)`, `u = o as T`, `u = o!` (in the parameter list and the body alike).
  - Also not aliases, in the list and the body alike: a string or computed key (`{ 'a': u } = o`, `{ [k]: u } = o`), a defaulted array element (`[u = 1] = o`), and in an anonymous owner a nested, rest or array pattern (`[u] = o`, `{ a: { b: u } } = o`, `{ ...r } = o`; named owners alias these through the body's extractor). The binding of such an entry still gets its def.
  - A conditional rebind (`u = o; if (c) u = p; u.x = 1`): the flow-insensitive alias map keeps the last alias (`u -> p`), in the list and the body alike.
  - A write through a nested callable's alias is deferred (`g((z = y) => { z.x = 1; })`): the outer mixed formal stays `exact`, as for the body form.
  - A nested callable's own list is read by that callable's pass only. The body form `const z = y` inside a nested callable also reaches the OUTER named owner's alias map through its line coverage, so the outer owner gains twin edges (`alias_unstable`) the list form does not give: a recall difference, not a label one (the exact edges are the same).
  - An object-pattern alias (`{ a: u } = o`) mints the field twin (`o.a`), a different path from the formal's def on the same node, which RD cannot order, so the formal's own edge `o@1->o@N` stays `exact`. Alias-derived twins never kill: the body's `killed@2` comes from the twin of `o` at the declaration line, which is there even with no write through `u` (`const { a: u } = o; sink(o.b)` is `killed` too), so matching it would mean reproducing a declaration-line kill, not catching a write.
  - An earlier direct def of the target path stays `exact` across a write through the alias (`o.x = 0; u.x = 1; sink(o.x)`): the alias-twin def never kills, in the list and the body alike (#783).
  - Not aliases at all: a self or forward reference (`u = u`, `a = b, b`; a TDZ error), and an alias of a seam, eval or `arguments` formal in an anonymous callable (the formal has no def).
  - `o[k] = v` written in the list is not a read of `o`: the formal stays member-only and `own-params` refuses its edges. Direct own-parameter member writes are refused (`own-params`).

## 7. Node sources and sinks

- Framework request sources apply per file: a handler defined in a module that does not import the framework (express, koa, fastify, nest) gets no `req` source, even when it is registered as a route elsewhere. Tracked: #600.
- Review-taint flat sinks match by call position (`src/algorithms/taint_sinks/`, #622 S1), not by name fragment: `req.query` reads, `sanitizedQuery`, `revalidate`, `Array.find` and regex-literal `/../.exec(x)` are no longer sinks (closes the false hits tracked in #601). The policy is hybrid: the name must be a callee, an unknown receiver keeps firing for the high-severity names (`exec`/`spawn`, `query`/`execute`/`raw`, `send`/`write`, `eval`/`Function`, `redirect`), and only proven non-sinks suppress (regex literal or regex-bound receivers, `console`/`logger`, socket/`process`/stdout channels, a function of the same name declared in the file). `document.write`/`writeln`, fluent response chains (`res.status(400).send(x)`), `fs.open`/`fse.remove`/`window.open`, `execa*`, dynamic `import('node:child_process')`, `as`/`!`-wrapped receivers and `promisify(require('child_process').exec)` are sinks. Known gaps: computed members (`cp['exec'](x)`, `window['eval'](x)`: the name is a string, not an identifier; same as before the change); `Handlebars.compile`/`res.render` (dropped on purpose, zero labelled TPs); an untyped parameter named like a regex (`function h(re) { re.exec(x) }`) is an unknown receiver and still fires; JS `remove`/`open` only fire on file-system or global receivers.
- Missing: `process.env` as a source (#604 item 2).
