# Final S1b-4 controller commit files — r2 review fold, 2026-10-01

READ: no Git writes by planner. Retain target/plan-s1b4 ignored receipts,
manifest, binaries, patches and checked snapshots before cleanup. No publication,
push or merge is authorized here. Plan922f00df; protobeec4a23 plus final fold.

## Plan tree

Recommended message: **docs: fold final S1b-4 spec review and r4 evidence**.
Stage these14 explicit paths; r2/r3 references and expected[] files unchanged:

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
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/S1b-controls-s1b4-r4-proto.txt`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/controls_gen.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/mutate_s1b4.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/prototype/s1b4-prototype-r4.diff.txt`

## Prototype tree

Recommended fold message: **fix(js-ts): preserve E5 namespace terminals and guard barrel escape**.
Five dirty fold paths relative to beec4a23:

- `src/ast.rs`
- `src/js_exports.rs`
- `src/resolution_js_namespace.rs`
- `tests/integration/js_binding_namespace_test.rs`
- `tests/integration/js_binding_namespace/spec_r2.rs`

READ: **the controller squashes `915fca43..298006b3` into one commit on
`proto/s1b-4-final`**. Recommended cumulative message:
**feat(js-ts): verify namespace exports while preserving E5 base rows**.
Fill that single cumulative starting SHA into IMPLEMENTOR and the adoption
field in BUILD-MANIFEST, then dispatch the implementer from it. The measured
beec4a23-dirty binary/source hashes remain their actual build identity.
The implementer starts from this body and completes implementation handback.
All18 cumulative owned files (including the new split test) are required:

- `eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/app.ts`
- `eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/expected.toml`
- `eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/util.ts`
- `src/ast.rs`
- `src/ast/js_binding.rs`
- `src/ast/js_binding_namespace.rs`
- `src/ast/js_binding_values_tests.rs`
- `src/call_graph.rs`
- `src/cpg_cache.rs`
- `src/js_exports.rs`
- `src/navigation/call_edge_cache.rs`
- `src/resolution.rs`
- `src/resolution_js_namespace.rs`
- `tests/integration/coverage_test.rs`
- `tests/integration/js_binding_namespace_test.rs`
- `tests/integration/main.rs`
- `tests/integration/module_binding_audit_test.rs`
- `tests/integration/js_binding_namespace/spec_r2.rs`

Never adopt docs/eval reports, eval/snapshots or generated target evidence.
Controller runs private F via CONTROLLER-S1b4.sh; no historical F receipt
certifies r4. Refresh HANDOFF with actual custody SHAs after Git writes.
