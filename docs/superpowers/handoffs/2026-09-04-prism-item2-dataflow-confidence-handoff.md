# Handoff — Prism roadmap item 2 reaching-definitions closeout

**Written:** 2026-09-05 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-item2-sol` · branch `item2-dataflow-confidence`
**Custody tuple:** `[MEASURED]` source head `7f3b2257c1d35738cc96c9345faa60c212290e5b` is the last source-changing commit; the closeout-docs commit carrying this handoff and roadmap row 19 sits directly on top of it; the tree is clean at that closeout-docs commit. Because a commit cannot contain its own SHA, identify it with `git log --oneline -2`: the top commit is the closeout-docs commit and its parent must be `7f3b225`. Final proof is recorded in `/Users/wesleyjinks/code/tools/logs/item2-task7b-step14.log`.

**Truth ordering:** measured live state > binding spec and explicit owner authority within their scope > this handoff > earlier summaries. `[MEASURED]` claims were probed in this run; `[INHERITED]` claims identify their source.

## 0. Gating facts

- `[MEASURED]` No sub-agents were spawned. No push or PR action was attempted.
- `[MEASURED]` Whole-branch fix wave 1 closed two WRONG and two SMELL findings. Main fix commit: `a690dfb40eaacefb5371d5db6f429fbfc9e5fb80`. Bounded clippy correction and final source head: `7f3b2257c1d35738cc96c9345faa60c212290e5b`.
- `[MEASURED]` The ignored Task 7 report contains `## Whole-branch fix wave 1 + Step 17`. Generated Tier-A report/snapshot files from the rerun were removed before final custody; no oracle was edited or committed.
- `[MEASURED]` Disk was 27 GiB free at Step 1 and 11 GiB during closeout, above the 8 GiB prune threshold and 3 GiB stop threshold.
- `[INHERITED]` Push remains controller-owned and requires explicit authorization.

## 1. Resume order

1. Run `git rev-parse HEAD`, `git log --oneline -2`, and `git status --short --untracked-files=all`. Expect a clean closeout-docs commit whose parent is `7f3b225`.
2. Read `.superpowers/sdd/2026-09-04-prism-item2-dataflow-confidence/task-7-report.md` and `/Users/wesleyjinks/code/tools/logs/item2-task7b-step{1..14}.log`.
3. Before any new binding semantics, read binding spec v6.5.5 and obtain a new ruling for B-open. Keep B8 provisional.
4. If the controller authorizes it, push and open the PR with cache 55→56, Task 0b, byte controls, Tier-A validity exclusions, and fix-wave convergence disclosed. Do not merge.

**STOP conditions:** a new source change during closeout; a different parent below the closeout-docs commit; dirty tracked state; a changed pre-existing call-stats leaf; a new Tier-A quick invalid reason; or missing push authorization.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Review item 1 | done | `[MEASURED]` Unclassified Rust binding-looking constructs on a def/use route force the flat-path kill equation. RED: if-let, match-arm, while-let were Exact; GREEN: Killed at 5, 6, 5. Classified controls remain Exact. |
| Review item 2 | done | `[MEASURED]` Capture detection consumes `Language::callable_boundary_node_types()`. Rust async/gen captures are `CfgIncomplete`; immediate block remains Exact; every language boundary kind agrees. B8 remains provisional. |
| Review items 3–4 | done | `[MEASURED]` Labels-only PartialHit replacement now checks identities, spans, graph payloads, counters, and subsequent Hit. CLI test name states default/explicit equivalence. |
| Spec | done | `[MEASURED]` Repo amendment 1c mirrors binding spec v6.5.5 verbatim. New `if let` semantics were not added to the ownership table. |
| Steps 1–3 | done | `[MEASURED]` Source tuple frozen at `7f3b225`; fmt/diff 0; same-worktree clippy candidate/base `172/172`, normalized candidate-only set empty. |
| Steps 4–7 | done | `[MEASURED]` Focused `435/0/0`; full suite plus doctests `3978/0/1`; byte controls `1598/1598` and `272/272`; cache selector `2/0/0`. |
| Step 8 | comparative pass, absolute validity excluded | `[MEASURED]` Release build 0; matrix `159/0/0`. Candidate and base quick share only corpus SHA drift and C-name `4/6`; canonical non-matrix diff 0. |
| Steps 9–10 | done | `[MEASURED]` Call-stats `11/11`, all additive payloads present and all pre-existing normalized leaves identical. Exact/Killed: prism `83,475/2,208`, caddy `42,331/14,703`, mypy `109,619/9,970`. |
| Steps 11–12 | done | `[MEASURED]` One cache 55→56 transition, sidecar 24, Cargo diff 0; four producers, nine delivery cases, six labeled-walk tests, four valid fresh DFG JSONL records. |
| Steps 13–14 | done | `[MEASURED]` Roadmap row 19 updated to v6.5.5; row 20 retained once; this handoff and final custody validated. |

