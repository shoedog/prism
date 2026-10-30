# S5b — Operator Control: `daemon-status` and `daemon-shutdown` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-16; review round 1 folded the same day).** Every task of this plan was built with a compiler before the plan was written, on the local scratch branch `scratch/s5b-validate` (worktree `~/code/prism-s5b-validate`) from `main` `5a63c7e8`, test first, and every mutant named in a task was applied there, each alone; all 38 were killed (37 after review round 1's fold, R10 after round 2's). The source was then cut into this plan's tasks on `scratch/s5b-tasks` (`074117f4`), the plan's directives were generated from that branch's per-task commits, and the plan was replayed by a script onto `5a63c7e8` (`scratch/s5b-replay`, `bf5a7b23`), each of its 66 anchors matching once, to a tree byte-identical with `scratch/s5b-tasks` and with the validated code (`1bf5f7fb`); each task's red and green steps below were run on partial replays. The daemon's, the shim's and the shared tests and the process tests were run on Linux (aarch64, the bridge's `a2a-toolchain` image, as root), 10 times each, all green but one S2a test that assumes it does not run as root and fails on the base in the same container too (V11). **Review round 1** (Opus 5.5 FIX, gpt-6.1-sol FIX) was folded test first, each code change with a test watched failing before it; the owner decided both open questions as recommended (2026-10-16); see "Review round 1" at the end. **Review round 2**, the cap (Opus 5.5 APPROVE, gpt-6.1-sol APPROVE), left one test and wording, folded after the cap without a third round (disclosed: "Review round 2"). Counts, red steps and mutant results below are the observed ones, after the fold. The mutation gate was not run (it selects no MCP code; lane README).

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file that was then read.

