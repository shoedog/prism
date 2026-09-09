# Handoff — ordered callable PR merges

## Completion supersedes the snapshots below

Owner reported all merged. Fresh Git fetch for the next approved design slice
confirmed PR281–293 merge history on main, ending at `7040ceb6` (PR293), with the
same final tree as the retained local stack. No remaining merge action is pending.
This verifies merged Git state, not retrospective CI success or stack-controller
internals. Continue from the [executable-owner design handoff](2026-09-09-callable-executable-owner-proof.md).

## Owner stack takeover — pre-transition snapshot

Recorded 2026-09-08T16:42Z. The owner created stack294 for286–293 and asked the
agent to watch286, verify automatic287 retarget/rebase and CI startup after286
merges, then leave287–293 for the owner to merge in the stack. This supersedes
the earlier agent-through293/manual-retarget resume instructions below.

Agent scope: finish286 under the four non-coverage gates and tested-tree check;
observe287 for up to10 minutes after merge without manually changing its base,
head or open state. Check actual head/main ancestry and CI run association; a
started run is not a green run. Stop and report if the expected transition does
not happen. No further agent merge of287–293 is authorized by this takeover.

At this snapshot286 is OPEN on main, head185163cf, CI34252139675 running;
287 is OPEN on `feat/callable-config-provenance`, head6e396c73, with no checks.
The owner's stack identifier294 is not exposed as a GitHub pull request by the
queried API; verify observable286/287 behavior directly, without claiming access
to the stack controller. Later operational truth is the live
[PR286](https://github.com/shoedog/prism/pull/286) /
[PR287](https://github.com/shoedog/prism/pull/287) state and the agent's transition
receipt. Everything below is the earlier timestamped merge snapshot, not a
claim that its pending states persist after the owner stack advances.

**Written:** 2026-09-08T16:37Z · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · docs/callable-production-authority-contract · **Measured state:** `[MEASURED]` PR281–285 merged in order, latest main0fe868d9 (owner merged285). PR286 already targeted main at refresh; reopened once for missing CI34252139675, running on unchanged185163cf. Owner explicitly waived waiting for coverage. Probe: gh PR/workflow queries; receipts in `/private/tmp/prism-ordered-merges-stfAUR`.
**Predecessor:** approved ten-increment sequence, published PR281–293.
**Truth ordering:** measured live state > explicit owner authority within scope > this current merge handoff > historical publication snapshots.
**Provenance:** written live from merge/API output; earlier implementation gate claims remain bound to their original checked HEADs.

## 0. Gating facts — settle these before starting anything below

(a) Ownership — RESOLVED: primary owns merging; no implementation work is in scope.
(b) Custody — RESOLVED: branch history retained; progress commits are published on final PR293. No branch deletion/reset/force push.
(c) In flight — RESOLVED: PR286 CI and nonblocking PR285 coverage; no local implementation test process.
(d) Authority — owner explicitly requested "proceed to merge in order", then "We can merge withoit waiting for coverage" after personally merging285. Pending coverage no longer delays merges; the other four checks and tested-tree verification remain required. No code-change or admin-bypass authority is added.

## 1. Resume order

1. Read current gh PR state, expected head, main base and latest CI before any mutation. Do not repeat a merge based on stale local status.
2. Wait for Test Suite, Clippy Lint, Format Check and Language Coverage Matrix on286; verify each required job's actual checkout tree equals a fresh merge of its head into current main, then merge with `--merge --match-head-commit`; verify merged state/commit/tree. Repeat through293. Record coverage separately without waiting for it.
3. Retarget only the next successor after its predecessor merges. Retarget alone did not trigger CI for282; one close/reopen produced the expected main-targeted run without changing source.
4. Before293 CI, finish all status/documentation updates on its branch and freeze the head. After merging, verify the remote final tree and preserve all branches/evidence.

STOP on a known failed CI job, conflict, unexpected head, requested changes, missing required checks or code-fix requirement. Pending coverage alone is not a stop. No admin bypass or auto-merge enrollment.

## 2. State ledger

| PR | State | Evidence / correction |
|---|---|---|
| 281 | done | merged172975a322b76256487f8fb630b401d3f1b594ea at2026-09-08T14:31:32Z; five SUCCESS checks on c57c3549 |
| 282 | done | merged91baaf8a3bc06aec52be69bde0689804dde4f543 at2026-09-08T15:08:10Z; five SUCCESS jobs in34238928555; all actual job trees and final merge tree0b98026e equal fresh-main preflight |
| 283 | done | merged5a4abb4a7e8da3b93923fd3cbf1c41c172f399d9 at2026-09-08T15:32:25Z; five SUCCESS jobs in34242873595; all actual job trees and final merge tree11c682d7 equal fresh-main preflight |
| 284 | done | mergedc1091d18d9bb1c16dcf7e8cc52174bd0c385590a at2026-09-08T16:13:37Z; five SUCCESS jobs in34245471664; all actual job trees and final merge treec225b137 equal fresh-main preflight |
| 285 | done by owner | merged0fe868d9f33b6b702357dc131be506a3ab0b431a at2026-09-08T16:33:39Z; four checks SUCCESS in34249817791 at refresh, coverage pending; landed tree101c6977 and parentsc1091d18/78be9044 verified. No agent pre-merge tree check is claimed |
| 286 | pending | main target; head185163cfba0fd759c50fab72a1e82b9b4d1c2574; CI34252139675 running |
| 287–293 | pending | original predecessor-base stack; do not merge ahead |

## 3. Corrections to standing documents and memory

The earlier sequence handoff required an owner merge decision. That decision has now arrived; its implementation-closeout claims remain historical. The later explicit coverage-wait waiver supersedes this lane's previous five-check waiting rule. Current merge status is this handoff and fresh GitHub state. No memory edits.

The PR285 monitor stopped on its PR/run association assertion, not a CI failure: the owner merged285 at16:33:39Z; the refreshed run's `pull_requests` array is empty while four jobs are successful and coverage runs. One read-only restart reproduced that assertion; no CI rerun or source fix occurred. New monitor snapshot names include a per-process timestamp to preserve prior receipts across restarts.

## 4. Open work

| Work | State | Next action |
|---|---|---|
| Ordered merges | pending | wait for286's four required jobs, then predecessor-first merge/retarget/check |
| Final status reconciliation | pending | finish before293 freezes; verify final merge externally afterward |

## 5. Invariants and traps — do not do these

- Preserve ancestor identity with merge commits, not squash/rebase.
- Main protection API returned404; lack of branch protection does not waive CI.
- No checks is not green. Require Test Suite, Clippy Lint, Format Check and Language Coverage Matrix all SUCCESS. Owner waived waiting for coverage; do not cancel it or misreport it as passed.
- An exit8 from `gh pr checks` meant pending jobs, not failed tests; inspect output.
- Retargeted282 had no run; reopening once created CI34238928555. Do not loop mutations blindly.
- Run metadata alone does not establish the checked-out merge tree. PR282's formatting job checked out a76a6f73 with an earlier feature-base parent; its tree0b98026e nevertheless exactly matched `git merge-tree --write-tree` of current main172975a3 and head559e80e9. Require that equality for every required successful job checkout and fresh main before merging; the reason for the earlier parent is not established.
- Keep public/private evidence separated and retain all remote/local branches.

## 6. Identifiers

Repository shoedog/prism. Starting final branch tip787dfe1a84652604ba5dcf8ca85ff8d596dd279a. Merge receipts `/private/tmp/prism-ordered-merges-stfAUR`. Implementation source remains producer32ae5bc4; CPG77/navigation45 unchanged.

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED for281–284 merge preconditions; SELF-PASS with remote CI-backed evidence. Exact head/main base, mergeability and all five jobs were checked before merging. PR282–284 additionally have all-five actual checkout tree equality and final merge-tree verification. Later merges remain gated, not preapproved as green.

**Questions the owner owes an answer to:** None while CI progresses.
