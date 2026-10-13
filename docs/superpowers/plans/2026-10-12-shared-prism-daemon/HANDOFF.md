# Handoff — shared per-repo prism daemon lane; owner decisions of 2026-10-13 recorded, S2a in review, C2 driver and M2 plan written, M2 experiment not yet run

**Written:** 2026-10-13T14:10-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5) · **Provider:** claude
**Workspace:** `~/code/prism-c2-plan` · `plan/c2-m2-cache-streaming` (this docs PR) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git status --short && git log --oneline -3` · Output inline in the session
**Predecessor:** this session's own handoff of 2026-10-13T12:30-06:00 (same file, previous revision), and before it Claude Code session `afaf5155-ea5e-4ad5-b8d8-be35f747f74d` (Fable 5.1)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` this session is the only one on the lane; another session works the JS/TS parameter-Def lane and does not touch `src/mcp`. — **RESOLVED 2026-10-13**
**(b) Custody exposure** — `[MEASURED]` on `main`: S1, C1, spec, plans, readout (`e2701e89`). Pushed branches: `feat/mcp-shared-s2a` (PR #364), `plan/c2-m2-cache-streaming` (this PR). **Local only:** `feat/memreport-experiment-kinds` in `~/code/prism-c2-driver` (the driver extension, 6 commits) until its PR is opened; `scratch/c2-m2-validate` `c7c92790` in `~/code/prism-m2` (the M2 prototype); `scratch/s2-validate` `cf873945` (S2b comes from its last four commits). Raw evidence is local only under `~/prism-evidence/mcp-memory/{c1,c2-m2}/`. — **OPEN until the two local branches are pushed**
**(c) In flight / irreversible** — `[MEASURED]` at the time of writing two reviewers are working on PR #364. No measurement is running. The host is shared with another session whose test runs push the load average past 50; check `uptime` and `pgrep -fl 'prism-mcp|cargo|nextest|mutgate'` before any measurement. — **OPEN**
**(d) Authorization granted but not exercised** — owner, 2026-10-13 (second message): answers to questions 1 and 3–7 (table in `README.md`), then "go ahead and write M2 experimenet plan and continue". Read as: write the M2 plan, implement S2 from `scratch/s2-validate`, and carry the lane forward. Question 2 (a client result cap below the floor) was asked back, not answered: S2b is built to refuse it and says so in its PR. Not authorized: merging M2 without the shipping-gate answer the plan asks for; any spec change beyond the ones recorded in §11.

## 1. Resume order
1. Read `README.md` in this directory: slice table, the decision table of 2026-10-13, measured facts.
2. PR #364 (S2a): fold the two reviews (cap two rounds), merge on approval and green CI. Then cut S2b from the last four commits of `scratch/s2-validate` (`e531a3d2`, `43eb8b28`, `a0aa033e`, `cf873945`) onto `main`, restore the module doc sentence about the preamble, verify, review, merge.
3. Driver extension: open the PR from `feat/memreport-experiment-kinds` (242 Python tests pass), smoke it once against a real release binary (`--kinds warm,build --reps 1` on excalidraw), review, merge.
4. M2: follow `C2-M2-cache-streaming.md` from Task 2. The prototype is `scratch/c2-m2-validate` `c7c92790` (29 module tests pass, eight mutants killed; full suite, mutation gate and Tier-A matrix not yet run on it). Run the pinned experiment only on a quiet host. Do not merge M2 until the owner answers the shipping-gate question in the plan.
5. When S2 is merged, write the S3a plan from its real interfaces. When the driver is merged and the owner wants M1, write the M1 plan against spec §6.1 as revised.

**STOP conditions:** a corpus whose projected peak would exhaust this host's memory (24 GB, usually already swapping): do not run it; report the projection and let the owner decide. A reviewer WRONG finding the controller disagrees with (escalate to the owner). A golden transcript changing in any later slice.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Part one: build on first tool call | done | `[INHERITED]` #356, main `cb0eeec0` |
| Spec v3 | done | `[MEASURED]` owner-approved 2026-10-12; implementation record added to §13 on 2026-10-13 |
| S1 transport refactor | done | `[MEASURED]` #361 squash-merged as `f6354e07`. Opus 5.5 and gpt-6.1-sol both APPROVE, no defect; two rounds; full suite 5,286 passed, 0 failed at `8e1873bb`; CI green on the head and on the golden-only commit alone |
| C1 memory harness | done | `[MEASURED]` #362 squash-merged as `7db8f195`. Two review rounds (both FIX, all folded) plus one confirmation of the last fold (APPROVE), which is one step past the declared cap; full suite 5,321 passed, 0 failed at `8d37f34c`; 145 Python tests; Tier-A matrix 178 of 178 |
| C1 measurement | done | `[MEASURED]` `run-1`: four corpora (slicing, excalidraw, prometheus, ruff), 48 of 48 runs and 4 of 4 priming runs admissible, binary `7db8f195`, one process at a time. Kubernetes not run: projected first build 27–70 GiB on a 24 GiB host. Supplement `settle60/`: 12 of 12 workload runs admissible at a 60 s settle; five of them overlapped a resident normal-mode probe (`idle-probe/`), disclosed in the readout. |
| C1 readout | done | `[MEASURED]` `docs/analysis/2026-10-prism-mcp-memory-readout.md`. M1 124–559 MiB and M2 136–693 MiB advance on all four corpora; M3 16–58 MiB does not advance. Verdicts recomputed against the live heap: unchanged. Reviewed by Opus 5.5 and gpt-6.1-sol, both of whom recomputed the figures: verdicts confirmed, both FIX on the interpretation (mechanism overstated, live heap is not a physical-memory metric, overlap undisclosed, refresh was a no-change reload); all folded. A confirmation pass by gpt-6.1-sol (one step past the declared one-round cap) found three wording errors in the fold; folded as specified, not re-reviewed. |
| S2 plan | done | `[MEASURED]` `S2-identity-preamble.md` committed `dd6b6c7e`; its code replayed on `scratch/s2-validate` `cf873945` (323 MCP unit tests, 13 binary tests; logs read by this writer) |
| S2a (plan Tasks 1–4) | in review | `[MEASURED]` PR #364, head `09137b83`: full suite 5,383 passed, 0 failed at `4cc45a2b`; clippy 258 lines (baseline). Mutation gate stopped at 82 of 230 (all killed): it builds without the `mcp` feature and cannot select this code. |
| S2b (plan Tasks 5–8) | pending | cut after S2a merges; builds the "refuse a sub-floor cap" reading (decision 2 is open) |
| Spec/plan PR #357 | done | `[MEASURED]` squash-merged as `dc3b6bbe` |
| C2 driver extension | implemented, not yet in review | `[MEASURED]` `feat/memreport-experiment-kinds` `71acf84a`: `run --kinds`, kind `build`, `compare --experiment m2`; 242 Python tests pass; never run against a real binary |
| C2-M2 plan | done | `[MEASURED]` `C2-M2-cache-streaming.md`. Prototype `c7c92790`; preliminary, slicing only, busy host: first-build peak 3,092 → 2,400 MiB, load peak 2,016 → 1,621 MiB, cache file byte-identical, load time not comparable |
| C2-M2 experiment | pending | needs the driver extension and a quiet host |
| C2-M1 plan | pending | metric decided (spec §6.1 as revised); not written |
| S3a–S6, B | pending | each planned when its predecessor merges |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `src/mcp/session.rs`, comment above `build_state` | "`LoadedRepo` is not `Sync`" | `[MEASURED]` `SessionProvider` is `Send + Sync`. Still not corrected in code; S3a replaces the comment with a compile-time assertion. |
| Spec §6.1 | attribution by "tree nodes × per-node size" | `[MEASURED]` not a bound in either direction (two reviewers, independent measurements). The harness measures tree-sitter's live heap instead. Recorded in spec §13 and at the top of the C1 plan. |
| Spec §6.1–§6.2 | the M2 transient can be read from the peak | `[MEASURED]` in most runs the footprint had not come down when the next sample was taken, so the delta reads near 0. M2's estimate is the CPG cache file size. Where the footprint did come down, the transient was 425–479 MiB on slicing and 803 MiB on prometheus, larger than the cache file. |
| Spec §6.1 | steady state = `phys_footprint` after a 5 s settle | `[MEASURED]` not repeatable (two levels 460–790 MiB apart). **Corrected in the spec on 2026-10-13** by owner decision 8: live heap with a minimum effect size as the gate, a footprint at a known level as a required confirmation for M1, M2 judged on a peak. |
| Spec §6.1 | five corpora | **Corrected in the spec:** four. Kubernetes is not run (owner decision 9). |
| Spec §6.2, row M2 | the transient is the file bytes read on load | `[MEASURED]` by reading `src/cpg_cache.rs`: there are three transients, and the largest is on **save** (a clone of the whole index plus the encoded file). Recorded in the lane README and the M2 plan; the spec row is not edited. |
| `CLAUDE.md`, "CPG cache v73" | cache version | `[MEASURED]` 114. Outside this lane; not corrected. |
| Memory `project_prism_mcp_memory_lane.md` | status lines | Updated 2026-10-13 for the readout result and the open questions. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | S2a | in review | fold reviews, merge | reviewers | PR #364 |
| 2 | S2b | pending | §1 step 2 | #1 | `scratch/s2-validate` `e531a3d2..cf873945` |
| 3 | Driver extension | implemented | open PR, real-binary smoke, review | — | `~/code/prism-c2-driver`, `71acf84a` |
| 4 | M2 candidate | prototype | plan Task 2: branch from the prototype, full verification | — | `~/code/prism-m2`, `c7c92790` |
| 5 | M2 experiment | pending | plan Task 3 | #3, #4, quiet host | `~/prism-evidence/mcp-memory/c2-m2/` |
| 6 | M2 shipping gate | decided | the fan-out check is waived for M2 only; `tier-a --quick` and the experiment still apply | — | spec §11 item 11; draft PR #369 |
| 7 | Decision 2 (sub-floor result cap) | open | owner: refuse (built) or replace with the default | owner | spec §11 item 5 |
| 8 | S3a plan | pending | write from S2's merged interfaces | #2 | lane README notes for S3a |
| 9 | M1 plan | pending | write against spec §6.1 as revised | #3 | readout verdict table |
| 10 | Refresh after files changed | not measured | none planned | — | readout, "Other observations" |

## 5. Invariants and traps — do not do these
- Never edit `src/mcp/transport_tests.rs` or `src/mcp/owner_transport_tests.rs`, and never re-record the golden transcripts, in S2–S5 — together they are the proof that standalone stdout is unchanged.
- Never present the mutation gate as covering this lane — it builds without `--features mcp` and selects no MCP code.
- Never attribute memory by freeing something and re-measuring, and never read a footprint taken seconds after a load as an idle figure — the footprint sits at one of two levels and the move between them did not happen at a reproducible time (measured three ways in this lane; mechanism not identified).
- Never gate a C2 experiment on a `phys_footprint` whose level is unknown, and never judge M1 on live heap alone — the first can swing 460–790 MiB by accident, the second cannot fail.
- Never run a second prism-mcp while a measurement is running — five supplement runs overlapped a probe this session and had to be disclosed.
- Never run the kubernetes corpus on this host — projected first build 27–70 GiB against 24 GiB of memory.
- Never run the memreport driver into a directory that already has results, and never re-run to change a rule: `summarize --run DIR` rescores saved evidence.
- Never call `mem_tree_alloc::install()` from library code or a library test — it swaps tree-sitter's allocator pointers and is only safe before anything parses, in the report-mode binary.
- Trap hit this session: two implementers in one worktree, and one ran `git commit --amend` onto the other's commit → recovered with `git reset --soft`; give parallel implementers separate worktrees, or forbid `--amend` in the brief.
- Trap hit this session: `gh pr merge --match-head-commit` needs the full 40-character id.
- Trap hit this session: this host has 24 GB and is usually several GB into swap; four concurrent Rust builds push it further. Stagger reviewers and verification.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Spec | `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` |
| Plans | `docs/superpowers/plans/2026-10-12-shared-prism-daemon/{README,S1-transport-refactor,C1-memory-harness,S2-identity-preamble}.md` |
| Spec and plan PR | `#357`, branch `spec/shared-prism-daemon` |
| S1 | PR `#361`, merge `f6354e07` |
| C1 | PR `#362`, merge `7db8f195` |
| C1 readout | PR `#363` |
| S2 validation branch (local) | `scratch/s2-validate` `cf873945`, on `f6354e07` |
| Measurement worktree | `~/code/prism-c1-corpus`, detached at `7db8f195`; release binary `target/release/prism-mcp`, build identity `d9414cb7b8bc…` |
| Readout | `docs/analysis/2026-10-prism-mcp-memory-readout.md`; workloads `scripts/memreport/workloads/` |
| Evidence | `~/prism-evidence/mcp-memory/c1/{run-1,settle60,idle-probe}`, `SHA256SUMS` |
| Corpora | ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43`, kubernetes `8d34ac5ef882` under `~/code/bench-repos/`; slicing = the measurement worktree |
| Test totals on `main` after both merges | full suite 5,321 passed at C1 `8d37f34c`; `cargo test --features mcp --lib mcp::` 241; `--test mcp` 23; Python 145 |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "S1 leaves the in-process server's stdout byte-for-byte unchanged" · pass: INDEPENDENT (two reviewers, each with a same-host base control or mutation probes) · evidence tier: TEST-BACKED · record: PR #361; goldens recorded on the base and CI-green alone (run 38402295489), unchanged through the merge

**§2c verdict:** SURVIVED · claim: "M1 and M2 advance and M3 does not, on four corpora" · pass: INDEPENDENT (Opus 5.5 and gpt-6.1-sol each recomputed the figures from the raw reports; also recomputed against a second base, the live heap) · evidence tier: MEASURED · record: PR #363; readout, section "Verdict"

**§2c verdict:** REFUTED · claim: "`phys_footprint` after a 5 s settle is a repeatable steady-state figure (spec §6.1)" · pass: SELF, by the plan's own cross-check against the 2026-10-09 figures, then a report-mode run at 60 s, normal-mode sampling from outside and a 12-run supplement; both reviewers reproduced the counts · evidence tier: MEASURED · record: readout, section "The Steady state column is not an idle figure"

**Questions the owner owes an answer to:**
1. A client-supplied result cap below the floor (12,000 characters): refuse the connection with an error naming the floor (built in S2b), or serve it with the default of 80,000? The client cannot see the daemon's stderr, so a replacement would be silent.
2. Should the shim check the uid of the socket's peer, as the daemon does (spec §4.4 has only the daemon check)? Two review rounds of S2a concluded that checks on the runtime directory's path are hardening and this check is where the guarantee between users has to live. Recommended: yes; it is an S5a item and costs nothing before then.

Answered on 2026-10-13: the fan-out latency gate is waived for M2 only (spec §11 item 11). Also answered that day and recorded in spec §11 items 4, 6–10 and in `README.md`: the runtime directory stays as the spec says; the peer-uid check stays in S4; `--warm-at-startup` is a flag on the daemon's command line; the C2 metric is live heap with a minimum effect size plus a footprint confirmation for M1; four corpora; S2 is implemented from `scratch/s2-validate`.
