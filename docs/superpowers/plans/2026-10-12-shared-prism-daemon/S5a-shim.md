# S5a — The Shim: Connect, Arbitration, Spawn, Fail-Closed, Pump Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-14).** Every task of this plan was built with a compiler before the plan was written, on the local scratch branch `scratch/s5a-validate` (worktree `~/code/prism-s5a-validate`) from `main` `296102a2`, test first, and every mutant named in a task was applied there and killed (see the Validation record). The source was then cut into this plan's tasks on `scratch/s5a-tasks` (`8d5ce396`), the plan's directives were generated from that branch's per-task commits, and the plan was replayed by a script onto `296102a2` (`scratch/s5a-replay`, `bf4a4e18`), each anchor matching once, to a tree byte-identical with `scratch/s5a-tasks`; each task's red and green steps below were run on partial replays. The shim's and the daemon's tests were run on Linux (aarch64, the bridge's `a2a-toolchain` image, as root): 10 × `--lib mcp::shim`, 10 × `--lib mcp::daemon` and 10 × `--test mcp`, all green. A first attempt failed for want of disk (the host's, shared with other agents, ran out and the Docker VM stopped itself for a while); it was re-run once there was room. One test outside this slice fails in that container on the base too (V11). Counts, red steps and mutant results below are the observed ones. The mutation gate was not run (it selects no MCP code). Not yet reviewed.

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file that was then read.

