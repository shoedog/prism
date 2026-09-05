# Handoff — Prism roadmap item 2 reaching-definitions closeout

**Written:** 2026-09-05 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-item2-sol` · branch `item2-dataflow-confidence`
**Custody tuple:** `[MEASURED]` source head `8858f5b786a77637532ef935c4e4530ba34cff3c` is the last source-changing commit; the closeout-docs commit carrying this handoff and roadmap row 19 sits directly on top of it; the tree is clean at that closeout-docs commit. Because a commit cannot contain its own SHA, identify it with `git log --oneline -2`: the top commit is the closeout-docs commit and its parent must be `8858f5b`. Final proof is recorded in `/Users/wesleyjinks/code/tools/logs/item2-task7c-step14.log`.

**Truth ordering:** measured live state > binding spec and explicit owner authority within their scope > this handoff > earlier summaries. `[MEASURED]` claims were probed in this run; `[INHERITED]` claims identify their source.

## 0. Gating facts

- `[MEASURED]` No sub-agents were spawned. No push or PR action was attempted.
- `[MEASURED]` Whole-branch fix wave 2, disclosed as round 3 past the two-wave cap because findings were same-class and converging, closed the remaining language-generic uncertainty WRONG and the binary-custody WRONG. Source fix: `8858f5b786a77637532ef935c4e4530ba34cff3c`.
- `[MEASURED]` The ignored Task 7 report contains `## Whole-branch fix wave 2 + Step 17`. Generated Tier-A report/snapshot files from the rerun were removed before final custody; no oracle was edited or committed.
- `[MEASURED]` Disk was 11 GiB free at Step 1, above the 8 GiB prune threshold and 3 GiB stop threshold.
- `[INHERITED]` Push remains controller-owned and requires explicit authorization.

## 1. Resume order

1. Run `git rev-parse HEAD`, `git log --oneline -2`, and `git status --short --untracked-files=all`. Expect a clean closeout-docs commit whose parent is `8858f5b`.
2. Read `.superpowers/sdd/2026-09-04-prism-item2-dataflow-confidence/task-7-report.md` and `/Users/wesleyjinks/code/tools/logs/item2-task7c-step{1..14}.log`.
3. Before any new binding semantics, read binding spec v6.5.6 and obtain a new ruling for B-open. Keep B8 provisional.
4. If the controller authorizes it, push and open the PR with cache 55→56, Task 0b, byte controls, Tier-A validity exclusions, and fix-wave convergence disclosed. Do not merge.

**STOP conditions:** a new source change during closeout; a different parent below the closeout-docs commit; dirty tracked state; a changed pre-existing call-stats leaf; a new Tier-A quick invalid reason; or missing push authorization.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Review item 1 | done | `[MEASURED]` Tree-sitter node-type schemas derive identifier-bearing name-introducing fields for every grammar. Any route-dominating introducing construct not classified by the 1b table forces the flat-path kill equation. Go range, JS/TS destructuring, Python exception/with, Rust if-let/match/while-let, and classified/downstream controls pass. |
| Review item 2 | done | `[MEASURED]` Capture detection consumes `Language::callable_boundary_node_types()`. Rust async/gen captures are `CfgIncomplete`; immediate block remains Exact; every language boundary kind agrees. B8 remains provisional. |
| Review items 3–4 | done | `[MEASURED]` Labels-only PartialHit replacement now checks identities, spans, graph payloads, counters, and subsequent Hit. CLI test name states default/explicit equivalence. |
| Spec | done | `[MEASURED]` Repo amendment 1c mirrors binding spec v6.5.6 verbatim. New binding semantics were not added to the ownership table. |
| Steps 1–3 | done | `[MEASURED]` Source tuple frozen at `8858f5b`; fmt/diff 0; same-worktree clippy candidate/base `172/172`, normalized candidate-only set empty. |
| Steps 4–7 | done | `[MEASURED]` Focused `439/0/0`; full suite plus doctests `3982/0/1`; byte controls `1598/1598` and `272/272`; cache selector `2/0/0`. |
| Step 8 | comparative pass, absolute validity excluded | `[MEASURED]` Release build 0; matrix `159/0/0`. Candidate and base quick share only corpus SHA drift and C-name `4/6`; canonical non-matrix diff 0. |
| Steps 9–10 | done | `[MEASURED]` Call-stats `11/11`, all additive payloads present and all pre-existing normalized leaves identical. Exact/Killed: prism `83,702/2,163`, caddy `42,343/14,691`, mypy `109,633/9,956`. |
| Steps 11–12 | done | `[MEASURED]` One cache 55→56 transition, sidecar 24, Cargo diff 0; four producers, nine delivery cases, six labeled-walk tests, four valid fresh DFG JSONL records. |
| Steps 13–14 | done | `[MEASURED]` Roadmap row 19 updated to v6.5.6; row 20 retained once; this handoff and final custody validated. |

