# S1b-4 controller commit files — spec-review r1 fold, 2026-10-01

READ: controller performs all Git writes. Planner performed none. Keep ignored
r3 evidence and snapshots under target/plan-s1b4 in durable custody before
cleanup. No publication, push or merge is performed by this handback.

## Plan tree

MEASURED: /Users/wesleyjinks/code/prism-s1b-4-plan, plan/s1b-4 @
564ebc8c59b1df8c6a2541d424b70f883f619642 plus the packet fold (PR335).
Recommended commit: **docs: fold S1b-4 export lookup spec review and r3 evidence**.
Stage only the following explicit paths; r2 reference files remain unchanged.

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
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/S1b-controls-s1b4-r3-proto.txt`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/S1b-replay-s1b4-r3-proto.txt`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/controls_gen.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/expected/S1b-4-r3-R.json`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/expected/S1b-4-r3-T.json`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/expected/S1b-4-r3-X.json`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/mutate_s1b4.py`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/prototype/s1b4-prototype-r3.diff.txt`

## Prototype tree

MEASURED: target/plan-s1b4/proto, proto/s1b-4 @
fceb0b4e75a8a2543acc3df70bb6c23ae6e5960e plus this owned fold; source base
915fca43d84ea1730959453091fbf8ae97763af8. Recommended fold/final message:
**fix(js-ts): preserve uncertain namespace exports and remove proven decoys**.
The eight currently dirty files relative to fceb0b4e are:

- `src/ast.rs`
- `src/ast/js_binding_namespace.rs`
- `src/cpg_cache.rs`
- `src/js_exports.rs`
- `src/navigation/call_edge_cache.rs`
- `src/resolution.rs`
- `src/resolution_js_namespace.rs`
- `tests/integration/js_binding_namespace_test.rs`

READ: dispatch adoption is the **cumulative prototype body**, not an incremental
fold-only commit. Controller freezes a final cumulative proto commit containing
all17 owned files below, based off main, and cherry-picks that final commit onto
a fresh implementer branch off main. If an incremental fold commit is retained
in custody, it must not be cherry-picked alone: the original prototype's nine
other owned files are required. This planner does not prescribe a Git history
rewrite or execute the controller's Git steps.

The Sonnet implementer **starts from the prototype's owned files**, completes
tests, source-bound RED evidence, verification and handback, and does not
re-implement the body. Final cumulative ownership is:

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

READ: fceb0b4e already excludes the stray Tier-A reports and generated snapshot.
Never stage docs/eval reports, eval/snapshots or target artifacts as implementation.
New quick inventories were retained under head/quick-r3{,-retry}, not in proto.
Only the existing Tier-A fixture remains among owned eval paths.

MEASURED: BUILD-MANIFEST.json, proto-r3.diff, proto-fold-r1.diff,
prototype/s1b4-prototype-r3.diff.txt and size-r3.json bind the actual dirty body
and r3 binary; BUILD-MANIFEST-r2.json retains the old identity. No commit-only
change is described as source behavior. Controller reruns F via the updated
script after retaining evidence; private rows and source stay private.
After Git custody, refresh the handoff with the real plan/final prototype SHAs.
