# S1b-4 round-2 controller commit files — 2026-10-01

No Git writes. Branch feat/s1b-4-namespace remains at29686b66 with the verified
dirty fold; base5048f443. Retain target/repair-r2/ receipts, frozen binaries,
BUILD-MANIFEST.json, MANIFEST.sha256 and final-source-packet-snapshot.tar before
cleanup. These14 paths are the complete Git-visible changed/new commit population.

Recommended implementation message: **fix(js-ts): keep unproven namespace terminals at base**.

- `src/js_exports.rs`
- `src/resolution.rs`
- `tests/integration/js_binding_namespace/repair_r1.rs`
- `tests/integration/js_binding_namespace/repair_r2.rs`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/mutate_s1b4.py`

Recommended packet message: **docs: bind S1b-4 round-2 fold and disclose projection cost**.

- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/COMMIT-FILES-s1b4.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/CONTROLLER-S1b4.sh`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/HANDOFF-s1b4.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/IMPLEMENTOR.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/OQ-S1b4.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/REVIEWER.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/S1b-4-CONTROLS.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/S1b-4-MEASUREMENTS.md`
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/SPEC.md`

Root VERIFICATION.md is an ignored operational receipt (.git/info/exclude);
retain it with target/repair-r2/ evidence and the final snapshot, outside the
14-path commit list.

Controller runs the updated CONTROLLER-S1b4.sh privately against
target/repair-r2/prism-head and its manifest. The SHA is the starting commit;
complete source hashes bind the dirty repaired bytes. After committing, retain
that association; a clean rebuild requires refreshed binary/source provenance.
The r5 controls reference is unchanged; no r6. No F or clean-commit acceptance
is claimed by the repairer. P6 is a disclosed coverage survivor; the restored-arm
mutant is killed by all six new tests. No third review round is requested.
