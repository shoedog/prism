# Handoff — shared per-repo prism daemon and memory lane: CLOSED 2026-10-30 (I0 idle unload active by default, deployed and checked in real use)

**Written:** 2026-10-17T22:45-06:00 · **By:** Claude Code session `fd583811-fdc5-4426-9c4a-55872d2ecc1c` (Opus 5.5, controller) · **Provider:** claude
**Workspace:** `~/code/prism-handoff` · `docs/daemon-lane-handoff-s6` (this docs PR, from `origin/main` `45f155c6`) · **Measured state:** `[MEASURED]` HEAD = the commit that adds this revision · Tree CLEAN after that commit · Probe `git log origin/main 61c2a9e5..origin/main`, `gh pr view` on #417, #422, #424, #439, #440, #442, #446, #451; `ps`, `ls`, `shasum` on the soak deployment · Output inline in the session
**Predecessor:** this session's handoff of 2026-10-16T13:35-06:00 (same file, previous revision, #421)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the controller. `[MEASURED]` claims were probed by this writer this session (git, `gh`, `ps`, file reads); `[INHERITED]` claims were not, and name their source.

## Lane closed — 2026-10-30 (supersedes every section below; they are kept as the 2026-10-17 record)

`[MEASURED]` Written by the controller session `fd583811` (Opus 5.5) at closing. Everything below this section is the
2026-10-17 revision and is historical.