| # | Command | Must show | Observed |
|---|---|---|---|
| V1 | `cargo test --features mcp --lib mcp::` and `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` on the untouched base | The base totals; every later total is "base + N". | `517 passed`; `31 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`, at `296102a2`. |
| V2 | Each task's red step, on a partial replay of this plan | What the step says. | Task 1: 14 errors (`E0425` for `marked` ×3, `E0599` for `build_marker` ×9, `E0282` ×2). Task 2: `E0583` for `shim`. Task 3: `E0583` for `arbitrate` and `key_files`. Task 5: `E0583` for `diagnostic`, `pump`, `spawn`. Task 6: `test result: FAILED. 31 passed; 11 failed`, each new test with `error: unexpected argument '--shared' found`. |
| V3 | Each task's green step, on the replay | The count the step states, no warning. | Task 1: 522. Task 2: 530. Task 3: 547. Task 5: 556. Task 6: `--test mcp` 42. No `warning` line in any of them. |
| V4 | `cargo test --features mcp --lib mcp::` at each head | base + 30, then base + 39; `0 failed`. | 547 at the S5a-1 head (Task 3's green, the head differs only in `CLAUDE.md`); 556 at the S5a-2 head's code (`7bff1bea`), `0 failed`. |
| V5 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `31 passed`, then `42 passed`, the goldens unblessed. | 42 at the S5a-2 head's code, the goldens' test `ok` in each of 10 runs; zero-line diff in `transport_tests.rs`, `owner_transport_tests.rs` and `tests/fixtures/mcp/golden/`. |
| V6 | `cargo check`, `cargo check --features mcp`, `cargo check --tests --features mcp,detached-owner-audit`, `cargo fmt --check` | `Finished`; no warning from the feature build; fmt silent. | All `Finished`, 0 warnings from either check (the module's dead-code allowance removed at Task 6); fmt silent; at the S5a-2 head. |
| V7 | `cargo clippy --all-targets --features mcp`, filtered to the touched files | No line but the base's. | 257 `warning` lines at the base `296102a2` and 257 at the S5a-2 head; the only one naming a touched file is `src/navigation/call_edge_cache.rs:279:9` (`io::Error::other`), present at the base, on a line this slice does not touch. |
| V8 | Every "prove the tests bite" step | The named tests fail under the named mutant. | 1A–1C, 2A–2H, 3A–3G, 3I, 3J, 5A, 5B, 5D–5G, 6A–6D killed (30 mutants). 5E survived the test as first written and the test was strengthened (Validation record). |
| V9 | Repeats on macOS at the S5a-2 head | All `ok`; no daemon process left. | At `7bff1bea`: 20 × `--lib mcp::shim` `20 test result: ok. 34 passed` (1.01–1.03 s each); 10 × with `--test-threads=1` `10 … ok. 34 passed`; 10 × `--lib mcp::daemon` `10 … ok. 133 passed`; 10 × `--test mcp` `10 … ok. 42 passed` (3.4–7.0 s), the goldens' test `ok` in all 10; no `prism-mcp` process of this worktree's binary left afterwards. One run of the whole `--lib mcp::` took 57.6 s (556 passed); which test was slow was not determined, and no assertion bounds a duration. |
| V10 | The zero-diff and replay checks | Nothing in the standalone files; the replay byte-identical. | `git diff --name-only 296102a2 8d5ce396`: 30 paths (29 under `src/` and `tests/`, three of them changed by both pull requests, and `CLAUDE.md`), none of `transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `tests/fixtures/mcp/golden/`. The replay `bf4a4e18` has tree `38253d57`, the tree of `scratch/s5a-tasks` `8d5ce396`. |
| V11 | Linux: the bridge's `a2a-toolchain` image (Debian, aarch64, rustc 1.94.0, uid 0), the worktree read-only, named target and registry volumes removed afterwards | All `ok`. | At `7bff1bea`: `10 test result: ok. 34 passed` (`--lib mcp::shim`), `10 test result: ok. 133 passed` (`--lib mcp::daemon`), `10 test result: ok. 42 passed` (`--test mcp`, the goldens' test `ok` each time). The whole `--lib mcp::` there: `547 passed; 1 failed`, the failure `mcp::shared::dir_chain::tests::another_users_private_directory_is_refused_for_what_it_holds` (`dir_chain_tests.rs:381`, `left: None`), an S2a test that assumes it does not run as root. Control in the same container on the base `296102a2`: the same test fails the same way (`508 passed; 1 failed`), so it is not this slice's; reported, not fixed. The first attempt at this row failed while downloading crates with `No space left on device` (the host's disk, shared with other agents, had 434 MiB free) and the Docker VM (OrbStack) stopped itself on a failed write to its disk image; the row was run once the host had room again. x86_64 Linux: CI only. |
| V12 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp` and `cargo test --doc --features mcp` at the S5a-2 head | `0 failed`. | At `7bff1bea` on macOS: 31 `test result` lines, **5,950 passed, 0 failed, 1 ignored**; doc tests `2 passed`. |

## Owner questions

**None that block this plan.** Two points depart from, or add to, the approved spec's text; each is built with the default below and is one small change to reverse:

1. **The restart bound (SI-22, D77).** S3b's SI-10 made a panic on the build thread fatal, as the spec says, and handed S5a a bound on the restart loop that follows: a build that panics every time (a grammar panic on one file) or is killed every time (an out-of-memory kill during the build, the likelier case on this host) would otherwise cost one daemon start, one build and one lost session per new shim. **Default built:** the daemon leaves a build marker (`<key16>.build`) while its builder runs and removes it when the builder returns, either way, and when it drains; a shim that finds the key free and a marker whose build began less than 60 s ago fails closed with a line naming the dead daemon's pid and the log, and leaves its files. After 60 s the next shim restarts the daemon. This departs from spec §4.6 ("Daemon crashes … next shim … restarts") for that one case and adds a sixth file to §4.3's list. A daemon that dies after its build has published (a dispatch panic, spec §9 test 12) is restarted at once, as the spec says. The cost is up to a minute of fail-closed starts after a build died once (an operator can run `--standalone`); the alternatives are no bound (one line), a longer window (a constant), or counting consecutive deaths (more state).
2. **The shim checks the uid of the daemon it connects to (SI-23, D78).** Spec §4.4 has only the daemon check its peer. The S2a review record (spec §13) concluded that the isolation guarantee between users is the peer-uid check on both ends of the socket, since path checks cannot enumerate every way another user might plant a listener, and asked the owner to confirm. **Default built:** the shim reads its peer's uid right after `connect`, before it sends anything, and fails closed on any other uid or an unreadable one. HANDOFF §4 item 6 recorded this as an S5a design item; it is listed here because the spec's text does not state it yet.

---

**Goal:** the lane's: let the a2a-bridge run its reviewer and implementer fan-out on this host without swap exhaustion, by holding one resident index per compatible (checkout, prism build, cache configuration, refresh policy), with no query-latency regression under fan-out. **What S5a must make true, and test:** `prism-mcp --repo X --shared` finds the one daemon of its configuration, or starts it when none is alive, and then carries its client's MCP session to it byte for byte (spec §4.1 steps 1–9; §5 "preserved"); every shim that starts at the same time ends up on the same daemon, whatever order the locks fall in and whether an earlier starter died mid-start (§4.1 step 6, §4.6; the shim's halves of §9 tests 8 and 15); a daemon that is retiring, dead or wedged is told apart and handled (re-enter arbitration, replace, or report and leave alone: the shim's halves of tests 3, 4 and 13); every way of not reaching `READY` before the one deadline fails closed with one actionable line and builds no index (§4.1 step 7, §10; tests 5 and 10); a daemon lost after the first forwarded byte ends the session and is never reconnected (§4.1 step 9; test 11); and a build that keeps dying does not cost a daemon per shim (S3b's SI-10). The in-process server does not change; `--shared` stays opt-in (spec §11 item 2).

**Architecture:** `src/mcp/shim/` is a new module. `connect.rs` makes one attempt: connect, check the peer's uid, send `HELLO`, read the reply line with every read bounded by the time left, and sort the outcome into reached, transient or refused. `arbitrate.rs` is the start protocol around it: try the socket; on anything transient take `<key16>.start`; under it try again, then look at `<key16>.owner`: held means a daemon is alive (poll; never touch its files), free means none is (holding the lock, remove the stale socket, pid file and build marker, release it, spawn a daemon through the `Starter` seam). `key_files.rs` opens those files through the runtime directory's descriptor with the daemon's own functions (`daemon::files`, now `pub(in crate::mcp)`). `spawn.rs` starts `prism-mcp daemon …` from the same executable in a session of its own, its output appended to the key's log. `diagnostic.rs` writes the fail-closed line. `pump.rs` is the byte pipe. `run.rs` is `run_shim`, called by `prism-mcp --shared`. On the daemon's side one wrapper (`run.rs::marked`) sets and clears the build marker around the builder (`files.rs::BuildMarker`).

**Tech Stack:** Rust 2021 (rustc 1.94), `std::os::unix::net::UnixStream`, `std::fs::File::try_lock` (`flock`), `std::os::unix::process::CommandExt::pre_exec` (`setsid`), `libc` (already an optional dependency of the `mcp` feature: `setsid`, `getpeereid` / `SO_PEERCRED` through `daemon::peer`, `fcntl` in a test), `clap` for the flags, `tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§3, §4.1, §4.3, §4.4, §4.6, §5, §7, §8 step 4, §9 tests 3, 4, 5, 8, 10, 11, 12, 13, 15, §10, §11 items 2 and 7, §12 row S5a, §13 the S2a record). Lane plan: `README.md` in this directory (the notes for S4 and S5a, and for S5a). Built on S4 as merged: `main` `296102a2`. Decisions are numbered on from the S4 plan's (D48–D76) and spec issues from its SI-14–SI-21, so that one number names one thing across the lane.

**Ships as two pull requests.** **S5a-1 = Tasks 1–4** (the build marker, one attempt, the start protocol, all tested in process against fake daemons): 16 files in `src/`, 2,170 insertions and 15 deletions, of which about 770 are code (`arbitrate.rs` 323, `connect.rs` 222, `key_files.rs` 100, `files.rs` +74, `run.rs` +21, the rest a few lines each) and the rest tests and test support; plus 19 lines of `CLAUDE.md`. **S5a-2 = Tasks 5–7** (spawning, the fail-closed line, the pump, `run_shim` and the flags, process tests with the real binary): 16 files in `src/` and `tests/`, 1,483 insertions and 22 deletions, of which about 590 are code (`run.rs` 164, `diagnostic.rs` 141, `spawn.rs` 140, `pump.rs` 92, the binary +28, the rest a few lines each) and the rest tests; plus 16 lines of `CLAUDE.md`. They are split where a reviewer's question changes: S5a-1 is the protocol (who starts the daemon, what each answer means), with no process; S5a-2 is the process (what is spawned, what the client sees) and depends on every interface of S5a-1. Each is about the size of S4-1 or S4-2 (about 2,100 lines), which each converged in two review rounds. S5a-2 is based on S5a-1; nothing in S5a-1 depends on S5a-2. S5a-1 has no production caller until S5a-2 (its module carries a dead-code allowance that Task 6 removes, as S3a's did until S4).

## Global Constraints

- All production code is under `--features mcp` and `#[cfg(unix)]`, in `src/mcp/shim/` (new), `src/mcp/daemon/{files,run,peer,mod}.rs`, `src/mcp/shared/paths.rs` (one path), `src/mcp/mod.rs` (one module line, one re-export), `src/bin/prism-mcp.rs` (the flags and one branch), and `src/navigation/call_edge_cache.rs` (one constant made `pub(crate)`, no behavior); tests in `src/mcp/shim/`, `src/mcp/daemon/`, `src/mcp/shared/paths_tests.rs` and `tests/mcp/`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged, byte for byte.** Not edited: `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, the golden transcripts under `tests/fixtures/mcp/golden/` (never set `PRISM_MCP_BLESS_GOLDEN`). `src/bin/prism-mcp.rs` changes only by flags the standalone server rejects today and one branch taken when `--shared` is given; without it nothing that runs changes. Tasks 4 and 7 check it.
- **`--shared` stays opt-in** (spec §11 item 2, §8 step 1): the default is the in-process server; `--standalone` is accepted and changes nothing.
- Do not re-decide D1–D76. S4's interfaces for S5a are settled here on purpose and say so: the start lock is the shim's (S4 §"Spec items"), the log is opened `O_APPEND` (SI-16), `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` (SI-17), the retirement window, a reset read as the daemon's close (D76), the shim halves of tests 3, 4, 8, 13, 15 (SI-18), and S3b's SI-10 restart bound.
- Spec values, copied exactly:
  - "Shared mode requires a cache-backed, non-eager, non-owner config. `--eager`, `--no-cache` and `--owner-*` are served in-process and `--shared` with any of them is a CLI error, not a silent downgrade" (§4.1 step 2).
  - "Resolve the connection settings from this shim's own environment and flags: `PRISM_MCP_STRUCTURED_CONTENT`, `PRISM_MCP_CONCISE_SHAPE`, the result cap, `--first-call-wait`, and `--warm-at-startup`. They are sent in the preamble, never inherited from whichever shim started the daemon" (§4.1 step 3).
  - "Check the socket path length (≤ 103 bytes); if it does not fit, fail closed (step 7). Establish the one absolute deadline (`--daemon-start-timeout`, default 10 s) that bounds everything from here to `READY`" (§4.1 step 4).
  - "Try `connect`; on success run the preamble (§4.4) and, on `READY`, go to 8. On `RETIRING`, `ECONNREFUSED`, `ENOENT`, or EOF/reset/broken pipe before `READY`, go to 6. Preamble-time failures are transient by definition: no MCP byte has been forwarded yet, so re-entering arbitration is always safe" (§4.1 step 5).
  - "Take the start lock (`<key>.start`, `flock(LOCK_EX|LOCK_NB)` retried until the deadline). Under it, re-try step 5 … If that fails, take the owner lock exclusively and non-blocking: Acquired: … Holding it, unlink any stale socket and pid file, then release it and spawn the daemon detached (own session via `setsid`, stdin null, stdout and stderr to `<key>.log`, close-on-exec on every lock descriptor so the child never inherits either lock, the shim's environment) … Held: a daemon is alive. It may be Starting (keep retrying step 5 until the deadline) or wedged (preamble times out at the deadline → fail closed naming the pid and start time from `<key>.pid`). A socket whose owner lock is held is never unlinked. Keep the start lock until `READY` or the deadline" (§4.1 step 6).
  - "Fail closed. If no daemon reached `READY` by the deadline, the shim prints one actionable line to stderr (key, runtime dir, socket path, pid file contents, log path, and the exact `--standalone` invocation) and exits non-zero. It never silently builds a private index" (§4.1 step 7).
  - "Pump bytes both ways. Stdin EOF: half-close the socket, drain pending responses, exit 0. Socket EOF or error while the client is still open: exit non-zero" (§4.1 step 8). "Once a single byte of MCP traffic has been forwarded in either direction, the shim never replays or reconnects; a lost daemon is a lost session" (§4.1 step 9).
  - §10: "`RETIRING` / `BUSY` / `MISMATCH` | Shim | Retry from the lock step (RETIRING); fail closed with the reason (BUSY, MISMATCH)"; "Daemon spawn fails (exec error) | Shim | Fail closed".
  - Flags: "`--shared` / `--standalone`, `--idle-ttl <SECS>` (default 900), `--daemon-start-timeout <SECS>` (default 10), `--max-clients <N>` (default 64), `--warm-at-startup` (passed by the starting shim on the daemon's command line …)" (§8 step 4).
  - Tests 3, 4, 5, 8, 10, 11, 13, 15 as worded in §9 (quoted in the tasks that hold their shim halves).
- **No panic on a path a client can reach** in code this slice adds: no `unwrap`, `expect`, indexing or unchecked arithmetic outside tests; the one slice is the pump's `&buffer[..n]` with `n` as `read` returned it, which the `Read` contract bounds by the buffer's length (`framing.rs` does the same). Every `unsafe` block has a `SAFETY` comment and is one FFI call (or, for `pre_exec`, one closure that makes one).
- **Tests do not depend on the host's speed.** A test waits for a condition up to `HANG` (30 s in process, 60 s in the process tests) and asserts no upper bound on an elapsed time; a deadline a test means to expire is short (300–500 ms, or `--daemon-start-timeout 2`) and only its outcome is asserted. One test pauses 200 ms before opening a gate; the pause orders nothing a correct shim depends on (it gives a wrong one time to misbehave) and says so.
- **No test aborts the test binary**, and **every process a test starts is ended on every path**: shims by `ShimProcess`'s `Drop`, daemons a shim started (not the test's children) by `Daemons`' `Drop`, which kills the pid file's pid until the owner lock is free.
- Every runtime directory a test uses is a fresh `tempfile::Builder::new().prefix("s5a").tempdir_in("/tmp")` (or S4's `s4` prefix in the shared helpers): the socket path must fit 103 bytes.
- Keep every file under 600 lines; tests in sibling `*_tests.rs` files included with `#[path]`. The largest file this slice edits is `files.rs`, 482 lines after it; the largest it creates is `tests/mcp/shim_process_test.rs`, 477.
- No new clippy warning in a file this slice adds or edits.
- **Commit before any experiment.** The "prove the tests bite" steps edit a source file and restore it with `git checkout -- <file>`; they come after the task's commit.
- **Run the shim's and the daemon's tests on Linux before each pull request** (Tasks 4 and 7): a connection closed with unread input is a reset on Linux and end of stream on macOS, and CI is Linux. On this host that is the bridge's `a2a-toolchain` image with the worktree mounted read-only and named volumes for the target and the registry, removed afterwards.
- Logs go to files in a directory of the implementer's own, `$LOG_DIR` below; read the file. Do not pipe cargo output into `grep` or `tail`. Every cargo command runs alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …` (this host has 24 GB and is usually in swap).

### What each mechanism covers, and what it does not

- **The start lock orders shims of one runtime directory.** Shims that resolve different runtime directories (different `PRISM_MCP_RUNTIME_DIR`, `$TMPDIR` or `$XDG_RUNTIME_DIR`) start different daemons; that is spec §4.3's "every launcher that should share a daemon must resolve the same directory".
- **The owner lock is what says a daemon is alive**; the socket and the pid file are evidence a dead daemon leaves. A shim removes them only while it holds the owner lock, and never while another process holds it.
- **The build marker bounds a fast restart loop, not a slow one.** A daemon whose build dies more than 60 s after the build began is restarted by the next shim, so a build that runs for minutes and then dies still costs one daemon per shim, one build at a time (D77). It cannot tell a kill from a panic, and it holds a key closed for at most 60 s after one death during a build.
- **The deadline bounds the shim's own waits.** A blocking `connect` to a daemon whose listen backlog is full (Linux: 4,096 by default) could wait past it; S4's loop accepts up to 16 connections a turn, so a full backlog means a wedged daemon with thousands of shims queued. Not handled (SI-28).
- **The peer check is the guarantee between users** (spec §13, S2a record); the runtime directory checks are hardening. Neither is a defense against a process of the same user (spec §2) or root.
- **The daemon is started from the shim's executable path.** If a new build replaced that file after the shim started, the daemon it starts is the new build, computes another key, and refuses `--key`; the shim fails closed naming the log (SI-27). A shim started from the new binary is unaffected.
- **Descriptors the shim inherited without close-on-exec pass to the daemon.** The daemon's standard streams are replaced (null and the log); anything else its launcher leaked into the shim would stay open for the daemon's life. Node's `child_process` (codex, Claude Code) does not leak; not handled.
- **The pump forwards bytes, not messages.** Nothing is parsed, buffered past one read, or replayed; a frame cut by a daemon's death is cut for the client too (spec §4.1 step 9).

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously, or an earlier slice's note this slice was told to settle. "Cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D77 | **The restart bound is a build marker.** The daemon's builder runs inside `marked`, which writes `<key16>.build` (`pid\nstarted\n`, through the runtime directory's descriptor, best effort) before the builder and removes it after the builder returns, with an index or an error; `DaemonFiles::release` removes it too. A shim that holds the owner lock of a free key reads it: a whole marker whose build began less than `BUILD_CRASH_BACKOFF` = 60 s ago fails the shim closed (`RecentBuildCrash`) and nothing is removed; an older, partial or future-dated marker is removed with the other stale files. | S3b's SI-10 and its interface for S5a ("notice that the key's last daemon died … shortly after it started … for example a marker beside the pid file, written by the daemon before its first build and removed when the build publishes"). A marker, unlike the pid file's start time, tells a death during a build (deterministic panics, out-of-memory kills: both recur) from a death after publication (spec §9 test 12, which must restart at once). A builder that panics never returns, so the marker stays; it is not removed from a destructor, which would run while unwinding. A future-dated marker is ignored so a clock set back cannot hold a key closed. | One `if` in `arbitrate` (no bound), or a constant. Owner questions, item 1. |
| D78 | **The shim checks its peer's uid** (`daemon::peer::peer_uid`, widened to `pub(in crate::mcp)`) right after `connect` and before writing anything; another uid, or one that cannot be read, is a refusal: the shim fails closed and sends nothing. | Lane README, notes for S5a, and spec §13: the peer check on both ends is the isolation guarantee. Refusing (not retrying) because a listener of another user at this user's socket path is an attack or a misconfiguration that waiting will not fix. | Owner questions, item 2. |
| D79 | **The shim reuses the daemon's file functions** (`open_runtime_dir`, `open_private` with its `O_EXCL`-then-open creation, `unlink_at`, `FilesError`) by widening them to `pub(in crate::mcp)`, and the peer check the same way, instead of moving them to `shared/`. | S4's interface suggested moving `peer_uid` to `shared/`; widening is a few words of diff in files reviewers have approved, and keeps every key file's I/O in `files.rs` (D53's macOS fix included). | A mechanical move later. |
| D80 | **How each answer is sorted** (`connect::reach`). Transient: any `connect` error, `RETIRING`, end of stream or any read error before a whole reply line (a reset included, D76), and no whole line by the deadline. Refused: `BUSY`, `MISMATCH`, `READY` for another identity, a peer of another uid or none, and a line this build cannot read (longer than 4 KiB included). A failed `HELLO` write never ends an attempt: the reply is read anyway. Every read is bounded by the time left, so the deadline bounds the line, not each silence. After `READY` both socket timeouts are cleared; a failure to clear them is transient (no MCP byte has been forwarded). | Spec §4.1 step 5, §10; S4's interface (a `BUSY` can arrive before the shim's write, which then fails with `EPIPE`; a close can read as a reset on Linux); the lane README's S2b notes (one call site compares `READY`'s identity with the key; `PreambleLineReader` has no deadline of its own). An unreadable reply is refused, not retried: the daemon at this path will not start making sense. | None. |
| D81 | **A daemon that accepts and does not answer by the deadline at the first attempt is reported at once**, without taking the start lock (`NoReady { last: NoReply }`). | Its deadline is spent; spec §4.4: a live but wedged daemon is "reported, never replaced". The line names the pid and start time from the pid file (§4.1 step 6). | None. |
| D82 | **While a daemon this shim started neither answers nor ends, the owner lock is not looked at again.** A started daemon that ends with status 0 before `READY` lost the key to another (S4: "exit 0 silently"); the shim looks again and, if the key is free, starts another. One that ends otherwise (a non-zero status, a signal) fails the shim closed at once (`DaemonExited`) and is not started again. | Looking takes the owner lock for a moment; a daemon claiming the key in that moment would leave believing another owns it (`a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for`, mutant 3J). A daemon that fails to start (a refused `--key`, a bad runtime directory, a bind error) will fail again; spec §10 "Daemon spawn fails → fail closed". | None. |
| D83 | **A held owner lock is looked at again every `POLL` = 10 ms until the deadline**, and nothing of the key's is touched while it is held. | The retirement window (S4's interface): a daemon removes its socket a moment before it releases the owner lock; a shim that gave up on a held lock would fail a fresh start, and one that never looked again would wait out its deadline. | A constant. |
| D84 | **How the daemon is spawned** (`spawn::daemon_command`): `std::env::current_exe()`, `daemon --repo <canonical root> --cache-dir <canonical cache base> --refresh-policy <p> --key <64 hex>` plus `--warm-at-startup`, `--idle-ttl`, `--max-clients` when the shim has them; `setsid` in `pre_exec`; current directory `/`; standard input null; standard output and error one descriptor of `<key16>.log` opened `O_WRONLY|O_APPEND`, created 0600 through the runtime directory's descriptor; the shim's environment without `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` and with `PRISM_MCP_RUNTIME_DIR` set to the runtime directory's parent. The child is reaped by a thread once `READY` is reached. | Spec §4.1 step 6; S4's interface for S5a (the daemon's command line; SI-16 `O_APPEND`; SI-17 the override, "S5a's spawn removes it"); spec §11 item 7. `/` so the daemon pins no directory of the shim's (a removed worktree). The runtime directory is passed so the daemon cannot resolve another one. Reaping keeps a daemon that ends while the shim lives from staying a zombie. | None. |
| D85 | **`--idle-ttl` and `--max-clients` are accepted with `--shared` and configure only a daemon this shim starts**; a running daemon keeps its own. | Spec §8 step 4 lists them without saying which process reads them; they are daemon settings, not connection settings, and not in the key (§4.3), so the first starter's values hold until the daemon retires. Tests use `--idle-ttl` to bound stray daemons. | Moving them into the key splits daemons per value. |
| D86 | **The flags.** `--shared` conflicts with `--standalone`, `--no-cache`, `--eager`, `--owner-compiler`, `--owner-config` (clap's own error); `--standalone` is accepted and is the default; `--daemon-start-timeout` 1–600 s (default 10), `--idle-ttl` 1–86,400 s, `--max-clients` 1–4,096 each require `--shared`. `run_shim` refuses a `NoCache` or `Eager` configuration too, for a library caller. | Spec §4.1 step 2 (a CLI error, not a downgrade), §8 step 4; the ranges are the daemon's (D60). `--standalone` lets launchers write the default explicitly before the default flip. | None. |
| D87 | **The fail-closed line** (`diagnostic::fail_closed_line`): `prism-mcp: --shared failed closed, no index was built: <reason>; key <64 hex>; runtime dir <d>; socket <s>; pid file <what it holds>; log <l>; to run without the shared daemon: <invocation>`, every control character escaped; exit status 1. The pid file is reported as its pid, start time and build, or as absent, unreadable, or "empty or partial" with its bytes. The invocation is this executable with the same repository (canonical), cache directory, refresh policy, first-call wait and warm start, and `--standalone`, shell-quoted. | Spec §4.1 step 7; the lane README's S2a note (escape paths where the one line is written); S4's interface (the pid file is rewritten in place, so it can be read empty or partial). | None. |
| D88 | **On `READY` the shim writes one line to standard error**: `prism-mcp: shared daemon pid <pid> (started <t>, generation <g>) at <socket>`. | Operators and the §8 soak (daemon restarts are counted by pid) need to know which daemon a session used; the process tests read it. Standard output is untouched. | One line to remove. |
| D89 | **The pump** (`pump::pump`): standard input to the socket on a thread, the socket to standard output on the calling thread, 64 KiB reads, a flush after each write. At the client's end of input the thread records it, then half-closes the socket; the daemon's end of stream after that is a clean end (exit 0); before it, or any read error, ends the session with exit 1 and one line. A failed write to the client ends it too. Nothing reconnects. | Spec §4.1 steps 8 and 9. Recording before half-closing means the daemon cannot end the connection in answer to the half-close before the shim knows the client finished. | None. |
| D90 | **In-process tests use fake daemons and a fake `Starter`**; process tests use the real binary. A fake daemon takes the real owner lock on a descriptor of its own (as a daemon process does) and answers by a script; the fake starter's children claim the key at start or when a gate opens. | Spec §9 asks for deterministic constructions. Every interleaving the start protocol must survive (a retirement between `connect` and `HELLO`, a starter dying between spawn and claim, a held owner lock released mid-wait) is played at a hook point (`connect::Point`) without sleeping; the real daemon's answers to the same situations are S4's tests. | — |
| D91 | **The hook is a production field set only by tests** (`Arbiter::hook: Option<Hook>`, `None` in `run_shim`). | Hook points need to exist in the code under test; the field costs a branch per point. | — |

## Spec items implemented, and spec issues found

Spec §12 row S5a, item by item: connect → Task 2 (`connect.rs`); arbitration → Task 3 (`arbitrate.rs`, `key_files.rs`); spawn → Task 5 (`spawn.rs`); fail-closed → Tasks 3 and 5 (`Failure`, `diagnostic.rs`); pump → Task 5 (`pump.rs`); the single deadline → Tasks 2, 3 and 6. Tests: 5 → Task 3 (`an_invalid_runtime_directory_fails_closed_before_any_file_is_made`) and Task 6 (`a_runtime_directory_that_fails_validation_fails_closed_and_creates_nothing`); 10 → Task 3 (`a_mismatch_or_a_peer_of_another_user_fails_closed_without_a_start`) and Task 6 (`a_daemon_that_answers_mismatch_fails_the_shim_closed`); 11 → Task 5 (`a_daemon_that_closes_while_the_client_is_there_ends_the_session_as_an_error`) and Task 6 (`a_daemon_killed_mid_session_ends_the_shim_without_a_reconnect`). The shim halves S4 deferred (SI-18): 3 → Tasks 3 and 6; 4 → Tasks 3 and 6; 8 and 15 → Task 3 (`arbitrate_race_tests.rs`) and Task 6 (`shims_started_together_with_an_orphan_daemon_share_one_daemon`); 13 → Task 3 (`a_shim_caught_by_a_retirement_is_admitted_by_a_fresh_daemon`). Also through real shims, as S3b's interface asked: test 12's "the next shim restarts it" (Task 6), test 9 (Task 6), test 7 in part (eight shims each served, Task 6), and spec §5's equivalence (Task 6, the transcript script standalone and through a shim, byte for byte). S3b's SI-10 restart bound → Tasks 1 and 3 (D77). S4's SI-16 → Task 5 (D84); SI-17 → Task 5 (D84).

- **SI-22 (§4.6 "Daemon crashes … next shim finds the owner lock free, unlinks the stale socket, restarts", against S3b's SI-10).** For a daemon that died during its build less than 60 s ago, the next shim fails closed instead (D77). Owner questions, item 1.
- **SI-23 (§4.4 "The daemon checks the peer's uid").** The shim checks too (D78); the spec's text names the daemon only. Owner questions, item 2.
- **SI-24 (§4.3 "Files").** A sixth file, `<key16>.build`, exists while a build runs and after a daemon died in one (D77). Removed by the daemon, by a draining daemon, or by the next shim that holds the owner lock; never a lock file.
- **SI-25 (§8 step 4 flags).** `--idle-ttl` and `--max-clients` are daemon settings a shim passes only to a daemon it starts (D85).
- **SI-26 (§4.1 step 6 "the shim's environment" against §7 "every answer-affecting setting is either in the key or in the per-connection preamble").** The one variable known to change answers outside both, `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`, is removed from the daemon's environment (D84; S4's SI-17 settled).
- **SI-27 (§4.1 step 6, spawning).** A shim whose executable was replaced by a new build since it started spawns the new build, whose key differs; the daemon refuses `--key` (D61) and the shim fails closed naming the log ("What each mechanism covers").
- **SI-28 (§4.1 step 4 "the one absolute deadline").** `UnixStream::connect` is blocking; on Linux it waits while the daemon's listen backlog is full. Not handled ("What each mechanism covers").

## Review Focus

Seven ways this code is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code.

1. **A fan-out starting at once.** The bridge starts eight or more agents together, and a starter can die mid-start; every shim must end up on one daemon and exactly one daemon must be started between them. → Task 3, `shims_racing_to_start_a_daemon_start_exactly_one` (paused at three boundaries), `a_starter_that_dies_after_spawning_leaves_exactly_one_daemon`, `a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for`; Task 6, `shims_started_together_with_an_orphan_daemon_share_one_daemon` (eight real shims and an orphan daemon).
2. **A daemon retiring while a shim starts.** `RETIRING`, an end of stream before `READY`, and a held owner lock with no socket must all lead to a fresh daemon within the deadline. → Task 3, `a_shim_caught_by_a_retirement_is_admitted_by_a_fresh_daemon`, `a_key_whose_owner_is_releasing_it_is_started_once_the_lock_is_free`.
3. **Dead, stale and wedged daemons told apart.** A dead daemon's files are replaced; a live daemon's are never touched, even when it does not answer. → Task 3, `a_stale_socket_and_pid_file_are_removed_under_the_owner_lock_before_the_start`, `a_live_but_wedged_owner_is_reported_and_its_files_are_left_alone`; Task 6, `a_stale_socket_left_by_a_dead_daemon_is_replaced`, `a_live_but_wedged_owner_fails_the_shim_closed_naming_its_pid`.
4. **The session through the shim is the session.** Byte for byte, partial lines and frames larger than a socket buffer included, and a clean end when the client closes. → Task 5, `bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly`; Task 6, `a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte`.
5. **A daemon lost mid-session.** The shim must end with an error and never start or reach another daemon. → Task 6, `a_daemon_killed_mid_session_ends_the_shim_without_a_reconnect`.
6. **A build that keeps dying.** Fail closed for a minute, then restart; a daemon lost after its build restarts at once. → Task 1 (`a_builder_that_panics_leaves_the_marker`), Task 3 (`a_build_that_died_moments_ago_is_not_restarted`, `an_old_future_or_partial_build_marker_does_not_hold_the_key`), Task 6 (`a_daemon_that_died_during_its_build_is_not_restarted_for_a_minute`, `the_next_shim_restarts_a_daemon_that_died`).
7. **The same tests on Linux.** A close reads as a reset there and a refusal can beat the shim's write. → Tasks 4 and 7 run the shim's and the daemon's tests on Linux; `busy_and_mismatch_are_refused_even_when_the_hello_cannot_be_written`.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or comments state, with the test that fails when it breaks and one wrong implementation that test rules out. Every mutant in the last column was applied in validation and the test failed.

| Claim | Test | A mutant it kills |
|---|---|---|
| A build that returns, with an index or an error, removes its marker; one that panics leaves it; a drain removes it. | T1 `the_marker_is_there_while_the_build_runs_and_gone_after_it_succeeds`, `the_marker_is_removed_when_the_build_fails`, `a_builder_that_panics_leaves_the_marker`, `releasing_the_files_removes_the_marker` | no `clear` (1A); `clear` from a drop guard (1B); `release` without the removal (1C) |
| No preamble timeout outlives the preamble. | T2 `a_ready_for_this_key_is_reached_and_the_stream_has_no_timeouts` | timeouts not cleared (2A) |
| The reply is read even when the `HELLO` could not be written. | T2 `busy_and_mismatch_are_refused_even_when_the_hello_cannot_be_written` | a failed write ends the attempt (2B) |
| A peer of another user is refused before anything is sent to it. | T2 `a_peer_of_another_user_is_refused_before_anything_is_sent`; T3 `a_mismatch_or_a_peer_of_another_user_fails_closed_without_a_start` | no peer check (2C); the check after `HELLO` (2D) |
| The deadline bounds the whole reply line, not each silence. | T2 `a_daemon_that_does_not_finish_its_reply_is_no_reply_at_the_deadline` | reads without a bound (2E) |
| `READY` admits only for this shim's key. | T2 `a_ready_for_another_identity_is_refused` | the identity not compared (2F) |
| `RETIRING` and a close before `READY` are transient, never a refusal. | T2 `retiring_and_a_close_before_any_reply_are_transient`; T3 `a_shim_caught_by_a_retirement_is_admitted_by_a_fresh_daemon` | `RETIRING` refused (2G); a close refused (2H) |
| A dead daemon's files are removed only while the shim holds the owner lock, and the lock is released before the spawn. | T3 `a_stale_socket_and_pid_file_are_removed_under_the_owner_lock_before_the_start` | no removal (3A); the lock released after the spawn (3B) |
| A socket whose owner lock is held is never unlinked, and nothing is started in its place. | T3 `a_live_but_wedged_owner_is_reported_and_its_files_are_left_alone` | removal and a start while the lock is held (3C) |
| A held owner lock is looked at again until it is free. | T3 `a_key_whose_owner_is_releasing_it_is_started_once_the_lock_is_free` | giving up on a held lock (3D) |
| A started daemon that ends unsuccessfully before `READY` ends the start and is not started again. | T3 `a_started_daemon_that_fails_before_ready_ends_the_start` | a failure taken as a lost claim (3E) |
| A key whose last daemon died during a build that began under 60 s ago is not restarted, and its files are left. | T3 `a_build_that_died_moments_ago_is_not_restarted`; T6 `a_daemon_that_died_during_its_build_is_not_restarted_for_a_minute` | no bound (3F) |
| An older, future-dated or partial marker does not hold the key. | T3 `an_old_future_or_partial_build_marker_does_not_hold_the_key` | a future start time counted as recent (3G) |
| Shims that race start exactly one daemon. | T3 `shims_racing_to_start_a_daemon_start_exactly_one`, `a_start_lock_held_until_the_deadline_fails_closed` | no start lock (3I) |
| The owner lock is not looked at while a started daemon is still starting. | T3 `a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for` | looking anyway (3J) |
| The daemon's environment has no `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`. | T5 `the_daemon_s_environment_drops_the_dirty_sidecar_override_and_names_the_runtime_dir` | not removed (5A) |
| The daemon's log is opened for appending. | T5 `the_log_is_opened_for_appending` | no `O_APPEND` (5B) |
| The client's end of input reaches the daemon as the end of its input, and is a clean end of the session. | T5 `bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly` | every daemon close taken as an error (5D); no half-close (5E; survived the test as first written, see the Validation record) |
| The fail-closed line is one line whatever the paths hold, and says what the pid file holds. | T5 `control_characters_in_paths_cannot_end_the_line`, `the_line_names_everything_an_operator_needs_on_one_line`, `an_empty_partial_or_missing_pid_file_is_reported_as_such` | no escaping (5F); the pid file's path instead of its contents (5G) |
| The daemon runs in a session of its own. | T6 `a_shim_starts_a_detached_daemon_and_serves_a_session_through_it` | no `setsid` (6A) |
| A daemon lost mid-session ends the shim with an error, and no daemon is reached or started again. | T6 `a_daemon_killed_mid_session_ends_the_shim_without_a_reconnect`; T5 `a_daemon_that_closes_while_the_client_is_there_ends_the_session_as_an_error` | a daemon's close taken as a clean end (6B) |
| The daemon holds none of the client's pipes. | T6 `a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte` | the daemon's standard output inherited (6C) |
| The shim never builds a private index. | T6 `a_runtime_directory_that_fails_validation_fails_closed_and_creates_nothing` | falling back to the in-process server (6D) |
| A session through the shim is byte for byte the standalone session (same snapshot, settings and readiness, spec §5). | T6 `a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte` | (6C) |
| Shims and an orphan daemon started together share one daemon. | T6 `shims_started_together_with_an_orphan_daemon_share_one_daemon` | (3I, in process) |

Stated and **not** held by a test: that a failure to clear the socket's timeouts after `READY` is transient (no seam makes `setsockopt` fail on a live socket); that a `READY` whose peer then closes is retried (it is, through that path, on macOS; not asserted); the bound on a blocking `connect` (SI-28); that the reaper thread reaps (a zombie is not observable from the test without racing the shim's exit); descriptors leaked into the shim ("What each mechanism covers"); the fail-closed line for a runtime directory that cannot be resolved at all (`PRISM_MCP_RUNTIME_DIR` relative), which uses the same function as the tested lines; x86_64 Linux before CI (the tests ran on aarch64 Linux).

## File Structure

| File | Responsibility | Change | PR |
|---|---|---|---|
| `src/mcp/shared/paths.rs`, `paths_tests.rs` | `DaemonPaths::build_marker` (`<key16>.build`) | Modify (Task 1) | S5a-1 |
| `src/mcp/daemon/files.rs` | `BuildMarker` and its removal at release (Task 1); `parse_build_marker` and `pub(in crate::mcp)` file functions (Task 3) | Modify | S5a-1 |
| `src/mcp/daemon/run.rs`, `run_tests.rs` | `marked`: the marker around the builder | Modify, create (Task 1) | S5a-1 |
| `src/mcp/daemon/peer.rs`, `src/mcp/daemon/mod.rs` | `peer_uid` and the `files`, `peer` modules visible to the shim | Modify (Tasks 2, 3) | S5a-1 |
| `src/mcp/mod.rs` | `mod shim;` (Task 2); the re-export (Task 6) | Modify | both |
| `src/mcp/shim/mod.rs` | The module (its dead-code allowance removed in Task 6) | Create (Task 2), modify (Tasks 3, 5, 6) | both |
| `src/mcp/shim/connect.rs`, `connect_tests.rs` | One attempt; `Point`, `Hook` | Create (Task 2), modify (Task 3) | S5a-1 |
| `src/mcp/shim/shim_support.rs` | Test support: runtime directory, keys, lines; fake daemons and starters, gates (Task 3) | Create (Task 2), modify (Task 3) | S5a-1 |
| `src/mcp/shim/key_files.rs` | The shim's view of the key's files (Task 3); the log (Task 5) | Create, modify | both |
| `src/mcp/shim/arbitrate.rs`, `arbitrate_tests.rs`, `arbitrate_race_tests.rs` | The start protocol; `Starter`, `Started`, `Failure` | Create (Task 3) | S5a-1 |
| `src/mcp/shim/spawn.rs`, `spawn_tests.rs` | The daemon's command and process | Create (Task 5) | S5a-2 |
| `src/mcp/shim/diagnostic.rs`, `diagnostic_tests.rs` | The fail-closed line, the standalone invocation | Create (Task 5) | S5a-2 |
| `src/mcp/shim/pump.rs`, `pump_tests.rs` | The byte pipe | Create (Task 5) | S5a-2 |
| `src/navigation/call_edge_cache.rs` | `LOAD_DIRTY_OVERRIDE` made `pub(crate)` | Modify (Task 5) | S5a-2 |
| `src/mcp/shim/run.rs` | `run_shim`, `ShimOptions` | Create (Task 6) | S5a-2 |
| `src/bin/prism-mcp.rs` | `--shared`, `--standalone`, `--daemon-start-timeout`, `--idle-ttl`, `--max-clients` | Modify (Task 6) | S5a-2 |
| `tests/mcp/main.rs`, `daemon_process_test.rs` | Two module lines; S4's helpers made `pub(crate)` | Modify (Task 6) | S5a-2 |
| `tests/mcp/shim_process_test.rs`, `shim_session_process_test.rs` | The shim as a process (tests 3, 4, 5, 7–13, 15, §5) | Create (Task 6) | S5a-2 |
| `CLAUDE.md` | MCP Adapter, the shim paragraph | Modify (Tasks 4, 7) | both |
| `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `tests/fixtures/mcp/golden/*` | The standalone contract | **Must not change** | |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file at that point of the plan, then "with this:" and its replacement.

The replacements were generated from the validated branch's per-task commits, so each carries a few lines of unchanged context around its change; the context is part of the anchor and is not edited.

---

# Part one: S5a-1, the start protocol (Tasks 1–4)

### Task 1: The daemon's build marker

**Files:**
- Modify: `src/mcp/shared/paths.rs`, `src/mcp/shared/paths_tests.rs`, `src/mcp/daemon/files.rs`, `src/mcp/daemon/run.rs`
- Create: `src/mcp/daemon/run_tests.rs`

**Interfaces:**
- Consumes: `DaemonFiles` (S4: `claim`, `paths`, `release`, its private `dir`), `open_private`, `unlink_at`, `Create`; `SessionBuilder` (`lazy.rs`); `run_daemon` (S4).
- Produces: `DaemonPaths::build_marker(&self) -> &Path` (`<key16>.build`); in `daemon::files`: `pub(super) struct BuildMarker` with `set(&self)` (writes `pid\nstarted\n`, best effort) and `clear(&self)`; `DaemonFiles::build_marker(&self) -> Result<BuildMarker, FilesError>`; `DaemonFiles::release` now also removes the marker; in `daemon::run`: `fn marked(builder: SessionBuilder, marker: BuildMarker) -> SessionBuilder`, which `run_daemon` wraps the real builder in.

- [ ] **Step 1: Write the failing tests**

In `src/mcp/daemon/run.rs`, replace this:

````rust
    }
    Ok(())
}
````

with this:

````rust
    }
    Ok(())
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
````

Create `src/mcp/daemon/run_tests.rs`:

````rust
//! Tests for the build marker around the daemon's builder (S5a; S3b's SI-10): set while the
//! builder runs, removed when it returns either way, left when it panics.
use super::*;
use crate::mcp::daemon::test_support::fixture;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::RefreshPolicy;
use std::path::{Path, PathBuf};

/// A key's files, owned, in a runtime directory of the test's own under `/tmp`.
fn owned_files() -> (tempfile::TempDir, DaemonFiles) {
    let base = tempfile::Builder::new()
        .prefix("s5a")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let key = IdentityKey::from_hex(&"ab".repeat(32)).expect("a key");
    match DaemonFiles::claim(&base.path().join("prism-mcpd"), &key).expect("claim") {
        Claim::Owned(files) => (base, *files),
        Claim::Taken => panic!("the key is taken"),
    }
}

fn marker_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// While the builder runs the marker holds this daemon's pid and a start time; once the builder
/// has returned an index the marker is gone. Rules out a marker that is never set, and one that
/// is set but not removed after a good build.
#[test]
fn the_marker_is_there_while_the_build_runs_and_gone_after_it_succeeds() {
    let (_base, files) = owned_files();
    let path: PathBuf = files.paths().build_marker().to_path_buf();
    let (_repo, cfg) = fixture(RefreshPolicy::WarnOnly);
    let seen = Arc::new(std::sync::Mutex::new(None));
    let builder: SessionBuilder = {
        let (path, seen) = (path.clone(), Arc::clone(&seen));
        Arc::new(move || {
            *seen.lock().expect("seen") = marker_text(&path);
            SessionProvider::bootstrap(&cfg)
        })
    };
    let built = marked(builder, files.build_marker().expect("a marker"))();
    assert!(built.is_ok());
    let during = seen
        .lock()
        .expect("seen")
        .clone()
        .expect("a marker during the build");
    let mut lines = during.lines();
    assert_eq!(lines.next(), Some(std::process::id().to_string().as_str()));
    let started: u64 = lines
        .next()
        .expect("a start time")
        .parse()
        .expect("seconds");
    assert!(started > 1_700_000_000, "{during}");
    assert!(marker_text(&path).is_none(), "removed after the build");
}

/// A build that fails is not a build that died: the marker is removed. Rules out removing it
/// only on success.
#[test]
fn the_marker_is_removed_when_the_build_fails() {
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    let builder: SessionBuilder = Arc::new(|| Err(anyhow::anyhow!("the build failed")));
    let built = marked(builder, files.build_marker().expect("a marker"))();
    assert!(built.is_err());
    assert!(marker_text(&path).is_none());
}

/// A builder that panics never returns, so the marker stays for the next shim (in production
/// the panic aborts the daemon, spec §3.1). Rules out removing the marker from a destructor,
/// which would run during the unwinding.
#[test]
fn a_builder_that_panics_leaves_the_marker() {
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    let builder: SessionBuilder = Arc::new(|| panic!("a builder panic, on purpose"));
    let wrapped = marked(builder, files.build_marker().expect("a marker"));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wrapped()));
    assert!(outcome.is_err());
    let left = marker_text(&path).expect("the marker is left");
    assert!(
        left.starts_with(&format!("{}\n", std::process::id())),
        "{left}"
    );
}

/// A drain can end a daemon while its build runs; that is not a build that died, so releasing
/// the files removes the marker too. Rules out a release that leaves it, which would make the
/// next shim refuse to start a daemon after an ordinary SIGTERM.
#[test]
fn releasing_the_files_removes_the_marker() {
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    files.build_marker().expect("a marker").set();
    assert!(marker_text(&path).is_some());
    files.release();
    assert!(marker_text(&path).is_none());
}
````

In `src/mcp/shared/paths_tests.rs`, replace this:

````rust
    assert_eq!(paths.owner_lock(), dir.join("0123456789abcdef.owner"));
    assert_eq!(paths.pid_file(), dir.join("0123456789abcdef.pid"));
    assert_eq!(paths.log_file(), dir.join("0123456789abcdef.log"));
}

/// The file names are a locator, not the identity: two keys that agree in their first 16
````

with this:

````rust
    assert_eq!(paths.owner_lock(), dir.join("0123456789abcdef.owner"));
    assert_eq!(paths.pid_file(), dir.join("0123456789abcdef.pid"));
    assert_eq!(paths.log_file(), dir.join("0123456789abcdef.log"));
}

/// The build marker a daemon leaves while its build runs (S5a) is named like the other files.
#[test]
fn the_build_marker_is_named_by_the_first_16_hex_digits_too() {
    let dir = Path::new("/run/user/501/prism-mcpd");
    let paths = DaemonPaths::new(dir, &key(KEY)).unwrap();
    assert_eq!(paths.build_marker(), dir.join("0123456789abcdef.build"));
}

/// The file names are a locator, not the identity: two keys that agree in their first 16
````

- [ ] **Step 2: Run them to see them fail**

```bash
LOG_DIR=/tmp/s5a; mkdir -p "$LOG_DIR"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t1-red.log" 2>&1
```

Expected in the log: `error[E0425]: cannot find function `marked` in this scope` (three times) and `error[E0599]: no method named `build_marker` found` for `DaemonPaths` and `DaemonFiles` (nine times), with two `E0282` that follow from them; `could not compile `prism` (lib test) due to 14 previous errors`.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/files.rs`, replace this:

````rust
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

/// The mode lock files and the pid file are created with.
const PRIVATE_FILE_MODE: libc::c_uint = 0o600;
````

with this:

````rust
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The mode lock files and the pid file are created with.
const PRIVATE_FILE_MODE: libc::c_uint = 0o600;
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
        }
    }

    /// Remove the pid file if it still holds what this daemon wrote, then release the owner
    /// lock (spec §3.1: "the pid file is removed, the owner lock released, exit 0").
    pub(super) fn release(self) {
        if let Some(written) = &self.pid_contents {
            let pid_file = self.paths.pid_file();
            let mut found = String::new();
````

with this:

````rust
        }
    }

    /// The marker this daemon's build sets while it runs (`<key16>.build`). It has a descriptor
    /// of the runtime directory of its own, so the build thread can use it while the accept
    /// loop holds this value.
    pub(super) fn build_marker(&self) -> Result<BuildMarker, FilesError> {
        let dir = self.dir.try_clone().map_err(|source| FilesError::Io {
            op: "duplicate the descriptor of",
            path: self.paths.runtime_dir().to_path_buf(),
            source,
        })?;
        Ok(BuildMarker {
            dir,
            path: self.paths.build_marker().to_path_buf(),
            pid: std::process::id(),
        })
    }

    /// Remove the pid file if it still holds what this daemon wrote, and the build marker, then
    /// release the owner lock (spec §3.1: "the pid file is removed, the owner lock released,
    /// exit 0"). A drain can end a daemon while its build runs; that is not a build that died.
    pub(super) fn release(self) {
        let _ = unlink_at(&self.dir, self.paths.build_marker());
        if let Some(written) = &self.pid_contents {
            let pid_file = self.paths.pid_file();
            let mut found = String::new();
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
        }
        // Closing the only descriptor of the lock file releases the lock.
        drop(self.owner);
    }
}

````

with this:

````rust
        }
        // Closing the only descriptor of the lock file releases the lock.
        drop(self.owner);
    }
}

/// What a daemon leaves in the runtime directory while its index build runs: `<key16>.build`,
/// holding `pid\nstarted\n` (`started` in seconds since the Unix epoch). The build removes it
/// when the builder returns, with an index or an error, and a drain removes it; a daemon that
/// dies during its build (a panic on the build thread aborts it, spec §3.1; or a kill) leaves
/// it. The next shim that finds the key free reads it and does not restart a daemon whose build
/// died moments ago (S5a; S3b's SI-10).
pub(super) struct BuildMarker {
    dir: OwnedFd,
    path: PathBuf,
    pid: u32,
}

impl BuildMarker {
    /// Record that a build is running. Best effort: a marker that cannot be written costs the
    /// restart bound, never the build.
    pub(super) fn set(&self) {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        if let Ok(mut file) = open_private(
            &self.dir,
            &self.path,
            libc::O_WRONLY | libc::O_TRUNC,
            Create::Yes,
        ) {
            let _ = file.write_all(format!("{}\n{started}\n", self.pid).as_bytes());
        }
    }

    /// The build returned: remove the marker.
    pub(super) fn clear(&self) {
        let _ = unlink_at(&self.dir, &self.path);
    }
}

````

In `src/mcp/daemon/run.rs`, replace this:

````rust
//! The daemon process, `prism-mcp daemon ...` (spec §3.1 "Starting", §4.2, §8 step 4): resolve
//! the identity, own the key or leave silently, bind, write the pid file, serve, drain.
use super::coordinator::DaemonState;
use super::files::{Claim, DaemonFiles};
use super::log;
use super::server::{ServeOptions, Served, Server, Waker, HANDSHAKE_TIMEOUT};
use crate::mcp::lazy::SessionBuilder;
````

with this:

````rust
//! The daemon process, `prism-mcp daemon ...` (spec §3.1 "Starting", §4.2, §8 step 4): resolve
//! the identity, own the key or leave silently, bind, write the pid file, serve, drain.
use super::coordinator::DaemonState;
use super::files::{BuildMarker, Claim, DaemonFiles};
use super::log;
use super::server::{ServeOptions, Served, Server, Waker, HANDSHAKE_TIMEOUT};
use crate::mcp::lazy::SessionBuilder;
````

In `src/mcp/daemon/run.rs`, replace this:

````rust

    let builder: SessionBuilder = {
        let cfg = cfg.clone();
        Arc::new(move || SessionProvider::bootstrap(&cfg))
    };
    let state = DaemonState::new(builder, cfg.startup)?;
    let served = Served {
````

with this:

````rust

    let builder: SessionBuilder = {
        let cfg = cfg.clone();
        marked(
            Arc::new(move || SessionProvider::bootstrap(&cfg)),
            files.build_marker()?,
        )
    };
    let state = DaemonState::new(builder, cfg.startup)?;
    let served = Served {
````

In `src/mcp/daemon/run.rs`, replace this:

````rust
    let reason = server.run()?;
    log::line(&format!("pid {pid} stopped: {}", reason.describe()));
    Ok(())
}

/// The descriptor the signal handler writes to; -1 until one is installed.
````

with this:

````rust
    let reason = server.run()?;
    log::line(&format!("pid {pid} stopped: {}", reason.describe()));
    Ok(())
}

/// `builder` with the build marker set while it runs and removed when it returns, with an index
/// or an error. A builder that panics does not return: the abort that follows (spec §3.1) leaves
/// the marker for the next shim to find (S5a, S3b's SI-10).
fn marked(builder: SessionBuilder, marker: BuildMarker) -> SessionBuilder {
    Arc::new(move || {
        marker.set();
        let built = builder();
        marker.clear();
        built
    })
}

/// The descriptor the signal handler writes to; -1 until one is installed.
````

In `src/mcp/shared/paths.rs`, replace this:

````rust
    owner_lock: PathBuf,
    pid_file: PathBuf,
    log_file: PathBuf,
}

impl DaemonPaths {
````

with this:

````rust
    owner_lock: PathBuf,
    pid_file: PathBuf,
    log_file: PathBuf,
    build_marker: PathBuf,
}

impl DaemonPaths {
````

In `src/mcp/shared/paths.rs`, replace this:

````rust
            owner_lock: file("owner"),
            pid_file: file("pid"),
            log_file: file("log"),
        })
    }

````

with this:

````rust
            owner_lock: file("owner"),
            pid_file: file("pid"),
            log_file: file("log"),
            build_marker: file("build"),
        })
    }

````

In `src/mcp/shared/paths.rs`, replace this:

````rust
    pub fn log_file(&self) -> &Path {
        &self.log_file
    }
}

#[cfg(test)]
````

with this:

````rust
    pub fn log_file(&self) -> &Path {
        &self.log_file
    }

    /// `<key16>.build`: present while the daemon's index build runs. A daemon that dies during
    /// its build leaves it, and the next shim reads it to bound how often it restarts a daemon
    /// whose build keeps dying (S5a).
    pub fn build_marker(&self) -> &Path {
        &self.build_marker
    }
}

#[cfg(test)]
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t1.log" 2>&1
```

Expected: `test result: ok. 522 passed; 0 failed` (517 before + 5), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/shared/paths.rs src/mcp/shared/paths_tests.rs src/mcp/daemon/files.rs src/mcp/daemon/run.rs src/mcp/daemon/run_tests.rs
git commit -m "feat(mcp): the daemon leaves a build marker while its index build runs"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::daemon::run::`, see the named test fail, then `git checkout -- <file>`.

| Mutant | Edit | Fails |
|---|---|---|
| 1A | in `run.rs`, `marked`: delete the line `marker.clear();` | `the_marker_is_there_while_the_build_runs_and_gone_after_it_succeeds`, `the_marker_is_removed_when_the_build_fails` |
| 1B | in `marked`, clear from a drop guard instead: make the closure body `struct Clear<'a>(&'a BuildMarker); impl Drop for Clear<'_> { fn drop(&mut self) { self.0.clear(); } } marker.set(); let _clear = Clear(&marker); builder()` | `a_builder_that_panics_leaves_the_marker` |
| 1C | in `files.rs`, `release`: delete `let _ = unlink_at(&self.dir, self.paths.build_marker());` | `releasing_the_files_removes_the_marker` |

### Task 2: One attempt to reach a daemon

**Files:**
- Modify: `src/mcp/mod.rs`, `src/mcp/daemon/mod.rs`, `src/mcp/daemon/peer.rs`
- Create: `src/mcp/shim/mod.rs`, `src/mcp/shim/connect.rs`, `src/mcp/shim/connect_tests.rs`, `src/mcp/shim/shim_support.rs`

**Interfaces:**
- Consumes: `IdentityKey`, `Hello`, `Reply`, `Ready`, `BuildState` (S2b); `PreambleLineReader`, `PreambleReadError` (S2b); `peer::peer_uid` (S4), widened to `pub(in crate::mcp)` with `pub(super) mod peer;`; `current_uid`, `validate_runtime_dir`, `DaemonPaths` (S2a).
- Produces (all `pub(super)` in `crate::mcp::shim::connect`): `type PeerUid = fn(&UnixStream) -> io::Result<u32>`; `enum Point { AfterConnect }` (Task 3 adds five points); `type Hook = Arc<dyn Fn(Point) + Send + Sync>`; `struct Target<'a> { socket: &'a Path, key: IdentityKey, hello: &'a str, deadline: Instant, peer_uid: PeerUid, uid: u32, hook: Option<&'a Hook> }`; `enum Reached { Ready(UnixStream, Ready), Transient(Transient), Refused(Refusal) }`; `enum Transient { Connect(io::Error), Retiring, Closed, NoReply }` and `enum Refusal { Busy { max_clients }, Mismatch { expected }, ReadyForAnother { identity }, ForeignPeer { uid }, PeerUnknown(io::Error), BadReply(String) }`, both `Display`; `fn reach(target: &Target<'_>) -> Reached`. Test support (`shim_support`): `HANG`, `key()`, `other_key()`, `runtime() -> (TempDir, DaemonPaths)`, `hello_line(key)`, `ready_line(key, pid)`, `this_user`, `another_user`, `read_line`, `read_to_end`.

- [ ] **Step 1: Write the failing tests**

In `src/mcp/mod.rs`, replace this:

````rust
pub mod settings;
#[cfg(unix)]
pub mod shared;
pub mod tools;
pub mod tools_reasoning;
pub mod tools_refresh;
````

with this:

````rust
pub mod settings;
#[cfg(unix)]
pub mod shared;
#[cfg(unix)]
mod shim;
pub mod tools;
pub mod tools_reasoning;
pub mod tools_refresh;
````

Create `src/mcp/shim/connect_tests.rs`:

````rust
//! Tests for one attempt to reach a daemon (spec §4.1 step 5, §4.4, §10): what is sent, and how
//! each answer is sorted into reached, transient or refused. Each fake daemon is a listener of
//! the test's own; nothing here sleeps to order two events.
use super::super::shim_support::{
    another_user, hello_line, key, other_key, read_line, read_to_end, ready_line, runtime,
    this_user, HANG,
};
use super::*;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::shared::runtime_dir::current_uid;
use std::os::unix::net::UnixListener;
use std::sync::mpsc;
use std::thread::JoinHandle;

const LONG: Duration = Duration::from_secs(60);

/// Attempt to reach the daemon of `paths` with this test's `HELLO`, on a thread of its own, and
/// give up after [`HANG`]: a regression that blocks must fail the test, not hang it.
fn attempt(
    paths: &DaemonPaths,
    within: Duration,
    peer_uid: PeerUid,
    hook: Option<Hook>,
) -> Reached {
    let socket = paths.socket().to_path_buf();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let hello = hello_line(key());
        let target = Target {
            socket: &socket,
            key: key(),
            hello: &hello,
            deadline: Instant::now() + within,
            peer_uid,
            uid: current_uid(),
            hook: hook.as_ref(),
        };
        let _ = tx.send(reach(&target));
    });
    rx.recv_timeout(HANG)
        .expect("the attempt ends within the hang limit")
}

/// A daemon that takes one connection and runs `answer` on it; the thread returns what
/// `answer` returns.
fn serve_one<T: Send + 'static>(
    paths: &DaemonPaths,
    answer: impl FnOnce(UnixStream) -> T + Send + 'static,
) -> JoinHandle<T> {
    let listener = UnixListener::bind(paths.socket()).expect("bind the fake daemon's socket");
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        answer(stream)
    })
}

/// Reads the `HELLO`, answers `reply`, and keeps the connection until the shim closes it, as a
/// daemon does.
fn answering(reply: String) -> impl FnOnce(UnixStream) -> String {
    move |mut stream| {
        let hello = read_line(&stream);
        let _ = stream.write_all(reply.as_bytes());
        let _ = read_to_end(&stream);
        hello
    }
}

/// Spec §4.4: the shim sends exactly its `HELLO` line and is admitted by `READY` for its key; the
/// stream it gets has no timeout left from the preamble. Rules out timeouts that outlive the
/// preamble (a quiet MCP session would end at the first silence longer than the deadline).
#[test]
fn a_ready_for_this_key_is_reached_and_the_stream_has_no_timeouts() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(ready_line(key(), 77)));
    let reached = attempt(&paths, LONG, this_user, None);
    let Reached::Ready(stream, ready) = reached else {
        panic!("not reached")
    };
    assert_eq!(ready.pid, 77);
    assert_eq!(stream.read_timeout().expect("read timeout"), None);
    assert_eq!(stream.write_timeout().expect("write timeout"), None);
    drop(stream);
    assert_eq!(daemon.join().expect("the daemon"), hello_line(key()));
}

/// Spec §4.1 step 5: `RETIRING`, and a connection that ends before any reply, are transient:
/// arbitration starts over.
#[test]
fn retiring_and_a_close_before_any_reply_are_transient() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(Reply::Retiring.encode()));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Retiring)
    ));
    daemon.join().expect("the daemon");
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    let daemon = serve_one(&paths, |stream| drop(read_line(&stream)));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Closed)
    ));
    daemon.join().expect("the daemon");
}

/// Spec §4.1 step 5: no socket (`ENOENT`) and a socket nobody listens on (`ECONNREFUSED`) are
/// transient.
#[test]
fn nothing_listening_is_transient() {
    let (_base, paths) = runtime();
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Connect(_))
    ));
    drop(UnixListener::bind(paths.socket()).expect("a stale socket"));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Connect(_))
    ));
}

/// Spec §10: `BUSY` and `MISMATCH` are refusals the shim fails closed on, and the reply is read
/// even when the daemon closed before the `HELLO` could be written (S4's interface: the daemon
/// answers `BUSY` at accept and closes; the shim's write then fails with `EPIPE`). Rules out
/// taking a failed write as the end of the attempt, which would retry to the deadline instead of
/// failing closed.
#[test]
fn busy_and_mismatch_are_refused_even_when_the_hello_cannot_be_written() {
    let (_base, paths) = runtime();
    let (closed_tx, closed_rx) = mpsc::channel::<()>();
    let closed_rx = std::sync::Mutex::new(closed_rx);
    let daemon = serve_one(&paths, move |mut stream| {
        let _ = stream.write_all(Reply::Busy { max_clients: 64 }.encode().as_bytes());
        drop(stream);
        let _ = closed_tx.send(());
    });
    // The shim waits, after connecting, until the daemon has answered and closed.
    let hook: Hook = Arc::new(move |point| {
        if point == Point::AfterConnect {
            let _ = closed_rx.lock().expect("rx").recv_timeout(HANG);
        }
    });
    assert!(matches!(
        attempt(&paths, LONG, this_user, Some(hook)),
        Reached::Refused(Refusal::Busy { max_clients: 64 })
    ));
    daemon.join().expect("the daemon");
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    let daemon = serve_one(
        &paths,
        answering(
            Reply::Mismatch {
                expected: other_key(),
            }
            .encode(),
        ),
    );
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Refused(Refusal::Mismatch { expected }) if expected == other_key()
    ));
    daemon.join().expect("the daemon");
}

/// The lane README's S2b note: the shim compares `READY`'s identity with its own key, so a
/// listener that answers `READY` for another identity is never pumped to.
#[test]
fn a_ready_for_another_identity_is_refused() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(ready_line(other_key(), 77)));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Refused(Refusal::ReadyForAnother { identity }) if identity == other_key()
    ));
    daemon.join().expect("the daemon");
}

/// The lane's S5a design item: a socket whose peer is another user is refused before anything
/// is sent to it, even when it would answer `READY`. Rules out a shim without the check, and one
/// that checks after sending its `HELLO`.
#[test]
fn a_peer_of_another_user_is_refused_before_anything_is_sent() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, |mut stream| {
        let _ = stream.write_all(ready_line(key(), 77).as_bytes());
        read_to_end(&stream)
    });
    assert!(matches!(
        attempt(&paths, LONG, another_user, None),
        Reached::Refused(Refusal::ForeignPeer { uid }) if uid == current_uid().wrapping_add(1)
    ));
    assert_eq!(daemon.join().expect("the daemon"), Vec::<u8>::new());
}

/// Spec §4.1 steps 4 and 6: a daemon that accepts and does not answer, or answers part of a line
/// and stops, holds the attempt only until the deadline. Rules out reads with no bound, and a
/// bound on each silence instead of the line.
#[test]
fn a_daemon_that_does_not_finish_its_reply_is_no_reply_at_the_deadline() {
    let (_base, paths) = runtime();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let daemon = serve_one(&paths, move |mut stream| {
        let _ = read_line(&stream);
        let _ = stream.write_all(b"RETIR");
        let _ = stop_rx.recv_timeout(HANG);
    });
    assert!(matches!(
        attempt(&paths, Duration::from_millis(300), this_user, None),
        Reached::Transient(Transient::NoReply)
    ));
    let _ = stop_tx.send(());
    daemon.join().expect("the daemon");
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    // A listener that never accepts: the connection waits in its backlog.
    let _listener = UnixListener::bind(paths.socket()).expect("bind");
    assert!(matches!(
        attempt(&paths, Duration::from_millis(300), this_user, None),
        Reached::Transient(Transient::NoReply)
    ));
}

/// A reply this build cannot read is a refusal, not a retry: the daemon at this path will not
/// start making sense.
#[test]
fn an_unreadable_reply_is_refused() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering("READY {}\n".to_string()));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Refused(Refusal::BadReply(_))
    ));
    daemon.join().expect("the daemon");
}
````

Create `src/mcp/shim/shim_support.rs`:

````rust
//! Pieces the shim's tests share: a runtime directory of the test's own, keys, preamble lines,
//! and a stand-in for one daemon connection.
//!
//! No test here sleeps to order two events: a fake daemon answers when it has read what it waits
//! for, and a test gives up on a result after [`HANG`].
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::{BuildState, Hello, Ready, Reply};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// How long a test waits for something that must happen before it calls the run hung.
pub(super) const HANG: Duration = Duration::from_secs(30);

/// The key the tests' daemons serve.
pub(super) fn key() -> IdentityKey {
    IdentityKey::from_hex(&"cd".repeat(32)).expect("a key")
}

pub(super) fn other_key() -> IdentityKey {
    IdentityKey::from_hex(&"ef".repeat(32)).expect("a key")
}

/// A validated runtime directory of the test's own under `/tmp` (short enough for a socket
/// path), and the paths of [`key`] in it.
pub(super) fn runtime() -> (tempfile::TempDir, DaemonPaths) {
    let base = tempfile::Builder::new()
        .prefix("s5a")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let dir: PathBuf = base.path().join("prism-mcpd");
    validate_runtime_dir(&dir).expect("a valid runtime directory");
    let paths = DaemonPaths::new(&dir, &key()).expect("paths");
    (base, paths)
}

/// The `HELLO` line a shim sends for `key`, newline included.
pub(super) fn hello_line(key: IdentityKey) -> String {
    let output = ConnectionSettings {
        result_cap: 80_000,
        structured_content: StructuredContentMode::Always,
        concise_shape: ConciseShapeMode::Slim,
    };
    let settings = ClientSettings::new(output, 20, false).expect("settings");
    Hello {
        identity: key,
        settings,
    }
    .encode()
}

/// The `READY` line of a daemon with `pid` serving `key`, newline included.
pub(super) fn ready_line(key: IdentityKey, pid: u32) -> String {
    Reply::Ready(Ready {
        identity: key,
        pid,
        start_time: 1_700_000_000,
        generation: 0,
        build: BuildState::Idle,
    })
    .encode()
}

/// This user's uid, as the peer check of a connection this process made reads it.
pub(super) fn this_user(stream: &UnixStream) -> std::io::Result<u32> {
    crate::mcp::daemon::peer::peer_uid(stream)
}

/// A peer of another user, as no test can be without root.
pub(super) fn another_user(_stream: &UnixStream) -> std::io::Result<u32> {
    Ok(current_uid().wrapping_add(1))
}

/// Read one line from a daemon-side stream, newline included, one byte at a time so nothing
/// past it is taken; what arrived before the end of stream if no newline came.
pub(super) fn read_line(mut stream: &UnixStream) -> String {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while let Ok(1) = stream.read(&mut byte) {
        line.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8_lossy(&line).into_owned()
}

/// Everything the peer sends until it closes.
pub(super) fn read_to_end(mut stream: &UnixStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    bytes
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t2-red.log" 2>&1
```

Expected in the log: `error[E0583]: file not found for module `shim``.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod fatal;
mod files;
mod log;
mod peer;
mod run;
mod server;
#[cfg(test)]
````

with this:

````rust
mod fatal;
mod files;
mod log;
pub(super) mod peer;
mod run;
mod server;
#[cfg(test)]
````

In `src/mcp/daemon/peer.rs`, replace this:

````rust
//!
//! This check, not the runtime directory's, is the isolation guarantee between users: the
//! directory checks are hardening by path, and two review rounds of S2a each found another way
//! around them. It is not a defense against a process of the same user (spec §2) or root.
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
````

with this:

````rust
//!
//! This check, not the runtime directory's, is the isolation guarantee between users: the
//! directory checks are hardening by path, and two review rounds of S2a each found another way
//! around them. It is not a defense against a process of the same user (spec §2) or root. The
//! shim makes the same check on the daemon it connects to (S5a), through this function.
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
````

In `src/mcp/daemon/peer.rs`, replace this:

````rust
/// The effective uid of the process that connected `stream`, as the kernel recorded it when it
/// connected.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
````

with this:

````rust
/// The effective uid of the process that connected `stream`, as the kernel recorded it when it
/// connected.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(in crate::mcp) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
````

In `src/mcp/daemon/peer.rs`, replace this:

````rust
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: the descriptor is an open socket, and `uid` and `gid` are valid for writes for
````

with this:

````rust
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub(in crate::mcp) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: the descriptor is an open socket, and `uid` and `gid` are valid for writes for
````

In `src/mcp/daemon/peer.rs`, replace this:

````rust
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
pub(super) fn peer_uid(_stream: &UnixStream) -> io::Result<u32> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no peer credential check on this platform",
````

with this:

````rust
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
pub(in crate::mcp) fn peer_uid(_stream: &UnixStream) -> io::Result<u32> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no peer credential check on this platform",
````

Create `src/mcp/shim/connect.rs`:

````rust
//! One attempt to reach a running daemon (spec §4.1 step 5, §4.4): connect to its socket, check
//! that the peer is this user, send `HELLO`, and read the one reply line before the deadline.
//!
//! Every way an attempt can end is sorted into one of three: `READY` for this key (the daemon is
//! reached), transient (nothing usable answered and arbitration may start over: spec §4.1 step
//! 5's `RETIRING`, `ECONNREFUSED`, `ENOENT`, and EOF, reset or broken pipe before `READY`), or a
//! refusal the shim fails closed on (spec §10: `BUSY`, `MISMATCH`; and a peer of another user).
//! No MCP byte has been forwarded when an attempt ends, so starting over is always safe.
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::preamble::{Ready, Reply};
use crate::mcp::shared::preamble_reader::{PreambleLineReader, PreambleReadError};
use std::fmt;
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How a peer's uid is read. Production: `daemon::peer::peer_uid`. A test plays another user.
pub(super) type PeerUid = fn(&UnixStream) -> io::Result<u32>;

/// Points in the start protocol a test can act at (spec §9 tests 8, 13 and 15 pause a shim at
/// them). Production installs no hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    /// Connected and the peer checked; `HELLO` not sent yet.
    AfterConnect,
}

pub(super) type Hook = Arc<dyn Fn(Point) + Send + Sync>;

/// What one attempt connects to and says.
pub(super) struct Target<'a> {
    pub(super) socket: &'a Path,
    pub(super) key: IdentityKey,
    /// The `HELLO` line, newline included.
    pub(super) hello: &'a str,
    /// The shim's one deadline (spec §4.1 step 4).
    pub(super) deadline: Instant,
    pub(super) peer_uid: PeerUid,
    /// This process's effective uid.
    pub(super) uid: u32,
    pub(super) hook: Option<&'a Hook>,
}

/// How an attempt ended.
pub(super) enum Reached {
    /// `READY` for this key. The stream is blocking with no timeouts, ready for the pump.
    Ready(UnixStream, Ready),
    Transient(Transient),
    Refused(Refusal),
}

/// Nothing usable answered; arbitration may start over within the deadline.
#[derive(Debug)]
pub(super) enum Transient {
    /// Nothing accepted the connection: no socket, a stale one, or a full backlog.
    Connect(io::Error),
    /// The daemon is draining (spec §3.1).
    Retiring,
    /// The connection ended, by end of stream or a reset, before a whole reply line.
    Closed,
    /// No whole reply line before the deadline: a daemon that accepts and does not answer.
    NoReply,
}

impl fmt::Display for Transient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(error) => write!(f, "connect failed: {error}"),
            Self::Retiring => write!(f, "the daemon answered RETIRING"),
            Self::Closed => write!(f, "the connection closed before a reply"),
            Self::NoReply => write!(f, "the daemon accepted but sent no reply"),
        }
    }
}

/// An answer the shim fails closed on (spec §10), or a peer it must not talk to.
#[derive(Debug)]
pub(super) enum Refusal {
    /// The daemon is at its connection bound.
    Busy { max_clients: u32 },
    /// The daemon serves another identity.
    Mismatch { expected: IdentityKey },
    /// `READY` for an identity other than the one asked for: no MCP byte may go to it.
    ReadyForAnother { identity: IdentityKey },
    /// The socket's peer is another user. The isolation guarantee between users is this check
    /// and the daemon's (lane README, notes for S5a); the runtime directory checks are hardening.
    ForeignPeer { uid: u32 },
    /// The peer's uid could not be read.
    PeerUnknown(io::Error),
    /// A reply line this build cannot read.
    BadReply(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy { max_clients } => write!(
                f,
                "the daemon answered BUSY: it already has {max_clients} clients (--max-clients)"
            ),
            Self::Mismatch { expected } => write!(
                f,
                "the daemon answered MISMATCH: it serves identity {expected}, another build or \
                 configuration"
            ),
            Self::ReadyForAnother { identity } => {
                write!(
                    f,
                    "the daemon answered READY for another identity, {identity}"
                )
            }
            Self::ForeignPeer { uid } => write!(
                f,
                "the socket's peer is uid {uid}, not this user; nothing was sent to it"
            ),
            Self::PeerUnknown(error) => write!(f, "the socket's peer uid is unknown: {error}"),
            Self::BadReply(error) => write!(f, "the daemon's reply is unreadable: {error}"),
        }
    }
}

/// The time left before `deadline`, or `None` once it has passed. Never zero, which a socket
/// timeout does not accept.
fn left(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
}

/// Make one attempt.
pub(super) fn reach(target: &Target<'_>) -> Reached {
    let mut stream = match UnixStream::connect(target.socket) {
        Ok(stream) => stream,
        Err(error) => return Reached::Transient(Transient::Connect(error)),
    };
    match (target.peer_uid)(&stream) {
        Ok(uid) if uid == target.uid => {}
        Ok(uid) => return Reached::Refused(Refusal::ForeignPeer { uid }),
        Err(error) => return Reached::Refused(Refusal::PeerUnknown(error)),
    }
    if let Some(hook) = target.hook {
        hook(Point::AfterConnect);
    }
    let Some(time) = left(target.deadline) else {
        return Reached::Transient(Transient::NoReply);
    };
    // A daemon that answers `BUSY` or `RETIRING`, or drops the connection, may close before the
    // line is written (`EPIPE`); its reply is read anyway (S4's interface for S5a).
    let _ = stream.set_write_timeout(Some(time));
    let _ = stream.write_all(target.hello.as_bytes());
    let line = match read_reply(&mut stream, target.deadline) {
        Ok(line) => line,
        Err(ended) => return ended,
    };
    match Reply::decode(&line) {
        Ok(Reply::Ready(ready)) if ready.identity == target.key => {
            // The preamble's timeouts must not outlive it: an MCP session may be quiet for hours.
            let cleared = stream
                .set_read_timeout(None)
                .and_then(|()| stream.set_write_timeout(None));
            match cleared {
                Ok(()) => Reached::Ready(stream, ready),
                Err(_) => Reached::Transient(Transient::Closed),
            }
        }
        Ok(Reply::Ready(ready)) => Reached::Refused(Refusal::ReadyForAnother {
            identity: ready.identity,
        }),
        Ok(Reply::Retiring) => Reached::Transient(Transient::Retiring),
        Ok(Reply::Busy { max_clients }) => Reached::Refused(Refusal::Busy { max_clients }),
        Ok(Reply::Mismatch { expected }) => Reached::Refused(Refusal::Mismatch { expected }),
        Err(error) => Reached::Refused(Refusal::BadReply(error.to_string())),
    }
}

/// Read the reply line, every read bounded by the time left, so the deadline bounds the whole
/// line and not each silence (S2b's note on `PreambleLineReader`).
fn read_reply(stream: &mut UnixStream, deadline: Instant) -> Result<Vec<u8>, Reached> {
    let mut reader = PreambleLineReader::new();
    loop {
        let Some(time) = left(deadline) else {
            return Err(Reached::Transient(Transient::NoReply));
        };
        // On macOS this fails with `EINVAL` once the peer has closed; a read then returns at
        // once, so the previous bound stands.
        let _ = stream.set_read_timeout(Some(time));
        match reader.read_line(stream) {
            Ok(line) => return Ok(line),
            Err(PreambleReadError::TooLong) => {
                return Err(Reached::Refused(Refusal::BadReply(
                    PreambleReadError::TooLong.to_string(),
                )))
            }
            Err(PreambleReadError::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            // End of stream, or a reset: on Linux a daemon that closes a connection with bytes
            // of this shim's unread is seen as a reset (S4's interface for S5a).
            Err(PreambleReadError::Eof { .. } | PreambleReadError::Io(_)) => {
                return Err(Reached::Transient(Transient::Closed))
            }
        }
    }
}

#[cfg(test)]
#[path = "connect_tests.rs"]
mod tests;
````

Create `src/mcp/shim/mod.rs`:

````rust
//! The shim: `prism-mcp --repo X --shared` (spec §3, §4.1). It holds no index. It finds the
//! shared daemon for its configuration, starting one if none is alive, and is then a byte pipe
//! between its client's stdio and the daemon's socket. If no daemon answers `READY` within one
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6, §10.
// The pieces are wired into a production caller by `run_shim` (Task 6), which removes this.
#![allow(dead_code)]

mod connect;
#[cfg(test)]
mod shim_support;
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t2.log" 2>&1
```

Expected: `test result: ok. 530 passed; 0 failed` (522 + 8), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/mod.rs src/mcp/daemon/mod.rs src/mcp/daemon/peer.rs src/mcp/shim/
git commit -m "feat(mcp): the shim's attempt to reach a daemon: peer check, HELLO, one reply line"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::shim::`, see the named test fail, then `git checkout -- src/mcp/shim/connect.rs`.

| Mutant | Edit in `connect.rs` | Fails |
|---|---|---|
| 2A | replace `let cleared = stream.set_read_timeout(None).and_then(\|()\| stream.set_write_timeout(None));` by `let cleared: io::Result<()> = Ok(());` | `a_ready_for_this_key_is_reached_and_the_stream_has_no_timeouts` |
| 2B | replace `let _ = stream.write_all(target.hello.as_bytes());` by `if stream.write_all(target.hello.as_bytes()).is_err() { return Reached::Transient(Transient::Closed); }` | `busy_and_mismatch_are_refused_even_when_the_hello_cannot_be_written` |
| 2C | replace `Ok(uid) => return Reached::Refused(Refusal::ForeignPeer { uid }),` by `Ok(_) => {}` | `a_peer_of_another_user_is_refused_before_anything_is_sent` |
| 2D | move the whole `match (target.peer_uid)(&stream) { … }` to just after `let _ = stream.write_all(target.hello.as_bytes());` | `a_peer_of_another_user_is_refused_before_anything_is_sent` |
| 2E | in `read_reply`, replace `let _ = stream.set_read_timeout(Some(time));` by `let _ = (stream.set_read_timeout(None), time);` | `a_daemon_that_does_not_finish_its_reply_is_no_reply_at_the_deadline` (gives up after `HANG`) |
| 2F | replace `if ready.identity == target.key` by `if ready.identity == ready.identity` | `a_ready_for_another_identity_is_refused` |
| 2G | replace `Ok(Reply::Retiring) => Reached::Transient(Transient::Retiring),` by `Ok(Reply::Retiring) => Reached::Refused(Refusal::BadReply("retiring".into())),` | `retiring_and_a_close_before_any_reply_are_transient` |
| 2H | in `read_reply`, make the end-of-stream arm return `Err(Reached::Refused(Refusal::BadReply("closed".into())))` | `retiring_and_a_close_before_any_reply_are_transient` |

### Task 3: The start protocol

**Files:**
- Modify: `src/mcp/daemon/files.rs`, `src/mcp/daemon/mod.rs`, `src/mcp/shim/mod.rs`, `src/mcp/shim/connect.rs`, `src/mcp/shim/shim_support.rs`
- Create: `src/mcp/shim/key_files.rs`, `src/mcp/shim/arbitrate.rs`, `src/mcp/shim/arbitrate_tests.rs`, `src/mcp/shim/arbitrate_race_tests.rs`

**Interfaces:**
- Consumes: Task 2's `reach`, `Target`, `Reached`, `Transient`, `Refusal`, `Point`, `Hook`, `PeerUid`; Task 1's marker format; `daemon::files::{open_private, open_runtime_dir, unlink_at, Create, FilesError}`, widened to `pub(in crate::mcp)` with `pub(super) mod files;`.
- Produces: `daemon::files::parse_build_marker(text: &str) -> Option<(u32, u64)>`; `connect::Point::{StartLockBusy, StartLockTaken, OwnerHeld, BeforeSpawn, AfterSpawn}`; in `shim::key_files`: `struct KeyDir<'a>` with `open(paths: &'a DaemonPaths) -> Result<KeyDir<'a>, FilesError>`, `start_lock(&self) -> Result<File, FilesError>`, `try_own(&self) -> Result<Option<File>, FilesError>`, `build_marker(&self) -> Option<(u32, u64)>`, `remove_stale(&self) -> Result<(), FilesError>`; in `shim::arbitrate`: `const POLL: Duration` (10 ms), `const BUILD_CRASH_BACKOFF: Duration` (60 s), `trait Starter { fn start(&mut self, dir: &KeyDir<'_>) -> io::Result<Box<dyn Started>>; }`, `trait Started: Send { fn pid(&self) -> u32; fn exit(&mut self) -> Option<Exit>; fn reap(self: Box<Self>); }`, `enum Exit { Success, Failure(String) }`, `struct Connected { stream: UnixStream, ready: Ready, started: Option<Box<dyn Started>> }`, `enum Owner { Unknown, Held, Free }`, `enum Failure { Files(FilesError), Refused(Refusal), StartLockTimeout { last }, NoReady { last, owner, started }, SpawnFailed(io::Error), DaemonExited { pid, exit }, RecentBuildCrash { pid, age_secs } }` (`Display`), `struct Arbiter<'a> { paths, key, hello, deadline, peer_uid, hook }` with `arbitrate(&self, starter: &mut dyn Starter) -> Result<Connected, Failure>`. Test support: `wait_until`, `Answer`, `Script`, `always`, `FakeDaemon` (`claim`, `listen`, `retire`), `FakeChild`, `ChildState`, `FakeStarter` (`new`, `claiming`, `exiting`, `start_fake`), `open_lock`, `Gate`, `unix_now`.

- [ ] **Step 1: Write the failing tests**

Create `src/mcp/shim/arbitrate_race_tests.rs`:

````rust
//! Tests for shims racing to start one daemon (spec §4.1 step 6, §4.6; the shim's halves of §9
//! tests 8 and 15). Each shim runs on a thread of the test with a starter of its own, as each
//! shim process spawns its own daemon; one is paused at a boundary of the start protocol while
//! the others go on.
use super::super::shim_support::{
    always, hello_line, key, runtime, this_user, wait_until, Answer, ChildState, FakeChild,
    FakeDaemon, FakeStarter, Gate,
};
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const LONG: Duration = Duration::from_secs(60);

/// Run a shim for `paths` on a thread with `starter`, its hook `hook`; the thread returns the pid
/// of the daemon it reached and how many daemons it started.
fn shim(
    paths: &DaemonPaths,
    mut starter: FakeStarter,
    hook: Option<Hook>,
) -> std::thread::JoinHandle<(u32, u32)> {
    let paths = paths.clone();
    std::thread::spawn(move || {
        let hello = hello_line(key());
        let shim = Arbiter {
            paths: &paths,
            key: key(),
            hello: &hello,
            deadline: Instant::now() + LONG,
            peer_uid: this_user,
            hook,
        };
        let pid = match shim.arbitrate(&mut starter) {
            Ok(connected) => connected.ready.pid,
            Err(failure) => panic!("no daemon reached: {failure}"),
        };
        (pid, starter.starts.load(Ordering::SeqCst))
    })
}

fn children() -> Arc<Mutex<Vec<Arc<Mutex<ChildState>>>>> {
    Arc::new(Mutex::new(Vec::new()))
}

/// How many of `children` own the key and serve it.
fn serving(children: &Mutex<Vec<Arc<Mutex<ChildState>>>>) -> usize {
    children
        .lock()
        .expect("children")
        .iter()
        .filter(|child| child.lock().expect("child").daemon.is_some())
        .count()
}

/// Spec §9 test 8, the shim's half: one shim is paused holding the start lock, before it spawns,
/// and after it spawns; a second shim started meanwhile waits for the lock (or, once a daemon
/// answers, is admitted by it). When the first goes on, both are admitted by the same daemon
/// and exactly one daemon was started between them. Rules out a second shim that starts its own
/// daemon while the first holds the start lock.
#[test]
fn shims_racing_to_start_a_daemon_start_exactly_one() {
    for boundary in [Point::StartLockTaken, Point::BeforeSpawn, Point::AfterSpawn] {
        let (_base, paths) = runtime();
        let started = children();
        let gate = Gate::new();
        let paused = Arc::new(AtomicBool::new(false));
        let first = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            Some({
                let (gate, paused) = (Arc::clone(&gate), Arc::clone(&paused));
                Arc::new(move |point| {
                    if point == boundary && !paused.swap(true, Ordering::SeqCst) {
                        gate.pass();
                    }
                })
            }),
        );
        wait_until("the first shim to reach the boundary", || {
            paused.load(Ordering::SeqCst)
        });
        let waiting = Arc::new(AtomicBool::new(false));
        let second = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            Some({
                let waiting = Arc::clone(&waiting);
                Arc::new(move |point| {
                    if point == Point::StartLockBusy {
                        waiting.store(true, Ordering::SeqCst);
                    }
                })
            }),
        );
        if boundary != Point::AfterSpawn {
            wait_until("the second shim to wait for the start lock", || {
                waiting.load(Ordering::SeqCst)
            });
        }
        gate.open();
        let (first_pid, first_starts) = first.join().expect("the first shim");
        let (second_pid, second_starts) = second.join().expect("the second shim");
        assert_eq!(first_pid, second_pid, "{boundary:?}");
        assert_eq!(first_starts + second_starts, 1, "{boundary:?}");
        assert_eq!(serving(&started), 1, "{boundary:?}");
    }
}

/// Spec §9 test 15, the shim's half, and §4.6 "Starter crashes mid-start": a shim dies right
/// after spawning a daemon, before that daemon takes the owner lock, while another shim starts.
/// Whether the orphan takes the key first or the second shim's own daemon does, exactly one
/// daemon serves and two later shims are both admitted by it.
#[test]
fn a_starter_that_dies_after_spawning_leaves_exactly_one_daemon() {
    for orphan_wins in [false, true] {
        let (_base, paths) = runtime();
        let release = Gate::new();
        let orphan = Arc::new(Mutex::new(ChildState::default()));
        // The dying shim's daemon takes the key only when `release` opens.
        let mut dying_starter = FakeStarter::new({
            let (paths, release, orphan) =
                (paths.clone(), Arc::clone(&release), Arc::clone(&orphan));
            move |_| {
                let (paths, release, state) =
                    (paths.clone(), Arc::clone(&release), Arc::clone(&orphan));
                std::thread::spawn(move || {
                    release.pass();
                    let mut child = state.lock().expect("orphan");
                    match FakeDaemon::claim(&paths, 2000, always(Answer::Ready)) {
                        Some(daemon) => child.daemon = Some(daemon),
                        None => child.exit = Some(Exit::Success),
                    }
                });
                Ok(Box::new(FakeChild {
                    pid: 2000,
                    state: Arc::clone(&orphan),
                }) as Box<dyn Started>)
            }
        });
        let hello = hello_line(key());
        let mut dying = Arbiter {
            paths: &paths,
            key: key(),
            hello: &hello,
            deadline: Instant::now() + LONG,
            peer_uid: this_user,
            hook: None,
        };
        dying.hook = Some(Arc::new(|point| {
            if point == Point::AfterSpawn {
                panic!("the starter dies here, on purpose");
            }
        }));
        let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dying
                .arbitrate(&mut dying_starter)
                .map(|connected| connected.ready.pid)
        }));
        assert!(died.is_err(), "the starter died");
        if orphan_wins {
            release.open();
            wait_until("the orphan to own the key", || {
                orphan.lock().expect("orphan").daemon.is_some()
            });
        }

        let started = children();
        let (second, _) = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            None,
        )
        .join()
        .expect("the second shim");
        if !orphan_wins {
            release.open();
            wait_until("the orphan to find the key taken and end", || {
                orphan.lock().expect("orphan").exit == Some(Exit::Success)
            });
        }
        let (third, third_starts) = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            None,
        )
        .join()
        .expect("the third shim");

        let survivor = if orphan_wins { 2000 } else { 1000 };
        assert_eq!(
            (second, third),
            (survivor, survivor),
            "orphan wins: {orphan_wins}"
        );
        assert_eq!(third_starts, 0);
        let orphan_serves = usize::from(orphan.lock().expect("orphan").daemon.is_some());
        assert_eq!(
            orphan_serves + serving(&started),
            1,
            "orphan wins: {orphan_wins}"
        );
    }
}

/// Spec §4.1 step 6 and the module doc: while a daemon this shim started has neither answered nor
/// ended, the shim waits for it and does not look at the owner lock (looking takes the lock for a
/// moment, and a daemon claiming the key in that moment leaves believing another owns it). Here
/// the started daemon takes the key only when the test lets it; until then exactly one daemon has
/// been started. The pause before the gate opens orders nothing a correct shim depends on; it
/// gives a shim that looks again the time to start a second daemon.
#[test]
fn a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for() {
    let (_base, paths) = runtime();
    let release = Gate::new();
    let starter = FakeStarter::new({
        let (paths, release) = (paths.clone(), Arc::clone(&release));
        move |n| {
            let state = Arc::new(Mutex::new(ChildState::default()));
            let (paths, release, child) = (paths.clone(), Arc::clone(&release), Arc::clone(&state));
            std::thread::spawn(move || {
                release.pass();
                let mut child = child.lock().expect("child");
                match FakeDaemon::claim(&paths, 1000 + n, always(Answer::Ready)) {
                    Some(daemon) => child.daemon = Some(daemon),
                    None => child.exit = Some(Exit::Success),
                }
            });
            Ok(Box::new(FakeChild {
                pid: 1000 + n,
                state,
            }) as Box<dyn Started>)
        }
    });
    let starts = Arc::clone(&starter.starts);
    let running = shim(&paths, starter, None);
    wait_until("the shim to start a daemon", || {
        starts.load(Ordering::SeqCst) == 1
    });
    std::thread::sleep(Duration::from_millis(200));
    release.open();
    let (pid, total) = running.join().expect("the shim");
    assert_eq!((pid, total), (1000, 1));
}
````

Create `src/mcp/shim/arbitrate_tests.rs`:

````rust
//! Tests for finding and starting the daemon (spec §4.1 steps 4–7, §4.6, §10; the shim's halves
//! of §9 tests 3, 4, 5, 10 and 13; the restart bound of S3b's SI-10). Daemons are fakes on
//! threads of the test that take the real owner lock on a descriptor of their own, as a daemon
//! process does; a starter that the test controls stands in for spawning one.
use super::super::shim_support::{
    always, another_user, hello_line, key, open_lock, runtime, this_user, unix_now, Answer,
    ChildState, FakeDaemon, FakeStarter,
};
use super::*;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const LONG: Duration = Duration::from_secs(60);

fn arbiter<'a>(paths: &'a DaemonPaths, hello: &'a str, within: Duration) -> Arbiter<'a> {
    Arbiter {
        paths,
        key: key(),
        hello,
        deadline: Instant::now() + within,
        peer_uid: this_user,
        hook: None,
    }
}

type Children = Arc<Mutex<Vec<Arc<Mutex<ChildState>>>>>;

fn children() -> Children {
    Arc::new(Mutex::new(Vec::new()))
}

/// A starter whose every start would be a mistake.
fn must_not_start() -> FakeStarter {
    FakeStarter::exiting(Exit::Failure("this test starts no daemon".into()))
}

fn inode(path: &Path) -> u64 {
    std::fs::symlink_metadata(path).expect("lstat").ino()
}

fn pid_of(result: Result<Connected, Failure>) -> u32 {
    match result {
        Ok(connected) => connected.ready.pid,
        Err(failure) => panic!("no daemon reached: {failure}"),
    }
}

/// Whether some process (this test, playing one) could take the owner lock now.
fn owner_is_free(paths: &DaemonPaths) -> bool {
    open_lock(paths.owner_lock()).try_lock().is_ok()
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).expect("write a key file");
}

/// Spec §4.1 step 5: a live daemon is reached by the first attempt, before the start lock is
/// even opened, and nothing is started.
#[test]
fn a_live_daemon_is_reached_without_the_start_lock_or_a_start() {
    let (_base, paths) = runtime();
    let _daemon = FakeDaemon::claim(&paths, 7, always(Answer::Ready)).expect("the key");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert_eq!(
        pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter)),
        7
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    assert!(
        !paths.start_lock().exists(),
        "the start lock was not opened"
    );
}

/// Spec §4.1 step 6: with no daemon, the shim starts one under the start lock, is admitted by
/// it, and hands back the started daemon; the start lock is free again afterwards.
#[test]
fn with_no_daemon_one_is_started_and_reached() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    let connected = arbiter(&paths, &hello, LONG)
        .arbitrate(&mut starter)
        .map_err(|failure| failure.to_string())
        .expect("reached");
    assert_eq!(connected.ready.pid, 1000);
    assert_eq!(
        connected.started.as_ref().map(|child| child.pid()),
        Some(1000)
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
    drop(connected);
    assert!(open_lock(paths.start_lock()).try_lock().is_ok());
}

/// Spec §9 test 3, the shim's half, and §4.1 step 6: a socket and a pid file left by a dead
/// daemon (no owner lock held) are removed while the shim holds the owner lock, and the lock is
/// released before the daemon is started. Rules out a shim that leaves them, and one that spawns
/// still holding the owner lock (its child could never own the key).
#[test]
fn a_stale_socket_and_pid_file_are_removed_under_the_owner_lock_before_the_start() {
    let (_base, paths) = runtime();
    drop(UnixListener::bind(paths.socket()).expect("a stale socket"));
    write(paths.pid_file(), "4242\n17\nbuild\n");
    let seen = Arc::new(Mutex::new(None));
    let hello = hello_line(key());
    let mut shim = arbiter(&paths, &hello, LONG);
    shim.hook = Some({
        let (paths, seen) = (paths.clone(), Arc::clone(&seen));
        Arc::new(move |point| {
            if point == Point::BeforeSpawn {
                *seen.lock().expect("seen") = Some((
                    paths.socket().exists(),
                    paths.pid_file().exists(),
                    owner_is_free(&paths),
                ));
            }
        })
    });
    let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    assert_eq!(pid_of(shim.arbitrate(&mut starter)), 1000);
    assert_eq!(
        *seen.lock().expect("seen"),
        Some((false, false, true)),
        "(socket there, pid file there, owner lock free) just before the start"
    );
}

/// Spec §9 test 4, the shim's half, and §4.4: a live daemon that accepts and never answers is
/// reported at the deadline and never replaced: its socket and pid file are left alone and
/// nothing is started. Also when it closes every connection instead (the owner lock is then
/// looked at, found held, and still nothing is touched).
#[test]
fn a_live_but_wedged_owner_is_reported_and_its_files_are_left_alone() {
    for answer in [Answer::Silent, Answer::Close] {
        let (_base, paths) = runtime();
        let _daemon = FakeDaemon::claim(&paths, 4242, always(answer)).expect("the key");
        write(paths.pid_file(), "4242\n17\nbuild\n");
        let socket = inode(paths.socket());
        let hello = hello_line(key());
        let mut starter = must_not_start();
        let result = arbiter(&paths, &hello, Duration::from_millis(500)).arbitrate(&mut starter);
        match (answer, result) {
            (
                Answer::Silent,
                Err(Failure::NoReady {
                    last: Transient::NoReply,
                    ..
                }),
            ) => {}
            (
                Answer::Close,
                Err(Failure::NoReady {
                    owner: Owner::Held, ..
                }),
            ) => {}
            (answer, other) => panic!("{answer:?}: {:?}", other.map(|c| c.ready.pid)),
        }
        assert_eq!(inode(paths.socket()), socket);
        assert_eq!(
            std::fs::read_to_string(paths.pid_file()).expect("pid file"),
            "4242\n17\nbuild\n"
        );
        assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    }
}

/// The retirement window (S4's interface): a daemon removes its socket a moment before it
/// releases the owner lock. A shim that arrives in between finds no socket and the lock held; it
/// waits, and once the lock is free it starts a daemon. Rules out a shim that gives up on a held
/// lock, and one that never looks again.
#[test]
fn a_key_whose_owner_is_releasing_it_is_started_once_the_lock_is_free() {
    let (_base, paths) = runtime();
    let retiring = FakeDaemon::claim(&paths, 7, always(Answer::Ready)).expect("the key");
    std::fs::remove_file(paths.socket()).expect("the socket is gone first");
    let retiring = Arc::new(Mutex::new(Some(retiring)));
    let hello = hello_line(key());
    let mut shim = arbiter(&paths, &hello, LONG);
    shim.hook = Some({
        let retiring = Arc::clone(&retiring);
        Arc::new(move |point| {
            if point == Point::OwnerHeld {
                // The retiring daemon releases its lock (and ends) after the shim saw it held.
                drop(retiring.lock().expect("retiring").take());
            }
        })
    });
    let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    assert_eq!(pid_of(shim.arbitrate(&mut starter)), 1000);
    assert!(
        retiring.lock().expect("retiring").is_none(),
        "the lock was seen held"
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
}

/// Spec §9 test 13, the shim's half: a shim paused after `connect` and before `HELLO` while the
/// daemon retires is answered `RETIRING`, or sees its connection end, re-enters arbitration and
/// is admitted by a fresh daemon within its original deadline. Rules out failing closed on
/// either.
#[test]
fn a_shim_caught_by_a_retirement_is_admitted_by_a_fresh_daemon() {
    for answer in [Answer::Retiring, Answer::Close] {
        let (_base, paths) = runtime();
        let old = FakeDaemon::claim(&paths, 7, always(answer)).expect("the key");
        let old = Arc::new(Mutex::new(Some(old)));
        let hello = hello_line(key());
        let mut shim = arbiter(&paths, &hello, LONG);
        shim.hook = Some({
            let old = Arc::clone(&old);
            Arc::new(move |point| {
                if point == Point::AfterConnect {
                    // Paused after connect: the daemon retires (socket removed, lock released);
                    // the connection it accepted is still answered by its script.
                    if let Some(old) = old.lock().expect("old").take() {
                        old.retire();
                    }
                }
            })
        });
        let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
        assert_eq!(pid_of(shim.arbitrate(&mut starter)), 1000, "{answer:?}");
        assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
    }
}

/// S4: a daemon that finds the key owned exits 0 silently. A daemon the shim started that ends
/// that way, before `READY`, lost the key; the shim looks at the owner lock again and, finding it
/// free, starts another.
#[test]
fn a_started_daemon_that_lost_the_key_is_followed_by_another_start() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut claiming = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    let mut starter = FakeStarter::new(move |n| {
        if n == 0 {
            FakeStarter::exiting(Exit::Success).start_fake()
        } else {
            claiming.start_fake()
        }
    });
    assert_eq!(
        pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter)),
        1000
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 2);
}

/// Spec §10 "Daemon spawn fails": a daemon the shim started that ends unsuccessfully before
/// `READY` ends the start at once, naming it; it is not started again. Rules out retrying such a
/// daemon to the deadline.
#[test]
fn a_started_daemon_that_fails_before_ready_ends_the_start() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut starter = FakeStarter::exiting(Exit::Failure("exit status: 1".into()));
    match arbiter(&paths, &hello, Duration::from_secs(5)).arbitrate(&mut starter) {
        Err(Failure::DaemonExited { pid: 1000, exit }) => assert_eq!(exit, "exit status: 1"),
        other => panic!("{:?}", other.map(|c| c.ready.pid)),
    }
    assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
}

/// Spec §10: a daemon that cannot be spawned fails the start closed.
#[test]
fn a_spawn_error_fails_closed() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut starter = FakeStarter::new(|_| Err(std::io::Error::other("no such program")));
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::SpawnFailed(_))
    ));
}

/// Spec §10 "Start lock not acquired by the deadline": another shim holds the start lock until
/// the deadline; this one fails closed and starts nothing.
#[test]
fn a_start_lock_held_until_the_deadline_fails_closed() {
    let (_base, paths) = runtime();
    let held = open_lock(paths.start_lock());
    held.try_lock().expect("the test holds the start lock");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, Duration::from_millis(300)).arbitrate(&mut starter),
        Err(Failure::StartLockTimeout { .. })
    ));
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    drop(held);
}

/// S3b's SI-10: a key whose last daemon died during its index build less than a minute ago (its
/// build marker left, the owner lock free) is not restarted; the shim fails closed and leaves the
/// dead daemon's files as they are. Rules out a shim without the bound.
#[test]
fn a_build_that_died_moments_ago_is_not_restarted() {
    let (_base, paths) = runtime();
    drop(UnixListener::bind(paths.socket()).expect("a stale socket"));
    write(paths.pid_file(), "4242\n17\nbuild\n");
    let marker = format!("4242\n{}\n", unix_now() - 50);
    write(paths.build_marker(), &marker);
    let hello = hello_line(key());
    let mut starter = must_not_start();
    match arbiter(&paths, &hello, LONG).arbitrate(&mut starter) {
        Err(Failure::RecentBuildCrash {
            pid: 4242,
            age_secs,
        }) => {
            assert!((50..60).contains(&age_secs), "{age_secs}")
        }
        other => panic!("{:?}", other.map(|c| c.ready.pid)),
    }
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    assert!(paths.socket().exists());
    assert!(paths.pid_file().exists());
    assert_eq!(
        std::fs::read_to_string(paths.build_marker()).expect("marker"),
        marker
    );
    assert!(owner_is_free(&paths), "the owner lock was released");
}

/// The bound holds a key closed for a minute after the dead build began, no longer; a marker
/// whose start time is in the future (a clock set back), or that is only partly written, does
/// not hold it at all. Each is removed with the other stale files and a daemon is started.
#[test]
fn an_old_future_or_partial_build_marker_does_not_hold_the_key() {
    for marker in [
        format!("4242\n{}\n", unix_now() - 70),
        format!("4242\n{}\n", unix_now() + 3600),
        "42".to_string(),
    ] {
        let (_base, paths) = runtime();
        write(paths.build_marker(), &marker);
        let hello = hello_line(key());
        let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
        assert_eq!(
            pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter)),
            1000,
            "{marker:?}"
        );
        assert!(!paths.build_marker().exists(), "{marker:?}");
    }
}

/// Spec §9 test 10 (in process): a daemon that answers `MISMATCH` fails the start closed at once;
/// nothing is started in its place. And the peer check on the daemon at the socket.
#[test]
fn a_mismatch_or_a_peer_of_another_user_fails_closed_without_a_start() {
    let (_base, paths) = runtime();
    let _daemon = FakeDaemon::claim(&paths, 7, always(Answer::Mismatch)).expect("the key");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::Refused(Refusal::Mismatch { .. }))
    ));
    drop(_daemon);
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    let _daemon = FakeDaemon::claim(&paths, 7, always(Answer::Ready)).expect("the key");
    let mut shim = arbiter(&paths, &hello, LONG);
    shim.peer_uid = another_user;
    assert!(matches!(
        shim.arbitrate(&mut starter),
        Err(Failure::Refused(Refusal::ForeignPeer { .. }))
    ));
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
}

/// Spec §9 test 5 (in process): a runtime directory that fails validation (here a symlink to a
/// directory of this user's) fails the start closed before any file is opened or created in it.
#[test]
fn an_invalid_runtime_directory_fails_closed_before_any_file_is_made() {
    let (base, paths) = runtime();
    let elsewhere = base.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).expect("a directory");
    std::fs::remove_dir(paths.runtime_dir()).expect("remove the runtime directory");
    std::os::unix::fs::symlink(&elsewhere, paths.runtime_dir()).expect("a symlink in its place");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::Files(FilesError::RuntimeDir(_)))
    ));
    assert_eq!(
        std::fs::read_dir(&elsewhere).expect("read").count(),
        0,
        "nothing was created through the symlink"
    );
}
````

In `src/mcp/shim/mod.rs`, replace this:

````rust
// The pieces are wired into a production caller by `run_shim` (Task 6), which removes this.
#![allow(dead_code)]

mod connect;
#[cfg(test)]
mod shim_support;
````

with this:

````rust
// The pieces are wired into a production caller by `run_shim` (Task 6), which removes this.
#![allow(dead_code)]

mod arbitrate;
mod connect;
mod key_files;
#[cfg(test)]
mod shim_support;
````

In `src/mcp/shim/shim_support.rs`, replace this:

````rust
//! and a stand-in for one daemon connection.
//!
//! No test here sleeps to order two events: a fake daemon answers when it has read what it waits
//! for, and a test gives up on a result after [`HANG`].
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
````

with this:

````rust
//! and a stand-in for one daemon connection.
//!
//! No test here sleeps to order two events: a fake daemon answers when it has read what it waits
//! for, a test waits for a condition that must become true (`wait_until`), and gives up after
//! [`HANG`].
use super::arbitrate::{Exit, Started, Starter};
use super::key_files::KeyDir;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
````

In `src/mcp/shim/shim_support.rs`, replace this:

````rust
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::{BuildState, Hello, Ready, Reply};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// How long a test waits for something that must happen before it calls the run hung.
pub(super) const HANG: Duration = Duration::from_secs(30);
````

with this:

````rust
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::{BuildState, Hello, Ready, Reply};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::fs::{File, TryLockError};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How long a test waits for something that must happen before it calls the run hung.
pub(super) const HANG: Duration = Duration::from_secs(30);
````

In `src/mcp/shim/shim_support.rs`, replace this:

````rust
    let _ = stream.read_to_end(&mut bytes);
    bytes
}
````

with this:

````rust
    let _ = stream.read_to_end(&mut bytes);
    bytes
}

/// Poll `condition` until it holds; panic, naming `what`, after [`HANG`].
pub(super) fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let give_up = Instant::now() + HANG;
    while !condition() {
        assert!(
            Instant::now() < give_up,
            "gave up after {HANG:?} waiting for: {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// What a fake daemon does with a connection, after reading its `HELLO`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// `READY` with the daemon's pid, then hold the connection until the shim closes it.
    Ready,
    /// `RETIRING`, then close.
    Retiring,
    /// Close without a reply.
    Close,
    /// Hold the connection and never answer.
    Silent,
    /// `MISMATCH`, naming [`other_key`].
    Mismatch,
}

/// What a fake daemon answers each connection, by its number (0 first).
pub(super) type Script = Arc<dyn Fn(u32) -> Answer + Send + Sync>;

pub(super) fn always(answer: Answer) -> Script {
    Arc::new(move |_| answer)
}

/// A daemon on a thread of the test: it owns the key's owner lock (taken as a daemon process
/// takes it, on a descriptor of its own) and answers on the key's socket by its script. Dropping
/// it stops it and releases the lock; it leaves its socket file, as a daemon that died does.
pub(super) struct FakeDaemon {
    owner: Option<File>,
    stop: Arc<AtomicBool>,
    socket: PathBuf,
    thread: Option<JoinHandle<()>>,
}

impl FakeDaemon {
    /// Take the owner lock of `paths` and serve: `None`, touching nothing, when another holds it.
    pub(super) fn claim(paths: &DaemonPaths, pid: u32, script: Script) -> Option<Self> {
        let owner = open_lock(paths.owner_lock());
        match owner.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return None,
            Err(TryLockError::Error(error)) => panic!("lock: {error}"),
        }
        let _ = std::fs::remove_file(paths.socket());
        let mut daemon = Self::listen(paths, pid, script);
        daemon.owner = Some(owner);
        Some(daemon)
    }

    /// Serve on the key's socket without the owner lock (a listener nobody owns).
    pub(super) fn listen(paths: &DaemonPaths, pid: u32, script: Script) -> Self {
        let listener = UnixListener::bind(paths.socket()).expect("bind the fake daemon");
        listener.set_nonblocking(true).expect("nonblocking");
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut accepted = 0;
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let answer = script(accepted);
                            accepted += 1;
                            let stop = Arc::clone(&stop);
                            std::thread::spawn(move || serve(stream, pid, answer, &stop));
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(2)),
                    }
                }
            })
        };
        Self {
            owner: None,
            stop,
            socket: paths.socket().to_path_buf(),
            thread: Some(thread),
        }
    }

    /// Retire as a daemon does: remove the socket, stop, release the owner lock.
    pub(super) fn retire(mut self) {
        let _ = std::fs::remove_file(&self.socket);
        self.stop_thread();
    }

    fn stop_thread(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.owner = None;
    }
}

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        self.stop_thread();
    }
}

fn serve(mut stream: UnixStream, pid: u32, answer: Answer, stop: &AtomicBool) {
    let _ = stream.set_nonblocking(false);
    let hello = read_line(&stream);
    let identity = Hello::decode(hello.trim_end_matches('\n').as_bytes())
        .map(|hello| hello.identity)
        .unwrap_or_else(|_| key());
    match answer {
        Answer::Ready => {
            let _ = stream.write_all(ready_line(identity, pid).as_bytes());
            hold(&stream, stop);
        }
        Answer::Retiring => {
            let _ = stream.write_all(Reply::Retiring.encode().as_bytes());
        }
        Answer::Close => {}
        Answer::Silent => hold(&stream, stop),
        Answer::Mismatch => {
            let reply = Reply::Mismatch {
                expected: other_key(),
            };
            let _ = stream.write_all(reply.encode().as_bytes());
        }
    }
}

/// Keep a connection until its peer closes it or the daemon stops.
fn hold(mut stream: &UnixStream, stop: &AtomicBool) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
    let mut buf = [0u8; 256];
    while !stop.load(Ordering::SeqCst) {
        match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return,
        }
    }
}

/// A started fake daemon process: it claims the key when started (or when its gate opens), and
/// exits successfully if the key was taken, as a daemon process does.
pub(super) struct FakeChild {
    pub(super) pid: u32,
    pub(super) state: Arc<Mutex<ChildState>>,
}

#[derive(Default)]
pub(super) struct ChildState {
    pub(super) exit: Option<Exit>,
    pub(super) daemon: Option<FakeDaemon>,
}

impl Started for FakeChild {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn exit(&mut self) -> Option<Exit> {
        self.state.lock().expect("child state").exit.clone()
    }

    fn reap(self: Box<Self>) {}
}

/// Starts fake daemons: the n-th start (0 first) is `make(n)`. Counts its starts.
pub(super) struct FakeStarter {
    pub(super) starts: Arc<AtomicU32>,
    make: Box<dyn FnMut(u32) -> std::io::Result<Box<dyn Started>> + Send>,
}

impl FakeStarter {
    pub(super) fn new(
        make: impl FnMut(u32) -> std::io::Result<Box<dyn Started>> + Send + 'static,
    ) -> Self {
        Self {
            starts: Arc::new(AtomicU32::new(0)),
            make: Box::new(make),
        }
    }

    /// Each start claims the key at once and serves `script`; the children's states are
    /// collected in `children` for the test to inspect.
    pub(super) fn claiming(
        paths: &DaemonPaths,
        script: Script,
        children: Arc<Mutex<Vec<Arc<Mutex<ChildState>>>>>,
    ) -> Self {
        let paths = paths.clone();
        Self::new(move |n| {
            let pid = 1000 + n;
            let state = Arc::new(Mutex::new(ChildState::default()));
            {
                let mut child = state.lock().expect("child state");
                match FakeDaemon::claim(&paths, pid, Arc::clone(&script)) {
                    Some(daemon) => child.daemon = Some(daemon),
                    None => child.exit = Some(Exit::Success),
                }
            }
            children.lock().expect("children").push(Arc::clone(&state));
            Ok(Box::new(FakeChild { pid, state }))
        })
    }

    /// Each start returns a child that has already ended with `exit` and served nothing.
    pub(super) fn exiting(exit: Exit) -> Self {
        Self::new(move |n| {
            let state = ChildState {
                exit: Some(exit.clone()),
                daemon: None,
            };
            Ok(Box::new(FakeChild {
                pid: 1000 + n,
                state: Arc::new(Mutex::new(state)),
            }))
        })
    }
}

impl FakeStarter {
    /// Start one fake daemon, as [`Starter::start`] does.
    pub(super) fn start_fake(&mut self) -> std::io::Result<Box<dyn Started>> {
        let n = self.starts.fetch_add(1, Ordering::SeqCst);
        (self.make)(n)
    }
}

impl Starter for FakeStarter {
    fn start(&mut self, _dir: &KeyDir<'_>) -> std::io::Result<Box<dyn Started>> {
        self.start_fake()
    }
}

/// Open (creating) a lock file on a descriptor of its own, as another process would. On macOS a
/// create that races another process's create of the same name can fail with `ENOENT` (S4's
/// D53); it is retried.
pub(super) fn open_lock(path: &std::path::Path) -> File {
    let mut tries = 0;
    loop {
        let opened = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path);
        match opened {
            Ok(file) => return file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && tries < 100 => {
                tries += 1;
            }
            Err(error) => panic!("open {}: {error}", path.display()),
        }
    }
}

/// A gate a thread waits at until the test opens it.
#[derive(Default)]
pub(super) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Wait until the gate is open; panic after [`HANG`].
    pub(super) fn pass(&self) {
        let open = self.open.lock().expect("gate");
        let (open, _) = self
            .changed
            .wait_timeout_while(open, HANG, |open| !*open)
            .expect("gate");
        assert!(*open, "the gate was never opened");
    }

    pub(super) fn open(&self) {
        *self.open.lock().expect("gate") = true;
        self.changed.notify_all();
    }
}

/// The current time in seconds since the Unix epoch, as a build marker writes it.
pub(super) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t3-red.log" 2>&1
```

Expected in the log: `error[E0583]: file not found for module `arbitrate`` and `error[E0583]: file not found for module `key_files``.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/files.rs`, replace this:

````rust
//! through the descriptor, and only while the entry is still the one this daemon bound. Like
//! the directory checks, this is hardening; the isolation guarantee between users is the peer
//! uid check on every connection.
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::{DaemonPaths, SocketPathTooLong};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir, RuntimeDirError};
````

with this:

````rust
//! through the descriptor, and only while the entry is still the one this daemon bound. Like
//! the directory checks, this is hardening; the isolation guarantee between users is the peer
//! uid check on every connection.
//!
//! The shim (S5a) opens the same files through the same functions: it takes the start lock,
//! and, while it holds the owner lock of a key no daemon owns, removes what a dead daemon left
//! (spec §4.1 step 6).
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::{DaemonPaths, SocketPathTooLong};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir, RuntimeDirError};
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
/// Why a daemon could not take or use its files. Each ends the daemon's start with a message;
/// none is retried.
#[derive(Debug)]
pub(super) enum FilesError {
    RuntimeDir(RuntimeDirError),
    SocketPath(SocketPathTooLong),
    /// The filesystem refused `op` on `path`.
````

with this:

````rust
/// Why a daemon could not take or use its files. Each ends the daemon's start with a message;
/// none is retried.
#[derive(Debug)]
pub(in crate::mcp) enum FilesError {
    RuntimeDir(RuntimeDirError),
    SocketPath(SocketPathTooLong),
    /// The filesystem refused `op` on `path`.
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
    }
}

/// The last component of `path`, as a C string, for the `*at` calls.
fn entry_name(path: &Path) -> io::Result<CString> {
    let name = path
````

with this:

````rust
    }
}

/// The pid and start time a build marker holds, when it holds exactly what
/// [`BuildMarker::set`] writes. A marker caught half-written reads as `None`.
pub(in crate::mcp) fn parse_build_marker(text: &str) -> Option<(u32, u64)> {
    let mut lines = text.strip_suffix('\n')?.split('\n');
    let pid = lines.next()?.parse().ok()?;
    let started = lines.next()?.parse().ok()?;
    lines.next().is_none().then_some((pid, started))
}

/// The last component of `path`, as a C string, for the `*at` calls.
fn entry_name(path: &Path) -> io::Result<CString> {
    let name = path
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
/// Open the runtime directory itself, not a symlink in its place, and check that what was
/// opened is a directory this user owns with mode 0700 (the validation's facts, now about the
/// descriptor every later file operation goes through).
fn open_runtime_dir(path: &Path) -> Result<OwnedFd, FilesError> {
    let io_error = |source| FilesError::Io {
        op: "open",
        path: path.to_path_buf(),
````

with this:

````rust
/// Open the runtime directory itself, not a symlink in its place, and check that what was
/// opened is a directory this user owns with mode 0700 (the validation's facts, now about the
/// descriptor every later file operation goes through).
pub(in crate::mcp) fn open_runtime_dir(path: &Path) -> Result<OwnedFd, FilesError> {
    let io_error = |source| FilesError::Io {
        op: "open",
        path: path.to_path_buf(),
````

In `src/mcp/daemon/files.rs`, replace this:

````rust

/// Whether `open_private` creates the file when it is missing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Create {
    Yes,
    No,
}
````

with this:

````rust

/// Whether `open_private` creates the file when it is missing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::mcp) enum Create {
    Yes,
    No,
}
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
/// plain `O_CREAT` is not used: on macOS it fails now and then with `ENOENT` when another
/// process creates the same name at the same moment, which is exactly what daemons racing for
/// one key do (D53; measured in validation, about 1 in 2,000 eight-way races).
fn open_private(
    dir: &OwnedFd,
    path: &Path,
    flags: libc::c_int,
````

with this:

````rust
/// plain `O_CREAT` is not used: on macOS it fails now and then with `ENOENT` when another
/// process creates the same name at the same moment, which is exactly what daemons racing for
/// one key do (D53; measured in validation, about 1 in 2,000 eight-way races).
pub(in crate::mcp) fn open_private(
    dir: &OwnedFd,
    path: &Path,
    flags: libc::c_int,
````

In `src/mcp/daemon/files.rs`, replace this:

````rust
}

/// `unlinkat` of `path`'s name in `dir`.
fn unlink_at(dir: &OwnedFd, path: &Path) -> io::Result<()> {
    let name = entry_name(path)?;
    // SAFETY: `dir` is an open directory descriptor and `name` a valid NUL-terminated string,
    // both alive for the call.
````

with this:

````rust
}

/// `unlinkat` of `path`'s name in `dir`.
pub(in crate::mcp) fn unlink_at(dir: &OwnedFd, path: &Path) -> io::Result<()> {
    let name = entry_name(path)?;
    // SAFETY: `dir` is an open directory descriptor and `name` a valid NUL-terminated string,
    // both alive for the call.
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod control;
mod coordinator;
mod fatal;
mod files;
mod log;
pub(super) mod peer;
mod run;
````

with this:

````rust
mod control;
mod coordinator;
mod fatal;
pub(super) mod files;
mod log;
pub(super) mod peer;
mod run;
````

Create `src/mcp/shim/arbitrate.rs`:

````rust
//! Finding the daemon for a key, and starting it when none is alive (spec §4.1 steps 4–7, §4.6),
//! all before one deadline.
//!
//! 1. Try the socket (`connect::reach`). `READY` for this key ends here.
//! 2. On anything transient, take the start lock (`<key16>.start`, retried until the deadline).
//!    Under it, try again; another shim may have started the daemon meanwhile.
//! 3. Still nothing: look at the owner lock. Held: a daemon is alive (starting, or wedged, or
//!    draining); try again shortly, never touching its files. Free: no daemon owns the key;
//!    holding the owner lock, remove the stale socket, pid file and build marker, release it, and
//!    spawn a daemon, which takes the owner lock itself. While that daemon is still running and
//!    has not answered, the owner lock is not looked at again (looking takes it for a moment, and
//!    a daemon claiming the key in that moment would leave believing another owns it).
//! 4. A daemon this shim spawned that exits unsuccessfully before `READY` ends the start: it will
//!    not do better on a second try. One that exits successfully lost the key to another daemon
//!    (S4: "exit 0 silently"); step 3 runs again.
//!
//! The build marker bounds restarts (S3b's SI-10): a key whose last daemon died during its index
//! build less than [`BUILD_CRASH_BACKOFF`] ago is not restarted, and the shim fails closed naming
//! the log. A panic on the build thread is fatal by the spec (§3.1), so a build that panics every
//! time would otherwise make every new shim start, and lose, another daemon.
use super::connect::{reach, Hook, PeerUid, Point, Reached, Refusal, Target, Transient};
use super::key_files::KeyDir;
use crate::mcp::daemon::files::FilesError;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Ready;
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::fmt;
use std::fs::TryLockError;
use std::io;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How long a shim waits between two looks at the socket and the locks.
pub(super) const POLL: Duration = Duration::from_millis(10);
/// A key whose last daemon died during its build less than this long ago (by the build's start)
/// is not restarted.
pub(super) const BUILD_CRASH_BACKOFF: Duration = Duration::from_secs(60);

/// Starts a daemon process for the key, detached from the shim (spec §4.1 step 6). Called with
/// the start lock held and the owner lock released.
pub(super) trait Starter {
    fn start(&mut self, dir: &KeyDir<'_>) -> io::Result<Box<dyn Started>>;
}

/// A daemon process this shim started.
pub(super) trait Started: Send {
    fn pid(&self) -> u32;
    /// `None` while it runs; how it ended once it has (and it is then reaped).
    fn exit(&mut self) -> Option<Exit>;
    /// Wait for it to end, so it does not stay a zombie of the shim's. Blocks; the caller runs it
    /// on a thread of its own.
    fn reap(self: Box<Self>);
}

/// How a started daemon ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Exit {
    /// Status 0: it served and drained, or it found the key owned by another daemon.
    Success,
    /// Anything else, described.
    Failure(String),
}

/// A daemon reached.
pub(super) struct Connected {
    /// Blocking, no timeouts; the next byte on it is the daemon's first MCP byte.
    pub(super) stream: UnixStream,
    pub(super) ready: Ready,
    /// The daemon this shim started on the way, if it did.
    pub(super) started: Option<Box<dyn Started>>,
}

/// What a shim knows about the owner lock when it gives up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Owner {
    /// Not looked at.
    Unknown,
    /// A live daemon held it.
    Held,
    /// No daemon held it (a daemon may have been started since).
    Free,
}

/// Why no daemon was reached. Each is a fail-closed diagnostic (spec §4.1 step 7, §10).
#[derive(Debug)]
pub(super) enum Failure {
    /// The runtime directory failed validation, or a key file could not be opened or removed.
    Files(FilesError),
    /// The daemon at the socket answered something the shim fails closed on.
    Refused(Refusal),
    /// Another shim held the start lock until the deadline.
    StartLockTimeout { last: Transient },
    /// The deadline passed. `started` is the pid of the daemon this shim spawned, if any.
    NoReady {
        last: Transient,
        owner: Owner,
        started: Option<u32>,
    },
    /// The daemon could not be spawned.
    SpawnFailed(io::Error),
    /// The daemon this shim spawned ended unsuccessfully before `READY`.
    DaemonExited { pid: u32, exit: String },
    /// The key's last daemon died during its index build `age_secs` ago (S3b's SI-10).
    RecentBuildCrash { pid: u32, age_secs: u64 },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Files(error) => error.fmt(f),
            Self::Refused(refusal) => refusal.fmt(f),
            Self::StartLockTimeout { last } => write!(
                f,
                "another shim held the start lock until the deadline (last attempt: {last})"
            ),
            Self::NoReady {
                last,
                owner,
                started,
            } => {
                write!(
                    f,
                    "no daemon answered READY before the deadline (last attempt: {last}"
                )?;
                match owner {
                    Owner::Held => write!(f, "; a live daemon holds the owner lock")?,
                    Owner::Free => write!(f, "; no daemon held the owner lock")?,
                    Owner::Unknown => {}
                }
                if let Some(pid) = started {
                    write!(
                        f,
                        "; the daemon this shim started, pid {pid}, did not answer"
                    )?;
                }
                write!(f, ")")
            }
            Self::SpawnFailed(error) => write!(f, "cannot start the daemon: {error}"),
            Self::DaemonExited { pid, exit } => write!(
                f,
                "the daemon this shim started, pid {pid}, ended before READY: {exit}"
            ),
            Self::RecentBuildCrash { pid, age_secs } => write!(
                f,
                "the last daemon for this key, pid {pid}, died during its index build, which \
                 began {age_secs} s ago; it is not restarted until {} s after that (a build that \
                 dies every time would otherwise cost a daemon per shim)",
                BUILD_CRASH_BACKOFF.as_secs()
            ),
        }
    }
}

impl From<FilesError> for Failure {
    fn from(error: FilesError) -> Self {
        Self::Files(error)
    }
}

/// One shim's search for its daemon.
pub(super) struct Arbiter<'a> {
    pub(super) paths: &'a DaemonPaths,
    pub(super) key: IdentityKey,
    /// The `HELLO` line, newline included.
    pub(super) hello: &'a str,
    /// The one deadline (spec §4.1 step 4).
    pub(super) deadline: Instant,
    pub(super) peer_uid: PeerUid,
    pub(super) hook: Option<Hook>,
}

impl Arbiter<'_> {
    fn hook(&self, point: Point) {
        if let Some(hook) = &self.hook {
            hook(point);
        }
    }

    fn reach(&self) -> Reached {
        reach(&Target {
            socket: self.paths.socket(),
            key: self.key,
            hello: self.hello,
            deadline: self.deadline,
            peer_uid: self.peer_uid,
            uid: current_uid(),
            hook: self.hook.as_ref(),
        })
    }

    fn past_deadline(&self) -> bool {
        Instant::now() >= self.deadline
    }

    fn pause(&self) {
        let left = self.deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(POLL.min(left));
    }

    /// Find the daemon, starting one with `starter` if none is alive.
    pub(super) fn arbitrate(&self, starter: &mut dyn Starter) -> Result<Connected, Failure> {
        validate_runtime_dir(self.paths.runtime_dir()).map_err(FilesError::from)?;
        let mut last = match self.reach() {
            Reached::Ready(stream, ready) => {
                return Ok(Connected {
                    stream,
                    ready,
                    started: None,
                })
            }
            Reached::Refused(refusal) => return Err(Failure::Refused(refusal)),
            Reached::Transient(transient) => transient,
        };
        if matches!(last, Transient::NoReply) {
            // A daemon accepted and did not answer before the deadline: live but wedged
            // (spec §4.4). It is reported, never replaced.
            return Err(Failure::NoReady {
                last,
                owner: Owner::Unknown,
                started: None,
            });
        }

        let dir = KeyDir::open(self.paths)?;
        let start_lock = dir.start_lock()?;
        let mut waited = false;
        loop {
            match start_lock.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) if !self.past_deadline() => {
                    if !waited {
                        waited = true;
                        self.hook(Point::StartLockBusy);
                    }
                    self.pause();
                }
                Err(TryLockError::WouldBlock) => return Err(Failure::StartLockTimeout { last }),
                Err(TryLockError::Error(source)) => {
                    return Err(Failure::Files(FilesError::Io {
                        op: "lock",
                        path: self.paths.start_lock().to_path_buf(),
                        source,
                    }))
                }
            }
        }
        self.hook(Point::StartLockTaken);

        // The start lock is held from here until this function returns (spec §4.1 step 6:
        // "Keep the start lock until READY or the deadline").
        let mut child: Option<Box<dyn Started>> = None;
        let mut owner = Owner::Unknown;
        loop {
            match self.reach() {
                Reached::Ready(stream, ready) => {
                    return Ok(Connected {
                        stream,
                        ready,
                        started: child,
                    })
                }
                Reached::Refused(refusal) => return Err(Failure::Refused(refusal)),
                Reached::Transient(transient) => last = transient,
            }
            if self.past_deadline() {
                return Err(Failure::NoReady {
                    last,
                    owner,
                    started: child.as_ref().map(|child| child.pid()),
                });
            }
            let child_running = match child.as_mut().map(|child| (child.pid(), child.exit())) {
                Some((_, None)) => true,
                Some((pid, Some(Exit::Failure(exit)))) => {
                    return Err(Failure::DaemonExited { pid, exit })
                }
                Some((_, Some(Exit::Success))) | None => false,
            };
            if !child_running {
                match dir.try_own()? {
                    Some(owner_lock) => {
                        owner = Owner::Free;
                        if let Some(crash) = recent_build_crash(&dir) {
                            return Err(crash);
                        }
                        dir.remove_stale()?;
                        drop(owner_lock);
                        self.hook(Point::BeforeSpawn);
                        child = Some(starter.start(&dir).map_err(Failure::SpawnFailed)?);
                        self.hook(Point::AfterSpawn);
                    }
                    None => {
                        owner = Owner::Held;
                        self.hook(Point::OwnerHeld);
                    }
                }
            }
            self.pause();
        }
    }
}

/// The key's last daemon died during a build that began less than [`BUILD_CRASH_BACKOFF`] ago.
/// A marker with a start time in the future (a clock set back) is not counted: it could hold the
/// key closed for as long as the clock is off.
fn recent_build_crash(dir: &KeyDir<'_>) -> Option<Failure> {
    let (pid, started) = dir.build_marker()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let age_secs = now.checked_sub(started)?;
    (age_secs < BUILD_CRASH_BACKOFF.as_secs())
        .then_some(Failure::RecentBuildCrash { pid, age_secs })
}

#[cfg(test)]
#[path = "arbitrate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "arbitrate_race_tests.rs"]
mod race_tests;
````

In `src/mcp/shim/connect.rs`, replace this:

````rust
pub(super) enum Point {
    /// Connected and the peer checked; `HELLO` not sent yet.
    AfterConnect,
}

pub(super) type Hook = Arc<dyn Fn(Point) + Send + Sync>;
````

with this:

````rust
pub(super) enum Point {
    /// Connected and the peer checked; `HELLO` not sent yet.
    AfterConnect,
    /// The start lock is held by another shim; this one waits for it.
    StartLockBusy,
    /// The start lock is held; the attempt under it has not run yet.
    StartLockTaken,
    /// The owner lock was found held by a live daemon.
    OwnerHeld,
    /// The owner lock was taken, the stale files removed and the lock released; nothing spawned.
    BeforeSpawn,
    /// A daemon was spawned; it may not own the key yet.
    AfterSpawn,
}

pub(super) type Hook = Arc<dyn Fn(Point) + Send + Sync>;
````

Create `src/mcp/shim/key_files.rs`:

````rust
//! A key's files as the shim uses them (spec §4.1 step 6): it takes the start lock, looks at the
//! owner lock, and, only while it holds the owner lock of a key no daemon owns, reads the build
//! marker and removes what a dead daemon left. Every file is opened through the runtime
//! directory's descriptor, with the daemon's own functions (`daemon::files`): lock files are
//! created with `O_EXCL` and then opened (S4's D53), never through a symlink, and never removed.
use crate::mcp::daemon::files::{
    open_private, open_runtime_dir, parse_build_marker, unlink_at, Create, FilesError,
};
use crate::mcp::shared::paths::DaemonPaths;
use std::fs::{File, TryLockError};
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::path::Path;

/// The runtime directory of one key, opened once.
pub(super) struct KeyDir<'a> {
    dir: OwnedFd,
    paths: &'a DaemonPaths,
}

impl<'a> KeyDir<'a> {
    /// Open the runtime directory of `paths`, which the caller has validated.
    pub(super) fn open(paths: &'a DaemonPaths) -> Result<Self, FilesError> {
        Ok(Self {
            dir: open_runtime_dir(paths.runtime_dir())?,
            paths,
        })
    }

    /// The start lock's file (spec §4.1 step 6), not locked yet.
    pub(super) fn start_lock(&self) -> Result<File, FilesError> {
        open_private(
            &self.dir,
            self.paths.start_lock(),
            libc::O_RDWR,
            Create::Yes,
        )
    }

    /// Take the owner lock if no daemon holds it: `Some` holds it until dropped, `None` means a
    /// daemon is alive (starting, serving or draining).
    pub(super) fn try_own(&self) -> Result<Option<File>, FilesError> {
        let owner = open_private(
            &self.dir,
            self.paths.owner_lock(),
            libc::O_RDWR,
            Create::Yes,
        )?;
        match owner.try_lock() {
            Ok(()) => Ok(Some(owner)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(source)) => Err(FilesError::Io {
                op: "lock",
                path: self.paths.owner_lock().to_path_buf(),
                source,
            }),
        }
    }

    /// The pid and build start time in the build marker, when one is there and whole. Call it
    /// holding the owner lock: then the marker is a dead daemon's.
    pub(super) fn build_marker(&self) -> Option<(u32, u64)> {
        let mut file = open_private(
            &self.dir,
            self.paths.build_marker(),
            libc::O_RDONLY,
            Create::No,
        )
        .ok()?;
        let mut text = String::new();
        file.read_to_string(&mut text).ok()?;
        parse_build_marker(&text)
    }

    /// Remove what a daemon that died left: its socket, pid file and build marker (spec §4.1
    /// step 6: "Holding it, unlink any stale socket and pid file"). Call it holding the owner
    /// lock, so that a daemon about to bind can never have its live socket removed.
    pub(super) fn remove_stale(&self) -> Result<(), FilesError> {
        for path in [
            self.paths.socket(),
            self.paths.pid_file(),
            self.paths.build_marker(),
        ] {
            remove(&self.dir, path)?;
        }
        Ok(())
    }
}

fn remove(dir: &OwnedFd, path: &Path) -> Result<(), FilesError> {
    match unlink_at(dir, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FilesError::Io {
            op: "remove",
            path: path.to_path_buf(),
            source,
        }),
    }
}
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t3.log" 2>&1
```

Expected: `test result: ok. 547 passed; 0 failed` (530 + 17), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/files.rs src/mcp/daemon/mod.rs src/mcp/shim/
git commit -m "feat(mcp): the shim's start protocol: start lock, owner lock, stale files, restart bound"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::shim::arbitrate`, see the named test fail, then `git checkout -- src/mcp/shim/arbitrate.rs`.

| Mutant | Edit in `arbitrate.rs` | Fails |
|---|---|---|
| 3A | delete `dir.remove_stale()?;` | `a_stale_socket_and_pid_file_are_removed_under_the_owner_lock_before_the_start` |
| 3B | move `drop(owner_lock);` to after `child = Some(starter.start(&dir).map_err(Failure::SpawnFailed)?);` | the same test (the owner lock is not free at `BeforeSpawn`) |
| 3C | in the `None =>` arm, after `self.hook(Point::OwnerHeld);`, add `dir.remove_stale()?; child = Some(starter.start(&dir).map_err(Failure::SpawnFailed)?);` | `a_live_but_wedged_owner_is_reported_and_its_files_are_left_alone` |
| 3D | in the `None =>` arm, return `Err(Failure::NoReady { last, owner: Owner::Held, started: None })` after the hook | `a_key_whose_owner_is_releasing_it_is_started_once_the_lock_is_free` |
| 3E | replace the two arms `Some((pid, Some(Exit::Failure(exit)))) => { return Err(Failure::DaemonExited { pid, exit }) }` and `Some((_, Some(Exit::Success))) \| None => false,` by `Some((_, Some(_))) \| None => false,` | `a_started_daemon_that_fails_before_ready_ends_the_start` |
| 3F | delete `if let Some(crash) = recent_build_crash(&dir) { return Err(crash); }` | `a_build_that_died_moments_ago_is_not_restarted` |
| 3G | in `recent_build_crash`, replace `let age_secs = now.checked_sub(started)?;` by `let age_secs = now.saturating_sub(started);` | `an_old_future_or_partial_build_marker_does_not_hold_the_key` |
| 3I | replace `match start_lock.try_lock() {` by `match Ok::<(), TryLockError>(()) {` (no start lock) | `shims_racing_to_start_a_daemon_start_exactly_one` |
| 3J | replace `Some((_, None)) => true,` by `Some((_, None)) => false,` (look at the owner lock while the started daemon is still starting) | `a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for` |

### Task 4: S5a-1 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
pid file removed, owner lock released, exit 0. `run.rs` is `run_daemon`: resolve the identity
(refusing a `--key` that differs), own the key or exit 0 silently, bind, write the pid file,
serve. `log.rs` caps the daemon's own lines (log truncated at start over 1 MiB, capped at 8 MiB).

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

with this:

````markdown
pid file removed, owner lock released, exit 0. `run.rs` is `run_daemon`: resolve the identity
(refusing a `--key` that differs), own the key or exit 0 silently, bind, write the pid file,
serve. `log.rs` caps the daemon's own lines (log truncated at start over 1 MiB, capped at 8 MiB).
While its builder runs the daemon leaves `<key16>.build` (`files.rs::BuildMarker`, set and cleared
around the builder by `run.rs::marked`, removed at release); a daemon that dies during its build
leaves it.

Shim (`src/mcp/shim/`, Unix only; no production caller yet): `connect.rs` makes one attempt to
reach a daemon: connect, check the peer's uid (`daemon::peer`; a peer of another uid is refused
before anything is sent), send `HELLO`, and read the one reply line with every read bounded by the
time left to the shim's one deadline. `READY` for this key reaches it (the socket's timeouts are
cleared); `RETIRING`, an end of stream or a reset before a reply, a failed connect, or no reply by
the deadline are transient; `BUSY`, `MISMATCH`, `READY` for another identity and an unreadable
line are refused (fail closed). `arbitrate.rs` is the start protocol: the socket first; then the
start lock (`<key16>.start`); under it the socket again, then the owner lock: held means a live
daemon (poll every 10 ms; never touch its files), free means none (holding it, remove the stale
socket, pid file and build marker, release it, start a daemon through the `Starter` seam). While a
started daemon neither answers nor ends the owner lock is not looked at again; one that ends 0
before `READY` lost the key (look again), one that ends otherwise fails the start. A free key whose
build marker says a build began under 60 s ago is not restarted (the restart bound for a build that
keeps dying). `key_files.rs` opens the key's files through the runtime directory's descriptor with
`daemon::files`' functions.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check > "$LOG_DIR/s51-fmt.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/s51-lib.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/s51-mcp.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check > "$LOG_DIR/s51-check.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check --features mcp > "$LOG_DIR/s51-checkmcp.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets --features mcp > "$LOG_DIR/s51-clippy.log" 2>&1
grep -E '^\s+--> (src/mcp/shim|src/mcp/daemon/(files|run|run_tests|peer|mod)\.rs|src/mcp/shared/paths)' "$LOG_DIR/s51-clippy.log"
```

Expected: `cargo fmt --check` silent; `test result: ok. 547 passed; 0 failed` (517 + 30); `31 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; both checks `Finished`, no `warning` in the feature check; clippy 257 `warning` lines, and the `grep` prints nothing.

- [ ] **Step 3: State the invariants**

```bash
git diff --name-only origin/main -- src/mcp/transport.rs src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs src/mcp/session.rs src/mcp/lazy.rs src/mcp/framing.rs src/mcp/output.rs src/mcp/settings.rs tests/fixtures/mcp/golden src/bin
git diff --name-only origin/main | wc -l
wc -l src/mcp/shim/*.rs src/mcp/daemon/files.rs src/mcp/daemon/run.rs src/mcp/daemon/run_tests.rs
```

Expected: the first prints nothing; `17` (16 under `src/` and `CLAUDE.md`); no file over 600 lines.

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
for i in $(seq 1 20); do nice -n 19 cargo test --features mcp --lib mcp::shim > "$LOG_DIR/s51-rep-$i.log" 2>&1; done
for i in $(seq 1 10); do nice -n 19 cargo test --features mcp --lib mcp::shim -- --test-threads=1 > "$LOG_DIR/s51-rep1-$i.log" 2>&1; done
grep -h '^test result' "$LOG_DIR"/s51-rep-*.log "$LOG_DIR"/s51-rep1-*.log | sed 's/finished.*//' | sort | uniq -c
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s51-full.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s51-doc.log" 2>&1
```

Then on Linux:

```bash
# Linux: the bridge's toolchain image, the worktree read-only, named volumes removed afterwards.
# Check first that the host and the Docker VM have room for a debug build (about 8 GB): a full
# disk stopped the Docker VM during this plan's validation (Validation record).
df -h ~
docker run --rm --cpus 4 --memory 8g -v "$PWD":/src:ro -v s5a-target:/target \
  -v s5a-registry:/usr/local/cargo/registry -e CARGO_TARGET_DIR=/target -w /src \
  --entrypoint sh a2a-toolchain:latest -c \
  'for i in $(seq 1 10); do cargo test --features mcp --lib mcp::shim 2>&1 | grep "^test result"; cargo test --features mcp --lib mcp::daemon 2>&1 | grep "^test result"; done' \
  > "$LOG_DIR/s51-linux.log" 2>&1
docker volume rm s5a-target s5a-registry
sort "$LOG_DIR/s51-linux.log" | sed 's/finished.*//' | uniq -c
```

Expected: `30 test result: ok. 25 passed; 0 failed`; the full suite and the doc tests `0 failed`; on Linux `10 test result: ok. 25 passed; 0 failed` and `10 test result: ok. 133 passed; 0 failed`.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S5a-1 is the shim's start protocol, tested against fake daemons: which answers are transient and which fail closed, who starts the daemon, what is removed and when, and the restart bound. (2) **Invariants:** Step 3. (3) **Decisions:** D77–D83, D90, D91; SI-22–SI-24. D77 (the restart bound) and D78 (the shim's peer check) are the owner's to see; D82 (not looking at the owner lock while a started daemon starts) is the one a reviewer is most likely to question. (4) **Claims:** the rows for Tasks 1–3, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base (Linux included), and mutants 1A–3J as observed. (6) **Not verified:** a real daemon process started by a shim (S5a-2); x86_64 Linux before CI. (7) **Tier-A:** not run (no file it watches is touched). (8) The owner questions of this plan, as defaults built.

```bash
git add CLAUDE.md
git commit -m "docs(mcp): the shim's start protocol"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): the shim's start protocol (shared daemon S5a-1)" --body-file "$LOG_DIR/s51-pr-body.md"
```

Review: Opus 5.5 and gpt-6.1-sol, same brief, same commit; two rounds (lane README).

---

# Part two: S5a-2, the shim as a process (Tasks 5–7)

### Task 5: Spawning the daemon, the fail-closed line, the pump

**Files:**
- Modify: `src/mcp/shim/mod.rs`, `src/mcp/shim/key_files.rs`, `src/navigation/call_edge_cache.rs`
- Create: `src/mcp/shim/spawn.rs`, `src/mcp/shim/spawn_tests.rs`, `src/mcp/shim/diagnostic.rs`, `src/mcp/shim/diagnostic_tests.rs`, `src/mcp/shim/pump.rs`, `src/mcp/shim/pump_tests.rs`

**Interfaces:**
- Consumes: Task 3's `Starter`, `Started`, `Exit`, `KeyDir`; `refresh_policy_name`, `IdentityKey`, `RUNTIME_DIR_ENV` (S2a); `LOAD_DIRTY_OVERRIDE` (`navigation::call_edge_cache`, made `pub(crate)`).
- Produces: `KeyDir::open_log(&self) -> Result<File, FilesError>`; in `shim::spawn`: `struct DaemonArgs { repo_root, cache_base, refresh_policy, key, warm_at_startup, idle_ttl: Option<u64>, max_clients: Option<u32> }` with `to_args(&self) -> Vec<OsString>`, `fn daemon_command(exe: &Path, args: &DaemonArgs, runtime_dir: &Path, log: File) -> io::Result<Command>`, `struct ProcessStarter { exe, args, runtime_dir }` (`Starter`); in `shim::diagnostic`: `struct Where<'a> { key, runtime_dir: Option<&Path>, paths: Option<&DaemonPaths> }`, `fn one_line(text: &str) -> String`, `fn fail_closed_line(reason: &str, at: &Where<'_>, standalone: &str) -> String`, `fn standalone_invocation(exe: &Path, cfg: &ServerConfig) -> String`; in `shim::pump`: `enum PumpEnd { ClientClosed, DaemonClosed, DaemonFailed(io::Error), ClientGone(io::Error) }` (`Display`), `fn pump(input: impl Read + Send + 'static, output: &mut impl Write, daemon: UnixStream) -> PumpEnd`.

- [ ] **Step 1: Write the failing tests**

Create `src/mcp/shim/diagnostic_tests.rs`:

````rust
//! Tests for the fail-closed line (spec §4.1 step 7).
use super::super::shim_support::{key, runtime};
use super::*;
use std::path::PathBuf;

/// Spec §4.1 step 7: the line names the key, the runtime directory, the socket, the pid file's
/// contents (its pid and start time, spec §4.1 step 6), the log, and the standalone invocation,
/// on one line.
#[test]
fn the_line_names_everything_an_operator_needs_on_one_line() {
    let (_base, paths) = runtime();
    std::fs::write(paths.pid_file(), "4242\n1700000000\nabc\n").expect("pid file");
    let key = key();
    let at = Where {
        key: &key,
        runtime_dir: Some(paths.runtime_dir()),
        paths: Some(&paths),
    };
    let line = fail_closed_line("the reason", &at, "prism-mcp --repo /r --standalone");
    assert!(!line.contains('\n'), "{line}");
    for part in [
        "the reason".to_string(),
        key.to_hex(),
        paths.runtime_dir().display().to_string(),
        paths.socket().display().to_string(),
        "pid 4242, start time 1700000000, build abc".to_string(),
        paths.log_file().display().to_string(),
        "prism-mcp --repo /r --standalone".to_string(),
    ] {
        assert!(line.contains(&part), "{part} in {line}");
    }
}

/// S4's interface: the pid file is rewritten in place, so a reader can find it empty or partial;
/// that is said, not misread. A missing one is said too.
#[test]
fn an_empty_partial_or_missing_pid_file_is_reported_as_such() {
    let (_base, paths) = runtime();
    let key = key();
    let at = Where {
        key: &key,
        runtime_dir: Some(paths.runtime_dir()),
        paths: Some(&paths),
    };
    assert!(fail_closed_line("r", &at, "s").contains("absent"));
    std::fs::write(paths.pid_file(), "").expect("pid file");
    assert!(fail_closed_line("r", &at, "s").contains("empty or partial (\"\")"));
    std::fs::write(paths.pid_file(), "42\n17").expect("pid file");
    let line = fail_closed_line("r", &at, "s");
    assert!(line.contains("empty or partial (\"42\\n17\")"), "{line}");
    assert!(!line.contains('\n'));
}

/// Lane README, notes for S5a: a path with a newline or a terminal escape cannot break the line.
#[test]
fn control_characters_in_paths_cannot_end_the_line() {
    let key = key();
    let dir = PathBuf::from("/tmp/a\nb\u{1b}[2J");
    let at = Where {
        key: &key,
        runtime_dir: Some(&dir),
        paths: None,
    };
    let line = fail_closed_line("r", &at, "s");
    assert!(!line.contains('\n') && !line.contains('\u{1b}'), "{line:?}");
    assert!(line.contains("/tmp/a\\nb\\u{1b}[2J"), "{line}");
}

/// Spec §4.1 step 7: "the exact `--standalone` invocation": the same repository, cache,
/// refresh policy, first-call wait and warm start, and words a shell reads back as they are.
#[test]
fn the_standalone_invocation_keeps_the_configuration() {
    let mut cfg = ServerConfig::new(PathBuf::from("/my repo"));
    assert_eq!(
        standalone_invocation(Path::new("/bin/prism-mcp"), &cfg),
        "/bin/prism-mcp --repo '/my repo' --standalone"
    );
    cfg.cache = CacheMode::Dir(PathBuf::from("/c/it's"));
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    cfg.first_call_wait = Duration::from_secs(60);
    cfg.startup = StartupMode::Background;
    assert_eq!(
        standalone_invocation(Path::new("/bin/prism-mcp"), &cfg),
        "/bin/prism-mcp --repo '/my repo' --cache-dir '/c/it'\\''s' --refresh-policy auto-full \
         --first-call-wait 60 --warm-at-startup --standalone"
    );
}
````

In `src/mcp/shim/mod.rs`, replace this:

````rust

mod arbitrate;
mod connect;
mod key_files;
#[cfg(test)]
mod shim_support;
````

with this:

````rust

mod arbitrate;
mod connect;
mod diagnostic;
mod key_files;
mod pump;
#[cfg(test)]
mod shim_support;
mod spawn;
````

Create `src/mcp/shim/pump_tests.rs`:

````rust
//! Tests for the byte pipe after `READY` (spec §4.1 steps 8 and 9). The daemon and the client
//! are each the far end of a socket pair.
use super::super::shim_support::HANG;
use super::*;
use std::sync::mpsc;

/// Run the pump on a thread; return the client's input end, the daemon's end, and a receiver of
/// how the pump ended with what it wrote to the client.
fn running() -> (UnixStream, UnixStream, mpsc::Receiver<(PumpEnd, Vec<u8>)>) {
    let (client, input) = UnixStream::pair().expect("the client's input");
    let (daemon, shim_side) = UnixStream::pair().expect("the daemon's connection");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let end = pump(input, &mut output, shim_side);
        let _ = tx.send((end, output));
    });
    (client, daemon, rx)
}

