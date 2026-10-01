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

## Addendum 4 (spec r1 fold, written before the first v8 run)

v8 applies the owner's r1 answers: **E5 = may-call rows keep base behavior** (a written binding, or a declarator
whose value is a call with a direct function argument, i.e. a Pattern-3 over-named argument), **E4 = `useCallback`
admitted**, **E6 = the narrower sealed-error rule**, **E7 = sibling-extension Exact / other stem NameOnly**, plus the
r1 collector fixes (§3.1 table), the kind allowlist fail-safe, and `(name, span)` matching. A declarator whose value
is a call *without* a direct function argument is not may-call: it holds no callable (refuse). TS `using x = e` is a
lexical declaration (the grammar spells it as an assignment with an anonymous `using` token), not a write.

Changed expectations for existing scenarios (v7 → v8):

| Scenario | v7 | v8 (predicted) |
|---|---|---|
| C21, C77, C139 written exported/top-level function or arrow | drop | **base** (MayCall, E5) |
| C73 list-exported `create(fn)` | drop | **base** (MayCall) |
| C89 `useCallback` | drop (strict) | **Exact** (E4) |
| C90 `throttle(fn)`, C107 `lazy(loader)` | drop | **base** (MayCall) |
| C91 `let g; g = () => 1` | drop | **base** (MayCall) |
| C114 (TS) `using h = res()` | both drop | `f`'s call: drop (a using binding holding a call result); `g`'s call: **Exact** → top-level `h` (a `using` is not a write) |
| everything else in C01–C113 | as v7 | unchanged from v7 |

New scenarios (both grammars unless TS-only):

| Scenario | base (predicted) | v8 (predicted) |
|---|---|---|
| C115 static-block `var f = () => 2` called from an arrow in the block, top-level `f` | Exact ×2 | Exact → the static-block arrow |
| C116 static-block `var f = 1; f()`, outer `function f` | Exact → outer `f` | drop |
| C117 (TS) namespace-body `var f = () => 2`, top-level `f` | Exact ×2 | Exact → the namespace arrow |
| C118 same-line `const f = function g(){}; const o = { f: () => 2 }`; `f()` and `o.f()` | `f()` Exact → `o.f`'s arrow (Pattern 2) | `f()` Exact → `g` (by name and span); `o.f()` unchanged |
| C119 `catch (f) { var f = () => 2; }` then `f()` in the function | Exact ×2 | drop |
| C120 `with (o) { var f = () => 2; }` then `f()` | Exact ×2 | drop |
| C121 `function run(a = arguments())`; arrow `(a = arguments()) => a` at module scope | Exact, Exact → `arguments@1-3` | drop; Exact (an arrow has no own `arguments`) |
| C122 `function f(g, a = g())` with top-level `g` | Exact → `g@1-3` | drop (the parameter `g` is visible in the default) |
| C123 `var h = (M.h = function(){…}); h()` | Exact → `h` | Exact → `h` (B3 unwraps the assignment chain; right edge kept) |
| C124 `caller({ f = () => 2 } = {})` then `f()` | Exact ×2 | drop (destructuring-default parameter) |
| C125 `({ f = other } = obj); f()` with a nested decoy `f` | Exact ×2 | **base** Exact ×2 (the binding is written: MayCall) |
| C126 lib `export function f` plus `const g = f => { f = 2; }`; app imports `f` | Exact | Exact (the arrow parameter is not the module binding) |
| C127 labelled function declaration called before it | Exact | Exact |
| C128 `Lib => Lib.f()` | Exact `import_qualified` | no `import_qualified` edge |
| C129 `with (obj) { Lib.f(); }` | Exact | no `import_qualified` edge |
| C130 `Lib = other; Lib.f()` | Exact | no `import_qualified` edge |
| C131 `import * as Lib from 'foo'` with a repo `foo` module | Exact | NameOnly |
| C132 `'./a/foo'` where only `b/foo` exports `f` | Exact → `b/foo` | NameOnly → `b/foo` (`./a/foo` does not resolve; stem fallback) |
| C133 `'./a/foo.js'` where `a/foo` lacks `f` and `b/foo` has it | Exact → `b/foo` | drop `UnknownName` (the sibling resolves; it has no `f`) |
| C134 `const f = function* () {}`; `f()`; `export { f }` | drop | drop (generator expressions are not indexed; pinned non-goal) |
| C135 switch-case `const f = () => 2; return f()` and a later `f()` | Exact ×2 each | case call Exact → case `f`; later call Exact → top-level `f` |
| C136 `for (var f in o) {}` then `f()` | Exact → top-level `f` | drop |
| C137 `var \u0066 = 2` in the function, then `f()` | Exact → top-level `f` | drop (`escaped_identifier`) |
| C138 `const t = throttle(() => 1)` with a nested decoy `t` | Exact ×2 | **base** Exact ×2 (MayCall) |
| C139 top-level `f = other` then `f()` | Exact → `f` | **base** (MayCall) |
| C140 method param `f` and a sibling method calling top-level `f()` | Exact | Exact |
| C141 (TS) `import f = M.g` with a nested decoy `f` | Exact → decoy | drop |
| C142 (TS) `(f as any) = …`, `(<any>f) = …`, `f! = …` with a nested decoy | Exact ×2 | **base** Exact ×2 (written: MayCall) |
| C143 (TS) `type f = number` next to `function f`; `interface g` next to `function g` | Exact, Exact | Exact, Exact (type space binds no value) |
| C144 (TS) `import type { f }` next to `function f` | Exact | Exact |
| C145 (TSX) parameter decorator `@f()` with outer `function f` | Exact → outer `f` | Exact → outer `f` |
| C146 (TSX) `export const Button` plus `export interface Button`; `<Button/>` | Exact | Exact |
| C147 (TS) `declare const f` with a nested decoy `f` | Exact → decoy | drop |
| C148 (TS) `'./lib.js'` naming `lib.ts` with a nested decoy | Exact ×2 | Exact ×1 → the exported `f` (E7 sibling) |

