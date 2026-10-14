# S4 — Daemon Ownership, Bounded Handshake Stage, Admission and Retirement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-14).** Every task of this plan was built with a compiler before the plan was written, on the local scratch branch `scratch/s4-validate` (worktree `~/code/prism-s4-validate`) from `main` `36c9aaf5`, test first, and every mutant named in a task was applied there and killed (two survived at first; the tests were strengthened, see the Validation record). The source was then cut into this plan's tasks on `scratch/s4-tasks4`, the plan's directives were generated from that branch's per-task commits, and the plan was replayed by a script onto `36c9aaf5` (`scratch/s4-replay3`, `bb35b152`), each anchor matching once, to a tree byte-identical with `scratch/s4-tasks4` (`0289a19b`); each task's red and green steps below were run on the replay. Counts, red steps and mutant results below are the observed ones. Nothing ran on Linux (the Linux-only FFI was type-checked with `cargo check --target x86_64-unknown-linux-gnu` in a stand-alone crate), and the mutation gate was not run (it selects no MCP code).

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file that was then read.

| # | Command | Must show | Observed |
|---|---|---|---|
| V1 | `cargo test --features mcp --lib mcp::daemon`, `--lib mcp::`, and `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` on the untouched base | The base totals; every later total is "base + N". | `83 passed` (S3b's count); `469 passed`; `23 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`, at `36c9aaf5`. |
| V2 | Each task's red step, on the replay | What the step says. | Task 1: `error[E0583]: file not found for module `admission``. Task 2: seven compile errors (`E0599` for `joined_at_startup` and `budget_for`, `E0433` for `StartupMode`). Task 3: `E0583` for `files`. Task 5: `E0583` for `log`, `peer`, `server`. Task 6: `test result: FAILED. 0 passed; 7 failed` (`error: unexpected argument 'daemon' found`). Task 2's behavioral red, the parity probe against S4-1's Task 1 code: the daemon's second line `warming` where the in-process server's is `build_failed`. |
| V3 | Each task's green step, on the replay | The count the step states. | Task 1: 92. Task 2: 96. Task 3: 105. Task 5: 121. Task 6: `--test mcp` 30, `--lib mcp::daemon` 121, `cargo check --features mcp` with 0 warnings. No warning in any test build. |
| V4 | `cargo test --features mcp --lib mcp::` at the end of each part | base + 22, then base + 38; `0 failed`. | `491 passed` at the S4-1 head; `507 passed` at the S4-2 head; `0 failed` both. |
| V5 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `23 passed`, then `30 passed`, the goldens unblessed. | `23 passed` at the S4-1 head; `30 passed` at the S4-2 head, 10 runs in a row, the goldens' test `ok` each time. |
| V6 | `cargo check`, `cargo check --features mcp`, `cargo check --tests --features mcp,detached-owner-audit` | `Finished`; no warning from the feature build. | `Finished` at both heads; 0 warnings from the feature build (the module allowance removed at S4-2); the owner-audit build `Finished`. |
| V7 | `cargo clippy --all-targets --features mcp`, filtered to the touched files | No line. | 257 `warning` lines at the base, at the S4-1 head and at the S4-2 head; none naming `src/mcp/daemon/`, `src/bin/prism-mcp.rs` or `tests/mcp/daemon_process_test.rs`. |
| V8 | Every "prove the tests bite" step | The named tests fail under the named mutant. | All 21 mutants killed on the final code (1A–1F, 2A–2B, 3A–3E, 5A–5G, 6A–6C). 1D and 3C survived against the tests as first written; the tests were strengthened (Validation record). |
| V9 | Repeats: 20 × `--lib mcp::daemon` (default threads) at each head, 10 × with `--test-threads=1`, and the process tests | All `ok`; no daemon process left. | S4-1 head: `20 test result: ok. 105 passed`. S4-2 head: `20 … ok. 121 passed`, `10 … ok. 121 passed`, `--test mcp` `10 … ok. 30 passed`; `pgrep -fl 'prism-mcp daemon'` empty afterwards. Two flakes of one process test (one in a replay run, one in these repeats) were traced and fixed in the test (Validation record, corrections 3 and 4). |
| V10 | The zero-diff checks of Tasks 4 and 7 | Nothing in the standalone files; only the listed paths changed. | `git diff --name-only 36c9aaf5 0289a19b`: 23 paths: 18 under `src/mcp/daemon/`, and `src/mcp/mod.rs`, `src/bin/prism-mcp.rs`, `tests/mcp/main.rs`, `tests/mcp/daemon_process_test.rs` and `CLAUDE.md`. |
| V11 | CI on Linux | Green. | **Not run.** The Linux-only FFI (`SO_PEERCRED` and `ucred`, `openat`, `fstatat`, `unlinkat`, `poll`, `signal`) was type-checked with `cargo check --target x86_64-unknown-linux-gnu` in a stand-alone crate holding those functions. |
| V12 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp` and `cargo test --doc --features mcp` at the S4-2 head | `0 failed`. | 31 `test result` lines, **5,869 passed, 0 failed, 1 ignored**; doc tests `2 passed`. (Run at `804eade9`, which differs from the final head only in the process tests' `Shim::hello`, then 10 × `--test mcp` at the final head.) |

## Owner questions

**None that block this plan.** Every point below follows the spec's text or a lane decision, and each is recorded with its cost under "Decisions" or "Spec issues". Three are listed here because the owner may want to see them before S5a, with the default this plan builds:

1. **A builder that never returns keeps its daemon up for ever (SI-15).** Spec §4.2 says retirement needs "no build in progress"; a hung builder (a FIFO in the repository, a dead network mount) is "in progress" until the process is killed. **Default built:** the spec's rule, unchanged. The cost is one resident daemon per such key until `daemon-shutdown` (S5b) or a kill; a bound would let the daemon retire under a hung build (safe, because retirement ends the process), and is one line in `retire_if_idle` if the owner wants it.
2. **Admitted clients see EOF when a forced drain ends the process, not before (D52).** Spec §3.1 sends a lost root and SIGTERM "through Draining" although clients are attached. **Default built:** the drain answers everything pending or queued `RETIRING`, removes the files, releases the owner lock, and the process exits, which closes every admitted connection; the loop does not shut admitted sockets itself. A client in the middle of a call gets no response to it, exactly as when an in-process server is killed.
3. **`PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` (SI-17).** It changes which call-edge sidecar loads and is in neither the key nor the preamble, so a daemon inherits it from whichever shim starts it (spec §7 says every answer-affecting setting is in one of the two). **Default:** nothing in S4 (the daemon does not choose its environment); S5a's spawn removes it, or the key includes it. A design point for the S5a plan, recorded so it is not lost.

---

**Goal:** the lane's: let the a2a-bridge run its reviewer and implementer fan-out on this host without swap exhaustion, by holding one resident index per compatible (checkout, prism build, cache configuration, refresh policy), with no query-latency regression under fan-out. **What S4 must make true, and test:** exactly one daemon owns a key at a time, and a second one leaves without touching the owner's files (spec §3.1 "Starting", §4.6; tests 3, 4, 8 and 15, the daemon's halves); a connection is admitted only through a bounded handshake stage that counts it against `--max-clients` before it has sent anything and answers it `READY`, `MISMATCH`, `BUSY` or `RETIRING` (§4.2, §4.4; test 16); only a peer of the same user is answered at all (§4.4, §7, owner decision §11 item 6); an idle daemon retires after its TTL and a daemon whose root was replaced, or that is told to stop, drains, in one critical section that no admission can interleave with (§3.1, §4.2; tests 2 and 13, the daemon's halves); and a warm daemon reports a failed startup build as the in-process server does (S3a's parity requirement). The in-process server does not change.

**Architecture:** The admission counts, the idle clock and the drain decision live beside the build phase in `ControlState`, changed only by new functions in `daemon/admission.rs`, each of which takes the control lock through `DaemonState::control_plane()`: a `ControlPlane` value that holds the lock and the control plane's `FatalOnUnwind` guard, declared so that the guard drops first (S3b's D47, now carried by a type). `daemon/files.rs` owns a key's files: it validates the runtime directory, opens it once, takes `<key16>.owner` with `flock(LOCK_EX|LOCK_NB)` through `File::try_lock`, binds the socket (replacing a stale one, which only the owner may), writes and removes the pid file, all but the bind through the directory's descriptor. `daemon/server.rs` is the accept loop: one thread polls the listener, a wake socket and every pending connection; it checks each peer's uid, reads each `HELLO` from a non-blocking socket against that connection's own deadline, admits or refuses it, starts a worker thread per admitted connection (`serve_admitted`, S3a), checks the root and its own files, and drains. `daemon/run.rs` is `prism-mcp daemon`: resolve the identity, own the key or exit 0 silently, bind, write the pid file, serve, drain.

**Tech Stack:** Rust 2021 (rustc 1.94), `std::fs::File::try_lock` (`flock`), `std::os::unix::net::{UnixListener, UnixStream}`, `libc` (already an optional dependency of the `mcp` feature: `openat`, `fstatat`, `unlinkat`, `poll`, `getpeereid` / `SO_PEERCRED`, `signal`), `clap` for the subcommand, `tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§3.1, §4.2 "Per connection", "Admission and retirement", "Refusal", §4.3, §4.4, §4.6, §7, §8 step 4, §9 tests 2, 3, 4, 8, 13, 15, 16, §10, §11 items 6 and 7, §12 row S4, §13). Lane plan: `README.md` in this directory (the notes for S4, and for S4 and S5a; the owner decisions). Built on S3b as merged: `main` `36c9aaf5`. Decisions are numbered on from the S3b plan's (D1–D47) and spec issues from its SI-1–SI-13, so that one number names one thing across the lane.

**Ships as two pull requests.** **S4-1 = Tasks 1–4** (admission and retirement in the control plane, the warm daemon's parity, the key's files): 11 files in `src/`, 1,488 insertions and 16 deletions, of which about 760 are code (`files.rs` 415, `admission.rs` 207, `control.rs`, `coordinator.rs`, `budget.rs`, `connection.rs`) and the rest tests; plus 13 lines of `CLAUDE.md`. **S4-2 = Tasks 5–7** (the accept loop and handshake stage, `prism-mcp daemon`, process tests): 15 files in `src/` and `tests/`, 2,037 insertions and 18 deletions, of which about 940 are code (`server.rs` 521, `run.rs` 139, `peer.rs` 91 and `log.rs` 89 with their small test modules, the binary +70 −10) and the rest tests and test support; plus 14 lines of `CLAUDE.md`. They are split where a reviewer's question changes: S4-1 is state and files, testable without a socket server; S4-2 is the I/O loop and the process, and depends on every interface of S4-1. Each is about the size of S3a-1 or S3a-2 (about 2,100 lines each), which each converged in two review rounds. S4-2 is based on S4-1; nothing in S4-1 depends on S4-2.

## Global Constraints