/// Spec §4.1 step 8: bytes go both ways unchanged, a partial line and a frame larger than the
/// pump's buffer included; when the client closes its input the daemon sees the end of its
/// input, its last response still reaches the client, and the session ends cleanly.
#[test]
fn bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly() {
    let (mut client, mut daemon, ended) = running();
    let big: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    // The daemon reads while the client writes: a socket holds far less than the frame.
    daemon
        .set_read_timeout(Some(HANG))
        .expect("a bound on the test");
    let reader = {
        let mut daemon = daemon.try_clone().expect("the daemon's end");
        std::thread::spawn(move || {
            let mut received = Vec::new();
            let saw_end = daemon.read_to_end(&mut received).is_ok();
            (saw_end, received)
        })
    };
    client.write_all(b"{\"partial").expect("write");
    client.write_all(&big).expect("write");
    client
        .shutdown(Shutdown::Write)
        .expect("the client closes its input");
    let (saw_end, received) = reader.join().expect("the daemon's reader");
    assert!(
        saw_end,
        "the daemon saw the end of its input, not a timeout"
    );
    assert_eq!(&received[..9], b"{\"partial");
    assert_eq!(&received[9..], &big[..]);
    daemon.write_all(b"the last response\n").expect("write");
    drop(daemon);
    let (end, output) = ended.recv_timeout(HANG).expect("the pump ends");
    assert!(matches!(end, PumpEnd::ClientClosed), "{end:?}");
    assert_eq!(output, b"the last response\n");
}

