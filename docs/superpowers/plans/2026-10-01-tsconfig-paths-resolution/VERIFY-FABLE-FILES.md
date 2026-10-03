# Adopted performance fix verification — files and custody

Proposed controller commit message: `docs(paths): record adopted perf verification lint-gate failure`.

No Git writes or production changes. HEAD remains `755f85fe953118589aefb9089419b0144f0622f9`.

Changed files under `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/`:

- `MEASUREMENTS.md`: current failed gate, historical r4 distinction.
- `BUILD-MANIFEST.md`: revision/toolchain/source binding, pinned PRISM_TYPESCRIPT path, no current release certification.
- `HANDOFF.md`: points at current stopped verification handoff.
- `SPEC.md`: verification-status header only; normative owner policy unchanged.
- `VERIFY-FABLE-RESULTS.md`: every requested check, failed gate, same-environment control and exclusions.
- `VERIFY-FABLE-HANDOFF.md`: operational stop and exact remaining work, using installed handoff template.
- `VERIFY-FABLE-FILES.md`: this inventory and proposed commit message.

Lean local evidence remains under `target/verify-fable/`: fmt/current-clippy/r4-control/partial-default logs; diagnostic and partial-test summaries; hypothesis log; production binding; cleanup/custody receipts; local stop snapshot. The disposable `build/` and `base-control/` directories were removed. No new binary or controller-script change is included. Prior-round directories were retained.

The receipt archive is local custody only. Controller owns committing and any further verification or production repair.
