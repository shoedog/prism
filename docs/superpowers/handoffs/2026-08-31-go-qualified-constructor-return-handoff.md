# Handoff — Go qualified constructor return ownership

**Refreshed:** 2026-08-31 · **By:** Codex `/root` · **Provider:** codex  
**Workspace:** `/private/tmp/slicing-fix-go-root-return-typed` · `fix-go-root-return-typed`  
**Exact local base:** `[MEASURED]` `c0de1d66be4beffa9270c995f224dd450a3162bd`; its tree is byte-identical to post-closeout server main `d4fd1da1e5e2ebe1052190b0e647ed6c0570e89d`.
**Verified implementation checkpoint:** `[MEASURED]` `f00a6a92fb1a2cb25b78f331c153f380b316540d`.

## 0. Verdict and authority

**Implementation and accuracy gates are verified locally; publication remains pending.** The full Tier-A refresh found a constructible Zap recall regression: in `zaptest/logger_test.go`, `log := NewLogger(ts)` is declared to return `*zap.Logger`, but calls such as `log.Debug()` and `log.Warn()` were dropped as `ExternalReceiver` instead of resolving to root-package `Logger` methods.

The bounded authority is declaration ownership: when an unshadowed same-package `NewX()` name heuristic and the function's exact declared return owner disagree, the declaration wins. Agreeing owners preserve the existing `ConstructorLocal` label. Unproven, conflicting, or externally unresolved declared owners fail closed. No other receiver-recovery form or same-scope-reuse behavior changes.

The steering carrier names an installed `handoff-template.md`, but the file is absent on this machine. This handoff follows the established adjacent lane shape.

## 1. Root cause and fix

`ParsedFile::constructor_type` provisionally strips `NewClient` to local type `Client` and labels it `ConstructorLocal`. `classify_go_receiver_expanded_with_partition` previously returned that heuristic before S1 could consult the declaration-backed return-type index. Prerequisite screening then either erased the unproven local owner (the Zap omission) or could resolve a same-named local decoy (a wrong target).

The fix lets S1 refine only an unshadowed, non-reuse `ConstructorLocal` call RHS:

- declaration owner differs from the heuristic owner: carry the declaration owner as `ReturnTyped`;
- declaration owner agrees: preserve `ConstructorLocal`;
- declaration selection is uncertain/conflicting: materialize a terminal drop;
- no declaration fact exists: preserve existing heuristic behavior.

## 2. RED/GREEN custody

The completed public fixture covers a root `zap.Logger`, a subpackage `q.Client`, a same-named local `Logger` decoy, a direct typed control, an internal `_test.go` caller, and an unbound `missing.Client` negative.

- Pre-change detached worktree: `/private/tmp/slicing-red-go-qualified-return` at exact `c0de1d6`, dirty only with the completed test overlay.
- RED: exact focused test exited `101`; `testClient` had `receiver_recovery: None` instead of `ReturnTyped`.
- GREEN: the same focused test passed on this branch. Root target is exactly `logger.go`; the missing provider remains `ExternalReceiver` with zero edges.

An earlier simpler RED on this branch before the production edit also failed with `receiver_recovery: None` for the root result. Invalid probes using a nonexistent Cargo target and two wrong CLI argument shapes were discarded without belief updates.

## 3. Verification