| # | Command | Must show | Observed (after review round 1's fold; after round 2's where a row says so) |
|---|---|---|---|
| V1 | `cargo test --features mcp --lib mcp::` and `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` on the untouched base | The base totals; every later total is "base + N". | `571 passed`; `46 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`, at `5a63c7e8`. |
| V2 | Each task's red step, on a partial replay of this plan | What the step says. | Task 1: `E0583` for `control`. Task 2: 15 errors (`Control`, `ControlSnapshot`, `control_exchange`, `StopReason::Shutdown`, `Admission.shutdown`). Task 3: `E0560` for `Served.build_identity`. Task 5: `E0583` for `control` and `status`, `E0599` for `from_locator` ×2. Task 6: `E0583` for `shutdown`. Task 7: `test result: FAILED. 46 passed; 6 failed`, each new test on `error: unexpected argument 'daemon-status' found` or `'daemon-shutdown'`. |
| V3 | Each task's green step, on the replay | The count the step states, no warning. | Task 1: 581. Task 2: 587. Task 3: 596. Task 5: 619. Task 6: 630 (629 before round 2's test). Task 7: `--test mcp` 52. No `warning` line in any of them. Red and green steps all re-run on the round-2 plan, as stated. |
| V4 | `cargo test --features mcp --lib mcp::` at each head | base + 25, then base + 59; `0 failed`. | 596 at the S5b-1 head (`7e17ba5e`); 629 at the S5b-2 head's code (`7e456df8`); after round 2's fold, 630 at `5dd1d799` (the tree of `074117f4`); `0 failed`. |
| V5 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `46 passed`, then `52 passed`, the goldens unblessed. | 46 at the S5b-1 head, 52 at the S5b-2 head, the goldens' test `ok` each time; zero-line diff in `transport_tests.rs`, `owner_transport_tests.rs` and `tests/fixtures/mcp/golden/`. |
| V6 | `cargo check`, `cargo check --features mcp`, `cargo check --tests --features mcp,detached-owner-audit`, `cargo fmt --check` | `Finished`; no warning from the feature build; fmt silent. | All `Finished`, 0 warnings from `cargo check` and `cargo check --features mcp` at both heads; the owner-audit build `Finished` with 11 warnings, all in test files this slice does not touch (`tests/ast/`, `tests/algo/`, `tests/integration/`); fmt silent at every task commit of the cut. |
| V7 | `cargo clippy --all-targets --features mcp`, filtered to the touched files | No line in a touched file. | 258 `warning` lines at each head, none naming a file this slice touches. |
| V8 | Every "prove the tests bite" step | The named tests fail under the named mutant. | 1A–1F, 2A–2F, 3A–3D, 5A–5F, 6A–6D, 7A, 7B and review round 1's R1–R9: all 37 killed, each alone, on the folded code (`fb821e47`), and round 2's R10 on `8c297de9`; 7A at the process test's 60 s hang limit, R1 and R5 at their 30 s waits. 7A's first run on the folded code selected no test (its filter named the test's old name) and was inadmissible; re-run with the new name, killed. |
| V9 | Repeats on macOS | All `ok`; no daemon process left. | S5b-1 head: `10 test result: ok. 150 passed` (`--lib mcp::daemon`). S5b-2 head: `10 test result: ok. 79 passed` (`--lib mcp::shim`) and `10 test result: ok. 6 passed` (`--test mcp daemon_control`); `pgrep -fl 'prism-mcp daemon'` empty afterwards. |
| V10 | The zero-diff and replay checks | Nothing in the standalone files or the shim's session; the replay byte-identical. | `git diff --name-only 5a63c7e8 bf8772fa`: 32 paths (31 under `src/` and `tests/`, and `CLAUDE.md`), none of `transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `shim/{arbitrate,pump,spawn,run,socket}.rs`, `tests/fixtures/mcp/golden/` (`shim/arbitrate_tests.rs` and `shim/key_files.rs` change: D109). After round 2's fold the replay `bf5a7b23` has tree `1bf5f7fb`, the tree of `scratch/s5b-tasks` `074117f4` and of the validated code `5dd1d799` (round 1's: `1b2d72ea`, `f09b8748`). |
| V11 | Linux: the bridge's `a2a-toolchain` image (Debian, aarch64, rustc 1.94.0, uid 0), with Task 8 Step 4's script (`EXTRA_SUITE="--test mcp"`), at the S5b-2 head; the disk checked (108 GiB free) in a command of its own first | All `ok`. | `failed Linux runs: 10`, every one the S2a test below: `10 test result: ok. 79 passed` (`--lib mcp::shim`, the FIFO and full-backlog tests included), `10 test result: ok. 150 passed` (`--lib mcp::daemon`), `10 test result: ok. 52 passed` (`--test mcp`, the goldens' test `ok` in all 10), no `left:` line; `--lib mcp::shared` `10 test result: FAILED. 136 passed; 1 failed`, the one failure `mcp::shared::dir_chain::tests::another_users_private_directory_is_refused_for_what_it_holds`, an S2a test that assumes it does not run as root. Control (before the fold), the base `5a63c7e8` in the same image: `--lib mcp::shared::dir_chain` `FAILED. 15 passed; 1 failed`, the same test. Not this slice's; reported, not fixed (as in S5a's V11). The volumes removed by the `trap`. Not run at the S5b-1 head on its own (its code is part of the S5b-2 head). x86_64 Linux: CI only. |
| V12 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp` and `cargo test --doc --features mcp` | `0 failed`. | At the S5b-2 head's code (`7e456df8`): 31 `test result` lines, **6,151 passed, 0 failed, 7 ignored**; doc tests `2 passed`. At the S5b-1 head (`7e17ba5e`): 31 `test result` lines, **6,112 passed, 0 failed, 7 ignored**; doc tests `2 passed`. The base's total, not run, is 6,112 − 25 = 6,087 by the same count. |

## Owner decisions (2026-10-16)

Two behaviors the spec leaves open with real tradeoffs were put to the owner with this plan's recommended defaults. Both reviewers of round 1 agreed with both defaults. Decided as recommended:

1. **The files of keys no daemon will use again (S4's D70, SI-33): report and count, remove nothing.** Every rebuild of prism gives every repository a new identity key (the key covers the content-hashed build identity), and spec §4.3 never removes lock files, so each rebuild leaves `<key16>.start`, `<key16>.owner` (empty) and `<key16>.log` (a few lines in normal use, at most 8 MiB) per repository that was served: in `$TMPDIR/prism-mcpd` on macOS, and on Linux in `$XDG_RUNTIME_DIR/prism-mcpd` (a small tmpfs, that is, memory) or `/tmp/prism-mcpd`. `daemon-status` counts these keys in one line with the bytes of their logs (`2 keys have nothing but lock files and logs (10 bytes of logs)`), so the §8 soak can see whether they matter. Not built: a pruning flag that, holding a dead key's start and owner locks, removes its socket, pid file, an expired build marker and its log, never a lock file (about 80 lines; it removes the evidence a crash leaves), a small follow-up if the soak shows logs piling up; and removing lock files, which spec §4.3 forbids because a process holding a descriptor of an unlinked lock file can hold "the" lock beside another holder.
2. **A daemon asked to shut down keeps admitting new clients until its count reaches 0 (D100, SI-30).** Spec §4.7 says "enter Draining when its admitted count reaches 0; it never disconnects other clients" and is silent about a client that arrives in between. A shim that arrives is served by the warm index; the daemon drains when its last client, old or new, has gone. Cost: a daemon in steady use may not drain while clients keep coming, so the bridge's reaper gets the memory back only when the other sessions on that repository have also ended. Not built: answering new clients `RETIRING` (they would find the owner lock held, poll to their deadline and fail closed, the count the soak wants at 0); releasing the key early so a fresh daemon starts beside the draining one (two resident indexes of one repository).

Recorded with them: **D97** (the control exchange has a version of its own and does not bump `SHARED_PROTOCOL_VERSION`) stands as a disclosed planner's decision. Both reviewers found it sound; it now carries a versioning rule (review round 1, Opus S3) and a corrected rationale (sol).

---

**Goal:** the lane's: let the a2a-bridge run its reviewer and implementer fan-out on this host without swap exhaustion, by holding one resident index per compatible (checkout, prism build, cache configuration, refresh policy), with no query-latency regression under fan-out. **What S5b must make true, and test:** an operator (a person, or the bridge's reaper) can see every shared daemon in a runtime directory and what each is doing (spec §4.7: "key, repo root, build identity, pid, start time, admitted and pending clients, build state and attempt, generation, idle time, and whether its socket answers the preamble"), including daemons of other builds of prism; looking at a daemon of this build never keeps it alive (S4's interface; a daemon built before this slice is the disclosed exception, SI-35) and never removes anything; and `daemon-shutdown` asks exactly one instance of a configuration's daemon to drain once it has no client, never disconnecting one (spec §4.7, §11 item 3: "graceful and instance-bound, validated through the live control exchange; it must never disconnect other active clients"), so that the reaper can free a repository's index when the bridge is done with it. The in-process server and the shim's session do not change; `--shared` stays opt-in.

**Architecture:** One new message pair on the daemon's socket, in place of the preamble's `HELLO`: `STATUS {"control":1}` and `SHUTDOWN {"control":1,"identity":…,"pid":…,"start_time":…}` (`src/mcp/shared/control.rs`), each answered by one `STATUS` line and a close. The exchange carries a version of its own, `control`, not the preamble's `protocol`, so it is not part of any identity key (D97). In the daemon the accept loop recognizes a control line by its verb (`server.rs::answer_control`) and asks the control plane (`admission.rs::control_exchange`) in one critical section to take the connection out of the pending count without restarting the idle time (a new `quiet_since` beside `idle_since`), to record a shutdown when the request names this very instance, and to read the counts and the build status for the reply. `retire_if_idle` drains a daemon with a recorded shutdown as soon as nothing is admitted or pending, ignoring the idle TTL and a running build (`StopReason::Shutdown`). The operator's side lives beside the shim, which already holds the client half of the protocol (D101): `shim/control.rs` makes one exchange (the shim's non-blocking connect, peer check and deadline-bounded line reader), `shim/status.rs` is `daemon-status`, `shim/shutdown.rs` is `daemon-shutdown`, and `src/bin/prism-mcp.rs` dispatches the two words as it dispatches `daemon`.

**Tech Stack:** Rust 2021 (rustc 1.94), `serde`/`serde_json` (the reply's strings), `std::fs::File::try_lock`, `libc` through the existing `daemon::files` and `shim::socket` functions, `clap`, `tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§3.1 Draining, §4.2 "Admission and retirement", §4.3 files, §4.4, §4.6 "New prism build installed", §4.7, §8 step 4, §11 item 3, §12 row S5b). Lane plan: `README.md` in this directory (the S2b note on new verbs and the protocol version; the S4 note on a cleanup rule). Built on S5a as merged: `main` `5a63c7e8`. Decisions are numbered on from the S5a plan's (D77–D96) and spec issues from its SI-22–SI-28.

**Interfaces this plan settles** (from S4's and S5a's "for S5b" lists): `DaemonState::status()` and `admission_status()` stay test-only (the reply reads the control plane in the exchange's own critical section instead); `StopReason` gains `Shutdown`, recorded by `retire_if_idle` when nothing is admitted or pending (S4's suggestion, without the TTL); the cleanup rule for other keys' files is the owner's decision 1 (report, remove nothing); the wire verbs do not bump `SHARED_PROTOCOL_VERSION` (D97); a probe does not reset the idle clock (D99); S5a's "a `daemon-shutdown` that drains ends attached shims' sessions" does not arise, because a shutdown drains a daemon only when no client is attached (D100), and a shim that arrives during a drain is answered `RETIRING` and starts a fresh daemon, as before (nothing changes in the shim); the build marker's sighting is shown by `daemon-status` (D102).

**Ships as two pull requests.** **S5b-1 = Tasks 1–4** (the control exchange on the wire, in the control plane and in the accept loop; tested in process against the real accept loop): 13 files in `src/`, 1,142 insertions and 27 deletions, of which 448 insertions are production code and comments (326 code lines: `shared/control.rs` 255, `server.rs` +84, `admission.rs` +75, `control.rs` +23, the rest a few lines each) and 694 are tests; plus 14 lines of `CLAUDE.md`. **S5b-2 = Tasks 5–8** (one exchange from the operator's side, `daemon-status`, `daemon-shutdown`, the command line, process tests): 19 files in `src/` and `tests/`, 2,121 insertions and 20 deletions, of which 843 are production (665 code lines: `status.rs`, `shutdown.rs`, the binary +95, `shim/control.rs`, `key_files.rs`, the rest a few lines each) and 1,278 tests; plus 17 lines of `CLAUDE.md`. Together about 1,290 production lines (991 of code), at the lane's threshold for a split; split (D108). Review round 1 added about 140 production and 270 test lines. They are split where a reviewer's question changes: S5b-1 is the daemon (what it answers, and when a shutdown drains it), tested in process against the real accept loop; S5b-2 is the operator (what the commands say and do, and what they never touch), tested against stand-in daemons and, as processes, against a real daemon and shim. S5b-2 is based on S5b-1; nothing in S5b-1 depends on S5b-2.

## Global Constraints

- All production code is under `--features mcp` and `#[cfg(unix)]`, in `src/mcp/shared/{control,preamble,paths,mod}.rs`, `src/mcp/daemon/{admission,control,coordinator,run,server}.rs`, `src/mcp/shim/{control,status,shutdown,connect,diagnostic,key_files,mod}.rs`, `src/mcp/mod.rs` (one re-export) and `src/bin/prism-mcp.rs` (two words dispatched before the standalone parser, as `daemon` is); tests in `src/mcp/shared/`, `src/mcp/daemon/`, `src/mcp/shim/` and `tests/mcp/`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged, byte for byte**, and so is the shim's session. Not edited: `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `src/mcp/shim/{arbitrate,pump,spawn,run,socket}.rs`, the golden transcripts under `tests/fixtures/mcp/golden/` (never set `PRISM_MCP_BLESS_GOLDEN`). `src/bin/prism-mcp.rs` changes only by two first words that today are parse errors (`daemon-status`, `daemon-shutdown`). The `HELLO`/`READY` wire format does not change: `preamble.rs` only widens helpers to `pub(super)` and `BuildState::word` to `pub`. Tasks 4 and 8 check it.
- Do not re-decide D1–D96. Settled here on purpose, and said so: S4's D63 (no wire status verb in S4; whether a verb bumps the protocol version, now D97), D66 (the dead-code allowances of `status` and `admission_status`), D70 (the cleanup rule, owner decision 1), S4's interface on the idle clock (D99), and S5a's interface on the build marker (D102).
- Spec values, copied exactly:
  - "`prism-mcp daemon-status [--repo X] [--runtime-dir D]` prints, for every live daemon in the runtime dir (or the one for `X`): key, repo root, build identity, pid, start time, admitted and pending clients, build state and attempt, generation, idle time, and whether its socket answers the preamble. `prism-mcp daemon-shutdown --repo X` asks the instance verified through the live control exchange (pid + start time echoed in `READY`) to enter Draining when its admitted count reaches 0; it never disconnects other clients (§11 decision 3). The integration tests use a test-only status probe over the same control exchange (S3a) and do not depend on the operator tool (S5b)" (§4.7).
  - "`daemon-shutdown` verb for the bridge's reaper: yes — DECIDED. Graceful and instance-bound, validated through the live control exchange; it must never disconnect other active clients" (§11 item 3).
  - "New prism build installed | New key, new daemon; the old daemon retires when its last client leaves (overlap is bounded only by client lifetime; `daemon-status` lists every live daemon with its build identity)" (§4.6).
  - "Draining: entered only from the admission-and-retirement critical section (§4.2) when admitted == 0 and pending == 0 for the TTL and no build is running. Under that same lock the daemon stops accepting, unlinks its socket while still holding the owner lock, and answers `RETIRING` to any handshake that races in before the unlink … Then the pid file is removed, the owner lock released, exit 0" (§3.1).
  - "Every launcher that should share a daemon must resolve the same directory; `docs/MCP.md` says so and `daemon-status` prints it" (§4.3).
  - "Lock files are created 0600 and never unlinked (stable inodes; unlinking a lock file is how two holders come to exist)" (§4.3).
- **The control exchange is read as strictly as the preamble:** one spelling per message, the one `encode` writes; a line of 4 KiB at most; another `control` version refused before its payload is read.
- **No panic on a path a client or a peer can reach** in code this slice adds: no `unwrap`, `expect`, indexing or unchecked arithmetic outside tests. No `unsafe` in production code (the tests' `mkfifo` is one FFI call with a `SAFETY` comment).
- **Nothing the operator prints carries a control character**: every line goes through the shim's `one_line`, whatever a daemon reported (a root may hold any byte).
- **`daemon-status` reads and never removes.** It takes a key's owner lock only while it holds that key's start lock, and only for a key with a socket, a pid file or a build marker; it never records a build marker's sighting (that is a shim's, D77); it never waits on a key's file (the marker and the pid file are opened `O_NONBLOCK`, regular files only, size-bounded); one key it cannot look at is reported on its own line and does not hide the others.
- **Tests do not depend on the host's speed.** A test waits for a condition up to its hang limit (30 s in process, 60 s for processes) and asserts no upper bound on an elapsed time; a deadline a test means to expire is short (300–500 ms) and only its outcome is asserted. The process test of the idle clock gives a daemon a 2 s TTL and probes it every 100 ms until it exits, and asserts nothing that depends on when its first probe runs; a probe that restarted the idle time would keep it up until the hang limit. The listing is asserted on a daemon with the default TTL (review round 1, Opus S1). The FIFO regressions run under an independent watchdog thread whose only bound is the hang limit.
- **No test aborts the test binary**, and every daemon a test starts is ended on every path (`Daemon`'s `Drop`, S4; `Daemons`, S5a).
- Every runtime directory a test uses is under `/tmp` (S2's note: a socket path must fit 103 bytes).
- File size (CLAUDE.md decision 7): weighted lines (`python3 scripts/file_size.py FILE...`), new files aim for 600 or fewer. Weighted sizes after this slice (`scripts/file_size.py` from `main`): every file it creates or edits is at most 546 (`src/mcp/daemon/server.rs`, 475 before); S5a's `src/mcp/shim/arbitrate_tests.rs` grows to 502.5; the largest file it creates is `src/mcp/shim/status_tests.rs`, 359.5. Tests stay in sibling `*_tests.rs` files included with `#[path]`.
- No new clippy warning in a file this slice adds or edits.
- **Commit before any experiment.** The "prove the tests bite" steps edit a source file and restore it with `git checkout -- <file>`; they come after the task's commit.
- **Run the daemon's, the shim's and the shared tests on Linux before each pull request** (Tasks 4 and 8), in the bridge's `a2a-toolchain` image with the worktree mounted read-only and named volumes for the target and the registry, removed by a `trap`.
- Logs go to files in a directory of the implementer's own, `$LOG_DIR` below; read the file. Do not pipe cargo output into `grep` or `tail`; `grep` the saved logs afterwards. Every cargo command runs alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`. **Before any large build or Docker run, check `df -h /System/Volumes/Data` in a separate command and keep at least 50 GiB free.**

### What each mechanism covers, and what it does not

- **The control exchange is what one daemon says at one moment.** Between a status and a shutdown the daemon can drain and another start; the shutdown names the instance (identity, pid, start time) and a daemon that is another instance records nothing (D98, D104).
- **A probe is a connection until its line is read.** It counts against `--max-clients`: a daemon at its bound answers it `BUSY`, and a shim that arrives at bound − 1 while a probe is pending (one turn of the loop) is answered `BUSY` and fails closed (spec §10), negligible at the default bound of 64 (review round 1, Opus S4). While it is pending the daemon does not retire, for at most the 10 s handshake timeout. A probe whose control line is read whole, in time or with the read completing across its deadline, leaves the idle time as it found it (D99). A connection that ends before its line is whole, or whose line is still incomplete when its deadline is checked before a read (an operator stalled 10 s between connect and write), or whose line is neither a control line nor a `HELLO`, cannot be told from a client's and restarts the idle time when it goes (review round 1, Opus W3, sol 3).
- **`daemon-shutdown` reaches this build's daemon of one configuration.** The key covers the build, the cache base and the refresh policy, so a daemon of another build of the same repository has another key: `daemon-status` lists it with its build identity, and it retires on its own after its last client and its idle TTL (spec §4.6). Stopping one at once is `kill -TERM <pid>`, which drains it whatever its clients (spec §3.1).
- **A shutdown request is not revocable.** A daemon asked while it serves clients drains the moment the last one leaves, however many came and went in between (owner decision 2).
- **A shutdown drains a daemon whose build is running when no client is attached** (D100): S4's SI-15 left a builder that never returns keeping its daemon up "until `daemon-shutdown` … or a kill". The build is abandoned with the process; the marker is removed by the drain, as for any drain during a build (S5a's D92), so the next shim does not take it for a crash.
- **"Drained" means the pid file is gone.** The daemon removes it just before it releases the owner lock and exits; the process can still exist for a moment after the command returns.
- **`daemon-status` cannot see a daemon between its claim and its bind.** A key with only lock files and a log is counted, not probed; a daemon that holds the owner lock and has not yet written its pid file or bound its socket is counted with them for that instant.
- **A daemon of a build without the control exchange** (any build before this slice) closes the line it does not know (S4's D54): `daemon-status` reports it alive but not answering, through its locks, and `daemon-shutdown` exits 1 for it. **And, as for any line it refuses, its idle time restarts at each probe**: an operator who polls it more often than its idle TTL keeps it up for as long as the polling goes on (SI-35; review round 1, Opus W2, sol 1; accepted by the controller and disclosed: new code cannot change an old daemon). Stop such a daemon with `kill -TERM <pid>` (spec §3.1). Measured with a daemon of the base build and this slice's `daemon-status` (Validation record): with a 3 s TTL and a probe a second it was alive after 9 s and retired 2 s after the probes stopped; a daemon of this slice under the same probes retired at 3.1 s. No released build of prism has `--shared`.
- **The root a daemon reports is shown, clipped to 512 characters and lossy where it is not UTF-8** (D105). The key, not the root, identifies the daemon.
- **An instance is (identity, pid, start second).** A daemon replaced by another with the same pid in the same wall-clock second (or across a clock step, or from another pid namespace that shares the runtime directory) is indistinguishable from it, and a shutdown meant for one is recorded by the other (review round 1, sol 5: an assumption stated, not a demonstrated failure). A unique per-instance token would remove the assumption and was not added: it changes `READY`, the pid file and the reaper's selector.
- **A key's build marker and pid file are read without waiting** (review round 1, sol 2, Opus S6): opened `O_NONBLOCK` through the runtime directory's descriptor, never through a symlink, a regular file of this user only, at most 256 (marker) or 200 (pid file) bytes. A FIFO or another non-regular file at the marker's path is reported as unusable and, by a shim, removed with the stale files (D109).
- **A refused connection is no daemon only when nobody holds the owner lock** (review round 1, Opus W1): a live daemon whose listen backlog is full (or, when a dead daemon's files are still there, one that has claimed the key and not bound yet) refuses connections too, and both commands then look at the key's locks (D102, D104). Two short windows remain (review round 2, Opus W2): (a) on a clean key a daemon that has claimed the key and not bound has no socket, pid file or marker yet, so neither command looks at its locks and `daemon-shutdown` reports nothing to shut down (exit 0) for the microseconds before it binds; (b) a daemon in the middle of its drain (socket unlinked, pid file and owner lock still there) is reported alive but not accepting (exit 1) for the milliseconds of the drain, which errs safe: a retry converges to exit 0.
- **The peer check is the guarantee between users**, as for the shim (S5a's D78): nothing is sent to a socket whose peer is another user. Neither command defends against a process of the same user.
- **The status's look at the locks can delay a shim by the moment it holds the start lock**; it never fails one (a shim retries the start lock until its deadline).

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously, or an earlier slice's note this slice was told to settle. "Cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D97 | **The control exchange has a version of its own, `control` (1), and does not bump `SHARED_PROTOCOL_VERSION`; no identity key changes.** | S4's D63 and the S2b note: by S2's rule a new verb bumps the preamble's version, which is part of every key. But `daemon-status` must list daemons of every build (spec §4.6), and a version in the key would make an operator tool of one build unable to talk to a daemon of another by construction. The `HELLO`/`READY` lines do not change, and a daemon that does not know a control line closes it (D54), which the operator reports. Keys change with every build anyway (the content-hashed build identity), so bumping would buy nothing. Corrected after review round 1 (sol): a bump would not have prevented cross-build status as first argued here, since `daemon-status` finds other keys by their file names anyway; the exchange stays out of the preamble's version because the `HELLO`/`READY` contract does not change and the control exchange needs a compatibility rule of its own, below. A bump would not have repaired the old-daemon idle restart either (SI-35). **Versioning rule (review round 1, Opus S3):** a daemon answers every `control` version it has ever spoken; an operator sends its highest version and, on a close without a reply, falls back to the next lower one. Version 1 is the only one so far, so nothing implements the fallback yet. Both reviewers found D97 sound; a planner's decision, disclosed, recorded by the owner on 2026-10-16. | A version bump later: one constant and the golden key tests. |
| D98 | **One line each way, then the daemon closes.** A request is `STATUS` or `SHUTDOWN`; both are answered by the same `STATUS` line (instance, build identity, root, admitted, pending, `--max-clients`, build state, attempt, generation, idle seconds or `null`, idle TTL, shutdown flag), or by `RETIRING`/`BUSY` as a `HELLO` would be; a control line this build refuses, or one whose read completes across its handshake deadline, is closed without a reply. A line is a control line when its verb is `STATUS` or `SHUTDOWN` followed by a space (`is_control_line`), decided before `HELLO` decoding. | The handshake stage already reads exactly one line per connection with a deadline; a second line would need a second stage. Answering a shutdown with the status lets the operator see which instance answered and whether it recorded the request. | None. |
| D99 | **A control exchange leaves the idle time as it found it.** `Admission` gains `quiet_since`: when a connection that was not a control exchange last left the counts (and the first retirement decision). `enter_handshake` clears `idle_since` (S4, unchanged) but not `quiet_since`; a control exchange that leaves the counts at zero puts `idle_since` back to `quiet_since`. | S4's interface: a probe is pending while its line is read, and leaving the way a client does would restart the TTL, so an operator who looks keeps the daemon up. Putting back the value found at entry would be wrong when a client came and went during the probe (`a_client_that_leaves_during_a_control_exchange_moves_the_idle_time`, mutant 2B). A control line whose read completes across its deadline (the deadline passes during the read that ends the line) is dropped the same way, unanswered and with no shutdown recorded (review round 1, sol 3; `a_control_line_completed_after_its_deadline_moves_nothing`, mutant R1); one still incomplete when its deadline is checked before a read is a client's, as below (review round 2, Opus W1: the window is a single read, so this covers little in production; the optional last read at the deadline was not added). A connection whose line never completes, or is not a control line or a `HELLO`, is treated as a client's (`a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does`, a pin). | None. |
| D100 | **A shutdown is recorded only when it names this very instance** (identity, pid and start time all equal), **and drains the daemon as soon as nothing is admitted or pending, whatever the idle TTL and a running build; until then the daemon goes on admitting new clients.** | Spec §4.7, §11 item 3; owner decision 2 (2026-10-16) for the new clients. Pending is counted as for the idle retirement, so a shim in the middle of its handshake is admitted rather than sent to start a second daemon. A running build is not waited for: with no client attached nobody waits on it, and S4's SI-15 named `daemon-shutdown` as the way out of a build that never returns. | About ten lines to refuse new clients instead. |
| D101 | **The operator's commands live in `src/mcp/shim/`** (`control.rs`, `status.rs`, `shutdown.rs`), beside the shim. | They are clients of the same sockets and files: they reuse the shim's non-blocking connect, its peer check, its deadline-bounded line reader (`connect::read_reply`, widened to `pub(super)`), its key-file functions (`KeyDir`) and its one-line escaping, with no visibility widened outside the module. | A mechanical move to a module of their own. |
| D102 | **`daemon-status` reads and never removes.** A key with a socket is asked; a key whose socket does not answer, or with a pid file or a build marker and no socket, is looked at through its locks: its start lock (busy: "being started or reached"), then, holding it, its owner lock (held: "alive but not answering", with the pid file; free: "no daemon", with what was left and the build marker's restart time, by `arbitrate`'s rule). Each lock is held for a moment (`under_locks`, shared with `daemon-shutdown`). A key with nothing but lock files and a log is counted. A key that cannot be looked at (a symlink at its start lock, say) is reported on a line of its own and the others are still listed (review round 1, Opus S7), and the command then exits 1, with "N keys could not be looked at" on standard error (PR #418 review round 2, sol 1). Every existence check, of the runtime directory and of a key's files, goes through `present`: only `NotFound` and `NotADirectory` (nothing can be at a path through a file) are absence; any other error is the key's "cannot be looked at" line, never "nothing but lock files" (PR #418 review rounds 1 and 2). | What makes the look safe (restated after review round 1, Opus S5): taking an owner lock nobody holds can at worst make a daemon that is about to claim the key find it taken and exit silently (S4's `Claim::Taken`), leaving no owner, never two; and the look happens only for a key with a socket, a pid file or a build marker, which a starting shim has already removed under the owner lock before it spawns (S5a), so a daemon a shim is starting is looked at only in a race of microseconds between the existence check and the lock. The start lock is taken first so that the look keeps out of a shim's start and waits for none. Two edges remain: a daemon started directly (`prism-mcp daemon …`, as tests do) holds no start lock; and a shim that gives up at its deadline releases the start lock while its child may still be before its claim. In both the outcome is the silent exit above, and the next shim starts the daemon again. S5a's interface: the marker's sighting and when the minute ends are what an operator wants; only a shim records a sighting (D77). Owner decision 1 for removal. | — |
| D103 | **`--runtime-dir` is the runtime base, as `PRISM_MCP_RUNTIME_DIR` gives it**; the daemons' files are in its `prism-mcpd` directory. Without it the commands resolve the directory as a shim does. A runtime directory that does not exist is reported and not created. | Spec §4.7's `--runtime-dir D` does not say which; the base is what the bridge sets (spec §8 step 1), so an operator can paste the same value. Creating a directory to report that nothing is in it would be a side effect of looking. | A flag's meaning (pre-release). |
| D104 | **`daemon-shutdown`:** a status exchange first (no daemon, or one that is draining: exit 0 and say so; one that does not answer, is at its bound or is refused: exit 1); the answering instance must serve this configuration's full key; with `--pid` and `--start-time` it must be that instance too, or it is left alone (exit 0: the named instance no longer serves the key); then `SHUTDOWN` naming it. A reply from another instance: left alone, exit 0. With clients: exit 0, "drains once its N admitted clients have disconnected". With none: wait until the pid file no longer holds that pid and start time, then exit 0 "drained"; at the timeout exit 1. A socket that refuses connections, at either exchange, is "no daemon" (exit 0) only when the key's owner lock is free; held, exit 1 "alive but not accepting connections" with the connect error; start lock held, exit 1 (review round 1, Opus W1). A `SHUTDOWN` whose reply is lost (no reply, or one this build cannot read) exits 1 saying the request may have been recorded, because a request cannot be withdrawn; a `BUSY` is answered before the line is read, so it records nothing (review round 1, Opus W4). The instance is (identity, pid, start second); the uniqueness assumption is stated under "What each mechanism covers" (sol 5). "Draining" (exit 0) means a daemon that answered `RETIRING`; one caught between unlinking its socket and removing its pid file is reported alive, exit 1, for the milliseconds of the drain (review round 2, Opus W2). A request accepted with `admitted == 0` and `pending > 0` (a client mid-handshake) waits to `--timeout` and exits 1, since that client is then admitted and keeps the daemon up; a retry sees it admitted and converges to exit 0 (PR #418 review round 1, Opus S3; documented, not changed: a concurrent probe also counts as pending, and the current branch errs toward "did not see the drain"). Only a runtime directory, or a pid file during the wait, that is not there (`NotFound`) is absence; any other error looking at it (`EACCES` from a parent this user cannot search, `ELOOP` from a symlink in the pid file's place) is reported, exit 1: never "nothing to shut down" or "drained" (PR #418 review round 1, sol 1, Opus S2). | Spec §4.7 "the instance verified through the live control exchange"; §11 item 3 "instance-bound". The two flags let the bridge's reaper name the daemon its shims reported (S5a's D88 line) and never stop a newer one. Exit 0 means "that instance no longer serves the key, or will drain on its own", what a reaper needs to know. | None. |
| D105 | **The status reply's root is lossy and clipped to 512 characters (`…` appended).** | The reply must fit one 4 KiB preamble line; JSON can write a character as six bytes. The key identifies the daemon; the root is for a person. | A constant. |
| D106 | **`--timeout` (1–600 s, default 10)**: for `daemon-status` the time each daemon has to answer; for `daemon-shutdown` the whole command, the wait for the drain included. | One flag; the default matches the handshake timeout and the shim's start deadline. | A constant. |
| D107 | **`--repo` (with `--cache-dir` and `--refresh-policy`) names this build's key for that configuration**, for both commands; `daemon-status` then reports that key only, and "no daemon for this configuration" without creating any file of it. | The key covers the cache base and the refresh policy (spec §4.3), so `--repo` alone does not name a daemon. | None. |
| D108 | **Two pull requests**, split between the daemon (Tasks 1–4) and the operator (Tasks 5–8). | About 2,900 lines in all; each half is the size of an earlier slice's PR that converged in two rounds, and each has its own reviewer question. | — |
| D109 | **A key's build marker is read without waiting and only so far, in S5a's `KeyDir`** (`read_build_marker`: `O_NONBLOCK`, through the directory's descriptor, never through a symlink, a regular file of this user only, at most 256 bytes; `build_marker`, which the shim's start protocol calls, returns `None` for anything else). `daemon-status` reports a non-regular marker as unusable. | Review round 1, sol 2 (reproduced: a FIFO at `<key>.build` held `daemon-status --timeout 1` past 2.5 s with both locks) and Opus S6. The shim had the same exposure: a blocking open of a FIFO marker held it for ever; now such a marker is no marker, and the stale files' removal removes it (`a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key`). For a regular marker nothing changes but the 256-byte bound, far past the three short lines a daemon and a shim write. | None. |

## Spec items implemented, and spec issues found

Spec §12 row S5b, item by item: `daemon-status` → Tasks 1–3 (the daemon's answer), Task 5 (`status.rs`), Task 7 (`prism-mcp daemon-status`); `daemon-shutdown` → Tasks 1–3 (the request and the drain), Task 6 (`shutdown.rs`), Task 7 (`prism-mcp daemon-shutdown`). Spec §9 lists no test for S5b (its integration tests use S3a's in-process probe); the tests here are this plan's own, listed under "Claims".

- **SI-29 (§4.7 "whether its socket answers the preamble").** The probe is the control exchange, not a `HELLO`: a `HELLO` would be admitted, counted as a client and would restart the idle time. A daemon answers the control exchange from its handshake stage, so "answers" means the same thing.
- **SI-30 (§4.7 "to enter Draining when its admitted count reaches 0").** Read as the idle retirement's condition without the TTL: admitted and pending both 0; a running build does not hold it off; new clients are still admitted until then (D100; owner decision 2, 2026-10-16).
- **SI-31 (§4.7 `daemon-status [--repo X] [--runtime-dir D]`, `daemon-shutdown --repo X`).** `D` is the runtime base (D103); `--repo` needs `--cache-dir` and `--refresh-policy` to name a key, and names this build's (D107).
- **SI-32 (§4.7 "the instance verified through the live control exchange (pid + start time echoed in `READY`)", §11 item 3).** The instance is the identity, the pid and the start time; it is verified by a status exchange and bound across the second exchange by the daemon's own comparison; `--pid` and `--start-time` name one from outside (D104).
- **SI-33 (§4.3 files; S4's D70).** The cleanup rule for other keys' files: owner decision 1 (2026-10-16): report and count, remove nothing.
- **SI-34 (S2's rule that a new verb bumps the protocol version; S4's D63).** Not bumped (D97), with a versioning rule of the control exchange's own.
- **SI-35 (§4.6 "the old daemon retires when its last client leaves").** Not while an operator of this slice polls it more often than its idle TTL: a daemon built before this slice restarts its idle time at each `STATUS` it refuses (review round 1; accepted and disclosed; `kill -TERM` stops it).

## Review Focus

Eleven ways this code is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code.

1. **An operator who looks keeps the daemon alive.** A probe is pending while its line is read; leaving as a client does restarts the TTL. → Task 2, `a_control_exchange_leaves_the_idle_time_as_it_found_it`, `a_client_that_leaves_during_a_control_exchange_moves_the_idle_time`; Task 3, `a_status_probe_does_not_keep_an_idle_daemon_up`, `a_refused_control_line_is_closed_without_a_reply_and_leaves_the_idle_time`; Task 7, `daemon_status_does_not_keep_an_idle_daemon_up` (a real daemon with a 2 s TTL, probed every 100 ms). The exception for daemons built before this slice is disclosed (SI-35).
2. **A shutdown for the wrong instance.** A daemon replaced between the status and the shutdown, or one the reaper did not mean. → Task 3, `a_shutdown_for_another_instance_records_nothing` (identity, pid, start time each); Task 6, `another_instance_named_by_pid_and_start_time_is_left_alone`, `a_daemon_replaced_between_the_exchanges_is_left_alone`, `a_daemon_of_another_identity_is_not_asked`; Task 7, `daemon_shutdown_of_another_instance_leaves_the_daemon_alone`.
3. **A shutdown that disconnects a client, or never drains.** → Task 2, `a_shutdown_waits_for_the_last_client_and_still_admits_new_ones`, `a_shutdown_drains_a_daemon_with_no_client_at_once_whatever_the_ttl_or_a_build`; Task 3, `a_shutdown_lets_every_client_finish_and_drains_when_the_last_leaves`; Task 7, `daemon_shutdown_never_disconnects_a_client`.
4. **A look that changes what it looks at.** Taking an owner lock while a shim starts a daemon, recording a sighting, removing a stale file, or creating a key's lock files. → Task 5, `a_key_a_shim_is_starting_is_not_looked_at_under_its_owner_lock`, `a_daemon_that_died_during_its_build_says_when_it_may_restart`, `a_dead_daemon_s_leftovers_are_named_and_left_in_place`, `with_a_repository_only_its_daemon_is_shown`, `a_missing_or_invalid_runtime_directory`, `a_key_that_cannot_be_looked_at_does_not_hide_the_others`.
5. **A control line taken for a `HELLO`, or the reverse.** → Task 1, `only_the_two_verbs_are_control_lines`; Task 3, `a_status_request_is_answered_with_the_daemon_s_state_and_closed`.
6. **A reply that does not fit, or does not round-trip.** A long root, a root with quotes or control characters. → Task 1, `a_long_root_is_clipped_so_the_reply_fits_one_preamble_line`, `a_root_with_any_characters_round_trips`; Task 5, `a_root_with_control_characters_is_printed_on_one_line`, `a_runtime_directory_with_control_characters_is_printed_on_one_line`.
7. **Talking to another user's socket.** → Task 5, `a_peer_of_another_user_is_refused_before_anything_is_sent`.
8. **A daemon that does not answer.** A wedged daemon, an older build, one at its bound. → Task 5, `a_close_a_silence_and_nothing_listening_are_told_apart`, `a_daemon_that_does_not_answer_is_named_alive_with_its_pid_file`; Task 3, `a_control_request_at_the_client_bound_is_answered_busy`; Task 6, `draining_busy_silent_and_refusing_daemons_are_told_apart`.

9. **A probe that arrives late, or a frame the daemon cannot read** (review round 1, sol 3, Opus W3). → Task 3, `a_control_line_completed_after_its_deadline_moves_nothing`, `a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does`.
10. **A file in a key's place that would make the operator wait** (review round 1, sol 2, Opus S6): a FIFO or an oversized build marker. → Task 5, `a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report`, and the shim's `a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key`.
11. **A live daemon taken for none** (review round 1, Opus W1): a full backlog, a daemon claimed and not bound. → Task 5, `a_live_daemon_that_does_not_accept_is_named_with_why`; Task 6, `a_live_daemon_that_does_not_accept_connections_is_not_taken_for_none`.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or comments state, with the test that fails when it breaks and one wrong implementation that test rules out. Every mutant in the last column was applied in validation and the test failed.

| Claim | Test | A mutant it kills |
|---|---|---|
| A control request and a status reply each have exactly one spelling; any other is refused. | T1 `a_request_spelled_otherwise_is_refused`, `a_status_reply_spelled_otherwise_is_refused` | no canonical check on a request (1A), on a reply (1B) |
| Another version of the control exchange is refused before its payload is read. | T1 `another_control_version_is_refused_as_unsupported` | no version check (1E) |
| Only a `STATUS` or `SHUTDOWN` verb followed by a space makes a control line; a `HELLO` never does. | T1 `only_the_two_verbs_are_control_lines`; T3 `a_status_request_is_answered_with_the_daemon_s_state_and_closed` | a prefix match without the space (1D); control lines taken down the `HELLO` path (3C) |
| A status reply always fits one preamble line, whatever the root. | T1 `a_long_root_is_clipped_so_the_reply_fits_one_preamble_line` | no clip (1C) |
| A root with any characters round-trips through the reply. | T1 `a_root_with_any_characters_round_trips` | strings written without JSON escaping (1F) |
| The control exchange's version is not the preamble's, so no identity key changes. | T1 `the_control_version_is_not_the_preamble_s` | (a pin of D97) |
| A control exchange whose line is read whole, answered or refused, and one whose read completes across its deadline, never moves the idle time: an operator who looks does not keep a daemon of this build up. | T2 `a_control_exchange_leaves_the_idle_time_as_it_found_it`; T3 `a_status_probe_does_not_keep_an_idle_daemon_up`, `a_refused_control_line_is_closed_without_a_reply_and_leaves_the_idle_time`, `a_control_line_completed_after_its_deadline_moves_nothing`; T7 `daemon_status_does_not_keep_an_idle_daemon_up` | the probe leaves as a client does (2A; 7A, the same edit, against a real daemon); a refused control line leaves as a client does (3B); an expired control line leaves as a client does (R1) |
| A connection whose line never completes, or is neither a control line nor a `HELLO`, is a client's: the idle time restarts when it goes (what a pre-S5b daemon does with `STATUS`, SI-35). | T3 `a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does` | (a pin of the stated behavior; no mutant) |
| A client that comes and goes during a probe moves the idle time to when it left. | T2 `a_client_that_leaves_during_a_control_exchange_moves_the_idle_time` | `quiet_since` set only when the counts reach zero (2B) |
| A status counts every connection but the request itself. | T2 `a_status_counts_every_connection_but_itself` | the request left in the pending count (2F) |
| A shutdown is recorded only when it names this very instance: identity, pid and start time. | T3 `a_shutdown_for_another_instance_records_nothing` | a pid-only comparison (3A) |
| A shutdown drains a daemon with no client at once, whatever the idle TTL and a running build. | T2 `a_shutdown_drains_a_daemon_with_no_client_at_once_whatever_the_ttl_or_a_build`; T3 `a_shutdown_for_this_instance_drains_an_idle_daemon`; T7 `daemon_shutdown_drains_an_idle_daemon` | the request ignored (2C) |
| A shutdown never drains a daemon while a client is admitted or pending, never disconnects one, and the daemon goes on admitting until it drains. | T2 `a_shutdown_waits_for_the_last_client_and_still_admits_new_ones`; T3 `a_shutdown_lets_every_client_finish_and_drains_when_the_last_leaves`; T7 `daemon_shutdown_never_disconnects_a_client` | the drain before the counts are checked (2D); new clients refused after the request (2E) |
| A request that arrives once the daemon drains, or whose read completes across its deadline, is answered `RETIRING` or nothing, and records no shutdown. | T2 `a_control_exchange_after_the_drain_decision_is_answered_retiring`; T3 `a_control_line_completed_after_its_deadline_moves_nothing` | the shutdown recorded before the drain check (R2); (2F); (R1) |
| The status reports the daemon's idle time. | T3 `a_status_request_is_answered_with_the_daemon_s_state_and_closed`, `a_status_probe_does_not_keep_an_idle_daemon_up` | no idle time (3D) |
| Every read of the clock in the control plane is under the control lock, the control exchange's included. | S4's `every_admission_and_retirement_decision_reads_the_clock_under_the_control_lock`, extended by one call | (S4's mutants) |
| Nothing is sent to a socket whose peer is another user. | T5 `a_peer_of_another_user_is_refused_before_anything_is_sent` | the request written before the peer check (5A) |
| A refused connection is "no daemon" only when nobody holds the key's owner lock: `daemon-status` names a live daemon that does not accept, with the connect error, and `daemon-shutdown` fails for it instead of reporting nothing to shut down. | T5 `a_live_daemon_that_does_not_accept_is_named_with_why`; T6 `a_live_daemon_that_does_not_accept_connections_is_not_taken_for_none`, `a_refused_second_exchange_is_judged_by_the_owner_lock_too` | the connect error dropped (R3); a refused connection taken for no daemon, at the first exchange (R4) and at the second (R10) |
| No key's file makes the operator or a shim wait: the build marker is opened without waiting, a regular file only, and read only 256 bytes far. | T5 `a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report`, the shim's `a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key` | a blocking open (R5, at the watchdog's 30 s); no size bound (R6) |
| One key that cannot be looked at does not hide the others. | T5 `a_key_that_cannot_be_looked_at_does_not_hide_the_others` | the error ends the report (R8) |
| `daemon-status` looks at a key's owner lock only while it holds the key's start lock. | T5 `a_key_a_shim_is_starting_is_not_looked_at_under_its_owner_lock` | no start lock (5B) |
| `daemon-status` never removes a file and never records a sighting. | T5 `a_dead_daemon_s_leftovers_are_named_and_left_in_place`, `a_daemon_that_died_during_its_build_says_when_it_may_restart`, `keys_with_only_lock_files_and_logs_are_counted_not_listed` | the stale files removed (5C) |
| The restart time it shows is `arbitrate`'s rule: 60 s from the first sighting. | T5 `a_daemon_that_died_during_its_build_says_when_it_may_restart` | the whole minute whatever the sighting's age (5D) |
| A key with nothing but lock files and a log is counted, not looked at, and none of a configuration's files is created. | T5 `keys_with_only_lock_files_and_logs_are_counted_not_listed`, `with_a_repository_only_its_daemon_is_shown`, `a_missing_or_invalid_runtime_directory` | every key's locks looked at (5E) |
| Nothing `daemon-status` prints carries a control character, its runtime directory's path included. | T5 `a_root_with_control_characters_is_printed_on_one_line`, `a_runtime_directory_with_control_characters_is_printed_on_one_line` | no escaping of a daemon's lines (5F), of the header (R7) |
| `daemon-shutdown` asks only the instance a status exchange has just shown, or the one `--pid` and `--start-time` name. | T6 `an_idle_daemon_is_asked_by_its_instance_and_waited_for`, `another_instance_named_by_pid_and_start_time_is_left_alone`; T7 `daemon_shutdown_of_another_instance_leaves_the_daemon_alone` | the flags ignored (6A; 7B, in the binary) |
| It never asks a daemon of another identity, and leaves alone a daemon that answers as another instance. | T6 `a_daemon_of_another_identity_is_not_asked`, `a_daemon_replaced_between_the_exchanges_is_left_alone` | no identity check (6C); no instance check of the reply (6D) |
| A lost reply to a sent `SHUTDOWN` is reported as possibly recorded, never as "not shut down". | T6 `a_lost_reply_to_the_shutdown_says_it_may_have_been_recorded` | the lost reply sent through `not_asked` (R9) |
| It says "drained" only once the pid file no longer holds that instance, and its wait is bounded. | T6 `an_idle_daemon_is_asked_by_its_instance_and_waited_for`, `a_daemon_that_does_not_drain_by_the_timeout_is_reported` | success without the wait (6B) |
| With no daemon it creates nothing. | T6 `with_no_daemon_there_is_nothing_to_shut_down` | — |
| Only a runtime directory, or a key's file, that is not there (`NotFound`, or `NotADirectory`) is absent: any other error looking at it is reported, exit 1, by both commands (PR #418 review rounds 1 and 2, sol 1, Opus S1, S2). | T5 and T6 `a_runtime_directory_that_cannot_be_looked_at_is_not_taken_for_none` (an unsearchable parent; skipped under root); T5 `a_key_that_cannot_be_looked_at_after_the_listing_is_not_counted_as_empty`, T6 `a_key_that_cannot_be_looked_at_after_the_status_is_not_taken_for_gone` (the parent made unsearchable after validation, the owner lock held; the permission part skipped under root); T5 `a_runtime_base_that_is_a_file_has_no_daemon`, T6 `a_runtime_base_that_is_a_file_has_nothing_to_shut_down` | any `lstat` error taken for absence (R13, in `present`, which every check uses); the per-key checks only (R14); `NotADirectory` taken for an error (R16) |
| `daemon-status` exits 1 when any key could not be looked at, the report still printed whole (PR #418 review round 2, sol 1). | T5 `a_report_with_a_key_that_cannot_be_looked_at_exits_1` | the count of such keys not kept (R15) |
| During the wait, only a pid file that is not there, or that names another instance, is a drain; one that cannot be read is reported at the timeout (PR #418 review round 1, Opus S2). | T6 `a_pid_file_that_cannot_be_read_during_the_wait_is_not_taken_for_a_drain` (a symlink in its place) | a read error taken for a drain (R12) |

Stated and **not** held by a test: that a daemon built before this slice restarts its idle time at each `STATUS` probe (SI-35): held only by a one-off measurement with a base-build daemon (Validation record), since the suite cannot build another commit's binary, and by the pin of the same mechanism in this build (`a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does`); that an instance is unique as (identity, pid, start second) (an assumption, sol 5); that `run_daemon_shutdown` escapes its line (`one_line`, the same function `daemon-status` uses; the tests call `shutdown` and read its unescaped result); that `KeyDir::pid_text` never follows a symlink and never waits for a writer (it opens through `open_private` with `O_NONBLOCK`, as S5a's `diagnostic::pid_file`, whose tests hold it); that `answer_control`'s reply, at most 4 KiB, is written whole on the non-blocking stream (as every reply of the handshake stage is; a full socket buffer drops it, which the operator reads as no reply); that the binary's `--timeout` bounds the whole `daemon-shutdown` command (held for the function by 6B's test); that `daemon-status` with many wedged daemons takes the timeout once per daemon; a real builder that never returns ended by `daemon-shutdown` (held in process with a blocked builder, T2); x86_64 Linux before CI; that the operator's output is safe in a bidi-aware terminal: `one_line` escapes only Unicode `Cc` (C0, DEL, C1), so bidi and format controls (U+202E, U+2066–2069) and U+2028/2029 pass through, which can reorder a line visually; the helper predates S5b (S5a) and only this user's own paths, reported by a peer-checked same-user daemon, are involved (PR #418 review round 1, Opus S4: a follow-up, also escaping `Cf`, `Zl` and `Zp`, for S5a's diagnostic too).

## File Structure

| File | Responsibility | Change | PR |
|---|---|---|---|
| `src/mcp/shared/control.rs`, `control_tests.rs` | The control exchange on the wire: `ControlRequest`, `Instance`, `DaemonStatus`, `is_control_line`, `display_root`, `CONTROL_VERSION` | Create (Task 1) | S5b-1 |
| `src/mcp/shared/preamble.rs` | Its decoding helpers `pub(super)` (Task 1); `BuildState::word` `pub` (Task 5) | Modify | both |
| `src/mcp/shared/mod.rs` | `pub mod control;` | Modify (Task 1) | S5b-1 |
| `src/mcp/daemon/control.rs` | `StopReason::Shutdown`; `Admission::{quiet_since, shutdown, left}` | Modify (Task 2) | S5b-1 |
| `src/mcp/daemon/admission.rs`, `admission_control_tests.rs`, `admission_tests.rs` | `control_exchange`, `Control`, `ControlSnapshot`; the shutdown in `retire_if_idle`; the clock test extended | Modify, create (Task 2) | S5b-1 |
| `src/mcp/daemon/coordinator.rs` | A stale dead-code comment | Modify (Task 2) | S5b-1 |
| `src/mcp/daemon/server.rs`, `server_control_tests.rs`, `server_support.rs`, `run.rs` | `answer_control`; `Served::build_identity`, `Served::instance` | Modify, create (Task 3) | S5b-1 |
| `src/mcp/shim/control.rs`, `control_tests.rs`, `control_support.rs` | One control exchange; a stand-in daemon for it, `mkfifo`, an independent watchdog | Create (Task 5) | S5b-2 |
| `src/mcp/shim/status.rs`, `status_tests.rs` | `daemon-status` | Create (Task 5) | S5b-2 |
| `src/mcp/shim/connect.rs`, `diagnostic.rs` | `read_reply`, `left`, `pid_file` made `pub(super)` | Modify (Task 5) | S5b-2 |
| `src/mcp/shared/paths.rs`, `paths_tests.rs` | `DaemonPaths::from_locator` | Modify (Task 5) | S5b-2 |
| `src/mcp/shim/key_files.rs` | `KeyDir::read_build_marker`, `Marker`; `build_marker` non-blocking and bounded (Task 5, review round 1, D109); `KeyDir::pid_text` (Task 6) | Modify | S5b-2 |
| `src/mcp/shim/arbitrate_tests.rs` | The shim over a FIFO build marker (review round 1) | Modify (Task 5) | S5b-2 |
| `src/mcp/shim/shutdown.rs`, `shutdown_tests.rs` | `daemon-shutdown` | Create (Task 6) | S5b-2 |
| `src/mcp/shim/mod.rs`, `src/mcp/mod.rs` | The modules and the re-exports | Modify (Tasks 5, 6) | S5b-2 |
| `src/bin/prism-mcp.rs` | `daemon-status`, `daemon-shutdown` dispatched before the standalone parser | Modify (Task 7) | S5b-2 |
| `tests/mcp/daemon_control_process_test.rs`, `main.rs` | The commands as processes against a real daemon and shim | Create, modify (Task 7) | S5b-2 |
| `CLAUDE.md` | MCP Adapter: the daemon's control exchange (Task 4), the operator's commands (Task 8) | Modify | both |
| `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `src/mcp/shim/{arbitrate,pump,spawn,run,socket}.rs`, `tests/fixtures/mcp/golden/*` | The standalone contract and the shim's session | **Must not change** | |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file at that point of the plan, then "with this:" and its replacement.

The replacements were generated from the validated branch's per-task commits, so each carries a few lines of unchanged context around its change; the context is part of the anchor and is not edited.

---

# Part one: S5b-1, the daemon answers the operator (Tasks 1–4)

### Task 1: The control exchange on the wire

**Files:**
- Modify: `src/mcp/shared/mod.rs`, `src/mcp/shared/preamble.rs`
- Create: `src/mcp/shared/control.rs`, `src/mcp/shared/control_tests.rs`

**Interfaces:**
- Consumes: `IdentityKey` (S2a); `BuildState`, `PreambleError`, `PREAMBLE_MAX_BYTES` and the preamble's decoding helpers (S2b: `clip`, `verb_and_payload`, `parse_object`, `key_field`, `require_canonical`, `MAX_VERB_BYTES`, `BuildState::word`/`from_word`), widened from private to `pub(super)`.
- Produces (in `crate::mcp::shared::control`, all `pub`): `CONTROL_VERSION: u32 = 1`; `MAX_ROOT_CHARS: usize = 512`; `fn is_control_line(line: &[u8]) -> bool`; `struct Instance { identity: IdentityKey, pid: u32, start_time: u64 }`; `enum ControlRequest { Status, Shutdown(Instance) }` with `encode(&self) -> String` and `decode(line: &[u8]) -> Result<Self, PreambleError>`; `struct DaemonStatus { instance, build_identity: String, repo_root: String, admitted: u32, pending: u32, max_clients: u32, build: BuildState, attempt: u64, generation: u64, idle_secs: Option<u64>, idle_ttl_secs: u64, shutdown: bool }` with `encode` and `decode`; `fn display_root(root: &Path) -> String`.

- [ ] **Step 1: Write the failing tests**

Create `src/mcp/shared/control_tests.rs`:

````rust
//! Tests for the operator's control exchange on the wire (spec §4.7): one spelling per message,
//! a version of its own, and a status line that fits the preamble's limit whatever the root is.
use super::*;
use crate::mcp::shared::preamble::{BuildState, PREAMBLE_MAX_BYTES};
use crate::mcp::shared::SHARED_PROTOCOL_VERSION;

fn key() -> IdentityKey {
    IdentityKey::from_hex(&"ab".repeat(32)).expect("a key")
}

fn instance() -> Instance {
    Instance {
        identity: key(),
        pid: 4242,
        start_time: 1_760_600_000,
    }
}

fn status() -> DaemonStatus {
    DaemonStatus {
        instance: instance(),
        build_identity: "f".repeat(64),
        repo_root: "/work/repo".to_string(),
        admitted: 2,
        pending: 1,
        max_clients: 64,
        build: BuildState::Ready,
        attempt: 1,
        generation: 3,
        idle_secs: None,
        idle_ttl_secs: 900,
        shutdown: false,
    }
}

/// The line without its newline, as a reader returns it.
fn line(encoded: &str) -> Vec<u8> {
    encoded
        .strip_suffix('\n')
        .expect("an encoded line ends in a newline")
        .as_bytes()
        .to_vec()
}

/// The exact text of each request: what a daemon of another build must be able to read.
#[test]
fn requests_have_one_spelling_and_round_trip() {
    assert_eq!(ControlRequest::Status.encode(), "STATUS {\"control\":1}\n");
    let shutdown = ControlRequest::Shutdown(instance());
    assert_eq!(
        shutdown.encode(),
        format!(
            "SHUTDOWN {{\"control\":1,\"identity\":\"{}\",\"pid\":4242,\"start_time\":1760600000}}\n",
            "ab".repeat(32)
        )
    );
    for request in [ControlRequest::Status, shutdown] {
        assert_eq!(
            ControlRequest::decode(&line(&request.encode())),
            Ok(request)
        );
    }
}

/// A request is refused unless it is spelled as `encode` spells it: another field order,
/// spaces, an unknown or a missing field, a malformed key. Rules out a lenient decoder, which
/// would let two spellings of one request exist.
#[test]
fn a_request_spelled_otherwise_is_refused() {
    let key = "ab".repeat(32);
    let refused = [
        "STATUS {\"control\": 1}".to_string(),
        "STATUS {}".to_string(),
        "STATUS {\"control\":1,\"extra\":0}".to_string(),
        format!("SHUTDOWN {{\"control\":1,\"pid\":4242,\"identity\":\"{key}\",\"start_time\":1}}"),
        format!("SHUTDOWN {{\"control\":1,\"identity\":\"{key}\",\"pid\":4242}}"),
        "SHUTDOWN {\"control\":1,\"identity\":\"AB\",\"pid\":1,\"start_time\":1}".to_string(),
        "STATUS [1]".to_string(),
    ];
    for text in refused {
        assert!(
            ControlRequest::decode(text.as_bytes()).is_err(),
            "accepted: {text}"
        );
    }
}

/// Another version of the control exchange is refused as such, before its payload is read, so
/// a daemon can say why in its log. Rules out reading a future request as this version's.
#[test]
fn another_control_version_is_refused_as_unsupported() {
    assert_eq!(
        ControlRequest::decode(b"STATUS {\"control\":2}"),
        Err(PreambleError::UnsupportedProtocol { got: 2 })
    );
    assert_eq!(
        ControlRequest::decode(b"SHUTDOWN {\"control\":2,\"anything\":true}"),
        Err(PreambleError::UnsupportedProtocol { got: 2 })
    );
}

/// The control exchange has a version of its own: changing the preamble's protocol version
/// (which is part of every identity key) would not change these lines (D97). Rules out a
/// control line that carries the preamble's version.
#[test]
fn the_control_version_is_not_the_preamble_s() {
    assert!(ControlRequest::Status
        .encode()
        .contains(&format!("\"control\":{CONTROL_VERSION}")));
    assert!(!ControlRequest::Status.encode().contains("\"protocol\""));
    assert_eq!(CONTROL_VERSION, 1);
    assert_eq!(
        SHARED_PROTOCOL_VERSION, 1,
        "the preamble's version is unchanged"
    );
}

/// Only a line whose verb is `STATUS` or `SHUTDOWN` is a control line; a `HELLO` and anything
/// that merely starts with those letters are not. Rules out taking a shim's `HELLO` down the
/// control path, or a verb with a longer name for a control verb.
#[test]
fn only_the_two_verbs_are_control_lines() {
    assert!(is_control_line(b"STATUS {\"control\":1}"));
    assert!(is_control_line(b"SHUTDOWN {}"));
    assert!(is_control_line(b"STATUS x"));
    assert!(!is_control_line(b"HELLO {\"protocol\":1}"));
    assert!(!is_control_line(b"STATUSES {}"));
    assert!(!is_control_line(b"STATUS"));
    assert!(!is_control_line(b"status {}"));
    assert!(!is_control_line(b""));
}

/// The status reply round-trips with every field, an idle time present or absent included.
#[test]
fn a_status_reply_round_trips() {
    let busy = status();
    let idle = DaemonStatus {
        admitted: 0,
        pending: 0,
        idle_secs: Some(12),
        shutdown: true,
        build: BuildState::Building,
        ..status()
    };
    for reply in [busy, idle] {
        let encoded = reply.encode();
        assert!(encoded.starts_with("STATUS {\"control\":1,"), "{encoded}");
        assert_eq!(DaemonStatus::decode(&line(&encoded)), Ok(reply));
    }
}

/// A status reply spelled otherwise is refused too. Rules out a decoder that drops the
/// canonical check (a reply with a missing idle time would read as "in use").
#[test]
fn a_status_reply_spelled_otherwise_is_refused() {
    let encoded = status().encode();
    let without_idle = encoded.replace(",\"idle_secs\":null", "");
    assert_ne!(without_idle, encoded);
    assert!(DaemonStatus::decode(&line(&without_idle)).is_err());
    let spaced = encoded.replace("\"pid\":4242", "\"pid\": 4242");
    assert!(DaemonStatus::decode(&line(&spaced)).is_err());
    assert!(DaemonStatus::decode(b"STATUS {\"control\":2}").is_err());
}

/// A root that holds quotes, backslashes, control characters and non-ASCII text comes back as
/// it went: the reply is JSON, not a format string.
#[test]
fn a_root_with_any_characters_round_trips() {
    let reply = DaemonStatus {
        repo_root: "/w/\"q\"\\b\n\u{1b}[31m\u{e9}t\u{e9}".to_string(),
        ..status()
    };
    let encoded = reply.encode();
    assert_eq!(encoded.matches('\n').count(), 1, "one line: {encoded:?}");
    assert_eq!(DaemonStatus::decode(&line(&encoded)), Ok(reply));
}

/// The root a daemon reports is clipped to [`MAX_ROOT_CHARS`] characters, so the reply fits the
/// preamble's 4 KiB line even when every character is one JSON writes as six bytes. Rules out
/// an unclipped root, whose reply the operator's reader refuses as too long.
#[test]
fn a_long_root_is_clipped_so_the_reply_fits_one_preamble_line() {
    let worst = format!("/{}", "\u{1}".repeat(4000));
    let shown = display_root(std::path::Path::new(&worst));
    assert_eq!(shown.chars().count(), MAX_ROOT_CHARS + 1);
    assert!(shown.ends_with('\u{2026}'), "marked as clipped");
    let reply = DaemonStatus {
        repo_root: shown,
        ..status()
    };
    let encoded = reply.encode();
    assert!(
        encoded.len() <= PREAMBLE_MAX_BYTES,
        "{} bytes",
        encoded.len()
    );
    assert_eq!(DaemonStatus::decode(&line(&encoded)), Ok(reply));
    assert_eq!(
        display_root(std::path::Path::new("/short/root")),
        "/short/root",
        "a short root is not marked"
    );
}

/// A root that is not UTF-8 is shown with its undecodable bytes replaced: the status is for a
/// person to read, and the key, not the root, names the daemon.
#[test]
fn a_root_that_is_not_utf8_is_shown_lossily() {
    use std::os::unix::ffi::OsStrExt;
    let root = std::path::Path::new(std::ffi::OsStr::from_bytes(b"/w/\xffx"));
    assert_eq!(display_root(root), "/w/\u{fffd}x");
}
````

In `src/mcp/shared/mod.rs`, replace this:

````rust

mod acl;
pub mod cache_base;
pub mod dir_chain;
pub mod identity;
pub mod paths;
````

with this:

````rust

mod acl;
pub mod cache_base;
pub mod control;
pub mod dir_chain;
pub mod identity;
pub mod paths;
````

- [ ] **Step 2: Run them to see them fail**

```bash
LOG_DIR=/tmp/s5b; mkdir -p "$LOG_DIR"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t1-red.log" 2>&1
```

Expected in the log: ``error[E0583]: file not found for module `control` `` (the module line is in place; its file comes in Step 3) and ``could not compile `prism` (lib test) due to 1 previous error``.

- [ ] **Step 3: Implement**

Create `src/mcp/shared/control.rs`:

````rust
//! The operator's control exchange with a running daemon (spec §4.7): `daemon-status` asks a
//! daemon what it is doing; `daemon-shutdown` asks one instance of it to drain once it has no
//! client. One line each way on a fresh connection, in place of the preamble's `HELLO`; then the
//! daemon closes the connection. The connection is never admitted, and the daemon's idle time
//! does not count it (S4's interface for S5b).
//!
//! ```text
//! operator -> daemon:  STATUS {"control":1}
//!                      SHUTDOWN {"control":1,"identity":"<64 hex>","pid":N,"start_time":N}
//! daemon -> operator:  STATUS {"control":1,"identity":"<64 hex>","pid":N,"start_time":N,...}
//!                  or  RETIRING {}  |  BUSY {"max_clients":N}  |  nothing, for a line it refuses
//! ```
//!
//! The exchange has a version of its own, `control`, apart from the preamble's `protocol`
//! (D97). The preamble's version is part of every identity key, and `daemon-status` lists the
//! daemons of every build in the runtime directory (spec §4.6): an operator tool must be able to
//! ask a daemon of another build. A daemon that does not know these verbs, or this version,
//! closes the connection without a reply. Decoding is as strict as the preamble's: one spelling
//! per message, the one `encode` writes.
use super::identity::IdentityKey;
use super::preamble::{
    clip, key_field, parse_object, require_canonical, verb_and_payload, BuildState, PreambleError,
    MAX_VERB_BYTES,
};
use serde::Deserialize;
use std::path::Path;

/// Version of the control exchange. Not part of any identity key.
pub const CONTROL_VERSION: u32 = 1;

/// Most characters of the repository root a status reply carries. A JSON string can take six
/// bytes per character, and the reply must fit one preamble line (4 KiB) with its other fields.
pub const MAX_ROOT_CHARS: usize = 512;

/// The verbs of the control exchange. Both requests and the status reply use them.
const CONTROL_VERBS: [&str; 2] = ["STATUS", "SHUTDOWN"];

/// Whether `line` (without its newline) is a control request rather than a `HELLO`: its verb is
/// `STATUS` or `SHUTDOWN`, followed by a space. Whether it is a valid one is
/// [`ControlRequest::decode`]'s to say.
pub fn is_control_line(line: &[u8]) -> bool {
    CONTROL_VERBS.iter().any(|verb| {
        line.strip_prefix(verb.as_bytes())
            .is_some_and(|rest| rest.first() == Some(&b' '))
    })
}

/// One daemon instance: the identity it serves, its pid and its start time (seconds since the
/// Unix epoch), as `READY` and the pid file name it (spec §4.7: "the instance verified through
/// the live control exchange (pid + start time echoed in `READY`)").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instance {
    pub identity: IdentityKey,
    pub pid: u32,
    pub start_time: u64,
}

/// What an operator asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlRequest {
    /// Say what you are doing.
    Status,
    /// If you are this instance, drain once nothing is admitted or pending (spec §11 item 3:
    /// graceful and instance-bound). A daemon that is another instance records nothing.
    Shutdown(Instance),
}

/// Read `control` alone and refuse any version but this build's, before the rest of the
/// payload is interpreted.
fn check_control(payload: &str) -> Result<(), PreambleError> {
    #[derive(Deserialize)]
    struct Versioned {
        control: u64,
    }
    let Versioned { control } = parse_object(payload)?;
    if control != u64::from(CONTROL_VERSION) {
        return Err(PreambleError::UnsupportedProtocol { got: control });
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusWire {
    #[allow(dead_code)] // read by `check_control`
    control: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShutdownWire {
    #[allow(dead_code)] // read by `check_control`
    control: u64,
    identity: String,
    pid: u32,
    start_time: u64,
}

impl ControlRequest {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        match self {
            Self::Status => format!("STATUS {{\"control\":{CONTROL_VERSION}}}\n"),
            Self::Shutdown(instance) => format!(
                "SHUTDOWN {{\"control\":{CONTROL_VERSION},\"identity\":\"{}\",\"pid\":{},\"start_time\":{}}}\n",
                instance.identity, instance.pid, instance.start_time
            ),
        }
    }

    /// Decode one received line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (verb, payload) = verb_and_payload(line, &CONTROL_VERBS)?;
        check_control(payload)?;
        let request = match verb {
            "STATUS" => {
                let StatusWire { .. } = parse_object(payload)?;
                Self::Status
            }
            "SHUTDOWN" => {
                let wire: ShutdownWire = parse_object(payload)?;
                Self::Shutdown(Instance {
                    identity: key_field("identity", &wire.identity)?,
                    pid: wire.pid,
                    start_time: wire.start_time,
                })
            }
            other => {
                return Err(PreambleError::UnknownVerb {
                    verb: clip(other, MAX_VERB_BYTES),
                })
            }
        };
        require_canonical(line, &request.encode())?;
        Ok(request)
    }
}

/// What a daemon says about itself (spec §4.7: "key, repo root, build identity, pid, start
/// time, admitted and pending clients, build state and attempt, generation, idle time").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonStatus {
    pub instance: Instance,
    /// `PRISM_CACHE_BUILD_IDENTITY` of the daemon's build (the pid file's third line).
    pub build_identity: String,
    /// The canonical repository root, for a person to read ([`display_root`]).
    pub repo_root: String,
    /// Connections served now, and connections in the handshake stage besides this one.
    pub admitted: u32,
    pub pending: u32,
    pub max_clients: u32,
    pub build: BuildState,
    /// The number of the latest build attempt; 0 before the first.
    pub attempt: u64,
    pub generation: u64,
    /// Seconds since the daemon last had a client; `None` while it has one.
    pub idle_secs: Option<u64>,
    pub idle_ttl_secs: u64,
    /// Whether an operator has asked it to drain once it has no client.
    pub shutdown: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DaemonStatusWire {
    #[allow(dead_code)] // read by `check_control`
    control: u64,
    identity: String,
    pid: u32,
    start_time: u64,
    build_identity: String,
    repo_root: String,
    admitted: u32,
    pending: u32,
    max_clients: u32,
    build: String,
    attempt: u64,
    generation: u64,
    idle_secs: Option<u64>,
    idle_ttl_secs: u64,
    shutdown: bool,
}

/// `text` as a JSON string, quotes included.
fn json_string(text: &str) -> String {
    serde_json::Value::from(text).to_string()
}

impl DaemonStatus {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        let idle = self
            .idle_secs
            .map_or_else(|| "null".to_string(), |secs| secs.to_string());
        format!(
            "STATUS {{\"control\":{CONTROL_VERSION},\"identity\":\"{}\",\"pid\":{},\"start_time\":{},\"build_identity\":{},\"repo_root\":{},\"admitted\":{},\"pending\":{},\"max_clients\":{},\"build\":\"{}\",\"attempt\":{},\"generation\":{},\"idle_secs\":{idle},\"idle_ttl_secs\":{},\"shutdown\":{}}}\n",
            self.instance.identity,
            self.instance.pid,
            self.instance.start_time,
            json_string(&self.build_identity),
            json_string(&self.repo_root),
            self.admitted,
            self.pending,
            self.max_clients,
            self.build.word(),
            self.attempt,
            self.generation,
            self.idle_ttl_secs,
            self.shutdown,
        )
    }

    /// Decode one received line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (_, payload) = verb_and_payload(line, &["STATUS"])?;
        check_control(payload)?;
        let wire: DaemonStatusWire = parse_object(payload)?;
        let status = Self {
            instance: Instance {
                identity: key_field("identity", &wire.identity)?,
                pid: wire.pid,
                start_time: wire.start_time,
            },
            build_identity: wire.build_identity,
            repo_root: wire.repo_root,
            admitted: wire.admitted,
            pending: wire.pending,
            max_clients: wire.max_clients,
            build: BuildState::from_word(&wire.build)?,
            attempt: wire.attempt,
            generation: wire.generation,
            idle_secs: wire.idle_secs,
            idle_ttl_secs: wire.idle_ttl_secs,
            shutdown: wire.shutdown,
        };
        require_canonical(line, &status.encode())?;
        Ok(status)
    }
}

/// The repository root as a status reply shows it: lossy where it is not UTF-8, and cut to
/// [`MAX_ROOT_CHARS`] characters with `…` after it when longer.
pub fn display_root(root: &Path) -> String {
    let text = root.to_string_lossy();
    if text.chars().count() <= MAX_ROOT_CHARS {
        return text.into_owned();
    }
    let mut shown: String = text.chars().take(MAX_ROOT_CHARS).collect();
    shown.push('\u{2026}');
    shown
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust

/// Longest piece of the other side's text an error message repeats.
const MAX_DETAIL_BYTES: usize = 200;
const MAX_VERB_BYTES: usize = 16;

/// Why a preamble line was refused. Nothing here is retried; what a daemon or a shim does
/// about each is theirs to decide (spec §10).
````

with this:

````rust

/// Longest piece of the other side's text an error message repeats.
const MAX_DETAIL_BYTES: usize = 200;
pub(super) const MAX_VERB_BYTES: usize = 16;

/// Why a preamble line was refused. Nothing here is retried; what a daemon or a shim does
/// about each is theirs to decide (spec §10).
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust
}

/// At most `max` bytes of `text`, cut on a character boundary.
fn clip(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
````

with this:

````rust
}

/// At most `max` bytes of `text`, cut on a character boundary.
pub(super) fn clip(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust
}

impl BuildState {
    fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
````

with this:

````rust
}

impl BuildState {
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust
        }
    }

    fn from_word(word: &str) -> Result<Self, PreambleError> {
        match word {
            "idle" => Ok(Self::Idle),
            "building" => Ok(Self::Building),
````

with this:

````rust
        }
    }

    pub(super) fn from_word(word: &str) -> Result<Self, PreambleError> {
        match word {
            "idle" => Ok(Self::Idle),
            "building" => Ok(Self::Building),
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust

/// Split a line into its verb and payload after the checks every line gets: length, UTF-8,
/// shape, and that the verb is one of `verbs`.
fn verb_and_payload<'l>(
    line: &'l [u8],
    verbs: &[&'static str],
) -> Result<(&'static str, &'l str), PreambleError> {
````

with this:

````rust

/// Split a line into its verb and payload after the checks every line gets: length, UTF-8,
/// shape, and that the verb is one of `verbs`.
pub(super) fn verb_and_payload<'l>(
    line: &'l [u8],
    verbs: &[&'static str],
) -> Result<(&'static str, &'l str), PreambleError> {
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust

/// Parse a payload as the JSON object `T`. Only an object is accepted: `serde` would also take
/// a positional array for a struct, and nothing here writes one.
fn parse_object<'p, T: Deserialize<'p>>(payload: &'p str) -> Result<T, PreambleError> {
    if !payload.starts_with('{') {
        return Err(invalid("the payload is not a JSON object"));
    }
````

with this:

````rust

/// Parse a payload as the JSON object `T`. Only an object is accepted: `serde` would also take
/// a positional array for a struct, and nothing here writes one.
pub(super) fn parse_object<'p, T: Deserialize<'p>>(payload: &'p str) -> Result<T, PreambleError> {
    if !payload.starts_with('{') {
        return Err(invalid("the payload is not a JSON object"));
    }
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust
    Ok(())
}

fn key_field(name: &str, hex: &str) -> Result<IdentityKey, PreambleError> {
    IdentityKey::from_hex(hex)
        .map_err(|error| invalid(format!("{name} is not an identity key: {error}")))
}

/// Refuse a decoded message whose line is not byte for byte what `encode` writes for it.
fn require_canonical(line: &[u8], encoded: &str) -> Result<(), PreambleError> {
    if encoded.strip_suffix('\n').map(str::as_bytes) == Some(line) {
        Ok(())
    } else {
````

with this:

````rust
    Ok(())
}

pub(super) fn key_field(name: &str, hex: &str) -> Result<IdentityKey, PreambleError> {
    IdentityKey::from_hex(hex)
        .map_err(|error| invalid(format!("{name} is not an identity key: {error}")))
}

/// Refuse a decoded message whose line is not byte for byte what `encode` writes for it.
pub(super) fn require_canonical(line: &[u8], encoded: &str) -> Result<(), PreambleError> {
    if encoded.strip_suffix('\n').map(str::as_bytes) == Some(line) {
        Ok(())
    } else {
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t1.log" 2>&1
```

Expected: `test result: ok. 581 passed; 0 failed` (571 before + 10), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/shared/mod.rs src/mcp/shared/preamble.rs src/mcp/shared/control.rs src/mcp/shared/control_tests.rs
git commit -m "feat(mcp): the operator's control exchange on the wire"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::shared::control::`, see the named test fail, then `git checkout -- src/mcp/shared/control.rs`.

| Mutant | Edit | Fails |
|---|---|---|
| 1A | in `ControlRequest::decode`, delete `require_canonical(line, &request.encode())?;` | `a_request_spelled_otherwise_is_refused` |
| 1B | in `DaemonStatus::decode`, delete `require_canonical(line, &status.encode())?;` | `a_status_reply_spelled_otherwise_is_refused` |
| 1C | in `display_root`, `if text.chars().count() <= MAX_ROOT_CHARS {` → `if true {` | `a_long_root_is_clipped_so_the_reply_fits_one_preamble_line` |
| 1D | in `is_control_line`, `.is_some_and(|rest| rest.first() == Some(&b' '))` → `.is_some()` | `only_the_two_verbs_are_control_lines` |
| 1E | in `check_control`, `if control != u64::from(CONTROL_VERSION) {` → `if false {` | `another_control_version_is_refused_as_unsupported` |
| 1F | in `json_string`, `serde_json::Value::from(text).to_string()` → `format!("\"{text}\"")` | `a_root_with_any_characters_round_trips`, `a_long_root_is_clipped_so_the_reply_fits_one_preamble_line` |

### Task 2: The control exchange in the control plane

**Files:**
- Modify: `src/mcp/daemon/control.rs`, `src/mcp/daemon/admission.rs`, `src/mcp/daemon/admission_tests.rs`, `src/mcp/daemon/coordinator.rs`
- Create: `src/mcp/daemon/admission_control_tests.rs`

**Interfaces:**
- Consumes: `DaemonState::control_plane`, `now`, `ControlState::build_status`, `BuildStatus` (S3a, S4); `Admission`, `StopReason`, `enter_handshake`, `leave_handshake`, `admit`, `release`, `retire_if_idle`, `drain` (S4); `test_support` (`fixture`, `scripted`, `state_with`, `advance`, `build_after`, `Gate`, `Conn`, `Outcome`, `status`, `ManualClock`).
- Produces: in `daemon::control`: `StopReason::Shutdown`; `Admission { quiet_since: Option<Instant>, shutdown: bool, .. }` and `Admission::left(&mut self, now: Instant)` (a connection that was not a control exchange left the counts; `leave_handshake`, `admit`'s refusal and `release` use it). In `daemon::admission` (`pub(super)`): `enum Control { Status(ControlSnapshot), Retiring }`; `struct ControlSnapshot { build: BuildStatus, admitted: u32, pending: u32, idle_for: Option<Duration>, shutdown: bool }`; `DaemonState::control_exchange(&self, shutdown: bool) -> Result<Control, Poisoned>`; `retire_if_idle` drains with `StopReason::Shutdown` when a shutdown was recorded and nothing is admitted or pending.

- [ ] **Step 1: Write the failing tests**

In `src/mcp/daemon/admission.rs`, replace this:

````rust
#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
````

with this:

````rust
#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "admission_control_tests.rs"]
mod control_tests;
````

Create `src/mcp/daemon/admission_control_tests.rs`:

````rust
//! Tests for the control exchange in the control plane (spec §4.7; S4's interface for S5b): an
//! operator's request leaves the daemon's idle time as it found it, a status counts everyone
//! but the request itself, and a shutdown request drains the daemon once nothing is admitted or
//! pending, whatever the idle TTL and a running build. Time is a clock the test moves.
use super::super::test_support::{
    advance, build_after, fixture, scripted, state_with, status, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::RefreshPolicy;

const TTL: Duration = Duration::from_secs(900);

fn lazy_state() -> (tempfile::TempDir, Arc<ManualClock>, Arc<DaemonState>) {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    (dir, clock, state)
}

fn retire(state: &DaemonState) -> Option<StopReason> {
    state
        .retire_if_idle(TTL, || {})
        .expect("the control lock is not poisoned")
}

/// A probe: counted as pending when it is accepted, then answered as a control request.
fn probe(state: &DaemonState, shutdown: bool) -> Control {
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    state
        .control_exchange(shutdown)
        .expect("the control lock is not poisoned")
}

fn snapshot(control: Control) -> ControlSnapshot {
    match control {
        Control::Status(snapshot) => snapshot,
        Control::Retiring => panic!("the daemon is not draining"),
    }
}

/// S4's interface for S5b: a status probe is counted as pending while its line is read, and must
/// not restart the idle time when it leaves, or an operator who looks at a daemon keeps it up.
/// Rules out a probe that leaves the way any other connection does (`leave_handshake`).
#[test]
fn a_control_exchange_leaves_the_idle_time_as_it_found_it() {
    let (_dir, clock, state) = lazy_state();
    assert_eq!(
        retire(&state),
        None,
        "the first decision starts the idle count"
    );
    advance(&clock, &state, TTL.as_secs() - 1);
    let answered = snapshot(probe(&state, false));
    assert_eq!(answered.idle_for, Some(TTL - Duration::from_secs(1)));
    assert_eq!(retire(&state), None, "one second short of the TTL");
    advance(&clock, &state, 1);
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// A client that comes and goes while a probe is in the handshake stage moves the idle time to
/// when it left, as it would have without the probe. Rules out a probe that puts back the idle
/// time it found when it entered.
#[test]
fn a_client_that_leaves_during_a_control_exchange_moves_the_idle_time() {
    let (_dir, clock, state) = lazy_state();
    assert_eq!(retire(&state), None);
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending), "the probe");
    advance(&clock, &state, 5);
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending), "a client");
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    advance(&clock, &state, 5);
    state.release().expect("release");
    let answered = snapshot(state.control_exchange(false).expect("control"));
    assert_eq!(
        answered.idle_for,
        Some(Duration::ZERO),
        "idle since the client left"
    );
    advance(&clock, &state, TTL.as_secs() - 1);
    assert_eq!(
        retire(&state),
        None,
        "the TTL counts from the client's release"
    );
    advance(&clock, &state, 1);
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// The status counts every connection but the request itself, and has no idle time while a
/// client is connected. Rules out a status read before the request has left the pending count.
#[test]
fn a_status_counts_every_connection_but_itself() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    let answered = snapshot(probe(&state, false));
    assert_eq!(
        answered,
        ControlSnapshot {
            build: BuildStatus {
                state: BuildState::Idle,
                attempt: 0,
                generation: 0,
                waiting: 0,
            },
            admitted: 1,
            pending: 1,
            idle_for: None,
            shutdown: false,
        }
    );
    assert_eq!(
        state.admission_status().expect("counts"),
        AdmissionStatus {
            pending: 1,
            admitted: 1,
            draining: None,
        },
        "the probe is no longer counted"
    );
}

/// Spec §4.7: `daemon-shutdown` drains a daemon with no client at once: the idle TTL does not
/// apply, and neither does a running build (S4's SI-15: a build that never returns would keep the
/// daemon up for ever). Rules out a shutdown that only shortens the TTL, or that waits for the
/// build.
#[test]
fn a_shutdown_drains_a_daemon_with_no_client_at_once_whatever_the_ttl_or_a_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    assert_eq!(status(&state).state, BuildState::Building);
    assert_eq!(retire(&state), None);

    let answered = snapshot(probe(&state, true));
    assert!(answered.shutdown);
    let mut unlinked = 0;
    assert_eq!(
        state.retire_if_idle(TTL, || unlinked += 1),
        Ok(Some(StopReason::Shutdown))
    );
    assert_eq!(
        unlinked, 1,
        "the socket is unlinked in the same critical section"
    );
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "the build still runs"
    );
    gate.open();
}

/// Spec §4.7 and §11 item 3: a shutdown never disconnects a client; the daemon drains when the
/// last one has gone. Until then it goes on admitting (D100), so a client that arrives is served
/// rather than failed closed. Rules out a drain while a client is admitted, and a daemon that
/// refuses new clients after the request.
#[test]
fn a_shutdown_waits_for_the_last_client_and_still_admits_new_ones() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    let answered = snapshot(probe(&state, true));
    assert_eq!((answered.admitted, answered.shutdown), (1, true));
    assert_eq!(retire(&state), None, "a client is admitted");

    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert_eq!(retire(&state), None, "a client is pending");
    assert!(
        matches!(state.admit(), Ok(Admit::Ready(_))),
        "a new client is still admitted"
    );
    state.release().expect("release");
    assert_eq!(retire(&state), None, "one client left");
    state.release().expect("release");
    assert_eq!(retire(&state), Some(StopReason::Shutdown));
}

/// A request that arrives once the daemon drains is answered `RETIRING`, leaves the pending
/// count, and records nothing.
#[test]
fn a_control_exchange_after_the_drain_decision_is_answered_retiring() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert_eq!(
        state.drain(StopReason::Signal, || {}),
        Ok(StopReason::Signal)
    );
    assert_eq!(state.control_exchange(true), Ok(Control::Retiring));
    assert_eq!(
        state.admission_status().expect("counts"),
        AdmissionStatus {
            pending: 0,
            admitted: 0,
            draining: Some(StopReason::Signal),
        }
    );
    assert!(
        !state
            .control_plane()
            .expect("the control lock")
            .admission
            .shutdown,
        "no shutdown was recorded (review round 1, Opus S2)"
    );
    assert_eq!(
        retire(&state),
        Some(StopReason::Signal),
        "the first reason stands"
    );
}
````

In `src/mcp/daemon/admission_tests.rs`, replace this:

````rust
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    state.release().expect("release");
    assert_eq!(state.retire_if_idle(TTL, || {}), Ok(None));

    let seen = clock.seen.lock().expect("seen").clone();
    assert_eq!(
        seen.len(),
        4,
        "one read each by leave, admit, release and retire"
    );
    assert!(seen.iter().all(|held| *held), "{seen:?}");
}
````

with this:

````rust
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    state.release().expect("release");
    assert_eq!(state.retire_if_idle(TTL, || {}), Ok(None));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(
        state.control_exchange(false),
        Ok(Control::Status(_))
    ));

    let seen = clock.seen.lock().expect("seen").clone();
    assert_eq!(
        seen.len(),
        5,
        "one read each by leave, admit, release, retire and the control exchange"
    );
    assert!(seen.iter().all(|held| *held), "{seen:?}");
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t2-red.log" 2>&1
```

Expected in the log: fifteen errors: `E0425`, `E0433` and `E0422` for `Control` and `ControlSnapshot` (eight), `E0599` for `control_exchange` (four) and for `StopReason::Shutdown` (two), `E0609` for `Admission.shutdown`; ``could not compile `prism` (lib test) due to 15 previous errors``.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/admission.rs`, replace this:

````rust
    Retiring,
}

/// The connection counts, for tests and the status probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // read by tests until S5b's status probe
pub(super) struct AdmissionStatus {
    pub(super) pending: u32,
    pub(super) admitted: u32,
````

with this:

````rust
    Retiring,
}

/// What a control request is answered (spec §4.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Control {
    /// The daemon's state, read in the critical section in which the request left the counts.
    Status(ControlSnapshot),
    /// The daemon drains: answer `RETIRING`. Nothing was recorded.
    Retiring,
}

/// The control plane's half of a status reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ControlSnapshot {
    pub(super) build: BuildStatus,
    /// Admitted and pending connections, the request itself not counted.
    pub(super) admitted: u32,
    pub(super) pending: u32,
    /// How long nothing has been admitted or pending; `None` while something is.
    pub(super) idle_for: Option<Duration>,
    /// Whether a shutdown has been requested, by this request or an earlier one.
    pub(super) shutdown: bool,
}

/// The connection counts, for tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // read by tests only
pub(super) struct AdmissionStatus {
    pub(super) pending: u32,
    pub(super) admitted: u32,
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
        let now = self.now();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        if admission.pending == 0 && admission.admitted == 0 {
            admission.idle_since = Some(now);
        }
        Ok(())
    }

````

with this:

````rust
        let now = self.now();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        admission.left(now);
        Ok(())
    }

````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        if admission.draining.is_some() {
            if admission.pending == 0 && admission.admitted == 0 {
                admission.idle_since = Some(now);
            }
            return Ok(Admit::Retiring);
        }
        admission.admitted = admission.admitted.saturating_add(1);
````

with this:

````rust
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        if admission.draining.is_some() {
            admission.left(now);
            return Ok(Admit::Retiring);
        }
        admission.admitted = admission.admitted.saturating_add(1);
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
        let now = self.now();
        let admission = &mut control.admission;
        admission.admitted = admission.admitted.saturating_sub(1);
        if admission.pending == 0 && admission.admitted == 0 {
            admission.idle_since = Some(now);
        }
        Ok(())
    }

    /// The retirement decision (spec §4.2): if nothing has been admitted or pending for
    /// `idle_ttl` and no build is running, the daemon drains from now on, and `unlink` (the
    /// socket's removal) runs inside the same critical section. Returns why the daemon drains,
    /// if it does, whether it started now or earlier.
    pub(super) fn retire_if_idle(
        &self,
        idle_ttl: Duration,
````

with this:

````rust
        let now = self.now();
        let admission = &mut control.admission;
        admission.admitted = admission.admitted.saturating_sub(1);
        admission.left(now);
        Ok(())
    }

    /// The retirement decision (spec §4.2): if nothing has been admitted or pending for
    /// `idle_ttl` and no build is running, the daemon drains from now on, and `unlink` (the
    /// socket's removal) runs inside the same critical section. An operator's shutdown request
    /// drains it as soon as nothing is admitted or pending, whatever the TTL and a running build
    /// (spec §4.7, D100). Returns why the daemon drains, if it does, whether it started now or
    /// earlier.
    pub(super) fn retire_if_idle(
        &self,
        idle_ttl: Duration,
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
        if admission.pending != 0 || admission.admitted != 0 {
            return Ok(None);
        }
        let since = *admission.idle_since.get_or_insert(now);
        if now.saturating_duration_since(since) < idle_ttl
            || matches!(control.build, BuildPhase::Building { .. })
        {
````

with this:

````rust
        if admission.pending != 0 || admission.admitted != 0 {
            return Ok(None);
        }
        if admission.shutdown {
            admission.draining = Some(StopReason::Shutdown);
            unlink();
            return Ok(Some(StopReason::Shutdown));
        }
        let since = *admission.idle_since.get_or_insert(now);
        admission.quiet_since.get_or_insert(since);
        if now.saturating_duration_since(since) < idle_ttl
            || matches!(control.build, BuildPhase::Building { .. })
        {
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
            .and_then(|since| since.checked_add(idle_ttl)))
    }

    #[cfg_attr(not(test), allow(dead_code))] // read by tests until S5b's status probe
    pub(super) fn admission_status(&self) -> Result<AdmissionStatus, Poisoned> {
        let control = self.control_plane()?;
        Ok(AdmissionStatus {
````

with this:

````rust
            .and_then(|since| since.checked_add(idle_ttl)))
    }

    /// A pending connection whose line was a control request leaves the handshake stage as if it
    /// had never been counted: if nothing is left pending or admitted, the idle time is the one
    /// it found (`quiet_since`), so an operator's probe does not keep an idle daemon up (S4's
    /// interface for S5b). With `shutdown`, the daemon drains once nothing is admitted or pending
    /// (D100). Unless the daemon drains already, the answer is its state, read in the same
    /// critical section, after the request has left the counts.
    pub(super) fn control_exchange(&self, shutdown: bool) -> Result<Control, Poisoned> {
        let mut control = self.control_plane()?;
        // Read inside the critical section, as every admission function does (review round 1).
        let now = self.now();
        let build = control.build_status();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        let idle = admission.pending == 0 && admission.admitted == 0;
        if idle {
            admission.idle_since = admission.quiet_since;
        }
        if admission.draining.is_some() {
            return Ok(Control::Retiring);
        }
        admission.shutdown |= shutdown;
        Ok(Control::Status(ControlSnapshot {
            build,
            admitted: admission.admitted,
            pending: admission.pending,
            idle_for: admission
                .idle_since
                .filter(|_| idle)
                .map(|since| now.saturating_duration_since(since)),
            shutdown: admission.shutdown,
        }))
    }

    #[cfg_attr(not(test), allow(dead_code))] // read by tests only
    pub(super) fn admission_status(&self) -> Result<AdmissionStatus, Poisoned> {
        let control = self.control_plane()?;
        Ok(AdmissionStatus {
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    FilesReplaced,
    /// SIGTERM or SIGINT.
    Signal,
}

impl StopReason {
````

with this:

````rust
    FilesReplaced,
    /// SIGTERM or SIGINT.
    Signal,
    /// An operator's `daemon-shutdown`, once nothing was admitted or pending (spec §4.7).
    Shutdown,
}

impl StopReason {
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
            Self::RootChanged => "the repository root was lost or replaced",
            Self::FilesReplaced => "its socket or owner lock file was replaced",
            Self::Signal => "a termination signal",
        }
    }
}
````

with this:

````rust
            Self::RootChanged => "the repository root was lost or replaced",
            Self::FilesReplaced => "its socket or owner lock file was replaced",
            Self::Signal => "a termination signal",
            Self::Shutdown => "an operator's daemon-shutdown, with no client left",
        }
    }
}
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    /// retirement decision have seen; `None` while either is not, and before the first
    /// decision (which starts the count for a daemon nobody has reached yet).
    pub(super) idle_since: Option<Instant>,
    /// Set once, by the critical section that decides the daemon drains, and never cleared:
    /// from then on no connection is admitted.
    pub(super) draining: Option<StopReason>,
````

with this:

````rust
    /// retirement decision have seen; `None` while either is not, and before the first
    /// decision (which starts the count for a daemon nobody has reached yet).
    pub(super) idle_since: Option<Instant>,
    /// When a connection that was not a control exchange last left the counts (or the first
    /// retirement decision). Unlike `idle_since`, accepting a connection does not clear it, so a
    /// control exchange, which is counted as pending until its line is read, can leave the idle
    /// time as it found it (S4's interface for S5b: an operator's probe must not keep an idle
    /// daemon up).
    pub(super) quiet_since: Option<Instant>,
    /// Whether an operator asked this daemon to drain once nothing is admitted or pending
    /// (spec §4.7, D100). Set once and never cleared.
    pub(super) shutdown: bool,
    /// Set once, by the critical section that decides the daemon drains, and never cleared:
    /// from then on no connection is admitted.
    pub(super) draining: Option<StopReason>,
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
            pending: 0,
            admitted: 0,
            idle_since: None,
            draining: None,
        }
    }
}
````

with this:

````rust
            pending: 0,
            admitted: 0,
            idle_since: None,
            quiet_since: None,
            shutdown: false,
            draining: None,
        }
    }

    /// A connection that was not a control exchange left the counts at `now`: the idle time
    /// starts now if nothing is left.
    pub(super) fn left(&mut self, now: Instant) {
        self.quiet_since = Some(now);
        if self.pending == 0 && self.admitted == 0 {
            self.idle_since = Some(now);
        }
    }
}
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    }

    /// The control plane's view of the build.
    #[cfg_attr(not(test), allow(dead_code))] // read by tests until S5b's status probe
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        Ok(self.control_plane()?.build_status())
    }
````

with this:

````rust
    }

    /// The control plane's view of the build.
    #[cfg_attr(not(test), allow(dead_code))] // read by tests only
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        Ok(self.control_plane()?.build_status())
    }
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t2.log" 2>&1
```

Expected: `test result: ok. 587 passed; 0 failed` (581 + 6), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/control.rs src/mcp/daemon/admission.rs src/mcp/daemon/admission_tests.rs src/mcp/daemon/admission_control_tests.rs src/mcp/daemon/coordinator.rs
git commit -m "feat(mcp): a control exchange in the daemon's control plane; a shutdown drains once no client is left"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::daemon::admission::`, see the named tests fail, then `git checkout -- <file>`.

| Mutant | Edit | Fails |
|---|---|---|
| 2A | in `control_exchange`, `admission.idle_since = admission.quiet_since;` → `admission.idle_since = Some(now);` (a probe leaves as a client does) | `a_control_exchange_leaves_the_idle_time_as_it_found_it` |
| 2B | in `control.rs`, `Admission::left`: set `quiet_since` only inside `if self.pending == 0 && self.admitted == 0 { … }` | `a_client_that_leaves_during_a_control_exchange_moves_the_idle_time` |
| 2C | in `retire_if_idle`, `if admission.shutdown {` → `if false {` | `a_shutdown_drains_a_daemon_with_no_client_at_once_whatever_the_ttl_or_a_build`, `a_shutdown_waits_for_the_last_client_and_still_admits_new_ones` |
| 2D | in `retire_if_idle`, move the `if admission.shutdown { … }` block above the `pending != 0 || admitted != 0` check | `a_shutdown_waits_for_the_last_client_and_still_admits_new_ones` |
| 2E | in `admit`, `if admission.draining.is_some() {` → `if admission.draining.is_some() || admission.shutdown {` | `a_shutdown_waits_for_the_last_client_and_still_admits_new_ones` |
| 2F | in `control_exchange`, delete `admission.pending = admission.pending.saturating_sub(1);` | `a_status_counts_every_connection_but_itself` and the five other tests of the file |
| R2 | in `control_exchange`, move `admission.shutdown |= shutdown;` above the `if admission.draining.is_some() { return Ok(Control::Retiring); }` check (review round 1, Opus S2) | `a_control_exchange_after_the_drain_decision_is_answered_retiring` |

### Task 3: The accept loop answers control lines

**Files:**
- Modify: `src/mcp/daemon/server.rs`, `src/mcp/daemon/server_support.rs`, `src/mcp/daemon/run.rs`
- Create: `src/mcp/daemon/server_control_tests.rs`

**Interfaces:**
- Consumes: Task 1's `ControlRequest`, `DaemonStatus`, `Instance`, `is_control_line`, `display_root`; Task 2's `control_exchange`, `Control`; `Server`, `Served`, `ServeOptions`, `send`, `log::line` (S4); `server_support::{start, Running, options, read_all, send_line, served_key, gone, TTL_SECS}`, `wire_support::PING` (S4, S3a).
- Produces: `Served { build_identity: String, .. }` (`run_daemon` fills it with `PRISM_CACHE_BUILD_IDENTITY`; the test support with `"build"`) and `Served::instance(&self) -> Instance`; `Server::answer` sends a control line to `answer_control(&self, stream: &UnixStream, line: &[u8]) -> anyhow::Result<()>`; `advance_handshakes` drops a control line completed after its deadline through `control_exchange(false)` (review round 1, sol 3). Test seam used: `Server::set_line_read_hook` (S4).

- [ ] **Step 1: Write the failing tests**

In `src/mcp/daemon/server.rs`, replace this:

````rust
#[cfg(test)]
#[path = "server_drain_tests.rs"]
mod drain_tests;
````

with this:

````rust
#[cfg(test)]
#[path = "server_drain_tests.rs"]
mod drain_tests;

#[cfg(test)]
#[path = "server_control_tests.rs"]
mod control_tests;
````

Create `src/mcp/daemon/server_control_tests.rs`:

````rust
//! Tests for the control exchange in the accept loop (spec §4.7): a `STATUS` or `SHUTDOWN` line
//! is answered with the daemon's state and closed, never admitted; a probe does not keep an idle
//! daemon up; a shutdown is for one instance, never disconnects a client, and drains the daemon
//! once the last client has gone.
use super::super::control::StopReason;
use super::super::server_support::{
    gone, options, read_all, send_line, served_key, start, Running, HANDSHAKE_SECS, TTL_SECS,
};
use super::super::test_support::{advance, wait_until};
use super::super::wire_support::PING;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus, Instance};
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::preamble::{BuildState, Reply};
use std::sync::{Arc, Mutex};

fn this_instance() -> Instance {
    Instance {
        identity: served_key(),
        pid: std::process::id(),
        start_time: 1234,
    }
}

/// Send `request` on a fresh connection and return everything the daemon sends until it closes.
fn exchange(running: &Running, line: &str) -> String {
    let stream = running.connect();
    send_line(&stream, line);
    read_all(stream)
}

/// Send `request` and decode the one status line it is answered with.
fn status_after(running: &Running, request: ControlRequest) -> DaemonStatus {
    let reply = exchange(running, &request.encode());
    let line = reply
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("one whole line: {reply:?}"));
    assert!(!line.contains('\n'), "one line, then the close: {reply:?}");
    DaemonStatus::decode(line.as_bytes()).unwrap_or_else(|error| panic!("{error}: {reply:?}"))
}

/// Spec §4.7: the status names the instance (identity, pid, start time), its build and root, the
/// clients admitted and pending (the probe not counted), the build's state and attempt, the
/// generation and the idle time; then the connection is closed. Rules out a probe that is
/// admitted, and a status with the probe counted as pending.
#[test]
fn a_status_request_is_answered_with_the_daemon_s_state_and_closed() {
    let running = start(options(64), |_| {});
    let client = running.admitted();
    let status = status_after(&running, ControlRequest::Status);
    assert_eq!(
        status,
        DaemonStatus {
            instance: this_instance(),
            build_identity: "build".to_string(),
            repo_root: running.repo.display().to_string(),
            admitted: 1,
            pending: 0,
            max_clients: 64,
            build: BuildState::Idle,
            attempt: 0,
            generation: 0,
            idle_secs: None,
            idle_ttl_secs: TTL_SECS,
            shutdown: false,
        }
    );
    drop(client);
    wait_until("the client's admission to be released", || {
        running
            .state
            .admission_status()
            .is_ok_and(|counts| counts.admitted == 0 && counts.pending == 0)
    });
    running.settle();
    let idle = status_after(&running, ControlRequest::Status);
    assert_eq!((idle.admitted, idle.idle_secs), (0, Some(0)));
}

/// S4's interface for S5b: an operator who looks at an idle daemon does not keep it up. Rules out
/// a probe that leaves the handshake stage as a client does, restarting the idle time.
#[test]
fn a_status_probe_does_not_keep_an_idle_daemon_up() {
    let mut running = start(options(64), |_| {});
    running.settle();
    running.advance(TTL_SECS - 1);
    let status = status_after(&running, ControlRequest::Status);
    assert_eq!(status.idle_secs, Some(TTL_SECS - 1));
    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// A line with a control verb that this build refuses (another version of the exchange) is
/// closed without a reply, and does not restart the idle time either. Rules out a refused
/// control line that leaves as a client does.
#[test]
fn a_refused_control_line_is_closed_without_a_reply_and_leaves_the_idle_time() {
    let mut running = start(options(64), |_| {});
    running.settle();
    running.advance(TTL_SECS - 1);
    assert_eq!(exchange(&running, "STATUS {\"control\":2}\n"), "");
    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// Spec §4.7: `daemon-shutdown` asks the instance to drain when its admitted count reaches 0; a
/// daemon with no client drains at once, without waiting for the idle TTL, and leaves its key
/// free. Rules out a shutdown that is recorded and never acted on.
#[test]
fn a_shutdown_for_this_instance_drains_an_idle_daemon() {
    let mut running = start(options(64), |_| {});
    let status = status_after(&running, ControlRequest::Shutdown(this_instance()));
    assert!(status.shutdown);
    assert_eq!(running.stopped(), StopReason::Shutdown);
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
}

/// Spec §11 item 3: the shutdown is instance-bound. A request naming another identity, pid or
/// start time (a daemon that has since been replaced, say) is answered with the status and
/// records nothing. Rules out a check of any one of the three alone.
#[test]
fn a_shutdown_for_another_instance_records_nothing() {
    let running = start(options(64), |_| {});
    let other_key = IdentityKey::from_hex(&"ef".repeat(32)).expect("a key");
    let others = [
        Instance {
            identity: other_key,
            ..this_instance()
        },
        Instance {
            pid: std::process::id().wrapping_add(1),
            ..this_instance()
        },
        Instance {
            start_time: 1235,
            ..this_instance()
        },
    ];
    for other in others {
        let status = status_after(&running, ControlRequest::Shutdown(other));
        assert!(!status.shutdown, "{other:?}");
        assert_eq!(status.instance, this_instance());
    }
    running.settle();
    assert!(running.is_running());
    assert!(!status_after(&running, ControlRequest::Status).shutdown);
}

/// Spec §4.7 and §11 item 3: a shutdown never disconnects a client. The admitted client goes on
/// being served, a client that arrives after the request is admitted too (D100), and the daemon
/// drains once the last of them has gone. Rules out a drain while a client is admitted, and a
/// daemon that turns new clients away after the request.
#[test]
fn a_shutdown_lets_every_client_finish_and_drains_when_the_last_leaves() {
    let mut running = start(options(64), |_| {});
    let mut first = running.admitted();
    let status = status_after(&running, ControlRequest::Shutdown(this_instance()));
    assert_eq!((status.admitted, status.shutdown), (1, true));
    running.settle();
    assert!(running.is_running());
    assert!(first.request(PING)["result"].is_object());

    let mut second = running.admitted();
    assert!(second.request(PING)["result"].is_object());
    drop(first);
    running.settle();
    assert!(running.is_running(), "one client is left");
    drop(second);
    assert_eq!(running.stopped(), StopReason::Shutdown);
}

/// A control request is a connection like any other until its line is read: it counts against
/// `--max-clients`, and a daemon at its bound answers it `BUSY` (spec §4.2). Recorded behavior,
/// not a choice: the bound protects the daemon from connections it has not read yet.
#[test]
fn a_control_request_at_the_client_bound_is_answered_busy() {
    let running = start(options(1), |_| {});
    let _client = running.admitted();
    assert_eq!(
        exchange(&running, &ControlRequest::Status.encode()),
        Reply::Busy { max_clients: 1 }.encode()
    );
}

/// Review round 1, sol 3: a control line whose last byte arrives at its handshake deadline is
/// not answered, records no shutdown, and leaves the idle time as it found it. The probe enters
/// one second before the idle TTL ends and its line completes at its deadline, ten seconds later
/// (the loop's line seam moves the clock between the read and the deadline check, D72); the
/// daemon then retires for being idle since before the probe. Rules out an expired control line
/// that leaves as a client does (the TTL would restart: the loop would not stop) and one that
/// records its shutdown (it would stop for `Shutdown`).
#[test]
fn a_control_line_completed_after_its_deadline_moves_nothing() {
    type Step = Box<dyn FnOnce() + Send>;
    let armed: Arc<Mutex<Option<Step>>> = Arc::default();
    let slot = Arc::clone(&armed);
    let mut running = start(options(64), move |server| {
        server.set_line_read_hook(Arc::new(move || {
            let step = slot.lock().expect("armed").take();
            if let Some(step) = step {
                step();
            }
        }))
    });
    running.settle();
    running.advance(TTL_SECS - 1);
    let (clock, state) = (Arc::clone(&running.clock), Arc::clone(&running.state));
    *armed.lock().expect("armed") = Some(Box::new(move || {
        advance(&clock, &state, HANDSHAKE_SECS);
    }));
    let reply = exchange(
        &running,
        &ControlRequest::Shutdown(this_instance()).encode(),
    );
    assert_eq!(reply, "", "an expired line is not answered");
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// What the daemon does with a connection that is not a control exchange it can read: a line
/// that is neither a `HELLO` nor a control line, or a connection that ends before its line is
/// whole, is treated as a client that left, and the idle time restarts (it cannot tell them
/// from a client's). This is also what a daemon of a build before S5b does with an S5b
/// operator's `STATUS`, whose verb it does not know (review round 1, Opus W2, sol 1: accepted
/// and disclosed). Pins the behavior, so that a change to it is a decision, not an accident.
#[test]
fn a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does() {
    for (sent, then_close) in [("PING {}\n", false), ("STATUS {\"cont", true)] {
        let mut running = start(options(64), |_| {});
        running.settle();
        running.advance(TTL_SECS - 1);
        let stream = running.connect();
        send_line(&stream, sent);
        if then_close {
            wait_until("the connection to be accepted", || {
                running
                    .state
                    .admission_status()
                    .is_ok_and(|counts| counts.pending == 1)
            });
            drop(stream);
        } else {
            assert_eq!(read_all(stream), "", "{sent:?} is not answered");
        }
        wait_until("the connection to leave the handshake stage", || {
            running
                .state
                .admission_status()
                .is_ok_and(|counts| counts.pending == 0)
        });
        running.advance(1);
        assert!(running.is_running(), "{sent:?}: the idle time restarted");
        running.advance_and_wake(TTL_SECS - 1);
        assert_eq!(running.stopped(), StopReason::Idle, "{sent:?}");
    }
}
````

In `src/mcp/daemon/server_support.rs`, replace this:

````rust
        root_fs: FsIdentity::of(&repo).expect("the root's identity"),
        pid: std::process::id(),
        start_time: 1234,
    };
    let mut server = Server::new(
        Arc::clone(&state),
````

with this:

````rust
        root_fs: FsIdentity::of(&repo).expect("the root's identity"),
        pid: std::process::id(),
        start_time: 1234,
        build_identity: "build".to_string(),
    };
    let mut server = Server::new(
        Arc::clone(&state),
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t3-red.log" 2>&1
```

Expected in the log: ``error[E0560]: struct `Served` has no field named `build_identity` `` (the test support's literal; the compiler stops there) and ``could not compile `prism` (lib test) due to 1 previous error``.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/run.rs`, replace this:

````rust
        root_fs: identity.root_fs(),
        pid,
        start_time,
    };
    let options = ServeOptions {
        max_clients: options.max_clients,
````

with this:

````rust
        root_fs: identity.root_fs(),
        pid,
        start_time,
        build_identity: identity.build().cache_build_identity.clone(),
    };
    let options = ServeOptions {
        max_clients: options.max_clients,
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
//! The loop is outside both of the daemon's domains, like a connection's framing: nothing in it
//! runs under the provider slot, and what it does under the control lock it does through the
//! guarded functions of `admission.rs`. A poisoned control lock ends the daemon there.
use super::admission::{Admit, Entry};
use super::connection::serve_admitted;
use super::control::StopReason;
use super::coordinator::DaemonState;
````

with this:

````rust
//! The loop is outside both of the daemon's domains, like a connection's framing: nothing in it
//! runs under the provider slot, and what it does under the control lock it does through the
//! guarded functions of `admission.rs`. A poisoned control lock ends the daemon there.
//!
//! A connection whose line is an operator's `STATUS` or `SHUTDOWN` (spec §4.7) is answered with
//! the daemon's state and closed; it is never admitted, and it leaves the idle time as it found
//! it, also when the read of its line completes across its deadline. A connection that ends or times out
//! before its line is whole, or whose line is neither a control line nor a `HELLO`, cannot be told
//! from a client's, and restarts the idle time when it goes.
use super::admission::{Admit, Control, Entry};
use super::connection::serve_admitted;
use super::control::StopReason;
use super::coordinator::DaemonState;
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
use super::peer::peer_uid;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::ClientSettings;
use crate::mcp::shared::identity::{FsIdentity, IdentityKey};
use crate::mcp::shared::preamble::{Hello, PreambleError, Ready, Reply};
use crate::mcp::shared::preamble_reader::{PreambleLineReader, PreambleReadError};
````

with this:

````rust
use super::peer::peer_uid;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::ClientSettings;
use crate::mcp::shared::control::{
    display_root, is_control_line, ControlRequest, DaemonStatus, Instance,
};
use crate::mcp::shared::identity::{FsIdentity, IdentityKey};
use crate::mcp::shared::preamble::{Hello, PreambleError, Ready, Reply};
use crate::mcp::shared::preamble_reader::{PreambleLineReader, PreambleReadError};
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
    pub(super) pid: u32,
    /// Seconds since the Unix epoch.
    pub(super) start_time: u64,
}

/// Wakes the loop from another thread: a connection ended, a test moved the clock, or the
````

with this:

````rust
    pub(super) pid: u32,
    /// Seconds since the Unix epoch.
    pub(super) start_time: u64,
    /// `PRISM_CACHE_BUILD_IDENTITY`, as the pid file's third line has it.
    pub(super) build_identity: String,
}

impl Served {
    /// This daemon instance, as a shutdown request must name it (spec §11 item 3).
    fn instance(&self) -> Instance {
        Instance {
            identity: self.key,
            pid: self.pid,
            start_time: self.start_time,
        }
    }
}

/// Wakes the loop from another thread: a connection ended, a test moved the clock, or the
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
                    }
                    if self.state.now() < pending.deadline {
                        self.answer(pending.stream, &line)?
                    } else {
                        self.state.leave_handshake()?
                    }
````

with this:

````rust
                    }
                    if self.state.now() < pending.deadline {
                        self.answer(pending.stream, &line)?
                    } else if is_control_line(&line) {
                        // An operator's line whose read completed across its deadline is not
                        // answered, records nothing, and leaves the idle time as it found it
                        // (review round 1, sol 3). One still incomplete when the deadline is
                        // checked before a read is a client's (review round 2, Opus W1).
                        self.state.control_exchange(false)?;
                    } else {
                        self.state.leave_handshake()?
                    }
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
        Ok(())
    }

    /// Answer a whole `HELLO` line.
    fn answer(&mut self, stream: UnixStream, line: &[u8]) -> anyhow::Result<()> {
        let settings = match admit_hello(Hello::decode(line), &self.served.key) {
            Ok(settings) => settings,
            Err(Some(reply)) => {
````

with this:

````rust
        Ok(())
    }

    /// Answer a whole `HELLO` line, or an operator's control line.
    fn answer(&mut self, stream: UnixStream, line: &[u8]) -> anyhow::Result<()> {
        if is_control_line(line) {
            return self.answer_control(&stream, line);
        }
        let settings = match admit_hello(Hello::decode(line), &self.served.key) {
            Ok(settings) => settings,
            Err(Some(reply)) => {
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
                self.start_worker(stream, settings)?;
            }
        }
        Ok(())
    }

````

with this:

````rust
                self.start_worker(stream, settings)?;
            }
        }
        Ok(())
    }

    /// Answer an operator's control line (spec §4.7) and close the connection: it is never
    /// admitted, and it leaves the idle time as it found it. A shutdown is recorded only when it
    /// names this very instance (spec §11 item 3); every request is answered with the status, so
    /// the operator sees which instance answered. A line this build refuses is closed without a
    /// reply, as a refused `HELLO` is (D54).
    fn answer_control(&self, stream: &UnixStream, line: &[u8]) -> anyhow::Result<()> {
        let request = match ControlRequest::decode(line) {
            Ok(request) => request,
            Err(error) => {
                log::line(&format!("refused a control line: {error}"));
                self.state.control_exchange(false)?;
                return Ok(());
            }
        };
        let shutdown = match request {
            ControlRequest::Status => false,
            ControlRequest::Shutdown(instance) => instance == self.served.instance(),
        };
        let snapshot = match self.state.control_exchange(shutdown)? {
            Control::Retiring => {
                send(stream, &Reply::Retiring);
                return Ok(());
            }
            Control::Status(snapshot) => snapshot,
        };
        if shutdown {
            log::line(
                "an operator asked this daemon to shut down; it drains once no client is left",
            );
        }
        let status = DaemonStatus {
            instance: self.served.instance(),
            build_identity: self.served.build_identity.clone(),
            repo_root: display_root(&self.served.repo_root),
            admitted: snapshot.admitted,
            pending: snapshot.pending,
            max_clients: self.options.max_clients,
            build: snapshot.build.state,
            attempt: snapshot.build.attempt,
            generation: snapshot.build.generation,
            idle_secs: snapshot.idle_for.map(|idle| idle.as_secs()),
            idle_ttl_secs: self.options.idle_ttl.as_secs(),
            shutdown: snapshot.shutdown,
        };
        let _ = (&*stream).write_all(status.encode().as_bytes());
        Ok(())
    }

````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t3.log" 2>&1
```

Expected: `test result: ok. 596 passed; 0 failed` (587 + 9), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/server.rs src/mcp/daemon/server_support.rs src/mcp/daemon/run.rs src/mcp/daemon/server_control_tests.rs
git commit -m "feat(mcp): the daemon answers STATUS and SHUTDOWN in place of HELLO"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::daemon::server::`, see the named test fail, then `git checkout -- src/mcp/daemon/server.rs`.

| Mutant | Edit | Fails |
|---|---|---|
| 3A | in `answer_control`, `instance == self.served.instance()` → `instance.pid == self.served.pid` | `a_shutdown_for_another_instance_records_nothing` |
| 3B | in `answer_control`, a refused line: `self.state.control_exchange(false)?;` → `self.state.leave_handshake()?;` | `a_refused_control_line_is_closed_without_a_reply_and_leaves_the_idle_time` |
| 3C | in `answer`, `if is_control_line(line) {` → `if false {` | `a_status_request_is_answered_with_the_daemon_s_state_and_closed` and four more of the file |
| 3D | in `answer_control`, `idle_secs: snapshot.idle_for.map(|idle| idle.as_secs()),` → `idle_secs: None,` | `a_status_request_is_answered_with_the_daemon_s_state_and_closed`, `a_status_probe_does_not_keep_an_idle_daemon_up` |
| R1 | in `advance_handshakes`, `} else if is_control_line(&line) {` → `} else if false {` (an expired control line leaves as a client does; review round 1, sol 3) | `a_control_line_completed_after_its_deadline_moves_nothing`, at its 30 s wait for the drain |

### Task 4: S5b-1 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
around the builder by `run.rs::marked`, removed at release); a daemon that dies during its build
leaves it. The marker shares a fence with `DaemonFiles`: release (or drop) sets it under its lock
before the owner lock is released, so a released daemon's late `set` or `clear` does nothing.

Shim (`src/mcp/shim/`, Unix only; `prism-mcp --repo X --shared [--daemon-start-timeout S]
[--idle-ttl S] [--max-clients N]`; `--shared` with `--no-cache`, `--eager` or `--owner-*` is a CLI
````

with this:

````markdown
around the builder by `run.rs::marked`, removed at release); a daemon that dies during its build
leaves it. The marker shares a fence with `DaemonFiles`: release (or drop) sets it under its lock
before the owner lock is released, so a released daemon's late `set` or `clear` does nothing.
An operator's control line (`shared/control.rs`; spec §4.7) is answered in place of a `HELLO`:
`STATUS {"control":1}`, or `SHUTDOWN` naming one instance by identity, pid and start time (the
exchange has a version of its own, not in the identity key, so an operator tool can ask a daemon
of another build). The daemon answers one `STATUS` line (instance, root, build identity, clients,
build state, idle time, whether a shutdown is requested), or `RETIRING`/`BUSY`, and closes; the
connection is never admitted, and `admission.rs::control_exchange` takes it out of the pending
count without restarting the idle time (`quiet_since`), so a probe does not keep an idle daemon
up; a control line whose read completes across its handshake deadline is dropped the same way, unanswered
and with nothing recorded. A connection that ends before its line is whole, or whose line is
neither a control line nor a `HELLO` (which is what a daemon built before this exchange makes of a
`STATUS`), is treated as a client's and restarts the idle time. A `SHUTDOWN` for this very
instance makes the daemon drain once nothing is admitted or pending, whatever the idle TTL or a
running build (`StopReason::Shutdown`); until then it goes on admitting new clients, and it never
disconnects one.

Shim (`src/mcp/shim/`, Unix only; `prism-mcp --repo X --shared [--daemon-start-timeout S]
[--idle-ttl S] [--max-clients N]`; `--shared` with `--no-cache`, `--eager` or `--owner-*` is a CLI
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check > "$LOG_DIR/s51-fmt.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/s51-lib.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/s51-mcp.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check > "$LOG_DIR/s51-check.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check --features mcp > "$LOG_DIR/s51-checkmcp.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets --features mcp > "$LOG_DIR/s51-clippy.log" 2>&1
grep -E '^\s+--> src/mcp/(shared|daemon)/' "$LOG_DIR/s51-clippy.log"
```

Expected: `cargo fmt --check` silent; `test result: ok. 596 passed; 0 failed` (571 + 25); `46 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; both checks `Finished`, no `warning` in either; clippy 258 `warning` lines, and the `grep` prints nothing.

- [ ] **Step 3: State the invariants**

```bash
git diff --name-only origin/main -- src/mcp/transport.rs src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs src/mcp/session.rs src/mcp/lazy.rs src/mcp/framing.rs src/mcp/output.rs src/mcp/settings.rs src/mcp/shim tests/fixtures/mcp/golden src/bin
git diff --name-only origin/main | wc -l
python3 scripts/file_size.py $(git diff --name-only origin/main -- '*.rs')
```

Expected: the first prints nothing; `14` (13 under `src/` and `CLAUDE.md`); every size at most 546 (`src/mcp/daemon/server.rs`).

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
fails=0
for i in $(seq 1 10); do nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/s51-rep-$i.log" 2>&1 || fails=$((fails+1)); done
echo "failed runs: $fails"
grep -h '^test result' "$LOG_DIR"/s51-rep-*.log | sed 's/finished.*//' | sort | uniq -c
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s51-full.log" 2>&1; echo "full suite rc=$?"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s51-doc.log" 2>&1; echo "doc tests rc=$?"
```

The cargo commands are written out in full, not through a variable: zsh, the default shell on macOS, does not split an unquoted variable into words, so `$J cargo …` runs a command named after the whole string and fails with status 127 (found in validation).

Then on Linux, in the bridge's toolchain image with the worktree read-only. Check the disk first, in its own command, and keep at least 50 GiB free; the volumes are removed by the `trap` however the run ends. Each run's whole output is a log of its own, and the script's exit status is the number of failed runs.

```bash
df -h /System/Volumes/Data
```

```bash
mkdir -p "$LOG_DIR/linux"
cat > "$LOG_DIR/linux.sh" <<'EOF'
#!/bin/bash
fails=0
for i in $(seq 1 10); do
  for suite in "--lib mcp::shim" "--lib mcp::daemon" "--lib mcp::shared" ${EXTRA_SUITE:+"$EXTRA_SUITE"}; do
    name=$(printf '%s' "$suite" | tr -c 'a-z0-9' '_')
    nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp $suite > "/logs/$name-$i.log" 2>&1 || fails=$((fails+1))
  done
done
for p in /proc/[0-9]*; do grep -qs 'prism-mc[p]' "$p/cmdline" && echo "left: $(tr '\0' ' ' < "$p/cmdline")"; done
exit $fails
EOF
(
  trap 'docker volume rm s5b-target s5b-registry >/dev/null' EXIT
  docker run --rm --cpus 4 --memory 8g -e CARGO_BUILD_JOBS=4 -e CARGO_TARGET_DIR=/target \
    -e EXTRA_SUITE="$EXTRA_SUITE" -v "$PWD":/src:ro -v s5b-target:/target \
    -v s5b-registry:/usr/local/cargo/registry -v "$LOG_DIR/linux.sh":/linux.sh:ro \
    -v "$LOG_DIR/linux":/logs -w /src --entrypoint bash a2a-toolchain:latest /linux.sh
) > "$LOG_DIR/s51-linux.out" 2>&1; echo "failed Linux runs: $?"
grep -h '^test result' "$LOG_DIR"/linux/*.log | sed 's/finished.*//' | sort | uniq -c
```

Expected: `failed runs: 0`, `10 test result: ok. 150 passed; 0 failed`; the full suite and the doc tests `rc=0`; on Linux (in validation this script ran at the S5b-2 head, whose code contains this head's, V11; the counts here are this head's) `failed Linux runs: 10` and every failure the one root-only S2a test (`--lib mcp::shared`: `10 test result: FAILED. 135 passed; 1 failed`, `mcp::shared::dir_chain::tests::another_users_private_directory_is_refused_for_what_it_holds`, which fails on the base in the same container; V11), `10 test result: ok. 47 passed` (`--lib mcp::shim`) and `10 test result: ok. 150 passed` (`--lib mcp::daemon`), and no `left:` line.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S5b-1 is the daemon's half of the operator's control: what a daemon answers to `STATUS` and `SHUTDOWN`, that looking does not keep it up, and when a shutdown drains it. (2) **Invariants:** Step 3. (3) **Decisions:** D97–D100, D105, D108; SI-29, SI-30, SI-32, SI-34; owner decision 2 (2026-10-16). D97 (no protocol bump) and D99 (`quiet_since`) are the ones a reviewer is most likely to question. (4) **Claims:** the rows for Tasks 1–3, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base (Linux included), and mutants 1A–3D, R1 and R2 as observed. (6) **Not verified:** x86_64 Linux before CI; an operator tool (S5b-2). (7) **Tier-A:** not run (no file it watches is touched). (8) **Reviews of the plan:** review round 1's folds for Tasks 1–3 (sol 3, Opus S2, the pin for Opus W2/sol 1), and round 2's wording folded after the two-round cap without a third round (disclosed).

```bash
git add CLAUDE.md
git commit -m "docs(mcp): the daemon's control exchange"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): the daemon answers status and shutdown requests (shared daemon S5b-1)" --body-file "$LOG_DIR/s51-pr-body.md"
```

Review: Opus 5.5 and gpt-6.1-sol, same brief, same commit; two rounds (lane README).

---

# Part two: S5b-2, the operator's commands (Tasks 5–8)

### Task 5: One control exchange, and `daemon-status`

**Files:**
- Modify: `src/mcp/shared/preamble.rs`, `src/mcp/shared/paths.rs`, `src/mcp/shared/paths_tests.rs`, `src/mcp/shim/connect.rs`, `src/mcp/shim/diagnostic.rs`, `src/mcp/shim/key_files.rs`, `src/mcp/shim/arbitrate_tests.rs`, `src/mcp/shim/mod.rs`, `src/mcp/mod.rs`
- Create: `src/mcp/shim/control.rs`, `src/mcp/shim/control_tests.rs`, `src/mcp/shim/control_support.rs`, `src/mcp/shim/status.rs`, `src/mcp/shim/status_tests.rs`

**Interfaces:**
- Consumes: Task 1's wire types; `connect::{PeerUid, Reached, Refusal, Transient}`, `socket::connect_within`, `KeyDir` (`open`, `start_lock`, `try_own`, `build_marker`), `diagnostic::one_line`, `arbitrate::BUILD_CRASH_BACKOFF` (S5a); `connect::read_reply` and `left`, and `diagnostic::pid_file`, widened from private to `pub(super)`; `MarkerContents` (S5a); `peer::peer_uid` (S4); `runtime_dir`, `runtime_dir_from`, `validate_runtime_dir`, `current_uid`, `DaemonIdentity`, `canonical_config` (S2a); `shim_support::{runtime, key, this_user, another_user, open_lock, read_line, unix_now, ready_line, HANG}` (S5a).
- Produces: `DaemonPaths::from_locator(runtime_dir: &Path, locator: &str) -> Result<Self, SocketPathTooLong>` (`new` calls it); `BuildState::word` `pub`. In `shim::key_files` (S5a's, changed: review round 1, D109): `enum Marker { Absent, Unusable(String), Read(MarkerContents) }`, `KeyDir::read_build_marker(&self) -> Marker` (non-blocking, regular file only, at most 256 bytes), and `build_marker` now returns `None` for anything but a readable regular file. In `shim::control` (`pub(super)`): `enum Exchanged { Status(DaemonStatus), Retiring, Busy { max_clients: u32 }, NotListening(io::Error), NoReply(Transient), Refused(Refusal) }`; `fn exchange(socket: &Path, request: &ControlRequest, deadline: Instant, peer_uid: PeerUid, uid: u32) -> Exchanged`. In `shim::status`: `pub struct StatusOptions { config: Option<ServerConfig>, runtime_base: Option<PathBuf>, timeout: Duration }`, `pub fn run_daemon_status(&StatusOptions) -> i32`, `pub const DEFAULT_CONTROL_TIMEOUT_SECS: u64 = 10`; `pub(super) fn status_report(&StatusOptions, PeerUid) -> Result<String, String>`, `operator_runtime_dir`, `key_of`, `unix_now`, `died_building`, `has_daemon_files(&DaemonPaths) -> bool`, `enum Holder<T> { Starting, Alive, Free(T) }` and `under_locks<T>(&DaemonPaths, impl FnOnce(&KeyDir<'_>) -> T) -> Result<Holder<T>, String>` (the look at the locks both commands share). Re-exported from `crate::mcp`: `run_daemon_status`, `StatusOptions`, `DEFAULT_CONTROL_TIMEOUT_SECS`. Test support (`control_support`): `ControlFake` (`start(paths, owned, script)`, `lines`, `retire`), `FakeReply { Line, Close, Silent }`, `ControlScript`, `always_line`, `fake_status(admitted, shutdown)`, `fake_instance`, `FAKE_PID`, `FAKE_START`, `mkfifo(path)`, `within_hang(what, work)` (an independent watchdog).

- [ ] **Step 1: Write the failing tests**

In `src/mcp/shared/paths_tests.rs`, replace this:

````rust
    assert!(text.contains("PRISM_MCP_RUNTIME_DIR"), "{text}");
    assert!(!text.contains('\n'), "one line: {text}");
}
````

with this:

````rust
    assert!(text.contains("PRISM_MCP_RUNTIME_DIR"), "{text}");
    assert!(!text.contains('\n'), "one line: {text}");
}

/// `daemon-status` knows a key only by the 16 digits of its file names (S5b): the paths of a
/// locator are the paths of every key that starts with it, the socket path limit included.
#[test]
fn a_locator_names_the_same_files_as_its_key() {
    let dir = Path::new("/run/user/501/prism-mcpd");
    assert_eq!(
        DaemonPaths::from_locator(dir, "0123456789abcdef").unwrap(),
        DaemonPaths::new(dir, &key(KEY)).unwrap()
    );
    let long = dir_of_len(MAX_SOCKET_PATH_BYTES - SOCKET_NAME_BYTES + 1);
    assert!(DaemonPaths::from_locator(&long, "0123456789abcdef").is_err());
}
````

In `src/mcp/shim/arbitrate_tests.rs`, replace this:

````rust
        "{without_spawn}"
    );
}
````

with this:

````rust
        "{without_spawn}"
    );
}

/// S5b review round 1 (sol 2, Opus S6): a build marker that is not a regular file (a FIFO here)
/// is not a daemon's. Reading it does not wait for a writer, it does not hold the key, and the
/// shim removes it with the other stale files and starts a daemon. Watched by an independent
/// thread, because a blocking open of the FIFO waited for a writer for ever.
#[test]
fn a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key() {
    let (base, paths) = runtime();
    super::super::control_support::mkfifo(paths.build_marker());
    let marker = paths.build_marker().to_path_buf();
    // The runtime directory comes back from the worker and lives through the last assertion, so
    // that it shows the shim's removal, not the fixture's cleanup (review round 2, sol).
    let (pid, _base) =
        super::super::control_support::within_hang("a shim over a FIFO marker", move || {
            let hello = hello_line(key());
            let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
            let pid = pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter));
            (pid, base)
        });
    assert_eq!(pid, 1000);
    assert!(
        std::fs::symlink_metadata(&marker).is_err(),
        "the FIFO was removed"
    );
}
````

Create `src/mcp/shim/control_support.rs`:

````rust
//! A stand-in for a daemon's answers to the operator's control exchange (S5b): it owns the key's
//! owner lock (or not), records every line it reads, and answers each by a script of the test's.
use super::shim_support::{key, open_lock, read_line, HANG};
use crate::mcp::shared::control::{DaemonStatus, Instance};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::BuildState;
use std::fs::{File, TryLockError};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// What the fake does with a connection after reading its line.
pub(super) enum FakeReply {
    /// Write this line (newline included), then close.
    Line(String),
    /// Close without a reply, as a daemon that refuses the line does.
    Close,
    /// Never answer; hold the connection until the peer closes it or the fake stops.
    Silent,
}

pub(super) type ControlScript = Arc<dyn Fn(&str) -> FakeReply + Send + Sync>;

/// The pid the fake reports.
pub(super) const FAKE_PID: u32 = 4242;
/// The start time the fake reports.
pub(super) const FAKE_START: u64 = 1_760_600_000;

/// The instance the fake is.
pub(super) fn fake_instance() -> Instance {
    Instance {
        identity: key(),
        pid: FAKE_PID,
        start_time: FAKE_START,
    }
}

/// A status of the fake's, with `admitted` clients and the shutdown flag as given.
pub(super) fn fake_status(admitted: u32, shutdown: bool) -> DaemonStatus {
    DaemonStatus {
        instance: fake_instance(),
        build_identity: "b".repeat(64),
        repo_root: "/work/repo".to_string(),
        admitted,
        pending: 0,
        max_clients: 64,
        build: BuildState::Ready,
        attempt: 1,
        generation: 2,
        idle_secs: (admitted == 0).then_some(7),
        idle_ttl_secs: 900,
        shutdown,
    }
}

/// A daemon on a thread of the test that answers control lines by its script.
pub(super) struct ControlFake {
    owner: Option<File>,
    lines: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    socket: PathBuf,
}

impl ControlFake {
    /// Bind the key's socket and answer; with `owned`, hold the key's owner lock as a daemon
    /// process does.
    pub(super) fn start(paths: &DaemonPaths, owned: bool, script: ControlScript) -> Self {
        let owner = owned.then(|| {
            let owner = open_lock(paths.owner_lock());
            match owner.try_lock() {
                Ok(()) => owner,
                Err(TryLockError::WouldBlock) => panic!("the owner lock is held"),
                Err(TryLockError::Error(error)) => panic!("lock: {error}"),
            }
        });
        let listener = UnixListener::bind(paths.socket()).expect("bind the fake daemon");
        listener.set_nonblocking(true).expect("nonblocking");
        let lines: Arc<Mutex<Vec<String>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (lines, stop) = (Arc::clone(&lines), Arc::clone(&stop));
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => answer(stream, &script, &lines, &stop),
                        Err(_) => std::thread::sleep(Duration::from_millis(2)),
                    }
                }
            })
        };
        Self {
            owner,
            lines,
            stop,
            thread: Some(thread),
            socket: paths.socket().to_path_buf(),
        }
    }

    /// Every line read so far, newlines included.
    pub(super) fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("lines").clone()
    }

    /// Stop answering, remove the socket and release the owner lock, as a drained daemon does.
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

impl Drop for ControlFake {
    fn drop(&mut self) {
        self.stop_thread();
    }
}

fn answer(
    mut stream: UnixStream,
    script: &ControlScript,
    lines: &Mutex<Vec<String>>,
    stop: &AtomicBool,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(HANG));
    let line = read_line(&stream);
    lines.lock().expect("lines").push(line.clone());
    match script(&line) {
        FakeReply::Line(reply) => {
            let _ = stream.write_all(reply.as_bytes());
        }
        FakeReply::Close => {}
        FakeReply::Silent => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
            let mut buf = [0u8; 64];
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
    }
}

/// A script that answers every line with `reply`.
pub(super) fn always_line(reply: String) -> ControlScript {
    Arc::new(move |_| FakeReply::Line(reply.clone()))
}

/// Make a FIFO at `path`, as a process of this user could plant one in place of a key's file.
pub(super) fn mkfifo(path: &std::path::Path) {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).expect("a path without NUL");
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
    let made = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
    assert_eq!(made, 0, "mkfifo: {}", std::io::Error::last_os_error());
}

