# Handoff — shared per-repo prism daemon lane; S3a-2 in its last review round, M2 save half in review, M2 load half measured at 06:00

**Written:** 2026-10-13T22:00-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5) · **Provider:** claude
**Workspace:** `~/code/prism-c2-plan` · `docs/daemon-lane-handoff-late` (this docs PR) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git status --short && git log --oneline -3` · Output inline in the session
**Predecessor:** this session's own handoff of 2026-10-13T16:45-06:00 (same file, previous revision), and before it Claude Code session `afaf5155-ea5e-4ad5-b8d8-be35f747f74d` (Fable 5.1)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` this session is the only one on the lane; another session works the JS/TS alias lane and does not touch `src/mcp` or `src/cpg_cache*`. — **RESOLVED 2026-10-13**
**(b) Custody exposure** — `[MEASURED]` on `main` (`e7dc1cc2`): S1, C1, spec, plans, readout, S2a, S2b, the driver, the M2 plan, S3a-1. Pushed branches with open PRs: `feat/mcp-daemon-s3a-2` (#377, head `15f67d84`), `plan/s3a-readiness-coordinator` (#373, head `04b51d57`), `perf/cpg-cache-streaming-save` (#380, `9f1e28b2`), `perf/cpg-cache-streaming` (#369, draft, to be closed), and `perf/cpg-cache-mmap-load` (`91e5a5af`, pushed, no PR yet). Raw evidence is local only under `~/prism-evidence/mcp-memory/{c1,c2-m2}/`; the M2 experiment's `SHA256SUMS` is written after the morning run. — **OPEN until then**
**(c) In flight / irreversible** — `[MEASURED]` a session cron (`3998d2d5`, session-only: it dies with the session) runs the M2 quiet-window measurements at **06:00 MDT 2026-10-14**, following `~/prism-evidence/mcp-memory/c2-m2/PROCEDURE-6am.md`; the three binaries and `bin/COMMITS-6am.txt` are in place. **From 06:00 until that run ends, start no cargo build, no test run, no review and no other prism-mcp on this host.** If this session is gone by then, a successor runs the procedure by hand. — **OPEN**
**(d) Authorization granted but not exercised** — owner, 2026-10-13: "1 ship the save half now. Can we schedule the other half for 6AM MDT?" (exercised: #380 open; merges on approval plus green CI); the D26 answer ("Building/indexing … retry"), built in S3a-2; "approved, waive check 3 for M2". Still open with the owner: §7.

## 1. Resume order
1. Read `README.md` in this directory: slice table, decision tables, measured facts, notes for S3a/S4/S5a.
2. **S3a-2 (#377):** round 2 of 2 is running (Opus 5.5 and gpt-6.1-sol; brief and reports under the session scratchpad, `reviews/s3a2-r2-*`). On APPROVE from both plus green CI: squash-merge with `--match-head-commit <full sha>`, then merge the plan #373, then apply SI-8's sentence to spec §4.2 in this docs PR. On FIX: classify per the convergence rule (cap reached; fold only if converging, disclose the extension).
3. **M2 save half (#380):** after CI is green, two reviewers (brief: session scratchpad `review-m2save-brief.md`), cap two rounds; merge on approval plus green CI. Fill the PR body's mutation-gate and Tier-A lines from `m2save-gates/`.
4. **06:00 run** (§0 (c)): report P1/P3/P5 for the mapped reader and P2/P4 for the save half; write the M2 readout `docs/analysis/2026-10-c2-m2-cache-streaming-readout.md` and `SHA256SUMS`. The load half gets a PR and reviews only if P1 passes and P3/P5 hold; otherwise it closes as "not worth it" (the plan's one retry). Then close #369.
5. **S3b plan:** from the merged S3a interfaces (`DaemonState`, `serve_admitted`, `run_tool_call`, `Answer`), the S3a plan's "interfaces for later slices", and the README notes.
6. **M1 plan:** against spec §6.1 as revised. The driver's `compare` needs an `m1` row set.

**STOP conditions:** a corpus whose projected peak would exhaust this host's memory (24 GB): do not run it. A reviewer WRONG finding the controller disagrees with: escalate to the owner. A golden transcript changing in any later slice. A review finding of a kind already folded twice: escalate, do not patch a third time.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Part one: build on first tool call | done | `[INHERITED]` #356, main `cb0eeec0` |
| Spec v3 | done | `[MEASURED]` owner-approved 2026-10-12; §11 items 4–11 and §13 records added 2026-10-13 |
| S1 transport refactor | done | `[MEASURED]` #361 → `f6354e07` |
| C1 harness and readout | done | `[MEASURED]` #362 → `7db8f195`; readout #363 → `e2701e89` |
| S2a / S2b | done | `[MEASURED]` #364 → `802844a8`; #370 → `8587c7b9` |
| C2 driver extension | done | `[MEASURED]` #367 → `c785fa4f` |
| M2 plan and decisions | done | `[MEASURED]` #366 → `86756aad` |
| S3a plan | in review with S3a-2 | `[MEASURED]` #373, head `04b51d57` (adds Tasks 11–12, D32, D33, SI-8, review-fold record) |
| S3a-1 coordinator and slot | done | `[MEASURED]` #374 → `e7dc1cc2` |
| S3a-2 one admitted connection | round 2 running | `[MEASURED]` #377, head `15f67d84`. Round 1: sol APPROVE, Opus FIX F1 (a timed-out client never heard its attempt's failure; the daemon retried on every call). Folded test first (D32, `answer_for` table tests, blocking stream, D33); 5 + 2 new mutants and the 19 earlier ones all killed; `mcp::daemon` 63 tests, 30 of 30 identical runs. Round-1 full suite 5,611 passed at `5e58e646`; the full suite on `15f67d84` is running (`s3a2-verify-2`). |
| M2 experiment (full candidate) | done | `[MEASURED]` `compare-1.md`: P1 load peak passes on 4 corpora (−19 to −24 %); P2 first-build peak passes on 3 (ruff n=2); P5 live heap lower everywhere; P3/P4 not resolvable on a loaded host (±20 % run to run). |
| M2 save half | in review | `[MEASURED]` #380, `9f1e28b2`: full suite 5,604 passed, 0 failed; clippy 257 lines (none new); mutation gate and Tier-A running (`m2save-gates/`). Ships before its own P4 by owner decision. |
| M2 load half (memory-mapped reader) | built, measured at 06:00 | `[MEASURED]` `perf/cpg-cache-mmap-load` `91e5a5af` (on the save half; `memmap2 =0.9.8`); 29 cache tests pass; no PR yet |
| C2-M1 plan | pending | metric decided (spec §6.1 as revised); not written |
| S3b–S6, B | pending | each planned when its predecessor merges |

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
| Memory `project_prism_mcp_memory_lane.md` | status lines | Updated 2026-10-13 late for S2b, the driver, S3a-1, the M2 split and the 06:00 run. |
| Spec §4.2 | "After Failed, the next readiness-requiring call from any connection starts exactly one retry" | `[MEASURED]` disagrees with §5 for a client whose wait ran out before the failure (S3a-2 review, Opus F1). D32 reads it per connection; the one-sentence clarification (S3a plan SI-8) is applied to the spec once the S3a-2 round-2 reviewers have judged it. |
| `C2-M2-cache-streaming.md` decision rule | "the load half is retried once with the memory-mapped reader" | `[MEASURED]` the save half shipped before its own P4 was measured, by owner decision ("1 ship the save half now"); P4 and the mapped reader's P1/P3 are measured at 06:00 2026-10-14. |


## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | S3a-2 merge, then plan #373 | round 2 | §1 step 2 | reviews | #377, #373, `~/code/prism-s3a2` |
| 2 | M2 save half | review | §1 step 3 | CI | #380, `~/code/prism-m2-save` |
| 3 | M2 06:00 run and readout | scheduled | §1 step 4 | 06:00 | `PROCEDURE-6am.md`, `~/code/prism-m2-load` |
| 4 | Spec §4.2 sentence (SI-8) | drafted | apply after round 2 | #1 | S3a plan SI-8 |
| 5 | Sub-floor result cap | open | owner: refuse (built) or replace | owner | spec §11 item 5 |
| 6 | Shim-side peer-uid check | open | owner: confirm as an S5a requirement | owner | README notes for S5a |
| 7 | Retry interval in the `indexing` answer | open | owner: keep 5 s or choose | owner | S3a plan D26 |
| 8 | Superseded call when the index is already ready | open | owner: tell it to retry (built, D26) or serve it | owner | S3a plan "Decisions owed" |
| 9 | S3b plan, M1 plan | pending | §1 steps 5–6 | #1 | — |

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
- Trap hit this session: M2 timings taken while other sessions' builds and this session's reviews ran were useless (a ruff build timed out at 1,800 s; an interleaved re-run showed ±20 % noise) → time criteria need a quiet window; peaks were stable under the same load.
- Trap hit this session: a second-round decision that "reversing X is a two-line change" was wrong because the decoder accepts only its own canonical spelling → check a claimed small change against the code before writing it into a decision table.
- Never run cargo, a review or a measurement between 06:00 and the end of the morning run (see §0 (c)).

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Spec | `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` |
| Plans | `docs/superpowers/plans/2026-10-12-shared-prism-daemon/{README,S1-transport-refactor,C1-memory-harness,S2-identity-preamble,C2-M2-cache-streaming}.md`; S3a plan on #373 |
| Worktrees | S3a-2 `~/code/prism-s3a2`; S3a plan `~/code/prism-s3a-plan`; M2 save `~/code/prism-m2-save`; M2 load `~/code/prism-m2-load`; M2 base build `~/code/prism-m2-base` (detached `e7dc1cc2`); driver `~/code/prism-c2-driver`; corpus slicing `~/code/prism-c1-corpus` |
| M2 binaries | `~/prism-evidence/mcp-memory/c2-m2/bin/{main,save,load}/prism-mcp`, sha256 in `bin/COMMITS-6am.txt` |
| M2 evidence | `~/prism-evidence/mcp-memory/c2-m2/{baseline-1,candidate-1,timing-*,compare-1.md}` |
| Corpora | ruff `44f6d1865fd2`, excalidraw `0642e72cfa2d`, prometheus `505095b64b43` under `~/code/bench-repos/`; slicing = `~/code/prism-c1-corpus` |
| Reviewers | Opus 5.5 subagent; `codex exec -m gpt-6.1-sol -c model_reasoning_effort=xhigh -s workspace-write -C <clone> -o <out> - < brief` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "S1 leaves the in-process server's stdout byte-for-byte unchanged" · pass: INDEPENDENT (two reviewers, each with a same-host base control or mutation probes) · evidence tier: TEST-BACKED · record: PR #361; goldens recorded on the base and CI-green alone (run 38402295489), unchanged through the merge

**§2c verdict:** SURVIVED · claim: "M1 and M2 advance and M3 does not, on four corpora" · pass: INDEPENDENT (Opus 5.5 and gpt-6.1-sol each recomputed the figures from the raw reports; also recomputed against a second base, the live heap) · evidence tier: MEASURED · record: PR #363; readout, section "Verdict"

**§2c verdict:** REFUTED · claim: "`phys_footprint` after a 5 s settle is a repeatable steady-state figure (spec §6.1)" · pass: SELF, by the plan's own cross-check against the 2026-10-09 figures, then a report-mode run at 60 s, normal-mode sampling from outside and a 12-run supplement; both reviewers reproduced the counts · evidence tier: MEASURED · record: readout, section "The Steady state column is not an idle figure"

**§2c verdict:** REFUTED · claim: "a runtime directory that is a real directory, owned by this user, mode 0700, is private (spec §4.3)" · pass: INDEPENDENT (two reviewers, two rounds, each with probes on this host) · evidence tier: TEST-BACKED · record: PR #364; the tests in `src/mcp/shared/dir_chain_tests.rs` and the list tests in `runtime_dir_tests.rs`

**Questions the owner owes an answer to:**
1. A client-supplied result cap below the floor (12,000 characters): refuse (built in S2b; both S2b reviewers recommend it) or serve with the default? Only a non-conforming client ever reaches this.
2. Should the shim check the socket peer's uid, as the daemon does? Recommended: yes, as an S5a requirement.
3. The `indexing` answer's retry interval: 5 seconds is the implementer's placeholder. Keep it, or name a value?
4. A call whose failed attempt was superseded by a retry that has already made the index ready is told to retry (as decided under D26), though it could be served at once. Keep, or serve? (Opus, S3a-2 round 1, S2.)

Answered on 2026-10-13 and recorded in spec §11 items 4–11, `README.md` and the S3a plan: runtime directory as the spec says; peer-uid check in S4; `--warm-at-startup` a daemon flag; C2 metric live heap plus footprint confirmation; four corpora; S2 from `scratch/s2-validate`; fan-out gate waived for M2; a superseded failure answers a retryable `indexing` status (D26); ship the M2 save half now and measure the rest at 06:00.
