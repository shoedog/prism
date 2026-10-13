# Controller notes on the alias-following analysis (2026-10-13)

Written by the controller after the planner (gpt-6-astra, effort high) delivered `ANALYSIS.md`, `PLAN.md` and `CONTROLLER-census.sh`. Nothing here is approved by the owner. X = Excalidraw, T = TypeScript compiler `src`, F = the private corpus (aggregate counts only; rows stay in the controller's private directory).

## Owner direction this serves
"I dont believe we need to search the whole repo, local alias's first, then args, thrn imports." Both readings of "args" apply (the lookup step, and following arguments at call sites). "I suspect an issue is lack of test first development ... Enumerating possible aliases variants would allow testing against them instead of blindly plugging holes."

## What was verified by the controller
| Claim in `ANALYSIS.md` | Check | Result |
|---|---|---|
| A relative import with an explicit `.js` suffix that points at a `.ts` file is not resolved | Synthetic fixture on the frozen main binary: `import { f } from "./callee.js"` beside `import { g } from "./callee"` | `f` dropped `UnknownName`; `g` Exact `import_member`. Confirmed, including through a `.js`-suffixed `export *` barrel |
| Files over 2 MiB are not loaded | `src/repo_loader.rs` `MAX_FILE_BYTES`; `compiler/checker.ts` is 3,151,774 bytes | Confirmed |
| Sites with no record are module-level code or anonymous callbacks | Synthetic fixture on the frozen main binary (before the planner's second run) | Confirmed |
| Census join on F | `CONTROLLER-census.sh` on F | 9,318 of 9,318 bare Source records matched; no STOP |

Not re-derived by the controller: the per-class X and T tables (taken from the planner's `decision-summary.json`; its artifact checks pass 684 of 684).

## Private corpus (F) census, aggregates
1,122 files: 1,009 JavaScript/JSX, 103 TypeScript/TSX, 10 other. Recorded bare call/JSX sites 9,316 (product 8,607). Sites with no record: 15,116 (product 2,253: 1,710 in anonymous callbacks, 543 at module level).

| Binding family (product) | Sites | Correct today | Wrong today | Known missing | No in-repo callee | Unknown, has edge | Unknown, no edge |
|---|---:|---:|---:|---:|---:|---:|---:|
| import | 5,902 | 87 | 0 | 0 | 0 | 2,729 | 3,086 |
| local | 2,044 | 534 | 109 | 4 | 0 | 382 | 1,015 |
| parameter | 119 | 0 | 0 | 0 | 0 | 0 | 119 |
| same-file | 61 | 53 | 0 | 0 | 0 | 8 | 0 |
| global or library | 112 | 0 | 0 | 0 | 112 | 0 | 0 |
| unresolved | 369 | 0 | 0 | 0 | 366 | 0 | 3 |

"Unknown" for imports is a limit of the census, not of prism: 5,778 product import sites use non-relative specifiers, which the census checker does not resolve (it runs without the project's path mapping). The tsconfig-paths lane measured those separately.

Local family on F, by class (product sites / correct / wrong): `const` arrow 447 / 431 / 0; call result 276 / 100 / 1 (`useCallback` 98, all correct; hook-named calls 113; `lazy` 20; other 42; `forwardRef` 3); destructure from a call result 819 / 0 / 70 (`useState` 580 with 27 wrong, hook-named calls 199 with 43 wrong, `useContext` 40); destructure from an identifier 222 / 0 / 37; tagged template 256 of the 258 "other" (the census cannot follow a tagged template; the usual source of this shape is a styled-component, which the controller did not confirm); plain identifier alias 5; member of an identifier 4 / 0 / 1.

The 109 wrong targets are exactly the set E5 removes (all `free_single`). In addition 287 product sites carry `free_multi` NameOnly candidates for a locally bound name, 2,222 NameOnly edges in total.

Parameters on F: 119 product sites, 105 bindings (84 exported). Argument-following ceiling: 6 sites have a registered candidate; 3 through existing Exact incoming calls.

Static opportunity for the planner's slices on F: 4 sites (all L2). I1 (import paths): none measured, because F's imports are non-relative.

## A second ceiling: what the type checker can name
The planner's tracer follows syntax and stops at any call result or untyped object, so it reports those endpoints as unknown. The controller asked the checker a different question for every recorded local- or parameter-bound site: does the resolved signature's declaration have a body in the repo? (`variants/typeflow.cjs`; the corpus `tsconfig.json` path mapping is applied.) This bounds what following return values, object literals and declared types could resolve. Product sites that are not already correct:

| Corpus | Family | Sites | Named repo implementation | Anonymous repo implementation | Type-only signature | Outside repo | No declaration |
|---|---|---:|---:|---:|---:|---:|---:|
| X | hook or other call result | 252 | 98 | 5 | 13 | 0 | 136 |
| X | parameter | 324 | 87 | 0 | 180 | 28 | 29 |
| X | destructure from an identifier | 24 | 16 | 0 | 5 | 0 | 3 |
| X | other local | 104 | 60 | 0 | 16 | 2 | 26 |
| T | hook or other call result | 267 | 22 | 1 | 243 | 0 | 1 |
| T | parameter | 931 | 11 | 0 | 920 | 0 | 0 |
| T | destructure from an identifier | 194 | 0 | 0 | 194 | 0 | 0 |
| T | other local | 180 | 32 | 5 | 110 | 0 | 33 |

X total 261 of 704; T total 65 of 1,572. "Other local" on X includes 36 `const` arrow sites that already have an Exact edge the census could not confirm. The parameter hits on X are type-directed (a prop typed as a class member or `typeof fn`), not argument-following.

**F: inadmissible.** F is 90% untyped JavaScript and has no installed dependencies, so the checker returns no declaration for 1,572 of 1,629 sites, including the 100 `useCallback` sites prism already resolves. That is a failure of the probe, not evidence that nothing is resolvable. A ceiling for F needs a syntactic census of hook returns.

## The variant matrix
`variants/` holds 1,159 enumerated fixtures with authored ground truth (`variants.cjs`), a runner against the frozen binaries with a checker cross-check (`run.cjs`), the report (`MATRIX.md`, `matrix.jsonl`) and the exporter that feeds the in-repo test (`export.cjs`). The checker can type the callee on 741 fixtures and agrees with the authored truth on 727; the 14 that differ are four shapes where the checker answers a different question (listed at the top of `MATRIX.md`). The in-repo copy of the matrix is PR #365; the two review rounds there corrected two authored truths and added the shape families listed in that PR. Every census class with sites now has at least one fixture.

Added on 2026-10-13 after the owner's decision: 30 module-path fixtures (explicit `.js` suffix, `.`/`..`, redundant segments), the two uncovered census classes (dynamic `import()` member and default, destructured parameter defaults), every input the two E5 review rounds reported (merged-namespace, enum and decorated-class script globals; static-property indirect records; the assignment and parenthesised `require` forms), and lookup controls (alias of a parameter, catch binding, inner parameter shadowing an outer alias, eight- and nine-hop chains, class-method and arrow callers).

Main on the matrix (1,159 fixtures): local aliases 78 correct, 541 missing, 124 wrong of 870; parameters 0 of 156; module paths 18 of 26 with an in-repo callee missing; script globals 9 of 9 correct. The E5 head drops four of those nine script-global rows, which reproduces the round-2 finding inside the matrix. Main follows no local alias in the same function or at module level. It produces an edge only for an alias declared in an enclosing function, through the repo-wide name search.

Two things the matrix shows that are not alias-following:
- `new X()` has no call-site record in JS/TS (34 fixtures).
- **WRONG on main, outside this lane's scope, reported not fixed:** `const L = lazy(() => import('./mod')); <L />` resolves Exact to the loader arrow, which prism names `L`; the element renders the default export of `./mod`. F has 20 such sites. Whether `lazy` belongs in the admitted wrapper list is a contract question.

## Following a returned value, measured without types (2026-10-13)
`variants/callresult.cjs` follows the callee's binding with syntax only: const initialisers, object and array literals, destructuring keys, and up to four "value returned by a called function" steps. The called function is taken from prism's own Exact target for the initialiser call, so the count reflects today's import resolution. This is the measurement the owner asked for before committing to hook-result following; it also gives F the ceiling the checker could not.

Recorded product sites whose callee is bound to a local, by where the value comes from:

| Where the value comes from | X | T | F | What it needs |
|---|---:|---:|---:|---|
| A function defined in place (`const f = () => …`) | 1,568 | 92 | 551 | resolved today (1,529 / 78 / 534 correct) |
| **A named function reached through one returned value** | **98** | **14** | **71** | one returned-value step |
| An anonymous function reached the same way | 1 | 2 | 9 | the same step plus an identity for the inline callable |
| The called function is imported and prism does not resolve the import | 45 | 251 | 0 | import resolution first (T: the `.js` suffix gap) |
| `useState` / `useReducer` result | 60 | 0 | 580 | nothing: no in-repo callee |
| A hook from an npm dependency | 0 | 0 | 146 | nothing: no in-repo callee |
| Tagged template (`styled.x\`…\``) | 0 | 0 | 257 | nothing: no in-repo callee |
| `useContext(...)` value | 25 | 0 | 124 | following the provider's `value` (not designed) |
| A parameter or a member of one (`const { onX } = props`) | 26 | 204 | 221 | argument following |
| Result of a method call (`obj.m()`) | 18 | 41 | 38 | receiver typing |
| `lazy(() => import(...))` | 0 | 0 | 20 | a contract decision (see the matrix section) |
| Everything else | 68 | 115 | 27 | mixed; see the per-class output |
| Total | 1,909 | 719 | 2,044 | |

- **Precision on typed code.** With `--check` the script compares each followed target with the checker's resolved declaration: X 1,678 agree, 0 disagree (22 where the checker has no body to name); T 106 agree, 0 disagree (10 without a body).
- **Where F's 109 wrong edges sit:** 42 on one-step sites (41 named, 1 anonymous), 38 on parameter members, 29 where there is no in-repo callee. Refusal alone deletes all 109; the returned-value step would replace 42 of them with the right callee.
- **Sites where the right answer is "no edge"** (state hooks, npm hooks, tagged templates) are 983 on F. Main has 28 wrong edges among them today.
- F rows stay in the private evidence directory; only these counts leave it.

## Where the controller's reading differs from the planner's plan (for the owner, not adjudicated)
1. **Call results.** The plan treats every call result except admitted React wrappers as unsupported. The census says that is where the React population is: destructure-from-call-result is 819 product sites on F and 199 on X, against 9 and 12 for plain or member aliases. On X the checker names a repo implementation for 98 of 252 unresolved call-result sites. A bounded "follow the returned object literal of a same-repo function" step is not in the plan. Its yield on F is unmeasured.
2. **Order of work by measured value.** The plan's order follows the owner's lookup order. By measured sites the largest single item is I1 (T 15,167, X 7, F 0), which is independent of alias-following and could land first.
3. **Refusal on F is most of the measured value there.** With the plan's positive slices F gains 4 sites; the binding-aware refusal removes 109 wrong Exact targets and 2,222 NameOnly candidates.


## A second wrong-edge class on main: the same-file by-name rung ignores a nearer binding (2026-10-13)
Found while reading the binder (`src/ast/js_binding.rs`) and reproduced on the frozen main binary. The binder already finds the nearest binding and classifies it; for an alias, a written binding or a call result it answers "keep base behaviour". Base behaviour then tries a same-file function of the same spelling before anything else, so a local alias, a reassigned `let` or a destructured value that shadows a same-file function of the same name resolves Exact `local_def` to the shadowed function. The matrix has 36 rows for it (`shadows-same-file-fn*`), all WRONG on main and on the E5 head. The control `call-with-inline-fn` (the result of a call that takes an inline function, which main binds to that inline function under the 2026-09-27 may-call ruling) is CORRECT and must stay so.

Edges of this kind in the corpora today, counted with `variants/r4_adjudicate.cjs` (a locally bound callee whose Exact same-file target lies outside the binding's own declaration):

| | X | T | F |
|---|---:|---:|---:|
| Same-file edge, target inside the binding's own declaration (kept) | 34 | 154 | 54 |
| Same-file edge, target elsewhere in the file | 20 | 26 | 28 |
| … of which the file passes the target under the same name (prop or object key) | 0 | 4 | 0 |

On F, 27 of the 28 are `const { x } = props` where an unrelated same-file function is also called `x`. The census had these as "unknown, with an edge", which is why they were not in the 109.

The refusal slice therefore has two arms, measured separately: no cross-file name search for a bound name (the E5 result), and no same-file by-name hit outside the binding's own declaration.

## Owner decisions, 2026-10-13
After reading the analysis the owner chose, in order: (1) the import-suffix fix goes first; (2) binding lookup is the foundation slice: the nearest binding decides, const alias chains resolve, a bound name whose value is unknown gets no repo-wide name search, which replaces E5's exemption list; (3) measure hook-result following on the private corpus before committing to it (measured above); (4) parameters: the lookup step now, argument following later at NameOnly. No review of the plan. Script globals (the plan's item G) were not put to the owner: unbound names keep today's behaviour until that is decided.

Two places where the import-suffix slice as dispatched differs from the plan's I1 text, both disclosed to the reviewers as questions:
- The plan gives a literal `./x.js` the TypeScript precedence `.ts`, `.tsx`, `.d.ts`, `.js`. The slice is additive instead: an existing `x.js` keeps winning, as on main, and the substitution applies only when nothing resolves today. With the plan's order a hand-typed `x.js` beside `x.d.ts` would lose the edge main has.
- `.d.ts` is not a substitution candidate, because a declaration file has no callable body to bind to.

## CORRECTION (2026-10-13, evening): the "wrong edges" the refusal removes are mostly right targets found by spelling
Earlier sections of these notes, and the E5 packet before them, count F 109 / X 19 / T 201 "wrong Exact edges" on locally bound names. That label came from the census verdict, which meant "the name is bound locally and prism resolved it by a repo-wide name search", i.e. unproven. It did not mean the target is the wrong function. Measured now, with the refusal slice implemented (branch `feat/js-bound-name-refusal`, binary `R1`) against the alias-following binary, every removed Exact edge adjudicated:

| Removed Exact edges (product code) | X | T | F |
|---|---:|---:|---:|
| Through the repo-wide name search (`free_single`) | 30 | 145 | 113 |
| … the target is the real callee | 23 (checker agrees) | 0 shown | 42 (38 equal the callee a one-step returned-value follow derives; 4 are lazily required or imported modules the census had already marked correct) |
| … the target is certainly not the callee | 1 | 1 | 32 (28 have no in-repo callee: state setters and the like; 4 differ from the followed value) |
| … unknown (the value is a parameter or a member of one) | 6 | 144 | 39 |
| Through the same-file by-name rung (`local_def`) | 15 | 26 | 28 |
| … the target is the real callee | 0 shown | 8 (checker agrees) | 0 shown |
| … certainly not / unknown | 6 / 9 | 5 / 13 | 1 / 27 |
| Sites that lose only NameOnly candidates (edges) | 3 (9) | 49 (296) | 286 (2,217) |

X method: the TypeScript checker's resolved declaration for the call against the removed target. F method: the syntactic value-following census (`variants/callresult.cjs`), since F is untyped. X's 23: a component receives the library module object as a typed prop and destructures its members (11), and hook results such as `const { t } = useI18n()` (the rest). They are right because destructured names follow the names of the functions they hold.

What this changes:
- The refusal is not "removing 109 wrong edges" on F. It removes 113 unproven Exact edges of which at least 42 are right and 32 are certainly wrong. On X nearly everything it removes is right.
- Order of slices: returned-value following and following `require()` / `import()` destructures must land BEFORE the refusal, so the right edges are kept with proof. After them, what remains unproven is mostly members of parameters (props): X 11 (the checker says right), F 38 and T 143 (unknown).
- Policy for that remainder is an owner decision with three options: drop (the 2026-10-13 decision as worded), demote to NameOnly, or keep Exact. It should be made on numbers measured after the positive slices.
- The refusal branch is implemented and pushed but NOT sent to review. Its rule 3 (the textual alias path respects bindings) and the binder data for rule 2 are reusable whatever the policy.
