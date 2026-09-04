# Handoff — JS/TS typed-parameter and new-constructor receiver recovery

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-js-ts-typed-new-receiver-recovery` · `js-ts-typed-new-receiver-recovery-owner`
**Exact base:** `deca1669947cd42d94d358ae80cb13cde0982750`

## 0. Current verdict

**INTENTIONAL RED ESTABLISHED; IMPLEMENTATION NEXT.** This is roadmap item 4 and depends on, but remains distinct from, the verified lexical-binding prerequisite.

## 1. Authority boundary

Recover only bare TS/TSX typed parameters and direct bare JS/TS/TSX `new Foo()` locals. Exact resolution requires one occurrence-clean module-scope class in the caller's file and a direct method on that class. Unsupported/ambiguous evidence is materialized residue.

## 2. Hypothesis-probe-result log

- The classifier and AST evidence gates exclude JavaScript, TypeScript, and TSX; `constructor_type` has no `new_expression` arm. Call-site construction stores the classifier output directly, ruling out later discard as the cause.
- Generic bare-owner lookup cannot prove imported/external/interface `Foo` identity. Caller-file `clean_class_spans` plus direct-method lookup is the required safety membrane.
- LSP semantic navigation is unavailable in this environment; targeted references, AST grammar schemas, compiler checks, and non-vacuous tests are the fallback. This remains a verification exclusion.

## 3. Verification state

- No production code changed yet. The exact-base JavaScript `new`, TypeScript typed-parameter, R3b-collision, TSX typed-parameter, CPG behavior, and navigation behavior positives all fail at missing receiver recovery (`receiver_type=None` or no recovered edge).
- CPG/navigation version pins fail exactly at production 58/27 versus required 59/28.
- Implementation, GREEN, two review rounds, full suites, and Tier-A are pending.

## 4. Custody

- Branch/worktree: `js-ts-typed-new-receiver-recovery-owner` at `/private/tmp/slicing-js-ts-typed-new-receiver-recovery`.
- Exact base is verified prerequisite commit `deca166`, itself based on PR #230 merge `5051918f`.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Implementation was explicitly authorized on 2026-09-04. Publication of this successor PR is not yet assumed by this handoff.
