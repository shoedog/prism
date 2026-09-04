# Handoff — Python module-qualified typed-receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-py-qualified-receiver` · `py-qualified-receiver-owner`
**Exact base:** `5e54d48381f329cae370557eeac35bc00ff7b801`

## 0. Current verdict

**IMPLEMENTED; Tier-A pending.** This next adjacent Fork-A production increment after PR #226 proves Python receiver ownership for exactly `import module [as alias]` plus `alias.Class`, while retaining direct-method and fail-closed boundaries.

## 1. Authority boundary

Exact requires one clean module-scope `ModuleImport` alias, one indexed module file, one clean direct class, and one non-ambiguous direct method. Duplicate, rebound, wildcard, local-shadow, inherited-only, re-export, and multi-hop forms are excluded. The existing imported-class proof-key mismatch guard will cover incremental authority transitions once qualified keys are added.

## 2. Verification state

- Source feasibility probe: qualified annotation text survives as `models.Client`; the missing mechanism is import-owner proof/routing, not AST retention.
- Alternative ruled out: no AST grammar extension is required.
- Exact-base qualified Python target: 4 selected, 1 passed and 3 failed. Failures were the expected missing Exact edge, missing subset Exact edge, and retained unproven local qualified type; the proof-barrier control passed.
- Exact-base structured-import REDs: 0/1 for unaliased dotted root binding (`models` observed, `pkg` expected) and 0/1 for clean module eligibility (`false` observed, `true` expected).
- Exact-base incremental target: 1 stable-authority control passed; 2 transition tests failed because qualified receiver state remained `Some("models.Client")` without class proof.
- Invalid probes discarded: the first Cargo commands named source files as test targets instead of the live aggregate targets.
- Focused GREEN after implementation and round-1 fixes: qualified behavior 5/5, qualified incremental parity 3/3, qualified stable proof-key unit 1/1, complete Python target 75/75, import-binding controls 58/58, and prior bare-import incremental controls 3/3.
- The two warnings emitted by the integration target are pre-existing unused-test-code warnings.
- Full default suite: 3,562 passed, 0 failed, 1 ignored across 28 summaries.
- Full `mcp` suite: 3,748 passed, 0 failed, 1 ignored across 30 summaries.
- `cargo check --all-targets` passed.
- Configured CI Clippy command passed with the repository warning population. Its short-form output identified one new warning at the proof-key loop; the bounded mechanical fix is included and will be rechecked before Tier-A.
- Format/diff checks and Tier-A evidence are pending after the gate fix.

## 3. Convergence

Declared review cap: two rounds.

- Round 1 WRONG: an arbitrary unimported two-component typed or constructor expression persisted a recovered type. Added RED and made every dotted Python type require imported-class proof.
- Round 1 WRONG: excluded `pkg.models.Client` multi-hop annotation persisted recovered state. Added the multi-hop edge and made only the exact two-component route capable of satisfying dotted-type proof.
- Round 1 SMELL: the stable incremental control could pass after an unnecessary rebuild and did not directly assert proof-key stability. Added a direct old/new proof-set equality unit.
- Round 2: 0 WRONG; one trailing-whitespace SMELL in this handoff, fixed. Consumer inspection confirmed every unqualified-call use still requires `MemberImport`; only the intended Python proof/route paths consume eligible module imports.
- At the cap, Clippy exposed one closed SMELL in the touched proof-key loop (`for_kv_map`). The finding sequence was converging and non-repeating, so one disclosed mechanical-fix extension replaced the ignored value iteration with `.keys()`.
- No findings remain open; the extension gates are pending.

## 4. Custody

- Branch/worktree: `py-qualified-receiver-owner` at `/private/tmp/slicing-py-qualified-receiver`.
- Base/main at branch creation: `5e54d48381f329cae370557eeac35bc00ff7b801`.
- Design checkpoint: `4096cb6`; intentional RED checkpoint: `1df642a`; focused-green implementation checkpoint: `411b9b9`. The gate-fix checkpoint is pending. Publication is not authorized for this successor increment.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
