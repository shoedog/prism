# Handoff — Go receiver type-origin binding Slice 3 terminal owner predicate

**Written:** 2026-08-28T14:46:22Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` · `a-receiver-provenance-slice3-terminal-owner-predicate` · **Base:** `[MEASURED]` `7fc719ae21ba130c554c318c3f8306093a804c92` (`origin/main` at worktree creation)
**Predecessor:** Slice 2 terminal closeout PR #211, merged at `7fc719ae21ba130c554c318c3f8306093a804c92`; Codex continuation of Claude session `a3bf14f1-6b47-464b-ba09-fc62e2ad7efb`
**Truth ordering:** measured live state > explicit owner/design authority within scope > this handoff > earlier handoffs and summaries. Conflicts stay open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` dedicated clean worktree/branch; no subagent dispatched. Primary `slicing` worktree has unrelated untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` and remains untouched.
**Custody exposure:** `[MEASURED]` plan/initial handoff are committed at `afdfb70` and their first reconciliation at `f0ff6f2`; compiled RED tests and this measured-plan correction are pending the immediate RED checkpoint.
**In flight / irreversible:** none.
**Authority:** owner said `authorized - proceed` after Slice 2 closeout, authorizing Slice 3 execution. Publication/merge authority will be reconciled against the owner's prior explicit push/merge authorization before remote mutation.

## 1. Resume order

1. Require `HEAD`/base `7fc719ae21ba130c554c318c3f8306093a804c92` and read the Slice 3 plan.
2. Preserve planning commit `afdfb70`; do not fold production code into or rewrite it.
3. Add the cross-file alias lost-provenance negatives and prove compiled RED across resolver, manifest, and navigation sidecar.
4. Implement only the shared predicate, its two consumers, and paired cache fences.
5. Run focused/full/Tier-A/corpus/oracle controls and the capped two-round review.
6. Refresh this handoff at every stable point and before publication.

**STOP conditions:** a valid Go recovered positive remains ownerless; a third independent receiver-edge consumer appears; the predicate changes non-Go/direct behavior; an edit populates owners or changes `CallSite::cmp_key`; a failing probe selects zero tests or fails for its own reason; generated eval output would be committed; or a same-environment exact-base control disproves attribution.

## 2. State ledger

| Item | State | Evidence / next action |
|---|---|---|
| Slice 2 landing | done | `[MEASURED]` exact base is terminal closeout merge `7fc719ae`; implementation/custody merges are in its history. |
| Design/census | done | `[MEASURED]` design §§1–8, `CallSite`, rematerialization, resolver, manifest, navigation edge builder, ownerless assertions, and cache pins read on exact base. Exactly two independent consumers found; sidecar reuses resolver. |
| Predicate contract | done | Go caller + recovered type/recovery + absent owner is terminal. Current positives for all local/cross-file recovery forms are owner-bearing; remaining absent owners are unproven/materialized/shadow cases. |
| Planning custody | done | `[MEASURED]` plan and initial handoff committed together at `afdfb70`; preserve it. |
| RED matrix | done | `[MEASURED]` resolver parity `1/1` and navigation parity `1/1` are already green; manifest `0/1` fails on the exact unauthorized zero-fanout ownerless record; CPG pin `0/1` fails `53 != 54`; sidecar pin `0/1` fails `21 != 22`. |
| Implementation | not started | One shared predicate; resolver and manifest consumers; CPG `54`, sidecar `22`. |
| Verification | not started | Focused, full suite, Tier-A, five corpora, oracle, site-count parity, exact-base controls as needed. |
| Review | not started | Declared cap: two rounds; classify all findings `WRONG` or `SMELL`. |
| Publication | not started | Push/PR/merge only after verification and authority reconciliation. |

## 3. Hypothesis/probe log

| Hypothesis | True observation | Falsifier / alternative | Result |
|---|---|---|---|
| H1: resolver and manifest are the only independent consumers; sidecar reuses resolver. | Resolver has ownerless special guard; manifest independently routes and runs legacy lookup; nav edge builder calls `resolve_call_site_full`. | A third path independently constructs receiver-derived edges; or manifest already delegates. | `[MEASURED]` supported; no third mint found. |
| H2: after Slices 0–2, any recovered Go site without owner is genuinely unproven. | All valid recovery forms retain exact owners; absent-owner rows are negative/materialized/shadow or explicit mutations. | A current positive resolves correctly while ownerless, indicating a missed producer. | `[MEASURED]` supported by source and fixture census. |
| H3: removing the real cross-file site's owner reconstructs the caller-alias rebinding defect in all consumers. | Resolver/manifest/nav expose `decoy/types.go` before the predicate. | The current route already drops, or failure is caused by malformed mutation. | `[MEASURED]` falsified for resolver/nav: both already drop. Manifest alone emits the ownerless zero-fanout record, proving the bounded parity defect. |

## 4. Invariants and traps

- The predicate must be one function with two direct production consumers; do not copy the boolean expression.
- Navigation sidecar parity is proved through its resolver-derived edge index, not a third predicate call.
- Check before `go_receiver_owner` or any legacy bare lookup; a late filter can still corrupt telemetry/manifest fanout.
- Do not weaken the guard to recovery kinds; all present recovery kinds require owner proof for Go.
- Do not gate rows with no recovered type/recovery; prerequisite/materialized drops and direct calls retain their existing paths.
- Do not change global owner resolution, owner population, shadowing, `cmp_key`, or parked #16 dispatch behavior.
- The LSP skill was selected, but its MCP tools are unavailable. Exhaustive bounded `rg` plus compiled tests is the disclosed semantic-navigation substitute.

## 5. Identifiers

| Item | Verbatim |
|---|---|
| Base | `7fc719ae21ba130c554c318c3f8306093a804c92` |
| Planning commit | `afdfb70` |
| Branch | `a-receiver-provenance-slice3-terminal-owner-predicate` |
| Worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` |
| Plan | `docs/superpowers/plans/2026-08-28-go-receiver-type-origin-binding-slice3.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-28-go-receiver-type-origin-binding-slice3-handoff.md` |
| Design | `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md` |
| Current cache pins | CPG `53`; sidecar `21` |
| Planned cache pins | CPG `54`; sidecar `22` |
| Review cap | `2` |

## 6. Owner questions

None blocking implementation. Reconfirm remote publication authority at the publication boundary if the current `authorized - proceed` plus earlier explicit push/merge authority is not treated as continuing authority for this slice.
