# S1b control expectations, written before the first run (2026-09-26)

Scenarios C68–C107 come from `controls_gen.py`. Each JS-syntax scenario is emitted twice (`_jsx`: `.jsx`/`.js`;
`_tsx`: `.tsx`/`.ts`). C102–C104 are TS-only. "base" is `a6d853f5`; "proto" is the S1b prototype with
`PRISM_S1B_USECALLBACK` unset (strict) unless stated. Expectations are the same for both grammar twins unless noted.

| Scenario | base (predicted) | proto (predicted) |
|---|---|---|
| C06 (S1) list ternary + nested `f` | Exact `import_member` → nested `f@2` (false) | drop |
| C21 (S1) `export let A = …; A = Other` | Exact `import_member` → `A@1` (false) | drop (`written`) |
| C62 (S1b-RED-1) namespace `<Lib.Island/>` + decoy | 2× Exact `import_qualified` (`@3-3`, `@6-8`) | 1× Exact → `Island@6-8` |
| C63 (S1b-RED-2) producer-local `Island({})` | Exact `local_def` → `@2-4` | `WrappedExportNonJsx` (both call sites) |
| C68 `export default f` (ternary) + nested `f` | Exact → nested `f` (false) | drop |
| C69 `export function f` + nested decoy | NameOnly ×2 `import_member` | Exact → `f@5-7` (re-target) |
| C70 `export const f = () =>` + nested decoy | NameOnly ×2 | Exact → `f@5-7` |
| C71 `const f = () =>; export { f }` + nested decoy | NameOnly ×2 | Exact → `f@5-7` |
| C72 list-exported `forwardRef`; `Island({})` and `<Island/>` | both Exact → inner arrow | JSX Exact; call `WrappedExportNonJsx` |
| C73 list-exported `create(fn)` | Exact → the `create` argument arrow (false) | drop |
| C74 two `var f = () =>` + `export { f }` | NameOnly ×2 | drop |
| C75 `export function f` + hoisted `var f` in a block | Exact → `f@1-3` | drop (`duplicate_declaration`) |
| C77 `function f`; `f = other`; `export default f` | Exact → `f@1-3` (false) | drop (`written`) |
| C78 `const f = function g(){}; export { f }` | drop (no function named `f`) | Exact → `g@1-3` (new edge) |
| C79 unrelated parse error in the exporting file | Exact → `f@1-3` | drop (`parse_recovery`, whole module scope) |
| C80 namespace `Lib.f()` + nested decoy | 2× Exact `import_qualified` | 1× Exact → `f@5-7` |
| C81 namespace `./lib.js` + nested decoy | 2× Exact | 1× Exact → `f@5-7` (`_jsx`: rule 1, the `.js` file resolves; `_tsx`: rule 2) |
| C82 namespace wrapped: `Lib.Island({})` and `<Lib.Island/>` | both Exact | JSX Exact; call `WrappedExportNonJsx` |
| C83 namespace to `export { g as f } from './impl'` | drop `ImportExternal` | Exact `import_qualified` → `impl:g` (new edge) |
| C84 named-object qualifier `obj.f()` | Exact → the Pattern-2 arrow | unchanged |
| C85 default-import qualifier `<Lib.Island/>` + decoy | 2× Exact | unchanged (S2 lane, recorded) |
| C86 two nested `const g` in two functions | each call Exact ×2 | each call Exact ×1 (its own `g`) |
| C87 parameter `f` shadows a nested arrow `f` | Exact → holder's arrow (false) | drop `LocalBindingUnproven` |
| C88 Pattern-2 `require:` arrow + global `require()` | Exact → the Pattern-2 arrow (false) | drop `LocalBindingUnproven` |
| C89 `const save = useCallback(() => …, [])`; `save()` | Exact → the arrow | strict: drop; toggle: Exact |
| C90 `const t = throttle(fn)`; `t()` | Exact → the arrow | drop |
| C91 `let g; g = () => 1; else g = () => 2; g()` | Exact ×2 | drop |
| C92 catch / for-of / class / `with` shadows of top-level `h` | 5× Exact → `h@1-3` | try-block `h()` Exact; the other 4 drop |
| C93 block function `inner`: call inside, call outside | both Exact | inside Exact; outside drop |
| C94 `var h = () =>` hoisted from a nested block | Exact | Exact |
| C95 same-line `const f` in two functions | each Exact ×2 | each drop (two span matches) |
| C96 `<div>`, `<island>…</island>`, `<my-el/>` with same-file `div`/`island` functions | `div`, `island` Exact `local_def` | all `JsxIntrinsic` |
| C97 (Z02b) `export const island`; `island()` and `<island/>` | both Exact `import_member` | call Exact; tag `JsxIntrinsic` |
| C98 `<lib.island/>` through a namespace | Exact `import_qualified` | Exact (member tags are not intrinsic) |
| C99 recursion `f()` and named-fe self call `h()` | Exact, Exact | Exact, Exact |
| C101 `x => x()` shadows a top-level `function x` | Exact → `x@1-3` (false) | drop |
| C105 parse error in the scope holding the binding | Exact ×2 | drop (`parse_recovery`) |
| C106 same-file `memo`: `Card({})` and `<Card/>` | both Exact | JSX Exact; call `WrappedExportNonJsx` |
| C107 `const Editor = lazy(() => import(…))`; `<Editor/>` | Exact → the loader arrow (false) | drop |
| C102 TS overloads + implementation; `f('x')` | Exact | Exact |
| C103 `declare function f`; nested decoy `f`; `f()` | Exact → the decoy (false) | drop |
| C104 block `enum E` shadows `function E`; `function N` + `namespace N` | both Exact | both drop (the second is a fail-closed recall loss: TS merges them) |

