# PR-B R6 / R6b final measurements

**Certificate NOT COMPLETE; mandatory STOP fired. No repair was attempted.**

Source frozen at `127cc2573d70b0f61d4d2243727302bdd218735b`; docs base `420167609d2c5cb2a2810ebbfb97ded3c74e5046`; source patch base `b249da9571990f5f0983b4e7310f7490ea856761`; main `da0604b3`; certificate comparison `f8c768b3`. Owner loop 3/3; no extension, product source changes, Git writes, network, package execution, frontend-portal access, delegation, merge or adoption in R6b.

The inherited R6 fix requires formal BoundNames membership before the named SEAM fallback. Synthetic JS/TS/TSX regressions failed all 6 shape/grammar comparisons on R5b and passed 54 focused callback/mechanism tests after the fix; PD93 is killed. All 35 historical outside-class rows are absent. R6b changes only evidence runners and documentation.

| Corpus | Expected | Completed | Capture admitted | Certificate passed | Certificate failed | Excluded | Unprocessed | Changed rows vs f8c768b3 | SyntaxError rows | Outside / named mismatch / forbidden / non-JS |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| X | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0/0/0/0 |
| Xi | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0/0/0/0 |
| T | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0/0/0/0 |
| SecBench | 583 | 392 | 388 | 387 | 1 | 4 | 191 | 873 | 64 | 595/0/0/0 |

Capture admission means producers completed; a captured root can still fail the certificate. No row credit is inferred from a timeout or an unprocessed root.

**WRONG — certificate scope violation:** `path-traversal/node-http-server_8.1.2` has 595 outside-class differences versus f8c768b3 (566 in `docs/assets/file_3.js`, 29 in `docs/assets/file_4.js`), alongside 10 accepted ARGS differences. Both independent capture stages report the same population. This is a certificate failure; semantic wrongness and attribution to R6 are not established. The representative `src` row at bytes 1156–1159 → 2031–2034 occurs once in old, zero times in main and head. See `r6b-STOP-evidence.json` and the full `outside.json` captures. No repair or additional root was dispatched after STOP.


Byte tables vs main: each cell is **CORRECT / WRONG / UNDECIDED**, after the documented source-bound ES overrides. Missing/excluded roots contribute no rows.

| Corpus | Table roots | ADDED | LOST | RELABELLED | RE-OWNED |
|---|---:|---|---|---|---|
| X | 1 | 15,513 / 0 / 0 | 0 / 3,415 / 2 | 0 / 0 / 0 | 0 / 0 / 0 |
| Xi | 1 | 15,513 / 0 / 0 | 0 / 3,415 / 2 | 0 / 0 / 0 | 0 / 0 / 0 |
| T | 1 | 19,434 / 0 / 0 | 0 / 374,016 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| SecBench | 387 | 213,329 / 0 / 0 | 0 / 493,866 / 2 | 1 / 0 / 1 | 0 / 0 / 0 |

Certificate exclusions and reasons:

- `prototype-pollution/swiper_6.5.0`: main, 600 seconds; retained original producer timeout; inadmissible.
- `prototype-pollution/total.js_3.4.6`: main, 600 seconds; producer timeout; inadmissible.
- `command-injection/total.js_3.4.6`: main, 600 seconds; producer timeout; inadmissible.
- `path-traversal/atropa-ide_0.2.2-2`: main, 600 seconds; producer timeout; inadmissible.

Identity results:

| Corpus | Admitted roots | Excluded roots | Call-site changes | Non-JS changes | Projection failures |
|---|---:|---:|---:|---:|---:|
| X | 1 | 0 | 0 | 0 | 0 |
| Xi | 1 | 0 | 0 | 0 | 0 |
| T | 1 | 0 | 0 | 0 | 0 |
| SecBench | 387 | 5 | 0 | 0 | 0 |

Each admitted identity root compares call-site bytes, byte/wire multiset projection on both binaries, and non-JS byte-row identity. The inherited mixed/native control has54,129 non-JS rows on each side, byte-equal.

