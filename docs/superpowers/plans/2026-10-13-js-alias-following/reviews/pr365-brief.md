# Review brief — PR #365: table-driven matrix pinning JS/TS callee resolution (test-only)

You are reviewing as a senior engineer who will live with this test for years. Round 1 of at most 2.

## The goal this PR serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves JS/TS call sites to callees. A previous attempt to fix resolution through local aliases was reviewed twice, and each round found new alias shapes the rule mishandled. The owner's direction: enumerate the variant space up front and test against it, instead of fixing one review finding at a time. This PR is that enumeration as a permanent repo test. The next product slices (import-path resolution; following `const` aliases; refusing the by-name search for bound names; later, following values returned by hooks) will each change rows, and the diff of the pinned expectations file is how reviewers will see exactly what each slice changed.

So the test is good if: (1) it reproduces prism's real behavior, the same as the CLI; (2) it reports every differing row, not the first; (3) a regression cannot be blessed silently; (4) the authored ground truth is right, because later slices will be judged against it; (5) the enumeration has no important hole for the upcoming slices.

## What to read

Worktree (read it there; branch `test/js-alias-variant-matrix`, two commits on top of `origin/main`): `/Users/wesleyjinks/code/prism-js-matrix`
- `tests/fixtures/js_alias_matrix/README.md`
- `tests/fixtures/js_alias_matrix/fixtures.jsonl` (1,011 fixtures: `id`, `step`, `shape`, `scope`, `form`, `naming`, `lang`, `contract`, `site_file`, `site_line`, `truth`, `files`)
- `tests/fixtures/js_alias_matrix/expected.jsonl` (pinned behavior on main)
- `tests/fixtures/js_alias_matrix/generator/{variants.cjs,run.cjs,export.cjs}` (how fixtures, truth and the first expectations were produced; `run.cjs` holds the reference `prismSite`, `matcher`, `verdict`)
- `tests/integration/js_alias_matrix_common.rs`, `tests/integration/js_alias_matrix_test.rs`
- For context on how prism resolves these sites today: `src/ast/js_binding.rs` (the binder), `src/resolution.rs` (`resolve_call_site`, the `None`-qualifier arm near line 3398), `CLAUDE.md` (the call-resolution section, including what `Exact` means: a static-binding grade, not runtime truth).

`gh pr view 365` and `gh pr diff 365` work from that directory.

## Questions

1. **Harness fidelity.** Does the test obtain the same records `prism nav call-stats --dump-sites` emits, and is the port of record collection, target matching and the verdict function faithful to `run.cjs`? Name any input where the Rust and the reference would disagree.
2. **Reporting and the bless guard.** Can a behavior change go unreported? Can a regression (a row moving into `WRONG`/`OVERCLAIM`, or out of `CORRECT`) be written into `expected.jsonl` without an `accepted_changes.jsonl` entry? Consider reordered or deleted rows, new fixtures, a changed truth, a changed verdict with unchanged targets, and a changed target with an unchanged verdict.
3. **Ground truth.** Sample the fixtures across shapes (at least every shape whose `contract` field is non-null, every shape in steps `global`, `import-path`, `none` and `param-lookup`, and a spread of `local` shapes) and check the authored `truth` by reading the fixture source as JavaScript/TypeScript. Report any fixture whose truth is wrong or arguable, with the id and your reading. A wrong truth is the most damaging defect this PR can have.
4. **Pinned WRONG rows.** The PR pins 113 `WRONG` rows as documentation of current behavior. Is that the right call, and is the `WRONG` label right for the ones you sample?
5. **Coverage for the next slices.** Given the upcoming slices named above, which static alias, import-path or shadowing shape that occurs in ordinary JS/TS/React code is missing? Name the shape and why it would behave differently from every shape present. A hole is MATERIAL only if a planned slice could be wrong on it without any existing row noticing.
6. **Hygiene.** Hermetic and deterministic (temp dirs, rayon, no cache)? Repo rules respected (files under 600 lines, `tests/integration/main.rs` wiring, the coverage matrix lists in `tests/integration/coverage_test.rs` unaffected)? Anything that will rot?

## How to report

Tag every finding **WRONG** (the code or a fixture provably does or states the wrong thing: name the input or state and the incorrect result) or **SMELL** (a risk or gap with no demonstrated incorrect behavior). A finding without a concrete failure scenario is a SMELL. Also tag each **MATERIAL** or **IMMATERIAL** to the goal above. List WRONG items first.

For each finding give: the evidence (file and line, fixture id, or the command you ran and its output); a recommended fix and at least one alternative with its cost; and when the finding would not apply.

End with a verdict: **APPROVE**, **FIX** (list the items that must change before merge) or **REJECT**, and a short "checked and found sound" list and a "not checked" list.

## Rules

- You may read anything in the worktree, run `cargo test --test integration js_alias_matrix` there, and run Node on small synthetic fixtures you write. A pinned TypeScript checker is at `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` if you want to check a truth by type.
- Write scratch files only under your scratch directory (given below). Do not modify, add or delete any file in the worktree; make no git writes of any kind (no commit, stash, checkout, push, PR comment or review submission). The controller folds findings.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-` or `f-alias-`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
