# Handoff — Go receiver type-origin binding Slice 3 terminal owner predicate

**Written:** 2026-08-31T00:04:03Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` · `a-receiver-provenance-slice3-terminal-owner-predicate` · **Base:** `[MEASURED]` `0139d7fab18d71cfa33f9de609bf280674df85e8` (`origin/main`, scope-aware owner prerequisite PR #212 merge)
**Predecessor:** scope-aware owner prerequisite PR #212, merged at `0139d7fab18d71cfa33f9de609bf280674df85e8`; the original Slice 3 lane followed Slice 2 PR #211 and continues Claude session `a3bf14f1-6b47-464b-ba09-fc62e2ad7efb`
**Truth ordering:** measured live state > explicit owner/design authority within scope > this handoff > earlier handoffs and summaries. Conflicts stay open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` dedicated worktree/branch; no subagent dispatched. Rebase onto PR #212 completed at `e5596ba4e91a39fbf1b4bf0affc5b7013365c835` after one additive test-fixture conflict; current pre-reconciliation HEAD is `59754aec0bcdd3f8e04f79199fe6c94c0e23c182`; primary `slicing` worktree remains untouched.
**Custody exposure:** `[MEASURED]` the 15-commit Slice 3 chain was preserved and rebased: planning `cd727884`, RED `d723287b`, implementation `9bf38001`, expectation repairs `00d91c9f` and `4a0facb6`, historical full/Tier-A/corpus checkpoints `4af62039`/`4dc60e29`/`f60ce6f3`, and historical review STOP `c9bbb136`; rebase custody is `59754ae`. Current post-rebase verification artifacts are hashed below and this refresh records final review round 2.
**In flight / irreversible:** none.
**Authority:** owner repeatedly said `authorized` and explicitly authorized push/merge; publish and merge remain authorized once non-coverage gates and the bounded review are green.

## 1. Resume order

1. Require base `0139d7fab18d71cfa33f9de609bf280674df85e8` and Slice 3 HEAD at or after `e5596ba4`.
2. Preserve the rebased 15-commit implementation chain; do not fold production code or discard the reviewed artifact.
3. Preserve the current focused/full/Clippy/release/Tier-A/corpus evidence and round-2 approval.
4. Push the branch, open the PR, and require all relevant non-coverage checks green; coverage is not a wait condition by owner direction.
5. Merge using the repository's merge-commit convention, then reconcile this handoff's publication state.

**STOP conditions:** a valid Go recovered positive remains ownerless; a third independent receiver-edge consumer appears; the predicate changes non-Go/direct behavior; an edit populates owners or changes `CallSite::cmp_key`; a failing probe selects zero tests or fails for its own reason; generated eval output would be committed; or a same-environment exact-base control disproves attribution.

## 2. State ledger

| Item | State | Evidence / next action |
|---|---|---|
| Base/prerequisite landing | done | `[MEASURED]` exact base is PR #212 merge `0139d7fa`; it contains Slice 2 merge `7fc719ae` and the scope-aware owner repair. |
| Design/census | done | `[MEASURED]` design §§1–8, `CallSite`, rematerialization, resolver, manifest, navigation edge builder, ownerless assertions, and cache pins read on exact base. Exactly two independent consumers found; sidecar reuses resolver. |
| Predicate contract | done | `[MEASURED]` the rebased composition retains pinned Prometheus line 302 with exact `prompb.unsafeLabelAdder` owner semantics, `interface_dispatch`, fanout `1`, and the `schema/labels.go` implementer. |
| Planning custody | done | `[MEASURED]` rebased planning commit is `cd727884`; preserve it. |
| RED matrix | done | `[MEASURED]` resolver parity `1/1` and navigation parity `1/1` are already green; manifest `0/1` fails on the exact unauthorized zero-fanout ownerless record; CPG pin `0/1` fails `53 != 54`; sidecar pin `0/1` fails `21 != 22`. |
| Implementation | done | `[MEASURED]` rebased checkpoint `9bf38001`: one shared predicate has exactly two consumers; resolver/manifest terminalize before routing; CPG `54`/sidecar `22`; shadowed negative pins the earlier terminal boundary. |
| Verification | done with one base-proven exclusion | `[MEASURED]` focused `54/54`; full suite 28 summaries `3,496/0/1`; fmt/check and ordinary all-target Clippy compile pass; exact `-D warnings` fails on candidate/base with `165/165` normalized diagnostics and zero differences; release, Tier-A, five-corpus, Prometheus sentinel, and four oracle gates are recorded below. |
| Review | done after round 2 | Declared cap `2`. Round 1 found one `WRONG`; PR #212 repaired it. Round 2 found zero `WRONG` and zero in-scope `SMELL` after the complete `0/30/0/16` removed-row census, source-mechanism inspection, consumer/order audit, and exact corpus controls. |
| Publication | authorized and pending | Push/open/merge after relevant non-coverage CI checks; coverage need not be awaited per owner direction. |

