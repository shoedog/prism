# R1 verification and limits

The retained full report is [VERIFICATION-root.md](VERIFICATION-root.md), within this packet. The previous link to a nonexistent repository-root report is corrected. Source is committed prototype 92c1d0bc plus R1; packet base is c89bc5b7; evidence is under `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1`.

| Check | Final result | Receipt |
|---|---|---|
| Requested single MCP nextest | 5160 pass / 0 fail / 1 skip; before final typesVersions guard, source-limited | nextest.log |
| Final full MCP Cargo suite | **5163 pass / 0 fail / 1 ignored**, 31 records including 2 doctests | full-r1-final.log, full-r1-totals.json |
| Package regressions | 24/24 GREEN; 10 graph-path tests behavioral RED on committed prototype, same environment | red-control-final.log; full-r1-final.log |
| TS 5.9.3 differential | 5096 cases; 2242 Bound / 861 ProvenUnresolved / 1993 Unsupported; 0 wrong/false absence; 1651 native-bound Unsupported | differential-r1-final/summary.json |
| Persisted caches | 12 checks; real CPG/sidecar full hits; all edit invalidations and cold/warm byte comparisons pass | cache-r1-final/summary.json |
| fmt | PASS | fmt-r1-final.log |
| clippy | PASS; 182 lib-test warnings, not attributed wholesale to base | clippy-r1-final.log |
| Advisory scoped mutants | 26 selected / 26 admissible / 26 KILLED | mutgate-r1-final/summary.json |
| Tier-A matrix | 178 OK / 0 regressions / 0 skips; immediate same-worktree rebuild | tier-a-r1-rebuild.log, tier-a-r1-final.log |
| S1b-4 | 411 unchanged / 1234 files byte-identical | s1b-r1-comparison.json, s1b-r1-byte-identity.json |
| Public/lane-P preservation | X, installed-X, R, T complete call streams byte-identical; 0 lost module proofs; 1 native-correct installed-X proof | public-r1-final/summary.json, public-r1-module-audit.json |
| Build custody | Final source/input census and measured immutable binaries rebound; retained MCP compiled artifact byte-identical | final-source-binding.json, BUILD-MANIFEST.json |
| Controller/probe preparation | bash syntax, Python AST/JSON and Node syntax checks pass | packet-checks.json |

No environment restriction excluded any part of the full MCP suite. The one ignored test is resolution_test::slice_elem_variant_reserved, reserved by the existing spec. The requested one nextest run is retained with its source limit; final source is certified by the full Cargo suite, not that earlier nextest receipt. Production/test bytes did not change after the final full suite; the final census was recorded afterward and frozen across custody rebuilds, not claimed as a pre-test census.

Older failed or setup-inadmissible receipts remain history: cache sidecar/filter setup, wrong eval-module import, earlier typesVersions placement and obsolete mutant witnesses. Their outputs certify neither the rejected hypothesis nor final R1. Cap extensions were disclosed, the complete typesVersions defect population enumerated, and no source restart occurred.

Not verified: Opus C3, C15, C17-C21, C23, C24, C26 (undefined in the supplied review and no public fixtures supplied); inherited ordinary lane-P C16b JSX follow-up; private F; new S2 execution/adoption; independent review; performance/RSS budgets; off-machine backup; complete Node/TS conformance; detached/all-features suites; final-source nextest repeat; Tier-A quick (explicitly skipped) and full multi-corpus. Unsupported native-bound cases remain an explicit S2-O9 gap. No public STOP, Git writes, network installs or private F access.
