# Review brief — PR #372, round 3 (final confirmation): follow `const` aliases to the callable they name (JS/TS)

You are reviewing as a senior engineer who will live with this resolver for years. Rounds 1 and 2 each returned FIX from two independent reviewers. Round 2 found more material items than round 1, in the same three families, so the controller did not treat the review as converging and took it to the owner, who approved exactly one more round. This is that round and it is the last. The rework since round 2 was aimed at closing each family by construction instead of by input; your job is to judge whether it did.

## The goal this change serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves a JS/TS call `f()` or element `<F />` by asking what the name is bound to. On main, when the nearest binding is a local alias (`const f = imported;`, `const f = N.f;`, `const { f } = N;`) the binder answers "alias, keep base behavior", and base behavior never looks at what the alias names. The owner's direction: **the nearest binding decides, and an alias is followed to what it names.** This PR adds the following of `const` aliases, and nothing else.

**The rule the PR claims (this is what to test):** there is one predicate, "the alias route produced a proven hit for this site": exactly one Exact target through the terminal's own route. Only three things depend on it: (i) the resolver returns that hit for the site's `Source` record; (ii) the older textual alias path stands down for that site; (iii) the navigation callers index and `--format callers` count the site once for that exact target (file, name, span). At every site where the predicate is false, the head's records are exactly main's.

`Exact` in prism is a static-binding grade (see `CLAUDE.md`), not runtime truth.

## Decided by the owner; do not raise as findings