All S1 controls C01–C67 other than C06, C21, C62 and C63: unchanged resolved targets. Rows whose only change is
`UnknownName` → `JsxIntrinsic` on a lowercase tag (`<div/>` in the fixtures) are expected and are not counted as
behavior changes.

## Addendum (written before the second run): C105 replaced, C108–C109 added

The first C105 fixture (`return <div>{</div>;`) turned `App` into a root `ERROR`, so no call site existed on base or
proto. That probe failed for its own reasons and is inadmissible; it is replaced.

| Scenario | base (predicted) | proto (predicted) |
|---|---|---|
| C105 `let x = ;` inside the function whose `const g` holds the binding | `g()` Exact ×2 (`g@1-3`, `g@5-5`) | drop `LocalBindingUnproven` (`parse_recovery`) |
| C108 JSX recovery `<div><span></div>` in the same function | `g()` Exact ×2 | drop |
| C109 `let g = ;` recovery in an inner block, top-level `function g` | `g()` Exact → `g@1-3` (or the recovered declarator, if registered) | drop: the binding scope found is the program, which contains the error |

## Addendum 2 (written before the v3 run): parameter scope, Annex B, namespace provenance

Found by reasoning over the scope rules, before any run: a parameter default is evaluated in the parameter scope,
which cannot see body declarations; a sloppy-mode block function declaration is also var-hoisted to the enclosing
function (Annex B.3.3), so a call outside the block may denote it; a namespace local that another module-scope
declaration competes with has no single static binding. The v1/v2 prototype mishandles the first two.

| Scenario | base (predicted) | v2 (predicted) | v3 (predicted) |
|---|---|---|---|
| C110 `function f(a = g())` with body `function g` and top-level `function g` | Exact ×2 (`g@1-3`, `g@5-7`) | Exact → body `g@5-7` (false) | Exact → top-level `g@1-3` |
| C111 `inner()` after `if (flag) { function inner(){} }`, top-level `inner` | Exact ×2 | Exact → top-level `inner@1-3` (false in sloppy scripts) | drop (the Annex B marker is not a callable) |
| C112 `import * as Lib` plus `var Lib = {…}` | Exact `import_qualified` → `lib:f` | same | drop (`Lib` has two module-scope declarations) |

## Addendum 3 (written before the v7 run): implicit `arguments`, TS `using`

Found while writing SPEC §3.1's closure argument. A non-arrow function implicitly binds `arguments`; an arrow
function inherits the enclosing one's. tree-sitter-typescript 0.23 parses `using x = e;` as the assignment `x = e`
with no error (MEASURED before this run with the Python grammar), so the collector cannot see that declaration.

| Scenario | base (predicted) | v6 (predicted) | v7 (predicted) |
|---|---|---|---|
| C113 `arguments()` inside `function f` and inside a top-level arrow, with a top-level `function arguments` | both Exact → `arguments@1-3` | both Exact (false for `f`'s call) | `f`'s call drops (implicit binding); the arrow's call stays Exact (an arrow at module scope has no `arguments` of its own, so the top-level function is the binding) |
| C114 (TS) `using h = res(); h();` in `f`, and `h()` in `g`, top-level `function h` | both Exact → `h@2-4` | both drop: the misparsed `using` is a write to the top-level `h` (B5), which refuses every site (fail-closed; `g`'s right edge is lost) | same as v6 |
