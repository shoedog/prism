# S1b-4 controller commit files — 2026-10-01

READ: controller only performs Git writes. The planner executed none. Copy
`target/plan-s1b4/` evidence/snapshots to durable custody before cleanup.
No push, merge, history rewrite or implementation adoption is authorized here.

## Plan tree: pending packet changes

MEASURED: `/Users/wesleyjinks/code/prism-s1b-4-plan`, branch `plan/s1b-4`,
HEAD `bd93e7e801b9b5a06801acc11a27cf9bbb77f88d` (predecessor draft53eb3155).
Commit message: **docs: finalize measured S1b-4 namespace dispatch and controls**.
Stage only these explicit paths relative to that tree:

- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/COMMIT-FILES-s1b4.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/CONTROLLER-S1b4.sh`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/HANDOFF-s1b4.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/IMPLEMENTOR.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/OQ-S1b4.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/REVIEWER.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/S1b-4-CONTROLS.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/S1b-4-MEASUREMENTS.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/SPEC.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/S1b-4-EXPECTATIONS.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/S1b-controls-s1b4-r2-proto.txt`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/S1b-replay-s1b4-r2-proto.txt`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/compare_controls_s1b4.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/expected/S1b-4-r2-R.json`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/expected/S1b-4-r2-T.json`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/expected/S1b-4-r2-X.json`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/export_counters_s1b4.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/mutate_s1b4.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/run_quick_s1b4.py`

READ: controls_gen.py, audit_s1b4.py and valueflow_guard_s1b4.py were already in
the committed draft; there is no new diff for them. S1b-4-EXPECTATIONS.md retains
the preregistration and appends the measured disposition. Keep the old S1b-4
expected files unchanged.

## Prototype tree: completed source body already captured by controller

MEASURED: `/Users/wesleyjinks/code/prism-s1b-4-plan/target/plan-s1b4/proto`,
branch `proto/s1b-4`, HEAD `39faa3aac79c71ca4b41423caacf9cead50da86d`,
parent/source base `915fca43d84ea1730959453091fbf8ae97763af8`. All 17 owned files
below are already committed and clean; no additional source commit is needed.
Recommended final source message when the controller freezes the prototype:
**fix(js-ts): bind namespace R3 to proven imports and export identities**.
The current message is `wip(proto): S1b-4 prototype snapshot (planner in progress)`;
retain its true SHA in provenance. This is not an instruction to rewrite history.
Source ownership/file list, relative to the prototype tree:

- `src/ast.rs`
- `src/ast/js_binding.rs`
- `src/ast/js_binding_values_tests.rs`
- `src/call_graph.rs`
- `src/cpg_cache.rs`
- `src/js_exports.rs`
- `src/navigation/call_edge_cache.rs`
- `src/resolution.rs`
- `tests/integration/coverage_test.rs`
- `tests/integration/main.rs`
- `tests/integration/module_binding_audit_test.rs`
- `src/ast/js_binding_namespace.rs`
- `src/resolution_js_namespace.rs`
- `tests/integration/js_binding_namespace_test.rs`
- `eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/app.ts`
- `eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/util.ts`
- `eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/expected.toml`

READ: the WIP also captured docs/eval/tier-a/2026-10-01-prism.{json,md} and
an eval/snapshots/prism-915fca43d84e.json generated inventory. They are outside
the 17 implementation-owned files and excluded from code LOC. No cleanup or
rewrite was performed; newer quick artifacts are retained in evidence rather
than committed as acceptance baselines.

MEASURED: BUILD-MANIFEST.json and proto-tracked.diff bind every owned byte to
915fca43→39faa3aa. The public measured binary has 915fca43-dirty identity because
it predates the controller commit; a separate clean39faa3aa binary verifies
quick and warm sidecar behavior. Both hashes are recorded, not conflated.
After the packet commit, refresh HANDOFF with the real packet SHA; run private
F and append only aggregates/custody hashes before review round 1 of 2.
