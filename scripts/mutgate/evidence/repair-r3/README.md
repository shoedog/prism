# PR 340 — repair round 3 certification

Uncommitted candidate on `tooling/mutation-gate`, based on
`781511dc64c12eee81af1fa79bedce646d4fcdd2`. No git writes or publication by
repairer. F was never opened. `git diff origin/main -- src` is empty.

Default/full authority uses isolated text builds with green original-source
preflight in the shared and worker contexts. Scoped schema mode is explicitly
ADVISORY; `--authoritative` makes the selected scope use text. All 56 Python
tests pass ([unit-cycle3.log](unit-cycle3.log)), including six retained compiled
counterexamples: default/scoped authoritative SURVIVED (exit 1), advisory schema
KILLED (exit 0, `authoritative: false`). Existing regressions remain.

| Run | Selected | Result | Wall | Peak target disk GiB | Peak process RSS GiB |
|---|---:|---|---:|---:|---:|
| jobs1 | 93 | 93/93 admissible KILLED | 471.46 s | 3.32 | 1.94 |
| jobs2 | 93 | 93/93 admissible KILLED | 277.06 s | 5.32 | 1.79 |
| jobs4 | 93 | 93/93 admissible KILLED | 202.87 s | 9.35 | 1.54 |
| scoped | 3 | 3/3 ADVISORY KILLED | 37.20 s | 3.32 | 1.71 |
| scoped-empty | 0 | no mutation coverage | 2.27 s | 5.28 | 0.05 |
| suite | 5146 | 5145 passed / 0 failed / 1 ignored | 213.71 s | 5.28 | 1.70 |

All three full gates match the 93-ID r2 pure-text receipt, with zero verdict or
admissibility differences, no schema verdicts, 54 green shared source selectors,
and green private-worker selectors. All worker trees/targets were removed.
Four-worker wall time **202.87 s** meets the five-minute warm lane-P bound.
Seeding, private baseline checks, mutation builds/tests and cleanup are included.
Build caches were warm; timing runs were serial and used the pinned local
TypeScript compiler. The scoped 37.20 s result uses injected changed lines at
`src/js_paths_boundary.rs:169` (real builds/tests, three IDs); the actual diff
selects zero, measured separately at 2.27 s, providing no mutation coverage.

Gate peak allocated target blocks: **10,035,216,384 bytes / 9.35 GiB**. Gate peak
child-process RSS: **2,087,731,200 bytes / 1.94 GiB**. Disk is sampled `du -sk`,
which can count shared APFS clone extents repeatedly. A few samples raced Cargo
cache renames and were skipped; receipt errors are retained. Unique physical
storage and exact instantaneous disk maxima are unverified. RSS comes from
macOS `getrusage(RUSAGE_CHILDREN)`, including nested waited children, validated
by [rss-capability.json](rss-capability.json); aggregate concurrent RSS is not
measured. Sandbox blocks process listing and time(1)'s -l sysctl. Timing uses
`/usr/bin/time -p`.

Full project command: `cargo test --offline --all-features --no-fail-fast`,
from the normal checkout manifest. **5145 passed, 0 failed, 1 ignored**, 31 result
blocks, 213.71 s ([suite.log](suite.log), [suite-totals.json](suite-totals.json)).
The existing ignored test is `resolution_test::slice_elem_variant_reserved`.
An initial scratch-manifest attempt produced 1469 passes/20 worker_schema
failures; byte-identical r2 Rust inputs reproduced all 20 in the same scratch
context ([scratch-suite-control.json](scratch-suite-control.json)). The script's
entrypoint compares invocation and canonical module paths; the scratch scripts
symlink suppresses main/output. All 20 pass from the normal checkout. Those
attempts are preserved in scratch-suite/ and scratch-base-control.log; no
production change or re-baseline was made.

[comparison.json](comparison.json) and [certification-table.md](certification-table.md)
show ID-bound agreement. [certification-inputs.json](certification-inputs.json)
binds 761 inputs and the compiler; comparison also rechecks the unchanged 747
r2 production/test/build inputs and lane registry. Replay without building:
`python3 scripts/mutgate/evidence/repair-r3/compare.py`.

Cap: two cycles, one disclosed bounded third cycle to fix a concrete worker
relocation false kill. Shared-only baseline passed an equivalent manifest-path
observer while worker context failed. Matched r2 control survived; fixed driver
preflights original source per worker and refuses both worker-only compile/test
failures. [Probe](worker-baseline-probe.json), [pre-fix regression](unit-worker-preflight-prechange.log),
[hypotheses](hypotheses.md). Earlier timings in before-worker-preflight/ are
superseded history. Original r2 false kills and paired text controls are retained
in unit-prechange.log and text-control.json. Invalid path/scope/capability probes
are explicitly excluded from behavioral evidence. No artifact restart or
lexical spelling-list extension.

No cold-cache/CI timing, repeated flakiness campaign, independent review, aggregate
RSS, unique APFS physical storage, or ignored reserved test execution. Tier-A
was not triggered by tooling-only edits. Build cleanup is recorded in
[cleanup.json](cleanup.json); final snapshot is
`/private/tmp/mutgate-repair-r3-final.tar.gz`. Controller commit remains pending
per "No git writes"; suggested message is in commit-message.txt.