/// Spec §4.1 steps 8 and 9 and §9 test 11: a daemon that closes the connection while the client
/// is still there ends the session as an error; what it sent before is delivered.
#[test]
fn a_daemon_that_closes_while_the_client_is_there_ends_the_session_as_an_error() {
    let (_client, mut daemon, ended) = running();
    daemon.write_all(b"one response\n").expect("write");
    drop(daemon);
    let (end, output) = ended.recv_timeout(HANG).expect("the pump ends");
    assert!(matches!(end, PumpEnd::DaemonClosed), "{end:?}");
    assert_eq!(output, b"one response\n");
}
````

Create `src/mcp/shim/spawn_tests.rs`:

````rust
//! Tests for how the daemon is started (spec §4.1 step 6, §7, §11 item 7; S4's SI-16 and SI-17).
use super::super::shim_support::{key, runtime};
use super::*;
use std::ffi::OsStr;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;

fn args(warm: bool, idle_ttl: Option<u64>, max_clients: Option<u32>) -> DaemonArgs {
    DaemonArgs {
        repo_root: PathBuf::from("/repo"),
        cache_base: PathBuf::from("/cache"),
        refresh_policy: RefreshPolicy::AutoIncremental,
        key: key(),
        warm_at_startup: warm,
        idle_ttl,
        max_clients,
    }
}

