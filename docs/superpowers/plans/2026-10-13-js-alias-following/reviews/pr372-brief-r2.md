# Review brief — PR #372, round 2 of 2 (confirmation): follow `const` aliases to the callable they name (JS/TS)

You are reviewing as a senior engineer who will live with this resolver for years. This is the second and last review round for this PR. Round 1 returned FIX from two independent reviewers; the findings were folded. Your job is to confirm the fold: check that each round-1 finding is actually closed, that the code the fold added is sound, and whether the fold introduced anything new and material.

## The goal this change serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves a JS/TS call `f()` or element `<F />` by asking what the name is bound to. On main, when the nearest binding is a local alias (`const f = imported;`, `const f = N.f;`, `const { f } = N;`) the binder answers "alias, keep base behavior", and base behavior never looks at what the alias names. The owner's direction: **the nearest binding decides, and an alias is followed to what it names.** This PR adds the following of `const` aliases.

**The slice's promise was tightened after round 1** (both reviewers showed the original contract removed edges main had): at every site this PR has exactly one of two effects: (1) the alias is followed AND its terminal is proven → one Exact target through the terminal's own route; or (2) anything else → exactly main's records for that site. It never refuses a name search; that is a later slice, deliberately held until slices that prove more values have landed.

`Exact` in prism is a static-binding grade (see `CLAUDE.md`), not runtime truth.

## What to read

