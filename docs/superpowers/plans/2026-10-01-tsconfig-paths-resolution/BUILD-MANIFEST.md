# Lane-P P1 repair round 1 build manifest

[MEASURED] Workspace `/Users/wesleyjinks/code/prism-paths-impl`, branch `feat/tsconfig-paths-p1`, dirty repairs on integrated parent `7f5a862afd002dd5953108a3ab5b611d8745bde9`. Base object `dab8251c6013b28b0db4cb4042f36db444eb556c`; no local main ref was required. No Git writes, network/install or F reads. Earlier prototype-era manifest claims are superseded for current operation; the controller commits the listed repair files on this parent.

[MEASURED] All evidence is under `target/repair-r1/`. Base and pre-repair sources were archived before production edits. Base binary and base import-facts helper were freshly built offline in this environment; base input hashes are checked against its Git blobs. Pre-repair binary supplies same-environment RED. Final release build: `cargo build --release --offline`, receipt `head-build-verified.log`. The final immutable path was created after that build and is never overwritten. Source binding lists 556 build/source/test inputs and the independently bound base inputs; test-only RED copies do not change the pre-repair production archive.

| Executable | SHA-256 |
|---|---|
| `target/repair-r1/base/prism` | `d5b3a84ff9b10fcc8fbda4b7050a625bdb10f3ef7b9eb27ffca9ac4e88151aa6` |
| `target/repair-r1/before/prism` | `7ec582fa3bad07d3a4be5c59252de9ddd46576ba4a49f2d5f9e24978e69cbc9b` |
| `target/repair-r1/head/prism-r1-verified` | `c8a4427ee44b147c7eaa45eea553a5dedc135eaa49c719b431bdd4d6fd4b75cb` |
| `target/repair-r1/base/dump_imports` | `fd77e531437d4907313ffd98d11f31d5289aecd0854ca60b72447de010f5a2ca` |

[MEASURED] Authoritative repaired binary: `target/repair-r1/head/prism-r1-verified`. Its dirty version banner is supplemented by exact source hashes and build receipt. Other head/prism variants are superseded. Offline TypeScript 5.9.3 SHA-256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`; `env.sh` records PRISM_TYPESCRIPT and installed Python 3.12 path. Cache versions advance integrated parent 105/61 to **106/62**; version pin tests pass.

`PRISM_TYPESCRIPT=/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` for all three full suites. Final custody: `target/repair-r1/{repair.patch,owned-files.tar.gz,owned-file-hashes.json,custody-final.json}`; the archive is byte-verified against all 26 current owned files, including the root VERIFICATION.md required by the verification gate.

[MEASURED] Verification-gate follow-up changes no production/Rust test bytes. Full-suite source bindings still match. Replayed pre-repair integration: 11 behavioral failures / one preservation pass; restored pre-change case ordering: one behavioral unit failure. Added executable process-level performance regression gate: repaired Nx and declined Bundler GREEN; fresh pre-repair replay RED with median RSS 3.07x and 2.91x their same-environment base. Receipts: all-repair-red-recheck.log, case-order-red-recheck.log, perf-gate-green.log, perf-gate-red.log, perf-prechange/summary.json. Current filesystem aliases the two config-name variants to one inode; native case-sensitive coexistence is environment-limited, while both-order unit coverage passes. Exact commands/totals and remaining exclusions are in VERIFICATION.md.

## Verified receipts

| Check | Receipt | Result |
|---|---|---|
| Full default/MCP/all-features | suite-summary.json and three suite logs | 4827 / 5020 / 5043 passed, zero failures, one existing ignore each |
| fmt / diff / clippy | fmt.log, diff-check.log, clippy-summary.json | clean format/diff; 372 base and head warnings, no new warnings |
| Tier-A matrix | tier-a-matrix.log | 170 ok, zero regressions, immediately preceded by release rebuild |
| Tier-A quick | tier-a-quick-base.json, tier-a-quick-verified.log | both complete but baseline-invalid with identical oracle reasons; SUT error rates zero and SUT probe/pinned values identical; no rebaseline |
| Expanded packet / oracle | controls-verify.log, controls-P0.json, controls-classified.json | 317 scenarios, 333 sites, 80 changes: 76 correct bindings, 2 correct refusals, 2 parked C80/S6 UNPROVEN |
| Kernel mutations | mutants-summary.json | 38/38 killed on final source |
| Integration/library mutations | integration-mutants-summary.json | 14/14 killed, 11 legacy integration + 2 new integration + 1 case-collision unit |
| Cache | cache-verified.log, repair-cache-verified/summary.json | legacy full probe and 16 new cases / 32 directions pass; unrelated text stays hit |
| Budget | budget-red.log, budget-verified/summary.json | missing signal RED before; two complete base rows with warning after |
| S1b-4 | s1b/byte-identity-verified.json | 411 scenarios, 639 sites, 1234 byte-identical artifacts |
| Public X/R/T | public-verified/FINAL-SUMMARY.json | 19219/953/61712 sites; 3121/0/0 changes; every changed row CORRECT_STATIC_BINDING |
| Performance | perf-summary.json | Nx wall +15.8%, RSS +16.6%; Bundler no observed time penalty; wildcard slower/variable, all numbers in MEASUREMENTS |
| Controller | controller-smoke-verdict.json | public synthetic input only; actual_F_run=false; two UNPROVEN rows intentionally prevent certification |

MEASUREMENTS explains the disposition of every inherited finding, conservative refusal costs, X barrel yield and limits. The mutation logs read intended assertion failures and exact selectors; compile, zero-test or setup failures are inadmissible. Same-environment pre-repair regression run: 11 behavioral RED groups and one existing preservation pass. Every new cut has both-grammar packet witnesses and killing mutations; the budget signal has a standalone behavioral RED/GREEN probe.

## Custody and controller actions

No commit/push is authorized here. Repair patch, owned-file snapshots/list and hashes are stored under `target/repair-r1/`; REPAIR-R1-FILES.md lists controller commit groups. A local ignored snapshot is not external backup. The controller must commit/rebuild or bind the snapshot, then execute CONTROLLER-paths.sh for F. Its default executable is the authoritative immutable path and aggregates include the executed SHA-256. The public synthetic smoke has the wrapper's fixed F label but is explicitly not a private-F receipt. Refusal reasons remain independent explanations with an explicit UNCLASSIFIED fallback; new cut costs are enumerated by the expanded control census, not inferred from process status.

No independent round-2 review, private F, full multi-corpus Tier-A, native Linux case-collision coexistence or concurrent-filesystem custody audit was performed. Existing S6/OQ2 is parked. Foreground verification has no Git/network/provider write; controller custody remains open.
