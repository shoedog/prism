# JS/TS alias variant matrix

Enumerated tiny repos (one per line of `fixtures.jsonl`), each with one call site
marked `// SITE` and an authored ground truth for what that site statically calls. The test
(`tests/integration/js_alias_matrix_test.rs`) runs prism on every fixture and
compares a derived row with the pinned row in `expected.jsonl`. It pins what
prism does today, so a product slice shows up as a diff of `expected.jsonl`,
row by row.

## Files

- `fixtures.jsonl`: one fixture per line: `id`, `step`, `shape`, `scope`, `form`,
  `naming`, `lang`, `contract`, `site_file`, `site_line` (1-indexed), `truth`,
  `files` (path to source).
- `expected.jsonl`: one row per fixture, same order: `id`, `verdict`, `record`,
  `exact`, `nameonly`, `drop`, `origins`.
- `accepted_changes.jsonl`: regressions a bless run may accept (see below).
  One `{"id": ..., "reason": ...}` per line. Empty by default.
- `generator/`: dev-time Node scripts that produced the two files above.

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

## Verdicts

A target matches when it is in the same file and has the same name or the same
declaration line.

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
`(step, expected verdict -> actual verdict)`. Nothing here needs Node.

## Updating after a slice

1. Run with the bless variable, then review the `expected.jsonl` diff:

   ```bash
   PRISM_BLESS_JS_ALIAS_MATRIX=1 cargo test --test integration js_alias_matrix_test::matrix_matches
   git diff tests/fixtures/js_alias_matrix/expected.jsonl
   ```

   Every changed row is a behavior change the PR must explain.
2. Bless refuses to write anything if a row would regress: its verdict moves into
   `WRONG` or `OVERCLAIM`, or out of `CORRECT`. A new fixture with no pinned row
   counts only if it lands in `WRONG` or `OVERCLAIM`. To accept a regression, add
   `{"id": "<fixture id>", "reason": "<why>"}` to `accepted_changes.jsonl` and
   rerun. Accepting one id excuses only that row.
3. The normal run fails if `accepted_changes.jsonl` names an id that is not a fixture.

## Adding fixtures

Edit `generator/variants.cjs`, then regenerate. This is a dev-time step, not part
of `cargo test`. It needs Node, a TypeScript checker (`typescript.js`, used by
`run.cjs` as a cross-check of the authored truth) and a built prism binary:

```bash
node generator/run.cjs <typescript.js> <out-dir> main=<path to prism binary>
node generator/export.cjs <out-dir>/matrix.jsonl <dest-dir> main
```

`run.cjs` writes `matrix.jsonl`; `export.cjs` takes that file, an output
directory and the binary tag (`main` above) and writes `fixtures.jsonl` and
`expected.jsonl`. Copy both here. The rows of any new fixture are pinned from
the binary that produced them, so check them against their `truth` before
committing.