/// Run `work` on a thread of its own and return its result, or fail the test if it has not
/// returned within [`HANG`]: an independent watchdog for code that must not wait on anything.
pub(super) fn within_hang<T: Send + 'static>(
    what: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (sent, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(work());
    });
    received
        .recv_timeout(HANG)
        .unwrap_or_else(|_| panic!("{what} did not return within {HANG:?}"))
}
````

Create `src/mcp/shim/control_tests.rs`:

````rust
//! Tests for one control exchange (S5b; spec §4.7): what is sent, how each answer is sorted, and
//! that nothing is sent to a peer of another user.
use super::super::control_support::{always_line, fake_status, ControlFake, FakeReply};
use super::super::shim_support::{another_user, runtime, this_user, HANG};
use super::*;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::shared::runtime_dir::current_uid;
use std::sync::Arc;
use std::time::Duration;

fn ask(socket: &Path, request: &ControlRequest, within: Duration) -> Exchanged {
    exchange(
        socket,
        request,
        Instant::now() + within,
        this_user,
        current_uid(),
    )
}

/// The request goes out as `encode` spells it and a status reply comes back decoded.
#[test]
fn a_status_reply_is_read_and_the_request_sent_as_encoded() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    match ask(paths.socket(), &ControlRequest::Status, HANG) {
        Exchanged::Status(status) => assert_eq!(status, fake_status(1, false)),
        other => panic!("{other:?}"),
    }
    assert_eq!(fake.lines(), vec![ControlRequest::Status.encode()]);
}

