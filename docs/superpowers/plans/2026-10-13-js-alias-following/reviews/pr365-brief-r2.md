# Review brief — PR #365, round 2 of 2 (confirmation): table-driven matrix pinning JS/TS callee resolution

You are reviewing as a senior engineer who will live with this test for years. This is the second and last review round for this PR. Round 1 returned FIX from two independent reviewers; the findings were folded. Your job is to confirm the fold, not to re-review from scratch: check that each round-1 finding is actually closed, that the code and fixtures the fold added are sound, and whether the fold introduced anything new and material.

## The goal this PR serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves JS/TS call sites to callees. A previous attempt to fix resolution through local aliases was reviewed twice and each round found new alias shapes the rule mishandled. The owner's direction: enumerate the variant space up front and test against it. This PR is that enumeration as a permanent repo test. The next product slices (import-path resolution, PR #368; following `const` aliases; refusing the by-name search for bound names; later, following values returned by hooks) will each change rows, and the diff of the pinned expectations file is how reviewers see exactly what each slice changed.

The test is good if: (1) it reproduces prism's real behavior, the same as the CLI; (2) it reports every differing row; (3) a regression cannot be blessed silently; (4) the authored ground truth is right; (5) the enumeration has no important hole for the upcoming slices; (6) it is reliable on CI.

## What to read

Worktree: `/Users/wesleyjinks/code/prism-js-matrix`, branch `test/js-alias-variant-matrix`, head `35557277`, four commits on `origin/main`. The fold is the last two commits: `git diff e9d411cc..HEAD` (harness: `eaa758b1`; fixtures and generator: `35557277`). `gh pr view 365` has the updated description; `gh pr checks 365` the CI state.
- Round-1 reports: `/Users/wesleyjinks/prism-evidence/js-alias/review-365/sol.md` and `/Users/wesleyjinks/prism-evidence/js-alias/review-365/opus.md`.
- `tests/fixtures/js_alias_matrix/README.md`, `fixtures.jsonl` (1,159), `expected.jsonl`, `accepted_changes.jsonl`, `generator/*.cjs`.
- `tests/integration/js_alias_matrix_common.rs`, `js_alias_matrix_guard.rs`, `js_alias_matrix_test.rs`.

## How each round-1 finding was handled

