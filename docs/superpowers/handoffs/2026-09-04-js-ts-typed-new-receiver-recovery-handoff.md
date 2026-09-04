# Handoff — JS/TS typed-parameter and new-constructor receiver recovery

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-js-ts-typed-new-receiver-recovery` · `js-ts-typed-new-receiver-recovery-owner`
**Exact base:** `deca1669947cd42d94d358ae80cb13cde0982750`

## 0. Current verdict

**TWO-ROUND REVIEW CONVERGED; FULL VERIFICATION NEXT.** This is roadmap item 4 and depends on, but remains distinct from, the verified lexical-binding prerequisite.

## 1. Authority boundary

Recover only bare TS/TSX typed parameters and direct bare JS/TS/TSX `new Foo()` locals. Exact resolution requires one occurrence-clean module-scope class in the caller's file and a direct non-static method on that class. Unsupported/ambiguous evidence is materialized residue. Constructor owners must themselves resolve unshadowed to that module class.

## 2. Hypothesis-probe-result log

- The classifier and AST evidence gates exclude JavaScript, TypeScript, and TSX; `constructor_type` has no `new_expression` arm. Call-site construction stores the classifier output directly, ruling out later discard as the cause.
- Generic bare-owner lookup cannot prove imported/external/interface `Foo` identity. Caller-file `clean_class_spans` plus direct-method lookup is the required safety membrane.
- Review round 1 fixed three WRONG mechanisms: typed-parameter reassignment retained stale types; locally shadowed constructor identifiers could bind to the module class; recovered instance receivers could target static-only methods. It also fixed the closer-block-shadow mutation SMELL by carrying binding-scope identity.
- Review round 2 reached the declared cap with one closed WRONG: array/object-pattern reassignment escaped the identifier-only mutation detector. The bounded fix reuses binding-pattern name extraction; no recurring or open-class finding remained.
- LSP semantic navigation is unavailable in this environment; targeted references, AST grammar schemas, compiler checks, and non-vacuous tests are the fallback. This remains a verification exclusion.

## 3. Verification state

- Committed RED evidence remains at `3b8cbb1`: JavaScript `new`, TypeScript typed-parameter, R3b-collision, TSX typed-parameter, CPG behavior, navigation behavior, and the 59/28 version pins failed before production edits.
- Focused GREEN: JS receiver module 7/7, TS receiver module 15/15, expanded TSX recovery 1/1, CPG v59 behavior 1/1, nav behavior/pin 2/2. Complete pre-review language targets were JS 66/66, TS 47/47, TSX 49/49; they must be rerun after review fixes.
- Review-1 implementation is committed at `7101ed3`; the round-2 targeted fix is green in focused tests and is the next stable commit. Configured Clippy, full suites, release/Tier-A, final handoff, and publication are pending.

## 4. Custody

- Branch/worktree: `js-ts-typed-new-receiver-recovery-owner` at `/private/tmp/slicing-js-ts-typed-new-receiver-recovery`.
- Exact base is verified prerequisite commit `deca166`, itself based on PR #230 merge `5051918f`.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Implementation was explicitly authorized on 2026-09-04. Publication of this successor PR is not yet assumed by this handoff.