- **Taint descent through a followed alias is a follow-on slice** (issue #376, item 1). `nav taint-reaches` selects call sites by the callee's spelling (`src/cpg/trace.rs`), so it stops at a followed-alias call where the direct call reaches; main does not reach either. Not changed in this PR.
- **Proving CommonJS and written-`let` exports on the alias route is a follow-on slice** (#376, item 2). In this PR such a site is not a proven hit, so it keeps main's records (for an alias under the import's own name that includes main's textual edge).
- Also recorded as follow-ups, unchanged here: `callers_of` / `resolve_callers` / the diff-scope skeleton stay keyed by spelling (#376, item 3); a direct call to an import dropped by the caller-locals guard; the direct routes' span-less export edge.

## What to read

Worktree: `/Users/wesleyjinks/code/prism-js-alias`, branch `feat/js-alias-follow`, head `{{HEAD}}`. Whole PR: `git diff origin/main...HEAD`. Rework since round 2: `git diff d22f8349..HEAD -- src tests/integration mutants CLAUDE.md`. `gh pr view 372`, `gh pr checks 372`.
- Reports of both earlier rounds: `/Users/wesleyjinks/prism-evidence/js-alias/review-L/{sol,opus,r2-sol,r2-opus}.md`.
- The predicate and its three uses: `src/resolution_js_alias.rs` (`js_ts_alias_route`, `js_ts_alias_route_hit`), the `None`-qualifier arm in `src/resolution.rs`, `compute_indirect_call_sites` / `recompute_indirect_calls` in `src/call_graph.rs` (the `stand_down` parameter), `build_resolved_call_edges` in `src/navigation/mod.rs`, `to_callers_output` in `src/output/review.rs`.
- Binder: `src/ast/js_binding_alias.rs` (pattern admission now also requires every bound name in the pattern to be unique).
- The structural test: `tests/integration/js_alias_rule_test.rs`. For every fixture of the matrix and every `Source` site in it whose binding is an `Alias*` variant and whose predicate is false, it resolves the site again with the binding rewritten to main's `MayCall("alias")` and requires identical records; and through a seam (`CallGraph::audit_indirect_records`) it requires that the textual records the graph holds differ from "stand-down forced off" only at sites where the predicate is true.
- Other tests: `src/ast/js_binding_alias_tests.rs`, `js_binding_alias_pattern_tests.rs`, `tests/integration/js_alias_follow_test.rs`, `js_alias_follow_fallback_test.rs`, `js_alias_callers_test.rs`; mutation lane `mutants/js-alias-follow.json` (45 mutants, all killed, authoritative); the matrix `tests/fixtures/js_alias_matrix/` (1,349 fixtures; every input named in rounds 1 and 2 is a row).

## How each round-2 finding was handled

| Finding (round-2 id) | Handling |
|---|---|
| A pattern binding ANOTHER name twice (`{ f, x: a, f: a }`) still got an edge (sol W1) | Admission collects every bound identifier of the pattern and refuses on any repeat. Control: the same property key bound to two different identifiers stays followable. Matrix shape `ns-destructure-dup-other-name`. |
| The callable terminal returned its own drop (`WrappedExportNonJsx`) instead of main's result (sol W2, Opus W2) | `js_ts_alias_route` returns `Some` only for a singleton Exact target, for all three terminals. Matrix shape `ident-of-wrapped-memo`. |
| The textual path stood down by binding kind, removing main's records where the alias was not proven, including a correct edge for an alias of a CommonJS import (sol W3, Opus W1) | The stand-down uses the predicate. Both build paths run export resolution before `recompute_indirect_calls` (implementer verified by reading: `call_graph.rs` near 2103, `cpg/build.rs` near 763). The earlier disclosed exception (a span-less named import losing a textual record) is gone: that site keeps main's two records. Matrix shape `ident-own-name-cjs-import`. |
| `import_proven` leaked into the fall-through, changing a drop reason (Opus W3) | The import proof applies only while resolving through the alias route. |
| `--format callers` credited a followed alias to every function of the target's name (sol W4) | `to_callers_output` keeps the target's `FunctionId` and credits the site to that exact function. |
| Counter semantics, stale module comment, `CLAUDE.md` line (Opus S3, sol) | `kept_main` now means the site's records equal main's; the `unresolved` outcome no longer exists; comment and doc fixed. |
| Taint descent (sol W5, Opus S1); CommonJS exports not proven (Opus S2) | Follow-on slices by owner decision (see above). |

One place where the head knowingly differs from main with the predicate TRUE, pinned and commented: `function f() { function f() {}` (two functions of one name starting on one line) with `const h = f; h()`: the alias route proves the outer `f` by start and end line, so the head has one `Source` record to `f@1-3`; main has a dropped `Source` record plus a textual record binding both functions.

## Evidence from the controller

{{EVIDENCE}}

## Questions

1. **Is the rule true by construction?** Read the predicate and its three uses, then every other place that reads the three `Alias*` variants (round 2 counted six) and every reader of a followed-alias site in the callers paths. Is there any code path where a site with a false predicate can produce records that differ from main's? Is there one where a site with a true predicate produces something other than one `Source` record with that one Exact target?
2. **Is the structural test sound?** Does `js_alias_rule_test.rs` actually test the rule (could it pass while the rule is false: vacuity, the rewrite not matching main's real binding, the seam not matching what the build does, sites it never visits such as JSX, non-`Source` records, sites outside the matrix shapes)? It compares against a rewritten binding in-process, not against a build of main; the controller's two-binary sweep above is the independent check.
3. **Closure of the round-2 rows above.** For each: closed, or give the input that still shows it (result on this branch and on main).
4. **Pattern admission.** With "every entry flat" plus "all bound names unique", is there any early-error or otherwise invalid declaration that is still followed?
5. **Anything new and material introduced by the rework** (the `stand_down` parameter and the `audit_indirect_records` seam are new public-but-hidden surface; the formatter now carries identities through its traversal).

## How to report

Tag every finding **WRONG** (name the input and the incorrect result, on this branch and on main) or **SMELL**, and **MATERIAL** or **IMMATERIAL** to the goal above. WRONG first. For each: evidence; a recommended fix and one alternative with its cost; when it would not apply. Do not re-raise items of earlier rounds unless the rework left them open, and do not raise the owner-decided follow-ons. A wrong edge that exists identically on main and is not made more reachable by this PR is a one-line follow-up, not a finding.

There is no round after this one. Say **APPROVE** if nothing material remains. If something material remains, say **FIX** and state plainly whether it is one more instance of a family already seen (in which case say what would close the family, not the instance) or something bounded and new. End with the verdict line, a "checked and found sound" list and a "not checked" list.

## Rules

- You may read anything in the worktree and run its tests (`cargo test --lib js_binding_alias`, `cargo test --test integration js_alias`). To compare behavior on small synthetic fixtures you write, use the two prebuilt release binaries (do not rebuild): this branch `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/L4-prism`, main `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/main-38a59994-prism` (main has since gained only MCP and documentation commits); `<binary> nav --cache-dir <fresh dir in your scratch> call-stats --repo <fixture dir> --dump-sites` (JSON lines with `record_kind` = `call_site`), `<binary> nav --cache-dir <dir> callers --repo <dir> --symbol <name>`.
- Do NOT run the matrix test with `PRISM_BLESS_JS_ALIAS_MATRIX` set in the worktree.
- Write scratch files only under your scratch directory (given below) and use it as the working directory of any script. Do not modify, add or delete any file in the worktree; make no git writes of any kind and post nothing to GitHub. Keep file searches inside the worktree and your scratch directory; do not list or search other directories under `~/code` or `~/prism-evidence`.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-`, `f-alias-` or `F-private`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
