# Handoff — Python namespace-package submodule receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-py-from-package-submodule-receiver` · `py-from-package-submodule-receiver-owner`
**Exact base:** `7488bb64f333bbc93f21c31c1104a551649467f4`

## 0. Current verdict

**DESIGN ACCEPTED; RED PENDING.** This is item 2b of the owner-selected Python authoritative module/scope queue.

## 1. Authority boundary

Exact is limited to `from pkg import models [as m]` plus `models.Class`/`m.Class` when the parent has neither an indexed `pkg.py` nor `pkg/__init__.py`, the composed `pkg.models` identity selects one indexed Python submodule, one clean direct class, and one direct method, and the local qualifier is unshadowed. Regular-package exports, relative imports, source-root inference, and general scope resolution remain excluded.

## 2. Hypothesis-probe-result log

- LSP semantic navigation is unavailable in this session; targeted source references and compiler-backed tests are the fallback.
- Hypothesis: extraction retains parent, imported member, and local alias. Expected `module_path=pkg`, `member=models`, `local=models|m`; loss would require a new syntax fact. Result: all three facts survive in `MemberImport`.
- Alternative: existing package-scope facts are strong enough to prove arbitrary regular-package exports. Result: falsified; module binding kinds do not prove absence of dynamic package attributes. The slice therefore requires namespace-parent absence and blocks every indexed parent module/initializer.
- Incremental/caching blast radius: existing four-field proof keys can represent the route, but call-site classification and navigation topology change. CPG/navigation versions must advance 56→57 and 25→26.

## 3. Verification state

- Root `main` and `origin/main` were rebound to PR #228 merge `7488bb64` before branch creation.
- Focused RED, implementation, review, broad gates, and publication have not begun.
- Declared implementation-review cap: two rounds.

## 4. Custody

- Branch/worktree: `py-from-package-submodule-receiver-owner` at `/private/tmp/slicing-py-from-package-submodule-receiver`.
- Exact base: `7488bb64f333bbc93f21c31c1104a551649467f4`.
- Design/roadmap checkpoint: this document's containing commit.
- PR #228 and its prior handoff are reconciled as merged in this checkpoint.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Publication/merge of this successor increment is not authorized by the PR #228 publication instruction.
