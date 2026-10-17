# S6 — Opt-in Launchers, Shared-Mode Docs and Soak Instrumentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-16; review round 1 folded the same day).** Every task of this plan was built before the plan was written, on the local scratch branch `scratch/s6-validate` (worktree `~/code/prism-s6-validate`) from `main` `61c2a9e5`, merged up to `main` `4a9e6476` (lane docs only in between), test first; review round 1's fold (Opus 5.5 FIX, gpt-6.1-sol FIX: the launcher and docs correct, the soak specification and its instruments not) was folded the same way, its tests committed red before the code (see "Review round 1" at the end). **Review round 2**, the cap (gpt-6.1-sol FIX with one finding, Opus 5.5 FIX with five WRONG, converging), was folded the same way after the cap without a third round, disclosed (see "Review round 2"). Every mutant named in a task was applied there, each alone: 68 of 68 were killed. The source was then cut into this plan's tasks on `scratch/s6-tasks`, the plan's directives were generated from that branch's per-task commits, and the plan was replayed by a script onto `4a9e6476` (`scratch/s6-replay`), each anchor matching once, to a tree byte-identical with `scratch/s6-tasks` and with the validated code, executable modes included; each task's red and green steps below were run on partial replays. The launcher's tests and the soak tools' tests (the live fan-out run against a container-built binary included) were run on Linux (aarch64, the bridge's `a2a-toolchain` image, as root), 10 times each (V11). The mutation gate was not run (it selects no MCP code and no Python; lane README). **The soak itself is not code and was not run**: it is the lane's acceptance, specified under "The soak" for the controller, after the owner has answered the questions below.

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file that was then read.