## 3. Corrections and convergence

- The first Step 3 rerun measured 173 candidate diagnostics versus 172 at the same-worktree base. One new `needless_borrow` came from the review fix. It was corrected in `7f3b225`, then Steps 1–14 restarted from that source head. Final counts are `172/172` with no candidate-only normalized headline.
- Focused totals moved from the wave-1 closeout's `435/0/0` to `439/0/0`. Full totals moved from `3978/0/1` to `3982/0/1`.
- Roadmap row 19 now names v6.5.6 and records grammar-derived language-generic uncertainty.
- The prior handoff named source head `7f3b225`, the pre-rebuild candidate hash `9aa44d…`, and `item2-task7b-step*` logs. This handoff supersedes them with the Step 17 rerun and the binary hash measured immediately after the Step 6 rebuild.
- Memory was not changed because the owner did not request a memory update.

## 4. Open work

| Work | State | Required next action |
|---|---|---|
| Push and PR | pending | Obtain explicit controller authorization, then push/open one PR; do not merge. |
| New binding semantics | parked | Obtain a binding-spec amendment under B-open. Amendment 1c's fail-closed default is already implemented. |
| B8 capture rule | provisional | Keep `CfgIncomplete` behavior until the owner settles or replaces B8. |
| Tier-A absolute validity | pending outside this slice | Re-anchor corpus SHA and restore C-name 6/6 before claiming an absolute quick pass. |
| Dependency hints | roadmap next | Implement row 20 in its own authorized slice. |

## 5. Invariants and traps

- Ownership must be established for every grammar-derived name-introducing construct of the name on relevant def/use routes before per-binding classification is allowed. Otherwise use the unconditional flat-path kill equation. Only constructs classified by table 1b establish ownership.
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
| Language-generic uncertainty fix | `8858f5b786a77637532ef935c4e4530ba34cff3c` |
| Last source-changing commit | `8858f5b786a77637532ef935c4e4530ba34cff3c` |
| Closeout-docs commit | `SELF` — commit carrying this handoff, directly on top of `8858f5b` |
| Task 0b behavior commit | `81f2943` |
| Task 0b control binary | `/Users/wesleyjinks/code/tools/bin/prism-item2-0b` · source `0b51138aa6fe` · SHA-256 `897840b5d5222dc463bd5ff883ebd476dbd0fde43f752882e5e6e8761343c9ae` |
| Base binary | `/Users/wesleyjinks/code/tools/bin/prism-base-bffb847` · SHA-256 `a7b4f69a044225edf310710d78fdcdda04e005af1cd3eeddff9154e5ac3c5b06` |
| Candidate release binary | Step 6 rebuilt source `8858f5b786a77637532ef935c4e4530ba34cff3c`, then the byte controls ran this exact file: SHA-256 `94555d1745df48df365e26590ca71783a7f5d9cd2657efae52213b15ac734895` |
| Cache versions | CPG 56; navigation sidecar 24 |
| RED logs | `/Users/wesleyjinks/code/tools/logs/item2-final-fix2-red-{1..4}.log` |
| Gate logs | `/Users/wesleyjinks/code/tools/logs/item2-task7c-step{1..14}.log` |
| Task 7 report | `.superpowers/sdd/2026-09-04-prism-item2-dataflow-confidence/task-7-report.md` |
| Controller session | `https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT` |

## 7. Verdict and owner questions

**Verdict:** SURVIVED · Whole-branch fix wave 2 and Step 17 pass at source head `8858f5b`, with the Tier-A quick absolute-validity exclusions stated above. Evidence tier: test-backed and same-base controlled. Review round 3 was disclosed past the two-wave cap because the findings remained same-class and converging.

**Owner questions:** authorize or decline push/PR; settle B8 before broader capture semantics; assign the Tier-A re-anchor outside this slice.
