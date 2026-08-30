# Handoff — Go receiver type-origin binding Slice 3 terminal owner predicate

**Written:** 2026-08-30T23:27:01Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` · `a-receiver-provenance-slice3-terminal-owner-predicate` · **Base:** `[MEASURED]` `0139d7fab18d71cfa33f9de609bf280674df85e8` (`origin/main`, scope-aware owner prerequisite PR #212 merge)
**Predecessor:** scope-aware owner prerequisite PR #212, merged at `0139d7fab18d71cfa33f9de609bf280674df85e8`; the original Slice 3 lane followed Slice 2 PR #211 and continues Claude session `a3bf14f1-6b47-464b-ba09-fc62e2ad7efb`
**Truth ordering:** measured live state > explicit owner/design authority within scope > this handoff > earlier handoffs and summaries. Conflicts stay open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` dedicated worktree/branch; no subagent dispatched. Rebase onto PR #212 completed at `e5596ba4e91a39fbf1b4bf0affc5b7013365c835` after one additive test-fixture conflict; primary `slicing` worktree remains untouched.
**Custody exposure:** `[MEASURED]` the 15-commit Slice 3 chain was preserved and rebased: planning `cd727884`, RED `d723287b`, implementation `9bf38001`, expectation repairs `00d91c9f` and `4a0facb6`, historical full/Tier-A/corpus checkpoints `4af62039`/`4dc60e29`/`f60ce6f3`, and historical review STOP `c9bbb136`; `e5596ba4` records the pre-rebase review reconciliation. This refresh rebinds the merged prerequisite; post-rebase verification is not yet claimed.
**In flight / irreversible:** none.
**Authority:** owner repeatedly said `authorized` and explicitly authorized push/merge; publish and merge remain authorized once non-coverage gates and the bounded review are green.

## 1. Resume order

1. Require base `0139d7fab18d71cfa33f9de609bf280674df85e8` and Slice 3 HEAD at or after `e5596ba4`.
2. Preserve the rebased 15-commit implementation chain; do not fold production code or discard the reviewed artifact.
3. Re-run focused and full repository gates on the rebased composition.
4. Rebuild release immediately before Tier-A and re-run the pinned Prometheus line-302 discriminator plus the established corpus/oracle checks.
5. Complete review round 2 of the declared two-round cap, explicitly testing whether PR #212 closes the round-1 `WRONG` without a new incorrect result.
6. Refresh this handoff, push, open the PR, and merge only after relevant non-coverage checks are green.

**STOP conditions:** a valid Go recovered positive remains ownerless; a third independent receiver-edge consumer appears; the predicate changes non-Go/direct behavior; an edit populates owners or changes `CallSite::cmp_key`; a failing probe selects zero tests or fails for its own reason; generated eval output would be committed; or a same-environment exact-base control disproves attribution.

## 2. State ledger

| Item | State | Evidence / next action |
|---|---|---|
| Base/prerequisite landing | done | `[MEASURED]` exact base is PR #212 merge `0139d7fa`; it contains Slice 2 merge `7fc719ae` and the scope-aware owner repair. |
| Design/census | done | `[MEASURED]` design §§1–8, `CallSite`, rematerialization, resolver, manifest, navigation edge builder, ownerless assertions, and cache pins read on exact base. Exactly two independent consumers found; sidecar reuses resolver. |
| Predicate contract | prerequisite merged; revalidation in progress | `[MEASURED]` PR #212 restores an exact owner and the sound manifest edge for pinned Prometheus line 302 in its own final release/corpus gate. The rebased Slice 3 composition must now reproduce that result. |
| Planning custody | done | `[MEASURED]` rebased planning commit is `cd727884`; preserve it. |
| RED matrix | done | `[MEASURED]` resolver parity `1/1` and navigation parity `1/1` are already green; manifest `0/1` fails on the exact unauthorized zero-fanout ownerless record; CPG pin `0/1` fails `53 != 54`; sidecar pin `0/1` fails `21 != 22`. |
| Implementation | done | `[MEASURED]` rebased checkpoint `9bf38001`: one shared predicate has exactly two consumers; resolver/manifest terminalize before routing; CPG `54`/sidecar `22`; shadowed negative pins the earlier terminal boundary. |
| Verification | in progress after rebase | Historical pre-prerequisite gates are retained for custody but are not current acceptance evidence. Run the focused, full-suite, Clippy, release, Tier-A, Prometheus, and corpus/oracle gates on `0139d7fa..HEAD`. |
| Review | round 2 pending | Declared cap remains two rounds. Round 1 found one `WRONG`; PR #212 is the bounded prerequisite repair. Round 2 must prove that failure closed and enumerate any remaining `WRONG` before `SMELL`. |
| Publication | pending gates | Authorized after post-rebase verification and review round 2; coverage need not be awaited per owner direction. |