## 3. Corrections and convergence

- The first Step 3 rerun measured 173 candidate diagnostics versus 172 at the same-worktree base. One new `needless_borrow` came from the review fix. It was corrected in `7f3b225`, then Steps 1–14 restarted from that source head. Final counts are `172/172` with no candidate-only normalized headline.
- Focused totals moved from the prior closeout's `427/0/0` to `435/0/0`. Full totals moved from `3969/0/1` to `3978/0/1`.
- Roadmap row 19 previously named v6.5.4 and described the pre-fix ownership state. It now names v6.5.5 and records the implemented fail-closed uncertainty rule.
- The prior handoff named source head `357221b`, one cache test, old non-inert counts, and `item2-task7-step*` logs. This handoff supersedes those values with the Step 17 rerun.
- Memory was not changed because the owner did not request a memory update.

## 4. Open work

| Work | State | Required next action |
|---|---|---|
| Push and PR | pending | Obtain explicit controller authorization, then push/open one PR; do not merge. |
| Review wave 2 | available under declared cap | Run only if dispatched; classify findings with concrete failure cases before source edits. |
| New binding semantics | parked | Obtain a binding-spec amendment under B-open. Amendment 1c's fail-closed default is already implemented. |
| B8 capture rule | provisional | Keep `CfgIncomplete` behavior until the owner settles or replaces B8. |
| Tier-A absolute validity | pending outside this slice | Re-anchor corpus SHA and restore C-name 6/6 before claiming an absolute quick pass. |
| Dependency hints | roadmap next | Implement row 20 in its own authorized slice. |

## 5. Invariants and traps

- Ownership must be established for every binding-looking construct of the name on relevant def/use routes before per-binding classification is allowed. Otherwise use the unconditional flat-path kill equation. “Not a recognized declaration” does not license resolving to an outer binding.
- Do not add `if let`, `while let`, match-arm, for-pattern, parameter, or comprehension semantics without a binding-table ruling.
- Use the language layer as the single source of callable-boundary kinds. B8's current result is provisional.
- Do not rebaseline the generated 2026-09-05 Tier-A artifacts. Candidate and control share the same two validity exclusions.
- Do not remove any call-stats field except additive `.dfg_labels` in the compatibility comparison.
- Do not count `dfg_label_loop_carried` as a seventh disjoint partition; it is a subset of Exact.
- Preserve Task 0b as the sole designated behavior commit and cache v56 as the sole CPG transition.

## 6. Identifiers

| Item | Value |
|---|---|
| Merge base | `bffb84750d97f80bfdbeafa8a7cb58ea4f63b8fd` |
| Review fix | `a690dfb40eaacefb5371d5db6f429fbfc9e5fb80` |
| Last source-changing commit | `7f3b2257c1d35738cc96c9345faa60c212290e5b` |
| Closeout-docs commit | `SELF` — commit carrying this handoff, directly on top of `7f3b225` |
| Task 0b behavior commit | `81f2943` |
| Task 0b control binary | `/Users/wesleyjinks/code/tools/bin/prism-item2-0b` · source `0b51138aa6fe` · SHA-256 `897840b5d5222dc463bd5ff883ebd476dbd0fde43f752882e5e6e8761343c9ae` |
| Base binary | `/Users/wesleyjinks/code/tools/bin/prism-base-bffb847` · SHA-256 `a7b4f69a044225edf310710d78fdcdda04e005af1cd3eeddff9154e5ac3c5b06` |
| Candidate release binary | SHA-256 `9aa44d47ecffe12b5f73391569e68155491374c4e61d7a4598d6a28a9be1391f` |
| Cache versions | CPG 56; navigation sidecar 24 |
| RED logs | `/Users/wesleyjinks/code/tools/logs/item2-final-fix1-red-{1..4}.log` |
| Gate logs | `/Users/wesleyjinks/code/tools/logs/item2-task7b-step{1..14}.log` |
| Task 7 report | `.superpowers/sdd/2026-09-04-prism-item2-dataflow-confidence/task-7-report.md` |
| Controller session | `https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT` |

## 7. Verdict and owner questions

**Verdict:** SURVIVED · Whole-branch fix wave 1 and Step 17 pass at source head `7f3b225`, with the Tier-A quick absolute-validity exclusions stated above. Evidence tier: test-backed and same-base controlled. Review cap consumed: wave 1 of 2.

**Owner questions:** authorize or decline push/PR; decide whether to dispatch review wave 2; settle B8 before broader capture semantics; assign the Tier-A re-anchor outside this slice.