/// `RETIRING` and `BUSY` are answers of their own, not failures to answer.
#[test]
fn retiring_and_busy_are_answers_of_their_own() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(Reply::Retiring.encode()));
    assert!(matches!(
        ask(paths.socket(), &ControlRequest::Status, HANG),
        Exchanged::Retiring
    ));
    fake.retire();
    let _fake = ControlFake::start(
        &paths,
        true,
        always_line(Reply::Busy { max_clients: 3 }.encode()),
    );
    assert!(matches!(
        ask(paths.socket(), &ControlRequest::Status, HANG),
        Exchanged::Busy { max_clients: 3 }
    ));
}

/// The peer check comes before anything is written (D78, as the shim does it): a listener of
/// another user at this user's socket path learns nothing. Rules out a check made after the
/// request is sent.
#[test]
fn a_peer_of_another_user_is_refused_before_anything_is_sent() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(fake_status(0, false).encode()));
    let answered = exchange(
        paths.socket(),
        &ControlRequest::Status,
        Instant::now() + HANG,
        another_user,
        current_uid(),
    );
    assert!(
        matches!(answered, Exchanged::Refused(Refusal::ForeignPeer { .. })),
        "{answered:?}"
    );
    super::super::shim_support::wait_until("the fake to see the connection end", || {
        !fake.lines().is_empty()
    });
    assert_eq!(fake.lines(), vec![String::new()], "nothing was sent");
}

