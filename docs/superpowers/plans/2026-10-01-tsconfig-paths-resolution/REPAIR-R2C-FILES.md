# Lane-P P1 r2c controller commit groups

No Git writes performed. Starting HEAD: `153fcbb3653d`; base: `dab8251c6013b28b0db4cb4042f36db444eb556c`. Cache stays **105/61**. Messages below are proposals for the controller after review.

1. **`fix(paths): admit absent type inputs and scan installed declarations`**

   `src/js_paths.rs`, `src/js_paths_snapshot.rs`; `tests/integration/{js_paths_r2c_test.rs,js_paths_r2b_test.rs,js_paths_cap_test.rs,main.rs}`.

   Distinguish scanned in-root, absent and unsafe type inputs. Preserve wildcard ambient refusals and whole-repo scanner-skip declines. Follow package declaration redirects and transitive references, including Node10 secondary lookup after custom roots miss. Keep absence/warning counters deduplicated and invalidate old P1 topology at unchanged cache versions.

2. **`test(paths): bind r2c absence, ambient and cache witnesses`**

   `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/{repair_r2c.py,oracle.cjs,mutants.py,integration_mutants.py,CONTROLLER-paths.sh}`.

   Native TS absence semantics, both-grammar cache/install/edit/remove/redirect witnesses and four requested case mutants, plus secondary/inherited coverage. Old I25's file-only path rule is superseded by the owner-authorized file-or-directory rule. Old I26's missing-input witness is superseded; the inherited recheck remains and M53 tests its outside-redirect obligation.

3. **`docs(paths): record r2c X recovery and final verification`**

   `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/{SPEC.md,IMPLEMENTOR.md,REVIEWER.md,HANDOFF.md,MEASUREMENTS.md,BUILD-MANIFEST.md,REPAIR-R2C-FILES.md}`.

   Bind final source/executable and requested measurements without changing baselines, thresholds, cache versions or parked S6/OQ2. The controller can combine these into one atomic commit. The controller wrapper defaults to the immutable r2c executable and was syntax-checked only; this worker never opens F or executes the wrapper. Local evidence is under `target/repair-r2c`.

Root `VERIFICATION.md` is ignored by repository policy and retained in the local custody archive, outside the proposed Git commit groups.
