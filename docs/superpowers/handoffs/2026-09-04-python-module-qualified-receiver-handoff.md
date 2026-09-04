# Handoff — Python module-qualified typed-receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex  
**Workspace:** `/private/tmp/slicing-py-qualified-receiver` · `py-qualified-receiver-owner`  
**Exact base:** `5e54d48381f329cae370557eeac35bc00ff7b801`

## 0. Current verdict

**DESIGNED; RED and implementation pending.** This is the next adjacent Fork-A production increment after PR #226: prove Python receiver ownership for exactly `import module [as alias]` plus `alias.Class`, while retaining direct-method and fail-closed boundaries.

## 1. Authority boundary

Exact requires one clean module-scope `ModuleImport` alias, one indexed module file, one clean direct class, and one non-ambiguous direct method. Duplicate, rebound, wildcard, local-shadow, inherited-only, re-export, and multi-hop forms are excluded. The existing imported-class proof-key mismatch guard will cover incremental authority transitions once qualified keys are added.

## 2. Verification state

- Source feasibility probe: qualified annotation text survives as `models.Client`; the missing mechanism is import-owner proof/routing, not AST retention.
- Alternative ruled out: no AST grammar extension is required.
- RED/GREEN, review, full-suite, and Tier-A evidence are pending.

## 3. Convergence

Declared review cap: two rounds. No rounds have run.

## 4. Custody

- Branch/worktree: `py-qualified-receiver-owner` at `/private/tmp/slicing-py-qualified-receiver`.
- Base/main at branch creation: `5e54d48381f329cae370557eeac35bc00ff7b801`.
- No production source or tests changed yet; publication is not authorized for this successor increment.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