## Addendum 5 (written before the v8b rerun): the narrower parse-recovery rule (E6)

| Scenario | base (predicted) | v8 (predicted) |
|---|---|---|
| C149 an error without braces inside another exported function; importer calls `f` | Exact | Exact (the error is sealed in `broken`, whose text never spells `f`) |
| C150 the same, but the erroneous function spells `f` | Exact | drop (not sealed: the error's function mentions the name) |
| C151 `let x = ;` inside the function that calls `g()`; another function also calls `g()` | Exact, Exact | drop (the error's function contains that site); Exact (sealed for the other site) |

## Addendum 6 (after the v8b run, before v8c)

C151's "other" call was predicted Exact but dropped: my prediction missed that `run`, the erroneous function, calls
`g()`, so its text spells `g` and the v8b mention check refused. The mention check guards only against text being
swallowed into the sealing function, which the file-level brace condition already excludes, so v8c removes it. v8c
predictions: C149 Exact; **C150 Exact** (the inner `let f = ;` is local to `broken`, so the export binding is
unaffected); C151 `run`'s call drop, `other`'s call Exact.

## Addendum 7 (spec r3 fold, written before the first v9 run)

v9 = v8 + the re-plan's Option K (the E-table walk, the leave predicate, T10, J3, the `with` object, K at unproven
positions), B1's structural-token and delimited-child folds, B0 with leaf kinds, `ModuleExportName`, and this round's
folds: J2 split by holder (Opus W1), dotted-namespace segments (Opus W2), the Annex-B applicability predicate (sol W1),
`for…in/of` left Inside and enum StringValue names (Opus S4).

Changed expectation for an existing scenario: **C111** (module file: it exports) → the outer `inner` **Exact**
(module code is strict, so the block declaration is block-scoped). C93's outside call still drops (strict: `inner` is
unbound there).

