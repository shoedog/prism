# Handoff — Go receiver type-origin binding Slice 2 eager local owners

**Written:** 2026-08-27T09:00:59Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s2` · `a-receiver-provenance-slice2-eager-local-owner` · **Measured state:** `[MEASURED]` HEAD `1a82bb0de43e2c1bac1eb8717a4166099c4e0c20` · Tree CLEAN before this planning checkpoint · Probe `git status --porcelain=v1 && git rev-parse HEAD && git rev-parse origin/main` · Output inline in the active Codex session
**Predecessor:** Codex continuation of Claude session `a3bf14f1-6b47-464b-ba09-fc62e2ad7efb`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by Codex from merged Slice 1 custody, design v3, and exact current source/test reads. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? `[MEASURED]` no subagent was dispatched and this fresh Slice 2 branch is the only worktree on its branch — **RESOLVED for this worker 2026-08-27**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` base is durably merged at `origin/main`; this plan and handoff become the only uncommitted artifacts when this checkpoint is written — **OPEN until the planning checkpoint is committed**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no long-running command has been launched in this Slice 2 worktree and no migration exists — **RESOLVED for known sessions 2026-08-27**
**(d) Authorization granted but not exercised** — none for Slice 2 publication. The owner's earlier quote, `authorized to push/merge`, was exercised for Slice 1 and is not silently reused for a new slice.

## 1. Resume order

1. In `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s2`, require `git rev-parse HEAD` and `git rev-parse origin/main` to share base `1a82bb0de43e2c1bac1eb8717a4166099c4e0c20`, then read `docs/superpowers/plans/2026-08-27-go-receiver-type-origin-binding-slice2.md`.
2. Commit this plan/handoff checkpoint before test edits.
3. Update only the named existing tests and cache pins, run the exact RED selectors, then implement the one screened caller-file owner mutation.
4. Run focused, cache, full-suite, corpus, and Accuracy Harness gates; refresh this handoff at each stable point.
5. Ask for fresh Slice 2 publication authority only after a reviewed green candidate exists.

**STOP conditions:** any proposed change populates a `proof_shadowed` owner; bypasses strict declaration admissibility; changes `CallSite::cmp_key`; implements Slice 3 absent-provenance behavior; changes cross-file owner semantics; treats a zero-selected or self-failing probe as evidence; modifies generated Tier-A artifacts; or publishes by reusing Slice 1 authority.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Slice 1 landing | done | `[MEASURED]` `origin/main` and Slice 2 base are custody merge `1a82bb0d`; implementation merge `d9aae91f` is its parent history. |
| Slice 2 contract/census | done | `[MEASURED]` design line 49, prerequisite screen, classifier branches, rematerialization consumer, ownerless assertions, cache pins, and four-path cache test were read on the exact base. |
| Planning custody | next | `[MEASURED]` this plan and handoff are the only intended checkpoint artifacts; commit SHA is not yet assigned. |
| RED matrix | pending | Existing unshadowed collision and four-form fixtures must be changed before production code. |
| Production implementation | pending | Only the admissible unshadowed `CallerFile` screen branch plus paired cache fences is in scope. |
| Full verification/review | pending | No Slice 2 test or harness result exists yet. |
| Publication | blocked | Fresh Slice 2 push/PR/merge authority has not been granted. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Slice 1 handoff §4 | Describes its custody reconciliation as in progress. | `[MEASURED]` PR #208 is merged at `1a82bb0d` and all five hosted checks, including coverage, completed successfully. This Slice 2 handoff supersedes that operational state; the merged historical file is not rewritten here. |
| Memory receiver-design entry | Predates Slice 0/1 landing. | `[MEASURED]` live Git supersedes that operational state: Slices 0 and 1 are merged. Memory is not edited without explicit owner request. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Planning custody | next | Commit the plan and this live handoff together. | None | base `1a82bb0d` |
| 2 | RED matrix | pending | Require owners on the existing local recovery fixtures while preserving the shadowed negative, then run exact selectors. | Planning checkpoint | `concrete_receiver_route`; `concrete_receiver_fix4`; cache parity |
| 3 | Screen mutation | pending | Retain the already-resolved strict owner only when `proof_shadowed == false`. | Admissible RED | `screen_go_receiver_prerequisites` |
| 4 | Cache fences | pending | Bump CPG `52->53` and sidecar `20->21` with history/pin updates. | RED | cache pins |
| 5 | Verification and review | pending | Run plan §6 plus exact-base controls for any exclusion and capped two-round review. | GREEN implementation | full suite; Tier-A; five corpora |
| 6 | Publication | blocked | Obtain fresh authority, then push, open PR, wait for all required checks, and merge only when green. | Owner authority and green candidate | branch `a-receiver-provenance-slice2-eager-local-owner` |

## 5. Invariants and traps — do not do these

- Never populate an owner for `proof_shadowed` — the first declaration no longer proves the live value binding and bypasses the existing collision bail.
- Never mutate each classifier producer independently — the existing strict post-merge screen is the one complete admissibility membrane.
- Never widen `resolve_go_owner_identity` — receiver proof uses the strict helper only.
- Never alter `CallSite::cmp_key` — owner replacement must not create a second occurrence.
- Never change a cross-file carried/un-carried origin in Slice 2 — those are Slice 1 semantics.
- Never treat a failed harness invocation as RED/GREEN evidence — require selected tests and read their produced assertion output.
- The LSP skill surface is unavailable in this session → use exhaustive text references and compiled tests, and record that semantic-navigation exclusion.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base / Slice 1 custody merge | `1a82bb0de43e2c1bac1eb8717a4166099c4e0c20` |
| Slice 1 implementation merge | `d9aae91f10d8a8355f052607cefeebe18f0f46fd` |
| Branch | `a-receiver-provenance-slice2-eager-local-owner` |
| Worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s2` |
| Plan | `docs/superpowers/plans/2026-08-27-go-receiver-type-origin-binding-slice2.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-27-go-receiver-type-origin-binding-slice2-handoff.md` |
| Design | `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md` |
| Primary RED | `cargo test --test lang_go concrete_receiver_fix4 -- --nocapture` |
| Four-form RED | `cargo test --test lang_go concrete_receiver_route -- --nocapture` |
| Cache RED | `cargo test --test navigation concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits -- --nocapture` |
| Planned cache fences | CPG `53`; sidecar `21` |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — this is the pre-RED planning checkpoint · claim: "one post-merge screen mutation can populate every admissible unshadowed caller-local owner without changing shadowed or cross-file behavior" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: active Codex Slice 2 source/test census

**Questions the owner owes an answer to:** None for local implementation. Fresh publication authority is required only after the candidate is green.
