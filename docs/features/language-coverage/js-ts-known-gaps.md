# JS/TS, React and Node: known gaps and edge cases

**Status:** living list, last updated 2026-10-13 (state of `main` after #372, const alias following).

Each entry says:
- what prism does today, with a small example;
- whether the result is a **wrong** edge (an edge, often Exact, to a function the call does not reach) or a **missing** one (no edge, i.e. prism fails closed);
- where the entry is tracked.

`Exact` is a static-binding grade, not a runtime claim; see the call-resolution section of `CLAUDE.md`.

The variant matrix in `tests/fixtures/js_alias_matrix/` is the executable form of many entries. It holds one row per binding shape, each with an authored truth and today's verdict. Its `WRONG`, `OVERCLAIM` and `MISSING` rows are known gaps of this kind, and its README explains the verdicts.

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
| `const { f } = require(…)` inside a function, `const f = require('./x').f`, `await import('./x')` | Not followed (§1.2) | planned after the returned-value slice |
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
  - wrong: when the namespace export proof fails, the fallback matches files by stem and grades every match Exact, so `N.f()` can bind `other/x.ts:f`;
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
  - `module.exports = …` and `exports.x = …` are modeled as the construction idiom (`src/ast/js_cjs_export_barriers.rs`). A CommonJS export has no source span, so an alias of a CommonJS import is not proven by the alias route; such a site keeps the base result. Tracked: #376 item 2.
  - The direct routes (`w()`, `N.w()`) bind a span-less export to any same-named function in the exporting file, including a nested one. Tracked: #376 item 5.
  - A direct call to an import is dropped when the caller declares another binding of that name in a nested scope. Tracked: #376 item 4.
- **Runtime mutation is out of model** for every route: monkey-patching, reflective writes, `eval`, host globals, re-acquisition through `require`/`import()`. See `CLAUDE.md`.

## 3. Consumers that still look calls up by spelling

A call through a followed alias is filed under the alias's spelling. The resolver, the navigation callers index (`nav callers`, MCP `nav_callers`) and `--format callers` all handle it. Two kinds of consumer do not:

- **Taint descent** (`nav taint-reaches`, MCP `taint_reaches`) stops at `const h = sink; h(x)`, where the direct call `sink(x)` reaches. `--algorithm taint` does cross it. Tracked: #376 item 1.
- **Name-keyed lookups.** `CallGraph::callers_of`, `callers_of_in_file`, `resolve_callers` and the diff-scope skeleton do not see followed-alias callers. Tracked: #376 item 3.

## 4. React

- The resolver treats a lowercase JSX tag (`<div />`, and `<h />` where `h` aliases a component) as an intrinsic element and gives it no target. The older textual alias path (§1.2), which scans source line by line, can still emit an edge for `const h = Comp;` on its own line followed by `<h />` in the same function. That edge is wrong, since `<h>` is a DOM element; it exists on main and is kept unchanged.
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

- Spread field provenance (`{ ...a, ...b }`) is not tracked per field: [`language-analysis-gaps.md`](language-analysis-gaps.md), gap 6.
- Reads of a captured variable inside a deferred or nested callable are graded `NameOnly(CfgIncomplete)`. This rule is provisional pending callable-timing work; see `CLAUDE.md`.
