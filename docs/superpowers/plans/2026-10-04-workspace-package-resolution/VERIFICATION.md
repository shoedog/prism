# Verification and limits

The hook-required root report is [VERIFICATION.md](../../../../VERIFICATION.md),
with exact commands and explicit Verified/Not verified sections. The follow-up
rechecked source/test identity and all 42 baseline-missing positive controls;
no production or test code changed after the completed full suite.

Final candidate is bound by BUILD-MANIFEST.json and planning/head-v2-binding.json. Old head-final receipts are historical after the bounded native mode correction. No F, network or Git writes.

| Check | Result | Evidence |
|---|---|---|
| Requested single MCP nextest | 5,145 pass, zero fail, one skip; **before final native condition/mode corrections** | planning/nextest.log, 317.959s; cannot certify final source |
| Final full MCP cargo suite | **5,147 passed, zero failed, one ignored**, including two doctests; 31 result records | planning/full-v2.log, full-v2-totals.json, final-test-binding.json |
| Package regressions | 8/8 GREEN; seven selected tests are behavioral RED on clean main in same environment; new mode witness also RED before bounded correction | targeted-4.log; base-red.log; bundler-mode-red.log |
| Native package controls | 58 cases, **42 changed CORRECT_STATIC_BINDING**, zero lost/unproven; 16 unchanged/refused | controls-complete/summary.json and per-case ProjectService census/comparison/input hashes |
| fmt | PASS, cargo fmt --all --check | fmt-v2-valid.log |
| clippy | PASS, offline all-targets with mcp. Lib-test warnings 181, same as exact same-environment base control | clippy-v2-valid.log; clippy-base.log |
| Advisory scoped mutgate | 2 selected / 2 admissible / 2 KILLED; registry has 8. New untracked module's six mutations excluded by Git-diff filter | mutgate-v2/summary.json; not a full-registry acceptance claim |
| Tier-A matrix | 178 OK, zero regressions/skips; immediate same-worktree release rebuild equals final pinned head bytes | matrix-v2-rebuild.log; tier-a-v2.log |
| S1b-4 | 411 unchanged scenarios; **1,234 files byte-identical**, zero stderr, all call-site keys equal | s1b-v2-comparison.json; s1b-v2-byte-identity.json |
| Lane-P public preservation | X, installed-X, R, T complete streams byte-identical; no lost base module proofs; one native-correct installed-X proof added | public-v2/summary.json; public-v2-preservation.json |
| S2 scratch measurement | 0/132 recovered on X and installed-X; zero lost/unproven; trace union/last-state limit explicit | s2-final/summary.json; MEASUREMENTS |
| Controller preparation | bash -n PASS; Python probe AST parses; mutant JSON parses; underlying native checker exercised by controls | CONTROLLER-pkg.sh; BUILD-MANIFEST.json |

Full suite uses PRISM_TYPESCRIPT pointing at the pinned 5.9.3 path, CARGO_PROFILE_TEST_DEBUG=0 and CARGO_INCREMENTAL=0. The single ignored test is resolution_test::slice_elem_variant_reserved: SliceElem is reserved by the existing spec and classifier returns None. No out-of-scope test was re-baselined or repaired. The 58 native controls directly certify the existing self-reference, ES-module and custom-condition branches; production code did not change after its full suite. Classification's new public API has no pre-change API to execute; it is tested on head, while seven existing-path behavior tests provide base RED.

Temporary helper race caused inadmissible first fmt/clippy checks; their errors name only the transient example. They were corrected by finishing helper removal, then rerunning both permanent-source gates. The final suite passed without exclusions caused by the environment.

Not verified: private F; Tier-A quick (explicitly skipped by brief), full multi-corpus Tier-A, detached all-features suites, whole S2 suite, independent review, performance/adoption limits, off-machine custody, complete Node/TS conformance or discovery formats. The requested one nextest run is retained with its source limitation; final source is validated by the complete MCP cargo test run covering the same 5,145 native tests plus doctests. Six registry mutations were not executed. No new cost, refusal cut, virtual-loader policy or source-output substitution is accepted.
