# Handoff — #18 RTA empty-live fallback disposition

**Recorded:** 2026-08-31T02:50:48Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-18-rta-fallback` · `p18-rta-empty-live-fallback`
**Exact base:** `[MEASURED]` `fe506487e4629757a8f9d7afeaa8fa033c193489` (`origin/main`, PR #216 merge)

## 0. Verdict and authority

Roadmap #18 is **closed as superseded by later owner-provenance routing**. The historical ledger classifies the pre-fix behavior as a `WRONG`: when interface identity was lost, the bare-keyed empty-live table could mint extra Exact edges. That historical `131`-site / `14`-over-approx run was inherited from the roadmap and was not replayed in this lane. On current main there is no constructible production result in which an arbitrary legacy-table candidate reaches a resolver or manifest output. The residual bare-keyed table and `NonLocalConstructionFallback` counter are therefore a `SMELL`—misleading legacy build telemetry with no demonstrated wrong production output—not a current blocker.

No runtime, cache, or schema edit is authorized by this disposition because the requested identity gate already exists at every production consumer. The owner repeatedly authorized continuation, push, and merge. Coverage is not a publication wait condition by explicit owner direction.

Review-round cap: `2`. Round 1 is the source, history, test, and retained-evidence audit recorded below. A second round is reserved for the final diff and publication state; the cap will not be silently extended.

## 1. Mechanism proof

`GoTypeProvider::compute_interface_dispatch` still builds a legacy table keyed by bare `(interface_name, method_name)` and records `NonLocalConstructionFallback` when the live intersection is empty. `CallGraph::apply_go_interface_dispatch` stores that table. An exhaustive `interface_impls` reference census found these consumers:

| Consumer | Reads | Edge authority |
|---|---:|---|
| `resolve_call_site_full` / `go_interface_dispatch_outcome` | 3 | No. Every read feeds `go_visible_s4_implementers`; its candidate parameter is `_candidates` and is ignored. |
| `interface_dispatch_manifest_inner` | 4 | No. Owner-bearing routes feed the same helper; the terminal route obtains `terminal_targets` from `go_proven_interface_outcome`. |
| `call_stats` | 1 | No. It builds an `interface_fanout` histogram only. |
| CallGraph construction | 1 write | No. It stores the provider result. |

There is no other source read of `interface_impls` and no direct edge emitter from `compute_interface_dispatch`.

`go_visible_s4_implementers` first recovers or accepts a full `GoOwnerIdentity`, constructs the interface identity with package directory, package clause, and name, and calls `go_caller_scoped_satisfiers`. That consult selects interface declarations by the full identity, checks that each defining file agrees with the owner, applies exact caller/build visibility, compares structural signatures against declaration-keyed concrete owners, and fails closed on conflicts or uncertainty. Only after that proof does `go_choose_live_satisfiers` use the empty-live fallback. `go_proven_interface_outcome` uses the same owner-qualified walk directly.

## 2. Hypothesis–probe–result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| H1: the legacy table still directly mints production edges. | A resolver or manifest path copies table IDs into Exact targets. | Every output path recomputes or obtains owner-qualified targets; the table is telemetry-only. | **Falsified.** All three resolver reads and all four manifest reads revalidate or bypass the table; the remaining read is a histogram. |
| H2: consumer revalidation is only nominal. | The helper iterates the supplied candidate set or selects an interface by bare name. | `_candidates` is ignored and interface declarations are selected by full owner identity with exact provenance. | **Falsified.** The helper rebuilds caller-scoped satisfiers from declaration snapshots. |
| H3: the roadmap row predates the authority membrane. | The row's commit predates the consumer rewrite. | The row was authored after the current helper and intentionally targets it. | **Supported.** Row #18 came from `18b585a` on 2026-08-23; the shared owner-qualified helper came from `b529d295` on 2026-08-30 and the terminal #16 route subsequently merged in PR #215. |
| H4: NLCF counts imply current fallback-minted edges. | Nonzero counters correlate with oracle over-approx/newly-Exact sites. | Counters remain nonzero while identity-aware oracle gates are clean. | **Falsified.** Retained post-#16 counts are `3/578/330/146`, while all four oracle deltas have no blockers or newly-Exact sites. The counters describe table construction, not emitted edges. |

Alternative mechanisms considered and ruled out: a manifest-only bypass (`terminal_targets` comes from the proven consult), same-named cross-package interface selection (full owner key), missing/corrupt profile provenance (uncertain drop), and a telemetry-to-edge conversion outside resolution (exhaustive source census found none).

The first broad resolver read was truncated and is inadmissible. It was replaced by bounded line-range reads. No conclusion depends on the truncated output.

## 3. Behavioral guards

Existing end-to-end tests already discriminate the historical failure mechanism:

- `direct_interface_identity_never_reintroduces_a_bare_signature_decoy` supplies `factory.Doer.Act(string)` with a non-live correct implementer and a live `other.Doer.Act(int)` decoy. Both resolver and manifest return only `factory.Local`.
- `s4_identity_never_reintroduces_a_bare_signature_decoy` exercises the embedded-interface/S4 route with the same live-decoy shape. Both resolver and manifest return only the owner-qualified local implementer.
- `receiver_owner_carrying_profileless_fact_is_materialized_drop` proves missing owner/profile authority drops rather than falling back to a same-named decoy.
- `empty_live_fires_fallback_full_set` intentionally preserves the provider-table behavior and NLCF telemetry. It is a table-construction contract, not an edge-authority contract.

A new table-mutation test would duplicate the stronger source-valid decoy fixtures and would pass before this docs-only disposition; it would not provide a red-first implementation proof. No new behavior is claimed.

## 4. Retained corpus evidence

The post-#16 Task 4 artifacts were re-parsed during this disposition. They were produced from the implementation now merged in PR #215; PR #216 changed documentation only.

| Corpus | NLCF table events | Oracle delta |
|---|---:|---|
| Caddy | 3 | gate pass; no blockers/newly-Exact sites |
| etcd | 578 | gate pass; no blockers/newly-Exact sites |
| Hugo | 330 | gate pass; no blockers/newly-Exact sites |
| Prometheus | 146 | gate pass; no blockers/newly-Exact sites |

Retained call-stats/manifests: `/private/tmp/p16-task4-corpora-20260831T0204Z`. Retained oracle reports: `/private/tmp/p16-task4-oracle-20260831T0230Z`. Their manifest custody and hashes were established in `2026-08-31-go-interface-identity-task4-evidence.md`; this disposition re-parsed the JSON values but did not rerun the corpora or oracle.

## 5. Confidence and downgrade discipline

Confidence that the historical `WRONG` is not reachable on current main increases with the exhaustive source census, the ignored-candidate mechanism, full-identity declaration selection, and both resolver/manifest decoy fixtures. It would decrease if a new `interface_impls` consumer appeared, if `go_visible_s4_implementers` began trusting its candidate parameter, or if an external supported API contract made the raw table edge-authoritative. It would collapse upon any constructible current source fixture where identity loss plus empty live selection emits a table-only target.

The historical finding is not erased: it remains valid for the 2026-08-22 pre-fix head. The current downgrade to `SMELL` is based on a mechanism-level proof that the flagged table cannot affect current production outputs, not on failure to find a counterexample.

## 6. Verification and review

Completed local verification for this docs-only closure:

| Gate | Result |
|---|---|
| Direct-interface and S4 live-decoy guards | `2` passed, `0` failed |
| Profileless owner-authority drop | `1` passed, `0` failed |
| Provider empty-live/NLCF contract | `1` passed, `0` failed |
| `cargo fmt --all -- --check` | exit `0` |
| `cargo check` | exit `0` |
| `cargo clippy --all-targets --all-features` | exit `0`; repository warning inventory only, no code diff |
| `git diff --check` | exit `0` |

The first focused command used nonexistent per-file Cargo targets and the first exact library selector selected zero tests. Its final shell status masked those failures, so the entire observation is inadmissible. Corrected `--list` probes found exactly the four selectors above before the successful run. Attempts to call `tools/check-links.sh` and `tools/check-codegen.sh` are also inadmissible: those scripts do not exist in this repository, and the tracked workflows expose no project-specific documentation checker. CI's actual format command was run locally.

Review round 2 found zero `WRONG` and one bounded `SMELL`: two metadata lines retained Markdown hard-break whitespace, causing the staged diff check to fail. The whitespace was removed and the post-fix staged diff check passed. The declared cap closes with zero open findings. Because no resolution, navigation, CPG, cache, or executable code changed, release rebuild, Tier-A, full-corpus, and oracle reruns are excluded; retained post-#16 evidence is cited as re-parsed rather than freshly executed.

## 7. Publication and next queue

Publication is complete. PR #217 squash-merged at 2026-08-31T03:02:35Z as `d293f7119a070424bee1669a369c30c8da46bed5`. Format Check, Clippy Lint, Test Suite (including MCP tests), and Language Coverage Matrix passed. Coverage was pending and intentionally not awaited. The first `gh pr merge --delete-branch` process returned a local error because `main` is already checked out in `/Users/wesleyjinks/code/slicing`; a direct PR-state query confirmed that the server-side merge had completed, so the error is local cleanup only. The primary worktree remains untouched and dirty only with its pre-existing untracked entries.

After publication, rebind the next open implementation rather than trusting the stale August 23 ordering. Already merged: oracle tag-set coverage (#189), R1(b) promoted routing (#190), docs/hygiene (#188/#194), return-flow taint (#193), receiver provenance (#209–#214), and #16 (#215/#216). The latest durable queue still names #13 Go-first sound Level-3 callbacks (measurement gate met) and #4b dot-import implementation (settled design, intended after #16); choose only after rebinding current branches, artifacts, and exact main.

## 8. Identifiers and exclusions

| Item | Value |
|---|---|
| Branch | `p18-rta-empty-live-fallback` |
| Closeout branch | `p18-rta-empty-live-fallback-closeout` |
| Worktree | `/Users/wesleyjinks/code/slicing-18-rta-fallback` |
| Exact base | `fe506487e4629757a8f9d7afeaa8fa033c193489` |
| Historical row commit | `18b585a50` |
| Owner-qualified helper commit | `b529d295` |
| #16 implementation merge | PR #215 · `d21c8cb4ae4f4a2a6b43ce66ea4ed5a76f8a15a9` |
| #16 docs closeout / current base | PR #216 · `fe506487e4629757a8f9d7afeaa8fa033c193489` |
| #18 publication | PR #217 · `d293f7119a070424bee1669a369c30c8da46bed5` |
| Review cap | `2` |

Not verified here: a fresh full suite, Tier-A, new corpus builds, or new oracle execution. Those checks exercised the same executable main during #16 and are unnecessary for this docs-only no-behavior diff. Focused guards, format, check, ordinary Clippy, and diff validation passed locally. Coverage is intentionally not awaited under owner direction.