## 3. Hypothesis/probe log

| Hypothesis | True observation | Falsifier / alternative | Result |
|---|---|---|---|
| H1: resolver and manifest are the only independent consumers; sidecar reuses resolver. | Resolver has ownerless special guard; manifest independently routes and runs legacy lookup; nav edge builder calls `resolve_call_site_full`. | A third path independently constructs receiver-derived edges; or manifest already delegates. | `[MEASURED]` supported; no third mint found. |
| H2: after Slices 0–2 plus PR #212, any recovered Go site without owner is outside the admitted producer set. | All admitted recovery forms retain exact owners; remaining absence is negative/materialized/shadow, explicit mutation, or a parked assignment/reuse/field-alias form. | A declaration-backed in-scope producer loses its owner, as line 302 did before PR #212. | `[MEASURED]` supported after the prerequisite: fresh rebased Prometheus retains line 302 at fanout `1`; all `30/16` Prometheus/Hugo removals were enumerated, and their sound subset is method-return plus assignment, map-index reuse, external zero-fanout, or closure field-alias capture—each already parked outside this slice. |
| H3: removing the real cross-file site's owner reconstructs the caller-alias rebinding defect in all consumers. | Resolver/manifest/nav expose `decoy/types.go` before the predicate. | The current route already drops, or failure is caused by malformed mutation. | `[MEASURED]` falsified for resolver/nav: both already drop. Manifest alone emits the ownerless zero-fanout record, proving the bounded parity defect. |
| H4: the shared predicate flips only the manifest RED while preserving resolver/nav controls. | Parity/pin selectors turn green without positive regression. | Manifest remains present or resolver/nav acquire an edge/regression. | `[MEASURED]` supported: language parity `2/2`, navigation `1/1`, pins `2/2`. |
| H5: early terminalization changes the shadowed collision-bail telemetry/manifest record. | Candidate fails the old telemetry/manifest expectations while exact base passes them. | Exact base fails identically. | `[MEASURED]` attributed: old expectations candidate `2/4`, exact base `4/4`; updated contract exact base RED `2/4`, candidate GREEN `4/4`. Edge/owner result remains unchanged. |
| H6: Clippy failures are exact-base-identical repository debt. | Candidate/base normalized diagnostic multisets match with no changed-region diagnostic. | Any diagnostic delta or new-code diagnostic. | `[MEASURED]` supported: exact `-D warnings` exits `101` on candidate and exact base with `165/165` normalized diagnostics and zero differences; ordinary all-target/all-feature Clippy exits `0` with 214 warning lines. |
| H7: full-suite failure is an obsolete ownerless legacy-emitter expectation. | Fixture synthetically sets owner `None`; base passes old expectation; updated terminal assertion is base RED/candidate GREEN. | Owner-bearing fixture or identical base failure. | `[MEASURED]` supported: old test base `1/1`, candidate `0/1`; converted populated-bucket negative base `0/1`, candidate `1/1`. |
| H8: the remaining three full-suite failures are obsolete ownerless R3 expectations rather than producer regressions. | Updated negatives fail only on exact-base legacy rows/edges; owner-bearing positives pass in the same binaries; candidate is green. | Compile/fixture failure, owner-bearing positive failure, or updated exact-base green. | `[MEASURED]` supported: no-fail-fast found no other target failure; fix2 base `1 passed/2 failed` vs candidate `3/3`, fix3 base `3 passed/1 failed` vs candidate `4/4`. Exact-base failures expose the old fallback telemetry, interface-dispatch edges, or manifest presence. |
| H9: the enumerated expectation updates close the full-suite failure population. | Complete captured rerun has zero failures and final doc-test summary. | Any failing summary, truncated log, or missing target completion. | `[MEASURED]` supported post-rebase: persistent no-fail-fast run completed 28 summaries at `3,496 passed / 0 failed / 1 ignored`, exit `0`. |
| H10: Tier-A quick exit `2` is an accuracy regression. | Candidate report contains a case-level regression absent on base. | Ineligibility with identical exact-base control outcomes. | `[MEASURED]` falsified post-rebase: candidate and exact base are invalid only for their respective SHA drift versus pinned `20c8490591a3`; both have `104/104`, zero oracle/SUT errors, identical pinned outcomes, missing `28`, and extra `0`. Candidate matched inventory is `7,042` versus base `7,034`. |
| H11: corpus verification is contaminated by the earlier ENOSPC mechanism. | Current filesystem has inadequate byte/inode headroom or a corpus probe fails during writes. | Ample headroom and every corpus/oracle probe exits normally. | `[MEASURED]` falsified for the current run: `313 GiB` available, negligible inode pressure, and all call-stats/manifest/oracle probes exited `0`. |
| H12: Slice 3 preserves call-site occurrences and changes only ownerless recovered routes. | Five pinned corpora retain total call-site counts; ripgrep is byte-identical; Go deltas are terminal drops/removed manifest rows with no added or changed retained rows. | Any total-count drift, cross-language leaf delta, added row, or changed retained implementer set. | `[MEASURED]` supported post-rebase: all five totals match; ripgrep is byte-identical; Caddy/Prometheus/etcd/Hugo remove `0/30/0/16` rows and `0/22/0/12` edges, with zero additions or changed retained rows. |
| H13: removing ownerless rows introduces no new precision defect after PR #212. | Refreshed candidate oracles have zero newly Exact/blocking sites, full coverage, and no removed admitted owner-bearing row. | A removed row belongs to an admitted producer yet loses an exact owner. | `[MEASURED]` supported: all four delta gates are true with full coverage and zero over-approx/timeout/unresolved/target-mismatch. The eight removed rows classified sound are completely accounted for by parked method-return/assignment, map-index reuse, external zero-fanout, and closure field-alias forms. |
| H14: the Prometheus line-302 removal was an ended-scope false shadow, not an intentional same-scope reassignment drop. | The typed parameter encloses the call; the only later `b := ...` is in an inner loop whose block ends before the call; scope-aware declaration filtering restores the owner. | The repaired composition leaves the call ownerless or removes its edge. | `[MEASURED]` supported and repaired: fresh base and rebased candidate both emit byte-identical line-302 rows with `interface_dispatch`, fanout `1`, and `schema/labels.go::IgnoreOverriddenMetadataLabelScratchBuilder`. |
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
| Verified HEAD before final reconciliation | `59754aec0bcdd3f8e04f79199fe6c94c0e23c182` |
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
| Exact merged-base control worktree | `/private/tmp/slicing-s3-base-0139d7fa` |
| Focused tests | seven selectors · `54/54` |
| Full-suite log | `/private/tmp/slicing-s3-full-59754ae.log` · 28 summaries · `3,496/0/1` |
| Exact Clippy logs | `/private/tmp/slicing-s3-clippy-candidate-59754ae.log`; `/private/tmp/slicing-s3-clippy-base-0139d7fa.log` · normalized `165/165`, zero differences |
| Candidate release binary | `target/release/prism` · SHA-256 `595bc05fcf5de441673ebf458f1531d53b7aaa98f1f59acdddf75bf08d216255` |
| Exact-base release binary | `/private/tmp/slicing-s3-base-0139d7fa/target/release/prism` · SHA-256 `73cf16d7147e8ec8709d45e81788b7bc9aa523de0a8a8fc5a99b22720d65ba70` |
| Candidate Tier-A evidence | `/private/tmp/slicing-s3-tier-a-candidate-59754ae` · report SHA-256 `bb4850412e13b3f4559d1e56e8f5edb0da714257ab17532da72d41ddca68dfa6` · snapshot `d55237604e5ebbe67579e3cc80a30036951ee6b51c7add60409ae9856ad84599` |
| Exact-base Tier-A evidence | `/private/tmp/slicing-s3-tier-a-base-0139d7fa` · report SHA-256 `0481b0823d0585146b65207872779639dec821461548545688df657e5d184b18` · snapshot `51b83b807175db3944afdcb99ae1984c5e871c1b7e85c68c3b59e6ffe29b1740` |
| Five-corpus/oracle evidence | `/private/tmp/slicing-s3-postrebase-corpus-59754ae` · 34-file `SHA256SUMS` SHA-256 `a16288e735b17b064f1e7b6da6045b0dccf7e4b0b3e3de9ba0664931a57a9c8a` |
| Prometheus call-site custody | `/private/tmp/slicing-s3-prometheus-candidate-sites-59754ae.json` · 110,647 records |
| Review cap | `2` |