- **Soak:** closed on Phase 1 (#497 → `85bf9ca2`); Phase 3 inadmissible; decision B (default flip of `--shared`) deferred by the owner.
- **Idle unload (I0):** freshness binding #500 (`ef18437f`); daemon lifecycle I0b-1 #508 (`6c46a465`) and options I0b-2 #512
  (`ef5c15fb`); O-lite lifecycle facts #516 (`ae49d937`); shim forwarding I0c-1 #517 (`8e994fb4`); memreport collector I0c-2
  #518 (`194d50e3`); `MallocSpaceEfficient=1` re-exec on macOS #537 (`af1675f9`); **activation #555 (`e71e3670`): the index
  unloads after 600 s idle by default in stdio and shared mode.** Confirmation run: unloaded footprint 5–19 MiB in all six
  cells, against about 1.9 GiB resident.
- **Guards:** the `macos-footprint` CI job #566 (`f2eb3bbe`); test fixes #525, #564, #565.
- **Deployed** 2026-10-29 from main `c84f3769` (prism-mcp sha256 prefix `8363c1b8b836`). **Real-use check** 2026-10-30: since the
  redeploy, 17 idle unloads all fired at 600 s, with disposal taking 0.4–6.4 s. There were 3 reloads: 3.1 s on a CPG cache hit
  and 14–19 s on a changed snapshot. The logs show no panics, errors or blocked-stderr symptoms. New-build processes sit at
  1.5–1.7 MB when unloaded or unused. The one ~2 GiB process is an old-build standalone server from a client session that
  predates the redeploy; it changes build when that session restarts.
- **Recorded follow-ups (issues or review records; none in flight in this lane):** #501; #538; #537's unfolded review items; the
  A+C relief readout returning 0 reload pairs; a more robust host-quiet ledger (#526 option 2); a host-quiet marker check in
  the bridge launcher; the 2026-10-17 rows 11, 16, 17, 20, 21 and 25 below. #507 is handled as PR #570 outside this lane.
- **Resume:** nothing to resume. A new memory or daemon slice starts from an owner decision; read `docs/MCP.md` (idle unload,
  environment table) and the specs and plans in this directory first.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` controller session `fd583811` owns the daemon lane, the soak and the optimization lane. The JS/TS lanes (#437–#450) are other sessions and do not touch `src/mcp`. — **RESOLVED**
**(b) Custody exposure** — `[MEASURED]` on `main`: everything through S6-2 (#440), DFG compaction PR F (#451) and the EMFILE back-off (#453). Local-only evidence (no git custody): `~/prism-evidence/mcp-memory/s6-soak/` (the soak; SHA256SUMS under `deploy/`), `~/prism-evidence/dfg-compaction/{prF-2026-10-17,dfgE-merge-b2cd10a1}/`, `~/prism-evidence/mcp-memory/optimization-2026-10-14/`. Not yet merged: the optimization round 2 plan draft (`plan/optimization-round2` `584b5ae6`, pushed, `~/code/prism-opt2`). — **RESOLVED** for main; the branch is tracked in §4.
**(c) In flight / irreversible** — `[MEASURED]` **the S6 soak is running on this host** (plan `S6-launcher-docs-soak.md`, "The soak"):
- Deployed binary: `~/code/slicing/target/release/prism-mcp` and `~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release/prism-mcp` = the S6 build (S6-2 head `0ed288f0`, tree-identical to `9b2fa09b`; pinned copy `$E/bin/prism-mcp-50f7a5abcfdf`, sha256 prefix `50f7a5abcfdf37cf`). Plugin launcher from S6-2 (sha256 prefix `f0c8474d6c45aade`). Originals in `$E/deploy/originals/` (`E=~/prism-evidence/mcp-memory/s6-soak`).
- Opted in: all 13 stockTrading bridge configs (21 prism entries → `$E/bin/prism-mcp-logged.sh` + `--shared` + `PRISM_MCP_RUNTIME_DIR=/tmp`); `~/.zshrc` exports `PRISM_MCP_SHARED=1` and `PRISM_MCP_RUNTIME_DIR=/tmp` (two marked lines); `~/code/slicing/.codex/config.toml` points at the wrapper (local, uncommitted, `skip-worktree`; original in `$E/deploy/snapshots/`).
- Background processes: Phase 1 runner (`$E/fanout/runner/phase1.py` v2, pid 42569 at writing; window 02:00–08:30 local on 2026-10-18 (owner: quiet until ~08:30 on Sunday), latest start 06:15, waits for a quiet host; STATUS in `$E/fanout/STATUS`), round sampler (`$E/round/samples.jsonl`, 5 s), week sampler (`$E/week/samples.jsonl`, 60 s, 259,200 s from T0 `$E/week/T0`). A host restart kills them: restart with the runbook commands (`--window-file` resumes the week sampler).
- **Do not** replace the deployed binary, edit the soaked configs, or start cargo/mutgate/measurements during the night's quiet window (02:00–07:00 local; 2026-10-18 to 08:30) while the soak is open. A soak finding is fixed in place under D132 (`--fix-at`), not by restarting the soak. — **OPEN** until the soak report
**(d) Authorization granted but not exercised** — `[INHERITED]` owner, 2026-10-17: "deploy the soak binary today after merging PR E"; "1 - all 2 - yes" (all 13 bridge configs; profile exports); soak "3 days, ≥ 2 nights", fix in place with a post-fix window; Codex "Point Codex at wrapper"; EMFILE "Do it anyway" (merge to main, not deployed during the soak); optimization round 2: measure all remaining candidates, advance what passes. Reviewer seats (owner 2026-10-17): Opus-5.5 (medium/high) + gpt-6.1-sol; astra only where Fable would be used; test-only changes get one reviewer (Sonnet-5.5), a second (gpt-5.6-terra) only with a stated reason. Standing: reviewer-approved + green CI + full authoritative mutgate → squash-merge.

## 1. Resume order
1. **Morning of 2026-10-18: read Phase 1.** `cat $E/fanout/STATUS`, `$E/fanout/runner/runner.log`, `$E/fanout/aa-compare.md`, `$E/fanout/excalidraw-compare.md`. If the A/A calibration is not PASS, escalate its figures to the owner and collect no more runs (plan stop condition).
2. **Phase 2:** waits for a real stockTrading bridge round from a soaked config. Twenty minutes after it ends: write T1, stop the round sampler, run `soak.py report --phase round` (runbook in the S6 plan, "Phase 2").
3. **Phase 3:** check the week sampler daily (age of the last sample). Nights 10-18 and 10-19 are the leak nights (sessions left open, host otherwise idle). Report after 2026-10-20 ~12:08 MDT; readout `docs/analysis/2026-10-shared-daemon-soak-readout.md`.
4. **After the soak:** roll back or keep per the readout (rollback order in the S6 plan, decision 5; restore `.codex/config.toml` from the snapshot and `git update-index --no-skip-worktree`); remove `~/code/prism-s6-2/target`; owner decision B (default flip).
5. **EMFILE back-off (row 15): merged** (#453 → `0841b1dc`); not deployed during the soak unless the soak hits it. Its follow-ups are row 25.
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
| EMFILE back-off | done | `[MEASURED]` #453 → `0841b1dc`; sol + Opus round 1 APPROVE (no WRONG); full mutgate 570/570 (selects no MCP code); host daemon tests 161/0 |
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
| 15 | Daemon accept loop: EMFILE/ENFILE spin | done | #453 → `0841b1dc`: log once per episode, listener out of `poll` for 100 ms; not deployed during the soak | — | #395 review Opus F1 |
| 25 | EMFILE follow-ups (#453 review, all IMMATERIAL) | follow-up | S1 end the log-once episode only on WouldBlock (churn at the limit logs ~1/100 ms); S2 ENOMEM/ENOBUFS still retry every turn; S3 two 'served during back-off' tests weaker than their names; S4 bound-socket and socket-free test files duplicate scenarios; S5 doc comments on `wait`/`accept_all`/`accept_retry_at`; sol: combined back-off cases lack own tests | — | `~/prism-evidence/mcp-memory/emfile-2026-10-17/review-r1-{opus,sol}.md` |
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
- Never run cargo, mutgate, a measurement or a `codex exec`/bridge run during 02:00–07:00 while a soak phase or measurement is scheduled — the Phase 1 quiet gate counts them and halts if quiet is not reached by its latest start (04:45 by default; 06:15 on 2026-10-18).
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
- Trap (2026-10-17): `phase1.py --help` started a second runner (the script takes no arguments) → never invoke the runner to test it; `python3 -m py_compile` only, and `pgrep -fl phase1.py` after any relaunch.
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
| Worktrees | `~/code/prism-s6-2` (keep until the soak ends), `~/code/prism-opt2`, this docs PR `~/code/prism-handoff`; merged: `~/code/prism-dfg-f`, `~/code/prism-emfile`, `~/code/prism-s6-1`, `~/code/prism-s6-validate` (targets deleted) |
| Corpora | excalidraw `0642e72c`, prometheus `505095b64b43`, ruff `44f6d1865fd2` under `~/code/bench-repos/`; slicing = `~/code/prism-c1-corpus` (`7db8f195`) |
| Implementers / reviewers | `codex exec -m gpt-6.1-sol -c model_reasoning_effort=high -s workspace-write -C <worktree> … < /dev/null`; reviewers per §0(d) |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT APPLICABLE — no claim-bearing content beyond merge states, PR-recorded gates and the soak's deployment record, each probed with `gh`/`ps`/file reads. The soak's own verdicts come from its report.

**Questions the owner owes an answer to:** None open.
