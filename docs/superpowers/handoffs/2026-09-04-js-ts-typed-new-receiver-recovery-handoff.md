# Handoff — JS/TS typed-parameter and new-constructor receiver recovery

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-js-ts-typed-new-receiver-recovery` · `js-ts-typed-new-receiver-recovery-owner`
**Exact base:** PR #231 merge `6771d530c02ab7719547d580b428188db6401b2f`

## 0. Current verdict

**IMPLEMENTED; TWO-ROUND REVIEW CLOSED; ALL CLAIMABLE LOCAL GATES GREEN; PR #232 OPEN.** This is roadmap item 4 and depends on, but remains distinct from, the merged lexical-binding prerequisite. Merge remains conditional on every hosted check passing.

## 1. Authority boundary

Recover only bare TS/TSX typed parameters and direct bare JS/TS/TSX `new Foo()` locals. Exact resolution requires one occurrence-clean module-scope class in the caller's file and a direct non-static method on that class. Unsupported/ambiguous evidence is materialized residue. Constructor owners must themselves resolve unshadowed to that module class.

## 2. Hypothesis-probe-result log

- The classifier and AST evidence gates exclude JavaScript, TypeScript, and TSX; `constructor_type` has no `new_expression` arm. Call-site construction stores the classifier output directly, ruling out later discard as the cause.
- Generic bare-owner lookup cannot prove imported/external/interface `Foo` identity. Caller-file `clean_class_spans` plus direct-method lookup is the required safety membrane.
- Review round 1 fixed three WRONG mechanisms: typed-parameter reassignment retained stale types; locally shadowed constructor identifiers could bind to the module class; recovered instance receivers could target static-only methods. It also fixed the closer-block-shadow mutation SMELL by carrying binding-scope identity.
- Review round 2 reached the declared cap with one closed WRONG: array/object-pattern reassignment escaped the identifier-only mutation detector. The bounded fix reuses binding-pattern name extraction; no recurring or open-class finding remained.
- The first full default suite exposed one stale pre-item-4 integration expectation: direct JavaScript `new Foo()` was still required not to recover. The same test passed on exact base `deca166` in the same environment, while the candidate's exact `ConstructorLocal` result is the intended new contract. The test now distinguishes that positive from the still-negative bare-factory residue.
- Rebase onto PR #231 merge `6771d530` was source-conflict-free; only the roadmap and handoff required reconciliation from prerequisite-pending to prerequisite-merged custody.
- Tier-A quick exited 2 on rebased candidate `7e97637106e2`. Its produced artifact completed with oracle/SUT error rates `0.0`, no oracle quiescence failure, a clean corpus, and exactly one invalid reason: corpus SHA `7e97637106e2` differed from pinned `20c8490591a3`. The report therefore supports only inherited pin drift, not a claim that quick is green. Generated reports and the worktree-specific snapshot were removed after inspection.
- LSP semantic navigation is unavailable in this environment; targeted references, AST grammar schemas, compiler checks, and non-vacuous tests are the fallback. This remains a verification exclusion.

## 3. Verification state

- Committed RED evidence remains at rewritten commit `2dd9d78`: JavaScript `new`, TypeScript typed-parameter, R3b-collision, TSX typed-parameter, CPG behavior, navigation behavior, and the 59/28 version pins failed before production edits.
- Post-review focused GREEN: JS receiver module 7/7, TS receiver module 15/15, expanded TSX recovery 1/1, CPG v59 behavior 1/1, nav behavior/pin 2/2. Full language targets: JS 68/68, TS 48/48, TSX 49/49.
- Static gates: `cargo fmt --all -- --check`, `git diff --check`, `cargo check --all-targets --features mcp`, and configured `cargo clippy --all-targets --features mcp -- -W clippy::all` all exited 0. Clippy retained only the repository's existing warning inventory.
- Full default suite: 3,682 passed, 0 failed, 1 ignored across 28 binaries (3,683 total). Full `mcp` suite: 3,868 passed, 0 failed, 1 ignored across 30 binaries (3,869 total).
- Accuracy harness: release build green; Tier-A matrix 104/104 green; immediate second release build green. Tier-A quick is not claimable as green because the committed baseline pins `20c8490591a3`; its report has zero oracle/SUT errors and only deterministic corpus-SHA drift.
- Stable rewritten implementation commits: `98bbb7b` main recovery, `cc562aa` fail-closed pattern-write repair, and `843cfe5` full-suite integration-contract alignment.

## 4. Custody

- Branch/worktree: `js-ts-typed-new-receiver-recovery-owner` at `/private/tmp/slicing-js-ts-typed-new-receiver-recovery`.
- Exact base is merged lexical-binding prerequisite PR #231 at merge commit `6771d530`; its reviewed head was `6125e223`.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Implementation and publication were explicitly authorized on 2026-09-04. The prerequisite is merged; successor PR #232 is open and authorized to merge only after its hosted checks are green.
