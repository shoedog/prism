# PR 340 — repair and certification, round 1

Repairer: Codex. Checkout `/Users/wesleyjinks/code/prism-mutation-gate`, branch
`tooling/mutation-gate`, unchanged HEAD `a524dcff72be13c10375633f773ee4fc2200591c`.
Base `origin/main`: `c50de85ad98527c6117b8acdc1edc078650fb103`.
This report supersedes the candidate status in the supplied `review-sol61.md`;
that review remains accurate historical evidence about the predecessor.
One repair/certification attempt completed within the declared two-attempt cap.
No commits, pushes, or other git writes were made.

## Finding dispositions

| Finding | Disposition | Evidence |
|---|---|---|
| W1 WRONG: source locations create false kills | FIXED. Both original and mutated bodies are checked. Direct observers, panic/unwrap/expect, track-caller attributes and unknown macro invocations demote to text. Every remaining schema kill requires a clean text kill. | Constructed `AFTER - f()` fixture is SURVIVED in both modes. Hidden track-caller fixture produces a schema failure and a text survivor, correctly reported SURVIVED. `regression-repaired.json`; compiled tests in `test_mutgate.py` |
| W2 WRONG: timeouts accepted as kills | FIXED. TIMEOUT is never a kill and fails the gate; there are no timeout exemptions in this driver. | A finite equivalent mutant reports TIMEOUT at 0.05 s and SURVIVED at 1 s in both modes, exit 1 throughout. `regression-repaired.json` |
| W3 WRONG: red text baseline counted as kill | FIXED. Both modes compile original source and preflight every selected selector before mutation; schema mode also preflights the inactive schema tree. Red selectors invalidate their mutants without removing the denominator. | Red baseline and mixed-good/red-baseline compiled fixtures; 54 source selectors green in each full mode and 54 inactive-schema selectors green |
| W4 WRONG: inadmissible selector coexists with gate success | FIXED. INADMISSIBLE outranks KILLED, aggregation requires every run admissible, and exit success requires every selected result KILLED. At least one admissible selector must kill; all need not kill. | Missing selector, selector removed after a green baseline, ignored selector, empty selector list and mixed kill/inadmissible cases all fail the gate |
| W5 WRONG: text fallback contains other schemas | FIXED. Source is resynchronized to original before text edits, and each mutant's own edits are restored in `finally`. Only Cargo artifacts are reused. | The constructed struct/initializer fallback kills in both modes. A failed text compilation does not contaminate the next mutant. Scratch source equals original after each run |
| W6 WRONG: R4-04 duplicates obsolete intent | REDEFINED, not retired. Historical ID retained; mutation now disables UTF-16 LE/BE BOM decoding. Registry records the r5 reason and fixture. | Existing native-decoding test exercises real LE/BE bytes, latin1, UTF-8/BOM, JSX/TSX and documentation-only negative cases. R4-04 is KILLED in both full modes; R4-11 remains the unreadable-file obligation |
| S7 SMELL: scoped selection is local | FIXED documentation; added `--scope file`, conservative rename diff handling, and missing-path/anchor selection outside the diff. Full lane remains the merge authority. | Scope unit tests and a nonempty three-mutant scoped control; no dependency-coverage claim |
| S8 SMELL: unusable target override | FIXED documentation and dormant code removed. Prefix routing is the supported lane format. | README and `test_target`; list semantics now match aggregation |

## Measured certification

| Run | Population | Baseline | Verdict/admissibility | Wall |
|---|---|---|---|---|
| Pure text | 93 | 54 original selectors green | 93 KILLED / 93 admissible | 724.59 s (12m 04.59s) |
| Schema, with mandatory text confirmation | 93; 27 schema candidates, 66 text demotions | 54 original + 54 inactive-schema selectors green | 93 KILLED / 93 admissible | 733.28 s (12m 13.28s) |
| Scoped control | I17, I73, R4-08 | 3 selectors green | 3 KILLED / 3 admissible | 37.54 s |

All 93 rows were compared by exact ID, verdict and admissibility: **zero
differences**. Both full runs meet 15 minutes; the scoped control meets five
minutes. It injects a changed line at `src/js_paths_boundary.rs:169` and selects
the three named mutants through fn scope and `--only`, without editing production.

The full run commands, driver/registry/test hashes, 272 production-file hashes,
base/HEAD and the pinned compiler SHA are in
[`provenance.json`](evidence/repair-r1/provenance.json).
The dependency artifacts were warm from the full Cargo suite; these are local
wall times, not a cold-cache or CI timing claim.

The row-wise table is [`certification-table.md`](evidence/repair-r1/certification-table.md).
Machine-readable comparison: [`comparison.json`](evidence/repair-r1/comparison.json).
Replay its population/hash/baseline/timing checks from the repository root:

```bash
python3 scripts/mutgate/evidence/repair-r1/compare.py
```

## Other verification

- `python3 -m unittest scripts/mutgate/test_mutgate.py`: **33 passed**.
  The same final tests against the committed predecessor produce 39 failing
  assertions/subtests and no probe errors. Original/repaired compiled-case
  receipts include actual summaries and test output, not just exit status.
- `cargo test --offline --all-features --no-fail-fast`, with the explicit pinned
  local TypeScript 5.9.3 compiler: **5145 passed, 0 failed, 1 existing ignored**.
  The ignored test is `resolution_test::slice_elem_variant_reserved`, reserved
  by its existing specification. Initial unconfigured run stopped at 21 library
  failures for missing `PRISM_TYPESCRIPT`; it is preserved as an environment
  setup failure and provides no evidence of a repair regression. The configured
  run passes every one of those tests.
- `cargo fmt --check` and `git diff --check`: passed.
- `git diff origin/main -- src`: empty. Tests, vendor, `Cargo.toml`, `Cargo.lock`
  and `build.rs` diffs are also empty. Gate and regression builds never edited
  production source.

## Controller files and proposed commits

1. `fix(mutgate): refuse false kills and isolate text mutants`
   - `scripts/mutgate/mutgate.py`
   - `scripts/mutgate/test_mutgate.py`
   - `scripts/mutgate/README.md`
2. `test(mutgate): redefine R4-04 around UTF-16 BOM decoding`
   - `mutants/lane-p-tsconfig-paths.json`
3. `docs(mutgate): record round-1 certification and handoff`
   - `scripts/mutgate/REPAIR-R1.md`
   - `scripts/mutgate/HANDOFF.md`
   - `VERIFICATION.md` (root verification-hook command ledger and coverage audit)
   - `scripts/mutgate/evidence/repair-r1/` (small logs, receipts, comparison and
     replay script; no build artifacts)

Git custody belongs to the controller. A separate final snapshot is saved
at `/private/tmp/mutgate-repair-r1-final.tar.gz` for handover.

## Not verified

CI/quiet-host or cold dependency-cache timing; repeated-run flakiness;
simultaneous gate invocations sharing a tree; arbitrary future Rust syntax and
macro eligibility; changes to live symlinked test/fixture inputs during a run.
The kill acceptance path is protected by clean text confirmation rather than
a claim that arbitrary duplicated Rust bodies preserve all semantics.
Tier-A was not run because production analysis source is unchanged.
This is a self-pass, not an independent repair review. No git publication or
merge was authorized or performed. The excluded frontend checkout was never
opened.

Cleanup completed after all verification processes exited: workspace `target`
was removed (4.5 GiB logical size before deletion). No generated build directories
remain; the small evidence files and snapshots are retained. See
[`cleanup.json`](evidence/repair-r1/cleanup.json). This is not a physical APFS
reclaim measurement.
