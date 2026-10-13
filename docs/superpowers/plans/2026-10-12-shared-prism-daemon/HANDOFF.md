# Handoff — shared per-repo prism daemon lane; S1 and C1 merged, five-corpus measurement next, S2 planned and waiting on owner answers

**Written:** 2026-10-13T09:45-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5) · **Provider:** claude
**Workspace:** `~/code/slicing/.claude/worktrees/spec-shared-prism-daemon` · `spec/shared-prism-daemon` · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git status --short && git log --oneline -3` · Output inline in the session
**Predecessor:** this session's own handoff of 2026-10-12T22:55-06:00 (same file, previous revision), and before it Claude Code session `afaf5155-ea5e-4ad5-b8d8-be35f747f74d` (Fable 5.1)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` this session is the only one on the lane; another session works the JS/TS parameter-Def lane and does not touch `src/mcp`. — **RESOLVED 2026-10-13**
**(b) Custody exposure** — `[MEASURED]` S1 and C1 are on `main`. Spec, plans and this handoff are pushed on `spec/shared-prism-daemon`. One branch is local only: `scratch/s2-validate` `cf873945` in `~/code/slicing` (worktree `.claude/worktrees/s2-validate`), the S2 plan's code applied and passing. It is reproducible from the S2 plan by replay, so losing it costs a re-run. — **RESOLVED 2026-10-13**
**(c) In flight / irreversible** — `[MEASURED]` at the time of writing a release build is running in `~/code/prism-c1-corpus` (detached worktree at `origin/main` `7db8f195`) for the C1 measurement. Nothing else. Check `pgrep -fl 'prism-mcp|memreport|cargo'` before starting anything heavy. — **OPEN until the measurement finishes**
**(d) Authorization granted but not exercised** — owner, 2026-10-13: "proceed as recommedned. merge pr357 with the code changes". Exercised for S1 (#361) and C1 (#362). Covers merging #357 and the C1 measurement and its readout PR. It does **not** cover starting S2 implementation: the S2 plan raised owner questions (§7).

## 1. Resume order
1. Read `README.md` in this directory: slice table, the owner questions from S2 planning, measured facts.
2. If the C1 measurement is not finished: follow `C1-memory-harness.md` Task 7 Steps 3–6 with the merged driver (`scripts/memreport/README.md` on `main`). Binary and "slicing" corpus: `~/code/prism-c1-corpus`. Evidence: `~/prism-evidence/mcp-memory/c1/`. Keep workloads outside the corpus with `--workloads`, then copy them into `scripts/memreport/workloads/` for the docs PR. Use a fresh `--out` directory for every run; `summarize --run DIR` rescores saved evidence.
3. Open the readout docs PR (`docs/analysis/2026-10-prism-mcp-memory-readout.md` plus the workloads). C2 planning starts from its verdict table.
4. Get the owner's answers to §7, then implement S2 as two PRs (S2a = plan Tasks 1–4, S2b = Tasks 5–8), each from `scratch/s2-validate` or by an implementer, reviewed by Opus 5.5 and gpt-6.1-sol, cap two rounds.
5. When S2 merges, write the S3a plan from its real interfaces (the S2 plan's last section lists what S3a, S4 and S5a consume).

**STOP conditions:** a corpus whose projected peak would exhaust this host's memory (24 GB, usually already swapping): do not run it; report the projection and let the owner decide. A reviewer WRONG finding the controller disagrees with (escalate to the owner). A golden transcript changing in any later slice.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Part one: build on first tool call | done | `[INHERITED]` #356, main `cb0eeec0` |
| Spec v3 | done | `[MEASURED]` owner-approved 2026-10-12; implementation record added to §13 on 2026-10-13 |
| S1 transport refactor | done | `[MEASURED]` #361 squash-merged as `f6354e07`. Opus 5.5 and gpt-6.1-sol both APPROVE, no defect; two rounds; full suite 5,286 passed, 0 failed at `8e1873bb`; CI green on the head and on the golden-only commit alone |
| C1 memory harness | done | `[MEASURED]` #362 squash-merged as `7db8f195`. Two review rounds (both FIX, all folded) plus one confirmation of the last fold (APPROVE), which is one step past the declared cap; full suite 5,321 passed, 0 failed at `8d37f34c`; 145 Python tests; Tier-A matrix 178 of 178 |
| C1 five-corpus measurement | in progress | release build started 2026-10-13 09:40 in `~/code/prism-c1-corpus` |
| C1 readout docs PR | next | after the measurement |
| S2 plan | done | `[MEASURED]` `S2-identity-preamble.md` committed `dd6b6c7e`; its code replayed on `scratch/s2-validate` `cf873945` (323 MCP unit tests, 13 binary tests; logs read by this writer) |
| S2 implementation | pending | blocked on §7 |
| Spec/plan PR #357 | next | merge with this handoff revision |
| S3a–S6, C2, B | pending | each planned when its predecessor merges |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `src/mcp/session.rs`, comment above `build_state` | "`LoadedRepo` is not `Sync`" | `[MEASURED]` `SessionProvider` is `Send + Sync`. Still not corrected in code; S3a replaces the comment with a compile-time assertion. |
| Spec §6.1 | attribution by "tree nodes × per-node size" | `[MEASURED]` not a bound in either direction (two reviewers, independent measurements). The harness measures tree-sitter's live heap instead. Recorded in spec §13 and at the top of the C1 plan. |
| Spec §6.1–§6.2 | the M2 transient can be read from the peak | `[MEASURED]` freed memory stays charged to `phys_footprint`, so the delta reads 0. M2's estimate is the CPG cache file size. |
| `CLAUDE.md`, "CPG cache v73" | cache version | `[MEASURED]` 114. Outside this lane; not corrected. |
| Memory `project_prism_mcp_memory_lane.md` | status lines | Updated 2026-10-13 for the S1 merge; update again when the measurement and #357 land. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | C1 measurement | in progress | §1 step 2 | — | `~/code/prism-c1-corpus`; `~/prism-evidence/mcp-memory/c1/` |
| 2 | C1 readout PR | next | §1 step 3 | #1 | `docs/analysis/2026-10-prism-mcp-memory-readout.md` |
| 3 | S2a, S2b | pending | §1 step 4 | §7 | plan `S2-identity-preamble.md`; scratch `cf873945` |
| 4 | C2 plans | pending | one per candidate the readout advances | #2 | spec §6.2 |
| 5 | S3a plan | pending | write from S2's merged interfaces | #3 | lane README notes for S3a |

## 5. Invariants and traps — do not do these
- Never edit `src/mcp/transport_tests.rs` or `src/mcp/owner_transport_tests.rs`, and never re-record the golden transcripts, in S2–S5 — together they are the proof that standalone stdout is unchanged.
- Never present the mutation gate as covering this lane — it builds without `--features mcp` and selects no MCP code.
- Never attribute memory by freeing something and re-measuring — freed memory stays charged to the footprint (measured twice in this lane).
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
| S2 validation branch (local) | `scratch/s2-validate` `cf873945`, on `f6354e07` |
| Measurement worktree | `~/code/prism-c1-corpus`, detached at `7db8f195` |
| Corpora | ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43`, kubernetes `8d34ac5ef882` under `~/code/bench-repos/`; slicing = the measurement worktree |
| Test totals on `main` after both merges | full suite 5,321 passed at C1 `8d37f34c`; `cargo test --features mcp --lib mcp::` 241; `--test mcp` 23; Python 145 |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "S1 leaves the in-process server's stdout byte-for-byte unchanged" · pass: INDEPENDENT (two reviewers, each with a same-host base control or mutation probes) · evidence tier: TEST-BACKED · record: PR #361; goldens recorded on the base and CI-green alone (run 38402295489), unchanged through the merge

**Questions the owner owes an answer to** (detail in `README.md`, "Owner questions raised by S2 planning"):
1. Runtime directory on multi-user Linux: keep `/tmp/prism-mcpd` as the spec says (other users fail closed, with an error naming the override), or use a per-user name in the temp-dir fallback (a spec change)?
2. A client-supplied result cap below the floor: refuse it (the plan's choice), or replace it with the default?
3. The peer-uid check: leave it to S4 where connections are accepted (the plan's choice), or put it in S2?
4. `--warm-at-startup` on a shared daemon: give the daemon command line the flag, or let the first admitted `HELLO` decide? (S4's decision; S2 only carries the field.)
5. S2 implementation: start from `scratch/s2-validate` as S1 and C1 did, or have Sonnet implementers work the plan?