- All production code is under `--features mcp`, in `src/mcp/daemon/`, `src/mcp/mod.rs` (one re-export) and `src/bin/prism-mcp.rs` (the `daemon` subcommand); tests in `src/mcp/daemon/` and `tests/mcp/`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged, byte for byte.** Not edited: `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `src/mcp/shared/*`, the golden transcripts under `tests/fixtures/mcp/golden/` (never set `PRISM_MCP_BLESS_GOLDEN`). `src/bin/prism-mcp.rs` changes only by a branch taken when the first argument is the word `daemon`, which the standalone parser rejects today, and by two helpers its existing parsing now calls with the same results. Tasks 4 and 7 check it.
- The in-process server keeps `LazySessionProvider` and the `SessionRuntime` trait unchanged (spec §4.2).
- Do not re-decide D1–D47. Two earlier notes are settled on purpose and say so: S3a's "S4 requirement: parity for a failed startup build under `Background`" (D59) and S3b's "the accept loop is outside both domains … S4 decides" (D69).
- Spec values, copied exactly:
  - "Starting: validate the runtime directory (§4.3), take the owner lock (`<key>.owner`, `flock(LOCK_EX|LOCK_NB)`; failure = another live daemon owns this key → exit 0 silently), bind the socket, write `<key>.pid` (`pid\nstart_time\nbuild_identity\n`), enter the accept loop. The owner lock is held for the daemon's whole life" (§3.1).
  - "Draining: entered only from the admission-and-retirement critical section (§4.2) when admitted == 0 and pending == 0 for the TTL and no build is running. Under that same lock the daemon stops accepting, unlinks its socket while still holding the owner lock, and answers `RETIRING` to any handshake that races in before the unlink … Then the pid file is removed, the owner lock released, exit 0" (§3.1). "Loss or change of the root's filesystem identity (§4.3) and SIGTERM/SIGINT go through Draining" (§3.1).
  - "accepted sockets enter a bounded handshake stage (non-blocking, preamble line ≤ 4 KiB, preamble timeout); a worker thread, `UnixStreamTransport` …, `Lifecycle` and `ConnectionSettings` are allocated only after admission. `--max-clients` (default 64) bounds pending + admitted; beyond it, new connections are answered `BUSY` without allocating a worker" (§4.2).
  - "Admission increments the count before `READY` is written and decrements it if the write fails. Retirement requires admitted == 0 and pending == 0 for `--idle-ttl` (default 15 min …) and no build in progress; it stops accepting and unlinks the socket inside the same critical section, so an admission and a retirement cannot interleave, and any handshake that was pending is answered `RETIRING` before exit" (§4.2).
  - "The daemon re-checks its root's `(st_dev, st_ino)` on every admission and on the TTL tick; loss or change enters Draining — existence of the pathname alone is not enough" (§4.3). "Lock files are created 0600 and never unlinked … The log is truncated at daemon start when over 1 MB and additionally capped during the daemon's life (the writer stops appending past 8 MB and notes the cap once)" (§4.3).
  - "The daemon checks the peer's uid (`getpeereid`) equals its own and drops the connection otherwise" (§4.4); §10: "Peer uid mismatch → Daemon → Connection dropped before the preamble reply".
  - Flags: "`--idle-ttl <SECS>` (default 900) … `--max-clients <N>` (default 64), `--warm-at-startup` (passed by the starting shim on the daemon's command line …)" (§8 step 4).
  - Tests 2, 3, 4, 8, 13, 15, 16 as worded in §9 (quoted in the tasks that hold their daemon halves).
- **Every new control-plane entry takes the lock through `DaemonState::control_plane()`** (S3b's interface, D47 → D48). Nothing in the accept loop takes the provider slot.
- **No panic on a path a client can reach** in code this slice adds: no `unwrap`, `expect`, indexing or unchecked arithmetic outside tests. Every `unsafe` block has a `SAFETY` comment and is one FFI call.
- **Tests do not depend on the host's speed.** An in-crate test waits for a condition up to `HANG` = 30 s (60 s in the process tests) and asserts no upper bound on an elapsed time. No test sleeps to order two events: the loop runs on a manual clock and is woken; a test that needs the loop to have looked waits for two more of its turns to begin (`Running::settle`), or holds it at the top of a turn (`Running::hold`).
- **No test aborts the test binary**, and **every process a test starts is killed and reaped on every path** (`Daemon`'s `Drop`, as S3b's `DaemonProcess`).
- Every runtime directory a test uses is a fresh `tempfile::Builder::new().prefix("s4").tempdir_in("/tmp")`: the socket path must fit 103 bytes (S2's note for S4).
- Keep every file under 600 lines; tests in sibling `*_tests.rs` files included with `#[path]`. The largest file this slice edits is `coordinator.rs`, 592 lines after it; the largest it creates is `server.rs`, 521.
- No new clippy warning in a file this slice adds or edits.
- **Commit before any experiment.** The "prove the tests bite" steps edit a source file and restore it with `git checkout -- <file>`; they come after the task's commit.
- Logs go to files in a directory of the implementer's own, `$LOG_DIR` below; read the file. Do not pipe cargo output into `grep` or `tail`. Every cargo command runs alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …` (this host has 24 GB and is usually in swap).

### What each mechanism covers, and what it does not

- **The owner lock decides who owns a key, within one host and one runtime directory.** Two daemons that resolve different runtime directories (different `PRISM_MCP_RUNTIME_DIR`, `$TMPDIR` or `$XDG_RUNTIME_DIR`) both own "the" key; that is spec §4.3's "every launcher that should share a daemon must resolve the same directory".
- **The files check notices a replaced runtime directory, not a renamed one under the same inode.** `still_ours` compares the inodes the paths name with the ones the daemon holds; a directory that is removed or replaced fails it within 5 s and the daemon drains, so a second daemon that took the key in the new directory is alone after that. A volume mounted to ignore ownership is not examined (S2's SI-5).
- **The root check is by device and inode.** A root deleted and recreated on a filesystem that reuses inodes (ext4, tmpfs) keeps its key and passes (lane README); only the freshness machinery notices.
- **The handshake deadline bounds a pending slot, not a slow admitted client.** After `READY` a connection is a worker thread that lives as long as its client; spec §4.2's `--max-clients` bounds those too, but nothing times them out (as in process).
- **A forced drain does not wait for admitted connections.** The process ends right after the drain returns; a call in progress gets no response (D52).
- **Retirement can be up to 5 s late** when the idle TTL passed while a build ran: the loop re-checks every `IDENTITY_CHECK_INTERVAL`, not when the build ends.
- **`poll` is level-triggered and the loop is single-threaded.** An accept error other than `WouldBlock` (a full descriptor table) is logged and retried on the next turn; a listener that keeps failing makes the loop spin and log until it stops (bounded by the log's 8 MiB cap). Not tested.
- **The peer check is the guarantee between users** (lane README, S2a review record); the runtime directory checks and `O_NOFOLLOW` are hardening. Neither is a defense against a process of the same user (spec §2) or root.

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously, or an earlier slice's note this slice was told to settle. "Cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D48 | **Every control-plane entry S4 adds takes the lock through `DaemonState::control_plane()`**, which returns a `ControlPlane` holding the `FatalOnUnwind` guard and the `MutexGuard`, the guard declared first so that it drops first. | S3b's D47 says the guard must be made after the lock and drop before it; a type carries that for every caller instead of each caller's line order (the lane README's "a convention the types do not carry"). Mutant 1D (fields swapped) is killed by `a_panic_inside_the_retirement_decision_ends_the_daemon_with_the_lock_held`. | None: S3a's `ensure_ready` and `status` keep their own two lines; `status` now goes through it too. |
| D49 | The admission state is a struct `Admission { pending, admitted, idle_since, draining }` inside `ControlState` (`control.rs`); the functions that change it are an `impl DaemonState` block in a new `admission.rs`. | Spec §4.2: one critical section, one mutex. `coordinator.rs` is at 592 lines; its private fields stay private (`control_plane()` and `now()` are its new accessors). | None. |
| D50 | **The idle time counts from the moment pending and admitted both reach zero, or, for a daemon nobody has reached, from the first retirement decision** (the loop's first turn). A running build holds retirement off but does not reset the idle time. | "admitted == 0 and pending == 0 for the TTL and no build is running" (§3.1) does not say when "for the TTL" starts. A daemon whose starter died before connecting must still retire. Counting from the last connection, not the last admission, keeps a long session from retiring the daemon the moment it ends. | A build that ends after the TTL retires the daemon at once (within 5 s): a client that comes back pays a cache load, not a build. |
| D51 | **The decision is a flag set once (`draining`), under the lock, and the socket's unlink runs inside the same critical section** (a closure the caller passes). `admit` and `enter_handshake` refuse once the flag is set. | The flag is what makes "an admission and a retirement cannot interleave" hold; the unlink under the lock is the spec's wording, and it costs one `unlinkat` under the control lock (S3a's lock is already held while a thread is spawned). Test `the_socket_is_unlinked_inside_the_critical_section_and_once`; mutant 1C. | One line to move the unlink after the lock. |
| D52 | **A forced drain (root lost or replaced, files replaced, SIGTERM/SIGINT) drains whatever the counts**: the same critical section records the reason and unlinks; pending and queued connections are answered `RETIRING`; the pid file is removed and the owner lock released; the process then exits, which ends every admitted connection (their clients see EOF, spec §4.6). The loop does not shut admitted sockets down itself. | Spec §3.1 routes these through Draining though clients are attached; the process's exit is what closes them, a moment later, and keeps the loop free of a registry of admitted sockets. In-crate tests cannot see the EOF (the process does not exit); the SIGTERM process test does. | A registry of admitted streams and a `shutdown` each (about 30 lines) if a client must see EOF before the pid file goes. |
| D53 | **A lock file is created with `O_CREAT|O_EXCL` and, if it exists, opened without `O_CREAT`.** | Found in validation: on macOS (APFS) a plain `openat(O_CREAT)` fails now and then with `ENOENT` when another process creates the same name at the same moment. Measured with a C probe, eight processes per race, 2,400 opens per run: plain `O_CREAT` gave `ENOENT` in every one of four runs with `O_NOFOLLOW` (1–3 each) and in one of four without it (2); `O_EXCL` then a plain open, 0 in four runs. Daemons racing for a key (spec test 8) hit it: before the change, 3 daemons over 12 four-way starts exited 1 with `cannot open …owner: No such file or directory`; after it, none in 40 eight-way starts. | None foreseen. |
| D54 | **SI-3 settled: a `HELLO` of another protocol version is answered `MISMATCH` with the served key; every other line the daemon refuses (not a `HELLO`, malformed, settings refused, longer than 4 KiB) is closed without a reply**, with one bounded log line. | S2 noted that another version is another identity (the version is in the key), so `MISMATCH` is the truth. Anything else comes only from a non-conforming client: a conforming shim of the same build cannot send it (its settings pass the same constructor). A new verb would change the wire format for that case alone. | A `REFUSED {"reason":…}` verb (and a protocol bump) if a shim must tell these apart from a crash. |
| D55 | **The peer check (owner decision §11 item 6) runs on every accepted connection before it is counted**: Linux `getsockopt(SO_PEERCRED)`, the BSDs and macOS `getpeereid`; a peer of another uid, or one whose uid cannot be read, is dropped with no reply and a log line naming the uid. On any other platform every connection is refused. | §4.4, §10. Counting it first would let another user fill `--max-clients`. The check is injectable (`Server::set_peer_uid`, test only) because no test can be another user without root. | None. |
| D56 | **The handshake stage is the accept loop's thread: `poll(2)` over the listener, a wake socket and every pending connection**; each pending connection is read with `PreambleLineReader` (one byte at a time, so nothing past the `HELLO` newline is taken) from a non-blocking socket, against a deadline of `HANDSHAKE_TIMEOUT` = 10 s from its acceptance on the daemon's clock. | Spec §4.2 "non-blocking … preamble timeout", and the lane README's S2b note that a read timeout bounds silence, not the line. 10 s is the shim's default whole deadline (§4.1 step 4); a dropped handshake is transient for the shim (§4.1 step 5). One thread allocates nothing per pending connection but its reader. | A constant. |
| D57 | **The root's device and inode, and the daemon's own files, are checked every `IDENTITY_CHECK_INTERVAL` = 5 s and the root at every admission** (spec §4.3's "TTL tick", SI-19). | A `stat` and two `lstat`s every 5 s cost nothing measurable; detection latency does not matter to any client. | A constant. |
| D58 | **Every file but the socket goes through the runtime directory's descriptor** (`openat`, `fstatat`, `unlinkat`, `O_NOFOLLOW`, `O_CLOEXEC`), opened once after `validate_runtime_dir` and checked again by `fstat` (a 0700 directory of this user). The socket is bound by path (there is no `bindat` on macOS) and removed through the descriptor only while the entry is still the one bound; the pid file is removed only if it still holds what this daemon wrote. | The lane README's notes for S4 (open through a held descriptor; a replaced directory). It also makes "files removed only by the owner" hold when the directory was replaced: the old daemon removes from its own directory. | None. |
| D59 | **S3a's parity requirement: a connection of a warm daemon (`StartupMode::Background`) admitted while attempt 1 is the only attempt starts holding attempt 1** (`DaemonState::budget_for`, `ConnectionBudget::joined_at_startup`), with no deadline until its first call (`Joined.deadline: Option<Instant>`). A failed attempt 1 is given as a filled `AttemptOutcome` built from `BuildPhase::Failed`'s error. | The S3a reviewer's construction, without a new field in `DaemonState` (the phase already holds the building attempt's outcome or the failure's text). The reviewer's probe is Task 2's test, and fails without the change. | None. |
| D60 | **`prism-mcp daemon …` is dispatched on the first argument before the standalone parser**, to its own `clap` struct: `--repo`, `--cache-dir`, `--refresh-policy`, `--warm-at-startup`, `--idle-ttl` (1–86,400 s, default 900), `--max-clients` (1–4,096, default 64), `--key`. | The standalone parser has a required `--repo` and no subcommands; adding one would change its help and errors. Today `prism-mcp daemon` is a parse error, so nothing that works changes. | A `clap` subcommand later, with its own transcript check. |
| D61 | **`--key` is checked, not adopted**: a daemon whose computed key differs refuses to start with a message, before it touches any file. | The lane README's S5a note: adopting it would hide a root replaced between the two resolutions. The daemon's side is S4's because S4 owns the command line. | None. |
| D62 | **SIGTERM and SIGINT set a flag and write one byte to a duplicate of the wake socket that is never closed**; the loop drains with `StopReason::Signal`. The handlers are installed only by `run_daemon`, never in a test binary. | Only async-signal-safe work in the handler; a descriptor that cannot be closed and reused under it. | None. |
| D63 | **No wire status verb in S4.** The in-process `status()` and `admission_status()` are what tests read; process tests observe `READY`'s pid, generation and build state. | Spec §4.7: the status probe and `daemon-shutdown` are S5b's, and a new verb changes the wire format and, by S2's rule, the protocol version and every key (README notes for S4 and S5a). | — |
| D64 | **The daemon's own log lines go through `log::line`**: control characters replaced, at most 1,000 bytes of text, nothing past 8 MiB but one note that the cap was reached. At start, a standard error that is a regular file over 1 MiB is truncated. | Spec §4.3. A panic message is not counted (it ends the process). Truncation needs the spawner to open the log with `O_APPEND`, or later writes leave a hole (S5a). | — |
| D65 | **The pid file's third line is `PRISM_CACHE_BUILD_IDENTITY`** (`BuildIdentity::cache_build_identity`). | S2's note (4): the spec does not say which of the three build values; this one determines the other two (S2's SI-4). | One field. |
| D66 | **The module-wide `#![cfg_attr(not(test), allow(dead_code))]` is removed** (S3a's D2 said S4 removes it). What only tests read until S5b carries its own narrow allowance (`status`, `admission_status`, `AdmissionStatus`), and what only tests call is `#[cfg(test)]` (`wake`, `Waker::request_stop`). | Dead code in production is a warning again, per item. | — |
| D67 | **A builder that never returns keeps its daemon from retiring** (S3a's first-review note, SI-15): the spec's "no build in progress", unchanged. | It is the approved text; the alternative needs a bound the spec does not have. Owner questions, item 1. | One condition in `retire_if_idle`. |
| D68 | **Each admitted connection gets a thread named `prism-mcp-conn`; its admission is released by a guard when the thread's closure ends**, so a thread that cannot be spawned (its closure is dropped with the error) releases too. A `READY` that cannot be written releases before any thread exists. | Spec §4.2 "decrements it if the write fails"; S3b's interface "admission's decrement belongs after `serve_admitted` returns, on every path". | — |
| D69 | **The accept loop is outside both domains** (S3b's interface left the choice to S4). A panic in the loop's own code unwinds `run` and ends the process from its main thread (exit status 101); the files stay for the next owner, as after an abort. What the loop does under the control lock goes through `admission.rs` and so through the control plane's guard. | Nothing in the loop holds a daemon lock when it could panic; an abort would add a crash report and nothing else. | `run` could call `on_fatal` from a guard of its own (one line). |
| D70 | **S4 does not remove other keys' files.** Every rebuild of prism leaves five files per repository (lane README); the cleanup rule is S5b's, beside `daemon-status`, which already lists every key. | The daemon knows only its own key. | — |
| D71 | **In-crate tests run the real loop on a thread of the test, with real files in a runtime directory of the test's own and a manual clock**; process tests run the real binary (`env!("CARGO_BIN_EXE_prism-mcp")`). | Spec §9 asks for deterministic constructions; the loop's turn hook (`Server::set_turn_hook`, test only) is the seam that orders a test's actions against the loop's checks without sleeping. | — |

## Spec items implemented, and spec issues found

Spec §12 row S4, item by item: daemon ownership (owner vs. start locks) → Task 3 (`files.rs`) and Task 6 (`run_daemon`); the **start lock** is the shim's (§4.1 step 6) and is S5a's, since only a shim takes it. Bounded handshake stage → Task 5 (`server.rs`, D56). Admission/retirement critical section → Task 1 (`admission.rs`, D48–D51). Draining → Tasks 1 and 5 (D52). Root-identity check → Task 5 (D57). Tests: 2 → Task 5 (`an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free`, injected clock) and Task 6 (`a_daemon_process_serves_a_shim_and_retires_after_its_idle_ttl`, the real-clock smoke test); 3 → Tasks 3 and 6; 4 → Tasks 3 and 6; 8 and 15 → Task 6 (`daemons_started_together_leave_exactly_one_serving`); 13 → Task 5; 16 → Task 5. Owner decision §11 item 6 (peer uid in S4) → Task 5 (D55); item 7 (`--warm-at-startup` on the daemon's command line) → Task 6 (D60).

- **SI-14 (§3.1 "Draining: entered only … when admitted == 0 and pending == 0" against "Loss or change of the root's filesystem identity … and SIGTERM/SIGINT go through Draining").** The second sentence's cases have clients attached. Read as a forced drain recorded in the same critical section (D52). Not an owner question: §4.6's "already attached shims get EOF" says what clients see.
- **SI-15 (§4.2 "no build in progress" with a builder that never returns).** D67; Owner questions, item 1.
- **SI-16 (§4.3 "The log is truncated at daemon start").** Truncating a log that another descriptor appends to needs `O_APPEND` on that descriptor; the shim opens the log (§4.1 step 6), so S5a must open it with `O_APPEND`. Recorded under "Interfaces".
- **SI-17 (§7 "every answer-affecting setting is either in the key or in the per-connection preamble").** `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` is in neither (lane README). Owner questions, item 3.
- **SI-18 (§9 tests 3, 4, 8, 13, 15).** Each has a shim half (the shim replaces the socket, fails closed naming the pid, re-enters arbitration, survives a starter's death). S4 holds the daemon halves; the shim halves are S5a's.
- **SI-19 (§4.3 "on the TTL tick").** No interval is given; D57 (5 s).
- **SI-20 (§4.2 "preamble timeout").** No value is given; D56 (10 s).
- **SI-21 (§4.1 step 6 and §3.1).** The shim unlinks a stale socket under the owner lock before it spawns; the daemon, which takes the same lock, also replaces whatever is at its socket's path before it binds. Both are "the owner"; the second makes a daemon started by anything but a shim (a test, an operator) work, and costs nothing.

## Review Focus

Seven ways this code is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code.

1. **Daemons racing for one key.** Exactly one must serve and the others must exit 0 having touched nothing; on macOS the race also hits a kernel quirk (D53). → Task 6, `daemons_started_together_leave_exactly_one_serving` (four processes at once); Task 3, `the_first_claim_owns_the_key_and_a_second_is_told_it_is_taken`, `a_daemon_that_finds_the_key_taken_touches_no_file`.
2. **A shim that connected while the daemon retires.** It must be answered `RETIRING`, not dropped and not admitted, and the key must be free for the next daemon. → Task 5, `a_connection_queued_while_the_daemon_retires_is_answered_retiring` (the loop is held at the top of a turn while the client connects and the clock passes the TTL).
3. **Stalled handshakes.** They must count against `--max-clients` (or a flood of them is unbounded), time out on a deadline for the whole line (not each silence), and free their slots. → Task 5, `connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out`, `the_handshake_deadline_bounds_the_whole_line`.
4. **Another user's process.** It must get no reply and take no slot. → Task 5, `a_connection_from_another_user_is_dropped_before_any_reply` (the uid injected; no test can be another user without root).
5. **A root replaced under a running daemon, or its runtime directory.** The daemon must drain, answer what is pending `RETIRING`, and remove nothing in the new directory. → Task 5, `a_replaced_root_drains_the_daemon_at_the_next_check`, `a_replaced_root_is_noticed_at_admission`, `replaced_files_drain_the_daemon_and_the_new_directory_is_left_alone`; Task 3, `a_replaced_runtime_directory_is_noticed_and_left_alone`.
6. **The idle clock.** It must count from the last connection's end, be held off by a build, and start for a daemon nobody reaches. → Task 1, `retirement_waits_for_the_idle_ttl_counted_from_the_last_connection`, `a_running_build_holds_retirement_off`; Task 5, `a_daemon_nobody_reaches_retires_after_the_ttl`, `a_running_build_keeps_an_idle_daemon_up_until_it_ends`.
7. **A warm daemon whose startup build failed.** Its first client must hear the failure, as in process. → Task 2, `a_warm_daemon_whose_startup_build_failed_before_the_first_call_reports_it_like_the_in_process_server`.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or comments state, with the test that fails when it breaks and one wrong implementation that test rules out. Every mutant in the last column was applied in validation and the test failed, except where the row says otherwise.

| Claim | Test | A mutant it kills |
|---|---|---|
| `--max-clients` bounds pending and admitted connections together. | T1 `the_bound_counts_pending_and_admitted_connections_together`; T5 `connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out` | a bound on admitted connections only (1A) |
| No retirement while a build runs. | T1 `a_running_build_holds_retirement_off`; T5 `a_running_build_keeps_an_idle_daemon_up_until_it_ends` | `retire_if_idle` without the build condition (1B) |
| The socket is unlinked inside the critical section that decides, and once. | T1 `the_socket_is_unlinked_inside_the_critical_section_and_once` | the unlink after the lock is released (1C) |
| A panic in a control-plane entry ends the daemon while the control lock is still held. | T1 `a_panic_inside_the_retirement_decision_ends_the_daemon_with_the_lock_held` | `ControlPlane`'s fields in the other order (1D); as first written the test's "held" was `try_lock().is_err()`, true for a poisoned unheld lock too, and 1D survived |
| Nothing is admitted after the drain decision. | T1 `after_the_retirement_decision_nothing_is_admitted`, `a_forced_drain_ignores_the_counts_and_refuses_new_connections` | `admit` that ignores the flag (1E) |
| The idle time counts from the last connection's end. | T1 `retirement_waits_for_the_idle_ttl_counted_from_the_last_connection` | `release` that does not start the idle time (1F) |
| A warm daemon's client hears a startup failure on its first call, as in process. | T2 the parity test; T2 `only_a_warm_daemon_s_startup_attempt_is_given_to_a_new_connection` | `serve_client` with an empty budget (2A, the code before Task 2); a failed attempt 1 seeded as an empty outcome (2B) |
| A second claimant of a key is told it is taken and touches no file. | T3 `the_first_claim_owns_the_key_and_a_second_is_told_it_is_taken`, `a_daemon_that_finds_the_key_taken_touches_no_file`; T6 `a_daemon_that_finds_a_live_owner_exits_silently_and_touches_nothing` | a claim that does not lock (3A); `Taken` as an error exit (6A) |
| The owner replaces a stale socket. | T3 `binding_replaces_a_stale_socket_left_by_a_dead_daemon`; T6 `a_daemon_replaces_a_stale_socket_left_by_a_dead_one` | a bind that does not remove the stale file (3B) |
| No file is opened through a symlink. | T3 `a_symlink_in_place_of_the_owner_lock_is_refused` | `openat` without `O_NOFOLLOW` (3C); as first written the test used a dangling symlink, which `O_EXCL` refuses anyway, and 3C survived |
| The daemon removes its socket only while it is still the one it bound. | T3 `the_socket_is_removed_only_if_it_is_still_the_one_this_daemon_bound` | removal without the inode check (3D) |
| A replaced runtime directory is noticed. | T3 `a_replaced_runtime_directory_is_noticed_and_left_alone`; T5 `replaced_files_drain_the_daemon_and_the_new_directory_is_left_alone` | `still_ours` always true (3E) |
| A peer of another user is dropped before any reply and never counted. | T5 `a_connection_from_another_user_is_dropped_before_any_reply` | a check that accepts every uid (5A) |
| A handshake's deadline bounds the whole line. | T5 `the_handshake_deadline_bounds_the_whole_line`, the test-16 test | no deadline (5B) |
| A connection queued when the daemon retires is answered `RETIRING`. | T5 `a_connection_queued_while_the_daemon_retires_is_answered_retiring` | a drain that closes the listener without answering the queue (5C) |
| The root is checked at every admission. | T5 `a_replaced_root_is_noticed_at_admission` | no check at admission (5D) |
| Another protocol version is answered `MISMATCH`. | T5 `another_identity_or_protocol_is_answered_mismatch_and_not_admitted` | no reply for it (5E) |
| An admission is released when its connection ends. | T5 `a_hello_for_the_served_identity_is_answered_ready_and_mcp_follows`, `an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free` | a release guard that does not release (5F) |
| Pending connections are answered `RETIRING` when the daemon drains. | T5 `a_replaced_root_drains_the_daemon_at_the_next_check` | a drain that drops them silently (5G) |
| SIGTERM drains the daemon: socket and pid file removed, exit 0, clients see EOF. | T6 `a_termination_signal_drains_a_daemon_with_a_shim_attached` | no handler installed (6B) |
| A daemon handed another key does not start. | T6 `a_daemon_handed_another_key_refuses_to_start` | no `--key` check (6C) |

Stated and **not** held by a test: the log's 8 MiB cap and its truncation at start (no test writes 8 MiB; the truncation runs only when standard error is a regular file); that a panic in the loop's own code ends the process (D69; no seam injects one); that a `READY` whose write fails releases its admission (a fresh socket's buffer always takes the line); that a worker thread that cannot be spawned releases (no seam); the busy-loop on a persistently failing `accept` ("What each mechanism covers"); anything on Linux (the Linux-only FFI was type-checked only); the shim halves of tests 3, 4, 8, 13 and 15 (S5a).

## File Structure

| File | Responsibility | Change | PR |
|---|---|---|---|
| `src/mcp/daemon/control.rs` | `StopReason`, `Admission` in `ControlState`, `ControlState::build_status` | Modify (Tasks 1, 5, 6) | both |
| `src/mcp/daemon/coordinator.rs` | `ControlPlane`, `control_plane()`, `now()`, `control_is_held()` (test), the `Admission` initializer; `status` through `control_plane` (Task 1); dead-code allowances (Task 6) | Modify | both |
| `src/mcp/daemon/admission.rs`, `admission_tests.rs` | Admission, release, the retirement and drain decisions, the idle deadline (Task 1); `budget_for` (Task 2) | Create (Task 1), modify (Tasks 2, 6) | both |
| `src/mcp/daemon/budget.rs`, `budget_tests.rs` | `ConnectionBudget::joined_at_startup`; a deadline set by the first call | Modify (Task 2) | S4-1 |
| `src/mcp/daemon/connection.rs` | `serve_client` takes its budget from `budget_for` | Modify (Task 2) | S4-1 |
| `src/mcp/daemon/connection_background_tests.rs` | S3a's parity probe for a warm daemon | Create (Task 2) | S4-1 |
| `src/mcp/daemon/files.rs`, `files_tests.rs` | The owner lock, the socket, the pid file (spec tests 3 and 4, the daemon's halves) | Create (Task 3) | S4-1 |
| `src/mcp/daemon/peer.rs` | The peer's uid | Create (Task 5) | S4-2 |
| `src/mcp/daemon/log.rs` | The daemon's capped log lines (Task 5); truncation at start (Task 6) | Create, modify | S4-2 |
| `src/mcp/daemon/server.rs` | The accept loop, the handshake stage, the checks, the drain | Create (Task 5), modify (Task 6) | S4-2 |
| `src/mcp/daemon/server_support.rs` | The loop on a test thread; a preamble client | Create (Task 5) | S4-2 |
| `src/mcp/daemon/server_tests.rs`, `server_drain_tests.rs` | The handshake stage (test 16); retirement and draining (tests 2 and 13) | Create (Task 5) | S4-2 |
| `src/mcp/daemon/run.rs` | `run_daemon`, `DaemonOptions`, the signal handlers | Create (Task 6) | S4-2 |
| `src/mcp/daemon/mod.rs` | Module lines; the re-export; the module allowance removed | Modify (Tasks 1, 3, 5, 6) | both |
| `src/mcp/mod.rs` | `pub use daemon::{run_daemon, DaemonOptions, …}` | Modify (Task 6) | S4-2 |
| `src/bin/prism-mcp.rs` | `prism-mcp daemon …` | Modify (Task 6) | S4-2 |
| `tests/mcp/main.rs`, `tests/mcp/daemon_process_test.rs` | The daemon as a process (tests 2, 3, 4, 8, 15) | Modify, create (Task 6) | S4-2 |
| `CLAUDE.md` | MCP Adapter, the shared-daemon paragraph | Modify (Tasks 4, 7) | both |
| `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `src/mcp/shared/*`, `tests/fixtures/mcp/golden/*` | The standalone contract and earlier slices' code | **Must not change** | |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file at that point of the plan, then "with this:" and its replacement.

The replacements were generated from the validated branch's per-task commits, so each carries a few lines of unchanged context around its change; the context is part of the anchor and is not edited.

---

# Part one: S4-1, the control plane and the key's files (Tasks 1–4)

### Task 1: Admission and retirement under the control lock

**Files:**
- Modify: `src/mcp/daemon/control.rs`, `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/mod.rs`
- Create: `src/mcp/daemon/admission.rs`, `src/mcp/daemon/admission_tests.rs`

**Interfaces:**
- Consumes: `DaemonState` (S3a), its private `lock_control`, `guard` and `clock`; `FatalOnUnwind` and `Domain::Control` (S3b); `BuildPhase`, `BuildStatus` (S3a).
- Produces (all `pub(super)` in `crate::mcp::daemon`): `control::StopReason { Idle, RootChanged, Signal }` (Task 5 adds `FilesReplaced`); `control::Admission { pending: u32, admitted: u32, idle_since: Option<Instant>, draining: Option<StopReason> }` as `ControlState::admission`; `ControlState::build_status(&self) -> BuildStatus`; `DaemonState::control_plane(&self) -> Result<ControlPlane<'_>, Poisoned>` (`ControlPlane` derefs to `ControlState`); `DaemonState::now(&self) -> Instant`; `#[cfg(test)] DaemonState::control_is_held(&self) -> bool`; in `admission.rs`: `enum Entry { Pending, Busy { max_clients: u32 }, Retiring }`, `enum Admit { Ready(BuildStatus), Retiring }`, `struct AdmissionStatus { pending, admitted, draining }`, and on `DaemonState`: `enter_handshake(&self, max_clients: u32) -> Result<Entry, Poisoned>`, `leave_handshake(&self) -> Result<(), Poisoned>`, `admit(&self) -> Result<Admit, Poisoned>`, `release(&self) -> Result<(), Poisoned>`, `retire_if_idle(&self, idle_ttl: Duration, unlink: impl FnOnce()) -> Result<Option<StopReason>, Poisoned>`, `drain(&self, reason: StopReason, unlink: impl FnOnce()) -> Result<StopReason, Poisoned>`, `idle_deadline(&self, idle_ttl: Duration) -> Result<Option<Instant>, Poisoned>`, `admission_status(&self) -> Result<AdmissionStatus, Poisoned>`.

- [ ] **Step 1: Write the failing tests and register the module**

Create `src/mcp/daemon/admission_tests.rs`:

````rust
//! Tests for admission and retirement under the control lock (spec §4.2 "Admission and
//! retirement", §9 tests 2 and 16 as far as the control plane decides them). Time is a clock
//! the test moves; nothing here sleeps to order two events.
use super::super::clock::Clock;
use super::super::coordinator::{DaemonState, Parts};
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    build_after, fixture, parts_with, scripted, state_with, status, wait_until, Conn, Gate,
    ManualClock, Outcome,
};
use super::*;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

const TTL: Duration = Duration::from_secs(900);

fn lazy_state() -> (tempfile::TempDir, Arc<ManualClock>, Arc<DaemonState>) {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    (dir, clock, state)
}

fn counts(state: &DaemonState) -> AdmissionStatus {
    state
        .admission_status()
        .expect("the control lock is not poisoned")
}

fn retire(state: &DaemonState) -> Option<StopReason> {
    state
        .retire_if_idle(TTL, || {})
        .expect("the control lock is not poisoned")
}

/// Move the clock without waking anyone: nothing here is parked.
fn later(clock: &ManualClock, state: &DaemonState, secs: u64) {
    super::super::test_support::advance(clock, state, secs);
}

/// Spec §4.2: `--max-clients` bounds pending and admitted connections together, and a
/// connection over the bound is answered `BUSY` without being counted. Rules out a bound on
/// admitted connections alone, under which stalled handshakes are unbounded.
#[test]
fn the_bound_counts_pending_and_admitted_connections_together() {
    let (_dir, _clock, state) = lazy_state();

    assert_eq!(state.enter_handshake(2), Ok(Entry::Pending));
    assert_eq!(state.enter_handshake(2), Ok(Entry::Pending));
    assert_eq!(
        state.enter_handshake(2),
        Ok(Entry::Busy { max_clients: 2 }),
        "two pending connections fill a bound of two"
    );
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    assert_eq!(
        state.enter_handshake(2),
        Ok(Entry::Busy { max_clients: 2 }),
        "one pending and one admitted fill it too"
    );
    assert_eq!(
        counts(&state),
        AdmissionStatus {
            pending: 1,
            admitted: 1,
            draining: None
        }
    );

    state.release().expect("release");
    assert_eq!(state.enter_handshake(2), Ok(Entry::Pending));
    state.leave_handshake().expect("leave");
    state.leave_handshake().expect("leave");
    assert_eq!(
        counts(&state),
        AdmissionStatus {
            pending: 0,
            admitted: 0,
            draining: None
        }
    );
}

/// `READY` carries the build state and generation read in the same critical section that
/// admits the connection.
#[test]
fn an_admission_reports_the_build_status_it_was_made_under() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    let Ok(Admit::Ready(admitted)) = state.admit() else {
        panic!("a connection is admitted");
    };
    assert_eq!(admitted, status(&state));
    assert_eq!(admitted.state, BuildState::Idle);
}

/// Spec §4.2: retirement needs nothing admitted and nothing pending for the idle TTL. The idle
/// time counts from the moment both counts reached zero, not from the daemon's start. Rules out
/// a TTL measured from the last admission (a long session would retire the daemon at once when
/// it ends) and retirement with a connection still pending.
#[test]
fn retirement_waits_for_the_idle_ttl_counted_from_the_last_connection() {
    let (_dir, clock, state) = lazy_state();

    assert_eq!(
        retire(&state),
        None,
        "the first decision starts the idle count"
    );
    later(&clock, &state, TTL.as_secs() - 1);
    assert_eq!(retire(&state), None, "one second short of the TTL");
    later(&clock, &state, 1);
    assert_eq!(
        counts(&state).draining,
        None,
        "nothing decides until the decision is asked for"
    );

    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    later(&clock, &state, 2 * TTL.as_secs());
    assert_eq!(retire(&state), None, "a pending connection holds it off");
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    later(&clock, &state, 2 * TTL.as_secs());
    assert_eq!(retire(&state), None, "an admitted connection holds it off");

    state.release().expect("release");
    later(&clock, &state, TTL.as_secs() - 1);
    assert_eq!(retire(&state), None, "the TTL counts from the release");
    later(&clock, &state, 1);
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// Spec §4.2: no retirement while a build runs, however long the daemon has been idle.
#[test]
fn a_running_build_holds_retirement_off() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));

    assert_eq!(retire(&state), None);
    later(&clock, &state, 2 * TTL.as_secs());
    assert_eq!(retire(&state), None);
    gate.open();
    wait_until("the build to publish", || {
        status(&state).state == BuildState::Ready
    });
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// Spec §4.2: an admission and a retirement cannot interleave. Once the decision is taken no
/// connection is counted or admitted, and a connection that was pending is told `RETIRING`.
#[test]
fn after_the_retirement_decision_nothing_is_admitted() {
    let (_dir, clock, state) = lazy_state();
    assert_eq!(retire(&state), None);
    later(&clock, &state, TTL.as_secs());
    assert_eq!(retire(&state), Some(StopReason::Idle));

    assert_eq!(state.enter_handshake(64), Ok(Entry::Retiring));
    assert_eq!(
        state.admit(),
        Ok(Admit::Retiring),
        "a connection that raced into the stage"
    );
    assert_eq!(counts(&state).admitted, 0);
}

/// Spec §3.1 and §4.2: the socket is unlinked inside the critical section that decides to
/// drain, and once. Rules out an unlink after the lock is released, which a connection could be
/// admitted before.
#[test]
fn the_socket_is_unlinked_inside_the_critical_section_and_once() {
    let (_dir, clock, state) = lazy_state();
    let unlinked = AtomicU64::new(0);
    let held = Mutex::new(Vec::new());
    let unlink = || {
        unlinked.fetch_add(1, Ordering::SeqCst);
        held.lock().expect("held").push(state.control_is_held());
    };
    assert_eq!(
        state.retire_if_idle(TTL, || panic!("unlinked too soon")),
        Ok(None)
    );
    later(&clock, &state, TTL.as_secs());
    assert_eq!(
        state.retire_if_idle(TTL, unlink),
        Ok(Some(StopReason::Idle))
    );
    assert_eq!(
        state.retire_if_idle(TTL, || panic!("unlinked twice")),
        Ok(Some(StopReason::Idle))
    );
    assert_eq!(
        state.drain(StopReason::Signal, || panic!("unlinked twice")),
        Ok(StopReason::Idle),
        "a later reason does not replace the first"
    );
    assert_eq!(unlinked.load(Ordering::SeqCst), 1);
    assert_eq!(*held.lock().expect("held"), vec![true]);
}

/// Spec §3.1: a lost root and a signal drain the daemon whatever its counts, and the counts go
/// on being kept for the connections that are still open.
#[test]
fn a_forced_drain_ignores_the_counts_and_refuses_new_connections() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));

    let mut unlinked = 0;
    assert_eq!(
        state.drain(StopReason::RootChanged, || unlinked += 1),
        Ok(StopReason::RootChanged)
    );
    assert_eq!(unlinked, 1);
    assert_eq!(state.enter_handshake(64), Ok(Entry::Retiring));
    assert_eq!(state.admit(), Ok(Admit::Retiring));
    state.release().expect("release");
    assert_eq!(
        counts(&state),
        AdmissionStatus {
            pending: 0,
            admitted: 0,
            draining: Some(StopReason::RootChanged)
        }
    );
}

/// The accept loop sleeps until the idle deadline at most: it exists only while nothing is
/// pending or admitted, and moves with the last release.
#[test]
fn the_idle_deadline_follows_the_last_connection() {
    let (_dir, clock, state) = lazy_state();
    let start = clock.now();
    assert_eq!(state.idle_deadline(TTL), Ok(None), "nothing counted yet");
    assert_eq!(retire(&state), None);
    assert_eq!(state.idle_deadline(TTL), Ok(Some(start + TTL)));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert_eq!(state.idle_deadline(TTL), Ok(None));
    later(&clock, &state, 7);
    state.leave_handshake().expect("leave");
    assert_eq!(
        state.idle_deadline(TTL),
        Ok(Some(start + Duration::from_secs(7) + TTL))
    );
}

/// Every entry point here is in the control plane: a panic in it ends the daemon while the
/// control lock is still held (D47). Rules out an entry point that takes the lock without the
/// guard, whose panic would leave the lock poisoned for every other client.
#[test]
fn a_panic_inside_the_retirement_decision_ends_the_daemon_with_the_lock_held() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts: Parts = parts_with(builder, &clock);
    let seen: Arc<Mutex<Vec<(Fatal, bool)>>> = Arc::default();
    let target: Arc<OnceLock<Weak<DaemonState>>> = Arc::default();
    let sink = Arc::clone(&seen);
    let watched = Arc::clone(&target);
    parts.on_fatal = Arc::new(move |fatal| {
        let held = watched
            .get()
            .and_then(Weak::upgrade)
            .is_some_and(|state| state.control_is_held());
        sink.lock().expect("seen").push((fatal, held));
    });
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let _ = target.set(Arc::downgrade(&state));
    assert_eq!(state.retire_if_idle(TTL, || {}), Ok(None));
    later(&clock, &state, TTL.as_secs());

    let panicking = Arc::clone(&state);
    let unwound = std::thread::spawn(move || {
        let _ = panicking.retire_if_idle(TTL, || panic!("injected panic in the unlink"));
    })
    .join();
    assert!(unwound.is_err());
    assert_eq!(
        *seen.lock().expect("seen"),
        vec![(Fatal::Panicked(Domain::Control), true)]
    );
}
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
// allowance there.
#![cfg_attr(not(test), allow(dead_code))]

mod budget;
mod clock;
mod connection;
````

with this:

````rust
// allowance there.
#![cfg_attr(not(test), allow(dead_code))]

mod admission;
mod budget;
mod clock;
mod connection;
````

- [ ] **Step 2: Run them to see them fail**

```bash
LOG_DIR=/tmp/s4; mkdir -p "$LOG_DIR"
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t1-red.log" 2>&1
```

Expected in the log: `error[E0583]: file not found for module `admission``.

- [ ] **Step 3: Implement**

Create `src/mcp/daemon/admission.rs`:

````rust
//! Admission and retirement (spec §4.2 "Admission and retirement", §3.1 "Draining"): the
//! counts of pending and admitted connections and the decision to drain, all under the control
//! lock, so that an admission and a retirement cannot interleave.
//!
//! Every function here is an entry point into the control plane and takes the lock through
//! `DaemonState::control_plane`, which makes the control plane's panic guard right after the
//! lock (D47): a panic or a poisoned lock in any of them ends the daemon.
use super::control::{BuildPhase, BuildStatus, StopReason};
use super::coordinator::{DaemonState, Poisoned};
use std::time::{Duration, Instant};

/// What the handshake stage does with a connection it has just accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// Counted as pending: read its `HELLO`.
    Pending,
    /// Pending and admitted connections are at the bound: answer `BUSY` and close it.
    Busy { max_clients: u32 },
    /// The daemon drains: answer `RETIRING` and close it.
    Retiring,
}

/// What a pending connection whose `HELLO` asked for the served identity is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Admit {
    /// Counted as admitted: write `READY` with this build status. If the write fails, call
    /// `release`.
    Ready(BuildStatus),
    /// The daemon drains: answer `RETIRING` and close it.
    Retiring,
}

/// The connection counts, for tests and the status probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AdmissionStatus {
    pub(super) pending: u32,
    pub(super) admitted: u32,
    pub(super) draining: Option<StopReason>,
}

impl DaemonState {
    /// Count a connection the accept loop has just accepted as pending, unless the daemon
    /// drains or pending and admitted connections together are at `max_clients` (spec §4.2:
    /// the bound is over both).
    pub(super) fn enter_handshake(&self, max_clients: u32) -> Result<Entry, Poisoned> {
        let mut control = self.control_plane()?;
        let admission = &mut control.admission;
        if admission.draining.is_some() {
            return Ok(Entry::Retiring);
        }
        if admission.pending.saturating_add(admission.admitted) >= max_clients {
            return Ok(Entry::Busy { max_clients });
        }
        admission.pending = admission.pending.saturating_add(1);
        admission.idle_since = None;
        Ok(Entry::Pending)
    }

    /// A pending connection left the handshake stage without being admitted: it timed out,
    /// closed, sent a line that was refused, or was answered `MISMATCH` or `RETIRING`.
    pub(super) fn leave_handshake(&self) -> Result<(), Poisoned> {
        let now = self.now();
        let mut control = self.control_plane()?;
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        if admission.pending == 0 && admission.admitted == 0 {
            admission.idle_since = Some(now);
        }
        Ok(())
    }

    /// Admit a pending connection whose `HELLO` asked for the served identity: in one
    /// acquisition of the control lock it stops being pending and, unless the daemon drains,
    /// becomes admitted, and the build status for its `READY` is read. The count goes up before
    /// `READY` is written (spec §4.2); the caller calls `release` if the write fails.
    pub(super) fn admit(&self) -> Result<Admit, Poisoned> {
        let now = self.now();
        let mut control = self.control_plane()?;
        let status = control.build_status();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        if admission.draining.is_some() {
            if admission.pending == 0 && admission.admitted == 0 {
                admission.idle_since = Some(now);
            }
            return Ok(Admit::Retiring);
        }
        admission.admitted = admission.admitted.saturating_add(1);
        admission.idle_since = None;
        Ok(Admit::Ready(status))
    }

    /// An admitted connection ended: its serve loop returned, or its `READY` could not be
    /// written, or its worker could not be started.
    pub(super) fn release(&self) -> Result<(), Poisoned> {
        let now = self.now();
        let mut control = self.control_plane()?;
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
        unlink: impl FnOnce(),
    ) -> Result<Option<StopReason>, Poisoned> {
        let now = self.now();
        let mut control = self.control_plane()?;
        if let Some(reason) = control.admission.draining {
            return Ok(Some(reason));
        }
        let admission = &mut control.admission;
        if admission.pending != 0 || admission.admitted != 0 {
            return Ok(None);
        }
        let since = *admission.idle_since.get_or_insert(now);
        if now.saturating_duration_since(since) < idle_ttl
            || matches!(control.build, BuildPhase::Building { .. })
        {
            return Ok(None);
        }
        control.admission.draining = Some(StopReason::Idle);
        unlink();
        Ok(Some(StopReason::Idle))
    }

    /// Drain whatever the counts say (root lost or replaced, files replaced, a signal). The
    /// first reason recorded is kept; `unlink` runs inside the critical section that records
    /// it, and not again. Returns the reason the daemon drains for.
    pub(super) fn drain(
        &self,
        reason: StopReason,
        unlink: impl FnOnce(),
    ) -> Result<StopReason, Poisoned> {
        let mut control = self.control_plane()?;
        if let Some(earlier) = control.admission.draining {
            return Ok(earlier);
        }
        control.admission.draining = Some(reason);
        unlink();
        Ok(reason)
    }

    /// When the idle TTL runs out if nothing changes: `None` while a connection is pending or
    /// admitted, and before the first retirement decision. The accept loop sleeps no longer
    /// than this.
    pub(super) fn idle_deadline(&self, idle_ttl: Duration) -> Result<Option<Instant>, Poisoned> {
        let control = self.control_plane()?;
        Ok(control
            .admission
            .idle_since
            .and_then(|since| since.checked_add(idle_ttl)))
    }

    pub(super) fn admission_status(&self) -> Result<AdmissionStatus, Poisoned> {
        let control = self.control_plane()?;
        Ok(AdmissionStatus {
            pending: control.admission.pending,
            admitted: control.admission.admitted,
            draining: control.admission.draining,
        })
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    }
}

/// What the control lock guards. Slice S4 adds the admission counters and the retirement
/// decision beside the build phase, under the same lock.
pub(super) struct ControlState {
    pub(super) build: BuildPhase,
    /// How many attempts have been started; also the number of the latest one.
````

with this:

````rust
    }
}

/// Why a daemon stops serving (spec §3.1 "Draining").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StopReason {
    /// Nothing admitted and nothing pending for the idle TTL, and no build running.
    Idle,
    /// The repository root is gone, or its device and inode are no longer the served ones.
    RootChanged,
    /// SIGTERM or SIGINT.
    Signal,
}

/// The connections of a daemon and whether it drains (spec §4.2 "Admission and retirement").
/// Changed only under the control lock, by the functions in `admission.rs`.
#[derive(Debug)]
pub(super) struct Admission {
    /// Connections accepted into the handshake stage and not yet admitted or dropped.
    pub(super) pending: u32,
    /// Connections that were answered `READY` and whose serve loop has not returned.
    pub(super) admitted: u32,
    /// Since when `pending` and `admitted` have both been 0, as far as admission and the
    /// retirement decision have seen; `None` while either is not, and before the first
    /// decision (which starts the count for a daemon nobody has reached yet).
    pub(super) idle_since: Option<Instant>,
    /// Set once, by the critical section that decides the daemon drains, and never cleared:
    /// from then on no connection is admitted.
    pub(super) draining: Option<StopReason>,
}

impl Admission {
    /// A daemon with no connection yet. Its idle time starts at the first retirement decision,
    /// so a daemon nobody ever reaches retires after the idle TTL like any other.
    pub(super) fn new() -> Self {
        Self {
            pending: 0,
            admitted: 0,
            idle_since: None,
            draining: None,
        }
    }
}

/// What the control lock guards: the build phase and, beside it under the same lock, the
/// admission counts and the retirement decision (spec §4.2).
pub(super) struct ControlState {
    pub(super) build: BuildPhase,
    /// How many attempts have been started; also the number of the latest one.
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    /// not depend on the phase, so it is true in the interval between publication and the
    /// announcement of `Ready` as well.
    pub(super) generation: u64,
}

/// A snapshot of the control plane: what the status probe reports about the build.
````

with this:

````rust
    /// not depend on the phase, so it is true in the interval between publication and the
    /// announcement of `Ready` as well.
    pub(super) generation: u64,
    pub(super) admission: Admission,
}

impl ControlState {
    /// What the status probe and `READY` report about the build.
    pub(super) fn build_status(&self) -> BuildStatus {
        BuildStatus {
            state: self.build.state(),
            attempt: self.attempts,
            generation: self.generation,
            waiting: self.waiting,
        }
    }
}

/// A snapshot of the control plane: what the status probe reports about the build.
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use super::control::answer_for;
pub(super) use super::control::{Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use super::fatal::{abort_daemon, Domain, Fatal, FatalOnUnwind, OnFatal};
use crate::mcp::lazy::SessionBuilder;
// The tests in this file's child modules name `BuildState` through `use super::*`.
````

with this:

````rust
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use super::control::answer_for;
pub(super) use super::control::{
    Admission, Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState,
};
use super::fatal::{abort_daemon, Domain, Fatal, FatalOnUnwind, OnFatal};
use crate::mcp::lazy::SessionBuilder;
// The tests in this file's child modules name `BuildState` through `use super::*`.
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use std::time::Instant;

````

with this:

````rust
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use std::time::Instant;

````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
                attempts: 0,
                waiting: 0,
                generation: 0,
            }),
            changed: Condvar::new(),
            provider: RwLock::new(None),
````

with this:

````rust
                attempts: 0,
                waiting: 0,
                generation: 0,
                admission: Admission::new(),
            }),
            changed: Condvar::new(),
            provider: RwLock::new(None),
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust

    /// The control plane's view of the build.
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        let control = self.lock_control()?;
        let _fatal = self.guard(Domain::Control);
        Ok(BuildStatus {
            state: control.build.state(),
            attempt: control.attempts,
            generation: control.generation,
            waiting: control.waiting,
        })
    }

    /// Run `query` on the provider in the slot with shared access; `None` when the slot is
````

with this:

````rust

    /// The control plane's view of the build.
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        Ok(self.control_plane()?.build_status())
    }

    /// The control lock, held, with the control plane's panic guard made right after it (D47).
    /// Every entry point S4 adds takes the lock through this, so the order is the type's.
    pub(super) fn control_plane(&self) -> Result<ControlPlane<'_>, Poisoned> {
        let control = self.lock_control()?;
        Ok(ControlPlane {
            _fatal: self.guard(Domain::Control),
            control,
        })
    }

    /// The daemon's clock.
    pub(super) fn now(&self) -> Instant {
        self.clock.now()
    }

    /// Whether the control lock is held right now, by this thread or another. A poisoned lock
    /// that nobody holds is not held. Tests only.
    #[cfg(test)]
    pub(super) fn control_is_held(&self) -> bool {
        matches!(
            self.control.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        )
    }

    /// Run `query` on the provider in the slot with shared access; `None` when the slot is
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    }
}

/// Ends the daemon if the build thread unwinds (spec §3.1: a panic on the build thread is
/// fatal). Under a test's `on_fatal`, which returns, it then ends the attempt as failed, so
/// that the phase is not left `Building` with nobody to end it.
````

with this:

````rust
    }
}

/// The control lock and the guard made right after it. Fields drop in declaration order, so the
/// guard fires while the lock is still held (D47).
pub(super) struct ControlPlane<'a> {
    _fatal: FatalOnUnwind<'a>,
    control: MutexGuard<'a, ControlState>,
}

impl Deref for ControlPlane<'_> {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl DerefMut for ControlPlane<'_> {
    fn deref_mut(&mut self) -> &mut ControlState {
        &mut self.control
    }
}

/// Ends the daemon if the build thread unwinds (spec §3.1: a panic on the build thread is
/// fatal). Under a test's `on_fatal`, which returns, it then ends the attempt as failed, so
/// that the phase is not left `Building` with nobody to end it.
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t1.log" 2>&1
```

Expected: `test result: ok. 92 passed; 0 failed` (83 before + 9), and no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/control.rs src/mcp/daemon/coordinator.rs src/mcp/daemon/mod.rs src/mcp/daemon/admission.rs src/mcp/daemon/admission_tests.rs
git commit -m "feat(mcp): admission and retirement under the daemon's control lock"
```

- [ ] **Step 6: Prove the tests bite**

Apply each mutant alone, run `cargo test --features mcp --lib mcp::daemon::admission`, see the named test fail, then `git checkout -- <file>`.

| Mutant | Edit | Fails |
|---|---|---|
| 1A | in `admission.rs`, `if admission.pending.saturating_add(admission.admitted) >= max_clients {` → `if admission.admitted >= max_clients {` | `the_bound_counts_pending_and_admitted_connections_together` |
| 1B | in `retire_if_idle`, drop the line `|| matches!(control.build, BuildPhase::Building { .. })` (and keep the `{`) | `a_running_build_holds_retirement_off` |
| 1C | in `retire_if_idle`, put `drop(control);` before `unlink();` | `the_socket_is_unlinked_inside_the_critical_section_and_once`, `a_panic_inside_the_retirement_decision_ends_the_daemon_with_the_lock_held` |
| 1D | in `coordinator.rs`, declare `control` before `_fatal` in `struct ControlPlane` | `a_panic_inside_the_retirement_decision_ends_the_daemon_with_the_lock_held` |
| 1E | in `admit`, delete the `if admission.draining.is_some() { … return Ok(Admit::Retiring); }` block | `after_the_retirement_decision_nothing_is_admitted`, `a_forced_drain_ignores_the_counts_and_refuses_new_connections` |
| 1F | in `release`, delete the `if admission.pending == 0 && admission.admitted == 0 { … }` block | `retirement_waits_for_the_idle_ttl_counted_from_the_last_connection` |

### Task 2: A warm daemon reports a failed startup build as the in-process server does

**Files:**
- Modify: `src/mcp/daemon/budget.rs`, `src/mcp/daemon/budget_tests.rs`, `src/mcp/daemon/admission.rs`, `src/mcp/daemon/admission_tests.rs`, `src/mcp/daemon/connection.rs`
- Create: `src/mcp/daemon/connection_background_tests.rs`

**Interfaces:**
- Consumes: Task 1's `control_plane()`; `ConnectionBudget` (S3a, D32); `BuildPhase::{Building, Failed}`; `StartupMode::Background`.
- Produces: `ConnectionBudget::joined_at_startup(client: &ClientSettings, outcome: Arc<AttemptOutcome>) -> ConnectionBudget`; `DaemonState::budget_for(&self, client: &ClientSettings) -> Result<ConnectionBudget, Poisoned>`, which `serve_client` now calls instead of `ConnectionBudget::new`.

S3a's interface for S4 states the requirement and gives the reviewer's probe as a test sketch: under `--warm-at-startup`, when the startup build fails before a client's first call, the in-process server answers that call `build_failed` with the cause and retries on the next; S3a's daemon started the retry on the first call and answered `warming`. The probe is the new test here, unchanged but for two assertions that name what each in-process response is.

- [ ] **Step 1: Write the failing tests**

In `src/mcp/daemon/admission_tests.rs`, replace this:

````rust
use super::super::coordinator::{DaemonState, Parts};
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    build_after, fixture, parts_with, scripted, state_with, status, wait_until, Conn, Gate,
    ManualClock, Outcome,
};
use super::*;
use crate::mcp::shared::preamble::BuildState;
````

with this:

````rust
use super::super::coordinator::{DaemonState, Parts};
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    build_after, client_settings, fail_now, fixture, parts_with, scripted, state_with, status,
    wait_until, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::shared::preamble::BuildState;
````

In `src/mcp/daemon/admission_tests.rs`, replace this:

````rust
        vec![(Fatal::Panicked(Domain::Control), true)]
    );
}
````

with this:

````rust
        vec![(Fatal::Panicked(Domain::Control), true)]
    );
}

/// S3a's parity requirement for S4: a connection of a warm daemon whose startup attempt failed
/// before the connection's first call holds that attempt, so its first call hears the failure.
/// A lazy daemon's connection, and one admitted after the retry started, start empty.
#[test]
fn only_a_warm_daemon_s_startup_attempt_is_given_to_a_new_connection() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![fail_now("the startup build failed")]);
    let state = DaemonState::new(builder, StartupMode::Background).expect("a warm daemon");
    wait_until("the startup build to fail", || {
        status(&state).state == BuildState::Failed
    });
    let mut seeded = state.budget_for(&client_settings(0)).expect("a budget");
    assert_eq!(
        seeded.unheard_failure(),
        Some("the startup build failed".to_string())
    );
    assert_eq!(seeded.unheard_failure(), None, "heard once");

    // The retry starts; a connection that arrives now does not hear attempt 1.
    let conn = Conn::open(&state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    let mut later_one = state.budget_for(&client_settings(0)).expect("a budget");
    assert_eq!(later_one.unheard_failure(), None);

    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![fail_now("a lazy build failed")]);
    let lazy = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon");
    let conn = Conn::open(&lazy, 600);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Failed(_)));
    let mut fresh = lazy.budget_for(&client_settings(0)).expect("a budget");
    assert_eq!(fresh.unheard_failure(), None);
}
````

In `src/mcp/daemon/budget_tests.rs`, replace this:

````rust
    second.fill(Err("second".to_string()));
    assert_eq!(budget.unheard_failure(), Some("second".to_string()));
}
````

with this:

````rust
    second.fill(Err("second".to_string()));
    assert_eq!(budget.unheard_failure(), Some("second".to_string()));
}

/// A connection given a warm daemon's startup attempt at admission has no deadline until its
/// first call: the wait is the client's from that call, as for any other attempt it joins.
/// Rules out a deadline set at admission, which would shorten the first call's wait by however
/// long the client took to send it.
#[test]
fn a_connection_given_the_startup_attempt_starts_its_wait_at_its_first_call() {
    let start = Instant::now();
    let startup = cell();
    let mut budget =
        ConnectionBudget::joined_at_startup(&client_settings(20), Arc::clone(&startup));

    assert_eq!(
        budget.join(1, &startup, start + secs(100)),
        start + secs(120)
    );
    assert_eq!(
        budget.join(1, &startup, start + secs(130)),
        start + secs(120),
        "later calls keep it"
    );
    assert_eq!(
        budget.join(2, &cell(), start + secs(140)),
        start + secs(160)
    );
}

/// The startup attempt's failure is this connection's to hear once, as if it had joined it.
#[test]
fn a_connection_given_a_failed_startup_attempt_hears_its_failure_once() {
    let startup = cell();
    startup.fill(Err("the startup build failed".to_string()));
    let mut budget = ConnectionBudget::joined_at_startup(&client_settings(0), startup);

    assert_eq!(
        budget.unheard_failure(),
        Some("the startup build failed".to_string())
    );
    assert_eq!(budget.unheard_failure(), None);
}
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
mod ready_tests;

#[cfg(test)]
#[path = "connection_refresh_tests.rs"]
mod refresh_tests;

````

with this:

````rust
mod ready_tests;

#[cfg(test)]
#[path = "connection_background_tests.rs"]
mod background_tests;

#[cfg(test)]
#[path = "connection_refresh_tests.rs"]
mod refresh_tests;

````

Create `src/mcp/daemon/connection_background_tests.rs`:

````rust
//! A warm daemon (`--warm-at-startup`, `StartupMode::Background`) whose startup build fails
//! before a client's first call: S3a's parity requirement for S4 (the second review of S3a-2,
//! Opus F1). The in-process server tells that call `build_failed` with the cause and retries
//! on the next call; so must the daemon.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    build_after, client_settings, fail_after, fixture, scripted, status, wait_until, Gate,
};
use super::super::wire_support::{close, connect, ended, Client, CALLEES, INIT, INITED};
use crate::mcp::framing::UnixStreamTransport;
use crate::mcp::lazy::{LazySessionProvider, LazyTestHooks};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::transport::serve_connection;
use crate::mcp::{RefreshPolicy, StartupMode};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// The reviewer's probe, which failed on S3a's code: both sides' startup builds fail before the
/// client's first call; the client then sends two calls. The daemon's bytes must be the
/// in-process server's: `build_failed` with the cause, then the retry's `warming`. Rules out a
/// first call that starts the retry and is answered `warming`, the failure never reported.
#[test]
fn a_warm_daemon_whose_startup_build_failed_before_the_first_call_reports_it_like_the_in_process_server(
) {
    let settings = client_settings(0);

    let (_in_dir, mut in_cfg) = fixture(RefreshPolicy::WarnOnly);
    in_cfg.startup = StartupMode::Background;
    let in_gate = Gate::closed();
    let in_retry = Gate::closed();
    let (in_builder, _in_builds) = scripted(
        &in_cfg,
        vec![
            fail_after(&in_gate, "the repository could not be loaded"),
            build_after(&in_retry, &in_cfg),
        ],
    );
    let delivered = Arc::new(AtomicU64::new(0));
    let hooks = LazyTestHooks {
        published: Some({
            let delivered = Arc::clone(&delivered);
            Arc::new(move || {
                delivered.fetch_add(1, Ordering::SeqCst);
            })
        }),
        before_wait: None,
    };
    let (near, far) = UnixStream::pair().expect("a connected socket pair");
    let output = *settings.output();
    let gate2 = Arc::clone(&in_gate);
    let delivered2 = Arc::clone(&delivered);
    let in_process = std::thread::spawn(move || {
        let mut provider =
            LazySessionProvider::with_builder_and_hooks(&in_cfg, Duration::ZERO, in_builder, hooks)
                .expect("a lazy provider");
        gate2.open();
        wait_until("the startup build to fail", || {
            delivered2.load(Ordering::SeqCst) == 1
        });
        let mut transport = UnixStreamTransport::from_stream(far).expect("a transport");
        serve_connection(
            &mut provider,
            &ToolRegistry::all_v1(),
            &mut transport,
            &output,
        )
    });
    let mut client = Client::new(near);
    let mut expected = String::new();
    client.send(INIT);
    expected.push_str(&client.recv_line());
    client.send(INITED);
    client.send(CALLEES);
    let failed = client.recv_line();
    assert!(failed.contains("build_failed"), "{failed}");
    expected.push_str(&failed);
    client.send(CALLEES);
    let retried = client.recv_line();
    assert!(retried.contains("warming"), "{retried}");
    expected.push_str(&retried);
    drop(client);
    in_retry.open();
    ended(in_process).expect("the in-process server ends cleanly");

    let (_d_dir, d_cfg) = fixture(RefreshPolicy::WarnOnly);
    let d_gate = Gate::closed();
    let d_retry = Gate::closed();
    let (d_builder, _d_builds) = scripted(
        &d_cfg,
        vec![
            fail_after(&d_gate, "the repository could not be loaded"),
            build_after(&d_retry, &d_cfg),
        ],
    );
    let state = DaemonState::new(d_builder, StartupMode::Background).expect("a warm daemon");
    d_gate.open();
    wait_until("the startup build to fail", || {
        status(&state).state == BuildState::Failed
    });
    let (mut client, worker) = connect(&state, settings);
    let mut actual = String::new();
    client.send(INIT);
    actual.push_str(&client.recv_line());
    client.send(INITED);
    client.send(CALLEES);
    actual.push_str(&client.recv_line());
    client.send(CALLEES);
    actual.push_str(&client.recv_line());
    close(client, worker);
    d_retry.open();
    assert_eq!(actual, expected);
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t2-red.log" 2>&1
```

Expected: seven compile errors, among them `error[E0599]: no function or associated item named `joined_at_startup` found for struct `budget::ConnectionBudget``, `error[E0599]: no method named `budget_for` found for struct `std::sync::Arc<coordinator::DaemonState>``, and `error[E0433]: failed to resolve: use of undeclared type `StartupMode`` (the new admission test names it through `use super::*`; `admission.rs` imports it in Step 3).

- [ ] **Step 3: Implement**

In `src/mcp/daemon/admission.rs`, replace this:

````rust
//! Every function here is an entry point into the control plane and takes the lock through
//! `DaemonState::control_plane`, which makes the control plane's panic guard right after the
//! lock (D47): a panic or a poisoned lock in any of them ends the daemon.
use super::control::{BuildPhase, BuildStatus, StopReason};
use super::coordinator::{DaemonState, Poisoned};
use std::time::{Duration, Instant};

/// What the handshake stage does with a connection it has just accepted.
````

with this:

````rust
//! Every function here is an entry point into the control plane and takes the lock through
//! `DaemonState::control_plane`, which makes the control plane's panic guard right after the
//! lock (D47): a panic or a poisoned lock in any of them ends the daemon.
use super::budget::ConnectionBudget;
use super::control::{AttemptOutcome, BuildPhase, BuildStatus, StopReason};
use super::coordinator::{DaemonState, Poisoned};
use crate::mcp::settings::ClientSettings;
use crate::mcp::StartupMode;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What the handshake stage does with a connection it has just accepted.
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
            draining: control.admission.draining,
        })
    }
}

#[cfg(test)]
````

with this:

````rust
            draining: control.admission.draining,
        })
    }

    /// The first-call budget of a connection that is about to be served. A connection of a warm
    /// daemon (`StartupMode::Background`) whose startup attempt is still the only attempt is
    /// given that attempt, so that its first call hears it fail as the in-process server's
    /// first call does, instead of starting the retry (S3a's parity requirement for S4). Every
    /// other connection starts with an empty budget.
    pub(super) fn budget_for(&self, client: &ClientSettings) -> Result<ConnectionBudget, Poisoned> {
        let control = self.control_plane()?;
        if self.startup_mode() != StartupMode::Background || control.attempts != 1 {
            return Ok(ConnectionBudget::new(client));
        }
        let startup = match &control.build {
            BuildPhase::Building {
                attempt: 1,
                outcome,
                ..
            } => Arc::clone(outcome),
            BuildPhase::Failed { attempt: 1, error } => {
                let outcome = Arc::new(AttemptOutcome::default());
                outcome.fill(Err(error.clone()));
                outcome
            }
            _ => return Ok(ConnectionBudget::new(client)),
        };
        Ok(ConnectionBudget::joined_at_startup(client, startup))
    }
}

#[cfg(test)]
````

In `src/mcp/daemon/budget.rs`, replace this:

````rust
/// The attempt a connection last joined.
struct Joined {
    attempt: u64,
    /// This connection's deadline for the attempt.
    deadline: Instant,
    /// The attempt's outcome, shared with the coordinator and every call that joined it.
    outcome: Arc<AttemptOutcome>,
    /// Whether a call of this connection has been told how the attempt ended.
````

with this:

````rust
/// The attempt a connection last joined.
struct Joined {
    attempt: u64,
    /// This connection's deadline for the attempt; `None` until the connection's first call
    /// during it, for a connection that was given the startup attempt at admission.
    deadline: Option<Instant>,
    /// The attempt's outcome, shared with the coordinator and every call that joined it.
    outcome: Arc<AttemptOutcome>,
    /// Whether a call of this connection has been told how the attempt ended.
````

In `src/mcp/daemon/budget.rs`, replace this:

````rust
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

````

with this:

````rust
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

    /// The budget of a connection admitted while a warm daemon's startup attempt (attempt 1,
    /// started by `--warm-at-startup` before any connection existed) is its only one: the
    /// connection holds that attempt as if it had joined it, without a deadline until its first
    /// call. Its first call then hears that attempt's failure, if it failed, instead of starting
    /// the retry: the in-process server's `build_failed`, then the retry on the next call.
    pub(super) fn joined_at_startup(client: &ClientSettings, outcome: Arc<AttemptOutcome>) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: Some(Joined {
                attempt: 1,
                deadline: None,
                outcome,
                told: false,
            }),
        }
    }

````

In `src/mcp/daemon/budget.rs`, replace this:

````rust
        outcome: &Arc<AttemptOutcome>,
        now: Instant,
    ) -> Instant {
        if let Some(joined) = &self.joined {
            if joined.attempt == attempt {
                return joined.deadline;
            }
        }
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        self.joined = Some(Joined {
            attempt,
            deadline,
            outcome: Arc::clone(outcome),
            told: false,
        });
````

with this:

````rust
        outcome: &Arc<AttemptOutcome>,
        now: Instant,
    ) -> Instant {
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        if let Some(joined) = &mut self.joined {
            if joined.attempt == attempt {
                return *joined.deadline.get_or_insert(deadline);
            }
        }
        self.joined = Some(Joined {
            attempt,
            deadline: Some(deadline),
            outcome: Arc::clone(outcome),
            told: false,
        });
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let settings: ConnectionSettings = *client.output();
    let mut budget = ConnectionBudget::new(client);
    let startup = state.startup_mode();
    serve_loop(transport, |message, lifecycle| {
        match handle_control(message, lifecycle, registry, startup, &settings) {
````

with this:

````rust
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let settings: ConnectionSettings = *client.output();
    // A warm daemon's connection may start holding the startup attempt (`budget_for`).
    let mut budget = state.budget_for(client)?;
    let startup = state.startup_mode();
    serve_loop(transport, |message, lifecycle| {
        match handle_control(message, lifecycle, registry, startup, &settings) {
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t2.log" 2>&1
```

Expected: `test result: ok. 96 passed; 0 failed` (92 + 4), no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/budget.rs src/mcp/daemon/budget_tests.rs src/mcp/daemon/admission.rs src/mcp/daemon/admission_tests.rs src/mcp/daemon/connection.rs src/mcp/daemon/connection_background_tests.rs
git commit -m "feat(mcp): a warm daemon reports a failed startup build as the in-process server does"
```

- [ ] **Step 6: Prove the tests bite**

| Mutant | Edit | Fails |
|---|---|---|
| 2A | in `connection.rs`, `let mut budget = state.budget_for(client)?;` → `let mut budget = super::budget::ConnectionBudget::new(client);` (the code before this task) | `a_warm_daemon_whose_startup_build_failed_before_the_first_call_reports_it_like_the_in_process_server`: the daemon's second line is `warming` where the in-process server's is `build_failed` with `"cause":"the repository could not be loaded"` |
| 2B | in `budget_for`, replace the `BuildPhase::Failed { attempt: 1, error } => { … }` arm with `BuildPhase::Failed { attempt: 1, .. } => Arc::new(AttemptOutcome::default()),` | the parity test and `only_a_warm_daemon_s_startup_attempt_is_given_to_a_new_connection` |

### Task 3: The key's files: the owner lock, the socket and the pid file

**Files:**
- Create: `src/mcp/daemon/files.rs`, `src/mcp/daemon/files_tests.rs`
- Modify: `src/mcp/daemon/mod.rs`

**Interfaces:**
- Consumes: `shared::runtime_dir::{validate_runtime_dir, current_uid, RuntimeDirError}`, `shared::paths::{DaemonPaths, SocketPathTooLong}`, `shared::identity::IdentityKey` (S2).
- Produces: `files::FilesError` (`Display`, `std::error::Error`); `files::Claim { Owned(Box<DaemonFiles>), Taken }`; `DaemonFiles::claim(runtime_dir: &Path, key: &IdentityKey) -> Result<Claim, FilesError>`, `paths(&self) -> &DaemonPaths`, `bind(&mut self) -> Result<UnixListener, FilesError>`, `write_pid(&mut self, pid: u32, start_time: u64, build_identity: &str) -> Result<(), FilesError>`, `still_ours(&self) -> bool`, `unlink_socket(&self)`, `release(self)`; `#[cfg(test)] files::is_socket(path: &Path) -> bool`.

Spec test 3 is "Stale socket: bind a fixture socket with no owner lock, close its listener, then start the shim → replaced", and test 4 "Live-but-wedged owner: a fixture holds the owner lock and accepts without answering the preamble → the shim fails closed naming the pid, and the socket is **not** unlinked". The daemon's half of each is here (and in a process in Task 6); the shim's is S5a's.

- [ ] **Step 1: Write the failing tests and register the module**

Create `src/mcp/daemon/files_tests.rs`:

````rust
//! Tests for the daemon's files (spec §3.1 "Starting", §4.3 "Files", §4.6; the daemon's half of
//! spec §9 tests 3 and 4). Every test uses a runtime directory of its own under `/tmp`, short
//! enough for a socket path (S2's note for S4).
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;

/// A runtime directory of this test's own, and a key for it.
fn runtime() -> (tempfile::TempDir, PathBuf, IdentityKey) {
    let base = tempfile::Builder::new()
        .prefix("s4")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let dir = base.path().join("prism-mcpd");
    let key = IdentityKey::from_hex(&"ab".repeat(32)).expect("a key");
    (base, dir, key)
}

fn owned(dir: &Path, key: &IdentityKey) -> DaemonFiles {
    match DaemonFiles::claim(dir, key).expect("the files can be claimed") {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the key was taken"),
    }
}

fn taken(dir: &Path, key: &IdentityKey) -> bool {
    matches!(
        DaemonFiles::claim(dir, key).expect("the files can be claimed"),
        Claim::Taken
    )
}

fn inode(path: &Path) -> u64 {
    std::fs::symlink_metadata(path).expect("lstat").ino()
}

/// Spec §3.1: the owner lock decides who owns a key. A second claim, from another descriptor
/// (as a second daemon process would make), is told the key is taken, and once the owner is
/// gone the key can be claimed again. Rules out a claim that does not lock, and a lock that
/// outlives its holder.
#[test]
fn the_first_claim_owns_the_key_and_a_second_is_told_it_is_taken() {
    let (_base, dir, key) = runtime();
    let first = owned(&dir, &key);
    assert!(taken(&dir, &key));
    assert!(taken(&dir, &key), "and again");
    first.release();
    let again = owned(&dir, &key);
    drop(again);
    let _third = owned(&dir, &key);
}

/// Spec §4.1 step 6 and §9 test 4, the daemon's half: a daemon that finds the owner lock held
/// leaves the live owner's socket and pid file alone, and the owner goes on accepting. Rules out
/// a starting daemon that clears the socket before it knows it owns the key.
#[test]
fn a_daemon_that_finds_the_key_taken_touches_no_file() {
    let (_base, dir, key) = runtime();
    let mut live = owned(&dir, &key);
    let listener = live.bind().expect("bind");
    live.write_pid(4242, 17, "build").expect("pid");
    let socket = live.paths().socket().to_path_buf();
    let pid_file = live.paths().pid_file().to_path_buf();
    let (socket_inode, pid_before) = (
        inode(&socket),
        std::fs::read_to_string(&pid_file).expect("pid"),
    );

    assert!(taken(&dir, &key));

    assert_eq!(inode(&socket), socket_inode);
    assert_eq!(std::fs::read_to_string(&pid_file).expect("pid"), pid_before);
    let _client = UnixStream::connect(&socket).expect("the live socket still answers");
    listener.accept().expect("the live owner accepts");
}

/// Spec §4.6 "Daemon crashes" and §9 test 3, the daemon's half: a socket left by a daemon that
/// died is replaced by the next owner, which then accepts on it. Rules out a bind that fails on
/// the leftover file.
#[test]
fn binding_replaces_a_stale_socket_left_by_a_dead_daemon() {
    let (_base, dir, key) = runtime();
    let socket = {
        let files = owned(&dir, &key);
        let socket = files.paths().socket().to_path_buf();
        drop(UnixListener::bind(&socket).expect("a socket nobody listens on"));
        drop(files);
        socket
    };
    // Only the file is checked: on macOS a child another test spawns while the listener is being
    // made close-on-exec can inherit it and keep it accepting.
    assert!(is_socket(&socket), "the stale socket is there");

    let mut files = owned(&dir, &key);
    let listener = files.bind().expect("bind over the stale socket");
    let _client = UnixStream::connect(&socket).expect("the new socket answers");
    listener.accept().expect("the new owner accepts");
}

/// Spec §4.3: lock files are created 0600 and never removed, not even by the owner when it
/// stops.
#[test]
fn the_lock_file_is_0600_and_stays_after_release() {
    let (_base, dir, key) = runtime();
    let files = owned(&dir, &key);
    let owner_lock = files.paths().owner_lock().to_path_buf();
    let mode = std::fs::metadata(&owner_lock)
        .expect("the lock file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let before = inode(&owner_lock);
    files.release();
    assert_eq!(inode(&owner_lock), before, "the same file, still there");
}

/// Spec §3.1: the pid file holds `pid\nstart_time\nbuild_identity\n` while the daemon lives and
/// is removed by the owner when it stops; the owner lock is free afterwards.
#[test]
fn the_pid_file_says_who_owns_the_key_and_is_removed_at_release() {
    let (_base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    files
        .write_pid(4242, 1_700_000_000, "0123abcd")
        .expect("pid");
    let pid_file = files.paths().pid_file().to_path_buf();
    assert_eq!(
        std::fs::read_to_string(&pid_file).expect("pid"),
        "4242\n1700000000\n0123abcd\n"
    );
    let mode = std::fs::metadata(&pid_file)
        .expect("pid")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);

    files.release();
    assert!(!pid_file.exists());
    let _again = owned(&dir, &key);
}

/// A symlink where a lock file should be is refused, whether it dangles or names a file this
/// user owns, and what it points at is neither created nor locked. Rules out an `open` that
/// follows it, which would let whoever placed it choose the file the daemon locks and writes.
#[test]
fn a_symlink_in_place_of_the_owner_lock_is_refused() {
    let (base, dir, key) = runtime();
    validate_runtime_dir(&dir).expect("create the runtime directory");
    let paths = DaemonPaths::new(&dir, &key).expect("paths");

    let dangling = base.path().join("dangling");
    std::os::unix::fs::symlink(&dangling, paths.owner_lock()).expect("symlink");
    let refused = DaemonFiles::claim(&dir, &key);
    assert!(
        matches!(refused, Err(FilesError::Io { op: "open", .. })),
        "{:?}",
        refused.err()
    );
    assert!(!dangling.exists());

    let existing = base.path().join("existing");
    std::fs::write(&existing, "untouched").expect("a file of this user");
    std::fs::remove_file(paths.owner_lock()).expect("remove the symlink");
    std::os::unix::fs::symlink(&existing, paths.owner_lock()).expect("symlink");
    let refused = DaemonFiles::claim(&dir, &key);
    assert!(
        matches!(refused, Err(FilesError::Io { op: "open", .. })),
        "{:?}",
        refused.err()
    );
    let target = File::open(&existing).expect("the target");
    target.try_lock().expect("the target is not locked");
    assert_eq!(
        std::fs::read_to_string(&existing).expect("read"),
        "untouched"
    );
}

/// A runtime directory that fails validation is refused before any file is created in it.
#[test]
fn an_invalid_runtime_directory_is_refused_before_anything_is_created() {
    let (_base, dir, key) = runtime();
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let refused = DaemonFiles::claim(&dir, &key);
    assert!(
        matches!(refused, Err(FilesError::RuntimeDir(_))),
        "{:?}",
        refused.err()
    );
    assert_eq!(std::fs::read_dir(&dir).expect("list").count(), 0);
}

/// The socket is removed only while the entry of its name is the one this daemon bound. Rules
/// out removing a socket that someone else put there since.
#[test]
fn the_socket_is_removed_only_if_it_is_still_the_one_this_daemon_bound() {
    let (_base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    let _listener = files.bind().expect("bind");
    let socket = files.paths().socket().to_path_buf();
    files.unlink_socket();
    assert!(!socket.exists(), "its own socket is removed");

    let _listener = files.bind().expect("bind again");
    std::fs::remove_file(&socket).expect("remove");
    let _other = UnixListener::bind(&socket).expect("another socket at the path");
    let other_inode = inode(&socket);
    files.unlink_socket();
    assert_eq!(inode(&socket), other_inode, "another socket is left alone");
}

/// Lane README, notes for S4: when the runtime directory is replaced under a live daemon, a
/// second daemon could take the key in the new directory. The daemon notices that its files are
/// no longer the ones at their paths, and what it removes as it stops is removed from its own
/// directory, not from the new one.
#[test]
fn a_replaced_runtime_directory_is_noticed_and_left_alone() {
    let (base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    let _listener = files.bind().expect("bind");
    files.write_pid(1, 2, "build").expect("pid");
    assert!(files.still_ours());

    let moved = base.path().join("moved");
    std::fs::rename(&dir, &moved).expect("move the runtime directory away");
    assert!(!files.still_ours(), "the directory is gone");
    let mut newcomer = owned(&dir, &key);
    let _new_listener = newcomer.bind().expect("bind");
    newcomer.write_pid(3, 4, "build").expect("pid");
    assert!(
        !files.still_ours(),
        "another daemon's files are at the paths"
    );

    files.unlink_socket();
    files.release();
    let paths = newcomer.paths().clone();
    assert!(is_socket(paths.socket()), "the newcomer's socket stays");
    assert_eq!(
        std::fs::read_to_string(paths.pid_file()).expect("pid"),
        "3\n4\nbuild\n",
        "the newcomer's pid file stays"
    );
    let old = DaemonPaths::new(&moved, &key).expect("paths");
    assert!(!old.socket().exists(), "its own socket is removed");
    assert!(!old.pid_file().exists(), "its own pid file is removed");
}
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod control;
mod coordinator;
mod fatal;
#[cfg(test)]
mod test_support;
#[cfg(test)]
````

with this:

````rust
mod control;
mod coordinator;
mod fatal;
mod files;
#[cfg(test)]
mod test_support;
#[cfg(test)]
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t3-red.log" 2>&1
```

Expected: `error[E0583]: file not found for module `files``.

- [ ] **Step 3: Implement**

Create `src/mcp/daemon/files.rs`:

````rust
//! A daemon's files in the runtime directory (spec §3.1 "Starting", §4.3 "Files"): the owner
//! lock, held for the daemon's whole life; the socket; the pid file.
//!
//! Who owns a key is decided by one lock: `<key16>.owner`, taken with `flock(LOCK_EX|LOCK_NB)`.
//! A daemon that cannot take it is not the owner and touches nothing (spec §3.1: "failure =
//! another live daemon owns this key → exit 0 silently"). The owner, and only the owner,
//! replaces a stale socket, writes the pid file and removes both. Lock files are created 0600
//! and never removed: removing one is how two holders of one lock come to exist.
//!
//! The runtime directory is validated (`validate_runtime_dir`) and then opened once; every file
//! but the socket is opened, examined and removed through that descriptor, with `O_NOFOLLOW`, so
//! a directory swapped under the daemon after the check is not the one it writes into or
//! removes from. `bind` takes a path, so the socket is bound by path (S2's SI-5); it is removed
//! through the descriptor, and only while the entry is still the one this daemon bound. Like
//! the directory checks, this is hardening; the isolation guarantee between users is the peer
//! uid check on every connection.
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::{DaemonPaths, SocketPathTooLong};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir, RuntimeDirError};
use std::ffi::CString;
use std::fmt;
use std::fs::{File, TryLockError};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

/// The mode lock files and the pid file are created with.
const PRIVATE_FILE_MODE: libc::c_uint = 0o600;

/// Why a daemon could not take or use its files. Each ends the daemon's start with a message;
/// none is retried.
#[derive(Debug)]
pub(super) enum FilesError {
    RuntimeDir(RuntimeDirError),
    SocketPath(SocketPathTooLong),
    /// The filesystem refused `op` on `path`.
    Io {
        op: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// What is at `path` is not what this daemon may use: `why` says what it is instead.
    Unusable {
        path: PathBuf,
        why: &'static str,
    },
}

impl fmt::Display for FilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeDir(error) => error.fmt(f),
            Self::SocketPath(error) => error.fmt(f),
            Self::Io { op, path, source } => {
                write!(f, "cannot {op} {}: {source}", path.display())
            }
            Self::Unusable { path, why } => write!(f, "{} {why}", path.display()),
        }
    }
}

impl std::error::Error for FilesError {}

impl From<RuntimeDirError> for FilesError {
    fn from(error: RuntimeDirError) -> Self {
        Self::RuntimeDir(error)
    }
}

impl From<SocketPathTooLong> for FilesError {
    fn from(error: SocketPathTooLong) -> Self {
        Self::SocketPath(error)
    }
}

/// The outcome of trying to own a key.
pub(super) enum Claim {
    /// This process holds the owner lock until the `DaemonFiles` is released or dropped.
    Owned(Box<DaemonFiles>),
    /// Another process holds it: a daemon for this key is alive (starting, serving or
    /// draining). Nothing was created but the lock file, if it was missing.
    Taken,
}

/// Which file a path or a descriptor names: device and inode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileId {
    dev: u64,
    ino: u64,
}

impl FileId {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
        }
    }
}

/// The files of the daemon that owns a key. Dropping it releases the owner lock; `release`
/// first removes the pid file.
pub(super) struct DaemonFiles {
    paths: DaemonPaths,
    dir: OwnedFd,
    owner: File,
    owner_id: FileId,
    socket_id: Option<FileId>,
    pid_contents: Option<String>,
}

impl DaemonFiles {
    /// Validate `runtime_dir`, then try to take the owner lock of `key` in it.
    pub(super) fn claim(runtime_dir: &Path, key: &IdentityKey) -> Result<Claim, FilesError> {
        validate_runtime_dir(runtime_dir)?;
        let paths = DaemonPaths::new(runtime_dir, key)?;
        let dir = open_runtime_dir(paths.runtime_dir())?;
        let owner = open_private(&dir, paths.owner_lock(), libc::O_RDWR, Create::Yes)?;
        let owner_id = FileId::of(&owner.metadata().map_err(|source| FilesError::Io {
            op: "inspect",
            path: paths.owner_lock().to_path_buf(),
            source,
        })?);
        match owner.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Ok(Claim::Taken),
            Err(TryLockError::Error(source)) => {
                return Err(FilesError::Io {
                    op: "lock",
                    path: paths.owner_lock().to_path_buf(),
                    source,
                })
            }
        }
        Ok(Claim::Owned(Box::new(Self {
            paths,
            dir,
            owner,
            owner_id,
            socket_id: None,
            pid_contents: None,
        })))
    }

    pub(super) fn paths(&self) -> &DaemonPaths {
        &self.paths
    }

    /// Bind the daemon's socket. Whatever is at its path is removed first: a socket there was
    /// left by a daemon that died (the owner lock this daemon holds says no other is alive), and
    /// only the owner removes it (spec §4.6 "Daemon crashes").
    pub(super) fn bind(&mut self) -> Result<UnixListener, FilesError> {
        let socket = self.paths.socket().to_path_buf();
        match unlink_at(&self.dir, &socket) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(FilesError::Io {
                    op: "remove the stale socket",
                    path: socket,
                    source,
                })
            }
        }
        let listener = UnixListener::bind(&socket).map_err(|source| FilesError::Io {
            op: "bind",
            path: socket.clone(),
            source,
        })?;
        let bound = stat_at(&self.dir, &socket).map_err(|source| FilesError::Io {
            op: "inspect",
            path: socket.clone(),
            source,
        })?;
        self.socket_id = Some(bound);
        Ok(listener)
    }

    /// Write the pid file: `pid\nstart_time\nbuild_identity\n` (spec §3.1).
    pub(super) fn write_pid(
        &mut self,
        pid: u32,
        start_time: u64,
        build_identity: &str,
    ) -> Result<(), FilesError> {
        let path = self.paths.pid_file().to_path_buf();
        let contents = format!("{pid}\n{start_time}\n{build_identity}\n");
        let mut file = open_private(
            &self.dir,
            &path,
            libc::O_WRONLY | libc::O_TRUNC,
            Create::Yes,
        )?;
        file.write_all(contents.as_bytes())
            .map_err(|source| FilesError::Io {
                op: "write",
                path,
                source,
            })?;
        self.pid_contents = Some(contents);
        Ok(())
    }

    /// Whether the owner lock and the socket that the paths name are still the ones this
    /// daemon holds and bound. Not, when the runtime directory was removed or replaced under
    /// it: another daemon could then take the key's lock in the new directory (lane README,
    /// notes for S4).
    pub(super) fn still_ours(&self) -> bool {
        let same = |path: &Path, id: FileId| {
            std::fs::symlink_metadata(path).is_ok_and(|metadata| FileId::of(&metadata) == id)
        };
        same(self.paths.owner_lock(), self.owner_id)
            && self
                .socket_id
                .is_none_or(|id| same(self.paths.socket(), id))
    }

    /// Remove the socket, if the entry of its name in this daemon's directory is still the one
    /// this daemon bound. Errors are ignored: the daemon is stopping, and a socket left behind
    /// is replaced by the next owner.
    pub(super) fn unlink_socket(&self) {
        let socket = self.paths.socket();
        if self
            .socket_id
            .is_some_and(|id| stat_at(&self.dir, socket).is_ok_and(|found| found == id))
        {
            let _ = unlink_at(&self.dir, socket);
        }
    }

    /// Remove the pid file if it still holds what this daemon wrote, then release the owner
    /// lock (spec §3.1: "the pid file is removed, the owner lock released, exit 0").
    pub(super) fn release(self) {
        if let Some(written) = &self.pid_contents {
            let pid_file = self.paths.pid_file();
            let mut found = String::new();
            let ours = open_private(&self.dir, pid_file, libc::O_RDONLY, Create::No)
                .and_then(|mut file| {
                    file.read_to_string(&mut found)
                        .map_err(|source| FilesError::Io {
                            op: "read",
                            path: pid_file.to_path_buf(),
                            source,
                        })
                })
                .is_ok_and(|_| found == *written);
            if ours {
                let _ = unlink_at(&self.dir, pid_file);
            }
        }
        // Closing the only descriptor of the lock file releases the lock.
        drop(self.owner);
    }
}

/// The last component of `path`, as a C string, for the `*at` calls.
fn entry_name(path: &Path) -> io::Result<CString> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    CString::new(name.as_bytes()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
}

/// Open the runtime directory itself, not a symlink in its place, and check that what was
/// opened is a directory this user owns with mode 0700 (the validation's facts, now about the
/// descriptor every later file operation goes through).
fn open_runtime_dir(path: &Path) -> Result<OwnedFd, FilesError> {
    let io_error = |source| FilesError::Io {
        op: "open",
        path: path.to_path_buf(),
        source,
    };
    let name = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io_error(io::Error::from(io::ErrorKind::InvalidInput)))?;
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
    let fd = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io_error(io::Error::last_os_error()));
    }
    // SAFETY: `fd` was just returned by `open` and nothing else owns it.
    let dir = unsafe { OwnedFd::from_raw_fd(fd) };
    let file = File::from(dir);
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_dir() || metadata.uid() != current_uid() || metadata.mode() & 0o777 != 0o700 {
        return Err(FilesError::Unusable {
            path: path.to_path_buf(),
            why: "changed after it was validated: it is no longer a 0700 directory of this user",
        });
    }
    Ok(OwnedFd::from(file))
}

/// Whether `open_private` creates the file when it is missing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Create {
    Yes,
    No,
}

/// Open the entry of `path`'s name in `dir`, never through a symlink, creating it 0600 if
/// `create` says so; refuse anything but a regular file this user owns.
///
/// A missing file is created with `O_EXCL`, and an existing one opened without `O_CREAT`. A
/// plain `O_CREAT` is not used: on macOS it fails now and then with `ENOENT` when another
/// process creates the same name at the same moment, which is exactly what daemons racing for
/// one key do (D53; measured in validation, about 1 in 2,000 eight-way races).
fn open_private(
    dir: &OwnedFd,
    path: &Path,
    flags: libc::c_int,
    create: Create,
) -> Result<File, FilesError> {
    let io_error = |op, source| FilesError::Io {
        op,
        path: path.to_path_buf(),
        source,
    };
    let name = entry_name(path).map_err(|source| io_error("open", source))?;
    let open = |flags: libc::c_int| {
        // SAFETY: `dir` is an open directory descriptor and `name` a valid NUL-terminated
        // string, both alive for the call; the mode is passed as the variadic `c_uint` `openat`
        // reads.
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                PRIVATE_FILE_MODE,
            )
        };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            // SAFETY: `fd` was just returned by `openat` and nothing else owns it.
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    };
    let opened = match create {
        Create::Yes => match open(flags | libc::O_CREAT | libc::O_EXCL) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => open(flags),
            created => created,
        },
        Create::No => open(flags),
    };
    let file = opened.map_err(|source| io_error("open", source))?;
    let metadata = file
        .metadata()
        .map_err(|source| io_error("inspect", source))?;
    if !metadata.file_type().is_file() || metadata.uid() != current_uid() {
        return Err(FilesError::Unusable {
            path: path.to_path_buf(),
            why: "is not a regular file owned by this user",
        });
    }
    Ok(file)
}

/// `fstatat` of `path`'s name in `dir`, not following a symlink.
fn stat_at(dir: &OwnedFd, path: &Path) -> io::Result<FileId> {
    let name = entry_name(path)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `dir` is an open directory descriptor, `name` a valid NUL-terminated string and
    // `stat` points to writable memory of the right size, all alive for the call.
    let result = unsafe {
        libc::fstatat(
            dir.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fstatat` returned 0, so it filled `stat`.
    let stat = unsafe { stat.assume_init() };
    // The field types differ between platforms (`dev_t` is `i32` on macOS); both are
    // reinterpreted the way `std`'s `MetadataExt` does, so ids from both sources compare.
    #[allow(clippy::unnecessary_cast)]
    Ok(FileId {
        dev: stat.st_dev as u64,
        ino: stat.st_ino as u64,
    })
}

/// `unlinkat` of `path`'s name in `dir`.
fn unlink_at(dir: &OwnedFd, path: &Path) -> io::Result<()> {
    let name = entry_name(path)?;
    // SAFETY: `dir` is an open directory descriptor and `name` a valid NUL-terminated string,
    // both alive for the call.
    let result = unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Whether `path` names a socket. Tests only.
#[cfg(test)]
pub(super) fn is_socket(path: &Path) -> bool {
    use std::os::unix::fs::FileTypeExt;
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_socket())
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t3.log" 2>&1
```

Expected: `test result: ok. 105 passed; 0 failed` (96 + 9), no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/files.rs src/mcp/daemon/files_tests.rs src/mcp/daemon/mod.rs
git commit -m "feat(mcp): the daemon's owner lock, socket and pid file"
```

- [ ] **Step 6: Prove the tests bite**

Run `cargo test --features mcp --lib mcp::daemon::files` under each mutant.

| Mutant | Edit | Fails |
|---|---|---|
| 3A | in `claim`, `match owner.try_lock() {` → `match Ok::<(), TryLockError>(()) {` | `the_first_claim_owns_the_key_and_a_second_is_told_it_is_taken`, `a_daemon_that_finds_the_key_taken_touches_no_file` |
| 3B | in `bind`, `match unlink_at(&self.dir, &socket) {` → `match Err::<(), io::Error>(io::ErrorKind::NotFound.into()) {` | `binding_replaces_a_stale_socket_left_by_a_dead_daemon` |
| 3C | in `open_private`, drop `libc::O_NOFOLLOW | ` | `a_symlink_in_place_of_the_owner_lock_is_refused` |
| 3D | in `unlink_socket`, `.is_some_and(\|id\| stat_at(&self.dir, socket).is_ok_and(\|found\| found == id))` → `.is_some_and(\|_id\| true)` | `the_socket_is_removed_only_if_it_is_still_the_one_this_daemon_bound` |
| 3E | make `still_ours` return `true` first | `a_replaced_runtime_directory_is_noticed_and_left_alone` |

### Task 4: S4-1 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
that stops reading stalls only its own connection. A panic anywhere else on a connection's
thread (framing, the readiness answers, writing a response) ends that connection only:
`serve_bounded` turns it into an error and the daemon goes on.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

with this:

````markdown
that stops reading stalls only its own connection. A panic anywhere else on a connection's
thread (framing, the readiness answers, writing a response) ends that connection only:
`serve_bounded` turns it into an error and the daemon goes on.
Admission and retirement (`admission.rs`) are entries of the control plane: each takes the lock
through `DaemonState::control_plane`, whose `ControlPlane` makes the panic guard right after the
lock. Pending and admitted connections are counted together against `--max-clients`; the idle
time counts from the moment both reach zero (or from the first decision, for a daemon nobody
reaches); the drain decision is taken once (idle TTL passed with no build running, or forced)
and the socket's unlink runs inside it, so nothing is admitted after it. A warm daemon's
connection starts holding the startup attempt (`budget_for`), so a startup build that failed
before its first call is reported as the in-process server reports it. `files.rs` owns a key's
files: the owner lock (`flock`, held for the daemon's life; a second claimant is told `Taken` and
touches nothing), the socket (a stale one is replaced by the owner, and the owner removes it only
while it is still the one it bound), and the pid file; every file but the socket is opened
through the validated runtime directory's descriptor with `O_NOFOLLOW`, and lock files are
created with `O_EXCL` and never removed.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check > "$LOG_DIR/s41-fmt.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp:: > "$LOG_DIR/s41-lib.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/s41-mcp.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check > "$LOG_DIR/s41-check.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check --features mcp > "$LOG_DIR/s41-check-mcp.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo clippy --features mcp --all-targets > "$LOG_DIR/s41-clippy.log" 2>&1
```

Expected: `s41-fmt.log` empty; `test result: ok. 491 passed; 0 failed` (469 + 22); `23 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; both checks `Finished` and no `warning` in the feature check; clippy prints the base's 257 `warning` lines and none naming `src/mcp/daemon/`.

- [ ] **Step 3: State the invariants**

```bash
git diff --stat origin/main -- src/mcp/transport.rs src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs src/mcp/session.rs src/mcp/lazy.rs src/mcp/framing.rs src/mcp/output.rs src/mcp/settings.rs src/mcp/shared tests/fixtures src/bin | tail -1
git diff --name-only origin/main | grep -v '^src/mcp/daemon/' | grep -v '^CLAUDE.md$' | wc -l
wc -l src/mcp/daemon/*.rs | sort -n | tail -3
```

Expected: nothing printed by the first; `0`; no file over 600 lines (`coordinator.rs` 592 is the largest this slice edits).

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
for i in $(seq 1 20); do nice -n 19 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/s41-rep-$i.log" 2>&1; done
grep -h '^test result' "$LOG_DIR"/s41-rep-*.log | sed 's/finished.*//' | sort | uniq -c
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s41-full.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s41-doc.log" 2>&1
```

Expected: `20 test result: ok. 105 passed; 0 failed; …`; the full suite and the doc tests `0 failed`.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S4-1 puts admission, retirement and the key's files in place, testable without a socket server. (2) **Invariants:** Step 3. (3) **Decisions:** D48–D53, D58, D59, D65, D67, D70, SI-14, SI-15, SI-21. D50 (when the idle time starts) and D52 (forced drain) are the ones a reviewer is most likely to want to change. (4) **Claims:** the rows for Tasks 1–3, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base, and mutants 1A–3E as observed, including 1D's and 3C's first survival. (6) **Not verified:** Linux; the shim halves of tests 3 and 4. (7) **Tier-A:** not run (no file it watches is touched). (8) The owner questions of this plan, as defaults built.

```bash
git add CLAUDE.md
git commit -m "docs(mcp): admission, retirement and the key's files of the shared daemon"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): daemon ownership, admission and retirement (shared daemon S4-1)" --body-file "$LOG_DIR/s41-pr-body.md"
```

Review: Opus 5.5 and gpt-6.1-sol, same brief and commit; the brief states the lane goal, asks for WRONG/SMELL and MATERIAL/IMMATERIAL tags, a fix and alternatives per finding, and when each would not apply; cap two rounds (lane README).

---

# Part two: S4-2, the accept loop and the daemon process (Tasks 5–7)

S4-2 is based on S4-1's merge.

### Task 5: The accept loop and the bounded handshake stage

**Files:**
- Create: `src/mcp/daemon/peer.rs`, `src/mcp/daemon/log.rs`, `src/mcp/daemon/server.rs`, `src/mcp/daemon/server_support.rs`, `src/mcp/daemon/server_tests.rs`, `src/mcp/daemon/server_drain_tests.rs`
- Modify: `src/mcp/daemon/control.rs` (`StopReason::FilesReplaced`), `src/mcp/daemon/mod.rs`

**Interfaces:**
- Consumes: Task 1's admission functions and `StopReason`; Task 3's `DaemonFiles`; `connection::serve_admitted` (S3a; it does not unwind, S3b); `shared::preamble::{Hello, Reply, Ready, PreambleError}` and `shared::preamble_reader::{PreambleLineReader, PreambleReadError}` (S2); `shared::identity::{FsIdentity, IdentityKey}`.
- Produces: `peer::peer_uid(&UnixStream) -> io::Result<u32>`; `log::line(&str)`, `log::printable_prefix(&str, usize) -> String`; in `server.rs`: `HANDSHAKE_TIMEOUT: Duration` (10 s), `IDENTITY_CHECK_INTERVAL: Duration` (5 s), `struct ServeOptions { max_clients: u32, idle_ttl: Duration, handshake_timeout: Duration }`, `struct Served { key, repo_root, root_fs, pid, start_time }`, `struct Waker` (`wake()`, `#[cfg(test)] request_stop()`), `type PeerUid`, `struct Server` with `new(state, registry: Arc<ToolRegistry>, files: DaemonFiles, listener: UnixListener, served, options) -> io::Result<Server>`, `waker()`, `run(self) -> anyhow::Result<StopReason>`, and test seams `set_peer_uid`, `set_turn_hook`.

