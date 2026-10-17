# Handoff — shared per-repo prism daemon lane; S5a and S5b merged (shim, `--shared`, operator control); next the S6 plan; optimization lane at DFG compaction PR D

**Written:** 2026-10-16T13:35-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5) · **Provider:** claude
**Workspace:** `~/code/prism-lane-docs` · `docs/s5-merged-handoff` (this docs PR, from `origin/main` `61c2a9e5`) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git fetch origin && git log origin/main --oneline -30`, `gh pr view --json state,mergeCommit,mergedAt,body` on #398, #401, #402, #405, #407, #408, #410–#413, #415–#418 · Output inline in the session
**Predecessor:** this session's own handoff of 2026-10-14T21:30-06:00 (same file, previous revision, #396), and before it Claude Code session `afaf5155-ea5e-4ad5-b8d8-be35f747f74d` (Fable 5.1)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker (a docs subagent of the controller session, which supplied the list of changes and the owner decisions). `[MEASURED]` claims were probed by this writer this turn (git log, `gh pr view`, file reads, `ls`/`shasum`/`strings` on binaries); `[INHERITED]` claims were not, and name their source.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` (controller brief, 2026-10-16) the controller session `fd583811` owns the daemon lane and the optimization lane; another agent is writing the S6 plan now; the JS/TS A2 lane (#409, #414, #419, #420 merged since #396) is a separate session and does not touch `src/mcp`. `[MEASURED]` worktrees `~/code/prism-s6-plan` (`plan/s6-launcher-docs-soak`) and `~/code/prism-s6-validate` (`scratch/s6-validate`) exist, both at `61c2a9e5`. — **RESOLVED 2026-10-16** (do not start a second S6 plan)
**(b) Custody exposure** — `[MEASURED]` on `main` (`61c2a9e5`): everything through S5b-2 (#418), DFG compaction PR C (#415), the Go speedup (#401) and release v3.2.0 (#405). Open, not merged: DFG PR D #417 (`perf/dfg-codec`, head `b6da8226`). Local-only, not for merge: `scratch/opt-probe` (`c3162f94`, `~/code/prism-opt-probe`), `scratch/s4-*` (`~/code/prism-s4-validate`), `scratch/s5a-{validate,tasks,tasks-r1,replay,replay-r1}`, `scratch/s5b-{tasks,tasks-r0,replay,replay-r0,replay-r1}`. Raw evidence local only: `~/prism-evidence/mcp-memory/{c1,c2-m2,optimization-2026-10-14,go-speedup-acceptance-2026-10-15}/` and `~/prism-evidence/dfg-compaction/prC-2026-10-16/` (each with `SHA256SUMS`). — **RESOLVED**
**(c) In flight / irreversible** — `[MEASURED]` deployed binaries: `~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release/prism-mcp` and `~/code/slicing/target/release/prism-mcp` are the same file (sha256 `9d6551803f57…`, 23,808,224 bytes, 2026-10-14 22:50), built from `main` `1c10c1e23f8c` (#403; read from the version string `3.1.2` + `1c10c1e23f8c` embedded in the binary). That build includes the Go speedup (#401) and **predates S5a**: it has no `--shared` and no `daemon-status`/`daemon-shutdown` (`--help` probed). Old copies kept beside it: `prism-mcp.pre-401` (2026-10-14 13:51, both locations) and `prism-mcp.pre-389` (plugin cache only). Nothing was changed. Any pre-S5b daemon still running restarts its idle clock at each S5b `daemon-status` probe (SI-35); stop it with `kill -TERM`. DFG PR D #417 is in review (`[MEASURED]` round 1 astra FIX / Opus APPROVE folded at `b6da8226`). — **OPEN** (redeploy is an owner/controller call, not part of this docs PR)
**(d) Authorization granted but not exercised** — `[INHERITED]` owner decisions 2026-10-15/16, all already applied in code: D77 "yes 60s rule" and its clock "yes go with A" (wall-clock UTC epoch seconds; a clock set back stretches the minute by up to the step; DST and time zones have no effect); D78 "keep the uid check"; S5b dead keys' files are reported and counted, nothing removed; a daemon asked to shut down keeps admitting until zero clients. For DFG compaction: CI runs an oracle compile check on every PR and a path-filtered parity job; pins are a controller gate re-pinned at the start of each compaction PR; cross-build replay treats targets `/producer/build_identity` like the SARIF provenance fields. Standing (memory): reviewer-approved + green CI → squash-merge. Still open with the owner: §7.

## 1. Resume order
1. Read `README.md` in this directory: slice table, decision tables (decisions 12–15 are S5a/S5b), notes for S6.
2. **S5a: done** (`[MEASURED]` plan #402 → `557c8550`; S5a-1 #408 → `b994f11e`; S5a-2 #411 → `0c47020b`, 2026-10-16). `prism-mcp --shared` exists on `main`.
3. **S5b: done** (`[MEASURED]` plan #412 → `dc20c8da`; S5b-1 #416 → `30df802d`; S5b-2 #418 → `61c2a9e5`, 2026-10-16). `prism-mcp daemon-status` and `daemon-shutdown` exist on `main`.
4. **S6 plan** (launcher/config opt-in, docs, soak instrumentation): being written by another agent in `~/code/prism-s6-plan`. Do not write a second one; wait for its PR, then review it (Opus 5.5 + gpt-6.1-sol, cap 2). README note for S6: the launcher is `scripts/prism-mcp-launch.sh`, which passes extra arguments through.
5. **Optimization lane (row 14):** DFG compaction PR D #417 — wait for review round 2 / CI, merge on approval. Then **PR E** (activate the compact layout; cache version bump from 120) and **PR F** (delete the oracle). Plan `docs/superpowers/plans/2026-10-14-dfg-compaction.md`; re-pin the pins at the start of each PR (controller gate).
6. **M1 plan:** still not written, against spec §6.1 as revised; the driver's `compare` needs an `m1` row set. M1 ships after S5a (now met).

**STOP conditions:** a corpus whose projected peak would exhaust this host's memory (24 GB): do not run it. A reviewer WRONG finding the controller disagrees with: escalate to the owner. A golden transcript changing in any later slice. A review finding of a kind already folded twice: escalate, do not patch a third time. Disk: `[MEASURED]` 119 GiB free on the data volume (87 % used) — check `df` before heavy builds.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Part one: build on first tool call | done | `[INHERITED]` #356, main `cb0eeec0` |
| Spec v3 | done | `[INHERITED]` owner-approved 2026-10-12; §11 items 4–11 and §13 records added 2026-10-13; `[MEASURED]` §4.2 SI-8 clarification present in the spec |
| S1 transport refactor | done | `[INHERITED]` #361 → `f6354e07` |
| C1 harness and readout | done | `[INHERITED]` #362 → `7db8f195`; readout #363 → `e2701e89` |
| S2a / S2b | done | `[INHERITED]` #364 → `802844a8`; #370 → `8587c7b9` |
| C2 driver extension | done | `[INHERITED]` #367 → `c785fa4f` |
| M2 plan, save half, load half | done | `[INHERITED]` #366 → `86756aad`; #380 → `88f92603`; readout #388; #389 → `9d5a67bc` (owner override of P3) |
| S3a (plan, S3a-1, S3a-2) | done | `[INHERITED]` #373 → `4e2e9c3c`; #374 → `e7dc1cc2`; #377 → `14e6f66c` |
| S3b (plan, S3b-1, S3b-2) | done | `[INHERITED]` #384 → `c3e1d3c3`; #385 → `c19ba553`; #386 → `a9ac0a63` |
| S4 (plan, S4-1, S4-2) | done | `[INHERITED]` #393 → `607b5a50`; #394 → `8b2744b3`; #395 → `296102a2`; handoff #396 → `56c97d93` |
| S5a plan | done | `[MEASURED]` #402 → `557c8550` (merged 2026-10-16T04:25Z). Round 1 Opus APPROVE / sol FIX (five WRONG, folded test first); round 2 (cap) Opus APPROVE / sol FIX (D77 wall clock) → owner option A, folded after the cap (disclosed). D77–D96, SI-22–SI-28. |
| S5a-1 build marker, connect, start protocol | done | `[MEASURED]` #408 → `b994f11e` (05:43Z). PR gates: `cargo test --features mcp` 6,048 passed, 0 failed; `cargo test` 5,458 passed. Review outcome not recorded in the PR body `[UNKNOWN]`; S5a-2 folded three IMMATERIAL SMELLs from its Opus round 1. |
| S5a-2 spawn, fail-closed, pump, `--shared` | done | `[MEASURED]` #411 → `0c47020b` (08:27Z). Round 1 Opus FIX / sol FIX folded; round 2 (cap) both APPROVE, F-A/F-B/F-D folded after the cap (disclosed). PR gates at `26d2d599`: 6,071 / 5,458 passed. F-C follow-up §4 row 16. |
| Mutant re-point | done | `[MEASURED]` #410 → `e50ff853`: four anchors stale after #403 (`CACHE_VERSION` 118 → 119: MO-18, PD-11, P2-M11) and #406 (AF-40, `certify_anonymous_site`); each killed 1/1. |
| S5b plan | done | `[MEASURED]` #412 → `dc20c8da` (16:20Z). Round 1 Opus FIX / sol FIX folded test first; round 2 (cap) both APPROVE, one test + wording folded after the cap (disclosed). D97–D109, SI-29–SI-35. |
| S5b-1 control exchange in the daemon | done | `[MEASURED]` #416 → `30df802d` (17:30Z). Round 1 both APPROVE (a test-only `BrokenPipe` race folded). PR gates: 6,147 / 5,504 passed; Linux `a2a-toolchain` shim 47, daemon 150, `--test mcp` 46. SI-35 disclosed. |
| S5b-2 `daemon-status`, `daemon-shutdown` | done | `[MEASURED]` #418 → `61c2a9e5` (19:32Z). Round 1 sol FIX / Opus APPROVE folded; round 2 (cap) Opus APPROVE / sol FIX (WRONG/MATERIAL: an `lstat` error taken for absence) folded test first after the cap, disclosed. PR gates: 6,204 / 5,521 passed; Linux shim 80, daemon 150, `--test mcp` 52. `one_line` follow-up §4 row 17. |
| S6 plan | in progress (another agent) | `[INHERITED]` controller brief; `[MEASURED]` worktrees `~/code/prism-s6-{plan,validate}` at `61c2a9e5` |
| C2-M1 plan | pending | `[MEASURED]` no M1 plan file in this directory; metric decided (spec §6.1 as revised) |
| B (default flip) | pending | owner decision after the S6 soak |
| Optimization: Go type-extract speedup | done | `[MEASURED]` plan #397 → `a1aa755b`; #401 → `cc2e612c` (2026-10-15T03:44Z). Acceptance `~/prism-evidence/mcp-memory/go-speedup-acceptance-2026-10-15/SCORE.txt`: prometheus warm index base 12,469 / 11,346 ms → cand 1,952 / 1,959 ms (16.4 % of base); `go.file_extract` 9,798 → 323 ms; cold whole Go extraction 19,804 → 1,555 ms; heap and peak unchanged; slicing control PASS. |
| Optimization: release v3.2.0 | done | `[MEASURED]` #405 → `65bdf035`: `release.yml` also builds `prism-mcp` and uploads `prism-mcp-` + target for both macOS targets |
| DFG compaction plan | done | `[MEASURED]` #398 → `b4a1ffe8` (six PRs, A–F) |
| DFG PR A (frozen oracle + pins) | done | `[MEASURED]` #407 → `5a63c7e8` |
| DFG PR B (ordered views, consumer migration) | done | `[MEASURED]` #413 → `14a9932d`; round 1 Opus APPROVE / astra FIX folded |
| DFG PR C (compact arena, updates over IDs) | done | `[MEASURED]` #415 → `cf22c8e8`; round 1 Opus + astra APPROVE; gates 6,127 passed, oracle 166/0/1, full mutation gate 493/493. Evidence `~/prism-evidence/dfg-compaction/prC-2026-10-16/` |
| DFG PR D (native codec, test-only) | open | `[MEASURED]` #417 OPEN, head `b6da8226`; round 1 astra FIX / Opus APPROVE folded; gates at `b6da8226` 6,201 passed, oracle 166/0, mutation gate 496/496 |
| DFG PR E (activate, cache version bump), PR F (delete oracle) | pending | `[INHERITED]` plan; E after D merges |
| DFG CI jobs | done | `[MEASURED]` `.github/workflows/ci.yml` jobs "DFG oracle compile check" (`cargo check --all-targets --features mcp,dfg-oracle`, every PR) and "DFG oracle parity" (path-filtered by `scripts/ci/dfg-oracle-paths.sh`; runs `--test dfg_compaction -- --skip pins::`) |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `src/mcp/session.rs`, comment above `build_state` | "`LoadedRepo` is not `Sync`" | `[MEASURED]` **corrected in code**: the comment now says `SessionProvider` is `Send + Sync`, asserted at compile time in `src/mcp/daemon/mod.rs`. |
| `CLAUDE.md`, "CPG cache v73" | cache version | `[MEASURED]` **corrected**: `CLAUDE.md` says "currently v120"; `src/cpg_cache.rs` has `CACHE_VERSION: u32 = 120`. PR E bumps it again. |
| Spec §4.2 | "After Failed, the next readiness-requiring call from any connection starts exactly one retry" | `[MEASURED]` **corrected in the spec** (clarification dated 2026-10-13, plan D32/SI-8). |
| Spec §6.1 | attribution by "tree nodes × per-node size" | `[INHERITED]` not a bound in either direction; recorded in spec §13 and at the top of the C1 plan. |
| Spec §6.1–§6.2 | the M2 transient can be read from the peak | `[INHERITED]` in most runs the footprint had not come down by the next sample; M2's estimate is the cache file size. |
| Spec §6.1 | steady state = `phys_footprint` after a 5 s settle; five corpora | `[INHERITED]` corrected in the spec 2026-10-13 (owner decisions 8 and 9). |
| Spec §6.2, row M2 | the transient is the file bytes read on load | `[INHERITED]` three transients, the largest on save; recorded in the README and the M2 plan; the spec row is not edited. |
| `C2-M2-cache-streaming.md` decision rule | "the load half is retried once with the memory-mapped reader" | `[INHERITED]` superseded by the owner's override (#389). |
| Previous revision of this file, §0(c) | deployed binaries are `9d5a67bc` | `[MEASURED]` now `1c10c1e23f8c` (#403, post-#401, pre-S5a); see §0(c). |
| Previous revision of this file, row 14 | "two plans being written" | `[MEASURED]` both merged (#397, #398); see §2. |
| Memory `project_prism_mcp_memory_lane.md` | status through S4 plan | not updated by this docs PR; the controller updates it. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 22 | Bridge fan-out across worktrees (owner request 2026-10-16) | follow-up; options documented, no implementation | choose shared task worktree vs capped/staggered worktrees from measurements; any content-addressed cross-worktree index needs a separate correctness design | owner choice / design | `docs/MCP.md` §Bridge fan-out across worktrees; `prism-mcp daemon-status`; soak `max_daemons_per_repo` (per canonical worktree) |
| 9 | S6 plan | in progress (another agent) | review it when its PR opens (Opus 5.5 + gpt-6.1-sol, cap 2) | its PR | `~/code/prism-s6-plan`, branch `plan/s6-launcher-docs-soak` |
| 18 | M1 plan | pending | write against spec §6.1 as revised; add an `m1` row set to `memreport compare` | — | `scripts/memreport/README.md` |
| 14 | Optimization lane: DFG compaction | PR D in review | merge #417 on approval + green CI; then plan PR E (activate the compact layout, bump `CACHE_VERSION` from 120), then PR F (delete the oracle). Re-pin at the start of each PR | #417 | plan `2026-10-14-dfg-compaction.md`; `~/prism-evidence/dfg-compaction/prC-2026-10-16/` |
| 19 | Redeploy `prism-mcp` | owner/controller call | the deployed binary (`1c10c1e2`) has no `--shared`; S6's soak needs a post-S5b build | S6 | §0(c) paths |
| 15 | Daemon accept loop: EMFILE/ENFILE spin | follow-up, still open | `[MEASURED]` `src/mcp/daemon/server.rs` still logs any other `accept` error and retries next turn (no back-off). On Linux, too many open files retries every turn (100 % CPU, log fills to its 8 MiB cap); needs ~2 × `--max-clients` over the fd limit. Back off ~100 ms and drop the listener from `poll` meanwhile | — | #395 review (Opus F1, IMMATERIAL) |
| 16 | Lib tests that spawn children can hold a dropped `flock` until `exec` | follow-up | `[MEASURED]` #411 review F-C / sol 1, not folded; predates S5a; CI's nextest (one process per test) is immune; local `cargo test` can see it | — | #411 |
| 17 | `one_line` escapes only Unicode `Cc` | follow-up | `[MEASURED]` #418 review Opus S4, deferred; predates S5b (from S5a); recorded in the S5b plan's "Stated and not held" list | — | #418 |
| 20 | SI-35: an S5b `daemon-status` probe restarts a pre-S5b daemon's idle clock | disclosed, no fix | stop old daemons with `kill -TERM` | — | #416, #418 |
| 11 | Nav call-edge sidecar temp leakage | follow-up, pre-existing | `src/navigation/call_edge_cache.rs` writes pid+nonce temp names with no cleanup; apply the M2 lock-file pattern | — | #380 review r3 (Opus S6) |
| 12 | SI-10: builder panic fatal (spec) vs. failed-and-retry | owner-informed; S5a bounded the restart loop with D77's 60 s cooldown | none unless the owner reverses | — | S3b plan SI-10, D42; S5a D77 |
| 13 | SI-11: a refresh holds the write lock for a whole rebuild | measure | at the §6.2 fan-out gate before any change | — | S3b plan SI-11 |
| 21 | S5b dead keys' files: pruning flag | follow-up if the §8 soak shows they matter | — | S6 soak | README decision 14 |
| 6 | Shim-side peer-uid check | done | kept (owner D78), built in S5a | — | #408/#411 |
| 4, 5, 7, 8, 10 | Spec §4.2 sentence; sub-floor cap; retry interval; superseded call; background-start parity | done | — | — | previous revision of this file |
| 3 | M2 | done | — | — | `~/prism-evidence/mcp-memory/c2-m2/` |

## 5. Invariants and traps — do not do these
- Never edit `src/mcp/transport_tests.rs` or `src/mcp/owner_transport_tests.rs`, and never re-record the golden transcripts, in S2–S6 — together they are the proof that standalone stdout is unchanged.
- Never present the mutation gate as covering this lane — it builds without `--features mcp` and selects no MCP code.
- Never write a second S6 plan — another agent is writing it (§0(a)).
- Never probe a pre-S5b daemon with `daemon-status` and read its idle time afterwards — the probe restarts that daemon's idle clock (SI-35); stop old daemons with `kill -TERM`.
- Never treat a local `cargo test` failure in a test that spawns children and takes an `flock` as a regression without running it under nextest or alone — a sibling test's child can hold the dropped lock until `exec` (F-C).
- Never skip the DFG pins at the start of a compaction PR — CI's parity job runs with `--skip pins::`; the pins are a controller gate.
- Never compare cross-build DFG replay outputs byte-for-byte without replacing exactly the three SARIF provenance fields and targets `/producer/build_identity` (each validated against its binary's receipt) — those differ by build, and nothing else may be normalized.
- Never attribute memory by freeing something and re-measuring, and never read a footprint taken seconds after a load as an idle figure — the footprint sits at one of two levels and the move between them is not reproducible in time.
- Never gate a C2 experiment on a `phys_footprint` whose level is unknown, and never judge M1 on live heap alone.
- Never run a second prism-mcp while a measurement is running.
- Never treat runtime-directory validation as the isolation guarantee between users — the guarantee is the peer-uid check on both ends of the socket (daemon S4, shim S5a D78).
- Never run the kubernetes corpus on this host — projected first build 27–70 GiB against 24 GiB of memory.
- Never run the memreport driver into a directory that already has results, and never re-run to change a rule: `summarize --run DIR` rescores saved evidence.
- Never call `mem_tree_alloc::install()` from library code or a library test.
- Never make a `FatalOnUnwind` guard before its function takes its lock (S3b D47).
- Trap (S5b-2 round 2): an `lstat` error other than `NotFound` read as "absent" made `daemon-status` report a live key as dead files → only `NotFound` (and `NotADirectory`) is absence; report any other error.
- Trap (S5a-2 round 2): ksh93 reads `\xHH` greedily → shell-quoted byte escapes are three-digit octal.
- Trap: a test that asks "what would another user be told" passed on macOS and failed on Linux CI (sticky `/tmp`) → assert the property, not the directory that reports it first.
- Trap: when a review finding repeats, look for the design that removes the class rather than patching a third time.
- Trap: two implementers in one worktree, one ran `git commit --amend` onto the other's commit → separate worktrees, or forbid `--amend` in the brief.
- Trap: `gh pr merge --match-head-commit` needs the full 40-character id.
- Trap: this host has 24 GB and is usually several GB into swap; several concurrent Rust builds push it further. Stagger reviewers and verification; time criteria need a quiet window.
- Trap: an age-based cleanup assumed "in use means recently modified"; a cache hit never touches the file → key cleanup on something a reader updates, or state the cost.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Spec | `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` |
| Plans | `docs/superpowers/plans/2026-10-12-shared-prism-daemon/{README,S1-transport-refactor,C1-memory-harness,S2-identity-preamble,C2-M2-cache-streaming,S3a-readiness-coordinator,S3b-concurrent-dispatch,S4-daemon-ownership,S5a-shim,S5b-daemon-control}.md` |
| Optimization plans | `docs/superpowers/plans/2026-10-14-{go-type-extract-speedup,dfg-compaction}{,-validation}.md` |
| Merge commits (this refresh) | S5a plan `557c8550`; S5a-1 `b994f11e`; S5a-2 `0c47020b`; re-point `e50ff853`; S5b plan `dc20c8da`; S5b-1 `30df802d`; S5b-2 `61c2a9e5`; Go speedup `cc2e612c`; v3.2.0 `65bdf035`; DFG plan `b4a1ffe8`, A `5a63c7e8`, B `14a9932d`, C `cf22c8e8` |
| Open PR | #417 `perf/dfg-codec` head `b6da8226e03181a16e82a3eb4a99c18952ed64dc` |
| Deployed `prism-mcp` | `~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release/prism-mcp` = `~/code/slicing/target/release/prism-mcp`, sha256 `9d6551803f57504264ce196b79ca64fa93d19267f2f0629d16b0230efa7f8e48`, built from `1c10c1e23f8c`; backups `prism-mcp.pre-401`, `prism-mcp.pre-389` |
| Worktrees | S6 `~/code/prism-s6-plan` (`plan/s6-launcher-docs-soak`), `~/code/prism-s6-validate` (`scratch/s6-validate`); S4 `~/code/prism-s4-validate`; probe `~/code/prism-opt-probe`; this docs PR `~/code/prism-lane-docs` |
| Evidence | `~/prism-evidence/mcp-memory/{c1,c2-m2,optimization-2026-10-14,go-speedup-acceptance-2026-10-15}/`; `~/prism-evidence/dfg-compaction/prC-2026-10-16/` |
| Corpora | ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43` under `~/code/bench-repos/`; slicing = `~/code/prism-c1-corpus` |
| Reviewers | daemon lane: Opus 5.5 subagent + `codex exec -m gpt-6.1-sol -c model_reasoning_effort=xhigh -s workspace-write -C <clone> -o <out> - < brief`; optimization lane: gpt-6-astra plans, gpt-6.1-sol implements, Opus 5.5 + gpt-6-astra review |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT APPLICABLE — this refresh carries no new claim-bearing content beyond merge states and PR-recorded gates, each probed with `gh pr view`. Earlier verdicts stand as recorded in the previous revision (#396): S1 stdout byte-identity SURVIVED (PR #361); M1/M2 advance, M3 does not SURVIVED (PR #363); `phys_footprint` after 5 s REFUTED (readout); runtime-directory privacy by path checks REFUTED (PR #364).

**Questions the owner owes an answer to:** None open. Decisions of 2026-10-15/16 are recorded in `README.md` decisions 12–15 and in the DFG compaction plan (CI oracle jobs, pins as a controller gate, cross-build `/producer/build_identity`). Redeploying `prism-mcp` (§4 row 19) is a controller/owner call, not a question owed.
