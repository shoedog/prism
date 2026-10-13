# Handoff — shared per-repo prism daemon lane; S2 (a and b) and the driver extension merged, M2 experiment running, S3a plan next

**Written:** 2026-10-13T16:45-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5) · **Provider:** claude
**Workspace:** `~/code/prism-c2-plan` · `plan/c2-m2-cache-streaming` (this docs PR) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git status --short && git log --oneline -3` · Output inline in the session
**Predecessor:** this session's own handoff of 2026-10-13T16:10-06:00 (same file, previous revision), and before it Claude Code session `afaf5155-ea5e-4ad5-b8d8-be35f747f74d` (Fable 5.1)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` this session is the only one on the lane; another session works the JS/TS parameter-Def lane and does not touch `src/mcp`. — **RESOLVED 2026-10-13**
**(b) Custody exposure** — `[MEASURED]` on `main` (`c785fa4f`): S1, C1, spec, plans, readout, S2a, S2b, the driver extension. Pushed branches with open PRs: `perf/cpg-cache-streaming` (#369, draft), `plan/c2-m2-cache-streaming` (#366, this PR). Local only: `scratch/s2-validate` and `scratch/c2-m2-validate` (superseded; safe to delete). Raw evidence is local only under `~/prism-evidence/mcp-memory/{c1,c2-m2}/`; the M2 experiment's run directories are being written there now. — **OPEN until the experiment's evidence is checksummed**
**(c) In flight / irreversible** — `[MEASURED]` the M2 experiment started 2026-10-13 16:36: `memreport.py run --kinds warm,workload,build --reps 3` on four corpora, baseline (`e2701e89`) into `~/prism-evidence/mcp-memory/c2-m2/baseline-1`, then candidate (`6770c624`) into `candidate-1`; about three hours in all. **Do not start a cargo build, a test run or another prism-mcp while it runs.** Check `pgrep -fl memreport.py` and the tail of `baseline-1.log` / `candidate-1.log`. The host cannot be made quiet: desktop applications hold the load average near 35, and another session starts test runs from time to time (the driver records the load average of every run). — **OPEN**
**(d) Authorization granted but not exercised** — owner, 2026-10-13: answers to the lane questions (table in `README.md`), "write M2 experimenet plan and continue", and later "approved, waive check 3 for M2" (the fan-out latency gate, M2 only; spec §11 item 11). Not yet exercised: running the M2 experiment and merging #369 if its criteria pass. Still open with the owner: §7.

## 1. Resume order
1. Read `README.md` in this directory: slice table, the decision table of 2026-10-13, measured facts, and the notes for S3a, S4 and S5a (several came out of the S2a and S2b reviews).
2. M2 experiment (draft PR #369, head `6770c624`): when both runs have finished, follow `C2-M2-cache-streaming.md` Task 3 from Step 3: `memreport.py compare --experiment m2 --baseline ~/prism-evidence/mcp-memory/c2-m2/baseline-1 --candidate …/candidate-1 --out docs/analysis/2026-10-c2-m2-cache-streaming-readout.md --json …/compare.json`; check one corpus per role by hand against the raw reports; add the hand-written section; `SHA256SUMS`; remove the two `cache/` directories. If a time verdict is *unresolved*, re-measure times alone (`--kinds warm`, and `build` if needed), alternating the two binaries. Then `cd eval && uv run tier-a --quick --allow-stale-sut` on the candidate's release build, rebase #369 onto `main`, two reviewers on the code and the readout (cap two rounds), merge when the criteria pass. The fan-out check is waived (spec §11 item 11).
3. S3a plan: write it from the merged interfaces (`src/mcp/shared/*`, `ClientSettings`, `serve_connection`, `dispatch_tool_read`/`refresh`/`auto_refresh`, `LazySessionProvider`) and the README notes for S3a; validate its code on a scratch branch before hand-off, as S1, C1 and S2 were. No cargo while the experiment runs.
4. M1 plan: against spec §6.1 as revised (live heap with a pinned minimum effect size, plus a footprint confirmation). The driver's `compare` needs an `m1` row set.

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
| S2a (plan Tasks 1–4) | done | `[MEASURED]` #364 squash-merged as `802844a8`. Two review rounds plus one confirmation pass (one step past the cap, disclosed). Review turned up four ways a second local user could alter or replace the runtime directory; all fixed in code, and the contract restated as hardening. Full suite 5,411 passed at `fdee64d0`; CI on Linux caught one host-specific test, corrected. |
| S2b (plan Tasks 5–8) | done | `[MEASURED]` #370 squash-merged as `8587c7b9`. Both reviewers approved in round 1; their non-blocking notes folded in one commit (not re-reviewed). Full suite 5,460 passed, 0 failed at `68f7112a`. Builds "refuse a sub-floor cap"; reversing that is a decoder change, not two lines. |
| Spec/plan PR #357 | done | `[MEASURED]` squash-merged as `dc3b6bbe` |
| C2 driver extension | done | `[MEASURED]` #367 squash-merged as `c785fa4f`. Two review rounds (both FIX each time) plus one check of the round-2 fix delta, which found a third instance of "cleanup can be separated from its check"; the cleanup was rewritten to delete only through a descriptor of the directory it created (not re-reviewed). 348 Python tests. Limit stated in its README: raw results are trusted as written. |
| C2-M2 plan and candidate | plan done; candidate verified; experiment running | `[MEASURED]` draft PR #369, head `6770c624`: full suite 5,334 passed, 0 failed; clippy 257 lines; mutation gate 230 of 230; Tier-A matrix 178 of 178; eight mutation probes each caught. Preliminary, slicing only: first-build peak 3,092 → 2,400 MiB, load peak 2,016 → 1,621 MiB, cache file byte-identical. `tier-a --quick` not run. Binaries: `~/prism-evidence/mcp-memory/c2-m2/bin/` with `COMMITS.txt`. |
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
| 1 | M2 experiment, readout, merge | running | §1 step 2 | the two runs | draft PR #369, `~/code/prism-m2`, `~/prism-evidence/mcp-memory/c2-m2/` |
| 2 | S3a plan | next | §1 step 3 | no cargo until #1's runs finish | lane README notes for S3a |
| 3 | Sub-floor result cap | open | owner: refuse (built, and what both reviewers recommend) or replace | owner | spec §11 item 5 |
| 4 | Shim-side peer-uid check | open | owner: confirm it as a requirement on S5a | owner | spec §13 S2a review record; README notes for S5a |
| 5 | M1 plan | pending | §1 step 4 | — | readout verdict table |
| 6 | Refresh after files changed | not measured | none planned | — | readout, "Other observations" |

## 5. Invariants and traps — do not do these
- Never edit `src/mcp/transport_tests.rs` or `src/mcp/owner_transport_tests.rs`, and never re-record the golden transcripts, in S2–S5 — together they are the proof that standalone stdout is unchanged.
- Never present the mutation gate as covering this lane — it builds without `--features mcp` and selects no MCP code.
- Never attribute memory by freeing something and re-measuring, and never read a footprint taken seconds after a load as an idle figure — the footprint sits at one of two levels and the move between them did not happen at a reproducible time (measured three ways in this lane; mechanism not identified).
- Never gate a C2 experiment on a `phys_footprint` whose level is unknown, and never judge M1 on live heap alone — the first can swing 460–790 MiB by accident, the second cannot fail.
- Never run a second prism-mcp while a measurement is running — five supplement runs overlapped a probe this session and had to be disclosed.
- Never treat runtime-directory validation as the isolation guarantee between users — two review rounds each found another way around path checks (mode bits, lists, lists on ancestors, mount options); the guarantee is the peer-uid check on both ends of the socket.
- Trap hit this session: a test that asks "what would another user be told" passed on macOS and failed on Linux CI, because the temp directory there is an entry of a sticky `/tmp` → assert the property (this user is named as the foreign party), not the directory that reports it first.
- Trap hit this session: a third review finding of the same kind ("the check and the act can be separated") was answered by changing the mechanism (act through a descriptor), not by a third patch → when a finding repeats, look for the design that removes the class.
- Trap hit this session: `shutil.rmtree(dir_fd=…)` does not exist before Python 3.11, and the system Python here is 3.9 → the driver recurses by descriptor itself.
- Trap hit this session: a review fix that adds a check can be undone by its own ordering (a mode read before a list is known absent) → when a reviewer names a race, add a step hook and a test that plays the other party at that step.
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

**§2c verdict:** REFUTED · claim: "a runtime directory that is a real directory, owned by this user, mode 0700, is private (spec §4.3)" · pass: INDEPENDENT (two reviewers, two rounds, each with probes on this host) · evidence tier: TEST-BACKED · record: PR #364; the tests in `src/mcp/shared/dir_chain_tests.rs` and the list tests in `runtime_dir_tests.rs`

**Questions the owner owes an answer to:**
1. A client-supplied result cap below the floor (12,000 characters): refuse the connection with an error naming the floor (built in S2b, and what both reviewers recommend), or serve it with the default of 80,000? A user's own misconfiguration never reaches this: the shim resolves its settings as a standalone server does (warning on its stderr, default applied). Only a non-conforming client is refused. "Replace" would need a decoder change and would be silent to the client.
2. Should the shim check the uid of the socket's peer, as the daemon does (spec §4.4 has only the daemon check)? Two review rounds of S2a concluded that checks on the runtime directory's path are hardening and this check is where the guarantee between users has to live. Recommended: yes; it is an S5a item and costs nothing before then.

Answered on 2026-10-13 and recorded in spec §11 items 4, 6–11 and in `README.md`: the runtime directory stays as the spec says; the peer-uid check stays in S4; `--warm-at-startup` is a flag on the daemon's command line; the C2 metric is live heap with a minimum effect size plus a footprint confirmation for M1; four corpora; S2 is implemented from `scratch/s2-validate`; the fan-out latency gate is waived for M2 only.