The loop's turn, in order: the test hook; `check()` (a stop request; every 5 s the root's identity and the files; the retirement decision), which drains and returns if any says so; `poll` until the first deadline (a handshake's, the idle deadline, the next check; at most 5 s); empty the wake socket; accept everything queued (peer check, then `enter_handshake`: count it, or answer `BUSY` or `RETIRING`); read every pending connection (a whole line is answered, a connection past its deadline or closed is dropped). Spec test 16 is "more than `--max-clients` connections stalled before `HELLO` → `BUSY` without worker allocation"; spec test 2 "Idle exit: injected clock; observe the disconnect, advance the clock past the TTL, assert Draining"; spec test 13 "a shim paused after `connect` and before `HELLO` while the daemon retires → the shim sees `RETIRING` or EOF, re-enters arbitration, and reaches `READY` on a fresh daemon within its original deadline" (the daemon's half: `RETIRING`, and a fresh daemon on the key admits it).

- [ ] **Step 1: Write the failing tests, their support, and register the modules**

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod coordinator;
mod fatal;
mod files;
#[cfg(test)]
mod test_support;
#[cfg(test)]
````

with this:

````rust
mod coordinator;
mod fatal;
mod files;
mod log;
mod peer;
mod server;
#[cfg(test)]
mod server_support;
#[cfg(test)]
mod test_support;
#[cfg(test)]
````

