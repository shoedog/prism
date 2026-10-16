# Handoff — DFG compaction PR B owner decisions

**Written:** 2026-10-16 · **By:** Codex · **Provider:** codex
**Workspace:** impl-dfgB · perf/dfg-views · **Measured state:** `[MEASURED]` HEAD 804f7e342f59163ebb2d2fc2cf28f8fbe62e3044 · Tree DIRTY · Probe git status/diff and requested gates · Output ../dfgB-scratch/owner-decisions-2026-10-16/
**Predecessor:** PR B ordered-views worker; prior handoff preserved as ../dfgB-scratch/owner-decisions-2026-10-16/before-handoff.md.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts remain open below.
**Provenance:** written live by worker. `[MEASURED]` claims were rerun in this follow-up; `[INHERITED]` claims name their source.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner assigns two decisions only; controller owns commits — RESOLVED by 2026-10-16 dispatch.
**(b) Custody exposure** — `[MEASURED]` uncommitted plan, CI filter/tests, handoff and root VERIFICATION.md; retained source/evidence in ../dfgB-scratch — RESOLVED for worker handoff by snapshot; controller must preserve and commit.
**(c) In flight / irreversible** — `[MEASURED]` replay, requested gates and stop-hook suites completed; no worker process remains and no commit/push attempted — RESOLVED.
**(d) Authorization granted but not exercised** — “Apply these two owner decisions; leave the changes uncommitted for the controller.” No Rust production change, publication or merge authorized.

## 1. Resume order

1. Read `../dfgB-scratch/owner-decisions-2026-10-16/comparison.log` and `summary.json` for the owner-authorized replay. Retained binary identities and receipts are in `../dfgB-scratch/binary-receipts.json`; no rebuild is claimed in this follow-up.
2. Inspect `git diff --stat` and `git diff --check`, then review the four modified tracked files plus new root VERIFICATION.md. Controller stages the four modified tracked paths and commits; root VERIFICATION.md is locally ignored by .git/info/exclude and retained in the scratch snapshot. Worker does not write .git.
3. Preserve the scratch evidence and snapshots. Historical implementation verification, pre-A reconciliation, Tier-A observations and mutation evidence remain in `../dfgB-scratch/SUMMARY.md`, `owner-decisions-2026-10-16/before-VERIFICATION.md`, and the prior handoff snapshot. Current verification is at root VERIFICATION.md and copied into scratch.

**STOP conditions:** unexplained byte drift, mismatched binary or provenance receipt, missing/unexpected provenance fields, or any need to widen the owner-authorized exception. Two replay attempts maximum before classification; no external reviews dispatched.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| PR B implementation | done | `[INHERITED]` owner says controller committed 804f7e34; host MCP 6090 passed / 0 failed, both W4 selectors ran; oracle 164 passed; authoritative mutation 472/472 |
| Targets provenance policy | done | `[MEASURED]` plan global constraints, Task 6 replay, and Cross-build SARIF scope carry dated 2026-10-16 owner extension |
| CI parity paths | done | `[MEASURED]` exact src/cpg/build.rs and src/cpg.rs routes added; unrelated src/cpg/query.rs remains false |
| Routing regression control | done | `[MEASURED]` base filter fails each positive row independently; routing-red-controls.log; suffix and unrelated-file negatives pass |
| Routing gates | done | `[MEASURED]` classify-changes.log: 35 passed; dfg-oracle-paths.log: 15 passed, including per-path .bak edge controls |
| Formatting gate | done | `[MEASURED]` env CARGO_BUILD_JOBS=4 cargo fmt --check passed; fmt.log |
| Comparison rejection checks | done | `[MEASURED]` normalizer-checks.log: missing/wrong/unexpected/duplicate provenance rejected; unrelated value/whitespace retained |
| Executable replay | done | `[MEASURED]` summary.json: 168 matrix invocations, 84/84 comparisons (21/21 targets, 42/42 identity attestations; 63 other pairs); fixture edge dump and two same-binary raw targets controls pass; 4 additional control invocations |
| Rust suites in this follow-up | done | `[MEASURED]` full default 5472/0, 7 ignored, 29 summaries; environment-limited MCP 6023/0, 7 ignored, 67 filtered, 31 summaries; both W4 selectors ran in each; exact commands/exclusions in root VERIFICATION.md |

All short log paths above are relative to `../dfgB-scratch/owner-decisions-2026-10-16/` unless `logs/` is specified (relative to scratch).

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Plan and prior handoff | Only SARIF provenance may differ; targets ruling pending | `[INHERITED]` explicit owner decision extends rule to exactly targets /producer/build_identity, checked against the same binary receipt |
| Prior host gate status | Full MCP and authoritative mutations still pending | `[INHERITED]` owner reports host MCP 6090/0, oracle 164 and authoritative gate 472/472 at 804f7e34; not independently rerun here |
| Historical scratch summaries | Prior targets gate failed under old contract | `[MEASURED]` follow-up status supersedes current pending-ruling assertions; old raw outputs and failure logs remain historical evidence |
| Memory | None changed | No owner request to write memory |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Replay | done | Read comparison.log, invocations.json, summary.json and targets-byte-audit.log | None | owner-decisions-2026-10-16/replay.py |
| 2 | Full-suite verification | done | Read root VERIFICATION.md and full-suite-totals.json; no failures observed | None | owner-decisions-2026-10-16/full-suite-runner.log |
| 3 | Controller commit | pending | Review diff and snapshot, stage exact four tracked paths; preserve ignored root report in scratch | Controller action | perf/dfg-views |

## 5. Invariants and traps — do not do these

- Within one binary compare every byte. Across binaries validate provenance before replacing only the three SARIF fields or one targets field in comparison copies.
- Preserve raw outputs and receipts; never remove production metadata or rebaseline goldens.
- The replay uses retained PR B binaries, with SHA-256 and saved build-output attestation; it does not claim freshly rebuilt 804f7e34 binaries.
- `nice` priority setting was denied in the predecessor run; owner authorized env CARGO_BUILD_JOBS=4 fallback. One cargo process at a time, logs to files.
- No installs/network/browser, .git writes, Rust production change, or real-corpus acceptance measurement in this follow-up.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| HEAD | `804f7e342f59163ebb2d2fc2cf28f8fbe62e3044` |
| Branch | `perf/dfg-views` |
| Evidence | `../dfgB-scratch/owner-decisions-2026-10-16` |
| Binary receipts | `../dfgB-scratch/binary-receipts.json` |
| Parent binary SHA-256 | `c4512fe1e43f808bf5154835f19613c182e14bd52a6001dae27f90616fb405c4` |
| Candidate binary SHA-256 | `825fd4dc3bf5b98d57789884086696b776932d06577f8370499c5ae76cf3d90e` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "Replay differs only in validated owner-authorized provenance fields" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: ../dfgB-scratch/owner-decisions-2026-10-16/comparison.log and targets-byte-audit.log

**Questions the owner owes an answer to:** None; both owner decisions are explicit. Controller retains commit/publication authority.
