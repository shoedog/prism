# Handoff — Python namespace-package submodule receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-py-from-package-submodule-receiver` · `py-from-package-submodule-receiver-owner`
**Exact base:** `7488bb64f333bbc93f21c31c1104a551649467f4`

## 0. Current verdict

**IMPLEMENTED; FOCUSED GREEN; TWO-ROUND REVIEW COMPLETE; BROAD GATES PENDING.** This is item 2b of the owner-selected Python authoritative module/scope queue.

## 1. Authority boundary

Exact is limited to `from pkg import models [as m]` plus `models.Class`/`m.Class` when the parent has neither an indexed `pkg.py` nor `pkg/__init__.py`, the composed `pkg.models` identity selects one indexed Python submodule, one clean direct class, and one direct method, and the local qualifier is unshadowed. Regular-package exports, relative imports, source-root inference, and general scope resolution remain excluded.

## 2. Hypothesis-probe-result log

- LSP semantic navigation is unavailable in this session; targeted source references and compiler-backed tests are the fallback.
- Hypothesis: extraction retains parent, imported member, and local alias. Expected `module_path=pkg`, `member=models`, `local=models|m`; loss would require a new syntax fact. Result: all three facts survive in `MemberImport`.
- Alternative: existing package-scope facts are strong enough to prove arbitrary regular-package exports. Result: falsified; module binding kinds do not prove absence of dynamic package attributes. The slice therefore requires namespace-parent absence and blocks every indexed parent module/initializer.
- Incremental/caching blast radius: existing four-field proof keys can represent the route, but call-site classification and navigation topology change. CPG/navigation versions must advance 56→57 and 25→26.
- The first attempted RED command named a nonexistent Cargo target and was classified inadmissible. The corrected target produced the expected RED: behavior and subset positives had `receiver_type=None`, the proof-key set was empty, navigation was `NameOnly`, incremental/cached positives were unclassified, and cache pins remained 56/25; the proof-barrier control passed.
- Implementation composes the exact namespace submodule from the eligible `MemberImport`, blocks an indexed parent module/initializer, reuses exact dotted-file selection, and feeds the shared receiver classification and final resolver. No parser fact or consumer signature changed.
- Review cap result: round 1 found no WRONG items and four bounded SMELLs (initializer-form coverage, constructor-negative coverage, a stale comment, and a redundant per-class rescan); all were closed. Round 2 found no WRONG items and no recurring/open-class SMELL.

## 3. Verification state

- Root `main` and `origin/main` were rebound to PR #228 merge `7488bb64` before branch creation.
- Focused behavior: 4 passed, 0 failed; includes unaliased/aliased typed and constructor receivers, `.py` and `__init__.py` submodules, direct-subset proof parity, 13 typed proof barriers, and constructor blocker edges.
- Proof key, cache pins, CPG cache round trip, navigation sidecar round trip, and seven incremental authority transitions are green.
- Complete Python language target: 83 passed, 0 failed. Python-filtered integration target: 25 passed, 0 failed. Python-filtered library target: 60 passed, 0 failed.
- Broad static/full-suite/release/Tier-A gates and publication have not begun.
- Declared implementation-review cap: two rounds; completed and converged.

## 4. Custody

- Branch/worktree: `py-from-package-submodule-receiver-owner` at `/private/tmp/slicing-py-from-package-submodule-receiver`.
- Exact base: `7488bb64f333bbc93f21c31c1104a551649467f4`.
- Design/roadmap checkpoint: `1950cb3e`; intentional RED checkpoint: `5dae8d8a`; implementation/review checkpoint: this document's containing commit.
- PR #228 and its prior handoff are reconciled as merged in this checkpoint.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Publication/merge of this successor increment is not authorized by the PR #228 publication instruction.
