# Handoff — ordered callable PR merges

**Written:** 2026-09-08T16:14Z · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · docs/callable-production-authority-contract · **Measured state:** `[MEASURED]` PR281–284 merged in order, latest mainc1091d18. PR285 retargeted to main and reopened once; CI34249817791 running on unchanged78be9044. Probe: gh PR/workflow queries; receipts in `/private/tmp/prism-ordered-merges-stfAUR`.
**Predecessor:** approved ten-increment sequence, published PR281–293.
**Truth ordering:** measured live state > explicit owner authority within scope > this current merge handoff > historical publication snapshots.
**Provenance:** written live from merge/API output; earlier implementation gate claims remain bound to their original checked HEADs.

## 0. Gating facts — settle these before starting anything below

(a) Ownership — RESOLVED: primary owns merging; no implementation work is in scope.
(b) Custody — RESOLVED: branch history retained; progress commits are published on final PR293. No branch deletion/reset/force push.
(c) In flight — RESOLVED: PR285 CI; no local implementation test process.
(d) Authority — owner explicitly requested "proceed to merge in order". This supersedes the earlier no-inferred-merge status, not code-change or CI-bypass limits.

## 1. Resume order

1. Read current gh PR state, expected head, main base and latest CI before any mutation. Do not repeat a merge based on stale local status.
2. Wait for all five CI jobs on285, verify each job's actual checkout tree equals a fresh merge of its head into current main, then merge with `--merge --match-head-commit`; verify merged state/commit/tree. Repeat through293.
3. Retarget only the next successor after its predecessor merges. Retarget alone did not trigger CI for282; one close/reopen produced the expected main-targeted run without changing source.
4. Before293 CI, finish all status/documentation updates on its branch and freeze the head. After merging, verify the remote final tree and preserve all branches/evidence.

STOP on failed CI, conflict, unexpected head, requested changes, missing checks or code-fix requirement. No admin bypass or auto-merge enrollment.

## 2. State ledger

| PR | State | Evidence / correction |
|---|---|---|
| 281 | done | merged172975a322b76256487f8fb630b401d3f1b594ea at2026-09-08T14:31:32Z; five SUCCESS checks on c57c3549 |
| 282 | done | merged91baaf8a3bc06aec52be69bde0689804dde4f543 at2026-09-08T15:08:10Z; five SUCCESS jobs in34238928555; all actual job trees and final merge tree0b98026e equal fresh-main preflight |
| 283 | done | merged5a4abb4a7e8da3b93923fd3cbf1c41c172f399d9 at2026-09-08T15:32:25Z; five SUCCESS jobs in34242873595; all actual job trees and final merge tree11c682d7 equal fresh-main preflight |
| 284 | done | mergedc1091d18d9bb1c16dcf7e8cc52174bd0c385590a at2026-09-08T16:13:37Z; five SUCCESS jobs in34245471664; all actual job trees and final merge treec225b137 equal fresh-main preflight |
| 285 | pending | main target; head78be9044afd10f993b20e905262c8d1188d2be76; CI34249817791 running |
| 286–293 | pending | original predecessor-base stack; do not merge ahead |

## 3. Corrections to standing documents and memory

The earlier sequence handoff required an owner merge decision. That decision has now arrived; its implementation-closeout claims remain historical. Current merge status is this handoff and fresh GitHub state. No memory edits.

## 4. Open work

| Work | State | Next action |
|---|---|---|
| Ordered merges | pending | wait for285 CI, then predecessor-first merge/retarget/check |
| Final status reconciliation | pending | finish before293 freezes; verify final merge externally afterward |

## 5. Invariants and traps — do not do these

- Preserve ancestor identity with merge commits, not squash/rebase.
- Main protection API returned404; lack of branch protection does not waive CI.
- No checks is not green. Require Test Suite, Clippy Lint, Format Check, Coverage and Language Coverage Matrix all SUCCESS.
- An exit8 from `gh pr checks` meant pending jobs, not failed tests; inspect output.
- Retargeted282 had no run; reopening once created CI34238928555. Do not loop mutations blindly.
- Run metadata alone does not establish the checked-out merge tree. PR282's formatting job checked out a76a6f73 with an earlier feature-base parent; its tree0b98026e nevertheless exactly matched `git merge-tree --write-tree` of current main172975a3 and head559e80e9. Require that equality for all five successful job checkouts and fresh main before merging; the reason for the earlier parent is not established.
- Keep public/private evidence separated and retain all remote/local branches.

## 6. Identifiers

Repository shoedog/prism. Starting final branch tip787dfe1a84652604ba5dcf8ca85ff8d596dd279a. Merge receipts `/private/tmp/prism-ordered-merges-stfAUR`. Implementation source remains producer32ae5bc4; CPG77/navigation45 unchanged.

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED for281–284 merge preconditions; SELF-PASS with remote CI-backed evidence. Exact head/main base, mergeability and all five jobs were checked before merging. PR282–284 additionally have all-five actual checkout tree equality and final merge-tree verification. Later merges remain gated, not preapproved as green.

**Questions the owner owes an answer to:** None while CI progresses.
