# Lane-P P1 r2b controller commit groups

No Git writes were performed. Starting HEAD is `d474714eb487765add12db7133fa1e41e5ba9f90`; base is `dab8251c6013b28b0db4cb4042f36db444eb556c`. These are proposed messages for the controller after review, not existing commits. Cache remains **105/61**. The final executable is `target/repair-r2b/head/prism-r2b-final`; bind its source using BUILD-MANIFEST.md and `target/repair-r2b/source-binary-binding.json`.

1. **`fix(paths): bound type-input declines to scanned reachability`**

   `src/js_paths.rs`, `src/js_paths_snapshot.rs`, `tests/integration/js_paths_cap_test.rs`, `tests/integration/js_paths_r2b_test.rs`, `tests/integration/main.rs`.

   Replace cut 3's presence test with covered input resolution. Preserve extends origins, child type-root overrides, file-only path references, scoped/custom types and cyclic reference closure. Check declaration redirects in Node10 package metadata; opaque metadata remains unresolved. Covered inputs admit Exact unless the existing global ambient cut matches. Outside, missing or unread inputs preserve base. The snapshot's test-only probe accessor supports the following performance regression.

2. **`perf(paths): prime only reachable alias export closures`**

   `src/repo_loader.rs`.

   Prime every import proof/refusal dependency, then extract exports only for admitted alias targets and their relative export/import-forward closure. The library regression proves required two-hop and missing probes are retained while an unrelated export is not probed. Restoring the HEAD loader fails that assertion. Kernel and integration mutation receipts cover missing and excessive priming.

3. **`test(paths): bind r2b reachability and measurement receipts`**

   `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/{SPEC.md,IMPLEMENTOR.md,REVIEWER.md,HANDOFF.md,MEASUREMENTS.md,BUILD-MANIFEST.md,REPAIR-R2B-FILES.md}` and `probes/{oracle.cjs,mutants.py,integration_mutants.py,repair_r2b.py,CONTROLLER-paths.sh}` in that directory.

   Update the independent diagnostic refusal reasons, finite both-grammar witnesses, same-version cache probe and per-arm mutants. Point the controller-only wrapper at the final immutable executable; it was syntax-checked only. Record the frozen X dependency limitation and serial performance result without changing thresholds or baselines.

The controller can combine these groups into one atomic repair commit. Root `VERIFICATION.md` is ignored by repository policy and is included in the local custody archive. Evidence under `target/repair-r2b` is local and untracked; archive/hash receipts preserve the new test and probe bytes. No private F, network, cache bump, baseline edit, independent approval or external backup is claimed.