/// A daemon that closes without a reply (one that does not know the control exchange closes the
/// line it refuses, S4's D54) and one that never answers are both no reply, the second at the
/// deadline; nothing listening is told apart.
#[test]
fn a_close_a_silence_and_nothing_listening_are_told_apart() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, Arc::new(|_| FakeReply::Close));
    assert!(matches!(
        ask(paths.socket(), &ControlRequest::Status, HANG),
        Exchanged::NoReply(Transient::Closed)
    ));
    fake.retire();
    let _silent = ControlFake::start(&paths, true, Arc::new(|_| FakeReply::Silent));
    assert!(matches!(
        ask(
            paths.socket(),
            &ControlRequest::Status,
            Duration::from_millis(300)
        ),
        Exchanged::NoReply(Transient::NoReply)
    ));
    let (_other_base, absent) = runtime();
    assert!(matches!(
        ask(absent.socket(), &ControlRequest::Status, HANG),
        Exchanged::NotListening(_)
    ));
}

/// A reply this build cannot read, or a preamble reply that does not answer a control request,
/// is refused. Rules out reading a `READY` as a status.
#[test]
fn an_unreadable_or_unexpected_reply_is_refused() {
    let (_base, paths) = runtime();
    for reply in [
        "STATUS {\"control\":1}\n".to_string(),
        "garbage\n".to_string(),
        super::super::shim_support::ready_line(super::super::shim_support::key(), 1),
    ] {
        let fake = ControlFake::start(&paths, false, always_line(reply.clone()));
        let answered = ask(paths.socket(), &ControlRequest::Status, HANG);
        assert!(
            matches!(answered, Exchanged::Refused(Refusal::BadReply(_))),
            "{reply:?}: {answered:?}"
        );
        fake.retire();
    }
}
````

In `src/mcp/shim/mod.rs`, replace this:

````rust

mod arbitrate;
mod connect;
mod diagnostic;
mod key_files;
mod pump;
````

with this:

````rust

mod arbitrate;
mod connect;
mod control;
#[cfg(test)]
mod control_support;
mod diagnostic;
mod key_files;
mod pump;
````

In `src/mcp/shim/mod.rs`, replace this:

````rust
mod shim_support;
mod socket;
mod spawn;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
````

with this:

````rust
mod shim_support;
mod socket;
mod spawn;
mod status;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
````

Create `src/mcp/shim/status_tests.rs`:

````rust
//! Tests for `daemon-status` against stand-in daemons and planted files (S5b; spec §4.7): what
//! each kind of key is reported as, that the report reads and never removes, and that a key a
//! shim is starting is not looked at under its owner lock.
use super::super::control_support::{
    always_line, fake_status, mkfifo, within_hang, ControlFake, FakeReply,
};
use super::super::shim_support::{
    full_backlog, key, open_lock, runtime, this_user, unix_now, HANG,
};
use super::*;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::CacheMode;
use std::sync::Arc;

const LOCATOR: &str = "cdcdcdcdcdcdcdcd";

fn options(base: &Path, timeout: Duration) -> StatusOptions {
    StatusOptions {
        config: None,
        runtime_base: Some(base.to_path_buf()),
        timeout,
    }
}

fn report(base: &Path) -> String {
    status_report(&options(base, HANG), this_user).expect("a report")
}

/// What the report says about the test's key: its lines after the first.
fn about_the_key(base: &Path, paths: &DaemonPaths) -> String {
    let report = report(base);
    let head = format!("runtime dir {}\n", paths.runtime_dir().display());
    report
        .strip_prefix(&head)
        .unwrap_or_else(|| panic!("starts with the runtime dir: {report}"))
        .to_string()
}

fn write_pid_file(paths: &DaemonPaths) {
    std::fs::write(paths.pid_file(), "4242\n1760600000\nbuild\n").expect("write the pid file");
}

/// A socket file nobody listens on, as a daemon that died leaves it.
fn stale_socket(paths: &DaemonPaths) {
    drop(std::os::unix::net::UnixListener::bind(paths.socket()).expect("bind"));
}

/// Spec §4.7: a daemon that answers is listed with its instance, root, build, clients, build state
/// and idle time.
#[test]
fn a_daemon_that_answers_is_listed_with_its_instance_and_state() {
    let (base, paths) = runtime();
    let _fake = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    assert_eq!(
        about_the_key(base.path(), &paths),
        format!(
            "{LOCATOR} pid 4242, started 1760600000, serving /work/repo\n  key {}; build {}\n  \
             clients: 1 admitted, 0 pending, at most 64; in use\n  index: ready (attempt 1, \
             generation 2)\n",
            key(),
            "b".repeat(64)
        )
    );
}

/// An idle daemon shows how long it has been idle and when it retires, and a requested
/// shutdown is shown.
#[test]
fn an_idle_daemon_shows_its_idle_time_and_a_requested_shutdown() {
    let (base, paths) = runtime();
    let _fake = ControlFake::start(&paths, true, always_line(fake_status(0, true).encode()));
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.contains("; idle 7 s; retires after 900 s idle\n"),
        "{said}"
    );
    assert!(
        said.ends_with("  shutdown requested: it drains once no client is left\n"),
        "{said}"
    );
}

/// `RETIRING` and `BUSY` are reported as what they are.
#[test]
fn a_draining_or_busy_daemon_is_named_as_such() {
    let (base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(Reply::Retiring.encode()));
    assert_eq!(
        about_the_key(base.path(), &paths),
        format!("{LOCATOR} draining: it answered RETIRING\n")
    );
    fake.retire();
    let _fake = ControlFake::start(
        &paths,
        true,
        always_line(Reply::Busy { max_clients: 2 }.encode()),
    );
    assert_eq!(
        about_the_key(base.path(), &paths),
        format!("{LOCATOR} busy: it answered BUSY at its bound of 2 clients, so no status\n")
    );
}

/// A daemon that holds its owner lock and does not answer (a wedged daemon, or one older than
/// the control exchange) is named alive, with what its pid file says.
#[test]
fn a_daemon_that_does_not_answer_is_named_alive_with_its_pid_file() {
    let (base, paths) = runtime();
    write_pid_file(&paths);
    let _fake = ControlFake::start(&paths, true, Arc::new(|_| FakeReply::Silent));
    let said = status_report(&options(base.path(), Duration::from_millis(300)), this_user)
        .expect("a report");
    assert!(
        said.contains(&format!(
            "{LOCATOR} alive but not answering (the daemon accepted but sent no reply); pid file \
             {}: pid 4242, start time 1760600000, build build\n",
            paths.pid_file().display()
        )),
        "{said}"
    );
}

/// A key no daemon owns is reported with what a dead daemon left, and nothing is removed: the
/// next shim of the key removes it under the owner lock (owner question 1). Rules out a status
/// that cleans up.
#[test]
fn a_dead_daemon_s_leftovers_are_named_and_left_in_place() {
    let (base, paths) = runtime();
    stale_socket(&paths);
    write_pid_file(&paths);
    let said = about_the_key(base.path(), &paths);
    assert_eq!(
        said,
        format!(
            "{LOCATOR} no daemon; a dead daemon left its socket and its pid file ({}: \
             pid 4242, start time 1760600000, build build)\n",
            paths.pid_file().display()
        )
    );
    assert!(std::fs::symlink_metadata(paths.socket()).is_ok());
    assert!(paths.pid_file().exists());
    let owner = open_lock(paths.owner_lock());
    assert!(owner.try_lock().is_ok(), "the owner lock was released");
}

/// S5a's interface for S5b: a key whose daemon died during its build says when a shim may start
/// it again (60 s from the first sighting), and the report records no sighting: only a shim
/// does. Rules out a status that starts the minute itself.
#[test]
fn a_daemon_that_died_during_its_build_says_when_it_may_restart() {
    let (base, paths) = runtime();
    let unseen = "4242\n1760000000\n";
    std::fs::write(paths.build_marker(), unseen).expect("plant a marker");
    let said = about_the_key(base.path(), &paths);
    assert_eq!(
        said,
        format!(
            "{LOCATOR} no daemon; its last daemon (pid 4242) died during its index build (started \
             1760000000); no shim has seen it yet: the next one records the sighting and waits \
             60 s\n"
        )
    );
    assert_eq!(
        std::fs::read_to_string(paths.build_marker()).expect("the marker"),
        unseen,
        "the report recorded nothing"
    );
    let seen = unix_now() - 21;
    std::fs::write(paths.build_marker(), format!("4242\n1760000000\n{seen}\n")).expect("marker");
    assert!(about_the_key(base.path(), &paths).contains(&format!(
        "first seen by a shim at {seen}; a shim restarts it in 39 s\n"
    )));
}

/// D102: while a shim holds a key's start lock (it may have spawned a daemon that has not taken
/// the owner lock yet), the report does not look at the owner lock, which would make that daemon
/// believe another owns the key. Rules out a look at the owner lock without the start lock.
#[test]
fn a_key_a_shim_is_starting_is_not_looked_at_under_its_owner_lock() {
    let (base, paths) = runtime();
    write_pid_file(&paths);
    let start = open_lock(paths.start_lock());
    start.try_lock().expect("the start lock");
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} being started or reached: a shim holds its start lock; "
        )),
        "{said}"
    );
}

/// Keys that have nothing of a daemon's left (every rebuild of prism leaves lock files and a log
/// per repository, S4's D70) are counted in one line with the size of their logs; files that are
/// not prism's are ignored; nothing is removed.
#[test]
fn keys_with_only_lock_files_and_logs_are_counted_not_listed() {
    let (base, paths) = runtime();
    let dir = paths.runtime_dir();
    for name in [
        "1111111111111111.start",
        "1111111111111111.owner",
        "2222222222222222.owner",
    ] {
        std::fs::write(dir.join(name), "").expect("a lock file");
    }
    std::fs::write(dir.join("1111111111111111.log"), "0123456789").expect("a log");
    for name in [
        "README",
        "123.sock",
        "ABCDEFABCDEFABCD.sock",
        "3333333333333333.tmp",
    ] {
        std::fs::write(dir.join(name), "x").expect("a stray file");
    }
    assert_eq!(
        about_the_key(base.path(), &paths),
        "2 keys have nothing but lock files and logs (10 bytes of logs)\n"
    );
    assert_eq!(
        std::fs::read_dir(dir).expect("list").count(),
        8,
        "nothing removed"
    );
}

/// `--repo` shows that configuration's daemon only, and for a configuration with no daemon says
/// so without creating any of its files.
#[test]
fn with_a_repository_only_its_daemon_is_shown() {
    let (base, paths) = runtime();
    let _other = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    let repo = tempfile::tempdir().expect("a repository");
    let cache = tempfile::tempdir().expect("a cache base");
    let mut config = ServerConfig::new(repo.path().to_path_buf());
    config.cache = CacheMode::Dir(cache.path().to_path_buf());
    let wanted = key_of(&config).expect("a key");
    let mut options = options(base.path(), HANG);
    options.config = Some(config);
    let said = status_report(&options, this_user).expect("a report");
    assert_eq!(
        said,
        format!(
            "runtime dir {}\n{} no daemon for this configuration\n",
            paths.runtime_dir().display(),
            wanted.locator()
        )
    );
    let mine = DaemonPaths::new(paths.runtime_dir(), &wanted).expect("paths");
    for path in [mine.start_lock(), mine.owner_lock()] {
        assert!(
            std::fs::symlink_metadata(path).is_err(),
            "{}",
            path.display()
        );
    }
}

/// A runtime directory that does not exist is reported and not created; one that fails
/// validation is an error, and nothing in it is read.
#[test]
fn a_missing_or_invalid_runtime_directory() {
    let base = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base");
    assert!(report(base.path()).ends_with("no daemon (the runtime directory does not exist)\n"));
    assert!(std::fs::symlink_metadata(base.path().join("prism-mcpd")).is_err());

    use std::os::unix::fs::PermissionsExt;
    let dir = base.path().join("prism-mcpd");
    std::fs::create_dir(&dir).expect("create");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let error = status_report(&options(base.path(), HANG), this_user).expect_err("invalid");
    assert!(error.contains("prism-mcpd"), "{error}");
}

/// An empty runtime directory has no daemon.
#[test]
fn an_empty_runtime_directory_has_no_daemon() {
    let (base, paths) = runtime();
    assert_eq!(about_the_key(base.path(), &paths), "no daemon\n");
}

/// Nothing the report prints carries a control character, whatever a daemon reports: a root may
/// hold any byte. Rules out printing what a daemon sent as it came.
#[test]
fn a_root_with_control_characters_is_printed_on_one_line() {
    let (base, paths) = runtime();
    let mut status = fake_status(0, false);
    status.repo_root = "/w/a\nb\u{1b}[31m".to_string();
    let _fake = ControlFake::start(&paths, true, always_line(status.encode()));
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} pid 4242, started 1760600000, serving /w/a\\nb\\u{{1b}}[31m\n"
        )),
        "{said:?}"
    );
    assert!(!said.contains('\u{1b}'), "{said:?}");
}

/// Review round 1, sol 2 and Opus S6: a build marker is read without waiting and only so far. A
/// FIFO in its place (a blocking open would wait for a writer for ever, holding the key's start
/// and owner locks) is reported as unusable at once; a marker of 4 MiB is read only as far as a
/// daemon's marker reaches. Watched by an independent thread, because the failure is a hang.
#[test]
fn a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report() {
    let (base, paths) = runtime();
    mkfifo(paths.build_marker());
    let root = base.path().to_path_buf();
    let said = within_hang("daemon-status over a FIFO marker", move || report(&root));
    assert!(
        said.contains(&format!(
            "{LOCATOR} no daemon; its build marker is not usable ("
        )),
        "{said}"
    );
    let owner = open_lock(paths.owner_lock());
    assert!(owner.try_lock().is_ok(), "the owner lock was released");
    drop(owner);

    std::fs::remove_file(paths.build_marker()).expect("remove the FIFO");
    // Two whole lines, then a third line that ends 4 MiB in: read whole, it would be a sighting
    // at time 0; only its first 256 bytes are read, so it is no sighting.
    let mut big = b"4242\n1760000000\n".to_vec();
    big.resize(4 << 20, b'0');
    big.push(b'\n');
    std::fs::write(paths.build_marker(), big).expect("an oversized marker");
    let root = base.path().to_path_buf();
    let said = within_hang("daemon-status over a 4 MiB marker", move || report(&root));
    assert!(
        said.contains(
            "its last daemon (pid 4242) died during its index build (started 1760000000); no \
             shim has seen it yet"
        ),
        "{said}"
    );
}

/// Review round 1, sol 4: the runtime directory's own path is printed on one line too.
#[test]
fn a_runtime_directory_with_control_characters_is_printed_on_one_line() {
    let base = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base");
    let odd = base.path().join("a\nb\u{1b}c");
    std::fs::create_dir(&odd).expect("an odd base");
    let said = report(&odd);
    assert_eq!(
        said,
        format!(
            "runtime dir {}/a\\nb\\u{{1b}}c/prism-mcpd\nno daemon (the runtime directory does not \
             exist)\n",
            base.path().display()
        )
    );
}

/// Review round 1, Opus S7: a key whose files cannot be opened (a symlink in place of its start
/// lock) is reported on its own line, and the other keys are still listed. Rules out one bad key
/// failing the whole report.
#[test]
fn a_key_that_cannot_be_looked_at_does_not_hide_the_others() {
    let (base, paths) = runtime();
    let _fake = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    let dir = paths.runtime_dir();
    std::fs::write(dir.join("1111111111111111.pid"), "1\n2\nb\n").expect("a pid file");
    std::os::unix::fs::symlink("/dev/null", dir.join("1111111111111111.start")).expect("symlink");
    let said = report(base.path());
    assert!(
        said.contains("\n1111111111111111 cannot be looked at: "),
        "{said}"
    );
    assert!(
        said.contains(&format!("\n{LOCATOR} pid 4242, started 1760600000")),
        "{said}"
    );
}

