# Handoff — shared per-repo prism daemon lane; S6 merged and the soak running (Phase 1 tonight, Phase 3 to 2026-10-20); DFG compaction complete; optimization round 2 being planned

**Written:** 2026-10-17T22:45-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5, controller) · **Provider:** claude
**Workspace:** `~/code/prism-handoff` · `docs/daemon-lane-handoff-s6` (this docs PR, from `origin/main` `45f155c6`) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git log origin/main 61c2a9e5..origin/main`, `gh pr view` on #417, #422, #424, #439, #440, #442, #446, #451; `ps`, `ls`, `shasum` on the soak deployment · Output inline in the session
**Predecessor:** this session's handoff of 2026-10-16T13:35-06:00 (same file, previous revision, #421)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the controller. `[MEASURED]` claims were probed by this writer this session (git, `gh`, `ps`, file reads); `[INHERITED]` claims were not, and name their source.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` controller session `fd583811` owns the daemon lane, the soak and the optimization lane. The JS/TS lanes (#437–#450) are other sessions and do not touch `src/mcp`. — **RESOLVED**
**(b) Custody exposure** — `[MEASURED]` on `main`: everything through S6-2 (#440) and DFG compaction PR F (#451). Local-only evidence (no git custody): `~/prism-evidence/mcp-memory/s6-soak/` (the soak; SHA256SUMS under `deploy/`), `~/prism-evidence/dfg-compaction/{prF-2026-10-17,dfgE-merge-b2cd10a1}/`, `~/prism-evidence/mcp-memory/optimization-2026-10-14/`. Not yet merged: the EMFILE back-off (row 15, branch `fix/daemon-accept-emfile-backoff`, `~/code/prism-emfile`) and the optimization round 2 plan (`plan/optimization-round2`, `~/code/prism-opt2`), both in progress 2026-10-17 evening. — **RESOLVED** for main; the two branches are tracked in §4.
**(c) In flight / irreversible** — `[MEASURED]` **the S6 soak is running on this host** (plan `S6-launcher-docs-soak.md`, "The soak"):
- Deployed binary: `~/code/slicing/target/release/prism-mcp` and `~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release/prism-mcp` = the S6 build (S6-2 head `0ed288f0`, tree-identical to `9b2fa09b`; pinned copy `$E/bin/prism-mcp-50f7a5abcfdf`, sha256 prefix `50f7a5abcfdf37cf`). Plugin launcher from S6-2 (sha256 prefix `f0c8474d6c45aade`). Originals in `$E/deploy/originals/` (`E=~/prism-evidence/mcp-memory/s6-soak`).
- Opted in: all 13 stockTrading bridge configs (21 prism entries → `$E/bin/prism-mcp-logged.sh` + `--shared` + `PRISM_MCP_RUNTIME_DIR=/tmp`); `~/.zshrc` exports `PRISM_MCP_SHARED=1` and `PRISM_MCP_RUNTIME_DIR=/tmp` (two marked lines); `~/code/slicing/.codex/config.toml` points at the wrapper (local, uncommitted, `skip-worktree`; original in `$E/deploy/snapshots/`).
- Background processes: Phase 1 runner (`$E/fanout/runner/phase1.py`, waits for 02:00 local and a quiet host; STATUS in `$E/fanout/STATUS`), round sampler (`$E/round/samples.jsonl`, 5 s), week sampler (`$E/week/samples.jsonl`, 60 s, 259,200 s from T0 `$E/week/T0`). A host restart kills them: restart with the runbook commands (`--window-file` resumes the week sampler).
- **Do not** replace the deployed binary, edit the soaked configs, or start cargo/mutgate/measurements during 02:00–07:00 local while the soak is open. A soak finding is fixed in place under D132 (`--fix-at`), not by restarting the soak. — **OPEN** until the soak report
**(d) Authorization granted but not exercised** — `[INHERITED]` owner, 2026-10-17: "deploy the soak binary today after merging PR E"; "1 - all 2 - yes" (all 13 bridge configs; profile exports); soak "3 days, ≥ 2 nights", fix in place with a post-fix window; Codex "Point Codex at wrapper"; EMFILE "Do it anyway" (merge to main, not deployed during the soak); optimization round 2: measure all remaining candidates, advance what passes. Reviewer seats (owner 2026-10-17): Opus-5.5 (medium/high) + gpt-6.1-sol; astra only where Fable would be used; test-only changes get one reviewer (Sonnet-5.5), a second (gpt-5.6-terra) only with a stated reason. Standing: reviewer-approved + green CI + full authoritative mutgate → squash-merge.

## 1. Resume order
1. **Morning of 2026-10-18: read Phase 1.** `cat $E/fanout/STATUS`, `$E/fanout/runner/runner.log`, `$E/fanout/aa-compare.md`, `$E/fanout/excalidraw-compare.md`. If the A/A calibration is not PASS, escalate its figures to the owner and collect no more runs (plan stop condition).
2. **Phase 2:** waits for a real stockTrading bridge round from a soaked config. Twenty minutes after it ends: write T1, stop the round sampler, run `soak.py report --phase round` (runbook in the S6 plan, "Phase 2").
3. **Phase 3:** check the week sampler daily (age of the last sample). Nights 10-18 and 10-19 are the leak nights (sessions left open, host otherwise idle). Report after 2026-10-20 ~12:08 MDT; readout `docs/analysis/2026-10-shared-daemon-soak-readout.md`.
4. **After the soak:** roll back or keep per the readout (rollback order in the S6 plan, decision 5; restore `.codex/config.toml` from the snapshot and `git update-index --no-skip-worktree`); remove `~/code/prism-s6-2/target`; owner decision B (default flip).
5. **EMFILE back-off (row 15):** review (cap 2), full mutgate, merge; do not deploy during the soak unless the soak hits it.
6. **Optimization round 2 (row 23):** review the plan (astra wrote it); attribution PR first; first quiet window for measurements is 2026-10-21 02:00.

**STOP conditions:** a corpus whose projected peak would exhaust this host's 24 GB (never kubernetes). A reviewer WRONG the controller disagrees with: escalate. A finding class folded twice: escalate. Phase 1 A/A calibration not PASS: stop collecting, escalate. Disk below 50 GiB free: clean merged worktrees' `target/` first (`[MEASURED]` 110 GiB free at writing).

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| S1–S5b, C1, C2-M2 | done | previous revision (#421) and `README.md` |
| S6 plan | done | `[MEASURED]` #422 → `e71ad1fc` |
| S6-1 opt-in launcher, soak instruments | done | `[MEASURED]` #424 → `0a269d5e` |
| S6-2 fan-out driver, shared-mode docs, Codex config, leak rule | done | `[MEASURED]` #440 → `9b2fa09b`; soak binary built from its head `0ed288f0` (same tree) |
| S6 soak Phase 0 (deploy) | done | `[MEASURED]` `$E/deploy/NOTES.md`; probes 1–4 PASS (`$E/deploy/probes.log`; probe 4 = the Codex wrapper command, 2026-10-17 22:15) |
| S6 soak Phase 1 (fan-out latency) | next (02:00 2026-10-18) | `[MEASURED]` runner alive, STATUS `waiting` at 22:40 |
| S6 soak Phase 2 (one bridge round) | pending | needs a real stockTrading round from a soaked config |
| S6 soak Phase 3 (72 h) | in progress | `[MEASURED]` week sampler alive; T0 2026-10-17 ~12:08 MDT |
| DFG compaction PR D | done | `[MEASURED]` #417 → `e5a45634` |
| DFG compaction PR E (activate) | done | `[MEASURED]` #439 → `6a63429a`; acceptance `docs/eval/dfg-compaction/acceptance.md` (prometheus heap −1,106 MiB, slicing −342 MiB, outputs byte-identical) |
| DFG compaction PR F (retire oracle) | done | `[MEASURED]` #451 → `9fcecb27`; full mutgate 549/549; host mcp 6,518/0/3; evidence `~/prism-evidence/dfg-compaction/prF-2026-10-17/` |
| DFG CI jobs | retired | `[MEASURED]` removed by #451; `dispatch_large` has a nextest `ci` override (480 s) |
| EMFILE back-off | in progress | branch `fix/daemon-accept-emfile-backoff` (row 15) |
| Optimization round 2 plan | in progress | branch `plan/optimization-round2` (row 23) |
| C2-M1 plan | pending | `[ASSUMPTION]` overlaps round 2 candidate #3 (retained trees); decide there |
| B (default flip) | pending | owner decision after the soak |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Previous revision §0(c), row 19 | deployed binary predates S5a | `[MEASURED]` S6 build deployed 2026-10-17 (§0(c)); row 19 done |
| Previous revision §5 | "Never skip the DFG pins …", cross-build DFG replay normalization | `[MEASURED]` obsolete: the oracle and pins were retired by #451; removed below |
| Previous revision row 14 | DFG PR D in review | `[MEASURED]` A–F all merged; lane complete |
| `CLAUDE.md` cache version | — | `[MEASURED]` says v123, matches `src/cpg_cache.rs:268` |
| Previous revision §6 reviewers | Opus + sol (daemon), Opus + astra (optimization) | owner 2026-10-17 seats, §0(d) |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 24 | S6 soak | running | §1 steps 1–4 | time | `$E`; plan "The soak" |
| 15 | Daemon accept loop: EMFILE/ENFILE spin | in progress | review, full mutgate, merge; not deployed during the soak | — | `~/code/prism-emfile`; #395 review Opus F1. Trigger needs ~2 × `--max-clients` (64) over the fd limit: a latent bug, not expected on this Mac at bridge scale |
| 23 | Optimization round 2 | planning | review astra's plan; attribution counters PR; mechanism checks (daytime); one sequential quiet-window batch (from 2026-10-21 02:00); advance on the C1 rule | soak (quiet windows) | `~/code/prism-opt2`; analysis `~/prism-evidence/mcp-memory/optimization-2026-10-14/astra-optimization-analysis.md` (#1 refresh ownership, #2 early cache reject, #3 retained trees, #6 capacity, #7 lookup keys, #8 direct decode, #9 verifier/allocator; #4 and #5 shipped) |
| 22 | Bridge fan-out across worktrees | follow-up; options documented | choose shared task worktree vs capped/staggered worktrees from the soak's `max_daemons_per_repo` | soak readout | `docs/MCP.md` §Bridge fan-out across worktrees |
| 16 | Lib tests that spawn children can hold a dropped `flock` until `exec` | follow-up | — (nextest is immune) | — | #411 F-C |
| 17 | `one_line` escapes only Unicode `Cc` | follow-up | — | — | #418 Opus S4 |
| 20 | SI-35: `daemon-status` restarts a pre-S5b daemon's idle clock | disclosed | stop old daemons with `kill -TERM` | — | #416, #418 |
| 11 | Nav call-edge sidecar temp leakage | follow-up | apply the M2 lock-file pattern | — | #380 r3 Opus S6 |
| 12 | SI-10 builder panic fatal vs retry | owner-informed | none unless reversed | — | S3b SI-10; S5a D77 |
| 13 | SI-11 refresh holds the write lock for a rebuild | reported by Phase 1 | read Phase 1's refresh overlap | Phase 1 | S3b SI-11 |
| 21 | S5b dead keys' files: pruning flag | if the soak shows they matter | — | soak | README decision 14 |
| 18 | M1 plan | folded into row 23 (#3) | — | — | — |
| 14, 19, 9 | DFG compaction; redeploy; S6 plan | done | — | — | §2 |

## 5. Invariants and traps — do not do these
- Never edit `src/mcp/transport_tests.rs` or `src/mcp/owner_transport_tests.rs`, or re-record the golden transcripts — they prove standalone stdout is unchanged.
- Never present the mutation gate as covering MCP code — it builds without `--features mcp`; still run it before every code merge (owner rule).
- Never touch the soak (binary, soaked configs, profile lines, samplers) except under D132 — a restart discards the soak's nights.
- Never run cargo, mutgate, a measurement or a `codex exec`/bridge run during 02:00–07:00 while a soak phase or measurement is scheduled — the Phase 1 quiet gate counts them and halts if quiet is not reached by 04:45.
- Never `cat`/`tail`/copy `~/.zshrc` — it holds live API keys; grep exact lines only.
- Never run two memory measurements at once — they contaminate each other through swap and the compressor.
- Never probe a pre-S5b daemon with `daemon-status` and read its idle time afterwards (SI-35).
- Never treat a local `cargo test` failure in a child-spawning `flock` test as a regression without nextest or a solo run (F-C).
- Never attribute memory by freeing something and re-measuring; never read a footprint seconds after a load as idle.
- Never treat runtime-directory validation as user isolation — the guarantee is the peer-uid check on both ends.
- Never run the kubernetes corpus on this host (27–70 GiB projected against 24 GiB).
- Never run the memreport driver into a directory with results; rescore with `summarize --run DIR`.
- Trap (2026-10-17): a brief said "no cargo after 01:30" and the worker read it as already past at 12:15 → give explicit dates for every window.
- Trap (2026-10-17): a test moved from an uninstrumented CI job into a coverage shard exceeded nextest's 300 s → time moved tests under coverage before deleting their old job.
- Trap: `codex exec` with stdin open waits forever → always `< /dev/null`.
- Trap (S5b-2): only `NotFound`/`NotADirectory` is absence for `lstat`; report other errors.
- Trap: `gh pr merge --match-head-commit` needs the full 40-character id.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Spec | `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` |
| Plans | `docs/superpowers/plans/2026-10-12-shared-prism-daemon/` (S1–S6, C1, C2-M2); `docs/superpowers/plans/2026-10-14-{go-type-extract-speedup,dfg-compaction}{,-validation}.md` |
| Merge commits (this refresh) | S6 plan `e71ad1fc`; S6-1 `0a269d5e`; S6-2 `9b2fa09b`; DFG D `e5a45634`, E `6a63429a`, F `9fcecb27` |
| Soak evidence root | `E=~/prism-evidence/mcp-memory/s6-soak` (`bin/`, `deploy/`, `fanout/`, `round/`, `week/`, `client-logs/`) |
| Soak binary | `$E/bin/prism-mcp-50f7a5abcfdf` (S6-2 head `0ed288f0`); daemon key at Phase 0 `f2dab2ce82fd56d5` |
| Soak tools | `~/code/prism-s6-2/scripts/soak/{soak.py,fanout.py}` |
| Worktrees | `~/code/prism-s6-2` (keep until the soak ends), `~/code/prism-emfile`, `~/code/prism-opt2`, this docs PR `~/code/prism-handoff`; merged: `~/code/prism-dfg-f`, `~/code/prism-s6-1`, `~/code/prism-s6-validate` (targets deleted) |
| Corpora | excalidraw `0642e72c`, prometheus `505095b64b43`, ruff `44f6d1865fd2` under `~/code/bench-repos/`; slicing = `~/code/prism-c1-corpus` (`7db8f195`) |
| Implementers / reviewers | `codex exec -m gpt-6.1-sol -c model_reasoning_effort=high -s workspace-write -C <worktree> … < /dev/null`; reviewers per §0(d) |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT APPLICABLE — no claim-bearing content beyond merge states, PR-recorded gates and the soak's deployment record, each probed with `gh`/`ps`/file reads. The soak's own verdicts come from its report.

**Questions the owner owes an answer to:** None open.
