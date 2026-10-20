# Shared prism daemon (S6) soak: readout

**Written:** 2026-10-20, after the Phase 3 window closed (2026-10-17 12:08 to 2026-10-20 12:09 MDT).
**Plan:** `docs/superpowers/plans/2026-10-12-shared-prism-daemon/S6-launcher-docs-soak.md`, "The soak".
**Evidence:** `E=~/prism-evidence/mcp-memory/s6-soak` (local only; checksums in `deploy/SHA256SUMS` and
`week/report/SHA256SUMS`). Deployed build: S6-2 head `0ed288f0` (tree-identical to #440 `9b2fa09b`),
pinned binary `prism-mcp-50f7a5abcfdf`.

How each claim was established:

- **[REPORT]**: printed by `soak.py report` or `fanout.py compare`, from the saved samples and logs.
- **[MEASURED]**: probed for this readout, read-only (samples analysis, `pmset -g log`, `ls`).
- **[INHERITED]**: taken from the soak's `NOTES.md` files or the lane handoff, and not re-probed.

## Verdicts

| Phase | Verdict | In one line |
|---|---|---|
| 1. Fan-out latency (excalidraw, 8 clients) | **PASS** | Shared is faster than standalone at p95 and p99 and uses 5.4× less memory. The refresh stall (SI-11) is confirmed. |
| 2. One bridge review round | **not run** | No bridge round used a soaked configuration. There is no T1 and no report. |
| 3. Three days of interactive use | **INADMISSIBLE** | Four of the six gated rules are UNKNOWN, but no breach was seen. The evidence is missing, not failing. |

Only PASS counts (plan D124), so the soak has not produced what the default flip (decision B) waits for.

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
- **Memory after the workload** [MEASURED, from each `run.json`]. One daemon and eight shims used
  873 MiB. Eight standalone servers used 4,680 to 4,692 MiB.
- **Refresh stall, SI-11 / handoff row 13** [REPORT]. In shared mode, the 14 calls that overlapped a
  refresh each waited for the whole rebuild: up to 4,073–4,090 ms on excalidraw and 1,350–1,388 ms on
  slicing. Standalone servers keep answering during their own refresh (at most 26–39 ms over about
  700 overlapping calls). This was reported and not gated. The owner decided on 2026-10-18 to fix it by
  building off-lock [INHERITED].
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

The Phase 2 sampler (pid 79694, one sample every 5 s) is **still running**: about 69 MB of samples
since 10-17 [MEASURED]. It was left running for the controller.

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
| no_startup_error (gated) | UNKNOWN | Same missing sources. Zero startup errors. |
| no_restart_bound (gated) | UNKNOWN | Same missing sources. Zero restart-bound refusals. |
| no_abort (gated) | PASS | No `prism-mcp` crash report in the window. |
| no_footprint_growth (gated) | UNKNOWN | 17 of 43 daemons reached a baseline. None had a quiet night. |
| no_new_standalone_index | FAIL (not gated) | These are Phase 1's own standalone arms (8 on excalidraw, 1 on the c1 corpus). |
| no_critical_pressure | PASS (not gated) | Highest pressure level was 2 (warning) in 53 of 2,621 samples. |

Floors [REPORT]:

- **Window:** 259,240 s, which meets 72 h. However, the samples only cover 156,636 s (43.5 h) at
  intervals of 120 s or less [MEASURED]. Thirteen intervals over 120 s were excluded. The largest gap
  was 11,294 s.
- **Local nights credited:** 2026-10-17 and 2026-10-18 (MDT). Running the report under the host's
  current timezone credits the same two nights.
- **Usage days:** 2026-10-18 and 2026-10-19 (UTC). That is exactly the floor of 2.

### Why the three log rules are UNKNOWN [REPORT + MEASURED]

The report's `coverage.logs` is `false` because `bridge.log` and `codex.log` do not exist. Neither
wrapper was invoked during the window:

- No bridge round went through a soaked configuration.
- No Codex session was started in the slicing checkout after the wrapper change (shims for
  `~/code/slicing` instead came from `codex exec` in worktrees, which use the tracked
  `.codex/config.toml` and call the binary directly).

Control: the same report with only the Claude logs named turns all three rules PASS
(`week/report/diag-claude-only/`). That variant is diagnostic, not a verdict. Claude's logs cover
only plugin sessions: 6 `shared daemon pid` lines from 2 projects, reaching 4 lazy `a2a-bridge`
daemons. The 106 shims seen in the samples all ran `~/code/slicing/target/release/prism-mcp` or the
pinned binary, and their standard error goes to no log. A fail-closed Codex shim would have been
invisible.

### Why no_footprint_growth is UNKNOWN [MEASURED]

The rule needs, for every judged daemon, a quiet hour after its baseline on each of two local nights.
No daemon came close, for three reasons.

1. **The idle TTL retired every daemon long before a night.** Each daemon with a built index lived
   0.3–3.0 h and retired about 15 minutes after its last client. An attached client is the only thing
   that keeps one alive (`docs/MCP.md`). The longest quiet stretch of any daemon was 1.97 h (pid 16299,
   slicing), in the daytime, with 0.23 MiB of growth.
2. **The sessions left open overnight were not opted in.** The plugin sessions that stayed up through
   the whole window (pids 56999 and 8068 on slicing, 60308 on a2a-bridge) were standalone the entire
   time. They were started before the profile export. One of them, 8068 (this controller's own
   session), held a 2,030 MiB standalone index for all three days. So the "leak nights" the handoff
   planned (sessions left open) had no daemon to measure.
3. **The host slept through the third night.** `pmset -g log` shows sleep starting at 2026-10-19
   07:04 MDT, exactly where the gaps begin. After that the only full wakes were 17:35–17:41 MDT on
   10-19 (ended by closing the lid) and 11:40 MDT on 10-20. The sampler only ran during the 116 dark
   wakes. No daemon existed at all after 10-19 00:46 MDT.

Per-daemon numbers for the slicing checkout's daemons with a built index (MiB) [REPORT]:

| pid | baseline | max | last | largest quiet growth / hours |
|---|---|---|---|---|
| 46393 | 1,569 | 1,919 | 1,569 | 0.0 / 0.50 |
| 16299 | 1,721 | 2,267 | 1,721 | 0.23 / 1.97 |
| 61870 | 1,477 | 1,502 | 1,502 | 24.9 / 0.92 |

Reviewer worktree daemons had baselines of 1,608–2,271 MiB and maxima up to 4,013 MiB (pid 74690,
prism-i0a, during work). The net MiB-per-hour figures include builds and work, so they say nothing
about leaks. None of these rows judges a leak; they show only that no daemon grew during the quiet
stretches that were observed.

### The usage floor rests on daemons the soak said to filter out [MEASURED]

- UTC 10-18 is credited by slicing daemons (46393, 16299, 61870), plus reviewer worktree daemons.
- UTC 10-19 is credited only by reviewer worktree daemons (prism-i0a, prism-s0, prism-t3a,
  a2a-bridge-prism-skill, prism-perf-c2, prism-perf-a).

The controller's note of 2026-10-18 says these worktree daemons are "NOT soak subjects; filter by
repo/key" [INHERITED]. The tool has no such filter. Counting only `~/code/slicing`, there is **one**
usage day. They are real shared-mode use of the deployed build, so whether they count is the owner's
call. Either way, the floor is met by at most a one-day margin.

### Peaks, compared with the incident (29 prism processes, 34 GiB compressed) [REPORT + MEASURED]

- Most prism processes in one sample: 12. Most index holders at once: 8 (Phase 1's standalone arm).
- Most daemons with a built index at once: 3.
- Peak footprint of all prism processes: 8,406 MiB, at 10-19 00:05. That is three worktree daemons
  plus the 2 GiB standalone controller session.
- Peak swap: 9,763 MiB. Peak compressor: 12,503 MiB (host-wide, not only prism).

## Rows 21, 22 and 24 of the handoff

- **Row 22 (bridge fan-out across worktrees)** [REPORT + MEASURED].
  - `max_daemons_per_repo` was 1 for every repository all week, across 12 repositories with daemons,
    and `repos_with_several_daemons` is empty. Dropping `--cache-dir` (owner question 3) produced no
    split, but the soaked bridge entries were never used, so that is shown only for Codex and plugin
    clients.
  - The generated bridge configurations still pass `--cache-dir prism-cache-host`. Once they go
    shared, they would give each checkout a second daemon.
  - Memory cost grows with the number of worktrees, not with clients per repository. Three worktree
    daemons at once (1.6–2.9 GiB each) set the week's peak.
  - Inference: on this 24 GB host, capping or staggering worktrees (about 3 built daemons at once) is
    the lever that bites. One shared task worktree also helps, but only for read-only reviewers.
- **Row 21 (pruning flag for dead keys)** [MEASURED]. `/tmp/prism-mcpd` holds 56 entries, 80 KiB in
  all, after three days. There is no case for a pruning flag.
- **Row 24 (the soak)**: this readout. The next actions are the owner's decisions below.

## What was not measured

- **Phase 2.** No bridge round of any kind ran through a soaked configuration.
- **Codex and bridge standard error.** No Codex or bridge shim's fail-closed or startup line could
  have been seen.
- **Long-lived daemons.** No daemon stayed resident long enough to judge it for leaks. The long-lived
  case (one daemon held across days by an attached session) is the one the flip would make common.
- **The third night,** lost to host sleep. About 29 of the 72 hours were unobserved.
- **Claude Code plugin sessions in shared mode.** Only 6 short sessions; none built an index.
- **SI-11 under real use.** No refresh events were attributed.

## Recommendation

**Keep the deployment and extend Phase 3 in place.** Do not roll back now. Reasons:

- No rule saw a breach in three days of real use: no fail-closed line, no crash, no restart-bound
  refusal, never two daemons for one key. Phase 1 passed with a 5.4× memory saving.
- Rollback (plan decision 5) does not remove shared mode from Codex. `.codex/config.toml` on `main`
  passes `--shared` in every checkout, and the S6 build stays at `~/code/slicing/target/release`.
  Rolling back would therefore turn off the plugin and bridge paths, which are the ones still
  unmeasured, while leaving the path that carried most of the week's use.

Tradeoff: keeping means opted-in clients wait 1.3–4.1 s whenever one of their calls overlaps a
refresh (row 13) until the off-lock fix lands. If the owner chooses rollback, the order is plan
decision 5: configurations first, then sessions, then daemons, then the plugin binary and launcher,
then the probe. Either way, restore the local `.codex/config.toml` from
`deploy/snapshots/slicing-codex-config.toml` and run `git update-index --no-skip-worktree` once
the soak ends.

To turn INADMISSIBLE into a verdict, resume the week sampler with a longer `--duration` against the
same `T0` (D132; never rewrite `T0`), and:

1. **Opt in the long-lived sessions.** Restart the Claude Code sessions that predate the export. Leave
   at least one opted-in session attached to a built slicing index on two nights, with the host awake
   and on power (lid open, or `caffeinate -s`).
2. **Exercise the named log sources, or settle them.** Either start a Codex session in the slicing
   checkout and run one soaked bridge round (the runbook's `a2a-bridge run-workflow plan-review`
   fallback also completes Phase 2), or have the owner rule which wrapper logs Phase 3 must name.
   Dropping an unused source is an owner decision under D120; this readout does not drop one.
3. **Decide the worktree question.** The owner rules whether reviewer worktree daemons count toward
   usage days.

**Decision B (default flip): not yet.** The flip's gate (spec §8 step 2) is the three phase reports,
and only Phase 1 has passed.

- **For flipping now:** it would bring the 5.4× fan-out memory saving to every user, and nothing seen
  argues against it.
- **Against:**
  - The paths the flip changes most (bridge rounds, long-lived plugin sessions) are the ones not
    measured.
  - A flip ships SI-11's stall to everyone before the off-lock fix.
  - A flip is costlier to reverse, because it needs a plugin version bump and the new cache directory
    must have its binary rebuilt.

Recommended order: the SI-11 off-lock fix, then the extended Phase 3 and a Phase 2 round, then B.

## Reproduce

Tools: `~/code/prism-s6-2/scripts/soak/` at `0ed288f0`. Outputs are in `$E/week/report/`:

- `runbook/`: the verdict.
- `diag-claude-only/`: the log-source control.
- `diag-host-tz/`: the timezone check.
- `T1` and `SHA256SUMS`.