| Scenario | base (predicted) | v9 (predicted) |
|---|---|---|
| C152 named class expression `class f`, member decorator `@f()` | Exact → outer `f` | drop (member decorators are evaluated in the class scope, where `f` is the class) |
| C153 class declaration `class f`, member decorator `@f()` | Exact → outer `f` | drop |
| C154 anonymous class expression, member decorator `@f()` | Exact | Exact (unchanged) |
| C155 `@f() class f {}` as an expression (OQ7) | Exact | Exact (unproven: base behavior, counted `decorated_class_expression`) |
| C156 (TS) `namespace A.B { B() }` with top-level `function B` | Exact → `B@1` | drop (`B` denotes the namespace) |
| C157 (TS) `namespace A.B.C { C() + A() }` with top-level `A`, `C` | Exact, Exact | drop (segment `C`), drop (`A` is the namespace declared by D6) |
| C158 (TS) `enum E { 'f' = 1, g = f() }` in a function | Exact → `f@1` | drop (`f` is the member `E.f`, by StringValue) |
| C159 (TS) script without a directive, Annex-B shape | Exact ×2 | base (Exact ×2; unknown strictness, counted `annex_b_strictness`) |
| C160 (JS) `.js` script without a directive, Annex-B shape | Exact ×2 | base (Exact ×2; unknown strictness) |
| C161 `'use strict'` script | Exact ×2 | Exact → the outer `inner` |
| C162 `"use strict"` in the function | Exact ×2 | Exact → the outer `inner` |
| C163 `.cjs` (sloppy) script | Exact ×2 | drop (Annex-B marker) |
| C164 `.cjs` block generator declaration | Exact ×2 | Exact → the outer generator... **no row**: generator declarations are indexed but a call to a generator is a call; predicted Exact → outer `inner` (Annex B does not apply to generators) |
| C165 `for (const { a = f() } of xs)`; `for (const { f = f() } of xs)` | Exact, Exact | Exact → top-level `f`; drop (the head's own `f`, TDZ) |

## Addendum 8 (S1b-3 r4 plan, v10 on the landed `3961cc21`) — NOT pre-registered

Disclosure: the expectations below come from the controller's carry-forward list (2026-09-27) and SPEC §3.8; they
were not written into this file before the first v10 run. They are recorded here so the implementer's RED rows cite
one place. The base for these rows is the S1b-2b head `3961cc21` (`S1b-controls-head-3961cc21.txt`), not `a6d853f5`.

| Scenario | head `3961cc21` (measured) | v10e (measured, matches the carry-forward's intent) |
|---|---|---|
| C166 `let f` in a default-parameter arrow | importer NameOnly pair; in-file Exact pair | importer Exact `lib:f`; in-file Exact `f` |
| C167 inner `function f` in a sealed-error function | the same | the same |
| C168 class inner name written from a computed key | the same | the same |
| C169 written `class f` / `for (var f in o)` head / parameter | Exact to the module `f` (base) | unchanged (M2 → `MayCall`, owner ruling) |
| C170 block-nested `const { f }`; module bare-block `var { f }`; module bare-block `const { f }` (P15) | Exact `free_single` ×3 | drop, drop, Exact kept |
| C171 (TS) `namespace N { export const { f } = o }` + module `run(){ f() }` | Exact `free_single` | unchanged |
| C172 `export function f(){ let x = ; }` + `export { f as g }` | `f` no edge, `g` Exact | both Exact |
| C173 `import { "\u{GG}" as h }` recovered as a top-level `ERROR`, `export function h` elsewhere | Exact `free_single` (wrong) | drop |
| C150 (sealed error mentioning the name) | Exact (D4 `MayCall` → base `Local`) | Exact (D4 `VerifiedLocal`; same edge) |

Every other scenario matches v9's reference (`S1b-controls-proto-v9.txt`) except the S1b-4 rows (C62, C80–C83,
C129–C133, C148, which S1b-3 does not own). C166–C173 are new.

## Addendum 9 (S1b-3 spec r1 fold, v11) — PRE-REGISTERED, written before v11 is built or run

Base: `prism-head-3961cc21`. "v10e" is the reviewed prototype; "v11" the fold (owner 2026-09-29: the R5 drop narrowed
to bindings that provably hold no function; B1 relaxation only for body / class-body errors; recovered-import
markers only after a leading static `import`; parser-confirmed Unicode identifiers). Both grammars.

| Scenario | head (predicted) | v10e (predicted) | v11 (predicted) |
|---|---|---|---|
| C174 `const { t } = useI18n(); t()`, `t` a function in `lib` | Exact `free_single lib:t` (right) | drop (the Opus W1 loss) | Exact kept, counted `alias` |
| C175 `const h = ctx.make; const { g } = ctx; h(); g()`, `h`, `g` functions in `lib` | Exact ×2 | drop ×2 | Exact ×2 kept, counted `alias` |
| C176 `const f = 0`, `const { k } = { k: 1 }`, `const [m] = [1, g]`; functions `f`, `k`, `m` in `lib` | Exact ×3 | drop ×3 | drop, drop, **Exact kept** (`m`: the array holds an identifier, alias) |
| C177 same-file `function t` + `const { t } = make()` in `run` | Exact `local_def` | drop | Exact `local_def` kept (alias at R4) |
| C178 `export function g(a, f ==) { return f(); }` + module `function f` | Exact `local_def` | Exact (the W2 bug) | drop (parameter-list error not sealed for a body site) |
| C179 `import.meta load)`; `import('./x') load2)` at top level; `load`, `load2` functions in `lib` | Exact ×2 | drop ×2 (W3 over-poison) | Exact ×2 |
| C180 `function é(){}`; `const é = 0; é()` (`é` = e + U+0301) | Exact `local_def` (sol W1) | Exact (unchanged) | drop |
| C181 `import { "\u{GG}" as é, "\u{GG}" as a‌b }` recovered as an `ERROR`; `é()`, `a‌b()` (ZWNJ), functions in `lib` | Exact ×2 | Exact ×2 (sol W2) | drop ×2 |

### Addendum 9 result for C174–C181, and 9b (pre-registered before C182–C184 run)

- **C177–C181: as predicted** in all three columns.
- **C174–C176: the head column was falsified.** Head drops all of them `UnknownName`, not Exact: S1b-1b's F4 collects a
  function-top-level destructuring into the caller's locals, so base's R5 guard already drops. v10e and v11 match
  head (no change). The Opus W1 rows are reached only when the **caller is a nested function** (an arrow callback)
  and the declaration is in an enclosing function, which the base guard does not see. 9b adds those shapes:

| Scenario | head (predicted) | v10e (predicted) | v11 (predicted) |
|---|---|---|---|
| C182 `const { t } = useI18n(); xs.map((x) => t(x))` | Exact `free_single lib:t` | drop | Exact kept, counted `alias` |
| C183 `const h = ctx.make; const { g } = ctx; xs.map(() => h() + g())` | Exact ×2 | drop ×2 | Exact ×2, counted `alias` |
| C184 `const f = 0; const { k } = { k: 1 }; const [m] = [1, g]; xs.map(() => f() + k() + m())` | Exact ×3 | drop ×3 | drop, drop, Exact (`m` alias) |

**9b probe fix (inadmissible first run, no comparison drawn):** the first C182–C184 fixtures called from anonymous
arrows (`xs.map((x) => t(x))`), which prism does not index as callers, so no row existed to compare. The fixtures now
call from a named nested `function each`; the predictions above stand unchanged.

## Addendum 10 (S1b-3 spec r2 fold, v12) — PRE-REGISTERED, written before v12 is built or run

Owner 2026-09-29 (conservative cut): a destructuring declarator whose pattern contains a default is `Alias` (keeps
base); the recovered-import check reads the first two non-comment tokens. Both grammars.

| Scenario | head (predicted) | v11 (predicted) | v12 (predicted) |
|---|---|---|---|
| C185 `const { missing: x = fallback } = {}; x()`, `fallback = function x(){}` | Exact `local_def x@1` | drop (sol r2 W1) | Exact kept (alias) |
| C186 `const [y = fallback] = []; y()` | Exact `local_def` | drop | Exact kept |
| C187 `const { z = fallback } = {}; z()` | Exact `local_def` | drop | Exact kept |
| C188 `const { f = 0 } = {}` in `run`, `f()` from nested `each`, `f` a function in `lib` | Exact `free_single` | drop (NoFn `{}`) | Exact kept (a default: alias, the conservative cut) |
| C189 `import /* c */ . meta load)`, `import /* c */ ('./x') load2)`, `import // c` + `.meta load3)` | Exact ×3 | drop ×3 (sol r2 W2) | Exact ×3 |

### Addendum 10 result

**C185–C189: as predicted in every column** (head, v11, v12), both grammars. The first v12 build (`9e8eaabb`)
skipped every extra leaf and so also skipped tree-sitter's recovery `ERROR` nodes (flagged as extras), regressing
C173 and C181 to Exact; the clean v12 (`0968ef78`) skips only non-`ERROR` extras and comments, and C173/C181 drop
again. That first build produced no corpus receipt.