fn command_for(args: &DaemonArgs) -> (tempfile::TempDir, Command) {
    let (base, paths) = runtime();
    let log = File::create(base.path().join("log")).expect("a log");
    let command = daemon_command(Path::new("/bin/prism-mcp"), args, paths.runtime_dir(), log)
        .expect("a command");
    (base, command)
}

/// S4's interface for S5a: the daemon gets the shim's canonical root, canonical cache base,
/// refresh policy and full key, and `--warm-at-startup` only when the shim has it (spec §11 item
/// 7); `--idle-ttl` and `--max-clients` only when given.
#[test]
fn the_daemon_is_started_with_the_shim_s_configuration_and_key() {
    let hex = key().to_hex();
    let (_base, plain) = command_for(&args(false, None, None));
    assert_eq!(
        plain.get_args().collect::<Vec<_>>(),
        [
            "daemon",
            "--repo",
            "/repo",
            "--cache-dir",
            "/cache",
            "--refresh-policy",
            "auto-incremental",
            "--key",
            hex.as_str()
        ]
        .map(OsStr::new)
    );
    let (_base, full) = command_for(&args(true, Some(5), Some(8)));
    let tail: Vec<_> = full.get_args().skip(9).collect();
    assert_eq!(
        tail,
        ["--warm-at-startup", "--idle-ttl", "5", "--max-clients", "8"].map(OsStr::new)
    );
}

