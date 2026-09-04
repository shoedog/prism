# Handoff — Python unaliased dotted-module receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-py-dotted-module-receiver` · `py-dotted-module-receiver-owner`
**Exact base:** `4298e548003cbb59cf506531142d177169a7a28e`

## 0. Current verdict

**DESIGN LOCKED; RED PENDING.** This is item 2a of the owner-selected Fork-A queue: the smallest authoritative Python module/scope increment after PRs #226 and #227.

## 1. Authority boundary

Exact is limited to `import pkg.models` plus `pkg.models.Class`, with exact qualifier/import-path equality, one eligible root binding, one indexed module file, one clean direct class, one direct method, and no function-local root shadow. Explicit aliases remain separately authorized only as `alias.Class`; all shortened, mismatched, re-exported, inherited, local, or ambiguous forms fail closed.

## 2. Hypothesis-probe-result log

- Hypothesis: extraction retains both bound root and full dotted module path, allowing a route-only extension. Expected `local=pkg`, `module_path=pkg.models`; loss of either would require broader extraction work. Result: both facts survive.
- Alternative: existing facts distinguish unaliased and explicitly aliased imports. Discriminator: compare `import pkg.models` with `import pkg.models as pkg`. Result: falsified; both currently serialize to the same binding tuple, so a new alias-shape fact is required.
- Blast radius: `ImportBinding` is serialized in `CallGraph`; resolved navigation topology also changes. Result: design requires CPG 55→56 and navigation sidecar 24→25.
- Tooling exclusion: the requested LSP navigation service is unavailable in this session; source-reference scans and compiler-backed gates are the fallback, not behavioral evidence.

## 3. Verification state

- Root and `origin/main` were rebound to PR #227 merge `4298e548` before branch creation.
- No source or test behavior has changed; RED and implementation are pending.
- Declared implementation-review cap: two rounds.

## 4. Custody

- Branch/worktree: `py-dotted-module-receiver-owner` at `/private/tmp/slicing-py-dotted-module-receiver`.
- Exact base: `4298e548003cbb59cf506531142d177169a7a28e`.
- The prior lane is merged as PR #227; its handoff and the living roadmap are reconciled in this design checkpoint.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Publication and merge of this successor increment are not authorized by the prior PR #227 publication instruction.
