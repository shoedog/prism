# PR 340 — repair round 2 evidence

Candidate is uncommitted on `tooling/mutation-gate`, based on controller's
round-1 commit `7cd7db1bbd8b797ff00ef5999385ad8a7a75bbb2`. No git writes or
publication by the repairer. F was never opened. `src` remains unchanged.

| Run | Selected | Admissible / killed | Wall |
|---|---:|---:|---:|
| Pure text | 93 | 93 / 93 | 513.64 s |
| Schema gate | 93 | 93 / 93 | 90.00 s |
| Synthetic fn-scoped control | 3 | 3 / 3 | 33.50 s |

Zero ID-bound verdict/admissibility differences. In the full schema gate,
89 mutants ran as schemas, four demoted for struct-field edits, and zero kills
needed text confirmation. All 54 source selectors and 54 inactive-schema
selectors passed. Compilation needed no retry. The actual source diff is empty;
the nonempty scoped control injects changed lines at `src/js_paths_boundary.rs:169`
and performs real builds/tests. Both full runs share the worker-created Cargo
target directory; timings depend on host/cache state.

- [Per-ID certification](certification-table.md), [demotions](demotion-table.md),
  [comparison JSON](comparison.json), [hash provenance](provenance.json).
- [Schema receipt](schema/summary.json), [text receipt](text/summary.json),
  [scoped receipt](scoped/summary.json); corresponding `.log` files include
  `/usr/bin/time -p` wall measurements.
- [Python suite](unit-green-cycle2.log): 49 tests passed. The first-cycle four
  failures are retained in `unit-green.log`, with the bounded fixes documented
  in [hypotheses.md](hypotheses.md). Repair cap was two cycles; both completed.
- [Full all-features Cargo suite](cargo-suite.log): 5,145 passed, zero failed,
  one existing ignored test, 31 result blocks, 283.14 s. [Exact totals and ignored
  test](full-suite-totals.json). Command and pinned compiler hash are in provenance.
- [Matched HEAD/candidate fixtures](fixture-receipts.json): W1 constructed and
  hidden track-caller cases survive; unwrap value/panic and local-safe-macro
  genuine kills move from text to schema without confirmation; payload and
  should-panic location cases retain confirmation. `fixture_receipts.py` reruns
  these controls in dependency-free temporary crates and captures the output.
  `unit-prechange.log` contains an earlier broad control; missing candidate-only
  APIs/fields there are interface errors, not behavioral evidence.

Recheck the bound bytes and comparison with `python3 scripts/mutgate/evidence/repair-r2/compare.py`.
`scoped_control.py` reruns the nonempty synthetic control. Rebuilding after the
worker's target cleanup is required to rerun gate tests. The suggested controller
commit is [commit-message.txt](commit-message.txt); operational custody and resume
details are in [handoff.md](handoff.md).

Not verified: Tier-A (no production source changes), the existing ignored reserved
SliceElem test, arbitrary future lanes or external/aliased/dynamic location
observations beyond the documented lexical protections, or independent review.