/// Spec §7 and S4's SI-17: `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` changes which call-edge sidecar a
/// daemon loads and is in neither the key nor the preamble, so it is removed from the daemon's
/// environment; the runtime directory is passed explicitly, and the daemon runs from `/`, so it
/// holds no directory of the shim's open.
#[test]
fn the_daemon_s_environment_drops_the_dirty_sidecar_override_and_names_the_runtime_dir() {
    let (base, paths) = runtime();
    let log = File::create(base.path().join("log")).expect("a log");
    let command = daemon_command(
        Path::new("/bin/prism-mcp"),
        &args(false, None, None),
        paths.runtime_dir(),
        log,
    )
    .expect("a command");
    let envs: Vec<_> = command.get_envs().collect();
    assert!(
        envs.contains(&(OsStr::new(LOAD_DIRTY_OVERRIDE), None)),
        "{envs:?}"
    );
    assert!(
        envs.contains(&(OsStr::new(RUNTIME_DIR_ENV), Some(base.path().as_os_str()))),
        "{envs:?}"
    );
    assert_eq!(command.get_current_dir(), Some(Path::new("/")));
}

/// S4's SI-16: the log the shim gives the daemon is opened for appending (the daemon that wins
/// the key truncates it when it is over 1 MiB; a descriptor without `O_APPEND` would overwrite
/// or leave a hole), created 0600, and an existing log is added to, not replaced.
#[test]
fn the_log_is_opened_for_appending() {
    let (_base, paths) = runtime();
    std::fs::write(paths.log_file(), "an earlier daemon's line\n").expect("an earlier log");
    let dir = KeyDir::open(&paths).expect("the runtime directory");
    let mut log = dir.open_log().expect("the log");
    // SAFETY: `F_GETFL` on an open descriptor reads its flags and touches no memory.
    let flags = unsafe { libc::fcntl(log.as_raw_fd(), libc::F_GETFL) };
    assert_ne!(flags & libc::O_APPEND, 0, "O_APPEND");
    log.write_all(b"this daemon's line\n").expect("write");
    assert_eq!(
        std::fs::read_to_string(paths.log_file()).expect("the log"),
        "an earlier daemon's line\nthis daemon's line\n"
    );
    std::fs::remove_file(paths.log_file()).expect("remove");
    drop(dir.open_log().expect("a new log"));
    let mode = std::fs::metadata(paths.log_file())
        .expect("log")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t5-red.log" 2>&1
```

Expected in the log: `error[E0583]: file not found for module `diagnostic``, the same for `pump` and for `spawn`.

- [ ] **Step 3: Implement**

Create `src/mcp/shim/diagnostic.rs`:

````rust
//! The line a shim prints when it fails closed (spec §4.1 step 7): why, the key, the runtime
//! directory, the socket, the pid file's contents, the log, and the exact `--standalone`
//! invocation that runs without the shared daemon. One line, whatever the paths hold: control
//! characters are escaped (lane README, notes for S5a).
use crate::mcp::shared::identity::{refresh_policy_name, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, StartupMode};
use std::io::Read;
use std::path::Path;
use std::time::Duration;

/// The default `--first-call-wait`, which the standalone invocation leaves out.
const DEFAULT_FIRST_CALL_WAIT: Duration = Duration::from_secs(20);
/// Most bytes of a pid file the line repeats.
const PID_FILE_BYTES: u64 = 200;

/// Where the shim looked, as far as it got.
pub(super) struct Where<'a> {
    pub(super) key: &'a IdentityKey,
    /// `None` when no runtime directory could be resolved.
    pub(super) runtime_dir: Option<&'a Path>,
    /// `None` when the socket path does not fit (or no runtime directory was resolved).
    pub(super) paths: Option<&'a DaemonPaths>,
}

/// `text` with every control character escaped (`\n`, `\u{1b}`), so it cannot end the line or
/// drive a terminal.
pub(super) fn one_line(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            let escaped: Vec<char> = if c.is_control() {
                c.escape_default().collect()
            } else {
                vec![c]
            };
            escaped
        })
        .collect()
}

/// The fail-closed line, without its newline.
pub(super) fn fail_closed_line(reason: &str, at: &Where<'_>, standalone: &str) -> String {
    let mut line = format!(
        "prism-mcp: --shared failed closed, no index was built: {reason}; key {}",
        at.key
    );
    match at.runtime_dir {
        Some(dir) => line.push_str(&format!("; runtime dir {}", dir.display())),
        None => line.push_str("; runtime dir unresolved"),
    }
    if let Some(paths) = at.paths {
        line.push_str(&format!(
            "; socket {}; pid file {}; log {}",
            paths.socket().display(),
            pid_file(paths.pid_file()),
            paths.log_file().display()
        ));
    }
    line.push_str(&format!("; to run without the shared daemon: {standalone}"));
    one_line(&line)
}

/// What the pid file says (spec §4.1 step 6: a wedged daemon is named by "the pid and start
/// time from `<key>.pid`"). The daemon rewrites it in place, so it can be empty or partial
/// while its owner writes it (S4's interface); that is reported as such.
fn pid_file(path: &Path) -> String {
    let mut text = String::new();
    let read = std::fs::File::open(path)
        .and_then(|file| file.take(PID_FILE_BYTES).read_to_string(&mut text));
    match read {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            format!("{} absent", path.display())
        }
        Err(error) => format!("{} unreadable ({error})", path.display()),
        Ok(_) => {
            let fields: Vec<&str> = text.split('\n').collect();
            match fields.as_slice() {
                [pid, start, build, ""]
                    if pid.parse::<u32>().is_ok() && start.parse::<u64>().is_ok() =>
                {
                    format!(
                        "{}: pid {pid}, start time {start}, build {build}",
                        path.display()
                    )
                }
                _ => format!("{}: empty or partial ({text:?})", path.display()),
            }
        }
    }
}

/// The invocation that serves the same configuration in process: this executable, the same
/// repository, cache, refresh policy, first-call wait and warm start, and `--standalone`.
pub(super) fn standalone_invocation(exe: &Path, cfg: &ServerConfig) -> String {
    let mut words = vec![
        exe.display().to_string(),
        "--repo".into(),
        cfg.repo_root.display().to_string(),
    ];
    if let CacheMode::Dir(base) = &cfg.cache {
        words.extend(["--cache-dir".into(), base.display().to_string()]);
    }
    if cfg.refresh_policy != RefreshPolicy::WarnOnly {
        words.extend([
            "--refresh-policy".into(),
            refresh_policy_name(cfg.refresh_policy).into(),
        ]);
    }
    if cfg.first_call_wait != DEFAULT_FIRST_CALL_WAIT {
        words.extend([
            "--first-call-wait".into(),
            cfg.first_call_wait.as_secs().to_string(),
        ]);
    }
    if cfg.startup == StartupMode::Background {
        words.push("--warm-at-startup".into());
    }
    words.push("--standalone".into());
    words
        .iter()
        .map(|word| shell_word(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as a POSIX shell reads it back: bare when it is plain, else single-quoted.
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./=:,+@%-".contains(c));
    if plain {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
#[path = "diagnostic_tests.rs"]
mod tests;
````

In `src/mcp/shim/key_files.rs`, replace this:

````rust
        parse_build_marker(&text)
    }

    /// Remove what a daemon that died left: its socket, pid file and build marker (spec §4.1
    /// step 6: "Holding it, unlink any stale socket and pid file"). Call it holding the owner
    /// lock, so that a daemon about to bind can never have its live socket removed.
````

with this:

````rust
        parse_build_marker(&text)
    }

    /// The key's log, opened for appending, for the daemon this shim starts (its standard output
    /// and standard error). Appending matters: the daemon that wins the key truncates its log
    /// when it is over 1 MiB (S4's SI-16), and a descriptor without `O_APPEND` would then go on
    /// writing at its old offset, leaving a hole, or overwrite the start of a shorter log.
    pub(super) fn open_log(&self) -> Result<File, FilesError> {
        open_private(
            &self.dir,
            self.paths.log_file(),
            libc::O_WRONLY | libc::O_APPEND,
            Create::Yes,
        )
    }

    /// Remove what a daemon that died left: its socket, pid file and build marker (spec §4.1
    /// step 6: "Holding it, unlink any stale socket and pid file"). Call it holding the owner
    /// lock, so that a daemon about to bind can never have its live socket removed.
````

Create `src/mcp/shim/pump.rs`:

````rust
//! After `READY`, the shim is a byte pipe (spec §4.1 steps 8 and 9): its client's standard input
//! to the daemon's socket on one thread, the socket to standard output on the other, byte for
//! byte. When the client closes its input the socket is half-closed and the daemon's remaining
//! responses are still delivered; a daemon that closes the connection while the client is still
//! there ends the session, and the shim never reconnects: once a byte has been forwarded, a lost
//! daemon is a lost session.
use std::fmt;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const BUFFER_BYTES: usize = 64 * 1024;

/// How a session ended.
#[derive(Debug)]
pub(super) enum PumpEnd {
    /// The client closed its input and the daemon then finished: exit 0.
    ClientClosed,
    /// The daemon closed the connection while the client was still there.
    DaemonClosed,
    /// Reading from the daemon failed (a reset, on Linux, when it died).
    DaemonFailed(io::Error),
    /// Writing to the client failed: it is gone.
    ClientGone(io::Error),
}

impl fmt::Display for PumpEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClientClosed => write!(f, "the client closed the session"),
            Self::DaemonClosed => write!(f, "the shared daemon closed the connection"),
            Self::DaemonFailed(error) => {
                write!(f, "the shared daemon's connection failed: {error}")
            }
            Self::ClientGone(error) => write!(f, "writing to the client failed: {error}"),
        }
    }
}

/// Copy `input` to `daemon` and `daemon` to `output` until the session ends.
pub(super) fn pump(
    mut input: impl Read + Send + 'static,
    output: &mut impl Write,
    daemon: UnixStream,
) -> PumpEnd {
    let input_closed = Arc::new(AtomicBool::new(false));
    match daemon.try_clone() {
        Ok(mut to_daemon) => {
            let input_closed = Arc::clone(&input_closed);
            std::thread::spawn(move || {
                let mut buffer = vec![0u8; BUFFER_BYTES];
                loop {
                    match input.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if to_daemon.write_all(&buffer[..n]).is_err() {
                                // The daemon is gone; the other direction sees it too.
                                return;
                            }
                        }
                    }
                }
                // Recorded before the half-close: the daemon can end the connection in answer to
                // it only after this is visible.
                input_closed.store(true, Ordering::SeqCst);
                let _ = to_daemon.shutdown(Shutdown::Write);
            });
        }
        Err(error) => return PumpEnd::DaemonFailed(error),
    }
    let mut from_daemon = daemon;
    let mut buffer = vec![0u8; BUFFER_BYTES];
    loop {
        match from_daemon.read(&mut buffer) {
            Ok(0) if input_closed.load(Ordering::SeqCst) => return PumpEnd::ClientClosed,
            Ok(0) => return PumpEnd::DaemonClosed,
            Ok(n) => {
                if let Err(error) = output.write_all(&buffer[..n]).and_then(|()| output.flush()) {
                    return PumpEnd::ClientGone(error);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return PumpEnd::DaemonFailed(error),
        }
    }
}

#[cfg(test)]
#[path = "pump_tests.rs"]
mod tests;
````

Create `src/mcp/shim/spawn.rs`:

````rust
//! Starting the daemon process (spec §4.1 step 6): `prism-mcp daemon …` from this same
//! executable, in a session of its own (`setsid`), with standard input null and standard output
//! and error appended to the key's log, from the root directory, with the shim's environment
//! minus what would change its answers (spec §7). The lock descriptors are close-on-exec, so the
//! child inherits neither lock; it takes the owner lock itself.
use super::arbitrate::{Exit, Started, Starter};
use super::key_files::KeyDir;
use crate::mcp::shared::identity::{refresh_policy_name, IdentityKey};
use crate::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use crate::mcp::RefreshPolicy;
use crate::navigation::call_edge_cache::LOAD_DIRTY_OVERRIDE;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// What the daemon is started with: the shim's resolved configuration and key (S4's interface
/// for S5a). The daemon resolves its own key from these and refuses to start on a difference.
#[derive(Clone, Debug)]
pub(super) struct DaemonArgs {
    /// The canonical repository root.
    pub(super) repo_root: PathBuf,
    /// The canonical cache base (`DaemonIdentity::cache_base`).
    pub(super) cache_base: PathBuf,
    pub(super) refresh_policy: RefreshPolicy,
    pub(super) key: IdentityKey,
    /// `--warm-at-startup` on the daemon's command line (spec §11 item 7).
    pub(super) warm_at_startup: bool,
    /// `--idle-ttl` and `--max-clients` for a daemon this shim starts; a running daemon keeps
    /// its own.
    pub(super) idle_ttl: Option<u64>,
    pub(super) max_clients: Option<u32>,
}

impl DaemonArgs {
    /// The daemon's arguments, `daemon` first.
    pub(super) fn to_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec![
            "daemon".into(),
            "--repo".into(),
            self.repo_root.clone().into(),
            "--cache-dir".into(),
            self.cache_base.clone().into(),
            "--refresh-policy".into(),
            refresh_policy_name(self.refresh_policy).into(),
            "--key".into(),
            self.key.to_hex().into(),
        ];
        if self.warm_at_startup {
            args.push("--warm-at-startup".into());
        }
        if let Some(secs) = self.idle_ttl {
            args.extend(["--idle-ttl".into(), secs.to_string().into()]);
        }
        if let Some(max) = self.max_clients {
            args.extend(["--max-clients".into(), max.to_string().into()]);
        }
        args
    }
}

/// The command that starts the daemon. `runtime_dir` is the directory the shim validated; the
/// daemon is pointed at the same one through `PRISM_MCP_RUNTIME_DIR` (its parent, the base).
pub(super) fn daemon_command(
    exe: &Path,
    args: &DaemonArgs,
    runtime_dir: &Path,
    log: File,
) -> io::Result<Command> {
    let mut command = Command::new(exe);
    command
        .args(args.to_args())
        // A daemon serves every client of its key; what changes its answers is in the key or
        // the preamble, never inherited from the shim that happened to start it (spec §7;
        // S4's SI-17).
        .env_remove(LOAD_DIRTY_OVERRIDE)
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    if let Some(base) = runtime_dir.parent() {
        command.env(RUNTIME_DIR_ENV, base);
    }
    // SAFETY: the closure runs in the child between `fork` and `exec` and calls only `setsid`,
    // which is async-signal-safe, and reads `errno`.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(command)
}

/// Starts daemon processes from this executable.
pub(super) struct ProcessStarter {
    pub(super) exe: PathBuf,
    pub(super) args: DaemonArgs,
    pub(super) runtime_dir: PathBuf,
}

impl Starter for ProcessStarter {
    fn start(&mut self, dir: &KeyDir<'_>) -> io::Result<Box<dyn Started>> {
        let log = dir
            .open_log()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let child = daemon_command(&self.exe, &self.args, &self.runtime_dir, log)?.spawn()?;
        Ok(Box::new(DaemonChild(child)))
    }
}

/// A daemon process this shim spawned.
struct DaemonChild(Child);

impl Started for DaemonChild {
    fn pid(&self) -> u32 {
        self.0.id()
    }

    fn exit(&mut self) -> Option<Exit> {
        match self.0.try_wait() {
            Ok(None) => None,
            Ok(Some(status)) if status.success() => Some(Exit::Success),
            Ok(Some(status)) => Some(Exit::Failure(status.to_string())),
            Err(error) => Some(Exit::Failure(format!("cannot wait for it: {error}"))),
        }
    }

    fn reap(mut self: Box<Self>) {
        let _ = self.0.wait();
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
````

In `src/navigation/call_edge_cache.rs`, replace this:

````rust
const NAV_CALL_EDGE_CACHE_VERSION: u32 = 65;
const CACHE_BIN: &str = "resolved-call-edge-index.bin";
const CACHE_META: &str = "resolved-call-edge-index-meta.json";
const LOAD_DIRTY_OVERRIDE: &str = "PRISM_NAV_EDGE_CACHE_LOAD_DIRTY";

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

````

with this:

````rust
const NAV_CALL_EDGE_CACHE_VERSION: u32 = 65;
const CACHE_BIN: &str = "resolved-call-edge-index.bin";
const CACHE_META: &str = "resolved-call-edge-index-meta.json";
pub(crate) const LOAD_DIRTY_OVERRIDE: &str = "PRISM_NAV_EDGE_CACHE_LOAD_DIRTY";

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t5.log" 2>&1
```

Expected: `test result: ok. 556 passed; 0 failed` (547 + 9), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/shim/ src/navigation/call_edge_cache.rs
git commit -m "feat(mcp): spawning the shared daemon, the fail-closed line and the shim's pump"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::shim::`, see the named test fail, then `git checkout -- <file>`.

| Mutant | Edit | Fails |
|---|---|---|
| 5A | in `spawn.rs`, delete `.env_remove(LOAD_DIRTY_OVERRIDE)` | `the_daemon_s_environment_drops_the_dirty_sidecar_override_and_names_the_runtime_dir` |
| 5B | in `key_files.rs`, `open_log`: `libc::O_WRONLY \| libc::O_APPEND,` → `libc::O_WRONLY,` | `the_log_is_opened_for_appending` |
| 5D | in `pump.rs`, delete the arm `Ok(0) if input_closed.load(Ordering::SeqCst) => return PumpEnd::ClientClosed,` | `bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly` |
| 5E | in `pump.rs`, replace `let _ = to_daemon.shutdown(Shutdown::Write);` by `let _ = Shutdown::Write; std::mem::forget(to_daemon);` | `bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly` (the daemon never sees the end of input) |
| 5F | in `diagnostic.rs`, `one_line`: `if c.is_control() {` → `if false {` | `control_characters_in_paths_cannot_end_the_line` |
| 5G | in `diagnostic.rs`, `fail_closed_line`: `pid_file(paths.pid_file()),` → `paths.pid_file().display(),` | `the_line_names_everything_an_operator_needs_on_one_line`, `an_empty_partial_or_missing_pid_file_is_reported_as_such` |

### Task 6: `prism-mcp --shared`

**Files:**
- Modify: `src/mcp/shim/mod.rs`, `src/mcp/mod.rs`, `src/bin/prism-mcp.rs`, `tests/mcp/main.rs`, `tests/mcp/daemon_process_test.rs`
- Create: `src/mcp/shim/run.rs`, `tests/mcp/shim_process_test.rs`, `tests/mcp/shim_session_process_test.rs`

**Interfaces:**
- Consumes: everything above; `canonical_config` (`session.rs`, `pub(super)`); `DaemonIdentity::resolve` (S2a); `ClientSettings::new`, `ConnectionSettings::from_env` (S1, S2b); `runtime_dir` (S2a); S4's process-test helpers (`setup`, `Setup`, `Daemon`, `wait_until`, `gone`, `HANG`, `INIT`, `INITED`, `CALLEES`), now `pub(crate)`; `transcript::{drive, script, write_fixture, spawn_line_reader, RESPONSE_TIMEOUT}` (S1).
- Produces: `prism::mcp::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS}`: `pub struct ShimOptions { pub config: ServerConfig, pub start_timeout: Duration, pub idle_ttl: Option<u64>, pub max_clients: Option<u32> }`, `pub fn run_shim(options: ShimOptions) -> i32` (0 when the client closed the session; 1, with one line on standard error, otherwise); the binary's `--shared`, `--standalone`, `--daemon-start-timeout`, `--idle-ttl`, `--max-clients`. Test support: `ShimProcess` (`start`, `start_with_cache`, `send`, `request`, `handshake`, `close_input`, `stderr`, `ended`, `daemon_pid`), `Daemons`, `end_daemons`, `owner_is_free`, `pid_file_pid`, `kill`, `owner_lock_held`.

- [ ] **Step 1: Write the failing tests**

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust
//! `prism-mcp daemon` as a process (spec §3.1, §4.2–§4.4; the daemon's halves of §9 tests 2, 3,
//! 4, 8 and 15, which the shim of slice S5a completes). Each test runs the real binary with a
//! runtime directory of its own and speaks the preamble to it as a shim will.
#![cfg(feature = "mcp")]

````

with this:

````rust
//! `prism-mcp daemon` as a process (spec §3.1, §4.2–§4.4; the daemon's halves of §9 tests 2, 3,
//! 4, 8 and 15, which the shim's process tests complete; they share this file's helpers). Each test runs the real binary with a
//! runtime directory of its own and speaks the preamble to it as a shim will.
#![cfg(feature = "mcp")]

````

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const HANG: Duration = Duration::from_secs(60);
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let give_up = Instant::now() + HANG;
    while !condition() {
        assert!(Instant::now() < give_up, "gave up waiting for: {what}");
````

with this:

````rust
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) const HANG: Duration = Duration::from_secs(60);
pub(crate) const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
pub(crate) const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
pub(crate) const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;

pub(crate) fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let give_up = Instant::now() + HANG;
    while !condition() {
        assert!(Instant::now() < give_up, "gave up waiting for: {what}");
````

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust

/// A repository, a cache base and a runtime base of the test's own, and the key and paths a
/// daemon for them has.
struct Setup {
    repo: tempfile::TempDir,
    cache: tempfile::TempDir,
    base: tempfile::TempDir,
    key: IdentityKey,
    paths: DaemonPaths,
}

fn setup() -> Setup {
    let repo = tempfile::tempdir().expect("a repository");
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n")
        .expect("write util.py");
````

with this:

````rust

/// A repository, a cache base and a runtime base of the test's own, and the key and paths a
/// daemon for them has.
pub(crate) struct Setup {
    pub(crate) repo: tempfile::TempDir,
    pub(crate) cache: tempfile::TempDir,
    pub(crate) base: tempfile::TempDir,
    pub(crate) key: IdentityKey,
    pub(crate) paths: DaemonPaths,
}

pub(crate) fn setup() -> Setup {
    let repo = tempfile::tempdir().expect("a repository");
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n")
        .expect("write util.py");
````

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust
}

/// A daemon process, killed and reaped when the test ends however it ends.
struct Daemon {
    child: Child,
    stderr: Option<JoinHandle<String>>,
}
````

with this:

````rust
}

/// A daemon process, killed and reaped when the test ends however it ends.
pub(crate) struct Daemon {
    child: Child,
    stderr: Option<JoinHandle<String>>,
}
````

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust

impl Daemon {
    /// Start a daemon whose standard error the test reads.
    fn start(setup: &Setup, extra: &[&str]) -> Self {
        let mut daemon = Self::spawn(setup, extra, Stdio::piped());
        let mut stderr = daemon
            .child
````

with this:

````rust

impl Daemon {
    /// Start a daemon whose standard error the test reads.
    pub(crate) fn start(setup: &Setup, extra: &[&str]) -> Self {
        let mut daemon = Self::spawn(setup, extra, Stdio::piped());
        let mut stderr = daemon
            .child
````

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().expect("the daemon's status")
    }

    /// Wait for the process to end; return how it ended and its standard error.
    fn ended(&mut self) -> (ExitStatus, String) {
        let mut status = None;
        wait_until("the daemon process to end", || {
            status = self.exited();
````

with this:

````rust
        }
    }

    pub(crate) fn pid(&self) -> u32 {
        self.child.id()
    }

    pub(crate) fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().expect("the daemon's status")
    }

    /// Wait for the process to end; return how it ended and its standard error.
    pub(crate) fn ended(&mut self) -> (ExitStatus, String) {
        let mut status = None;
        wait_until("the daemon process to end", || {
            status = self.exited();
````

In `tests/mcp/daemon_process_test.rs`, replace this:

````rust
    }
}

fn gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err()
}

````

with this:

````rust
    }
}

pub(crate) fn gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err()
}

````

In `tests/mcp/main.rs`, replace this:

````rust
mod golden_transcript_test;
mod mem_report_test;
mod settings_test;
mod smoke_test;
mod transcript;
mod transcript_driver_test;
````

with this:

````rust
mod golden_transcript_test;
mod mem_report_test;
mod settings_test;
mod shim_process_test;
mod shim_session_process_test;
mod smoke_test;
mod transcript;
mod transcript_driver_test;
````

Create `tests/mcp/shim_process_test.rs`:

````rust
//! `prism-mcp --shared` as a process: the shim (spec §4.1, §4.6, §10; §9 tests 5, 10 and 11,
//! and the shim's halves of tests 3, 4 and 12). Each test runs the real binary with a runtime
//! directory of its own; the daemons a shim starts are detached from it, so every test ends them
//! itself (`Daemons`), however it ends.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{gone, setup, wait_until, Setup, CALLEES, HANG, INIT, INITED};
use crate::transcript::spawn_line_reader;
use prism::mcp::shared::paths::DaemonPaths;
use prism::mcp::shared::preamble::Reply;
use prism::mcp::shared::runtime_dir::{validate_runtime_dir, RUNTIME_DIR_ENV};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A shim process with piped stdio; its standard error goes to a file the test reads.
pub(crate) struct ShimProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: mpsc::Receiver<std::io::Result<Vec<u8>>>,
    stderr_path: std::path::PathBuf,
    _stderr_dir: tempfile::TempDir,
}

impl Drop for ShimProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl ShimProcess {
    /// `prism-mcp --repo R --cache-dir C --shared <extra>`, with the setup's runtime base and
    /// `env`.
    pub(crate) fn start(setup: &Setup, extra: &[&str], env: &[(&str, &str)]) -> Self {
        Self::start_with_cache(setup, setup.cache.path(), extra, env)
    }

    pub(crate) fn start_with_cache(
        setup: &Setup,
        cache: &std::path::Path,
        extra: &[&str],
        env: &[(&str, &str)],
    ) -> Self {
        let stderr_dir = tempfile::tempdir().expect("a directory for standard error");
        let stderr_path = stderr_dir.path().join("stderr.txt");
        let mut command = Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
        command
            .arg("--repo")
            .arg(setup.repo.path())
            .arg("--cache-dir")
            .arg(cache)
            .arg("--shared")
            .args(extra)
            .env(RUNTIME_DIR_ENV, setup.base.path())
            .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
            .env_remove("PRISM_MCP_CONCISE_SHAPE")
            .env_remove("PRISM_MCP_MAX_RESULT_CHARS");
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(
                File::create(&stderr_path).expect("stderr file"),
            ))
            .spawn()
            .expect("start the shim");
        let stdin = child.stdin.take();
        let (lines, _reader) = spawn_line_reader(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            lines,
            stderr_path,
            _stderr_dir: stderr_dir,
        }
    }

    pub(crate) fn send(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{line}").expect("write to the shim");
        stdin.flush().expect("flush");
    }

    pub(crate) fn request(&mut self, line: &str) -> serde_json::Value {
        self.send(line);
        match self.lines.recv_timeout(HANG) {
            Ok(Ok(response)) => serde_json::from_slice(&response).expect("a JSON response"),
            other => panic!(
                "no response from the shim: {other:?}; stderr: {}",
                self.stderr()
            ),
        }
    }

    /// `initialize`, then `notifications/initialized`.
    pub(crate) fn handshake(&mut self) {
        let initialized = self.request(INIT);
        assert!(initialized["result"].is_object(), "{initialized}");
        self.send(INITED);
    }

    pub(crate) fn close_input(&mut self) {
        self.stdin = None;
    }

    pub(crate) fn stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }

    /// Wait for the shim to end; return how it ended and its standard error.
    pub(crate) fn ended(&mut self) -> (ExitStatus, String) {
        let mut status = None;
        wait_until("the shim to end", || {
            status = self.child.try_wait().expect("the shim's status");
            status.is_some()
        });
        (status.expect("ended"), self.stderr())
    }

    /// The pid of the daemon the shim reports it reached.
    pub(crate) fn daemon_pid(&self) -> u32 {
        let stderr = self.stderr();
        let rest = stderr
            .split("shared daemon pid ")
            .nth(1)
            .unwrap_or_else(|| panic!("no daemon reached: {stderr}"));
        rest.split(' ')
            .next()
            .and_then(|pid| pid.parse().ok())
            .expect("a pid")
    }
}

/// Ends, on drop, whatever daemon owns the setup's key: the daemons a shim starts are not the
/// test's children, so nothing else would.
pub(crate) struct Daemons<'a>(pub(crate) &'a DaemonPaths);

impl Drop for Daemons<'_> {
    fn drop(&mut self) {
        end_daemons(self.0);
    }
}

/// Whether a process of this test could take the owner lock now.
pub(crate) fn owner_is_free(paths: &DaemonPaths) -> bool {
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(paths.owner_lock())
    {
        Ok(lock) => lock.try_lock().is_ok(),
        Err(_) => false,
    }
}

/// The pid the pid file names, when it is whole.
pub(crate) fn pid_file_pid(paths: &DaemonPaths) -> Option<u32> {
    let text = std::fs::read_to_string(paths.pid_file()).ok()?;
    text.lines().next()?.parse().ok()
}

pub(crate) fn kill(pid: u32, signal: libc::c_int) {
    let pid = libc::pid_t::try_from(pid).expect("a pid");
    // SAFETY: `kill` with a valid signal number has no memory-safety preconditions.
    unsafe { libc::kill(pid, signal) };
}

/// Kill the daemon that owns the key, if one does, and wait until the key is free.
pub(crate) fn end_daemons(paths: &DaemonPaths) {
    if !paths.runtime_dir().exists() {
        return;
    }
    let give_up = std::time::Instant::now() + HANG;
    while !owner_is_free(paths) && std::time::Instant::now() < give_up {
        if let Some(pid) = pid_file_pid(paths) {
            kill(pid, libc::SIGKILL);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}

/// Spec §4.1 and §3: a shim with no daemon running starts one, detached in a session of its own,
/// with its standard error appended to the key's log; serves a session through it; and exits 0
/// when its client closes its input. The pid it reports is the one in the pid file.
#[test]
fn a_shim_starts_a_detached_daemon_and_serves_a_session_through_it() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    std::fs::write(setup.paths.log_file(), "an earlier daemon's line\n").expect("a log");
    let mut shim = ShimProcess::start(&setup, &["--idle-ttl", "30"], &[]);
    shim.handshake();
    let answer = shim.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    let pid = shim.daemon_pid();
    assert_eq!(pid_file_pid(&setup.paths), Some(pid));
    let sid = {
        let pid = libc::pid_t::try_from(pid).expect("a pid");
        // SAFETY: `getsid` takes a pid and touches no memory.
        unsafe { libc::getsid(pid) }
    };
    assert_eq!(
        sid,
        libc::pid_t::try_from(pid).expect("a pid"),
        "its own session"
    );
    let log = std::fs::read_to_string(setup.paths.log_file()).expect("the log");
    assert!(log.starts_with("an earlier daemon's line\n"), "{log}");
    assert!(log.contains(&format!("pid {pid} serves")), "{log}");

    shim.close_input();
    let (status, stderr) = shim.ended();
    assert!(status.success(), "{status:?} {stderr}");
}

/// Spec §9 test 5: a runtime directory that fails validation (a symlink, or a directory other
/// users can read) fails the shim closed: a non-zero exit, one line on standard error naming the
/// cause and the `--standalone` invocation, no daemon, and no cache directory created anywhere.
#[test]
fn a_runtime_directory_that_fails_validation_fails_closed_and_creates_nothing() {
    for bad in ["symlink", "mode"] {
        let setup = setup();
        let dir = setup.paths.runtime_dir();
        match bad {
            "symlink" => {
                let elsewhere = setup.base.path().join("elsewhere");
                std::fs::create_dir(&elsewhere).expect("a directory");
                std::os::unix::fs::symlink(&elsewhere, dir).expect("a symlink");
            }
            _ => {
                std::fs::create_dir(dir).expect("the runtime directory");
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))
                    .expect("mode 0755");
            }
        }
        let cache = setup.cache.path().join("never-created");
        let mut shim = ShimProcess::start_with_cache(&setup, &cache, &[], &[]);
        let (status, stderr) = shim.ended();
        assert!(!status.success(), "{bad}: {status:?}");
        assert_eq!(stderr.lines().count(), 1, "{bad}: {stderr}");
        assert!(stderr.contains("failed closed"), "{bad}: {stderr}");
        assert!(stderr.contains("--standalone"), "{bad}: {stderr}");
        let cause = if bad == "symlink" {
            "symlink"
        } else {
            "mode 0755"
        };
        assert!(stderr.contains(cause), "{bad}: {stderr}");
        assert!(!cache.exists(), "{bad}: a cache directory was created");
        assert!(gone(
            &dir.join(setup.paths.socket().file_name().expect("name"))
        ));
    }
}

/// Spec §9 test 10: a daemon at the key's socket that answers `MISMATCH` (here a fixture that
/// owns the key, standing in for a daemon of another build) fails the shim closed, naming the
/// identity it serves, and nothing replaces it.
#[test]
fn a_daemon_that_answers_mismatch_fails_the_shim_closed() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = owner_lock_held(&setup.paths);
    let listener = UnixListener::bind(setup.paths.socket()).expect("the fixture's socket");
    let socket = std::fs::symlink_metadata(setup.paths.socket())
        .expect("socket")
        .ino();
    let expected = "f".repeat(64);
    let fixture = {
        let expected = expected.clone();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let _ = read_line(&stream);
            let reply = Reply::Mismatch {
                expected: prism::mcp::shared::identity::IdentityKey::from_hex(&expected)
                    .expect("a key"),
            };
            let _ = stream.write_all(reply.encode().as_bytes());
        })
    };
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    let (status, stderr) = shim.ended();
    assert!(!status.success());
    assert!(stderr.contains("MISMATCH"), "{stderr}");
    assert!(stderr.contains(&expected), "{stderr}");
    fixture.join().expect("the fixture");
    assert_eq!(
        std::fs::symlink_metadata(setup.paths.socket())
            .expect("socket")
            .ino(),
        socket
    );
    drop::<File>(owner);
}

/// Spec §9 test 11 and §4.1 step 9: a daemon killed after MCP bytes have been forwarded ends the
/// shim with a non-zero exit, and the shim does not reconnect: no daemon owns the key afterwards,
/// and the pid file still names the killed one.
#[test]
fn a_daemon_killed_mid_session_ends_the_shim_without_a_reconnect() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    let pid = shim.daemon_pid();
    kill(pid, libc::SIGKILL);
    let (status, stderr) = shim.ended();
    assert!(!status.success(), "{status:?}");
    assert!(stderr.contains("does not reconnect"), "{stderr}");
    assert!(owner_is_free(&setup.paths), "a daemon was started again");
    assert_eq!(pid_file_pid(&setup.paths), Some(pid));
}

/// Spec §9 test 3, the shim's half: a socket and a pid file left by a dead daemon are replaced:
/// the shim starts a daemon that serves on the same path.
#[test]
fn a_stale_socket_left_by_a_dead_daemon_is_replaced() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    drop(UnixListener::bind(setup.paths.socket()).expect("a stale socket"));
    std::fs::write(setup.paths.pid_file(), "999999\n1\nold\n").expect("a stale pid file");
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    let answer = shim.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(pid_file_pid(&setup.paths), Some(shim.daemon_pid()));
}

/// Spec §9 test 4, the shim's half: a live daemon (it holds the owner lock) that accepts and
/// never answers fails the shim closed at its deadline, naming the pid and start time from the
/// pid file; its socket is not unlinked and no daemon is started in its place.
#[test]
fn a_live_but_wedged_owner_fails_the_shim_closed_naming_its_pid() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = owner_lock_held(&setup.paths);
    let listener = UnixListener::bind(setup.paths.socket()).expect("the wedged socket");
    let socket = std::fs::symlink_metadata(setup.paths.socket())
        .expect("socket")
        .ino();
    std::fs::write(setup.paths.pid_file(), "4242\n1700000000\nwedged\n").expect("pid file");
    // The fixture accepts until it is told to stop, or its sender is dropped by a failing test;
    // it never blocks in `accept`, so joining it cannot hang.
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    listener.set_nonblocking(true).expect("nonblocking");
    let fixture = std::thread::spawn(move || {
        let mut held = Vec::new();
        while let Err(mpsc::TryRecvError::Empty) = stop_rx.try_recv() {
            if let Ok((stream, _)) = listener.accept() {
                held.push(stream);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
    let mut shim = ShimProcess::start(&setup, &["--daemon-start-timeout", "2"], &[]);
    let (status, stderr) = shim.ended();
    let _ = stop_tx.send(());
    fixture.join().expect("the fixture");
    assert!(!status.success());
    assert!(
        stderr.contains("pid 4242, start time 1700000000"),
        "{stderr}"
    );
    assert_eq!(
        std::fs::symlink_metadata(setup.paths.socket())
            .expect("socket")
            .ino(),
        socket
    );
    assert_eq!(
        std::fs::read_to_string(setup.paths.pid_file()).expect("pid file"),
        "4242\n1700000000\nwedged\n"
    );
    drop::<File>(owner);
}

/// Spec §9 test 12, the shim's half ("the next shim restarts it"): after the daemon dies (here
/// killed; S3b's process test aborts one by an injected panic), the next shim finds the key free,
/// replaces the dead daemon's files and starts a daemon that serves it.
#[test]
fn the_next_shim_restarts_a_daemon_that_died() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let mut first = ShimProcess::start(&setup, &[], &[]);
    first.handshake();
    assert_eq!(first.request(CALLEES)["result"]["isError"], false);
    let dead = first.daemon_pid();
    kill(dead, libc::SIGKILL);
    let _ = first.ended();

    let mut next = ShimProcess::start(&setup, &[], &[]);
    next.handshake();
    assert_eq!(next.request(CALLEES)["result"]["isError"], false);
    assert_ne!(next.daemon_pid(), dead);
}

/// S3b's SI-10: a key whose last daemon died during its index build moments ago (its build
/// marker left) is not restarted: the shim fails closed naming the log and leaves the files. A
/// minute after that build began, the next shim starts a daemon again and the marker is gone.
#[test]
fn a_daemon_that_died_during_its_build_is_not_restarted_for_a_minute() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    drop(UnixListener::bind(setup.paths.socket()).expect("the dead daemon's socket"));
    std::fs::write(setup.paths.pid_file(), "4242\n1700000000\nb\n").expect("pid file");
    std::fs::write(
        setup.paths.build_marker(),
        format!("4242\n{}\n", unix_now() - 5),
    )
    .expect("a build marker");
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    let (status, stderr) = shim.ended();
    assert!(!status.success());
    assert!(stderr.contains("died during its index build"), "{stderr}");
    assert!(
        stderr.contains(&setup.paths.log_file().display().to_string()),
        "{stderr}"
    );
    assert!(setup.paths.socket().exists() && setup.paths.pid_file().exists());
    assert!(owner_is_free(&setup.paths));

    std::fs::write(
        setup.paths.build_marker(),
        format!("4242\n{}\n", unix_now() - 120),
    )
    .expect("an old build marker");
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    assert_eq!(shim.request(CALLEES)["result"]["isError"], false);
    assert!(!setup.paths.build_marker().exists());
}

/// Hold the key's owner lock, as a live daemon does.
pub(crate) fn owner_lock_held(paths: &DaemonPaths) -> File {
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(paths.owner_lock())
        .expect("the owner lock file");
    owner.try_lock().expect("the test owns the key");
    owner
}

/// One line from a stream, newline included, read one byte at a time.
pub(crate) fn read_line(mut stream: &UnixStream) -> String {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while let Ok(1) = stream.read(&mut byte) {
        line.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8_lossy(&line).into_owned()
}
````

Create `tests/mcp/shim_session_process_test.rs`:

````rust
//! Several `prism-mcp --shared` processes and one daemon (spec §4.1 step 6, §4.6, §5; the shim's
//! halves of §9 tests 8 and 15; tests 7 and 9 through real shims), and the spec §5 equivalence:
//! one scripted session, run standalone and through a shim, gives the same bytes.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{setup, Daemon, CALLEES};
use crate::shim_process_test::{end_daemons, owner_is_free, Daemons, ShimProcess};
use crate::transcript::{drive, script, write_fixture, RESPONSE_TIMEOUT};
use prism::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use std::process::Command;

/// Spec §9 tests 8 and 15, the shim's halves: eight shims start at once while a daemon started on
/// its own (as a dead starter's child would be) starts too. All eight are admitted by one daemon
/// and each is served; every other daemon process ends having served nothing.
#[test]
fn shims_started_together_with_an_orphan_daemon_share_one_daemon() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let key = setup.key.to_hex();
    let mut orphan = Daemon::start(&setup, &["--key", &key]);
    let mut shims: Vec<ShimProcess> = (0..8)
        .map(|_| ShimProcess::start(&setup, &[], &[]))
        .collect();
    for shim in &mut shims {
        shim.handshake();
        let answer = shim.request(CALLEES);
        assert_eq!(answer["result"]["isError"], false, "{answer}");
    }
    let pids: Vec<u32> = shims.iter().map(ShimProcess::daemon_pid).collect();
    assert!(pids.iter().all(|pid| *pid == pids[0]), "{pids:?}");
    if orphan.pid() != pids[0] {
        let (status, stderr) = orphan.ended();
        assert!(status.success(), "{status:?} {stderr}");
        assert_eq!(stderr, "", "the orphan lost the key and left silently");
    }
    drop(shims);
    end_daemons(&setup.paths);
    assert!(owner_is_free(&setup.paths));
}

/// Spec §9 test 9 through real shims: two shims whose environments ask for different output
/// settings are served by one daemon, each in its own shape (the default path omits
/// `structuredContent`; `always` keeps it).
#[test]
fn two_shims_with_different_output_settings_get_their_own_shapes_from_one_daemon() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let mut always = ShimProcess::start(&setup, &[], &[("PRISM_MCP_STRUCTURED_CONTENT", "always")]);
    let mut default = ShimProcess::start(&setup, &[], &[]);
    always.handshake();
    default.handshake();
    let with = always.request(CALLEES);
    let without = default.request(CALLEES);
    assert_eq!(always.daemon_pid(), default.daemon_pid());
    assert!(with["result"]["structuredContent"].is_object(), "{with}");
    assert!(
        without["result"].get("structuredContent").is_none(),
        "{without}"
    );
}

/// Spec §5 ("preserved, for the same indexed snapshot, the same connection settings and the same
/// readiness state") and the lane README's note for S5a: the transcript script (framing errors,
/// the handshake, every tool, an edit, a refresh) run through a shim gives byte for byte what the
/// same script gives a standalone lazy server with a cache, for the default settings and for
/// `always`/`legacy`.
#[test]
fn a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte() {
    for env in [
        &[][..],
        &[
            ("PRISM_MCP_STRUCTURED_CONTENT", "always"),
            ("PRISM_MCP_CONCISE_SHAPE", "legacy"),
        ][..],
    ] {
        let transcript = |shared: bool| {
            let repo = tempfile::tempdir().expect("a repository");
            write_fixture(repo.path());
            let cache = tempfile::tempdir().expect("a cache");
            let base = tempfile::Builder::new()
                .prefix("s5a")
                .tempdir_in("/tmp")
                .expect("a runtime base");
            let mut command = Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
            command
                .arg("--repo")
                .arg(repo.path())
                .arg("--cache-dir")
                .arg(cache.path())
                .args(["--first-call-wait", "60"])
                .arg(if shared { "--shared" } else { "--standalone" })
                .env(RUNTIME_DIR_ENV, base.path());
            for name in [
                "PRISM_MCP_STRUCTURED_CONTENT",
                "PRISM_MCP_CONCISE_SHAPE",
                "PRISM_MCP_MAX_RESULT_CHARS",
            ] {
                command.env_remove(name);
            }
            for (name, value) in env {
                command.env(name, value);
            }
            let label = if shared { "shared" } else { "standalone" };
            let output = drive(command, label, repo.path(), &script(), RESPONSE_TIMEOUT);
            // The daemon of a shared run is ended with its runtime directory.
            let key = {
                let mut cfg = prism::mcp::ServerConfig::new(repo.path().to_path_buf());
                cfg.cache = prism::mcp::CacheMode::Dir(cache.path().to_path_buf());
                prism::mcp::shared::identity::DaemonIdentity::resolve(&cfg)
                    .expect("an identity")
                    .key()
            };
            let paths = prism::mcp::shared::paths::DaemonPaths::new(
                &base
                    .path()
                    .join(prism::mcp::shared::runtime_dir::RUNTIME_DIR_NAME),
                &key,
            )
            .expect("paths");
            end_daemons(&paths);
            output.unwrap_or_else(|failure| panic!("{failure}"))
        };
        let standalone = transcript(false);
        let shared = transcript(true);
        assert!(
            standalone == shared,
            "{env:?}: the transcripts differ\nstandalone:\n{}\nshared:\n{}",
            String::from_utf8_lossy(&standalone),
            String::from_utf8_lossy(&shared)
        );
    }
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/t6-red.log" 2>&1
```

Expected in the log: `test result: FAILED. 31 passed; 11 failed`: every new shim test fails, each with `error: unexpected argument '--shared' found` on the shim's standard error (the 31 are the base's tests).

- [ ] **Step 3: Implement**

In `src/bin/prism-mcp.rs`, replace this:

````rust
    /// Measurement only: print a workload for this many functions and exit.
    #[arg(long, hide = true, value_parser = clap::value_parser!(u64).range(1..=10000), conflicts_with_all = ["mem_workload", "mem_refresh"])]
    mem_emit_workload: Option<u64>,
}

