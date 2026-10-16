# Handoff — DFG compaction PR C Tasks 3–4

**Written:** 2026-10-16T15:27:01+00:00 · **By:** Codex · **Provider:** codex
**Workspace:** /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/impl-dfgC · perf/dfg-arena · **Measured state:** `[MEASURED]` HEAD 14a9932d644f723991efa8200aacc84fd8f92846 · Tree DIRTY · Probe git status/rev-parse and captured gates · Output /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/dfgC-scratch/logs/
**Predecessor:** PR B #413 merged on main; controller's PR C dispatch. Prior handoff retained as before-handoff.md in scratch.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts stay OPEN.
**Provenance:** written live by worker; all gate totals below `[MEASURED]`. Owner scope/commit authority `[INHERITED]` from dispatch.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` controller owns commits/publication; worker implements PR C — RESOLVED by dispatch.
**(b) Custody exposure** — `[MEASURED]` uncommitted source/new files; ordered repin/task3/task4 patches, task3-source.tar.gz, source-final.tar.gz and evidence-final.tar.gz in scratch; custody-final.sha256 records integrity — RESOLVED for worker snapshot, OPEN for controller durable custody/commits.
**(c) In flight / irreversible** — `[MEASURED]` worker gates complete; no worker Git index/ref writes, no commit/push/reset/stash; no worker gate in flight — RESOLVED by completed receipts.
**(d) Authorization granted but not exercised** — “controller commits your work ... leave changes in the working tree”. Worker has no merge/publication authority.

## 1. Resume order

1. Read /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/dfgC-scratch/SUMMARY.md, VERIFICATION.md, repin-result.md and split-replay.json; preserve evidence-final.tar.gz and custody-final.sha256 outside ephemeral scratch before cleanup.
2. Review/stage only the two manifests; controller first commit `re-pin on main 14a9932d`. All 132 payload and 18 index changes are explained by b199f79b #409; 114 indexes unchanged.
3. Use task3.patch then task4.patch as exact commit boundaries. They replay after repin.patch on clean 14a9932d and reproduce all PR files. Do not apply them over the final working tree. Root VERIFICATION.md is an untracked review artifact; do not stage it.
4. Controller reviews and commits: `feat: add compact DFG arena behind test oracle`, then `feat: preserve DFG update semantics with compact IDs`; both plan trailers required. Task 3 snapshot independently compiled/unit/oracle checked; final PR gates cover both tasks.
5. Before merge run the 67 exact listener selectors on a capable host and the global `python3 scripts/mutgate/mutgate.py --authoritative --jobs 1` with the Cargo wrapper and no --since/--only. Refresh scoped selection after new files are staged/committed.

**STOP conditions:** any output/hash difference from re-pinned main; unexplained base failure; missing evidence custody. Two-round review cap; no fresh restart. No cache activation/importer or real-corpus measurement in PR C.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Re-pin | done | `[MEASURED]` clean 14a capture; 5a63 old control 264/264; b199 control reproduces all 264 new hashes; repin-result.md / repin-comparison.txt |
| Task 3 | done | `[MEASURED]` immutable exact rows/ranks/representatives, borrowed compact views, copied query bodies; saved source and task3.patch; boundary units 26/0 and oracle 165/0/1 ignored |
| Task 4 | done | `[MEASURED]` ID filter/combine/reclamation, all operation states and held-clone checks; final DFG units 31/0; final oracle 166/0/1 ignored |
| Behavioral RED | done | `[MEASURED]` 15/15 registered + 18/18 supplementary isolated text kills; 17 new unit tests covered by new-test-red-map.json; accounting shadow positive/negative controls |
| Fixture checkpoint | done | `[MEASURED]` 132 + 3 rows; repeated 613404 -> 16772 B; distinct 150142 -> 162238 B; empty 0 -> 0; accounting/ |
| Full default | done | `[MEASURED]` 5509/0/1 ignored, 0 filtered, 29 summaries; logs/full-default.log |
| Runnable full MCP | done | `[MEASURED]` 6060/0/1 ignored, 67 filtered, 31 summaries; logs/full-mcp.log; exact list mcp-exclusions.txt |
| Format / feature compile | done | `[MEASURED]` fmt check clean; mcp,dfg-oracle all-target no-run compiles 36 executables |
| Release / Tier-A | done | `[MEASURED]` release rebuilt; matrix 178 ok, zero regressions/flips; quick 3 VALID/0 INVALID, no regressions/flips; release-tier-results.json / tier-a-verdicts.json |
| Final maintenance audit | done | `[MEASURED]` payload historical row order retained, clean-main hashes unchanged; focused pins 7/0; ordered patch replay and file ledger pass |
| Host exclusions | pending | `[MEASURED]` clean-main listener control fails bind with EPERM; 67 exact selectors excluded; no repeated listener probes |
| Global pre-merge mutgate | pending | `[MEASURED]` catalog population 493; full all-lanes gate not performed in implementation handoff |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Stored pins | pre-#409 b994 hashes | `[MEASURED]` main 14a values with old/new control receipts; historical payload row order retained |
| Previous handoff | PR B operational status | Superseded by this PR C handoff; original preserved in before-handoff.md |
| Plan sample row views | allocates one iterator box per row | Compact companion uses stack ID iterator; production boxed views stay until Task 6; do not re-box when forwarding |
| Memory | None | No memory update requested or performed |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Durable custody / three commits | pending | Preserve archives, review ordered patches, commit pins then Tasks 3/4 | Controller Git authority | perf/dfg-arena |
| 2 | Listener coverage | pending | Run mcp-exclusions.txt selectors on socket-capable host | Sandbox Unix bind EPERM | 51 unit + 16 process tests |
| 3 | Global pre-merge gate | pending | Authoritative all-lanes gate, jobs 1, wrapped Cargo | Controller merge gate | 493 mutants |
| 4 | Codec / activation / acceptance | next | Proceed with Tasks 5/6, retain oracle until Task 7 accepted and E merged | Later PRs | Cache currently 120 |

## 5. Invariants and traps — do not do these

- Production remains legacy; compact is tests/dfg-oracle only. No production heap win claimed.
- Exact IDs never substitute for legacy sort ranks; map-local key/label representatives retain their own spans.
- Some adjacency imports faithfully; None rebuilds. Remove/merge always rebuild, including empty/unknown exclusions.
- Liveness includes every primary reference, including label-only/exact-only rows; empty binding keys survive by key-file rules.
- Do not count scaffold compile RED, zero-selected scoped gate or exit code alone as behavioral evidence.
- Literal main ref is absent here; exact-parent scoped gate selected zero because new source is untracked. Coverage comes from explicit isolated text lane/control gates.
- Nice denied: owner-authorized env fallback, jobs 4, offline, one Cargo process; no install/fetch/Git writes.
- Fixture accounted bytes exclude BTree nodes/slack/allocator rounding and private RD key heaps as documented; they are not real-corpus live heap, peak or timing.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| HEAD | 14a9932d644f723991efa8200aacc84fd8f92846 |
| Branch | perf/dfg-arena |
| Scratch | /private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/dfgC-scratch |
| Commit boundaries | repin.patch; task3.patch; task4.patch |
| Source custody | task3-source.tar.gz; source-final.tar.gz; evidence-final.tar.gz; custody-final.sha256 |
| Release receipt | release-binary.json; target/release/prism |
| Raw report | VERIFICATION.md (scratch copy and untracked root copy) |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "Compact storage, queries and ID updates preserve exact bytes on the finite corpus and adversarial operation sequences" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: final-focused-results.json, new-test-red-map.json, mutgate-lane/summary.json, mutgate-supplementary/summary.json, accounting-control.json

**Questions the owner owes an answer to:** None. Controller review/commits, host listener coverage and global pre-merge mutation gate remain explicit actions.
