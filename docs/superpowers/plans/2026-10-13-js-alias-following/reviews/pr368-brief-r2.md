# Review brief — PR #368, round 2 of 2 (confirmation): relative imports with an explicit `.js` suffix, bare dots and redundant segments

You are reviewing as a senior engineer who will live with this resolver for years. This is the second and last review round for this PR. Round 1 returned FIX from two independent reviewers; the findings were folded. Your job is to confirm the fold: check that each round-1 finding is actually closed, that the code the fold added is sound, and whether the fold introduced anything new and material.

## The goal this change serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves a JS/TS call through an import by finding the imported module's file. On main, three ordinary spellings of a relative specifier find no file, so every call through them is dropped: `"./x.js"` when the file is `x.ts`/`x.tsx`; `"."` / `".."`; and redundant segments (`./a//b`, `./a/./b`, `./a/../b`). The goal: these shapes find their module.

**The compatibility claim was restated after round 1**, because both reviewers showed the original one ("nothing that resolves today changes") is false at edge level. The claim now is:
1. A specifier that has a candidate on main returns the identical candidate list.
2. A newly found module behaves in every consumer as the canonical spelling of the same module does, with one exception that protects precision: the namespace route does not extend its legacy Exact stem match to a module main did not find (it keeps main's NameOnly outcome there).
3. At edge level the only changes are: gains; removal of an Exact edge where a newly resolved `export *` makes a name ambiguous (the language's own answer); and loss of a NameOnly receiver edge where the canonical spelling's receiver route already drops (an existing limit of that route). Each is pinned by a test. On the three measured corpora only gains occur.

`Exact` in prism is a static-binding grade (see `CLAUDE.md`), not runtime truth.

## What to read

Worktree: `/Users/wesleyjinks/code/prism-js-i1`, branch `feat/js-relative-import-extension`, head `a34a1d45`, nine commits on `origin/main`. The fold is the last five: `git diff aa74eae0..HEAD`. `gh pr view 368` / `gh pr checks 368`.
- Round-1 reports: `/Users/wesleyjinks/prism-evidence/js-alias/review-i1/sol.md` and `/Users/wesleyjinks/prism-evidence/js-alias/review-i1/opus.md`.
- Product: `src/call_graph.rs` (`js_ts_relative_module_candidates`, `js_ts_normalized_module_candidates`, `js_ts_literal_module_candidates`, `is_js_ts_relative_specifier`), `src/resolution_js_namespace.rs` (`js_ts_namespace_outcome`, `js_ts_namespace_proof`, `namespace_sibling`), cache versions, `CLAUDE.md`.
- Tests: `src/call_graph/js_relative_candidates_test.rs`; `tests/integration/js_relative_{extension,namespace,edges,receiver}_test.rs`; mutation lane `mutants/js-relative-candidates.json`.

## How each round-1 finding was handled

| Finding (round-1 id) | Handling |
|---|---|
| Namespace import of a newly found module that lacks the member is upgraded from NameOnly to Exact on an unrelated same-stem file (sol W1, Opus W3) | Fixed. When the positive namespace proof fails and the module is one main did not find (not in the literal candidate list and not found by `namespace_sibling`), the function returns main's NameOnly "E7" outcome instead of `None`. Modules main already finds are untouched, including the canonical case pinned by `d5_authoritative_missing_member_and_fallback`; that legacy fallback is recorded as a follow-up, not changed here. Tests: `tests/integration/js_relative_namespace_test.rs`. |
| Normalization appends extensions to an explicit `.js` name and binds `x.js.ts` (sol W2) | Partly accepted. The fallback now follows TypeScript's order for a target that already ends in a JS/TS extension: the literal file; then, for `.js`, the same stem with `.ts` then `.tsx`; then the appended-extension list; directory index files after the chosen tier. So `./a//x.js` over {`a/x.js.ts`, `a/x.ts`} gives `a/x.ts`. **Not accepted: the claim that a lone `x.js.ts` must stay unresolved.** The pinned TypeScript 5.9.3 `ts.resolveModuleName('./x.js', …)` resolves `x.js.ts` when it is the only file, in Bundler, Node10 and NodeNext resolution, and `x.ts` when both exist (the probe is one `resolveModuleName` call per mode over two two-file directories; rerun it if you doubt it). The fallback keeps that. sol: please confirm or contest this with evidence; if contested it goes to the owner. |
| `./x.d.js` could select `x.d.ts` (Opus S5) | Fixed: no substitution when the stem ends in `.d`. |
| A newly resolved `export *` exposes a star conflict and removes an Exact edge main had (sol W3, Opus S1) | No product change: the removal is the language's answer. Pinned in `tests/integration/js_relative_edges_test.rs` with both controls (same declaration through both stars; second module lacks the name). The compatibility claim was restated (above). |
| Props type through `./props.js` with an inherited method: main NameOnly, branch drops `ExternalReceiver`, as the canonical spelling does on main (Opus W2) | No product change: pinned as equal to the canonical twin and named as the receiver route's existing limit. Rule applied: a new spelling must not mint a wrong Exact, and may inherit the canonical spelling's recall limits. |
| Three mutation-registry anchors orphaned by the cache bump; the pre-merge gate would fail (Opus W1) | Fixed: anchors advanced; `mutgate.py --plan-only` exits 0; the three mutants are KILLED. |
| Surviving mutants and untested consumers (sol S1, Opus S4) | Tests added for the `..` and `.` arms of `directory_only`, the `!directory_only` substitution guard, `trim()`, all six index extensions, and the receiver consumers through a `.js` spelling (typed parameter, constructor local, the two-candidate refusal). New mutation lane `mutants/js-relative-candidates.json`: 17 mutants, all KILLED (authoritative). |
| `CLAUDE.md` sentence overstates sharing (sol S3, Opus S5) | Reworded: scoped to import-member, re-export and namespace resolution; tsconfig `paths` and import-forwarding keep their own rules; the namespace exception named. |
| PR table mixed units; one stored adjudication line was stale (Opus S5) | Description rewritten: site keys and targets stated separately; adjudication outputs regenerated and stored. |
| Recorded as follow-ups, not changed: bare-dot namespace / default-qualifier / import-forward forms and tsconfig-paths `.js` hops (fail closed; sol S2, Opus S2, S3); `package.json` `main` for directory specifiers (Opus W4); the `.jsx` arm of `namespace_sibling` (Opus S6); the canonical namespace stem fallback. |

## Measured effect of the folded head

Controller measurement, release binary of head `a34a1d45` against the frozen main binary, every `call-stats --dump-sites` record compared (X = Excalidraw, T = the TypeScript compiler's `src`, F = a private React application, aggregates only):

| | X | T | F |
|---|---:|---:|---:|
| Call-site records compared | 19,219 | 61,712 | 13,299 |
| Site keys whose targets or drop reason changed | 7 | 15,341 | 0 |
| … dropped (`UnknownName`) → Exact `import_member` | 7 | 15,244 | 0 |
| … receiver-typed method call, NameOnly or dropped → Exact | 0 | 97 | 0 |
| Record added or removed; existing Exact target changed or removed; any site resolved → dropped | 0 | 0 | 0 |
| New Exact targets | 7 | 15,419 | 0 |
| … that agree with the TypeScript 5.9.3 checker's resolved declaration | 7 | 15,419 | n/a |

The per-site diff is byte-identical to the one measured before the fold on X and T, so the fold changed only the corner cases it targeted. The checker comparison counts 2,775 of T's targets as agreeing because the checker names an overload signature and the target is that function's implementation. On the 1,159-fixture matrix of PR #365 the head changes 18 rows, all module-path rows from MISSING to CORRECT; the only module-path row left unresolved is `.jsx` naming a `.tsx` file, which is out of scope. CI is green on all nine checks.

## Questions

1. **Closure.** For each row above: is the finding closed? If not, give the input that still shows the defect, with the result on this branch and on main.
2. **The fold's own code.** `js_ts_namespace_outcome`: is "found on main" computed correctly (think of `.js` spellings `namespace_sibling` already handled on main, bare dots, redundant segments, a specifier found by both)? Can the new early return change any outcome for a module main already found? `js_ts_normalized_module_candidates`: is the tiering right, and does any tier let a canonical (literal-list) answer change?
3. **The restated compatibility claim.** Is it now true as written? Name any consumer where a newly found module produces an edge change outside the three listed kinds.
4. **Anything new and material introduced by the fold.**
5. **The disputed point** (lone `x.js.ts`): state your position with evidence.

## How to report

Tag every finding **WRONG** (name the input and the incorrect result, on this branch and on main) or **SMELL**, and **MATERIAL** or **IMMATERIAL** to the goal above. WRONG first. For each: evidence; a recommended fix and one alternative with its cost; when it would not apply. Do not re-raise a round-1 item as new unless the fold left it open.

This is the last round. If nothing material remains, say APPROVE. If closed, enumerable items remain, list them precisely so they can be fixed without another review (FIX). End with the verdict line, a "checked and found sound" list and a "not checked" list.

## Rules

- You may read anything in the worktree and run its tests (`cargo test --lib js_relative_candidates`, `cargo test --test integration js_relative`). To compare behavior on small synthetic fixtures you write, use the two prebuilt release binaries (do not rebuild): this branch `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/i1c-prism` (built from `a34a1d45`), main `/Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism`; run `<binary> nav --cache-dir <fresh dir in your scratch> call-stats --repo <fixture dir> --dump-sites` and read the `call_site` records (the discriminator field is `record_kind`).
- Write scratch files only under your scratch directory (given below). Do not modify, add or delete any file in the worktree; make no git writes of any kind and post nothing to GitHub.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-`, `f-alias-` or `F-private`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