/// Review round 1, Opus W1: a live daemon whose socket refuses connections (its backlog is full)
/// is named alive, with the connect error, not "no socket"; one without a socket says so.
#[test]
fn a_live_daemon_that_does_not_accept_is_named_with_why() {
    let (base, paths) = runtime();
    write_pid_file(&paths);
    let owner = open_lock(paths.owner_lock());
    owner.try_lock().expect("the owner lock");
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} alive but not answering (it has no socket: starting or draining); "
        )),
        "{said}"
    );
    let (_listener, _clients) = full_backlog(paths.socket());
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} alive but not answering (connect failed: "
        )),
        "{said}"
    );
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t5-red.log" 2>&1
```

Expected in the log: ``error[E0583]: file not found for module `control` `` and the same for `status`, and ``error[E0599]: no function or associated item named `from_locator` found for struct `shared::paths::DaemonPaths` `` (twice); ``could not compile `prism` (lib test) due to 4 previous errors``.

- [ ] **Step 3: Implement**

In `src/mcp/mod.rs`, replace this:

````rust
    FIRST_CALL_WAIT_MAX,
};
#[cfg(unix)]
pub use shim::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
````

with this:

````rust
    FIRST_CALL_WAIT_MAX,
};
#[cfg(unix)]
pub use shim::{
    run_daemon_status, run_shim, ShimOptions, StatusOptions, DEFAULT_CONTROL_TIMEOUT_SECS,
    DEFAULT_DAEMON_START_TIMEOUT_SECS,
};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
````

In `src/mcp/shared/paths.rs`, replace this:

````rust
    /// than [`MAX_SOCKET_PATH_BYTES`]. `runtime_dir` is used as given; validating it is the
    /// caller's separate step (`validate_runtime_dir`).
    pub fn new(runtime_dir: &Path, key: &IdentityKey) -> Result<Self, SocketPathTooLong> {
        let locator = key.locator();
        let file = |extension: &str| runtime_dir.join(format!("{locator}.{extension}"));
        let socket = file("sock");
        let len = socket.as_os_str().as_bytes().len();
````

with this:

````rust
    /// than [`MAX_SOCKET_PATH_BYTES`]. `runtime_dir` is used as given; validating it is the
    /// caller's separate step (`validate_runtime_dir`).
    pub fn new(runtime_dir: &Path, key: &IdentityKey) -> Result<Self, SocketPathTooLong> {
        Self::from_locator(runtime_dir, &key.locator())
    }

    /// The paths of the daemon whose key starts with `locator`, for `daemon-status`, which lists a
    /// runtime directory by its file names and so knows only the first 16 hex digits of each key
    /// (S5b). `locator` is used as given: the caller takes it from a file name it has checked is
    /// 16 lowercase hexadecimal digits.
    pub fn from_locator(runtime_dir: &Path, locator: &str) -> Result<Self, SocketPathTooLong> {
        let file = |extension: &str| runtime_dir.join(format!("{locator}.{extension}"));
        let socket = file("sock");
        let len = socket.as_os_str().as_bytes().len();
````

In `src/mcp/shared/preamble.rs`, replace this:

````rust
}

impl BuildState {
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
````

with this:

````rust
}

impl BuildState {
    /// The word `READY` and the operator's status reply use for this state.
    pub fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
````

In `src/mcp/shim/connect.rs`, replace this:

````rust

/// The time left before `deadline`, or `None` once it has passed. Never zero, which a socket
/// timeout does not accept.
fn left(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
````

with this:

````rust

/// The time left before `deadline`, or `None` once it has passed. Never zero, which a socket
/// timeout does not accept.
pub(super) fn left(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
````

In `src/mcp/shim/connect.rs`, replace this:

````rust
}

/// Read the reply line before the deadline: every read is bounded by the time left, and a line
/// whose last byte arrives at the deadline is not accepted.
fn read_reply(
    stream: &mut UnixStream,
    deadline: Instant,
    hook: Option<&Hook>,
````

with this:

````rust
}

/// Read the reply line before the deadline: every read is bounded by the time left, and a line
/// whose last byte arrives at the deadline is not accepted. The control exchange of
/// `daemon-status` and `daemon-shutdown` reads its reply with it too (S5b).
pub(super) fn read_reply(
    stream: &mut UnixStream,
    deadline: Instant,
    hook: Option<&Hook>,
````

Create `src/mcp/shim/control.rs`:

````rust
//! One control exchange with a daemon (spec §4.7), for `daemon-status` and `daemon-shutdown`:
//! connect without waiting past the deadline (`socket.rs`), check that the peer is this user
//! before anything is sent (D78, as the shim does), send one control line, and read the one reply
//! line with every byte's read bounded by the time left (`connect::read_reply`). The daemon closes
//! the connection after its reply; the connection is never admitted.
use super::connect::{left, read_reply, PeerUid, Reached, Refusal, Transient};
use super::socket::connect_within;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus};
use crate::mcp::shared::preamble::Reply;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

/// How a control exchange ended.
#[derive(Debug)]
pub(super) enum Exchanged {
    /// The daemon's status.
    Status(DaemonStatus),
    /// The daemon drains (spec §3.1).
    Retiring,
    /// The daemon is at its client bound; a control request counts against it until its line is
    /// read (spec §4.2).
    Busy { max_clients: u32 },
    /// Nothing accepted the connection: no socket, one that no daemon listens on, or a full
    /// backlog.
    NotListening(io::Error),
    /// The daemon accepted and sent no whole reply line: it closed (a daemon that does not know
    /// this exchange, or this version of it, closes the line it refuses), or the deadline passed.
    NoReply(Transient),
    /// A peer of another user, or whose uid could not be read (nothing was sent to it), or a
    /// reply this build cannot read.
    Refused(Refusal),
}

/// Make one control exchange with the daemon at `socket`; `uid` is this process's effective uid.
pub(super) fn exchange(
    socket: &Path,
    request: &ControlRequest,
    deadline: Instant,
    peer_uid: PeerUid,
    uid: u32,
) -> Exchanged {
    let mut stream = match connect_within(socket, deadline) {
        Ok(stream) => stream,
        Err(error) => return Exchanged::NotListening(error),
    };
    match peer_uid(&stream) {
        Ok(found) if found == uid => {}
        Ok(found) => return Exchanged::Refused(Refusal::ForeignPeer { uid: found }),
        Err(error) => return Exchanged::Refused(Refusal::PeerUnknown(error)),
    }
    let Some(time) = left(deadline) else {
        return Exchanged::NoReply(Transient::NoReply);
    };
    // A daemon at its bound answers `BUSY` and may close before the line is written (`EPIPE`);
    // its reply is read anyway, as the shim reads it (S4's interface for S5a).
    let _ = stream.set_write_timeout(Some(time));
    let _ = stream.write_all(request.encode().as_bytes());
    let line = match read_reply(&mut stream, deadline, None) {
        Ok(line) => line,
        Err(Reached::Transient(transient)) => return Exchanged::NoReply(transient),
        Err(Reached::Refused(refusal)) => return Exchanged::Refused(refusal),
        // `read_reply` reads a line; it never reaches a daemon.
        Err(Reached::Ready(..)) => return Exchanged::NoReply(Transient::Closed),
    };
    if line.starts_with(b"STATUS ") {
        return match DaemonStatus::decode(&line) {
            Ok(status) => Exchanged::Status(status),
            Err(error) => Exchanged::Refused(Refusal::BadReply(error.to_string())),
        };
    }
    match Reply::decode(&line) {
        Ok(Reply::Retiring) => Exchanged::Retiring,
        Ok(Reply::Busy { max_clients }) => Exchanged::Busy { max_clients },
        Ok(other) => Exchanged::Refused(Refusal::BadReply(format!(
            "{} does not answer a control request",
            other.encode().trim_end()
        ))),
        Err(error) => Exchanged::Refused(Refusal::BadReply(error.to_string())),
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
````

In `src/mcp/shim/diagnostic.rs`, replace this:

````rust
/// files are: through the directory's descriptor, never through a symlink, without waiting for
/// a writer (`O_NONBLOCK`), and only when it is a regular file of this user. So nothing at its
/// path can keep the line from being printed (review round 1, Opus F1, sol 1).
fn pid_file(paths: &DaemonPaths, dir_validated: bool) -> String {
    let path = paths.pid_file();
    if !dir_validated {
        return format!(
````

with this:

````rust
/// files are: through the directory's descriptor, never through a symlink, without waiting for
/// a writer (`O_NONBLOCK`), and only when it is a regular file of this user. So nothing at its
/// path can keep the line from being printed (review round 1, Opus F1, sol 1).
pub(super) fn pid_file(paths: &DaemonPaths, dir_validated: bool) -> String {
    let path = paths.pid_file();
    if !dir_validated {
        return format!(
````

In `src/mcp/shim/key_files.rs`, replace this:

````rust
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::path::Path;

/// The runtime directory of one key, opened once.
pub(super) struct KeyDir<'a> {
````

with this:

````rust
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::path::Path;

/// Most bytes of a build marker read: three short lines.
const MARKER_BYTES: u64 = 256;

/// What is at a key's build marker path.
pub(super) enum Marker {
    Absent,
    /// Something a daemon did not write there (not a regular file of this user), or a marker
    /// that cannot be opened; why, in words.
    Unusable(String),
    Read(MarkerContents),
}

/// The runtime directory of one key, opened once.
pub(super) struct KeyDir<'a> {
````

In `src/mcp/shim/key_files.rs`, replace this:

````rust
        }
    }

    /// What the build marker holds, when there is one. Call it holding the owner lock: then
    /// the marker is a dead daemon's.
    pub(super) fn build_marker(&self) -> Option<MarkerContents> {
        let mut file = open_private(
            &self.dir,
            self.paths.build_marker(),
            libc::O_RDONLY,
            Create::No,
        )
        .ok()?;
        let mut text = String::new();
        // A marker that cannot be read whole is still a dead build's: its fields are unknown.
        let _ = file.read_to_string(&mut text);
        Some(parse_build_marker(&text))
    }

    /// Record in the build marker that a shim first saw its daemon dead at `now` (seconds since
````

with this:

````rust
        }
    }

    /// What the build marker holds, when there is one that a daemon could have written (a
    /// regular file of this user). Call it holding the owner lock: then the marker is a dead
    /// daemon's. Anything else at its path is no marker, and the stale files' removal removes it.
    pub(super) fn build_marker(&self) -> Option<MarkerContents> {
        match self.read_build_marker() {
            Marker::Read(marker) => Some(marker),
            Marker::Absent | Marker::Unusable(_) => None,
        }
    }

    /// Read the build marker without waiting and only so far (S5b review round 1, sol 2 and
    /// Opus S6): through the directory's descriptor, never through a symlink, without waiting for
    /// a writer (`O_NONBLOCK`: a FIFO in its place would otherwise hold the caller, and the locks
    /// it holds, for ever), only a regular file of this user, and at most [`MARKER_BYTES`] of it,
    /// far more than the three short lines a daemon and a shim write.
    pub(super) fn read_build_marker(&self) -> Marker {
        let path = self.paths.build_marker();
        match open_private(
            &self.dir,
            path,
            libc::O_RDONLY | libc::O_NONBLOCK,
            Create::No,
        ) {
            Err(FilesError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                Marker::Absent
            }
            Err(error) => Marker::Unusable(error.to_string()),
            Ok(file) => {
                let mut text = String::new();
                // A marker that cannot be read whole is still a dead build's: its fields are
                // unknown.
                let _ = file.take(MARKER_BYTES).read_to_string(&mut text);
                Marker::Read(parse_build_marker(&text))
            }
        }
    }

    /// Record in the build marker that a shim first saw its daemon dead at `now` (seconds since
````

In `src/mcp/shim/mod.rs`, replace this:

````rust
//! between its client's stdio and the daemon's socket. If no daemon answers `READY` within one
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6, §10.

mod arbitrate;
mod connect;
````

with this:

````rust
//! between its client's stdio and the daemon's socket. If no daemon answers `READY` within one
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! The operator's commands, `prism-mcp daemon-status` and `daemon-shutdown` (spec §4.7), are
//! clients of the same sockets and files and live here too (D101).
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6,
//! §4.7, §10.

mod arbitrate;
mod connect;
````

In `src/mcp/shim/mod.rs`, replace this:

````rust
mod status;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
````

with this:

````rust
mod status;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
pub use status::{run_daemon_status, StatusOptions, DEFAULT_CONTROL_TIMEOUT_SECS};
````

Create `src/mcp/shim/status.rs`:

````rust
//! `prism-mcp daemon-status` (spec §4.7): what every daemon in the runtime directory is doing, or
//! the one daemon of a configuration.
//!
//! It reads and never removes. A key with a socket is asked through the control exchange. A key
//! whose socket does not answer, or that has a pid file or a build marker and no socket, is
//! looked at through its locks: its start lock first, and its owner lock only while the start
//! lock is held, each for a moment, so that the look keeps out of a shim's start; at worst a daemon
//! about to claim the key finds the owner lock taken and exits, leaving no owner, never two
//! (D102). Keys with nothing but
//! lock files and a log, which every build of prism leaves behind (S4's D70), are counted in one
//! line, not listed and not removed (owner question 1).
use super::arbitrate::BUILD_CRASH_BACKOFF;
use super::connect::PeerUid;
use super::control::{exchange, Exchanged};
use super::diagnostic::{one_line, pid_file};
use super::key_files::{KeyDir, Marker};
use crate::mcp::daemon::files::MarkerContents;
use crate::mcp::daemon::peer::peer_uid;
use crate::mcp::session::canonical_config;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus};
use crate::mcp::shared::identity::{DaemonIdentity, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::runtime_dir::{
    current_uid, runtime_dir, runtime_dir_from, validate_runtime_dir,
};
use crate::mcp::ServerConfig;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs::TryLockError;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// `--timeout` default, seconds: how long one daemon has to answer.
pub const DEFAULT_CONTROL_TIMEOUT_SECS: u64 = 10;

/// The suffixes of a key's files (spec §4.3; the build marker, S5a).
const KEY_FILE_SUFFIXES: [&str; 6] = ["sock", "start", "owner", "pid", "log", "build"];

/// What `prism-mcp daemon-status` was started with.
#[derive(Clone, Debug)]
pub struct StatusOptions {
    /// `--repo` (with `--cache-dir` and `--refresh-policy`): show this configuration's daemon
    /// only. `None` shows every key in the runtime directory.
    pub config: Option<ServerConfig>,
    /// `--runtime-dir`: the base the runtime directory is in, as `PRISM_MCP_RUNTIME_DIR` gives it
    /// (D103). `None` resolves it as a shim does.
    pub runtime_base: Option<PathBuf>,
    /// `--timeout`: how long each daemon has to answer.
    pub timeout: Duration,
}

/// Print the report on standard output and return 0, or one line on standard error and 1.
pub fn run_daemon_status(options: &StatusOptions) -> i32 {
    match status_report(options, peer_uid) {
        Ok(report) => {
            print!("{report}");
            0
        }
        Err(error) => {
            eprintln!("prism-mcp daemon-status: {}", one_line(&error));
            1
        }
    }
}

/// The runtime directory for `base` (`--runtime-dir`), or as a shim resolves it.
pub(super) fn operator_runtime_dir(base: Option<&Path>) -> Result<PathBuf, String> {
    match base {
        Some(base) => runtime_dir_from(
            Some(base.as_os_str().to_owned()),
            dirs::runtime_dir(),
            std::env::temp_dir(),
        ),
        None => runtime_dir(),
    }
    .map_err(|error| error.to_string())
}

/// The identity key of `cfg` for this build, as its shim computes it.
pub(super) fn key_of(cfg: &ServerConfig) -> Result<IdentityKey, String> {
    let cfg = canonical_config(cfg).map_err(|error| error.to_string())?;
    let identity = DaemonIdentity::resolve(&cfg).map_err(|error| error.to_string())?;
    Ok(identity.key())
}

/// Seconds since the Unix epoch, as the daemon and the build marker write them.
pub(super) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The report, or why there is none.
pub(super) fn status_report(options: &StatusOptions, peer_uid: PeerUid) -> Result<String, String> {
    let dir = operator_runtime_dir(options.runtime_base.as_deref())?;
    let wanted = options.config.as_ref().map(key_of).transpose()?;
    let mut report = String::new();
    let _ = writeln!(
        report,
        "{}",
        one_line(&format!("runtime dir {}", dir.display()))
    );
    if std::fs::symlink_metadata(&dir).is_err() {
        report.push_str("no daemon (the runtime directory does not exist)\n");
        return Ok(report);
    }
    validate_runtime_dir(&dir).map_err(|error| error.to_string())?;
    let locators = match wanted {
        Some(key) => BTreeSet::from([key.locator()]),
        None => locators_in(&dir)?,
    };
    let (mut quiet, mut log_bytes) = (0u64, 0u64);
    for locator in &locators {
        let paths = DaemonPaths::from_locator(&dir, locator).map_err(|error| error.to_string())?;
        match look(&paths, locator, wanted, options.timeout, peer_uid) {
            Ok(Some(text)) => report.push_str(&text),
            Ok(None) if wanted.is_some() => {
                let _ = writeln!(report, "{locator} no daemon for this configuration");
            }
            Ok(None) => {
                quiet += 1;
                log_bytes += std::fs::symlink_metadata(paths.log_file()).map_or(0, |m| m.len());
            }
            // One key that cannot be looked at does not hide the others (review round 1, Opus S7).
            Err(error) => {
                let _ = writeln!(
                    report,
                    "{}",
                    one_line(&format!("{locator} cannot be looked at: {error}"))
                );
            }
        }
    }
    if locators.is_empty() {
        report.push_str("no daemon\n");
    }
    if quiet > 0 {
        let _ = writeln!(
            report,
            "{quiet} keys have nothing but lock files and logs ({log_bytes} bytes of logs)"
        );
    }
    Ok(report)
}

/// The locators of every key that has a file in `dir`: names of 16 lowercase hexadecimal digits
/// and one of the key files' suffixes. Anything else in the directory is not prism's and is
/// ignored.
fn locators_in(dir: &Path) -> Result<BTreeSet<String>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|error| format!("cannot list {}: {error}", dir.display()))?;
    let mut locators = BTreeSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some((stem, suffix)) = name.to_str().and_then(|name| name.split_once('.')) else {
            continue;
        };
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if stem.len() == 16 && stem.chars().all(hex) && KEY_FILE_SUFFIXES.contains(&suffix) {
            locators.insert(stem.to_string());
        }
    }
    Ok(locators)
}

/// What one key's lines say, or `None` for a key with no socket, no pid file and no build marker
/// (nothing of a daemon's but lock files and a log).
fn look(
    paths: &DaemonPaths,
    locator: &str,
    wanted: Option<IdentityKey>,
    timeout: Duration,
    peer_uid: PeerUid,
) -> Result<Option<String>, String> {
    let exists = |path: &Path| std::fs::symlink_metadata(path).is_ok();
    let mut no_answer = None;
    if exists(paths.socket()) {
        let deadline = Instant::now() + timeout;
        match exchange(
            paths.socket(),
            &ControlRequest::Status,
            deadline,
            peer_uid,
            current_uid(),
        ) {
            Exchanged::Status(status) => return Ok(Some(describe(locator, &status, wanted))),
            Exchanged::Retiring => {
                return Ok(Some(format!("{locator} draining: it answered RETIRING\n")))
            }
            Exchanged::Busy { max_clients } => {
                return Ok(Some(format!(
                    "{locator} busy: it answered BUSY at its bound of {max_clients} clients, so \
                     no status\n"
                )))
            }
            Exchanged::NotListening(error) => no_answer = Some(format!("connect failed: {error}")),
            Exchanged::NoReply(reason) => no_answer = Some(reason.to_string()),
            Exchanged::Refused(reason) => no_answer = Some(reason.to_string()),
        }
    } else if !has_daemon_files(paths) {
        return Ok(None);
    }
    let line = match under_locks(paths, |dir| dead(paths, locator, dir))? {
        Holder::Starting => format!(
            "{locator} being started or reached: a shim holds its start lock; pid file {}",
            pid_file(paths, true)
        ),
        Holder::Alive => format!(
            "{locator} alive but not answering ({}); pid file {}",
            no_answer.unwrap_or_else(|| "it has no socket: starting or draining".to_string()),
            pid_file(paths, true)
        ),
        Holder::Free(line) => line,
    };
    Ok(Some(one_line(&line) + "\n"))
}

/// Whether anything of a daemon's is at a key's paths: a socket, a pid file or a build marker.
/// A key with none of them has nothing but lock files and a log, and is not looked at.
pub(super) fn has_daemon_files(paths: &DaemonPaths) -> bool {
    let exists = |path: &Path| std::fs::symlink_metadata(path).is_ok();
    exists(paths.socket()) || exists(paths.pid_file()) || exists(paths.build_marker())
}

/// Who holds a key, as `daemon-status` and `daemon-shutdown` look (D102).
pub(super) enum Holder<T> {
    /// A shim holds the start lock: it is starting or reaching the daemon.
    Starting,
    /// A daemon holds the owner lock.
    Alive,
    /// Nobody holds either; what `free` found, run while this process held both.
    Free(T),
}

/// Look at a key's locks without waiting: the start lock first and, holding it, the owner lock,
/// each for a moment; `free` runs while both are held. Neither lock file is ever removed.
pub(super) fn under_locks<T>(
    paths: &DaemonPaths,
    free: impl FnOnce(&KeyDir<'_>) -> T,
) -> Result<Holder<T>, String> {
    let dir = KeyDir::open(paths).map_err(|error| error.to_string())?;
    let start = dir.start_lock().map_err(|error| error.to_string())?;
    match start.try_lock() {
        Err(TryLockError::WouldBlock) => Ok(Holder::Starting),
        Err(TryLockError::Error(error)) => Err(format!(
            "cannot lock {}: {error}",
            paths.start_lock().display()
        )),
        Ok(()) => match dir.try_own().map_err(|error| error.to_string())? {
            None => Ok(Holder::Alive),
            Some(_owner) => Ok(Holder::Free(free(&dir))),
        },
    }
}

/// A key no daemon owns: what a dead daemon left, and the restart bound if it died during its
/// build. Nothing is removed (the next shim of the key removes the socket and the pid file, D77).
fn dead(paths: &DaemonPaths, locator: &str, dir: &KeyDir<'_>) -> String {
    let mut left = Vec::new();
    if std::fs::symlink_metadata(paths.socket()).is_ok() {
        left.push("its socket".to_string());
    }
    if std::fs::symlink_metadata(paths.pid_file()).is_ok() {
        left.push(format!("its pid file ({})", pid_file(paths, true)));
    }
    let mut line = format!("{locator} no daemon");
    if !left.is_empty() {
        let _ = write!(line, "; a dead daemon left {}", left.join(" and "));
    }
    match dir.read_build_marker() {
        Marker::Absent => {}
        Marker::Read(marker) => {
            let _ = write!(line, "; {}", died_building(&marker, unix_now()));
        }
        Marker::Unusable(why) => {
            let _ = write!(
                line,
                "; its build marker is not usable ({why}); the next shim removes it"
            );
        }
    }
    line
}

/// What a build marker of a dead daemon says (S5a's D77): who died during its build, and when a
/// shim may start the key's daemon again. The rule is `arbitrate`'s: 60 s from the first sighting,
/// and a sighting dated in the future does not count.
pub(super) fn died_building(marker: &MarkerContents, now: u64) -> String {
    let known = |field: Option<u64>| field.map_or_else(|| "unknown".to_string(), |v| v.to_string());
    let backoff = BUILD_CRASH_BACKOFF.as_secs();
    let when = match marker.seen {
        None => format!(
            "no shim has seen it yet: the next one records the sighting and waits {backoff} s"
        ),
        Some(seen) => match now.checked_sub(seen) {
            Some(age) if age < backoff => format!(
                "first seen by a shim at {seen}; a shim restarts it in {} s",
                backoff - age
            ),
            _ => format!("first seen by a shim at {seen}; the next shim restarts it"),
        },
    };
    format!(
        "its last daemon (pid {}) died during its index build (started {}); {when}",
        known(marker.pid.map(u64::from)),
        known(marker.started)
    )
}

/// The lines of a daemon that answered.
fn describe(locator: &str, status: &DaemonStatus, wanted: Option<IdentityKey>) -> String {
    let instance = &status.instance;
    let idle = match status.idle_secs {
        Some(secs) => format!(
            "idle {secs} s; retires after {} s idle",
            status.idle_ttl_secs
        ),
        None => "in use".to_string(),
    };
    let mut lines = vec![
        format!(
            "{locator} pid {}, started {}, serving {}",
            instance.pid, instance.start_time, status.repo_root
        ),
        format!(
            "  key {}; build {}",
            instance.identity, status.build_identity
        ),
        format!(
            "  clients: {} admitted, {} pending, at most {}; {idle}",
            status.admitted, status.pending, status.max_clients
        ),
        format!(
            "  index: {} (attempt {}, generation {})",
            status.build.word(),
            status.attempt,
            status.generation
        ),
    ];
    if status.shutdown {
        lines.push("  shutdown requested: it drains once no client is left".to_string());
    }
    if let Some(key) = wanted.filter(|key| *key != instance.identity) {
        lines.push(format!(
            "  this is not the daemon of this configuration ({key}): another key with the same \
             first 16 digits"
        ));
    }
    lines.iter().map(|line| one_line(line) + "\n").collect()
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t5.log" 2>&1
```

Expected: `test result: ok. 619 passed; 0 failed` (596 + 23), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/shared src/mcp/shim src/mcp/mod.rs
git commit -m "feat(mcp): prism-mcp daemon-status, the operator's view of the shared daemons"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::shim::`, see the named test fail, then `git checkout -- <file>`.

| Mutant | Edit | Fails |
|---|---|---|
| 5A | in `shim/control.rs`, `exchange`: write the request before the peer check (insert `let _ = stream.write_all(request.encode().as_bytes());` above `match peer_uid(&stream) {`) | `a_peer_of_another_user_is_refused_before_anything_is_sent` |
| 5B | in `status.rs`, `under_locks`: `match start.try_lock() {` → `match Ok::<(), TryLockError>(()) {` (the owner lock looked at without the start lock) | `a_key_a_shim_is_starting_is_not_looked_at_under_its_owner_lock` |
| 5C | in `dead`, first line: `let _ = dir.remove_stale();` | `a_dead_daemon_s_leftovers_are_named_and_left_in_place`, `a_daemon_that_died_during_its_build_says_when_it_may_restart` |
| 5D | in `died_building`, `backoff - age` → `backoff` | `a_daemon_that_died_during_its_build_says_when_it_may_restart` |
| 5E | in `look`, `} else if !has_daemon_files(paths) {` → `} else if false {` (every key's locks looked at) | `keys_with_only_lock_files_and_logs_are_counted_not_listed`, `with_a_repository_only_its_daemon_is_shown` |
| 5F | in `describe`, `one_line(line) + "\n"` → `line.clone() + "\n"` | `a_root_with_control_characters_is_printed_on_one_line` |
| R3 | in `look`, `Exchanged::NotListening(error) => no_answer = Some(format!("connect failed: {error}")),` → `Exchanged::NotListening(_) => {}` (review round 1, Opus W1) | `a_live_daemon_that_does_not_accept_is_named_with_why` |
| R5 | in `key_files.rs`, `read_build_marker`: `libc::O_RDONLY | libc::O_NONBLOCK,` → `libc::O_RDONLY,` (review round 1, sol 2) | `a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report` and the shim's `a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key`, each at its 30 s watchdog |
| R6 | in `read_build_marker`, read the whole file: `file.take(MARKER_BYTES).read_to_string(&mut text)` → `file.read_to_string(&mut text)` (with `let mut file = file;`) | `a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report` (its third line, 4 MiB in, becomes a sighting at time 0) |
| R7 | in `status_report`, the header without `one_line` (review round 1, sol 4) | `a_runtime_directory_with_control_characters_is_printed_on_one_line` |
| R8 | in `status_report`, a key's error ends the report: `Err(error) => return Err(error),` (review round 1, Opus S7) | `a_key_that_cannot_be_looked_at_does_not_hide_the_others` |
| R11 | (superseded by R13 in PR #418 review round 2, which moved its rule into `present`) in `runtime_dir_exists`, `Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),` → `Err(_) => Ok(false),` (every lookup error taken for absence; PR #418 review round 1, sol 1) | `a_runtime_directory_that_cannot_be_looked_at_is_not_taken_for_none`, in `status` and in `shutdown` (not under root) |
| R13 | in `present`, `Err(error) => Err(format!("cannot look at {}: {error}", path.display())),` → `Err(_) => Ok(false),` (every lookup error taken for absence; PR #418 review round 2, sol 1) | both `a_runtime_directory_that_cannot_be_looked_at_is_not_taken_for_none`, `a_key_that_cannot_be_looked_at_after_the_listing_is_not_counted_as_empty`, `a_key_that_cannot_be_looked_at_after_the_status_is_not_taken_for_gone` (not under root) |
| R14 | in `status.rs`, every `present(paths.<file>())?` → `present(paths.<file>()).unwrap_or(false)` (the per-key checks alone lax) | `a_key_that_cannot_be_looked_at_after_the_listing_is_not_counted_as_empty`, `a_key_that_cannot_be_looked_at_after_the_status_is_not_taken_for_gone` (not under root) |
| R15 | in `status_lines`, `unlooked += 1;` removed | `a_report_with_a_key_that_cannot_be_looked_at_exits_1` |
| R16 | in `present`, `NotFound \| NotADirectory` → `NotFound` | `a_runtime_base_that_is_a_file_has_no_daemon`, `a_runtime_base_that_is_a_file_has_nothing_to_shut_down` |

### Task 6: `daemon-shutdown`

**Files:**
- Modify: `src/mcp/shim/key_files.rs`, `src/mcp/shim/mod.rs`, `src/mcp/mod.rs`
- Create: `src/mcp/shim/shutdown.rs`, `src/mcp/shim/shutdown_tests.rs`

**Interfaces:**
- Consumes: Task 5's `exchange`, `Exchanged`, `key_of`, `operator_runtime_dir`, `has_daemon_files`, `under_locks`, `Holder`, `ControlFake`; `shim_support::{full_backlog, open_lock}` (S5a); `arbitrate::POLL`, `diagnostic::{one_line, pid_file}`, `KeyDir` (S5a).
- Produces: `KeyDir::pid_text(&self) -> Option<String>` (at most 200 bytes, read through the directory's descriptor, never through a symlink, without waiting for a writer); in `shim::shutdown`: `pub struct ShutdownOptions { config: ServerConfig, runtime_base: Option<PathBuf>, instance: Option<(u32, u64)>, timeout: Duration }`, `pub fn run_daemon_shutdown(&ShutdownOptions) -> i32`, `pub(super) fn shutdown(&ShutdownOptions, PeerUid) -> Result<String, String>` (`Ok` is the exit-0 line); private `not_listening` (a refused connection judged by the locks) and `maybe_recorded` (a lost reply to the request). Re-exported from `crate::mcp`: `run_daemon_shutdown`, `ShutdownOptions`.

- [ ] **Step 1: Write the failing tests**

In `src/mcp/shim/mod.rs`, replace this:

````rust
mod run;
#[cfg(test)]
mod shim_support;
mod socket;
mod spawn;
mod status;
````

with this:

````rust
mod run;
#[cfg(test)]
mod shim_support;
mod shutdown;
mod socket;
mod spawn;
mod status;
````

Create `src/mcp/shim/shutdown_tests.rs`:

````rust
//! Tests for `daemon-shutdown` against a stand-in daemon (S5b; spec §4.7, §11 item 3): which
//! instance is asked, what is never sent, and when the command says the daemon drained.
use super::super::control_support::{
    fake_instance, fake_status, ControlFake, FakeReply, FAKE_PID, FAKE_START,
};
use super::super::shim_support::{full_backlog, open_lock, this_user, HANG};
use super::*;
use crate::mcp::shared::control::DaemonStatus;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::shared::runtime_dir::validate_runtime_dir;
use crate::mcp::CacheMode;
use std::sync::Arc;

/// A configuration of the test's own, its key for this build, and a validated runtime
/// directory under `/tmp` with that key's paths.
struct Setup {
    _repo: tempfile::TempDir,
    _cache: tempfile::TempDir,
    base: tempfile::TempDir,
    config: ServerConfig,
    key: IdentityKey,
    paths: DaemonPaths,
}

fn setup() -> Setup {
    let repo = tempfile::tempdir().expect("a repository");
    let cache = tempfile::tempdir().expect("a cache base");
    let base = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base under /tmp");
    let mut config = ServerConfig::new(repo.path().to_path_buf());
    config.cache = CacheMode::Dir(cache.path().to_path_buf());
    let key = key_of(&config).expect("a key");
    let dir = base.path().join("prism-mcpd");
    validate_runtime_dir(&dir).expect("a valid runtime directory");
    let paths = DaemonPaths::new(&dir, &key).expect("paths");
    Setup {
        _repo: repo,
        _cache: cache,
        base,
        config,
        key,
        paths,
    }
}

impl Setup {
    fn options(&self, instance: Option<(u32, u64)>, timeout: Duration) -> ShutdownOptions {
        ShutdownOptions {
            config: self.config.clone(),
            runtime_base: Some(self.base.path().to_path_buf()),
            instance,
            timeout,
        }
    }

    /// The fake's status, as the daemon of this configuration.
    fn status(&self, admitted: u32, shutdown: bool) -> DaemonStatus {
        let mut status = fake_status(admitted, shutdown);
        status.instance.identity = self.key;
        status
    }

    fn instance(&self) -> Instance {
        Instance {
            identity: self.key,
            ..fake_instance()
        }
    }

    fn write_pid_file(&self) {
        std::fs::write(
            self.paths.pid_file(),
            format!("{FAKE_PID}\n{FAKE_START}\n{}\n", "b".repeat(64)),
        )
        .expect("write the pid file");
    }

    /// A fake that answers `STATUS` with `status` and `SHUTDOWN` with `accepted`, running
    /// `on_shutdown` first.
    fn fake(
        &self,
        status: DaemonStatus,
        accepted: DaemonStatus,
        on_shutdown: impl Fn() + Send + Sync + 'static,
    ) -> ControlFake {
        ControlFake::start(
            &self.paths,
            true,
            Arc::new(move |line: &str| {
                if line.starts_with("SHUTDOWN ") {
                    on_shutdown();
                    FakeReply::Line(accepted.encode())
                } else {
                    FakeReply::Line(status.encode())
                }
            }),
        )
    }
}

fn run(options: &ShutdownOptions) -> Result<String, String> {
    shutdown(options, this_user)
}

/// No daemon, or no runtime directory at all: nothing to do, and nothing is created.
#[test]
fn with_no_daemon_there_is_nothing_to_shut_down() {
    let setup = setup();
    let said = run(&setup.options(None, HANG)).expect("nothing to do is not a failure");
    assert!(said.contains("nothing to shut down"), "{said}");
    let empty = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base");
    let options = ShutdownOptions {
        runtime_base: Some(empty.path().to_path_buf()),
        ..setup.options(None, HANG)
    };
    assert!(run(&options)
        .expect("nothing to do")
        .contains("nothing to shut down"));
    assert!(
        std::fs::symlink_metadata(empty.path().join("prism-mcpd")).is_err(),
        "the runtime directory is not created"
    );
}

/// Spec §4.7: the instance a status exchange has just shown is the one asked, and with no client
/// the command waits until it has drained (its pid file is gone). Rules out a request that names
/// no instance, and a command that reports success before the drain.
#[test]
fn an_idle_daemon_is_asked_by_its_instance_and_waited_for() {
    let setup = setup();
    setup.write_pid_file();
    let pid_path = setup.paths.pid_file().to_path_buf();
    let fake = setup.fake(setup.status(0, false), setup.status(0, true), move || {
        let _ = std::fs::remove_file(&pid_path);
    });
    let said = run(&setup.options(None, HANG)).expect("drained");
    assert_eq!(
        said,
        format!("pid {FAKE_PID} (started {FAKE_START}) drained")
    );
    assert_eq!(
        fake.lines(),
        vec![
            ControlRequest::Status.encode(),
            ControlRequest::Shutdown(setup.instance()).encode()
        ]
    );
}

/// Spec §11 item 3: a shutdown never disconnects a client. A daemon with clients accepts the
/// request and the command says it drains when they have gone, without waiting for that.
#[test]
fn a_daemon_with_clients_is_left_serving_them() {
    let setup = setup();
    setup.write_pid_file();
    let _fake = setup.fake(setup.status(2, false), setup.status(2, true), || {});
    let said = run(&setup.options(None, HANG)).expect("accepted");
    assert!(
        said.contains("drains once its 2 admitted clients have disconnected"),
        "{said}"
    );
    assert!(
        setup.paths.pid_file().exists(),
        "the daemon was not drained"
    );
}

/// `--pid` and `--start-time` bind the request to one instance: when the daemon that answers is
/// another, it is left alone and no `SHUTDOWN` is sent. Rules out a command that ignores them.
#[test]
fn another_instance_named_by_pid_and_start_time_is_left_alone() {
    let setup = setup();
    let fake = setup.fake(setup.status(0, false), setup.status(0, true), || {});
    let said = run(&setup.options(Some((FAKE_PID, FAKE_START + 1)), HANG)).expect("left alone");
    assert!(said.contains("which was left alone"), "{said}");
    assert_eq!(fake.lines(), vec![ControlRequest::Status.encode()]);
}

/// The status exchange verifies the instance before it is asked: a daemon at the key's socket
/// that serves another identity (another key with the same first 16 digits) is not asked.
#[test]
fn a_daemon_of_another_identity_is_not_asked() {
    let setup = setup();
    let other = fake_status(0, false);
    assert_ne!(other.instance.identity, setup.key);
    let fake = setup.fake(other.clone(), other, || {});
    let said = run(&setup.options(None, HANG)).expect_err("not this configuration's");
    assert!(said.contains("it was not asked"), "{said}");
    assert_eq!(fake.lines(), vec![ControlRequest::Status.encode()]);
}

/// A daemon that was replaced between the two exchanges answers the `SHUTDOWN` with its own,
/// other, instance and records nothing; the command says so and does not wait.
#[test]
fn a_daemon_replaced_between_the_exchanges_is_left_alone() {
    let setup = setup();
    let mut replaced = setup.status(0, false);
    replaced.instance.pid = FAKE_PID + 1;
    let _fake = setup.fake(setup.status(0, false), replaced, || {});
    let said = run(&setup.options(None, HANG)).expect("left alone");
    assert!(said.contains("was replaced by pid"), "{said}");
}

/// A daemon that accepts the request and does not drain by the timeout (a client arrived) is
/// reported, with exit status 1. Rules out a wait without a bound.
#[test]
fn a_daemon_that_does_not_drain_by_the_timeout_is_reported() {
    let setup = setup();
    setup.write_pid_file();
    let _fake = setup.fake(setup.status(0, false), setup.status(0, true), || {});
    let said = run(&setup.options(None, Duration::from_millis(500))).expect_err("not drained");
    assert!(said.contains("still held key"), "{said}");
}

/// A daemon that is draining is not asked; one at its bound, one that does not answer and one
/// that refuses the line could not be asked (exit status 1).
#[test]
fn draining_busy_silent_and_refusing_daemons_are_told_apart() {
    let setup = setup();
    let retiring = ControlFake::start(
        &setup.paths,
        true,
        Arc::new(|_| FakeReply::Line(Reply::Retiring.encode())),
    );
    assert!(run(&setup.options(None, HANG))
        .expect("draining")
        .contains("already draining"));
    retiring.retire();
    let replies: [(FakeReply, &str); 3] = [
        (
            FakeReply::Line(Reply::Busy { max_clients: 3 }.encode()),
            "bound of 3",
        ),
        (FakeReply::Silent, "does not answer"),
        (FakeReply::Close, "does not answer"),
    ];
    for (reply, says) in replies {
        let reply = Arc::new(std::sync::Mutex::new(Some(reply)));
        let fake = ControlFake::start(
            &setup.paths,
            true,
            Arc::new(move |_| {
                reply
                    .lock()
                    .expect("reply")
                    .take()
                    .unwrap_or(FakeReply::Close)
            }),
        );
        let said = run(&setup.options(None, Duration::from_millis(300))).expect_err(says);
        assert!(said.contains(says), "{said}");
        fake.retire();
    }
}

/// Review round 1, Opus W1: a live daemon that does not accept connections (its listen backlog is
/// full; or, when a dead daemon's files are still there, one that has claimed the key and not
/// bound yet) is not "nothing to shut down": its owner lock is held, and the command fails (exit
/// 1). A key whose socket refuses and whose owner lock
/// is free has no daemon. Rules out taking every connect failure for a missing daemon.
#[test]
fn a_live_daemon_that_does_not_accept_connections_is_not_taken_for_none() {
    let setup = setup();
    setup.write_pid_file();
    let owner = open_lock(setup.paths.owner_lock());
    owner.try_lock().expect("the owner lock");
    let (_listener, _clients) = full_backlog(setup.paths.socket());
    let said = run(&setup.options(None, Duration::from_millis(500))).expect_err("alive");
    assert!(
        said.contains("alive but not accepting connections"),
        "{said}"
    );
    drop(owner);
    let said = run(&setup.options(None, Duration::from_millis(500))).expect("no daemon");
    assert!(said.contains("nothing to shut down"), "{said}");
}

/// Review round 1, Opus W4: when the reply to the `SHUTDOWN` is lost, the request may already be
/// recorded (it cannot be withdrawn), and the command says so instead of "not shut down".
#[test]
fn a_lost_reply_to_the_shutdown_says_it_may_have_been_recorded() {
    let setup = setup();
    let status = setup.status(0, false);
    let _fake = ControlFake::start(
        &setup.paths,
        true,
        Arc::new(move |line: &str| {
            if line.starts_with("SHUTDOWN ") {
                FakeReply::Close
            } else {
                FakeReply::Line(status.encode())
            }
        }),
    );
    let said = run(&setup.options(None, HANG)).expect_err("no reply");
    assert!(said.contains("it may have been recorded"), "{said}");
}

/// Review round 2, Opus S1: the same judgment at the second exchange. The daemon answers the
/// status, then its socket is gone before the `SHUTDOWN` connects: with its owner lock still held
/// it is alive (exit 1), not "gone"; with the lock free it is gone (exit 0).
#[test]
fn a_refused_second_exchange_is_judged_by_the_owner_lock_too() {
    let setup = setup();
    setup.write_pid_file();
    for (held, says) in [
        (true, "alive but not accepting connections"),
        (false, "is gone"),
    ] {
        let owner = open_lock(setup.paths.owner_lock());
        if held {
            owner.try_lock().expect("the owner lock");
        }
        let status = setup.status(0, false);
        let socket = setup.paths.socket().to_path_buf();
        let _fake = ControlFake::start(
            &setup.paths,
            false,
            Arc::new(move |_| {
                let _ = std::fs::remove_file(&socket);
                FakeReply::Line(status.encode())
            }),
        );
        let said = run(&setup.options(None, HANG));
        let line = match &said {
            Ok(line) | Err(line) => line.clone(),
        };
        assert!(line.contains(says), "{said:?}");
        assert_eq!(said.is_err(), held, "{said:?}");
        drop(owner);
    }
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t6-red.log" 2>&1
```

Expected in the log: ``error[E0583]: file not found for module `shutdown` `` and ``could not compile `prism` (lib test) due to 1 previous error``.

- [ ] **Step 3: Implement**

In `src/mcp/mod.rs`, replace this:

````rust
};
#[cfg(unix)]
pub use shim::{
    run_daemon_status, run_shim, ShimOptions, StatusOptions, DEFAULT_CONTROL_TIMEOUT_SECS,
    DEFAULT_DAEMON_START_TIMEOUT_SECS,
};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
````

with this:

````rust
};
#[cfg(unix)]
pub use shim::{
    run_daemon_shutdown, run_daemon_status, run_shim, ShimOptions, ShutdownOptions, StatusOptions,
    DEFAULT_CONTROL_TIMEOUT_SECS, DEFAULT_DAEMON_START_TIMEOUT_SECS,
};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
````