Create `src/mcp/daemon/server_drain_tests.rs`:

````rust
//! Tests for retirement and Draining in the accept loop (spec §3.1, §4.2 "Admission and
//! retirement", §4.3 root identity; §9 tests 2 and 13 as far as they run before the shim).
use super::super::control::StopReason;
use super::super::files::{is_socket, Claim, DaemonFiles};
use super::super::server_support::{
    gone, hello_line, options, read_all, runtime_base, send_line, served_key, start, start_with,
    TTL_SECS,
};
use super::super::test_support::{build_after, fixture, scripted, state_with, Gate, ManualClock};
use super::super::wire_support::CALLEES;
use super::IDENTITY_CHECK_INTERVAL;
use crate::mcp::shared::preamble::{BuildState, Reply};
use crate::mcp::RefreshPolicy;
use std::os::unix::net::UnixStream;

fn retiring() -> String {
    Reply::Retiring.encode()
}

/// Spec §9 test 2, with an injected clock: after the last client leaves, the daemon stays up
/// for the idle TTL and then drains: its socket and pid file are removed and its owner lock is
/// free for the next daemon. Rules out a TTL that counts from the daemon's start or the
/// client's arrival, and a drain that leaves the key owned.
#[test]
fn an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free() {
    let mut running = start(options(64), |_| {});
    let client = running.admitted();
    running.advance(2 * TTL_SECS);
    assert!(running.is_running(), "an admitted client holds it up");
    drop(client);
    super::super::test_support::wait_until("the client's admission to be released", || {
        running
            .state
            .admission_status()
            .is_ok_and(|status| status.admitted == 0)
    });
    running.advance(TTL_SECS - 1);
    assert!(running.is_running(), "one second short of the TTL");

    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
    assert!(
        UnixStream::connect(running.paths.socket()).is_err(),
        "nothing answers at the socket's path"
    );
    assert!(matches!(
        DaemonFiles::claim(&running.runtime_dir, &served_key()),
        Ok(Claim::Owned(_))
    ));
}

