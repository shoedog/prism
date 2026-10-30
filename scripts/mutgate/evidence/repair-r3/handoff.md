# Handoff — PR 340 mutation gate repair round 3

**Written:** 2026-10-03T19:48:41.217322+00:00 · **By:** /root repairer · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-mutation-gate · tooling/mutation-gate · **Measured state:** `[MEASURED]` HEAD 781511dc64c12eee81af1fa79bedce646d4fcdd2 · Tree DIRTY · Probe git status/rev-parse/diff · Output final-status.txt, production-src.diff, certification-inputs.json
**Predecessor:** controller-committed r2; confirm-sol61.md exposed open-class observer false kills.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by worker; [MEASURED] claims rerun here; [INHERITED] supplied claims distinguished.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — [INHERITED] user assigns this repairer; no subagents — RESOLVED.
**(b) Custody exposure** — [MEASURED] uncommitted changes and small evidence, snapshot /private/tmp/mutgate-repair-r3-final.tar.gz — RESOLVED by snapshot; controller commit pending.
**(c) In flight / irreversible** — [MEASURED] all certification processes completed; worker trees cleaned; owned target removal recorded in cleanup.json — RESOLVED.
**(d) Authorization granted but not exercised** — user: "No git writes." Controller owns committing; no push/merge by repairer.

## 1. Resume order

1. Run python3 scripts/mutgate/evidence/repair-r3/compare.py (seconds, no build/network); expect 761 hashes matched, 93 IDs, zero differences, 5145 passed/0 failed/1 ignored.
2. Review final-diff.patch, mutgate.py, tests, fixtures, README and CLAUDE subsection with the receipts.
3. Controller may commit using commit-message.txt after review. Warm artifacts were removed; new execution requires a rebuild.

**STOP conditions:** never open F; no git writes by repairer; src diff against origin/main must remain empty; no process-listing/time-l retries; two-cycle cap extended once for a bounded fix, no further implementation extension planned.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Text authority / advisory scope | done | [MEASURED] mutgate.py, README, CLAUDE; isolated trees/targets, original preflight per worker |
| Python regressions | done | [MEASURED] unit-cycle3.log: 56 passed, six compiled indirect-location fixtures and worker compile/test negatives |
| Full lane certification | done | [MEASURED] jobs1/2/4 summaries: each 93/93 admissible text kills; wall 471.46/277.06/202.87 seconds |
| Scoped certification | done | [MEASURED] synthetic three-ID fn scope 37.20s; actual empty diff 2.27s, zero IDs, both advisory |
| Full Cargo suite | done | [MEASURED] suite.log/totals: 5145 passed, 0 failed, 1 existing ignored, all features/no-fail-fast, 213.71s |
| Resource metrics | done | [MEASURED] per-run resources: gate peak du blocks 9.35 GiB, peak child-process RSS 1.94 GiB; definitions/exclusions in README |
| Provenance | done | [MEASURED] compare.py/comparison.json: 761 input hashes, 747 unchanged r2 production/test/build hashes, unchanged lane, zero ID differences |
| Custody / cleanup | done | [MEASURED] final snapshot and cleanup.json; only run-owned target removed |
| Controller commit | pending | [INHERITED] repairer prohibited from git writes; commit-message.txt supplied |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| README / CLAUDE mutation section | Full schema authority | [MEASURED] full text gate, advisory scoped contract, scoped authority option, measured target |
| before-worker-preflight receipts | Initial candidate certified as sound | [MEASURED] superseded by worker-context counterexample and final three timings; retained as history |
| Scratch suite failure | Possible repair regression | [MEASURED] identical r2 Rust control failed same 20; normal-root suite green; entrypoint symlink mismatch documented |
| r2 handoff/evidence | Lexical protections suffice | [INHERITED] confirmation review supersedes; historical receipts retained |
| Memory | None | No update requested |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Controller review/commit | pending | Review diff/evidence and commit-message.txt | Controller authority | PR 340, base 781511dc |

## 5. Invariants and traps — do not do these

- Never open F; no production src edits or git writes.
- Shared baseline alone cannot validate a relocated worker: private original preflight is required.
- Scope is local, without dependency closure; actual src diff here selects zero.
- Normal project suite must use the checkout manifest; scratch symlink changes native worker entrypoint identity.
- RSS is maximum waited-child process RSS, not aggregate; du may count APFS extents repeatedly.
- Clone mutable Cargo targets, never hard-link them; exclude mutgate from seed recursion.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base | `781511dc64c12eee81af1fa79bedce646d4fcdd2` |
| Evidence | `scripts/mutgate/evidence/repair-r3` |
| Snapshot | `/private/tmp/mutgate-repair-r3-final.tar.gz` |
| Suggested commit | `fix(mutgate): isolate authoritative text workers and label scoped advice` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "The authoritative 93-ID text gate meets the five-minute warm bound at four workers, with scoped schema results advisory" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: comparison.json, jobs4-resources.json, unit-cycle3.log, suite-totals.json

**Questions the owner owes an answer to:** None. Cold/CI timing, repeated flakiness, independent review, aggregate RSS, unique physical storage, ignored reserved test and Tier-A are not verified.
