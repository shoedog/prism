# Controller commit groups

All changes are uncommitted. No S2 scratch file belongs in these commits.

## Source, tests and behavioral registry

Proposed message: `feat(resolution): prototype workspace package entry proofs`

- src/js_packages.rs — new immutable native package-entry proof and scheme classification.
- src/js_paths.rs — paths-first package rung and P2 hop integration.
- src/js_paths_snapshot.rs — retained canonical package-root lookup.
- src/lib.rs — module registration.
- src/cpg_cache.rs and src/navigation/call_edge_cache.rs — epochs 107/63 and pinned assertions.
- tests/integration/js_packages_test.rs and tests/integration/main.rs — eight package regressions/registration.
- mutants/lane-pkg-resolution.json — eight mutation witnesses; scoped execution limitation recorded.

## Planning and measurement packet

Proposed message: `docs(plan): record workspace package census and S2 unblock measurements`

The root `VERIFICATION.md` and complete `docs/superpowers/plans/2026-10-04-workspace-package-resolution/` packet: CENSUS, SPEC, IMPLEMENTOR, MEASUREMENTS, PROBES, PROBE-LOG, OQ, CONTROLLER-pkg.sh, BUILD-MANIFEST.md/json, VERIFICATION, FILES, HANDOFF and public-only probe sources. Exclude __pycache__ and generated build/receipt directories. Raw public receipts and pinned binaries remain in `/Users/wesleyjinks/prism-evidence/pkgres/planning`, bound by manifest/custody, rather than checking corpus bytes into the code repository.

These are proposed prototype/planning commits, not an adoption recommendation. Controller evaluates F and independent review before adopting any refusal cut or accepted model/cost change.
