# Adopted performance verification — files and custody

Proposed controller commit message: `test(paths): record adopted perf verification and refresh mutant drivers`.

No worker Git writes. Production, tests, vendor and build inputs are unchanged from HEAD `b28f6e721552c946fab8a41039e7b30552fc7f41`. Verification-only changes under `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/`:

- `MEASUREMENTS.md`
- `BUILD-MANIFEST.md`
- `HANDOFF.md`
- `SPEC.md`
- `VERIFY-FABLE-RESULTS.md`
- `VERIFY-FABLE-HANDOFF.md`
- `VERIFY-FABLE-FILES.md`
- `probes/CONTROLLER-paths.sh`
- `probes/mutants.py`
- `probes/integration_mutants.py`
- `probes/adopted-mutants.json`

MEASUREMENTS/BUILD-MANIFEST bind current totals, medians, pinned release/oracle and limitations. HANDOFF/SPEC status headers reconcile the superseded lint stop; normative owner policy is unchanged. RESULTS and the current handoff carry every check and disposition. Kernel/integration drivers and adopted-mutants.json rebind only moved injections, preserve 67/93 active denominators, and retain old policy retirements. CONTROLLER-paths.sh defaults to current/head/prism and is syntax-checked; no F execution.

Lean logs, source/driver/binary/fixture hashes, classifications, summaries, final snapshot and pinned head binary remain under `target/verify-fable/current/`. Disposable build, extracted/mutated source copies and restored fixtures are enumerated in cleanup-summary.json. The binary remains available to the controller after build cleanup. Snapshots are local custody; no pushed/external backup claim. Controller owns committing.