## 6. Post-rebase five-corpus and oracle ledger

| Corpus | Total call sites base/candidate | Manifest sites base/candidate | Removed rows / edges | Sound base/candidate | Recall gap base/candidate | Over-approx base/candidate | Delta gate |
|---|---:|---:|---:|---:|---:|---:|---|
| ripgrep | `14,169 / 14,169` | n/a | `0 / 0` | n/a | n/a | n/a | call-stats byte-identical |
| caddy | `20,594 / 20,594` | `452 / 452` | `0 / 0` | `42 / 42` | `15 / 15` | `0 / 0` | true; coverage `1.0 / 1.0` |
| prometheus | `110,647 / 110,647` | `3,107 / 3,077` | `30 / 22` | `889 / 885` | `77 / 59` | `0 / 0` | true; coverage `1.0 / 1.0` |
| etcd | `69,207 / 69,207` | `3,504 / 3,504` | `0 / 0` | `1,870 / 1,870` | `242 / 242` | `0 / 0` | true; coverage `1.0 / 1.0` |
| hugo | `58,681 / 58,681` | `1,808 / 1,792` | `16 / 12` | `590 / 586` | `365 / 357` | `0 / 0` | true; coverage `1.0 / 1.0` |

Every total call-site count is conserved. No manifest adds a row or changes a retained row/implementer set. Caddy and etcd manifests are identical; their only call-stats movement is terminal suppression of `5` and `7` zero-hit fallback attempts. Prometheus removes 30 rows: 22 zero-fanout rows and four sound var-local sites carrying 22 edges; Hugo removes 16 rows: 12 zero-fanout rows and two sound typed-parameter sites carrying 12 edges. Resolver leaf changes and downstream return-flow movements reconcile with those terminal drops.