| Finding (round-1 id) | Handling |
|---|---|
| Stack overflow at ≤4 rayon threads; CI red (Opus W1, sol S2) | The fixture loop runs inside `prism::build_pool::install`. Implementer ran the test binary at `RAYON_NUM_THREADS` 1 (×3), 2, 4 (×3) and unset: all pass; with the wrapper removed and 2 threads it aborts. Controller reran at 1 and 4 on the final fixtures: pass. |
| `memo()` result called directly has truth `one` (sol R1, Opus W2) | Truth is `none` for the 12 call-form rows; JSX rows unchanged. Pins moved MISSING → NONE-OK. |
| `param.shadows-import` truth `none` (sol R2, Opus S5) | Truth is `may: []`. Pins moved NONE-OK → MAY-NONE; recorded in the ledger because leaving NONE-OK is in the regression set. |
| Identity = file AND (name OR line) (sol R3) | Identity = file AND name AND (no truth line OR same line), in the Rust harness and in `generator/run.cjs`; the exporter rejects a truth target with two unmarked declarations (`// TARGET:<name>` disambiguates). No existing row changed verdict. |
| Deleting a pinned row erases history (sol R4, Opus W5); accepted id is a permanent waiver (Opus W3) | Bless compares with `git show HEAD:…/expected.jsonl`; a fixture id absent from that baseline is new and never a regression; entries are `{id, from, to, reason}` and excuse exactly that transition; the normal run fails on a stale entry (unknown id, or `to` ≠ the pinned verdict); bless refuses orphan rows and writes atomically. Regression set widened (Opus S4): into WRONG / OVERCLAIM / NONE+NAMEONLY / NO-RECORD, or out of CORRECT / NONE-OK. |
| README told authors to copy an exported `expected.jsonl` (Opus W4) | `export.cjs` writes `fixtures.jsonl` only; pins come solely from bless; README rewritten. |
| UTF-8 vs UTF-16 sort order (sol R5) | Moot: the exporter no longer writes pins, so the Rust renderer is the only writer. Not otherwise changed. |
| Generator over 600 lines (sol R6, Opus S10) | Split into `variants-common/-local-shapes/-local/-params/-imports.cjs` and `lib.cjs`. The split was checked to regenerate the previous 1,011 fixtures byte-identically apart from the intended edits. |
| Missing shapes (sol S1; Opus S1, S2, S3) | Added: `rhs-shadowed-at-site`; `ns-receiver-is-parameter`; `ns-via-alias-hop`; `same-stem-two-dirs` (+ `.js`-suffix twin); `hook-result-two-keys`, `hook-result-two-slots`; `ident-own-name-import`, `ident-same-file-fn`, `ident-rhs-is-outer-const`, `ident-rhs-is-catch-param`, `ident-rhs-is-outer-param`, `ident-rhs-is-own-param` (main's older textual alias path now fires in fixtures); naming `trap2` (two same-named decoys) on six shapes, so 12 rows carry NameOnly candidates; `dir-index`, `parent-file`, `file-beats-directory`, `js-suffix-is-not-a-directory`. |
| Policy rows stated as truth (sol S3) | The both-`x.js`-and-`x.ts` row has one target with its contract text saying it is prism's policy; the context fixture exports its context so an invisible provider is constructible. |
| Script-global truths hold only under script loading (Opus S6) | Contract text added to those rows. |
| No structural check on truths; checker cross-check not asserted (Opus S9) | New Rust test `every_truth_points_at_real_source`; `generator/run.cjs` exits non-zero on an unexplained difference from the TypeScript checker (four shapes are listed there with reasons). |
| PR description wrong on WRONG families and silent on CI (Opus S12) | Rewritten. |
| Not changed, by decision: `react-lazy` pinned WRONG (Opus S7: it is a contract row and says so); expected rows carry no truth digest (Opus S8, sol's note: fixture diffs are reviewed alongside); same-verdict target changes are reported but not guarded. |

## Questions

1. **Closure.** For each row above: is the finding closed? If not, give the input or state that still shows the defect.
2. **The fold's own code.** Is the guard logic in `js_alias_matrix_guard.rs` right, including its interaction with git (run from a subdirectory, a shallow or detached checkout, a first commit where the baseline does not exist, CI where bless never runs)? Is the `build_pool::install` wrapper a real fix for the nesting, or does it only move the threshold? Is the tightened identity rule applied consistently in Rust and in the reference?
3. **New fixtures and corrected truths.** Check the authored truth of every NEW shape listed above and the corrected ones by reading the fixture source; the new shapes are the least reviewed part of this PR. Report any wrong or arguable truth with the fixture id.
4. **Anything new and material introduced by the fold.**
5. **Remaining holes for the upcoming slices**, only if a planned slice could be wrong on the shape without any row noticing.

## How to report

Tag every finding **WRONG** (name the input or state and the incorrect result) or **SMELL** (a risk or gap with no demonstrated incorrect behavior), and **MATERIAL** or **IMMATERIAL** to the goal above. WRONG first. For each: evidence; a recommended fix and one alternative with its cost; when it would not apply. Do not re-raise a round-1 item as new unless the fold left it open.

This is the last round. Classify what remains: if nothing material remains, say APPROVE. If closed, enumerable items remain, list them precisely so they can be fixed without another review (FIX). End with the verdict line, a "checked and found sound" list and a "not checked" list.

## Rules

- You may read anything in the worktree, run `cargo test --test integration js_alias_matrix` there (also with `RAYON_NUM_THREADS` set), and run Node on small synthetic fixtures you write. A pinned TypeScript checker is at `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`. Do NOT run the test with `PRISM_BLESS_JS_ALIAS_MATRIX` set in the worktree (it writes a tracked file); to exercise bless, copy the three test files' logic or the fixture directory into your scratch directory.
- Write scratch files only under your scratch directory (given below). Do not modify, add or delete any file in the worktree; make no git writes of any kind and post nothing to GitHub.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-` or `f-alias-`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
