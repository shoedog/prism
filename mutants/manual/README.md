# Manual feature and Python mutations

The shared `scripts/mutgate/mutgate.py` gate reads only `mutants/*.json`, builds without features and has
no Python test runner. Catalogs here are intentionally outside that glob. Do not pass them to that runner.
The shared runner and its ordinary catalogs remain unchanged.

`memreport-changed-refresh.json` records `[file, exact original text, mutant text, one test selector]`.
For each entry, on the final candidate:

1. Read the file, require exactly one original anchor, save its current bytes and SHA-256 externally.
2. Replace only that anchor with the mutant text. Never reset source to a parent revision.
3. For Rust, run `nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features memreport-changed-refresh --lib <selector> -- --exact`.
   For Python, from `scripts/memreport`, run `python3 -m unittest <selector>`.
4. Require exactly one executed test and a behavioral failure exposing the intended missing behavior.
   Compile errors, missing anchors, zero tests and sandbox refusal are INADMISSIBLE, not kills.
5. In a `finally` block restore the saved current bytes, verify the original SHA-256, and run the same
   selector to GREEN. Stop on any restoration mismatch or survivor; do not continue with a mutant applied.
6. Save `mutant-<id>-killed.log` and `mutant-<id>-restored.log` plus the restore/hash checklist.

S0 fix-round-1 receipts are kept in the lane's private evidence (`FIX-R1.md`, `mutant-results.json`); these
README steps are the authority for re-running the population. An import-time error (`_FailedTest`,
`SyntaxError`, `ImportError`) is INADMISSIBLE even though unittest reports it as one failed test.
The authoritative full shared gate is a separate controller gate; these lane receipts certify only this
manual population. A shared `--plan-only` run confirms catalog exclusion, not mutant kills.

## `idle-unload-i0a.json` (I0a idle unload, `--features mcp`)

Records are `{old, new}` edit lists with one exact selector each. Follow the same steps with
`nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib SELECTOR -- --exact`, applying every listed
edit once. The first four records are the plan's required mutants; later records add negative witnesses. These
are fault injections, not claims that the parent supported the new lifecycle API. I0a receipts are kept in the
lane's private evidence.

## `i0-freshness.json` (F source-bound freshness, `--features mcp`)

Apply each record's exact edits to the current candidate and run its single selector with
`nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib SELECTOR -- --exact`.
Use steps 1–6 above: require one behavioral failure, restore saved bytes in `finally`,
verify SHA-256, then require one restored GREEN. Compiler errors and zero selections are
inadmissible. The eight mutants cover late stamping, lost divergence, wrong comparison,
loader input replay, final service recheck, repeated uncertifiable auto refresh and
permission-only recovery of suppressed auto refresh, and lost suppression after a Diverged commit.
Save receipts outside the checkout; the full shared-catalog gate remains separate.

## `i0b-1.json` (daemon idle lifecycle, `--features mcp`)

Apply each record to the current candidate using steps 1–6 above and its one exact selector:
`nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib SELECTOR -- --exact`.
The 16 distinct selectors cover serving admission, both response boundaries, whole-session and
uncollected-outcome release, re-arm, worker destruction, publication time, history, failure
supersession, fatal disposal/poison, a mutant-only retained-provider reload that skips bootstrap,
idle-off drain answers, successful build publication/settlement during drain, and exponential
spawn-failure backoff.
Keep before/mutant/restored SHA-256 receipts and behavioral RED/restored GREEN outside the repo.
Do not count compiler failures, zero selections, aborts without a test verdict, or host refusals as kills.