The removed sound subset was inspected completely. Prometheus's four sites are an inferred method-return receiver followed by `=` assignment and a map-index comma-ok reuse; Hugo's two fanout-positive sites are closure-captured field aliases, while two additional oracle-`sound` rows are external zero-fanout `io/fs.DirEntry` calls. These are the already parked assignment/reuse/field-alias/external forms, not admitted owner-bearing producers. The former blocker at `prompb/io/prometheus/client/decoder.go:302` remains byte-identical between fresh base and candidate at `interface_dispatch`, fanout `1`, targeting `schema/labels.go::IgnoreOverriddenMetadataLabelScratchBuilder`.

## 7. Owner direction

Resolved: the owner authorized the prerequisite, then authorized push/merge and successor continuation. PR #212 merged, Slice 3 is rebased and approved after review round 2, and publication is now gated only by relevant non-coverage CI checks; coverage is explicitly waived as a wait condition.

## 8. Review verdict

**APPROVE AFTER ROUND 2.** `WRONG`: zero. `SMELL`: zero in scope. Confidence decreases if a removed site is shown to belong to the admitted owner-producer set and collapses if the repaired line-302 edge disappears or resolver/manifest parity diverges; the complete current census found neither. Exact `-D warnings` remains a disclosed base-identical repository exclusion, not a candidate finding.
