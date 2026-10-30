# Slice3 nonRust final receipt

Built source `fcb497327d941b1ec76f6a291b78fffdca84f885`, tree `ec64557615031fb6ba3fd35511982cf5e9cfdfb8`. Source-equivalence JSON/patch binds the authorized test-only successor `34db2e3536a67533779672bcf1b879403609e97b`; later test-only work requires controller reconciliation. Frozen binary hashes are in `frozen-binary-hashes.json`; commands, source pins, timing and log hashes are in individual result JSONs. No production changes, bootstrap repeat, publication or rebaseline.

| Gate | Actual result |
|---|---|
| Grammar | PASS: upstream baselines and patched parsers reproduced |
| Callable authority | 40 results, 0 failures |
| Node | 46 modules; 786 selected, 785 pass, 0 fail, 1 expected archive skip |
| Python | 940 cases covered = 895 initial eval passes + 1 exact targeted supplement + 44 unit passes; not one 940-pass run |
| Tier-A matrix | 159/159 actual successful rows |
| Genuine P15 | PASS: predecessor CPG95 direct Miss, separate CPG96 rebuild/Hit; exact argument→optional-entry tuple and full fresh/warm parity; navigation53 unchanged |
| Tier-A quick | INCOMPLETE/INVALID for accuracy: 1,200.037s wall cap, SIGTERM exit143, no terminal report |

Python initial sandbox loopback setup errors were inadmissible. A single authorized same-environment rerun selected the actual suites. Its sole eval failure was test_run_arm_isolated_prewarm_order: the test double classifies executable basename prism-python as other. Identical test on unchanged accepted base with identical binary bytes failed too; copying the same bytes to basename prism made the exact candidate test pass. Initial failure and base/candidate supplement logs remain. No package changes or broad retry.

Quick used an immediately rebuilt, frozen SUT; the corpus evolved during authorized test-only edits, so even a completed result would not support a clean accuracy claim. There is no terminal JSON/Markdown report and no SUT error-rate claim. The controller-authorized watchdog gracefully terminated only owned PIDs 21142,21143,21153,21173,57180; none remained. Process tree/state, partial snapshot and termination receipt are retained. Snapshot SHA256 `572711f7de752f0133e90c5ae1a46e2fbbbed724be2e690ca26e9f18dba1c4b2` was copied before removal of its exact unique generated worktree path. No retry. The runner's final old-HEAD guard then failed because the authorized test-only commit had advanced HEAD; individual gate artifacts and the explicit equivalence receipt govern results. Future runbook now requires immutable full-source corpus or a coordination lock.

Exclusions: three historical imported-props source-custody tests (missing five PRISM_AUDIT_* inputs/real-sites archive); one expected grammar archive tamper branch skip; live adoption file eval/adoption/tests/test_prism_adoption.py; human-triggered full Tier-A corpus. Quick accuracy unavailable as above. P15 genuine cache refusal is observed at version guard; build identity also differs, so this is not exclusive version-causation proof.

Artifact index: run_gates.py; all *.result.json; logs/; node-module-inventory.txt; p15-receipt.md; source-equivalence.json; quick-final-summary.json; quick-artifact-custody.json; quick-timeout-state/; tier-a-quick-artifacts/; verification runbook in parent orchestration root. SHA256 inventory is evidence-sha256.json.
