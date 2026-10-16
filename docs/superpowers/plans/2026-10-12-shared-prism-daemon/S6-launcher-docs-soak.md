# S6 — Opt-in Launchers, Shared-Mode Docs and Soak Instrumentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-16).** Every task of this plan was built before the plan was written, on the local scratch branch `scratch/s6-validate` (worktree `~/code/prism-s6-validate`) from `main` `61c2a9e5`, and every mutant named in a task was applied there, each alone: 30 of 30 were killed. The source was then cut into this plan's tasks on `scratch/s6-tasks`, the plan's directives were generated from that branch's per-task commits, and the plan was replayed by a script onto `61c2a9e5` (`scratch/s6-replay`), each anchor matching once, to a tree byte-identical with `scratch/s6-tasks` and with the validated code; each task's red and green steps below were run on partial replays. The launcher's tests and the soak tools' tests (the live fan-out run against a container-built binary included) were run on Linux (aarch64, the bridge's `a2a-toolchain` image, as root), 10 times each (V11). The mutation gate was not run (it selects no MCP code and no Python; lane README). **The soak itself is not code and was not run**: it is the lane's acceptance, specified under "The soak" for the controller, after the owner has answered the questions below.

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file that was then read.

| # | Command | Must show | Observed |
|---|---|---|---|
| V1 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` on the untouched base | The base total; later totals are "base + N". | `52 passed`, the goldens' test `ok`, at `61c2a9e5`. |
| V2 | Each task's red step, on a partial replay of this plan | What the step says. | Task 1: `test result: FAILED. 3 passed; 3 failed`, the three named. Task 2: `ModuleNotFoundError: No module named 'soak'`, `FAILED (errors=1)`. Task 3: `No module named 'fanout'` twice, `FAILED (errors=2)`. |
| V3 | Each task's green step, on the replay | The count the step states, no warning. | Task 1: `6 passed` (52 filtered out), no `warning`. Task 2: `Ran 16 tests`, `OK`. Task 3: `Ran 33 tests`, `OK (skipped=1)`; with `PRISM_SOAK_BIN`, `Ran 33 tests`, `OK`. |
| V4 | `cargo test --features mcp` (the whole suite) and `cargo test --doc --features mcp` | `0 failed`. | 31 `test result` lines, **6,232 passed, 0 failed, 1 ignored**; doc tests `2 passed`. Run at `c9be7b01`, whose `src/`, `tests/`, `Cargo.toml`, `Cargo.lock` and `build.rs` equal the head's (`git diff --stat c9be7b01 9e82b670 -- src tests Cargo.toml Cargo.lock build.rs` is empty; later commits touch only `scripts/soak/`). The base's total, not run, is 6,232 − 6 = 6,226 by the same count. |
| V5 | The soak tools' tests, with and without `PRISM_SOAK_BIN`, and memreport's | All `OK`. | `Ran 33 tests`, `OK (skipped=1)`; with the debug binary `Ran 33 tests`, `OK`; `scripts/memreport` (which `fanout.py` imports) `Ran 349 tests`, `OK`. |
| V6 | `cargo check`, `cargo check --features mcp`, `cargo fmt --check` | `Finished`; no warning; fmt silent. | Both `Finished`, 0 warnings; fmt silent at the head and at every task commit of the cut. |
| V7 | `cargo clippy --all-targets --features mcp -- -W clippy::all`, filtered to the touched files | No line in a touched file. | 313 `warning` lines, none naming `tests/mcp/launcher_test.rs` or `tests/mcp/main.rs` (the only Rust files touched). |
| V8 | Every "prove the tests bite" step | The named tests fail under the named mutant. | L1–L5, S1–S13, F1–F12: **30 of 30 killed**, each alone, at the head (`9e82b670`; L1–L5 at `f97a04ed`, whose launcher and its tests are the head's). S7 is killed only where the local time zone is not UTC (this host: MDT). |
| V9 | Repeats on macOS | All `ok`; no daemon left. | At the head (`9e82b670`): `10 test result: ok. 6 passed` (`--test mcp launcher`) and `10 Ran 33 tests` / `10 OK` (the soak tools with the debug binary, the live test included); `pgrep -fl 'prism-mcp daemon'` empty afterwards. |
| V10 | The zero-diff and replay checks | Nothing in `src/`, `.claude-plugin/` or the goldens; the replay byte-identical. | `git diff --stat 61c2a9e5 9e82b670 -- src .claude-plugin tests/fixtures/mcp/golden` is empty. The replay `scratch/s6-replay` (`525b7659`, 20 directives, every anchor matching once) has tree `c6aeb376`, the tree of `scratch/s6-tasks` (`bf7d3c23`) and of the validated head (`9e82b670`), executable modes included. |
| V11 | Linux: the bridge's `a2a-toolchain` image (Debian, aarch64, rustc 1.94, Python 3.12, uid 0, no `ps`/`pgrep`), Task 4 Step 4's script, at the head; the disk checked (111 GiB free) in a command of its own first | All `ok`. | `failed Linux runs: 0`: `10 test result: ok. 6 passed` (`--test mcp launcher`, the process test included), `10 Ran 33 tests` / `10 OK` (the soak tools, the live fan-out test against the container-built binary included); no `left:` line; volumes removed by the `trap`. x86_64 Linux: CI only (the launcher's tests; CI runs no Python here, D117). |

## Owner questions (to answer before dispatch)

Each is a behavior or a value the spec leaves to the owner, or an action outside this repository that needs the owner's approval. The plan is built on the recommended answer; a different answer changes the named place only.

1. **How a Claude Code plugin user opts in (D110).** The plugin's MCP arguments live in its installed manifest (`.claude-plugin/plugin.json`, copied into `~/.claude/plugins/cache/…`), which a user does not edit. **Recommended: (A) an environment variable, `PRISM_MCP_SHARED=1`, read by the launcher** (built and tested here): opt-in per user and per shell, nothing changes for anyone else, and the default flip later is one line in the launcher. (B) `--shared` in `plugin.json` for every plugin user: makes shared mode the default for the plugin before the soak, against spec §8 step 1 and §11 item 2. (C) Documentation only: a plugin user cannot opt in without editing the cache.
2. **The runtime directory on this host (SI-42, D115).** Every launcher that should share a daemon must resolve one directory. Claude Code passes `$TMPDIR` (measured); whether a host-run Codex does was not measured. **Recommended: set `PRISM_MCP_RUNTIME_DIR=/tmp` explicitly everywhere** — in `.codex/config.toml` (Task 4), in the bridge's `[[agents.mcp.env]]`, and in the shell profile Claude Code and the operator start from — giving `/tmp/prism-mcpd`: one literal for every configuration file, short (37-byte socket paths), the spec's own Linux fallback, cleared at reboot. Cost: `/tmp/prism-mcpd` is one name per host, so on a host with a second user that user must set another value (this Mac has one). Alternatives: (B) leave it unset and rely on `$TMPDIR` reaching every launcher (a split costs a second index, silently, until the soak's `one_index_per_key` rule shows it); (C) a directory under the home, for example `~/.local/state/prism-mcp` (also valid; each configuration file then carries the home path).
3. **One cache directory for the bridge and for interactive sessions.** The host-run bridge configurations pass `--cache-dir ~/.local/share/a2a/prism-cache-host`; interactive sessions and the project's `.codex/config.toml` use the default cache. The cache base is in the identity key (spec §4.3), so the same checkout gets two daemons and two indexes during a bridge round. **Recommended: drop `--cache-dir` from the host-run bridge entries for the soak**, so bridge agents and interactive sessions share one daemon per checkout (the default cache of `~/code/slicing` is already warm from interactive use). Alternative: keep it (two daemons per checkout; the soak report shows them as `max_daemons_per_repo` 2, information, not a failure). Containerized configurations are not affected (their agents have daemons of their own inside their containers).
4. **When `.codex/config.toml` changes.** A checkout with the new configuration runs `prism-mcp --shared` from `~/code/slicing/target/release/prism-mcp`; the binary deployed there today (`9d5a67bc`, before S5a) rejects `--shared`, and Codex then has no prism server in that checkout. **Recommended: keep the change in this pull request and merge it only after the soak's Phase 0 has installed a binary of this slice's merge at that path.** Alternative: drop the file from Task 4 and apply it at Phase 0.
5. **The soak's deployment, outside this repository** (Phase 0 below): replace the two deployed binaries (`~/code/slicing/target/release/prism-mcp`, `~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release/prism-mcp`) and the plugin cache's launcher by rename (`.pre-s6` kept), export the two variables in the shell profile, and edit the bridge configurations the soak uses. **Recommended: yes, as listed, with the rollback stated there.** No step of this plan does any of it; the controller does it after this approval.
6. **The plugin's version.** **Recommended: no bump in S6** (the opt-in is additive and this host's plugin is updated by Phase 0's file replacement); bump at the default flip, noting that an updated plugin's new cache directory has no built binary, so the launcher falls back to `PATH` (and fails with its "build it" message when there is none).
7. **The soak's pass thresholds** (each phase below). **Recommended:** fan-out: on excalidraw, eight shims against eight standalone servers, three interleaved pairs, every run admissible, the shared median p95 at most 1.10× and median p99 at most 1.25× the standalone medians (`P95_MAX_RATIO`, `P99_MAX_RATIO`); bridge round: all five report rules pass (`one_index_per_key`, `no_standalone_index`, `no_fail_closed`, `no_abort`, `no_critical_pressure`); week: `one_index_per_key`, `no_fail_closed` and `no_abort` pass, with no restart-bound refusal, and the other two reported. Alternatives: a looser latency bound (1.25×/1.5×) if the host cannot be made quiet; a footprint ceiling per key (for example the corpus's C1 refresh peak) in place of `no_critical_pressure` for the round.
8. **The week's scope and floor (SI-41).** **Recommended:** Claude Code sessions through the plugin, bridge rounds, and Codex sessions in the slicing checkout count; the week counts when at least 5 of its 7 days have a sample with at least one shim; fail-closed lines are counted from Claude Code's logs and the wrapper's logs, and the project's Codex sessions are pointed at the wrapper for the week (a local, uncommitted change to that checkout's `.codex/config.toml`) or are left uncounted, disclosed. A week below the floor is extended, not passed.
9. **The latency baseline for larger repositories (SI-39).** Eight standalone servers of slicing need about 8 × 2 GiB on this 24 GB host (C1 readout), which is the incident itself. **Recommended: gate on excalidraw (8 × about 0.7 GiB) and report slicing as information**: eight shims against one standalone client running the same workload, three runs each.
10. **The bridge's reaper** (`daemon-shutdown --pid --start-time`, S5b). **Recommended: not built for the soak**: after a round the idle TTL (15 min) frees the index; build it in the bridge only if the round's report shows the residency after rounds matters.

---

**Goal:** the lane's: let the a2a-bridge run its reviewer and implementer fan-out on this host without swap exhaustion, by holding one resident index per compatible (checkout, prism build, cache configuration, refresh policy), with no query-latency regression under fan-out. **What S6 must make true, and test:** a user can opt in to `--shared` from every launcher the spec names (spec §8 step 1: the project's `.codex/config.toml`, the Claude Code plugin's launcher, the bridge's `[[agents.mcp]]` entry, with `PRISM_MCP_RUNTIME_DIR`), without editing the installed plugin and without changing what anyone gets who does not opt in; a user can find in `docs/MCP.md` everything the earlier slices left to S6 (what changes for a client, every fail-closed reason and what to do, the operator's commands, the runtime directory rule); and the controller has the instruments the §8 soak needs (resident index count and aggregate footprint over time, fail-closed lines, daemon restarts and aborts, fan-out latency against standalone), with the soak itself specified precisely enough to run and to judge. Nothing in `src/` changes: the in-process server, the shim, the daemon and the operator's commands are as S5b left them, and `--shared` stays opt-in.

**Architecture:** Three independent pieces. (1) **The plugin launcher** (`scripts/prism-mcp-launch.sh`) reads `PRISM_MCP_SHARED`: `1` inserts `--shared` after `--repo`, unset/empty/`0` changes nothing, anything else is refused before the binary runs, and an explicit `--shared`/`--standalone` argument wins (D110–D113). Claude Code hands plugin MCP servers its whole environment (measured, SI-42), so an `export` in the user's profile reaches the launcher; `plugin.json` is unchanged. The other launchers take `"--shared"` in their arguments (`.codex/config.toml` in this repo; the bridge's configuration, documented, outside the repo). (2) **Docs:** a "Shared daemon" section of `docs/MCP.md`, a bullet of the navigation skill, a paragraph of `CLAUDE.md`. (3) **Soak instrumentation** in `scripts/soak/` (Python 3.9, standard library): `soak.py sample` appends, per interval, every `prism-mcp` process classified from its command line (daemon, shim, standalone, operator), its `phys_footprint` and the host's swap, memory pressure and compressor; `soak.py report` judges the soak's rules from those samples, the shims' standard-error lines (Claude Code's MCP logs, and `prism-mcp-logged.sh` for clients that keep none) and macOS crash reports; `fanout.py run` drives N real clients of one corpus in shared or standalone mode through the C1 pinned workloads with one concurrent `refresh_index`, timing every call, and `fanout.py compare` judges shared against standalone under a pinned rule. The sampler only reads the process table; it never connects to a daemon unless asked to store `daemon-status` text (D121).

**Tech Stack:** bash (3.2-compatible: macOS's `/bin/bash`), Rust 2021 for the launcher's tests (`std::process`, `tempfile`, the S4/S5a process-test helpers), Python 3.9 standard library for `scripts/soak/` (it imports `scripts/memreport/` for workload binding and host state). No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§4.3 runtime directory, §4.6, §4.7, §5, §6.2 fan-out gate, §8 steps 1–4, §9 "Soak: §8 step 2", §10, §11 items 1–3, §12 row S6). Lane plan: `README.md` in this directory (the S6 note on the launcher; the S2a/S2b/S4 notes on docs). Built on S5b as merged: `main` `61c2a9e5`. Decisions are numbered on from the S5b plan's (D97–D109) and spec issues from its SI-29–SI-35.

**Interfaces this plan settles** (from the "S6 owes" lists of S3a, S3b, S4, S5a and S5b, each checked under "Self-review record"): the readiness vocabulary, what a client sees on a crash or a drain, the daemon's command line, exit statuses and log lines, the runtime-directory rule, the fail-closed line and every reason, D85's silently ignored flags, D77's minute and its clock, SI-27's rebuilt binary, the `sudo -E` trap, the operator's commands and exit statuses, SI-35's old daemons and `kill -TERM` (all in `docs/MCP.md`); `--shared` in `.codex/config.toml` and the plugin launcher (Tasks 1, 4); the bridge's entry and `PRISM_MCP_RUNTIME_DIR` (documented; applied at the soak with the owner's approval, SI-37); the soak's instruments (Tasks 2, 3) and the soak (specified, "The soak").

**Ships as one pull request** (D114): 15 files, 1,773 insertions and 1 deletion. **Production** (what runs outside a test): 833 lines — `scripts/soak/soak.py` 407, `scripts/soak/fanout.py` 381, `scripts/prism-mcp-launch.sh` +22, `scripts/soak/prism-mcp-logged.sh` 21, `.codex/config.toml` +2/−1; **tests** 748 lines (`scripts/soak/test_soak.py` 213, `test_fanout.py` 204, `tests/mcp/launcher_test.rs` 189, `test_fanout_live.py` 80, `fake_prism_mcp.py` 55, the stand-in 6, one `mod` line); **docs** 192 lines (`docs/MCP.md` +176, `CLAUDE.md` +12, the skill +4). Under the lane's threshold of about 1,000 production lines for a split, and the pieces share one reviewer question (can the soak start, and will it judge the flip correctly?), so one pull request (D114). Nothing in `src/`.

## Global Constraints

- **Nothing under `src/` changes**, and nothing in `.claude-plugin/`. Standalone behavior, the shim, the daemon and the operator's commands are unchanged; the golden transcripts under `tests/fixtures/mcp/golden/` are not touched (never set `PRISM_MCP_BLESS_GOLDEN`). Task 4 checks it.
- **Opt-in stays opt-in.** No launcher in the repository adds `--shared` for a user who has not asked for it: the plugin's launcher only on `PRISM_MCP_SHARED=1`; `.codex/config.toml` is this repository's own configuration (owner question 4).
- **The launcher runs under bash 3.2** (macOS's `/bin/bash`): no arrays expanded under `set -u`, no `${var,,}`; the tests run it with `/bin/bash`. A value of `PRISM_MCP_SHARED` the launcher does not accept stops it with exit 2 and one line naming the variable, before anything else runs.
- **The soak tools are Python 3.9, standard library only** (this host's `python3` is 3.9.6; the image's is 3.12), and never connect to a daemon unless `--status-bin` is given (`sample`) or the run is a shared fan-out arm, whose daemon is its own. They are measurement instruments: they run on the controller's request, never from CI. Their unit tests are hermetic (a fake server, temporary directories); the live test runs only with `PRISM_SOAK_BIN`.
- **One measurement process at a time** (lane rule): `fanout.py` and `soak.py sample` record the host's state with every run so that an overlap is visible; the soak's runbook (below) never overlaps two measured runs.
- **Tests do not depend on the host's speed.** No test asserts an upper bound on a duration; the process tests wait for a condition up to their hang limit (60 s); the one subprocess test of the fan-out driver bounds only a hang (60 s).
- **No test writes a file it then executes**: the launcher's stand-in is a committed executable fixture reached through a symbolic link (on Linux a freshly written executable can fail with `ETXTBSY` while another test thread forks); the fake server and the wrapper's stand-in are a committed script and `sys.executable`.
- **Every process a test starts is ended on every path** (`Launched`'s and S5a's `Daemons`' `Drop`; the fan-out run's own `daemon-shutdown` and its 60 s `SIGTERM` fallback; the client watchdog). After every run of the suite: `pgrep -fl 'prism-mcp daemon'` prints nothing.
- File size (CLAUDE.md decision 7): every file this slice creates is under 600 weighted lines (`scripts/file_size.py`, which weighs Rust; the Python files are counted by plain lines: `soak.py` 407, `fanout.py` 381, `test_soak.py` 213, `test_fanout.py` 204, `test_fanout_live.py` 80, `fake_prism_mcp.py` 55; `tests/mcp/launcher_test.rs` weighs 168.0).
- No new clippy warning in a file this slice adds or edits (one Rust file, `tests/mcp/launcher_test.rs`, and one line of `tests/mcp/main.rs`).
- **Commit before any experiment.** The "prove the tests bite" steps edit a file and restore it with `git checkout -- <file>`; they come after the task's commit.
- **Run the launcher's tests and the soak tools' tests on Linux before the pull request** (Task 4), in the bridge's `a2a-toolchain` image with the worktree mounted read-only and named volumes for the target and the registry, removed by a `trap`.
- Logs go to files in a directory of the implementer's own, `$LOG_DIR` below; read the file. Do not pipe cargo output into `grep` or `tail`; `grep` the saved logs afterwards. Every cargo command runs alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, written out in full (in zsh an unquoted `$J cargo` is one word, S5b correction 3). **Before any large build or Docker run, check `df -h /System/Volumes/Data` in a separate command and keep at least 50 GiB free.**

### What each mechanism covers, and what it does not

- **The launcher's opt-in reaches only the plugin's server.** Claude Code passes its whole environment to plugin MCP servers (SI-42), so `PRISM_MCP_SHARED=1` in the profile Claude Code starts from opts every new session in; a session started before the export keeps its standalone server until it restarts. Codex, Kiro, a manual `claude mcp add` and the bridge do not run this launcher: they take `"--shared"` in their arguments.
- **A runtime directory that differs between two launchers is not an error**: it gives a second daemon, a second index, and no failure anywhere. The docs tell users to set `PRISM_MCP_RUNTIME_DIR` to one value; the soak's `one_index_per_key` rule catches it after the fact (two daemons of one key at once can only be in two runtime directories, since one directory's owner lock admits one).
- **Different cache directories, refresh policies or builds give different daemons by design** (spec §4.3); the soak report lists repositories with more than one daemon at once (`max_daemons_per_repo`), as information for owner question 3, never as a failure.
- **An index holder is recognized, not measured exactly**: every daemon, and a standalone server whose footprint is at least 100 MiB (one that has built nothing is a few MiB; D118). A standalone server in the middle of its first load is counted from the sample in which it passes 100 MiB.
- **A command line is split on spaces.** A repository path with a space in it is cut short in the report's labels; roles do not depend on paths.
- **The sampler is blind between samples.** A daemon that lives and dies inside one interval (an abort at its first build) is missing from the samples; it is counted by its crash report and by the shims' lines instead.
- **Fail-closed lines are counted only where they are kept**: Claude Code's MCP logs (measured to hold the server's standard error) and the files `prism-mcp-logged.sh` writes. A Codex session launched by the project's `.codex/config.toml` keeps none unless the owner points it at the wrapper for the soak (owner question 8).
- **Crash reports name the process**: `prism-mcp-<date>.ips`. Test binaries (`prism-<hash>`) are not counted. A crash report outside the window or deleted before the report is missed.
- **`fanout.py` times a call from its write to its response at the client**, one call outstanding per client, as an agent makes them. It measures the whole path (pipe, shim, socket, daemon, query, writes back), not the query alone. A run's refresh stalls every shared client's calls for the length of the refresh under S3b's write lock (SI-11), and only client 0's in a standalone run; the p95/p99 rule reads that as it is.
- **`fanout.py` does not stop another prism-mcp process** and does not refuse to run beside one: the runbook makes the host quiet first and the run records the host's state before and after.
- **The pinned rule compares medians across runs**; one bad run in three does not decide it. All runs must be admissible (every call answered, none retryable after warm-up, one daemon instance for a shared arm).

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously, or an earlier slice's note this slice was told to settle. "Cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D110 | **A Claude Code plugin user opts in with `PRISM_MCP_SHARED=1`, read by `scripts/prism-mcp-launch.sh`; `plugin.json` is unchanged.** | Spec §8 step 1 names "the Claude plugin launcher"; its arguments are in the installed manifest, which a user does not edit, and an unconditional `--shared` there would flip the plugin's default before the soak (spec §8 step 1 "the default stays in-process", §11 item 2). Claude Code hands plugin servers its whole environment (SI-42), so an `export` reaches the launcher. Owner question 1. | Owner question 1's other answers: one line of `plugin.json`, or nothing. |
| D111 | **Only `1` opts in; unset, empty and `0` do not; any other value stops the launcher with exit 2 and one line naming the variable, before the binary is looked for.** | A typo (`yes`, `true`, ` 1`) must decide nothing silently in either direction: silently standalone is the incident under fan-out, silently shared is an opt-in nobody gave. Exit 2 is the shell's usage error; 127 stays "no binary". | Accepting more spellings: one `case` arm. |
| D112 | **An explicit `--shared` or `--standalone` among the passed-through arguments wins over the variable** (nothing is added). | `--shared` twice is a clap error, and `--shared --standalone` a conflict: the launcher must not turn a user's explicit choice into a startup failure. | None. |
| D113 | **`--shared` goes right after `--repo <path>`**, before the passed-through arguments. | Clap accepts any order; a fixed place gives one argument vector to test (mutant L5 pins it), and `set -- --shared "$@"` is bash 3.2 safe, where an empty array expanded under `set -u` is not. | None. |
| D114 | **One pull request.** | 829 production lines, under the lane's about 1,000 for a split; the launcher, the docs and the soak's instruments answer one reviewer question (can the soak start, and will it judge the flip correctly?). | A split along the tasks (1 and 4, then 2 and 3). |
| D115 | **The repository's `.codex/config.toml` adds `--shared` and `env = { PRISM_MCP_RUNTIME_DIR = "/tmp" }`.** | Spec §8 step 1 names the file; it is this repository's own configuration (absolute paths of this host), not a user default. The runtime directory is owner question 2; when the change lands, owner question 4. | One file. |
| D116 | **Users' documentation of shared mode is one section of `docs/MCP.md`** (with two flag rows and two environment rows), plus one bullet of the navigation skill and a paragraph of `CLAUDE.md`. | Spec §4.3: "`docs/MCP.md` says so"; every "S6 owes" list points there; a separate page would split the flags from the server's other flags. The skill learns the retryable answers (S3a's note: neither the docs nor the skills named `warming` or `build_failed`). | Moving the section to its own page. |
| D117 | **The soak's instruments are Python 3.9 standard library in `scripts/soak/`, tested by `unittest` and not by CI**, like `scripts/memreport/`. `fanout.py` imports memreport's workload binding and host state. | They are run by the controller on this host (Python 3.9.6), never by users or CI; CI's Python job runs only `eval/`. The live test needs a built binary and runs only with `PRISM_SOAK_BIN`. Disclosed in the PR. | A CI step `python3 -m unittest discover -s scripts/soak` (and memreport), a few lines of `ci.yml`. |
| D118 | **An index holder is every daemon, and a standalone server whose footprint is at least 100 MiB.** | A lazy standalone server that has built nothing is about 2–4 MiB (measured: 1.8 MiB for this session's plugin server); a built index is hundreds of MiB to GiB (C1). A shim never holds one. | A constant. |
| D119 | **`fanout.py` times each call at the client, from its write to its response, one call outstanding per client**; clients warm up first (the first workload call, retried while it is answered retryable), then start together at a barrier, each from its own share of the way through the workload; client 0 calls `refresh_index` once, half way through; percentiles are nearest-rank; refresh calls are reported apart. The workload is memreport's pinned one for the corpus, bound to the corpus's commit. | It measures what an agent waits for, under the interleaving fan-out produces, with the refresh the spec's §6.2 gate names, and the C1 workloads are already pinned and checked. Lockstep clients would measure one seed eight times; a warming answer counted as a timed call would measure the build. | Re-running with other parameters: flags. |
| D120 | **Fail-closed lines and daemon losses are read from the clients' standard error: Claude Code's MCP logs (`~/Library/Caches/claude-cli-nodejs/<project>/mcp-logs-<server containing prism>/*.jsonl`, entries `"error": "Server stderr: …"`, measured) and `prism-mcp-logged.sh` logs for clients that keep none; aborts from macOS crash reports `prism-mcp[-_]*.ips`.** | A fail-closed shim reaches no daemon, so no daemon file records it; the shim's own line (S5a's D87) is the only record. Daemon logs carry no time stamps and are truncated at a start over 1 MiB; crash reports are time-stamped and name the process (test binaries are `prism-<hash>` and are not counted). | A product-side event log (a shim appending to `<runtime>/prism-mcpd/`), which would touch S5a's code. |
| D121 | **The sampler reads the process table (`ps -A -ww -o pid=,ppid=,rss=,command=`) and `footprint`; it never connects to a daemon** unless `--status-bin` is given, in which case each sample stores `daemon-status` text. | Sampling must not change what it samples: a daemon of a build before S5b restarts its idle time at every `STATUS` probe (SI-35). | — |
| D122 | **A shared fan-out run uses a runtime directory of its own under `/tmp`, stops its daemon with `daemon-shutdown` (expecting exit 0, then waits up to 60 s for the pid to go, then `SIGTERM`) and removes the directory; every client starts with `PRISM_MCP_STRUCTURED_CONTENT`, `PRISM_MCP_CONCISE_SHAPE`, `PRISM_MCP_MAX_RESULT_CHARS`, `PRISM_MCP_SHARED`, `PRISM_MCP_RUNTIME_DIR` and `PRISM_MCP_MEM_REPORT` removed from its environment.** | A run must not reach, or leave, a daemon anyone else uses, and both arms must answer with the same output settings whatever the controller's shell holds. | None. |
| D123 | **`fanout.py compare` judges the medians across runs: candidate p95 ≤ `P95_MAX_RATIO` × baseline p95 and candidate p99 ≤ `P99_MAX_RATIO` × baseline p99 (1.10 and 1.25), all runs admissible and of one corpus commit and workload.** | Spec §6.2 "no regression on … fan-out latency (8 shims, p95/p99 …)" and §8 step 2 "fan-out latency p95 vs. standalone" need a pinned rule before a run; timings on this host vary run to run (M2: ±20 % on a loaded host), so one run decides nothing and a median of three is the least that does. The constants are owner question 7. | Two constants. |

## Spec items implemented, and spec issues found

Spec §12 row S6, item by item: "opt-in launcher/config changes (`.codex/config.toml`, plugin launcher, bridge entry, `PRISM_MCP_RUNTIME_DIR`)" → Task 4 (`.codex/config.toml`), Task 1 (the launcher), `docs/MCP.md` and "The soak" Phase 0 (the bridge entry and the runtime directory, SI-37); "docs" → Task 4; "soak instrumentation" → Tasks 2 and 3, and the soak's specification. Spec §9 lists no numbered test for S6 ("Soak: §8 step 2"); the tests here are this plan's own, listed under "Claims".

- **SI-36 (§8 step 1 "the Claude plugin launcher … add `--shared`"; lane README's S6 note "a config change, not a script change").** The plugin's arguments are in its installed manifest, not in user configuration, and an unconditional `--shared` would make shared mode the plugin's default before the soak. Read as: the launcher reads an opt-in from the environment (D110; owner question 1).
- **SI-37 (§8 step 1 "the bridge's `[[agents.mcp]]` entry add[s] `--shared`. The bridge also sets `PRISM_MCP_RUNTIME_DIR`").** The bridge's configurations live outside this repository (`~/.config/a2a-bridge/*.toml`, and the bridge's own `examples/`), which this slice does not edit. S6 documents the entry (`docs/MCP.md`, with a TOML snippet parsed in validation) and the soak's Phase 0 applies it with the owner's approval (owner question 5).
- **SI-38 (§8 step 2 "aggregate footprint (the 10-03 incident's own probe commands)").** Those commands are recorded nowhere found: not in the spec, the lane's plans or handoff, the controller handoffs of 2026-10-03 to 2026-10-09, the project memory, or `~/prism-evidence`. The soak measures what the incident reported: per-process `phys_footprint` (`footprint`), resident size (`ps`), and the host's swap (`sysctl vm.swapusage`), memory pressure level (`kern.memorystatus_vm_pressure_level`) and compressor (`vm_stat`, "Pages occupied by compressor"), which is the incident's "34 GiB compressed".
- **SI-39 (§8 step 2 "fan-out latency p95 vs. standalone").** Eight standalone servers of a mid-size repository are the memory incident itself (slicing: 8 × about 2 GiB of footprint on a 24 GB host). Read as: a like-for-like comparison of eight shims against eight standalone servers on the smallest pinned corpus (excalidraw, about 0.7 GiB each), and eight shims against one standalone client on slicing as information (owner question 9).
- **SI-40 (§8 step 2 "daemon restarts").** A daemon that starts again after an idle retirement is the design (TTL 15 minutes), not a failure. Read as: daemon instances are reported (from the samples and from the shims' `shared daemon pid` lines); what fails the soak is an abort (a crash report) or a restart-bound refusal (`died during its index build`).
- **SI-41 (§8 step 2 "one week of interactive use").** A week in which few sessions opted in shows nothing. Read as: the week counts only with soaked launchers in use, at least 5 of 7 days with a sample holding a shim (owner question 8); a shorter one is extended.
- **SI-42 (§4.3 "Every launcher that should share a daemon must resolve the same directory; `docs/MCP.md` says so").** Measured: a plugin `prism-mcp` started by Claude Code on this host has the user's whole shell environment (its `ps eww` shows `TMPDIR`, `HOME`, `PATH`, and the user's own variables such as `GOROOT`), so its shims resolve `$TMPDIR/prism-mcpd`. Inherited, not measured here: the bridge's investigation of a containerized Codex (`~/code/a2a-bridge/docs/containerized-mcp-env-trap.md`) found its MCP servers started with `HOME`, a `PATH` of Codex's own and the configured `env` only. Whether a host-run Codex passes `TMPDIR` was not measured. A server whose directory differs from its peers' starts a second daemon silently. Read as: set `PRISM_MCP_RUNTIME_DIR` explicitly in every launcher (owner question 2); the soak's `one_index_per_key` rule detects a split.

## Review Focus

Ten ways this work is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code, or is a question for the owner.

1. **A launcher change that opts in users who did not ask**, or opts out users who did. → Task 1, `without_the_variable_the_arguments_pass_through_unchanged`, `zero_or_empty_changes_nothing`, `prism_mcp_shared_1_adds_shared_after_the_repository`, `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` (a real shim reaches a real daemon through the launcher).
2. **A typo that silently decides** which mode a session gets (`PRISM_MCP_SHARED=yes`, `true`, ` 1`). → Task 1, `any_other_value_is_refused_before_anything_runs` (the binary never runs).
3. **The launcher's own `--shared` colliding with the user's** (`--shared --shared` is a clap error; `--shared --standalone` a conflict). → Task 1, `an_explicit_shared_or_standalone_argument_wins_over_the_variable`.
4. **A launcher that only works under a recent bash.** `"${array[@]}"` of an empty array under `set -u` fails in bash 3.2, the `/bin/bash` of every Mac. → Task 1 runs every argument test under `/bin/bash` (3.2 on macOS) and the process test through the script's own `#!`.
5. **Two clients that should share a daemon and do not** (different runtime directory, cache directory or build) — invisible to every client. → Task 2, `test_two_daemons_of_one_key_at_once_are_found`, `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates`; the docs' runtime-directory section; owner questions 2 and 3.
6. **A soak report that passes because it saw nothing**: fail-closed lines read from the wrong log, a time window in local time, crash reports of other programs. → Task 2, `test_claude_code_logs_are_read_for_prism_servers_in_the_window`, `test_iso_times_are_utc`, `test_crash_reports_of_prism_mcp_in_the_window_are_counted`, `test_report_writes_the_table_and_the_verdicts`; SI-41's usage floor.
7. **A latency comparison that is not like for like**: a client that inherits `PRISM_MCP_CONCISE_SHAPE=legacy` from the controller's shell, a warm-up answered `warming` counted as a timed call, clients in lockstep on the same seed. → Task 3, `test_clients_start_from_the_default_output_settings`, `test_every_client_makes_every_call_and_client_0_refreshes_once` (two warming answers first), `test_each_client_starts_its_own_share_of_the_way_through`.
8. **A fan-out run that hangs or leaves a daemon behind** when a client dies. → Task 3, `test_clients_that_die_while_warming_up_end_the_run`, `test_a_client_that_dies_makes_the_run_inadmissible_without_hanging_it`, `test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down`; the live test asserts the daemon is gone.
9. **A docs claim that does not match the code** (a reason's wording, an exit status, a default). → The docs quote the code's own strings; the reviewer is asked to check each row of the fail-closed table and the operator's exit statuses against `src/mcp/shim/{arbitrate,connect,diagnostic,run,pump,status,shutdown}.rs` and `src/mcp/shared/{runtime_dir,paths}.rs`.
10. **A soak that cannot fail, or cannot pass.** → "The soak": each rule names what would fail it; the pass thresholds are owner question 7; a gate shown unreachable by its own measurements goes to the owner at once (lane rule).

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or docs state about behavior this slice adds, with the test that fails when it breaks and one wrong implementation that test rules out. Every mutant in the last column was applied in validation and the test failed.

| Claim | Test | A mutant it kills |
|---|---|---|
| Without `PRISM_MCP_SHARED`, or with it empty or `0`, the launcher passes its arguments through unchanged. | T1 `without_the_variable_the_arguments_pass_through_unchanged`, `zero_or_empty_changes_nothing` | `0` taken as on (L4) |
| `PRISM_MCP_SHARED=1` adds exactly one `--shared`, right after `--repo <path>`. | T1 `prism_mcp_shared_1_adds_shared_after_the_repository` | the variable ignored (L1); the flag last (L5) |
| The opt-in reaches a real shared daemon through the launcher run by its own `#!`. | T1 `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` | the variable ignored (L1) |
| An explicit `--shared` or `--standalone` always wins over the variable. | T1 `an_explicit_shared_or_standalone_argument_wins_over_the_variable` | no override (L2) |
| Any other value stops the launcher with exit 2 before the binary runs. | T1 `any_other_value_is_refused_before_anything_runs` | any value opts in (L3) |
| A process is a shim only with `--shared`, a daemon only as `prism-mcp daemon`, an operator only as `daemon-status`/`daemon-shutdown`; a program whose binary is not named `prism-mcp` is never listed. | T2 `test_roles_come_from_the_command_line` | `--shared` not looked at (S1) |
| A standalone server is an index holder exactly from 100 MiB of footprint; a shim never is. | T2 `test_an_index_holder_is_a_daemon_or_a_standalone_server_past_100_mib` | `>` for `>=` (S2) |
| Two daemons of one key count as duplicates only when they are in the same sample. | T2 `test_two_daemons_of_one_key_at_once_are_found`, `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` | holders accumulated across samples (S3) |
| Only days with a shim in a sample count toward the week's floor. | T2 `test_only_days_with_a_shim_count_toward_the_week` | every sampled day (S13) |
| Only daemons count toward a repository's daemons. | T2 `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` | every role counted (S12) |
| A daemon instance a client reached is counted once however many lines name it. | T2 `test_client_lines_are_counted` | a list for the set (S4) |
| Only the logs of servers whose name contains `prism`, and only entries in the window, are read. | T2 `test_claude_code_logs_are_read_for_prism_servers_in_the_window` | every server's logs (S5); no window (S6) |
| Times are read as UTC. | T2 `test_iso_times_are_utc` | local time (S7; killed where the zone is not UTC) |
| Only prism-mcp's crash reports, and only in the window, count as aborts. | T2 `test_crash_reports_of_prism_mcp_in_the_window_are_counted` | any program's (S8) |
| Each rule fails on its own breach and on no other. | T2 `test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach` | critical taken as normal (S9) |
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

**Not held by a test:** the docs' prose about the shim, the daemon and the operator's commands (held by S4–S5b's tests of that code; checked by review against the code, Review Focus 9); the `.codex/config.toml` change (a configuration the owner reads; its TOML was parsed with Python 3.14's `tomllib` in validation); `footprints` on macOS against a real daemon, and `host_memory` (exercised by the live sample test and the live fan-out test, values not asserted); `stop_daemon`'s `SIGTERM` fallback (a daemon that ignores `daemon-shutdown` for 60 s; not constructed).

## File Structure

| File | What | Change | Task |
|---|---|---|---|
| `scripts/prism-mcp-launch.sh` | `PRISM_MCP_SHARED` read, refused or turned into `--shared` | Modify | 1 |
| `tests/fixtures/mcp/launcher/argv-stub.sh` | A stand-in binary that prints its arguments (executable) | Create | 1 |
| `tests/mcp/launcher_test.rs`, `tests/mcp/main.rs` | The launcher's argument tests and its process test | Create, modify | 1 |
| `scripts/soak/soak.py`, `scripts/soak/test_soak.py` | The sampler and the soak report | Create | 2 |
| `scripts/soak/prism-mcp-logged.sh` | A client's prism-mcp standard error logged with time and pid (executable) | Create | 2 |
| `scripts/soak/fanout.py`, `scripts/soak/test_fanout.py`, `scripts/soak/test_fanout_live.py` | The fan-out latency driver and its tests | Create | 3 |
| `scripts/soak/fake_prism_mcp.py` | A stand-in server for the driver's tests (executable) | Create | 3 |
| `docs/MCP.md` | "Shared daemon" section; two flags; two environment variables | Modify | 4 |
| `skills/prism-code-navigation/SKILL.md` | Retryable answers; shared refresh | Modify | 4 |
| `CLAUDE.md` | The launcher's opt-in and the soak tools | Modify | 4 |
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

Expected: `test result: FAILED. 3 passed; 3 failed` (52 filtered out), the three failures `any_other_value_is_refused_before_anything_runs` (`PRISM_MCP_SHARED="yes"`, `left: Some(0)`, `right: Some(2)`: the old launcher runs the binary whatever the value), `prism_mcp_shared_1_adds_shared_after_the_repository` (`left: ["--repo", "/some/repo", "--cache-dir", "/a dir"]`) and `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` (`no response: Err(Disconnected)`; the binary refuses `--daemon-start-timeout` without `--shared`: `error: the following required arguments were not provided: --shared`). The three that pass describe today's pass-through, which the opt-in must keep.

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
# Unset, empty or 0 changes nothing. Any other value is refused before anything runs, so a typo
# never decides silently which way a client goes.
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

Expected: `rc=0`, `test result: ok. 6 passed; 0 failed` (52 filtered out), no `warning` line; `pgrep` prints nothing.

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
| L1 | `  1) shared=1 ;;` → `  1) shared=0 ;;` (the variable ignored) | `prism_mcp_shared_1_adds_shared_after_the_repository`, `with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon` |
| L2 | `    --shared \| --standalone) shared=0 ;;` → `    --never-matches) shared=0 ;;` (no explicit argument wins) | `an_explicit_shared_or_standalone_argument_wins_over_the_variable` |
| L3 | a `*) shared=1 ;;` arm before the refusal (any value opts in) | `any_other_value_is_refused_before_anything_runs` |
| L4 | `"" \| 0) shared=0 ;;` / `1) shared=1 ;;` → `"") shared=0 ;;` / `0 \| 1) shared=1 ;;` (`0` opts in) | `zero_or_empty_changes_nothing` |
| L5 | `set -- --shared "$@"` → `set -- "$@" --shared` (the flag last) | `prism_mcp_shared_1_adds_shared_after_the_repository` |

---

### Task 2: The soak's sampler, report and client log wrapper

**Files:** Create `scripts/soak/soak.py`, `scripts/soak/test_soak.py`, `scripts/soak/prism-mcp-logged.sh` (executable).

**Interfaces** (Task 3 and the soak use them): `soak.footprints(pids) -> {pid: bytes}`; `soak.py sample --out FILE.jsonl [--interval S] [--count N | --duration S] [--status-bin BIN --runtime-dir BASE]`, one JSON object per line (`t`, `host`: `swap_mib` {`total`,`used`,`free`}, `pressure`, `compressed_bytes`; `processes`: `pid`, `ppid`, `role`, `repo`, `key`, `cache_dir`, `rss_bytes`, `footprint_bytes`, `holder`, `command`; `daemon_status` text when asked); `soak.py report --samples FILE --out FILE.md [--json FILE] [--since EPOCH] [--until EPOCH] [--claude-logs DIR] [--client-log FILE ...] [--crash-reports DIR]`, verdicts for `RULES` (`one_index_per_key`, `no_standalone_index`, `no_fail_closed`, `no_abort`, `no_critical_pressure`); `prism-mcp-logged.sh LOGFILE BIN [args...]`, lines `<UTC time> <pid> <line>`.

- [ ] **Step 1: Write the failing tests**

Create `scripts/soak/test_soak.py`:

````python
"""Tests of soak.py and prism-mcp-logged.sh.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
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
PS = """  101     1   4096 /x/target/release/prism-mcp daemon --repo /r --cache-dir /c --refresh-policy warn-only --key %s
  102    50   3000 /x/target/release/prism-mcp --repo /r --cache-dir /c --shared
  103    50 900000 /x/prism-mcp --repo /other
  104    50   2000 /x/prism-mcp daemon-status --runtime-dir /tmp
  105    50   1000 /usr/bin/vim prism-mcp
  106    50   1000 /x/prism-mcp.pre-389 --repo /r
""" % KEY


def proc(role, pid, footprint=0, key=KEY, repo='/r'):
    return {'role': role, 'pid': pid, 'key': key if role == 'daemon' else None, 'repo': repo,
            'footprint_bytes': footprint, 'holder': soak.is_holder({'role': role, 'footprint_bytes': footprint})}


def sample(t, processes, swap_used=0.0, pressure=1, compressed=0):
    return {'t': t, 'processes': processes,
            'host': {'swap_mib': {'used': swap_used}, 'pressure': pressure, 'compressed_bytes': compressed}}


class Processes(unittest.TestCase):
    def test_roles_come_from_the_command_line(self):
        found = {p['pid']: p for p in soak.parse_ps(PS)}
        self.assertEqual(sorted(found), [101, 102, 103, 104])
        self.assertEqual((found[101]['role'], found[101]['key'], found[101]['repo']), ('daemon', KEY, '/r'))
        self.assertEqual(found[102]['role'], 'shim')
        self.assertEqual((found[103]['role'], found[103]['rss_bytes']), ('standalone', 900000 * 1024))
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
        self.assertEqual(summary['standalone_holders'], {'/a': [7]})
        self.assertEqual(summary['peaks'], {'holders': 1, 'shims': 1, 'prism_footprint_bytes': 201 * MIB,
                                            'swap_used_mib': 10.0, 'pressure': 2, 'compressed_bytes': 9})

    def test_only_days_with_a_shim_count_toward_the_week(self):
        summary = soak.summarize_samples([sample(1, [proc('daemon', 1)]), sample(2 * 86400, [proc('shim', 2)]),
                                          sample(2 * 86400 + 60, [proc('shim', 2)])])
        self.assertEqual(summary['days_with_shims'], ['1970-01-03'])


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

    def test_iso_times_are_utc(self):
        self.assertEqual(soak.iso_epoch('1970-01-01T00:01:40Z'), 100)

    def test_crash_reports_of_prism_mcp_in_the_window_are_counted(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ('prism-mcp-2026-10-16-101010.ips', 'prism-mcp_2026-10-16-101011_host.crash',
                         'Safari-2026-10-16-101010.ips'):
                (Path(directory) / name).write_text('x')
            old = Path(directory) / 'prism-mcp-2026-01-01-000000.ips'
            old.write_text('x')
            os.utime(old, (100, 100))
            self.assertEqual(soak.crash_reports(directory, since=1000),
                             ['prism-mcp-2026-10-16-101010.ips', 'prism-mcp_2026-10-16-101011_host.crash'])
            self.assertEqual(soak.crash_reports(str(Path(directory) / 'missing')), [])


class Report(unittest.TestCase):
    def test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach(self):
        clean = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('shim', 2)])])
        quiet = soak.scan_lines([REACHED])
        self.assertTrue(all(soak.judge(clean, quiet, []).values()))
        two = soak.summarize_samples([sample(1, [proc('daemon', 1), proc('daemon', 2)])])
        self.assertEqual([r for r, ok in soak.judge(two, quiet, []).items() if not ok], ['one_index_per_key'])
        alone = soak.summarize_samples([sample(1, [proc('standalone', 3, 500 * MIB)])])
        self.assertEqual([r for r, ok in soak.judge(alone, quiet, []).items() if not ok], ['no_standalone_index'])
        failed = soak.scan_lines([FAILED])
        self.assertEqual([r for r, ok in soak.judge(clean, failed, []).items() if not ok], ['no_fail_closed'])
        self.assertEqual([r for r, ok in soak.judge(clean, quiet, ['prism-mcp-x.ips']).items() if not ok],
                         ['no_abort'])
        critical = soak.summarize_samples([sample(1, [], pressure=4)])
        self.assertEqual([r for r, ok in soak.judge(critical, quiet, []).items() if not ok],
                         ['no_critical_pressure'])

    def test_report_writes_the_table_and_the_verdicts(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            samples.write_text(json.dumps(sample(time.time(), [proc('daemon', 1)])) + '\n')
            client = Path(directory) / 'client.log'
            client.write_text('%s 77 %s\n' % (time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), FAILED))
            out, data = Path(directory) / 'r.md', Path(directory) / 'r.json'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--samples', str(samples), '--client-log', str(client), '--out', str(out),
                           '--json', str(data), '--crash-reports', directory])
            self.assertIn('| no_fail_closed | FAIL |', out.read_text())
            self.assertEqual(json.loads(data.read_text())['verdicts']['one_index_per_key'], True)

    def test_sample_appends_one_line_per_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 's.jsonl'
            with redirect_stdout(io.StringIO()):
                soak.main(['sample', '--out', str(out), '--count', '2', '--interval', '0'])
                soak.main(['sample', '--out', str(out), '--count', '1', '--interval', '0'])
            self.assertEqual(len(soak.load_samples(out)), 3)


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

- [ ] **Step 2: Run them and watch them fail**

```bash
python3 -m unittest discover -s scripts/soak -p 'test_soak.py' > "$LOG_DIR/t2-red.log" 2>&1; echo "rc=$?"
tail -5 "$LOG_DIR/t2-red.log"
```

Expected: `ImportError: Failed to import test module: test_soak`, `ModuleNotFoundError: No module named 'soak'`, `Ran 1 test`, `FAILED (errors=1)`.

- [ ] **Step 3: Implement the sampler, the report and the wrapper**

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

    soak.py sample --out FILE.jsonl [--interval 60] [--count N | --duration SECS]
                   [--status-bin BIN --runtime-dir BASE]
    soak.py report --samples FILE.jsonl --out FILE.md [--json FILE.json] [--since EPOCH] [--until EPOCH]
                   [--claude-logs DIR] [--client-log FILE ...] [--crash-reports DIR]

`sample` appends one JSON line per interval: every prism-mcp process (its role, repository, key,
resident size and footprint) and the host's swap, memory pressure and compressor. It reads the
process table and never connects to a daemon unless --status-bin is given, in which case it also
stores the text of `daemon-status`. Appending makes it resumable: run it again on the same file.

`report` reads the samples and the clients' standard error (Claude Code's MCP logs, the files
prism-mcp-logged.sh writes) and macOS crash reports, and judges the soak's pinned rules (RULES).

Python 3.9, standard library only. Unit tests:
    python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import argparse
import calendar
import json
import os
import re
import subprocess
import sys
import tempfile
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
CLIENT_LINE = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ) (\d+) (.*)')
CRASH_REPORT = re.compile(r'prism-mcp[-_].*\.(ips|crash)$')
RULES = ('one_index_per_key', 'no_standalone_index', 'no_fail_closed', 'no_abort', 'no_critical_pressure')


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


def classify(command):
    """Role and configuration of one process from its command line, or None for a process that is
    not prism-mcp. Roles: daemon, shim, standalone, operator. Words are split on spaces, so a path
    with a space in it is cut short; the role does not depend on paths."""
    tokens = command.split()
    if not tokens or os.path.basename(tokens[0]) != 'prism-mcp':
        return None
    verb = tokens[1] if len(tokens) > 1 else ''
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


def parse_ps(text):
    """prism-mcp processes in `ps -A -ww -o pid=,ppid=,rss=,command=` output."""
    processes = []
    for line in text.splitlines():
        fields = line.split(None, 3)
        if len(fields) < 4 or not fields[0].isdigit():
            continue
        found = classify(fields[3])
        if found is not None:
            found.update(pid=int(fields[0]), ppid=int(fields[1]), rss_bytes=int(fields[2]) * 1024,
                         command=fields[3])
            processes.append(found)
    return processes


def footprints(pids):
    """phys_footprint in bytes by pid (macOS `footprint`), or resident plus swapped (Linux /proc);
    a process that has gone is missing."""
    if not pids:
        return {}
    if sys.platform == 'darwin':
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'footprint.json'
            capture(['footprint', '-f', 'bytes', '--noCategories', '-j', str(report)] + [str(p) for p in pids])
            try:
                data = json.loads(report.read_text())
            except (OSError, ValueError):
                return {}
        return {entry['pid']: entry['auxiliary']['phys_footprint'] for entry in data.get('processes', [])
                if 'auxiliary' in entry and 'phys_footprint' in entry['auxiliary']}
    sizes = {}
    for pid in pids:
        try:
            status = Path('/proc/%d/status' % pid).read_text()
        except OSError:
            continue
        kib = [int(line.split()[1]) for line in status.splitlines() if line.startswith(('VmRSS:', 'VmSwap:'))]
        sizes[pid] = sum(kib) * 1024
    return sizes


def parse_swapusage(text):
    """MiB figures of `sysctl -n vm.swapusage`: total, used, free."""
    found = dict(re.findall(r'(total|used|free) = ([0-9.]+)M', text or ''))
    return {name: float(value) for name, value in found.items()}


def parse_compressed(vm_stat):
    """Bytes held by the compressor, from `vm_stat`; None when it does not say."""
    size = re.search(r'page size of (\d+) bytes', vm_stat or '')
    pages = re.search(r'Pages occupied by compressor:\s+(\d+)', vm_stat or '')
    return int(size.group(1)) * int(pages.group(1)) if size and pages else None


def host_memory():
    if sys.platform == 'darwin':
        level = capture(['sysctl', '-n', 'kern.memorystatus_vm_pressure_level'])
        return {'swap_mib': parse_swapusage(capture(['sysctl', '-n', 'vm.swapusage'])),
                'pressure': int(level) if level and level.strip().isdigit() else None,
                'compressed_bytes': parse_compressed(capture(['vm_stat']))}
    try:
        info = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines() if ':' in line)
    except OSError:
        return {}
    kib = {name: int(info[name].split()[0]) for name in ('SwapTotal', 'SwapFree') if name in info}
    if len(kib) < 2:
        return {}
    return {'swap_mib': {'total': kib['SwapTotal'] / 1024, 'free': kib['SwapFree'] / 1024,
                         'used': (kib['SwapTotal'] - kib['SwapFree']) / 1024},
            'pressure': None, 'compressed_bytes': None}


def is_holder(process):
    """Whether a process holds an index: every daemon, and a standalone server past the size of one
    that has not built one. A shim never does."""
    if process['role'] == 'daemon':
        return True
    return process['role'] == 'standalone' and (process.get('footprint_bytes') or 0) >= HOLDER_MIN_BYTES


def sample_once(status_bin=None, runtime_dir=None):
    processes = parse_ps(capture(['ps', '-A', '-ww', '-o', 'pid=,ppid=,rss=,command=']) or '')
    sizes = footprints([process['pid'] for process in processes])
    for process in processes:
        process['footprint_bytes'] = sizes.get(process['pid'])
        process['holder'] = is_holder(process)
    sample = {'t': time.time(), 'host': host_memory(), 'processes': processes}
    if status_bin:
        command = [status_bin, 'daemon-status'] + (['--runtime-dir', runtime_dir] if runtime_dir else [])
        sample['daemon_status'] = capture(command, timeout=120)
    return sample


def cmd_sample(args):
    end = time.time() + args.duration if args.duration else None
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


def load_samples(path, since=None, until=None):
    samples = []
    with open(path) as source:
        for line in source:
            if line.strip():
                sample = json.loads(line)
                if (since is None or sample['t'] >= since) and (until is None or sample['t'] <= until):
                    samples.append(sample)
    return samples


def holder_identity(process):
    """What one index is for: a daemon's key, or a standalone server's repository."""
    if process['role'] == 'daemon':
        return 'daemon ' + (process.get('key') or '?')
    return 'standalone ' + (process.get('repo') or '?')


def summarize_samples(samples):
    worst_per_key, worst_per_repo, standalone, daemons = {}, {}, {}, {}
    shim_days = set()
    peaks = {'holders': 0, 'shims': 0, 'prism_footprint_bytes': 0, 'swap_used_mib': 0.0, 'pressure': 0,
             'compressed_bytes': 0}
    for sample in samples:
        holders = {}
        for process in sample['processes']:
            if process.get('holder'):
                holders.setdefault(holder_identity(process), []).append(process['pid'])
            if process['role'] == 'standalone' and process.get('holder'):
                standalone.setdefault(process.get('repo') or '?', set()).add(process['pid'])
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
        if any(p['role'] == 'shim' for p in sample['processes']):
            shim_days.add(time.strftime('%Y-%m-%d', time.gmtime(sample['t'])))
        host = sample.get('host') or {}
        peaks['holders'] = max(peaks['holders'], sum(len(p) for p in holders.values()))
        peaks['shims'] = max(peaks['shims'], sum(1 for p in sample['processes'] if p['role'] == 'shim'))
        peaks['prism_footprint_bytes'] = max(peaks['prism_footprint_bytes'], sum(
            p.get('footprint_bytes') or 0 for p in sample['processes']))
        peaks['swap_used_mib'] = max(peaks['swap_used_mib'], (host.get('swap_mib') or {}).get('used', 0.0))
        peaks['pressure'] = max(peaks['pressure'], host.get('pressure') or 0)
        peaks['compressed_bytes'] = max(peaks['compressed_bytes'], host.get('compressed_bytes') or 0)
    return {'samples': len(samples), 'first': samples[0]['t'] if samples else None,
            'last': samples[-1]['t'] if samples else None, 'peaks': peaks,
            'max_daemons_per_key': max([len(p) for p in worst_per_key.values()] or [0]),
            'duplicated_keys': {k: v for k, v in worst_per_key.items() if len(v) > 1},
            'max_daemons_per_repo': max([len(p) for p in worst_per_repo.values()] or [0]),
            'days_with_shims': sorted(shim_days),
            'repos_with_several_daemons': {k: v for k, v in worst_per_repo.items() if len(v) > 1},
            'standalone_holders': {repo: sorted(pids) for repo, pids in standalone.items()},
            'daemon_instances': [{'pid': pid, 'key': key, 'first_seen': span[0], 'last_seen': span[1]}
                                 for (pid, key), span in sorted(daemons.items(), key=lambda item: item[1][0])]}


def scan_lines(lines):
    """Client events in shim standard-error lines."""
    events = {'fail_closed': [], 'reached': set(), 'lost': 0, 'build_crash_refusals': 0}
    for line in lines:
        if FAIL_CLOSED in line:
            events['fail_closed'].append(line.strip())
            if BUILD_CRASH in line:
                events['build_crash_refusals'] += 1
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
    """macOS crash reports of prism-mcp in `directory` whose modification time is in the window."""
    found = []
    for path in sorted(Path(directory).glob('*')) if directory and Path(directory).is_dir() else []:
        if CRASH_REPORT.match(path.name) and in_window(path.stat().st_mtime, since, until):
            found.append(path.name)
    return found


def judge(summary, events, crashes):
    """Each pinned rule's verdict: True passes."""
    return {'one_index_per_key': summary['max_daemons_per_key'] <= 1,
            'no_standalone_index': not summary['standalone_holders'],
            'no_fail_closed': not events['fail_closed'],
            'no_abort': not crashes,
            'no_critical_pressure': summary['peaks']['pressure'] < PRESSURE_CRITICAL}


def render(summary, events, crashes, verdicts):
    peaks = summary['peaks']
    lines = ['# prism-mcp soak report', '',
             'Samples: %d, from %s to %s (epoch seconds).' % (summary['samples'], summary['first'], summary['last']),
             '', '| Rule | Verdict |', '|---|---|']
    lines += ['| %s | %s |' % (rule, 'PASS' if verdicts[rule] else 'FAIL') for rule in RULES]
    lines += ['', '| Measure | Value |', '|---|---|',
              '| Most index holders at once | %d |' % peaks['holders'],
              '| Most daemons for one key at once | %d |' % summary['max_daemons_per_key'],
              '| Most daemons for one repository at once (any configuration or build) | %d |'
              % summary['max_daemons_per_repo'],
              '| Most shims at once | %d |' % peaks['shims'],
              '| Days (UTC) with a shim in a sample | %d |' % len(summary['days_with_shims']),
              '| Peak footprint of all prism-mcp processes (MiB) | %.1f |' % (peaks['prism_footprint_bytes'] / MIB),
              '| Peak swap used (MiB) | %.1f |' % peaks['swap_used_mib'],
              '| Peak compressor (MiB) | %.1f |' % (peaks['compressed_bytes'] / MIB),
              '| Highest memory pressure level | %d |' % peaks['pressure'],
              '| Daemon instances seen in samples | %d |' % len(summary['daemon_instances']),
              '| Daemon instances clients reached | %d |' % len(events['reached']),
              '| Fail-closed lines | %d |' % len(events['fail_closed']),
              '| Of which a build that died (restart bound) | %d |' % events['build_crash_refusals'],
              '| Sessions ended by their daemon | %d |' % events['lost'],
              '| prism-mcp crash reports | %d |' % len(crashes)]
    if summary['duplicated_keys']:
        lines += ['', 'Keys with more than one daemon at once: %s' % json.dumps(summary['duplicated_keys'])]
    if summary['repos_with_several_daemons']:
        lines += ['', 'Repositories with several daemons at once (other cache directories, refresh policies or '
                  'builds): %s' % json.dumps(summary['repos_with_several_daemons'])]
    if summary['standalone_holders']:
        lines += ['', 'Standalone index holders: %s' % json.dumps(summary['standalone_holders'])]
    lines += ['', 'Fail-closed lines:'] + ['- `%s`' % line for line in events['fail_closed'][:50]]
    lines += ['', 'Crash reports:'] + ['- `%s`' % name for name in crashes]
    return '\n'.join(lines) + '\n'


def cmd_report(args):
    samples = load_samples(args.samples, args.since, args.until)
    if not samples:
        raise SystemExit('no samples in %s for the window' % args.samples)
    lines = []
    if args.claude_logs:
        lines += list(claude_log_lines(args.claude_logs, args.since, args.until))
    for path in args.client_log or []:
        lines += list(client_log_lines(path, args.since, args.until))
    summary = summarize_samples(samples)
    events = scan_lines(lines)
    crashes = crash_reports(args.crash_reports, args.since, args.until)
    verdicts = judge(summary, events, crashes)
    Path(args.out).write_text(render(summary, events, crashes, verdicts))
    if args.json:
        Path(args.json).write_text(json.dumps(
            {'summary': summary, 'events': dict(events, reached=sorted(events['reached'])), 'crash_reports': crashes,
             'verdicts': verdicts}, indent=1, sort_keys=True))
    print('wrote %s: %s' % (args.out, ', '.join('%s %s' % (r, 'PASS' if verdicts[r] else 'FAIL') for r in RULES)))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    sample = commands.add_parser('sample')
    sample.add_argument('--out', required=True)
    sample.add_argument('--interval', type=float, default=60.0)
    sample.add_argument('--count', type=int)
    sample.add_argument('--duration', type=float, help='seconds; with neither this nor --count, forever')
    sample.add_argument('--status-bin', help='a prism-mcp binary whose daemon-status text each sample stores')
    sample.add_argument('--runtime-dir', help='the runtime base daemon-status is given')
    sample.set_defaults(handler=cmd_sample)
    report = commands.add_parser('report')
    report.add_argument('--samples', required=True)
    report.add_argument('--out', required=True)
    report.add_argument('--json')
    report.add_argument('--since', type=float)
    report.add_argument('--until', type=float)
    report.add_argument('--claude-logs', help='Claude Code MCP log root (~/Library/Caches/claude-cli-nodejs)')
    report.add_argument('--client-log', action='append', help='a file prism-mcp-logged.sh wrote')
    report.add_argument('--crash-reports', default=str(Path.home() / 'Library/Logs/DiagnosticReports'))
    report.set_defaults(handler=cmd_report)
    args = parser.parse_args(argv)
    args.handler(args)


if __name__ == '__main__':
    main()
````

- [ ] **Step 4: Run them and watch them pass**

```bash
python3 -m unittest discover -s scripts/soak -p 'test_soak.py' > "$LOG_DIR/t2-green.log" 2>&1; echo "rc=$?"
tail -3 "$LOG_DIR/t2-green.log"
```

Expected: `rc=0`, `Ran 16 tests`, `OK`.

- [ ] **Step 5: Commit**

```bash
git add scripts/soak/soak.py scripts/soak/test_soak.py scripts/soak/prism-mcp-logged.sh
git commit -m "feat(soak): S6 — prism-mcp process sampler, soak report and client log wrapper"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run Step 4's command, then `git checkout -- <file>`.

| Mutant | File | Edit | Expected failing tests (observed) |
|---|---|---|---|
| S1 | `soak.py` | `elif '--shared' in tokens:` → `elif '--never' in tokens:` (a shim taken for a standalone server) | `test_roles_come_from_the_command_line` |
| S2 | `soak.py` | `>= HOLDER_MIN_BYTES` → `> HOLDER_MIN_BYTES` | `test_an_index_holder_is_a_daemon_or_a_standalone_server_past_100_mib` |
| S3 | `soak.py` | `holders = {}` moved before the loop over samples (daemons of different samples counted as at once) | `test_two_daemons_of_one_key_at_once_are_found`, `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` |
| S4 | `soak.py` | `'reached': set()` → `'reached': []` (a daemon counted once per line) | `test_client_lines_are_counted`, `test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach` |
| S5 | `soak.py` | `'*/mcp-logs-*prism*/*.jsonl'` → `'*/mcp-logs-*/*.jsonl'` (every server's log read) | `test_claude_code_logs_are_read_for_prism_servers_in_the_window` |
| S6 | `soak.py` | the window check of a Claude Code entry → `if True:` | `test_claude_code_logs_are_read_for_prism_servers_in_the_window` |
| S7 | `soak.py` | `calendar.timegm(…)` → `time.mktime(…)` (UTC read as local time; killed only where the local zone is not UTC) | `test_iso_times_are_utc` |
| S8 | `soak.py` | `CRASH_REPORT` → `r'.*\.(ips\|crash)$'` (every program's crash report) | `test_crash_reports_of_prism_mcp_in_the_window_are_counted` |
| S9 | `soak.py` | `pressure < PRESSURE_CRITICAL` → `<=` | `test_the_rules_pass_on_a_clean_soak_and_each_fails_on_its_own_breach` |
| S10 | `prism-mcp-logged.sh` | `while IFS= read -r line \|\| [ -n "$line" ]; do` → `while IFS= read -r line; do` (a last line without a newline lost) | `test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes` |
| S11 | `prism-mcp-logged.sh` | the `printf '%s\n' "$line" >&2` line removed (standard error swallowed) | `test_standard_error_is_logged_with_time_and_pid_and_nothing_else_changes` |
| S12 | `soak.py` | the `if process['role'] == 'daemon':` of the per-repository count → `if True:` | `test_daemons_of_different_keys_and_one_after_another_are_not_duplicates` |
| S13 | `soak.py` | the shim check of the day count → `if True:` (every sampled day counted) | `test_only_days_with_a_shim_count_toward_the_week` |

---

### Task 3: The fan-out latency driver

**Files:** Create `scripts/soak/fanout.py`, `scripts/soak/test_fanout.py`, `scripts/soak/test_fanout_live.py`, `scripts/soak/fake_prism_mcp.py` (executable).

**Interfaces** (the soak and C2-M1's fan-out gate use them): `fanout.py run --bin BIN --corpus NAME=PATH --cache-dir DIR --mode shared|standalone --out DIR [--clients N] [--workloads DIR] [--refresh-at F | --no-refresh] [--timeout S]` writes `run.json`, `calls.jsonl`, `summary.json` (`p50_ms`, `p95_ms`, `p99_ms`, `max_ms`, `mean_ms`, `refresh_ms`, `outcomes`, `admissible`, `problems`) and `clients/<i>.stderr`; it refuses a directory that already has results and a corpus whose commit is not its workload's (memreport's `bind_workload`). `fanout.py compare --baseline DIR … --candidate DIR … --out FILE.md [--json FILE]` (`P95_MAX_RATIO`, `P99_MAX_RATIO`).

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
It writes the PRISM_MCP_CONCISE_SHAPE it was given to standard error."""
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
            answer(message['id'], {'isError': False, 'content': [{'type': 'text', 'text': '{}'}],
                                   '_meta': {'prism/refresh_status': 'unchanged'}})
        else:
            answer(message['id'], {'isError': False, 'content': [{'type': 'text', 'text': '{}'}]})


if __name__ == '__main__':
    main()
````

Make `scripts/soak/fake_prism_mcp.py` executable.

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
        'name': 'nav_callers', 'arguments': {'seed': {'kind': 'symbol', 'name': 'f%d' % n}}}}) + '\n'
        for n in range(1, requests + 1)).encode()
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


def run_info(mode='standalone', clients=1, requests=2, instances=None, failures=None):
    return {'mode': mode, 'clients': clients, 'workload': {'requests': requests, 'sha256': 'w'},
            'daemon_instances': instances if instances is not None else [], 'failures': failures or [],
            'corpus': {'commit': 'c'}}


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
    run = run_info()
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


class Runs(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.corpus, self.workloads = corpus_and_workload(self.root.name)
        self.saved_env = dict(os.environ)
        self.addCleanup(lambda: (os.environ.clear(), os.environ.update(self.saved_env)))
        self.saved_retry = fanout.WARMUP_RETRY_SECS
        fanout.WARMUP_RETRY_SECS = 0.01
        self.addCleanup(lambda: setattr(fanout, 'WARMUP_RETRY_SECS', self.saved_retry))

    def run_fanout(self, name, mode, clients=3, extra=()):
        out = Path(self.root.name) / name
        with redirect_stdout(io.StringIO()):
            fanout.main(['run', '--bin', str(FAKE), '--corpus', 'tiny=%s' % self.corpus, '--cache-dir',
                         str(Path(self.root.name) / 'cache'), '--mode', mode, '--out', str(out), '--clients',
                         str(clients), '--workloads', str(self.workloads), '--timeout', '60'] + list(extra))
        calls = [json.loads(line) for line in (out / 'calls.jsonl').read_text().splitlines()]
        return out, calls, json.loads((out / 'summary.json').read_text()), json.loads((out / 'run.json').read_text())

    def test_every_client_makes_every_call_and_client_0_refreshes_once(self):
        os.environ['FAKE_WARMING'] = '2'
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
            self.assertEqual(shared['refresh_outcome'], 'ok')
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

Expected: `ModuleNotFoundError: No module named 'fanout'` for `test_fanout` and `test_fanout_live`, `Ran 2 tests`, `FAILED (errors=2)`.

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
from soak import footprints  # noqa: E402

PROTOCOL = '2025-11-25'
# Settings a client would otherwise inherit from this shell; each run starts from the defaults.
CLEARED_ENV = ('PRISM_MCP_STRUCTURED_CONTENT', 'PRISM_MCP_CONCISE_SHAPE', 'PRISM_MCP_MAX_RESULT_CHARS',
               'PRISM_MCP_SHARED', 'PRISM_MCP_RUNTIME_DIR', 'PRISM_MCP_MEM_REPORT')
WARMUP_RETRY_SECS = 1.0
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
    return {'calls': len(timed), 'outcomes': counts, 'p50_ms': percentile(ms, 0.50), 'p95_ms': percentile(ms, 0.95),
            'p99_ms': percentile(ms, 0.99), 'max_ms': max(ms) if ms else None,
            'mean_ms': round(statistics.mean(ms), 3) if ms else None,
            'refresh_ms': refresh[0]['ms'] if refresh else None,
            'refresh_outcome': refresh[0]['outcome'] if refresh else None,
            'admissible': not problems, 'problems': problems}


def stop_daemon(binary, repo, cache_dir, runtime_base, instances, timeout=60):
    """Ask the run's daemon to drain (it has no client left) and wait until its process is gone."""
    done = subprocess.run([str(binary), 'daemon-shutdown', '--repo', str(repo), '--cache-dir', str(cache_dir),
                           '--runtime-dir', str(runtime_base), '--timeout', str(timeout)],
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout + 30)
    left = []
    for pid, _ in instances:
        give_up = time.monotonic() + timeout
        while time.monotonic() < give_up and alive(pid):
            time.sleep(0.1)
        if alive(pid):
            os.kill(pid, signal.SIGTERM)
            left.append(pid)
    return {'exit': done.returncode, 'output': done.stdout.decode('utf-8', 'replace')[-2000:],
            'terminated': left}


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


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
           'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'failures': []}
    clients = [Client(i, run['argv'], env, out / 'clients' / ('%d.stderr' % i)) for i in range(args.clients)]
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
        barrier.wait()
        instances = sorted({(int(m.group(1)), int(m.group(2))) for c in clients for m in REACHED.finditer(c.stderr())})
        run['memory_after_warmup'] = memory([c.process.pid for c in clients] + [pid for pid, _ in instances])
        clock[0] = time.monotonic()
        barrier.wait()
    except threading.BrokenBarrierError:
        pass
    for thread in threads:
        thread.join()
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


def judge(baseline, candidate):
    """The pinned rule's verdict and the figures it rests on."""
    problems = ['%s is not admissible: %s' % (entry['dir'], '; '.join(entry['summary']['problems']))
                for entry in baseline + candidate if not entry['summary']['admissible']]
    keys = {(e['run']['corpus'].get('commit'), e['run']['workload']['sha256']) for e in baseline + candidate}
    if len(keys) != 1:
        problems.append('the runs are not of one corpus commit and workload: %s' % sorted(keys))
    figures = {}
    for name, arm in (('baseline', baseline), ('candidate', candidate)):
        figures[name] = {stat: statistics.median([e['summary'][stat] for e in arm])
                         for stat in ('p50_ms', 'p95_ms', 'p99_ms')}
    p95 = figures['candidate']['p95_ms'] / figures['baseline']['p95_ms']
    p99 = figures['candidate']['p99_ms'] / figures['baseline']['p99_ms']
    passed = not problems and p95 <= P95_MAX_RATIO and p99 <= P99_MAX_RATIO
    return {'pass': passed, 'problems': problems, 'figures': figures, 'p95_ratio': round(p95, 4),
            'p99_ratio': round(p99, 4), 'rule': {'p95_max_ratio': P95_MAX_RATIO, 'p99_max_ratio': P99_MAX_RATIO}}


def render(baseline, candidate, verdict):
    lines = ['# prism-mcp fan-out latency', '', '| Arm | Run | Mode | Clients | p50 ms | p95 ms | p99 ms | max ms | '
             'refresh ms | admissible |', '|---|---|---|---|---|---|---|---|---|---|']
    for name, arm in (('baseline', baseline), ('candidate', candidate)):
        for entry in arm:
            s, r = entry['summary'], entry['run']
            lines.append('| %s | %s | %s | %d | %s | %s | %s | %s | %s | %s |' % (
                name, entry['dir'], r['mode'], r['clients'], s['p50_ms'], s['p95_ms'], s['p99_ms'], s['max_ms'],
                s['refresh_ms'], 'yes' if s['admissible'] else 'no'))
    lines += ['', 'Medians across runs: baseline %s; candidate %s.' % (verdict['figures']['baseline'],
                                                                      verdict['figures']['candidate']),
              'Ratios: p95 %.4f (at most %.2f), p99 %.4f (at most %.2f).' % (
                  verdict['p95_ratio'], P95_MAX_RATIO, verdict['p99_ratio'], P99_MAX_RATIO),
              '', '**Verdict: %s**' % ('PASS' if verdict['pass'] else 'FAIL')]
    lines += ['- %s' % problem for problem in verdict['problems']]
    return '\n'.join(lines) + '\n'


def cmd_compare(args):
    baseline, candidate = load_arm(args.baseline), load_arm(args.candidate)
    verdict = judge(baseline, candidate)
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
    run.set_defaults(handler=cmd_run)
    compare = commands.add_parser('compare')
    compare.add_argument('--baseline', action='append', required=True)
    compare.add_argument('--candidate', action='append', required=True)
    compare.add_argument('--out', required=True)
    compare.add_argument('--json')
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

Expected: `rc=0`, `Ran 33 tests`, `OK (skipped=1)` (the live test skipped); `build rc=0`; with the binary `Ran 33 tests`, `OK` (the live test ran: three shims of one daemon and three standalone servers on a three-file repository, every call `ok`, the daemon drained by `daemon-shutdown` and gone); `pgrep` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add scripts/soak/fanout.py scripts/soak/test_fanout.py scripts/soak/test_fanout_live.py scripts/soak/fake_prism_mcp.py
git commit -m "feat(soak): S6 — fan-out latency driver, shared against standalone"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone to `scripts/soak/fanout.py`, run Step 4's first command, then `git checkout -- scripts/soak/fanout.py`.

| Mutant | Edit | Expected failing tests (observed) |
|---|---|---|
| F1 | `percentile`: `ordered[max(1, math.ceil(fraction * len(ordered))) - 1]` → `ordered[min(len(ordered) - 1, int(fraction * len(ordered)))]` | `test_percentile_is_nearest_rank` |
| F2 | `outcome`: the `prism/retryable` check → `if False:` | `test_outcome_tells_the_four_answers_apart` |
| F3 | `rotated`: `start = (client * len(items)) // clients` → `start = 0` (clients in lockstep) | `test_each_client_starts_its_own_share_of_the_way_through` |
| F4 | `summarize`: the call-count check → `if False:` | `test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible` |
| F5 | `summarize`: the one-daemon check → `if False:` | `test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible` |
| F6 | `judge`: `<=` → `<` on both ratios | `test_the_rule_passes_at_its_bounds_and_fails_past_them` |
| F7 | `judge`: `statistics.median` → `statistics.mean` | `test_medians_across_runs_are_compared` |
| F8 | `drive`: `refresh_at is not None` → `refresh_at is None` (no refresh) | `test_every_client_makes_every_call_and_client_0_refreshes_once` |
| F9 | `drive`: the warm-up loop breaks on the first answer, retryable or not | `test_every_client_makes_every_call_and_client_0_refreshes_once` |
| F10 | `cmd_run`: `barrier.abort()` removed from a failing client (the main thread waits for ever) | `test_clients_that_die_while_warming_up_end_the_run` (its 60 s bound) |
| F11 | `cmd_run`: the `stop_daemon` call removed | `test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down` |
| F12 | `cmd_run`: `CLEARED_ENV` not applied (`env = dict(os.environ)`) | `test_clients_start_from_the_default_output_settings` |

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
library, not run by CI): `fanout.py` (N clients of one corpus in shared or standalone mode, per-call
latency, `compare` under the pinned rule; also the spec §6.2 fan-out gate), `soak.py` (`sample`
appends the prism-mcp processes and the host's swap, pressure and compressor; `report` judges the
soak's rules from the samples, the clients' standard error and crash reports) and
`prism-mcp-logged.sh` (logs a client's prism-mcp standard error with time and pid). Run their tests
with `python3 -m unittest discover -s scripts/soak -p 'test_*.py'`; `PRISM_SOAK_BIN=<prism-mcp>` adds
a live run against the binary.

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
  arguments wins over the variable.
- **Manual Claude Code, Codex, Kiro:** add `"--shared"` to the server's arguments, for example in Codex:

  ```toml
  [mcp_servers.prism]
  command = "/abs/path/to/prism/target/release/prism-mcp"
  args = ["--repo", "/abs/path/to/your/repo", "--shared"]
  env = { PRISM_MCP_RUNTIME_DIR = "/tmp" }
  ```
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

Expected: `fmt rc=0` and nothing printed; `check rc=0`, `check mcp rc=0`, no `warning`; `clippy rc=0` and `0` lines naming `launcher_test`; `full suite rc=0`, `31` result lines, `6232 passed, 0 failed, 1 ignored` (V4); `doc tests rc=0`; `soak tests rc=0` (`Ran 33 tests`, `OK`); `memreport tests rc=0` (`Ran 349 tests`); the repeats `10 test result: ok. 6 passed` and `10 OK`; `pgrep` prints nothing.

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

Expected: `failed Linux runs: 0`, no `left:` line in `s6-linux.out`; `10 test result: ok. 6 passed; 0 failed` (52 filtered out); `10 Ran 33 tests` and `10 OK` (the live test runs: the image has no `ps` or `pgrep`, which the soak tools do not need for it).

- [ ] **Step 5: Commit and open the pull request**

```bash
git add docs/MCP.md skills/prism-code-navigation/SKILL.md CLAUDE.md .codex/config.toml
git commit -m "docs(mcp): S6 — shared-daemon docs, the project's Codex opt-in, soak tools in CLAUDE.md"
```

The PR body states, in this order: (1) **Goal:** the lane goal; S6 is the opt-in from every launcher, the docs the earlier slices left to it, and the soak's instruments; the soak is specified in the plan and is the controller's. (2) **Invariants:** nothing under `src/` or `.claude-plugin/` changes; opt-in stays opt-in (Global Constraints). (3) **Decisions:** D110–D123; SI-36–SI-42; the owner's answers to the questions (or "pending" for each that is). D110 (an environment variable for the plugin) and D118 (what counts as an index holder) are the ones a reviewer is most likely to want to change. (4) **Claims:** the table, with what no test holds. (5) **Totals:** Step 3 next to the base, Step 4, and the mutants as observed. (6) **Not verified:** x86_64 Linux before CI; CI does not run the Python tests (D117); the soak (the controller's, after merge); a Codex session's environment (SI-42). (7) **Tier-A:** not run (no file it watches is touched). (8) **Reviews of the plan:** what each round folded.

Review brief and cap as in the lane README (two rounds; Opus 5.5 and gpt-6.1-sol; the goal above; WRONG/SMELL and MATERIAL/IMMATERIAL; a fix and alternatives per finding and when it would not apply). Ask each reviewer to check the docs' fail-closed table and the operator's exit statuses line by line against the code (Review Focus 9).

---

## The soak (the lane's acceptance; the controller's, after this slice merges)

The soak is the gate for the default flip (spec §8 step 2, §11 item 2, §12 row B): "over one bridge review round plus one week of interactive use, record resident `prism-mcp` count and aggregate footprint …, shim fail-closed count (target 0), daemon restarts, and fan-out latency p95 vs. standalone. The flip does not wait for any memory reduction." It is a measurement, not code. It starts only after the owner has answered questions 2, 3, 5, 7, 8 and 9; the values below are the recommended answers.

### What it shows, and what would fail it

| Spec §8 step 2 asks for | Instrument | Phase | Pass (owner question 7, recommended) | What would fail it |
|---|---|---|---|---|
| Resident `prism-mcp` count | `soak.py sample` (index holders per key, per repository, in total; shims) | 2, 3 | `one_index_per_key` at every sample; in Phase 2 also `no_standalone_index` | Two daemons of one key at once (a split runtime directory); a launcher still standalone in the round |
| Aggregate footprint | the same samples (`phys_footprint` of every prism-mcp process; the host's swap, pressure level, compressor) | 2, 3 | Phase 2: `no_critical_pressure`; peaks reported against the incident's (29 processes, 34 GiB compressed) | Memory pressure at its critical level during the round |
| Shim fail-closed count, target 0 | `soak.py report` over Claude Code's MCP logs and `prism-mcp-logged.sh` logs | 2, 3 | `no_fail_closed` | Any `prism-mcp: --shared failed closed` line in the window |
| Daemon restarts | daemon instances in the samples and in the shims' `shared daemon pid` lines; crash reports; restart-bound refusals | 2, 3 | `no_abort`; no `died during its index build` refusal; starts reported (SI-40) | A `prism-mcp-*.ips` in the window; a restart-bound refusal |
| Fan-out latency p95 vs. standalone | `fanout.py run` and `compare` | 1 | excalidraw, 8 shims against 8 standalone, 3 interleaved pairs, all admissible, median p95 ≤ 1.10×, median p99 ≤ 1.25× | A ratio over its bound; an inadmissible run (a retryable answer after warm-up, a lost client, two daemons) |

The default flip (row B) is the owner's decision with the three phase reports in hand; "the flip does not wait for any memory reduction" (§8 step 2).

### Fixed inputs

- **Build:** one release `prism-mcp` of this slice's merge commit, built in a worktree of its own that stays for the whole soak (`$S` below), copied to `$E/bin/prism-mcp`, its sha256 recorded. The tools are run from `$S/scripts/soak/` (they find memreport's workloads beside them).
- **Corpora** (lane README "Facts"; clean, at these commits; checked before each phase with `git -C <corpus> rev-parse HEAD` and `git status --porcelain`): excalidraw `~/code/bench-repos/excalidraw` at `0642e72cfa2d`; slicing `~/code/prism-c1-corpus` at `7db8f1951166`. Workloads: `scripts/memreport/workloads/{excalidraw,slicing}.jsonl`, 200 requests each, bound to those commits by `MANIFEST.json`. Prometheus and ruff are not used (8 standalone servers of either do not fit this host; their shared runs add nothing the gate needs).
- **Host:** this Mac (24 GB, macOS 26). Every run records swap, compressor, pressure and load before and after.
- **Runtime directory:** `/tmp` as the base (owner question 2), so `/tmp/prism-mcpd`; the fan-out runs use directories of their own under `/tmp`.
- **Evidence:** `E=~/prism-evidence/mcp-memory/s6-soak`, one directory per phase; `SHA256SUMS` written at each phase's end; never a second run into a directory that has results (the tools refuse).

### Phase 0: deployment (outside the repository; owner question 5)

```bash
E=~/prism-evidence/mcp-memory/s6-soak; B="$E/bin/prism-mcp"; mkdir -p "$E"/{bin,deploy,fanout,round,week,client-logs}
S=~/code/prism-s6-soak; git -C ~/code/slicing worktree add --detach "$S" <the S6 merge commit>
```

In a command of its own: `df -h /System/Volumes/Data` (at least 50 GiB). Then:

```bash
cd "$S" && nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --release --features mcp --bin prism-mcp > "$E/deploy/build.log" 2>&1; echo "build rc=$?"
cp "$S/target/release/prism-mcp" "$E/bin/prism-mcp"
cp "$S/scripts/soak/prism-mcp-logged.sh" "$E/bin/"
shasum -a 256 "$E/bin/prism-mcp" "$E/bin/prism-mcp-logged.sh" > "$E/deploy/SHA256SUMS"
for d in ~/code/slicing/target/release ~/.claude/plugins/cache/prism-dev/prism/3.1.2/target/release; do
  mv "$d/prism-mcp" "$d/prism-mcp.pre-s6" && cp "$E/bin/prism-mcp" "$d/prism-mcp"
done
P=~/.claude/plugins/cache/prism-dev/prism/3.1.2/scripts
mv "$P/prism-mcp-launch.sh" "$P/prism-mcp-launch.sh.pre-s6" && cp "$S/scripts/prism-mcp-launch.sh" "$P/"
```

Then, by the owner or with the owner's approval: `export PRISM_MCP_SHARED=1` and `export PRISM_MCP_RUNTIME_DIR=/tmp` in the shell profile Claude Code starts from (sessions started before keep their standalone servers until restarted); and in each bridge configuration the soak uses, every host-run `[[agents.mcp]]` prism entry becomes (owner question 3 drops `--cache-dir`):

```toml
[[agents.mcp]]
name = "prism"
command = "/Users/wesleyjinks/prism-evidence/mcp-memory/s6-soak/bin/prism-mcp-logged.sh"
args = ["/Users/wesleyjinks/prism-evidence/mcp-memory/s6-soak/client-logs/bridge.log", "/Users/wesleyjinks/code/slicing/target/release/prism-mcp", "--repo", "{cwd}", "--shared"]
[[agents.mcp.env]]
name = "PRISM_MCP_RUNTIME_DIR"
value = "/tmp"
```

Record in `$E/deploy/NOTES.md`: the merge commit, the sha256s, which configurations were edited, the time. **Rollback** (any time): unset the two variables; `mv` each `.pre-s6` back; restore the bridge configurations; `prism-mcp daemon-shutdown` (or `kill -TERM <pid>`) for each daemon `prism-mcp daemon-status --runtime-dir /tmp` lists.

### Phase 1: fan-out latency (a quiet hour; about 30 minutes)

Preconditions, checked and saved: no bridge run, build or other measurement on the host (lane rule: one measured process at a time); `python3 "$S/scripts/soak/soak.py" sample --count 1 --interval 0 --out "$E/fanout/host-before.jsonl"` shows no index holder; both corpora at their commits and clean. Prime one cache per corpus (discarded runs):

```bash
B="$E/bin/prism-mcp"; C="$E/fanout/cache"
"$B" --repo ~/code/bench-repos/excalidraw --cache-dir "$C/excalidraw" --eager < /dev/null > "$E/fanout/prime-excalidraw.log" 2>&1; echo "rc=$?"
"$B" --repo ~/code/prism-c1-corpus --cache-dir "$C/slicing" --eager < /dev/null > "$E/fanout/prime-slicing.log" 2>&1; echo "rc=$?"
```

The gate (excalidraw), each run alone, in this order (interleaved so that drift in the host hits both arms):

```bash
for rep in 1 2 3; do
  for mode in standalone shared; do
    python3 "$S/scripts/soak/fanout.py" run --bin "$B" --corpus excalidraw="$HOME/code/bench-repos/excalidraw" --cache-dir "$C/excalidraw" --mode "$mode" --clients 8 --out "$E/fanout/excalidraw-$mode-$rep"
  done
done
python3 "$S/scripts/soak/fanout.py" compare --baseline "$E/fanout/excalidraw-standalone-1" --baseline "$E/fanout/excalidraw-standalone-2" --baseline "$E/fanout/excalidraw-standalone-3" --candidate "$E/fanout/excalidraw-shared-1" --candidate "$E/fanout/excalidraw-shared-2" --candidate "$E/fanout/excalidraw-shared-3" --out "$E/fanout/excalidraw-compare.md" --json "$E/fanout/excalidraw-compare.json"
```

Information (slicing; owner question 9): the same loop with `--corpus slicing="$HOME/code/prism-c1-corpus" --cache-dir "$C/slicing"`, the shared arm with `--clients 8` and the standalone arm with `--clients 1`, out directories `slicing-shared-$rep` and `slicing-standalone1-$rep`, and a `compare` written to `slicing-compare.md`, whose verdict is reported and not gated.

Read and keep for the readout: each run's `summary.json` (p50/p95/p99/max, the refresh's time), each `run.json`'s `memory_after_workload.total_bytes` (eight standalone footprints against one daemon and eight shims: the memory side of the same experiment) and `daemon_instances` (exactly one per shared run), and both `compare` files. **Pass:** `excalidraw-compare.md` says `**Verdict: PASS**`.

### Phase 2: one bridge review round (fan-out under real load)

The round: the next real review round of any lane run through the bridge with a soaked configuration, or else one `a2a-bridge run-workflow plan-review` (five nodes, codex and claude) on a checkout of `main` at or after the merge, `--session-cwd` that checkout. Owner sessions may go on as usual; they are part of the load.

```bash
python3 "$S/scripts/soak/soak.py" sample --out "$E/round/samples.jsonl" --interval 5 --status-bin "$B" --runtime-dir /tmp > "$E/round/sampler.log" 2>&1 &
T0=$(date +%s)
# … the round …
# then wait 20 minutes after its last agent ends (the idle TTL, 15 minutes, and a margin) …
T1=$(date +%s); kill %1
python3 "$S/scripts/soak/soak.py" report --samples "$E/round/samples.jsonl" --since "$T0" --until "$T1" --claude-logs ~/Library/Caches/claude-cli-nodejs --client-log "$E/client-logs/bridge.log" --out "$E/round/report.md" --json "$E/round/report.json"
```

**Pass:** all five rules `PASS`, no restart-bound refusal (`events.build_crash_refusals` 0 in `report.json`). Reported with it: the most index holders at once against the number of checkouts the round used, the peak footprint of all prism-mcp processes, the peak swap and compressor, and whether the round's daemon had retired by `T1` (the last sample).

### Phase 3: one week of interactive use

```bash
nohup python3 "$S/scripts/soak/soak.py" sample --out "$E/week/samples.jsonl" --interval 60 --duration 604800 --status-bin "$B" --runtime-dir /tmp > "$E/week/sampler.log" 2>&1 &
echo "$(date +%s)" > "$E/week/T0"
```

Check daily that the last line of `samples.jsonl` is recent; a restart of the host or of the session kills the sampler, and running the same command again appends (owner's lesson: restarts kill workers). Write in `$E/week/NOTES.md` anything that changes the deployed binaries (a rebuild of `~/code/slicing/target/release/prism-mcp` by other work gives new keys; the old daemons retire on their own). At the end:

```bash
python3 "$S/scripts/soak/soak.py" report --samples "$E/week/samples.jsonl" --since "$(cat "$E/week/T0")" --until "$(date +%s)" --claude-logs ~/Library/Caches/claude-cli-nodejs --client-log "$E/client-logs/bridge.log" --out "$E/week/report.md" --json "$E/week/report.json"
```

(add a `--client-log` for each other wrapper log the week used). **Pass:** `one_index_per_key`, `no_fail_closed` and `no_abort` `PASS`, no restart-bound refusal, and at least 5 days with a shim (`summary.days_with_shims` in `report.json`; SI-41); `no_standalone_index` and `no_critical_pressure` are reported, not gated (other work on the host runs standalone servers and uses memory).

### Stop conditions (any phase)

- **An abort or a restart-bound refusal:** stop the phase; keep the crash report (`~/Library/Logs/DiagnosticReports/prism-mcp-*.ips`) and the daemon's log (`/tmp/prism-mcpd/<key16>.log`); escalate to the owner.
- **A fail-closed line:** read its reason. A configuration cause (a runtime directory, a cache directory, a stale binary in a configuration) is fixed and the phase starts again from zero, disclosed; any other cause is escalated.
- **Memory pressure at its critical level during Phase 1 or 2:** stop the run (the clients, or the bridge), keep the samples, escalate.
- **A gate shown unreachable by its own measurements** (for example a latency ratio that cannot pass on this host however quiet): escalate to the owner at once; never collect more runs toward it.
- **Two measured processes at once:** the overlapping run is inadmissible; re-run it alone.

### The readout

`docs/analysis/2026-10-shared-daemon-soak-readout.md` (a docs pull request after the week): the three reports' verdicts and figures next to the incident's, the fan-out tables, the deployment notes, what was not measured, and the owner's flip question. Evidence stays under `$E` with its `SHA256SUMS`, and is offloaded to `prism-evidence` when the lane closes.

## Interfaces for the next slices

**B (the default flip; owner decision after the soak) consumes:** the soak's three reports. The flip itself: `--shared` becomes the default and `--standalone` opts out (spec §8 step 3); the launcher's `PRISM_MCP_SHARED` then means "unset = shared" or is retired, and `docs/MCP.md`, `.codex/config.toml` and the bridge's entries drop the flag. The plugin's version is bumped then (owner question 6), with a note that an updated plugin needs its binary built or copied again (the plugin cache of a new version has no `target/release/prism-mcp`).

**C2-M1 (and every candidate that ships through the §6.2 fan-out gate) consumes:** `fanout.py run --mode shared --clients 8` on the baseline build and on the candidate build, `compare --baseline … --candidate …` under the same pinned rule (its constants may be re-pinned by the candidate's plan), with SI-11 in mind.

**The bridge (outside this repository) can consume:** `prism-mcp daemon-shutdown --repo X --pid N --start-time T` from the shim's `shared daemon pid N (started T, …)` line as its reaper (S5b), if the soak shows that residency after a round matters (owner question 10).

## Self-review record

- **Spec coverage (§12 row S6, §8 steps 1–2).** "`.codex/config.toml`" → Task 4 (D115). "plugin launcher" → Task 1 (D110–D113, SI-36). "bridge entry, `PRISM_MCP_RUNTIME_DIR`" → `docs/MCP.md` and Phase 0 (SI-37). "docs" → Task 4 (D116). "soak instrumentation" → Tasks 2 and 3; §8 step 2's five measures → "The soak", one row each (SI-38–SI-41). §4.3 "`docs/MCP.md` says so" → the docs' runtime-directory section (SI-42). §6.2's fan-out gate → `fanout.py`, reusable by C2-M1 ("Interfaces for the next slices").
- **What S3a left to S6** (its "S6 (docs) owes"): the readiness vocabulary — `warming`, `build_failed`, the daemon's `indexing` with `retry_after_secs`, the `prism/index_state` values — in `docs/MCP.md` ("What changes for a client") and the skill's new bullet (the `S3a-2` review's note that neither documented `warming` or `build_failed`).
- **What S3b left:** what a client sees when a shared daemon panics (its session ends; the next start makes a new daemon) and the macOS crash report for each abort → "A crash ends every attached session".
- **What S4 left:** the daemon's command line and its exit statuses, what a client sees for each drain reason, the runtime directory the same for every launcher → "The daemon" paragraph and the runtime-directory section.
- **What S5a left, one by one:** `--shared` in `.codex/config.toml` (Task 4), the plugin launcher (Task 1), the bridge's `[[agents.mcp]]` with `PRISM_MCP_RUNTIME_DIR` (docs, Phase 0); the fail-closed line and what to do about each reason (the docs' table, each reason in the code's words); `--idle-ttl`/`--max-clients` configure only a daemon the shim starts, silently ignored otherwise (D85; the flags table); the restart bound and its minute, and the wall clock (D77, owner decision 12, option A; "A crash ends …"); a rebuilt binary's old shims cannot start a daemon (SI-27; the table's spawn row); the `sudo -E` root-owned directory (the lane README's S5a note; the runtime-directory traps). The soak reads, as S5a specified: fail-closed lines starting `prism-mcp: --shared failed closed` (D87), daemon starts from `prism-mcp: shared daemon pid … (started …` (D88), `RecentBuildCrash` (D77), `pgrep`-style counts (here the sampler's process table) and the 10-03 probes (SI-38), and the fan-out with eight shims and one concurrent refresh (S3b's SI-11 noted under "What each mechanism covers").
- **What S5b left, one by one:** the bridge's reaper interface (`daemon-shutdown --pid --start-time` from the shim's line) is documented, and building the reaper is owner question 10; the soak's resident count through `daemon-status` (`sample --status-bin`); pre-S5b daemons stopped with `kill -TERM` and their idle restart at each probe (SI-35) in the docs; the two commands, their exit statuses, `--runtime-dir` as the base, other builds listed and stopped with `kill -TERM`, exit 1's meanings, "never disconnects a client and keeps admitting until it drains" (owner decision 15), "a probe does not keep a daemon of this build up" — all in "Operating daemons". The sampler connects to no daemon by default (D121), so it cannot keep a pre-S5b daemon alive.
- **The lane README's notes.** "S6: the launcher … passes extra arguments through, so `--shared` is a config change, not a script change": true for every launcher but the plugin's, whose arguments are not user configuration (SI-36). S2a "the docs should say" (the root-owned directory): done. S2a `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`: the daemon paragraph says the shim does not pass it (S5a's D84). Owner decision 14 (keys left by rebuilds, counted, nothing removed): the "Files" paragraph. The HANDOFF's open items not S6's: EMFILE back-off (item 15), the call-edge sidecar's temp files (item 11), SI-10 and SI-11 (measured by Phase 1's refresh).
- **The lane's review lessons.** (1) Every stated property has a test and a mutant, or is listed as not held: "Claims". (2) What each mechanism does not cover: its section. (3) Interleavings are played, not hoped for: a client that dies before the barrier (the barrier is aborted), a client that dies after it, warming answers before the barrier. (4) No test asserts an upper bound on a duration; the one subprocess bound is a hang limit. (5) No test writes a file it then executes. (6) Measurement first: the claim that Claude Code passes its environment and that its logs keep the server's standard error was measured on this host before the design relied on it (SI-42, D120); the claim about Codex's environment is marked inherited.
- **Proportionality.** Every piece serves the soak, which is the gate for the default flip, which is the lane's goal: the launcher (an opt-in for the clients that hold most sessions on this host), the docs (users of the opt-in, and the earlier slices' debts), the sampler (the resident count and footprint the incident was measured by), the fan-out driver (the latency half of the goal, and C2-M1's gate). Not built: a product-side event log (D120), a pruning flag (owner decision 14), the bridge's reaper (owner question 10), a CI job for the Python tests (D117).

## Validation record

Built on 2026-10-16 on macOS (arm64), in `~/code/prism-s6-validate`, from `main` `61c2a9e5`. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file under the session's scratch directory, read from the file. Nothing was pushed. Local branches:

- `scratch/s6-validate`, where the code was first written: `5838de63` (the launcher and its tests), `8b3dc0fb` (the soak tools), `f81cf81a` (docs and configuration), `c9be7b01` (two fan-out tests the mutant table asked for), `f97a04ed` (the live test without `pgrep`; daemons per repository), `9e82b670` (days with a shim; the head). The full suite ran at `c9be7b01` (V4); the mutants at `f97a04ed` (L1–L5) and `9e82b670` (S1–S13, F1–F12), each alone, the file restored after each.
- `scratch/s6-tasks`, the same source cut into this plan's tasks, one commit per task, every file taken from `9e82b670`: `56454f19` (Task 1), `628dcdae` (2), `71ff99d5` (3), `bf7d3c23` (4). `cargo fmt --check` passed at every commit; its tree is `c6aeb376`, the head's. An earlier cut at `f97a04ed` is kept as `scratch/s6-tasks-r0`, and its replay as `scratch/s6-replay-r0`.
- `scratch/s6-replay`: this plan's directives replayed by a script onto `61c2a9e5`, every anchor matching exactly once (20 directives, three of them "Make … executable."): `525b7659`, tree `c6aeb376`, the tree of `bf7d3c23` and of `9e82b670`. The red and green steps of Tasks 1–3 were run on partial replays in the validate worktree (detached at `61c2a9e5`, the plan replayed up to the task's Step 2, then up to its Step 4; V2, V3).

The tools (the generator of the directives from the per-task commits, the replay, the cut, the mutant runner, the assembly of this file) are S5b's, with three changes: a created file whose mode is `100755` gets a "Make `path` executable." directive, which the replay applies with `chmod 755` (the tree comparison includes modes); Python and bash fences; and the test files of this slice (`tests/…`, `scripts/soak/test_*.py`, `fake_prism_mcp.py`) are told from production files by name.

**Measured on this host before the design relied on it.** (1) Claude Code gives a plugin's MCP server its whole environment: `ps eww` of a running plugin `prism-mcp` (Claude Code session `fd583811`) lists the user's shell variables, `TMPDIR`, `GOROOT`, API keys among them (names only were read). (2) Claude Code keeps an MCP server's standard error: `~/Library/Caches/claude-cli-nodejs/-Users-wesleyjinks-code-slicing/mcp-logs-plugin-prism-prism/*.jsonl` holds entries `{"error":"Server stderr: nav cache: CPG cache miss …","timestamp":…}`. (3) A lazy plugin server that has built nothing has a `phys_footprint` of 1.8 MiB (`footprint -p 8068`). (4) macOS names a crash report after the process: the lane's aborted test binaries left `prism-49efe5ab01e24782-2026-10-14-*.ips`. (5) The image `a2a-toolchain:latest` has Python 3.12, git and bash 5.2, and no `ps`, `pgrep` or `footprint`. (6) Both pinned corpora are at their commits and clean.

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