/// `prism-mcp daemon ...`: the shared per-repository daemon (spec §3.1, §4.2). Internal: the
````

with this:

````rust
    /// Measurement only: print a workload for this many functions and exit.
    #[arg(long, hide = true, value_parser = clap::value_parser!(u64).range(1..=10000), conflicts_with_all = ["mem_workload", "mem_refresh"])]
    mem_emit_workload: Option<u64>,
    /// Use the shared per-repository daemon instead of an index of this process's own; starts it
    /// when none is running, and fails closed when it cannot be reached.
    #[arg(long, conflicts_with_all = ["standalone", "no_cache", "eager", "owner_compiler", "owner_config"])]
    shared: bool,
    /// Serve from an index of this process's own (the default).
    #[arg(long)]
    standalone: bool,
    /// With --shared: seconds to reach a ready daemon, starting one if needed (default 10).
    #[arg(long, requires = "shared", value_parser = clap::value_parser!(u64).range(1..=600))]
    daemon_start_timeout: Option<u64>,
    /// With --shared: idle seconds before a daemon this shim starts retires (default 900).
    #[arg(long, requires = "shared", value_parser = clap::value_parser!(u64).range(1..=86_400))]
    idle_ttl: Option<u64>,
    /// With --shared: connections a daemon this shim starts accepts at once (default 64).
    #[arg(long, requires = "shared", value_parser = clap::value_parser!(u32).range(1..=4096))]
    max_clients: Option<u32>,
}