| Gate | Result |
|---|---|
| Focused final fixture | `1 passed, 0 failed` |
| Complete Go target | `281 passed, 0 failed` |
| Default full suite | `3,543 passed, 0 failed, 1 ignored` |
| MCP-feature full suite | `3,729 passed, 0 failed, 1 ignored` |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets --features mcp -- -W clippy::all` | exit `0`; repository warning population retained |
| Release build | pass at exact `f00a6a9` |
| Tier-A matrix-only | `104/104` capability rows `ok`, exit `0` |
| Tier-A quick | oracle/SUT errors `0.000/0.000`; matrix `104/104`; exit `2` only for `f00a6a9 != pinned 20c8490` |
| Zap no-cache production control | five root `logger.go` targets exact at score `1.0`, all `return_typed` |
| Targeted Zap Tier-A | valid; SUT errors `0.000`; exact Q-scoped callees `41/0/14`; zero exact FP |

The full-suite warnings are pre-existing unused/dead-code warnings in tests. Clippy's configured `-W` gate emitted the repository's existing warning population but exited successfully; no unrelated warning was edited or attributed to this change.

The quick run's only invalid reason is corpus/SUT pin drift. Its pre-existing `target-c-method` pinned item remains a flip candidate. The targeted Zap report is valid (`baseline_invalid: false`); compared with the identical-harness regressed `c0de1d6` report, exact caller sets are unchanged (`170/170`) and exact callee sets add only lines `43`, `44`, `45`, `46`, and `49` in `zaptest/logger_test.go`. The older `fb81481d` base had three of those exact and two candidate; the fix makes all five oracle-listed sites exact.

## 4. Debugging log

| Hypothesis | Discriminator | Result |
|---|---|---|
| Root package import-path key is missing | Direct `*zap.Logger` typed receiver should also fail | falsified; direct typed root owner resolves exactly |
| Any imported qualified result fails because of root identity | Subpackage `*q.Client` control should pass | falsified; it failed identically |
| Return fact extraction/selection drops `*q.Client` | Temporary selection diagnostic should fire | falsified; selection was never reached |
| `NewX` constructor heuristic preempts S1 | Base classifier yields `ConstructorLocal(X)` and screening erases/rebinds it | supported by exact control flow and RED/GREEN behavior |

Alternative causes ruled out: oracle drift (same-harness exact-base Tier-A control restored the old Zap tuple), stale navigation cache (`--no-cache` reproduced all five drops), test exclusion (ordinary internal Go test file), and root import identity (typed root control).

## 5. Review and convergence

Declared implementation/review cap: `2` rounds.

- Round 1: one closed documentation mismatch—the classifier contract still said S1 only handled calls the heuristic could not type. Updated to include declaration correction of `NewX` owners.
- Round 2: settled with zero open findings. Zap exact-callee locality changed only the five intended sites; exact caller sets were identical; all five sites are in the oracle set; no exact FP appeared.
- No open `WRONG` or in-scope `SMELL` remains at the declared cap.

## 6. Next actions

1. Commit this verified handoff refresh.
2. Publish a PR from the exact verified tree and merge after required non-coverage checks are green under the owner's standing authorization.
3. Reconcile this handoff with the live PR and merge SHA/tree in the publication commit.
4. Preserve the full-refresh evidence in `/private/tmp/slicing-post221-closeout`; do not rebaseline the invalid 2026-08-31 Prism anchor.

## 7. Custody and exclusions

- Active implementation branch/worktree: `fix-go-root-return-typed` at `/private/tmp/slicing-fix-go-root-return-typed`.
- Verified implementation checkpoint: `f00a6a92fb1a2cb25b78f331c153f380b316540d`.
- Test-only RED control: detached `c0de1d6` at `/private/tmp/slicing-red-go-qualified-return`.
- Tier-A base control: detached `fb81481dafa7398dd8b539b99e137269567f2bb3` at `/private/tmp/slicing-tiera-base-fb81481d`.
- Full-refresh evidence worktree: `/private/tmp/slicing-post221-closeout`, local `c0de1d6`, with generated reports/snapshot untracked and preserved.
- Primary `/Users/wesleyjinks/code/slicing` remains untouched, including user-owned `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json`.
- Full multi-corpus Tier-A was already run and adjudicated before this fix; it is not repeated automatically. Coverage is not awaited by owner direction.
- Generated Prism quick and targeted Zap report/snapshot files are reproducible and intentionally untracked; the adjudicated results above are the durable custody record, not a baseline update.
