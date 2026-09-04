# Handoff — JS/TS typed-parameter and new-constructor receiver recovery

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-js-ts-typed-new-receiver-recovery` · `js-ts-typed-new-receiver-recovery-owner`
**Exact base:** `deca1669947cd42d94d358ae80cb13cde0982750`

## 0. Current verdict

**IMPLEMENTED; TWO-ROUND REVIEW CLOSED; ALL RUNNABLE GATES GREEN.** This is roadmap item 4 and depends on, but remains distinct from, the verified lexical-binding prerequisite. Publication is pending.

## 1. Authority boundary

Recover only bare TS/TSX typed parameters and direct bare JS/TS/TSX `new Foo()` locals. Exact resolution requires one occurrence-clean module-scope class in the caller's file and a direct non-static method on that class. Unsupported/ambiguous evidence is materialized residue. Constructor owners must themselves resolve unshadowed to that module class.

## 2. Hypothesis-probe-result log

- The classifier and AST evidence gates exclude JavaScript, TypeScript, and TSX; `constructor_type` has no `new_expression` arm. Call-site construction stores the classifier output directly, ruling out later discard as the cause.
- Generic bare-owner lookup cannot prove imported/external/interface `Foo` identity. Caller-file `clean_class_spans` plus direct-method lookup is the required safety membrane.
- Review round 1 fixed three WRONG mechanisms: typed-parameter reassignment retained stale types; locally shadowed constructor identifiers could bind to the module class; recovered instance receivers could target static-only methods. It also fixed the closer-block-shadow mutation SMELL by carrying binding-scope identity.
- Review round 2 reached the declared cap with one closed WRONG: array/object-pattern reassignment escaped the identifier-only mutation detector. The bounded fix reuses binding-pattern name extraction; no recurring or open-class finding remained.
- The first full default suite exposed one stale pre-item-4 integration expectation: direct JavaScript `new Foo()` was still required not to recover. The same test passed on exact base `deca166` in the same environment, while the candidate's exact `ConstructorLocal` result is the intended new contract. The test now distinguishes that positive from the still-negative bare-factory residue.
- Tier-A quick exited 2 on the candidate. Its produced artifact completed with oracle/SUT error rates `0.0`, no oracle quiescence failure, and one invalid reason: candidate corpus SHA `4663a8026f93` differed from pinned `20c8490591a3`. An immediate release rebuild plus quick run on exact base `deca166` reproduced the same sole invalidation and the same compact pinned/M3 outcome signature, ruling out this slice as the cause. Generated reports, runs, and worktree-specific snapshots were removed after inspection.
- LSP semantic navigation is unavailable in this environment; targeted references, AST grammar schemas, compiler checks, and non-vacuous tests are the fallback. This remains a verification exclusion.

## 3. Verification state

- Committed RED evidence remains at `3b8cbb1`: JavaScript `new`, TypeScript typed-parameter, R3b-collision, TSX typed-parameter, CPG behavior, navigation behavior, and the 59/28 version pins failed before production edits.
- Post-review focused GREEN: JS receiver module 7/7, TS receiver module 15/15, expanded TSX recovery 1/1, CPG v59 behavior 1/1, nav behavior/pin 2/2. Full language targets: JS 68/68, TS 48/48, TSX 49/49.
- Static gates: `cargo fmt --all -- --check`, `git diff --check`, `cargo check --all-targets --features mcp`, and configured `cargo clippy --all-targets --features mcp -- -W clippy::all` all exited 0. Clippy retained only the repository's existing warning inventory.
- Full default suite: 3,601 passed, 0 failed, 1 ignored across 28 binaries (3,602 total). Full `mcp` suite: 3,787 passed, 0 failed, 1 ignored across 30 binaries (3,788 total).
- Accuracy harness: release build green; Tier-A matrix 104/104 green; immediate second release build green. Tier-A quick is not claimable as green because the committed baseline pins `20c8490591a3`; the same-environment base control proves the exit-2 condition is inherited corpus-SHA drift rather than a candidate regression.
- Stable implementation commits: `7101ed3` main recovery, `4663a80` fail-closed pattern-write repair, and `5d4b44f` full-suite integration-contract alignment.

## 4. Custody

- Branch/worktree: `js-ts-typed-new-receiver-recovery-owner` at `/private/tmp/slicing-js-ts-typed-new-receiver-recovery`.
- Exact base is verified prerequisite commit `deca166`, itself based on PR #230 merge `5051918f`.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Implementation was explicitly authorized on 2026-09-04. Publication of the prerequisite and this successor PR remains pending and is not assumed by this handoff.
