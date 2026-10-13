# JS/TS alias variant matrix

Enumerated tiny repos (one per line of `fixtures.jsonl`), each with one call site
marked `// SITE` and an authored ground truth for what that site statically calls.
The test (`tests/integration/js_alias_matrix_test.rs`) runs prism on every fixture
and compares a derived row with the pinned row in `expected.jsonl`. It pins what
prism does today, so a product slice shows up as a diff of `expected.jsonl`, row
by row.

## Files

- `fixtures.jsonl`: one fixture per line: `id`, `step`, `shape`, `scope`, `form`,
  `naming`, `lang`, `contract`, `site_file`, `site_line` (1-indexed), `truth`,
  `files` (path to source).
- `expected.jsonl`: one row per fixture, same order: `id`, `verdict`, `record`,
  `exact`, `nameonly`, `drop`, `origins`.
- `accepted_changes.jsonl`: the ledger of accepted regressions still in effect
  (see "Updating after a slice"). Empty by default.
- `generator/`: dev-time Node scripts that produce `fixtures.jsonl`.

`truth` is one of: `one` (exactly one in-repo callable), `set` (a closed set of
in-repo callables), `may` (possibly these; may be empty; not provable from
visible code), `none` (no in-repo callable is the callee), `anon` (the anonymous
callable on the `// ANON` line).

## What a row is

The test calls the same library path as `prism nav call-stats --dump-sites`
(in-process, no cache) and keeps the `call_site` records whose `source_span`
matches `site_file`/`site_line`. `exact` and `nameonly` are those records'
resolved targets split by confidence (`exact` vs anything else), sorted. `drop`
is the distinct drop reasons joined with `+`; `origins` the distinct origins,
sorted. `record: false` means no record exists at the site.

## Callable identity

A resolved target is the truth target when it has the same file, the same name,
and the same declaration line when the truth records a line (a truth without a
line matches on file and name). A matching name or a matching line alone is not
enough. An `anon` truth matches on file and the `// ANON` line.

## Verdicts

- `NO-RECORD`: no call-site record exists at the site.
- `CORRECT`: every exact target is in the truth and all truth targets are covered
  (for `anon`: every exact target is the anonymous callable).
- `PARTIAL`: `set` truth only; exact targets are all in the truth but some truth
  targets are not covered.
- `DEMOTED`: no exact target, but a name-only target is in the truth (`one`/`set`/`anon`).
- `MISSING`: no exact target, and no name-only target is in the truth.
- `WRONG`: an exact target is outside the truth (for `none`: any exact target;
  for `anon`: an exact target is not the anonymous callable).
- `OVERCLAIM`: `may` truth; exact targets exist and all are in the truth, which
  is not provable from visible code.
- `MAY-NAMEONLY`: `may` truth; no exact target, a name-only target is in the truth.
- `MAY-NONE`: `may` truth; no exact target, no name-only target in the truth.
- `NONE-OK`: `none` truth; no exact and no name-only target.
- `NONE+NAMEONLY`: `none` truth; no exact target, but a name-only one.

## Run

```bash
cargo test --test integration js_alias_matrix
```

The test reports every differing row (expected row, actual row) and a count by
`(step, expected verdict -> actual verdict)`. Nothing here needs Node. It also
checks the fixture file itself: unique ids, a `// SITE` on every site line, truth
targets that name real files, lines and (for `anon`) a `// ANON` marker, and an
expected row for every fixture.

## Updating after a slice

1. Run with the bless variable, then review the `expected.jsonl` diff:

   ```bash
   PRISM_BLESS_JS_ALIAS_MATRIX=1 cargo test --test integration js_alias_matrix_test::matrix_matches
   git diff tests/fixtures/js_alias_matrix/expected.jsonl
   ```

   Every changed row is a behavior change the PR must explain.
2. Bless compares with the committed baseline, not the working tree: it reads
   `expected.jsonl` from `git show HEAD:...`, so it needs git and a committed
   `expected.jsonl`, and editing a pinned row by hand cannot hide a change. A
   fixture id absent from the baseline is a new fixture and is never a regression.
3. A row regresses when its new verdict is `WRONG`, `OVERCLAIM` or `NONE+NAMEONLY`;
   or its old verdict is `CORRECT` or `NONE-OK` and the new one differs; or its new
   verdict is `NO-RECORD` and the old one was not. Bless refuses, and writes
   nothing, while any regression is unaccepted.
4. To accept a regression, add `{"id": ..., "from": ..., "to": ..., "reason": ...}`
   to `accepted_changes.jsonl`. It excuses exactly that transition for that id
   (`from` is the baseline verdict, `to` the new one).
5. The file is a ledger of accepted regressions still in effect. The normal run
   fails on a stale entry: an id that is not a fixture, or an entry whose `to` is
   not the currently pinned verdict of that id. When a later change improves the
   row, delete its entry in that same change.
6. Bless also refuses when a pinned row names no fixture; delete that row from
   `expected.jsonl` by hand in the same change. Bless writes atomically (temp file
   in the same directory, then rename).

## Adding fixtures

Edit the generator under `generator/`, then, from the repository root, regenerate
`fixtures.jsonl` and bless:

```bash
node tests/fixtures/js_alias_matrix/generator/export.cjs tests/fixtures/js_alias_matrix
PRISM_BLESS_JS_ALIAS_MATRIX=1 cargo test --test integration js_alias_matrix_test::matrix_matches
```

Regeneration is a dev-time step needing Node; it is not part of `cargo test`.
`export.cjs` writes `fixtures.jsonl` only; `expected.jsonl` always comes from the
bless step. Check the new rows against their `truth` before committing.

`generator/run.cjs` is an optional dev-time cross-check: it runs a prism binary
and the TypeScript checker over the fixtures and writes nothing into the repo.
