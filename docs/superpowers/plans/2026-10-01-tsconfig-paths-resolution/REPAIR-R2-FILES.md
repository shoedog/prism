# Repair round 2 files and proposed commits

No Git writes were made. These are proposed controller commit groups on fd297691; the local source/evidence snapshot is under target/repair-r2. Do not replay the round-1 patch.

## fix(paths): enforce structural JS and declaration fences

- `src/js_paths.rs`
- `src/js_paths_snapshot.rs`

## test(paths): cover round-two cuts and dependency parity

- `tests/integration/js_paths_cap_test.rs`
- `tests/integration/fixtures/js_paths_cap.json`
- `tests/integration/js_paths_common.rs`
- `tests/integration/js_paths_test.rs`
- `tests/integration/js_paths_r1_test.rs`
- `tests/integration/js_paths_repair_test.rs`
- `tests/integration/main.rs`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/mutants.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/integration_mutants.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/resolver_driver.rs`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/repair_cache_probe.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/repair_r2_cache_probe.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/verify_controls.py`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/oracle.cjs`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/final_receipts.py`

## docs(paths): bind round-two costs and controller handoff

- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/IMPLEMENTOR.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REVIEWER.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/MEASUREMENTS.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/BUILD-MANIFEST.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R2-FILES.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh`
- `VERIFICATION.md`

The first group changes the resolver and bounded scan. The second adds source-bound reviewer fixtures, structural positive/negative witnesses, dependency parity and mutation/oracle explanations. The third reconciles current receipts and points the private controller wrapper at the immutable final binary.

The wrapper was syntax-checked only. F remains a controller task.

Root VERIFICATION.md is Git-ignored. It is explicitly included in the owned archive/hash list; the tracked diff alone does not carry it or the new untracked fixtures/probe. Preserve the archive alongside the patch.