In `src/mcp/shim/key_files.rs`, replace this:

````rust
use std::os::fd::OwnedFd;
use std::path::Path;

/// Most bytes of a build marker read: three short lines.
const MARKER_BYTES: u64 = 256;

````

with this:

````rust
use std::os::fd::OwnedFd;
use std::path::Path;

/// Most bytes of a pid file read: three short lines.
const PID_TEXT_BYTES: u64 = 200;
/// Most bytes of a build marker read: three short lines.
const MARKER_BYTES: u64 = 256;

````

In `src/mcp/shim/key_files.rs`, replace this:

````rust
        }
    }

    /// Record in the build marker that a shim first saw its daemon dead at `now` (seconds since
    /// the Unix epoch), keeping the pid and the build's start (an empty line when unknown, which
    /// reads back as unknown, so no later shim names a pid 0). Call it holding the start and
````

with this:

````rust
        }
    }

    /// The pid file's text, at most [`PID_TEXT_BYTES`] of it, when there is one: read through the
    /// directory's descriptor, never through a symlink, without waiting for a writer, and only
    /// from a regular file of this user. The daemon rewrites it in place, so it can be empty or
    /// partial (S4's interface for S5a).
    pub(super) fn pid_text(&self) -> Option<String> {
        let file = open_private(
            &self.dir,
            self.paths.pid_file(),
            libc::O_RDONLY | libc::O_NONBLOCK,
            Create::No,
        )
        .ok()?;
        let mut text = String::new();
        file.take(PID_TEXT_BYTES).read_to_string(&mut text).ok()?;
        Some(text)
    }

    /// Record in the build marker that a shim first saw its daemon dead at `now` (seconds since
    /// the Unix epoch), keeping the pid and the build's start (an empty line when unknown, which
    /// reads back as unknown, so no later shim names a pid 0). Call it holding the start and
````

In `src/mcp/shim/mod.rs`, replace this:

````rust
mod status;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
pub use status::{run_daemon_status, StatusOptions, DEFAULT_CONTROL_TIMEOUT_SECS};
````

with this:

````rust
mod status;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
pub use shutdown::{run_daemon_shutdown, ShutdownOptions};
pub use status::{run_daemon_status, StatusOptions, DEFAULT_CONTROL_TIMEOUT_SECS};
````

Create `src/mcp/shim/shutdown.rs`:

````rust
//! `prism-mcp daemon-shutdown --repo X` (spec §4.7, §11 item 3): ask the daemon of one
//! configuration to drain once it has no client.
//!
//! Graceful: it never disconnects a client, and until it drains the daemon goes on admitting new
//! ones (D100). Instance-bound: the request names the instance (identity, pid, start time) that a
//! status exchange on the same socket has just shown, or the one `--pid` and `--start-time` name,
//! and a daemon that is another instance records nothing. When the instance had no client, the
//! command waits, up to its timeout, until the daemon has removed its pid file, which it does
//! just before it releases the key.
//!
//! Exit 0 when the instance asked about no longer serves the key when the command ends (it
//! drained, or was already gone or draining), or has accepted the request and drains once its
//! clients have left; exit 1 when the command could not ask, or could not see the drain.
use super::arbitrate::POLL;
use super::connect::PeerUid;
use super::control::{exchange, Exchanged};
use super::diagnostic::{one_line, pid_file};
use super::key_files::KeyDir;
use super::status::{has_daemon_files, key_of, operator_runtime_dir, under_locks, Holder};
use crate::mcp::daemon::peer::peer_uid;
use crate::mcp::shared::control::{ControlRequest, Instance};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use crate::mcp::ServerConfig;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// What `prism-mcp daemon-shutdown` was started with.
#[derive(Clone, Debug)]
pub struct ShutdownOptions {
    /// `--repo`, `--cache-dir`, `--refresh-policy`: the configuration whose daemon (of this
    /// build) is asked.
    pub config: ServerConfig,
    /// `--runtime-dir`, as for `daemon-status`.
    pub runtime_base: Option<PathBuf>,
    /// `--pid` and `--start-time`: ask this instance only. `None` asks the one that answers.
    pub instance: Option<(u32, u64)>,
    /// `--timeout`: the whole command, the wait for the drain included.
    pub timeout: Duration,
}

/// Print one line, on standard output with exit status 0 or on standard error with 1.
pub fn run_daemon_shutdown(options: &ShutdownOptions) -> i32 {
    match shutdown(options, peer_uid) {
        Ok(message) => {
            println!("prism-mcp daemon-shutdown: {}", one_line(&message));
            0
        }
        Err(message) => {
            eprintln!("prism-mcp daemon-shutdown: {}", one_line(&message));
            1
        }
    }
}

fn named(instance: &Instance) -> String {
    format!("pid {} (started {})", instance.pid, instance.start_time)
}

/// The command; `Ok` is the exit-0 line and `Err` the exit-1 line.
pub(super) fn shutdown(options: &ShutdownOptions, peer_uid: PeerUid) -> Result<String, String> {
    let deadline = Instant::now() + options.timeout;
    let key = key_of(&options.config)?;
    let locator = key.locator();
    let nothing = || {
        format!(
            "no daemon serves this configuration of this build (key {locator}); nothing to shut down"
        )
    };
    let dir = operator_runtime_dir(options.runtime_base.as_deref())?;
    if std::fs::symlink_metadata(&dir).is_err() {
        return Ok(nothing());
    }
    validate_runtime_dir(&dir).map_err(|error| error.to_string())?;
    let paths = DaemonPaths::new(&dir, &key).map_err(|error| error.to_string())?;
    let ask = |request: &ControlRequest| {
        exchange(paths.socket(), request, deadline, peer_uid, current_uid())
    };
    let found = match ask(&ControlRequest::Status) {
        Exchanged::Status(status) => status.instance,
        Exchanged::Retiring => {
            return Ok(format!("the daemon of key {locator} is already draining"))
        }
        Exchanged::NotListening(error) => {
            return not_listening(&paths, &locator, &error, nothing())
        }
        other => return Err(not_asked(&paths, &locator, other)),
    };
    if found.identity != key {
        return Err(format!(
            "the daemon at {} serves identity {}, not this configuration's {key}; it was not asked",
            paths.socket().display(),
            found.identity
        ));
    }
    if let Some((pid, start_time)) = options.instance {
        if (found.pid, found.start_time) != (pid, start_time) {
            return Ok(format!(
                "pid {pid} (started {start_time}) does not serve key {locator}; its daemon is {}, \
                 which was left alone",
                named(&found)
            ));
        }
    }
    let accepted = match ask(&ControlRequest::Shutdown(found)) {
        Exchanged::Status(status) => status,
        Exchanged::Retiring => return Ok(format!("{} is already draining", named(&found))),
        Exchanged::NotListening(error) => {
            return not_listening(
                &paths,
                &locator,
                &error,
                format!("{} is gone", named(&found)),
            )
        }
        // `BUSY` is answered before the line is read: nothing was recorded.
        busy @ Exchanged::Busy { .. } => return Err(not_asked(&paths, &locator, busy)),
        // The line was sent and no reply came back: the daemon may have recorded the request
        // before the reply was lost, and a request cannot be withdrawn (review round 1, Opus W4).
        Exchanged::NoReply(reason) => return Err(maybe_recorded(&found, &reason.to_string())),
        Exchanged::Refused(reason) => return Err(maybe_recorded(&found, &reason.to_string())),
    };
    if accepted.instance != found {
        return Ok(format!(
            "{} was replaced by {} before it was asked; the new daemon was left alone",
            named(&found),
            named(&accepted.instance)
        ));
    }
    if !accepted.shutdown {
        return Err(format!("{} did not record the request", named(&found)));
    }
    if accepted.admitted > 0 {
        return Ok(format!(
            "{} drains once its {} admitted clients have disconnected; it serves them, and any \
             new client, until then",
            named(&found),
            accepted.admitted
        ));
    }
    // Nothing is admitted: the daemon drains at its accept loop's next turn, and removes its pid
    // file just before it releases the key.
    let key_dir = KeyDir::open(&paths).map_err(|error| error.to_string())?;
    let written = format!("{}\n{}\n", found.pid, found.start_time);
    loop {
        if !key_dir
            .pid_text()
            .is_some_and(|text| text.starts_with(&written))
        {
            return Ok(format!("{} drained", named(&found)));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "{} accepted the request and still held key {locator} at the timeout; a client \
                 that arrived meanwhile keeps it up until it leaves (see daemon-status)",
                named(&found)
            ));
        }
        std::thread::sleep(POLL);
    }
}

/// The reply to a sent `SHUTDOWN` was lost.
fn maybe_recorded(found: &Instance, why: &str) -> String {
    format!(
        "{} was sent the request and no reply came back ({why}); it may have been recorded, and a \
         request cannot be withdrawn: see daemon-status",
        named(found)
    )
}

/// Nothing accepted a connection at the key's socket. That is no daemon only when nobody holds
/// the key's owner lock: a live daemon whose listen backlog is full, or one that has claimed the
/// key and not bound yet, refuses connections too (review round 1, Opus W1). The locks are
/// looked at as `daemon-status` looks at them (D102), and only when a daemon's files are there.
fn not_listening(
    paths: &DaemonPaths,
    locator: &str,
    error: &std::io::Error,
    none: String,
) -> Result<String, String> {
    if !has_daemon_files(paths) {
        return Ok(none);
    }
    match under_locks(paths, |_| ())? {
        Holder::Free(()) => Ok(none),
        Holder::Alive => Err(format!(
            "the daemon of key {locator} is alive but not accepting connections (connect failed: \
             {error}); it was not asked; pid file {}",
            pid_file(paths, true)
        )),
        Holder::Starting => Err(format!(
            "the daemon of key {locator} is being started or reached (a shim holds its start \
             lock); it was not asked; try again"
        )),
    }
}