Worktree: `/Users/wesleyjinks/code/prism-js-alias`, branch `feat/js-alias-follow`, head `d22f8349`. The branch contains merges of main: use `git diff origin/main...HEAD` for the whole PR and `git diff 3dd5b7db..HEAD -- src tests/integration mutants CLAUDE.md` for the fold (that range also contains a merge of main's unrelated MCP work; ignore `src/mcp`, `src/bin`, `docs`). `gh pr view 372`, `gh pr checks 372`.
- Round-1 reports: `/Users/wesleyjinks/prism-evidence/js-alias/review-L/sol.md` and `/Users/wesleyjinks/prism-evidence/js-alias/review-L/opus.md`.
- Binder: `src/ast/js_binding_alias.rs`. Resolver: `src/resolution_js_alias.rs` (`js_ts_alias_route`) and the `None`-qualifier arm in `src/resolution.rs`. Site mapping, stand-down: `src/call_graph.rs`. Consumers: `src/navigation/mod.rs` (`build_resolved_call_edges`, `js_ts_alias_route_hit`), `to_callers_output`, the `js_alias_follow` counters in `nav call-stats`.
- Tests: `src/ast/js_binding_alias_tests.rs`, `src/ast/js_binding_alias_pattern_tests.rs`, `tests/integration/js_alias_follow_test.rs`, `js_alias_follow_fallback_test.rs`, `js_alias_callers_test.rs`; mutation lane `mutants/js-alias-follow.json`; the matrix `tests/fixtures/js_alias_matrix/` (`git diff origin/main...HEAD -- tests/fixtures/js_alias_matrix/expected.jsonl`).

## How each round-1 finding was handled

| Finding (round-1 id) | Handling |
|---|---|
| A pattern that binds a name twice (`{f, ...f}`, `{f, x:{f}}`, `{f: a, ['x']: a}`, `{f: a, 'x': a}`; early SyntaxError) got an Exact edge (sol W1) | Fixed by whole-pattern admission: a flat object pattern is followed only when every entry is a shorthand identifier or a plain-key pair with an identifier value. Any rest, nested, computed, string or numeric key, or default → not followed. Consequence: the valid `{ f, ...rest } = N` and `{ f, x: { y } } = N` are no longer followed. Matrix rows `ns-destructure-dup-rest`, `ns-destructure-dup-key`, `ns-destructure-rest-sibling`, `ns-destructure-nested-sibling` (all equal to main on this head). |
| A followed alias whose terminal is not proven dropped an edge main had: an import whose export is a const alias (sol S1); a namespace through a tsconfig `paths` alias or a CommonJS `exports` module (Opus W2) | Contract changed (see the promise above): `js_ts_alias_route` returns a proven hit or `None`; on `None` the site continues exactly as an unfollowed alias does on main. No support was added for path-alias or CommonJS namespace members. Matrix rows `ident-of-exported-alias`, `ns-member-exported-alias` (naming `same`), `paths-ns-destructure-nested`, `cjs-exports-ns-destructure-nested`; tests in `js_alias_follow_fallback_test.rs`. |
| A span-less export bound a nested same-named function from alias sites (Opus W4) | Both alias routes now require a resolved export WITH a span; otherwise not a proven hit (so main's behavior). The direct `w()` / `N.w()` routes are unchanged (same wrong edge on main; recorded as a follow-up). Matrix row `spanless-export-nested-decoy`. |
| `nav callers` did not list a followed-alias site, and lost a caller main reported (Opus W1) | `build_resolved_call_edges` counts a site once for its target when the alias route produced the hit, whatever the spelling. `--format callers` (`to_callers_output`, which walks a name-keyed map) now includes followed-alias sites that resolved to the walked name. Tests: `js_alias_callers_test.rs`. NOT changed and disclosed: `CallGraph::callers_of` and `resolve_callers` (`caller_sites_scoped`, name-keyed) still miss alias callers. |
| End-span identity not independently tested (sol S3); stand-down mutant for the namespace variant, nested-sibling unit row (Opus S1) | Test `a_member_binds_the_outer_function_by_its_end_line_too` and mutant `AF-31`; mutants `AF-35`, `AF-41`; `src/ast/js_binding_alias_pattern_tests.rs`. Lane: 41 mutants, all killed (authoritative). |
| No counter for followed aliases (Opus S2) | `nav call-stats` reports `js_alias_follow` keyed `<callable|import|namespace_member>_<proven|kept_main|unresolved>`. |
| `CLAUDE.md` sentence said "only" (sol S2, Opus W3) | Reworded: describes the followed route; an alias that is not followed, or whose terminal is not proven, keeps the earlier resolution. |
| PR description mislabeled the field-only rows and said rest/nested destructuring "is followed" (Opus W5); undisclosed effect: arguments at a followed-alias site now bind to the target's parameters, so data-flow and taint can cross it (Opus S4) | Description rewritten. |
| Recorded as follow-ups, not changed: a direct call to an import dropped by the caller-locals guard where an alias of it resolves (Opus S5); the visited set is redundant with the bound (Opus S3, sol agreed it is equivalent); the direct routes' span-less export edge; `callers_of` / `resolve_callers`. |

One disclosed difference from "exactly main's records": for a span-less named import whose alias is followed but not proven, main had a second, textual `IndirectResolution` record carrying the same wrong edge to the nested function; the stand-down (keyed on the binding kind) removes that record, so the head has one dropped `Source` record. It removes a wrong edge; it is pinned in `js_alias_follow_fallback_test.rs`.

## Measured effect of the folded head

Release binary of the folded head (`d22f8349`) against the release binary of main `38a59994`, every `call-stats --dump-sites` record compared (X = Excalidraw, T = the TypeScript compiler's `src`, F = a private React application, aggregates only):

| | X | T | F |
|---|---:|---:|---:|
| Call-site records compared | 19,219 | 61,712 | 13,299 |
| Sites that gained an Exact edge | 1 | 0 | 3 |
| … confirmed | 1 (TypeScript 5.9.3 checker) | n/a | 3 (module file and export name derived from syntax and the app's path aliases) |
| Record added or removed; existing target changed or removed; any site resolved → dropped | 0 | 0 | 0 |

The result is identical to the pre-fold measurement. The corpus yield is small by design of this slice (plain `const` aliases of imports are rare); its purpose is the rule that later slices build on. Matrix (1,335 fixtures; 166 are new in this PR): against main's 1,169 pinned rows, 278 verdict moves (239 MISSING → CORRECT, 39 WRONG → CORRECT) and 25 field-only changes (19 where only the kind changes from `free_single` to `import_member`; 6 where the textual `IndirectResolution` record is gone because the alias is followed). No ledger entry. The new fixtures include every input the two round-1 reports named. CI: all nine checks pass on `d22f8349`; one test shard failed on its first attempt on `mcp::lazy::tests::lazy_construction_starts_no_build_until_the_first_readiness_check` (a 1-second-wait test in code this PR does not touch, tracked in issue #375) and passed when rerun on the same commit.

## Questions

1. **Closure.** For each row above: is the finding closed? If not, give the input that still shows the defect, with the result on this branch and on main.
2. **The promise.** Is it true as stated? Name any input where the head's records at a site differ from main's and the difference is not a proven hit through the alias route (other than the disclosed textual-record case). Pay attention to the stand-down being keyed on the binding kind while the fallback is decided at resolution time.
3. **The fold's own code.** Whole-pattern admission; `js_ts_alias_route` and the fall-through; the span requirement on both routes; the callers-index count (exactly once, also when the alias spelling equals the target name; no effect on non-alias sites) and the `--format callers` change; the counters.
4. **Anything new and material introduced by the fold.**
5. **The undisclosed-consumer question from round 1.** Besides `callers_of` / `resolve_callers`, is there another consumer keyed by callee spelling that now disagrees with the resolved edges for a followed alias?

## How to report

Tag every finding **WRONG** (name the input and the incorrect result, on this branch and on main) or **SMELL**, and **MATERIAL** or **IMMATERIAL** to the goal above. WRONG first. For each: evidence; a recommended fix and one alternative with its cost; when it would not apply. Do not re-raise a round-1 item as new unless the fold left it open. A wrong edge that exists identically on main and is not made more reachable by this PR is a one-line follow-up, not a finding.

This is the last round. If nothing material remains, say APPROVE. If closed, enumerable items remain, list them precisely so they can be fixed without another review (FIX). End with the verdict line, a "checked and found sound" list and a "not checked" list.

## Rules

- You may read anything in the worktree and run its tests (`cargo test --lib js_binding_alias`, `cargo test --test integration js_alias`). To compare behavior on small synthetic fixtures you write, use the two prebuilt release binaries (do not rebuild): this branch `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/L3-prism`, main `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/main-38a59994-prism` (main has since gained only MCP and documentation commits); run `<binary> nav --cache-dir <fresh dir in your scratch> call-stats --repo <fixture dir> --dump-sites` and read the JSON lines whose `record_kind` is `call_site`; `<binary> nav --cache-dir <dir> callers --repo <dir> --symbol <name>` for the callers index.
- Do NOT run the matrix test with `PRISM_BLESS_JS_ALIAS_MATRIX` set in the worktree.
- Write scratch files only under your scratch directory (given below); if a script needs a working directory, use your scratch directory, never the worktree. Do not modify, add or delete any file in the worktree; make no git writes of any kind and post nothing to GitHub.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-`, `f-alias-` or `F-private`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
