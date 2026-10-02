# Controller commit files — Lane-P P1 repair round 1

Parent `7f5a862afd002dd5953108a3ab5b611d8745bde9` on `feat/tsconfig-paths-p1`. No Git writes were made by the repairer. Suggested commit groups follow; the final snapshot/list under target/repair-r1 is authoritative for bytes. Controller commits, rebuilds/binds the new commit and measures private F with CONTROLLER-paths.sh.

1. `fix(paths): prove alias export hops and refuse Node10 ambiguity`

- `src/call_graph.rs`
- `src/cpg_cache.rs`
- `src/js_paths.rs`
- `src/js_paths_snapshot.rs`
- `src/navigation/call_edge_cache.rs`
- `src/repo_loader.rs`
- `src/resolution.rs`
- `tests/integration/js_paths_common.rs`
- `tests/integration/js_paths_r1_test.rs`
- `tests/integration/js_paths_repair_test.rs`
- `tests/integration/js_paths_test.rs`
- `tests/integration/main.rs`

2. `test(paths): cover repair cuts cache occupancy and mutation witnesses`

- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/controls_gen.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/integration_mutants.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/mutants.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/repair_budget_probe.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/repair_cache_probe.py`

3. `docs(paths): bind repair round one measurements and handoff`

- `VERIFICATION.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/BUILD-MANIFEST.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/IMPLEMENTOR.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/MEASUREMENTS.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REVIEWER.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R1-FILES.md`

Evidence: `target/repair-r1/`. Ignore-path evidence is local; controller must preserve it externally with the reviewed commit. F was never opened and has no repair receipt here.
