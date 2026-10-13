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

Local family on F, by class (product sites / correct / wrong): `const` arrow 447 / 431 / 0; call result 276 / 100 / 1 (`useCallback` 98, all correct; hook-named calls 113; `lazy` 20; other 42; `forwardRef` 3); destructure from a call result 819 / 0 / 70 (`useState` 580 with 27 wrong, hook-named calls 199 with 43 wrong, `useContext` 40); destructure from an identifier 222 / 0 / 37; tagged template 256 of the 258 "other" (styled components, no in-repo callee); plain identifier alias 5; member of an identifier 4 / 0 / 1.

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
`variants/` holds 780 enumerated fixtures with authored ground truth (`variants.cjs`), a runner against the frozen binaries with a checker cross-check (`run.cjs`), and the report (`MATRIX.md`, `matrix.jsonl`). The checker agrees with the authored truth on every fixture it can type (0 disagreements). `MATRIX.md` ends with the census population per class and the shapes that cover it. Two classes have sites and no fixture yet: `import.dynamic_import` (X 1) and `parameter.destructured.default` (T 5, F 12).

Main on the matrix: local aliases 53 correct, 364 missing, 67 wrong of 572; parameters 0 of 132; imports 21 of 25 with an in-repo callee. Main follows no local alias in the same function or at module level. It produces an edge only for an alias declared in an enclosing function, through the repo-wide name search.

Two things the matrix shows that are not alias-following:
- `new X()` has no call-site record in JS/TS (34 fixtures).
- **WRONG on main, outside this lane's scope, reported not fixed:** `const L = lazy(() => import('./mod')); <L />` resolves Exact to the loader arrow, which prism names `L`; the element renders the default export of `./mod`. F has 20 such sites. Whether `lazy` belongs in the admitted wrapper list is a contract question.

## Where the controller's reading differs from the planner's plan (for the owner, not adjudicated)
1. **Call results.** The plan treats every call result except admitted React wrappers as unsupported. The census says that is where the React population is: destructure-from-call-result is 819 product sites on F and 199 on X, against 9 and 12 for plain or member aliases. On X the checker names a repo implementation for 98 of 252 unresolved call-result sites. A bounded "follow the returned object literal of a same-repo function" step is not in the plan. Its yield on F is unmeasured.
2. **Order of work by measured value.** The plan's order follows the owner's lookup order. By measured sites the largest single item is I1 (T 15,167, X 7, F 0), which is independent of alias-following and could land first.
3. **Refusal on F is most of the measured value there.** With the plan's positive slices F gains 4 sites; the binding-aware refusal removes 109 wrong Exact targets and 2,222 NameOnly candidates.
