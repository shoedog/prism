# Handoff — Go receiver type-origin binding Slice 3 terminal owner predicate

**Written:** 2026-08-29T03:21:29Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` · `a-receiver-provenance-slice3-terminal-owner-predicate` · **Base:** `[MEASURED]` `7fc719ae21ba130c554c318c3f8306093a804c92` (`origin/main` at worktree creation)
**Predecessor:** Slice 2 terminal closeout PR #211, merged at `7fc719ae21ba130c554c318c3f8306093a804c92`; Codex continuation of Claude session `a3bf14f1-6b47-464b-ba09-fc62e2ad7efb`
**Truth ordering:** measured live state > explicit owner/design authority within scope > this handoff > earlier handoffs and summaries. Conflicts stay open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` dedicated clean worktree/branch; no subagent dispatched. Primary `slicing` worktree has unrelated untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` and remains untouched.
**Custody exposure:** `[MEASURED]` plan/initial handoff are committed at `afdfb70`, first reconciliation at `f0ff6f2`, RED matrix at `e218a6a`, RED reconciliation at `ca86f02`, implementation at `89ea24e`, implementation reconciliation at `d7542c4`, bounded integration-test contract repair at `1a1e016`, its reconciliation at `068fa9a`, and the enumerated fix2/fix3 ownerless contract repair at `52348c9`; only this handoff reconciliation awaits its immediate checkpoint.
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
| Implementation | done | `[MEASURED]` checkpoint `89ea24e`: one shared predicate has exactly two consumers; resolver/manifest terminalize before routing; CPG `54`/sidecar `22`; shadowed negative pins the earlier terminal boundary. |
| Verification | in progress | `[MEASURED]` focused GREEN: owner carrying `7/7`, route `13/13`, shadow/collision `4/4`, prerequisites `13/13`, navigation/cache `2/2`, fix2 `3/3`, fix3 `4/4`. Format/check green. Clippy candidate/base each have 171 identical diagnostic headers. `cargo test --no-fail-fast` enumerated exactly three obsolete ownerless R3 expectations after the integration repair; updated fix2 expectations are exact-base RED `1 passed/2 failed`, fix3 exact-base RED `3 passed/1 failed`, and candidate GREEN. Full-suite rerun pending. |
| Review | not started | Declared cap: two rounds; classify all findings `WRONG` or `SMELL`. |
| Publication | not started | Push/PR/merge only after verification and authority reconciliation. |

## 3. Hypothesis/probe log

| Hypothesis | True observation | Falsifier / alternative | Result |
|---|---|---|---|
| H1: resolver and manifest are the only independent consumers; sidecar reuses resolver. | Resolver has ownerless special guard; manifest independently routes and runs legacy lookup; nav edge builder calls `resolve_call_site_full`. | A third path independently constructs receiver-derived edges; or manifest already delegates. | `[MEASURED]` supported; no third mint found. |
| H2: after Slices 0–2, any recovered Go site without owner is genuinely unproven. | All valid recovery forms retain exact owners; absent-owner rows are negative/materialized/shadow or explicit mutations. | A current positive resolves correctly while ownerless, indicating a missed producer. | `[MEASURED]` supported by source and fixture census. |
| H3: removing the real cross-file site's owner reconstructs the caller-alias rebinding defect in all consumers. | Resolver/manifest/nav expose `decoy/types.go` before the predicate. | The current route already drops, or failure is caused by malformed mutation. | `[MEASURED]` falsified for resolver/nav: both already drop. Manifest alone emits the ownerless zero-fanout record, proving the bounded parity defect. |
| H4: the shared predicate flips only the manifest RED while preserving resolver/nav controls. | Parity/pin selectors turn green without positive regression. | Manifest remains present or resolver/nav acquire an edge/regression. | `[MEASURED]` supported: language parity `2/2`, navigation `1/1`, pins `2/2`. |
| H5: early terminalization changes the shadowed collision-bail telemetry/manifest record. | Candidate fails the old telemetry/manifest expectations while exact base passes them. | Exact base fails identically. | `[MEASURED]` attributed: old expectations candidate `2/4`, exact base `4/4`; updated contract exact base RED `2/4`, candidate GREEN `4/4`. Edge/owner result remains unchanged. |
| H6: Clippy failures are exact-base-identical repository debt. | Candidate/base sorted diagnostic headers match with no changed-region diagnostic. | Any header delta or new-code diagnostic. | `[MEASURED]` supported after discarding one sandbox-lock capture: valid logs each contain 171 headers, zero multiset difference; no new-region diagnostic. |
| H7: full-suite failure is an obsolete ownerless legacy-emitter expectation. | Fixture synthetically sets owner `None`; base passes old expectation; updated terminal assertion is base RED/candidate GREEN. | Owner-bearing fixture or identical base failure. | `[MEASURED]` supported: old test base `1/1`, candidate `0/1`; converted populated-bucket negative base `0/1`, candidate `1/1`. |
| H8: the remaining three full-suite failures are obsolete ownerless R3 expectations rather than producer regressions. | Updated negatives fail only on exact-base legacy rows/edges; owner-bearing positives pass in the same binaries; candidate is green. | Compile/fixture failure, owner-bearing positive failure, or updated exact-base green. | `[MEASURED]` supported: no-fail-fast found no other target failure; fix2 base `1 passed/2 failed` vs candidate `3/3`, fix3 base `3 passed/1 failed` vs candidate `4/4`. Exact-base failures expose the old fallback telemetry, interface-dispatch edges, or manifest presence. |

## 4. Invariants and traps

- The predicate must be one function with two direct production consumers; do not copy the boolean expression.
- Navigation sidecar parity is proved through its resolver-derived edge index, not a third predicate call.
- Check before `go_receiver_owner` or any legacy bare lookup; a late filter can still corrupt telemetry/manifest fanout.
- Do not weaken the guard to recovery kinds; all present recovery kinds require owner proof for Go.
- Do not gate rows with no recovered type/recovery; prerequisite/materialized drops and direct calls retain their existing paths.
- Do not change global owner resolution, owner population, shadow classification, `cmp_key`, or parked #16 dispatch behavior. The shared predicate intentionally supersedes the old shadow collision-bail telemetry/manifest record while preserving its ownerless zero-edge result.
- The LSP skill was selected, but its MCP tools are unavailable. Exhaustive bounded `rg` plus compiled tests is the disclosed semantic-navigation substitute.

## 5. Identifiers

| Item | Verbatim |
|---|---|
| Base | `7fc719ae21ba130c554c318c3f8306093a804c92` |
| Planning commit | `afdfb70` |
| RED checkpoint | `e218a6a` |
| Implementation checkpoint | `89ea24e` |
| Implementation reconciliation | `d7542c4` |
| Integration-test checkpoint | `1a1e016` |
| Integration-test reconciliation | `068fa9a` |
| Enumerated ownerless-test checkpoint | `52348c9` |
| Branch | `a-receiver-provenance-slice3-terminal-owner-predicate` |
| Worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` |
| Plan | `docs/superpowers/plans/2026-08-28-go-receiver-type-origin-binding-slice3.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-28-go-receiver-type-origin-binding-slice3-handoff.md` |
| Design | `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md` |
| Base cache pins | CPG `53`; sidecar `21` |
| Candidate cache pins | CPG `54`; sidecar `22` |
| Exact-base control worktree | `/private/tmp/slicing-s3-base-7fc719ae` |
| Review cap | `2` |

## 6. Owner questions

None blocking implementation. Reconfirm remote publication authority at the publication boundary if the current `authorized - proceed` plus earlier explicit push/merge authority is not treated as continuing authority for this slice.
