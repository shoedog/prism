# Handoff — DFG PR E merge of main b2cd10a1

**Written:** 2026-10-17T16:37:26.430954+00:00 · **By:** Codex root · **Provider:** codex
**Workspace:** /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/impl-dfgE · perf/dfg-activate · **Measured state:** `[MEASURED]` HEAD 330db2e8d176c042d537ee4f7fa6ee52604eafa9 · Tree DIRTY; merge index unresolved · Probe git status/rev-parse · Output ../dfgE-merge-b2cd10a1/source-identity.json
**Predecessor:** controller dispatch; previous worker handoff retained in ../dfgE-merge-b2cd10a1/before-handoff.md.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts stay OPEN.
**Provenance:** written live by worker. `[MEASURED]` rerun here; `[INHERITED]` supplied, not rerun.

## 0. Gating facts — settle these before starting anything below
**(a) Lane ownership** — `[INHERITED]` owner assigned this worker; no delegation — RESOLVED.
**(b) Custody exposure** — `[MEASURED]` final-source.tar.gz, merge-working.patch, raw logs and receipts in ../dfgE-merge-b2cd10a1; controller must commit merge and preserve evidence outside ephemeral scratch — OPEN for controller.
**(c) In flight / irreversible** — `[MEASURED]` all worker Cargo/probe sessions completed; no Git writes; merge still in progress — RESOLVED for worker activity, OPEN for controller merge completion.
**(d) Authorization granted but not exercised** — "You cannot run git commands that write .git; edit files only; the controller commits the merge."

## 1. Resume order
1. Read VERIFICATION.md and ../dfgE-merge-b2cd10a1/gate-totals.json; verify custody.sha256.json and preserve final-source.tar.gz, merge-working.patch and raw receipts in durable controller storage.
2. Stage existing merge changes and the resolution paths in ../dfgE-merge-b2cd10a1/resolution-paths.txt. The new re-pin receipt is untracked; VERIFICATION.md is ignored and needs force-add if committed. Commit the merge; no worker .git write was attempted.
3. Run the 109 named exclusions in ../dfgE-merge-b2cd10a1/mcp-exclusions.txt on a capable host and obtain CI. Keep Task 7 acceptance bound to activation 3739ab38; PR F remains a separate task after E merge.
**STOP conditions:** unexplained output drift; missing custody; any requested gate failure. Two validation/fix rounds were allowed; one candidate passed without production fixes.

## 2. State ledger
| Item | State | Evidence / correction |
|---|---|---|
| Five conflict files | done | `[MEASURED]` no markers; README uses main procedure; manifests use main hashes plus current provenance; index still UU until controller stages |
| Cache/mutants | done | `[MEASURED]` CPG122, nav67; cache-mutant-anchors.json lists all four unique 122->121 anchors |
| Main retention | done | `[MEASURED]` main-merge-retention.json: all 31 main-only paths exact plus two authorized cache-version replacements |
| Accounting/census | done | `[MEASURED]` auto-merge-check.json: all four files equal both parents, one argv-stub registration |
| Captures | done | `[MEASURED]` repin-result.json: main and branch 133+133 equal; 30 accepted #442 changes, 103 unchanged per map |
| MCP suite | done | `[MEASURED]` logs/full-mcp.log: environment-limited, 6249 passed, 0 failed, 3 ignored, 109 excluded |
| Oracle umbrella | done | `[MEASURED]` logs/oracle-umbrella.log: 168 passed, 0 failed, 1 ignored, including all pins |
| CI parity | done | `[MEASURED]` logs/ci-parity.log: 161 passed, 0 failed, 1 ignored, 7 pins tests filtered |
| Cache unit/fmt | done | `[MEASURED]` logs/cpg-cache-final.log: 76 passed, 0 failed; logs/fmt-final.log PASS |
| Pre-bump control | done | `[MEASURED]` isolated exact 122->121 source mutation: 72 passed, four expected version assertion failures; candidate then 76/0 |
| Task 7 acceptance | done | `[INHERITED]` user and committed docs/eval/dfg-compaction/acceptance.md: PASS bound to 3739ab38, not this merge |

## 3. Corrections to standing documents and memory
| Location | Stale or false assertion | Correction |
|---|---|---|
| Cache docs/mutants | CPG121 | `[MEASURED]` CLAUDE.md, current feature docs and four mutants updated to122; historical measured versions remain unchanged |
| Previous lane handoff | Task 7 pending, old base/pins | `[INHERITED]` acceptance PASS at330db2e8; `[MEASURED]` current merge captured on b2cd10a1, 133 cases |
| Feature documentation | Acceptance pending | `[INHERITED]` now links controller's committed PASS; no new measurement claim |
| Memory | None | No memory update requested or performed |

## 4. Open work
| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Merge commit/custody | pending | Stage resolved paths, commit merge, preserve receipts | controller .git authority | resolution-paths.txt, final-source.tar.gz |
| 2 | Socket/host checks | blocked | Run exact named exclusions on capable host | sandbox listener/host facilities | mcp-exclusions.txt:109 names |
| 3 | CI | pending | Run controller publication/CI process | controller | PR439 |

## 5. Invariants and traps — do not do these
- No worker .git writes, installs, network or corpus measurements occurred.
- nice setpriority denied but Cargo continued; subsequent commands used env CARGO_BUILD_JOBS=4 fallback, offline dependencies and RUST_MIN_STACK=16777216.
- Initial formatter briefly overlapped branch-capture launch; no two build/test Cargo jobs overlapped. All subsequent Cargo commands were serial.
- Both manifests observe field-wise legacy payload and ordered indexes; native cache format is122. Do not replace these observations with native codec bytes.
- Known 108 socket tests plus one launcher case are exclusions, not fresh passes; all109 names are preserved.
- Source/control copies and test receipts are ephemeral until controller custody. Do not delete scratch first.
- Source cache module remains above1000 weighted lines because only metadata/assertions changed; no responsibility was added and no over2000 file grew.

## 6. Identifiers
| Item | Verbatim |
|---|---|
| HEAD | 330db2e8d176c042d537ee4f7fa6ee52604eafa9 |
| MERGE_HEAD/main | b2cd10a157bb7d7e75283be6519b6fe2ce485e0f |
| Measured activation | 3739ab383a6e333e3c172bcb1eaf26fcf3d26b92 |
| Evidence | ../dfgE-merge-b2cd10a1 |
| Versions | CPG122; nav67 |
| Exact regression control | src/cpg_cache.rs: const CACHE_VERSION: u32 = 122; ->121; isolated copy only |

## 7. Refutation verdict and owner questions
**§2c verdict:** SURVIVED · claim: "merge preserves all clean-main pinned output and enforces cache122" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: repin-result.json, gate-totals.json, cache-control-result.json, probe-log.md
**Questions the owner owes an answer to:** None.

Verification-hook addendum: root VERIFICATION.md now embeds all exact commands
and all 109 excluded names under explicit environment-limited / Excluded: wording.
No Rust source or test receipts changed; no test rerun was needed.
