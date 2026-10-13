# Handoff — shared per-repo prism daemon lane; S1, C1 and the C1 readout done, S2 planned, seven owner questions open

**Written:** 2026-10-13T12:30-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5) · **Provider:** claude
**Workspace:** `~/code/prism-c1-readout` · `docs/mcp-memory-readout` (the readout docs PR; this file is on `main` once it merges) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git status --short && git log --oneline -3` · Output inline in the session
**Predecessor:** this session's own handoff of 2026-10-13T09:45-06:00 (same file, previous revision), and before it Claude Code session `afaf5155-ea5e-4ad5-b8d8-be35f747f74d` (Fable 5.1)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` this session is the only one on the lane; another session works the JS/TS parameter-Def lane and does not touch `src/mcp`. — **RESOLVED 2026-10-13**
**(b) Custody exposure** — `[MEASURED]` S1, C1, the spec and the plans are on `main` (`dc3b6bbe`). The readout and this handoff are PR #363 (branch `docs/mcp-memory-readout`), on `main` once it is squash-merged. Raw measurement evidence is local only, in `~/prism-evidence/mcp-memory/c1/` with `SHA256SUMS`; it is about two hours of machine time to reproduce. One branch is local only: `scratch/s2-validate` `cf873945` in `~/code/slicing` (worktree `.claude/worktrees/s2-validate`), the S2 plan's code applied and passing; reproducible from the S2 plan by replay. — **RESOLVED 2026-10-13**
**(c) In flight / irreversible** — `[MEASURED]` nothing is running. The measurement finished at 11:34 and the supplement at 11:54 on 2026-10-13. — **RESOLVED 2026-10-13**
**(d) Authorization granted but not exercised** — owner, 2026-10-13: "proceed as recommedned. merge pr357 with the code changes". Fully exercised: S1 (#361), C1 (#362), #357, the measurement and the readout PR. It does **not** cover S2 implementation, writing C2 plans, or any change to the spec's pinned parameters. Two different things hold these back: S2 waits on §7 questions 1–4 and 7; the M1 plan waits on question 5; the M2 plan has no open question and needs only the owner's go-ahead.

## 1. Resume order
1. Read `README.md` in this directory: slice table, the owner questions from S2 planning, measured facts.
2. Read `docs/analysis/2026-10-prism-mcp-memory-readout.md`, last section first. M1 and M2 advance, M3 does not; the "Steady state" footprint column is not a repeatable figure.
3. Get the owner's answers to §7. Question 5 (the steady-state metric) gates the M1 plan; questions 1–4 and 7 gate S2. The M2 plan has no open metric question (its base is the load peak); it needs the owner's go-ahead, which has not been asked for separately.
4. Implement S2 as two PRs (S2a = plan Tasks 1–4, S2b = Tasks 5–8), each from `scratch/s2-validate` or by an implementer, reviewed by Opus 5.5 and gpt-6.1-sol, cap two rounds. Write the C2 plan for M2 from the readout, and the one for M1 once question 5 is answered; a C2 candidate may begin after C1 but ship only after S5a.
5. When S2 merges, write the S3a plan from its real interfaces (the S2 plan's last section lists what S3a, S4 and S5a consume).

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
| S2 implementation | pending | blocked on §7 |
| Spec/plan PR #357 | done | `[MEASURED]` squash-merged as `dc3b6bbe` |
| C2 plans (M2, M1) | pending | M2: no open metric question, needs the owner's go-ahead. M1: also blocked on §7 question 5 |
| S3a–S6, B | pending | each planned when its predecessor merges |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `src/mcp/session.rs`, comment above `build_state` | "`LoadedRepo` is not `Sync`" | `[MEASURED]` `SessionProvider` is `Send + Sync`. Still not corrected in code; S3a replaces the comment with a compile-time assertion. |
| Spec §6.1 | attribution by "tree nodes × per-node size" | `[MEASURED]` not a bound in either direction (two reviewers, independent measurements). The harness measures tree-sitter's live heap instead. Recorded in spec §13 and at the top of the C1 plan. |
| Spec §6.1–§6.2 | the M2 transient can be read from the peak | `[MEASURED]` in most runs the footprint had not come down when the next sample was taken, so the delta reads near 0. M2's estimate is the CPG cache file size. Where the footprint did come down, the transient was 425–479 MiB on slicing and 803 MiB on prometheus, larger than the cache file. |
| Spec §6.1 | steady state = `phys_footprint` after a 5 s settle | `[MEASURED]` two levels 460–790 MiB apart, not decided by the settle time (35 of 36 runs at the higher level at 5 s; 4 of 13 at the lower at 60 s). Mechanism not identified; forcing the move not tried. The live heap is repeatable to 0.13 MiB but counts allocations, not physical memory, and is circular for M1. Spec text unchanged pending §7 question 5; recorded in spec §13. |
| Spec §6.1 | five corpora | `[MEASURED]` four. Kubernetes not run (§7 question 6). |
| `CLAUDE.md`, "CPG cache v73" | cache version | `[MEASURED]` 114. Outside this lane; not corrected. |
| Memory `project_prism_mcp_memory_lane.md` | status lines | Updated 2026-10-13 for the readout result and the open questions. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | C2 plans for M2 and M1 | pending | M2: write from the readout (base = load peak) on the owner's go-ahead. M1: pin the metric the owner chooses, with a footprint confirmation | owner go-ahead; M1 also §7 question 5 | spec §6.2; readout verdict table |
| 2 | Kubernetes corpus | pending | none unless the owner asks for it | §7 question 6 | `~/code/bench-repos/kubernetes` `8d34ac5ef882` |
| 3 | S2a, S2b | pending | §1 step 4 | §7 | plan `S2-identity-preamble.md`; scratch `cf873945` |
| 4 | Test whether report mode can release free memory before the settled sample | pending | only if the owner picks option (b) of question 5; it would also rank "release after load" as a candidate (157–715 MiB) | §7 question 5 | `src/mcp/mem_report.rs` |
| 5 | S3a plan | pending | write from S2's merged interfaces | #3 | lane README notes for S3a |
| 6 | Refresh after files changed | not measured | the measured refresh found nothing stale and reloaded from the cache; a rebuilding refresh may peak higher than the 1.9–2.0 × recorded | — | readout, "Other observations" |

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

**Questions the owner owes an answer to** (detail in `README.md`, the two "Owner questions" sections):
1. Runtime directory on multi-user Linux: keep `/tmp/prism-mcpd` as the spec says (other users fail closed, with an error naming the override), or use a per-user name in the temp-dir fallback (a spec change)?
2. A client-supplied result cap below the floor: refuse it (the plan's choice), or replace it with the default?
3. The peer-uid check: leave it to S4 where connections are accepted (the plan's choice), or put it in S2?
4. `--warm-at-startup` on a shared daemon: give the daemon command line the flag, or let the first admitted `HELLO` decide? (S4's decision; S2 only carries the field.)
5. The steady-state metric for C2 experiments: live heap with a pinned minimum effect size as the gate plus a footprint at a known level as a required confirmation for M1 (recommended); or one of the single options (live heap alone, footprint after a forced release, footprint tagged by level)?
6. Kubernetes: accept the four measured corpora (recommended), run a subtree, or run it on a larger host?
7. S2 implementation: start from `scratch/s2-validate` as S1 and C1 did, or have Sonnet implementers work the plan?
