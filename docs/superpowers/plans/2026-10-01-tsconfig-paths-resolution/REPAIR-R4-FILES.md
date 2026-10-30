# Lane-P P1 r4 owned files and controller custody

No Git writes were performed. Entry HEAD remains `e8da66f92ea48356cde2cf464dc729e7fb796e39`; this repair is an uncommitted delta on the existing r3 artifact. The local source snapshot and final receipt binding live in `target/repair-r4/`. See REPAIR-R4-RESULTS.md for current gates and REPAIR-R4-HANDOFF.md for operational state.

Suggested controller commit messages:

1. `fix(paths): tolerate repository ambient scan under accepted-risk policy`
2. `perf(paths): bound alias projections and stream site dumps`

The first covers the tolerant scan, removed ambient-boundary closure, retained explicit-type handling, both-grammar fixtures and accepted-cost documentation. The second covers the export resolver's root subset, lazy CLI JSONL output and their regressions. Documentation and controller pointer should accompany the final cumulative state. These are proposed messages, not existing commits.

Owned existing paths:

- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/BUILD-MANIFEST.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/MEASUREMENTS.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh`
- `src/call_graph.rs`
- `src/js_exports.rs`
- `src/js_paths.rs`
- `src/js_paths_boundary.rs`
- `src/js_paths_snapshot.rs`
- `src/main.rs`
- `src/navigation/queries.rs`
- `src/repo_loader.rs`
- `tests/integration/js_paths_cap_test.rs`
- `tests/integration/js_paths_r2b_test.rs`
- `tests/integration/js_paths_r2c_test.rs`
- `tests/integration/js_paths_r3_test.rs`
- `tests/integration/js_paths_repair_test.rs`

New documents:

- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R4-HANDOFF.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R4-RESULTS.md`
- `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R4-FILES.md`
