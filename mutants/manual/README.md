# Manual feature and Python mutations

The shared `scripts/mutgate/mutgate.py` gate reads only `mutants/*.json`, loads `src/**/*.rs`, and
builds without features. Catalogs here are intentionally outside that glob. Do not pass them to that runner.
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

S0 fix-round-1 receipts and the deadline-checked execution script are in
`~/prism-evidence/mcp-memory/optimization-round2/s0/` (`FIX-R1.md`, `manual-mutants.py`, `mutant-results.json`).
Check `date` before every Cargo command; start none at/after 2026-10-19 00:45 MDT for this round.
The authoritative full shared gate is a separate controller gate; these lane receipts certify only this
manual population. A shared `--plan-only` run confirms catalog exclusion, not mutant kills.
