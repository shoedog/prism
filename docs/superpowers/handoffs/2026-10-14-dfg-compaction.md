# Handoff — DFG compaction

## PR E — activation and merge of main

PR E (#439) activated compact DFG storage and incorporated main's module-owner
format change. CPG cache version is 122; the navigation sidecar remains at 67.
Task 7 memory/load/peak acceptance remains bound to activation `3739ab38`;
see [the measured readout](../../eval/dfg-compaction/acceptance.md).

The pre-PR-F handoff records the merge of main `b2cd10a1`: all 31 main-only paths
were retained, with the two authorized cache-version replacements; accounting and
census sources matched both parents. The main and branch captures each contained
133 cases (30 accepted #442 changes, 103 unchanged). Historical verification
reported 6,249 MCP passes, 3 ignored and 109 host-facility exclusions; oracle
umbrella 168 passes and 1 ignored; CI parity 161 passes and 1 ignored; cache unit
76 passes; formatting passed. A cache 122-to-121 control produced four expected
version-assertion failures, followed by 76 passes after restoration. These are
historical receipts, not new measurements.

The original E handoff is preserved in
`~/prism-evidence/dfg-compaction/prF-2026-10-17/before-dfg-handoff.md`;
its raw merge receipts are in the sibling `dfgE-merge-b2cd10a1/` directory named
there. PR F evidence also preserves the acceptance source/probe/binary hashes and
corpus/workload provenance under `equivalence/acceptance-*.json`. Historical
field-wise payload and ordered-index captures must not be replaced by native
codec bytes or relabeled as new acceptance measurements.

## PR F

PR F (#451) retired the temporary migration oracle, frozen legacy engine,
payload/lookup pins, oracle Cargo feature/target and oracle CI/path-filter
machinery. Authored representative/order expectations, native cache round-trips,
renderer/dispatch controls and all 15 lane mutant witnesses remain in ordinary
test targets; production behavior, storage layout and cache versions are unchanged.
The final oracle receipt is
`~/prism-evidence/dfg-compaction/prF-2026-10-17/step0/RESULT.md`
(168 passes, 1 ignored; explicit capture 1 pass; both 133-case manifests reproduced).
The evidence root `~/prism-evidence/dfg-compaction/prF-2026-10-17/` contains
`selector-table.md`, `full-mutgate/summary.json`, production-equivalence receipts
and `fold-r1/` review-fold logs. CI's nextest `dispatch_large` override allows
480 seconds (eight 60-second periods) for that exact output test under coverage.
The lane has no remaining implementation work beyond follow-ups: maintain the
ordinary regressions and retained mutation witnesses, and keep historical
`dfg_oracle` receipt validation in acceptance tooling until those receipt formats
are deliberately retired. Real-fixture payload/query drift is no longer pinned.
