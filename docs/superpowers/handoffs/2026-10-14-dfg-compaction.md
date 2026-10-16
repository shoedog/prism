# Handoff — DFG PR B (#413) round-1 review fold

**Written:** 2026-10-16 · **By:** Codex · **Provider:** codex
**Workspace:** impl-dfgB · perf/dfg-views · **Measured state:** `[MEASURED]` HEAD 782716812dbed240f5a6854a1d69aadfa6d9556b · Tree DIRTY · Probe git status/diff and gates · Output ../dfgB-scratch/review-r1-fold/
**Predecessor:** controller's round-1 fold dispatch; previous owner-decision handoff preserved as review-r1-fold/before-handoff.md.
**Truth ordering:** measured live state > explicit owner authority within scope > this handoff for current operational state > historical handoffs and summaries. Conflicts stay OPEN.
**Provenance:** written live by worker. `[MEASURED]` claims rerun this turn; `[INHERITED]` claims supplied and named.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner assigns review fold; controller owns commits — RESOLVED by dispatch.
**(b) Custody exposure** — `[MEASURED]` eleven tracked edits plus locally ignored root VERIFICATION.md; source-final.tar.gz and fold.patch retain source/report — RESOLVED for worker handoff; controller must preserve evidence and commit.
**(c) In flight / irreversible** — `[MEASURED]` sequential gates completed; mutant reverted; no worker Cargo process remains; no commit/push attempted — RESOLVED.
**(d) Authorization granted but not exercised** — “Fold these small items; leave changes uncommitted for the controller.” No production, merge or publication authority.

## 1. Resume order

1. Read root VERIFICATION.md and ../dfgB-scratch/review-r1-fold/gate-results.json; inspect comparator-mutant-red.log for the requested RED.
2. Review git diff --stat and git diff --check; stage the eleven tracked paths only if accepting the fold. Root VERIFICATION.md remains locally ignored and is copied into the source/evidence snapshot.
3. Preserve the evidence directory and source-final.tar.gz, then controller commits. Run the 67 excluded listener tests on a socket-capable host if host verification of this fold is required.

**STOP conditions:** production edits, unrelated failures, unexpected source drift, network/Git writes by worker. Cap two verification/repair rounds; one bounded compile correction, no behavioral failure repairs or review extension.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Reviews | done | `[INHERITED]` owner supplies astra/Opus round-1 APPROVE; no WRONG findings; no fresh external review dispatched |
| Real comparator negative coverage | done | `[MEASURED]` new umbrella selector passes unmutated; cpg_bytes(a) mutant fails outer corruption-rejection assertion; restored umbrella 165/0/1 ignored |
| S2–S6 | done | `[MEASURED]` label snapshots hoisted, debug print removed, save outcome asserted, docs corrected, double negation removed |
| S7 | done | `[MEASURED]` cfg(test) edge_snapshot shared by optional tests; all 101 assertions retained; 1322.5 -> 1294.5 weighted lines |
| S1 | parked | `[MEASURED]` Task 7 measurement checkbox / PR C note added; no query implementation or measurement folded |
| Targeted tests | done | `[MEASURED]` unit 40/0; label-store 5/0; C algorithm 21/0; nine exact-caller tests pass in full default suite |
| Full default suite | done | `[MEASURED]` 5472 passed / 0 failed / 7 ignored / 0 filtered; 29 target/doc summaries; full-default.log |
| Runnable MCP suite | done | `[MEASURED]` 6023 passed / 0 failed / 7 ignored / 67 filtered; 31 summaries; exact exclusions and inherited reasons retained |
| Stop-hook report | done | `[MEASURED]` root VERIFICATION.md now includes inline exact commands, Verified / Not verified headings and explicit environment-limited Excluded list; no Rust edits or new Cargo run needed |
| Format and sizes | done | `[MEASURED]` fmt.log, file-sizes.log, file-size-ledger.json; all touched files below 2000, existing large optional test shrinks |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Release note | Empty adjacency rows implied | `[MEASURED]` empty existing rows belong only to defs/uses |
| Plan | Query-row iterator cost lacked explicit timing follow-up | `[MEASURED]` Task 7 now records Prometheus chop/taint/provenance timing and PR C accessor options |
| Previous handoff/root report | Previous owner-decision turn at 804f7e34 | Historical originals retained; this handoff/report supersede current status at 78271681 plus uncommitted fold |
| Memory | None | No memory update requested or performed |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Controller commit | pending | Review/stage/commit eleven tracked paths and retain ignored report | Controller action | perf/dfg-views |
| 2 | Host listener coverage | pending | Run exact 67 selectors from mcp-exclusions.txt | Sandbox cannot bind Unix listeners | mcp-exclusions.json |
| 3 | S1 query timing | parked | Execute Task 7 checkbox when measuring compact candidate | Later PR C/E acceptance | plan Task 7 |

## 5. Invariants and traps — do not do these

- No production change: Rust edits are unit/integration tests and cfg(test) edge_snapshot only.
- The actual cache helper must expect cpg_bytes(cold), never cpg_bytes(a); requested mutant was reverted.
- nice was denied in the predecessor; use owner-authorized env CARGO_BUILD_JOBS=4 fallback, offline, one Cargo process at a time.
- Full suites use RUST_MIN_STACK=16777216 and --test-threads=4, matching prior local full-suite environment; targeted gates use ordinary stack/thread defaults.
- The grouped cpg::exact_caller_read_tests filter selected none; its correct namespace is cpg::build::exact_caller_read_tests. Full-suite output proves all nine ran; do not count the wrong filter as evidence.
- Preserve exact listener exclusions; socketpair-only tests remain included. No rebaseline or ignored-test change.
- No Tier-A trigger: only CPG unit tests changed, not construction/resolution/navigation. No fresh Tier-A, authoritative mutation gate, executable replay or real-corpus timing claimed.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| HEAD | 782716812dbed240f5a6854a1d69aadfa6d9556b |
| Branch | perf/dfg-views |
| Evidence | ../dfgB-scratch/review-r1-fold |
| New test | outputs::cache_comparator_rejects_identical_decoded_counter_corruption |
| Retained unit | data_flow::tests::outputs::decoded_corruption_is_rejected_against_cold_reference |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "The real umbrella comparator negative control kills the requested mutant; requested gates and runnable suites pass without production changes" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: review-r1-fold/comparator-mutant-red.log, gate-results.json, hypothesis-log.md and fold.patch

**Questions the owner owes an answer to:** None. Controller retains commit and publication authority; host listener coverage remains explicitly excluded.
