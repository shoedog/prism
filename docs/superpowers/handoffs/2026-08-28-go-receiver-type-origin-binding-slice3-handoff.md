# Handoff — Go receiver type-origin binding Slice 3 terminal owner predicate

**Written:** 2026-08-29T20:34:46Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` · `a-receiver-provenance-slice3-terminal-owner-predicate` · **Base:** `[MEASURED]` `7fc719ae21ba130c554c318c3f8306093a804c92` (`origin/main` at worktree creation)
**Predecessor:** Slice 2 terminal closeout PR #211, merged at `7fc719ae21ba130c554c318c3f8306093a804c92`; Codex continuation of Claude session `a3bf14f1-6b47-464b-ba09-fc62e2ad7efb`
**Truth ordering:** measured live state > explicit owner/design authority within scope > this handoff > earlier handoffs and summaries. Conflicts stay open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` dedicated clean worktree/branch; no subagent dispatched. Primary `slicing` worktree has unrelated untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` and remains untouched.
**Custody exposure:** `[MEASURED]` plan/initial handoff are committed at `afdfb70`, first reconciliation at `f0ff6f2`, RED matrix at `e218a6a`, RED reconciliation at `ca86f02`, implementation at `89ea24e`, implementation reconciliation at `d7542c4`, bounded integration-test contract repair at `1a1e016`, its reconciliation at `068fa9a`, enumerated fix2/fix3 ownerless contract repair at `52348c9`, its reconciliation at `aef6c0c`, full-suite verification at `ecbc8d6`, qualified Tier-A controls at `f3035c0`, five-corpus/oracle evidence at `938d7be`, and the review-round-1 STOP finding at `b3a51f6`; this refresh is the immediate custody reconciliation.
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
| Predicate contract | blocked by disproved prerequisite | Go caller + recovered type/recovery + absent owner is terminal only after every valid positive has an owner. Prometheus `prompb/io/prometheus/client/decoder.go:302` disproves that prerequisite: its typed parameter is falsely marked shadowed by an ended inner-block `b := ...`. |
| Planning custody | done | `[MEASURED]` plan and initial handoff committed together at `afdfb70`; preserve it. |
| RED matrix | done | `[MEASURED]` resolver parity `1/1` and navigation parity `1/1` are already green; manifest `0/1` fails on the exact unauthorized zero-fanout ownerless record; CPG pin `0/1` fails `53 != 54`; sidecar pin `0/1` fails `21 != 22`. |
| Implementation | done | `[MEASURED]` checkpoint `89ea24e`: one shared predicate has exactly two consumers; resolver/manifest terminalize before routing; CPG `54`/sidecar `22`; shadowed negative pins the earlier terminal boundary. |
| Verification | done with disclosed exclusions | `[MEASURED]` focused GREEN; full suite `3,488 passed / 0 failed / 1 ignored`; release GREEN. Clippy candidate/base each have 171 identical pre-existing diagnostic headers. Tier-A matrix `104/104 ok`; quick is ineligible on candidate/base solely for pinned corpus-SHA drift, with no candidate-only regression. Five-corpus call-site parity and all four refreshed exact-base oracle gates pass; quantified recall cost is in §6. |
| Review | round 1 stopped on WRONG | Declared cap: two rounds. `WRONG`: the candidate removes a sound interface-dispatch target at pinned Prometheus line 302. The producer repair is explicitly outside this slice, so the lane hit its named STOP condition before round 2. |
| Publication | blocked | Do not push/open/merge while the valid ownerless positive remains unresolved. |

## 3. Hypothesis/probe log

| Hypothesis | True observation | Falsifier / alternative | Result |
|---|---|---|---|
| H1: resolver and manifest are the only independent consumers; sidecar reuses resolver. | Resolver has ownerless special guard; manifest independently routes and runs legacy lookup; nav edge builder calls `resolve_call_site_full`. | A third path independently constructs receiver-derived edges; or manifest already delegates. | `[MEASURED]` supported; no third mint found. |
| H2: after Slices 0–2, any recovered Go site without owner is genuinely unproven. | All valid recovery forms retain exact owners; absent-owner rows are negative/materialized/shadow or explicit mutations. | A current positive resolves correctly while ownerless, indicating a missed producer. | `[MEASURED]` falsified in review round 1: pinned Prometheus `parseLabel` has typed parameter `b unsafeLabelAdder`; an unrelated inner-loop `b := dAtA[iNdEx]` ends at line 204, but the ordinary AST binding walk counts it at the line-302 `b.Add(...)` call. Exact base emits one sound implementer; candidate removes the row. |
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
| H13: removing ownerless rows introduces no new precision defect and exposes only the designed recall cost. | Refreshed candidate oracles have zero newly Exact/blocking sites, full coverage, and over-approximation does not rise; removed rows account exactly for classification movement. | A removed base row is independently sound and its receiver owner is syntactically provable. | `[MEASURED]` falsified: all aggregate gates passed, but the base oracle classifies Prometheus line 302 `sound` and the source proves the typed-parameter owner remains in scope. Candidate absence is therefore an unintended false negative hidden by aggregate delta acceptance. |
| H14: the Prometheus line-302 removal is an ended-scope false shadow, not an intentional same-scope reassignment drop. | The typed parameter encloses the call; the only later `b := ...` is in an inner loop whose block ends before the call; ordinary binding scan lacks a call-scope filter. | The call lies in the inner block, a later same-scope assignment changes `b`, or the ordinary scan filters bindings to enclosing scopes. | `[MEASURED]` supported: source lines 184/198-204/302 establish lexical scope; `receiver_type_evidence_in_fn_mode` invokes `walk_receiver_bindings` with scope filtering disabled outside the special same-scope-reuse path; base manifest has one edge to `IgnoreOverriddenMetadataLabelScratchBuilder`, and the independent base oracle marks it `sound`. |

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
| Enumerated ownerless-test reconciliation | `aef6c0c` |
| Full-suite verification checkpoint | `ecbc8d6` |
| Tier-A controls checkpoint | `f3035c0` |
| Five-corpus/oracle checkpoint | `938d7be` |
| Review-round-1 STOP checkpoint | `b3a51f6` |
| Branch | `a-receiver-provenance-slice3-terminal-owner-predicate` |
| Worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` |
| Plan | `docs/superpowers/plans/2026-08-28-go-receiver-type-origin-binding-slice3.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-28-go-receiver-type-origin-binding-slice3-handoff.md` |
| Design | `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md` |
| Base cache pins | CPG `53`; sidecar `21` |
| Candidate cache pins | CPG `54`; sidecar `22` |
| Exact-base control worktree | `/private/tmp/slicing-s3-base-7fc719ae` |
| Candidate Tier-A evidence | `/private/tmp/slicing-s3-tier-a-candidate-ecbc8d6` |
| Exact-base Tier-A evidence | `/private/tmp/slicing-s3-tier-a-base-7fc719ae` |
| Five-corpus/oracle evidence | `/private/tmp/slicing-s3-receiver-oracle.UU5Ufy` |
| Candidate release binary SHA-256 | `7fa0daaa12921b7b06a8f1d5d3e6479d76c5b249529c56479746ae45169f2529` |
| Exact-base release binary SHA-256 | `c538e12d6f83fb364823235ac210669173be8ed0cbe13515b0cc2a85908e4a0f` |
| Review cap | `2` |