/// `prism-mcp daemon ...`: the shared per-repository daemon (spec §3.1, §4.2). Internal: the
````

In `src/bin/prism-mcp.rs`, replace this:

````rust
        return Ok(stderr.flush()?);
    }

    match owner {
        Some(owner) => prism::mcp::run_with_owner(cfg, owner),
        None => prism::mcp::run(cfg),
````

with this:

````rust
        return Ok(stderr.flush()?);
    }

    if c.shared {
        std::process::exit(prism::mcp::run_shim(prism::mcp::ShimOptions {
            config: cfg,
            start_timeout: std::time::Duration::from_secs(
                c.daemon_start_timeout
                    .unwrap_or(prism::mcp::DEFAULT_DAEMON_START_TIMEOUT_SECS),
            ),
            idle_ttl: c.idle_ttl,
            max_clients: c.max_clients,
        }));
    }

    match owner {
        Some(owner) => prism::mcp::run_with_owner(cfg, owner),
        None => prism::mcp::run(cfg),
````

In `src/mcp/mod.rs`, replace this:

````rust
    CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider, StartupMode,
    FIRST_CALL_WAIT_MAX,
};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
````

with this:

````rust
    CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider, StartupMode,
    FIRST_CALL_WAIT_MAX,
};
#[cfg(unix)]
pub use shim::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
````

In `src/mcp/shim/mod.rs`, replace this:

````rust
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6, §10.
// The pieces are wired into a production caller by `run_shim` (Task 6), which removes this.
#![allow(dead_code)]

mod arbitrate;
mod connect;
mod diagnostic;
mod key_files;
mod pump;
#[cfg(test)]
mod shim_support;
mod spawn;
````

with this:

````rust
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6, §10.

mod arbitrate;
mod connect;
mod diagnostic;
mod key_files;
mod pump;
mod run;
#[cfg(test)]
mod shim_support;
mod spawn;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
````

Create `src/mcp/shim/run.rs`:

````rust
//! `prism-mcp --repo X --shared`: the shim's process (spec §4.1). Resolve the configuration, the
//! connection settings and the key exactly as a standalone server would; find or start the
//! daemon before one deadline; then pump. Every failure is one line on standard error and a
//! non-zero exit; nothing builds a private index.
use super::arbitrate::Arbiter;
use super::diagnostic::{fail_closed_line, one_line, standalone_invocation, Where};
use super::pump::{pump, PumpEnd};
use super::spawn::{DaemonArgs, ProcessStarter};
use crate::mcp::daemon::peer::peer_uid;
use crate::mcp::session::canonical_config;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::shared::identity::DaemonIdentity;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Hello;
use crate::mcp::shared::runtime_dir::runtime_dir;
use crate::mcp::{CacheMode, ServerConfig, StartupMode};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// `--daemon-start-timeout` default, seconds (spec §4.1 step 4, §8 step 4).
pub const DEFAULT_DAEMON_START_TIMEOUT_SECS: u64 = 10;

/// What `prism-mcp --shared` was started with.
#[derive(Clone, Debug)]
pub struct ShimOptions {
    /// The repository, the cache (`Default` or a directory; never `NoCache`), the refresh
    /// policy, `startup` (`Lazy`, or `Background` for `--warm-at-startup`; never `Eager`) and
    /// the first-call wait, as for a standalone server.
    pub config: ServerConfig,
    /// `--daemon-start-timeout`: the one deadline from here to `READY`.
    pub start_timeout: Duration,
    /// `--idle-ttl` and `--max-clients` for a daemon this shim starts.
    pub idle_ttl: Option<u64>,
    pub max_clients: Option<u32>,
}

/// Run a shim until its client or its daemon ends the session. Returns the exit status: 0 when
/// the client closed the session, 1 otherwise, with one line on standard error.
pub fn run_shim(options: ShimOptions) -> i32 {
    match shim(&options) {
        Ok(()) => 0,
        Err(line) => {
            eprintln!("{line}");
            1
        }
    }
}

fn shim(options: &ShimOptions) -> Result<(), String> {
    let error = |error: &dyn std::fmt::Display| one_line(&format!("prism-mcp: {error}"));
    // Spec §4.1 step 2: never a silent downgrade. The command line rejects these first.
    if matches!(options.config.cache, CacheMode::NoCache) {
        return Err(error(
            &"--shared needs a cache; --no-cache is served in process",
        ));
    }
    if options.config.startup == StartupMode::Eager {
        return Err(error(
            &"--shared cannot build eagerly; --eager is served in process",
        ));
    }
    let cfg = canonical_config(&options.config).map_err(|e| error(&e))?;
    let identity = DaemonIdentity::resolve(&cfg).map_err(|e| error(&e))?;
    let key = identity.key();
    // Spec §4.1 step 3: this shim's own settings, from its own environment and flags; an
    // invalid value warns here, as it does for a standalone server.
    let settings = ClientSettings::new(
        ConnectionSettings::from_env(),
        cfg.first_call_wait.as_secs(),
        cfg.startup == StartupMode::Background,
    )
    .map_err(|e| error(&e))?;
    let hello = Hello {
        identity: key,
        settings,
    }
    .encode();
    // Spec §4.1 step 4: the one deadline, from here to `READY`.
    let deadline = Instant::now() + options.start_timeout;
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("prism-mcp"));
    let standalone = standalone_invocation(
        &exe,
        &ServerConfig {
            repo_root: cfg.repo_root.clone(),
            ..options.config.clone()
        },
    );
    let fail = |reason: &dyn std::fmt::Display, at: Where<'_>| {
        fail_closed_line(&reason.to_string(), &at, &standalone)
    };
    let dir = runtime_dir().map_err(|e| {
        fail(
            &e,
            Where {
                key: &key,
                runtime_dir: None,
                paths: None,
            },
        )
    })?;
    let paths = DaemonPaths::new(&dir, &key).map_err(|e| {
        fail(
            &e,
            Where {
                key: &key,
                runtime_dir: Some(&dir),
                paths: None,
            },
        )
    })?;
    let at = || Where {
        key: &key,
        runtime_dir: Some(&dir),
        paths: Some(&paths),
    };
    let mut starter = ProcessStarter {
        exe,
        args: DaemonArgs {
            repo_root: identity.repo_root().to_path_buf(),
            cache_base: identity.cache_base().to_path_buf(),
            refresh_policy: identity.refresh_policy(),
            key,
            warm_at_startup: cfg.startup == StartupMode::Background,
            idle_ttl: options.idle_ttl,
            max_clients: options.max_clients,
        },
        runtime_dir: dir.clone(),
    };
    let arbiter = Arbiter {
        paths: &paths,
        key,
        hello: &hello,
        deadline,
        peer_uid,
        hook: None,
    };
    let connected = arbiter
        .arbitrate(&mut starter)
        .map_err(|failure| fail(&failure, at()))?;
    let pid = connected.ready.pid;
    eprintln!(
        "{}",
        one_line(&format!(
            "prism-mcp: shared daemon pid {pid} (started {}, generation {}) at {}",
            connected.ready.start_time,
            connected.ready.generation,
            paths.socket().display()
        ))
    );
    if let Some(started) = connected.started {
        std::thread::spawn(move || started.reap());
    }
    match pump(
        std::io::stdin(),
        &mut std::io::stdout().lock(),
        connected.stream,
    ) {
        PumpEnd::ClientClosed => Ok(()),
        end => Err(one_line(&format!(
            "prism-mcp: {end} (shared daemon pid {pid}); this session cannot continue, and the \
             shim does not reconnect once MCP bytes have been forwarded"
        ))),
    }
}
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/t6.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check --features mcp > "$LOG_DIR/t6-check.log" 2>&1
pgrep -fl 'prism-mcp daemon'
```

Expected: `test result: ok. 42 passed; 0 failed` (31 + 11), with the goldens' test `ok`; the check `Finished` with no `warning` (the module's dead-code allowance is gone); `pgrep` prints nothing.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/shim/ src/mcp/mod.rs src/bin/prism-mcp.rs tests/mcp/
git commit -m "feat(mcp): prism-mcp --shared, the stdio shim of the shared daemon"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run the named test with `cargo test --features mcp --test mcp <name>`, see it fail, then `git checkout -- <file>`; afterwards `pgrep -fl 'prism-mcp daemon'` prints nothing.

| Mutant | Edit | Fails |
|---|---|---|
| 6A | in `spawn.rs`, the `pre_exec` closure: `if libc::setsid() == -1 {` → `if libc::getpid() == -1 {` (no session of its own) | `a_shim_starts_a_detached_daemon_and_serves_a_session_through_it` |
| 6B | in `run.rs`: `PumpEnd::ClientClosed => Ok(()),` → `PumpEnd::ClientClosed \| PumpEnd::DaemonClosed => Ok(()),` | `a_daemon_killed_mid_session_ends_the_shim_without_a_reconnect` |
| 6C | in `spawn.rs`: `.stdout(Stdio::from(log.try_clone()?))` → `.stdout(Stdio::inherit())` (the daemon holds the client's pipe) | `a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte` (its stdout never ends; the test then waits for the daemon to end, so kill the `prism-mcp daemon` it started rather than wait for the idle TTL) |
| 6D | in `run.rs`, fall back to a private index: replace the `let connected = arbiter.arbitrate(&mut starter).map_err(…)?;` statement by a `match` whose `Err(_)` arm runs `crate::mcp::run(cfg.clone())` and returns `Ok(())` | `a_runtime_directory_that_fails_validation_fails_closed_and_creates_nothing` (after the 60 s hang limit: the fallback server waits on its input) |

### Task 7: S5a-2 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
Startup modes: by default the repository index is built on the first valid `tools/call`
(`initialize`, `ping`, and `tools/list` never start it, so a client that only handshakes costs
a few MB); `--warm-at-startup` starts the background build at process start; `--eager` builds
before answering `initialize`. See `docs/MCP.md`.

The server exposes nine tools: seven read-only navigation tools (the six graph/evidence
queries return Prism `Evidence` JSON; `nav_symbol_spans` returns a dedicated coordinate
````

with this:

````markdown
Startup modes: by default the repository index is built on the first valid `tools/call`
(`initialize`, `ping`, and `tools/list` never start it, so a client that only handshakes costs
a few MB); `--warm-at-startup` starts the background build at process start; `--eager` builds
before answering `initialize`. See `docs/MCP.md`. `--shared` (opt-in, Unix) serves through the
shared per-repository daemon instead of an index of the process's own (the shim, below).

The server exposes nine tools: seven read-only navigation tools (the six graph/evidence
queries return Prism `Evidence` JSON; `nav_symbol_spans` returns a dedicated coordinate
````

In `CLAUDE.md`, replace this:

````markdown

Shared daemon (`src/mcp/daemon/`, Unix only; `prism-mcp daemon --repo X [--cache-dir D]
[--refresh-policy P] [--warm-at-startup] [--idle-ttl S] [--max-clients N] [--key HEX]`, internal,
started by the shim of a later slice): `coordinator.rs` holds `DaemonState`, which is a control plane
(`Mutex<ControlState>` and one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the
attempt count, the parked calls, the generation of the provider in the slot) and a provider slot
(`RwLock<Option<SessionProvider>>`). `ensure_ready` picks its build attempt once and waits for
````

with this:

````markdown

Shared daemon (`src/mcp/daemon/`, Unix only; `prism-mcp daemon --repo X [--cache-dir D]
[--refresh-policy P] [--warm-at-startup] [--idle-ttl S] [--max-clients N] [--key HEX]`, internal,
started by `prism-mcp --shared`): `coordinator.rs` holds `DaemonState`, which is a control plane
(`Mutex<ControlState>` and one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the
attempt count, the parked calls, the generation of the provider in the slot) and a provider slot
(`RwLock<Option<SessionProvider>>`). `ensure_ready` picks its build attempt once and waits for
````

In `CLAUDE.md`, replace this:

````markdown
around the builder by `run.rs::marked`, removed at release); a daemon that dies during its build
leaves it.

Shim (`src/mcp/shim/`, Unix only; no production caller yet): `connect.rs` makes one attempt to
reach a daemon: connect, check the peer's uid (`daemon::peer`; a peer of another uid is refused
before anything is sent), send `HELLO`, and read the one reply line with every read bounded by the
time left to the shim's one deadline. `READY` for this key reaches it (the socket's timeouts are
````

with this:

````markdown
around the builder by `run.rs::marked`, removed at release); a daemon that dies during its build
leaves it.

Shim (`src/mcp/shim/`, Unix only; `prism-mcp --repo X --shared [--daemon-start-timeout S]
[--idle-ttl S] [--max-clients N]`; `--shared` with `--no-cache`, `--eager` or `--owner-*` is a CLI
error, and `--standalone`, the default, changes nothing): `connect.rs` makes one attempt to
reach a daemon: connect, check the peer's uid (`daemon::peer`; a peer of another uid is refused
before anything is sent), send `HELLO`, and read the one reply line with every read bounded by the
time left to the shim's one deadline. `READY` for this key reaches it (the socket's timeouts are
````

In `CLAUDE.md`, replace this:

````markdown
before `READY` lost the key (look again), one that ends otherwise fails the start. A free key whose
build marker says a build began under 60 s ago is not restarted (the restart bound for a build that
keeps dying). `key_files.rs` opens the key's files through the runtime directory's descriptor with
`daemon::files`' functions.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

with this:

````markdown
before `READY` lost the key (look again), one that ends otherwise fails the start. A free key whose
build marker says a build began under 60 s ago is not restarted (the restart bound for a build that
keeps dying). `key_files.rs` opens the key's files through the runtime directory's descriptor with
`daemon::files`' functions. `spawn.rs` starts `prism-mcp daemon` from the same executable with the
shim's canonical root, cache base, refresh policy and full key (and `--warm-at-startup`,
`--idle-ttl`, `--max-clients` when the shim has them), in a session of its own (`setsid`), from `/`,
standard input null, output appended (`O_APPEND`) to `<key16>.log`, without
`PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`. `diagnostic.rs` writes the one fail-closed line (reason, key,
runtime dir, socket, pid file contents, log, and the `--standalone` invocation; control characters
escaped); exit 1. `pump.rs` carries the session byte for byte: the client's end of input
half-closes the socket and the daemon's last responses are still delivered (exit 0); a daemon that
closes first ends the session (exit 1) and the shim never reconnects. `run.rs` is `run_shim`; on
`READY` it writes `prism-mcp: shared daemon pid N (started T, generation G) at <socket>` to stderr.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

- [ ] **Step 2: Run the MCP surface and the checks**

The commands of Task 4 Step 2, with `s52` in the log names and the clippy `grep` extended to `src/bin/prism-mcp.rs`, `src/navigation/call_edge_cache.rs` and `tests/mcp/(shim|daemon_process|main)`. Expected: `cargo fmt --check` silent; `test result: ok. 556 passed; 0 failed` (517 + 39); `42 passed` with the goldens' test `ok`; both checks `Finished`, no `warning` in the feature check; clippy 257 `warning` lines; the `grep` prints one line, `src/navigation/call_edge_cache.rs:279:9`, a warning the base has (`io::Error::other`) on a line this slice does not touch.

- [ ] **Step 3: State the invariants**

The commands of Task 4 Step 3, except that the first no longer lists `src/bin`, and:

```bash
git diff origin/main -- src/bin/prism-mcp.rs > "$LOG_DIR/s52-bin.diff"
git diff origin/main -- src/navigation/call_edge_cache.rs > "$LOG_DIR/s52-nav.diff"
```

Expected: the first prints nothing; no file over 600 lines; `s52-bin.diff` adds the five flags to `Cli` and the `if c.shared { … }` branch before `match owner`, and nothing else; `s52-nav.diff` is one line, `const` → `pub(crate) const`.

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
for i in $(seq 1 20); do nice -n 19 cargo test --features mcp --lib mcp::shim > "$LOG_DIR/s52-rep-$i.log" 2>&1; done
for i in $(seq 1 10); do nice -n 19 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp shim > "$LOG_DIR/s52-proc-$i.log" 2>&1; done
grep -h '^test result' "$LOG_DIR"/s52-rep-*.log "$LOG_DIR"/s52-proc-*.log | sed 's/finished.*//' | sort | uniq -c
pgrep -fl 'prism-mcp daemon'
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s52-full.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s52-doc.log" 2>&1
```

Then the Linux commands of Task 4 Step 4, with `s52` in the log name and `--test mcp` repeated 10 times as well.

Expected: `20 test result: ok. 34 passed; 0 failed`, `10 test result: ok. 13 passed; 0 failed`; `pgrep` prints nothing; the full suite and the doc tests `0 failed`; on Linux `10 test result: ok. 34 passed; 0 failed`, `10 test result: ok. 133 passed; 0 failed` and `10 test result: ok. 42 passed; 0 failed`.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S5a-2 is the shim as a process: `prism-mcp --shared` reaches or starts the daemon, carries the session byte for byte, and fails closed in one line. (2) **Invariants:** Step 3. (3) **Decisions:** D84–D89, SI-25–SI-28. D85 (the daemon flags on the shim) and D88 (the stderr line on `READY`) are the ones a reviewer is most likely to want to change. (4) **Claims:** the rows for Tasks 5 and 6, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base (Linux included), and mutants 5A–6D as observed. (6) **Not verified:** x86_64 Linux before CI; descriptors leaked into the shim; SI-28. (7) **Tier-A:** not run (`src/navigation/call_edge_cache.rs` changes one visibility word; no behavior). (8) The owner questions of this plan, as defaults built.

```bash
git add CLAUDE.md
git commit -m "docs(mcp): prism-mcp --shared"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): prism-mcp --shared, the stdio shim (shared daemon S5a-2)" --body-file "$LOG_DIR/s52-pr-body.md"
```

Review brief and cap as in Task 4.

---

## Interfaces for the next slices

Everything below is in `crate::mcp::shim` and `pub(super)` unless a path says otherwise.

**S5b (`daemon-status`, `daemon-shutdown`) consumes:**

- **How to find and talk to a daemon:** `connect::reach` with a `Target` is one preamble exchange with the peer check (D78); a status or shutdown verb is a new line after, or instead of, `HELLO`, and by S2's rule a new verb bumps `SHARED_PROTOCOL_VERSION` and every key (S4's D63). Decide that before S6: an operator tool of one build cannot then talk to a daemon of another.
- **The files:** `key_files::KeyDir` opens a key's files the safe way; S5b's cleanup of other keys' files (S4's D70) needs the same owner-lock rule: remove a key's socket, pid file and build marker only while holding its owner lock, and never its lock files.
- **The idle clock:** a status probe through the handshake stage is a pending connection and resets the idle time (S4's interface); a probe must not keep a daemon alive.
- **A `daemon-shutdown` that drains** ends attached shims' sessions with exit 1 and the line of D89 (they do not reconnect); a shim started during the drain is answered `RETIRING` and starts a fresh daemon (D80, D83).
- **The restart bound's marker** (D77) is evidence an operator wants: `daemon-status` should show a key whose daemon died during its build, and when the bound expires.

**The §8 soak gate (S6, then the default flip) reads:**

- **Fail-closed count (target 0):** every fail-closed start is one line on the shim's standard error starting `prism-mcp: --shared failed closed` (D87); count them in the clients' MCP logs.
- **Daemon restarts:** every shim that reaches a daemon writes `prism-mcp: shared daemon pid <pid> (started <t>, …)` (D88); distinct `(pid, started)` pairs per key over the soak are its daemon starts. A `RecentBuildCrash` line means a build died (D77).
- **Resident count and footprint:** `pgrep -fl 'prism-mcp daemon'` and the 10-03 incident's probes; shims are a few MB each.
- **Fan-out latency p95 vs standalone:** eight shims, the §6.2 workload and one concurrent `refresh_index` (S3b's SI-11 applies).

**S6 (launchers and docs)** owes, besides S3a's, S3b's and S4's lists: `--shared` in `.codex/config.toml`, the plugin launcher and the bridge's `[[agents.mcp]]` (`scripts/prism-mcp-launch.sh` passes arguments through); `PRISM_MCP_RUNTIME_DIR` set by the bridge; the fail-closed line and what to do about each reason; that `--idle-ttl`/`--max-clients` on a shim configure only a daemon it starts (D85); the restart bound and its minute (D77); that a rebuilt binary's old shims cannot start a daemon (SI-27).

**C2-M1 (and any candidate that ships through the fan-out gate)** can now run the §6.2 fan-out check: eight `prism-mcp --shared` processes against one daemon.

## Self-review record

- **Spec coverage (§12 row S5a, §4.1 steps 1–9).** Step 1 (configuration and cache base) → `run_shim` through `canonical_config` and `DaemonIdentity::resolve` (S2a's normalization). Step 2 → D86 and `run_shim`'s own refusals. Step 3 → `ConnectionSettings::from_env` and `ClientSettings::new` in `run_shim`. Step 4 → `runtime_dir`, `DaemonPaths::new` (the 103-byte check, a fail-closed line), `validate_runtime_dir` at the start of `arbitrate`, and one `deadline`. Step 5 → `connect::reach` (D80). Step 6 → `arbitrate` (D81–D83) and `spawn` (D84). Step 7 → `Failure` and `fail_closed_line` (D87). Steps 8 and 9 → `pump` (D89) and `run_shim`, which never calls `arbitrate` again.
- **S4's interfaces for S5a, one by one.** The command line, with `--key` and the runtime directory (D84). Exit status 0 for a lost claim → `Exit::Success` and a new look (D82); non-zero → fail closed naming the log (D82, D87). Open the log `O_APPEND` (D84, test `the_log_is_opened_for_appending`, mutant 5B). The pid file rewritten in place, read empty or partial (D87, `an_empty_partial_or_missing_pid_file_is_reported_as_such`). Lock files created `O_EXCL` then opened (D79: the shim uses the same `open_private`). What a connection is told before `READY`, a reset as the close, the reply read when the `HELLO` write failed (D80, mutant 2B), `HELLO` within 10 s of `connect` (the shim writes it at once). The peer check moved or shared (D79). The process-test helpers reused (`daemon_process_test.rs`, visibility only). SI-10's restart bound (D77), SI-17 (D84), the shim halves of tests 3, 4, 8, 13 and 15 ("Spec items"). S4's list also named test 2; a shim's part in an idle exit is only to close its connection, which S4's real-clock smoke test already does with its own test shim, so no shim test is added for it.
- **The lane README's notes for S5a, one by one.** The shim checks its peer's uid (D78). The daemon checks the handed key, not adopts it (S4's D61; the shim always passes `--key`). Pass the full key (D84). `DaemonIdentity::resolve` accepts an eager configuration: the command line rejects it, and `run_shim` refuses it too (D86). Paths in error messages escaped (D87). A root-owned `prism-mcpd` after `sudo -E`: the runtime directory's error names the uid, and the fail-closed line carries it; documented in S6. The transcript driver for spec §5 (Task 6, `a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte`). From the S2b reviews: one call site compares `READY`'s identity with the key (`reach`, mutant 2F).
- **The lane's review lessons.** (1) Every stated property has a test and a mutant, or is listed as not held: "Claims". (2) What each mechanism does not cover: its section. (3) Interleavings are played at hook points, not hoped for: a retirement between `connect` and `HELLO`, a starter dying between spawn and claim, a held lock released mid-wait, a second shim waiting on the start lock. (4) No test asserts an upper bound on a duration; one pause gives a wrong implementation time to misbehave and orders nothing a right one needs. (5) Nothing the shim prints carries a control character (`one_line`). (6) No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests in the new code but the pump's bounded slice (Global Constraints); the `unsafe` blocks are one FFI call each (`setsid` in `pre_exec`, `fcntl` and `getsid` in tests).
- **Size.** From `scratch/s5a-tasks`: S5a-1 (`296102a2..d447df4e`) `src/` 16 files, +2,170 −15, code about 770; S5a-2 (`d447df4e..8d5ce396`) `src/` and `tests/` 16 files, +1,483 −22, code about 590; `CLAUDE.md` +19, then +16 −4. The largest file edited is `files.rs` (482 lines); the largest created is `tests/mcp/shim_process_test.rs` (477).

## Validation record

Built on 2026-10-14 on macOS (arm64), in `~/code/prism-s5a-validate`, from `main` `296102a2`. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file under the session's scratch directory, read from the file. Nothing was pushed. Local branches:

- `scratch/s5a-validate`, where the code was first written, test first: `89a37538` (Task 1), `2a1ff330` (Task 2), `26d8e3a2` (Task 3), `1971c52e` (Tasks 5 and 6 as one piece), then the folds (`fold:` commits): test additions and fixes found by the mutants and the red steps, last `7bff1bea`. Every mutant ran on `905bd4e8` (5E again on its fold); the last fold (`7bff1bea`, correction 5) changes one process test's fixture.
- `scratch/s5a-tasks`, the same source cut into this plan's tasks, one commit per task: `1a07ad8a` (Task 1), `db1bf687` (2), `db6f85fc` (3), `d447df4e` (4, the S5a-1 head), `f16e7770` (5), `dd8f4a6e` (6), `8d5ce396` (7, the S5a-2 head). Its `src/` and `tests/` are byte-identical with `7bff1bea`.
- `scratch/s5a-replay` (`bf4a4e18`): this plan's directives replayed by a script onto `296102a2`, every anchor matching exactly once: its tree is `38253d57`, the tree of `8d5ce396`. The red and green steps of Tasks 1, 2, 3, 5 and 6 were run on partial replays in the validate worktree (reset to `296102a2`, the plan replayed up to the task's Step 1, then up to its Step 3), built with `CARGO_INCREMENTAL=0`; Task 6's again after correction 5.

| Step | Command | Observed |
|---|---|---|
| Base | `--lib mcp::`; `--test mcp` (unblessed), at `296102a2` | `517 passed`; `31 passed`, the goldens' test `ok` |
| T1–T6 red and green | as in V2 and V3 | as there |
| Mutants | the 30 of the tasks' tables, each applied alone on `905bd4e8`, its test command run, the file restored | all killed but 5E at first (correction 3), which was then killed on the fold. 6C's test does not end by itself (its daemon holds the shim's stdout, so the transcript waits for the daemon's 15-minute idle TTL); the daemon was killed by hand after about two minutes and the test then failed as named. 6D failed after the 60 s hang limit, as its row says |
| V4–V12 | as in the table at the top | as there |

**Corrections made to the plan by the validation.**

| # | Where | What was first written | What was wrong | What it says now |
|---|---|---|---|---|
| 1 | Task 2, `a_ready_for_this_key_is_reached_and_the_stream_has_no_timeouts` | a fake daemon that wrote `READY` and closed | The attempt came back transient, not reached: on macOS clearing a socket's timeout fails with `EINVAL` once the peer has closed, and `reach` takes that as transient (D80). Hypothesis checked by keeping the connection open, as a daemon does; the alternative (a decode failure) would not have changed | the fake answers and holds the connection until the shim closes it (`answering`) |
| 2 | Task 5, `bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly` | the test wrote the whole 200 KB frame, then read the daemon's side | The test hung: the pump's input thread blocked writing to the daemon's full socket, so it stopped reading the client, so the test's own write blocked. The test's construction, not the pump: reading the daemon's side on its own thread ended it | the daemon's side is read on a thread while the client writes |
| 3 | the same test | the daemon's reader returned whatever it had read when its 30 s bound passed | Mutant 5E (no half-close) survived: the reader timed out with every byte read, and the test accepted that as the end of input | the reader reports whether it reached the end of stream, and the test asserts it did; 5E killed |
| 4 | Task 3, `a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for` | not written | D82 (no look at the owner lock while a started daemon starts) had no test: every fake child claimed the key at once | the test, with a child that claims when a gate opens; mutant 3J killed |
| 5 | Task 6, `a_live_but_wedged_owner_fails_the_shim_closed_naming_its_pid` | the fixture thread blocked in `accept` and was joined before the assertions | Task 6's red step hung: with no `--shared` flag the shim exits at once, nothing connects, and the join never returned (found on the partial replay; the test process was killed) | the fixture accepts from a non-blocking listener until it is told to stop or its sender is dropped; the red step then reads `31 passed; 11 failed` |

**Not run in this validation:** x86_64 Linux (CI); `--lib mcp::` on Linux beyond what V11 states (one base failure as root); the mutation gate (it selects no MCP code; lane README); a release build; the Tier-A harness (no file it watches changes behavior: `call_edge_cache.rs` changes one visibility word); a real builder panic in a daemon a shim started (the marker's daemon half is held in process, Task 1, and the shim's half with a planted marker, Tasks 3 and 6); the §8 soak.