/// A daemon nobody ever reaches (its starter died before connecting) retires after the idle TTL
/// like any other: the count starts with the loop's first turn.
#[test]
fn a_daemon_nobody_reaches_retires_after_the_ttl() {
    let mut running = start(options(64), |_| {});
    running.settle();
    running.advance(TTL_SECS - 1);
    assert!(running.is_running());
    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// Spec §4.2: no retirement while a build runs, however long nobody has been connected.
#[test]
fn a_running_build_keeps_an_idle_daemon_up_until_it_ends() {
    let (repo_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (base, runtime_dir) = runtime_base();
    let mut running = start_with(
        state,
        clock,
        repo_dir,
        base,
        runtime_dir,
        options(64),
        |_| {},
    );
    let mut client = running.admitted_waiting(0);
    let answer = client.request(CALLEES);
    assert_eq!(
        answer["result"]["structuredContent"]["status"], "warming",
        "{answer}"
    );
    drop(client);
    super::super::test_support::wait_until("the client's admission to be released", || {
        running
            .state
            .admission_status()
            .is_ok_and(|status| status.admitted == 0)
    });

    running.advance(2 * TTL_SECS);
    assert!(running.is_running(), "the build holds it up");
    gate.open();
    super::super::test_support::wait_until("the build to be announced ready", || {
        running
            .state
            .status()
            .is_ok_and(|status| status.state == BuildState::Ready)
    });
    running.advance_and_wake(0);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// Spec §9 test 13, the daemon's half: a shim that has connected but whose connection has not
/// been accepted when the daemon retires is answered `RETIRING`, not dropped, and the key is free
/// for a new daemon that admits it on its next try. The loop is held at the top of a turn while
/// the client connects and the clock passes the TTL, so the retirement decision and the queued
/// connection meet deterministically.
#[test]
fn a_connection_queued_while_the_daemon_retires_is_answered_retiring() {
    let mut running = start(options(64), |_| {});
    running.settle();
    let gate = running.hold();
    let queued = running.connect();
    send_line(&queued, &hello_line(served_key()));
    super::super::test_support::advance(&running.clock, &running.state, TTL_SECS);
    gate.open();
    assert_eq!(read_all(queued), retiring());
    assert_eq!(running.stopped(), StopReason::Idle);

    let (repo_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (base, _unused) = runtime_base();
    let fresh = start_with(
        state,
        clock,
        repo_dir,
        base,
        running.runtime_dir.clone(),
        options(64),
        |_| {},
    );
    let _admitted = fresh.admitted();
}

/// Spec §3.1, §4.3: a repository root that is replaced (renamed away, and a new directory made
/// at its path) drains the daemon at the next check, whatever its connections; a connection in
/// the handshake stage is answered `RETIRING`. Rules out a check of the path's existence alone.
#[test]
fn a_replaced_root_drains_the_daemon_at_the_next_check() {
    let mut running = start(options(64), |_| {});
    let _admitted = running.admitted();
    let pending = running.connect();
    running.settle();

    let moved = running.repo.with_extension("moved");
    std::fs::rename(&running.repo, &moved).expect("move the root away");
    std::fs::create_dir(&running.repo).expect("a new directory at the root's path");
    running.advance_and_wake(IDENTITY_CHECK_INTERVAL.as_secs());
    assert_eq!(running.stopped(), StopReason::RootChanged);
    assert_eq!(read_all(pending), retiring());
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
    std::fs::remove_dir(&running.repo).expect("remove the new directory");
    std::fs::rename(&moved, &running.repo).expect("restore the root for the cleanup");
}

/// Spec §4.3: the root is also checked on every admission, so a `HELLO` that arrives after the
/// root was replaced and before the next periodic check is answered `RETIRING`, not admitted.
#[test]
fn a_replaced_root_is_noticed_at_admission() {
    let mut running = start(options(64), |_| {});
    running.settle();
    let moved = running.repo.with_extension("moved");
    std::fs::rename(&running.repo, &moved).expect("move the root away");

    let (mut client, reply) = running.hello(served_key());
    assert_eq!(reply, retiring());
    client.expect_closed();
    running.waker.wake();
    assert_eq!(running.stopped(), StopReason::RootChanged);
    std::fs::rename(&moved, &running.repo).expect("restore the root for the cleanup");
}

/// Lane README, notes for S4: a runtime directory replaced under the daemon (a cleaner, the last
/// logout) drains it at the next check, and nothing in the new directory is touched.
#[test]
fn replaced_files_drain_the_daemon_and_the_new_directory_is_left_alone() {
    let mut running = start(options(64), |_| {});
    running.settle();
    let moved = running.runtime_dir.with_extension("moved");
    std::fs::rename(&running.runtime_dir, &moved).expect("move the runtime directory");
    let mut newcomer = match DaemonFiles::claim(&running.runtime_dir, &served_key())
        .expect("claim in the new directory")
    {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the new directory's key is free"),
    };
    let _listener = newcomer.bind().expect("bind");

    running.advance_and_wake(IDENTITY_CHECK_INTERVAL.as_secs());
    assert_eq!(running.stopped(), StopReason::FilesReplaced);
    assert!(
        is_socket(newcomer.paths().socket()),
        "the newcomer's socket stays"
    );
}

/// Spec §3.1: SIGTERM and SIGINT go through Draining whatever the connections. A stop request is
/// what the signal handler makes; the daemon drains, and nothing new is admitted.
#[test]
fn a_stop_request_drains_the_daemon_with_a_client_attached() {
    let mut running = start(options(64), |_| {});
    let _admitted = running.admitted();
    running.waker.request_stop();
    assert_eq!(running.stopped(), StopReason::Signal);
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
}
````

Create `src/mcp/daemon/server_support.rs`:

````rust
//! A daemon's accept loop on a thread of the test, with real files in a runtime directory of
//! the test's own, a manual clock, and a raw client that speaks the preamble. This is what
//! stands in for a daemon process and a shim until the shim exists (S5a).
//!
//! No test sleeps to order events: the loop is woken (`Waker::wake`) after the clock is moved,
//! and a test that needs the loop to have looked at the new time waits for two more turns of it
//! to begin (`Running::settle`).
use super::control::StopReason;
use super::coordinator::DaemonState;
use super::files::{Claim, DaemonFiles};
use super::server::{ServeOptions, Served, Server, Waker, HANDSHAKE_TIMEOUT};
use super::test_support::{
    advance, client_settings, fixture, scripted, state_with, wait_until, HANG,
};
use super::test_support::{Gate, ManualClock};
use super::wire_support::Client;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::identity::{FsIdentity, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Hello;
use crate::mcp::RefreshPolicy;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub(super) const TTL_SECS: u64 = 900;
/// The production handshake timeout, in whole seconds.
pub(super) const HANDSHAKE_SECS: u64 = HANDSHAKE_TIMEOUT.as_secs();

pub(super) fn options(max_clients: u32) -> ServeOptions {
    ServeOptions {
        max_clients,
        idle_ttl: Duration::from_secs(TTL_SECS),
        handshake_timeout: HANDSHAKE_TIMEOUT,
    }
}

/// The key the test daemon serves.
pub(super) fn served_key() -> IdentityKey {
    IdentityKey::from_hex(&"cd".repeat(32)).expect("a key")
}

pub(super) fn other_key() -> IdentityKey {
    IdentityKey::from_hex(&"ef".repeat(32)).expect("a key")
}

/// A runtime directory of the test's own under `/tmp` (short enough for a socket path).
pub(super) fn runtime_base() -> (tempfile::TempDir, PathBuf) {
    let base = tempfile::Builder::new()
        .prefix("s4")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let dir = base.path().join("prism-mcpd");
    (base, dir)
}

/// Where the loop is, for a test that holds it at the top of a turn.
#[derive(Default)]
pub(super) struct Turns {
    count: AtomicU64,
    hold: Mutex<Option<Arc<Gate>>>,
    held: AtomicBool,
}

/// A daemon's accept loop running on a thread of the test.
pub(super) struct Running {
    pub(super) state: Arc<DaemonState>,
    pub(super) clock: Arc<ManualClock>,
    pub(super) waker: Waker,
    pub(super) paths: DaemonPaths,
    pub(super) runtime_dir: PathBuf,
    pub(super) repo: PathBuf,
    pub(super) turns: Arc<Turns>,
    pub(super) handle: Option<JoinHandle<anyhow::Result<StopReason>>>,
    pub(super) _repo_dir: tempfile::TempDir,
    pub(super) _base: tempfile::TempDir,
}

/// Start a lazy daemon's loop with `options`; `configure` may replace the peer check.
pub(super) fn start(options: ServeOptions, configure: impl FnOnce(&mut Server)) -> Running {
    let (repo_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (base, runtime_dir) = runtime_base();
    start_with(
        state,
        clock,
        repo_dir,
        base,
        runtime_dir,
        options,
        configure,
    )
}

pub(super) fn start_with(
    state: Arc<DaemonState>,
    clock: Arc<ManualClock>,
    repo_dir: tempfile::TempDir,
    base: tempfile::TempDir,
    runtime_dir: PathBuf,
    options: ServeOptions,
    configure: impl FnOnce(&mut Server),
) -> Running {
    let key = served_key();
    let mut files = match DaemonFiles::claim(&runtime_dir, &key).expect("claim the files") {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the key is taken"),
    };
    let listener = files.bind().expect("bind");
    files
        .write_pid(std::process::id(), 1234, "build")
        .expect("pid");
    let paths = files.paths().clone();
    let repo = std::fs::canonicalize(repo_dir.path()).expect("canonical repository root");
    let served = Served {
        key,
        repo_root: repo.clone(),
        root_fs: FsIdentity::of(&repo).expect("the root's identity"),
        pid: std::process::id(),
        start_time: 1234,
    };
    let mut server = Server::new(
        Arc::clone(&state),
        Arc::new(ToolRegistry::all_v1()),
        files,
        listener,
        served,
        options,
    )
    .expect("a server");
    let turns = Arc::new(Turns::default());
    let seen = Arc::clone(&turns);
    server.set_turn_hook(Arc::new(move || {
        seen.count.fetch_add(1, Ordering::SeqCst);
        let hold = seen.hold.lock().expect("hold").take();
        if let Some(gate) = hold {
            seen.held.store(true, Ordering::SeqCst);
            let _opened = gate.pass();
            seen.held.store(false, Ordering::SeqCst);
        }
    }));
    configure(&mut server);
    let waker = server.waker();
    let handle = std::thread::spawn(move || server.run());
    Running {
        state,
        clock,
        waker,
        paths,
        runtime_dir,
        repo,
        turns,
        handle: Some(handle),
        _repo_dir: repo_dir,
        _base: base,
    }
}

impl Running {
    fn turn_count(&self) -> u64 {
        self.turns.count.load(Ordering::SeqCst)
    }

    /// Wake the loop and wait until it has begun one more turn.
    fn one_turn(&self) {
        let before = self.turn_count();
        self.waker.wake();
        wait_until("the accept loop to turn", || self.turn_count() > before);
    }

    /// Make sure the loop has looked at everything that happened before this call: after two
    /// more turns have begun, the first of them has run its checks and handled every readable
    /// connection.
    pub(super) fn settle(&self) {
        self.one_turn();
        self.one_turn();
    }

    /// Move the daemon's clock and let the loop see it.
    pub(super) fn advance(&self, secs: u64) {
        advance(&self.clock, &self.state, secs);
        self.settle();
    }

    /// Move the daemon's clock and wake the loop, without waiting for it (it may stop).
    pub(super) fn advance_and_wake(&self, secs: u64) {
        advance(&self.clock, &self.state, secs);
        self.waker.wake();
    }

    /// Hold the loop at the top of its next turn until the returned gate is opened.
    pub(super) fn hold(&self) -> Arc<Gate> {
        let gate = Gate::closed();
        *self.turns.hold.lock().expect("hold") = Some(Arc::clone(&gate));
        self.waker.wake();
        wait_until("the accept loop to be held", || {
            self.turns.held.load(Ordering::SeqCst)
        });
        gate
    }

    /// Wait for the loop to end and return why it drained.
    pub(super) fn stopped(&mut self) -> StopReason {
        let handle = self.handle.take().expect("the loop is running");
        wait_until("the accept loop to drain", || handle.is_finished());
        handle
            .join()
            .expect("the accept loop must not panic")
            .expect("the accept loop drains without an error")
    }

    pub(super) fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    }

    pub(super) fn connect(&self) -> UnixStream {
        UnixStream::connect(self.paths.socket()).expect("connect to the daemon's socket")
    }

    /// Connect, send a `HELLO` for `key`, and return the client with the reply line read.
    pub(super) fn hello(&self, key: IdentityKey) -> (Client, String) {
        // The client (and its read timeout) exists before the daemon can answer and close.
        let mut client = Client::new(self.connect());
        client.send(hello_line(key).trim_end());
        let reply = client.recv_line();
        (client, reply)
    }

    /// An admitted, initialized MCP client that waits up to 600 s for the index.
    pub(super) fn admitted(&self) -> Client {
        self.admitted_waiting(600)
    }

    /// An admitted, initialized MCP client that waits up to `wait_secs` for the index.
    pub(super) fn admitted_waiting(&self, wait_secs: u64) -> Client {
        let mut client = Client::new(self.connect());
        client.send(hello_line_waiting(served_key(), wait_secs).trim_end());
        let reply = client.recv_line();
        assert!(reply.starts_with("READY "), "{reply}");
        client.handshake();
        client
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // A test that failed with the loop still running stops it, so that no thread is left
        // polling a directory that is about to be removed.
        if let Some(handle) = self.handle.take() {
            self.waker.request_stop();
            let _ = handle.join();
        }
    }
}

pub(super) fn hello_line(key: IdentityKey) -> String {
    hello_line_waiting(key, 600)
}

pub(super) fn hello_line_waiting(key: IdentityKey, wait_secs: u64) -> String {
    Hello {
        identity: key,
        settings: client_settings(wait_secs),
    }
    .encode()
}

pub(super) fn send_line(stream: &UnixStream, line: &str) {
    (&*stream)
        .write_all(line.as_bytes())
        .expect("write to the daemon");
}

/// Everything the daemon sends on `stream` until it closes it. Use this, not `Client`, on a
/// connection the daemon may already have closed: on macOS setting a read timeout on a socket
/// whose peer has closed fails with `EINVAL`, so a failure to set it is taken to mean that.
pub(super) fn read_all(mut stream: UnixStream) -> String {
    let _ = stream.set_read_timeout(Some(HANG));
    let mut received = String::new();
    stream
        .read_to_string(&mut received)
        .expect("the daemon closes the connection within the hang limit");
    received
}

/// Whether nothing named `path` exists.
pub(super) fn gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err()
}
````

Create `src/mcp/daemon/server_tests.rs`:

````rust
//! Tests for the bounded handshake stage (spec §4.2 "Per connection", §4.4, §7, §10; §9 test
//! 16): what each connection is answered, and what is counted, over a real socket.
use super::super::admission::AdmissionStatus;
use super::super::server_support::{
    hello_line, options, other_key, read_all, send_line, served_key, start, HANDSHAKE_SECS,
};
use super::super::test_support::wait_until;
use super::super::wire_support::{Client, CALLEES};
use crate::mcp::shared::preamble::{BuildState, Ready, Reply, PREAMBLE_MAX_BYTES};
use crate::mcp::shared::runtime_dir::current_uid;
use std::io::Write;

fn counts(pending: u32, admitted: u32) -> AdmissionStatus {
    AdmissionStatus {
        pending,
        admitted,
        draining: None,
    }
}

/// Spec §4.4: a `HELLO` for the served identity is answered `READY` with the daemon's pid,
/// start time, generation and build state, and MCP traffic follows on the same connection;
/// when the client leaves, its admission is released. Rules out a reader that takes bytes past
/// the `HELLO` line (the `initialize` that follows would be lost) and an admission that is never
/// released (the daemon would never retire).
#[test]
fn a_hello_for_the_served_identity_is_answered_ready_and_mcp_follows() {
    let running = start(options(64), |_| {});
    let stream = running.connect();
    // `HELLO` and `initialize` in one write: the reader must stop at the newline.
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
    send_line(&stream, &format!("{}{init}\n", hello_line(served_key())));
    let mut client = Client::new(stream);
    let reply = client.recv_line();
    let decoded = Reply::decode(reply.trim_end().as_bytes()).expect("a reply");
    assert_eq!(
        decoded,
        Reply::Ready(Ready {
            identity: served_key(),
            pid: std::process::id(),
            start_time: 1234,
            generation: 0,
            build: BuildState::Idle,
        })
    );
    let initialized = client.recv();
    assert!(initialized["result"].is_object(), "{initialized}");
    client.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let answer = client.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(running.state.admission_status(), Ok(counts(0, 1)));

    drop(client);
    wait_until("the admission to be released", || {
        running.state.admission_status() == Ok(counts(0, 0))
    });
}

/// Spec §4.2 "Refusal", §4.4: a `HELLO` for another identity is answered `MISMATCH` with the
/// served key and is not admitted; so is one of another protocol version, which is another
/// identity (S2's SI-3 note).
#[test]
fn another_identity_or_protocol_is_answered_mismatch_and_not_admitted() {
    let running = start(options(64), |_| {});
    let (mut client, reply) = running.hello(other_key());
    assert_eq!(
        reply,
        Reply::Mismatch {
            expected: served_key()
        }
        .encode()
    );
    client.expect_closed();

    let stream = running.connect();
    let version_two = hello_line(served_key()).replace("\"protocol\":1", "\"protocol\":2");
    send_line(&stream, &version_two);
    assert_eq!(
        read_all(stream),
        Reply::Mismatch {
            expected: served_key()
        }
        .encode()
    );
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// SI-3, D54: a line the daemon refuses for any other reason (not a `HELLO`, malformed, refused
/// settings, longer than a preamble line) is closed without a reply and is not counted.
#[test]
fn a_refused_preamble_line_is_closed_without_a_reply() {
    let running = start(options(64), |_| {});
    let too_long = format!("HELLO {}\n", "x".repeat(PREAMBLE_MAX_BYTES + 10));
    let below_floor = hello_line(served_key()).replace("\"result_cap\":80000", "\"result_cap\":1");
    for line in [
        "garbage\n",
        "READY {}\n",
        below_floor.as_str(),
        too_long.as_str(),
    ] {
        let stream = running.connect();
        // A long line may be cut off by the close; that is the refusal too.
        let _ = (&stream).write_all(line.as_bytes());
        assert_eq!(read_all(stream), "", "{line:.40}");
    }
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// Spec §4.4, §7, §10: the peer's uid is checked on every accepted connection, and a peer of
/// another user is dropped before any reply and never counted, so it cannot take a slot of
/// `--max-clients` either. Rules out a check after the reply, or none.
#[test]
fn a_connection_from_another_user_is_dropped_before_any_reply() {
    let running = start(options(1), |server| {
        server.set_peer_uid(Box::new(|_| Ok(current_uid().wrapping_add(1))))
    });
    let stream = running.connect();
    send_line(&stream, &hello_line(served_key()));
    assert_eq!(read_all(stream), "");
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// Spec §9 test 16: connections stalled before `HELLO` count against `--max-clients`; the one
/// over the bound is answered `BUSY` without being admitted, so no worker is started for it
/// (workers exist only for admitted connections). When the stalled ones reach their handshake
/// deadline they are dropped and their slots are free again. Rules out a bound on admitted
/// connections only, and a handshake stage with no deadline.
#[test]
fn connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out() {
    let running = start(options(2), |_| {});
    let stalled_a = running.connect();
    let stalled_b = running.connect();
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(2, 0)));

    let (mut busy, reply) = running.hello(served_key());
    assert_eq!(reply, Reply::Busy { max_clients: 2 }.encode());
    busy.expect_closed();
    assert_eq!(running.state.admission_status(), Ok(counts(2, 0)));

    running.advance(HANDSHAKE_SECS - 1);
    assert_eq!(
        running.state.admission_status(),
        Ok(counts(2, 0)),
        "not before the deadline"
    );
    running.advance(1);
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
    for stalled in [stalled_a, stalled_b] {
        assert_eq!(read_all(stalled), "");
    }
    let _admitted = running.admitted();
}

/// A `HELLO` that arrives a few bytes at a time is read to its end, and the deadline bounds the
/// whole line, not each silence (lane README, notes for S4 and S5a: a reader with only a read
/// timeout could be held for 4,097 timeouts by one byte at a time).
#[test]
fn the_handshake_deadline_bounds_the_whole_line() {
    let running = start(options(64), |_| {});
    let line = hello_line(served_key());
    let (first, rest) = line.split_at(20);

    let slow = running.connect();
    send_line(&slow, first);
    running.settle();
    running.advance(HANDSHAKE_SECS - 1);
    send_line(&slow, &rest[..10]);
    running.advance(1);
    assert_eq!(read_all(slow), "");

    let patient = running.connect();
    send_line(&patient, first);
    running.settle();
    running.advance(HANDSHAKE_SECS - 1);
    send_line(&patient, rest);
    let mut client = Client::new(patient);
    assert!(client.recv_line().starts_with("READY "));
}
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t5-red.log" 2>&1
```

Expected: `error[E0583]: file not found for module `log`` (and the same for `peer` and `server`).

- [ ] **Step 3: Implement**

In `src/mcp/daemon/control.rs`, replace this:

````rust
    Idle,
    /// The repository root is gone, or its device and inode are no longer the served ones.
    RootChanged,
    /// SIGTERM or SIGINT.
    Signal,
}
````

with this:

````rust
    Idle,
    /// The repository root is gone, or its device and inode are no longer the served ones.
    RootChanged,
    /// The socket or the owner lock's file at its path is no longer the one this daemon holds
    /// (the runtime directory was removed or replaced under it).
    FilesReplaced,
    /// SIGTERM or SIGINT.
    Signal,
}
````

Create `src/mcp/daemon/log.rs`:

````rust
//! The daemon's log: its standard error, which the shim that starts it points at `<key16>.log`
//! (spec §4.1 step 6, §4.3). The daemon stops adding its own lines past 8 MiB and says so once
//! (§4.3). A panic message, which the panic hook writes, is not counted: it is followed by the
//! end of the process.
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

/// The daemon writes no line of its own past this size.
const CAP_BYTES: u64 = 8 << 20;
/// Longest line the daemon writes; text that comes from elsewhere (an error's message) is cut.
const MAX_LINE_BYTES: usize = 1000;

/// Bytes this process has written to the log.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// Write one line, `prism-mcp daemon: <text>`, with control characters replaced and at most
/// 1000 bytes of `text`, unless the log has reached its cap. The line that would cross the cap
/// is replaced by one note that the cap was reached. A failed write is ignored: there is nowhere
/// else to report it.
pub(super) fn line(text: &str) {
    let text = printable_prefix(text, MAX_LINE_BYTES);
    let line = format!("prism-mcp daemon: {text}\n");
    let len = u64::try_from(line.len()).unwrap_or(u64::MAX);
    let before = WRITTEN.fetch_add(len, Ordering::SeqCst);
    let mut stderr = std::io::stderr().lock();
    if before.saturating_add(len) <= CAP_BYTES {
        let _ = stderr.write_all(line.as_bytes());
    } else if before <= CAP_BYTES {
        let _ = writeln!(
            stderr,
            "prism-mcp daemon: the log reached its cap of {} MiB; later lines are dropped",
            CAP_BYTES >> 20
        );
    }
}

/// At most `max` bytes of `text`, cut on a character boundary, with every control character
/// replaced: a line must not be able to start another or drive a terminal.
pub(super) fn printable_prefix(text: &str, max: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max));
    for c in text.chars() {
        let c = if c.is_control() { '\u{fffd}' } else { c };
        if out.len() + c.len_utf8() > max {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text from elsewhere cannot start a new line in the log or grow it past a line's bound.
    #[test]
    fn a_line_has_no_control_characters_and_a_bounded_length() {
        assert_eq!(
            printable_prefix("a\nb\x1b[2Jc", 100),
            "a\u{fffd}b\u{fffd}[2Jc"
        );
        assert_eq!(printable_prefix("ééé", 5), "éé");
        assert_eq!(printable_prefix(&"x".repeat(5000), 1000).len(), 1000);
    }
}
````

Create `src/mcp/daemon/peer.rs`:

````rust
//! Who is on the other end of an accepted connection (spec §4.4: "The daemon checks the peer's
//! uid (`getpeereid`) equals its own and drops the connection otherwise"; §7).
//!
//! This check, not the runtime directory's, is the isolation guarantee between users: the
//! directory checks are hardening by path, and two review rounds of S2a each found another way
//! around them. It is not a defense against a process of the same user (spec §2) or root.
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

/// The effective uid of the process that connected `stream`, as the kernel recorded it when it
/// connected.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: the descriptor is an open socket, and `credentials` and `len` are valid for
    // writes of the sizes passed, for the duration of the call.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            std::ptr::addr_of_mut!(credentials).cast(),
            &mut len,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(credentials.uid)
}

/// The effective uid of the process that connected `stream`, as the kernel recorded it when it
/// connected.
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: the descriptor is an open socket, and `uid` and `gid` are valid for writes for
    // the duration of the call.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(uid)
}

/// No peer credential call is wired for this platform: every connection is refused, which is
/// the closed way to fail.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
pub(super) fn peer_uid(_stream: &UnixStream) -> io::Result<u32> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no peer credential check on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::shared::runtime_dir::current_uid;

    /// Both ends of a socket pair this process made are this user's.
    #[test]
    fn the_peer_of_a_connection_this_process_made_is_this_user() {
        let (near, far) = UnixStream::pair().expect("a connected socket pair");
        assert_eq!(peer_uid(&near).expect("the peer's uid"), current_uid());
        assert_eq!(peer_uid(&far).expect("the peer's uid"), current_uid());
    }
}
````

Create `src/mcp/daemon/server.rs`:

````rust
//! The accept loop and the bounded handshake stage of a shared daemon (spec §4.2 "Per
//! connection", "Admission and retirement", §4.4), and how it drains (§3.1).
//!
//! One thread runs the loop. It accepts connections, drops any whose peer is another user,
//! reads each pending connection's `HELLO` from a non-blocking socket against that
//! connection's own deadline, and answers it: `READY` (and a worker thread from then on),
//! `MISMATCH`, `BUSY`, `RETIRING`, or nothing for a line it refuses. It never reads or writes an
//! admitted connection again. It also takes the retirement decision and notices a lost root or
//! replaced files, and then drains: the socket is unlinked under the control lock, every
//! connection still pending or queued is answered `RETIRING`, the pid file is removed and the
//! owner lock released.
//!
//! The loop is outside both of the daemon's domains, like a connection's framing: nothing in it
//! runs under the provider slot, and what it does under the control lock it does through the
//! guarded functions of `admission.rs`. A poisoned control lock ends the daemon there.
use super::admission::{Admit, Entry};
use super::connection::serve_admitted;
use super::control::StopReason;
use super::coordinator::DaemonState;
use super::files::DaemonFiles;
use super::log;
use super::peer::peer_uid;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::ClientSettings;
use crate::mcp::shared::identity::{FsIdentity, IdentityKey};
use crate::mcp::shared::preamble::{Hello, PreambleError, Ready, Reply};
use crate::mcp::shared::preamble_reader::{PreambleLineReader, PreambleReadError};
use crate::mcp::shared::runtime_dir::current_uid;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long a connection may take to send its `HELLO` line, from its acceptance. The shim's
/// whole start deadline defaults to the same 10 s (spec §4.1 step 4); a shim whose handshake is
/// dropped sees EOF and starts over within its own deadline (§4.1 step 5).
pub(super) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// How often the loop checks the repository root's identity and its own files when nothing
/// else wakes it (spec §4.3: "on the TTL tick").
pub(super) const IDENTITY_CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// Name of each admitted connection's thread. Linux keeps 15 bytes of it.
const WORKER_THREAD_NAME: &str = "prism-mcp-conn";

/// The bounds a daemon serves within (spec §8 step 4: `--max-clients`, `--idle-ttl`).
#[derive(Clone, Copy, Debug)]
pub(super) struct ServeOptions {
    /// Pending and admitted connections together (spec §4.2).
    pub(super) max_clients: u32,
    pub(super) idle_ttl: Duration,
    pub(super) handshake_timeout: Duration,
}

/// What this daemon is, as `READY` reports it and its checks need it.
#[derive(Clone, Debug)]
pub(super) struct Served {
    pub(super) key: IdentityKey,
    /// The canonical repository root and its device and inode when the daemon started.
    pub(super) repo_root: PathBuf,
    pub(super) root_fs: FsIdentity,
    pub(super) pid: u32,
    /// Seconds since the Unix epoch.
    pub(super) start_time: u64,
}

/// Wakes the loop from another thread: a connection ended, a test moved the clock, or the
/// daemon is asked to stop.
#[derive(Clone)]
pub(super) struct Waker {
    stream: Arc<UnixStream>,
    stop: Arc<AtomicBool>,
}

impl Waker {
    pub(super) fn wake(&self) {
        // A full socket means a wake is already pending; nothing else can fail that matters.
        let _ = (&*self.stream).write(&[1]);
    }

    /// Drain the daemon whatever its connections, as a signal does (spec §3.1: SIGTERM and
    /// SIGINT go through Draining). Tests only; production reads its signal flag.
    #[cfg(test)]
    pub(super) fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.wake();
    }
}

/// How a peer's uid is read. Production: `peer::peer_uid`. A test can play another user.
pub(super) type PeerUid = Box<dyn Fn(&UnixStream) -> io::Result<u32> + Send>;

/// A test seam: called at the top of each turn of the loop, with no lock held.
#[cfg(test)]
pub(super) type TurnHook = Arc<dyn Fn() + Send + Sync>;

/// A connection in the handshake stage.
struct Pending {
    stream: UnixStream,
    reader: PreambleLineReader,
    deadline: Instant,
}

pub(super) struct Server {
    state: Arc<DaemonState>,
    registry: Arc<ToolRegistry>,
    files: DaemonFiles,
    listener: UnixListener,
    served: Served,
    options: ServeOptions,
    wakes: UnixStream,
    waker: Waker,
    peer_uid: PeerUid,
    pending: Vec<Pending>,
    next_identity_check: Instant,
    #[cfg(test)]
    turn: Option<TurnHook>,
}

impl Server {
    pub(super) fn new(
        state: Arc<DaemonState>,
        registry: Arc<ToolRegistry>,
        files: DaemonFiles,
        listener: UnixListener,
        served: Served,
        options: ServeOptions,
    ) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        let (wakes, wake_end) = UnixStream::pair()?;
        wakes.set_nonblocking(true)?;
        wake_end.set_nonblocking(true)?;
        let next_identity_check = state.now();
        Ok(Self {
            state,
            registry,
            files,
            listener,
            served,
            options,
            wakes,
            waker: Waker {
                stream: Arc::new(wake_end),
                stop: Arc::new(AtomicBool::new(false)),
            },
            peer_uid: Box::new(peer_uid),
            pending: Vec::new(),
            next_identity_check,
            #[cfg(test)]
            turn: None,
        })
    }

    pub(super) fn waker(&self) -> Waker {
        self.waker.clone()
    }

    #[cfg(test)]
    pub(super) fn set_peer_uid(&mut self, peer_uid: PeerUid) {
        self.peer_uid = peer_uid;
    }

    #[cfg(test)]
    pub(super) fn set_turn_hook(&mut self, turn: TurnHook) {
        self.turn = Some(turn);
    }

    /// Serve until the daemon drains, then drain and say why. An error means a poisoned
    /// control lock (which has already ended a production daemon) or a failed `poll`.
    pub(super) fn run(mut self) -> anyhow::Result<StopReason> {
        loop {
            #[cfg(test)]
            if let Some(turn) = &self.turn {
                turn();
            }
            if let Some(reason) = self.check()? {
                return self.drain(reason);
            }
            let timeout = self.poll_timeout()?;
            let accept = self.wait(timeout)?;
            self.empty_wakes();
            if accept {
                self.accept_all()?;
            }
            self.advance_handshakes()?;
        }
    }

    /// Everything that can start Draining, in one place: a signal or a stop request, the
    /// periodic root and files check, and the idle retirement decision.
    fn check(&mut self) -> anyhow::Result<Option<StopReason>> {
        let files = &self.files;
        if self.waker.stop.load(Ordering::SeqCst) {
            return Ok(Some(
                self.state
                    .drain(StopReason::Signal, || files.unlink_socket())?,
            ));
        }
        let now = self.state.now();
        if now >= self.next_identity_check {
            self.next_identity_check = now.checked_add(IDENTITY_CHECK_INTERVAL).unwrap_or(now);
            if let Some(reason) = self.lost_identity() {
                return Ok(Some(self.state.drain(reason, || files.unlink_socket())?));
            }
        }
        Ok(self
            .state
            .retire_if_idle(self.options.idle_ttl, || files.unlink_socket())?)
    }

    /// Spec §4.3: the root's device and inode are checked on the TTL tick and on every
    /// admission; loss or change drains the daemon. Its own files are checked with them.
    fn lost_identity(&self) -> Option<StopReason> {
        let root = FsIdentity::of(&self.served.repo_root);
        if root.ok() != Some(self.served.root_fs) {
            return Some(StopReason::RootChanged);
        }
        if !self.files.still_ours() {
            return Some(StopReason::FilesReplaced);
        }
        None
    }

    /// How long the next `poll` may sleep: until the first handshake deadline, the idle
    /// deadline or the next identity check, whichever is first.
    fn poll_timeout(&self) -> anyhow::Result<Duration> {
        let now = self.state.now();
        let idle = self
            .state
            .idle_deadline(self.options.idle_ttl)?
            .filter(|deadline| *deadline > now);
        let first = self
            .pending
            .iter()
            .map(|pending| pending.deadline)
            .chain(idle)
            .chain(Some(self.next_identity_check))
            .min()
            .unwrap_or(now);
        Ok(first
            .saturating_duration_since(now)
            .min(IDENTITY_CHECK_INTERVAL))
    }

    /// Sleep in `poll` until the listener or a pending connection is readable, a wake byte
    /// arrives, or `timeout` passes. Returns whether the listener is readable.
    fn wait(&self, timeout: Duration) -> anyhow::Result<bool> {
        let mut fds: Vec<libc::pollfd> = [self.listener.as_raw_fd(), self.wakes.as_raw_fd()]
            .into_iter()
            .chain(
                self.pending
                    .iter()
                    .map(|pending| pending.stream.as_raw_fd()),
            )
            .map(|fd| libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        // Round up, so that a deadline a fraction of a millisecond away is not polled for 0 ms
        // again and again.
        let millis = timeout.as_micros().div_ceil(1000);
        let millis = libc::c_int::try_from(millis).unwrap_or(libc::c_int::MAX);
        let count = libc::nfds_t::try_from(fds.len()).unwrap_or(libc::nfds_t::MAX);
        // SAFETY: `fds` is a valid array of `count` initialized `pollfd`s, alive and not
        // otherwise borrowed for the call.
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), count, millis) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error.into());
        }
        Ok(fds.first().is_some_and(|listener| listener.revents != 0))
    }

    fn empty_wakes(&self) {
        let mut buffer = [0u8; 64];
        while matches!((&self.wakes).read(&mut buffer), Ok(n) if n > 0) {}
    }

    /// Accept every connection that is waiting, until the listener would block.
    fn accept_all(&mut self) -> anyhow::Result<()> {
        loop {
            match self.listener.accept() {
                Ok((stream, _address)) => self.enter(stream)?,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                // A connection that went away before it was accepted, or a full descriptor
                // table: try again on the next turn.
                Err(error) => {
                    log::line(&format!("accepting a connection failed: {error}"));
                    return Ok(());
                }
            }
        }
    }

    /// A connection just accepted: check its peer, then count it as pending or answer it.
    fn enter(&mut self, stream: UnixStream) -> anyhow::Result<()> {
        // Spec §4.4, §10: a peer of another user is dropped before any reply and is never
        // counted.
        match (self.peer_uid)(&stream) {
            Ok(uid) if uid == current_uid() => {}
            Ok(uid) => {
                log::line(&format!("refused a connection from uid {uid}"));
                return Ok(());
            }
            Err(error) => {
                log::line(&format!(
                    "refused a connection whose peer is unknown: {error}"
                ));
                return Ok(());
            }
        }
        if stream.set_nonblocking(true).is_err() {
            return Ok(());
        }
        match self.state.enter_handshake(self.options.max_clients)? {
            Entry::Pending => {
                let now = self.state.now();
                self.pending.push(Pending {
                    stream,
                    reader: PreambleLineReader::new(),
                    deadline: now
                        .checked_add(self.options.handshake_timeout)
                        .unwrap_or(now),
                });
            }
            Entry::Busy { max_clients } => send(&stream, &Reply::Busy { max_clients }),
            Entry::Retiring => send(&stream, &Reply::Retiring),
        }
        Ok(())
    }

    /// Read what each pending connection has sent. A whole line is answered; a connection
    /// past its deadline, closed, or sending more than a preamble line is dropped.
    fn advance_handshakes(&mut self) -> anyhow::Result<()> {
        let now = self.state.now();
        for mut pending in std::mem::take(&mut self.pending) {
            match pending.reader.read_line(&mut pending.stream) {
                Ok(line) => self.answer(pending.stream, &line)?,
                Err(PreambleReadError::Io(error))
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) && now < pending.deadline =>
                {
                    self.pending.push(pending);
                }
                Err(_) => self.state.leave_handshake()?,
            }
        }
        Ok(())
    }

    /// Answer a whole `HELLO` line.
    fn answer(&mut self, stream: UnixStream, line: &[u8]) -> anyhow::Result<()> {
        let settings = match admit_hello(Hello::decode(line), &self.served.key) {
            Ok(settings) => settings,
            Err(Some(reply)) => {
                send(&stream, &reply);
                return Ok(self.state.leave_handshake()?);
            }
            Err(None) => return Ok(self.state.leave_handshake()?),
        };
        // Spec §4.3: the root is checked on every admission. A root that changed drains the
        // daemon, and this connection is told so.
        if let Some(reason) = self.lost_identity() {
            let files = &self.files;
            self.state.drain(reason, || files.unlink_socket())?;
        }
        match self.state.admit()? {
            Admit::Retiring => send(&stream, &Reply::Retiring),
            Admit::Ready(status) => {
                let ready = Reply::Ready(Ready {
                    identity: self.served.key,
                    pid: self.served.pid,
                    start_time: self.served.start_time,
                    generation: status.generation,
                    build: status.state,
                });
                if (&stream).write_all(ready.encode().as_bytes()).is_err() {
                    return Ok(self.state.release()?);
                }
                self.start_worker(stream, settings)?;
            }
        }
        Ok(())
    }

    /// The admitted connection's own thread, from `READY` on. Its admission is released when
    /// it ends, on every path.
    fn start_worker(&self, stream: UnixStream, settings: ClientSettings) -> anyhow::Result<()> {
        let released = Released {
            state: Arc::clone(&self.state),
            waker: self.waker(),
        };
        let registry = Arc::clone(&self.registry);
        let spawned = std::thread::Builder::new()
            .name(WORKER_THREAD_NAME.to_string())
            .spawn(move || {
                if let Err(error) = serve_admitted(&released.state, &registry, stream, &settings) {
                    log::line(&format!("a connection ended with an error: {error:#}"));
                }
                drop(released);
            });
        if let Err(error) = spawned {
            // The closure, with the stream and the release, was dropped with the error.
            log::line(&format!("could not start a connection's thread: {error}"));
        }
        Ok(())
    }

    /// Drain (spec §3.1): the socket is unlinked under the control lock (already, for an idle
    /// retirement), every connection still pending or queued on the listener is answered
    /// `RETIRING`, then the pid file is removed and the owner lock released. Admitted
    /// connections are not touched here; they end with the process.
    fn drain(self, reason: StopReason) -> anyhow::Result<StopReason> {
        let Self {
            state,
            files,
            listener,
            pending,
            peer_uid,
            ..
        } = self;
        let reason = state.drain(reason, || files.unlink_socket())?;
        for pending in pending {
            send(&pending.stream, &Reply::Retiring);
            state.leave_handshake()?;
        }
        // The socket's name is gone, so nothing new can queue: whatever is queued now is all
        // that ever will be.
        loop {
            match listener.accept() {
                Ok((stream, _address)) => {
                    if peer_uid(&stream).is_ok_and(|uid| uid == current_uid()) {
                        send(&stream, &Reply::Retiring);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        drop(listener);
        files.release();
        Ok(reason)
    }
}

/// The one place a decoded `HELLO` becomes an admission (lane README, notes for S4: admission
/// must not read the settings of a `HELLO` without comparing its identity). `Ok` is the
/// client's settings; `Err(Some(reply))` is the reply it is owed; `Err(None)` means the line is
/// refused without a reply (SI-3, D54).
fn admit_hello(
    decoded: Result<Hello, PreambleError>,
    served: &IdentityKey,
) -> Result<ClientSettings, Option<Reply>> {
    match decoded {
        Ok(hello) => match hello.mismatch_against(served) {
            Some(reply) => Err(Some(reply)),
            None => Ok(hello.settings),
        },
        // Another protocol version is another identity (the version is part of the key), so
        // `MISMATCH` is the truth (S2's SI-3 note).
        Err(PreambleError::UnsupportedProtocol { .. }) => {
            Err(Some(Reply::Mismatch { expected: *served }))
        }
        Err(error) => {
            log::line(&format!("refused a preamble line: {error}"));
            Err(None)
        }
    }
}

/// Write one reply on a connection that is about to be closed. A failure is ignored: the peer
/// has gone, and it would have been told nothing else.
fn send(stream: &UnixStream, reply: &Reply) {
    let _ = (&*stream).write_all(reply.encode().as_bytes());
}

/// Releases an admission when the connection's thread ends.
struct Released {
    state: Arc<DaemonState>,
    waker: Waker,
}

impl Drop for Released {
    fn drop(&mut self) {
        // A poisoned control lock has already ended the daemon (`on_fatal`).
        let _ = self.state.release();
        self.waker.wake();
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "server_drain_tests.rs"]
mod drain_tests;
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t5.log" 2>&1
```

Expected: `test result: ok. 121 passed; 0 failed` (105 + 16), no `warning` line.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/daemon/
git commit -m "feat(mcp): the shared daemon's accept loop and bounded handshake stage"
```

- [ ] **Step 6: Prove the tests bite**

Run `cargo test --features mcp --lib mcp::daemon::server` under each mutant. Mutants 5A, 5B and 5F make a test wait out its 30 s limit before it fails.

| Mutant | Edit | Fails |
|---|---|---|
| 5A | in `enter`, `Ok(uid) if uid == current_uid() => {}` → `Ok(_) => {}` | `a_connection_from_another_user_is_dropped_before_any_reply` |
| 5B | in `advance_handshakes`, drop `&& now < pending.deadline` | `connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out`, `the_handshake_deadline_bounds_the_whole_line` |
| 5C | in `drain`, make the accept loop `break` before its first `accept` | `a_connection_queued_while_the_daemon_retires_is_answered_retiring` |
| 5D | in `answer`, `if let Some(reason) = self.lost_identity() {` → `if let Some(reason) = None::<StopReason> {` | `a_replaced_root_is_noticed_at_admission` |
| 5E | in `admit_hello`, answer `UnsupportedProtocol` with `Err(None)` | `another_identity_or_protocol_is_answered_mismatch_and_not_admitted` |
| 5F | in `Released::drop`, delete `let _ = self.state.release();` | `a_hello_for_the_served_identity_is_answered_ready_and_mcp_follows`, `an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free`, `a_running_build_keeps_an_idle_daemon_up_until_it_ends` |
| 5G | in `drain`, delete `send(&pending.stream, &Reply::Retiring);` | `a_replaced_root_drains_the_daemon_at_the_next_check` |

### Task 6: `prism-mcp daemon`

**Files:**
- Create: `src/mcp/daemon/run.rs`, `tests/mcp/daemon_process_test.rs`
- Modify: `src/mcp/daemon/mod.rs`, `src/mcp/daemon/server.rs`, `src/mcp/daemon/log.rs`, `src/mcp/daemon/control.rs`, `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/admission.rs`, `src/mcp/mod.rs`, `src/bin/prism-mcp.rs`, `tests/mcp/main.rs`

**Interfaces:**
- Consumes: everything above; `session::canonical_config` (`pub(super)` in `crate::mcp`); `shared::identity::DaemonIdentity`; `shared::runtime_dir::runtime_dir`.
- Produces: `prism::mcp::{run_daemon, DaemonOptions, DEFAULT_IDLE_TTL_SECS, DEFAULT_MAX_CLIENTS}` (`DaemonOptions { config: ServerConfig, idle_ttl: Duration, max_clients: u32, expected_key: Option<String> }`); `Server::set_signalled(fn() -> bool)`, `Waker::leak_raw_fd() -> io::Result<i32>`; `log::start()`; `StopReason::describe(self) -> &'static str`; the command line of D60.

Spec tests 8 ("Start race: pause the winner at lock/spawn/readiness boundaries and release contenders; count daemons and builders, not pid files") and 15 ("Starter death: starter killed between spawning its child and the child taking the owner lock while a second shim starts → exactly one daemon survives and both shims reach `READY`") come down, for the daemon, to one property: daemons started together for one key leave exactly one serving, and the others exit 0. The process test starts four at once; the pauses and the starter's death are the shim's (S5a).

- [ ] **Step 1: Write the failing process tests**

Create `tests/mcp/daemon_process_test.rs`:

````rust
//! `prism-mcp daemon` as a process (spec §3.1, §4.2–§4.4; the daemon's halves of §9 tests 2, 3,
//! 4, 8 and 15, which the shim of slice S5a completes). Each test runs the real binary with a
//! runtime directory of its own and speaks the preamble to it as a shim will.
#![cfg(feature = "mcp")]

use prism::mcp::concise_shape::ConciseShapeMode;
use prism::mcp::output::StructuredContentMode;
use prism::mcp::settings::{ClientSettings, ConnectionSettings};
use prism::mcp::shared::identity::{DaemonIdentity, IdentityKey};
use prism::mcp::shared::paths::DaemonPaths;
use prism::mcp::shared::preamble::{BuildState, Hello, Reply};
use prism::mcp::shared::runtime_dir::{validate_runtime_dir, RUNTIME_DIR_ENV, RUNTIME_DIR_NAME};
use prism::mcp::{CacheMode, ServerConfig};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
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
        std::thread::sleep(Duration::from_millis(5));
    }
}

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
    std::fs::write(
        repo.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("write main.py");
    let cache = tempfile::tempdir().expect("a cache base");
    let base = tempfile::Builder::new()
        .prefix("s4")
        .tempdir_in("/tmp")
        .expect("a runtime base under /tmp");
    let mut cfg = ServerConfig::new(repo.path().to_path_buf());
    cfg.cache = CacheMode::Dir(cache.path().to_path_buf());
    let key = DaemonIdentity::resolve(&cfg).expect("an identity").key();
    let paths = DaemonPaths::new(&base.path().join(RUNTIME_DIR_NAME), &key).expect("paths");
    Setup {
        repo,
        cache,
        base,
        key,
        paths,
    }
}

/// A daemon process, killed and reaped when the test ends however it ends.
struct Daemon {
    child: Child,
    stderr: Option<JoinHandle<String>>,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Daemon {
    fn start(setup: &Setup, extra: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
            .arg("daemon")
            .arg("--repo")
            .arg(setup.repo.path())
            .arg("--cache-dir")
            .arg(setup.cache.path())
            .args(extra)
            .env(RUNTIME_DIR_ENV, setup.base.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start the daemon");
        let mut stderr = child.stderr.take().expect("the daemon's standard error");
        let drained = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        Self {
            child,
            stderr: Some(drained),
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
            status.is_some()
        });
        let stderr = self
            .stderr
            .take()
            .map(|drained| drained.join().expect("standard error is read"))
            .unwrap_or_default();
        (status.expect("ended"), stderr)
    }
}

/// One shim's end of a connection, after its `HELLO`.
struct Shim {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Shim {
    /// Connect (waiting for the socket to appear), send a `HELLO` for `key`, and return the
    /// reply line too. As a shim does (spec §4.1 step 5), a connection that is refused or that
    /// ends before any reply is transient, and the shim connects again: on macOS a listener can
    /// outlive its owner in a child another test spawned at the moment it was created (it is made
    /// close-on-exec only after it exists), so a stale socket can accept and then close.
    fn hello(paths: &DaemonPaths, key: IdentityKey) -> (Self, String) {
        let give_up = Instant::now() + HANG;
        loop {
            if let Some(answered) = Self::try_hello(paths, key) {
                return answered;
            }
            assert!(
                Instant::now() < give_up,
                "no reply to HELLO within {HANG:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn try_hello(paths: &DaemonPaths, key: IdentityKey) -> Option<(Self, String)> {
        let writer = UnixStream::connect(paths.socket()).ok()?;
        writer.set_read_timeout(Some(HANG)).ok()?;
        let settings = ClientSettings::new(
            ConnectionSettings {
                result_cap: 80_000,
                structured_content: StructuredContentMode::Always,
                concise_shape: ConciseShapeMode::Slim,
            },
            600,
            false,
        )
        .expect("settings");
        let mut shim = Self {
            reader: BufReader::new(writer.try_clone().ok()?),
            writer,
        };
        let hello = Hello {
            identity: key,
            settings,
        }
        .encode();
        shim.writer.write_all(hello.as_bytes()).ok()?;
        let mut reply = String::new();
        shim.reader.read_line(&mut reply).ok()?;
        (!reply.is_empty()).then_some((shim, reply))
    }

    fn send_raw(&mut self, text: &str) {
        self.writer.write_all(text.as_bytes()).expect("write");
    }

    fn line(&mut self) -> String {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("a line within the hang limit");
        line
    }

    fn request(&mut self, line: &str) -> serde_json::Value {
        self.send_raw(&format!("{line}\n"));
        serde_json::from_str(&self.line()).expect("a JSON response")
    }

    /// `initialize`, then `notifications/initialized`.
    fn handshake(&mut self) {
        let initialized = self.request(INIT);
        assert!(initialized["result"].is_object(), "{initialized}");
        self.send_raw(&format!("{INITED}\n"));
    }

    fn expect_end_of_stream(&mut self) {
        let mut rest = String::new();
        let read = self.reader.read_to_string(&mut rest);
        assert!(matches!(read, Ok(0)), "{read:?} {rest}");
    }
}

fn ready_pid(reply: &str) -> u32 {
    match Reply::decode(reply.trim_end().as_bytes()) {
        Ok(Reply::Ready(ready)) => ready.pid,
        other => panic!("expected READY, got {other:?} from {reply:?}"),
    }
}

fn gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err()
}

/// Spec §9 test 2's real-clock smoke test: a daemon process serves an admitted shim from a real
/// index, and once the shim leaves and its idle TTL (1 s here) passes, it drains, removes its
/// socket and pid file, and exits 0.
#[test]
fn a_daemon_process_serves_a_shim_and_retires_after_its_idle_ttl() {
    let setup = setup();
    let mut daemon = Daemon::start(&setup, &["--idle-ttl", "1"]);
    let (mut shim, reply) = Shim::hello(&setup.paths, setup.key);
    assert_eq!(ready_pid(&reply), daemon.pid());
    let pid_file = std::fs::read_to_string(setup.paths.pid_file()).expect("the pid file");
    assert!(
        pid_file.starts_with(&format!("{}\n", daemon.pid())),
        "{pid_file}"
    );
    shim.handshake();
    let answer = shim.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    drop(shim);

    let (status, stderr) = daemon.ended();
    assert!(status.success(), "{status:?} {stderr}");
    assert!(stderr.contains("stopped: idle"), "{stderr}");
    assert!(gone(setup.paths.socket()));
    assert!(gone(setup.paths.pid_file()));
}

/// Spec §9 test 3, the daemon's half: a socket file left by a dead daemon, with no owner lock
/// held, is replaced by the next daemon, which answers `READY` on it.
#[test]
fn a_daemon_replaces_a_stale_socket_left_by_a_dead_one() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    drop(UnixListener::bind(setup.paths.socket()).expect("a stale socket"));
    // Not `connect(...).is_err()`: on macOS a socket is made close-on-exec only after it is
    // created, so a child another test spawns in that moment can inherit the listener and keep
    // it accepting. That the file is there is the precondition; the daemon replaces it either way.
    assert!(std::fs::symlink_metadata(setup.paths.socket())
        .is_ok_and(|metadata| metadata.file_type().is_socket()));

    let daemon = Daemon::start(&setup, &[]);
    let (_shim, reply) = Shim::hello(&setup.paths, setup.key);
    assert_eq!(ready_pid(&reply), daemon.pid());
}

/// Spec §9 test 4, the daemon's half, and §3.1 "exit 0 silently": a daemon started while another
/// process holds the owner lock (and a socket that accepts and never answers) exits 0 without a
/// word and leaves the socket and the pid file alone. That the shim then fails closed naming the
/// pid is S5a's half.
#[test]
fn a_daemon_that_finds_a_live_owner_exits_silently_and_touches_nothing() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(setup.paths.owner_lock())
        .expect("the owner lock file");
    owner.try_lock().expect("the test owns the key");
    let wedged = UnixListener::bind(setup.paths.socket()).expect("a wedged owner's socket");
    let socket_inode = std::fs::symlink_metadata(setup.paths.socket())
        .expect("socket")
        .ino();

    let mut daemon = Daemon::start(&setup, &[]);
    let (status, stderr) = daemon.ended();
    assert!(status.success(), "{status:?}");
    assert_eq!(stderr, "");
    assert_eq!(
        std::fs::symlink_metadata(setup.paths.socket())
            .expect("the socket is still there")
            .ino(),
        socket_inode
    );
    assert!(gone(setup.paths.pid_file()));
    let _client = UnixStream::connect(setup.paths.socket()).expect("the wedged owner's socket");
    wedged.accept().expect("still the wedged owner's");
    drop::<File>(owner);
}

/// Spec §9 tests 8 and 15, the daemon's halves: daemons started together for one key (as
/// racing shims, or a dead starter's child and the next starter's, would start them) leave
/// exactly one serving; the others exit 0, and every shim is admitted by the survivor.
#[test]
fn daemons_started_together_leave_exactly_one_serving() {
    let setup = setup();
    let mut daemons: Vec<Daemon> = (0..4).map(|_| Daemon::start(&setup, &[])).collect();
    wait_until("all but one daemon to exit", || {
        daemons
            .iter_mut()
            .map(|daemon| daemon.exited().is_some())
            .filter(|exited| *exited)
            .count()
            == 3
    });
    let mut survivors = Vec::new();
    for daemon in &mut daemons {
        match daemon.exited() {
            Some(_) => {
                let (status, stderr) = daemon.ended();
                assert!(status.success(), "{status:?} {stderr}");
                assert_eq!(stderr, "");
            }
            None => survivors.push(daemon.pid()),
        }
    }
    assert_eq!(survivors.len(), 1);
    for _ in 0..4 {
        let (_shim, reply) = Shim::hello(&setup.paths, setup.key);
        assert_eq!(ready_pid(&reply), survivors[0]);
    }
}

/// Spec §3.1: SIGTERM goes through Draining: an attached shim sees its connection end, the
/// socket and the pid file are removed, and the process exits 0.
#[test]
fn a_termination_signal_drains_a_daemon_with_a_shim_attached() {
    let setup = setup();
    let mut daemon = Daemon::start(&setup, &[]);
    let (mut shim, reply) = Shim::hello(&setup.paths, setup.key);
    assert_eq!(ready_pid(&reply), daemon.pid());
    shim.handshake();

    let pid = libc::pid_t::try_from(daemon.pid()).expect("a pid");
    // SAFETY: `kill` with a valid signal number has no memory-safety preconditions.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
    shim.expect_end_of_stream();
    let (status, stderr) = daemon.ended();
    assert!(status.success(), "{status:?} {stderr}");
    assert!(stderr.contains("termination signal"), "{stderr}");
    assert!(gone(setup.paths.socket()));
    assert!(gone(setup.paths.pid_file()));
}

/// Lane README, notes for S5a: a daemon handed a key that is not the one it computes refuses to
/// start, with a message, and binds nothing.
#[test]
fn a_daemon_handed_another_key_refuses_to_start() {
    let setup = setup();
    let other = "f".repeat(64);
    let mut daemon = Daemon::start(&setup, &["--key", &other]);
    let (status, stderr) = daemon.ended();
    assert!(!status.success());
    assert!(stderr.contains("--key"), "{stderr}");
    assert!(gone(setup.paths.socket()));
}

/// Spec §11 item 7: `--warm-at-startup` on the daemon's own command line builds the index before
/// any tool call; a later `READY` reports it ready.
#[test]
fn a_warm_daemon_builds_before_any_tool_call() {
    let setup = setup();
    let _daemon = Daemon::start(&setup, &["--warm-at-startup"]);
    wait_until("a READY that reports the index ready", || {
        let (_shim, reply) = Shim::hello(&setup.paths, setup.key);
        matches!(
            Reply::decode(reply.trim_end().as_bytes()),
            Ok(Reply::Ready(ready)) if ready.build == BuildState::Ready
        )
    });
}
````

In `tests/mcp/main.rs`, replace this:

````rust
mod golden_transcript_test;
mod mem_report_test;
mod settings_test;
````

with this:

````rust
mod daemon_process_test;
mod golden_transcript_test;
mod mem_report_test;
mod settings_test;
````

- [ ] **Step 2: Run them to see them fail**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp daemon_process_test > "$LOG_DIR/t6-red.log" 2>&1
```

Expected, after about a minute: `test result: FAILED. 0 passed; 7 failed`: the binary refuses the word `daemon` (`error: unexpected argument 'daemon' found` in the log), so no socket appears and each test gives up waiting (60 s), or sees a non-zero exit where it expects 0.

- [ ] **Step 3: Implement**

In `src/bin/prism-mcp.rs`, replace this:

````rust
    mem_emit_workload: Option<u64>,
}

fn main() -> anyhow::Result<()> {
    let c = <Cli as clap::Parser>::parse();
    let cache = if c.no_cache {
        prism::mcp::CacheMode::NoCache
    } else if let Some(d) = c.cache_dir {
        prism::mcp::CacheMode::Dir(d)
    } else {
        prism::mcp::CacheMode::Default
    };
    let refresh_policy = match c.refresh_policy.as_str() {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
        "auto-full" => prism::mcp::RefreshPolicy::AutoFull,
        "auto-incremental" => prism::mcp::RefreshPolicy::AutoIncremental,
        _ => unreachable!("clap value_parser restricts refresh-policy"),
    };
    let mut cfg = prism::mcp::ServerConfig::new(c.repo);
    cfg.cache = cache;
    cfg.refresh_policy = refresh_policy;
````

with this:

````rust
    mem_emit_workload: Option<u64>,
}

/// `prism-mcp daemon ...`: the shared per-repository daemon (spec §3.1, §4.2). Internal: the
/// shim of a later slice starts it; it serves only through its Unix socket.
#[derive(clap::Parser)]
#[command(name = "prism-mcp daemon")]
struct DaemonCli {
    #[arg(long)]
    repo: std::path::PathBuf,
    /// The cache base directory; the user cache directory when absent.
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value = "warn-only", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: String,
    /// Build the index at start instead of on the first tool call (spec §11 item 7).
    #[arg(long)]
    warm_at_startup: bool,
    /// Seconds with no client and no build before the daemon retires.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_IDLE_TTL_SECS, value_parser = clap::value_parser!(u64).range(1..=86_400))]
    idle_ttl: u64,
    /// Pending and admitted connections together.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_MAX_CLIENTS, value_parser = clap::value_parser!(u32).range(1..=4096))]
    max_clients: u32,
    /// The full identity key the starting shim computed; the daemon refuses to start on a
    /// different one.
    #[arg(long)]
    key: Option<String>,
}

fn refresh_policy_from(name: &str) -> prism::mcp::RefreshPolicy {
    match name {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
        "auto-full" => prism::mcp::RefreshPolicy::AutoFull,
        "auto-incremental" => prism::mcp::RefreshPolicy::AutoIncremental,
        _ => unreachable!("clap value_parser restricts refresh-policy"),
    }
}

fn cache_mode(no_cache: bool, cache_dir: Option<std::path::PathBuf>) -> prism::mcp::CacheMode {
    if no_cache {
        prism::mcp::CacheMode::NoCache
    } else if let Some(d) = cache_dir {
        prism::mcp::CacheMode::Dir(d)
    } else {
        prism::mcp::CacheMode::Default
    }
}

fn daemon_main() -> anyhow::Result<()> {
    // The first argument is the word `daemon`; clap reads it as the program name.
    let c = <DaemonCli as clap::Parser>::parse_from(std::env::args_os().skip(1));
    let mut config = prism::mcp::ServerConfig::new(c.repo);
    config.cache = cache_mode(false, c.cache_dir);
    config.refresh_policy = refresh_policy_from(&c.refresh_policy);
    config.startup = if c.warm_at_startup {
        prism::mcp::StartupMode::Background
    } else {
        prism::mcp::StartupMode::Lazy
    };
    prism::mcp::run_daemon(prism::mcp::DaemonOptions {
        config,
        idle_ttl: std::time::Duration::from_secs(c.idle_ttl),
        max_clients: c.max_clients,
        expected_key: c.key,
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
    let refresh_policy = refresh_policy_from(&c.refresh_policy);
    let mut cfg = prism::mcp::ServerConfig::new(c.repo);
    cfg.cache = cache;
    cfg.refresh_policy = refresh_policy;
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust

/// The connection counts, for tests and the status probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AdmissionStatus {
    pub(super) pending: u32,
    pub(super) admitted: u32,
````

with this:

````rust

/// The connection counts, for tests and the status probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // read by tests until S5b's status probe
pub(super) struct AdmissionStatus {
    pub(super) pending: u32,
    pub(super) admitted: u32,
````

In `src/mcp/daemon/admission.rs`, replace this:

````rust
            .and_then(|since| since.checked_add(idle_ttl)))
    }

    pub(super) fn admission_status(&self) -> Result<AdmissionStatus, Poisoned> {
        let control = self.control_plane()?;
        Ok(AdmissionStatus {
````

with this:

````rust
            .and_then(|since| since.checked_add(idle_ttl)))
    }

    #[cfg_attr(not(test), allow(dead_code))] // read by tests until S5b's status probe
    pub(super) fn admission_status(&self) -> Result<AdmissionStatus, Poisoned> {
        let control = self.control_plane()?;
        Ok(AdmissionStatus {
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    Signal,
}

/// The connections of a daemon and whether it drains (spec §4.2 "Admission and retirement").
/// Changed only under the control lock, by the functions in `admission.rs`.
#[derive(Debug)]
````

with this:

````rust
    Signal,
}

impl StopReason {
    /// The words the daemon's log line uses.
    pub(super) fn describe(self) -> &'static str {
        match self {
            Self::Idle => "idle for the idle TTL with no client and no build",
            Self::RootChanged => "the repository root was lost or replaced",
            Self::FilesReplaced => "its socket or owner lock file was replaced",
            Self::Signal => "a termination signal",
        }
    }
}

/// The connections of a daemon and whether it drains (spec §4.2 "Admission and retirement").
/// Changed only under the control lock, by the functions in `admission.rs`.
#[derive(Debug)]
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    }

    /// The control plane's view of the build.
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        Ok(self.control_plane()?.build_status())
    }
````

with this:

````rust
    }

    /// The control plane's view of the build.
    #[cfg_attr(not(test), allow(dead_code))] // read by tests until S5b's status probe
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        Ok(self.control_plane()?.build_status())
    }
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// A running daemon does not need this: a parked call wakes by itself when its deadline
    /// passes. Tests that move a manual clock call it. The control lock is taken first, so a
    /// call that has read the time and not yet parked is woken too.
    pub(super) fn wake(&self) {
        let _control = self.control.lock();
        self.changed.notify_all();
````

with this:

````rust
    /// A running daemon does not need this: a parked call wakes by itself when its deadline
    /// passes. Tests that move a manual clock call it. The control lock is taken first, so a
    /// call that has read the time and not yet parked is woken too.
    #[cfg(test)]
    pub(super) fn wake(&self) {
        let _control = self.control.lock();
        self.changed.notify_all();
````

In `src/mcp/daemon/log.rs`, replace this:

````rust
//! The daemon's log: its standard error, which the shim that starts it points at `<key16>.log`
//! (spec §4.1 step 6, §4.3). The daemon stops adding its own lines past 8 MiB and says so once
//! (§4.3). A panic message, which the panic hook writes, is not counted: it is followed by the
//! end of the process.
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

/// The daemon writes no line of its own past this size.
const CAP_BYTES: u64 = 8 << 20;
/// Longest line the daemon writes; text that comes from elsewhere (an error's message) is cut.
const MAX_LINE_BYTES: usize = 1000;

/// Bytes this process has written to the log.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// Write one line, `prism-mcp daemon: <text>`, with control characters replaced and at most
/// 1000 bytes of `text`, unless the log has reached its cap. The line that would cross the cap
````

with this:

````rust
//! The daemon's log: its standard error, which the shim that starts it points at `<key16>.log`
//! (spec §4.1 step 6, §4.3). The log is truncated at daemon start when it is over 1 MiB, and the
//! daemon stops adding its own lines past 8 MiB and says so once (§4.3). A panic message, which
//! the panic hook writes, is not counted: it is followed by the end of the process.
use std::io::Write;
use std::os::fd::AsFd;
use std::sync::atomic::{AtomicU64, Ordering};

/// A log over this size at daemon start is emptied first.
const TRUNCATE_AT_START_BYTES: u64 = 1 << 20;
/// The daemon writes no line of its own past this size.
const CAP_BYTES: u64 = 8 << 20;
/// Longest line the daemon writes; text that comes from elsewhere (an error's message) is cut.
const MAX_LINE_BYTES: usize = 1000;

/// Bytes in the log as far as this process knows: what was there at start, plus its own lines.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// At daemon start: if standard error is a regular file over 1 MiB, empty it; count what is left.
/// Standard error that is not a regular file (a terminal, a pipe in a test) is left alone.
pub(super) fn start() {
    let Ok(stderr) = std::io::stderr().as_fd().try_clone_to_owned() else {
        return;
    };
    let file = std::fs::File::from(stderr);
    let Ok(metadata) = file.metadata() else {
        return;
    };
    if !metadata.file_type().is_file() {
        return;
    }
    let size = if metadata.len() > TRUNCATE_AT_START_BYTES && file.set_len(0).is_ok() {
        0
    } else {
        metadata.len()
    };
    WRITTEN.store(size, Ordering::SeqCst);
}

/// Write one line, `prism-mcp daemon: <text>`, with control characters replaced and at most
/// 1000 bytes of `text`, unless the log has reached its cap. The line that would cross the cap
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
//! The core of the shared daemon (spec §4.2): the readiness coordinator, which decides when
//! the one index is built and how long each client waits for it, and the provider slot the
//! built index is published into. Nothing here accepts a connection, takes a file lock or
//! starts a process: the accept loop, admission and retirement are slice S4.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §3.1, §4.2, §5.
// No production code calls into this module until slice S4 adds the accept loop. Remove this
// allowance there.
#![cfg_attr(not(test), allow(dead_code))]

mod admission;
mod budget;
````

with this:

````rust
//! The shared daemon (spec §3.1, §4.2): `prism-mcp daemon ...`. The readiness coordinator
//! decides when the one index is built and how long each client waits for it; the provider slot
//! holds the built index; the accept loop owns the key's files, admits connections through a
//! bounded handshake stage, and drains when the daemon retires or loses its root.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §3.1, §4.2–§4.4, §5.

mod admission;
mod budget;
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod files;
mod log;
mod peer;
mod server;
#[cfg(test)]
mod server_support;
````

with this:

````rust
mod files;
mod log;
mod peer;
mod run;
mod server;
#[cfg(test)]
mod server_support;
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod test_support;
#[cfg(test)]
mod wire_support;

// One provider is queried from many connection threads at once, one registry serves all of
// them, and the coordinator is shared by every one (spec §4.2). A field that stops being
````

with this:

````rust
mod test_support;
#[cfg(test)]
mod wire_support;

pub use run::{run_daemon, DaemonOptions, DEFAULT_IDLE_TTL_SECS, DEFAULT_MAX_CLIENTS};

// One provider is queried from many connection threads at once, one registry serves all of
// them, and the coordinator is shared by every one (spec §4.2). A field that stops being
````

Create `src/mcp/daemon/run.rs`:

````rust
//! The daemon process, `prism-mcp daemon ...` (spec §3.1 "Starting", §4.2, §8 step 4): resolve
//! the identity, own the key or leave silently, bind, write the pid file, serve, drain.
use super::coordinator::DaemonState;
use super::files::{Claim, DaemonFiles};
use super::log;
use super::server::{ServeOptions, Served, Server, Waker, HANDSHAKE_TIMEOUT};
use crate::mcp::lazy::SessionBuilder;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::session::canonical_config;
use crate::mcp::shared::identity::{DaemonIdentity, IdentityKey};
use crate::mcp::shared::runtime_dir::runtime_dir;
use crate::mcp::{ServerConfig, SessionProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `--idle-ttl` default, seconds (spec §8 step 4, §11 item 1).
pub const DEFAULT_IDLE_TTL_SECS: u64 = 900;
/// `--max-clients` default (spec §4.2, §8 step 4).
pub const DEFAULT_MAX_CLIENTS: u32 = 64;

/// What `prism-mcp daemon` was started with.
#[derive(Clone, Debug)]
pub struct DaemonOptions {
    /// The repository, the cache base, the refresh policy, and `startup`: `Lazy`, or
    /// `Background` for `--warm-at-startup` on the daemon's own command line (spec §11 item 7).
    /// `first_call_wait` is not read: each client sends its own.
    pub config: ServerConfig,
    pub idle_ttl: Duration,
    pub max_clients: u32,
    /// The full key the starting shim computed (`--key`). The daemon refuses to start when the
    /// key it computes differs, rather than adopt it (lane README, notes for S5a).
    pub expected_key: Option<String>,
}

/// Run a daemon for `options` until it drains. Returns `Ok` when it drained, and also at once,
/// silently, when another live daemon owns the key (spec §3.1: "exit 0 silently").
pub fn run_daemon(options: DaemonOptions) -> anyhow::Result<()> {
    log::start();
    let cfg = canonical_config(&options.config)?;
    let identity = DaemonIdentity::resolve(&cfg)?;
    let key = identity.key();
    if let Some(expected) = &options.expected_key {
        let expected = IdentityKey::from_hex(expected)
            .map_err(|error| anyhow::anyhow!("--key is not an identity key: {error}"))?;
        anyhow::ensure!(
            expected == key,
            "--key {expected} is not the identity this daemon computes for its configuration \
             ({key}); the repository root or the cache base changed between the two resolutions"
        );
    }
    let dir = runtime_dir()?;
    let mut files = match DaemonFiles::claim(&dir, &key)? {
        Claim::Owned(files) => *files,
        Claim::Taken => return Ok(()),
    };
    let listener = files.bind()?;
    let pid = std::process::id();
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    files.write_pid(pid, start_time, &identity.build().cache_build_identity)?;

    let builder: SessionBuilder = {
        let cfg = cfg.clone();
        Arc::new(move || SessionProvider::bootstrap(&cfg))
    };
    let state = DaemonState::new(builder, cfg.startup)?;
    let served = Served {
        key,
        repo_root: identity.repo_root().to_path_buf(),
        root_fs: identity.root_fs(),
        pid,
        start_time,
    };
    let options = ServeOptions {
        max_clients: options.max_clients,
        idle_ttl: options.idle_ttl,
        handshake_timeout: HANDSHAKE_TIMEOUT,
    };
    let socket = files.paths().socket().to_path_buf();
    let mut server = Server::new(
        state,
        Arc::new(ToolRegistry::all_v1()),
        files,
        listener,
        served,
        options,
    )?;
    install_signal_handlers(&server.waker())?;
    server.set_signalled(signalled);
    log::line(&format!(
        "pid {pid} serves {} for {} at {}",
        key.locator(),
        identity.repo_root().display(),
        socket.display()
    ));
    let reason = server.run()?;
    log::line(&format!("pid {pid} stopped: {}", reason.describe()));
    Ok(())
}

/// The descriptor the signal handler writes to; -1 until one is installed.
static SIGNAL_WAKE_FD: AtomicI32 = AtomicI32::new(-1);
static SIGNALLED: AtomicBool = AtomicBool::new(false);
static WAKE_BYTE: u8 = 1;

/// SIGTERM and SIGINT: only async-signal-safe work, a flag and one `write`.
extern "C" fn on_termination_signal(_signal: libc::c_int) {
    SIGNALLED.store(true, Ordering::SeqCst);
    let fd = SIGNAL_WAKE_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        // SAFETY: `write` is async-signal-safe; `fd` is a descriptor that is never closed and
        // `WAKE_BYTE` a static byte.
        unsafe {
            libc::write(fd, std::ptr::addr_of!(WAKE_BYTE).cast(), 1);
        }
    }
}

fn signalled() -> bool {
    SIGNALLED.load(Ordering::SeqCst)
}

/// Make SIGTERM and SIGINT drain the daemon (spec §3.1) instead of ending it where it stands.
fn install_signal_handlers(waker: &Waker) -> anyhow::Result<()> {
    SIGNAL_WAKE_FD.store(waker.leak_raw_fd()?, Ordering::SeqCst);
    let handler = on_termination_signal as extern "C" fn(libc::c_int);
    for signal in [libc::SIGTERM, libc::SIGINT] {
        // SAFETY: the handler does only async-signal-safe work (an atomic store and `write`).
        let previous = unsafe { libc::signal(signal, handler as libc::sighandler_t) };
        anyhow::ensure!(
            previous != libc::SIG_ERR,
            "could not install the handler for signal {signal}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
        self.stop.store(true, Ordering::SeqCst);
        self.wake();
    }
}

/// How a peer's uid is read. Production: `peer::peer_uid`. A test can play another user.
````

with this:

````rust
        self.stop.store(true, Ordering::SeqCst);
        self.wake();
    }

    /// A descriptor that wakes the loop when one byte is written to it, for a signal handler.
    /// It is a duplicate that is never closed, so it stays valid for the life of the process.
    pub(super) fn leak_raw_fd(&self) -> io::Result<i32> {
        use std::os::fd::IntoRawFd;
        Ok(self.stream.try_clone()?.into_raw_fd())
    }
}

/// How a peer's uid is read. Production: `peer::peer_uid`. A test can play another user.
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
    peer_uid: PeerUid,
    pending: Vec<Pending>,
    next_identity_check: Instant,
    #[cfg(test)]
    turn: Option<TurnHook>,
}
````

with this:

````rust
    peer_uid: PeerUid,
    pending: Vec<Pending>,
    next_identity_check: Instant,
    /// Set when the loop finds that a signal arrived (`signals.rs`).
    signalled: fn() -> bool,
    #[cfg(test)]
    turn: Option<TurnHook>,
}
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
            peer_uid: Box::new(peer_uid),
            pending: Vec::new(),
            next_identity_check,
            #[cfg(test)]
            turn: None,
        })
````

with this:

````rust
            peer_uid: Box::new(peer_uid),
            pending: Vec::new(),
            next_identity_check,
            signalled: || false,
            #[cfg(test)]
            turn: None,
        })
````

In `src/mcp/daemon/server.rs`, replace this:

````rust

    pub(super) fn waker(&self) -> Waker {
        self.waker.clone()
    }

    #[cfg(test)]
````

with this:

````rust

    pub(super) fn waker(&self) -> Waker {
        self.waker.clone()
    }

    /// Where the loop learns that a termination signal arrived.
    pub(super) fn set_signalled(&mut self, signalled: fn() -> bool) {
        self.signalled = signalled;
    }

    #[cfg(test)]
````

In `src/mcp/daemon/server.rs`, replace this:

````rust
    /// periodic root and files check, and the idle retirement decision.
    fn check(&mut self) -> anyhow::Result<Option<StopReason>> {
        let files = &self.files;
        if self.waker.stop.load(Ordering::SeqCst) {
            return Ok(Some(
                self.state
                    .drain(StopReason::Signal, || files.unlink_socket())?,
````

with this:

````rust
    /// periodic root and files check, and the idle retirement decision.
    fn check(&mut self) -> anyhow::Result<Option<StopReason>> {
        let files = &self.files;
        if self.waker.stop.load(Ordering::SeqCst) || (self.signalled)() {
            return Ok(Some(
                self.state
                    .drain(StopReason::Signal, || files.unlink_socket())?,
````

In `src/mcp/mod.rs`, replace this:

````rust
pub mod tools_refresh;
pub mod transport;

pub(crate) use session::{AutoRefreshSummary, BuildStage, RefreshVerification};
pub use session::{
    CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider, StartupMode,
````

with this:

````rust
pub mod tools_refresh;
pub mod transport;

#[cfg(unix)]
pub use daemon::{run_daemon, DaemonOptions, DEFAULT_IDLE_TTL_SECS, DEFAULT_MAX_CLIENTS};
pub(crate) use session::{AutoRefreshSummary, BuildStage, RefreshVerification};
pub use session::{
    CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider, StartupMode,
````

- [ ] **Step 4: Run the tests**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > "$LOG_DIR/t6.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/t6-lib.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo check --features mcp > "$LOG_DIR/t6-check.log" 2>&1
```

Expected: `test result: ok. 30 passed; 0 failed` (23 + 7) with the goldens' test `ok`; `121 passed; 0 failed`; the feature check `Finished` with no `warning` (the module allowance is gone, D66).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/ src/bin/prism-mcp.rs tests/mcp/
git commit -m "feat(mcp): prism-mcp daemon"
```

- [ ] **Step 6: Prove the tests bite**

Run `cargo test --features mcp --test mcp daemon_process_test` under each mutant.

| Mutant | Edit | Fails |
|---|---|---|
| 6A | in `run_daemon`, `Claim::Taken => return Ok(()),` → `Claim::Taken => anyhow::bail!("the key is taken"),` | `a_daemon_that_finds_a_live_owner_exits_silently_and_touches_nothing`, `daemons_started_together_leave_exactly_one_serving` |
| 6B | in `run_daemon`, `install_signal_handlers(&server.waker())?;` → `let _ = install_signal_handlers;` | `a_termination_signal_drains_a_daemon_with_a_shim_attached` |
| 6C | in `run_daemon`, `if let Some(expected) = &options.expected_key {` → `if let Some(expected) = None::<&String> {` | `a_daemon_handed_another_key_refuses_to_start` (it waits out the 60 s limit) |

### Task 7: S4-2 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.

Shared-daemon core (`src/mcp/daemon/`, Unix only; it has no production caller until the accept
loop of a later slice): `coordinator.rs` holds `DaemonState`, which is a control plane
(`Mutex<ControlState>` and one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the
attempt count, the parked calls, the generation of the provider in the slot) and a provider slot
(`RwLock<Option<SessionProvider>>`). `ensure_ready` picks its build attempt once and waits for
````

with this:

````markdown
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.

Shared daemon (`src/mcp/daemon/`, Unix only; `prism-mcp daemon --repo X [--cache-dir D]
[--refresh-policy P] [--warm-at-startup] [--idle-ttl S] [--max-clients N] [--key HEX]`, internal,
started by the shim of a later slice): `coordinator.rs` holds `DaemonState`, which is a control plane
(`Mutex<ControlState>` and one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the
attempt count, the parked calls, the generation of the provider in the slot) and a provider slot
(`RwLock<Option<SessionProvider>>`). `ensure_ready` picks its build attempt once and waits for
````

In `CLAUDE.md`, replace this:

````markdown
while it is still the one it bound), and the pid file; every file but the socket is opened
through the validated runtime directory's descriptor with `O_NOFOLLOW`, and lock files are
created with `O_EXCL` and never removed.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

with this:

````markdown
while it is still the one it bound), and the pid file; every file but the socket is opened
through the validated runtime directory's descriptor with `O_NOFOLLOW`, and lock files are
created with `O_EXCL` and never removed.
`server.rs` is the accept loop, one thread polling the listener and every pending connection:
a peer of another uid (`peer.rs`) is dropped before any reply and never counted; a pending
connection's `HELLO` is read from a non-blocking socket against its own deadline (10 s) and
answered `READY` (then a worker thread runs `serve_admitted` and releases the admission when it
returns), `MISMATCH` (another identity or protocol version), `BUSY` (over `--max-clients`), or
nothing (a refused line). The root's device and inode and the daemon's own files are checked on
every admission and every 5 s; a change, SIGTERM/SIGINT, or the idle retirement drains the
daemon: socket unlinked under the control lock, `RETIRING` to everything still pending or queued,
pid file removed, owner lock released, exit 0. `run.rs` is `run_daemon`: resolve the identity
(refusing a `--key` that differs), own the key or exit 0 silently, bind, write the pid file,
serve. `log.rs` caps the daemon's own lines (log truncated at start over 1 MiB, capped at 8 MiB).

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
````

- [ ] **Step 2: Run the MCP surface and the checks**

The commands of Task 4 Step 2, with `s42` in the log names. Expected: `cargo fmt --check` silent; `test result: ok. 507 passed; 0 failed` (469 + 38); `30 passed` with the goldens' test `ok`; both checks `Finished`, no `warning` in the feature check; clippy 257 `warning` lines, none naming `src/mcp/daemon/`, `src/bin/prism-mcp.rs` or `tests/mcp/daemon_process_test.rs`.

- [ ] **Step 3: State the invariants**

The commands of Task 4 Step 3, except that the first no longer lists `src/bin` and the second also excludes `src/mcp/mod.rs`, `src/bin/prism-mcp.rs` and `tests/mcp/`; and:

```bash
git diff origin/main -- src/bin/prism-mcp.rs > "$LOG_DIR/s42-bin.diff"
```

Expected: the first prints nothing; `0`; no file over 600 lines; `s42-bin.diff` adds `DaemonCli`, `refresh_policy_from`, `cache_mode`, `daemon_main` and the first-argument branch, and replaces the two inline `match`/`if` blocks of `main` by calls to the helpers with the same arms.

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
for i in $(seq 1 20); do nice -n 19 cargo test --features mcp --lib mcp::daemon > "$LOG_DIR/s42-rep-$i.log" 2>&1; done
for i in $(seq 1 10); do nice -n 19 cargo test --features mcp --lib mcp::daemon -- --test-threads=1 > "$LOG_DIR/s42-rep1-$i.log" 2>&1; done
for i in $(seq 1 5); do nice -n 19 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp daemon_process_test > "$LOG_DIR/s42-proc-$i.log" 2>&1; done
grep -h '^test result' "$LOG_DIR"/s42-rep-*.log "$LOG_DIR"/s42-rep1-*.log "$LOG_DIR"/s42-proc-*.log | sed 's/finished.*//' | sort | uniq -c
pgrep -fl 'prism-mcp daemon'
nice -n 19 env CARGO_BUILD_JOBS=4 env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp > "$LOG_DIR/s42-full.log" 2>&1
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --doc --features mcp > "$LOG_DIR/s42-doc.log" 2>&1
```

Expected: `30 test result: ok. 121 passed; 0 failed`, `5 test result: ok. 7 passed; 0 failed`; `pgrep` prints nothing; the full suite and the doc tests `0 failed`.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S4-2 is the daemon as a process: it owns its key, admits through a bounded handshake stage, retires and drains. (2) **Invariants:** Step 3. (3) **Decisions:** D52, D54–D57, D60–D64, D66, D68, D69, D71, SI-16–SI-20. D54 (no reply to a refused line), D56 (one polling thread) and D69 (a panic in the loop) are the ones a reviewer is most likely to want to change. (4) **Claims:** the rows for Tasks 5 and 6, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base, and mutants 5A–6C as observed. (6) **Not verified:** Linux; the shim halves of tests 2, 3, 4, 8, 13 and 15 (S5a); the log's cap. (7) **Tier-A:** not run. (8) The owner questions of this plan, as defaults built.

```bash
git add CLAUDE.md
git commit -m "docs(mcp): the shared daemon's accept loop and process"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): accept loop, handshake stage and prism-mcp daemon (shared daemon S4-2)" --body-file "$LOG_DIR/s42-pr-body.md"
```

Review brief and cap as in Task 4.

---

## Interfaces for the next slices

Everything below is in `crate::mcp::daemon` and `pub(super)` unless a path says otherwise.

**S5a (the shim) consumes:**

- **The command line:** `prism-mcp daemon --repo <canonical root> --cache-dir <the canonical cache base> --refresh-policy <warn-only|auto-full|auto-incremental> [--warm-at-startup] [--idle-ttl <s>] [--max-clients <n>] --key <the shim's full 64-hex key>`, with `PRISM_MCP_RUNTIME_DIR` set to what the shim resolved (or the same environment the shim resolved it from). The daemon resolves its own key from these and refuses to start when it differs from `--key` (D61).
- **Exit status and log:** 0 when it served and drained, and also, silently, when another daemon owns the key (the losing child of a race, spec §4.1 step 6); non-zero with one `Error: …` line on standard error when it could not start (runtime directory, `--key`, bind, an identity that cannot be resolved). The daemon's own lines start `prism-mcp daemon: `; the start line names its pid, key locator, root and socket. **Open the log with `O_APPEND`** (and point both standard output and standard error at it): the daemon truncates its standard error when it is a regular file over 1 MiB (SI-16).
- **Files:** the daemon takes `<key16>.owner` itself, replaces a stale socket, writes `<key16>.pid` as `pid\nstart_time\nbuild_identity\n` (build identity = `PRISM_CACHE_BUILD_IDENTITY`), and removes the socket and the pid file when it drains. After an abort or a crash the socket and the pid file remain and the owner lock is free (S3b); the shim's step 6 replaces them under the owner lock. `READY`'s `pid` and `start_time` equal the pid file's first two lines. Lock files are created with `O_EXCL` then opened (D53); a shim that opens `<key16>.start` or `<key16>.owner` must do the same on macOS.
- **What a connection is told before `READY`:** `RETIRING` (draining; re-enter arbitration), `BUSY {"max_clients":N}` (fail closed, spec §10), `MISMATCH {"expected":…}` (another identity or protocol version; fail closed), or nothing and EOF (a refused line, a handshake not finished within 10 s, a peer of another uid; transient for the shim, spec §4.1 step 5). A shim must send its `HELLO` within 10 s of `connect` succeeding.
- **The shim's peer check** (the lane's S5a design item): `peer::peer_uid` is the call; move it to `shared/` (`pub(crate)`) rather than copy it.
- **For its tests:** `tests/mcp/daemon_process_test.rs`'s `setup`, `Daemon` (killed and reaped on drop) and `Shim`; `server_support`'s `Running` (`settle`, `hold`, `advance_and_wake`, `stopped`) for anything in-process.
- **Still open, S5a's to settle:** SI-10's restart bound (a recurring builder panic aborts every daemon on its first tool call); SI-17 (`PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` in the daemon's environment); the shim halves of spec tests 2–4, 8, 13 and 15.

**S5b (`daemon-status`, `daemon-shutdown`) consumes:** `DaemonState::status()` and `admission_status()` (each with a narrow dead-code allowance until then, D66); `StopReason` (a graceful `daemon-shutdown` is a new reason, recorded by a variant of `retire_if_idle` that ignores the TTL but still needs admitted == 0); the files cleanup rule for other keys (D70); the wire verbs and whether they bump `SHARED_PROTOCOL_VERSION` (D63).

**S6 (docs)** owes, besides S3a's and S3b's lists: the daemon's command line and its exit statuses; what a client sees for each drain reason; that the runtime directory must be the same for every launcher that should share a daemon.

## Self-review record

- **Spec coverage (§12 row S4).** "Spec items implemented" above, item by item; every test of the row's list has a daemon half here and a named owner for its shim half.
- **Spec §4.2 sentences checked against the code:** "bounded handshake stage (non-blocking, preamble line ≤ 4 KiB, preamble timeout)" (`enter`, `advance_handshakes`, `PreambleLineReader`, D56); "a worker thread … allocated only after admission" (`start_worker` runs only after `Admit::Ready`); "`--max-clients` … bounds pending + admitted; beyond it, new connections are answered `BUSY` without allocating a worker" (`enter_handshake`, `Entry::Busy`, answered on the loop's thread); "Admission increments the count before `READY` is written and decrements it if the write fails" (`admit`, then the write, then `release` on failure); "Retirement requires admitted == 0 and pending == 0 for `--idle-ttl` … and no build in progress; it stops accepting and unlinks the socket inside the same critical section" (`retire_if_idle`, D50, D51); "any handshake that was pending is answered `RETIRING` before exit" (`drain`); "a daemon serves exactly the identity it was started with; the preamble compares the full identity" (`admit_hello` → `Hello::mismatch_against`); "Reads and writes happen outside every lock" (the loop writes every reply after its admission call has returned).
- **The lane README's notes for S4, one by one.** Validation is stricter than §4.3 and is hardening (`files.rs` module doc, D58). "Open the lock, pid and log files through a held directory descriptor" (D58; the log is opened by the shim, S5a). "'Validate on every use' is a convention the types do not carry" (`DaemonFiles::claim` is the one function that validates and then builds `DaemonPaths`; nothing else in S4 opens a daemon file). "If the runtime directory is removed under a live daemon … compare its socket's inode" (`still_ours`, D57). "every rebuild leaves five files per repository … A cleanup rule is needed (S4 or S5b)" (D70, S5b). "Delete-and-recreate of a repository root can reuse the inode" ("What each mechanism covers"). "`PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`" (SI-17). From the S2b reviews: "Admission is a convention the types do not carry" (`admit_hello` is the one call site that reads a `HELLO`'s settings, after the identity comparison); "`PreambleLineReader::read_line` has no deadline" (D56); "A `HELLO` whose settings are refused … cannot be answered `MISMATCH`" (D54); "New verbs … bump `SHARED_PROTOCOL_VERSION`" (D63); "A daemon builds its `ConnectionSettings` from `ClientSettings::output()` only" (unchanged, S3a's `serve_client`).
- **S3a's and S3b's interfaces for S4, one by one.** The guard after the lock for every new entry (D48). The accept loop outside both domains (D69). `serve_admitted` no longer unwinds; the decrement after it returns, on every path (D68). After an abort the files remain (Task 6's tests start each daemon in a fresh directory; S5a replaces them). `DaemonState::new(builder, startup)` with `Background` for `--warm-at-startup` (Task 6, `run_daemon`). `serve_admitted` sets the stream blocking; the handshake stage left it non-blocking and set no write timeout (S3a's D33 note). `ControlState` holds the admission state (D49). The Background parity requirement (D59, Task 2). "An attempt has no end if the builder never returns" (D67). "Any new kind of waiter on the control plane's one condition variable must keep `notify_all`": S4 adds no waiter (the loop waits in `poll`). "`BuildPhase::Failed { error }` … bound and strip control characters wherever it is logged or sent": S4 neither logs it nor sends it (`READY` carries the state word only); `budget_for` copies it into an outcome, as S3a's calls do. "Remove `#![cfg_attr(not(test), allow(dead_code))]`" (D66). "SI-5 (a parked call of a departed client holds its admitted slot for up to its first-call wait)": unchanged; the slot is released when that call returns and the serve loop sees end of input, within the client's own wait (at most 600 s).
- **The lane's review lessons.** (1) Every stated property has a test and a mutant, or is listed as not held: "Claims". Two mutants survived in validation (1D, 3C) and the tests were strengthened, not the mutants waived. (2) What each mechanism does not cover: its section. (3) Interleavings are played, not hoped for: the retirement race holds the loop at a turn (Task 5); the start race runs four processes (Task 6). (4) No test asserts an upper bound on a duration or sleeps to order events. (5) Nothing logged carries more than 1,000 bytes or a control character (`log::line`). (6) No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests in the new code; every `unsafe` block is one FFI call with a `SAFETY` comment.
- **Size.** From the validated branch `scratch/s4-tasks4`: S4-1 (`36c9aaf5..289fbb5f`) `src/` 11 files, +1,488 −16, code about 760; S4-2 (`289fbb5f..0289a19b`) `src/` and `tests/` 15 files, +2,037 −18, code about 940; `CLAUDE.md` +13, then +14 −2. The largest file edited is `coordinator.rs` (592 lines); the largest created is `server.rs` (521).

## Validation record

Built on 2026-10-14 on macOS (arm64), in `~/code/prism-s4-validate`, from `main` `36c9aaf5`. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file under the session's scratch directory, read from the file, at host load averages between 20 and 45. Nothing was pushed. Local branches:

- `scratch/s4-validate`, where the code was first written, test first: `b9b03870` (Task 1), `ccfbfe51` (Task 2), `1d1bab14` (Task 3), `7b349dfa` and `0964dd7e` (S4-2 as one piece).
- `scratch/s4-tasks`, the same source cut into this plan's tasks, then the folds found by the mutants and the repeats (`fold:` commits), on which every mutant ran.
- `scratch/s4-tasks4`, the final cut, one commit per task: `e92f55d3` (Task 1), `7b0225b0` (2), `716afb53` (3), `289fbb5f` (4, the S4-1 head), `4d86d0b8` (5), `4af3335c` (6), `0289a19b` (7, the S4-2 head). Its tree is byte-identical with the folded `scratch/s4-tasks`. `scratch/s4-tasks2` and `scratch/s4-tasks3` are earlier cuts, superseded.
- `scratch/s4-replay3` (`bb35b152`): this plan's directives replayed by a script onto `36c9aaf5`, every anchor matching exactly once: byte-identical with `0289a19b`. The red and green steps of every task were run on partial replays of the plan (stopped after the task's Step 1, then after its Step 3).

| Step | Command | Observed |
|---|---|---|
| Base | `--lib mcp::`; `--test mcp` (unblessed), at `36c9aaf5` | `469 passed`; `23 passed`, the goldens' test `ok` |
| T1–T6 red and green | as in V2 and V3 | as there |
| Mutants | 1A–1F, 2A–2B, 3A–3E (`--lib` filtered to the module), 5A–5G (`mcp::daemon::server`), 6A–6C (`--test mcp daemon_process_test`) | 1A 1 of 10 failed; 1B 1; 1C 2; 1D 1 (after correction 1); 1E 2; 1F 1; 2A the parity test; 2B 2; 3A 2; 3B 1; 3C 1 (after correction 2); 3D 1; 3E 1; 5A 1 of 14; 5B 2; 5C 1; 5D 1; 5E 1; 5F 3; 5G 1; 6A 2 of 7; 6B 1; 6C 1 |
| D53's race | four, then eight, `prism-mcp daemon` processes started at once on one key, from a shell, 1 s idle TTL | before D53: 3 daemons exited 1 with `Error: cannot open …owner: No such file or directory` over 12 four-way starts with the directory pre-created, and none over 12 with the lock file pre-created; after D53: 0 errors and exactly one `serves` line in each of 40 eight-way starts |
| D53's mechanism | a C probe: eight forked processes `openat` one new name in a fresh directory, 300 times | plain `O_CREAT`: `ENOENT` 1, 3, 1, 1 per 2,400 opens with `O_NOFOLLOW` and 0, 2, 0, 0 without; `O_CREAT|O_EXCL` then plain open: 0 in 4 × 2,400 |
| V4–V12 | as in the table at the top | as there |

**Corrections made to the plan by the validation.**

| # | Where | What was first written | What was wrong | What it says now |
|---|---|---|---|---|
| 1 | Task 1, `DaemonState::control_is_held` | `self.control.try_lock().is_err()` | Mutant 1D (the guard dropped after the lock) survived: after a panic the lock is poisoned and unheld, and `try_lock` on a poisoned mutex is an error too, so the test read "held" | `matches!(try_lock(), Err(TryLockError::WouldBlock))`, as S3b's own lock-held test reads it; 1D killed |
| 2 | Task 3, `a_symlink_in_place_of_the_owner_lock_is_refused` | a dangling symlink only | Mutant 3C (no `O_NOFOLLOW`) survived: `O_CREAT|O_EXCL` refuses a symlink of any kind, and the fallback open of a dangling one fails anyway | a second case, a symlink to an existing file of this user, which the fallback open without `O_NOFOLLOW` would lock; 3C killed |
| 3 | Task 3 and Task 6, the stale-socket tests | the precondition `connect(stale socket).is_err()` | Failed once in a replay run (`assertion failed: UnixStream::connect(...).is_err()`): on macOS `std` makes a socket close-on-exec only after it is created, so a child another test spawns in that moment can inherit the listener and keep it accepting. Two tests' paths cannot collide (each has its own temporary directory and repository, so its own key), which leaves the inheritance as the only mechanism found; it was not reproduced on purpose | the precondition is that the socket file exists |
| 4 | Task 6, `Shim::hello` | one connection, one reply line | Failed once in the repeats: `expected READY, got Err(Malformed) from ""`, the same inherited listener accepting and then closing. The spec's shim treats EOF before `READY` as transient (§4.1 step 5) | the test shim connects again on a refused or closed connection, until a reply or its 60 s limit; then 10 × `--test mcp` green |
| 5 | Task 3, `open_private` (D53) | `openat(O_CREAT)` | the macOS race above: one of four racing daemons exited 1 | `O_CREAT|O_EXCL`, then a plain open on `EEXIST` |
| 6 | Task 5, the in-crate tests | `Client::new(stream)` on a connection the daemon closes | on macOS setting a read timeout on a socket whose peer has closed fails with `EINVAL` (three tests failed at once) | `read_all` (which ignores that error) wherever the daemon may already have closed, and `Running::hello` makes its client before it sends |
| 7 | Task 5, `the_handshake_deadline_bounds_the_whole_line` | the clock moved right after `connect` | the connection was accepted after the clock had moved, so its deadline was later than the test assumed | `settle()` after the first bytes, before the clock moves |
| 8 | Task 1, the idle start | `Admission::new(now)` read the clock at construction | S3b's `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` builds a daemon on a clock that panics when read; construction then panicked outside every guard | the idle time starts at the first retirement decision (D50) |

**Not run in this validation:** the mutation gate (it selects no MCP code; lane README); anything on Linux but the FFI type check; a release build; the Tier-A harness (no file it watches is touched); a test of the log's cap; a deliberate reproduction of the inherited-listener mechanism behind corrections 3 and 4. The same inheritance can in principle hold an accepted server-side socket open in a child of the panic test and delay an EOF an in-crate test waits for (at most until that child is killed, well inside `HANG`); 30 repeats of `mcp::daemon` showed none.
