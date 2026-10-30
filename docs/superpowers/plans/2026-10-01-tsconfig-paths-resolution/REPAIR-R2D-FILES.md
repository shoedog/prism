# Lane P P1 r2d controller commit groups

No repairer Git writes performed. Original comparison base a66b877f49ba858c27b749b0a36bfccf4bc7da7d. Controller custody WIP 6965da75eb85f32d3ae0dda9e96704ad432375e6 was created during this repair. Cache remains105/61. These are proposed messages for controller custody after its acceptance checks; they can be combined into one atomic commit.

Controller snapshot message: `wip(paths): r2d in-progress snapshot (controller custody; repairer still running)`. It is custody only, with the final repair delta still dirty.

1. **`fix(paths): replace JS target cut with Node10 first-pass absence`**

   `src/{js_paths.rs,js_paths_first_pass.rs,js_paths_snapshot.rs,lib.rs}`; `tests/integration/{js_paths_r2d_test.rs,js_paths_common.rs,js_paths_cap_test.rs,js_paths_r1_test.rs,js_paths_repair_test.rs,js_paths_test.rs,main.rs}`; `tests/integration/fixtures/{js_paths_first_pass_ranges.json,js_paths_version_whitespace.json}`.

   Port the pinned TypeScript 5.9.3 priority-pass locations, including package JSON ordering/version mappings, custom roots and importing-file ancestor node_modules files/directories/@types. Bind JS-only targets after readable absence, keep installed/opaque/outside/skipped candidates at base, and make occupancy/cache dependencies explicit. Keep cuts2–6 and the r2c type-input rule. Shared positive helpers now require Exact; JS export-hop helpers still require the base row. Both grammars cover the R2-W2/r1-W3 inputs, absence/install/opaque cases and native package normalization, ECMAScript whitespace, root-sibling opacity and physical alias occupancy.

2. **`test(paths): bind r2d native cache and mutation witnesses`**

   `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/{repair_r2d.py,mutants.py,integration_mutants.py,resolver_driver.rs,oracle.cjs,verify_controls.py,CONTROLLER-paths.sh}`.

   Add native first-pass/cache transitions and one killed mutant per location class; preserve the legacy mutation population. The kernel runner accepts an exact Cargo extern manifest to prevent mixed serde artifacts. The independent native oracle removes its blanket JS label while retaining JS export-hop refusal. The controller wrapper defaults to the immutable r2d binary and is syntax-checked only; the repairer never opens F or runs the private wrapper.

3. **`docs(paths): record r2d control recovery and final verification`**

   `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/{SPEC.md,IMPLEMENTOR.md,REVIEWER.md,HANDOFF.md,MEASUREMENTS.md,BUILD-MANIFEST.md,REPAIR-R2D-FILES.md}`.

   Bind source, binary and receipts; separate certified recoveries from parked ownership rows and retained cut2 costs. No baseline, threshold, cache-version or parked S6/OQ2 changes. The controller must measure F before acceptance and handle Git custody/external backup.

Root VERIFICATION.md is ignored by repository policy, included in the local archive, and outside these proposed Git groups. Local evidence is target/repair-r2d/boundary-final; earlier target/repair-r2d root receipts are historical for preceding candidates. Final source remains frozen; measurement/docs changes do not alter the1017 build inputs.