/// Why the daemon at the key's socket could not be asked.
fn not_asked(paths: &DaemonPaths, locator: &str, ended: Exchanged) -> String {
    let why = match ended {
        Exchanged::Busy { max_clients } => {
            format!("it is at its bound of {max_clients} clients; try again")
        }
        Exchanged::NoReply(reason) => format!("it does not answer ({reason})"),
        Exchanged::Refused(reason) => reason.to_string(),
        Exchanged::Status(_) | Exchanged::Retiring | Exchanged::NotListening(_) => {
            "it answered out of turn".to_string()
        }
    };
    format!(
        "the daemon of key {locator} was not shut down: {why}; pid file {}",
        pid_file(paths, true)
    )
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/t6.log" 2>&1
```

Expected: `test result: ok. 630 passed; 0 failed` (619 + 11), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/shim src/mcp/mod.rs
git commit -m "feat(mcp): prism-mcp daemon-shutdown, graceful and instance-bound"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::shim::shutdown::`, see the named test fail, then `git checkout -- src/mcp/shim/shutdown.rs`.

| Mutant | Edit | Fails |
|---|---|---|
| 6A | `if let Some((pid, start_time)) = options.instance {` → `if let Some((pid, start_time)) = options.instance.filter(|_| false) {` | `another_instance_named_by_pid_and_start_time_is_left_alone` |
| 6B | above `let key_dir = KeyDir::open(…)`, insert `return Ok(format!("{} drained", named(&found)));` (success without the wait) | `a_daemon_that_does_not_drain_by_the_timeout_is_reported` |
| 6C | `if found.identity != key {` → `if false {` | `a_daemon_of_another_identity_is_not_asked` |
| 6D | `if accepted.instance != found {` → `if false {` | `a_daemon_replaced_between_the_exchanges_is_left_alone` |
| R4 | at the first exchange, `Exchanged::NotListening(error) => { return not_listening(…) }` → `Exchanged::NotListening(_) => return Ok(nothing()),` (review round 1, Opus W1) | `a_live_daemon_that_does_not_accept_connections_is_not_taken_for_none` |
| R10 | at the second exchange, `Exchanged::NotListening(error) => { return not_listening(…, format!("{} is gone", …)) }` → `Exchanged::NotListening(_) => return Ok(format!("{} is gone", named(&found))),` (review round 2, Opus S1: his probe showed it survived every test before) | `a_refused_second_exchange_is_judged_by_the_owner_lock_too` |
| R9 | at the second exchange, a lost reply through `not_asked`: `Exchanged::NoReply(reason) => return Err(maybe_recorded(…)),` → `… Err(not_asked(&paths, &locator, Exchanged::NoReply(reason))),` (review round 1, Opus W4) | `a_lost_reply_to_the_shutdown_says_it_may_have_been_recorded` |
| R12 | in `key_files.rs`, `pid_text`: `opened => opened?,` → `opened => match opened { Ok(file) => file, Err(_) => return Ok(None) },` (every open error taken for a removed pid file, as before PR #418 review round 1, Opus S2) | `a_pid_file_that_cannot_be_read_during_the_wait_is_not_taken_for_a_drain` |

### Task 7: `prism-mcp daemon-status` and `daemon-shutdown`

**Files:**
- Modify: `src/bin/prism-mcp.rs`, `tests/mcp/main.rs`
- Create: `tests/mcp/daemon_control_process_test.rs`

**Interfaces:**
- Consumes: Tasks 5 and 6's `run_daemon_status`, `StatusOptions`, `run_daemon_shutdown`, `ShutdownOptions`, `DEFAULT_CONTROL_TIMEOUT_SECS`; `cache_mode`, `refresh_policy_from` (the binary); `daemon_process_test::{setup, Setup, Daemon, wait_until, gone}` (S4) and `shim_process_test::{ShimProcess, Daemons}` (S5a).
- Produces: `prism-mcp daemon-status [--repo X [--cache-dir D] [--refresh-policy P]] [--runtime-dir BASE] [--timeout S]` and `prism-mcp daemon-shutdown --repo X [--cache-dir D] [--refresh-policy P] [--runtime-dir BASE] [--pid N --start-time T] [--timeout S]`, each dispatched on the first word before the standalone parser (S4's D60 pattern), each exiting with the status its function returns; clap's usage errors exit 2.

- [ ] **Step 1: Write the failing tests**

Create `tests/mcp/daemon_control_process_test.rs`:

````rust
//! `prism-mcp daemon-status` and `prism-mcp daemon-shutdown` as processes, against a real daemon
//! process and a real shim (spec §4.7, §11 item 3). Each test has a runtime directory of its own.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{gone, setup, wait_until, Daemon, Setup};
use crate::shim_process_test::{Daemons, ShimProcess};
use prism::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use std::process::{Command, Output};

const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

/// `prism-mcp <verb> <args>`, with the setup's runtime base given as `--runtime-dir`.
fn operator(setup: &Setup, verb: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
        .arg(verb)
        .arg("--runtime-dir")
        .arg(setup.base.path())
        .args(args)
        .env_remove(RUNTIME_DIR_ENV)
        .output()
        .expect("run the operator command")
}

/// `prism-mcp daemon-shutdown` for the setup's configuration.
fn shutdown(setup: &Setup, extra: &[&str]) -> Output {
    let repo = setup.repo.path().to_str().expect("a UTF-8 path");
    let cache = setup.cache.path().to_str().expect("a UTF-8 path");
    let mut args = vec!["--repo", repo, "--cache-dir", cache];
    args.extend_from_slice(extra);
    operator(setup, "daemon-shutdown", &args)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn started(setup: &Setup, extra: &[&str]) -> Daemon {
    let daemon = Daemon::start(setup, extra);
    wait_until("the daemon's socket", || setup.paths.socket().exists());
    daemon
}

/// S4's interface for S5b: an operator who keeps looking does not keep an idle daemon up. With
/// probes every 100 ms the daemon still retires after its 2 s idle TTL; with a probe that
/// restarted the idle time it would never retire, and the wait below would give up at its hang
/// limit. Nothing here depends on when the first probe runs (review round 1, Opus S1: the
/// listing is asserted on a daemon with the default TTL, below).
#[test]
fn daemon_status_does_not_keep_an_idle_daemon_up() {
    let setup = setup();
    let mut daemon = started(&setup, &["--idle-ttl", "2"]);
    wait_until("the daemon to retire while it is being looked at", || {
        let output = operator(&setup, "daemon-status", &[]);
        assert!(output.status.success(), "{}", stderr(&output));
        std::thread::sleep(std::time::Duration::from_millis(100));
        daemon.exited().is_some()
    });
    let (status, log) = daemon.ended();
    assert!(status.success(), "{status:?} {log}");
    assert!(log.contains("stopped: idle"), "{log}");
}

/// Spec §4.7: `daemon-status` lists a running daemon; `daemon-shutdown` drains a daemon with no
/// client at once, waits for the drain, and the daemon exits 0 having removed its socket and pid
/// file.
#[test]
fn daemon_shutdown_drains_an_idle_daemon() {
    let setup = setup();
    let mut daemon = started(&setup, &[]);
    // Spec §4.7: `daemon-status` lists the daemon with its pid, root, key and idle TTL, found
    // through `PRISM_MCP_RUNTIME_DIR` as a shim finds it.
    let listed = Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
        .arg("daemon-status")
        .env(RUNTIME_DIR_ENV, setup.base.path())
        .output()
        .expect("run daemon-status");
    assert!(listed.status.success(), "{}", stderr(&listed));
    let said = stdout(&listed);
    let root = std::fs::canonicalize(setup.repo.path()).expect("the canonical root");
    for expected in [
        format!("{} pid {}, started ", setup.key.locator(), daemon.pid()),
        format!("serving {}\n", root.display()),
        format!("  key {}; build ", setup.key),
        "retires after 900 s idle".to_string(),
    ] {
        assert!(said.contains(&expected), "{expected:?} in {said}");
    }
    let output = shutdown(&setup, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let said = stdout(&output);
    assert!(
        said.starts_with(&format!(
            "prism-mcp daemon-shutdown: pid {} (started ",
            daemon.pid()
        )),
        "{said}"
    );
    assert!(said.ends_with(") drained\n"), "{said}");
    let (status, log) = daemon.ended();
    assert!(status.success(), "{status:?} {log}");
    assert!(
        log.contains("stopped: an operator's daemon-shutdown"),
        "{log}"
    );
    assert!(gone(setup.paths.socket()));
    assert!(gone(setup.paths.pid_file()));
}

/// Spec §4.7 and §11 item 3: `daemon-shutdown` never disconnects a client. The shim's session
/// goes on after the request, `daemon-status` shows the request, and the daemon exits once the
/// client has closed its session.
#[test]
fn daemon_shutdown_never_disconnects_a_client() {
    let setup = setup();
    let mut daemon = started(&setup, &[]);
    let _daemons = Daemons(&setup.paths);
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    assert_eq!(shim.daemon_pid(), daemon.pid());

    let output = shutdown(&setup, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("drains once its 1 admitted clients have disconnected"),
        "{}",
        stdout(&output)
    );
    let listed = shim.request(LIST);
    assert!(listed["result"]["tools"].is_array(), "{listed}");
    let status = stdout(&operator(&setup, "daemon-status", &[]));
    assert!(
        status.contains("  shutdown requested: it drains once no client is left\n"),
        "{status}"
    );
    assert!(daemon.exited().is_none(), "the daemon serves its client");

    shim.close_input();
    let (shim_status, shim_log) = shim.ended();
    assert!(shim_status.success(), "{shim_status:?} {shim_log}");
    let (status, log) = daemon.ended();
    assert!(status.success(), "{status:?} {log}");
    assert!(
        log.contains("stopped: an operator's daemon-shutdown"),
        "{log}"
    );
}

/// Spec §11 item 3: `--pid` and `--start-time` bind the request to one instance; a daemon that
/// is another instance is left alone, and the command still exits 0 (the instance it named does
/// not serve the key).
#[test]
fn daemon_shutdown_of_another_instance_leaves_the_daemon_alone() {
    let setup = setup();
    let mut daemon = started(&setup, &[]);
    let pid = daemon.pid().to_string();
    let output = shutdown(&setup, &["--pid", &pid, "--start-time", "1"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("which was left alone"),
        "{}",
        stdout(&output)
    );
    let status = stdout(&operator(&setup, "daemon-status", &[]));
    assert!(!status.contains("shutdown requested"), "{status}");
    assert!(daemon.exited().is_none());
}

/// With no daemon for the configuration there is nothing to do: exit 0, and it says so.
#[test]
fn daemon_shutdown_with_no_daemon_says_so() {
    let setup = setup();
    let output = shutdown(&setup, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("nothing to shut down"),
        "{}",
        stdout(&output)
    );
}

/// The flags: `daemon-shutdown` needs `--repo`; `--pid` and `--start-time` go together; the
/// configuration flags of `daemon-status` need `--repo`. Each is a usage error (status 2).
#[test]
fn the_operator_commands_check_their_arguments() {
    let setup = setup();
    let cache = setup.cache.path().to_str().expect("a UTF-8 path");
    for (verb, args, names) in [
        ("daemon-shutdown", vec![], "--repo"),
        (
            "daemon-shutdown",
            vec!["--repo", "/", "--pid", "1"],
            "--start-time",
        ),
        (
            "daemon-shutdown",
            vec!["--repo", "/", "--start-time", "1"],
            "--pid",
        ),
        ("daemon-status", vec!["--cache-dir", cache], "--repo"),
        ("daemon-status", vec!["--timeout", "0"], "--timeout"),
    ] {
        let output = operator(&setup, verb, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{verb} {args:?}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains(names),
            "{verb} {args:?}: {}",
            stderr(&output)
        );
    }
}
````

In `tests/mcp/main.rs`, replace this:

````rust
mod daemon_process_test;
mod golden_transcript_test;
mod mem_report_test;
````

with this:

````rust
mod daemon_control_process_test;
mod daemon_process_test;
mod golden_transcript_test;
mod mem_report_test;
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/t7-red.log" 2>&1
```

Expected in the log: `test result: FAILED. 46 passed; 6 failed`: every new test, each on the standalone parser's `error: unexpected argument 'daemon-status' found` or `error: unexpected argument 'daemon-shutdown' found`.

- [ ] **Step 3: Implement**

In `src/bin/prism-mcp.rs`, replace this:

````rust
    key: Option<String>,
}

fn refresh_policy_from(name: &str) -> prism::mcp::RefreshPolicy {
    match name {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
````

with this:

````rust
    key: Option<String>,
}

/// `prism-mcp daemon-status`: what the shared daemons in the runtime directory are doing (spec
/// §4.7). It reads and never removes anything.
#[derive(clap::Parser)]
#[command(name = "prism-mcp daemon-status")]
struct StatusCli {
    /// Show only the daemon of this repository (with the cache directory and refresh policy
    /// given, for this build of prism).
    #[arg(long)]
    repo: Option<std::path::PathBuf>,
    #[arg(long, requires = "repo")]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, requires = "repo", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: Option<String>,
    /// The runtime base, as PRISM_MCP_RUNTIME_DIR gives it: the daemons' files are in its
    /// prism-mcpd directory.
    #[arg(long)]
    runtime_dir: Option<std::path::PathBuf>,
    /// Seconds each daemon has to answer.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_CONTROL_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..=600))]
    timeout: u64,
}

/// `prism-mcp daemon-shutdown`: ask the shared daemon of a repository to drain once it has no
/// client (spec §4.7). It never disconnects a client.
#[derive(clap::Parser)]
#[command(name = "prism-mcp daemon-shutdown")]
struct ShutdownCli {
    #[arg(long)]
    repo: std::path::PathBuf,
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value = "warn-only", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: String,
    /// The runtime base, as PRISM_MCP_RUNTIME_DIR gives it.
    #[arg(long)]
    runtime_dir: Option<std::path::PathBuf>,
    /// Ask only the daemon with this pid and start time (as `prism-mcp --shared` reports them).
    #[arg(long, requires = "start_time")]
    pid: Option<u32>,
    #[arg(long, requires = "pid")]
    start_time: Option<u64>,
    /// Seconds for the whole command, the wait for the drain included.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_CONTROL_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..=600))]
    timeout: u64,
}

fn refresh_policy_from(name: &str) -> prism::mcp::RefreshPolicy {
    match name {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
````

In `src/bin/prism-mcp.rs`, replace this:

````rust
    })
}

fn main() -> anyhow::Result<()> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|first| first == "daemon")
    {
        return daemon_main();
    }
    let c = <Cli as clap::Parser>::parse();
    let cache = cache_mode(c.no_cache, c.cache_dir);
````

with this:

````rust
    })
}

/// The configuration whose daemon an operator command is about.
fn operator_config(
    repo: std::path::PathBuf,
    cache_dir: Option<std::path::PathBuf>,
    refresh_policy: &str,
) -> prism::mcp::ServerConfig {
    let mut config = prism::mcp::ServerConfig::new(repo);
    config.cache = cache_mode(false, cache_dir);
    config.refresh_policy = refresh_policy_from(refresh_policy);
    config
}

fn status_main() -> i32 {
    // The first argument is the word `daemon-status`; clap reads it as the program name.
    let c = <StatusCli as clap::Parser>::parse_from(std::env::args_os().skip(1));
    prism::mcp::run_daemon_status(&prism::mcp::StatusOptions {
        config: c.repo.map(|repo| {
            operator_config(
                repo,
                c.cache_dir,
                c.refresh_policy.as_deref().unwrap_or("warn-only"),
            )
        }),
        runtime_base: c.runtime_dir,
        timeout: std::time::Duration::from_secs(c.timeout),
    })
}

fn shutdown_main() -> i32 {
    let c = <ShutdownCli as clap::Parser>::parse_from(std::env::args_os().skip(1));
    prism::mcp::run_daemon_shutdown(&prism::mcp::ShutdownOptions {
        config: operator_config(c.repo, c.cache_dir, &c.refresh_policy),
        runtime_base: c.runtime_dir,
        instance: c.pid.zip(c.start_time),
        timeout: std::time::Duration::from_secs(c.timeout),
    })
}

fn main() -> anyhow::Result<()> {
    let first = std::env::args_os().nth(1);
    if first.as_ref().is_some_and(|first| first == "daemon") {
        return daemon_main();
    }
    if first.as_ref().is_some_and(|first| first == "daemon-status") {
        std::process::exit(status_main());
    }
    if first
        .as_ref()
        .is_some_and(|first| first == "daemon-shutdown")
    {
        std::process::exit(shutdown_main());
    }
    let c = <Cli as clap::Parser>::parse();
    let cache = cache_mode(c.no_cache, c.cache_dir);
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/t7.log" 2>&1
```

Expected: `test result: ok. 52 passed; 0 failed` (46 + 6), with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`, no `warning` line, and no `prism-mcp daemon` process left (`pgrep -fl 'prism-mcp daemon'` prints nothing).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/bin/prism-mcp.rs tests/mcp/main.rs tests/mcp/daemon_control_process_test.rs
git commit -m "feat(mcp): prism-mcp daemon-status and daemon-shutdown on the command line"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run the named process test, see it fail, then `git checkout -- <file>`. 7A takes about a minute: the daemon it keeps up is ended by the test's hang limit.

| Mutant | Edit | Test | Fails |
|---|---|---|---|
| 7A | in `src/mcp/daemon/admission.rs`, `control_exchange`: `admission.idle_since = admission.quiet_since;` → `admission.idle_since = Some(now);` | `cargo test --features mcp --test mcp daemon_control_process_test::daemon_status_does_not` | `daemon_status_does_not_keep_an_idle_daemon_up`, at its 60 s hang limit |
| 7B | in `src/bin/prism-mcp.rs`, `shutdown_main`: `instance: c.pid.zip(c.start_time),` → `instance: None,` | `cargo test --features mcp --test mcp daemon_control_process_test::daemon_shutdown_of_another_instance` | `daemon_shutdown_of_another_instance_leaves_the_daemon_alone` |

### Task 8: S5b-2 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
reconnects. `run.rs` is `run_shim`; on `READY` it writes `prism-mcp: shared daemon pid N (started
T, generation G) at <socket>` to stderr.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
2. **Graph-based** (require the CPG; source of truth `SlicingAlgorithm::needs_cpg()`, src/slice.rs:210): `left_flow`, `full_flow`, `relevant_slice`, `conditioned_slice`, `barrier_slice`, `chop`, `taint`, `delta_slice`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `gradient_slice`, `provenance_slice`, `membrane_slice`, `echo_slice`
````

with this:

````markdown
reconnects. `run.rs` is `run_shim`; on `READY` it writes `prism-mcp: shared daemon pid N (started
T, generation G) at <socket>` to stderr.

The operator's commands (`src/mcp/shim/`, clients of the same sockets and files; spec §4.7):
`prism-mcp daemon-status [--repo X [--cache-dir D] [--refresh-policy P]] [--runtime-dir BASE]
[--timeout S]` (`status.rs`) lists every key in the runtime directory: a daemon that answers the
control exchange (`control.rs`) with its status; one that does not (its connect error or why it
gave no reply), through its locks (the owner lock only under the start lock, each for a moment,
`under_locks`); what a dead daemon left, with its build marker's restart time (the marker is read
without waiting, a regular file only, 256 bytes at most: `KeyDir::read_build_marker`, which the
shim uses too); keys with nothing but lock files and logs in one count line; a key that cannot be
looked at on a line of its own. It never removes anything. `prism-mcp daemon-shutdown --repo X [--cache-dir D] [--refresh-policy P]
[--runtime-dir BASE] [--pid N --start-time T] [--timeout S]` (`shutdown.rs`) asks the instance a
status exchange has just shown (or the one named) to drain, and when it had no client waits for
its pid file to go; a socket that refuses connections is no daemon only when nobody holds the
owner lock; exit 0 when that instance no longer serves the key, or has accepted and drains after
its clients; exit 1 when it could not ask, the reply to its request was lost (the request may
have been recorded), or it did not see the drain. `--runtime-dir` is the
base, as `PRISM_MCP_RUNTIME_DIR` gives it.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
2. **Graph-based** (require the CPG; source of truth `SlicingAlgorithm::needs_cpg()`, src/slice.rs:210): `left_flow`, `full_flow`, `relevant_slice`, `conditioned_slice`, `barrier_slice`, `chop`, `taint`, `delta_slice`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `gradient_slice`, `provenance_slice`, `membrane_slice`, `echo_slice`
````

- [ ] **Step 2: Run the MCP surface and the checks**

The commands of Task 4 Step 2, with `s52` in the log names and the clippy `grep` extended to `src/mcp/shim/`, `src/mcp/mod.rs`, `src/bin/prism-mcp.rs` and `tests/mcp/(daemon_control|main)`.

Expected: `cargo fmt --check` silent; `test result: ok. 630 passed; 0 failed` (571 + 59); `52 passed` with the goldens' test `ok`; both checks `Finished`, no `warning` in either; clippy 258 `warning` lines, and the `grep` prints nothing.

- [ ] **Step 3: State the invariants**

The commands of Task 4 Step 3, except that the first lists `src/mcp/shim/{arbitrate,pump,spawn,run,socket}.rs` instead of `src/mcp/shim` and `src/bin`, and:

```bash
git diff origin/main -- src/bin/prism-mcp.rs > "$LOG_DIR/s52-bin.diff"
```

Expected: the first prints nothing; `32` (31 under `src/` and `tests/`, and `CLAUDE.md`); every size at most 546; `s52-bin.diff` adds the `StatusCli` and `ShutdownCli` structs, `operator_config`, `status_main`, `shutdown_main`, and two branches beside the `daemon` one in `main` (whose test of the first word is rewritten to share it), and nothing else.

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
fails=0
for i in $(seq 1 10); do nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::shim > "$LOG_DIR/s52-rep-$i.log" 2>&1 || fails=$((fails+1)); done
for i in $(seq 1 10); do nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp daemon_control > "$LOG_DIR/s52-proc-$i.log" 2>&1 || fails=$((fails+1)); done
echo "failed runs: $fails"
grep -h '^test result' "$LOG_DIR"/s52-rep-*.log "$LOG_DIR"/s52-proc-*.log | sed 's/finished.*//' | sort | uniq -c
pgrep -fl 'prism-mcp daemon'
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s52-full.log" 2>&1; echo "full suite rc=$?"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s52-doc.log" 2>&1; echo "doc tests rc=$?"
```

Then the Linux commands of Task 4 Step 4 with `EXTRA_SUITE="--test mcp"` exported, `s52` in the output's name and an empty `$LOG_DIR/linux`.

Expected: `failed runs: 0`, `10 test result: ok. 80 passed; 0 failed` and `10 test result: ok. 6 passed; 0 failed`; `pgrep` prints nothing; the full suite and the doc tests `rc=0`; on Linux `failed Linux runs: 10`, every failure the root-only S2a test as in Task 4, `10 test result: ok. 80 passed` (`--lib mcp::shim`), `10 test result: ok. 150 passed` (`--lib mcp::daemon`), `10 test result: ok. 52 passed` (`--test mcp`), and no `left:` line.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S5b-2 is the operator's half: `prism-mcp daemon-status` (what every daemon is doing, read without changing anything) and `prism-mcp daemon-shutdown` (one instance, drained once its clients have gone). (2) **Invariants:** Step 3. (3) **Decisions:** D101–D107, D109; SI-31, SI-33, SI-35 (the disclosed old-daemon idle restart); owner decision 1 (2026-10-16). D102 (the look at the locks) and D104 (exit statuses) are the ones a reviewer is most likely to want to change. (4) **Claims:** the rows for Tasks 5–7, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base (Linux included), and mutants 5A–7B, R3–R9 and R10 as observed. (6) **Not verified:** x86_64 Linux before CI; the bridge's reaper (S6). (7) **Tier-A:** not run (no file it watches is touched). (8) **Reviews of the plan:** review round 1's folds for Tasks 5–7 (Opus W1, W4, S1, S7, S8; sol 2, 4), and round 2's test and wording folded after the two-round cap without a third round (disclosed).

```bash
git add CLAUDE.md
git commit -m "docs(mcp): prism-mcp daemon-status and daemon-shutdown"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): prism-mcp daemon-status and daemon-shutdown (shared daemon S5b-2)" --body-file "$LOG_DIR/s52-pr-body.md"
```

Review brief and cap as in Task 4.

---

## Interfaces for the next slices

**S6 (launchers, docs, soak instrumentation) consumes:**

- **The bridge's reaper:** `prism-mcp daemon-shutdown --repo X [--cache-dir D] [--refresh-policy P] --pid N --start-time T` with the pid and start time from the shim's `prism-mcp: shared daemon pid N (started T, …)` line (S5a's D88): exit 0 when that instance no longer serves the key or drains after its clients; exit 1 otherwise. The bridge must pass the same `PRISM_MCP_RUNTIME_DIR` (or `--runtime-dir`) as its shims, and the same cache directory and refresh policy (D107).
- **The soak's resident count:** `prism-mcp daemon-status` lists every daemon with its pid, build identity, clients and idle time; keys with no daemon are one count line (owner decision 1). An S5b `daemon-status` sweep keeps pre-S5b daemons alive (SI-35): during the switch, stop them with `kill -TERM`.
- **Docs (`docs/MCP.md`):** the two commands and their exit statuses; that `--runtime-dir` is the base; that a daemon of another build is listed and stopped with `kill -TERM`, and that a daemon built before S5b restarts its idle time at each `daemon-status` probe, so polling it keeps it up (SI-35; stop it with `kill -TERM <pid>`); that exit 1 from `daemon-shutdown` can mean "alive but not accepting connections" or "the request may have been recorded" (see `daemon-status`); that `daemon-shutdown` never disconnects a client and the daemon keeps admitting until it drains (owner decision 2); that a probe does not keep a daemon of this build up.

**C2-M1** and later candidates: unchanged by this slice.

## Self-review record

- **Spec coverage (§12 row S5b, §4.7).** "for every live daemon in the runtime dir (or the one for `X`)" → `status_report` over `locators_in` or the one key (D107). "key, repo root, build identity, pid, start time" → the reply's `instance`, `repo_root`, `build_identity` (Task 1) and `describe`'s first two lines (Task 5). "admitted and pending clients" → `ControlSnapshot` (Task 2), the probe itself not counted. "build state and attempt, generation" → `BuildStatus` read in the same critical section. "idle time" → `idle_for`, from the restored `idle_since`. "whether its socket answers the preamble" → the status line, or "alive but not answering" / "being started" / "no daemon" through the locks (SI-29). "`daemon-shutdown --repo X` asks the instance verified through the live control exchange (pid + start time echoed in `READY`)" → D104, SI-32. "to enter Draining when its admitted count reaches 0" → D100, SI-30. "it never disconnects other clients" → D100 and its three tests. §4.3 "`daemon-status` prints it" (the runtime directory) → the report's first line. §4.6 "`daemon-status` lists every live daemon with its build identity" → D97 (the exchange is not tied to a build's key) and the `build` field.
- **S4's interfaces for S5b, one by one.** `status()` and `admission_status()` (D66): kept test-only; the reply reads the control plane inside `control_exchange`, so the counts, the build status and the idle time are one snapshot. `StopReason`: `Shutdown`, recorded by `retire_if_idle` (S4 suggested "a variant of `retire_if_idle` that ignores the TTL but still needs admitted == 0"; built in `retire_if_idle` itself, which is where every drain decision is taken, and it also needs pending == 0, SI-30). The files cleanup rule (D70): owner decision 1. The wire verbs and the protocol version (D63): D97. "A status probe … must not reset `idle_since`": D99.
- **S5a's interfaces for S5b, one by one.** "a status or shutdown verb is a new line after, or instead of, `HELLO`": instead of (D98). "by S2's rule a new verb bumps `SHARED_PROTOCOL_VERSION`. Decide that before S6": D97. "`key_files::KeyDir` opens a key's files the safe way; S5b's cleanup … needs the same owner-lock rule": `daemon-status` uses `KeyDir` and removes nothing (D102, owner decision 1). "a probe must not keep a daemon alive": D99. "A `daemon-shutdown` that drains ends attached shims' sessions with exit 1": it does not drain while a client is attached (D100), so no session is ended by it; a SIGTERM still does that (spec §3.1). "`daemon-status` should show a key whose daemon died during its build, when a shim first saw that, and when the minute ends": `died_building` (D102).
- **The lane README's notes.** S2b: "New verbs … bump `SHARED_PROTOCOL_VERSION` and every key … decide before S6 whether control verbs are part of the versioned preamble": decided, they are not (D97). S4: "A cleanup rule is needed (S4 or S5b)": owner decision 1. S5a: the peer check on both ends: the operator checks its peer before it sends (Task 5, mutant 5A).
- **The lane's review lessons.** (1) Every stated property has a test and a mutant, or is listed as not held: "Claims". (2) What each mechanism does not cover: its section. (3) Interleavings are played deterministically: a client that comes and goes during a probe (T2), a shim holding the start lock during a look (T5), a daemon replaced between two exchanges (T6), all without sleeping. (4) No test asserts an upper bound on a duration; the one process test that waits on a TTL bounds only by its hang limit. (5) Nothing the operator prints carries a control character (`one_line`). (6) No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests; no `unsafe`. (7) A look changes nothing (D102), and a probe changes no clock (D99): each is held by a test that fails under the mutant that would change it.
- **Size.** From `scratch/s5b-tasks` (after review round 1): S5b-1 (`5a63c7e8..7e17ba5e`) 14 paths, `src/` +1,142 −27 (production +448, tests +694; 326 production code lines), `CLAUDE.md` +14; S5b-2 (`7e17ba5e..bf8772fa`) 20 paths, `src/` and `tests/` +2,121 −20 (production +843, tests +1,278; 665 code lines), `CLAUDE.md` +17. Weighted sizes under Global Constraints.

## Validation record

Built on 2026-10-16 on macOS (arm64), in `~/code/prism-s5b-validate`, from `main` `5a63c7e8`. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file under the session's scratch directory, read from the file. Nothing was pushed. Local branches:

- `scratch/s5b-validate`, where the code was first written, test first: `488335d0` (the wire), `8c7ac076` (the control plane), `398fe1f5` (the accept loop), `5f92fb7d` (the operator's exchange, `daemon-status`, `daemon-shutdown`), `4b32ab61` (the command line and the process tests), `3e630e78` (`CLAUDE.md`), `839e0529` (clippy), `c2e9b576` (the escaping test and mutant 5F). The mutants ran on `839e0529` (5F on `c2e9b576`), each alone, the file restored after each.
- `scratch/s5b-tasks`, the same source cut into this plan's tasks, one commit per task: `bf36d3ff` (Task 1), `2bb65306` (2), `06399b40` (3), `90f6ea98` (4, the S5b-1 head), `2479cfc4` (5), `d75c21b4` (6), `c5d68cf3` (7), `c96ddf8b` (8, the S5b-2 head). Every file is taken from `c2e9b576` except two intermediates: Task 1's `preamble.rs` (`BuildState::word` still `pub(super)`, without its doc line) and Task 5's `shim/mod.rs` and `mcp/mod.rs` (without `shutdown`). `cargo fmt --check` passed at every commit; the head's tree is `c2e9b576`'s.
- `scratch/s5b-replay` (`12284c83`): this plan's directives replayed by a script onto `5a63c7e8`, every anchor matching exactly once (62 directives): its tree is `6442efde`, the tree of `c96ddf8b` and of `c2e9b576`. The red and green steps of Tasks 1, 2, 3, 5, 6 and 7 were run on partial replays in the validate worktree (detached at `5a63c7e8`, the plan replayed up to the task's Step 1, then up to its Step 3), built with `CARGO_INCREMENTAL=0`.

The tools (the generator of the directives, the replay, the cut, the mutant runner) are S5a's, with two changes: a `pub mod x;` line counts as a test-registration hunk (Task 1's red step needs `pub mod control;` before `control.rs` exists), and `control_support.rs` is a test file.

| Step | Command | Observed |
|---|---|---|
| Base | `--lib mcp::`; `--test mcp` (unblessed), at `5a63c7e8` | `571 passed`; `46 passed`, the goldens' test `ok` |
| T1–T7 red and green | as in V2 and V3 | as there |
| Mutants | the tasks' tables, each applied alone, its test command run, the file restored | 28 of 28 killed: 1A–1F, 2A–2F, 3A–3D, 5A–5F, 6A–6D on the lib tests; 7A (`finished in 61.07s`, at the process test's hang limit) and 7B on the process tests |
| V4–V12 | as in the table at the top | as there |

**Corrections made to the plan by the validation.**

| # | Where | What was first written | What was wrong | What it says now |
|---|---|---|---|---|
| 1 | Task 5, `status.rs`, and Task 6, `shutdown.rs` | `format!("… {}", format!("pid file {}", pid_file(paths, true)))` | Three clippy `format_in_format_args` warnings in files this slice adds | the words in the outer format string |
| 2 | Task 5, `status_tests.rs` | no test of the report's escaping | The "nothing the operator prints carries a control character" claim had no test: a `describe` without `one_line` survived every test | `a_root_with_control_characters_is_printed_on_one_line`; mutant 5F killed |
| 3 | Tasks 4 and 8, Step 4 | `J="nice -n 19 env CARGO_BUILD_JOBS=4"` and `$J cargo …` (S5a's form) | In zsh an unquoted variable is not split into words: every command exited 127 and the run looked like a result | each cargo command written out in full |
| 4 | Task 5, `shim/control.rs` | `Exchanged::NotListening(io::Error)` | The error was never read (a dead-code warning in the test build) | `NotListening` without a payload |

**Not run in this validation:** x86_64 Linux (CI); the mutation gate (it selects no MCP code; lane README); a release build; the Tier-A harness (no file it watches is touched); the Linux run at the S5b-1 head on its own (its code is part of the S5b-2 head, which ran there); a daemon of a build without the control exchange inside the suite (measured once by hand in review round 1, below: the suite cannot build another commit's binary); the §8 soak; the bridge's reaper (S6).

**Review round 1's fold (2026-10-16).** On `scratch/s5b-validate`: `1e1d0793`, `8cb45f94`, `a08855fb`, `fb821e47`, `7e456df8` (Review round 1). The first cut and replay are kept as `scratch/s5b-tasks-r0` (`c96ddf8b`) and `scratch/s5b-replay-r0` (`12284c83`); the new cut is `scratch/s5b-tasks`: `b5c4ee04` (Task 1), `800b9c99` (2), `9c823726` (3), `7e17ba5e` (4, the S5b-1 head), `4bf9d4ea` (5), `37008601` (6), `f686831c` (7), `bf8772fa` (8, the S5b-2 head), every file from `7e456df8` but the intermediates named above and Task 5's `key_files.rs` (without Task 6's `pid_text`). The replay `scratch/s5b-replay` `1b2d72ea` (66 directives) has tree `f09b8748`, the tree of `bf8772fa` and `7e456df8`. Red and green steps were run again on partial replays (V2, V3), and all 37 mutants on `fb821e47` (V8).

**The old-daemon measurement (Opus W2, sol 1; SI-35).** Hypothesis: a daemon built before this slice restarts its idle time at every `STATUS` probe. Predicted if true: with `--idle-ttl 3` and this slice's `daemon-status` every second for 9 s, it is alive at 9 s and retires a few seconds after the probes stop; falsified if it exits while probed. Control: the same probes against a daemon of this slice. The base build's `prism-mcp` was built in `~/code/prism-s5b-plan` (code identical to `5a63c7e8`; its `target/` removed afterwards) and driven by a script (`oldprobe.py`, kept with the session's logs). Observed: the base daemon was reported `alive but not answering (the connection closed before a reply)`, was alive after 9 s of probes, and exited 2.0 s after they stopped, status 0; its log has one `refused a preamble line: unexpected preamble verb "STATUS"` per probe. The control daemon exited 3.1 s into the probes (`stopped: idle`). Confirmed; the alternative (that this slice's own daemons are kept up the same way) is refuted by the control. No process was left.

**Corrections made by round 1's validation.**

| # | Where | What was first written | What was wrong | What it says now |
|---|---|---|---|---|
| 5 | Task 3, `a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does` | waited for `pending == 0` right after a partial line and a close | `connect` returns before the loop accepts, so `pending == 0` held before the connection was counted; its close was seen a turn later, after the clock had moved, and the test hung at its 30 s wait (found by a debug print of the idle deadline: 900 s, not 899 s, ahead) | waits for the connection to be counted (`pending == 1`) before closing it |
| 6 | Task 5, the oversized-marker test | a 4 MiB marker of `x` after its two lines | read whole or bounded, it gave the same report, so the size bound (R6) survived | its third line ends 4 MiB in: read whole it is a sighting at time 0; bounded it is none |
| 7 | the mutant runner, 7A | the old test name as its filter after the rename (Opus S1) | the first run selected no test and passed: inadmissible | the new name; killed at the hang limit |

## Review round 1

PR #412 at `45d3df39` (code `c2e9b576`). **Opus 5.5: FIX** (four WRONG, two MATERIAL; nine SMELLs). **gpt-6.1-sol: FIX** (three WRONG/MATERIAL, one WRONG/IMMATERIAL, one SMELL/MATERIAL). Both: the core mechanisms hold; the two owner defaults are right; D97 is sound. Owner decisions of 2026-10-16: both defaults (now "Owner decisions" at the top); D97 stands as a disclosed planner's decision, with Opus S3's rule and sol's corrected rationale. The controller's ruling on Opus W2 / sol 1: accept and disclose, pin, remove the universal claim.

Folded on `scratch/s5b-validate`, each code change test first: `1e1d0793` (the daemon's half; its tests were run against the unfolded code first, `test result: FAILED. 149 passed; 1 failed`, the expired-line test at its 30 s wait, and the commit holds the tests and the fix), `8cb45f94` (the operator's tests, committed red: `test result: FAILED. 71 passed; 8 failed`), `a08855fb` (the operator's fixes: `629 passed`), `fb821e47` (Opus S1; the marker's size bound pinned), `7e456df8` (`CLAUDE.md`). Re-cut on `scratch/s5b-tasks`, the directives regenerated and replayed (Validation record).

| Finding | Ruling | What changed | Test, RED before the change → GREEN after |
|---|---|---|---|
| Opus W1, WRONG/MATERIAL: `daemon-shutdown` reported "nothing to shut down" for a live daemon whose connect failed (full backlog, claimed and not bound) | folded | `Exchanged::NotListening` carries the connect error again; `daemon-shutdown` judges a refused connection by the key's locks (`not_listening` over `under_locks`, which `daemon-status` uses too): owner held → exit 1 "alive but not accepting connections"; start lock held → exit 1; free → nothing to shut down; at both exchanges. `daemon-status` prints the connect error, and "it has no socket: starting or draining" only without a socket (D104, D102) | `a_live_daemon_that_does_not_accept_connections_is_not_taken_for_none`: RED `alive: "no daemon serves this configuration of this build (key 8d129ba6e52627f7); nothing to shut down"` → GREEN. `a_live_daemon_that_does_not_accept_is_named_with_why`: RED `… alive but not answering (it has no socket yet) …` → GREEN. Mutants R3, R4 |
| sol 2 / Opus S6, WRONG/MATERIAL: the build marker's blocking, uncapped read could hang `daemon-status` holding both locks (and a shim) | folded in S5a's helper | `KeyDir::read_build_marker`: `O_NONBLOCK`, descriptor-relative, regular file only, 256 bytes at most; `build_marker` (the shim's) returns `None` for anything else, so a shim removes a FIFO with the stale files instead of waiting on it; `daemon-status` reports it unusable (D109). Shims otherwise unaffected: `--lib mcp::shim` green, the S5a marker tests unchanged | `a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report`: RED `daemon-status over a FIFO marker did not return within 30s` → GREEN; the shim's `a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key`: RED `a shim over a FIFO marker did not return within 30s` → GREEN. Both under an independent watchdog thread. Mutants R5, R6 |
| sol 3, WRONG/MATERIAL: a control line completed after its deadline went through `leave_handshake` and restarted the TTL (probe at idle 899 s → retirement moved ~900 s) | folded | `advance_handshakes`: an expired control line goes through `control_exchange(false)`: no reply, no shutdown recorded, idle time as found (D98, D99); the module doc states the treatment of incomplete and unrecognized frames | `a_control_line_completed_after_its_deadline_moves_nothing` (a `SHUTDOWN` line completing at its deadline, one second before the TTL ends): RED `gave up after 30s waiting for: the accept loop to drain` → GREEN, the loop stops for `Idle`, not `Shutdown`. Mutant R1 |
| Opus W3, WRONG (text)/IMMATERIAL: "a stalled probe … never moves the idle time" | folded | "What each mechanism covers": a connection that never completes its line, or whose line is not a control line or a `HELLO`, is a client's | `a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does` (a pin; passes before and after) |
| Opus W2 / sol 1, WRONG/MATERIAL: an S5b `STATUS` probe restarts the idle clock of a daemon built before S5b | accepted and disclosed (controller) | SI-35; the old-build bullet of "What each mechanism covers"; the goal's universal claim narrowed to daemons of this build; the S6 docs item (`kill -TERM`); the pin above (the same mechanism in this build); a one-off measurement against a base-build daemon process | Measured (Validation record): the base build's daemon with `--idle-ttl 3`, probed by this slice's `daemon-status` every second, alive after 9 s, retired 2 s after the probes stopped; its log has `refused a preamble line: unexpected preamble verb "STATUS"` per probe. Control, this slice's daemon under the same probes: retired at 3.1 s |
| Opus W4, WRONG (message)/IMMATERIAL: a lost reply to `SHUTDOWN` said "was not shut down" | folded | `maybe_recorded`: "was sent the request and no reply came back (…); it may have been recorded, and a request cannot be withdrawn: see daemon-status"; `BUSY` still "not shut down" (answered before the line is read) | `a_lost_reply_to_the_shutdown_says_it_may_have_been_recorded`: RED `… was not shut down: it does not answer (the connection closed before a reply) …` → GREEN. Mutant R9 |
| sol 4, WRONG/IMMATERIAL: the runtime-directory header was not escaped | folded | `one_line` on the header | `a_runtime_directory_with_control_characters_is_printed_on_one_line`: RED (a raw newline in the header) → GREEN. Mutant R7 |
| sol 5, SMELL/MATERIAL: instance uniqueness is an assumption | stated | "What each mechanism covers", D104: (identity, pid, start second); same-pid same-second replacement indistinguishable; no token (it would change `READY`, the pid file and the reaper's selector) | — |
| Opus S1: the 2 s-TTL process test asserted the listing before retirement (host-speed dependent) | folded | the listing is asserted on the default-TTL daemon of `daemon_shutdown_drains_an_idle_daemon`; the 2 s test (renamed `daemon_status_does_not_keep_an_idle_daemon_up`) asserts only exit 0 per probe and the idle retirement | 7A still kills it |
| Opus S2: "records nothing" for `RETIRING` was not asserted | folded | the test asserts the shutdown flag | mutant R2 (the record moved above the drain check), killed |
| Opus S3: no rule for the next control version | folded | D97's versioning rule | — |
| Opus S4: the consequence of a probe counting against `--max-clients` | folded | "What each mechanism covers" | — |
| Opus S5: D102's premise was wrong | folded | D102's reason restated (an owner lock taken in a race leaves no owner, never two; the look happens only where a starting shim has already removed the files) and its two edges named | — |
| Opus S7: one bad key aborted the whole report | folded | a key's error is a line of its own | `a_key_that_cannot_be_looked_at_does_not_hide_the_others`: RED `a report: "cannot open …/1111111111111111.start: Too many levels of symbolic links (os error 62)"` → GREEN. Mutant R8 |
| Opus S8: wording ("N other keys", "no socket yet") | folded | "N keys have nothing but lock files and logs (…)"; "it has no socket: starting or draining" | `keys_with_only_lock_files_and_logs_are_counted_not_listed` updated (RED on the old wording, GREEN after) |
| Opus S9: N × timeout over wedged daemons | already disclosed | — | — |

Convergence: round 1 of the two-round cap. Every finding is closed and bounded; none needed a re-plan. The fold adds about 230 lines of code and tests.

## Review round 2 (the cap), folded after it without a third round

PR #412 at `cb45d047` (code `7e456df8`). **Opus 5.5: APPROVE** (two WRONG, both plan text and IMMATERIAL; three SMELLs, one MATERIAL: a claimed behavior with no test). **gpt-6.1-sol: APPROVE** (one SMELL, IMMATERIAL). Both: every round-1 finding is closed. The controller had these small items folded after the two-round cap without a third round; disclosed here and in the PR body. One test and wording; no production code changes (the code changes are a comment in `server.rs` and the module doc of `status.rs`).

On `scratch/s5b-validate`: `8c297de9` (the test, the FIFO test's fixture, the comments), `5dd1d799` (`CLAUDE.md` wording). Re-cut on `scratch/s5b-tasks` (`074117f4`; round 1's cut kept as `scratch/s5b-tasks-r1`, `bf8772fa`), the directives regenerated and replayed (`scratch/s5b-replay` `bf5a7b23`, tree `1bf5f7fb`; round 1's kept as `scratch/s5b-replay-r1`).

| Finding | What changed | Test |
|---|---|---|
| Opus W1, WRONG (text)/IMMATERIAL: the claims row, D99 and a `server.rs` comment said a control line "completed too late" never moves the idle time, but the deadline is checked before each read, so a line that arrives after the deadline is a client's | "whose read completes across its deadline" in the claims rows, D98, D99, the probe bullet, the comment and `CLAUDE.md`; the single-read window is stated. The optional last read at the deadline was not added (controller) | — |
| Opus W2, WRONG (text)/IMMATERIAL: a claimed-not-bound daemon on a clean key gets exit 0 without a look; a daemon mid-drain (socket gone, pid file and owner lock there) gets exit 1, against D104's "draining: exit 0" | both windows stated under "What each mechanism covers" and in D104, and that (b) errs safe; the W1 test's doc narrowed. No code change (controller) | — |
| Opus S1, SMELL/MATERIAL: the second exchange's refusal path had no test (his mutant survived every test) | `a_refused_second_exchange_is_judged_by_the_owner_lock_too`: the stand-in answers the status and removes its socket; with the owner lock held, exit 1 "alive but not accepting connections"; free, exit 0 "is gone" | RED under mutant R10 (the branch replaced by an unconditional `Ok("… is gone")`): `test result: FAILED. 10 passed; 1 failed`, this test; GREEN on the code: `--lib mcp::shim` 80 passed |
| sol, SMELL/IMMATERIAL: the FIFO test's removal assertion ran after its fixture was dropped | the worker returns the runtime directory with the pid, alive through the assertion | — |
| Opus S2: `status.rs`'s module doc stated D102's old premise | reworded to D102's restated reason | — |
| Opus S3: the PR-body list order; D109 before D108 | (1)–(8) in order; D108 then D109 | — |

Gates of this fold (macOS; Linux not run: test, comment and documentation changes only, no shared helper changed): `cargo fmt --check` silent; `--lib mcp::` 630 passed; `--test mcp` 52 passed, the goldens' test `ok`; clippy 258 lines, none in a touched file; mutant R10 killed; red and green steps run again on partial replays (V2, V3); the replay byte-identical. The 37 earlier mutants were not re-run: no line they edit changed.

## PR #418 review round 1 (gpt-6.1-sol FIX, Opus 5.5 APPROVE), folded

Each behaviour change was folded test first, the test watched failing on the code before it.

| Finding | Ruling | Change | Evidence |
|---|---|---|---|
| sol 1, WRONG/MATERIAL, and Opus S2 (the same class): a lookup error taken for absence | folded | `runtime_dir_exists` (`status.rs`), used by both commands: only `NotFound` is absence; any other `lstat` error is returned, escaped by `one_line`, exit 1. `KeyDir::pid_text` returns `Result<Option<String>, FilesError>`, `NotFound` as `Ok(None)`; the drain wait keeps polling on an error and reports it at the timeout, exit 1 (D104) | `a_runtime_directory_that_cannot_be_looked_at_is_not_taken_for_none` (T5 and T6; an owner lock held, a pid file, the parent mode 0; skipped under root): RED `"… no daemon (the runtime directory does not exist)"` and `"… nothing to shut down"` → GREEN. `a_pid_file_that_cannot_be_read_during_the_wait_is_not_taken_for_a_drain` (a symlink in the pid file's place, `ELOOP`; runs as root): RED `"pid 4242 (started 1760600000) drained"` → GREEN. Mutants R11, R12 |
| sol 2 / Opus W1, WRONG/IMMATERIAL (test): the countdown assertion crosses a second boundary | folded | the countdown asserted through `died_building` (`seen` 1000, `now` 1021 → `39 s`); the report-level assertion keeps the text before the number | RED: the old test with the report's clock one second later (`seen = now − 22`) printed `restarts it in 38 s` and failed; the new test passes under the same shift. Mutants 5C, 5D still killed |
| Opus S1, SMELL/IMMATERIAL (tests): short deadlines on paths that must not expire | folded | `BUSY` and `Close` get `HANG`, only `Silent` keeps 300 ms; the no-drain test 2 s | no failing run was observed (a timing SMELL); 6B still killed |
| Opus S3, SMELL/IMMATERIAL: `admitted == 0`, `pending > 0` exits 1 at the timeout | documented, no code change | D104 and `shutdown.rs`'s module doc: the wait runs to `--timeout`, exit 1; a retry converges to exit 0 | — |
| Opus S4, SMELL/IMMATERIAL: `one_line` escapes only `Cc` | deferred (predates S5b) | recorded in "Stated and not held" as a follow-up | — |

The other lookups of a key's files (`look`'s and `has_daemon_files`'s `exists`, `dead`'s leftovers) still read any `lstat` error as "not there". They run only after the runtime directory has been found and validated (0700, this user's), so an `EACCES` there needs the mode of a directory on its path to change during the look; not changed (no failure shown). **Superseded by round 2:** sol reproduced exactly that change of mode, and they now go through `present` (below).

## PR #418 review round 2 (the cap: Opus 5.5 APPROVE, gpt-6.1-sol FIX), folded after it without a third round

Disclosed: the cap was two rounds, and this fold was not reviewed again. Each behaviour change was folded test first.

| Finding | Ruling | Change | Evidence |
|---|---|---|---|
| sol 1, WRONG/MATERIAL (Opus S1, SMELL, the same gap round 1 disclosed): the per-key lookups took any `lstat` error for absence. sol reproduced it with the CLI: after validation and listing, with the parent's search permission removed and an owner lock held, `daemon-status` exited 0 and counted the live key among "nothing but lock files and logs" | folded | `present` (`status.rs`): the one existence check, with `runtime_dir_exists`'s rule; used by `look`, `has_daemon_files` (now `Result<bool, String>`), `dead` (now `Result<String, String>`) and `runtime_dir_exists`. `daemon-status` prints the key's existing "cannot be looked at" line (escaped) and exits 1 with "N keys could not be looked at" on standard error; `daemon-shutdown`'s `not_listening` exits 1, "could not be looked at (…); it was not asked" | `a_key_that_cannot_be_looked_at_after_the_listing_is_not_counted_as_empty` (a stand-in daemon on an earlier key removes the parent's search permission while it answers): RED `"…2 keys have nothing but lock files and logs (0 bytes of logs)"` → GREEN. `a_key_that_cannot_be_looked_at_after_the_status_is_not_taken_for_gone` (the status exchange removes it before the `SHUTDOWN` connects): RED `"pid 4242 (started 1760600000) is gone"` → GREEN. Both run a control with the permission kept first, as root too; the permission part is skipped under root. `a_report_with_a_key_that_cannot_be_looked_at_exits_1`: RED `left: 0, right: 1` → GREEN. Mutants R13, R14, R15 |
| Opus S2, SMELL/IMMATERIAL: round 1 made a runtime base through a regular file exit 1 "Not a directory", where it had been "no daemon", exit 0 | folded, as offered | `present` treats `NotADirectory` like `NotFound`: `ENOTDIR` from `lstat` means a component of the path is not a directory, so nothing can be at it; a reaper is not told to retry a path that can never hold a daemon | `a_runtime_base_that_is_a_file_has_no_daemon`, `a_runtime_base_that_is_a_file_has_nothing_to_shut_down`: RED `"cannot look at …/prism-mcpd: Not a directory (os error 20)"` → GREEN. Mutant R16 |

Still read with any error as "nothing": the size of a counted key's log (`status_lines`'s byte count, a number in the count line). `KeyDir::pid_text` keeps `NotFound` alone as absence (an `ENOTDIR` there needs the validated runtime directory replaced by a file during the wait, and errs toward exit 1).