## 3. Hypothesis/probe log

| Hypothesis | True observation | Falsifier / alternative | Result |
|---|---|---|---|
| H1: resolver and manifest are the only independent consumers; sidecar reuses resolver. | Resolver has ownerless special guard; manifest independently routes and runs legacy lookup; nav edge builder calls `resolve_call_site_full`. | A third path independently constructs receiver-derived edges; or manifest already delegates. | `[MEASURED]` supported; no third mint found. |
| H2: after Slices 0–2 plus PR #212, any recovered Go site without owner is genuinely unproven. | All valid recovery forms retain exact owners; absent-owner rows are negative/materialized/shadow or explicit mutations. | A current positive resolves correctly while ownerless, indicating another missed producer. | `[MEASURED]` the original form was falsified in review round 1. PR #212's final pinned Prometheus probe restores line 302 with exact owner `prompb.unsafeLabelAdder` and fanout `1`; post-rebase Slice 3 reproduction is pending. |
| H3: removing the real cross-file site's owner reconstructs the caller-alias rebinding defect in all consumers. | Resolver/manifest/nav expose `decoy/types.go` before the predicate. | The current route already drops, or failure is caused by malformed mutation. | `[MEASURED]` falsified for resolver/nav: both already drop. Manifest alone emits the ownerless zero-fanout record, proving the bounded parity defect. |
| H4: the shared predicate flips only the manifest RED while preserving resolver/nav controls. | Parity/pin selectors turn green without positive regression. | Manifest remains present or resolver/nav acquire an edge/regression. | `[MEASURED]` supported: language parity `2/2`, navigation `1/1`, pins `2/2`. |
| H5: early terminalization changes the shadowed collision-bail telemetry/manifest record. | Candidate fails the old telemetry/manifest expectations while exact base passes them. | Exact base fails identically. | `[MEASURED]` attributed: old expectations candidate `2/4`, exact base `4/4`; updated contract exact base RED `2/4`, candidate GREEN `4/4`. Edge/owner result remains unchanged. |
| H6: Clippy failures are exact-base-identical repository debt. | Candidate/base sorted diagnostic headers match with no changed-region diagnostic. | Any header delta or new-code diagnostic. | `[MEASURED]` supported after discarding one sandbox-lock capture: valid logs each contain 171 headers, zero multiset difference; no new-region diagnostic. |
| H7: full-suite failure is an obsolete ownerless legacy-emitter expectation. | Fixture synthetically sets owner `None`; base passes old expectation; updated terminal assertion is base RED/candidate GREEN. | Owner-bearing fixture or identical base failure. | `[MEASURED]` supported: old test base `1/1`, candidate `0/1`; converted populated-bucket negative base `0/1`, candidate `1/1`. |
| H8: the remaining three full-suite failures are obsolete ownerless R3 expectations rather than producer regressions. | Updated negatives fail only on exact-base legacy rows/edges; owner-bearing positives pass in the same binaries; candidate is green. | Compile/fixture failure, owner-bearing positive failure, or updated exact-base green. | `[MEASURED]` supported: no-fail-fast found no other target failure; fix2 base `1 passed/2 failed` vs candidate `3/3`, fix3 base `3 passed/1 failed` vs candidate `4/4`. Exact-base failures expose the old fallback telemetry, interface-dispatch edges, or manifest presence. |
| H9: the enumerated expectation updates close the full-suite failure population. | Complete captured rerun has zero failures and final doc-test summary. | Any failing summary, truncated log, or missing target completion. | `[MEASURED]` supported: 28 summaries aggregate to `3,488 passed`, `0 failed`, `1 ignored`; command exit `0`. |
| H10: Tier-A quick exit `2` is an accuracy regression. | Candidate report contains a case-level regression absent on base. | Ineligibility with identical exact-base control outcomes. | `[MEASURED]` falsified: candidate and base are both invalid only for corpus-SHA drift versus pinned `20c8490591a3`; both have `104/104` matrix `ok`, zero oracle/SUT errors, and identical pinned outcomes. Candidate matched inventory is `7,029` vs base `7,021`; missing counts remain `28`. |
| H11: the first base-control failure indicates bytes exhausted rather than an oracle failure. | `df` shows near-zero bytes and large reproducible debug targets; inode pool remains available. | Adequate bytes or exhausted inodes. | `[MEASURED]` supported: 117 MiB free, 5.5 GiB measured debug trees, 80% inode use. The ENOSPC run was discarded; cleaning only dev profiles recovered space, and the fresh release+quick control completed. |
| H12: Slice 3 preserves call-site occurrences and changes only ownerless recovered routes. | Five pinned corpora retain total call-site counts; ripgrep is byte-identical; Go deltas are terminal drops/removed manifest rows with no added or changed retained rows. | Any total-count drift, cross-language leaf delta, added row, or changed retained implementer set. | `[MEASURED]` supported: all five totals match; ripgrep call-stats SHA matches; caddy/prometheus/etcd/hugo removed `1/59/0/16` manifest rows and `0/23/0/12` edges, with zero additions or retained-row changes. |
| H13: removing ownerless rows introduces no new precision defect after PR #212. | Refreshed candidate oracles have zero newly Exact/blocking sites, full coverage, and no removed independently sound owner-bearing row. | A removed base row is independently sound and its receiver owner is syntactically provable. | `[PENDING]` the pre-prerequisite candidate failed this discriminator at Prometheus line 302; re-run the complete transition census and oracle join on the rebased composition. |
| H14: the Prometheus line-302 removal was an ended-scope false shadow, not an intentional same-scope reassignment drop. | The typed parameter encloses the call; the only later `b := ...` is in an inner loop whose block ends before the call; scope-aware declaration filtering restores the owner. | The call lies in the inner block, a later same-scope assignment changes `b`, or the repaired producer still leaves the call ownerless. | `[MEASURED]` supported and repaired by PR #212: its final release manifest retains line 302 with exact owner and the one sound implementer edge. Reproduction through the rebased Slice 3 consumer remains pending. |
| H15: rebasing preserved both prerequisite and Slice 3 test intent. | The sole conflict retains `manifest_fanout`, the cross-file alias fixture, both ownerless terminal tests, and all prerequisite scope/partition tests. | Any conflict marker, lost test, or whole-side resolution. | `[MEASURED]` supported: rebase completed after one additive conflict; `git diff --check` and marker search were clean before continuing. |

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
| Base | `0139d7fab18d71cfa33f9de609bf280674df85e8` |
| Rebased HEAD before custody refresh | `e5596ba4e91a39fbf1b4bf0affc5b7013365c835` |
| Planning commit | `cd727884` |
| RED checkpoint | `d723287b` |
| Implementation checkpoint | `9bf38001` |
| Implementation reconciliation | `b22f5ec3` |
| Integration-test checkpoint | `00d91c9f` |
| Integration-test reconciliation | `e58838b1` |
| Enumerated ownerless-test checkpoint | `4a0facb6` |
| Enumerated ownerless-test reconciliation | `f3e7d6fe` |
| Historical full-suite checkpoint | `4af62039` |
| Historical Tier-A checkpoint | `4dc60e29` |
| Historical five-corpus/oracle checkpoint | `f60ce6f3` |
| Historical review-round-1 STOP checkpoint | `c9bbb136` |
| Branch | `a-receiver-provenance-slice3-terminal-owner-predicate` |
| Worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` |
| Plan | `docs/superpowers/plans/2026-08-28-go-receiver-type-origin-binding-slice3.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-28-go-receiver-type-origin-binding-slice3-handoff.md` |
| Design | `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md` |
| Base cache pins | CPG `53`; sidecar `21` |
| Candidate cache pins | CPG `54`; sidecar `22` |
| Historical exact-base control worktree | `/private/tmp/slicing-s3-base-7fc719ae` |
| Historical candidate Tier-A evidence | `/private/tmp/slicing-s3-tier-a-candidate-ecbc8d6` |
| Historical exact-base Tier-A evidence | `/private/tmp/slicing-s3-tier-a-base-7fc719ae` |
| Historical five-corpus/oracle evidence | `/private/tmp/slicing-s3-receiver-oracle.UU5Ufy` |
| Post-rebase evidence | pending current verification |
| Review cap | `2` |

## 6. Historical pre-prerequisite five-corpus and oracle ledger

The table below remains custody evidence for the round-1 STOP and is not post-rebase acceptance evidence. Recompute it against base `0139d7fa` before publication.

| Corpus | Total call sites base/candidate | Manifest sites base/candidate | Removed rows / edges | Sound base/candidate | Recall gap base/candidate | Over-approx base/candidate | Delta gate |
|---|---:|---:|---:|---:|---:|---:|---|
| ripgrep | `14,169 / 14,169` | n/a | `0 / 0` | n/a | n/a | n/a | call-stats byte-identical |
| caddy | `20,594 / 20,594` | `452 / 451` | `1 / 0` | `42 / 42` | `15 / 15` | `0 / 0` | true; coverage `1.0 / 1.0` |
| prometheus | `110,647 / 110,647` | `3,089 / 3,030` | `59 / 23` | `693 / 688` | `269 / 251` | `0 / 0` | true; coverage `1.0 / 1.0` |
| etcd | `69,207 / 69,207` | `3,495 / 3,495` | `0 / 0` | `1,866 / 1,866` | `242 / 242` | `0 / 0` | true; coverage `1.0 / 1.0` |
| hugo | `58,681 / 58,681` | `1,802 / 1,786` | `16 / 12` | `586 / 582` | `364 / 356` | `0 / 0` | true; coverage `1.0 / 1.0` |

No manifest added a row or changed a retained keyed row/implementer set. Caddy removes one zero-fanout `not_dispatch` row. Prometheus removes five sound fanout-positive sites/23 interface edges plus 54 zero-fanout rows; its resolver also removes 31 ownerless constructor-local and six ownerless typed-parameter direct edges, so return-flow leaves move downstream. Hugo removes two fanout-positive sites/12 interface edges plus 14 zero-fanout rows; its resolver also removes four constructor-local and eight typed-parameter direct edges. Etcd's manifest/oracle is identical; seven legacy fallback attempts are suppressed outside the manifest denominator. Hugo's `extended` and `withdeploy` tag passes adjudicated all constrained sites. Every candidate oracle has zero newly Exact sites, zero blockers, and `gate_ok=true` against the freshly cut exact-base oracle.

Those historical aggregate gates did not authorize publication. Review found a concrete prerequisite violation at pinned Prometheus `prompb/io/prometheus/client/decoder.go:302`: the old base resolved `b.Add(...)` through local interface `unsafeLabelAdder` to `schema.IgnoreOverriddenMetadataLabelScratchBuilder`, while the pre-prerequisite candidate removed it because the AST binding census treated line 198's ended inner-block byte variable `b` as a live shadow. PR #212 is the separately reviewed bounded repair and is now the exact Slice 3 base; current corpus evidence must show that the sound row survives the terminal predicate.

## 7. Owner direction

Resolved: the owner authorized the prerequisite, then authorized push/merge and successor continuation. PR #212 merged, Slice 3 is rebased, and publication remains gated only by current verification and review round 2; coverage is explicitly waived as a wait condition.
