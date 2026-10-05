# R4 patch and proposed commit boundaries

**STOP: R4 yield 0/0/0/0; X shortfall 132 from132 exceeds10; selection NONE.**

| Proposed controller-only commit message | Files and exact parent |
|---|---|
| `fix(resolution): gate S2 writer exclusion on PKG proven absence` | R4-src.patch relative61641bdb + PKG03fa9c29: src/js_paths.rs,src/js_import_qualifiers.rs,src/call_graph.rs,CPG/nav epochs,integration S2 tests,29-mutant S2 registry. R4-s2-only.patch projects the same S2 changes onto61641bdb without PKG hunks/files. |
| `docs(plan): record S2 R4 yield STOP and PKG coverage gaps` | R4-docs.patch relativeaa55a532: S2 packet/spec/OQ/dispatch/report/verification/measurements/handoff/inventory/manifest plus facts writer-status extraction and usage-mode native gap oracle. |

No commits made. Patches are archival/review artifacts, not selected product changes. Separate PKG implementation is03fa9c29; S2 depends on its prior merge. Exact replay checks and patch hashes:repair-r4/patch-application-check.json/PATCHES.md. Private F and controller acceptance were never executed. Preserve snapshots before cleanup.
