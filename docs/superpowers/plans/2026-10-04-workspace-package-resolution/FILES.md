# Controller commit groups — R1 patches

No Git writes were performed. Source patch is relative to committed prototype 92c1d0bc; docs patch is relative to packet c89bc5b7. Neither adopts S2.

Proposed source commit: `fix(resolution): repair workspace package semantics and preserve native absence status`

- src/js_packages.rs: exact/legacy extension tables, pattern-only star replacement, per-file modes, null/outer continuation, self-name and three-way result, JSX/root-typesVersions barriers.
- src/js_paths.rs: status-preserving package integration, colon paths precedence and Node ESM export hops.
- src/js_paths_snapshot.rs: retained outside-root package scope and cache topology.
- src/cpg_cache.rs and src/navigation/call_edge_cache.rs: epochs 108/64.
- tests/integration/js_packages_test.rs: R1 regressions/edge controls and corrected unresolved-target assertion.
- mutants/lane-pkg-resolution.json: 26 rule/classification witnesses with explicit intent revisions.

Proposed packet commit: `docs(resolution): bind R1 repairs to the TS 5.9.3 differential acceptance gate`

The full packet directory: current SPEC, IMPLEMENTOR, OQ, R1-REPORT, MEASUREMENTS, BUILD-MANIFEST, VERIFICATION, HANDOFF, FILES, controller script and probes, including the differential generator/oracle/runner, retained reviewer case descriptions, resolution helper and persisted-cache test. Earlier planning documents remain explicitly dated history where retained. Generated fixtures, caches, binaries and receipts stay under prism-evidence/pkgres/repair-r1, not in the repository.

Patches: `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/R1-src.patch` and R1-docs.patch. Controller must rebind source/probe inputs before any private F run; a local patch or green gate is not independent acceptance or S2 adoption.