| # | Command | Must show | Observed |
|---|---|---|---|
| V1 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` on the untouched base | The base total; later totals are "base + N". | `52 passed`, the goldens' test `ok`, at `61c2a9e5` (`4a9e6476` differs from it only in two lane documents). |
| V2 | Each task's red step, on a partial replay of this plan | What the step says. | Task 1: `test result: FAILED. 3 passed; 4 failed`, the four named. Task 2: `No module named 'soak'` three times, `FAILED (errors=3)`. Task 3: `No module named 'fanout'` three times, `FAILED (errors=3)`. Each review round's tests were also committed red against the unfolded code (round 1: `Ran 59 tests`, `FAILED (failures=4, errors=35)`; round 2: `Ran 66 tests`, `FAILED (failures=3, errors=7)`). |
| V3 | Each task's green step, on the replay | The count the step states, no warning. | Task 1: `7 passed` (52 filtered out), no `warning`. Task 2: `Ran 38 tests`, `OK`. Task 3: `Ran 66 tests`, `OK (skipped=1)`; with `PRISM_SOAK_BIN`, `Ran 66 tests`, `OK`. |
| V4 | `cargo test --features mcp` (the whole suite) and `cargo test --doc --features mcp` | `0 failed`. | At `f76f8e33` (review round 1's head): 31 `test result` lines, **6,233 passed, 0 failed, 1 ignored**; doc tests `2 passed`. Review round 2 changed no file cargo builds or tests (`git diff --stat f76f8e33 cbc32ee3 -- src tests Cargo.toml Cargo.lock build.rs` is empty: only `scripts/soak/` and `CLAUDE.md`), so the total stands for the head. The base's total by the same count is 6,233 − 7 = 6,226. |
| V5 | The soak tools' tests, with and without `PRISM_SOAK_BIN`, and memreport's | All `OK`. | `Ran 66 tests`, `OK (skipped=1)`; with the debug binary `Ran 66 tests`, `OK`; `scripts/memreport` (which `fanout.py` imports) `Ran 349 tests`, `OK` (round 0; memreport untouched since). |
| V6 | `cargo check`, `cargo check --features mcp`, `cargo fmt --check` | `Finished`; no warning; fmt silent. | Both `Finished`, 0 warnings; fmt silent at the head and at every task commit of the cut. |
| V7 | `cargo clippy --all-targets --features mcp -- -W clippy::all`, filtered to the touched files | No line in a touched file. | 313 `warning` lines at `f76f8e33` (no Rust file changed since), none naming `tests/mcp/launcher_test.rs` or `tests/mcp/main.rs` (the only Rust files touched). |
| V8 | Every "prove the tests bite" step | The named tests fail under the named mutant. | L1–L6, S1–S12, S7b, U1–U9, U11–U17, C1–C4, W1, R1–R4, G1, F1–F22, A1: **68 of 68 killed**, each alone, the full set at `2e948c5c` and the three that depend on the last change (U9, U11, R3) again at the head. Four first survived review round 1's fold (U11, F9, F15, F16): each got a test that fails under it (Validation record, corrections 7–10). In round 2, U10 survived because its check was implied by another; the check was removed (correction 12). S7 and S7b are killed in every zone (the test sets its own). |
| V9 | Repeats on macOS | All `ok`; no daemon left. | At the head (`cbc32ee3`): `10 test result: ok. 7 passed` (`--test mcp launcher`) and `10 Ran 66 tests` / `10 OK` (the soak tools with the debug binary, the live test included); `pgrep -fl 'prism-mcp daemon'` empty afterwards. |
| V10 | The zero-diff and replay checks | Nothing in `src/`, `.claude-plugin/` or the goldens; the replay byte-identical. | `git diff --stat 4a9e6476 cbc32ee3 -- src .claude-plugin tests/fixtures/mcp/golden` is empty. The replay `scratch/s6-replay` (`0daffe4f`, 25 directives, every anchor matching once) has tree `4b940d6e`, the tree of `scratch/s6-tasks` (`678fd702`) and of the validated head (`cbc32ee3`), executable modes included. |
| V11 | Linux: the bridge's `a2a-toolchain` image (Debian, aarch64, rustc 1.94, Python 3.12, uid 0, no `ps`/`pgrep`), Task 4 Step 4's script, at the head (`cbc32ee3`); the disk checked (118 GiB free) in a command of its own first | All `ok`. | `failed Linux runs: 0`: `10 test result: ok. 7 passed` (`--test mcp launcher`, the process test included), `10 Ran 66 tests` / `10 OK` (the soak tools, the live fan-out test against the container-built binary included); no `left:` line; volumes removed by the `trap`. x86_64 Linux: CI only. |

## Owner decisions (2026-10-16)

**Decided by the owner on 2026-10-16: "accept all".** Every question below is answered with its recommended answer, as revised in review rounds 1 and 2 (questions 4, 5, 7 and 8). **Later the same day the owner revised question 8's usage-day rule a third time** (escalated by the controller after PR #424's review round 1, Opus W1): a day counts when some single interval of at most 120 s shows at least 0.05 s of one daemon's CPU growth, with CPU readings for that pid at both consecutive endpoints; a pid or CPU reading missing at either end contributes nothing, and intervals longer than 120 s earn no usage credit and are counted in the report (D128); question 8 below carries the new text. **Also decided by the owner on 2026-10-16, after PR #424 review round 2:** cap usage-credit intervals at 120 s (twice the default 60 s cadence), report the excluded count, and retain the requirement for observed CPU readings at both consecutive endpoints. This fold proceeds after the two-round review cap without a third round, disclosed below. The text is otherwise kept as asked, so each decision's reasons stay with it.

Each is a behavior or a value the spec leaves to the owner, or an action outside this repository that needs the owner's approval. The plan is built on the recommended answer; a different answer changes the named place only.

1. **How a Claude Code plugin user opts in (D110).** The plugin's MCP arguments live in its installed manifest (`.claude-plugin/plugin.json`, copied into `~/.claude/plugins/cache/…`), which a user does not edit. **Recommended: (A) an environment variable, `PRISM_MCP_SHARED=1`, read by the launcher** (built and tested here): opt-in per user and per shell, nothing changes for anyone else, and the default flip later is one line in the launcher. (B) `--shared` in `plugin.json` for every plugin user: makes shared mode the default for the plugin before the soak, against spec §8 step 1 and §11 item 2. (C) Documentation only: a plugin user cannot opt in without editing the cache.
2. **The runtime directory on this host (SI-42, D115).** Every launcher that should share a daemon must resolve one directory. Claude Code passes `$TMPDIR` (measured); whether a host-run Codex does was not measured. **Recommended: set `PRISM_MCP_RUNTIME_DIR=/tmp` explicitly everywhere** — in `.codex/config.toml` (Task 4), in the bridge's `[[agents.mcp.env]]`, and in the shell profile Claude Code and the operator start from — giving `/tmp/prism-mcpd`: one literal for every configuration file, short (37-byte socket paths), the spec's own Linux fallback, cleared at reboot. Cost: `/tmp/prism-mcpd` is one name per host, so on a host with a second user that user must set another value (this Mac has one). Alternatives: (B) leave it unset and rely on `$TMPDIR` reaching every launcher (a split costs a second index, silently, until the soak's `one_index_per_key` rule shows it); (C) a directory under the home, for example `~/.local/state/prism-mcp` (also valid; each configuration file then carries the home path).
3. **One cache directory for the bridge and for interactive sessions.** The host-run bridge configurations pass `--cache-dir ~/.local/share/a2a/prism-cache-host`; interactive sessions and the project's `.codex/config.toml` use the default cache. The cache base is in the identity key (spec §4.3), so the same checkout gets two daemons and two indexes during a bridge round. **Recommended: drop `--cache-dir` from the host-run bridge entries for the soak**, so bridge agents and interactive sessions share one daemon per checkout (the default cache of `~/code/slicing` is already warm from interactive use). Alternative: keep it (two daemons per checkout; the soak report shows them as `max_daemons_per_repo` 2, information, not a failure). Containerized configurations are not affected (their agents have daemons of their own inside their containers).
4. **When `.codex/config.toml` changes** (revised after review round 1, Opus W5, sol 1). A checkout with the new configuration runs `prism-mcp --shared` from `~/code/slicing/target/release/prism-mcp`; the binary deployed there today (`9d5a67bc`, before S5a) rejects `--shared`, and Codex then has no prism server in that checkout. **Recommended: keep the change in S6-2, build the soak's binary from S6-2's head (it contains S6-1; its `src/` is `main`'s, so the binary is the merge's in all but its commit; if `main` has gained `src/` changes, merge `main` into the branch first), install it in Phase 0, check every launcher with `soak.py probe`, and only then merge S6-2.** S6-1 may merge before Phase 0 (its launcher change is opt-in). The earlier text said "a binary of this slice's merge", which cannot exist before the merge it gates. Alternative: drop the file from Task 4 and apply it at Phase 0 (rollback then reverts it with the bridge's configurations).
5. **The soak's deployment, outside this repository** (Phase 0 below, revised after review round 1, sol 1, Opus W4, W6, S2): copy every file it replaces into `$E/deploy/originals/` with its sha256 first, then replace the two deployed binaries (`~/code/slicing/target/release/prism-mcp`, `~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release/prism-mcp`) and the plugin cache's launcher, each step guarded so that a second run never overwrites a kept original; point the bridge's entries (and, for the soak, nothing else) at the pinned `$E/bin/prism-mcp`; snapshot the bridge's and Codex's configurations before editing them; export the two variables in the shell profile. **Rollback** (revised after review round 2, Opus W-C, S-B) returns interactive sessions to standalone without ever leaving a configuration that passes `--shared` with a binary that rejects it: configurations first (the bridge's from their snapshots; the profile's variables unset), opted-in sessions ended or restarted, the daemons stopped with the S6 binary (`kill -TERM` for one that still has clients), then only the plugin cache's binary and launcher restored (copy and rename). **`~/code/slicing/target/release/prism-mcp` keeps the S6 build**: once S6-2 is on `main`, its `.codex/config.toml` passes `--shared` from every checkout, and the S6 build serves standalone clients exactly as before, so restoring the pre-S5a build there would break Codex in every later checkout. The alternative, a revert pull request of the `.codex` change as part of rollback, was not chosen: it would reach checkouts only as they pull. **Recommended: yes, as listed.** No step of this plan does any of it; the controller does it after this approval.
6. **The plugin's version.** **Recommended: no bump in S6** (the opt-in is additive and this host's plugin is updated by Phase 0's file replacement); bump at the default flip, noting that an updated plugin's new cache directory has no built binary, so the launcher falls back to `PATH` (and fails with its "build it" message when there is none).
7. **The soak's pass thresholds** (revised after review round 1, Opus W1, S1, sol 3, sol 4). **Recommended:** fan-out: first an A/A calibration (three standalone runs against three standalone runs on excalidraw, in ABBA order, compared in both directions, `compare --both-directions`; review round 2, Opus W-A): if either direction's ratio exceeds 1.10 (p95) or 1.25 (p99), the bound is unreachable on this host and goes to the owner before any gate run; then the gate, eight shims against eight standalone servers, three pairs in ABBA order, every run admissible (exactly one successful `refresh_index`, no critical or unreadable pressure reading, every call answered, one daemon), shared median p95 ≤ 1.10× and median p99 ≤ 1.25× the standalone medians. The refresh's stall (the calls of other clients overlapping the refresh, and the longest of them) is reported for both arms and **not gated**: with one refresh in 1,600 calls no percentile can see it (Opus W1), so S3b's SI-11 is reported here, not measured against a bound. Option: gate it too, for example the shared arm's longest overlapping call at most twice the standalone arm's `refresh_ms`. Bridge round: every rule `PASS` (none `UNKNOWN`) and use seen (shims, a daemon reached, an index built); week: the five gated rules `PASS` and the usage floor (question 8). Alternatives: a looser bound if the calibration shows the host's own spread needs it; a footprint ceiling per key in place of `no_critical_pressure` for the round.
8. **The week's scope and floor (SI-41; revised after review round 1, Opus S3).** **Recommended:** Claude Code sessions through the plugin, bridge rounds, and Codex sessions in the slicing checkout count; a day counts when some single interval of at most 120 s shows at least 0.05 s of one daemon's CPU growth, with CPU readings for that pid at both consecutive endpoints; a pid or CPU reading missing at either end contributes nothing, and intervals longer than 120 s earn no usage credit and are counted in the report (**owner decisions 2026-10-16**: the per-interval threshold and both-endpoint requirement, then the 120 s cap after PR #424 review round 2, revising the answer accepted earlier that day, "at least 0.5 s of CPU that day": PR #424's review round 1, Opus W1, measured an idle daemon with a session attached at about 0.86 s a day, which met the daily sum every day; review round 2, Opus W-B: a resident daemon or an attached idle session is not use; a light day of a few very quick calls may not count, which extends the week rather than passing it); the week counts when at least 5 of its 7 days do; fail-closed lines and startup errors are counted from Claude Code's logs and the wrapper's logs, and the project's Codex sessions are pointed at the wrapper for the week (a local, uncommitted change to that checkout's `.codex/config.toml`, snapshotted and restored at the end) or are left uncounted, disclosed. A week below the floor is extended, not passed.
9. **The latency baseline for larger repositories (SI-39).** Eight standalone servers of slicing need about 8 × 2 GiB on this 24 GB host (C1 readout), which is the incident itself. **Recommended: gate on excalidraw (8 × about 0.7 GiB) and report slicing as information**: eight shims against one standalone client running the same workload, three runs each.
10. **The bridge's reaper** (`daemon-shutdown --pid --start-time`, S5b). **Recommended: not built for the soak**: after a round the idle TTL (15 min) frees the index; build it in the bridge only if the round's report shows the residency after rounds matters.

---

**Goal:** the lane's: let the a2a-bridge run its reviewer and implementer fan-out on this host without swap exhaustion, by holding one resident index per compatible (checkout, prism build, cache configuration, refresh policy), with no query-latency regression under fan-out. **What S6 must make true, and test:** a user can opt in to `--shared` from every launcher the spec names (spec §8 step 1: the project's `.codex/config.toml`, the Claude Code plugin's launcher, the bridge's `[[agents.mcp]]` entry, with `PRISM_MCP_RUNTIME_DIR`), without editing the installed plugin and without changing what anyone gets who does not opt in; a user can find in `docs/MCP.md` everything the earlier slices left to S6 (what changes for a client, every fail-closed reason and what to do, the operator's commands, the runtime directory rule); and the controller has the instruments the §8 soak needs (resident index count and aggregate footprint over time, fail-closed lines, daemon restarts and aborts, fan-out latency against standalone), with the soak itself specified precisely enough to run and to judge. Nothing in `src/` changes: the in-process server, the shim, the daemon and the operator's commands are as S5b left them, and `--shared` stays opt-in.

**Architecture:** Three independent pieces. (1) **The plugin launcher** (`scripts/prism-mcp-launch.sh`) reads `PRISM_MCP_SHARED`: `1` inserts `--shared` after `--repo`, unset/empty/`0` changes nothing, anything else is refused before the binary runs, and an explicit `--shared`/`--standalone` argument wins (D110–D113). Claude Code hands plugin MCP servers its whole environment (measured, SI-42), so an `export` in the user's profile reaches the launcher; `plugin.json` is unchanged. The other launchers take `"--shared"` in their arguments (`.codex/config.toml` in this repo; the bridge's configuration, documented, outside the repo). (2) **Docs:** a "Shared daemon" section of `docs/MCP.md`, a bullet of the navigation skill, a paragraph of `CLAUDE.md`. (3) **Soak instrumentation** in `scripts/soak/` (Python 3.9, standard library): `soak.py sample` appends, per interval, every `prism-mcp` process classified from its command line (daemon, shim, standalone, operator), its `phys_footprint` and the host's swap, memory pressure and compressor; `soak.py report` judges the soak's rules from those samples, the shims' standard-error lines (Claude Code's MCP logs, and `prism-mcp-logged.sh` for clients that keep none) and macOS crash reports; `fanout.py run` drives N real clients of one corpus in shared or standalone mode through the C1 pinned workloads with one concurrent `refresh_index`, timing every call, and `fanout.py compare` judges shared against standalone under a pinned rule. The sampler only reads the process table; it never connects to a daemon unless asked to store `daemon-status` text (D121).

**Tech Stack:** bash (3.2-compatible: macOS's `/bin/bash`), Rust 2021 for the launcher's tests (`std::process`, `tempfile`, the S4/S5a process-test helpers), Python 3.9 standard library for `scripts/soak/` (it imports `scripts/memreport/` for workload binding and host state). No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§4.3 runtime directory, §4.6, §4.7, §5, §6.2 fan-out gate, §8 steps 1–4, §9 "Soak: §8 step 2", §10, §11 items 1–3, §12 row S6). Lane plan: `README.md` in this directory (the S6 note on the launcher; the S2a/S2b/S4 notes on docs). Built on S5b as merged: `main` `61c2a9e5`. Decisions are numbered on from the S5b plan's (D97–D109) and spec issues from its SI-29–SI-35.

**Interfaces this plan settles** (from the "S6 owes" lists of S3a, S3b, S4, S5a and S5b, each checked under "Self-review record"): the readiness vocabulary, what a client sees on a crash or a drain, the daemon's command line, exit statuses and log lines, the runtime-directory rule, the fail-closed line and every reason, D85's silently ignored flags, D77's minute and its clock, SI-27's rebuilt binary, the `sudo -E` trap, the operator's commands and exit statuses, SI-35's old daemons and `kill -TERM` (all in `docs/MCP.md`); `--shared` in `.codex/config.toml` and the plugin launcher (Tasks 1, 4); the bridge's entry and `PRISM_MCP_RUNTIME_DIR` (documented; applied at the soak with the owner's approval, SI-37); the soak's instruments (Tasks 2, 3) and the soak (specified, "The soak").

**Ships as two pull requests** (D114): 20 paths in all, 2,714 insertions and 1 deletion; nothing in `src/`. **S6-1 = Tasks 1–2:** production 760 lines (`scripts/soak/soak.py` 359, `scripts/soak/soak_judge.py` 352, `scripts/prism-mcp-launch.sh` +23, `scripts/soak/prism-mcp-logged.sh` 21, `.github/workflows/ci.yml` +5), tests 823 (`scripts/soak/test_soak.py` 228, `tests/mcp/launcher_test.rs` 215, `test_soak_coverage.py` 195, `test_soak_round2.py` 106, `fake_prism_mcp.py` 72, the stand-in 6, one `mod` line). **S6-2 = Tasks 3–4:** production 502 (`scripts/soak/fanout.py` 500, `.codex/config.toml` +2/−1), tests 428 (`test_fanout.py` 214, `test_fanout_gate.py` 133, `test_fanout_live.py` 81), docs 201 (`docs/MCP.md` +180, `CLAUDE.md` +17, the skill +4). Split after review round 1 took production from 833 to 1,157 lines, past the lane's threshold of about 1,000 (D114); after review round 2's fold, 1,262.

## Global Constraints

- **Nothing under `src/` changes**, and nothing in `.claude-plugin/`. Standalone behavior, the shim, the daemon and the operator's commands are unchanged; the golden transcripts under `tests/fixtures/mcp/golden/` are not touched (never set `PRISM_MCP_BLESS_GOLDEN`). Task 4 checks it.
- **Opt-in stays opt-in.** No launcher in the repository adds `--shared` for a user who has not asked for it: the plugin's launcher only on `PRISM_MCP_SHARED=1`; `.codex/config.toml` is this repository's own configuration (owner question 4).
- **The launcher runs under bash 3.2** (macOS's `/bin/bash`): no arrays expanded under `set -u`, no `${var,,}`; the tests run it with `/bin/bash`. A value of `PRISM_MCP_SHARED` the launcher does not accept stops it with exit 2 and one line naming the variable, before anything else runs.
- **The soak tools are Python 3.9, standard library only** (this host's `python3` is 3.9.6; the image's and CI's are 3.12), and never connect to a daemon unless `--status-bin` is given (`sample`) or the run is a shared fan-out arm, whose daemon is its own. They are measurement instruments run on the controller's request; **CI runs their hermetic tests** (a fake server, temporary directories; D117). The live test runs only with `PRISM_SOAK_BIN`.
- **One measurement process at a time** (lane rule): `fanout.py` and `soak.py sample` record the host's state with every run so that an overlap is visible; the soak's runbook (below) never overlaps two measured runs.
- **Tests do not depend on the host's speed.** No test asserts an upper bound on a duration; the process tests wait for a condition up to their hang limit (60 s); the one subprocess test of the fan-out driver bounds only a hang (60 s).
- **No test writes a file it then executes**: the launcher's stand-in is a committed executable fixture reached through a symbolic link (on Linux a freshly written executable can fail with `ETXTBSY` while another test thread forks); the fake server and the wrapper's stand-in are a committed script and `sys.executable`.
- **Every process a test starts is ended on every path** (`Launched`'s and S5a's `Daemons`' `Drop`; the fan-out run's own `daemon-shutdown` and its 60 s `SIGTERM` fallback; the client watchdog and the pressure observer's stop; the run's barrier bounded by its timeout; `probe`'s kill of a silent server). After every run of the suite: `pgrep -fl 'prism-mcp daemon'` prints nothing.
- File size (CLAUDE.md decision 7): every file this slice creates is under 600 weighted lines (`scripts/file_size.py`, which weighs Rust; the Python files are counted by plain lines: `soak.py` 359, `soak_judge.py` 352, `fanout.py` 500, `test_soak.py` 228, `test_soak_coverage.py` 195, `test_soak_round2.py` 106, `test_fanout.py` 214, `test_fanout_gate.py` 133, `test_fanout_live.py` 81, `fake_prism_mcp.py` 72; `tests/mcp/launcher_test.rs` weighs 191.5).
- No new clippy warning in a file this slice adds or edits (one Rust file, `tests/mcp/launcher_test.rs`, and one line of `tests/mcp/main.rs`).
- **Commit before any experiment.** The "prove the tests bite" steps edit a file and restore it with `git checkout -- <file>`; they come after the task's commit.
- **Run the launcher's tests and the soak tools' tests on Linux before the pull request** (Task 4), in the bridge's `a2a-toolchain` image with the worktree mounted read-only and named volumes for the target and the registry, removed by a `trap`.
- Logs go to files in a directory of the implementer's own, `$LOG_DIR` below; read the file. Do not pipe cargo output into `grep` or `tail`; `grep` the saved logs afterwards. Every cargo command runs alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, written out in full (in zsh an unquoted `$J cargo` is one word, S5b correction 3). **Before any large build or Docker run, check `df -h /System/Volumes/Data` in a separate command and keep at least 50 GiB free.**

### What each mechanism covers, and what it does not

- **The launcher's opt-in reaches only the plugin's server.** Claude Code passes its whole environment to plugin MCP servers (SI-42), so `PRISM_MCP_SHARED=1` in the profile Claude Code starts from opts every new session in; a session started before the export keeps its standalone server until it restarts. Codex, Kiro, a manual `claude mcp add` and the bridge do not run this launcher: they take `"--shared"` in their arguments.
- **A runtime directory that differs between two launchers is not an error**: it gives a second daemon, a second index, and no failure anywhere. The docs tell users to set `PRISM_MCP_RUNTIME_DIR` to one value; the soak's `one_index_per_key` rule catches it after the fact (two daemons of one key at once can only be in two runtime directories, since one directory's owner lock admits one).
- **Different cache directories, refresh policies or builds give different daemons by design** (spec §4.3); the soak report lists repositories with more than one daemon at once (`max_daemons_per_repo`), as information for owner question 3, never as a failure.
- **An index holder is recognized, not measured exactly**: every daemon, and a standalone server whose footprint is at least 100 MiB (one that has built nothing is a few MiB; D118). A standalone server in the middle of its first load is counted from the sample in which it passes 100 MiB.
- **A command line is split on spaces.** A repository path with a space in it is cut short in the report's labels; roles do not depend on paths. A binary path with a space in it is recognized when the prefix ending in `/prism-mcp` is an executable file (review round 1, Opus S5).
- **What was not observed is UNKNOWN, never PASS** (D124): a sample without a process table, a live standalone server without a footprint, a sample without a pressure reading, a named log source or the crash-report directory missing. A process that ended between the listing and its footprint is `gone`, not a failure. A rule still FAILs on a breach it did see.
- **The sampler is blind between samples, and to other machines.** A daemon that lives and dies inside one interval (an abort at its first build) is missing from the samples; it is counted by its crash report and by the shims' lines instead. Agents in containers run in the OrbStack VM, outside the host's process table: the bridge round must use a host-run configuration, and its report is INADMISSIBLE without shims, a reached daemon and a built index (D125).
- **Fail-closed lines and startup errors are counted only where they are kept**: Claude Code's MCP logs (measured to hold the server's standard error) and the files `prism-mcp-logged.sh` writes. A Codex session launched by the project's `.codex/config.toml` keeps none unless the owner points it at the wrapper for the soak (owner question 8). A named source that is missing makes the rules UNKNOWN; a source that exists but that a client never wrote to cannot be told from a quiet one.
- **Standalone servers that predate a phase are listed, not counted** (D125): one first seen after the phase's first sample is counted. A server restarted with a new pid during the phase counts as new.
- **Crash reports name the process**: `prism-mcp-<date>.ips`. Test binaries (`prism-<hash>`) are not counted. A crash report outside the window or deleted before the report is missed.
- **`fanout.py` times a call from its write to its response at the client**, one call outstanding per client, as an agent makes them. It measures the whole path (pipe, shim, socket, daemon, query, writes back), not the query alone.
- **A run's one refresh stalls few calls, which no percentile shows** (SI-43): under S3b's write lock the other clients' outstanding calls wait for it in a shared run, only client 0's in a standalone one. The run reports the overlapping calls and the longest of them; the rule does not gate it (owner question 7).
- **The pressure observer is in the measuring process** (D126): a reading at the start, every second, and at the end. A critical episode shorter than a second between two readings can be missed; one that is read stops the run's clients.
- **`fanout.py` does not stop another prism-mcp process** and does not refuse to run beside one: the runbook makes the host quiet first and the run records the host's state before and after.
- **The pinned rule compares medians across runs**; one bad run in three does not decide it. All runs must be admissible and gating (every call answered, none retryable after warm-up, one daemon instance for a shared arm, exactly one successful refresh, no critical or unreadable pressure reading). Whether the bound can be met at all on this host is what the A/A calibration answers before the gate (D123).
- **`soak.py probe` proves a launcher starts and answers `initialize`**, not that it serves a tool call: a configuration whose server starts and then fails its first call is caught by the logs (`no_fail_closed`), not by the probe.

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously, or an earlier slice's note this slice was told to settle. "Cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D110 | **A Claude Code plugin user opts in with `PRISM_MCP_SHARED=1`, read by `scripts/prism-mcp-launch.sh`; `plugin.json` is unchanged.** | Spec §8 step 1 names "the Claude plugin launcher"; its arguments are in the installed manifest, which a user does not edit, and an unconditional `--shared` there would flip the plugin's default before the soak (spec §8 step 1 "the default stays in-process", §11 item 2). Claude Code hands plugin servers its whole environment (SI-42), so an `export` reaches the launcher. Owner question 1. | Owner question 1's other answers: one line of `plugin.json`, or nothing. |
| D111 | **Only `1` opts in; unset, empty and `0` do not; any other value stops the launcher with exit 2 and one line naming the variable, before the binary is looked for and before the arguments are read, so an explicit `--shared` or `--standalone` does not rescue it** (precedence stated after review round 1, sol S7; every combination tested). | A typo (`yes`, `true`, ` 1`) must decide nothing silently in either direction: silently standalone is the incident under fan-out, silently shared is an opt-in nobody gave; and a configuration with a bad value should be fixed, not hidden by a flag. Exit 2 is the shell's usage error; 127 stays "no binary". | Letting an explicit flag skip the check: moving the loop above the `case`. |
| D112 | **An explicit `--shared` or `--standalone` among the passed-through arguments wins over a valid value** (nothing is added). | `--shared` twice is a clap error, and `--shared --standalone` a conflict: the launcher must not turn a user's explicit choice into a startup failure. | None. |
| D113 | **`--shared` goes right after `--repo <path>`**, before the passed-through arguments. | Clap accepts any order; a fixed place gives one argument vector to test (mutant L5 pins it), and `set -- --shared "$@"` is bash 3.2 safe, where an empty array expanded under `set -u` is not. | None. |
| D114 | **Two pull requests** (after review round 1): **S6-1 = Tasks 1–2** (the launcher's opt-in; the sampler, the report and the probe; the CI step for the soak tools, moved here in review round 2, Opus S-E), **S6-2 = Tasks 3–4** (the fan-out driver; the docs, `.codex/config.toml`). | Review round 1's fold took production from 833 to 1,157 lines (1,262 after round 2's), past the lane's threshold for a split; each half is under it (760 and 502); the task boundary is where the reviewer's question changes (can users opt in and can the soak judge what it sees; then can the latency gate be trusted, and is everything documented). S6-2 builds on S6-1 (`fanout.py` imports `soak.py`); `.codex/config.toml` lands with S6-2, the last merge, after Phase 0 (owner question 4). | One PR again: merging the two branches. |
| D115 | **The repository's `.codex/config.toml` adds `--shared` and `env = { PRISM_MCP_RUNTIME_DIR = "/tmp" }`.** | Spec §8 step 1 names the file; it is this repository's own configuration (absolute paths of this host), not a user default. The runtime directory is owner question 2; when the change lands, owner question 4. | One file. |
| D116 | **Users' documentation of shared mode is one section of `docs/MCP.md`** (with two flag rows and two environment rows), plus one bullet of the navigation skill and a paragraph of `CLAUDE.md`. | Spec §4.3: "`docs/MCP.md` says so"; every "S6 owes" list points there; a separate page would split the flags from the server's other flags. The skill learns the retryable answers (S3a's note: neither the docs nor the skills named `warming` or `build_failed`). | Moving the section to its own page. |
| D117 | **The soak's instruments are Python 3.9 standard library in `scripts/soak/`; CI runs their hermetic tests** (one step of the build-checks job, a few seconds; added after review round 1, sol and Opus S7, which both recommended it; it lands with S6-1, whose tests it runs, review round 2). `fanout.py` imports memreport's workload binding and host state, so a memreport change that breaks them fails CI. The live test needs a built binary and runs only with `PRISM_SOAK_BIN` (Task 3, Task 4, Linux). memreport's own tests stay manual. | The instruments are the lane's acceptance; a regression in them should fail where everyone sees it. | Removing one step of `ci.yml`. |
| D118 | **An index holder is every daemon, and a standalone server whose measured footprint is at least 100 MiB; a daemon whose measured footprint is at least 100 MiB has built its index.** A process whose footprint could not be measured while it still ran is recorded `failed`, one that had ended `gone` (review round 1, sol 2). | A lazy standalone server that has built nothing is about 2–4 MiB (measured: 1.8 MiB for this session's plugin server); a built index is hundreds of MiB to GiB (C1). A shim never holds one. | A constant. |
| D119 | **`fanout.py` times each call at the client, from its write to its response, one call outstanding per client**; clients warm up first (the first workload call, retried while it is answered retryable, then one call of every other tool of the workload, review round 1, Opus S6), then start together at a barrier, each from its own share of the way through the workload; client 0 calls `refresh_index` once, half way through; percentiles are nearest-rank; refresh calls are reported apart. A gating run needs exactly that one refresh answered `ok`; `--no-refresh` makes a diagnostic run, which `compare` refuses (review round 1, sol 3). The workload is memreport's pinned one for the corpus, bound to the corpus's commit. | It measures what an agent waits for, under the interleaving fan-out produces, with the refresh the spec's §6.2 gate names. A refresh that failed or never ran would leave a run that measured only reads. | Re-running with other parameters: flags. |
| D120 | **Fail-closed lines, startup errors (clap's `error: …`, the launcher's refusals) and daemon losses are read from the clients' standard error: Claude Code's MCP logs and `prism-mcp-logged.sh` logs; aborts from macOS crash reports `prism-mcp[-_]*.ips`. A named log source or crash-report directory that is missing makes its rules UNKNOWN; no source at all counts as missing** (review round 1, sol 2, Opus S2). | A fail-closed shim reaches no daemon, so no daemon file records it; a binary that rejects `--shared` never becomes a shim at all. Daemon logs carry no time stamps and are truncated at a start over 1 MiB; crash reports are time-stamped and name the process. | A product-side event log, which would touch S5a's code. |
| D121 | **The sampler reads the process table (`ps -A -ww -o pid=,ppid=,rss=,time=,command=`; `time=` since review round 2, for D128) and `footprint`; it never connects to a daemon** unless `--status-bin` is given, and the soak's runbook does not give it (reconciled after review round 1, Opus S4). | Sampling must not change what it samples: a daemon of a build before S5b restarts its idle time at every `STATUS` probe (SI-35). `--status-bin` stays for a person's diagnosis. | — |
| D122 | **A shared fan-out run uses a runtime directory of its own under `/tmp`, stops its daemon with `daemon-shutdown` (expecting exit 0, then waits up to 60 s for the pid to go, then `SIGTERM`) and removes the directory; every client starts with `PRISM_MCP_STRUCTURED_CONTENT`, `PRISM_MCP_CONCISE_SHAPE`, `PRISM_MCP_MAX_RESULT_CHARS`, `PRISM_MCP_SHARED`, `PRISM_MCP_RUNTIME_DIR` and `PRISM_MCP_MEM_REPORT` removed from its environment.** | A run must not reach, or leave, a daemon anyone else uses, and both arms must answer with the same output settings whatever the controller's shell holds. | None. |
| D123 | **`fanout.py compare` judges the medians across runs: candidate p95 ≤ `P95_MAX_RATIO` × baseline p95 and candidate p99 ≤ `P99_MAX_RATIO` × baseline p99 (1.10 and 1.25); every run admissible, a gating run, of one corpus commit, workload, refresh point and client count, on a clean corpus. A run without percentiles fails the comparison instead of raising** (review round 1, Opus W8). The refresh's overlap with other clients' calls is reported per run, not gated (Opus W1). The runbook calibrates the bound with an A/A comparison, its runs in ABBA order and judged in both directions (`compare --both-directions`, `calibrate`; review round 2, Opus W-A), and runs the gate's pairs in ABBA order (Opus S1). | Spec §6.2 and §8 step 2 need a pinned rule before a run; timings on this host vary run to run, so one run decides nothing and a median of three is the least that does; an A/A comparison shows whether the bound can be met at all on this host. The constants are owner question 7. | Two constants. |
| D124 | **A soak rule is PASS, FAIL or UNKNOWN, and a phase PASS, FAIL or INADMISSIBLE** (review round 1, sol 2, Opus W2). A rule FAILs on a breach that was seen, whatever else is missing; it is UNKNOWN when the evidence it needs was not collected (the process table could not be read, a live standalone server's footprint could not be measured, the pressure level was unreadable, a named log source or the crash-report directory was missing); a phase is INADMISSIBLE when a gated rule is UNKNOWN or its use was not seen. | A rule that passes on what it could not see is the failure Review Focus 6 named; the reviewers reproduced five PASS verdicts from a fixture with the observations removed. | — |
| D125 | **The bridge round needs observed use**: at least `--min-shims` (2) shims at once, a daemon instance reached in the clients' logs and in every `--client-log` (the bridge's own; review round 2, Opus S-A), and a daemon with a built index; it counts only standalone index holders first seen after its first sample (review round 1, Opus W2, W3) and, of those, only the ones in the round's checkouts, named with the required `--round-repo` (review round 2, Opus W-D; paths resolved). | A round whose agents ran in containers (invisible to the host's process table) or whose wrapper path was wrong would otherwise pass on nothing; a session started before the export, or a launcher the soak does not edit, would otherwise fail a healthy round. Those already running are listed, not counted. | — |
| D126 | **`fanout.py` reads the memory pressure level at the start, every second, and at the end of a run, keeps every reading, stops the run's clients at the first critical reading, and makes the run inadmissible on a critical reading or, when required (the default on macOS), on an unreadable one** (review round 1, sol 4). | The Phase 1 stop condition had no observer: a critical episode that ended before the run did left an admissible run. A reading a second from the same process keeps one measured process. | A constant. |
| D127 | **`soak.py probe` starts a launcher's exact command line, sends `initialize`, and passes only when an answer comes back; the runbook probes every soaked launcher after deployment and after rollback, and the soak's binary is a pinned copy (`$E/bin/prism-mcp`, its sha256 recorded) for the bridge** (review round 1, sol 1, Opus W4, W5, S2). | A configuration that passes `--shared` to a binary that rejects it leaves a client without its server and writes no fail-closed line; the probe makes that a visible failure at the two moments it can be introduced, and `no_startup_error` catches it in the logs afterwards. A rebuild of the owner's working checkout must not change the soak's binary mid-phase. | — |
| D128 | **A usage day is a UTC day in which some single interval of at most `USAGE_INTERVAL_MAX_SECS` (120 s) shows at least `USAGE_INTERVAL_CPU_SECS` (0.05 s) of one daemon's CPU growth**: CPU readings from `ps`'s `time` for the same pid must exist at both consecutive endpoints; a pid or CPU reading missing at either end contributes nothing. Intervals longer than 120 s earn no usage credit, remain in the report, and are counted as `usage_intervals_excluded` (**owner decisions 2026-10-16**: per-interval growth and both endpoints after PR #424 review round 1, Opus W1; 120 s cap after round 2, gpt-6.1-sol and gpt-6-astra; fold after the two-round cap without a third round). | An idle window keeps an index resident all week. PR #424 round 1 measured about 0.8–0.9 s of idle CPU a day, so the former daily 0.5 s sum counted residency; the former `ps` measurement before and after 120 s could not see this at 10 ms resolution. The 0.05 s threshold now applies to one measured interval, rounded to microseconds. The 120 s cap is twice the default 60 s cadence and prevents long sampler gaps from accumulating idle CPU into usage credit. The interval's ending sample determines its UTC day. | Two constants; the day rule, one function. |
| D129 | **`sample --window-file T0` writes the window's start once; a resumed sampler keeps it and runs to the end of that window, printing a `resumed` line to its log; `report --window-file T0` takes the same start** (review round 2, sol 1, Opus W-E). | A resume that rewrote `T0` dropped a day-2 failure and the earlier usage days from the week's verdict (sol's probe: FAIL became PASS). | — |
| D130 | **The report gives the time the samples covered, the largest gap between two samples, and the count of intervals longer than 120 s excluded from usage credit** (owner decision 2026-10-16, PR #424 review round 2) (review round 2, Opus S-F). Isolated `ps` failures still make the process rules UNKNOWN (not tolerated up to a share): a week with one is re-judged by the owner from the report, rather than a tolerance pinned now. | A sampler dead for days left no mark. | — |
| D131 | **`no_footprint_growth` (owner request 2026-10-16): per (pid, key), baseline at the first built-index sample after 10 quiet minutes; a ≥ 5 s CPU burst in an observed interval, or a newly observed daemon rebuild-log line, resets it. FAIL only for growth within a quiet stretch lasting ≥ 1 hour, strictly above max(200 MiB, 25% of that epoch's baseline). Work growth is reported. Missing required footprints/process/CPU evidence is UNKNOWN; an observed breach takes precedence. A measured daemon that never establishes a baseline contributes nothing. All phases report every daemon's baseline, max, last, net growth per observed residency hour and largest quiet-stretch growth; only Phase 3 gates the rule.** | Pressure alone misses a daemon leaking steadily below the pressure line; build/warm-up, refreshes and working caches legitimately grow. | Owner-tunable named constants below; no automatic re-baselining of failures. |

## Spec items implemented, and spec issues found

Spec §12 row S6, item by item: "opt-in launcher/config changes (`.codex/config.toml`, plugin launcher, bridge entry, `PRISM_MCP_RUNTIME_DIR`)" → Task 4 (`.codex/config.toml`), Task 1 (the launcher), `docs/MCP.md` and "The soak" Phase 0 (the bridge entry and the runtime directory, SI-37); "docs" → Task 4; "soak instrumentation" → Tasks 2 and 3, and the soak's specification. Spec §9 lists no numbered test for S6 ("Soak: §8 step 2"); the tests here are this plan's own, listed under "Claims".

- **SI-36 (§8 step 1 "the Claude plugin launcher … add `--shared`"; lane README's S6 note "a config change, not a script change").** The plugin's arguments are in its installed manifest, not in user configuration, and an unconditional `--shared` would make shared mode the plugin's default before the soak. Read as: the launcher reads an opt-in from the environment (D110; owner question 1).
- **SI-37 (§8 step 1 "the bridge's `[[agents.mcp]]` entry add[s] `--shared`. The bridge also sets `PRISM_MCP_RUNTIME_DIR`").** The bridge's configurations live outside this repository (`~/.config/a2a-bridge/*.toml`, and the bridge's own `examples/`), which this slice does not edit. S6 documents the entry (`docs/MCP.md`, with a TOML snippet parsed in validation) and the soak's Phase 0 applies it with the owner's approval (owner question 5).
- **SI-38 (§8 step 2 "aggregate footprint (the 10-03 incident's own probe commands)").** Those commands are recorded nowhere found: not in the spec, the lane's plans or handoff, the controller handoffs of 2026-10-03 to 2026-10-09, the project memory, or `~/prism-evidence`. The soak measures what the incident reported: per-process `phys_footprint` (`footprint`), resident size (`ps`), and the host's swap (`sysctl vm.swapusage`), memory pressure level (`kern.memorystatus_vm_pressure_level`) and compressor (`vm_stat`, "Pages occupied by compressor"), which is the incident's "34 GiB compressed".
- **SI-39 (§8 step 2 "fan-out latency p95 vs. standalone").** Eight standalone servers of a mid-size repository are the memory incident itself (slicing: 8 × about 2 GiB of footprint on a 24 GB host). Read as: a like-for-like comparison of eight shims against eight standalone servers on the smallest pinned corpus (excalidraw, about 0.7 GiB each), and eight shims against one standalone client on slicing as information (owner question 9).
- **SI-40 (§8 step 2 "daemon restarts").** A daemon that starts again after an idle retirement is the design (TTL 15 minutes), not a failure. Read as: daemon instances are reported (from the samples and from the shims' `shared daemon pid` lines); what fails the soak is an abort (a crash report), a restart-bound refusal (`died during its index build`), or a server that never started (`no_startup_error`, D127).
- **SI-41 (§8 step 2 "one week of interactive use").** A week in which few sessions opted in, or sessions sat idle, shows nothing. Read as: the week counts only on days of observed use (a sample interval of daemon work: D128, revised in review round 2 and by the owner on 2026-10-16), at least 5 of 7 (owner question 8); a shorter one is extended.
- **SI-42 (§4.3 "Every launcher that should share a daemon must resolve the same directory; `docs/MCP.md` says so").** Measured: a plugin `prism-mcp` started by Claude Code on this host has the user's whole shell environment (its `ps eww` shows `TMPDIR`, `HOME`, `PATH`, and the user's own variables such as `GOROOT`), so its shims resolve `$TMPDIR/prism-mcpd`. Inherited, not measured here: the bridge's investigation of a containerized Codex (`~/code/a2a-bridge/docs/containerized-mcp-env-trap.md`) found its MCP servers started with `HOME`, a `PATH` of Codex's own and the configured `env` only. Whether a host-run Codex passes `TMPDIR` was not measured. A server whose directory differs from its peers' starts a second daemon silently. Read as: set `PRISM_MCP_RUNTIME_DIR` explicitly in every launcher (owner question 2); the soak's `one_index_per_key` rule detects a split.
- **SI-43 (S3b's SI-11, "a refresh holds the write lock for a whole rebuild; measure at the §6.2 fan-out gate").** One refresh among 1,600 calls stalls at most the 7 calls other clients have outstanding (and the next few that build the call-edge index again), which neither p95 nor p99 can show (review round 1, Opus W1). Read as: the gate reports the stall (`refresh_overlap`: the overlapping calls and the longest of them, both arms) and does not gate it; whether to bound it is in owner question 7. The earlier statement that Phase 1 "measures" SI-11 is withdrawn.

## Review Focus

Thirteen ways this work is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code, or is a question for the owner.

1. **A launcher change that opts in users who did not ask**, or opts out users who did. → Task 1, `without_the_variable_the_arguments_pass_through_unchanged`, `zero_or_empty_changes_nothing`, `prism_mcp_shared_1_adds_shared_after_the_repository`, `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` (a real shim reaches a real daemon through the launcher).
2. **A typo that silently decides** which mode a session gets (`PRISM_MCP_SHARED=yes`, `true`, ` 1`). → Task 1, `any_other_value_is_refused_before_anything_runs` (the binary never runs).
3. **The launcher's own `--shared` colliding with the user's** (`--shared --shared` is a clap error; `--shared --standalone` a conflict). → Task 1, `an_explicit_shared_or_standalone_argument_wins_over_the_variable`.
4. **A launcher that only works under a recent bash.** `"${array[@]}"` of an empty array under `set -u` fails in bash 3.2, the `/bin/bash` of every Mac. → Task 1 runs every argument test under `/bin/bash` (3.2 on macOS) and the process test through the script's own `#!`.
5. **Two clients that should share a daemon and do not** (different runtime directory, cache directory or build) — invisible to every client. → Task 2, `test_two_daemons_of_one_key_at_once_are_found`, `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates`; the docs' runtime-directory section; owner questions 2 and 3.
6. **A soak report that passes because it saw nothing**: fail-closed lines read from the wrong log, a time window in local time, crash reports of other programs. → Task 2, `test_claude_code_logs_are_read_for_prism_servers_in_the_window`, `test_iso_times_are_utc`, `test_crash_reports_of_prism_mcp_in_the_window_are_counted`, `test_report_writes_the_table_and_the_verdicts`; SI-41's usage floor.
7. **A latency comparison that is not like for like**: a client that inherits `PRISM_MCP_CONCISE_SHAPE=legacy` from the controller's shell, a warm-up answered `warming` counted as a timed call, clients in lockstep on the same seed. → Task 3, `test_clients_start_from_the_default_output_settings`, `test_every_client_makes_every_call_and_client_0_refreshes_once` (two warming answers first), `test_each_client_starts_its_own_share_of_the_way_through`.
8. **A fan-out run that hangs or leaves a daemon behind** when a client dies. → Task 3, `test_clients_that_die_while_warming_up_end_the_run`, `test_a_client_that_dies_makes_the_run_inadmissible_without_hanging_it`, `test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down`; the live test asserts the daemon is gone.
9. **A docs claim that does not match the code** (a reason's wording, an exit status, a default). → The docs quote the code's own strings; the reviewer is asked to check each row of the fail-closed table and the operator's exit statuses against `src/mcp/shim/{arbitrate,connect,diagnostic,run,pump,status,shutdown}.rs` and `src/mcp/shared/{runtime_dir,paths}.rs`.
10. **A soak that cannot fail, or cannot pass.** → "The soak": each rule names what would fail it; the A/A calibration shows whether the latency bound can be met on this host before any gate run; the pass thresholds are owner question 7; a gate shown unreachable by its own measurements goes to the owner at once (lane rule).
11. **A rule that passes on what it could not see** (review round 1, sol 2, Opus W2): a failed `ps`, a footprint not measured, no pressure reading, a missing log root, containers outside the process table, a round with no use. → Task 2, `test_soak_coverage.py` (`Unobserved`, `Use`, `test_a_round_report_without_its_log_sources_is_inadmissible`).
12. **A latency gate that passes without its refresh or under memory pressure** (review round 1, sol 3, sol 4). → Task 3, `test_a_gating_run_needs_exactly_one_successful_refresh`, `test_a_refresh_answered_with_a_tool_error_makes_the_run_inadmissible`, `test_a_critical_pressure_reading_stops_and_invalidates_the_run`, `test_an_unavailable_pressure_reading_invalidates_a_run_that_requires_it`.
13. **A deployment or a rollback that leaves a client without its server** (review round 1, sol 1, Opus W4): a configuration that passes `--shared` to a binary that rejects it. → Task 2, `test_a_binary_that_rejects_shared_fails_and_says_why` (`soak.py probe`), `test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules` (`no_startup_error`); "The soak", Phase 0's order and probes.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or docs state about behavior this slice adds, with the test that fails when it breaks and one wrong implementation that test rules out. Every mutant in the last column was applied in validation and the test failed.

| Claim | Test | A mutant it kills |
|---|---|---|
| Without `PRISM_MCP_SHARED`, or with it empty or `0`, the launcher passes its arguments through unchanged. | T1 `without_the_variable_the_arguments_pass_through_unchanged`, `zero_or_empty_changes_nothing` | `0` taken as on (L4) |
| `PRISM_MCP_SHARED=1` adds exactly one `--shared`, right after `--repo <path>`. | T1 `prism_mcp_shared_1_adds_shared_after_the_repository` | the variable ignored (L1); the flag last (L5) |
| The opt-in reaches a real shared daemon through the launcher run by its own `#!`. | T1 `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` | the variable ignored (L1) |
| An explicit `--shared` or `--standalone` always wins over the variable. | T1 `an_explicit_shared_or_standalone_argument_wins_over_the_variable` | no override (L2) |
| Any other value stops the launcher with exit 2 before the binary runs, an explicit `--shared` or `--standalone` included. | T1 `any_other_value_is_refused_before_anything_runs`, `the_variable_and_an_explicit_argument_in_every_combination` | any value opts in (L3); an explicit argument rescues it (L6) |
| A process is a shim only with `--shared`, a daemon only as `prism-mcp daemon`, an operator only as `daemon-status`/`daemon-shutdown`; a program whose binary is not named `prism-mcp` is never listed. | T2 `test_roles_come_from_the_command_line` | `--shared` not looked at (S1) |
| A standalone server is an index holder exactly from 100 MiB of footprint; a shim never is. | T2 `test_an_index_holder_is_a_daemon_or_a_standalone_server_past_100_mib` | `>` for `>=` (S2) |
| Two daemons of one key count as duplicates only when they are in the same sample. | T2 `test_two_daemons_of_one_key_at_once_are_found`, `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` | holders accumulated across samples (S3) |
| A usage day is a day daemons spent at least 0.5 s of CPU; a resident daemon or an attached idle session is not use. | T2 `test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident` | a daemon first sampled counted whole (C1); any growth (C2); presence counted (C4) |
| `ps`'s CPU time is read (macOS and procps forms). | T2 `test_roles_come_from_the_command_line` | not read (C3) |
| A resumed sampler keeps its window's start; a failure and usage days before a resume stay in the week's verdict. | T2 `test_a_resumed_sampler_keeps_the_window_it_started`, `test_a_day_two_failure_and_earlier_usage_days_survive_a_resume` | the start rewritten (W1) |
| A round gates new standalone servers only in its own checkouts (paths resolved), and refuses to run without them named. | T2 `test_only_standalone_servers_of_the_round_s_checkouts_are_gated`, `test_a_round_report_without_its_checkouts_is_refused` | every checkout gated (R1); paths not resolved (R2); no `--round-repo` accepted (R4) |
| A round needs a reached daemon in every client log. | T2 `test_a_round_needs_a_reached_daemon_in_every_client_log` | the check ignored (R3) |
| The report gives the largest gap between samples and the time they covered. | T2 `test_the_largest_gap_between_samples_and_the_time_covered_are_reported` | the smallest gap (G1) |
| Only daemons count toward a repository's daemons. | T2 `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` | every role counted (S12) |
| A daemon instance a client reached is counted once however many lines name it. | T2 `test_client_lines_are_counted` | a list for the set (S4) |
| Only the logs of servers whose name contains `prism`, and only entries in the window, are read. | T2 `test_claude_code_logs_are_read_for_prism_servers_in_the_window` | every server's logs (S5); no window (S6) |
| Times are read as UTC, in any local zone, winter and summer. | T2 `test_iso_times_are_utc_in_any_local_zone_winter_and_summer` | local time (S7); local time less the standard offset (S7b) |
| Only prism-mcp's crash reports, and only in the window, count as aborts. | T2 `test_crash_reports_of_prism_mcp_in_the_window_are_counted` | any program's (S8) |
| Each rule fails on its own breach and on no other. | T2 `test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach` | critical taken as normal (S9) |
| A rule is never PASS on evidence that was not collected: no process table, a live standalone server's footprint not measured, no pressure reading, a named log source or the crash-report directory missing (UNKNOWN); a breach that was seen still FAILs. | T2 `test_a_failed_process_table_makes_the_process_rules_unknown`, `test_a_failed_footprint_of_a_standalone_server_is_unknown_and_a_vanished_one_is_not`, `test_a_missing_pressure_reading_is_unknown_and_a_critical_one_still_fails`, `test_unread_logs_or_crash_reports_are_unknown_and_what_was_read_still_fails`, `test_log_sources_are_complete_only_when_every_named_one_exists` | U1–U5, U8 |
| A process that ended is `gone`; one that still runs without a reading `failed`; a failed `ps` is recorded as such. | T2 `test_a_vanished_pid_is_gone_and_a_live_one_without_a_reading_failed`, `test_a_sample_whose_process_table_could_not_be_read_says_so` | U6, U7 |
| A round passes only with use seen (shims, a reached daemon, a built index) and every gated rule PASS; otherwise INADMISSIBLE, or FAIL on a breach. | T2 `test_a_round_needs_shims_a_reached_daemon_and_a_built_index`, `test_a_round_report_without_its_log_sources_is_inadmissible` | U9, U11 |
| Standalone holders already running at a phase's first sample are never counted against it. | T2 `test_standalone_holders_already_running_at_the_start_do_not_fail_a_phase` | U12 |
| A server that never started (clap's `error: …`, the launcher's refusal) and a restart-bound refusal each fail their rule. | T2 `test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules` | U13, U14 |
| A binary path with a space in it is still recognized; another program naming a `prism-mcp` path is not. | T2 `test_a_binary_path_with_a_space_is_still_prism_mcp` | U15 |
| A sample line cut short is skipped and counted, never fatal. | T2 `test_a_truncated_last_line_is_skipped_and_counted` | U16 |
| `probe` passes only a server that answers `initialize`; a binary that rejects `--shared` and a silent one fail, with the error shown. | T2 `test_a_server_that_answers_initialize_passes`, `test_a_binary_that_rejects_shared_fails_and_says_why`, `test_a_server_that_never_answers_fails_at_the_timeout` | U17 |
| The wrapper changes nothing a client sees (standard input, output, error, exit status) and logs every standard-error line, the last one without a newline included, with the prism-mcp process's pid. | T2 `test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes` | a last line lost (S10); standard error swallowed (S11) |
| Percentiles are nearest-rank. | T3 `test_percentile_is_nearest_rank` | an off-by-one rank (F1) |
| A retryable answer is never taken for a result; a tool's own error is an answer. | T3 `test_outcome_tells_the_four_answers_apart`, `test_a_tool_error_is_an_answer_not_a_failure` | retryable not recognized (F2) |
| No two clients start at the same place in the workload (fewer clients than calls). | T3 `test_each_client_starts_its_own_share_of_the_way_through` | lockstep (F3) |
| A run is admissible only when every client made every call, none was answered retryable or with a JSON-RPC error, and a shared run reached exactly one daemon. | T3 `test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible` | count not checked (F4); daemons not checked (F5) |
| The rule passes at its bounds and fails past them, on medians across runs; an inadmissible run or another workload fails it. | T3 `test_the_rule_passes_at_its_bounds_and_fails_past_them`, `test_medians_across_runs_are_compared`, `test_an_inadmissible_run_or_another_workload_fails_the_comparison` | strict bounds (F6); means (F7) |
| Client 0 refreshes exactly once; warming answers before the barrier are retried and never timed. | T3 `test_every_client_makes_every_call_and_client_0_refreshes_once` | no refresh (F8); no retry (F9) |
| A client that dies, while warming up or later, ends the run inadmissible and never hangs it. | T3 `test_clients_that_die_while_warming_up_end_the_run`, `test_a_client_that_dies_makes_the_run_inadmissible_without_hanging_it` | no barrier abort (F10) |
| A shared run always asks its daemon to shut down. | T3 `test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down`; the live test (the daemon gone) | no shutdown (F11) |
| Every client starts from the default output settings whatever the controller's shell holds. | T3 `test_clients_start_from_the_default_output_settings` | the environment passed whole (F12) |
| A used run directory is never overwritten. | T3 `test_a_used_run_directory_is_refused` | — |
| An A/A calibration passes only when each arm passes against the other. | T3 `test_a_calibration_passes_only_when_both_directions_pass` | one direction (A1) |
| A gating run is admissible only with exactly one refresh answered `ok`; a run without one is diagnostic and never compared. | T3 `test_a_gating_run_needs_exactly_one_successful_refresh`, `test_a_refresh_answered_with_a_tool_error_makes_the_run_inadmissible`, `test_a_run_without_a_refresh_is_diagnostic_and_never_compared` | F13, F14 |
| The refresh's overlap counts only other clients' calls whose time overlaps it. | T3 `test_the_calls_that_overlap_the_refresh_are_counted_and_their_longest_reported` | F15 |
| A critical pressure reading always stops the run's clients and invalidates the run; an unreadable one invalidates a run that requires readings. | T3 `test_a_critical_pressure_reading_stops_and_invalidates_the_run`, `test_an_unavailable_pressure_reading_invalidates_a_run_that_requires_it` | F16, F17 |
| `compare` never raises on a run without calls, and never compares runs of different client counts (unless told it is information) or refresh points. | T3 `test_a_run_with_no_calls_fails_the_comparison_instead_of_crashing_it`, `test_runs_of_other_client_counts_or_refresh_points_are_not_compared` | F18, F19, F20 |
| A shared run keeps its daemon's logs; every tool of the workload is warmed before the barrier. | T3 `test_a_shared_run_keeps_its_daemon_logs`, `test_every_tool_of_the_workload_is_warmed_before_the_barrier` | F21, F22 |

**Not held by a test:** `stop_daemon`'s handling of a `daemon-shutdown` that does not return within its own timeout plus 30 s (caught and recorded; not constructed); the runbook (a procedure, checked by review and by `soak.py probe` when it runs); the docs' prose about the shim, the daemon and the operator's commands (held by S4–S5b's tests of that code; checked by review against the code, Review Focus 9); the `.codex/config.toml` change (a configuration the owner reads; its TOML was parsed with Python 3.14's `tomllib` in validation); `footprints` on macOS against a real daemon, and `host_memory` (exercised by the live sample test and the live fan-out test, values not asserted); `stop_daemon`'s `SIGTERM` fallback (a daemon that ignores `daemon-shutdown` for 60 s; not constructed).

## File Structure

| File | What | Change | Task |
|---|---|---|---|
| `scripts/prism-mcp-launch.sh` | `PRISM_MCP_SHARED` read, refused or turned into `--shared` | Modify | 1 |
| `tests/fixtures/mcp/launcher/argv-stub.sh` | A stand-in binary that prints its arguments (executable) | Create | 1 |
| `tests/mcp/launcher_test.rs`, `tests/mcp/main.rs` | The launcher's argument tests and its process test | Create, modify | 1 |
| `scripts/soak/soak.py`, `scripts/soak/soak_judge.py` | The sampler and the probe; the soak report's judging | Create | 2 |
| `scripts/soak/test_soak.py`, `scripts/soak/test_soak_coverage.py` | Their tests; what is not observed, and the use a phase needs | Create | 2 |
| `scripts/soak/prism-mcp-logged.sh` | A client's prism-mcp standard error logged with time and pid (executable) | Create | 2 |
| `scripts/soak/fake_prism_mcp.py` | A stand-in server for the probe's and the driver's tests (executable) | Create | 2 |
| `scripts/soak/fanout.py`, `scripts/soak/test_fanout.py`, `scripts/soak/test_fanout_gate.py`, `scripts/soak/test_fanout_live.py` | The fan-out latency driver and its tests | Create | 3 |
| `docs/MCP.md` | "Shared daemon" section; two flags; two environment variables | Modify | 4 |
| `skills/prism-code-navigation/SKILL.md` | Retryable answers; shared refresh | Modify | 4 |
| `CLAUDE.md` | The launcher's opt-in and the soak tools | Modify | 4 |
| `.github/workflows/ci.yml` | One step: the soak tools' hermetic tests | Modify | 2 |
| `.codex/config.toml` | `--shared` and `PRISM_MCP_RUNTIME_DIR` for this repository's Codex sessions | Modify | 4 |

---

### Task 1: The launcher's opt-in (`PRISM_MCP_SHARED`)

**Files:** Modify `scripts/prism-mcp-launch.sh`, `tests/mcp/main.rs`; create `tests/fixtures/mcp/launcher/argv-stub.sh` (executable), `tests/mcp/launcher_test.rs`.

**Interfaces:** the launcher's command line is unchanged (`prism-mcp-launch.sh <repo-path> [extra prism-mcp args...]`); it reads one more environment variable. Exit statuses: 2 for a refused `PRISM_MCP_SHARED` (new), 127 when no binary is found (unchanged), otherwise the binary's.

- [ ] **Step 1: Write the failing tests**

Create `tests/fixtures/mcp/launcher/argv-stub.sh`:

````bash
#!/bin/sh
# A stand-in for prism-mcp in the launcher's tests (tests/mcp/launcher_test.rs): it prints each
# argument it was given on a line of its own, and nothing else.
for arg in "$@"; do
  printf '%s\n' "$arg"
done
````

Make `tests/fixtures/mcp/launcher/argv-stub.sh` executable.

Create `tests/mcp/launcher_test.rs`:

````rust
//! `scripts/prism-mcp-launch.sh`, the Claude Code plugin's launcher, and its opt-in to the shared
//! daemon (`PRISM_MCP_SHARED`). The argument tests run it against a stand-in that prints its
//! arguments (`tests/fixtures/mcp/launcher/argv-stub.sh`); the last test runs it against the
//! built binary and reaches a shared daemon. The binary is reached through a symbolic link in a
//! plugin root of the test's own, so no test writes a file it then executes.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{setup, CALLEES, HANG, INIT, INITED};
use crate::shim_process_test::Daemons;
use crate::transcript::spawn_line_reader;
use prism::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SHARED_ENV: &str = "PRISM_MCP_SHARED";

fn launcher() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/prism-mcp-launch.sh")
}

/// A plugin root whose `target/release/prism-mcp` is a symbolic link to `binary`.
fn plugin_root(binary: &Path) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a plugin root");
    let release = root.path().join("target/release");
    std::fs::create_dir_all(&release).expect("target/release");
    std::os::unix::fs::symlink(binary, release.join("prism-mcp")).expect("link the binary");
    root
}

/// The launcher under `/bin/bash` (on macOS bash 3.2, the oldest a user has) with the stand-in
/// as the binary, `shared` as `PRISM_MCP_SHARED` (`None`: unset) and `args` after the
/// repository.
fn launch(shared: Option<&str>, args: &[&str]) -> Output {
    let stub =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp/launcher/argv-stub.sh");
    let root = plugin_root(&stub);
    let mut command = Command::new("/bin/bash");
    command
        .arg(launcher())
        .arg("/some/repo")
        .args(args)
        .env("CLAUDE_PLUGIN_ROOT", root.path())
        .env_remove(SHARED_ENV)
        .stdin(Stdio::null());
    if let Some(value) = shared {
        command.env(SHARED_ENV, value);
    }
    command.output().expect("run the launcher")
}

/// The arguments the stand-in was given, one per line.
fn argv(output: &Output) -> Vec<String> {
    assert!(
        output.status.success(),
        "the launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .expect("UTF-8")
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn without_the_variable_the_arguments_pass_through_unchanged() {
    assert_eq!(
        argv(&launch(None, &["--cache-dir", "/a dir"])),
        ["--repo", "/some/repo", "--cache-dir", "/a dir"]
    );
}

#[test]
fn prism_mcp_shared_1_adds_shared_after_the_repository() {
    assert_eq!(
        argv(&launch(Some("1"), &["--cache-dir", "/a dir"])),
        ["--repo", "/some/repo", "--shared", "--cache-dir", "/a dir"]
    );
    assert_eq!(
        argv(&launch(Some("1"), &[])),
        ["--repo", "/some/repo", "--shared"]
    );
}

#[test]
fn zero_or_empty_changes_nothing() {
    for value in ["0", ""] {
        assert_eq!(
            argv(&launch(Some(value), &["--eager"])),
            ["--repo", "/some/repo", "--eager"],
            "PRISM_MCP_SHARED={value:?}"
        );
    }
}

#[test]
fn an_explicit_shared_or_standalone_argument_wins_over_the_variable() {
    assert_eq!(
        argv(&launch(Some("1"), &["--standalone"])),
        ["--repo", "/some/repo", "--standalone"]
    );
    assert_eq!(
        argv(&launch(Some("1"), &["--cache-dir", "/c", "--shared"])),
        ["--repo", "/some/repo", "--cache-dir", "/c", "--shared"]
    );
}

#[test]
fn any_other_value_is_refused_before_anything_runs() {
    for value in ["yes", "true", "2", " 1"] {
        let output = launch(Some(value), &[]);
        assert_eq!(output.status.code(), Some(2), "PRISM_MCP_SHARED={value:?}");
        assert!(output.stdout.is_empty(), "the binary ran for {value:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("PRISM_MCP_SHARED must be 1") && stderr.contains(&format!("'{value}'")),
            "{stderr}"
        );
    }
}

/// Every combination of the variable and an explicit mode argument (review round 1, sol S7): an
/// explicit `--shared` or `--standalone` wins over a valid value, and a value the launcher does
/// not accept is refused even then, because the variable is checked first.
#[test]
fn the_variable_and_an_explicit_argument_in_every_combination() {
    for value in [None, Some(""), Some("0"), Some("1"), Some("yes")] {
        for explicit in [None, Some("--shared"), Some("--standalone")] {
            let args: Vec<&str> = explicit.into_iter().collect();
            let output = launch(value, &args);
            let what = format!("PRISM_MCP_SHARED={value:?} with {explicit:?}");
            if value == Some("yes") {
                assert_eq!(output.status.code(), Some(2), "{what}");
                assert!(output.stdout.is_empty(), "{what}: the binary ran");
                continue;
            }
            let mut expected = vec!["--repo", "/some/repo"];
            match explicit {
                Some(flag) => expected.push(flag),
                None if value == Some("1") => expected.push("--shared"),
                None => {}
            }
            assert_eq!(argv(&output), expected, "{what}");
        }
    }
}

/// A launcher process, killed and reaped when the test ends however it ends.
struct Launched(std::process::Child);

impl Drop for Launched {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The launcher run as the plugin runs it (directly, through its `#!` line) with the built
/// binary and `PRISM_MCP_SHARED=1` starts a shim that reaches a shared daemon and serves a tool
/// call through it.
#[test]
fn with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let root = plugin_root(Path::new(env!("CARGO_BIN_EXE_prism-mcp")));
    let stderr_dir = tempfile::tempdir().expect("a directory for standard error");
    let stderr_path = stderr_dir.path().join("stderr.txt");
    let stderr = || std::fs::read_to_string(&stderr_path).unwrap_or_default();
    let mut launched = Launched(
        Command::new(launcher())
            .arg(setup.repo.path())
            .arg("--cache-dir")
            .arg(setup.cache.path())
            .args(["--daemon-start-timeout", "60"])
            .env("CLAUDE_PLUGIN_ROOT", root.path())
            .env(SHARED_ENV, "1")
            .env(RUNTIME_DIR_ENV, setup.base.path())
            .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
            .env_remove("PRISM_MCP_CONCISE_SHAPE")
            .env_remove("PRISM_MCP_MAX_RESULT_CHARS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(
                File::create(&stderr_path).expect("stderr file"),
            ))
            .spawn()
            .expect("start the launcher"),
    );
    let mut stdin = launched.0.stdin.take().expect("stdin");
    let (lines, _reader) = spawn_line_reader(launched.0.stdout.take().expect("stdout"));
    let request = |stdin: &mut std::process::ChildStdin, line: &str| {
        writeln!(stdin, "{line}").expect("write a request");
        stdin.flush().expect("flush");
        match lines.recv_timeout(HANG) {
            Ok(Ok(response)) => {
                serde_json::from_slice::<serde_json::Value>(&response).expect("a JSON response")
            }
            other => panic!("no response: {other:?}; stderr: {}", stderr()),
        }
    };
    assert!(request(&mut stdin, INIT)["result"].is_object());
    writeln!(stdin, "{INITED}").expect("write the notification");
    let answer = request(&mut stdin, CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    drop(stdin);
    let status = launched.0.wait().expect("the shim's status");
    assert!(status.success(), "{status:?}: {}", stderr());
    assert!(
        stderr().contains("prism-mcp: shared daemon pid "),
        "no daemon was reached: {}",
        stderr()
    );
}
````

In `tests/mcp/main.rs`, replace this:

````rust
mod daemon_control_process_test;
mod daemon_process_test;
mod golden_transcript_test;
mod mem_report_test;
mod member_only_param_test;
mod settings_test;
````

with this:

````rust
mod daemon_control_process_test;
mod daemon_process_test;
mod golden_transcript_test;
mod launcher_test;
mod mem_report_test;
mod member_only_param_test;
mod settings_test;
````

- [ ] **Step 2: Run them and watch three fail**

```bash
LOG_DIR=${LOG_DIR:?set LOG_DIR to a directory of your own}
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp launcher > "$LOG_DIR/t1-red.log" 2>&1; echo "rc=$?"
grep -E '^test |^test result' "$LOG_DIR/t1-red.log"
```

Expected: `test result: FAILED. 3 passed; 4 failed` (52 filtered out), the four failures `any_other_value_is_refused_before_anything_runs` (`PRISM_MCP_SHARED="yes"`, `left: Some(0)`, `right: Some(2)`: the old launcher runs the binary whatever the value), `prism_mcp_shared_1_adds_shared_after_the_repository` (`left: ["--repo", "/some/repo", "--cache-dir", "/a dir"]`), `the_variable_and_an_explicit_argument_in_every_combination` (`PRISM_MCP_SHARED=Some("1") with None`: no `--shared` added) and `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` (`no response: Err(Disconnected)`; the binary refuses `--daemon-start-timeout` without `--shared`: `error: the following required arguments were not provided: --shared`). The three that pass describe today's pass-through, which the opt-in must keep.

- [ ] **Step 3: Implement the opt-in**

In `scripts/prism-mcp-launch.sh`, replace this:

````bash
# navigate is passed as the first argument (the plugin wires it to ${CLAUDE_PROJECT_DIR}); any
# further arguments are passed through to prism-mcp (e.g. --cache-dir, --no-cache).
#
# Usage: prism-mcp-launch.sh <repo-path> [extra prism-mcp args...]
set -euo pipefail

repo="${1:?usage: prism-mcp-launch.sh <repo-path> [args...]}"
shift

root="${CLAUDE_PLUGIN_ROOT:-}"
if [ -n "$root" ] && [ -x "$root/target/release/prism-mcp" ]; then
````

with this:

````bash
# navigate is passed as the first argument (the plugin wires it to ${CLAUDE_PROJECT_DIR}); any
# further arguments are passed through to prism-mcp (e.g. --cache-dir, --no-cache).
#
# PRISM_MCP_SHARED=1 in the environment opts in to the shared per-repository daemon: the launcher
# adds --shared after --repo, unless the arguments already say --shared or --standalone.
# Unset, empty or 0 changes nothing. Any other value is refused before anything runs, an explicit
# --shared or --standalone included (the variable is checked first), so a typo never decides
# silently which way a client goes.
#
# Usage: prism-mcp-launch.sh <repo-path> [extra prism-mcp args...]
set -euo pipefail

repo="${1:?usage: prism-mcp-launch.sh <repo-path> [args...]}"
shift

case "${PRISM_MCP_SHARED:-}" in
  "" | 0) shared=0 ;;
  1) shared=1 ;;
  *)
    echo "prism-mcp-launch.sh: PRISM_MCP_SHARED must be 1 (use the shared daemon) or 0 or unset (an index of this process's own), not '${PRISM_MCP_SHARED}'" >&2
    exit 2
    ;;
esac
for arg in "$@"; do
  case "$arg" in
    --shared | --standalone) shared=0 ;;
  esac
done
if [ "$shared" = 1 ]; then
  set -- --shared "$@"
fi

root="${CLAUDE_PLUGIN_ROOT:-}"
if [ -n "$root" ] && [ -x "$root/target/release/prism-mcp" ]; then
````

- [ ] **Step 4: Run them and watch them pass**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp launcher > "$LOG_DIR/t1-green.log" 2>&1; echo "rc=$?"
grep -E '^test result|^warning' "$LOG_DIR/t1-green.log"
pgrep -fl 'prism-mcp daemon'
```

Expected: `rc=0`, `test result: ok. 7 passed; 0 failed` (52 filtered out), no `warning` line; `pgrep` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add scripts/prism-mcp-launch.sh tests/fixtures/mcp/launcher/argv-stub.sh tests/mcp/launcher_test.rs tests/mcp/main.rs
git commit -m "feat(mcp): S6 — the plugin launcher opts in to the shared daemon on PRISM_MCP_SHARED=1"
```

(End the message with the lane's `Co-Authored-By` and `Claude-Session` lines.)

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone to `scripts/prism-mcp-launch.sh`, run Step 4's command, then `git checkout -- scripts/prism-mcp-launch.sh`.

| Mutant | Edit | Expected failing tests (observed) |
|---|---|---|
| L1 | `  1) shared=1 ;;` → `  1) shared=0 ;;` (the variable ignored) | `prism_mcp_shared_1_adds_shared_after_the_repository`, `the_variable_and_an_explicit_argument_in_every_combination`, `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` |
| L2 | `    --shared \| --standalone) shared=0 ;;` → `    --never-matches) shared=0 ;;` (no explicit argument wins) | `an_explicit_shared_or_standalone_argument_wins_over_the_variable`, `the_variable_and_an_explicit_argument_in_every_combination` |
| L3 | a `*) shared=1 ;;` arm before the refusal (any value opts in) | `any_other_value_is_refused_before_anything_runs`, `the_variable_and_an_explicit_argument_in_every_combination` |
| L4 | `"" \| 0) shared=0 ;;` / `1) shared=1 ;;` → `"") shared=0 ;;` / `0 \| 1) shared=1 ;;` (`0` opts in) | `zero_or_empty_changes_nothing`, `the_variable_and_an_explicit_argument_in_every_combination` |
| L5 | `set -- --shared "$@"` → `set -- "$@" --shared` (the flag last) | `prism_mcp_shared_1_adds_shared_after_the_repository` |
| L6 | `exit 2` → `[ "$#" -gt 0 ] \|\| exit 2` then `shared=0` (an explicit argument rescues a bad value; review round 1, sol S7) | `the_variable_and_an_explicit_argument_in_every_combination` |

---

### Task 2: The soak's sampler, report and client log wrapper

**Files:** Create `scripts/soak/soak.py`, `scripts/soak/soak_judge.py`, `scripts/soak/test_soak.py`, `scripts/soak/test_soak_coverage.py`, `scripts/soak/test_soak_round2.py`, `scripts/soak/fake_prism_mcp.py` (executable), `scripts/soak/prism-mcp-logged.sh` (executable); modify `.github/workflows/ci.yml` (one step).

**Interfaces** (Task 3 and the soak use them): `soak.footprints(pids) -> {pid: bytes}`, `soak.measure_footprints(pids) -> (sizes, {pid: 'ok'|'gone'|'failed'})`, `soak.read_pressure() -> int | None`, `soak.alive(pid)`; `soak.py sample --out FILE.jsonl [--interval S] [--count N | --duration S [--window-file T0]] [--status-bin BIN --runtime-dir BASE]`, one JSON object per line (`t`, `ps_ok`, `host`: `swap_mib` {`total`,`used`,`free`}, `pressure` (or null), `compressed_bytes`; `processes`: `pid`, `ppid`, `role`, `repo`, `key`, `cache_dir`, `rss_bytes`, `cpu_secs`, `footprint_bytes`, `footprint_status`, `rebuild_observed` (optional passive log event), `holder`, `command`; `daemon_status` text when asked); `soak.py report --phase fanout|round|week --samples FILE --out FILE.md [--json FILE] [--since EPOCH | --window-file T0] [--until EPOCH] [--claude-logs DIR] [--client-log FILE ...] [--crash-reports DIR] [--round-repo DIR ...] [--min-shims N] [--min-days N]` (`--round-repo` required for a round), each of `RULES` (`one_index_per_key`, `no_new_standalone_index`, `no_fail_closed`, `no_startup_error`, `no_restart_bound`, `no_abort`, `no_critical_pressure`, `no_footprint_growth`) `PASS`, `FAIL` or `UNKNOWN`, the use seen, and `**Verdict: PASS|FAIL|INADMISSIBLE**`; `soak.py probe [--timeout S] -- COMMAND …`, exit 0 (`PASS`) or 1 (`FAIL` with the server's standard error); `prism-mcp-logged.sh LOGFILE BIN [args...]`, lines `<UTC time> <pid> <line>`.

- [ ] **Step 1: Write the failing tests**

Create `scripts/soak/fake_prism_mcp.py`:

````python
#!/usr/bin/env python3
"""A stand-in for prism-mcp in the fan-out driver's tests (test_fanout.py). It speaks just enough
MCP: `initialize`, notifications, `tools/call` (refresh_index included). Environment:
FAKE_WARMING=N answers the first N tool calls retryable (as an index that is warming);
FAKE_DIE_AFTER=N exits after N tool calls; FAKE_DELAY_MS=N waits before each answer;
FAKE_DAEMON_PID=P makes a `--shared` process report daemon pid P on standard error;
`daemon-shutdown` as the first argument prints a line and exits FAKE_SHUTDOWN_EXIT (default 0).
It writes the PRISM_MCP_CONCISE_SHAPE it was given to standard error, and at the end of its input
`fake: N tool calls`. FAKE_REFRESH=retryable|rpc_error|tool_error answers refresh_index so. A
`--shared` process appends a line to `fake.log` in $PRISM_MCP_RUNTIME_DIR/prism-mcpd, as a daemon
would to its log."""
import json
import os
import sys
import time


def answer(ident, result):
    sys.stdout.write(json.dumps({'jsonrpc': '2.0', 'id': ident, 'result': result}) + '\n')
    sys.stdout.flush()


def main():
    if sys.argv[1:2] == ['daemon-shutdown']:
        print('fake: nothing to shut down')
        sys.exit(int(os.environ.get('FAKE_SHUTDOWN_EXIT', '0')))
    sys.stderr.write('fake: PRISM_MCP_CONCISE_SHAPE=%s\n' % os.environ.get('PRISM_MCP_CONCISE_SHAPE', '<unset>'))
    if '--shared' in sys.argv and os.environ.get('PRISM_MCP_RUNTIME_DIR'):
        logs = os.path.join(os.environ['PRISM_MCP_RUNTIME_DIR'], 'prism-mcpd')
        os.makedirs(logs, exist_ok=True)
        with open(os.path.join(logs, 'fake.log'), 'a') as log:
            log.write('fake daemon log\n')
    if '--shared' in sys.argv:
        sys.stderr.write('prism-mcp: shared daemon pid %s (started 7, generation 1) at /fake.sock\n'
                         % os.environ.get('FAKE_DAEMON_PID', '1'))
        sys.stderr.flush()
    warming = int(os.environ.get('FAKE_WARMING', '0'))
    die_after = int(os.environ.get('FAKE_DIE_AFTER', '0'))
    delay = int(os.environ.get('FAKE_DELAY_MS', '0')) / 1000.0
    calls = 0
    for line in sys.stdin:
        message = json.loads(line)
        if 'id' not in message:
            continue
        if message['method'] == 'initialize':
            answer(message['id'], {'serverInfo': {'name': 'prism-mcp', 'version': 'fake'}, 'capabilities': {}})
            continue
        calls += 1
        if die_after and calls > die_after:
            sys.exit(1)
        time.sleep(delay)
        if calls <= warming:
            answer(message['id'], {'isError': True, 'content': [{'type': 'text', 'text': 'warming'}],
                                   '_meta': {'prism/retryable': True, 'prism/index_state': 'warming'}})
        elif message['params']['name'] == 'refresh_index':
            refresh = os.environ.get('FAKE_REFRESH', 'ok')
            if refresh == 'rpc_error':
                sys.stdout.write(json.dumps({'jsonrpc': '2.0', 'id': message['id'],
                                             'error': {'code': -32603, 'message': 'fake'}}) + '\n')
                sys.stdout.flush()
            elif refresh == 'retryable':
                answer(message['id'], {'isError': True, 'content': [], '_meta': {'prism/retryable': True}})
            else:
                answer(message['id'], {'isError': refresh == 'tool_error', 'content': [{'type': 'text', 'text': '{}'}],
                                       '_meta': {'prism/refresh_status': 'unchanged'}})
        else:
            answer(message['id'], {'isError': False, 'content': [{'type': 'text', 'text': '{}'}]})
    sys.stderr.write('fake: %d tool calls\n' % calls)


if __name__ == '__main__':
    main()
````

Make `scripts/soak/fake_prism_mcp.py` executable.

Create `scripts/soak/test_soak.py`:

````python
"""Tests of soak.py and prism-mcp-logged.sh.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import calendar
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402

MIB = 1024 * 1024
KEY = 'ab' * 32
PS = """  101     1   4096   1:02.50 /x/target/release/prism-mcp daemon --repo /r --cache-dir /c --refresh-policy warn-only --key %s
  102    50   3000   0:00.01 /x/target/release/prism-mcp --repo /r --cache-dir /c --shared
  103    50 900000 01:02:03 /x/prism-mcp --repo /other
  104    50   2000   0:00.00 /x/prism-mcp daemon-status --runtime-dir /tmp
  105    50   1000   0:00.00 /usr/bin/vim prism-mcp
  106    50   1000   0:00.00 /x/prism-mcp.pre-389 --repo /r
""" % KEY


def proc(role, pid, footprint=0, key=KEY, repo='/r', status='ok', cache='/c', cpu=None):
    return {'role': role, 'pid': pid, 'key': key if role == 'daemon' else None, 'repo': repo, 'cache_dir': cache,
            'cpu_secs': cpu,
            'footprint_bytes': footprint if status == 'ok' else None, 'footprint_status': status,
            'holder': soak.is_holder({'role': role, 'footprint_bytes': footprint if status == 'ok' else None})}


def sample(t, processes, swap_used=0.0, pressure=1, compressed=0, ps_ok=True):
    return {'t': t, 'processes': processes, 'ps_ok': ps_ok,
            'host': {'swap_mib': {'used': swap_used}, 'pressure': pressure, 'compressed_bytes': compressed}}


ALL_READ = {'logs': True, 'crash_reports': True}


class Processes(unittest.TestCase):
    def test_roles_come_from_the_command_line(self):
        found = {p['pid']: p for p in soak.parse_ps(PS)}
        self.assertEqual(sorted(found), [101, 102, 103, 104])
        self.assertEqual((found[101]['role'], found[101]['key'], found[101]['repo']), ('daemon', KEY, '/r'))
        self.assertEqual(found[102]['role'], 'shim')
        self.assertEqual((found[103]['role'], found[103]['rss_bytes']), ('standalone', 900000 * 1024))
        self.assertEqual((found[101]['cpu_secs'], found[103]['cpu_secs']), (62.5, 3723.0))
        self.assertEqual(found[104]['role'], 'operator')

    def test_an_index_holder_is_a_daemon_or_a_standalone_server_past_100_mib(self):
        self.assertTrue(soak.is_holder({'role': 'daemon', 'footprint_bytes': 0}))
        self.assertTrue(soak.is_holder({'role': 'standalone', 'footprint_bytes': 100 * MIB}))
        self.assertFalse(soak.is_holder({'role': 'standalone', 'footprint_bytes': 100 * MIB - 1}))
        self.assertFalse(soak.is_holder({'role': 'standalone', 'footprint_bytes': None}))
        self.assertFalse(soak.is_holder({'role': 'shim', 'footprint_bytes': 900 * MIB}))

    def test_host_figures_are_parsed(self):
        self.assertEqual(soak.parse_swapusage('total = 3072.00M  used = 1887.25M  free = 1184.75M  (encrypted)'),
                         {'total': 3072.0, 'used': 1887.25, 'free': 1184.75})
        vm_stat = 'Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages occupied by compressor:   10.\n'
        self.assertEqual(soak.parse_compressed(vm_stat), 163840)
        self.assertIsNone(soak.parse_compressed('nothing'))

    def test_a_live_sample_has_the_host_and_a_process_list(self):
        taken = soak.sample_once()
        self.assertIsInstance(taken['processes'], list)
        self.assertIn('t', taken)
        self.assertIsInstance(taken['host'], dict)


class Samples(unittest.TestCase):
    def test_two_daemons_of_one_key_at_once_are_found(self):
        summary = soak.summarize_samples([
            sample(1, [proc('daemon', 1), proc('shim', 2)]),
            sample(2, [proc('daemon', 1), proc('daemon', 3), proc('shim', 2)])])
        self.assertEqual(summary['max_daemons_per_key'], 2)
        self.assertEqual(list(summary['duplicated_keys'].values()), [[1, 3]])
        self.assertEqual(len(summary['daemon_instances']), 2)

    def test_daemons_of_different_keys_and_one_after_another_are_not_duplicates(self):
        summary = soak.summarize_samples([
            sample(1, [proc('daemon', 1), proc('daemon', 2, key='cd' * 32)]),
            sample(2, [proc('daemon', 3)])])
        self.assertEqual(summary['max_daemons_per_key'], 1)
        self.assertEqual(summary['peaks']['holders'], 2)
        self.assertEqual(summary['max_daemons_per_repo'], 2)
        self.assertEqual(summary['repos_with_several_daemons'], {'/r': [1, 2]})
        other = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('daemon', 2, key='cd' * 32, repo='/s'),
                                                   proc('shim', 5, repo='/s')])])
        self.assertEqual((other['max_daemons_per_repo'], other['repos_with_several_daemons']), (1, {}))

    def test_standalone_holders_and_peaks_are_reported(self):
        summary = soak.summarize_samples([
            sample(1, [proc('standalone', 7, 200 * MIB, repo='/a'), proc('standalone', 8, MIB, repo='/b')],
                   swap_used=10.0, pressure=2, compressed=5),
            sample(2, [proc('shim', 9, MIB)], swap_used=4.0, pressure=1, compressed=9)])
        self.assertEqual(summary['new_standalone_holders'], {})
        self.assertEqual(summary['preexisting_standalone_holders'], {'/a': [7]})
        self.assertEqual(summary['peaks'], {'holders': 1, 'shims': 1, 'prism_footprint_bytes': 201 * MIB,
                                            'swap_used_mib': 10.0, 'pressure': 2, 'compressed_bytes': 9,
                                            'built_daemons': 0})



FAILED = ('prism-mcp: --shared failed closed, no index was built: no daemon answered READY before the deadline; '
          'key k; runtime dir /tmp/prism-mcpd')
CRASHED = ('prism-mcp: --shared failed closed, no index was built: the last daemon for this key, pid 9, died '
           'during its index build; the key is not restarted')
REACHED = 'prism-mcp: shared daemon pid 41 (started 1700000000, generation 0) at /tmp/prism-mcpd/x.sock'
LOST = 'prism-mcp: the shared daemon closed the connection (shared daemon pid 41); this session cannot continue'


class Events(unittest.TestCase):
    def test_client_lines_are_counted(self):
        events = soak.scan_lines([FAILED, CRASHED, REACHED, REACHED, LOST, 'nav cache: CPG cache miss'])
        self.assertEqual(len(events['fail_closed']), 2)
        self.assertEqual(events['build_crash_refusals'], 1)
        self.assertEqual(events['reached'], {(41, 1700000000)})
        self.assertEqual(events['lost'], 1)

    def test_claude_code_logs_are_read_for_prism_servers_in_the_window(self):
        with tempfile.TemporaryDirectory() as root:
            logs = Path(root) / '-Users-me-repo' / 'mcp-logs-plugin-prism-prism'
            other = Path(root) / '-Users-me-repo' / 'mcp-logs-plugin-slack'
            logs.mkdir(parents=True)
            other.mkdir(parents=True)
            entries = [{'error': 'Server stderr: %s\n' % FAILED, 'timestamp': '2026-10-16T10:00:00.000Z'},
                       {'error': 'Server stderr: %s\n' % REACHED, 'timestamp': '2026-10-16T12:00:00.000Z'},
                       {'debug': 'Starting connection', 'timestamp': '2026-10-16T10:00:00.000Z'}]
            (logs / 'a.jsonl').write_text('\n'.join(json.dumps(e) for e in entries) + '\nnot json\n')
            (other / 'a.jsonl').write_text(json.dumps(entries[0]) + '\n')
            since = soak.iso_epoch('2026-10-16T09:00:00')
            until = soak.iso_epoch('2026-10-16T11:00:00')
            self.assertEqual(list(soak.claude_log_lines(root, since, until)), [FAILED])
            self.assertEqual(len(list(soak.claude_log_lines(root))), 2)

    def test_iso_times_are_utc_in_any_local_zone_winter_and_summer(self):
        # Review round 1 (sol S7, Opus S7): run under explicit zones, so that local time read as UTC
        # fails everywhere, and in July, where `mktime(...) - time.timezone` is an hour out.
        stamps = {'2026-01-15T12:00:00Z': calendar.timegm((2026, 1, 15, 12, 0, 0)),
                  '2026-07-15T12:00:00Z': calendar.timegm((2026, 7, 15, 12, 0, 0))}
        code = 'import sys, soak; print(soak.iso_epoch(sys.argv[1]))'
        for zone in ('UTC0', 'MST7MDT,M3.2.0,M11.1.0'):
            for stamp, expected in stamps.items():
                done = subprocess.run([sys.executable, '-c', code, stamp], cwd=str(HERE), capture_output=True,
                                      text=True, env=dict(os.environ, TZ=zone))
                self.assertEqual(done.stdout.strip(), str(expected), (zone, stamp, done.stderr))

    def test_crash_reports_of_prism_mcp_in_the_window_are_counted(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ('prism-mcp-2026-10-16-101010.ips', 'prism-mcp_2026-10-16-101011_host.crash',
                         'Safari-2026-10-16-101010.ips'):
                (Path(directory) / name).write_text('x')
            old = Path(directory) / 'prism-mcp-2026-01-01-000000.ips'
            old.write_text('x')
            os.utime(old, (100, 100))
            self.assertEqual(soak.crash_reports(directory, since=1000),
                             (['prism-mcp-2026-10-16-101010.ips', 'prism-mcp_2026-10-16-101011_host.crash'], True))
            self.assertEqual(soak.crash_reports(str(Path(directory) / 'missing')), ([], False))


class Report(unittest.TestCase):
    def test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach(self):
        def failing(summary, events, crashes=()):
            return [r for r, v in soak.judge(summary, events, list(crashes), ALL_READ).items() if v != 'PASS']
        clean = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('shim', 2)])])
        quiet = soak.scan_lines([REACHED])
        self.assertEqual(failing(clean, quiet), [])
        two = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('daemon', 2)])])
        self.assertEqual(failing(two, quiet), ['one_index_per_key'])
        alone = soak.summarize_samples([sample(1, []), sample(2, [proc('standalone', 3, 500 * MIB)])])
        self.assertEqual(failing(alone, quiet), ['no_new_standalone_index'])
        self.assertEqual(failing(clean, soak.scan_lines([FAILED])), ['no_fail_closed'])
        self.assertEqual(failing(clean, quiet, ['prism-mcp-x.ips']), ['no_abort'])
        critical = soak.summarize_samples([sample(1, [], pressure=4)])
        self.assertEqual(failing(critical, quiet), ['no_critical_pressure'])

    def test_report_writes_the_table_and_the_verdicts(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            samples.write_text(json.dumps(sample(time.time(), [proc('daemon', 1)])) + '\n')
            client = Path(directory) / 'client.log'
            client.write_text('%s 77 %s\n' % (time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), FAILED))
            out, data = Path(directory) / 'r.md', Path(directory) / 'r.json'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'week', '--samples', str(samples), '--client-log', str(client),
                           '--out', str(out), '--json', str(data), '--crash-reports', directory])
            self.assertIn('| no_fail_closed | FAIL |', out.read_text())
            self.assertIn('**Verdict: FAIL**', out.read_text())
            self.assertEqual(json.loads(data.read_text())['verdicts']['one_index_per_key'], 'PASS')

    def test_sample_appends_one_line_per_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 's.jsonl'
            with redirect_stdout(io.StringIO()):
                soak.main(['sample', '--out', str(out), '--count', '2', '--interval', '0'])
                soak.main(['sample', '--out', str(out), '--count', '1', '--interval', '0'])
            self.assertEqual(soak.load_samples(out), (soak.load_samples(out)[0], 0))
            self.assertEqual(len(soak.load_samples(out)[0]), 3)


class Wrapper(unittest.TestCase):
    def test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'stderr.log'
            code = ('import sys; sys.stdout.write(sys.stdin.read()); sys.stderr.write("one\\ntwo"); '
                    'sys.exit(3)')
            child = subprocess.Popen([str(HERE / 'prism-mcp-logged.sh'), str(log), sys.executable, '-c', code],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            out, err = child.communicate(b'{"jsonrpc":"2.0"}\n')
            self.assertEqual((child.returncode, out, err), (3, b'{"jsonrpc":"2.0"}\n', b'one\ntwo\n'))
            lines = log.read_text().splitlines()
            self.assertEqual([soak.CLIENT_LINE.match(line).group(3) for line in lines], ['one', 'two'])
            self.assertEqual({soak.CLIENT_LINE.match(line).group(2) for line in lines}, {str(child.pid)})
            self.assertEqual(list(soak.client_log_lines(log)), ['one', 'two'])
            self.assertTrue(all(re.match(r'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ ', line) for line in lines))


if __name__ == '__main__':
    unittest.main()
````

Create `scripts/soak/test_soak_coverage.py`:

````python
"""Tests of what soak.py does when it could not observe something, and of the use a phase needs
(review round 1: sol 2, Opus W2, W3, S2, S3, S5). A rule that could not be judged is UNKNOWN, never
PASS; a phase that saw no use is INADMISSIBLE.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402
from test_soak import ALL_READ, FAILED, MIB, REACHED, proc, sample  # noqa: E402

CLAP = "error: unexpected argument '--shared' found"
CRASHED = 'prism-mcp: --shared failed closed, no index was built: the last daemon for this key died during its index build'


def dead_pid():
    child = subprocess.Popen(['true'])
    child.wait()
    return child.pid


def verdicts(samples, lines=(), crashes=(), coverage=None):
    summary = soak.summarize_samples(samples)
    return soak.judge(summary, soak.scan_lines(list(lines)), list(crashes), coverage or ALL_READ)


class Unobserved(unittest.TestCase):
    def test_a_failed_process_table_makes_the_process_rules_unknown(self):
        found = verdicts([sample(1, [], ps_ok=False)], [REACHED])
        self.assertEqual((found['one_index_per_key'], found['no_new_standalone_index']), ('UNKNOWN', 'UNKNOWN'))

    def test_a_failed_footprint_of_a_standalone_server_is_unknown_and_a_vanished_one_is_not(self):
        failed = verdicts([sample(1, []), sample(2, [proc('standalone', 7, status='failed')])], [REACHED])
        self.assertEqual(failed['no_new_standalone_index'], 'UNKNOWN')
        gone = verdicts([sample(1, []), sample(2, [proc('standalone', 7, status='gone')])], [REACHED])
        self.assertEqual(gone['no_new_standalone_index'], 'PASS')

    def test_a_missing_pressure_reading_is_unknown_and_a_critical_one_still_fails(self):
        self.assertEqual(verdicts([sample(1, [], pressure=None)], [REACHED])['no_critical_pressure'], 'UNKNOWN')
        both = verdicts([sample(1, [], pressure=None), sample(2, [], pressure=4)], [REACHED])
        self.assertEqual(both['no_critical_pressure'], 'FAIL')

    def test_unread_logs_or_crash_reports_are_unknown_and_what_was_read_still_fails(self):
        unread = verdicts([sample(1, [])], [], coverage={'logs': False, 'crash_reports': False})
        for rule in ('no_fail_closed', 'no_startup_error', 'no_restart_bound', 'no_abort'):
            self.assertEqual(unread[rule], 'UNKNOWN', rule)
        partial = verdicts([sample(1, [])], [FAILED], ['prism-mcp-1.ips'], {'logs': False, 'crash_reports': False})
        self.assertEqual((partial['no_fail_closed'], partial['no_abort']), ('FAIL', 'FAIL'))

    def test_a_vanished_pid_is_gone_and_a_live_one_without_a_reading_failed(self):
        saved = (soak.FOOTPRINT_COMMAND, soak.PROC)
        soak.FOOTPRINT_COMMAND, soak.PROC = 'no-such-footprint-command', Path('/no/such/proc')
        try:
            sizes, statuses = soak.measure_footprints([os.getpid(), dead_pid()])
        finally:
            soak.FOOTPRINT_COMMAND, soak.PROC = saved
        self.assertEqual(sizes, {})
        self.assertEqual(sorted(statuses.values()), ['failed', 'gone'])
        self.assertEqual(statuses[os.getpid()], 'failed')

    def test_a_sample_whose_process_table_could_not_be_read_says_so(self):
        saved = soak.PS_COMMAND
        soak.PS_COMMAND = ['no-such-ps-command']
        try:
            taken = soak.sample_once()
        finally:
            soak.PS_COMMAND = saved
        self.assertEqual((taken['ps_ok'], taken['processes']), (False, []))

    def test_log_sources_are_complete_only_when_every_named_one_exists(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'bridge.log'
            log.write_text('')
            self.assertTrue(soak.logs_readable(directory, [str(log)]))
            self.assertFalse(soak.logs_readable(str(Path(directory) / 'missing'), [str(log)]))
            self.assertFalse(soak.logs_readable(directory, [str(Path(directory) / 'missing.log')]))
            self.assertFalse(soak.logs_readable(None, []))


class Use(unittest.TestCase):
    def built(self, t):
        return sample(t, [proc('daemon', 1, 300 * MIB), proc('shim', 2), proc('shim', 3)])

    def test_a_round_needs_shims_a_reached_daemon_and_a_built_index(self):
        def phase(samples, lines):
            summary = soak.summarize_samples(samples)
            events = soak.scan_lines(lines)
            use = soak.usage('round', summary, events, min_shims=2, min_days=5, sources_reached=[bool(events['reached'])])
            return soak.overall('round', soak.judge(summary, events, [], ALL_READ), use)
        self.assertEqual(phase([self.built(1)], [REACHED]), 'PASS')
        self.assertEqual(phase([self.built(1), sample(2, [], ps_ok=False)], [REACHED]), 'INADMISSIBLE')
        self.assertEqual(phase([sample(1, [])], []), 'INADMISSIBLE')
        self.assertEqual(phase([self.built(1)], []), 'INADMISSIBLE')
        idle = sample(1, [proc('daemon', 1, 3 * MIB), proc('shim', 2), proc('shim', 3)])
        self.assertEqual(phase([idle], [REACHED]), 'INADMISSIBLE')
        self.assertEqual(phase([self.built(1), sample(2, [proc('daemon', 1), proc('daemon', 5)])], [REACHED]),
                         'FAIL')

    def test_standalone_holders_already_running_at_the_start_do_not_fail_a_phase(self):
        before = proc('standalone', 7, 500 * MIB, repo='/old')
        summary = soak.summarize_samples([sample(1, [before]), sample(2, [before, proc('standalone', 8, 500 * MIB)])])
        self.assertEqual(summary['preexisting_standalone_holders'], {'/old': [7]})
        self.assertEqual(summary['new_standalone_holders'], {'/r': [8]})

    def test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident(self):
        # Review round 2 (Opus W-B): one call on day 1, then a window left open all week, is one day.
        day = 86400
        resident = [proc('daemon', 1, 400 * MIB, cpu=3.2), proc('shim', 2), proc('shim', 3)]
        samples = [sample(0, [proc('daemon', 1, 3 * MIB, cpu=0.2), proc('shim', 2)]),
                   sample(600, [proc('daemon', 1, 400 * MIB, cpu=3.2), proc('shim', 2)])]
        samples += [sample(d * day + 600 * k, resident) for d in range(1, 7) for k in range(3)]
        self.assertEqual(soak.summarize_samples(samples)['usage_days'], ['1970-01-01'])
        quiet = soak.summarize_samples([sample(0, [proc('daemon', 1, 400 * MIB, cpu=50.0), proc('shim', 2)]),
                                        sample(600, [proc('daemon', 1, 400 * MIB, cpu=50.1), proc('shim', 2)])])
        self.assertEqual(quiet['usage_days'], [])
        new = soak.summarize_samples([sample(0, []), sample(day, [proc('daemon', 5, 400 * MIB, cpu=4.0)])])
        self.assertEqual(new['usage_days'], ['1970-01-02'])

    def test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules(self):
        found = verdicts([sample(1, [])], [CLAP, CRASHED])
        self.assertEqual((found['no_startup_error'], found['no_restart_bound']), ('FAIL', 'FAIL'))
        launcher = verdicts([sample(1, [])], ["prism-mcp-launch.sh: PRISM_MCP_SHARED must be 1 (…), not 'yes'"])
        self.assertEqual(launcher['no_startup_error'], 'FAIL')


class Edges(unittest.TestCase):
    def test_a_binary_path_with_a_space_is_still_prism_mcp(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / 'a b' / 'prism-mcp'
            binary.parent.mkdir()
            binary.write_text('#!/bin/sh\n')
            binary.chmod(0o755)
            found = soak.classify('%s --repo /r --shared' % binary)
            self.assertEqual((found['role'], found['repo']), ('shim', '/r'))
            self.assertIsNone(soak.classify('/usr/bin/vim %s' % binary))
        self.assertIsNone(soak.classify('/usr/bin/vim /r/prism-mcp'))

    def test_a_truncated_last_line_is_skipped_and_counted(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 's.jsonl'
            path.write_text(json.dumps(sample(1, [])) + '\n' + '{"t": 2, "proc')
            samples, skipped = soak.load_samples(path)
            self.assertEqual((len(samples), skipped), (1, 1))

    def test_a_round_report_without_its_log_sources_is_inadmissible(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            built = sample(time.time(), [proc('daemon', 1, 300 * MIB), proc('shim', 2), proc('shim', 3)])
            samples.write_text(json.dumps(built) + '\n')
            out = Path(directory) / 'r.md'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'round', '--round-repo', '/r', '--samples', str(samples), '--claude-logs',
                           str(Path(directory) / 'no-such-root'), '--crash-reports', directory, '--out', str(out)])
            self.assertIn('**Verdict: INADMISSIBLE**', out.read_text())
            self.assertIn('| no_fail_closed | UNKNOWN |', out.read_text())


class Probe(unittest.TestCase):
    def probe(self, command, timeout='20'):
        out = io.StringIO()
        with redirect_stdout(out):
            code = soak.main(['probe', '--timeout', timeout, '--'] + command)
        return code, out.getvalue()

    def test_a_server_that_answers_initialize_passes(self):
        code, text = self.probe([str(HERE / 'fake_prism_mcp.py'), '--repo', '/r', '--shared'])
        self.assertEqual(code, 0, text)
        self.assertIn('PASS', text)

    def test_a_binary_that_rejects_shared_fails_and_says_why(self):
        script = 'import sys; sys.stderr.write("%s\\n"); sys.exit(2)' % CLAP
        code, text = self.probe([sys.executable, '-c', script])
        self.assertEqual(code, 1)
        self.assertIn('FAIL', text)
        self.assertIn(CLAP, text)

    def test_a_server_that_never_answers_fails_at_the_timeout(self):
        code, text = self.probe([sys.executable, '-c', 'import time; time.sleep(60)'], timeout='1')
        self.assertEqual(code, 1)
        self.assertIn('no answer to initialize within 1', text)


if __name__ == '__main__':
    unittest.main()
````

Create `scripts/soak/test_soak_round2.py`:

````python
"""Tests of review round 2's fold of soak.py (Opus W-D, W-E = sol 1, S-A, S-F): the week's window
survives a resume, a round gates only on its own checkouts, every client log must show a reached
daemon, and the report says how much time the samples covered.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402
from test_soak import ALL_READ, FAILED, MIB, REACHED, proc, sample  # noqa: E402

DAY = 86400


def stamp(epoch):
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(epoch))


class Window(unittest.TestCase):
    def test_a_resumed_sampler_keeps_the_window_it_started(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'T0'
            self.assertEqual(soak.open_window(path, 7 * DAY, now=1000.0), (1000, 1000 + 7 * DAY, False))
            self.assertEqual(soak.open_window(path, 7 * DAY, now=1000.0 + 3 * DAY), (1000, 1000 + 7 * DAY, True))
            self.assertEqual(path.read_text().strip(), '1000')

    def test_a_day_two_failure_and_earlier_usage_days_survive_a_resume(self):
        with tempfile.TemporaryDirectory() as directory:
            window = Path(directory) / 'T0'
            soak.open_window(window, 7 * DAY, now=0.0)
            soak.open_window(window, 7 * DAY, now=3 * DAY + 5.0)   # the sampler restarted on day 4
            samples = Path(directory) / 's.jsonl'
            lines = []
            for day in range(7):
                for k, cpu in enumerate((10.0 * day, 10.0 * day + 5.0)):
                    lines.append(json.dumps(sample(day * DAY + 600 * k, [proc('daemon', 1, 400 * MIB, cpu=cpu)])))
            samples.write_text('\n'.join(lines) + '\n')
            client = Path(directory) / 'bridge.log'
            client.write_text('%s 7 %s\n' % (stamp(1 * DAY + 60), FAILED))
            out, data = Path(directory) / 'r.md', Path(directory) / 'r.json'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'week', '--samples', str(samples), '--window-file', str(window),
                           '--until', str(7 * DAY), '--client-log', str(client), '--crash-reports', directory,
                           '--out', str(out), '--json', str(data)])
            report = json.loads(data.read_text())
            self.assertEqual(report['verdicts']['no_fail_closed'], 'FAIL')
            self.assertEqual(len(report['summary']['usage_days']), 7)


class Round(unittest.TestCase):
    def built(self, t, *more):
        return sample(t, [proc('daemon', 1, 300 * MIB, cpu=9.0), proc('shim', 2), proc('shim', 3)] + list(more))

    def test_only_standalone_servers_of_the_round_s_checkouts_are_gated(self):
        with tempfile.TemporaryDirectory() as directory:
            checkout = os.path.join(directory, 'checkout')
            os.mkdir(checkout)
            alias = os.path.join(directory, 'alias')
            os.symlink(checkout, alias)
            samples = [self.built(1), self.built(2, proc('standalone', 9, 500 * MIB, repo='/elsewhere'))]
            summary = soak.summarize_samples(samples)
            events = soak.scan_lines([REACHED])
            elsewhere = soak.judge(summary, events, [], ALL_READ, round_repos=[checkout])
            self.assertEqual(elsewhere['no_new_standalone_index'], 'PASS')
            inside = soak.summarize_samples([self.built(1), self.built(2, proc('standalone', 9, 500 * MIB, repo=alias))])
            self.assertEqual(soak.judge(inside, events, [], ALL_READ, round_repos=[checkout])['no_new_standalone_index'],
                             'FAIL')

    def test_a_round_needs_a_reached_daemon_in_every_client_log(self):
        summary = soak.summarize_samples([self.built(1)])
        events = soak.scan_lines([REACHED])
        verdicts = soak.judge(summary, events, [], ALL_READ, round_repos=['/r'])
        use = soak.usage('round', summary, events, 2, 5, sources_reached=[True, False])
        self.assertEqual(soak.overall('round', verdicts, use), 'INADMISSIBLE')
        use = soak.usage('round', summary, events, 2, 5, sources_reached=[True, True])
        self.assertEqual(soak.overall('round', verdicts, use), 'PASS')
        self.assertEqual(soak.overall('round', verdicts, soak.usage('round', summary, events, 2, 5, sources_reached=[])),
                         'INADMISSIBLE')

    def test_a_round_report_without_its_checkouts_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            samples.write_text(json.dumps(self.built(time.time())) + '\n')
            with redirect_stdout(io.StringIO()), self.assertRaises(SystemExit):
                soak.main(['report', '--phase', 'round', '--samples', str(samples), '--out',
                           str(Path(directory) / 'r.md')])


class Coverage(unittest.TestCase):
    def test_the_largest_gap_between_samples_and_the_time_covered_are_reported(self):
        summary = soak.summarize_samples([sample(0, []), sample(60, []), sample(3 * DAY, []), sample(3 * DAY + 60, [])])
        self.assertEqual((summary['largest_gap_secs'], summary['covered_secs']), (3 * DAY - 60, 3 * DAY + 60))


if __name__ == '__main__':
    unittest.main()
````

- [ ] **Step 2: Run them and watch them fail**

```bash
python3 -m unittest discover -s scripts/soak -p 'test_soak*.py' > "$LOG_DIR/t2-red.log" 2>&1; echo "rc=$?"
tail -5 "$LOG_DIR/t2-red.log"
```

Expected: `ModuleNotFoundError: No module named 'soak'` for `test_soak`, `test_soak_coverage` and `test_soak_round2`, `Ran 3 tests`, `FAILED (errors=3)`.

- [ ] **Step 3: Implement the sampler, the report and the wrapper**

In `.github/workflows/ci.yml`, replace this:

````
      - name: Run doctests
        run: cargo test --doc --features mcp

  clippy:
    name: Clippy Lint
    needs: changes
````

with this:

````
      - name: Run doctests
        run: cargo test --doc --features mcp

      # The soak instrumentation's hermetic tests (Python standard library, a few seconds; the live
      # fan-out test skips itself without PRISM_SOAK_BIN). scripts/soak/ imports scripts/memreport/.
      - name: Soak tool tests (Python)
        run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'

  clippy:
    name: Clippy Lint
    needs: changes
````

Create `scripts/soak/prism-mcp-logged.sh`:

````bash
#!/usr/bin/env bash
#
# Soak instrumentation (scripts/soak/soak.py): run prism-mcp with its standard error also appended
# to LOGFILE, one line per line, each prefixed with the UTC time and the pid of the prism-mcp
# process. Standard input and output (the MCP stream) are untouched. Clients that keep no log of an
# MCP server's standard error (Codex, the a2a-bridge) are pointed at this wrapper during a soak;
# Claude Code already logs it.
#
# Usage: prism-mcp-logged.sh LOGFILE BIN [prism-mcp args...]
set -euo pipefail

log="${1:?usage: prism-mcp-logged.sh LOGFILE BIN [args...]}"
bin="${2:?usage: prism-mcp-logged.sh LOGFILE BIN [args...]}"
shift 2
pid=$$
exec "$bin" "$@" 2> >(
  while IFS= read -r line || [ -n "$line" ]; do
    printf '%s %s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$pid" "$line" >>"$log"
    printf '%s\n' "$line" >&2
  done
)
````

Make `scripts/soak/prism-mcp-logged.sh` executable.

Create `scripts/soak/soak.py`:

````python
#!/usr/bin/env python3
"""Soak instrumentation for the shared prism-mcp daemon (spec section 8 step 2).

    soak.py sample --out FILE.jsonl [--interval 60] [--count N | --duration SECS [--window-file T0]]
                   [--status-bin BIN --runtime-dir BASE]
    soak.py report --phase round|week --samples FILE.jsonl --out FILE.md [--json FILE.json]
                   [--since EPOCH | --window-file T0] [--until EPOCH] [--claude-logs DIR]
                   [--client-log FILE ...] [--crash-reports DIR] [--round-repo DIR ...]
                   [--min-shims 2] [--min-days 5]
    soak.py probe [--timeout 30] -- COMMAND [ARGS...]

`sample` appends one JSON line per interval: every prism-mcp process (its role, repository, key,
resident size, CPU time, and footprint with whether it was measured, had gone, or could not be measured),
whether the process table could be read, and the host's swap, memory pressure and compressor. It
reads the process table and never connects to a daemon unless --status-bin is given, in which case
it also stores the text of `daemon-status` (a daemon built before S5b restarts its idle time at
each such probe: SI-35). Appending makes it resumable: run it again on the same file. With
--window-file the window starts once, when the file is first written, and a resumed sampler runs
to the end of that window, not for a new --duration (review round 2, sol 1, Opus W-E).

`report` (soak_judge.py) reads the samples, the clients' standard error (Claude Code's MCP logs,
the files prism-mcp-logged.sh writes) and macOS crash reports, and judges the soak's rules; a rule
whose evidence was not collected is UNKNOWN, and a phase that saw no use is INADMISSIBLE. A usage
day needs at least 0.05 s of one daemon's CPU growth with CPU readings at both consecutive
endpoints, at most 120 s apart. Longer intervals earn no usage credit and are counted in the report.

`probe` starts a launcher's exact command line, sends `initialize`, and passes only when an answer
comes back: the check, at deployment and at rollback, that a configuration starts a server with
the binary it names (a binary that rejects --shared fails it, with its error shown).

Python 3.9, standard library only. Unit tests:
    python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import argparse
import json
import os
import queue
import re
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

from soak_judge import (BUILD_CRASH, CLIENT_LINE, CRASH_REPORT, FAIL_CLOSED, GATED, HOLDER_MIN_BYTES,  # noqa: F401
                        LOST, MIB, PRESSURE_CRITICAL, REACHED, RULES, STARTUP_ERROR, built, claude_log_lines,
                        client_log_lines, cmd_report, crash_reports, holder_identity, in_window, is_holder,
                        iso_epoch, judge, load_samples, logs_readable, overall, render, scan_lines,
                        summarize_samples, usage, usage_days)

PS_COMMAND = ['ps', '-A', '-ww', '-o', 'pid=,ppid=,rss=,time=,command=']
FOOTPRINT_COMMAND = 'footprint'
PROC = Path('/proc')
BINARY = re.compile(r'^(.*?(?:^|/)prism-mcp)(?: (.*))?$')


def capture(command, timeout=30):
    """The stdout of a command as text, or None when it could not run or exited non-zero."""
    try:
        done = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return done.stdout.decode('utf-8', 'replace') if done.returncode == 0 else None


def option(tokens, name):
    """The word after `name` in `tokens`, or None."""
    for index, token in enumerate(tokens[:-1]):
        if token == name:
            return tokens[index + 1]
    return None


def split_command(command):
    """(binary, the other words) when the command runs a binary named prism-mcp, else None. The
    binary is the first word, or, when its path has a space in it (review round 1, Opus S5), the
    shortest prefix ending in `/prism-mcp` that is an executable file."""
    words = command.split()
    if words and os.path.basename(words[0]) == 'prism-mcp':
        return words[0], words[1:]
    found = BINARY.match(command)
    if found and ' ' in found.group(1) and os.path.isfile(found.group(1)) and os.access(found.group(1), os.X_OK):
        return found.group(1), (found.group(2) or '').split()
    return None


def classify(command):
    """Role and configuration of one process from its command line, or None for a process that is
    not prism-mcp. Roles: daemon, shim, standalone, operator. Arguments are split on spaces, so a
    repository path with a space in it is cut short in the labels; roles do not depend on it."""
    split = split_command(command)
    if split is None:
        return None
    tokens = split[1]
    verb = tokens[0] if tokens else ''
    if verb == 'daemon':
        role = 'daemon'
    elif verb in ('daemon-status', 'daemon-shutdown'):
        role = 'operator'
    elif '--shared' in tokens:
        role = 'shim'
    else:
        role = 'standalone'
    return {'role': role, 'repo': option(tokens, '--repo'), 'key': option(tokens, '--key'),
            'cache_dir': option(tokens, '--cache-dir')}


def parse_cputime(text):
    """Seconds of `ps -o time=`: `[DD-]HH:MM:SS` (procps) or `MM:SS.ss` / `HH:MM:SS.ss` (macOS);
    None when it is neither."""
    days, _, clock = text.rpartition('-')
    try:
        parts = [float(part) for part in clock.split(':')]
        whole = int(days) if days else 0
    except ValueError:
        return None
    if not 2 <= len(parts) <= 3:
        return None
    seconds = 0.0
    for part in parts:
        seconds = seconds * 60 + part
    return whole * 86400 + seconds


def parse_ps(text):
    """prism-mcp processes in `ps -A -ww -o pid=,ppid=,rss=,time=,command=` output."""
    processes = []
    for line in text.splitlines():
        fields = line.split(None, 4)
        if len(fields) < 5 or not fields[0].isdigit():
            continue
        found = classify(fields[4])
        if found is not None:
            found.update(pid=int(fields[0]), ppid=int(fields[1]), rss_bytes=int(fields[2]) * 1024,
                         cpu_secs=parse_cputime(fields[3]), command=fields[4])
            processes.append(found)
    return processes


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def measure_footprints(pids):
    """({pid: bytes}, {pid: 'ok' | 'gone' | 'failed'}): phys_footprint (macOS `footprint`), or
    resident plus swapped (Linux /proc). A pid without a reading is 'gone' when the process has
    ended and 'failed' while it still runs (review round 1, sol 2)."""
    sizes = {}
    if pids and sys.platform == 'darwin':
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'footprint.json'
            capture([FOOTPRINT_COMMAND, '-f', 'bytes', '--noCategories', '-j', str(report)] + [str(p) for p in pids])
            try:
                data = json.loads(report.read_text())
            except (OSError, ValueError):
                data = {}
        sizes = {entry['pid']: entry['auxiliary']['phys_footprint'] for entry in data.get('processes', [])
                 if entry.get('pid') in pids and 'phys_footprint' in entry.get('auxiliary', {})}
    elif pids:
        for pid in pids:
            try:
                status = (PROC / str(pid) / 'status').read_text()
            except OSError:
                continue
            kib = [int(line.split()[1]) for line in status.splitlines() if line.startswith(('VmRSS:', 'VmSwap:'))]
            sizes[pid] = sum(kib) * 1024
    statuses = {pid: 'ok' if pid in sizes else 'failed' if alive(pid) else 'gone' for pid in pids}
    return sizes, statuses


def footprints(pids):
    """phys_footprint in bytes by pid, for the pids that could be measured."""
    return measure_footprints(pids)[0]


def parse_swapusage(text):
    """MiB figures of `sysctl -n vm.swapusage`: total, used, free."""
    found = dict(re.findall(r'(total|used|free) = ([0-9.]+)M', text or ''))
    return {name: float(value) for name, value in found.items()}


def parse_compressed(vm_stat):
    """Bytes held by the compressor, from `vm_stat`; None when it does not say."""
    size = re.search(r'page size of (\d+) bytes', vm_stat or '')
    pages = re.search(r'Pages occupied by compressor:\s+(\d+)', vm_stat or '')
    return int(size.group(1)) * int(pages.group(1)) if size and pages else None


def read_pressure():
    """kern.memorystatus_vm_pressure_level (1 normal, 2 warning, 4 critical), or None when it
    cannot be read (not macOS, or the sysctl failed)."""
    if sys.platform != 'darwin':
        return None
    level = capture(['sysctl', '-n', 'kern.memorystatus_vm_pressure_level'])
    return int(level) if level and level.strip().isdigit() else None


def host_memory():
    """Swap, pressure and compressor; a figure that could not be read is None (or {} for swap)."""
    if sys.platform == 'darwin':
        return {'swap_mib': parse_swapusage(capture(['sysctl', '-n', 'vm.swapusage'])),
                'pressure': read_pressure(), 'compressed_bytes': parse_compressed(capture(['vm_stat']))}
    try:
        info = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines() if ':' in line)
    except OSError:
        info = {}
    kib = {name: int(info[name].split()[0]) for name in ('SwapTotal', 'SwapFree') if name in info}
    swap = ({'total': kib['SwapTotal'] / 1024, 'free': kib['SwapFree'] / 1024,
             'used': (kib['SwapTotal'] - kib['SwapFree']) / 1024} if len(kib) == 2 else {})
    return {'swap_mib': swap, 'pressure': None, 'compressed_bytes': None}


def sample_once(status_bin=None, runtime_dir=None):
    listing = capture(PS_COMMAND)
    processes = parse_ps(listing or '')
    sizes, statuses = measure_footprints([process['pid'] for process in processes])
    for process in processes:
        process['footprint_bytes'] = sizes.get(process['pid'])
        process['footprint_status'] = statuses[process['pid']]
        process['holder'] = is_holder(process)
    sample = {'t': time.time(), 'ps_ok': listing is not None, 'host': host_memory(), 'processes': processes}
    if status_bin:
        command = [status_bin, 'daemon-status'] + (['--runtime-dir', runtime_dir] if runtime_dir else [])
        sample['daemon_status'] = capture(command, timeout=120)
    return sample


def open_window(path, duration, now=None):
    """(start, end, resumed) of a sampling window kept in `path`: the start is written once, the
    first time, and every later call keeps it, so a resumed sampler and the report agree on the
    window whatever restarts came between (review round 2, sol 1, Opus W-E)."""
    now = time.time() if now is None else now
    path = Path(path)
    text = path.read_text().strip() if path.exists() else ''
    resumed = text.isdigit()
    if not resumed:
        path.write_text('%d\n' % int(now))
    start = int(text) if resumed else int(now)
    return start, (start + duration if duration else None), resumed


def cmd_sample(args):
    end = time.time() + args.duration if args.duration else None
    if args.window_file:
        start, end, resumed = open_window(args.window_file, args.duration)
        print('sampler %s at %d: window from %d, %s s left' % (
            'resumed' if resumed else 'started', time.time(), start,
            'unbounded' if end is None else '%d' % max(0, end - time.time())), flush=True)
    taken = 0
    while True:
        sample = sample_once(args.status_bin, args.runtime_dir)
        with open(args.out, 'a') as out:
            out.write(json.dumps(sample, sort_keys=True) + '\n')
        taken += 1
        if (args.count and taken >= args.count) or (end is not None and time.time() + args.interval > end):
            break
        time.sleep(args.interval)
    print('%d samples appended to %s' % (taken, args.out))
    return 0


def cmd_probe(args):
    """Start the command, send `initialize`, wait for an answer; PASS (0) or FAIL (1) with why."""
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    with tempfile.TemporaryFile() as stderr:
        try:
            child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
        except OSError as error:
            print('FAIL: cannot start %s: %s' % (command[0], error))
            return 1
        answers = queue.Queue()
        threading.Thread(target=lambda: answers.put(child.stdout.readline()), daemon=True).start()
        hello = {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {
            'protocolVersion': '2025-11-25', 'capabilities': {}, 'clientInfo': {'name': 'probe', 'version': '1'}}}
        try:
            child.stdin.write((json.dumps(hello) + '\n').encode())
            child.stdin.flush()
        except OSError:
            pass
        try:
            line = answers.get(timeout=args.timeout)
        except queue.Empty:
            line = None
        if line is None:
            child.kill()
        try:
            child.stdin.close()
        except OSError:
            pass
        try:
            status = child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            status = child.wait()
        child.stdout.close()
        stderr.seek(0)
        err = stderr.read().decode('utf-8', 'replace').strip()
    try:
        answered = line is not None and 'result' in json.loads(line)
    except ValueError:
        answered = False
    if answered:
        print('PASS: %s answered initialize' % command[0])
        return 0
    why = 'no answer to initialize within %ss' % args.timeout if line is None else 'it ended (exit %s)' % status
    print('FAIL: %s: %s; standard error: %s' % (command[0], why, err[-2000:] or '(empty)'))
    return 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    sample = commands.add_parser('sample')
    sample.add_argument('--out', required=True)
    sample.add_argument('--interval', type=float, default=60.0)
    sample.add_argument('--count', type=int)
    sample.add_argument('--duration', type=float, help='seconds; with neither this nor --count, forever')
    sample.add_argument('--window-file', help='keeps the window start: --duration counts from it, across resumes')
    sample.add_argument('--status-bin', help='a prism-mcp binary whose daemon-status text each sample stores')
    sample.add_argument('--runtime-dir', help='the runtime base daemon-status is given')
    sample.set_defaults(handler=cmd_sample)
    report = commands.add_parser('report')
    report.add_argument('--phase', required=True, choices=sorted(GATED))
    report.add_argument('--samples', required=True)
    report.add_argument('--out', required=True)
    report.add_argument('--json')
    report.add_argument('--since', type=float)
    report.add_argument('--window-file', help='the start of the window, as sample --window-file wrote it')
    report.add_argument('--until', type=float)
    report.add_argument('--claude-logs', help='Claude Code MCP log root (~/Library/Caches/claude-cli-nodejs)')
    report.add_argument('--client-log', action='append', help='a file prism-mcp-logged.sh wrote')
    report.add_argument('--crash-reports', default=str(Path.home() / 'Library/Logs/DiagnosticReports'))
    report.add_argument('--round-repo', action='append',
                        help='round: a checkout the round ran in; standalone servers elsewhere are not gated')
    report.add_argument('--min-shims', type=int, default=2, help='round: shims at once that show use')
    report.add_argument('--min-days', type=int, default=5, help='week: usage days that show use')
    report.set_defaults(handler=cmd_report)
    probe = commands.add_parser('probe')
    probe.add_argument('--timeout', type=float, default=30.0)
    probe.add_argument('command', nargs=argparse.REMAINDER)
    probe.set_defaults(handler=cmd_probe)
    args = parser.parse_args(argv)
    if args.command == 'report':
        if args.phase == 'round' and not args.round_repo:
            parser.error('report --phase round needs --round-repo for each checkout the round ran in')
        if args.window_file:
            if args.since is not None:
                parser.error('--since and --window-file are one or the other')
            args.since = float(Path(args.window_file).read_text().strip())
    return args.handler(args)


if __name__ == '__main__':
    sys.exit(main())
````

Create `scripts/soak/soak_judge.py`:

````python
"""The soak report's reading and judging (soak.py report; spec section 8 step 2).

A rule is PASS, FAIL or UNKNOWN. UNKNOWN means the evidence the rule needs was not collected (the
process table could not be read, a live process's footprint could not be measured, the pressure
level was unavailable, a named log source or the crash-report directory was missing): a rule never
passes on what it could not see (review round 1, sol 2). A phase also needs evidence of use; a
phase whose rules all pass but that saw no use is INADMISSIBLE (Opus W2, S3). A usage day needs
at least 0.05 s of one daemon's CPU growth between observed consecutive endpoints at most 120 s
apart; intervals over 120 s earn no usage credit and their excluded count is reported.
"""
import calendar
import json
import os
import re
import time
from pathlib import Path

MIB = 1024 * 1024
# A standalone prism-mcp that has not built an index is a few MiB; one that has holds hundreds.
HOLDER_MIN_BYTES = 100 * MIB
# kern.memorystatus_vm_pressure_level: 1 normal, 2 warning, 4 critical.
PRESSURE_CRITICAL = 4
FAIL_CLOSED = 'prism-mcp: --shared failed closed'
REACHED = re.compile(r'prism-mcp: shared daemon pid (\d+) \(started (\d+),')
LOST = re.compile(r'prism-mcp: (the shared daemon closed the connection|the shared daemon\'s connection '
                  r'failed)')
BUILD_CRASH = 'died during its index build'
# A server that never started: clap's refusal (an old binary given --shared), the launcher's own
# refusals (review round 1, Opus S2).
STARTUP_ERROR = re.compile(r'^(error: |prism-mcp-launch\.sh: |prism-mcp not found)')
CLIENT_LINE = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ) (\d+) (.*)')
CRASH_REPORT = re.compile(r'prism-mcp[-_].*\.(ips|crash)$')
RULES = ('one_index_per_key', 'no_new_standalone_index', 'no_fail_closed', 'no_startup_error', 'no_restart_bound',
         'no_abort', 'no_critical_pressure')
# The rules each phase gates on; the others are reported.
GATED = {'round': RULES,
         'week': ('one_index_per_key', 'no_fail_closed', 'no_startup_error', 'no_restart_bound', 'no_abort')}


def is_holder(process):
    """Whether a process holds an index: every daemon, and a standalone server whose measured
    footprint is past the size of one that has not built one. A shim never does."""
    if process['role'] == 'daemon':
        return True
    return process['role'] == 'standalone' and (process.get('footprint_bytes') or 0) >= HOLDER_MIN_BYTES


def built(process):
    """A daemon whose measured footprint shows an index was built."""
    return process['role'] == 'daemon' and (process.get('footprint_bytes') or 0) >= HOLDER_MIN_BYTES


def load_samples(path, since=None, until=None):
    """(samples in the window, lines that could not be read: a sampler killed mid-write)."""
    samples, skipped = [], 0
    with open(path) as source:
        for line in source:
            if not line.strip():
                continue
            try:
                sample = json.loads(line)
            except ValueError:
                skipped += 1
                continue
            if (since is None or sample['t'] >= since) and (until is None or sample['t'] <= until):
                samples.append(sample)
    return samples, skipped


def holder_identity(process):
    """What one index is for: a daemon's key, or a standalone server's repository."""
    if process['role'] == 'daemon':
        return 'daemon ' + (process.get('key') or '?')
    return 'standalone ' + (process.get('repo') or '?')


# CPU seconds one daemon must spend between two consecutive samples for that interval's UTC day
# to show use (owner decision 2026-10-16, PR #424 review round 1, Opus W1; replaces 0.5 s summed
# over the day). An idle daemon with a session attached spends about 0.0006 s a minute on its 5 s
# identity tick (0.86 s a day, measured at nanosecond precision), about 80 times below this in a
# 60 s interval; a tool call on a warm index costs tens of milliseconds or more.
USAGE_INTERVAL_CPU_SECS = 0.05
# Owner decision 2026-10-16, PR #424 review round 2: twice the default 60 s cadence.
USAGE_INTERVAL_MAX_SECS = 120


def usage_days(samples):
    """The UTC days of the sample intervals in which one daemon's CPU time grew by at least
    USAGE_INTERVAL_CPU_SECS. CPU readings for that pid are required at both consecutive endpoints,
    at most USAGE_INTERVAL_MAX_SECS apart (owner decisions 2026-10-16). A missing pid or CPU reading
    contributes nothing for that interval. A resident daemon or attached shim is not use. Growth
    is rounded to microseconds: `ps` prints hundredths, and 0.30 - 0.25 is 0.0499... in floating point."""
    days, previous = set(), {}
    for index, sample in enumerate(samples):
        now = {p['pid']: p.get('cpu_secs') for p in sample['processes'] if p['role'] == 'daemon'}
        excluded = index == 0 or sample['t'] - samples[index - 1]['t'] > USAGE_INTERVAL_MAX_SECS
        for pid, cpu in now.items():
            if cpu is None or excluded:
                continue
            before = previous.get(pid)
            if before is not None and round(cpu - before, 6) >= USAGE_INTERVAL_CPU_SECS:
                days.add(time.strftime('%Y-%m-%d', time.gmtime(sample['t'])))
        previous = now
    return sorted(days)


def summarize_samples(samples):
    worst_per_key, worst_per_repo, new_standalone, old_standalone, daemons = {}, {}, {}, {}, {}
    missing = {'ps': 0, 'pressure': 0, 'standalone_footprint': 0, 'footprint': 0}
    first_pids = {p['pid'] for p in samples[0]['processes']} if samples else set()
    peaks = {'holders': 0, 'shims': 0, 'prism_footprint_bytes': 0, 'swap_used_mib': 0.0, 'pressure': 0,
             'compressed_bytes': 0, 'built_daemons': 0}
    for sample in samples:
        missing['ps'] += not sample.get('ps_ok', True)
        holders = {}
        for process in sample['processes']:
            if process.get('footprint_status') == 'failed':
                missing['footprint'] += 1
                missing['standalone_footprint'] += process['role'] == 'standalone'
            if process.get('holder'):
                holders.setdefault(holder_identity(process), []).append(process['pid'])
            if process['role'] == 'standalone' and process.get('holder'):
                found = old_standalone if process['pid'] in first_pids else new_standalone
                found.setdefault(process.get('repo') or '?', set()).add(process['pid'])
            if process['role'] == 'daemon':
                daemons.setdefault((process['pid'], process.get('key')), [sample['t'], sample['t']])[1] = sample['t']
        for identity, pids in holders.items():
            if identity.startswith('daemon ') and len(pids) > len(worst_per_key.get(identity, [])):
                worst_per_key[identity] = pids
        repos = {}
        for process in sample['processes']:
            if process['role'] == 'daemon':
                repos.setdefault(process.get('repo') or '?', []).append(process['pid'])
        for repo, pids in repos.items():
            if len(pids) > len(worst_per_repo.get(repo, [])):
                worst_per_repo[repo] = pids
        host = sample.get('host') or {}
        missing['pressure'] += host.get('pressure') is None
        peaks['holders'] = max(peaks['holders'], sum(len(p) for p in holders.values()))
        peaks['shims'] = max(peaks['shims'], sum(1 for p in sample['processes'] if p['role'] == 'shim'))
        peaks['built_daemons'] = max(peaks['built_daemons'], sum(1 for p in sample['processes'] if built(p)))
        peaks['prism_footprint_bytes'] = max(peaks['prism_footprint_bytes'], sum(
            p.get('footprint_bytes') or 0 for p in sample['processes']))
        peaks['swap_used_mib'] = max(peaks['swap_used_mib'], (host.get('swap_mib') or {}).get('used', 0.0))
        peaks['pressure'] = max(peaks['pressure'], host.get('pressure') or 0)
        peaks['compressed_bytes'] = max(peaks['compressed_bytes'], host.get('compressed_bytes') or 0)
    return {'samples': len(samples), 'first': samples[0]['t'] if samples else None,
            'last': samples[-1]['t'] if samples else None, 'peaks': peaks, 'missing': missing,
            'max_daemons_per_key': max([len(p) for p in worst_per_key.values()] or [0]),
            'duplicated_keys': {k: v for k, v in worst_per_key.items() if len(v) > 1},
            'max_daemons_per_repo': max([len(p) for p in worst_per_repo.values()] or [0]),
            'usage_days': usage_days(samples),
            'usage_intervals_excluded': sum(b['t'] - a['t'] > USAGE_INTERVAL_MAX_SECS
                                            for a, b in zip(samples, samples[1:])),
            'largest_gap_secs': max([b['t'] - a['t'] for a, b in zip(samples, samples[1:])] or [0]),
            'covered_secs': samples[-1]['t'] - samples[0]['t'] if samples else 0,
            'repos_with_several_daemons': {k: v for k, v in worst_per_repo.items() if len(v) > 1},
            'new_standalone_holders': {repo: sorted(pids) for repo, pids in new_standalone.items()},
            'preexisting_standalone_holders': {repo: sorted(pids) for repo, pids in old_standalone.items()},
            'daemon_instances': [{'pid': pid, 'key': key, 'first_seen': span[0], 'last_seen': span[1]}
                                 for (pid, key), span in sorted(daemons.items(), key=lambda item: item[1][0])]}


def scan_lines(lines):
    """Client events in shim standard-error lines."""
    events = {'fail_closed': [], 'reached': set(), 'lost': 0, 'build_crash_refusals': 0, 'startup_errors': []}
    for line in lines:
        if FAIL_CLOSED in line:
            events['fail_closed'].append(line.strip())
            if BUILD_CRASH in line:
                events['build_crash_refusals'] += 1
        if STARTUP_ERROR.match(line):
            events['startup_errors'].append(line.strip())
        found = REACHED.search(line)
        if found:
            events['reached'].add((int(found.group(1)), int(found.group(2))))
        if LOST.search(line):
            events['lost'] += 1
    return events


def iso_epoch(text):
    """Epoch seconds of a UTC time written `YYYY-MM-DDTHH:MM:SS…`."""
    return calendar.timegm(time.strptime(text[:19], '%Y-%m-%dT%H:%M:%S'))


def in_window(epoch, since, until):
    return (since is None or epoch >= since) and (until is None or epoch <= until)


def logs_readable(claude_root, client_logs):
    """Whether every named log source exists: the Claude Code root a directory, every client log a
    file. No source at all is not coverage."""
    if not claude_root and not client_logs:
        return False
    if claude_root and not Path(claude_root).is_dir():
        return False
    return all(Path(path).is_file() for path in client_logs)


def claude_log_lines(root, since=None, until=None):
    """Standard-error lines of prism servers in Claude Code's MCP logs under `root`
    (`<project>/mcp-logs-<server>/*.jsonl`, server names containing `prism`)."""
    for path in sorted(Path(root).glob('*/mcp-logs-*prism*/*.jsonl')):
        with open(path, errors='replace') as source:
            for raw in source:
                try:
                    entry = json.loads(raw)
                except ValueError:
                    continue
                text = entry.get('error') if isinstance(entry, dict) else None
                stamp = entry.get('timestamp') if isinstance(entry, dict) else None
                if not isinstance(text, str) or not text.startswith('Server stderr: ') or not stamp:
                    continue
                if in_window(iso_epoch(stamp), since, until):
                    for line in text[len('Server stderr: '):].splitlines():
                        yield line


def client_log_lines(path, since=None, until=None):
    """Lines of a log prism-mcp-logged.sh wrote (`<UTC time> <pid> <line>`), in the window."""
    with open(path, errors='replace') as source:
        for raw in source:
            found = CLIENT_LINE.match(raw.rstrip('\n'))
            if found and in_window(iso_epoch(found.group(1)), since, until):
                yield found.group(3)


def crash_reports(directory, since=None, until=None):
    """(prism-mcp crash reports in `directory` modified in the window, whether it could be read)."""
    try:
        paths = sorted(Path(directory).iterdir()) if directory else None
    except OSError:
        paths = None
    if paths is None:
        return [], False
    return [p.name for p in paths if CRASH_REPORT.match(p.name) and in_window(p.stat().st_mtime, since, until)], True


def in_round(holders, round_repos):
    """The standalone holders whose repository is one of the round's checkouts (paths resolved),
    or all of them when no checkouts are named."""
    if round_repos is None:
        return holders
    checkouts = {os.path.realpath(repo) for repo in round_repos}
    return {repo: pids for repo, pids in holders.items() if os.path.realpath(repo) in checkouts}


def judge(summary, events, crashes, coverage, round_repos=None):
    """Each rule's verdict: FAIL on a breach that was seen; else UNKNOWN when the evidence it
    needs was not collected; else PASS. `coverage`: {'logs': bool, 'crash_reports': bool}. With
    `round_repos`, a new standalone holder fails the round only in one of those checkouts (review
    round 2, Opus W-D): a launcher the soak does not edit, elsewhere, is reported."""
    missing = summary['missing']

    def verdict(breach, unseen):
        return 'FAIL' if breach else 'UNKNOWN' if unseen else 'PASS'
    logs = not coverage['logs']
    return {'one_index_per_key': verdict(summary['max_daemons_per_key'] > 1, missing['ps']),
            'no_new_standalone_index': verdict(in_round(summary['new_standalone_holders'], round_repos),
                                               missing['ps'] or missing['standalone_footprint']),
            'no_fail_closed': verdict(events['fail_closed'], logs),
            'no_startup_error': verdict(events['startup_errors'], logs),
            'no_restart_bound': verdict(events['build_crash_refusals'], logs),
            'no_abort': verdict(crashes, not coverage['crash_reports']),
            'no_critical_pressure': verdict(summary['peaks']['pressure'] >= PRESSURE_CRITICAL, missing['pressure'])}


def usage(phase, summary, events, min_shims, min_days, sources_reached=None):
    """The use a phase must have seen, check by check (True when seen). `sources_reached`: for
    each --client-log, whether it holds a reached daemon in the window (review round 2, Opus S-A:
    the bridge's own log must show the bridge's agents reached one)."""
    if phase == 'round':
        return {'shims_at_once_at_least_%d' % min_shims: summary['peaks']['shims'] >= min_shims,
                'a_daemon_reached_in_every_client_log': bool(sources_reached) and all(sources_reached),
                'an_index_built': summary['peaks']['built_daemons'] >= 1}
    return {'usage_days_at_least_%d' % min_days: len(summary['usage_days']) >= min_days}


def overall(phase, verdicts, use):
    """FAIL on any gated rule's FAIL; INADMISSIBLE when a gated rule is UNKNOWN or the use was not
    seen; else PASS."""
    gated = [verdicts[rule] for rule in GATED[phase]]
    if 'FAIL' in gated:
        return 'FAIL'
    return 'INADMISSIBLE' if 'UNKNOWN' in gated or not all(use.values()) else 'PASS'


def render(phase, summary, events, crashes, verdicts, use, result, skipped):
    peaks = summary['peaks']
    lines = ['# prism-mcp soak report (%s)' % phase, '',
             'Samples: %d, from %s to %s (epoch seconds); unreadable sample lines skipped: %d.'
             % (summary['samples'], summary['first'], summary['last'], skipped),
             '', '**Verdict: %s**' % result, '', '| Rule | Verdict | Gated |', '|---|---|---|']
    lines += ['| %s | %s | %s |' % (rule, verdicts[rule], 'yes' if rule in GATED[phase] else 'no') for rule in RULES]
    lines += ['', '| Use | Seen |', '|---|---|'] + ['| %s | %s |' % (k, 'yes' if v else 'NO') for k, v in use.items()]
    lines += ['', '| Measure | Value |', '|---|---|',
              '| Most index holders at once | %d |' % peaks['holders'],
              '| Most daemons for one key at once | %d |' % summary['max_daemons_per_key'],
              '| Most daemons for one repository at once (any configuration or build) | %d |'
              % summary['max_daemons_per_repo'],
              '| Most shims at once | %d |' % peaks['shims'],
              '| Most daemons with a built index at once | %d |' % peaks['built_daemons'],
              "| Usage days (UTC; a sample interval with at least %.2f s of one daemon's CPU; "
              "both endpoints observed, at most %d s) | %d |"
              % (USAGE_INTERVAL_CPU_SECS, USAGE_INTERVAL_MAX_SECS, len(summary['usage_days'])),
              '| Intervals excluded from usage credit (> %d s) | %d |'
              % (USAGE_INTERVAL_MAX_SECS, summary['usage_intervals_excluded']),
              '| Time covered by the samples / largest gap between two (s) | %d / %d |'
              % (summary['covered_secs'], summary['largest_gap_secs']),
              '| Peak footprint of all prism-mcp processes (MiB) | %.1f |' % (peaks['prism_footprint_bytes'] / MIB),
              '| Peak swap used (MiB) | %.1f |' % peaks['swap_used_mib'],
              '| Peak compressor (MiB) | %.1f |' % (peaks['compressed_bytes'] / MIB),
              '| Highest memory pressure level | %d |' % peaks['pressure'],
              '| Samples without a process table / without pressure | %d / %d |'
              % (summary['missing']['ps'], summary['missing']['pressure']),
              '| Live processes whose footprint could not be measured | %d |' % summary['missing']['footprint'],
              '| Daemon instances seen in samples | %d |' % len(summary['daemon_instances']),
              '| Daemon instances clients reached | %d |' % len(events['reached']),
              '| Fail-closed lines | %d |' % len(events['fail_closed']),
              '| Of which a build that died (restart bound) | %d |' % events['build_crash_refusals'],
              '| Startup errors (a server that never started) | %d |' % len(events['startup_errors']),
              '| Sessions ended by their daemon | %d |' % events['lost'],
              '| prism-mcp crash reports | %d |' % len(crashes)]
    for title, key in (('Keys with more than one daemon at once', 'duplicated_keys'),
                       ('Repositories with several daemons at once (other cache directories, refresh policies or '
                        'builds)', 'repos_with_several_daemons'),
                       ('Standalone index holders first seen after the start (a round gates only those in its '
                        'checkouts)', 'new_standalone_holders'),
                       ('Standalone index holders already running at the start (not counted)',
                        'preexisting_standalone_holders')):
        if summary[key]:
            lines += ['', '%s: %s' % (title, json.dumps(summary[key]))]
    lines += ['', 'Fail-closed lines:'] + ['- `%s`' % line for line in events['fail_closed'][:50]]
    lines += ['', 'Startup errors:'] + ['- `%s`' % line for line in events['startup_errors'][:50]]
    lines += ['', 'Crash reports:'] + ['- `%s`' % name for name in crashes]
    return '\n'.join(lines) + '\n'


def cmd_report(args):
    samples, skipped = load_samples(args.samples, args.since, args.until)
    if not samples:
        raise SystemExit('no samples in %s for the window' % args.samples)
    client_logs = args.client_log or []
    lines, sources_reached = [], []
    if args.claude_logs and Path(args.claude_logs).is_dir():
        lines += list(claude_log_lines(args.claude_logs, args.since, args.until))
    for path in client_logs:
        found = list(client_log_lines(path, args.since, args.until)) if os.path.isfile(path) else []
        sources_reached.append(bool(scan_lines(found)['reached']))
        lines += found
    summary = summarize_samples(samples)
    events = scan_lines(lines)
    crashes, crash_ok = crash_reports(args.crash_reports, args.since, args.until)
    coverage = {'logs': logs_readable(args.claude_logs, client_logs), 'crash_reports': crash_ok}
    verdicts = judge(summary, events, crashes, coverage, args.round_repo)
    use = usage(args.phase, summary, events, args.min_shims, args.min_days, sources_reached)
    result = overall(args.phase, verdicts, use)
    Path(args.out).write_text(render(args.phase, summary, events, crashes, verdicts, use, result, skipped))
    if args.json:
        Path(args.json).write_text(json.dumps(
            {'phase': args.phase, 'verdict': result, 'summary': summary, 'coverage': coverage, 'use': use,
             'events': dict(events, reached=sorted(events['reached'])), 'crash_reports': crashes,
             'verdicts': verdicts, 'skipped_lines': skipped}, indent=1, sort_keys=True))
    print('wrote %s: %s' % (args.out, result))
    return 0
````

- [ ] **Step 4: Run them and watch them pass**

```bash
python3 -m unittest discover -s scripts/soak -p 'test_soak*.py' > "$LOG_DIR/t2-green.log" 2>&1; echo "rc=$?"
tail -3 "$LOG_DIR/t2-green.log"
```

Expected: `rc=0`, `Ran 38 tests`, `OK`.

- [ ] **Step 5: Commit**

```bash
git add scripts/soak/soak.py scripts/soak/soak_judge.py scripts/soak/test_soak.py scripts/soak/test_soak_coverage.py scripts/soak/test_soak_round2.py scripts/soak/fake_prism_mcp.py scripts/soak/prism-mcp-logged.sh .github/workflows/ci.yml
git commit -m "feat(soak): S6 — prism-mcp process sampler, soak report and client log wrapper"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run Step 4's command, then `git checkout -- <file>`.

| Mutant | File | Edit | Expected failing tests (observed) |
|---|---|---|---|
| S1 | `soak.py` | `elif '--shared' in tokens:` → `elif '--never' in tokens:` (a shim taken for a standalone server) | `test_roles_come_from_the_command_line`, `test_a_binary_path_with_a_space_is_still_prism_mcp` |
| S2 | `soak_judge.py` | `is_holder`: `>= HOLDER_MIN_BYTES` → `>` | `test_an_index_holder_is_a_daemon_or_a_standalone_server_past_100_mib` |
| S3 | `soak_judge.py` | `holders = {}` moved before the loop over samples | `test_two_daemons_of_one_key_at_once_are_found`, `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` |
| S4 | `soak_judge.py` | `'reached': set()` → `'reached': []` | `test_client_lines_are_counted`, `test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach` and four of `test_soak_coverage.py` |
| S5 | `soak_judge.py` | `'*/mcp-logs-*prism*/*.jsonl'` → `'*/mcp-logs-*/*.jsonl'` | `test_claude_code_logs_are_read_for_prism_servers_in_the_window` |
| S6 | `soak_judge.py` | the window check of a Claude Code entry → `if True:` | `test_claude_code_logs_are_read_for_prism_servers_in_the_window` |
| S7 | `soak_judge.py` | `calendar.timegm(…)` → `time.mktime(…)` | `test_iso_times_are_utc_in_any_local_zone_winter_and_summer` (now in every zone: the test sets its own) |
| S7b | `soak_judge.py` | `calendar.timegm(…)` → `time.mktime(…) - time.timezone` (the earlier draft's bug; review round 1, sol S7) | `test_iso_times_are_utc_in_any_local_zone_winter_and_summer` (July, MST7MDT) |
| S8 | `soak_judge.py` | `CRASH_REPORT` → `r'.*\.(ips\|crash)$'` | `test_crash_reports_of_prism_mcp_in_the_window_are_counted` |
| S9 | `soak_judge.py` | `pressure >= PRESSURE_CRITICAL` → `>` | `test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach`, `test_a_missing_pressure_reading_is_unknown_and_a_critical_one_still_fails` |
| S10 | `prism-mcp-logged.sh` | `while IFS= read -r line \|\| [ -n "$line" ]; do` → `while IFS= read -r line; do` | `test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes` |
| S11 | `prism-mcp-logged.sh` | the `printf '%s\n' "$line" >&2` line removed | `test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes` |
| S12 | `soak_judge.py` | the `if process['role'] == 'daemon':` of the per-repository count → `if True:` | `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` |
| U1 | `soak_judge.py` | a sample without a process table not counted | `test_a_failed_process_table_makes_the_process_rules_unknown` |
| U2 | `soak_judge.py` | a standalone server's failed footprint not counted | `test_a_failed_footprint_of_a_standalone_server_is_unknown_and_a_vanished_one_is_not` |
| U3 | `soak_judge.py` | a missing pressure reading not counted | `test_a_missing_pressure_reading_is_unknown_and_a_critical_one_still_fails` |
| U4 | `soak_judge.py` | `logs = not coverage['logs']` → `logs = False` | `test_unread_logs_or_crash_reports_are_unknown_and_what_was_read_still_fails`, `test_a_round_report_without_its_log_sources_is_inadmissible` |
| U5 | `soak_judge.py` | the crash-report coverage ignored | `test_unread_logs_or_crash_reports_are_unknown_and_what_was_read_still_fails` |
| U6 | `soak.py` | a live pid without a reading → `'gone'` | `test_a_vanished_pid_is_gone_and_a_live_one_without_a_reading_failed` |
| U7 | `soak.py` | `'ps_ok': listing is not None` → `True` | `test_a_sample_whose_process_table_could_not_be_read_says_so` |
| U8 | `soak_judge.py` | `logs_readable` with no source → `True` | `test_log_sources_are_complete_only_when_every_named_one_exists` |
| U9 | `soak_judge.py` | the round's `an_index_built` → `True` | `test_a_round_needs_shims_a_reached_daemon_and_a_built_index` |
| U11 | `soak_judge.py` | `overall`: a gated UNKNOWN no longer makes the phase INADMISSIBLE | `test_a_round_needs_shims_a_reached_daemon_and_a_built_index` (its failed-`ps` case, added after the mutant first survived) |
| U12 | `soak_judge.py` | every standalone holder counted as new | `test_standalone_holders_already_running_at_the_start_do_not_fail_a_phase`, `test_standalone_holders_and_peaks_are_reported` |
| U13 | `soak_judge.py` | startup errors not scanned | `test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules` |
| U14 | `soak_judge.py` | `no_restart_bound` judged on 0 | `test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules` |
| U15 | `soak.py` | the binary path with a space not looked for | `test_a_binary_path_with_a_space_is_still_prism_mcp` |
| U16 | `soak_judge.py` | `load_samples` without its `try` | `test_a_truncated_last_line_is_skipped_and_counted` |
| U17 | `soak.py` | `probe`: `answered = True` | `test_a_binary_that_rejects_shared_fails_and_says_why`, `test_a_server_that_never_answers_fails_at_the_timeout` |
| C1 | `soak_judge.py` | a daemon in the first sample counted with its whole CPU time | `test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident` |
| C2 | `soak_judge.py` | `seconds >= USAGE_CPU_SECS` → `>= 0` | `test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident` |
| C3 | `soak.py` | `cpu_secs=None` (CPU time not read) | `test_roles_come_from_the_command_line` |
| C4 | `soak_judge.py` | every daemon sample adds `USAGE_CPU_SECS` (presence is use: round 1's rule in another form) | `test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident` |
| W1 | `soak.py` | `open_window`: `resumed = False` (the start rewritten on a resume) | `test_a_resumed_sampler_keeps_the_window_it_started`, `test_a_day_two_failure_and_earlier_usage_days_survive_a_resume` |
| R1 | `soak_judge.py` | `in_round` returns every holder | `test_only_standalone_servers_of_the_round_s_checkouts_are_gated` |
| R2 | `soak_judge.py` | `in_round` compares paths as written | `test_only_standalone_servers_of_the_round_s_checkouts_are_gated` |
| R3 | `soak_judge.py` | `a_daemon_reached_in_every_client_log` → `True` | `test_a_round_needs_a_reached_daemon_in_every_client_log` |
| R4 | `soak.py` | `--phase round` accepted without `--round-repo` | `test_a_round_report_without_its_checkouts_is_refused` |
| G1 | `soak_judge.py` | the smallest gap for the largest | `test_the_largest_gap_between_samples_and_the_time_covered_are_reported` |

- [ ] **Step 7: Verify S6-1 and open its pull request**

S6-1 is Tasks 1–2. Run Task 4 Step 3's commands at this head (the soak tests are `test_soak*.py` here: `Ran 38 tests`), then open the pull request with the body of Task 4 Step 5, its decisions narrowed to D110–D113, D117, D118, D120, D121, D124, D125, D127–D130 and its claims to the rows of Tasks 1–2. The CI step for the soak tools is part of this pull request (review round 2, Opus S-E: S6-1's tests should not wait for S6-2 to run in CI); at this head it finds the `test_soak*` files only. The Linux run is at the S6-2 head (its code contains this head's).

---

### Task 3: The fan-out latency driver

**Files:** Create `scripts/soak/fanout.py`, `scripts/soak/test_fanout.py`, `scripts/soak/test_fanout_gate.py`, `scripts/soak/test_fanout_live.py`.

**Interfaces** (the soak and C2-M1's fan-out gate use them): `fanout.py run --bin BIN --corpus NAME=PATH --cache-dir DIR --mode shared|standalone --out DIR [--clients N] [--workloads DIR] [--refresh-at F | --no-refresh] [--timeout S] [--pressure required|off]` writes `run.json` (with `profile` `gate`/`diagnostic` and `pressure`: every reading), `calls.jsonl`, `summary.json` (`p50_ms`, `p95_ms`, `p99_ms`, `max_ms`, `mean_ms`, `refresh_ms`, `refresh_outcome`, `refresh_overlap` {`calls`, `max_ms`, `refresh_ms`}, `profile`, `outcomes`, `admissible`, `problems`), `clients/<i>.stderr` and, for a shared run, `daemon-logs/`; it refuses a directory that already has results and a corpus whose commit is not its workload's (memreport's `bind_workload`). `fanout.py compare --baseline DIR … --candidate DIR … --out FILE.md [--json FILE] [--allow-client-mismatch]` (`P95_MAX_RATIO`, `P99_MAX_RATIO`).

- [ ] **Step 1: Write the failing tests**

Create `scripts/soak/test_fanout.py`:

````python
"""Tests of fanout.py: the pure parts, and whole runs against fake_prism_mcp.py.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import fanout  # noqa: E402

FAKE = HERE / 'fake_prism_mcp.py'


def dead_pid():
    """The pid of a process that has ended and been reaped."""
    child = subprocess.Popen(['true'])
    child.wait()
    return child.pid


def corpus_and_workload(root, requests=6):
    """A one-commit git corpus and a workload directory bound to it."""
    corpus = Path(root) / 'corpus'
    corpus.mkdir()
    (corpus / 'main.py').write_text('def run():\n    return 1\n')
    git = ['git', '-C', str(corpus), '-c', 'user.name=t', '-c', 'user.email=t@t']
    subprocess.run(git[:3] + ['init', '-q'], check=True)
    subprocess.run(git + ['add', '.'], check=True)
    subprocess.run(git + ['commit', '-qm', 'c'], check=True)
    commit = subprocess.run(git[:3] + ['rev-parse', 'HEAD'], check=True, capture_output=True, text=True).stdout.strip()
    lines = ''.join(json.dumps({'id': n, 'jsonrpc': '2.0', 'method': 'tools/call', 'params': {
        'name': ('nav_callers', 'nav_callees')[n % 2 == 0], 'arguments': {
            'seed': {'kind': 'symbol', 'name': 'f%d' % n}}}}) + '\n' for n in range(1, requests + 1)).encode()
    workloads = Path(root) / 'workloads'
    workloads.mkdir()
    (workloads / 'tiny.jsonl').write_bytes(lines)
    (workloads / 'MANIFEST.json').write_text(json.dumps({'tiny': {
        'commit': commit, 'requests': requests, 'sha256': hashlib.sha256(lines).hexdigest()}}))
    return corpus, workloads


class Pure(unittest.TestCase):
    def test_percentile_is_nearest_rank(self):
        values = list(range(1, 101))
        self.assertEqual(fanout.percentile(values, 0.50), 50)
        self.assertEqual(fanout.percentile(values, 0.95), 95)
        self.assertEqual(fanout.percentile(values, 0.99), 99)
        self.assertEqual(fanout.percentile([3, 1, 2], 0.95), 3)
        self.assertEqual(fanout.percentile([7], 0.5), 7)
        self.assertIsNone(fanout.percentile([], 0.5))

    def test_outcome_tells_the_four_answers_apart(self):
        self.assertEqual(fanout.outcome({'error': {'code': -32602}}), 'rpc_error')
        self.assertEqual(fanout.outcome({'result': {'isError': True, '_meta': {'prism/retryable': True}}}),
                         'retryable')
        self.assertEqual(fanout.outcome({'result': {'isError': True}}), 'tool_error')
        self.assertEqual(fanout.outcome({'result': {'isError': False}}), 'ok')

    def test_each_client_starts_its_own_share_of_the_way_through(self):
        items = list(range(10))
        self.assertEqual([fanout.rotated(items, c, 3)[0] for c in range(3)], [0, 3, 6])
        self.assertEqual(sorted(fanout.rotated(items, 2, 3)), items)
        self.assertEqual([fanout.rotated(list(range(200)), c, 8)[0] for c in range(8)],
                         [0, 25, 50, 75, 100, 125, 150, 175])


def record(ms, outcome='ok', refresh=False):
    return {'client': 0, 'seq': 0, 'tool': 't', 'refresh': refresh, 'start': 0.0, 'ms': ms, 'outcome': outcome}


def run_info(mode='standalone', clients=1, requests=2, instances=None, failures=None, refresh_at=None):
    return {'mode': mode, 'clients': clients, 'workload': {'requests': requests, 'sha256': 'w'},
            'daemon_instances': instances if instances is not None else [], 'failures': failures or [],
            'corpus': {'commit': 'c', 'dirty': 0}, 'refresh_at': refresh_at, 'pressure': {'problems': []}}


class Summaries(unittest.TestCase):
    def test_a_complete_run_is_admissible_and_the_refresh_is_kept_apart(self):
        summary = fanout.summarize([record(1.0), record(3.0), record(50.0, refresh=True)], run_info())
        self.assertTrue(summary['admissible'], summary['problems'])
        self.assertEqual((summary['calls'], summary['p95_ms'], summary['refresh_ms']), (2, 3.0, 50.0))

    def test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible(self):
        self.assertFalse(fanout.summarize([record(1.0), record(1.0, 'retryable')], run_info())['admissible'])
        self.assertFalse(fanout.summarize([record(1.0), record(1.0, 'rpc_error')], run_info())['admissible'])
        self.assertFalse(fanout.summarize([record(1.0)], run_info())['admissible'])
        two = run_info('shared', instances=[(1, 7), (2, 7)])
        self.assertFalse(fanout.summarize([record(1.0), record(1.0)], two)['admissible'])
        one = run_info('shared', instances=[(1, 7)])
        self.assertTrue(fanout.summarize([record(1.0), record(1.0)], one)['admissible'])

    def test_a_tool_error_is_an_answer_not_a_failure(self):
        self.assertTrue(fanout.summarize([record(1.0), record(1.0, 'tool_error')], run_info())['admissible'])


def arm(p95, p99, admissible=True, sha='w'):
    run = run_info(refresh_at=0.5)
    run['workload']['sha256'] = sha
    return {'dir': 'd', 'run': run, 'summary': {'p50_ms': 1.0, 'p95_ms': p95, 'p99_ms': p99,
                                                 'admissible': admissible, 'problems': [] if admissible else ['x']}}


class Verdicts(unittest.TestCase):
    def test_the_rule_passes_at_its_bounds_and_fails_past_them(self):
        base = [arm(10.0, 20.0)]
        self.assertTrue(fanout.judge(base, [arm(11.0, 25.0)])['pass'])
        self.assertFalse(fanout.judge(base, [arm(11.01, 20.0)])['pass'])
        self.assertFalse(fanout.judge(base, [arm(10.0, 25.01)])['pass'])

    def test_medians_across_runs_are_compared(self):
        verdict = fanout.judge([arm(10.0, 20.0), arm(10.0, 20.0), arm(100.0, 200.0)],
                               [arm(10.5, 21.0), arm(10.5, 21.0), arm(1.0, 1.0)])
        self.assertTrue(verdict['pass'])
        self.assertEqual(verdict['figures']['baseline']['p95_ms'], 10.0)

    def test_an_inadmissible_run_or_another_workload_fails_the_comparison(self):
        self.assertFalse(fanout.judge([arm(10.0, 20.0)], [arm(1.0, 1.0, admissible=False)])['pass'])
        self.assertFalse(fanout.judge([arm(10.0, 20.0)], [arm(1.0, 1.0, sha='other')])['pass'])


class RunCase(unittest.TestCase):
    """Whole runs of fanout.py against fake_prism_mcp.py, in a directory of the test's own."""

    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.corpus, self.workloads = corpus_and_workload(self.root.name)
        self.saved_env = dict(os.environ)
        self.addCleanup(lambda: (os.environ.clear(), os.environ.update(self.saved_env)))
        self.saved_retry = fanout.WARMUP_RETRY_SECS
        fanout.WARMUP_RETRY_SECS = 0.01
        self.addCleanup(lambda: setattr(fanout, 'WARMUP_RETRY_SECS', self.saved_retry))
        self.saved_pressure = fanout.read_pressure
        fanout.read_pressure = lambda: 1
        self.addCleanup(lambda: setattr(fanout, 'read_pressure', self.saved_pressure))

    def run_fanout(self, name, mode, clients=3, extra=()):
        out = Path(self.root.name) / name
        with redirect_stdout(io.StringIO()):
            fanout.main(['run', '--bin', str(FAKE), '--corpus', 'tiny=%s' % self.corpus, '--cache-dir',
                         str(Path(self.root.name) / 'cache'), '--mode', mode, '--out', str(out), '--clients',
                         str(clients), '--workloads', str(self.workloads), '--timeout', '60'] + list(extra))
        calls = [json.loads(line) for line in (out / 'calls.jsonl').read_text().splitlines()]
        return out, calls, json.loads((out / 'summary.json').read_text()), json.loads((out / 'run.json').read_text())


class Runs(RunCase):

    def test_every_client_makes_every_call_and_client_0_refreshes_once(self):
        # Three warming answers: with the extra warm-up call of the second tool, a warm-up that did not
        # retry would leave the first timed call answered retryable.
        os.environ['FAKE_WARMING'] = '3'
        _, calls, summary, run = self.run_fanout('standalone', 'standalone')
        self.assertTrue(summary['admissible'], summary['problems'])
        self.assertEqual(len([c for c in calls if not c['refresh']]), 18)
        refreshes = [c for c in calls if c['refresh']]
        self.assertEqual([(c['client'], c['seq'], c['outcome']) for c in refreshes], [(0, 3, 'ok')])
        self.assertEqual(run['server'], {'name': 'prism-mcp', 'version': 'fake'})
        self.assertEqual(run['exits'], [0, 0, 0])

    def test_no_refresh_means_none(self):
        _, calls, summary, _ = self.run_fanout('quiet', 'standalone', extra=['--no-refresh'])
        self.assertEqual([c for c in calls if c['refresh']], [])
        self.assertIsNone(summary['refresh_ms'])

    def test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down(self):
        os.environ['FAKE_DAEMON_PID'] = str(dead_pid())
        _, _, summary, run = self.run_fanout('shared', 'shared')
        self.assertTrue(summary['admissible'], summary['problems'])
        self.assertEqual(run['daemon_instances'], [[int(os.environ['FAKE_DAEMON_PID']), 7]])
        self.assertEqual(run['shutdown']['exit'], 0)
        self.assertIn('--shared', run['argv'])

    def test_a_client_that_dies_makes_the_run_inadmissible_without_hanging_it(self):
        os.environ['FAKE_DIE_AFTER'] = '3'
        _, _, summary, run = self.run_fanout('dies', 'standalone', clients=2)
        self.assertFalse(summary['admissible'])
        self.assertTrue(run['failures'])

    def test_clients_start_from_the_default_output_settings(self):
        os.environ['PRISM_MCP_CONCISE_SHAPE'] = 'legacy'
        out, _, _, _ = self.run_fanout('settings', 'standalone', clients=1)
        self.assertIn('PRISM_MCP_CONCISE_SHAPE=<unset>', (out / 'clients' / '0.stderr').read_text())

    def test_clients_that_die_while_warming_up_end_the_run(self):
        out = Path(self.root.name) / 'warmup-death'
        env = dict(os.environ, FAKE_WARMING='5', FAKE_DIE_AFTER='1')
        done = subprocess.run([sys.executable, str(HERE / 'fanout.py'), 'run', '--bin', str(FAKE), '--corpus',
                               'tiny=%s' % self.corpus, '--cache-dir', str(Path(self.root.name) / 'cache'),
                               '--mode', 'standalone', '--out', str(out), '--clients', '2', '--workloads',
                               str(self.workloads), '--timeout', '600'],
                              env=env, capture_output=True, text=True, timeout=60)
        self.assertEqual(done.returncode, 0, done.stderr)
        summary = json.loads((out / 'summary.json').read_text())
        self.assertFalse(summary['admissible'])
        self.assertEqual(summary['calls'], 0)

    def test_a_used_run_directory_is_refused(self):
        self.run_fanout('used', 'standalone', clients=1)
        with self.assertRaises(SystemExit):
            self.run_fanout('used', 'standalone', clients=1)


if __name__ == '__main__':
    unittest.main()
````

Create `scripts/soak/test_fanout_gate.py`:

````python
"""Tests of the fan-out gate's admissibility (review round 1: sol 3, sol 4, Opus W1, W8, S6): a
gating run needs exactly one successful refresh, a critical or unreadable pressure reading
invalidates a run, the refresh's overlap with other clients' calls is measured, and a comparison
never crashes on a run without calls.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import fanout  # noqa: E402
from test_fanout import RunCase, arm, dead_pid, run_info  # noqa: E402


def call(client, start, ms, outcome='ok', refresh=False, seq=0):
    return {'client': client, 'seq': seq, 'tool': 'refresh_index' if refresh else 't', 'refresh': refresh,
            'start': start, 'ms': ms, 'outcome': outcome}


class Refresh(unittest.TestCase):
    def gate(self, records):
        return fanout.summarize(records, run_info(clients=1, requests=2, refresh_at=0.5))

    def test_a_gating_run_needs_exactly_one_successful_refresh(self):
        calls = [call(0, 0.0, 1.0), call(0, 0.1, 1.0, seq=1)]
        self.assertTrue(self.gate(calls + [call(0, 0.05, 5.0, refresh=True)])['admissible'])
        self.assertFalse(self.gate(calls)['admissible'])
        for outcome in ('retryable', 'rpc_error', 'tool_error'):
            summary = self.gate(calls + [call(0, 0.05, 5.0, outcome, refresh=True)])
            self.assertFalse(summary['admissible'], outcome)
            self.assertIn('refresh', ' '.join(summary['problems']))
        two = calls + [call(0, 0.05, 5.0, refresh=True), call(0, 0.07, 5.0, refresh=True)]
        self.assertFalse(self.gate(two)['admissible'])

    def test_a_run_without_a_refresh_is_diagnostic_and_never_compared(self):
        diagnostic = fanout.summarize([call(0, 0.0, 1.0), call(0, 0.1, 1.0, seq=1)], run_info(clients=1))
        self.assertTrue(diagnostic['admissible'])
        self.assertEqual(diagnostic['profile'], 'diagnostic')
        quiet = arm(10.0, 20.0)
        quiet['run']['refresh_at'] = None
        verdict = fanout.judge([arm(10.0, 20.0)], [quiet])
        self.assertFalse(verdict['pass'])
        self.assertIn('diagnostic', ' '.join(verdict['problems']))

    def test_the_calls_that_overlap_the_refresh_are_counted_and_their_longest_reported(self):
        records = [call(0, 1.0, 500.0, refresh=True),
                   call(1, 0.9, 400.0), call(2, 1.2, 50.0), call(3, 1.6, 9.0), call(1, 0.1, 99.0, seq=1),
                   call(0, 0.5, 300.0, seq=1), call(0, 1.1, 50.0, seq=2)]
        overlap = fanout.refresh_overlap(records)
        self.assertEqual(overlap, {'calls': 2, 'max_ms': 400.0, 'refresh_ms': 500.0})
        self.assertEqual(fanout.refresh_overlap([call(1, 0.0, 1.0)]), {'calls': 0, 'max_ms': None,
                                                                         'refresh_ms': None})


class Compare(unittest.TestCase):
    def test_a_run_with_no_calls_fails_the_comparison_instead_of_crashing_it(self):
        empty = arm(None, None, admissible=False)
        verdict = fanout.judge([arm(10.0, 20.0)], [empty])
        self.assertFalse(verdict['pass'])
        self.assertIn('**Verdict: FAIL**', fanout.render([arm(10.0, 20.0)], [empty], verdict))
        three = fanout.judge([arm(10.0, 20.0)] * 3, [arm(10.0, 20.0), empty, empty])
        self.assertFalse(three['pass'])

    def test_a_calibration_passes_only_when_both_directions_pass(self):
        # Review round 2 (Opus W-A): a 30 % spread between two identical arms passes one way only.
        slow, fast = [arm(100.0, 200.0)], [arm(70.0, 140.0)]
        self.assertTrue(fanout.judge(slow, fast)['pass'])
        self.assertFalse(fanout.judge(fast, slow)['pass'])
        self.assertFalse(fanout.calibrate(slow, fast)['pass'])
        self.assertFalse(fanout.calibrate(fast, slow)['pass'])
        self.assertTrue(fanout.calibrate([arm(100.0, 200.0)], [arm(105.0, 210.0)])['pass'])

    def test_runs_of_other_client_counts_or_refresh_points_are_not_compared(self):
        other = arm(10.0, 20.0)
        other['run']['clients'] = 4
        self.assertIn('clients', ' '.join(fanout.judge([arm(10.0, 20.0)], [other])['problems']))
        allowed = fanout.judge([arm(10.0, 20.0)], [other], allow_client_mismatch=True)
        self.assertTrue(allowed['pass'], allowed['problems'])
        later = arm(10.0, 20.0)
        later['run']['refresh_at'] = 0.9
        self.assertFalse(fanout.judge([arm(10.0, 20.0)], [later])['pass'])


class GateRuns(RunCase):
    def test_a_refresh_answered_with_a_tool_error_makes_the_run_inadmissible(self):
        os.environ['FAKE_REFRESH'] = 'tool_error'
        _, _, summary, _ = self.run_fanout('refresh-error', 'standalone', clients=2)
        self.assertFalse(summary['admissible'])

    def test_a_critical_pressure_reading_stops_and_invalidates_the_run(self):
        readings = iter([1, 1, 4])
        fanout.read_pressure = lambda: next(readings, 4)
        saved = fanout.PRESSURE_INTERVAL
        fanout.PRESSURE_INTERVAL = 0.01
        self.addCleanup(lambda: setattr(fanout, 'PRESSURE_INTERVAL', saved))
        os.environ['FAKE_DELAY_MS'] = '20'
        _, _, summary, run = self.run_fanout('critical', 'standalone', clients=2, extra=['--pressure', 'off'])
        self.assertFalse(summary['admissible'])
        self.assertIn('critical memory pressure', ' '.join(summary['problems']))
        self.assertEqual(run['pressure']['max'], 4)
        self.assertTrue(run['failures'], 'the clients were not stopped')
        self.assertLess(summary['calls'], 12)

    def test_an_unavailable_pressure_reading_invalidates_a_run_that_requires_it(self):
        fanout.read_pressure = lambda: None
        _, _, required, _ = self.run_fanout('unread', 'standalone', clients=1, extra=['--pressure', 'required'])
        self.assertFalse(required['admissible'])
        self.assertIn('pressure', ' '.join(required['problems']))
        _, _, off, _ = self.run_fanout('unread-off', 'standalone', clients=1, extra=['--pressure', 'off'])
        self.assertTrue(off['admissible'], off['problems'])

    def test_a_shared_run_keeps_its_daemon_logs(self):
        os.environ['FAKE_DAEMON_PID'] = str(dead_pid())
        out, _, _, _ = self.run_fanout('logs', 'shared', clients=2)
        self.assertEqual((out / 'daemon-logs' / 'fake.log').read_text().count('fake daemon log'), 2)

    def test_every_tool_of_the_workload_is_warmed_before_the_barrier(self):
        out, calls, summary, _ = self.run_fanout('warm', 'standalone', clients=2)
        self.assertTrue(summary['admissible'], summary['problems'])
        tools = len({c['tool'] for c in calls if not c['refresh']})
        timed = len([c for c in calls if c['client'] == 1])
        self.assertIn('fake: %d tool calls' % (tools + timed), (out / 'clients' / '1.stderr').read_text())


if __name__ == '__main__':
    unittest.main()
````

Create `scripts/soak/test_fanout_live.py`:

````python
"""fanout.py against a real prism-mcp, on a three-file repository: three shims of one daemon, then
three standalone servers. Skipped unless PRISM_SOAK_BIN names a binary built with --features mcp:

    PRISM_SOAK_BIN=target/debug/prism-mcp python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import fanout  # noqa: E402

BIN = os.environ.get('PRISM_SOAK_BIN')


def call(n, name, symbol, file):
    return {'id': n, 'jsonrpc': '2.0', 'method': 'tools/call',
            'params': {'name': name, 'arguments': {'seed': {'kind': 'symbol', 'name': symbol, 'file': file}}}}


@unittest.skipUnless(BIN, 'set PRISM_SOAK_BIN to a prism-mcp binary to run the live fan-out test')
class Live(unittest.TestCase):
    def test_shared_clients_reach_one_daemon_and_leave_none_behind(self):
        with tempfile.TemporaryDirectory() as root:
            repo = Path(root) / 'live'
            repo.mkdir()
            (repo / 'util.py').write_text('def helper():\n    return 1\n')
            (repo / 'main.py').write_text('from util import helper\n\ndef run():\n    return helper()\n')
            git = ['git', '-C', str(repo), '-c', 'user.name=t', '-c', 'user.email=t@t']
            subprocess.run(git[:3] + ['init', '-q'], check=True)
            subprocess.run(git + ['add', '.'], check=True)
            subprocess.run(git + ['commit', '-qm', 'c'], check=True)
            commit = subprocess.run(git[:3] + ['rev-parse', 'HEAD'], check=True, capture_output=True,
                                    text=True).stdout.strip()
            requests = [call(1, 'nav_callers', 'helper', 'util.py'), call(2, 'nav_callees', 'run', 'main.py'),
                        call(3, 'nav_ego_graph', 'run', 'main.py'), call(4, 'nav_callers', 'run', 'main.py')]
            data = ''.join(json.dumps(r) + '\n' for r in requests).encode()
            workloads = Path(root) / 'workloads'
            workloads.mkdir()
            (workloads / 'live.jsonl').write_bytes(data)
            (workloads / 'MANIFEST.json').write_text(json.dumps({'live': {
                'commit': commit, 'requests': 4, 'sha256': hashlib.sha256(data).hexdigest()}}))
            runs = {}
            for mode in ('shared', 'standalone'):
                out = Path(root) / mode
                with redirect_stdout(io.StringIO()):
                    fanout.main(['run', '--bin', BIN, '--corpus', 'live=%s' % repo, '--cache-dir',
                                 str(Path(root) / 'cache'), '--mode', mode, '--out', str(out), '--clients', '3',
                                 '--workloads', str(workloads), '--timeout', '300'])
                runs[mode] = (json.loads((out / 'run.json').read_text()),
                              json.loads((out / 'summary.json').read_text()))
            shared_run, shared = runs['shared']
            self.assertTrue(shared['admissible'], shared['problems'])
            self.assertEqual(shared['outcomes'], {'ok': 12})
            self.assertEqual((shared['profile'], shared['refresh_outcome']), ('gate', 'ok'))
            self.assertEqual(shared['refresh_overlap']['refresh_ms'], shared['refresh_ms'])
            self.assertEqual(len(shared_run['daemon_instances']), 1)
            self.assertEqual((shared_run['shutdown']['exit'], shared_run['shutdown']['terminated']), (0, []))
            self.assertEqual(shared_run['exits'], [0, 0, 0])
            standalone_run, standalone = runs['standalone']
            self.assertTrue(standalone['admissible'], standalone['problems'])
            self.assertEqual(standalone['outcomes'], {'ok': 12})
            self.assertEqual(standalone_run['daemon_instances'], [])
            daemon_pid = shared_run['daemon_instances'][0][0]
            self.assertFalse(fanout.alive(daemon_pid), 'the daemon %d is still running' % daemon_pid)
            with redirect_stdout(io.StringIO()):
                fanout.main(['compare', '--baseline', str(Path(root) / 'standalone'), '--candidate',
                             str(Path(root) / 'shared'), '--out', str(Path(root) / 'compare.md')])
            self.assertIn('**Verdict:', (Path(root) / 'compare.md').read_text())


if __name__ == '__main__':
    unittest.main()
````

- [ ] **Step 2: Run them and watch them fail**

```bash
python3 -m unittest discover -s scripts/soak -p 'test_fanout*.py' > "$LOG_DIR/t3-red.log" 2>&1; echo "rc=$?"
tail -5 "$LOG_DIR/t3-red.log"
```

Expected: `ModuleNotFoundError: No module named 'fanout'` for `test_fanout`, `test_fanout_gate` and `test_fanout_live`, `Ran 3 tests`, `FAILED (errors=3)`.

- [ ] **Step 3: Implement the driver**

Create `scripts/soak/fanout.py`:

````python
#!/usr/bin/env python3
"""Fan-out latency driver for prism-mcp (shared-daemon spec sections 6.2 and 8 step 2).

    fanout.py run --bin BIN --corpus NAME=PATH --cache-dir DIR --mode shared|standalone --out DIR
                  [--clients 8] [--workloads DIR] [--refresh-at 0.5 | --no-refresh] [--timeout 1800]
    fanout.py compare --baseline DIR [--baseline DIR ...] --candidate DIR [--candidate DIR ...]
                      --out FILE.md [--json FILE.json]

`run` starts CLIENTS prism-mcp processes on one corpus: with --mode shared each is a shim
(`--shared`) of one daemon in a runtime directory of the run's own, which `run` stops at the end
with `daemon-shutdown`; with --mode standalone each holds an index of its own. Every client
initializes and warms up (its first workload call, retried while the index is not ready); then
all start the corpus's pinned workload together, each from its own offset, one call at a time,
and each call is timed from its write to its response. Client 0 also calls refresh_index once,
REFRESH_AT of the way through its calls. The run directory holds run.json (provenance, host,
memory, daemon instances), calls.jsonl (one line per timed call) and summary.json.

`compare` judges candidate runs against baseline runs of the same corpus and workload under the
pinned rule (P95_MAX_RATIO, P99_MAX_RATIO on the medians across runs; every run admissible).

Python 3.9, standard library only. Unit tests:
    python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import argparse
import hashlib
import json
import math
import os
import re
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / 'memreport'))
from memreport import WORKLOADS, bind_workload, corpus_provenance, host_state, parse_corpora  # noqa: E402
from soak import PRESSURE_CRITICAL, alive, footprints, read_pressure  # noqa: E402

PROTOCOL = '2025-11-25'
# Settings a client would otherwise inherit from this shell; each run starts from the defaults.
CLEARED_ENV = ('PRISM_MCP_STRUCTURED_CONTENT', 'PRISM_MCP_CONCISE_SHAPE', 'PRISM_MCP_MAX_RESULT_CHARS',
               'PRISM_MCP_SHARED', 'PRISM_MCP_RUNTIME_DIR', 'PRISM_MCP_MEM_REPORT')
WARMUP_RETRY_SECS = 1.0
# How often the run reads the host's memory pressure level (review round 1, sol 4).
PRESSURE_INTERVAL = 1.0
REACHED = re.compile(r'prism-mcp: shared daemon pid (\d+) \(started (\d+),')
# The pinned rule (owner question 7 of the S6 plan): the candidate's median p95 and p99 over its
# runs may exceed the baseline's by at most these factors.
P95_MAX_RATIO = 1.10
P99_MAX_RATIO = 1.25


class ClientGone(Exception):
    pass


def percentile(values, fraction):
    """Nearest rank: the smallest value with at least `fraction` of the values at or below it."""
    if not values:
        return None
    ordered = sorted(values)
    return ordered[max(1, math.ceil(fraction * len(ordered))) - 1]


def outcome(response):
    """ok; tool_error (the tool answered isError); retryable (warming, indexing, build_failed:
    `_meta.prism/retryable`); rpc_error (a JSON-RPC error)."""
    if 'error' in response:
        return 'rpc_error'
    result = response.get('result') or {}
    if (result.get('_meta') or {}).get('prism/retryable') is True:
        return 'retryable'
    return 'tool_error' if result.get('isError') else 'ok'


def rotated(items, client, clients):
    """`items` starting at this client's share of the way through, wrapping around."""
    start = (client * len(items)) // clients
    return items[start:] + items[:start]


class Client:
    """One prism-mcp process with piped standard input and output; one request at a time."""

    def __init__(self, index, argv, env, stderr_path):
        self.index = index
        self.stderr_path = stderr_path
        with open(stderr_path, 'wb') as stderr:
            self.process = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr,
                                            env=env)
        self.next_id = 1
        self.server = None

    def send(self, message):
        self.process.stdin.write((json.dumps(message, separators=(',', ':')) + '\n').encode())
        self.process.stdin.flush()

    def call(self, method, params=None):
        """(response, seconds from the write to the response)."""
        ident = self.next_id
        self.next_id += 1
        message = {'jsonrpc': '2.0', 'id': ident, 'method': method}
        if params is not None:
            message['params'] = params
        started = time.monotonic()
        self.send(message)
        while True:
            raw = self.process.stdout.readline()
            if not raw:
                raise ClientGone('client %d ended its output' % self.index)
            response = json.loads(raw)
            if 'id' not in response:
                continue
            if response['id'] != ident:
                raise ClientGone('client %d answered id %r to request %d' % (self.index, response['id'], ident))
            return response, time.monotonic() - started

    def stderr(self):
        return Path(self.stderr_path).read_text(errors='replace')


def client_argv(binary, repo, cache_dir, mode):
    argv = [str(binary), '--repo', str(repo), '--cache-dir', str(cache_dir), '--first-call-wait', '600']
    return argv + (['--shared', '--daemon-start-timeout', '120'] if mode == 'shared' else [])


def drive(client, workload, clients, refresh_at, deadline, barrier, clock, records):
    """Initialize, warm up, wait for every client, then run the timed workload."""
    hello, _ = client.call('initialize', {'protocolVersion': PROTOCOL, 'capabilities': {},
                                          'clientInfo': {'name': 'fanout', 'version': '1'}})
    client.server = (hello.get('result') or {}).get('serverInfo')
    client.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
    while True:
        response, _ = client.call('tools/call', workload[0]['params'])
        if outcome(response) != 'retryable':
            break
        if time.monotonic() > deadline:
            raise ClientGone('client %d: the index was not ready before the timeout' % client.index)
        time.sleep(WARMUP_RETRY_SECS)
    # Every other tool of the workload once, so that no tool's lazily built state is built inside
    # the timed window by each standalone server (review round 1, Opus S6).
    seen = {workload[0]['params'].get('name')}
    for request in workload:
        if request['params'].get('name') not in seen:
            seen.add(request['params'].get('name'))
            client.call('tools/call', request['params'])
    barrier.wait()
    barrier.wait()
    refresh_seq = int(len(workload) * refresh_at) if client.index == 0 and refresh_at is not None else None
    for seq, request in enumerate(rotated(workload, client.index, clients)):
        if seq == refresh_seq:
            start = time.monotonic() - clock[0]
            response, seconds = client.call('tools/call', {'name': 'refresh_index', 'arguments': {}})
            records.append({'client': client.index, 'seq': seq, 'tool': 'refresh_index', 'refresh': True,
                            'start': round(start, 6), 'ms': round(seconds * 1000, 3), 'outcome': outcome(response)})
        start = time.monotonic() - clock[0]
        response, seconds = client.call('tools/call', request['params'])
        records.append({'client': client.index, 'seq': seq, 'tool': request['params'].get('name'),
                        'refresh': False, 'start': round(start, 6), 'ms': round(seconds * 1000, 3),
                        'outcome': outcome(response)})


def memory(pids):
    sizes = footprints(sorted(pids))
    return {'pids': sorted(pids), 'footprint_bytes': sizes, 'total_bytes': sum(sizes.values())}


class PressureWatch:
    """Reads the memory pressure level at start, every PRESSURE_INTERVAL seconds until stop, and
    at stop, keeping every reading; calls `on_critical` once at the first critical reading."""

    def __init__(self, on_critical):
        self.readings, self.fired, self.on_critical = [], False, on_critical
        self.stopped = threading.Event()
        self.thread = threading.Thread(target=self.loop, daemon=True)

    def read(self):
        level = read_pressure()
        self.readings.append([round(time.time(), 3), level])
        if level is not None and level >= PRESSURE_CRITICAL and not self.fired:
            self.fired = True
            self.on_critical()

    def loop(self):
        while not self.stopped.wait(PRESSURE_INTERVAL):
            self.read()

    def start(self):
        self.read()
        self.thread.start()

    def stop(self):
        self.stopped.set()
        self.thread.join()
        self.read()

    def report(self, required):
        levels = [level for _, level in self.readings]
        known = [level for level in levels if level is not None]
        problems = []
        if any(level >= PRESSURE_CRITICAL for level in known):
            problems.append('critical memory pressure (level %d) during the run; its clients were stopped' % max(known))
        if required and len(known) != len(levels):
            problems.append('the memory pressure level could not be read %d of %d times'
                            % (len(levels) - len(known), len(levels)))
        return {'required': required, 'readings': self.readings, 'max': max(known) if known else None,
                'unavailable': len(levels) - len(known), 'problems': problems}


def refresh_overlap(records):
    """The timed calls of other clients whose time overlaps the refresh's: how many, and the
    longest (review round 1, Opus W1: a refresh's stall reaches at most a few calls, which no
    percentile of the whole run shows)."""
    refresh = [r for r in records if r['refresh']]
    if not refresh:
        return {'calls': 0, 'max_ms': None, 'refresh_ms': None}
    first = refresh[0]
    begin, end = first['start'], first['start'] + first['ms'] / 1000.0
    over = [r['ms'] for r in records if not r['refresh'] and r['client'] != first['client']
            and r['start'] < end and r['start'] + r['ms'] / 1000.0 > begin]
    return {'calls': len(over), 'max_ms': max(over) if over else None, 'refresh_ms': first['ms']}


def summarize(records, run):
    timed = [r for r in records if not r['refresh']]
    counts = {}
    for record in timed:
        counts[record['outcome']] = counts.get(record['outcome'], 0) + 1
    ms = [r['ms'] for r in timed]
    refresh = [r for r in records if r['refresh']]
    problems = list(run.get('failures', []))
    if counts.get('retryable') or counts.get('rpc_error'):
        problems.append('timed calls were answered retryable or with a JSON-RPC error: %s' % counts)
    if len(timed) != run['clients'] * run['workload']['requests']:
        problems.append('%d of %d timed calls were made' % (len(timed), run['clients'] * run['workload']['requests']))
    if run['mode'] == 'shared' and len(run.get('daemon_instances', [])) != 1:
        problems.append('the shims reached %d daemon instances, not 1' % len(run.get('daemon_instances', [])))
    profile = 'gate' if run.get('refresh_at') is not None else 'diagnostic'
    if profile == 'gate' and [r['outcome'] for r in refresh] != ['ok']:
        problems.append('a gating run needs exactly one successful refresh_index; it had %s'
                        % ([r['outcome'] for r in refresh] or 'none'))
    problems += (run.get('pressure') or {}).get('problems', [])
    return {'calls': len(timed), 'outcomes': counts, 'profile': profile, 'refresh_overlap': refresh_overlap(records), 'p50_ms': percentile(ms, 0.50), 'p95_ms': percentile(ms, 0.95),
            'p99_ms': percentile(ms, 0.99), 'max_ms': max(ms) if ms else None,
            'mean_ms': round(statistics.mean(ms), 3) if ms else None,
            'refresh_ms': refresh[0]['ms'] if refresh else None,
            'refresh_outcome': refresh[0]['outcome'] if refresh else None,
            'admissible': not problems, 'problems': problems}


def stop_daemon(binary, repo, cache_dir, runtime_base, instances, timeout=60):
    """Ask the run's daemon to drain (it has no client left) and wait until its process is gone."""
    try:
        done = subprocess.run([str(binary), 'daemon-shutdown', '--repo', str(repo), '--cache-dir', str(cache_dir),
                               '--runtime-dir', str(runtime_base), '--timeout', str(timeout)],
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout + 30)
        code, output = done.returncode, done.stdout.decode('utf-8', 'replace')[-2000:]
    except (OSError, subprocess.TimeoutExpired) as error:
        code, output = None, str(error)
    left = []
    for pid, _ in instances:
        give_up = time.monotonic() + timeout
        while time.monotonic() < give_up and alive(pid):
            time.sleep(0.1)
        if alive(pid):
            os.kill(pid, signal.SIGTERM)
            left.append(pid)
    return {'exit': code, 'output': output, 'terminated': left}


def cmd_run(args):
    out = Path(args.out)
    if any((out / name).exists() for name in ('run.json', 'calls.jsonl', 'summary.json')):
        raise SystemExit('run directory already has results; use a new --out')
    corpora = parse_corpora([args.corpus])
    (name, repo), = corpora.items()
    provenance = corpus_provenance(repo)
    data, digest = bind_workload(name, args.workloads, provenance['commit'])
    workload = [json.loads(line) for line in data.decode().splitlines() if line.strip()]
    refresh_at = None if args.no_refresh else args.refresh_at
    (out / 'clients').mkdir(parents=True, exist_ok=True)
    runtime_base = Path(tempfile.mkdtemp(prefix='fanout', dir='/tmp')) if args.mode == 'shared' else None
    env = {key: value for key, value in os.environ.items() if key not in CLEARED_ENV}
    if runtime_base:
        env['PRISM_MCP_RUNTIME_DIR'] = str(runtime_base)
    run = {'mode': args.mode, 'clients': args.clients, 'corpus': dict(provenance, name=name),
           'workload': digest, 'refresh_at': refresh_at, 'bin': str(args.bin),
           'bin_sha256': hashlib.sha256(Path(args.bin).read_bytes()).hexdigest(),
           'argv': client_argv(args.bin, repo, args.cache_dir, args.mode), 'host_before': host_state(),
           'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'failures': [],
           'profile': 'gate' if refresh_at is not None else 'diagnostic'}
    clients = [Client(i, run['argv'], env, out / 'clients' / ('%d.stderr' % i)) for i in range(args.clients)]
    watch = PressureWatch(lambda: [c.process.kill() for c in clients])
    watch.start()
    deadline = time.monotonic() + args.timeout
    watchdog = threading.Timer(args.timeout, lambda: [c.process.kill() for c in clients])
    watchdog.start()
    barrier = threading.Barrier(args.clients + 1)
    clock = [0.0]
    records = []

    def body(client):
        try:
            drive(client, workload, args.clients, refresh_at, deadline, barrier, clock, records)
        except (ClientGone, OSError, ValueError, threading.BrokenBarrierError) as error:
            run['failures'].append('%s: %s' % (type(error).__name__, error))
            barrier.abort()

    threads = [threading.Thread(target=body, args=(client,)) for client in clients]
    for thread in threads:
        thread.start()
    try:
        # Bounded, so that no failure a client did not report can hold the run past its timeout.
        barrier.wait(timeout=max(1.0, deadline - time.monotonic()))
        instances = sorted({(int(m.group(1)), int(m.group(2))) for c in clients for m in REACHED.finditer(c.stderr())})
        run['memory_after_warmup'] = memory([c.process.pid for c in clients] + [pid for pid, _ in instances])
        clock[0] = time.monotonic()
        barrier.wait(timeout=max(1.0, deadline - time.monotonic()))
    except threading.BrokenBarrierError:
        pass
    for thread in threads:
        thread.join()
    watch.stop()
    run['pressure'] = watch.report(args.pressure == 'required')
    instances = sorted({(int(m.group(1)), int(m.group(2))) for c in clients for m in REACHED.finditer(c.stderr())})
    run['daemon_instances'] = instances
    run['memory_after_workload'] = memory([c.process.pid for c in clients] + [pid for pid, _ in instances])
    run['server'] = next((c.server for c in clients if c.server), None)
    for client in clients:
        try:
            client.process.stdin.close()
        except OSError:
            pass
    run['exits'] = []
    for client in clients:
        try:
            run['exits'].append(client.process.wait(timeout=60))
        except subprocess.TimeoutExpired:
            client.process.kill()
            client.process.wait()
            run['exits'].append('killed')
        client.process.stdout.close()
    watchdog.cancel()
    if runtime_base:
        run['shutdown'] = stop_daemon(args.bin, repo, args.cache_dir, runtime_base, instances)
        # The daemon's own log is the evidence for an inadmissible shared run (review round 1, Opus S6).
        logs = sorted((runtime_base / 'prism-mcpd').glob('*.log'))
        if logs:
            (out / 'daemon-logs').mkdir(exist_ok=True)
            for log in logs:
                shutil.copyfile(log, out / 'daemon-logs' / log.name)
        shutil.rmtree(runtime_base, ignore_errors=True)
    run['host_after'] = host_state()
    run['finished_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
    records.sort(key=lambda r: (r['client'], r['seq'], not r['refresh']))
    with open(out / 'calls.jsonl', 'w') as calls:
        for record in records:
            calls.write(json.dumps(record, sort_keys=True) + '\n')
    summary = summarize(records, run)
    (out / 'run.json').write_text(json.dumps(run, indent=1, sort_keys=True, default=str))
    (out / 'summary.json').write_text(json.dumps(summary, indent=1, sort_keys=True))
    print('%s %s x%d: p50 %s ms, p95 %s ms, p99 %s ms, %s' % (
        name, args.mode, args.clients, summary['p50_ms'], summary['p95_ms'], summary['p99_ms'],
        'admissible' if summary['admissible'] else 'NOT admissible: %s' % '; '.join(summary['problems'])))


def load_arm(directories):
    arm = []
    for directory in directories:
        run = json.loads((Path(directory) / 'run.json').read_text())
        summary = json.loads((Path(directory) / 'summary.json').read_text())
        arm.append({'dir': str(directory), 'run': run, 'summary': summary})
    return arm


def judge(baseline, candidate, allow_client_mismatch=False):
    """The pinned rule's verdict and the figures it rests on. Never raises on a run without
    percentiles (review round 1, Opus W8): it fails the comparison."""
    entries = baseline + candidate
    problems = ['%s is not admissible: %s' % (entry['dir'], '; '.join(entry['summary']['problems']))
                for entry in entries if not entry['summary']['admissible']]
    problems += ['%s is a diagnostic run (no refresh_index); only gating runs are compared' % entry['dir']
                 for entry in entries if entry['run'].get('refresh_at') is None]
    keys = {(e['run']['corpus'].get('commit'), e['run']['workload']['sha256']) for e in entries}
    if len(keys) != 1:
        problems.append('the runs are not of one corpus commit and workload: %s' % sorted(keys))
    if len({e['run'].get('refresh_at') for e in entries}) > 1:
        problems.append('the runs refresh at different points: %s' % sorted({str(e['run'].get('refresh_at'))
                                                                             for e in entries}))
    if not allow_client_mismatch and len({e['run']['clients'] for e in entries}) > 1:
        problems.append('the runs have different numbers of clients: %s' % sorted({e['run']['clients'] for e in entries}))
    problems += ['%s ran on a corpus with uncommitted changes' % e['dir'] for e in entries if e['run']['corpus'].get('dirty')]
    figures = {}
    for name, arm in (('baseline', baseline), ('candidate', candidate)):
        figures[name] = {}
        for stat in ('p50_ms', 'p95_ms', 'p99_ms'):
            values = [e['summary'].get(stat) for e in arm]
            known = all(isinstance(value, (int, float)) for value in values) and values
            figures[name][stat] = statistics.median(values) if known else None
    if None in figures['baseline'].values() or None in figures['candidate'].values():
        problems.append('a run has no percentiles (no timed call was made)')
        p95 = p99 = None
    else:
        p95 = round(figures['candidate']['p95_ms'] / figures['baseline']['p95_ms'], 4)
        p99 = round(figures['candidate']['p99_ms'] / figures['baseline']['p99_ms'], 4)
    passed = not problems and p95 <= P95_MAX_RATIO and p99 <= P99_MAX_RATIO
    return {'pass': passed, 'problems': problems, 'figures': figures, 'p95_ratio': p95, 'p99_ratio': p99,
            'rule': {'p95_max_ratio': P95_MAX_RATIO, 'p99_max_ratio': P99_MAX_RATIO}}


def calibrate(first, second):
    """An A/A calibration: two arms of one configuration pass only when each passes against the
    other, so that their spread is within the bound whichever way it falls (review round 2, Opus
    W-A: a one-way comparison passed a 30 % spread)."""
    forward, backward = judge(first, second), judge(second, first)
    return {'pass': forward['pass'] and backward['pass'], 'forward': forward, 'backward': backward}


def render(baseline, candidate, verdict):
    lines = ['# prism-mcp fan-out latency', '', '| Arm | Run | Mode | Clients | p50 ms | p95 ms | p99 ms | max ms | '
             'refresh ms | calls overlapping the refresh | longest of them, ms | admissible |',
             '|---|---|---|---|---|---|---|---|---|---|---|---|']
    for name, arm in (('baseline', baseline), ('candidate', candidate)):
        for entry in arm:
            s, r = entry['summary'], entry['run']
            overlap = s.get('refresh_overlap') or {}
            lines.append('| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |' % (
                name, entry['dir'], r['mode'], r['clients'], s['p50_ms'], s['p95_ms'], s['p99_ms'], s.get('max_ms'),
                s.get('refresh_ms'), overlap.get('calls'), overlap.get('max_ms'), 'yes' if s['admissible'] else 'no'))
    lines += ['', 'Medians across runs: baseline %s; candidate %s.' % (verdict['figures']['baseline'],
                                                                      verdict['figures']['candidate']),
              'Ratios: p95 %s (at most %.2f), p99 %s (at most %.2f). The refresh overlap is reported, not gated.'
              % (verdict['p95_ratio'], P95_MAX_RATIO, verdict['p99_ratio'], P99_MAX_RATIO),
              '', '**Verdict: %s**' % ('PASS' if verdict['pass'] else 'FAIL')]
    lines += ['- %s' % problem for problem in verdict['problems']]
    return '\n'.join(lines) + '\n'


def cmd_compare(args):
    baseline, candidate = load_arm(args.baseline), load_arm(args.candidate)
    if args.both_directions:
        verdict = calibrate(baseline, candidate)
        Path(args.out).write_text('# A/A calibration: both directions must pass\n\n**Verdict: %s**\n\n## Forward\n\n%s\n'
                                  '## Backward\n\n%s' % ('PASS' if verdict['pass'] else 'FAIL',
                                                           render(baseline, candidate, verdict['forward']),
                                                           render(candidate, baseline, verdict['backward'])))
        if args.json:
            Path(args.json).write_text(json.dumps(verdict, indent=1, sort_keys=True))
        print('wrote %s: %s' % (args.out, 'PASS' if verdict['pass'] else 'FAIL'))
        return
    verdict = judge(baseline, candidate, args.allow_client_mismatch)
    Path(args.out).write_text(render(baseline, candidate, verdict))
    if args.json:
        Path(args.json).write_text(json.dumps(verdict, indent=1, sort_keys=True))
    print('wrote %s: %s' % (args.out, 'PASS' if verdict['pass'] else 'FAIL'))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    run = commands.add_parser('run')
    run.add_argument('--bin', required=True, type=Path)
    run.add_argument('--corpus', required=True, metavar='NAME=PATH')
    run.add_argument('--cache-dir', required=True, type=Path)
    run.add_argument('--mode', required=True, choices=('shared', 'standalone'))
    run.add_argument('--out', required=True)
    run.add_argument('--clients', type=int, default=8)
    run.add_argument('--workloads', type=Path, default=WORKLOADS)
    run.add_argument('--refresh-at', type=float, default=0.5)
    run.add_argument('--no-refresh', action='store_true')
    run.add_argument('--timeout', type=float, default=1800)
    run.add_argument('--pressure', choices=('required', 'off'), default='required' if sys.platform == 'darwin' else 'off',
                     help='whether an unreadable memory pressure level makes the run inadmissible (a critical '
                          'reading always stops it); default: required on macOS')
    run.set_defaults(handler=cmd_run)
    compare = commands.add_parser('compare')
    compare.add_argument('--baseline', action='append', required=True)
    compare.add_argument('--candidate', action='append', required=True)
    compare.add_argument('--out', required=True)
    compare.add_argument('--json')
    compare.add_argument('--both-directions', action='store_true',
                         help='an A/A calibration: pass only when each arm passes against the other')
    compare.add_argument('--allow-client-mismatch', action='store_true',
                         help='compare arms of different client counts (information only, never a gate)')
    compare.set_defaults(handler=cmd_compare)
    args = parser.parse_args(argv)
    if args.command == 'run' and not (args.clients >= 1 and 0.0 <= args.refresh_at < 1.0):
        parser.error('--clients must be at least 1 and --refresh-at in [0, 1)')
    args.handler(args)


if __name__ == '__main__':
    main()
````

- [ ] **Step 4: Run them and watch them pass, then once against the real binary**

```bash
python3 -m unittest discover -s scripts/soak -p 'test_*.py' > "$LOG_DIR/t3-green.log" 2>&1; echo "rc=$?"
tail -3 "$LOG_DIR/t3-green.log"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --features mcp --bin prism-mcp > "$LOG_DIR/t3-build.log" 2>&1; echo "build rc=$?"
PRISM_SOAK_BIN="$PWD/target/debug/prism-mcp" python3 -m unittest discover -s scripts/soak -p 'test_*.py' > "$LOG_DIR/t3-live.log" 2>&1; echo "rc=$?"
tail -3 "$LOG_DIR/t3-live.log"
pgrep -fl 'prism-mcp daemon'
```

Expected: `rc=0`, `Ran 66 tests`, `OK (skipped=1)` (the live test skipped); `build rc=0`; with the binary `Ran 66 tests`, `OK` (the live test ran: three shims of one daemon and three standalone servers on a three-file repository, every call `ok`, one successful refresh each, the daemon drained by `daemon-shutdown` and gone); `pgrep` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add scripts/soak/fanout.py scripts/soak/test_fanout.py scripts/soak/test_fanout_gate.py scripts/soak/test_fanout_live.py
git commit -m "feat(soak): S6 — fan-out latency driver, shared against standalone"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone to `scripts/soak/fanout.py`, run Step 4's first command, then `git checkout -- scripts/soak/fanout.py`.

| Mutant | Edit | Expected failing tests (observed) |
|---|---|---|
| F1 | `percentile`: `ordered[max(1, math.ceil(fraction * len(ordered))) - 1]` → `ordered[min(len(ordered) - 1, int(fraction * len(ordered)))]` | `test_percentile_is_nearest_rank` |
| F2 | `outcome`: the `prism/retryable` check → `if False:` | `test_outcome_tells_the_four_answers_apart` |
| F3 | `rotated`: `start = 0` (clients in lockstep) | `test_each_client_starts_its_own_share_of_the_way_through` |
| F4 | `summarize`: the call-count check → `if False:` | `test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible` |
| F5 | `summarize`: the one-daemon check → `if False:` | `test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible` |
| F6 | `judge`: `<=` → `<` on both ratios | `test_the_rule_passes_at_its_bounds_and_fails_past_them` |
| F7 | `judge`: `statistics.median` → `statistics.mean` | `test_medians_across_runs_are_compared` |
| F8 | `drive`: `refresh_at is not None` → `refresh_at is None` (no refresh) | `test_every_client_makes_every_call_and_client_0_refreshes_once` and three more |
| F9 | `drive`: the warm-up loop breaks on the first answer, retryable or not | `test_every_client_makes_every_call_and_client_0_refreshes_once` (three warming answers since round 1: with the extra warm-up call two were no longer enough, and the mutant first survived) |
| F10 | `cmd_run`: `barrier.abort()` removed from a failing client | `test_clients_that_die_while_warming_up_end_the_run` (its 60 s bound) |
| F11 | `cmd_run`: `stop_daemon` not called | `test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down` |
| F12 | `cmd_run`: `CLEARED_ENV` not applied | `test_clients_start_from_the_default_output_settings` |
| F13 | `summarize`: the one-successful-refresh check → `if False:` | `test_a_gating_run_needs_exactly_one_successful_refresh`, `test_a_refresh_answered_with_a_tool_error_makes_the_run_inadmissible` |
| F14 | `judge`: diagnostic runs not refused | `test_a_run_without_a_refresh_is_diagnostic_and_never_compared` |
| F15 | `refresh_overlap`: client 0's own calls counted | `test_the_calls_that_overlap_the_refresh_are_counted_and_their_longest_reported` (a synthetic client-0 call added after the mutant first survived) |
| F16 | `PressureWatch.read`: `on_critical` not called | `test_a_critical_pressure_reading_stops_and_invalidates_the_run` (asserting the clients were stopped, added after the mutant first survived) |
| F17 | `PressureWatch.report`: an unreadable level never a problem | `test_an_unavailable_pressure_reading_invalidates_a_run_that_requires_it` |
| F18 | `judge`: percentiles not checked before the median | `test_a_run_with_no_calls_fails_the_comparison_instead_of_crashing_it` |
| F19 | `judge`: client counts not compared | `test_runs_of_other_client_counts_or_refresh_points_are_not_compared` |
| F20 | `judge`: refresh points not compared | `test_runs_of_other_client_counts_or_refresh_points_are_not_compared` |
| F21 | `cmd_run`: the daemon's logs not copied | `test_a_shared_run_keeps_its_daemon_logs` |
| F22 | `drive`: the other tools not warmed | `test_every_tool_of_the_workload_is_warmed_before_the_barrier` |
| A1 | `calibrate`: only the forward direction | `test_a_calibration_passes_only_when_both_directions_pass` |

---

### Task 4: Shared-mode docs, the project's Codex configuration, CLAUDE.md; verification and the pull request

**Files:** Modify `docs/MCP.md`, `skills/prism-code-navigation/SKILL.md`, `CLAUDE.md`, `.codex/config.toml`.

**Before this task:** owner question 2 decides the runtime directory written in `.codex/config.toml` and in the docs' examples (`/tmp` below); owner question 4 decides whether `.codex/config.toml` is in this pull request at all (if not, skip its directive, and apply it at the soak's Phase 0).

- [ ] **Step 1: Write the docs and the configuration**

In `.codex/config.toml`, replace this:

````toml
[mcp_servers.prism]
command = "/Users/wesleyjinks/code/slicing/target/release/prism-mcp"
args = ["--repo", "/Users/wesleyjinks/code/slicing"]
startup_timeout_sec = 60
````

with this:

````toml
[mcp_servers.prism]
command = "/Users/wesleyjinks/code/slicing/target/release/prism-mcp"
args = ["--repo", "/Users/wesleyjinks/code/slicing", "--shared"]
startup_timeout_sec = 60
env = { PRISM_MCP_RUNTIME_DIR = "/tmp" }
````

In `CLAUDE.md`, replace this:

````markdown
have been recorded), or it did not see the drain. `--runtime-dir` is the
base, as `PRISM_MCP_RUNTIME_DIR` gives it.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
2. **Graph-based** (require the CPG; source of truth `SlicingAlgorithm::needs_cpg()`, src/slice.rs:210): `left_flow`, `full_flow`, `relevant_slice`, `conditioned_slice`, `barrier_slice`, `chop`, `taint`, `delta_slice`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `gradient_slice`, `provenance_slice`, `membrane_slice`, `echo_slice`
````

with this:

````markdown
have been recorded), or it did not see the drain. `--runtime-dir` is the
base, as `PRISM_MCP_RUNTIME_DIR` gives it.

Opting in and the soak: user documentation of shared mode is `docs/MCP.md` ("Shared daemon"). The
plugin's launcher (`scripts/prism-mcp-launch.sh`) adds `--shared` when `PRISM_MCP_SHARED=1` (unset,
empty or `0`: nothing; any other value: exit 2; an explicit `--shared`/`--standalone` wins); its tests
are `tests/mcp/launcher_test.rs`. The soak instrumentation is `scripts/soak/` (Python 3.9, standard
library; CI runs its hermetic tests in the build-checks job): `fanout.py` (N clients of one corpus
in shared or standalone mode, per-call latency, a pressure observer that stops the run at a
critical level, one required `refresh_index` and its overlap with other clients' calls, `compare`
under the pinned rule; also the spec §6.2 fan-out gate), `soak.py` (`sample` appends the prism-mcp
processes, whether each footprint was measured, and the host's swap, pressure and compressor;
`probe` checks that a launcher's command line starts a server; `--window-file` keeps a window's
start across resumes), `soak_judge.py` (`soak.py report --phase round|week`: each rule PASS, FAIL
or UNKNOWN when its evidence was not collected, a phase INADMISSIBLE without observed use: a usage
day is a day daemons spent CPU, and a round names its checkouts with `--round-repo` and needs a
reached daemon in every client log) and `prism-mcp-logged.sh` (logs a client's prism-mcp standard
error with time and pid). Run their tests with `python3 -m unittest discover -s scripts/soak -p
'test_*.py'`; `PRISM_SOAK_BIN=<prism-mcp>` adds a live run against the binary.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
2. **Graph-based** (require the CPG; source of truth `SlicingAlgorithm::needs_cpg()`, src/slice.rs:210): `left_flow`, `full_flow`, `relevant_slice`, `conditioned_slice`, `barrier_slice`, `chop`, `taint`, `delta_slice`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `gradient_slice`, `provenance_slice`, `membrane_slice`, `echo_slice`
````

In `docs/MCP.md`, replace this:

````markdown
| `--eager` | no | Build the repository index before answering `initialize`, preserving the historical synchronous startup. Use for explicit pre-warming; normally leave it off. |
| `--warm-at-startup` | no | Start the background index build when the process starts instead of on the first valid `tools/call`. Trades memory for a warm first call; every process that uses it holds the full index whether or not a tool is ever called. Conflicts with `--eager`. |
| `--first-call-wait <SECS>` | no | Maximum wait for the first valid `tools/call` while the background build finishes (default `20`, range `0`–`600`). `0` returns a warming result immediately. Accepted but ignored with `--eager`. |

> **The first index build warms a cache.** A cold whole-repo CPG build can take ~30 s on a large repo;
> later builds on an unchanged tree are near-instant (the cache is keyed by the canonical repo path + a
````

with this:

````markdown
| `--eager` | no | Build the repository index before answering `initialize`, preserving the historical synchronous startup. Use for explicit pre-warming; normally leave it off. |
| `--warm-at-startup` | no | Start the background index build when the process starts instead of on the first valid `tools/call`. Trades memory for a warm first call; every process that uses it holds the full index whether or not a tool is ever called. Conflicts with `--eager`. |
| `--first-call-wait <SECS>` | no | Maximum wait for the first valid `tools/call` while the background build finishes (default `20`, range `0`–`600`). `0` returns a warming result immediately. Accepted but ignored with `--eager`. |
| `--shared` | no | Serve through the shared per-repository daemon (a shim of a few MB) instead of an index of this process's own; opt-in. See [Shared daemon](#shared-daemon---shared-opt-in-macos-and-linux). |
| `--standalone` | no | An index of this process's own (the default). |

> **The first index build warms a cache.** A cold whole-repo CPG build can take ~30 s on a large repo;
> later builds on an unchanged tree are near-instant (the cache is keyed by the canonical repo path + a
````

In `docs/MCP.md`, replace this:

````markdown
| `PRISM_MCP_MAX_RESULT_CHARS` | `80000` | Wire byte cap per tool result (floor `12000`). |
| `PRISM_MCP_STRUCTURED_CONTENT` | `omit-default-path` | `omit-default-path` (the default) drops `structuredContent` from the wire on the default (`canonical_json`) path — `content[0].text` already carries the identical JSON, so nothing is lost, only a redundant second copy (~31% of the result). Agent views (`format: agent_markdown` / `agent_json`) always keep `structuredContent`; it is their only canonical-Evidence carrier once `content_text` has been rewritten into prose. `always` opts back into repeating canonical Evidence in both `content[0].text` and `structuredContent` (the pre-2026-07-03 shape) for clients that read only `structuredContent`. |
| `PRISM_MCP_CONCISE_SHAPE` | `slim` | Shape of items in `Verbosity::Concise` results (Concise is the MCP default when a tool call omits `verbosity`). `slim` (the default) drops each item's `symbol` byte-offset/`ordinal` fields, drops the separate `location` field when it duplicates the symbol's file/line span, and omits `snippet` when null (instead of serializing `"snippet": null`). `Verbosity::Detailed` and agent views are never affected. `legacy` opts back into the pre-2026-07-03 item shape, byte-identical to historical output. |

The `slim` transform only touches `items`; it never reaches graph-carried results —
`nav_ego_graph` / `nav_repo_map` payloads live under `graph`, untouched by design.
````

with this:

````markdown
| `PRISM_MCP_MAX_RESULT_CHARS` | `80000` | Wire byte cap per tool result (floor `12000`). |
| `PRISM_MCP_STRUCTURED_CONTENT` | `omit-default-path` | `omit-default-path` (the default) drops `structuredContent` from the wire on the default (`canonical_json`) path — `content[0].text` already carries the identical JSON, so nothing is lost, only a redundant second copy (~31% of the result). Agent views (`format: agent_markdown` / `agent_json`) always keep `structuredContent`; it is their only canonical-Evidence carrier once `content_text` has been rewritten into prose. `always` opts back into repeating canonical Evidence in both `content[0].text` and `structuredContent` (the pre-2026-07-03 shape) for clients that read only `structuredContent`. |
| `PRISM_MCP_CONCISE_SHAPE` | `slim` | Shape of items in `Verbosity::Concise` results (Concise is the MCP default when a tool call omits `verbosity`). `slim` (the default) drops each item's `symbol` byte-offset/`ordinal` fields, drops the separate `location` field when it duplicates the symbol's file/line span, and omits `snippet` when null (instead of serializing `"snippet": null`). `Verbosity::Detailed` and agent views are never affected. `legacy` opts back into the pre-2026-07-03 item shape, byte-identical to historical output. |
| `PRISM_MCP_RUNTIME_DIR` | the user runtime or temporary directory | Base of the shared daemon's runtime directory (`<base>/prism-mcpd`). Every launcher that should share a daemon must resolve the same one. |
| `PRISM_MCP_SHARED` | unset | Read by the plugin's launcher (`scripts/prism-mcp-launch.sh`), not by `prism-mcp`: `1` adds `--shared`; unset, empty or `0` adds nothing; any other value stops the launcher. |

The `slim` transform only touches `items`; it never reaches graph-carried results —
`nav_ego_graph` / `nav_repo_map` payloads live under `graph`, untouched by design.
````

In `docs/MCP.md`, replace this:

````markdown
the pre-flip wire shapes. The `initialize` response's `instructions` field states the
snapshot-freshness and agent-view notices once for the whole session; each tool description keeps
only a one-line pointer to it.

## Skills (the judgment layer)

````

with this:

````markdown
the pre-flip wire shapes. The `initialize` response's `instructions` field states the
snapshot-freshness and agent-view notices once for the whole session; each tool description keeps
only a one-line pointer to it.

## Shared daemon (`--shared`, opt-in; macOS and Linux)

Every MCP client process starts its own `prism-mcp`, and every one that answers a tool call holds the
whole index (1–3 GiB on a mid-size repository). Codex starts one per thread and the a2a-bridge fans out
reviewers, so a busy host can hold dozens of copies of one index. With `--shared` the process is
instead a **shim** of a few MB: it connects to one **daemon** per repository checkout (more exactly, per
checkout, prism build, cache directory and refresh policy), starting it if none is running, and passes
the MCP session through byte for byte. The daemon builds the index on the first tool call from any
client, serves every client from it, and exits 15 minutes after its last client has gone.

The default is still today's in-process server (`--standalone`). `--shared` is opt-in until a soak has
shown it holds under real fan-out.

### Opting in

- **Claude Code plugin:** set `PRISM_MCP_SHARED=1` in the environment Claude Code starts from (your
  shell profile). The plugin's launcher then adds `--shared`. Unset, empty or `0` changes nothing; any
  other value makes the launcher refuse to start the server, naming the variable, so that a typo never
  decides silently which mode a session gets. An explicit `--shared` or `--standalone` in the
  arguments wins over a valid value; a value the launcher does not accept is refused even then
  (the variable is checked first). With the variable set to `1`, `--no-cache`, `--eager`,
  `--owner-compiler` or `--owner-config` among the launcher's arguments is a startup error (each
  conflicts with `--shared`); add `--standalone` with them.
- **Manual Claude Code, Codex, Kiro:** add `"--shared"` to the server's arguments, for example in Codex:

  ```toml
  [mcp_servers.prism]
  command = "/abs/path/to/prism/target/release/prism-mcp"
  args = ["--repo", "/abs/path/to/your/repo", "--shared"]
  env = { PRISM_MCP_RUNTIME_DIR = "/tmp" }
  ```

  (`/tmp` suits a host with one user. On a shared host give each user a directory of their own,
  for example `$XDG_RUNTIME_DIR` or one under the home directory, in this example and the next.)
- **a2a-bridge:** add `"--shared"` to the `[[agents.mcp]]` arguments and set the runtime directory
  explicitly, so that every agent and your own shell agree on it:

  ```toml
  [[agents.mcp]]
  name = "prism"
  command = "/abs/path/to/prism/target/release/prism-mcp"
  args = ["--repo", "{cwd}", "--shared"]
  [[agents.mcp.env]]
  name = "PRISM_MCP_RUNTIME_DIR"
  value = "/tmp"
  ```

  Agents share a daemon only when their `--repo` resolves to the same checkout: each clone or worktree
  an implementer works in has a daemon of its own. A containerized agent has a daemon of its own
  inside its container; `--shared` saves nothing for a container that runs one agent.

`--shared` needs the nav cache and a lazy start: with `--no-cache`, `--eager` or `--owner-*` it is a
command-line error, never a quiet fallback.

### Every launcher must agree on the runtime directory, cache directory and refresh policy

The daemon's socket, locks, pid file and log live in `<runtime>/prism-mcpd/`, where `<runtime>` is
`$PRISM_MCP_RUNTIME_DIR` if set, else `$XDG_RUNTIME_DIR` (Linux), else the temporary directory
(`$TMPDIR` on macOS, `/tmp` on Linux). Two clients share a daemon only when they resolve the same
directory **and** pass the same `--cache-dir` (or none) and `--refresh-policy`, with the same prism
build. Claude Code passes its whole environment to the server, so its sessions use `$TMPDIR`; other
clients may start servers with a reduced environment. To be sure, set `PRISM_MCP_RUNTIME_DIR` to one
value in every launcher (and in the shell you run `daemon-status` from). A client that resolves
another directory does not fail: it starts a second daemon, with a second copy of the index.

The directory must be yours and private: owned by you, mode 0700, no access control list, and nothing
on its path that another user could replace. Two traps:

- `/tmp/prism-mcpd` is one name for every user of a Linux host without `$XDG_RUNTIME_DIR`; the second
  user's shims fail closed with a message naming `PRISM_MCP_RUNTIME_DIR`. Set it per user there.
- A `sudo -E prism-mcp --shared …` keeps your `PRISM_MCP_RUNTIME_DIR` or `$TMPDIR` and creates
  `prism-mcpd` owned by root; your own shims then fail closed ("owned by uid 0"). Remove that
  directory with `sudo rm -r`.

The socket path must fit in 103 bytes; a long `PRISM_MCP_RUNTIME_DIR` fails closed saying so.

### Flags

| Flag | Default | Meaning |
|---|---|---|
| `--shared` / `--standalone` | `--standalone` | Shim of the shared daemon / an index of this process's own. |
| `--daemon-start-timeout <SECS>` | `10` (1–600) | The one deadline for reaching a ready daemon, starting it included. |
| `--idle-ttl <SECS>` | `900` (1–86400) | Idle time before a daemon retires. **Only for a daemon this shim starts**; a shim that reaches a running daemon ignores it, silently. |
| `--max-clients <N>` | `64` (1–4096) | Connections a daemon admits at once. Same rule as `--idle-ttl`. |
| `--warm-at-startup` | off | Passed to a daemon this shim starts (it builds at once); otherwise ignored. |
| `--first-call-wait`, `PRISM_MCP_STRUCTURED_CONTENT`, `PRISM_MCP_CONCISE_SHAPE`, `PRISM_MCP_MAX_RESULT_CHARS` | as standalone | Per connection: each shim's own values apply to its own session. |

### What changes for a client

- **One index for everyone.** A `refresh_index` from one client changes what every client sees next,
  generation included; stale-index warnings and their evidence are the daemon's, not the session's.
- **One build.** The first tool call from any client starts the build, and every client waits on that
  build. While it runs a call is answered with a retryable result, as standalone: `status` `warming`,
  `_meta` `prism/retryable: true` and `prism/index_state: "warming"`; a failed build is
  `status: "build_failed"` with its `cause` and `prism/index_state: "failed"`, and the next call
  retries it. The daemon adds one answer of its own: `status: "indexing"` with `retry_after_secs: 5`,
  for a call whose own build attempt failed while a later attempt is already building; retry after
  that many seconds, or ignore it if a retry already succeeded.
- **The shim says which daemon it reached**, on its standard error:
  `prism-mcp: shared daemon pid <pid> (started <epoch seconds>, generation <g>) at <socket>`.
- **A session ends with its daemon.** If the daemon stops while a client is attached (a termination
  signal, its repository root removed or replaced, its runtime files replaced, or a crash), the shim
  exits 1 with `prism-mcp: the shared daemon closed the connection (shared daemon pid <pid>); this
  session cannot continue, …`; it never reconnects once MCP bytes have flowed. Restart the client's
  MCP server (in Claude Code, `/mcp`); the next shim starts a fresh daemon.
- **A crash ends every attached session.** A panic in a query, a refresh, the build or the daemon's
  control plane aborts the daemon (macOS writes a crash report,
  `~/Library/Logs/DiagnosticReports/prism-mcp-<date>.ips`, for each). If the daemon died during its
  index build, shims of that configuration fail closed for 60 s from when the first of them noticed,
  rather than start a daemon per shim that would die the same way; the minute is counted on the system
  clock, so a clock set back after the sighting lengthens it.

### When a shim fails closed

A shim that cannot reach a ready daemon by its deadline never builds a private index (under fan-out,
that is the memory incident this mode exists to prevent). It prints one line and exits 1:

`prism-mcp: --shared failed closed, no index was built: <reason>; key <64 hex>; runtime dir <dir>; socket <path>; pid file <contents>; log <path>; to run without the shared daemon: <the --standalone command>`

| Reason begins | What to do |
|---|---|
| `runtime directory … is owned by uid …`, `has mode …`, `could be replaced by another user`, `has an access control list` | Fix the directory as the message says, or set `PRISM_MCP_RUNTIME_DIR` to a directory of your own. |
| `daemon socket path … is … bytes` | A shorter `PRISM_MCP_RUNTIME_DIR` (the path can be at most 103 bytes). |
| `the daemon answered BUSY` | More clients than `--max-clients`. Stop the daemon (`daemon-shutdown` or `kill -TERM`) and let a shim with a higher `--max-clients` start the next. |
| `the daemon answered MISMATCH`, `READY for another identity` | A daemon of another build or configuration answers at this key's socket; see `daemon-status`. |
| `the socket's peer is uid …` | Another user's process holds the socket. Nothing was sent to it. |
| `no daemon answered READY before the deadline`, `another shim held the start lock` | The daemon is slow to start or wedged: read its log (the line names it); raise `--daemon-start-timeout`; a wedged daemon is named by its pid file, stop it with `kill -TERM <pid>`. |
| `the daemon this shim started, pid …, ended before READY`, `cannot start the daemon` | Read the log. After a rebuild of prism, a shim started from the old binary cannot start a daemon (on macOS the new binary computes another key and refuses; on Linux the old file is gone): restart the client. |
| `the last daemon for this key … died during its index build` | Wait out the minute and read the log for the build's failure. |

### Operating daemons: `daemon-status`, `daemon-shutdown`

```bash
prism-mcp daemon-status [--repo R [--cache-dir D] [--refresh-policy P]] [--runtime-dir BASE] [--timeout S]
prism-mcp daemon-shutdown --repo R [--cache-dir D] [--refresh-policy P] [--runtime-dir BASE] [--pid N --start-time T] [--timeout S]
```

`--runtime-dir` is the base, as `PRISM_MCP_RUNTIME_DIR` gives it (the files are in its `prism-mcpd`);
without it both commands resolve the directory as a shim does. `--repo` with `--cache-dir` and
`--refresh-policy` names this build's daemon of that configuration.

- **`daemon-status`** prints the runtime directory, then for each daemon its key, repository,
  build, pid, start time, admitted and pending clients, build state, generation and idle time; a
  daemon that does not answer, through its locks ("alive but not answering", "being started"); what a
  dead daemon left; and in one line the keys that hold nothing but lock files and logs. It reads and
  removes nothing, and asking does not keep a daemon of this build alive. It exits 1 when some key
  could not be looked at.
- **`daemon-shutdown`** asks one daemon to drain once it has no client left. It never disconnects a
  client, and the daemon keeps admitting new clients until its count reaches 0. Exit 0: that
  instance no longer serves the key, or it has accepted and will drain after its clients (the
  message says how many), or it drained. Exit 1: it could not ask (alive but not accepting
  connections, at its client limit, silent), the reply to the request was lost (it may have been
  recorded, and cannot be withdrawn: see `daemon-status`), or the drain was not seen within
  `--timeout`. With `--pid` and `--start-time` (from the shim's `shared daemon pid …` line) it asks
  only that instance, and leaves a newer one alone.
- **Daemons of another build** (the key covers the build, so every rebuild of prism starts new
  daemons) are listed by `daemon-status` and retire on their own after their clients and their idle
  time. To stop one now, `kill -TERM <pid>`: it drains at once, ending its attached sessions. A daemon
  built before `daemon-status` existed restarts its idle time at every `daemon-status` that looks at
  it, so polling keeps it alive; stop it with `kill -TERM <pid>`.

**Files.** In `<runtime>/prism-mcpd/`, per key (its first 16 hex digits): `.sock`, `.start` and
`.owner` (locks, never removed while anything may use them), `.pid` (`pid`, start time, build), `.log`
(the daemon's lines: truncated at start when over 1 MiB, capped at 8 MiB) and, while a build runs,
`.build`. Each rebuild of prism leaves the lock files and log of the old key behind; `daemon-status`
counts them. With no `prism-mcp` process running the whole `prism-mcpd` directory can be removed.

**The daemon** is started by shims as `prism-mcp daemon --repo <root> --cache-dir <base>
--refresh-policy <p> --key <64 hex> [--warm-at-startup] [--idle-ttl S] [--max-clients N]`, in a
session of its own, from `/`, with the shim's environment (but not `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`).
It exits 0 when it has drained, or at once and silently when another daemon already owns the key; it
exits non-zero with one `Error:` line in its log when it cannot start. Its log says
`prism-mcp daemon: pid N serves <key16> for <root> at <socket>` and, when it drains,
`pid N stopped: <reason>`, the reason one of: idle for the idle TTL with no client and no build (no
session was attached); an operator's daemon-shutdown, with no client left (none attached); a
termination signal, the repository root was lost or replaced, or its socket or owner lock file was
replaced (attached sessions end, as above). A crash leaves no `stopped` line.

## Skills (the judgment layer)

````

In `skills/prism-code-navigation/SKILL.md`, replace this:

````markdown
  `unavailable`. It never returns body text or edits source.
- **Graphs truncate (50 items by default, raise with `max_results` up to 1000; 80 KB byte cap)** and the
  **first call can take ~30 s** (building the CPG); instant after.
- **Read-only.** These tools never edit or run code. Make changes with your normal file tools.
````

with this:

````markdown
  `unavailable`. It never returns body text or edits source.
- **Graphs truncate (50 items by default, raise with `max_results` up to 1000; 80 KB byte cap)** and the
  **first call can take ~30 s** (building the CPG); instant after.
- **A retryable answer is not a result.** `_meta` `prism/retryable: true` with `status` `warming` or
  `indexing` means the index is still building: retry the same call (after `retry_after_secs`, when
  given). `build_failed` reports why; the next call retries the build. Under a shared server
  (`--shared`) another agent's `refresh_index` changes what you see too.
- **Read-only.** These tools never edit or run code. Make changes with your normal file tools.
````

- [ ] **Step 2: Check what the docs quote**

```bash
grep -c 'prism-mcp: --shared failed closed, no index was built' src/mcp/shim/diagnostic.rs docs/MCP.md
grep -c 'shared daemon pid' src/mcp/shim/run.rs docs/MCP.md
grep -n '"indexing"\|retry_after_secs\|INDEXING_RETRY_AFTER_SECS: u64 = 5' src/mcp/daemon/connection.rs
grep -n 'BUILD_CRASH_BACKOFF: Duration' src/mcp/shim/arbitrate.rs
git diff --stat 61c2a9e5 -- src .claude-plugin tests/fixtures/mcp/golden
```

Expected: each `grep -c` prints a count of at least 1 for both files; the connection grep shows the three; the backoff is 60 s; the last command prints nothing (no file under `src/`, `.claude-plugin/` or the goldens changed).

- [ ] **Step 3: Run the checks and the whole suite**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo fmt --check > "$LOG_DIR/s6-fmt.log" 2>&1; echo "fmt rc=$?"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check > "$LOG_DIR/s6-check.log" 2>&1; echo "check rc=$?"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check --features mcp > "$LOG_DIR/s6-checkmcp.log" 2>&1; echo "check mcp rc=$?"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets --features mcp -- -W clippy::all > "$LOG_DIR/s6-clippy.log" 2>&1; echo "clippy rc=$?"
grep -c 'launcher_test' "$LOG_DIR/s6-clippy.log"
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s6-full.log" 2>&1; echo "full suite rc=$?"
grep -c '^test result' "$LOG_DIR/s6-full.log"
grep '^test result' "$LOG_DIR/s6-full.log" | awk '{p+=$4; f+=$6; i+=$8} END {print p" passed, "f" failed, "i" ignored"}'
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s6-doc.log" 2>&1; echo "doc tests rc=$?"
PRISM_SOAK_BIN="$PWD/target/debug/prism-mcp" python3 -m unittest discover -s scripts/soak -p 'test_*.py' > "$LOG_DIR/s6-py.log" 2>&1; echo "soak tests rc=$?"
python3 -m unittest discover -s scripts/memreport -p 'test_*.py' > "$LOG_DIR/s6-memreport.log" 2>&1; echo "memreport tests rc=$?"
for i in $(seq 1 10); do nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp launcher > "$LOG_DIR/s6-rep-launcher-$i.log" 2>&1; PRISM_SOAK_BIN="$PWD/target/debug/prism-mcp" python3 -m unittest discover -s scripts/soak -p 'test_*.py' > "$LOG_DIR/s6-rep-soak-$i.log" 2>&1; done
grep -h '^test result' "$LOG_DIR"/s6-rep-launcher-*.log | sed 's/finished.*//' | sort | uniq -c
grep -h '^OK\|^FAILED' "$LOG_DIR"/s6-rep-soak-*.log | sort | uniq -c
pgrep -fl 'prism-mcp daemon'
```

Expected: `fmt rc=0` and nothing printed; `check rc=0`, `check mcp rc=0`, no `warning`; `clippy rc=0` and `0` lines naming `launcher_test`; `full suite rc=0`, `31` result lines, `6233 passed, 0 failed, 1 ignored` (V4); `doc tests rc=0`; `soak tests rc=0` (`Ran 66 tests`, `OK`); `memreport tests rc=0` (`Ran 349 tests`); the repeats `10 test result: ok. 7 passed` and `10 OK`; `pgrep` prints nothing.

- [ ] **Step 4: Run the launcher's and the soak tools' tests on Linux**

In a command of its own first: `df -h /System/Volumes/Data` (at least 50 GiB free). Then:

```bash
mkdir -p "$LOG_DIR/linux"
cat > "$LOG_DIR/linux.sh" <<'EOF'
#!/bin/bash
fails=0
nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --features mcp --bin prism-mcp > /logs/build.log 2>&1 || exit 100
for i in $(seq 1 10); do
  nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp launcher > "/logs/launcher-$i.log" 2>&1 || fails=$((fails+1))
  PYTHONDONTWRITEBYTECODE=1 PRISM_SOAK_BIN=/target/debug/prism-mcp python3 -m unittest discover -s scripts/soak -p 'test_*.py' > "/logs/soak-$i.log" 2>&1 || fails=$((fails+1))
