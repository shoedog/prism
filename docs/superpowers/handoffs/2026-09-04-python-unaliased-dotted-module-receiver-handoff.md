# Handoff — Python unaliased dotted-module receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-py-dotted-module-receiver` · `py-dotted-module-receiver-owner`
**Exact base:** `4298e548003cbb59cf506531142d177169a7a28e`

## 0. Current verdict

**CLOSED AND MERGED.** Item 2a of the owner-selected Fork-A queue merged as PR #228 at `7488bb64f333bbc93f21c31c1104a551649467f4` on 2026-09-04.

## 1. Authority boundary

Exact is limited to `import pkg.models` plus `pkg.models.Class`, with exact qualifier/import-path equality, one eligible root binding, one indexed module file, one clean direct class, one direct method, and no function-local root shadow. Explicit aliases remain separately authorized only as `alias.Class`; all shortened, mismatched, re-exported, inherited, local, or ambiguous forms fail closed.

## 2. Hypothesis-probe-result log

- Hypothesis: extraction retains both bound root and full dotted module path, allowing a route-only extension. Expected `local=pkg`, `module_path=pkg.models`; loss of either would require broader extraction work. Result: both facts survive.
- Alternative: existing facts distinguish unaliased and explicitly aliased imports. Discriminator: compare `import pkg.models` with `import pkg.models as pkg`. Result: falsified; both currently serialize to the same binding tuple, so a new alias-shape fact is required.
- Blast radius: `ImportBinding` is serialized in `CallGraph`; resolved navigation topology also changes. Result: design requires CPG 55→56 and navigation sidecar 24→25.
- Round-2 scope hypothesis: the shared function-local census poisons every constructible root binder. Alternative: assignment/import coverage omits other grammar shapes. Result: the alternative won for `with`/`except` aliases, `del`, starred targets, and local type aliases; the focused barrier reproduced an incorrect recovered `pkg.models.Client` and Exact route for `with ... as pkg` and `del pkg`. The bounded grammar population was added to the census and the expanded barrier is green.
- Exact-path hypothesis: a dotted absolute import can match only its complete filesystem path. Alternative: the legacy stem fallback could select `other/models.py`. Result: full-path candidates are already mandatory for multi-component Python imports, and the wrong-package form is already regression-tested.
- Tooling exclusion: the requested LSP navigation service is unavailable in this session; source-reference scans and compiler-backed gates are the fallback, not behavioral evidence.

## 3. Verification state

- Root and `origin/main` were rebound to PR #227 merge `4298e548` before branch creation.
- Exact-base behavior target: 1 proof-barrier control passed; the full and subset positives failed because `receiver_type` remained `None` instead of `Some("pkg.models.Client")`.
- Exact-base extraction target: 0/1; explicit `import pkg.models as pkg` reported `ModuleImport` instead of the required `AliasedModuleImport` distinction.
- Exact-base incremental target: 0/3; both authority transitions and the stable-proof case failed at the missing dotted receiver proof.
- Exact-base direct proof-key unit: 0/1; observed `("app.py", "pkg.Client", "pkg/models.py", "Client")`, expected the full `pkg.models.Client` spelling.
- Exact-base cache pins: 0/2; observed CPG/navigation versions 55/24, expected 56/25.
- Focused GREEN: dotted full/subset/proof barriers 3/3, explicit-alias extraction 1/1, incremental lifecycle 3/3, stable proof-key unit 1/1, and cache pins 2/2.
- Review GREEN: targeted CPG round trip 1/1 preserves both explicit-alias and unaliased-dotted import kinds plus Exact targets; targeted navigation sidecar round trip 1/1 preserves the dotted `TypedParam` Exact edge.
- Round 1 — WRONG: none. SMELL: the acceptance text relied on generic cache suites without a fixture for the new serialized alias kind and topology. Fixed with the two targeted round-trip tests above.
- Round 2 — WRONG: the root-shadow proof omitted constructible Python local binders (`with`/`except` aliases, `del`, starred targets, and local type aliases), allowing an Exact edge after authority should have failed closed. Fixed as one finite grammar-census population with focused RED then GREEN. SMELL: conservative `global`/`nonlocal`, generic-type-parameter, and Unicode-identifier handling may reduce recall; none can mint a wrong Exact edge in this slice.
- The declared two-round review cap is exhausted and closed. The findings were bounded and non-repeating; no third review round was dispatched.
- Production changes add a distinct Python explicit-alias import kind, exact full-qualifier/module-path proof, root-based shadow screening, qualified-chain constructor recovery, and paired CPG/navigation cache bumps.
- Complete related targets: Python 79/79, import binding 59/59, and dotted incremental 3/3.
- Static gates: `cargo fmt --all -- --check`, `git diff --check`, `cargo check --all-targets`, and configured `cargo clippy --all-targets --features mcp -- -W clippy::all` all passed. Clippy emitted warnings but no errors; the warnings were not re-baselined or repaired as part of this slice.
- Full default suite: 3,572 passed, 0 failed, 1 ignored. Full `mcp` suite: 3,758 passed, 0 failed, 1 ignored.
- Accuracy Harness custody: release build passed immediately before matrix-only; matrix-only passed 104/104. A second release build passed immediately before quick.
- Tier-A quick ran to completion but exited 2 because the structured report marked the baseline invalid solely for `corpus_sha_drift: 4aeb3214707f != pinned 20c8490591a3`. The report recorded oracle error rate 0.000, SUT error rate 0.000, a quiescent oracle, and matrix 104/104; scoring is excluded and no baseline was rewritten. The reproducible untracked report/snapshot copies were removed after evidence extraction; the ignored run artifact remains at `eval/runs/2026-09-04-prism.json` in this worktree.

## 4. Custody

- Branch/worktree: `py-dotted-module-receiver-owner` at `/private/tmp/slicing-py-dotted-module-receiver`.
- Exact base: `4298e548003cbb59cf506531142d177169a7a28e`.
- Design/roadmap checkpoint: `6756914`; intentional RED checkpoint: `a3371bf`; focused-green implementation checkpoint: `00e0ff6`; review-fix checkpoint: `4aeb321`; final verification/handoff checkpoint: this document's containing commit.
- The prior lane is merged as PR #227; its handoff and the living roadmap are reconciled in the design checkpoint.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Published head: `8f456b632f503983a91bd09e40af0856680afba2`; PR #228; merge: `7488bb64f333bbc93f21c31c1104a551649467f4`. All five GitHub checks passed: Test Suite, Clippy Lint, Format Check, Coverage, and Language Coverage Matrix.
- Root `main` and `origin/main` were rebound to the merge before the next lane was created.
- No work remains in this lane. The successor is the separately designed namespace-package submodule receiver increment.
