# Handoff — post-316 slice 1 exact nested-call ownership

**Written:** 2026-09-16T09:41:39Z · **By:** /root/implement_slice1 · **Provider:** codex
**Workspace:** prism · feat/post316-slice1-call-ownership · **Measured state:** `[MEASURED]` HEAD 07ed5ceb6098f2b5a21f5c05008a2c823a5617d7 · Tree DIRTY (verification evidence only) · Probe `git status --short` · Output `docs/eval/post316-slice1/source-manifest.md`
**Predecessor:** controller checkpoint `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` `/root/review_slice1` issued final acceptance APPROVE with 0 WRONG / 1 nonblocking SMELL and no source-change request — **RESOLVED 2026-09-16**
**(b) Custody exposure** — `[MEASURED]` completed Rust and non-Rust gate logs, reports and final receipts are an uncommitted evidence-only delta over checkpoint `07ed5ceb` in `/private/tmp/prism-post316-slice1`; ten source/test hashes remain frozen in the manifest — **OPEN until controller custody commit**
**(c) In flight / irreversible** — `[INHERITED]` all authorized gates and final evidence reconciliation completed; Tier-A quick is retained as INVALID, not retried. No push/PR/merge occurred — **RESOLVED 2026-09-16T09:41:39Z**
**(d) Authorization granted but not exercised** — “great - proceed to orchestrate this. minimize your own reads and writes, prioritize delegation and orchestration”

## 1. Resume order

1. Verify the ten source/test hashes in `docs/eval/post316-slice1/source-manifest.md`; stop if any differs.
2. Preserve `evidence/rust-final/` and `evidence/nonrust-final/`; their receipts bind every log/report to the frozen commit/tree and classify both setup-invalid attempts.
3. Read `docs/eval/post316-slice1/final-verification-receipt.md` and `evidence/reviewer-round2/FINAL-ACCEPTANCE.md`; Tier-A quick remains an explicit INVALID limitation and must not be represented as an accuracy pass or regression.
4. Commit the exact docs/evidence delta before switching this worktree to the separately dispatched slice-2 branch. Do not change slice-1 source or tests.

**STOP conditions:** source hash drift, open-class findings at review cap two, any request to redesign `FunctionId`, class phases, anonymous indexing, or receiver/parameter semantics.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Main fail-first matrix | done | `[MEASURED]` final test patch on unchanged base is 2 pass / 6 fail; candidate is 8 pass / 0 fail in `evidence/focused-green-round1-repair.log` |
| Query/manual ownership | done | `[MEASURED]` all eight routes pass across JS/TS/TSX assignment, initializer, call-argument, returned, computed-key callback, recovery and class-baseline forms |
| Identity collisions | done | `[MEASURED]` Unicode byte tuples remain raw-distinct; full/skeleton/subset refuse all colliding sites and retain one direct site |
| Navigation | done | `[MEASURED]` exact symbols/bytes/reasons/multiplicity candidate 1/0; same final test on unchanged base 0/1 |
| Cache invalidation | done | `[MEASURED]` simulated guard checks pass; genuine base-produced v93/v52 bytes are rejected/rebuilt to v94/v53 and cached endpoints equal uncached |
| Exact-16 reconciliation | done | `[MEASURED]` every original failure is mapped once in `docs/eval/post316-slice1/round1-repair-receipt.md`; focused affected modules are green |
| Full verification | done with disclosed limitation | `[MEASURED]` Rust default 4,518/0/1; MCP 4,711/0/1; widest audit 4,734/0/1; examples 32/0/0; Node 785/0/1; authority 40/0; Python 940 collected cases closed; Tier-A matrix 159/159 `ok`. Tier-A quick report is INVALID for corpus-pin drift plus 4/6 C-method oracle probes. |
| Independent review | accepted | `[INHERITED]` final evidence reconciliation APPROVE with 0 WRONG / 1 nonblocking performance SMELL and no source-change request; report, manifest and acceptance are under `evidence/reviewer-round2/` |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `docs/eval/receiver-closure/2026-09-14-nested-execution-owner-repair-baseline.json` | CPG cache 93 and navigation cache 52 are current | `[MEASURED]` candidate versions are 94 and 53; historical file remains baseline evidence and was not rewritten |
| inherited epoch test | epochs 1-3 retain outer and inner Exact callers | `[MEASURED]` candidate retains only inner; epochs 0/4 retain outer |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Final custody | pending | Commit exact final receipts, logs, reports and handoff delta | controller `.git` authority | final verification receipt |
| 2 | Tier-A baseline pin repair | outside slice | Resolve corpus pin/oracle completeness before seeking a clean quick accuracy verdict | separate owner authority | non-Rust receipt |

## 5. Invariants and traps — do not do these

- Never reuse the rvalue body-only predicate — own parameter defaults are part of the existing call inventory.
- Never let an unindexed callable borrow ancestor ownership — no `FunctionId` is still a callable boundary.
- Query byte ranges admit overlapping ancestor calls; containment and nearest execution owner are both required.
- Cargo accepts one test-name filter; the retained `cache-green-invalid-cli.log` is inadmissible, use the four separate cache logs.
- Do not claim class-phase correctness, occurrence repair, public accuracy gain, runtime authority, publication, CI, or merge.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base commit | `f5350044a18bf95f3deb51a41b4b09e9d585b134` |
| Base tree | `2e68dde9c1b587869a93acefa7923a1d337652a3` |
| Worktree | `/private/tmp/prism-post316-slice1` |
| Evidence root | `/private/tmp/prism-post316-slice1/docs/eval/post316-slice1/evidence` |
| Source manifest | `/private/tmp/prism-post316-slice1/docs/eval/post316-slice1/source-manifest.md` |
| Review cap | `2` |
| Environmental retry cap | `1 per failing gate class after diagnosis` |
| Final receipt | `/private/tmp/prism-post316-slice1/docs/eval/post316-slice1/final-verification-receipt.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: “epochs 1-3 retain only the exact inner caller while direct outer calls, own defaults, eager keys, class baseline, unrelated callers and non-JS inventories survive” · pass: INDEPENDENT APPROVE (0 WRONG / 1 nonblocking SMELL) · evidence tier: TEST-BACKED · records: `docs/eval/post316-slice1/evidence/focused-green-round1-repair.log`, `docs/eval/post316-slice1/evidence/reviewer-round2/REVIEW-round2.md`, and `docs/eval/post316-slice1/evidence/reviewer-round2/FINAL-ACCEPTANCE.md`

**Questions the owner owes an answer to:** None.