done
for p in /proc/[0-9]*; do grep -qs 'prism-mc[p]' "$p/cmdline" && echo "left: $(tr '\0' ' ' < "$p/cmdline")"; done
exit $fails
EOF
(
  trap 'docker volume rm s6-target s6-registry >/dev/null' EXIT
  docker run --rm --cpus 4 --memory 8g -e CARGO_BUILD_JOBS=4 -e CARGO_TARGET_DIR=/target \
    -v "$PWD":/src:ro -v s6-target:/target -v s6-registry:/usr/local/cargo/registry \
    -v "$LOG_DIR/linux.sh":/linux.sh:ro -v "$LOG_DIR/linux":/logs -w /src --entrypoint bash \
    a2a-toolchain:latest /linux.sh
) > "$LOG_DIR/s6-linux.out" 2>&1; echo "failed Linux runs: $?"
cat "$LOG_DIR/s6-linux.out"
grep -h '^test result' "$LOG_DIR"/linux/launcher-*.log | sed 's/finished.*//' | sort | uniq -c
grep -h '^Ran\|^OK\|^FAILED' "$LOG_DIR"/linux/soak-*.log | sed 's/ in [0-9.]*s//' | sort | uniq -c
```

Expected: `failed Linux runs: 0`, no `left:` line in `s6-linux.out`; `10 test result: ok. 7 passed; 0 failed` (52 filtered out); `10 Ran 66 tests` and `10 OK` (the live test runs; `--pressure` defaults to `off` off macOS, and the image has no `ps` or `pgrep`, which these tests do not need).

- [ ] **Step 5: Commit and open the pull request**

```bash
git add docs/MCP.md skills/prism-code-navigation/SKILL.md CLAUDE.md .codex/config.toml
git commit -m "docs(mcp): S6 — shared-daemon docs, the project's Codex opt-in, soak tools in CLAUDE.md"
```

The S6-2 PR body states, in this order (S6-1's, Task 2 Step 7, the same shape): (1) **Goal:** the lane goal; S6 is the opt-in from every launcher, the docs the earlier slices left to it, and the soak's instruments; the soak is specified in the plan and is the controller's. (2) **Invariants:** nothing under `src/` or `.claude-plugin/` changes; opt-in stays opt-in (Global Constraints). (3) **Decisions:** D110–D130; SI-36–SI-43; the owner's answers to the questions (or "pending" for each that is). D110 (an environment variable for the plugin) and D118 (what counts as an index holder) are the ones a reviewer is most likely to want to change. (4) **Claims:** the table, with what no test holds. (5) **Totals:** Step 3 next to the base, Step 4, and the mutants as observed. (6) **Not verified:** x86_64 Linux before CI (CI runs the launcher's tests and, since review round 1, the soak tools' hermetic tests, D117); the soak (the controller's, after merge); a Codex session's environment (SI-42). (7) **Tier-A:** not run (no file it watches is touched). (8) **Reviews of the plan:** what each round folded; round 2's fold came after the two-round cap, without a third round, and is disclosed as such.

Review brief and cap as in the lane README (two rounds; Opus 5.5 and gpt-6.1-sol; the goal above; WRONG/SMELL and MATERIAL/IMMATERIAL; a fix and alternatives per finding and when it would not apply). Ask each reviewer to check the docs' fail-closed table and the operator's exit statuses line by line against the code (Review Focus 9).

---

## The soak (the lane's acceptance; the controller's, after this slice merges)

The soak is the gate for the default flip (spec §8 step 2, §11 item 2, §12 row B): "over one bridge review round plus one week of interactive use, record resident `prism-mcp` count and aggregate footprint …, shim fail-closed count (target 0), daemon restarts, and fan-out latency p95 vs. standalone. The flip does not wait for any memory reduction." It is a measurement, not code. It starts only after the owner has answered questions 2, 3, 5, 7, 8 and 9; the values below are the recommended answers.

### What it shows, and what would fail it

| Spec §8 step 2 asks for | Instrument | Phase | Pass (owner question 7, recommended) | What would fail it, or leave it unjudged |
|---|---|---|---|---|
| Resident `prism-mcp` count | `soak.py sample` (index holders per key, per repository, in total; shims; built daemons) | 2, 3 | `one_index_per_key` at every sample; in Phase 2 also `no_new_standalone_index` | FAIL: two daemons of one key at once (a split runtime directory); a standalone index holder that appeared during the round. UNKNOWN: a sample without a process table; a live standalone server whose footprint could not be measured |
| Aggregate footprint | the same samples (`phys_footprint` of every prism-mcp process; the host's swap, pressure level, compressor) | 2, 3 | Phase 2: `no_critical_pressure`; peaks reported against the incident's (29 processes, 34 GiB compressed) | FAIL: pressure at its critical level. UNKNOWN: a sample without a pressure reading |
| Footprint growth (owner request 2026-10-16, D131) | per-daemon trends from `soak.py sample` / `report` | 1, 2 report only; 3 gating | `no_footprint_growth`; baseline/max/last, MiB per residency hour, largest quiet-stretch growth always reported | FAIL: ≥ 1 h quiet stretch grows strictly more than max(200 MiB, 25% baseline); work growth reported only. UNKNOWN: required footprints, process table or CPU continuity missing. No baseline: no contribution |
| Shim fail-closed count, target 0 | `soak.py report` over Claude Code's MCP logs and `prism-mcp-logged.sh` logs | 2, 3 | `no_fail_closed`, `no_startup_error` | FAIL: a `prism-mcp: --shared failed closed` line, or a server that never started (`error: …`, the launcher's refusal). UNKNOWN: a named log source missing, or none named |
| Daemon restarts | daemon instances in the samples and in the shims' `shared daemon pid` lines; crash reports; restart-bound refusals | 2, 3 | `no_abort`, `no_restart_bound`; starts reported (SI-40) | FAIL: a `prism-mcp-*.ips` in the window; a `died during its index build` refusal. UNKNOWN: the crash-report directory unreadable |
| Fan-out latency p95 vs. standalone | `fanout.py run` and `compare` | 1 | A/A calibration within the bound; then excalidraw, 8 shims vs 8 standalone, 3 pairs in ABBA order, all admissible, median p95 ≤ 1.10×, median p99 ≤ 1.25×; the refresh's stall reported (SI-43) | FAIL: a ratio over its bound. Inadmissible run: no successful refresh, a critical or unreadable pressure reading, a lost client, a retryable answer after warm-up, two daemons |
| (use, so that a pass means something) | `soak.py report` | 2, 3 | Phase 2: ≥ 2 shims at once, a daemon reached in the logs, an index built; Phase 3: ≥ 5 usage days (D128) | INADMISSIBLE when not seen |

A phase's report says `**Verdict: PASS**`, `FAIL` or `INADMISSIBLE`. Only PASS counts. INADMISSIBLE is never read as a pass: the phase is repaired (the missing source named, the use made) and run again. The default flip (row B) is the owner's decision with the three phase reports in hand; "the flip does not wait for any memory reduction" (§8 step 2).

**Owner-tunable footprint defaults (2026-10-16).** `scripts/soak/soak_growth.py` names
`FOOTPRINT_BASELINE_QUIET_SECS = 600`, `FOOTPRINT_QUIET_STRETCH_SECS = 3600`,
`FOOTPRINT_REBUILD_CPU_SECS = 5.0`, `FOOTPRINT_GROWTH_MIN_BYTES = 200 * MIB`, and
`FOOTPRINT_GROWTH_BASELINE_FRACTION = 0.25`. Quiet uses the existing
`soak_judge.py` constants `USAGE_INTERVAL_CPU_SECS = 0.05` (rounded to microseconds) and
`USAGE_INTERVAL_MAX_SECS = 120`; built-index detection uses `HOLDER_MIN_BYTES = 100 * MIB`.
Changing these is an owner decision, not a reason to re-baseline saved evidence silently.

Only consecutive, observed endpoints establish quiet time. Work, missing footprints/CPU, a missing
pid, CPU rollback or an interval over 120 s breaks a stretch; missing required evidence is UNKNOWN,
never proof of quiet. A new stretch starts at the work interval's ending footprint, excluding cache
growth during that work. Baseline acquisition excludes the build and ten-minute warm-up. Rebuilds
start a new baseline epoch after another ten quiet minutes; earlier observed breaches survive resets.
The largest quiet-stretch growth includes short stretches for diagnosis, but only stretches at least
one hour gate. JSON retains baseline epochs; Markdown shows the latest baseline, max, last, endpoint
net growth per measured residency hour (including work/rebuilds) and largest quiet growth with duration.

The sampler passively reads newly appended `nav cache: …, rebuilding` lines in each daemon's
key-prefixed `.log` under `--runtime-dir` (or its runtime environment/default). Untimestamped content
already present at the first sighting, or after sampler restart, is not assigned to a current interval;
CPU bursts remain the fallback when logs are absent, inaccessible, or in another runtime directory.
It does not connect to a daemon to read those logs.

### Fixed inputs

- **Build:** one release `prism-mcp` built from **S6-2's head** (it contains S6-1; owner question 4: its `src/` is `main`'s and the merge's, since nothing under `src/` changes; the binary is the merge's in all but the commit it names), in a worktree of its own that stays for the whole soak (`$S`), installed as `$E/bin/prism-mcp` with its sha256. The bridge's entries name `$E/bin/prism-mcp`, never a working checkout's `target/` (owner question 5). The tools run from `$S/scripts/soak/`.
- **Corpora** (lane README "Facts"; clean, at these commits; checked before each phase with `git -C <corpus> rev-parse HEAD` and `git status --porcelain`): excalidraw `~/code/bench-repos/excalidraw` at `0642e72cfa2d`; slicing `~/code/prism-c1-corpus` at `7db8f1951166`. Workloads: `scripts/memreport/workloads/{excalidraw,slicing}.jsonl`, 200 requests each, bound to those commits by `MANIFEST.json`.
- **Host:** this Mac (24 GB, macOS 26). Every fan-out run records swap, compressor and load before and after, and the pressure level every second (D126).
- **Runtime directory:** `/tmp` as the base (owner question 2), so `/tmp/prism-mcpd`; the fan-out runs use directories of their own under `/tmp` and keep their daemon's log.
- **Evidence:** `E=~/prism-evidence/mcp-memory/s6-soak`, one directory per phase; `SHA256SUMS` written at each phase's end; never a second run into a directory that has results (the tools refuse).
- **The controller's shell does not keep state between commands**: every runbook step that needs a time or a process id writes it to a file under `$E` and reads it back (review round 1, Opus W7).

### Phase 0: deployment (outside the repository; owner question 5)

Variables, used by every phase. The controller's shell keeps nothing between commands, so every command block below starts with these lines (and, in Phase 1, with the `C=`, `run()` and `X=` lines too):

```bash
E=~/prism-evidence/mcp-memory/s6-soak; B="$E/bin/prism-mcp"; S=~/code/prism-s6-soak
P=~/.claude/plugins/cache/prism-dev/prism/3.1.2
```

1. **Build** (a command of its own first: `df -h /System/Volumes/Data`, at least 50 GiB):

```bash
mkdir -p "$E"/{bin,deploy/originals,deploy/snapshots,fanout,round,week,client-logs}
git -C ~/code/slicing fetch -q origin && git -C ~/code/slicing worktree add --detach "$S" <S6-2's head commit>
git -C "$S" rev-parse HEAD >> "$E/deploy/COMMIT"; git -C "$S" diff --stat origin/main -- src | tail -1 > "$E/deploy/SRC-DIFF"   # must be empty
cd "$S" && nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --release --features mcp --bin prism-mcp > "$E/deploy/build.log" 2>&1; echo "build rc=$?"
```

`SRC-DIFF` empty is the condition for calling this binary the merge's. If it is not empty (`main` gained `src/` changes after S6-2's base), merge `main` into S6-2's branch, push, and start step 1 again from the new head (review round 2, Opus S-E).

Install `$B` without ever writing over a file a process runs (review round 2, Opus S-C): the first install names it by its hash; a later build gets a new name and the phases that use it are written in `NOTES.md`; `SHA256SUMS` is appended, never rewritten.

```bash
H=$(shasum -a 256 "$S/target/release/prism-mcp" | cut -c1-12)
[ -e "$E/bin/prism-mcp-$H" ] || { cp "$S/target/release/prism-mcp" "$E/bin/.prism-mcp-$H.new" && mv "$E/bin/.prism-mcp-$H.new" "$E/bin/prism-mcp-$H"; }
[ -e "$B" ] || ln -s "prism-mcp-$H" "$B"   # the first build only; a later one is used by its own name
[ -e "$E/bin/prism-mcp-logged.sh" ] || cp "$S/scripts/soak/prism-mcp-logged.sh" "$E/bin/"
shasum -a 256 "$E/bin/prism-mcp-$H" "$E/bin/prism-mcp-logged.sh" >> "$E/deploy/SHA256SUMS"
```

2. **Keep the originals, once** (a second run of Phase 0 must never overwrite them, review round 1, Opus W6):

```bash
[ -e "$E/deploy/originals/SHA256SUMS" ] || {
  cp -p ~/code/slicing/target/release/prism-mcp "$E/deploy/originals/slicing-prism-mcp"
  cp -p "$P/target/release/prism-mcp" "$E/deploy/originals/plugin-prism-mcp"
  cp -p "$P/scripts/prism-mcp-launch.sh" "$E/deploy/originals/prism-mcp-launch.sh"
  cp -p ~/code/slicing/.codex/config.toml "$E/deploy/snapshots/slicing-codex-config.toml"
  for f in <each bridge configuration the soak will edit>; do cp -p "$f" "$E/deploy/snapshots/$(basename "$f")"; done
  (cd "$E/deploy/originals" && shasum -a 256 * > SHA256SUMS); (cd "$E/deploy/snapshots" && shasum -a 256 * > SHA256SUMS)
}
```

3. **Install** (each copy replaces the file; the originals are in `$E/deploy/originals`):

```bash
cp "$B" ~/code/slicing/target/release/prism-mcp.s6-new && mv ~/code/slicing/target/release/prism-mcp.s6-new ~/code/slicing/target/release/prism-mcp
cp "$B" "$P/target/release/prism-mcp.s6-new" && mv "$P/target/release/prism-mcp.s6-new" "$P/target/release/prism-mcp"
cp "$S/scripts/prism-mcp-launch.sh" "$P/scripts/prism-mcp-launch.sh.s6-new" && mv "$P/scripts/prism-mcp-launch.sh.s6-new" "$P/scripts/prism-mcp-launch.sh"
```

(A rename over the old file leaves running processes on their old inode.) Then, by the owner or with the owner's approval: `export PRISM_MCP_SHARED=1` and `export PRISM_MCP_RUNTIME_DIR=/tmp` in the shell profile Claude Code starts from (sessions started before keep their standalone servers until restarted); and in each bridge configuration the soak uses, every **host-run** `[[agents.mcp]]` prism entry becomes (owner question 3 drops `--cache-dir`):

```toml
[[agents.mcp]]
name = "prism"
command = "/Users/wesleyjinks/prism-evidence/mcp-memory/s6-soak/bin/prism-mcp-logged.sh"
args = ["/Users/wesleyjinks/prism-evidence/mcp-memory/s6-soak/client-logs/bridge.log", "/Users/wesleyjinks/prism-evidence/mcp-memory/s6-soak/bin/prism-mcp", "--repo", "{cwd}", "--shared"]
[[agents.mcp.env]]
name = "PRISM_MCP_RUNTIME_DIR"
value = "/tmp"
```

4. **Probe every soaked launcher** before anything uses it (D127); each must print `PASS`:

```bash
cd "$S"
env PRISM_MCP_SHARED=1 PRISM_MCP_RUNTIME_DIR=/tmp CLAUDE_PLUGIN_ROOT="$P" python3 scripts/soak/soak.py probe -- "$P/scripts/prism-mcp-launch.sh" ~/code/slicing
env PRISM_MCP_RUNTIME_DIR=/tmp python3 scripts/soak/soak.py probe -- ~/code/slicing/target/release/prism-mcp --repo ~/code/slicing --shared
env PRISM_MCP_RUNTIME_DIR=/tmp python3 scripts/soak/soak.py probe -- "$E/bin/prism-mcp-logged.sh" "$E/client-logs/probe.log" "$B" --repo ~/code/slicing --shared
```

(The second is the project's `.codex/config.toml` command; the third the bridge entry's, with `{cwd}` filled in.) A `FAIL` names the binary's error: fix it before going on. The probes start lazy daemons of `~/code/slicing` (no index is built), which retire after the idle TTL. **Then merge S6-2** (owner question 4; S6-1 may already be merged).

Record in `$E/deploy/NOTES.md`: the commit, the sha256s, which configurations were edited, the time.

**Rollback** (any time), in this order, so that no configuration ever passes `--shared` with a binary that rejects it (review round 1, sol 1, Opus W4; revised in review round 2, Opus W-C, S-B):

1. Configurations: restore each bridge configuration from `$E/deploy/snapshots/`; unset `PRISM_MCP_SHARED` and `PRISM_MCP_RUNTIME_DIR` in the profile. **Leave `.codex/config.toml` and `~/code/slicing/target/release/prism-mcp` as they are**: once S6-2 is on `main`, every checkout's `.codex/config.toml` passes `--shared` to that path, and the S6 build there serves standalone clients exactly as the old one did, so keeping it keeps every checkout's Codex working. (Before S6-2 merges, the checkouts' `.codex/config.toml` has no `--shared` and keeping the build changes nothing either.)
2. Sessions: end or restart every opted-in Claude Code session (in Claude Code, `/mcp` then reconnect) so that its server starts again from the restored environment.
3. Daemons: with the S6 binary, which stays: `"$B" daemon-status --runtime-dir /tmp`, then `"$B" daemon-shutdown --repo <root> --runtime-dir /tmp` for each daemon it lists; a daemon that still has clients (a session not yet restarted) does not drain on request: stop it with `kill -TERM <pid>`, which ends those sessions; check `pgrep -fl 'prism-mcp daemon'` prints nothing (Codex sessions of a merged checkout keep their own shared daemons; they are working configurations, not leftovers).
4. The plugin: restore `$E/deploy/originals/{plugin-prism-mcp,prism-mcp-launch.sh}` (checked against `$E/deploy/originals/SHA256SUMS` first) by copying each to a `.s6-old` name beside its place and renaming it over the file, never copying over a file a process runs.
5. Probe the plugin's launcher without `PRISM_MCP_SHARED` (step 4 above, without the variable); it must print `PASS`. Then restart any session that was started during the rollback.

### Phase 1: fan-out latency (a quiet hour; about 45 minutes)

Preconditions, checked and saved: no bridge run, build or other measurement on the host (lane rule: one measured process at a time); at least 20 minutes after Phase 0's probes, whose lazy daemons retire after the idle TTL; `python3 "$S/scripts/soak/soak.py" sample --count 1 --interval 0 --out "$E/fanout/host-before.jsonl"` shows no standalone index holder and no daemon of 100 MiB or more (an opted-in session's lazy daemon, a few MiB, does not count: review round 2, Opus S-D); both corpora at their commits and clean. Prime one cache per corpus (discarded runs):

```bash
C="$E/fanout/cache"
"$B" --repo ~/code/bench-repos/excalidraw --cache-dir "$C/excalidraw" --eager < /dev/null > "$E/fanout/prime-excalidraw.log" 2>&1; echo "rc=$?"
"$B" --repo ~/code/prism-c1-corpus --cache-dir "$C/slicing" --eager < /dev/null > "$E/fanout/prime-slicing.log" 2>&1; echo "rc=$?"
```

Start the passive footprint sampler before the fan-out runs (owner request 2026-10-16):

```bash
python3 "$S/scripts/soak/soak.py" sample --interval 60 --out "$E/fanout/footprints.jsonl" > "$E/fanout/footprints-sampler.log" 2>&1 &
echo $! > "$E/fanout/footprints-sampler.pid"
```

After all Phase 1 runs, stop that sampler and write the diagnostic trend report:

```bash
kill "$(cat "$E/fanout/footprints-sampler.pid")"
python3 "$S/scripts/soak/soak.py" report --phase fanout --samples "$E/fanout/footprints.jsonl" --out "$E/fanout/footprints.md" --json "$E/fanout/footprints.json"
```

`--phase fanout` gates no rules or use checks; its PASS is diagnostic and cannot satisfy the latency
gate. Read `no_footprint_growth` and every daemon's trend even when no baseline was reached. These
short runs may never establish one. Fan-out uses temporary runtime directories, so this host sampler
normally relies on the CPU rebuild detector rather than those daemons' private logs.

Every run below is alone, and keeps its default `--refresh-at 0.5` (a gating run) and `--pressure required` (the default on macOS). Define once per command:

```bash
run() { python3 "$S/scripts/soak/fanout.py" run --bin "$B" --corpus "$1" --cache-dir "$C/${1%%=*}" --mode "$2" --clients "$3" --out "$E/fanout/$4"; }
X=excalidraw="$HOME/code/bench-repos/excalidraw"
```

**Calibration (A/A; review round 1, Opus S1; review round 2, Opus W-A).** Six standalone runs of eight clients, groups a and b in ABBA order, judged in both directions:

```bash
run "$X" standalone 8 aa-a-1; run "$X" standalone 8 aa-b-1
run "$X" standalone 8 aa-b-2; run "$X" standalone 8 aa-a-2
run "$X" standalone 8 aa-a-3; run "$X" standalone 8 aa-b-3
python3 "$S/scripts/soak/fanout.py" compare --both-directions --baseline "$E/fanout/aa-a-1" --baseline "$E/fanout/aa-a-2" --baseline "$E/fanout/aa-a-3" --candidate "$E/fanout/aa-b-1" --candidate "$E/fanout/aa-b-2" --candidate "$E/fanout/aa-b-3" --out "$E/fanout/aa-compare.md" --json "$E/fanout/aa-compare.json"
```

If `aa-compare.md` does not say `**Verdict: PASS**` (in either direction the host's own spread between two identical arms exceeds the bound, or a run was inadmissible), the gate cannot be judged on this host as pinned: stop and take the A/A figures to the owner (owner question 7) before any gate run; never collect more runs toward it.

**The gate** (excalidraw), pairs in ABBA order so that drift in the host does not always fall on one arm:

```bash
run "$X" standalone 8 standalone-1; run "$X" shared 8 shared-1
run "$X" shared 8 shared-2;         run "$X" standalone 8 standalone-2
run "$X" standalone 8 standalone-3; run "$X" shared 8 shared-3
python3 "$S/scripts/soak/fanout.py" compare --baseline "$E/fanout/standalone-1" --baseline "$E/fanout/standalone-2" --baseline "$E/fanout/standalone-3" --candidate "$E/fanout/shared-1" --candidate "$E/fanout/shared-2" --candidate "$E/fanout/shared-3" --out "$E/fanout/excalidraw-compare.md" --json "$E/fanout/excalidraw-compare.json"
```

Information (slicing; owner question 9): `run slicing="$HOME/code/prism-c1-corpus" shared 8 slicing-shared-$rep` and `run slicing="$HOME/code/prism-c1-corpus" standalone 1 slicing-standalone1-$rep` for rep 1–3 in ABBA order, and a `compare … --allow-client-mismatch` written to `slicing-compare.md`; its verdict is reported, never gated.

Read and keep for the readout: each run's `summary.json` (p50/p95/p99/max, the refresh's time and outcome, `refresh_overlap`: the calls of other clients that overlapped the refresh and the longest of them, SI-43), each `run.json`'s `pressure` (every reading), `memory_after_workload.total_bytes` (eight standalone footprints against one daemon and eight shims) and `daemon_instances`, each shared run's `daemon-logs/`, and the compare files. **Pass:** the calibration passed and `excalidraw-compare.md` says `**Verdict: PASS**`.

### Phase 2: one bridge review round (fan-out under real load)

The round: the next real review round of any lane run through the bridge with a soaked **host-run** configuration, its agents' checkouts written down (each is a `--round-repo`) (agents in containers run in the OrbStack VM, outside the host's process table, so the sampler cannot see them), or else one `a2a-bridge run-workflow plan-review` (five nodes, codex and claude) on a checkout of `main` at or after the merge, `--session-cwd` that checkout. Preconditions: Phase 0's probes passed; the Claude Code sessions the owner will use were started after the profile export; `soak.py sample --count 1` lists the standalone index holders already running (they are listed in the report and not counted, D125).

```bash
[ -e "$E/client-logs/bridge.log" ] && mv "$E/client-logs/bridge.log" "$E/client-logs/bridge.log.attempt-$(date +%s)"   # each attempt starts with no bridge log (review round 2, Opus S-A)
nohup python3 "$S/scripts/soak/soak.py" sample --out "$E/round/samples.jsonl" --interval 5 >> "$E/round/sampler.log" 2>&1 &
echo $! > "$E/round/sampler.pid"; [ -s "$E/round/T0" ] || date +%s > "$E/round/T0"
```

(A new attempt after a stop condition uses a new `$E/round-N` directory, so that its `T0`, samples and report are its own.)

Run the round. Twenty minutes after its last agent ends (the idle TTL, 15 minutes, and a margin):

```bash
date +%s > "$E/round/T1"; kill "$(cat "$E/round/sampler.pid")"; pgrep -fl 'soak.py sample'   # prints nothing
python3 "$S/scripts/soak/soak.py" report --phase round --round-repo <each checkout of the round> --samples "$E/round/samples.jsonl" --since "$(cat "$E/round/T0")" --until "$(cat "$E/round/T1")" --claude-logs ~/Library/Caches/claude-cli-nodejs --client-log "$E/client-logs/bridge.log" --out "$E/round/report.md" --json "$E/round/report.json"
```

**Pass:** `report.md` says `**Verdict: PASS**`: every rule `PASS` (none `UNKNOWN`), and the use seen: at least two shims at once, a daemon reached in the logs and in `bridge.log` itself, an index built. A standalone server outside the round's checkouts (a launcher the soak does not edit, opened during the round) is listed and not counted (review round 2, Opus W-D). Reported with it: the most index holders at once against the number of checkouts the round used, the peak footprint of all prism-mcp processes, the peak swap and compressor, the standalone servers already running at the start, and whether the round's daemon had retired by `T1` (the last sample).

### Phase 3: one week of interactive use

```bash
nohup python3 "$S/scripts/soak/soak.py" sample --out "$E/week/samples.jsonl" --interval 60 --duration 604800 --window-file "$E/week/T0" >> "$E/week/sampler.log" 2>&1 &
echo $! > "$E/week/sampler.pid"
```

Check daily that the last line of `samples.jsonl` is recent (`tail -1 … | python3 -c 'import json,sys,time; print(time.time() - json.loads(sys.stdin.read())["t"])'`). A restart of the host or of the session kills the sampler; run the same command again: `--window-file` keeps the window's start (written once, the first time), the sampler runs to the end of that window rather than for another week, and `sampler.log` gets a `resumed` line with the time left (review round 2, sol 1, Opus W-E). A line cut short by the kill is skipped and counted by the report, and the report gives the largest gap between samples. Write in `$E/week/NOTES.md` anything that changes the deployed binaries. At the end:

```bash
kill "$(cat "$E/week/sampler.pid")" 2>/dev/null; date +%s > "$E/week/T1"
python3 "$S/scripts/soak/soak.py" report --phase week --samples "$E/week/samples.jsonl" --window-file "$E/week/T0" --until "$(cat "$E/week/T1")" --claude-logs ~/Library/Caches/claude-cli-nodejs --client-log "$E/client-logs/bridge.log" --out "$E/week/report.md" --json "$E/week/report.json"
```

(add a `--client-log` for each other wrapper log the week used). **Pass:** `**Verdict: PASS**`: `one_index_per_key`, `no_fail_closed`, `no_startup_error`, `no_restart_bound`, `no_abort` and `no_footprint_growth` `PASS`, and at least 5 usage days (days with at least 0.05 s of one daemon's CPU growth between observed consecutive endpoints at most 120 s apart, D128); read the largest gap and the count of intervals excluded from usage credit (> 120 s) with it (a sampler that was dead for days is visible there); `no_new_standalone_index` and `no_critical_pressure` are reported, not gated (other work on the host runs standalone servers and uses memory).

### Stop conditions (any phase)

- **An abort or a restart-bound refusal:** stop the phase; keep the crash report (`~/Library/Logs/DiagnosticReports/prism-mcp-*.ips`) and the daemon's log (`/tmp/prism-mcpd/<key16>.log`); escalate to the owner.
- **A fail-closed line or a startup error:** read it. A configuration cause (a runtime directory, a cache directory, a binary a configuration names) is fixed, probed (Phase 0 step 4), and the phase starts again from zero, disclosed; any other cause is escalated.
- **Memory pressure at its critical level** stops a fan-out run by itself (D126); during Phase 2 stop the bridge, keep the samples, escalate.
- **A gate shown unreachable by its own measurements** (the A/A calibration, or a latency ratio that cannot pass however quiet the host): escalate to the owner at once; never collect more runs toward it.
- **Two measured processes at once:** the overlapping run is inadmissible; re-run it alone.
- **An INADMISSIBLE report:** name what was missing (a source, a sample, the use) and run the phase again; never read it as a pass.

### After the soak

The readout `docs/analysis/2026-10-shared-daemon-soak-readout.md` (a docs pull request after the week): the three reports' verdicts and figures next to the incident's, the fan-out tables with the A/A calibration and the refresh stall, the deployment notes, what was not measured, and the owner's flip question. Evidence stays under `$E` with its `SHA256SUMS`, and is offloaded to `prism-evidence` when the lane closes. Remove `$S`'s `target/` (several GiB; disk hygiene) once nothing needs `$B`'s build again, and the worktree with it.

## Interfaces for the next slices

**B (the default flip; owner decision after the soak) consumes:** the soak's three reports. The flip itself: `--shared` becomes the default and `--standalone` opts out (spec §8 step 3); the launcher's `PRISM_MCP_SHARED` then means "unset = shared" or is retired, and `docs/MCP.md`, `.codex/config.toml` and the bridge's entries drop the flag. The plugin's version is bumped then (owner question 6), with a note that an updated plugin needs its binary built or copied again (the plugin cache of a new version has no `target/release/prism-mcp`).

**C2-M1 (and every candidate that ships through the §6.2 fan-out gate) consumes:** `fanout.py run --mode shared --clients 8` on the baseline build and on the candidate build, `compare --baseline … --candidate …` under the same pinned rule (its constants may be re-pinned by the candidate's plan), with SI-11 in mind.

**The bridge (outside this repository) can consume:** `prism-mcp daemon-shutdown --repo X --pid N --start-time T` from the shim's `shared daemon pid N (started T, …)` line as its reaper (S5b), if the soak shows that residency after a round matters (owner question 10).

## Self-review record

- **Spec coverage (§12 row S6, §8 steps 1–2).** "`.codex/config.toml`" → Task 4 (D115). "plugin launcher" → Task 1 (D110–D113, SI-36). "bridge entry, `PRISM_MCP_RUNTIME_DIR`" → `docs/MCP.md` and Phase 0 (SI-37). "docs" → Task 4 (D116). "soak instrumentation" → Tasks 2 and 3; §8 step 2's five measures → "The soak", one row each (SI-38–SI-41). §4.3 "`docs/MCP.md` says so" → the docs' runtime-directory section (SI-42). §6.2's fan-out gate → `fanout.py`, reusable by C2-M1 ("Interfaces for the next slices").
- **What S3a left to S6** (its "S6 (docs) owes"): the readiness vocabulary — `warming`, `build_failed`, the daemon's `indexing` with `retry_after_secs`, the `prism/index_state` values — in `docs/MCP.md` ("What changes for a client") and the skill's new bullet (the `S3a-2` review's note that neither documented `warming` or `build_failed`).
- **What S3b left:** what a client sees when a shared daemon panics (its session ends; the next start makes a new daemon) and the macOS crash report for each abort → "A crash ends every attached session".
- **What S4 left:** the daemon's command line and its exit statuses, what a client sees for each drain reason, the runtime directory the same for every launcher → "The daemon" paragraph and the runtime-directory section.
- **What S5a left, one by one:** `--shared` in `.codex/config.toml` (Task 4), the plugin launcher (Task 1), the bridge's `[[agents.mcp]]` with `PRISM_MCP_RUNTIME_DIR` (docs, Phase 0); the fail-closed line and what to do about each reason (the docs' table, each reason in the code's words); `--idle-ttl`/`--max-clients` configure only a daemon the shim starts, silently ignored otherwise (D85; the flags table); the restart bound and its minute, and the wall clock (D77, owner decision 12, option A; "A crash ends …"); a rebuilt binary's old shims cannot start a daemon (SI-27; the table's spawn row); the `sudo -E` root-owned directory (the lane README's S5a note; the runtime-directory traps). The soak reads, as S5a specified: fail-closed lines starting `prism-mcp: --shared failed closed` (D87), daemon starts from `prism-mcp: shared daemon pid … (started …` (D88), `RecentBuildCrash` (D77), `pgrep`-style counts (here the sampler's process table) and the 10-03 probes (SI-38), and the fan-out with eight shims and one concurrent refresh (S3b's SI-11 reported, SI-43).
- **What S5b left, one by one:** the bridge's reaper interface (`daemon-shutdown --pid --start-time` from the shim's line) is documented, and building the reaper is owner question 10; the soak's resident count through `daemon-status` (`sample --status-bin`); pre-S5b daemons stopped with `kill -TERM` and their idle restart at each probe (SI-35) in the docs; the two commands, their exit statuses, `--runtime-dir` as the base, other builds listed and stopped with `kill -TERM`, exit 1's meanings, "never disconnects a client and keeps admitting until it drains" (owner decision 15), "a probe does not keep a daemon of this build up" — all in "Operating daemons". The sampler connects to no daemon by default (D121), so it cannot keep a pre-S5b daemon alive.
- **The lane README's notes.** "S6: the launcher … passes extra arguments through, so `--shared` is a config change, not a script change": true for every launcher but the plugin's, whose arguments are not user configuration (SI-36). S2a "the docs should say" (the root-owned directory): done. S2a `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`: the daemon paragraph says the shim does not pass it (S5a's D84). Owner decision 14 (keys left by rebuilds, counted, nothing removed): the "Files" paragraph. The HANDOFF's open items not S6's: EMFILE back-off (item 15), the call-edge sidecar's temp files (item 11), SI-10; SI-11 is reported by Phase 1 (the refresh's overlap), not measured against a bound (SI-43; the earlier "measured by Phase 1" withdrawn after review round 1, Opus W1).
- **The lane's review lessons.** (1) Every stated property has a test and a mutant, or is listed as not held: "Claims". (2) What each mechanism does not cover: its section. (3) Interleavings are played, not hoped for: a client that dies before the barrier (the barrier is aborted), a client that dies after it, warming answers before the barrier, a critical pressure reading in the middle of a run. (4) No test asserts an upper bound on a duration; the one subprocess bound is a hang limit. (5) No test writes a file it then executes. (6) Measurement first: the claim that Claude Code passes its environment and that its logs keep the server's standard error was measured on this host before the design relied on it (SI-42, D120); the claim about Codex's environment is marked inherited.
- **Proportionality.** Every piece serves the soak, which is the gate for the default flip, which is the lane's goal: the launcher (an opt-in for the clients that hold most sessions on this host), the docs (users of the opt-in, and the earlier slices' debts), the sampler (the resident count and footprint the incident was measured by), the fan-out driver (the latency half of the goal, and C2-M1's gate). Not built: a product-side event log (D120), a pruning flag (owner decision 14), the bridge's reaper (owner question 10), a CI job for the Python tests (D117).
- **Review round 1's lesson, applied.** Every instrument answers three ways, not two: what it saw breach, what it saw clean, and what it could not see. A rule built on absence (no fail-closed line, no crash report, no second daemon) needs proof the absence was observed; the report now says UNKNOWN or INADMISSIBLE where it was not.

## Validation record

Built on 2026-10-16 on macOS (arm64), in `~/code/prism-s6-validate`, from `main` `61c2a9e5`, merged up to `main` `4a9e6476` for review round 1 (lane docs only in between). Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file under the session's scratch directory, read from the file. Nothing was pushed. Local branches:

- `scratch/s6-validate`, where the code was first written: `5838de63` (the launcher and its tests), `8b3dc0fb` (the soak tools), `f81cf81a` (docs and configuration), `c9be7b01` (two fan-out tests the mutant table asked for), `f97a04ed` (the live test without `pgrep`; daemons per repository), `9e82b670` (days with a shim; the round-0 head), `d251849e` (merge of `main` `4a9e6476`), then review round 1: `0bd92f49` (its tests, red), `67d2d407` (the fold), `f2239814` (`CLAUDE.md`), `f76f8e33` (four tests for mutants that first survived; the barrier bounded), then review round 2: `7faddf0c` (its tests, red), `22a43371` (the fold), `2e948c5c` (`CLAUDE.md`), `cbc32ee3` (a redundant check removed; the head). The full suite ran at `f76f8e33` (V4; nothing cargo builds changed after it); the full mutant set at `2e948c5c` and U9, U11, R3 again at the head, each alone, the file restored after each.
- `scratch/s6-tasks`, the same source cut into this plan's tasks, one commit per task on `4a9e6476`, every file taken from `cbc32ee3`: `111993b5` (Task 1), `ac6ddc7a` (2, the S6-1 head), `fc612b47` (3), `678fd702` (4, the S6-2 head). `cargo fmt --check` passed at every commit; its tree is `4b940d6e`, the head's. Earlier cuts are kept as `scratch/s6-tasks-r0`, `scratch/s6-tasks-r1base` (round 0) and `scratch/s6-tasks-r1` (round 1), with their replays `scratch/s6-replay-r0`, `scratch/s6-replay-r1base` and `scratch/s6-replay-r1`.
- `scratch/s6-replay`: this plan's directives replayed by a script onto `4a9e6476`, every anchor matching exactly once (25 directives, three of them "Make … executable."): `0daffe4f`, tree `4b940d6e`, the tree of `678fd702` and of `cbc32ee3`. The red and green steps of Tasks 1–3 were run on partial replays in the validate worktree (detached at `4a9e6476`, the plan replayed up to the task's Step 2, then up to its Step 4; V2, V3).

The tools (the generator of the directives from the per-task commits, the replay, the cut, the mutant runner, the assembly of this file) are S5b's, with three changes: a created file whose mode is `100755` gets a "Make `path` executable." directive, which the replay applies with `chmod 755` (the tree comparison includes modes); Python and bash fences; and the test files of this slice (`tests/…`, `scripts/soak/test_*.py`, `fake_prism_mcp.py`) are told from production files by name.

**Measured on this host before the design relied on it.** (1) Claude Code gives a plugin's MCP server its whole environment: `ps eww` of a running plugin `prism-mcp` (Claude Code session `fd583811`) lists the user's shell variables, `TMPDIR`, `GOROOT`, API keys among them (names only were read). (2) Claude Code keeps an MCP server's standard error: `~/Library/Caches/claude-cli-nodejs/-Users-wesleyjinks-code-slicing/mcp-logs-plugin-prism-prism/*.jsonl` holds entries `{"error":"Server stderr: nav cache: CPG cache miss …","timestamp":…}`. (3) A lazy plugin server that has built nothing has a `phys_footprint` of 1.8 MiB (`footprint -p 8068`). (4) macOS names a crash report after the process: the lane's aborted test binaries left `prism-49efe5ab01e24782-2026-10-14-*.ips`. (5) The image `a2a-toolchain:latest` has Python 3.12, git and bash 5.2, and no `ps`, `pgrep` or `footprint`. (6) Both pinned corpora are at their commits and clean.

**Measured in review round 2.** An idle daemon's CPU: a `prism-mcp --shared` shim of the debug build on a three-file repository, one `nav_callers` call, then 120 s idle with the shim attached: the daemon's `ps -o time=` read `0:00.17` before and after (D128's threshold, 0.5 s a day, rests on it; the daemon was shut down with `daemon-shutdown` and its runtime directory removed). **Superseded** in PR #424's review round 1 (Opus W1): at nanosecond precision the idle daemon spends about 0.86 s a day, which `ps`'s 10 ms resolution cannot show in 120 s; the owner then changed D128 to a per-interval rule (2026-10-16).

**Not measured.** Whether a host-run Codex passes `TMPDIR` to its MCP servers (SI-42; a look at the Codex binary's strings was refused by the session's permission policy, and a Codex session was not started for it); the soak.

**Corrections made by the validation.**

| # | Where | What was first written | What was wrong | What it says now |
|---|---|---|---|---|
| 1 | Task 3, `fanout.py` | the clients' standard output pipes left open after they exit | `ResourceWarning: unclosed file` in every run of the tests | closed after each client's exit |
| 2 | Task 3, `test_fanout.py` | no test of a client that dies before the barrier, nor of the cleared environment | mutants F10 (no `barrier.abort()`: the main thread waits for ever) and F12 (the shell's output settings reach the clients) had no test to fail | `test_clients_that_die_while_warming_up_end_the_run` (in a subprocess with a 60 s bound), `test_clients_start_from_the_default_output_settings` (the fake reports what it was given) |
| 3 | Task 3, `test_fanout_live.py` | `pgrep -f 'prism-mcp daemon …'` to show no daemon was left | the Linux image has no `pgrep`: the test would have errored there | the shared run's own daemon pid is checked with `fanout.alive` |
| 4 | Task 2, `soak.py` | `time.mktime(…) - time.timezone` for a UTC time stamp | wrong by an hour under daylight saving time | `calendar.timegm`; `test_iso_times_are_utc`; mutant S7 |
| 5 | Task 2, `soak.py` | no count of daemons per repository, no count of days with a shim | owner question 3 needs to see two daemons of one checkout (other cache directories), and SI-41's floor needs the days | `max_daemons_per_repo`, `repos_with_several_daemons`, `days_with_shims`, with tests and mutants S12, S13 |
| 6 | Task 4, `.codex/config.toml` | — | Python 3.9 has no `tomllib`: the file could not be parsed with the host's `python3` | parsed with Python 3.14's `tomllib` (uv's), as was the docs' bridge snippet |
| 7 | Task 3, `test_fanout.py` | two warming answers before the first ready one | with review round 1's extra warm-up call of the second tool, a warm-up that did not retry no longer reached a timed call: mutant F9 survived | three warming answers |
| 8 | Task 3, `test_fanout_gate.py` | the overlap test without a call of client 0 inside the refresh's window | counting client 0's own calls (mutant F15) changed nothing it asserted | a client-0 call inside the window |
| 9 | Task 3, `test_fanout_gate.py` | the critical-pressure test asserted the problem and the reading | a watch that recorded the reading and never stopped the clients (mutant F16) passed | asserts the clients were stopped and the calls cut short |
| 10 | Task 2, `test_soak_coverage.py` | the round's phase test without a gated UNKNOWN under seen use | an `overall` that ignored UNKNOWN (mutant U11) passed | a failed `ps` in a used round must be INADMISSIBLE |
| 12 | Task 2, `soak_judge.py` | the round's use checks included "a daemon reached in any log" beside "in every client log" | the full mutant run left U10 alive: the first is implied by the second (an equivalent mutant) | the redundant check removed |
| 11 | Task 3, `fanout.py` | the main thread's barrier waits without a bound | under mutant F10 a client killed during warm-up (the pressure test) held the whole suite past its 900 s limit | the barrier waits at most until the run's own deadline |

## Review round 1

PR #422 at `e27aee5e` (plan `1436eb96`, code `c6aeb376`). **gpt-6.1-sol: FIX** (four WRONG/MATERIAL; three SMELL/IMMATERIAL). **Opus 5.5: FIX** (seven WRONG, six MATERIAL, one IMMATERIAL; eight SMELLs). Both: the launcher and the docs are correct and match the code; the plan's code is the replay byte for byte; the defects are in the soak's specification and its instruments. The controller's ruling: fold all on the existing plan, test first; keep the owner questions open with the agreed corrections to questions 4, 5, 7 and 8. The owner has not yet answered them.

Folded on `scratch/s6-validate` (merged up to `main` `4a9e6476` first, lane docs only): `0bd92f49` (every new test, committed red: `Ran 59 tests`, `FAILED (failures=4, errors=35)`), `67d2d407` (the fold: `OK`), `f2239814` (`CLAUDE.md`), `f76f8e33` (four tests for mutants that first survived; the run's barrier bounded by its timeout). Re-cut, regenerated and replayed (Validation record).

| Finding | Ruling | What changed | Test, RED before the change → GREEN after |
|---|---|---|---|
| sol 1 / Opus W4, WRONG/MATERIAL: rollback restores a binary that rejects the `--shared` Codex's configuration still passes | folded | Rollback order: configurations first (bridge and Codex from snapshots, `--shared` removed), daemons stopped with the kept S6 binary, binaries restored, every launcher probed without `--shared`; `soak.py probe` (D127) | `test_a_binary_that_rejects_shared_fails_and_says_why`: RED (no `probe` command: `SystemExit: 2`) → GREEN; the check an executable that rejects `--shared` must fail. Mutant U17 |
| Opus W5, WRONG/MATERIAL (low): question 4's order is circular | folded | The soak binary is built from the PR head (its `src/` is `main`'s); merge after Phase 0's probes (owner question 4, revised) | — (a procedure) |
| sol 2, WRONG/MATERIAL: unavailable observations become PASS | folded | Rules PASS/FAIL/UNKNOWN, phases PASS/FAIL/INADMISSIBLE (D124); `ps_ok`, `footprint_status` (`gone` vs `failed`), missing pressure, `logs_readable`, crash directory readability | `test_soak_coverage.py` `Unobserved` (seven tests): RED (`judge() takes 3 positional arguments`, no `PS_COMMAND`, no `FOOTPRINT_COMMAND`, no `logs_readable`) → GREEN. Mutants U1–U8 |
| Opus W2, WRONG/MATERIAL: the bridge round can pass seeing nothing | folded | `--phase round` needs ≥ 2 shims at once, a reached daemon, a built index (D125); host-run configuration required; containers named as invisible | `test_a_round_needs_shims_a_reached_daemon_and_a_built_index`, `test_a_round_report_without_its_log_sources_is_inadmissible`: RED (no `usage`; `--phase` unknown) → GREEN. Mutants U9–U11 |
| Opus W3, WRONG/MATERIAL: a standalone server that predates the round fails it | folded | Only standalone holders first seen after the first sample count; the others are listed (D125) | `test_standalone_holders_already_running_at_the_start_do_not_fail_a_phase`: RED (`KeyError: 'preexisting_standalone_holders'`) → GREEN. Mutant U12 |
| sol 3, WRONG/MATERIAL: the latency gate accepts a missing or failed refresh | folded | A gating run needs exactly one refresh answered `ok`; `--no-refresh` is diagnostic and never compared (D119, D123) | `test_a_gating_run_needs_exactly_one_successful_refresh`: RED (`True is not false`: a run without its refresh was admissible) → GREEN; `test_a_refresh_answered_with_a_tool_error_makes_the_run_inadmissible`, `test_a_run_without_a_refresh_is_diagnostic_and_never_compared`. Mutants F13, F14 |
| Opus W1, WRONG/MATERIAL: the rule cannot see SI-11 | folded (reported, not gated) | `refresh_overlap` per run and in `compare`; SI-43 withdraws "measured by Phase 1"; gating it is an option in owner question 7 | `test_the_calls_that_overlap_the_refresh_are_counted_and_their_longest_reported`: RED (no `refresh_overlap`) → GREEN. Mutant F15 |
| sol 4, WRONG/MATERIAL: Phase 1's pressure stop has no observer | folded | `PressureWatch` in `fanout.py`: a reading at the start, every second, at the end; a critical one stops the clients and invalidates the run; an unreadable one invalidates a run that requires readings (D126) | `test_a_critical_pressure_reading_stops_and_invalidates_the_run`, `test_an_unavailable_pressure_reading_invalidates_a_run_that_requires_it`: RED (no `read_pressure` to inject) → GREEN. Mutants F16, F17 |
| Opus W6, WRONG/MATERIAL: Phase 0 not idempotent | folded | Originals kept once in `$E/deploy/originals` with sha256s; installs copy and rename; rollback restores from there | — (a procedure) |
| Opus W7, WRONG/MATERIAL: Phase 2 loses its sampler and `T0` between commands | folded | pid and time files, `kill "$(cat …)"`, a `pgrep` check; every block restates its variables | — (a procedure) |
| Opus W8, WRONG/IMMATERIAL: `compare` raises on a run without calls | folded | Medians only over percentiles that exist; otherwise FAIL with the reason | `test_a_run_with_no_calls_fails_the_comparison_instead_of_crashing_it`: RED (`TypeError: unsupported operand type(s) for /: 'NoneType' and 'float'`, the reviewer's reproduction) → GREEN. Mutant F18 |
| Opus S1, SMELL/MATERIAL: no calibration, order biased | folded | A/A calibration before the gate (escalate if it fails); ABBA order | — (a procedure) |
| Opus S2, SMELL/MATERIAL: the binary not pinned; a clap rejection invisible | folded | The bridge names `$E/bin/prism-mcp`; `no_startup_error` rule; `probe` | `test_a_startup_error_and_a_restart_bound_refusal_fail_their_rules`: RED → GREEN. Mutants U13, U14 |
| Opus S3, SMELL/MATERIAL: an idle session counts as a usage day | folded | A usage day needs a built index or two shims of one configuration (D128) | `test_a_usage_day_needs_a_built_index_or_two_shims_of_one_configuration`: RED (`KeyError: 'usage_days'`) → GREEN. Mutant S13 (rewritten) |
| Opus S4, SMELL: D121 contradicted by `--status-bin` in the runbook | folded | The runbook no longer passes `--status-bin` (D121 restated) | — |
| Opus S5, SMELL: a binary path with a space; a truncated sample line | folded | `split_command`; `load_samples` skips and counts | `test_a_binary_path_with_a_space_is_still_prism_mcp` (RED `TypeError: 'NoneType' object is not subscriptable`), `test_a_truncated_last_line_is_skipped_and_counted` (RED `JSONDecodeError`) → GREEN. Mutants U15, U16. Not folded: excluding test daemons under temporary directories, and a per-reason histogram of fail-closed lines (the stop conditions read reasons by hand) |
| Opus S6, SMELL: fan-out edges | folded | `stop_daemon` catches its timeout; daemon logs kept; client counts, refresh points and a dirty corpus refused by `compare` (`--allow-client-mismatch` for information); every tool warmed | `test_a_shared_run_keeps_its_daemon_logs`, `test_runs_of_other_client_counts_or_refresh_points_are_not_compared` (RED `'clients' not found in ''`), `test_every_tool_of_the_workload_is_warmed_before_the_barrier` → GREEN. Mutants F19–F22 |
| sol S7 / Opus S7, SMELL: the time-zone test depends on the host's zone and misses the July bug | folded | Subprocesses under `UTC0` and `MST7MDT,M3.2.0,M11.1.0`, January and July | `test_iso_times_are_utc_in_any_local_zone_winter_and_summer`: a pin (the code was already right); kills S7 and S7b (the earlier draft's `mktime - timezone`) in every zone |
| sol S7 / Opus S7, SMELL: Python tests not in CI | folded | One step of CI's build-checks job (D117) | — |
| sol S7, SMELL: precedence of an explicit flag over a bad value | folded | Documented (D111, the launcher's comment, `docs/MCP.md`); every combination tested | `the_variable_and_an_explicit_argument_in_every_combination`: a pin; kills L6 |
| Opus S8, SMELL: docs examples for one user; 829 vs 833; no cleanup of `$S` | folded | A note beside the examples; the counts restated (now two PRs, D114); cleanup after the soak | — |

The production growth from the fold (833 → 1,157 lines) passed the lane's threshold, so the slice now ships as two pull requests (D114). Not folded, recorded: test daemons under temporary directories in `max_daemons_per_repo` (a millisecond overlap in a test is improbable in a 5 s sample; it would show as a FAIL the stop conditions read); `footprint`'s effect on a large process (not measured).

## Review round 2 (the cap; folded after it, disclosed)

PR #422 at `b06c12aa` (code `84c42714`). **gpt-6.1-sol: FIX** (one WRONG/MATERIAL; every round-1 finding of its own closed). **Opus 5.5: FIX** (five WRONG, four MATERIAL and one MATERIAL-low; six SMELLs), converging: three are incomplete round-1 folds (S3, W3, W4) and two came with the fold (the calibration, the week's restart). sol's finding is Opus W-E. The controller's ruling: the loop converges (eight WRONG, then five, each bounded, none of a kind folded twice), so the fold goes ahead **after the two-round cap, without a third round, disclosed** here and in the pull request bodies; any fix that showed another instance of the same kind of gap was to stop the fold and go to the owner. None did. The owner questions stay open; 4, 5, 7 and 8 carry the corrections.

Folded on `scratch/s6-validate`: `7faddf0c` (every new test, committed red: `Ran 66 tests`, `FAILED (failures=3, errors=7)`), `22a43371` (the fold), `2e948c5c` (`CLAUDE.md`), `cbc32ee3` (the round's "a daemon reached in any log" check removed: the full mutant run showed its mutant U10 survive, because "a reached daemon in every client log" implies it; an equivalent mutant, so the redundant check went rather than a test). Re-cut, regenerated and replayed (Validation record).

| Finding | Ruling | What changed | Test, RED before the change → GREEN after |
|---|---|---|---|
| sol 1 = Opus W-E, WRONG/MATERIAL: a resumed week rewrote `T0` and dropped a day-2 failure and earlier usage days | folded | `sample --window-file T0` writes the start once and runs to the end of that window on a resume, logging a `resumed` line; `report --window-file`; the runbook appends to `sampler.log`; Phase 2's `T0` guarded the same way (D129) | `test_a_resumed_sampler_keeps_the_window_it_started`, `test_a_day_two_failure_and_earlier_usage_days_survive_a_resume` (seven days, a day-2 fail-closed line, a resume on day 4: `no_fail_closed` FAIL and seven usage days): RED (no `open_window`) → GREEN. Mutant W1 |
| Opus W-A, WRONG/MATERIAL: the A/A calibration passed a 30 % spread one way | folded | `fanout.calibrate` and `compare --both-directions`: both directions must pass; calibration runs in ABBA order | `test_a_calibration_passes_only_when_both_directions_pass` (100/200 ms against 70/140 ms: forward PASS, backward FAIL, calibration FAIL both ways): RED (no `calibrate`) → GREEN. Mutant A1 |
| Opus W-B, WRONG/MATERIAL (round-1 S3 incomplete): a usage day counted residency | folded | `ps`'s `time` read (`cpu_secs`); a usage day needs 0.5 s of daemon CPU that day (D128). Measured: an idle daemon with a client attached spent `0:00.00` of CPU in 120 s | `test_a_usage_day_is_a_day_a_daemon_worked_not_one_it_was_resident` (one call on day 1, a window open all week: one day): RED (`['1970-01-01', … seven days]` against one) → GREEN. Mutants C1–C4 |
| Opus W-C, WRONG/MATERIAL (round-1 W4 incomplete): rollback restored a pre-S5a build where `main`'s `.codex` passes `--shared` | folded (the first alternative) | Rollback leaves `~/code/slicing/target/release/prism-mcp` (and `.codex`) as they are and restores only the plugin's binary and launcher: the S6 build serves standalone clients exactly as before, and keeping it keeps every checkout's Codex working; a revert pull request would reach checkouts only as they pull (owner question 5) | — (a procedure) |
| Opus W-D, WRONG/MATERIAL (round-1 W3 incomplete): an unedited launcher opened during the round failed it | folded (the first alternative) | `report --phase round` requires `--round-repo` (each of the round's checkouts) and gates `no_new_standalone_index` only on those (paths resolved); the others are listed (D125) | `test_only_standalone_servers_of_the_round_s_checkouts_are_gated` (a symlinked alias of a checkout counts), `test_a_round_report_without_its_checkouts_is_refused`: RED (`judge()` takes no `round_repos`; the report ran without one) → GREEN. Mutants R1, R2, R4 |
| Opus S-A, SMELL/MATERIAL (low): a round could pass on interactive sessions alone | folded (both) | Every `--client-log` (the bridge's) must hold a reached daemon in the window; the runbook moves `bridge.log` aside before each attempt | `test_a_round_needs_a_reached_daemon_in_every_client_log`: RED → GREEN. Mutant R3 |
| Opus S-B, SMELL/MATERIAL (low): rollback could not finish with sessions attached; a plain copy over a running executable | folded | Rollback ends or restarts opted-in sessions, uses `kill -TERM` for a daemon that still has clients, restores by copy and rename | — (a procedure) |
| Opus S-C, SMELL/MATERIAL (low): a second Phase 0 rewrote `$B` in place | folded | `$B` installed by copy and rename under a per-hash name (`prism-mcp-<sha12>`, `$B` a link to the first); `SHA256SUMS` appended | — (a procedure) |
| Opus S-D, SMELL: Phase 1's precondition counted lazy daemons | folded | 20 minutes after the probes; no standalone holder and no daemon of 100 MiB or more | — |
| Opus S-E, SMELL: CI step in S6-2; "this pull request"; a non-empty `SRC-DIFF` | folded | The CI step moves to S6-1 (Task 2); Phase 0 builds S6-2's head and merges S6-2; merge `main` into S6-2's branch when `SRC-DIFF` is not empty | — |
| Opus S-F, SMELL: the week's coverage measure | folded (in part) | The report gives the time covered and the largest gap between samples (D130). Not folded: tolerating isolated `ps` failures (one makes the process rules UNKNOWN; the owner judges such a week from the report) | `test_the_largest_gap_between_samples_and_the_time_covered_are_reported`: RED (`KeyError`) → GREEN. Mutant G1 |

## PR #424 review round 1 (S6-1's code; folded 2026-10-16)

gpt-6.1-sol FIX (two WRONG/MATERIAL), Opus 5.5 FIX (one WRONG/MATERIAL, one WRONG/IMMATERIAL, five SMELL/IMMATERIAL), round 1 of 2. Folded on `feat/s6-1-launcher-soak`, each behaviour change test first (its test committed red, then the fix). **Task 2's code blocks above predate this fold**: the code on `main` after PR #424 is authoritative, and S6-2 builds on it rather than replaying Task 2's blocks. S6-2's soak test count rises by the 18 tests of `scripts/soak/test_soak_pr424.py` (Task 4 Step 3's and Step 4's `Ran 66 tests` become `Ran 84 tests`). The CI step's comment no longer mentions the fan-out test or `scripts/memreport/` (both false at S6-1's head); S6-2 may restore them when they become true.

| Finding | Disposition | Change | Test (RED → GREEN), mutants |
|---|---|---|---|
| Opus W1, WRONG/MATERIAL: an idle resident daemon with a session attached spends about 0.86 s a day (its 5 s identity tick, measured at ns precision), so D128's 0.5 s daily sum counted every resident day | escalated; **owner decision 2026-10-16**: a usage day needs some single sample interval with ≥ 0.05 s of one daemon's CPU growth | `usage_days` per pid per interval of consecutive samples (growth rounded to µs: `ps` hundredths); a pid missing from either end contributes nothing; D128, owner question 8 and the round-2 measurement note revised | `test_a_resident_idle_daemon_shows_no_usage_day_in_a_week` (7 days at +0.0006 s a minute: was 7 days), `test_one_interval_of_work_counts_its_day`, `test_exactly_the_threshold_counts_and_just_below_does_not` (0:00.25 → 0:00.30 counts; 0.049 does not), `test_a_pid_missing_from_a_sample_contributes_nothing_for_that_interval`, `test_the_report_states_the_interval_rule`. Mutants N1 (the daily 0.5 s sum restored), N1b (a daily 0.05 s sum), N2 (no rounding), N3 (`>`), N4a/N4b (a pid across a gap) |
| sol 1, WRONG/MATERIAL: `--repo=/r` not parsed and `'/a dir'` cut to `/a`; the round's rule PASSed with a new in-round standalone holder | folded (with Opus S3) | Options read as `--name value` or `--name=value` up to the next ` --`; a relative `--repo` joined to the process's working directory (`/proc`, `lsof`), kept as written in `repo_arg`; an unresolved one makes `no_new_standalone_index` UNKNOWN in a round | `test_the_repository_is_read_in_every_form_and_kept_whole`, and parser-to-report: `test_a_new_standalone_holder_named_with_equals_fails_the_round`, `..._in_a_path_with_a_space_fails_the_round`, `test_a_relative_repository_resolves_against_the_process_s_working_directory`, `test_an_unresolved_repository_makes_the_round_rule_unknown` (all four PASS with the old parser), `test_the_working_directory_of_a_live_process_is_read`. Mutants N5–N8, N17 |
| sol 2, WRONG/MATERIAL: `prism-mcp-<sha12>` (Phase 0's install name) not recognized: healthy runs INADMISSIBLE, two same-key daemons under the two names PASS | folded | A binary named exactly `prism-mcp` or `prism-mcp-<12 lowercase hex>` (also in a path with a space); `prism-mcp-logged.sh`, the launcher and test binaries still excluded; processes counted by pid | `test_prism_mcp_and_prism_mcp_dash_twelve_hex_are_recognized_and_nothing_else`, `test_a_healthy_round_of_hash_named_processes_passes` (was INADMISSIBLE), `test_two_daemons_of_one_key_under_the_two_names_fail_one_index_per_key` (was PASS). Mutants N9, N9b, N10 |
| Opus W2, WRONG/IMMATERIAL (before Phase 0): the wrapper's logger died on a failed append and took stderr with it | folded | Each line forwarded first, then appended in a group with errors to `/dev/null`, `|| true` on both | `test_an_unwritable_log_passes_every_line_to_the_client_and_keeps_the_server_alive` (was exit 120, no stdout). Mutants N11, N12 |
| Opus S1, SMELL: `PRISM_MCP_SHARED=1` with `--no-cache`/`--eager`/`--owner-*` is a clap conflict | documented | The launcher's comment; Task 4's `docs/MCP.md` text ("Opting in") for S6-2 | — |
| Opus S2, SMELL: any unmeasured standalone server made the round's rule UNKNOWN | folded | Only a new one in the round's checkouts (or of unresolved repository) does; the report lists them | `test_only_a_new_unmeasured_standalone_server_in_the_round_is_unknown`. Mutants N13, N14 (U2 re-expressed) |
| Opus S3, SMELL: a relative `--repo` resolved against the report's directory | folded with sol 1 | — | — |
| Opus S4, SMELL: the CI comment described S6-2 | folded | Comment trimmed | — |
| Opus S5, SMELL: `probe` without a command; a crash report vanishing between list and stat; D121's `ps` text | folded | `probe` prints its usage and exits 2; such a report is skipped; D121 quotes `time=` | `test_probe_without_a_command_prints_its_usage_and_exits_2`, `test_a_crash_report_that_vanished_after_the_listing_is_skipped`. Mutants N15, N16 |

Mutants at the fold's head, each alone, file restored after each: the plan's 45 S6-1 mutants (L1–L6, S1–S12, S7b, U1–U9, U11–U17, C1–C4, W1, R1–R4, G1; C1, C2, C4 and U2 re-expressed against the new functions) and 20 new ones (N1, N1b, N2, N3, N4a, N4b, N5–N8, N9, N9b, N10–N17): 65 of 65 killed, each failing set containing the named tests.

Limits left by the round-1 fold, disclosed: a value containing ` --` is cut there; a new daemon reusing an earlier pid can be missed when its CPU counter resets (conservative). The former long-gap usage-credit limit is closed by round 2's owner-decided 120 s cap below; first-sighting lifetime CPU credit is also removed.

## PR #424 review round 2 (S6-1; folded after the cap, disclosed)

gpt-6.1-sol FIX and gpt-6-astra FIX, the same **WRONG/MATERIAL** finding: an unseen pid's inferred zero CPU baseline could produce five usage days and week PASS without a qualifying measured interval. The owner authorized this bounded fold on the existing artifact **after the two-round review cap, without a third round**, test first. Both owner decisions of 2026-10-16 now apply: at least 0.05 s growth with CPU readings at both consecutive endpoints, and an interval cap of 120 s (twice the default 60 s cadence). Longer intervals remain visible and are counted in JSON (`usage_intervals_excluded`) and Markdown. D128, question 8, the docstrings and usage-rule fragments above carry the final rule; other Task 2 code blocks remain historical as noted in round 1.

`test_soak_pr424_round2.py` adds seven tests, including uninterrupted 60 s sampling over seven days: on five days a fresh daemon appears at 0.06 s, adds 0 or 0.01 s, then disappears; zero usage days and INADMISSIBLE are required. Its control observes 0.00 → 0.06 and still passes with five usage days. A 121 s interval with 0.6 s growth earns nothing and is reported; exactly 120 s with 0.06 s counts. Tests also cover missing CPU readings, empty/single-sample input, counting long intervals without daemons, and retaining the next interval's baseline. Contrary first-sighting assertions are corrected; two older fixtures' 600 s work intervals become 60 s to retain their original purpose under the cap. S6-2 gains these seven tests (its prospective 84 becomes 91).

RED on unchanged production at `8cc607ff`: seven new tests, failures=3 and errors=4 (subtests counted individually): the fresh-daemon week falsely credited five days, the 121 s interval earned credit, and the exclusion count was absent; the observed-endpoint week control passed. Full discovery: 63 tests, failures=7 and errors=4, including two existing wrapper tests denied `/dev/fd/62` by the sandbox before child execution. These denials are inadmissible as wrapper-behavior evidence and predate production edits. The final runnable gates and mutants are recorded in `HANDOFF-S6-1-r2.md`; full host/CI wrapper validation is left to the controller. No Cargo or live soak was run for this Python/docs fold.

GREEN: both discovery patterns (`test_*.py` and `test_soak*.py`), using the evidence runner to skip only those two exact wrapper tests, discovered 63 tests: 61 passed, 2 skipped. The three requested mutants, applied individually in temporary source copies, were all killed by the named new tests: first-sighting credit restored (the week regression, both growth subtests), 120 s cap removed (121 s regression), and eligibility `> 120` changed to `>= 120` (120 s boundary control). Each mutant run selected seven tests and had assertion failures with no test errors; the working-tree judge stayed unchanged. Evidence and reproduction commands: `/private/tmp/s6-1-r2-fold-8cc607ff/`, with the final file snapshot and this fold's handoff. The earlier round-1 mutant totals above are inherited records, not a rerun here.

## S6-2 owner fold — footprint growth and bridge fan-out (2026-10-16)

Owner request implemented against `feat/s6-2-fanout-docs` at `d52e5717`, without Rust changes.
`soak_growth.py` owns D131's trends and passive rebuild-log observations; the report adds the rule,
gates it only in Phase 3, and accepts diagnostic `--phase fanout`. `docs/MCP.md` documents the three
cross-worktree fan-out options and their tradeoffs; lane `HANDOFF.md` §4 row 22 tracks the follow-up.

Watched RED: initial 19 tests on unchanged production, failures=1/errors=31; the final 23 tests
against HEAD's unchanged production in an isolated copy give failures=1/errors=36 (subtests included).
GREEN: all 23 new tests pass; full available soak discovery finds 114, with 111 passing and 3 skipped
(two wrapper tests denied `/dev/fd/62` in the same-environment pre-change control; the existing live
fan-out test needs a host binary and daemon/socket facilities). The requested raw gate was attempted;
the wrapper denials are not behavioral evidence. Broader `cargo test --features mcp --offline` stops
in library tests: 2,320 passed, 88 failed (86 explicit permission denials, two daemon-listen timeouts);
integration/doc tests were not reached by that stopped run. The hook follow-up ran all 25 non-MCP
integration targets separately: 3,812 passed, 0 failed, 1 ignored; binary tests 6/6, doc tests 2/2.
Aggregate unique Rust totals: 6,140 passed, 88 failed, 1 ignored. Root `VERIFICATION.md` records exact
commands, the behavior/edge coverage map and environment-limited exclusions. Full host checks remain
with the controller.

New mutants FG1–FG8: absolute threshold, relative threshold, strict growth boundary, baseline reset,
quiet-only gating, inclusive one-hour boundary, log reset and warm-up duration. All eight killed in
isolated copies by named assertion failures, no test errors. Reproduce with
`python3 scripts/soak/mutate_footprint_growth.py --out /private/tmp/s6-2-footprint-mutants`.
Evidence, exact exclusions and controller custody: `HANDOFF-S6-2-footprint.md` and
`/private/tmp/s6-2-footprint-evidence/`. No commit attempted; controller commits the fold.