## 6. Five-corpus and oracle ledger

| Corpus | Total call sites base/candidate | Manifest sites base/candidate | Removed rows / edges | Sound base/candidate | Recall gap base/candidate | Over-approx base/candidate | Delta gate |
|---|---:|---:|---:|---:|---:|---:|---|
| ripgrep | `14,169 / 14,169` | n/a | `0 / 0` | n/a | n/a | n/a | call-stats byte-identical |
| caddy | `20,594 / 20,594` | `452 / 451` | `1 / 0` | `42 / 42` | `15 / 15` | `0 / 0` | true; coverage `1.0 / 1.0` |
| prometheus | `110,647 / 110,647` | `3,089 / 3,030` | `59 / 23` | `693 / 688` | `269 / 251` | `0 / 0` | true; coverage `1.0 / 1.0` |
| etcd | `69,207 / 69,207` | `3,495 / 3,495` | `0 / 0` | `1,866 / 1,866` | `242 / 242` | `0 / 0` | true; coverage `1.0 / 1.0` |
| hugo | `58,681 / 58,681` | `1,802 / 1,786` | `16 / 12` | `586 / 582` | `364 / 356` | `0 / 0` | true; coverage `1.0 / 1.0` |

No manifest added a row or changed a retained keyed row/implementer set. Caddy removes one zero-fanout `not_dispatch` row. Prometheus removes five sound fanout-positive sites/23 interface edges plus 54 zero-fanout rows; its resolver also removes 31 ownerless constructor-local and six ownerless typed-parameter direct edges, so return-flow leaves move downstream. Hugo removes two fanout-positive sites/12 interface edges plus 14 zero-fanout rows; its resolver also removes four constructor-local and eight typed-parameter direct edges. Etcd's manifest/oracle is identical; seven legacy fallback attempts are suppressed outside the manifest denominator. Hugo's `extended` and `withdeploy` tag passes adjudicated all constrained sites. Every candidate oracle has zero newly Exact sites, zero blockers, and `gate_ok=true` against the freshly cut exact-base oracle.

Those aggregate gates do not authorize publication. Review of the removed sound rows found a concrete prerequisite violation at pinned Prometheus `prompb/io/prometheus/client/decoder.go:302`: exact base resolves `b.Add(...)` through local interface `unsafeLabelAdder` to `schema.IgnoreOverriddenMetadataLabelScratchBuilder`, and the base oracle independently labels that edge `sound`; candidate removes it because the AST binding census treats line 198's ended inner-block byte variable `b` as a live shadow. This is `WRONG`, with a real same-environment base/candidate regression and a bounded future regression fixture (typed interface parameter, inner-block same-name short declaration, post-block method call). The mechanical repair belongs in scope-aware owner/shadow production, which §4 explicitly forbids changing in this slice.

## 7. Owner questions

Owner direction is required because the named STOP condition fired. Choose whether to authorize a prerequisite slice repairing scope-aware Go receiver owner/shadow production, after which Slice 3 must be rebased and fully reverified, or to reject/defer Slice 3. Existing remote publication authority is not sufficient to publish a candidate with this open `WRONG`.
