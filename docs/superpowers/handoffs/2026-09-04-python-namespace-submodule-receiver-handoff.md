# Handoff — Python namespace-package submodule receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-py-from-package-submodule-receiver` · `py-from-package-submodule-receiver-owner`
**Exact base:** `7488bb64f333bbc93f21c31c1104a551649467f4`

## 0. Current verdict

**CLOSED AND MERGED.** PR #230 merged reviewed head `fbdade96dd544e0a3fab8010811416652c6b7121` as `5051918f61c99fda83eb18936992fb62025b7669` on 2026-09-04. This completes item 2b and the owner-selected Python authoritative module/scope queue.

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
- Static gates: `cargo fmt --all -- --check`, `git diff --check`, `cargo check --all-targets`, and configured `cargo clippy --all-targets --features mcp -- -W clippy::all` passed. Clippy emitted the repository warning inventory but no errors.
- Full default suite: 3,579 passed, 0 failed, 1 ignored across 28 binaries. Full `mcp` suite: 3,765 passed, 0 failed, 1 ignored across 30 binaries.
- Two immediate-predecessor `cargo build --release` runs passed. Tier-A matrix-only reported 104/104 `ok`.
- Tier-A quick completed but exited 2 solely because `corpus_sha_drift: 9ca42ae3ee6d != pinned 20c8490591a3`; its artifact reported 104/104 matrix entries and 30/30 real probes `ok`, zero oracle/SUT error rates, a quiescent oracle, and a clean corpus. No baseline was rewritten; all generated untracked reports/snapshots were removed.
- LSP semantic navigation could not be run because its tools were unavailable in this session.
- GitHub checks passed on exact final head `fbdade96`: Test Suite 6m13s, Clippy Lint 51s, Format Check 19s, Coverage 4m35s, and Language Coverage Matrix 8s.
- Declared implementation-review cap: two rounds; completed and converged.

## 4. Custody

- Branch/worktree: `py-from-package-submodule-receiver-owner` at `/private/tmp/slicing-py-from-package-submodule-receiver`.
- Exact base: `7488bb64f333bbc93f21c31c1104a551649467f4`.
- Design/roadmap checkpoint: `1950cb3e`; intentional RED checkpoint: `5dae8d8a`; implementation/review checkpoint: `9ca42ae3`; local verification checkpoint: `5e758a0e`; publication-authorization checkpoint: `86637c4`; PR-identity handoff checkpoint: `fbdade96`.
- PR #228 and its prior handoff are reconciled as merged in this checkpoint.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Publication and merge were explicitly authorized by the owner on 2026-09-04. PR #230: https://github.com/shoedog/prism/pull/230
- Root `main` and `origin/main` were rebound to merge `5051918f`; successor work moved to `/private/tmp/slicing-js-ts-lexical-receiver-binding` on `js-ts-lexical-receiver-binding-owner`.
