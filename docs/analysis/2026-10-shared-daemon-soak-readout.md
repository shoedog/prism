# Shared prism daemon (S6) soak: readout

**Written:** 2026-10-20, after the Phase 3 window closed. Samples run from 2026-10-17 12:08 to
2026-10-20 12:09 MDT (the last sample); the report's `--until` (T1) is 12:20:45 MDT.
**Plan:** `docs/superpowers/plans/2026-10-12-shared-prism-daemon/S6-launcher-docs-soak.md`, "The soak".
**Evidence:** `E=~/prism-evidence/mcp-memory/s6-soak` (local only; checksums in `deploy/SHA256SUMS` and
`week/report/SHA256SUMS`). Deployed build: S6-2 head `0ed288f0` (tree-identical to #440 `9b2fa09b`),
pinned binary `prism-mcp-50f7a5abcfdf`.

How each claim was established:

- **[REPORT]**: printed by `soak.py report` or `fanout.py compare`, from the saved samples and logs.
- **[MEASURED]**: derived for this readout, read-only (samples and `calls.jsonl` analysis, `pmset -g log`, `ls`).
- **[INHERITED]**: taken from the soak's `NOTES.md` files or the lane handoff, and not re-probed.

## Verdicts

| Phase | Verdict | In one line |
|---|---|---|
| 1. Fan-out latency (excalidraw, 8 clients) | **PASS** | On one checkout with 8 clients, shared beats standalone at p95 and p99 (p50 is 1.11× slower) and uses 5.4× less memory. Percentiles cannot see the refresh stall (SI-11), which is confirmed separately. |
| 2. One bridge review round | **not run** | No bridge round used a soaked configuration. There is no T1 and no report. |
| 3. Three days of interactive use | **INADMISSIBLE** | Two gated rules passed. The other four are UNKNOWN: their evidence was not collected, so breaches there are unobserved, not absent. |

Only PASS counts (owner question 7 in the plan; D124 defines the verdicts), so the soak has not
produced what the default flip (decision B) waits for.

### Owner decisions (2026-10-20)

- **Close the soak on Phase 1.** Phase 3 is not extended. Shared mode stays deployed for opted-in
  sessions (no rollback). The leak and log rules remain unmeasured, and decision B waits for a future
  soak; the recommendation below is kept as written for the record.
- **Reviewer worktree daemons count toward usage days**, so the usage floor (2 days) is met.
- Done the same day: the Phase 2 round sampler (running since 10-17) stopped; the local
  `.codex/config.toml` restored from `deploy/snapshots/slicing-codex-config.toml` with
  `--no-skip-worktree`; the owner's checkout moved to `main`; `~/code/prism-s6-2/target` removed.

## Phase 1: fan-out latency (2026-10-18, 02:00 to 08:30 window)

- **A/A calibration: PASS** [REPORT]. Six standalone runs in ABBA order. The p95 ratio is 1.002 forward
  and 0.998 backward; the p99 ratio is 1.013 and 0.988. The bounds (1.10 and 1.25) can be reached on
  this host.
- **Gate: PASS** [REPORT]. Eight shims were compared with eight standalone servers, in three pairs, ABBA
  order, every run admissible. Median p95 was 13.29 ms shared vs 14.72 ms standalone (ratio 0.903).
  Median p99 was 13.71 vs 18.94 ms (ratio 0.724).
- **Below and beyond the gated percentiles** [MEASURED, from `calls.jsonl`]. p50 is 10.19 ms shared vs
  9.18 ms standalone (1.11×). The mean is 24.7 ms shared vs 7.6–8.3 ms standalone, because of the
  refresh stall below. With 14 overlapping calls out of 1,600, the stall stays under the 1 % that p99
  can see (plan question 7).
- **Memory after the workload** [MEASURED, from each `run.json`]. One daemon and eight shims used
  873 MiB. Eight standalone servers used 4,680 to 4,692 MiB. That 5.4× is for 8 clients of one
  checkout; it says nothing about fan-out across worktrees (see row 22).
- **Refresh stall, SI-11 / handoff row 13** [REPORT for the counts; MEASURED for the split, from
  `calls.jsonl`]. In each shared run, 14 calls of other clients overlapped the refresh. Seven of them,
  one per non-refreshing client, were blocked for about the whole rebuild: 4.05–4.09 s on excalidraw
  and 1.32–1.39 s on slicing. The other seven took 1–13 ms. Standalone servers keep answering during
  their own refresh (at most 26–39 ms over about 700 overlapping calls). This was reported and not
  gated. The owner's decision of 2026-10-18 on row 13 was conditional: "wait for Phase 2's re-index
  frequency, then fix" [INHERITED]. Phase 2 never ran and no refresh events were attributed in real use,
  so that condition is unmet; when to fix is the owner's call.
- **Slicing runs (information only)** [REPORT]. They FAIL by the same ratios (p95 1.63, p99 1.60), but
  the comparison is not like-for-like: one standalone client against eight shared clients
  (`--allow-client-mismatch`).
- **Footprint report** (diagnostic, `--phase fanout`) [REPORT]. Peak footprint of all prism processes
  was 4,997 MiB, peak swap 2,600 MiB, pressure at most 1. Its `no_new_standalone_index` FAIL comes from
  the measurement's own standalone arms.

## Phase 2: bridge round (not run)

`$E/round` has `T0` (2026-10-17 14:08 MDT) and samples, but no `T1` and no report [MEASURED].
`client-logs/bridge.log` was never created, so no agent ever started the soak's bridge wrapper.
No `stockTrading` repository appears in any week sample [MEASURED].

Bridge work did run during the window, but through configurations the soak did not edit [MEASURED]:

- Generated `.a2a-implement/impl-*` and `.a2a-dispatch/.../session` runs (8 runs between 10-17 and
  10-18) started standalone servers with `--cache-dir ~/.local/share/a2a/prism-cache-host`, plus
  plugin servers that were also standalone.
- All of them stayed lazy (about 2 MiB each, no index built).

Phase 0's notes already said that generated `implement.toml` files would not be edited [INHERITED].

The Phase 2 sampler (one sample every 5 s, running since 10-17) was stopped on 2026-10-20 (owner
decisions above).

## Phase 3: three days of use

Report: `$E/week/report/runbook/report.md`. The command was the runbook's, run with the reporting
timezone fixed at MDT (`TZ=America/Denver`; the host is now set to America/New_York):

```
soak.py report --phase week --samples $E/week/samples.jsonl --window-file $E/week/T0 \
  --until <T1> --claude-logs ~/Library/Caches/claude-cli-nodejs \
  --client-log $E/client-logs/bridge.log --client-log $E/client-logs/codex.log --min-days 2
```

`bridge.log` is the runbook's source. `codex.log` is the wrapper log that `week/NOTES.md` configured
for Codex in the slicing checkout. `T1` is saved in `$E/week/report/T1`. The crash-report directory
is the default (`~/Library/Logs/DiagnosticReports`).

| Rule | Verdict | Evidence |
|---|---|---|
| one_index_per_key (gated) | PASS | At most 1 daemon per key at any sample. 43 daemon instances were seen. |
| no_fail_closed (gated) | UNKNOWN | Two named log sources are missing (see below). Zero fail-closed lines in the logs that exist. |
| no_startup_error (gated) | UNKNOWN | Same missing sources. Zero startup errors in the logs that exist. |
| no_restart_bound (gated) | UNKNOWN | Same missing sources. Zero restart-bound refusals in the logs that exist. |
| no_abort (gated) | PASS | No `prism-mcp` crash report in the window. |
| no_footprint_growth (gated) | UNKNOWN | 17 of 43 daemons reached a baseline. None had a quiet night. |
| no_new_standalone_index | FAIL (not gated) | These are Phase 1's own standalone arms (8 on excalidraw, 1 on the c1 corpus). |
| no_critical_pressure | PASS (not gated) | Highest pressure level was 2 (warning) in 53 of 2,621 samples. |

Floors [REPORT]:

- **Window:** 259,240 s of wall time against the 259,200 s floor, so it is met by 40 s (the margin
  depends on the T1 passed). Only 156,637 s (43.5 h) were covered by intervals of 120 s or less
  [MEASURED]; the other 28.5 h had no samples. Thirteen intervals over 120 s were excluded. The
  largest gap was 11,294 s.
- **Local nights credited:** 2026-10-17 and 2026-10-18 (MDT). Running the report under the host's
  current timezone credits the same two nights.
- **Usage days:** 2026-10-18 and 2026-10-19 (UTC). That is exactly the floor of 2, and it is met only
  because of the owner's ruling on worktree daemons (below).

### Why the three log rules are UNKNOWN [REPORT + MEASURED]

The report's `coverage.logs` is `false` because `bridge.log` and `codex.log` do not exist. Neither
wrapper was invoked during the window:

- No bridge round went through a soaked configuration.
- No Codex session was started in the slicing checkout after the wrapper change. Shims for
  `~/code/slicing` instead came from `codex exec` in worktrees, which use the tracked
  `.codex/config.toml` and call the binary directly.

Control: the same report with only the Claude logs named turns all three rules PASS
(`week/report/diag-claude-only/`). That variant is diagnostic, not a verdict. Claude's logs cover
only plugin sessions: 6 `shared daemon pid` lines. Five reached 3 lazy `a2a-bridge` daemons. One
reached a home-directory daemon (pid 59912) started at 12:17 MDT on 10-20, after sampling ended.

Of the 106 shims seen in the samples, 16 were Phase 1's harness shims (pinned binary). The other 90
were interactive shims running `~/code/slicing/target/release/prism-mcp`, Codex's configured
command. Their standard error goes to no log, so a fail-closed Codex shim would have been invisible.

### Why no_footprint_growth is UNKNOWN [MEASURED]

The rule needs, for every judged daemon, a quiet hour after its baseline on each of two local nights.
No daemon got one, for three reasons.

1. **The idle TTL retired every daemon with a built index before it gave a quiet night-time hour.**
   These daemons lived 0.3–3.0 h and retired about 15 minutes after their last client. An attached
   client is the only thing that keeps one alive (`docs/MCP.md`). Three of them (pids 91776, 91833,
   70053) did overlap the start of the 10-18 night, staying until 00:21–00:46 MDT on 10-19, but they
   were working and none had a quiet hour.
2. **The sessions left open overnight were not opted in.** The plugin sessions that stayed up through
   the whole window (pids 56999 and 8068 on slicing, 60308 on a2a-bridge) were standalone the entire
   time. They were started before the profile export. One of them, 8068 (the controller's own
   session), was lazy until 10-18 15:22 MDT and then held a standalone index of about 2,030 MiB until
   the end. So the "leak nights" the handoff planned (sessions left open) had no daemon to measure.
3. **The host slept from 2026-10-19 07:04 MDT,** losing all of that day as well as the third night.
   `pmset -g log` shows sleep starting at that minute, exactly where the gaps begin. After that the
   only full wakes were 17:35–17:41 MDT on 10-19 (ended by closing the lid) and 11:40 MDT on 10-20.
   The host logged 116 dark wakes on 10-19 and 10-20. The sampler caught 11 of them, one sample each,
   and also took 7 samples in the 10-19 full wake and 30 after 11:40 on 10-20 (48 samples in all).
   No daemon existed at all after 10-19 00:46 MDT.

Per-daemon numbers for the slicing checkout's daemons with a built index (MiB) [REPORT]:

| pid | baseline | max | last | largest quiet growth / hours |
|---|---|---|---|---|
| 46393 | 1,569 | 1,919 | 1,569 | 0.0 / 0.50 |
| 16299 | 1,721 | 2,267 | 1,721 | 0.23 / 1.97 |
| 61870 | 1,477 | 1,502 | 1,502 | 24.9 / 0.92 |

Reviewer worktree daemons had baselines of 1,608–2,271 MiB and maxima up to 4,013 MiB (pid 74690,
prism-i0a, during work). The net MiB-per-hour figures include builds and work, so they say nothing
about leaks. The only quiet stretch long enough to judge (16299, 1.97 h, daytime) grew 0.23 MiB.
Shorter stretches are below the rule's one-hour bar and are not judged; they show up to 113 MiB
(pid 37514, 4 minutes) and 24.9 MiB (pid 61870, 0.92 h), probably the tail of a build.

### The usage floor is met only because worktree daemons count [MEASURED]

- UTC 10-18 is credited by slicing daemons and by reviewer worktree daemons. The slicing credit is
  thin: 7 one-minute intervals during three index builds (46393 at 01:49–01:51 MDT, before the Phase 1
  window opened; 16299 at 08:48–08:49; 61870 at 12:06–12:07).
- UTC 10-19 is credited only by reviewer worktree daemons (prism-i0a, prism-s0, prism-t3a,
  a2a-bridge-prism-skill, prism-perf-c2, prism-perf-a).

Counting only `~/code/slicing` gives 1 usage day of the 2 required. The controller's note of
2026-10-18 had said to filter worktree daemons out [INHERITED]; the owner ruled on 2026-10-20 that
they count. The floor is met only by that ruling.

### Peaks, compared with the incident (29 prism processes, 34 GiB compressed) [REPORT + MEASURED]

- Most prism processes in one sample: 12. Most index holders at once: 8 (Phase 1's standalone arm).
- Most daemons with a built index at once: 3.
- Peak footprint of all prism processes: 8,406 MiB, at 10-19 00:05. That is three worktree daemons
  (1,917–2,360 MiB each) plus the controller's standalone session (1,872 MiB at that sample).
- Peak swap used: 9,763 MiB. Peak compressor: 12,503 MiB. Both are host-wide, not only prism.

## Rows 21, 22 and 24 of the handoff

- **Row 22 (bridge fan-out across worktrees)** [REPORT + MEASURED].
  - `max_daemons_per_repo` was 1 for every repository all week, across 12 repositories with daemons,
    and `repos_with_several_daemons` is empty.
  - Not tested: the effect of dropping `--cache-dir` from bridge entries (owner question 3). No soaked
    bridge entry ran, and the Codex and plugin clients that did run never passed `--cache-dir`.
  - The generated bridge configurations still pass `--cache-dir prism-cache-host`. Once they go
    shared, they would give each checkout a second daemon.
  - Memory cost grows with the number of worktrees, not with clients per repository. Three worktree
    daemons at once (1.9–2.4 GiB each at the peak sample) set the week's peak.
  - Inference: on this 24 GB host, capping or staggering worktrees (about 3 built daemons at once) is
    the lever that bites. One shared task worktree also helps, but only for read-only reviewers.
- **Row 21 (pruning flag for dead keys)** [MEASURED]. `/tmp/prism-mcpd` holds 56 entries, 80 KiB in
  all, after three days. There is no case for a pruning flag.
- **Row 24 (the soak)**: closed on Phase 1 by the owner's decision above.

## What was not measured

- **Phase 2.** No bridge round of any kind ran through a soaked configuration.
- **Codex and bridge standard error.** No Codex or bridge shim's fail-closed or startup line could
  have been seen.
- **Long-lived daemons.** No daemon stayed resident long enough to judge it for leaks. The long-lived
  case (one daemon held across days by an attached session) is the one the flip would make common.
- **28.5 of the 72 hours,** lost mostly to host sleep from 10-19 07:04 MDT, including the third night.
- **Claude Code plugin sessions in shared mode.** Only 6 short sessions; none built an index.
- **SI-11 under real use.** No refresh events were attributed.
- **The memory-capacity goal: not established.** The lane's goal is bridge fan-out on this 24 GB host
  without swap exhaustion. No fan-out round ran under shared mode. The week's peak, with only three
  worktree daemons and one standalone session, already reached 8,406 MiB of prism footprint, 9,763 MiB
  of swap used and 12,503 MiB compressed (host-wide). Phase 1's 5.4× saving is for one checkout with
  8 clients, and it comes with a p50 1.11× slower and a mean 3× higher during a refresh-bearing run.

## Recommendation at writing (superseded by the owner decisions)

At writing, this readout recommended keeping the deployment, extending Phase 3 in place, and holding
decision B. The owner has since closed the soak on Phase 1, kept shared mode for opted-in sessions,
and done the clean-up (owner decisions above). Two parts of the original reasoning still hold as
context:

- **What actually passed.** Two gated rules passed (`one_index_per_key`, `no_abort`) over 43.5 h of
  sampled use. The three log rules and the leak rule are unmeasured, so absence of a breach there is
  not evidenced.
- **Why rollback would not have removed shared mode from Codex.** Rollback (plan decision 5) leaves
  `.codex/config.toml` on `main` passing `--shared` in every checkout, with the S6 build at
  `~/code/slicing/target/release`.

What a future soak would need, to judge the rules this one left UNKNOWN:

1. **Opted-in long-lived sessions.** Start them after the profile export. Leave at least one attached
   to a built index on two nights, with the host awake and on power (lid open, or `caffeinate -s`).
2. **Exercised log sources.** Run Codex in the instrumented checkout and one soaked bridge round (the
   runbook's `a2a-bridge run-workflow plan-review` fallback also gives Phase 2), or decide in advance
   which wrapper logs must be named (D120).
3. **A refresh-frequency measurement from a real round,** which the row-13 decision is waiting on.

For decision B, the tradeoffs at writing were:

- **For flipping:** it would bring Phase 1's per-checkout memory saving to every user, and nothing
  observed argues against it.
- **Against:**
  - The paths the flip changes most (bridge rounds, long-lived plugin sessions) are unmeasured.
  - The flip would ship the SI-11 stall to every client.
  - It is costlier to reverse: it needs a plugin version bump, and the new cache directory must have
    its binary rebuilt.

## Reproduce

Tools: `~/code/prism-s6-2/scripts/soak/` at `0ed288f0`. Outputs are in `$E/week/report/`:

- `runbook/`: the verdict.
- `diag-claude-only/`: the log-source control.
- `diag-host-tz/`: the timezone check.
- `T1` and `SHA256SUMS`.
