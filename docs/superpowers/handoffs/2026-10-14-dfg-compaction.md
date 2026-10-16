# Handoff — DFG compaction PR D Task 5

**Written:** 2026-10-16T17:12:15.175743+00:00 · **By:** Codex · **Provider:** codex
**Workspace:** /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/impl-dfgD · perf/dfg-codec · **Measured state:** `[MEASURED]` HEAD cf22c8e8517f9eec68751e2ea7739d4c6dead2f6 · Tree DIRTY · Probe git status/rev-parse and gate logs · Output ../dfgD-scratch/
**Predecessor:** `[INHERITED]` PR C #415 merged; prior operational handoff preserved as ../dfgD-scratch/before-handoff.md.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts stay OPEN.
**Provenance:** written live by worker. `[MEASURED]` means captured this run; `[INHERITED]` means controller dispatch/prior receipts; `[ASSUMPTION]` and `[UNKNOWN]` are explicitly marked when applicable.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` controller owns commits/publication; worker implements Task 5 — RESOLVED by dispatch.
**(b) Custody exposure** — `[MEASURED]` dirty tracked and new files preserved in source-final.tar.gz, task5.patch, source-paths.json and custody-final.sha256 — RESOLVED for worker checkpoint; OPEN for controller durable custody/commit.
**(c) In flight / irreversible** — `[MEASURED]` all worker gates completed; no Cargo process or Git index/ref write in flight — RESOLVED by final receipts.
**(d) Authorization granted but not exercised** — “The controller commits your work ... leave changes in the working tree”; no worker commit/push/merge authority.

## 1. Resume order

1. Read root VERIFICATION.md, ../dfgD-scratch/SUMMARY.md, new-test-red-map.json and custody-final.sha256; preserve the two final archives and patch outside ephemeral scratch before cleanup.
2. Review source-paths.json / task5.patch. Run the 67 exact socket exclusions on a capable host, refresh staged scoped selection and the all-lanes authoritative pre-merge gate (496 catalog entries). Literal main ref was absent here; exact-parent scoped selection was zero, not coverage.
3. Controller reviews exact Task 5 paths and commits `test: stage validated compact DFG cache codec` with both plan trailers. No re-pin commit: both manifests reproduce unchanged on main cf22c8e8.

**STOP conditions:** output/hash drift, unexplained base failure, missing custody; two-round review cap. No runtime activation/version bump, old-cache importer or real-corpus measurement in PR D.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Clean-main re-pin | done | `[MEASURED]` 132 payload + 132 index hashes unchanged; repin-result.json and logs/capture-main.log |
| Native codec | done | `[MEASURED]` borrowed/owned field order, ordered map sequences, persisted rank arrays, linear rank/reference guards; source-checkpoint.tar.gz |
| Focused tests | done | `[MEASURED]` 78 DFG + 3 cache-codec pass; scaffold compile RED is only API evidence |
| Behavioral REDs | done | `[MEASURED]` catalog 15/15, supplementary 15/15 after targeted round 2 (2/2); 50/50 new tests in new-test-red-map.json; initial arena-order survivor resolved by test-input correction, no implementation edit |
| Full suites | done | `[MEASURED]` final default 5571/0/1 ignored, no filtered; final MCP 6122/0/1 ignored, 67 filtered; logs/full-*-totals.json; rerun after test strengthening |
| Oracle / branch pins | done | `[MEASURED]` 166/0/1 ignored including pins; branch capture 132+132 equals clean main; branch-pin-control.json |
| Release / Tier-A | done | `[MEASURED]` fresh release; 178 matrix OK; quick 3 VALID/0 INVALID/0 regressions; one target-c-method flip candidate and two missing pinned outcomes retained in tier-a-verdicts.json and SUMMARY.md |
| Verification report | done | `[MEASURED]` root VERIFICATION.md records exact commands, totals, behavior/RED map and all 67 environment-limited exclusions; documentation-only addition after final suites |
| Format / feature compile | done | `[MEASURED]` final fmt clean; mcp,dfg-oracle all-target compile 36 executables |
| Scoped gate | pending | `[MEASURED]` literal main missing; exact-parent selected 0/496, not coverage; controller refresh after staging; lane text 15/15 and supplementary final 15/15 are the executed mutation evidence |
| Listener control | done | `[MEASURED]` clean base Unix bind denied; 67 exact exclusions in mcp-exclusions.txt |
| PR C accounting checkpoint | done | `[INHERITED]` prior receipt copied to pr-c-accounting/ before codec work; no new heap claim |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Previous handoff | PR C workspace/status | Superseded here; original retained in before-handoff.md |
| Plan historical cache v117/118 | Current main version | `[MEASURED]` current version 120, unchanged in PR D; PR E must choose current main + 1 |
| “No dead rows” shorthand | Strict liveness immediately after combine | `[MEASURED]` label collisions can leave unreferenced endpoint rows until the next update; codec accepts them, preserves IDs/bytes; colliding_label_merge_allows_unreferenced_rows_until_update asserts both transient rows and later reclamation |
| Prior Tier-A shorthand | Previous handoff said no flips | `[MEASURED]` raw pinned results include target-c-method flip; current run preserves it and makes no exact-parent attribution; older raw reports remain historical evidence |
| Memory | None | No update requested or performed |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Staged scoped gate | pending | Refresh --since main --scope fn after staging/committing | Missing local main ref / untracked codec files | ../dfgD-scratch/logs/mutgate-scoped-*.log |
| 2 | Durable custody/commit | pending | Preserve scratch artifacts, review exact paths, commit Task 5 | Controller Git authority | perf/dfg-codec |
| 3 | Socket tests/global mutation gate | pending | Run exact exclusions on capable host; full authoritative all-lanes gate before merge | Sandbox bind restriction / controller merge gate | mcp-exclusions.txt |
| 4 | PR E arena privacy | parked | Make arena fields private with read-only accessors at activation | F3 is explicitly later scope | Task 6 |

## 5. Invariants and traps — do not do these

- Compact code is cfg(test)/dfg-oracle only; production legacy layout/version 120 stays unchanged.
- RD decoded counts must match private identity sets (F1); no builder creates mismatches. Reject rather than change combine semantics.
- Unreferenced arena rows are permitted (F2); reference integrity and rank identity/order are mandatory. The next update reclaims losing label-only endpoints.
- Each adjacency/label representative and neighbor sequence is primary data; decode never rebuilds or sorts those maps.
- Nice is denied: owner-authorized env fallback, jobs 4, offline; no installs/network/Git writes.
- RED compile failures, panics instead of returned errors, zero-selected tests and environment refusal do not prove validation behavior.
- Registered mutation budget remains 15: new codec group guard replaces redundant projector-label omission; projector-edge omission and the ordinary label regression remain.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| HEAD | cf22c8e8517f9eec68751e2ea7739d4c6dead2f6 |
| Branch | perf/dfg-codec |
| Scratch | /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/dfgD-scratch |
| Commit | test: stage validated compact DFG cache codec |
| Trailers | Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>; Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "Native compact decode preserves ordered observations and rejects malformed references before indexing" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: ../dfgD-scratch/new-test-red-map.json, mutation-verdicts.json, gate-results.json, logs/full-*-totals.json, tier-a-verdicts.json

**Questions the owner owes an answer to:** None. Controller commits and host/global gates remain explicit actions; no zero-flips or merge-ready claim.