- table exclusion `prototype-pollution/swiper_6.5.0`: {"admitted": false, "log": "/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r6/certificate-SecBench.log", "name": "prototype-pollution/swiper_6.5.0", "reason": "retained original producer timeout; inadmissible", "seconds": 600, "side": "main"}
- table exclusion `prototype-pollution/total.js_3.4.6`: {"admitted": false, "command_sha256": "b6c47c9e1df074f8b6da296eeefd47f4e731fff40859933503fd90b554eb81db", "elapsed_seconds": 600.021, "name": "prototype-pollution/total.js_3.4.6", "reason": "producer timeout; inadmissible", "seconds": 600, "side": "main", "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "timeout": true}
- table exclusion `command-injection/total.js_3.4.6`: {"admitted": false, "command_sha256": "39de87445633b4af25bf0f35e0a90f1d184e3af8b2911087f49635e82df0aaf7", "elapsed_seconds": 600.013, "name": "command-injection/total.js_3.4.6", "reason": "producer timeout; inadmissible", "seconds": 600, "side": "main", "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "timeout": true}
- table exclusion `path-traversal/atropa-ide_0.2.2-2`: {"admitted": false, "command_sha256": "8d7c6d61b44d80c175fd43a7adcd71a611779b9112d81b1d7dbb62fff42fdf1d", "elapsed_seconds": 600.039, "name": "path-traversal/atropa-ide_0.2.2-2", "reason": "producer timeout; inadmissible", "seconds": 600, "side": "main", "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "timeout": true}
- table exclusion `path-traversal/node-http-server_8.1.2`: {"STOP": true, "SyntaxError_rows": 0, "admitted": true, "capture": "/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r6/r6b/SecBench/path-traversal__node-http-server_8.1.2/capture", "census": {"named|ARGS": 28, "synthetic|ARGS": 10}, "changed_row_multiset_count": 605, "classes": {"ARGS": 10}, "name": "path-traversal/node-http-server_8.1.2", "named_mismatch_bindings": 0, "non_js_changed_rows": 0, "outside_count": 595, "retained": false, "row_counts": {"head": 21792, "main": 22405, "old": 22397}, "synthetic_forbidden_rows": 0}
- identity exclusion `prototype-pollution/swiper_6.5.0`: {"admitted": false, "log": "/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r6/certificate-SecBench.log", "name": "prototype-pollution/swiper_6.5.0", "reason": "retained original producer timeout; inadmissible", "seconds": 600, "side": "main"}
- identity exclusion `prototype-pollution/total.js_3.4.6`: {"admitted": false, "command_sha256": "b6c47c9e1df074f8b6da296eeefd47f4e731fff40859933503fd90b554eb81db", "elapsed_seconds": 600.021, "name": "prototype-pollution/total.js_3.4.6", "reason": "producer timeout; inadmissible", "seconds": 600, "side": "main", "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "timeout": true}
- identity exclusion `command-injection/total.js_3.4.6`: {"admitted": false, "command_sha256": "39de87445633b4af25bf0f35e0a90f1d184e3af8b2911087f49635e82df0aaf7", "elapsed_seconds": 600.013, "name": "command-injection/total.js_3.4.6", "reason": "producer timeout; inadmissible", "seconds": 600, "side": "main", "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "timeout": true}
- identity exclusion `path-traversal/atropa-ide_0.2.2-2`: {"admitted": false, "command_sha256": "8d7c6d61b44d80c175fd43a7adcd71a611779b9112d81b1d7dbb62fff42fdf1d", "elapsed_seconds": 600.039, "name": "path-traversal/atropa-ide_0.2.2-2", "reason": "producer timeout; inadmissible", "seconds": 600, "side": "main", "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "timeout": true}
- identity exclusion `path-traversal/node-http-server_8.1.2`: {"STOP": true, "SyntaxError_rows": 0, "admitted": true, "capture": "/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r6/r6b/SecBench/path-traversal__node-http-server_8.1.2/capture", "census": {"named|ARGS": 28, "synthetic|ARGS": 10}, "changed_row_multiset_count": 605, "classes": {"ARGS": 10}, "name": "path-traversal/node-http-server_8.1.2", "named_mismatch_bindings": 0, "non_js_changed_rows": 0, "outside_count": 595, "retained": false, "row_counts": {"head": 21792, "main": 22405, "old": 22397}, "synthetic_forbidden_rows": 0}

Navigation: fresh same-binary base controls and SHA/exit-bound retained head captures; each difference requires an exact removed-checker-WRONG proof.

- X: 241/241 admitted queries; 3 differences, 3 proved; callers/callees/repo-map identical=True; exclusions=0; base-control differences=0; source inputs unchanged=True.
- T: 81/81 admitted queries; 2 differences, 2 proved; callers/callees/repo-map identical=True; exclusions=0; base-control differences=0; source inputs unchanged=True.

O1 uses97 callback rows per run. All2720 raw invocation hashes, binary/selector bindings, physical callable/parameter identities and outcome replays were audited; no exclusions or invalid receipts.

| Run | Traced | Partial | Function only | Analyzer errors |
|---|---:|---:|---:|---:|
| o1-standalone-main | 0 | 0 | 1 | 96 |
| o1-standalone-head | 0 | 2 | 4 | 91 |
| o1-joint-main | 1 | 0 | 0 | 96 |
| o1-joint-head | 90 | 2 | 5 | 0 |

Inherited completed R6 gates were read and bound to the unchanged source, not rerun in R6b: matrix763/763 semantic cells (2289 grammar cells),0 DESIGN-CHANGE, all272 former parity failures PASS; focused54/54; nextest5228 passed/0 failed/1 ignored; doctests2/2; effective authoritative mutants87/87 (85 initial+2 bounded registry rebindings), PD93 killed; advisory3/3; cache-coupled1/1; fmt PASS; clippy235 head/235 main; Node probe unit30/30; Tier-A matrix178/178 with0 regressions/flip candidates; TS and Node quick VALID. R6b recovery runner13/13 tests passed, including both-side timeouts, terminal reuse, corrupt capture, source-change invalidation, capture publication, matched nav JSON refusals, zero-count classes and missing oracle records.

Bootstrap attribution control: head5/5 and main5/5 in the same environment. Initial full head5226 passed/2 MCP readiness timing failures; same-environment main full control5168 passed/6 Gitless archive failures, and both MCP cases passed. Timing remains a hypothesis, not a proved environment attribution. Final head full suite at4 workers passed5228/5228. Initial failures and main archive exclusions remain disclosed.

Matrix ES audit: raw233 LOST-CORRECT is53 correct-valued SyntaxError overrides+180 catch-var overrides; 67 total early-error overrides affect all verdicts. Effective LOST-CORRECT0/ADDED-WRONG0;45 matrix LOST-UNDECIDED rows receive no credit. The public and SecBench UNDECIDED rows in the table also receive no credit. Three inherited false-Exact X rows remain as main has them (E12), not ADDED.

Frozen binaries (verified before and after; source manifest573/573, all726 source/test/mutant files equal127cc257):

- `bin-final/prism-head-r6`: `64e9fce044e451474f8bb498daabe44ce060fe29395bf071c3a3f98d522c1205`
- `bin-final/seam-census`: `b188f6cc9cdf97a43eb724505f34d0cb118ab40f3f50d92991f4f406fccc77ae`
- `bin-final/prism-head-r6-bytes`: `fcb8cc57717127b36f4386606088536e03e0d504162d5b8004cd769da4bbbbc9`

Main/old binary bindings:

- `/Users/wesleyjinks/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3-bytes`: `184812b37f3f1d536d8a71b4cb49b0a46b22479d3c664452a6cdff0882fb6d83`
- `/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r2b/bin/prism-head-r2b-bytes`: `947ed1e53009898adf9c8ff2ea8aae64aae28fdc3e507bfafcd2bac1790ce174`
- `/Users/wesleyjinks/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3`: `294b18d0de2d33ff7faa207640fa5099e711d32100c2acb48b61e0674c50548f`

All173 paired retained scan/final head outputs are byte-identical; newly admitted roots use the final frozen byte binary. All measured source input hashes are unchanged at final audit. Legacy terminal receipts were bound to the verified frozen binaries without recomputation.

Artifacts: `R6-src.patch` relative to b249da95 reproduces127cc257 src/tests/mutants exactly (726 files/no extras); `R6-docs.patch` relative to42016760; `REPORT.md`, `HANDOFF.md`, `PROBE-LOG.md`, `r6b-summary.json`, `r6b-final-invariants.json`, `r6b-binary-binding.json`, `r6b-o1-audit.json`, `r6b-src-patch-check.json`, per-root raw outputs/exclusion receipts and `COMMIT-MESSAGES.md`. Controller owns commits.

Commit messages: `fix(js-param-defs): restrict named seam fallback to formal bindings` (source, already held in127cc257); `docs(js-param-defs): record R6 certificate denominators and recovery evidence` (documentation).

Not verified: unprocessed roots, excluded roots/stages and tables/identity on the certificate-STOP root; semantic truth of UNDECIDED rows and outside-class differences; F/frontend-portal (explicitly forbidden); controller Rust quick and full multi-corpus Tier-A; independent review/merge/adoption. The incidental eligible sweep remains272/373 on each side and supplies no whole-sweep acceptance claim. The full product suite and other completed gates were not rerun in this continuation. Frozen binaries were authenticated by original build/source manifests and before/after byte hashes; no fresh rebuild was needed or performed in R6b.

Table coverage audit: 390 admitted roots; rowdiff, changed-row stream, oracle-detail stream and raw verdict partition agree numerically for every class; zero coverage mismatches. This proves coverage, not the semantic truth of UNDECIDED rows.

**STOP:** mandatory; see `r6b/STOP.json` and `r6b-summary.json`.
