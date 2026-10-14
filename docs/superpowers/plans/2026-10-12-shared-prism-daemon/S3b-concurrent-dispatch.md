# S3b — Concurrent Dispatch, Refresh Visibility and Panic Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-14).** Every task of this plan was built with a compiler before the plan was written, on the local scratch branch `scratch/s3b-validate` (worktree `~/code/prism-s3b-validate`) from `main` `d93d9047` (S3a merged), test first, and every mutant named in a task was applied there and killed. Review round 1 (Opus FIX, sol APPROVE) was folded the same way, test first, with its mutants applied and killed. The plan's directives were then generated from the same source cut into this plan's tasks on `scratch/s3b-tasks3` and replayed by a script onto `main`, each anchor matching once, to a tree byte-identical with that branch. Counts, red steps and mutant results below are the observed ones; the unfiltered suite and the doc tests passed on the S3b-2 head. Nothing ran on Linux, and the mutation gate was not run (it selects no MCP code).

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file that was then read, at host load averages between 20 and 62.

| # | Command | Must show | Observed |
|---|---|---|---|
| V1 | `cargo test --features mcp --lib mcp::daemon`, `--lib mcp::`, and `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` on the untouched base | The base totals; every later total is "base + N". | `66 passed`; `452 passed`; `23 passed` with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`, at `d93d9047`. |
| V2 | Each task's red step | What the step says: a compile failure, or the named test failing. | Tasks 1, 4, 5, 6: compile errors, as each step states (Task 5: two). Task 1 with its seam and without its fix: the named test failed (`2 passed; 1 failed`). Task 5's lock-held test failed against the guards as first placed. Tasks 2, 3 and 7 have none: Task 2 proves properties S3a's construction already has, and its mutants show the tests bite. |
| V3 | Each task's green step | The count the step states. | Task 1: 69. Task 2: 72. Task 4: 76, with one warning that Task 5 removes. Task 5: 77. Task 6: 83. |
| V4 | `cargo test --features mcp --lib mcp::` at the end of each part | base + 4, then base + 13; `0 failed`. | `458 passed` at the S3b-1 head (`c4c57cdd`); `469 passed` at the S3b-2 head (`d332af28`); `0 failed` both. |
| V5 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `23 passed`, the golden transcripts unblessed. | `23 passed`, the goldens' test `ok`, at `d332af28`, and again inside the full suite (neither part edits `transport.rs`). |
| V6 | `cargo check`, `cargo check --features mcp`, `cargo check --tests --features mcp,detached-owner-audit` | `Finished`; no warning from the feature build. | `Finished` × 3 at `d332af28`; 0 warnings from the feature build; the owner-audit build prints only the base's warnings in `tests/`. |
| V7 | `cargo clippy --all-targets --features mcp -- -W clippy::all`, filtered to `src/mcp/daemon/` | No line. | No line, at `c4c57cdd` and at `d332af28` (257 `warning` lines in each whole log). |
| V8 | Every "prove the tests bite" step | The named tests fail under the named mutant. | All 16 mutants killed on the final code: Task 1 A–B, Task 2 A–D, Task 4 A, Task 5 A–D, Task 6 A–E. Task 2 C (the provider guard held across the write) was killed by the held-write test with default and with 212,992-byte socket buffers, at Task 2's code and at the end; Task 5 C was killed only after its test was made to count each find (Validation record). |
| V9 | 30 runs of `mcp::daemon`, default threads and `--test-threads=1` | 30 × `ok`, `0 failed`. | `30 test result: ok. 82 passed; 0 failed` in each mode, at `d332af28` (the 83rd test, the process test, skipped; test time 1.25–1.29 s with default threads, 1.78–2.04 s with one), load average about 22–24; the process tests alone, 5 times: `5 test result: ok. 6 passed; 0 failed`; afterwards `pgrep -f a_daemon_process_for_the_panic_test` printed nothing. |
| V10 | The zero-diff checks of Tasks 3 and 7 | `0` for `transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, the goldens and every serving file outside `src/mcp/daemon/`. | `0`; 15 paths changed, all under `src/mcp/daemon/` but `CLAUDE.md`. |
| V11 | CI on Linux | Green. | **Not run.** |
| V12 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp` and `cargo test --doc --features mcp` at the S3b-2 head | `0 failed`. | 5,762 passed, 0 failed, 1 ignored; doc tests `2 passed`. |

**Still not verified:** the mutation gate (it selects no MCP code; lane README); the unfiltered suite at the S3b-1 head alone (it ran at the S3b-2 head: 5,762 passed, 0 failed); anything on Linux, where socket buffers are larger, the stalled client of Task 2 sends more frames before its socket is full, and the cut-off-frame path is likely to run; a release build; the spec's tests 6, 7, 9 and 12 through a real shim and a real daemon process started by one (S5a).

## Decisions owed

**None.** Nothing here needs the owner before it is built or before its pull requests are opened. Two findings are recorded for the owner's attention under "Spec issues found", with recommendations, because they bear on the lane goal and on the spec's text, and neither is a question the owner has answered: SI-10 (a panic on the build thread is fatal by the spec's text, where the in-process server reports it as a failed build and retries) and SI-11 (a refresh holds the write lock for the whole rebuild, so every other client waits for it). The plan follows the spec on both; SI-10 is one function body to reverse, and SI-11 is measured by the fan-out gate the spec already requires (§6.2) before anything is changed.

---

**Goal:** the lane's: let the a2a-bridge run its reviewer and implementer fan-out on this host without swap exhaustion, by holding one resident index per compatible (checkout, prism build, cache configuration, refresh policy), with no query-latency regression under fan-out. **What S3b must make true, and test:** many clients of one daemon are served at once, each with its own output settings, and a client that stops reading stalls nobody else (spec §9 tests 7 and 9); a refresh by one client is what every other client's next query sees (test 6); two clients that race on one stale index under an automatic policy refresh it once, and a refresh that is still needed is not skipped (§4.2 "re-checks its trigger after acquiring exclusive access"); and a panic ends the daemon exactly where the spec says it does and a connection where the spec says it does (§3.1, §4.2 "Panic policy", §10; test 12). The in-process server does not change.

**Architecture:** S3a's `connection::run_tool_call` takes the read lock for a query and, when an automatic policy finds the index stale, takes the write lock and refreshes. S3b changes that last step: under the write lock the call asks the read's question again (`dispatch_tool_read` on the same provider); if the index is now current it answers as a read, otherwise it refreshes from the report found under this access. Responses are owned values, so no lock is held while one is written; Task 2 proves it with a client that stops reading while another refreshes. For panics, a new `daemon/fatal.rs` holds `Fatal`, the injected `OnFatal` handler (production: one line on stderr, then `std::process::abort()`), and `FatalOnUnwind`, a guard made at the entry of each piece of code in a domain (`read`, `write`, `ensure_ready`, `status`, the background start; the build thread's existing `PanicGuard`) that calls the handler if the thread unwinds through it. Every `Poisoned` error is made by a function that calls the handler first. A panic outside those domains reaches `serve_bounded`, a `catch_unwind` around a connection's serve loop, and ends that connection only.

**Tech Stack:** Rust 2021, `std::sync::{Mutex, Condvar, RwLock, Barrier}`, `std::panic::catch_unwind`, `std::process::{abort, Command}`, `std::os::unix::net::{UnixStream, UnixListener}`, `serde_json`, `anyhow`, `libc` (already an optional dependency of the `mcp` feature; one constant, `SIGABRT`, in a test); `tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§3.1 "Fatal", §4.2 "Provider", "Per connection", "Panic policy", §4.5, §5, §9 tests 6, 7, 9 and 12, §10, §11, §12 row S3b, §13). Lane plan: `README.md` in this directory ("Notes for the planners of later slices", S3b; the owner decisions table). Built on S3a as merged: `main` `d93d9047` (S3a-1 #374, S3a-2 #377, the spec clarification #383). Decisions are numbered on from the S3a plan's (D1–D33) and spec issues from its SI-1–SI-8, so that one number names one thing across the lane.

**Ships as two pull requests.** **S3b-1 = Tasks 1–3** (the re-check, refresh visibility, the concurrency proof): 6 files, 694 insertions and 12 deletions in `src/`, of which about 45 are code, plus 8 lines of `CLAUDE.md`. **S3b-2 = Tasks 4–7** (the panic boundaries): 10 files, 769 insertions and 34 deletions in `src/`, of which about 210 are code, plus `CLAUDE.md`. Each is well under S3a-1 and S3a-2 (about 2,100 lines each), which each converged in two review rounds. They are split because they ask a reviewer different questions (a race on a lock against what ends a process) and because S3b-2 changes the expectations of four S3a tests, which a reviewer should judge on their own. S3b-2 is based on S3b-1 (both edit `connection.rs`); nothing in S3b-1 depends on S3b-2.

## Global Constraints

- All work is under `--features mcp`, in `src/mcp/daemon/` and `CLAUDE.md`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged, byte for byte.** No file outside `src/mcp/daemon/` and `CLAUDE.md` is edited: not `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `src/mcp/shared/*`, `src/bin/prism-mcp.rs`, `tests/`, nor the golden transcripts under `tests/fixtures/mcp/golden/` (never set `PRISM_MCP_BLESS_GOLDEN`). Tasks 3 and 7 check it.
- The in-process server keeps `LazySessionProvider` and the `SessionRuntime` trait unchanged (spec §4.2).
- Do not re-decide S3a's decisions D1–D33. Two are changed on purpose and say so: D14's list of S3b work (D35 keeps the post-refresh query under the write lock) and D10/D11's interim panic behavior (D39–D43 replace it as D10 and D11 said S3b would).
- Spec values, copied exactly:
  - "`refresh_index` and automatic refresh take the write lock; automatic refresh re-checks its trigger after acquiring exclusive access, and every response is bound to the generation it actually queried" (§4.2).
  - "Reads and writes happen outside every lock; a slow reader can hold at most one bounded response in flight" (§4.2).
  - "A panic anywhere in provider dispatch, the build thread or the control plane aborts the daemon (§3.1 Fatal); no lock is ever unwrapped past poisoning. Panics in a connection's framing or output shaping (outside both domains) close that connection only; the implementation marks these boundaries explicitly" (§4.2). "Any panic inside provider dispatch, the build thread or the control plane is daemon-fatal: the process aborts, attached clients see EOF (exactly what a crash of an in-process server looks like today), and the next shim restarts the daemon through the ordinary start protocol" (§3.1).
  - §10: "Panic in provider dispatch, build, or control plane → Daemon → Abort (fatal); clients see EOF"; "Panic in a connection's framing/output → Daemon → That connection closes".
  - Test 6: "await A's completed refresh, then B's query returns the changed name and the new generation". Test 7: "barrier-released requests from N shims with correlated ids and expected payloads, plus one blocked writer; every response complete and matched". Test 9: "two shims with different `PRISM_MCP_STRUCTURED_CONTENT` get their own shapes from one daemon". Test 12: "an injected panic in dispatch aborts the daemon; all attached shims see EOF; the next shim restarts it" (§9).
- **No panic on a path a client can reach** in code this slice adds: no `unwrap`, `expect`, indexing or unchecked arithmetic outside tests. `abort_daemon` writes its line with `writeln!` and ignores the result, because `eprintln!` panics when standard error is gone and it runs while a thread unwinds.
- **Tests do not depend on the host's speed.** A test waits for a condition up to `HANG` = 30 s and asserts no upper bound on any elapsed time. No test sleeps to order two events: Task 1 holds calls at a test seam, Task 2 releases clients with a `Barrier` and detects a full socket by `WouldBlock`.
- **No test aborts the test binary.** `Parts::production` aborts on a fatal event. Every in-crate test that reaches one installs a `FatalLog` first; the one test that needs the real abort runs it in a child process (Task 6).
- Keep every file under 600 lines; tests in sibling `*_tests.rs` files included with `#[path]`. The largest file this slice edits is `coordinator.rs`, 548 lines after it (`connection_tests.rs`, 577, is not edited); the largest it creates is `connection_panic_tests.rs`, 314.
- No new clippy warning in a file this slice adds or edits.
- **Commit before any experiment.** The "prove the tests bite" steps edit a source file and restore it with `git checkout -- <file>`, which discards uncommitted work in it; they come after the task's commit.
- Logs go to files; read the file. Do not pipe cargo output into `grep` or `tail`.

### What each mechanism covers, and what it does not

- **The re-check stops a redundant refresh; it does not serialize refreshes.** Two explicit `refresh_index` calls each rebuild, as two in-process calls would. Two automatic refreshes run one after the other (the write lock), and the second runs only if the index is still stale when it gets the lock.
- **A refresh holds the write lock for the whole rebuild** (`SessionProvider::refresh_verified` builds the new snapshot under `&mut self`). Every query that arrives meanwhile waits for it, as long as it takes, on every connection (SI-11). The first-call budget does not bound that wait (S3a's "What each mechanism covers").
- **"No lock across a write" is held by construction and by a deterministic test.** `run_tool_call` returns an owned `Value`; `serve_loop` writes it after the closure has returned. Task 2's `a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held` holds one connection inside `write_message` at a gate and requires another client's refresh and query to complete meanwhile, on any host. Task 2's socket test (a client that reads nothing until its socket is full) is a smoke test of completion and settings under load: whether its stalled thread is blocked in `write` when the refresh runs depends on socket-buffer sizes and scheduling (review round 1: with Linux-sized buffers a provider guard held across the write passed it in three of three runs), so no claim rests on it.
- **A stalled client is held by the kernel's socket buffers, not by the daemon's memory:** its connection's thread holds one response it is writing, and its unread requests sit in the socket. The spec's "at most one bounded response in flight" holds per connection. A client that never reads holds its thread until it disconnects.
- **The abort ends the process; it does not clean up.** The socket file and the pid file stay; the next shim's start protocol (spec §4.1 step 6, S5a) finds the owner lock free and replaces them. On macOS each abort also writes a crash report (about 80 KB) to `~/Library/Logs/DiagnosticReports/`, one per run of Task 6's process test.
- **A test's `FatalLog` returns where the abort would not.** The code after each fatal call then runs (the build attempt is ended as failed, the caller gets `Poisoned`, the connection ends with an error). That code keeps tests bounded; no running daemon reaches it, and no claim of this plan rests on it except that it does not hang.
- **The connection's boundary is only as good as the build's unwinding.** A `panic = "abort"` profile would turn every panic, inside a domain or not, into the process's end. No profile in `Cargo.toml` sets it.

### Panic boundaries: what is caught where, and what the client sees

| Where the panic is raised | Code that runs there | Caught by | What happens (production) | What every attached client sees |
|---|---|---|---|---|
| A tool handler or result shaping in a query | `dispatch_tool_read` inside `DaemonState::read` | `FatalOnUnwind` (`Domain::Dispatch`) made in `read` right after the slot's lock is taken | One stderr line `prism-mcp daemon: a panic in provider dispatch; …` after the panic message; `abort()`, while the panicking call still holds its lock | EOF on its connection (spec §3.1). The panicking call gets no response. A call already running on another connection under a shared lock (another query) may still finish and be answered before the abort; no call that needs the lock the panic happened under gets it |
| A refresh, or the query after an automatic refresh | `dispatch_tool_refresh`, `dispatch_tool_auto_refresh` inside `DaemonState::write` | the same guard, made in `write` right after the slot's lock is taken | same | same |
| The builder, publication, or recording the outcome | `run_build` on the build thread | `PanicGuard` (S3a's), whose drop now calls `on_fatal(Panicked(Build))` | `… a panic in the index build; …`; `abort()` | same |
| The readiness coordinator under the control lock, including the injected `Clock` and `Spawner` | `ensure_ready`, `status`, the start of a `--warm-at-startup` build in `from_parts` | `FatalOnUnwind` (`Domain::Control`), made in each right after the control lock is taken | `… a panic in the control plane; …`; `abort()`, the control lock still held | same |
| Nowhere: a lock found poisoned | `lock_control`, the condition-variable wait, `read`, `write`, `publish`, `settle` | not a panic; each calls `on_fatal(Poisoned { lock })` before it returns the error | `… the control lock is poisoned; …` (or `provider`); `abort()` | same |
| Framing, `handle_control`, the readiness answers (`warming`, `build_failed`, `indexing`), serializing or writing a response | `serve_loop`, `serve_client`, `run_tool_call` outside `read`/`write`/`ensure_ready` | `serve_bounded`'s `catch_unwind` | `serve_admitted` returns `Err("a panic on this connection's thread outside the index and the control plane; …")`; the caller drops the socket; the daemon goes on | That one client: EOF. Every other client: nothing |

A panic in a domain never reaches `serve_bounded` in production: the guard's drop aborts while the thread unwinds. In a test whose `FatalLog` returns, it does reach it, and the connection ends with `serve_bounded`'s error; the tests assert the `FatalLog`, not that text.

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously, or an S3a decision this slice was told to settle. "Cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D34 | **The re-check asks the read's question again.** Under the write lock, `run_tool_call` runs `dispatch_tool_read` on the same provider: `Response` (the index is now current) is the answer; `NeedsAutoRefresh` refreshes, from the report found under this access, not the one found under the read lock. | Spec §4.2's trigger is "this query finds the index stale under an automatic policy", with the sticky evidence first (S3a Task 8). Asking it again is the only re-check that agrees with the read on every state. A re-check that compared generations ("has anyone refreshed since?") would skip the retry a `Diverged` refresh needs (Task 1's third test); one that consulted the stamp probe alone would drop the sticky evidence (spec review N2). | When the trigger still holds the freshness probe runs twice, once under each lock. When it no longer holds, the one query runs under the write lock (the race window only). |
| D35 | **The query after an automatic refresh stays under the same exclusive access.** S3a's D14 listed "the query outside the write lock" as S3b work; this plan does not do it. | It binds the refresh metadata (`prism/refresh_generation`) and the result to one snapshot by construction (spec §4.2 "bound to the generation it actually queried"). Moving the query under a read lock needs a generation check after the downgrade and a loop when another refresh lands in between, in code S1 shares with the in-process server (`transport.rs`). The refresh it follows already holds every reader for the whole rebuild (SI-11), so the query adds little. | Readers wait one more query, and the first query on a fresh snapshot may build the call-edge index (lane README facts). The lane README says it "splits mechanically into refresh (exclusive) and finish (shared)" if a measurement asks for it. |
| D36 | **"Bound to the generation it actually queried" holds by construction and adds no field.** Every response is computed inside one `read` or one `write`; the only generation a response carries is the auto-refresh metadata, computed under the same `write`. | Adding a generation to every tool result would change the bytes spec §5 preserves for one client (S3a's parity test). | None. A field could be added later as a deliberate wire change. |
| D37 | **Spec test 7 is three tests** (revised by review round 1, both reviewers). (a) **The blocked writer, deterministic:** one connection is served through `serve_client` over a test transport whose response write signals that it has been entered and then waits at a gate; while it is held, another client's `refresh_index` (the provider's write lock) and its next query must complete. (b) **Overlapping reads:** a test tool whose handler returns only once two calls are inside it at the same moment; two clients' calls must both report that they met. (c) **The socket test, kept as a smoke test** of completion and of connection-local settings under load (test 9 folded in): eight clients, two each of four output-setting combinations, pipelined requests with ids of their own, a client that reads nothing until its socket is full, and a refresh; every answer byte-equal to the in-process server's. | As first written the plan had only (c) and claimed it ruled out a lock held across a write. It does not on every host: whether the stalled thread is inside `write` when the refresh runs depends on buffer sizes and scheduling, and barrier-released clients pass if queries run one at a time. (a) and (b) put the threads where the property is decided, with gates instead of buffers. S3a's `two_clients_share_one_build` already holds test 9's two-shape case. | Two more tests (about 180 lines with the tool). |
| D38 | Test seam `TestHooks::exclusive`, called through `DaemonState::before_exclusive()` with no lock held, between a call's read and its request for exclusive access. `#[cfg(test)]` only. | The race the re-check exists for needs both calls past their read before either has the write lock; nothing else can place them there without sleeping. | Removed with its tests. |
| D39 | **The fatal handler is injected:** `Parts::on_fatal: OnFatal` (`Arc<dyn Fn(Fatal) + Send + Sync>`); production `abort_daemon` writes one line to stderr and calls `std::process::abort()`. | In-crate tests must see *where* a panic is caught without ending the test binary; the real abort is tested in a child process (D44). `abort`, not `exit`: the spec's word, and `exit` would run exit handlers while other threads hold locks. The panic message is already on stderr (the default hook prints it first); the line names the domain. | A test that reaches a fatal event without a `FatalLog` aborts the whole test binary, loudly. |
| D40 | **The boundaries are drop guards, not `catch_unwind`:** `FatalOnUnwind::new(&on_fatal, domain)` in `read`, `write`, `ensure_ready`, `status` and the background start in `from_parts`, each made right after the function's lock is taken (D47); the build thread keeps S3a's `PanicGuard`, whose drop now calls `on_fatal` first. A guard made while its thread is already unwinding does not fire. | A guard needs no `UnwindSafe` reasoning, no change to any return type, and catches a panic in everything its function calls, including the injected `Clock`, `Spawner` and builder. Under a test's handler the unwinding goes on to the connection's boundary, as it would without the guard. The "already unwinding" rule keeps code that runs in a destructor during an unrelated panic from being blamed for it. | One guard per entry point; a new entry point without one is a gap (the S4 interface says so). |
| D47 | **Each `FatalOnUnwind` is made right after its function has taken its lock**, never before (review round 1, Opus F2). | Locals drop in reverse order: a guard made before the lock fires after the lock is let go, so a refresh waiting on a reader that panicked, or a call that finds the control lock poisoned, can run (or report a different cause) before the abort. Made after, `on_fatal` runs while the lock is held. Taking a lock cannot panic; a poisoned one goes through `poisoned` (D41). Task 5's `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` checks, inside `on_fatal`, that another thread's attempt to take the lock would block. | None: one line moved per entry point. The panic table also no longer says calls in flight get no response (a query already running under the shared read lock may finish first). |
| D41 | **Every `Poisoned` is made by `DaemonState::poisoned`, which calls `on_fatal(Poisoned { lock })` first**; `publish` and `settle`, which find a poisoned lock without returning an error, call it directly. | Spec §4.2: "no lock is ever unwrapped past poisoning", and §3.1 makes the cause daemon-fatal. With the guards in place a lock is poisoned only by a panic that already ended the daemon, so this is the backstop, and what the S3a tests that poison a lock from outside now exercise. | None foreseen. |
| D42 | **A panic on the build thread is fatal** (spec §3.1, §4.2, §10), settling S3a's SI-6 in favor of the spec's text. Under a test's handler the attempt still ends `Failed` with `index build panicked`, so no test hangs. | Three sections state it; the owner approved them. The difference from the in-process server, which reports `build_failed` and retries, is recorded as SI-10 with its cost. | One line in `PanicGuard::drop` (remove the `on_fatal` call) restores S3a's behavior. |
| D43 | **The connection's boundary is `serve_bounded`**, a `catch_unwind(AssertUnwindSafe(..))` around `serve_client`, called by `serve_admitted`. A panic that reaches it becomes an error with fixed text (no panic payload, no peer bytes); `serve_admitted` never unwinds into S4's worker. | Spec §4.2: "the implementation marks these boundaries explicitly". Inside the closure, after a panic nothing is used again but `state`, whose shared parts change only inside guarded functions (which end the daemon on a panic), so the unwind-safety assertion is sound. | A worker that wants the payload text can get it later; it is in the log already through the panic hook. |
| D44 | **Spec test 12 runs in a child process:** the test re-executes the library's test binary with `--exact` on a second test, `a_daemon_process_for_the_panic_test`, which, when `PRISM_MCP_TEST_PANIC_DAEMON_SOCKET` is set, binds a `UnixListener` there and serves every connection with `serve_admitted` and the production `on_fatal`. Unset (every ordinary run), it returns at once and counts as one passing test. | What test 12 checks is that the process ends and every attached client sees EOF; only a process can show that. There is no accept loop before S4 and no daemon binary before S5a; this needs neither. "The next shim restarts it" is S5a's (SI-13). | The listener is test code. S4's accept loop replaces it when test 12 is re-run through real processes. |
| D45 | **The panicking tool is a test registry entry**, `panic_in_dispatch`, added to `ToolRegistry::all_v1()` with `ToolRegistry::register`. No production hook. | `serve_admitted` already takes the registry as an argument (S3a D21), and a tool handler runs exactly where a panic in provider dispatch runs. | None. |
| D46 | **The control plane, for the guards, is `ensure_ready`, `status` and the background start.** `publish`, `settle` and `record_generation` run on the build thread (under `PanicGuard`) or inside `write` (under its guard); `wake` takes the lock to notify and does nothing that can panic, and is used by tests. | Every function that holds the control lock is reached through one of the guarded entry points or the build thread. | S4's admission and retirement functions are new entry points and need their own guard (interfaces below). |

## Spec issues found (not resolved by this plan)

- **SI-9 (§9 test 6, "and the new generation").** A query's response carries no generation, and adding one would change every response's bytes against the in-process server (§5, D36). Task 1's test reads the generation from the status snapshot (`DaemonState::status`, S3a D15), which is what S4's status probe will report; the refresh's own response does carry it (`structuredContent.generation`). Through real shims (S5a) the same test reads it from the probe.
- **SI-10 (§3.1, §4.2, §10 against §5; S3a's SI-6), for the owner's attention.** By the spec's text a panic on the build thread ends the daemon (D42). The in-process server reports the same panic as `build_failed` and retries on the next call (`lazy.rs`, "index build panicked"), so after S3b a client of a shared daemon is worse off than a standalone client in this one case: every attached agent loses its MCP session, and a deterministic builder panic (a grammar panic on one file, say) makes every new daemon abort on its first tool call, for as long as clients keep starting one. The builder runs on its own thread, holds no daemon lock while it runs, and touches no shared state until it publishes, so the spec's reason for the policy ("no lock is ever unwrapped past poisoning"; nothing served from half-changed state) does not apply to the builder itself; it does apply to publication, which holds the slot's write lock. **Recommendation:** keep the spec's text now (it is approved, and a panic in a builder is a defect to fix, not a state to serve through), and, if the owner prefers parity, change `PanicGuard::drop` to call `on_fatal` only when the panic was raised after the builder returned (publication), so that a builder panic is a failed attempt again. Not an owner question: the spec answers it; recorded because S3a asked S3b to decide it "on purpose". **What it requires of S5a** (review round 1, Opus F6): the shim's start protocol must bound the restart loop that a recurring builder panic makes (each shim start launches a daemon that aborts on its first tool call; on macOS each abort also writes a crash report); see "Interfaces for the next slices".
- **SI-11 (§4.2 "`refresh_index` and automatic refresh take the write lock" against §1 "no regression in query latency under fan-out"), for the owner's attention.** `SessionProvider::refresh_verified` builds the whole new snapshot under `&mut self`, so a refresh holds the write lock for a full rebuild, and every query of every client waits for it. Standalone clients were never blocked by one another's refresh. Under the default warn-only policy refreshes are explicit and rare; under an automatic policy the first query after any edit blocks every client for a rebuild (an incremental one under `auto-incremental` when the change allows it). The fan-out gate of §6.2 ("8 shims, p95/p99 of the workload, with one concurrent `refresh_index`") measures exactly this, and it runs after S5a. The change that removes it is in `session.rs` (shared with the in-process server): build the candidate with `&self` under the read lock, then commit under the write lock if the generation has not moved, with refreshes serialized by their own mutex so that at most one candidate exists at a time (the refresh peak is already about twice the steady footprint; C1 readout). **Recommendation:** measure at the fan-out gate before changing anything; nothing in S3b or S4 makes the change harder.
- **SI-12 (§4.2 "output shaping (outside both domains)").** A tool's result is shaped (`ToolContext`, `shape_result`, the freshness and auto-refresh metadata) inside `dispatch_tool_read` and `dispatch_tool_auto_refresh`, under the provider's lock, so a panic there is a panic in provider dispatch and ends the daemon. This plan reads "output shaping outside both domains" as what is left: the readiness and status envelopes (`readiness_response`, `retryable_status_response`) and the serialization and writing of a response. The table "Panic boundaries" above is the reading in full.
- **SI-13 (§9 test 12, "the next shim restarts it").** Needs the shim's start protocol (S5a). Task 6's process test shows the abort and that every attached client sees its connection end; the restart is S5a's to show.

## Review Focus

Six ways this code is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code.

1. **Two clients that both found the index stale.** The second to get the write lock must not rebuild an index the first has just refreshed, and must not skip a refresh that is still needed because the first one verified `Diverged`. → Task 1, `a_client_that_finds_the_index_refreshed_once_it_has_exclusive_access_does_not_refresh_again`, `a_client_that_finds_the_index_still_known_stale_once_it_has_exclusive_access_refreshes_it`. Both calls are held at the test seam after their reads, so the race is played, not hoped for.
2. **A connection blocked writing a response while another refreshes, and two queries at once.** If any lock were held while a response is written, a refresh would wait for the blocked connection for ever, and every new query behind the waiting writer with it; if queries were served one at a time, fan-out latency would add up. → Task 2, `a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held` (the write held at a gate) and `two_clients_queries_are_inside_dispatch_at_the_same_time` (a handler that waits for a second reader); `many_clients_get_their_own_answers_while_one_stops_reading_and_another_refreshes` is the smoke test under load.
3. **Responses under concurrency.** Eight clients, four settings, pipelined requests with ids of their own, a refresh in the middle: every answer whole, in order, on the right connection, in the right shape, and byte-equal to the in-process server's. → Task 2, the same test.
4. **Where a panic is caught.** A panic in a tool handler must end the daemon, not only its connection (the connection boundary would otherwise swallow it); one in framing must end the connection, not the daemon; one under the control lock must end the daemon, not leave the lock poisoned for everyone else. → Task 6, `a_panic_in_a_query_ends_the_daemon`, `a_panic_outside_both_domains_closes_that_connection_only`, `a_panic_in_the_control_plane_ends_the_daemon`; Task 5, `a_builder_panic_is_fatal`.
5. **A lock found poisoned from each place that takes it.** Each find ends the daemon, one by one; nothing reads through it. → Task 5, `a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang` (counts every find), `a_poisoned_provider_slot_fails_the_attempt_and_is_an_error_for_every_reader`, `a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on`.
6. **The production abort, seen from outside.** The process must end, by `SIGABRT`, and every attached client, including one that was idle, must see its connection end. → Task 6, `a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end`.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or comments state, with the test that fails when it breaks and one wrong implementation that test rules out. Every mutant in the last column was applied in validation and the test failed, except where the row says otherwise.

| Claim | Test | A mutant it kills |
|---|---|---|
| Clients that race on one stale index under an automatic policy refresh it once. | T1 `a_client_that_finds_the_index_refreshed_once_it_has_exclusive_access_does_not_refresh_again` | refresh under the write lock whatever the re-check finds (Task 1 A; it is the code before Task 1) |
| A refresh that is still needed after another client's refresh is not skipped. | T1 `a_client_that_finds_the_index_still_known_stale_once_it_has_exclusive_access_refreshes_it` | a re-check by generation (not applied: not a one-line change; the test is an over-reach guard and passes before Task 1 too) |
| A refresh by one client is what every other client's next query sees, and the daemon reports the new generation. | T1 `a_refresh_by_one_client_is_what_the_next_query_of_another_client_sees` | exclusive access that does not copy the generation into the control plane (Task 1 B) |
| No lock is held while a response is written; a connection blocked in a write stalls only itself. | T2 `a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held` | the provider slot's read guard held from a tool call until the next frame, across the write (Task 2 C), killed at default and at 212,992-byte socket buffers alike; a global lock held across the write (Task 2 A) |
| Two queries run inside dispatch at the same time. | T2 `two_clients_queries_are_inside_dispatch_at_the_same_time` | queries take the slot's write lock (Task 2 D) |
| A fatal event is raised while the lock its panic happened under is still held. | T5 `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` | the guard in `read` made before the lock (Task 5 D) |
| Every client gets its own answers, whole, in order and in its own shape, under concurrency. | T2 `many_clients_get_their_own_answers_while_one_stops_reading_and_another_refreshes` | output settings from the daemon's environment (Task 2 B) |
| A guard fires on a panic in its domain and only then; not for a panic it was made during. | T4 the three `fatal` tests | arm the guard whatever the thread is doing (Task 4 A) |
| A panic in a query ends the daemon. | T6 `a_panic_in_a_query_ends_the_daemon`; T6 the process test | no guard in `read` (Task 6 A) |
| A panic under the write lock ends the daemon. | T5 `a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on`, `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` | no guard in `write` (Task 5 A) |
| A panic in the control plane ends the daemon. | T6 `a_panic_in_the_control_plane_ends_the_daemon` | no guard in `ensure_ready` (Task 6 B) |
| A panic on the build thread ends the daemon. | T5 `a_builder_panic_is_fatal` | `PanicGuard` that only fails the attempt, as in S3a (Task 5 B) |
| Each find of a poisoned lock ends the daemon. | T5 `a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang` | `lock_control` that returns `Poisoned` without `on_fatal` (Task 5 C). As first written the test checked only that every recorded event was the control lock's, and this mutant survived; the test now counts each find |
| A panic outside both domains ends that connection only, and the daemon goes on. | T6 `a_panic_outside_both_domains_closes_that_connection_only` | no `catch_unwind` in `serve_bounded` (Task 6 C) |
| In production a fatal event ends the process, and every attached client sees its connection end. | T6 the process test | `abort_daemon` that writes its line and returns (Task 6 D) |
| The test's daemon process ends when the test that started it is gone. | T6 `the_daemon_process_ends_when_its_standard_input_is_closed` | no exit on standard-input end of file (Task 6 E) |

Stated and **not** held by a test: that the socket smoke test's stalled thread is blocked in `write` when its refresh runs (it need not be; the held-write test is what holds the property); the guard in `status` (review round 1, Opus F3: deleting it fails no test; `status` reads four fields under the control lock and calls no injected `Clock` or `Spawner`, so no test can make it panic without a production seam; S4's status probe is a new caller and may add one); that `abort_daemon`'s line is written when standard error is closed (it ignores the error by construction); that no guard is missing from an entry point S4 adds (S4's to keep); the `--warm-at-startup` start's guard (no test injects a panic into a background start; the guard is the same line as `ensure_ready`'s); that a failing run of the process test leaves no process behind (`DaemonProcess` kills it on drop when the test unwinds, and the child exits when its standard input closes, which the stdin test holds; observed with `pgrep` after every mutant run, not asserted); that Task 2's cut-off frame is finished without a deadlock (on this host the socket never cut a frame); anything on Linux.

## File Structure

| File | Responsibility | Change | PR |
|---|---|---|---|
| `src/mcp/daemon/coordinator.rs` | `TestHooks::exclusive`, `before_exclusive` (Task 1); `Parts::on_fatal`, the guards, `poisoned`, the fatal calls in `publish`, `settle`, `PanicGuard` (Task 5) | Modify | both |
| `src/mcp/daemon/connection.rs` | the re-check in `run_tool_call` (Task 1); `serve_bounded` (Task 6); module docs | Modify | both |
| `src/mcp/daemon/connection_refresh_tests.rs` | Spec test 6; the re-check, both ways | Create (Task 1) | S3b-1 |
| `src/mcp/daemon/connection_concurrency_tests.rs` | Spec tests 7 (smoke) and 9 | Create (Task 2) | S3b-1 |
| `src/mcp/daemon/connection_isolation_tests.rs` | Spec test 7: a response write held at a gate; two queries inside dispatch at once | Create (Task 2) | S3b-1 |
| `src/mcp/daemon/wire_support.rs` | `Client::flood_until_full`, `send_raw_in_background`, `connect_with` (Task 2) | Modify (Task 2) | S3b-1 |
| `src/mcp/daemon/fatal.rs`, `fatal_tests.rs` | `Domain`, `Fatal`, `OnFatal`, `abort_daemon`, `FatalOnUnwind`; their tests | Create (Task 4) | S3b-2 |
| `src/mcp/daemon/mod.rs` | `mod fatal;` | Modify (Task 4) | S3b-2 |
| `src/mcp/daemon/test_support.rs` | `FatalLog`, `recording_state` | Modify (Task 5) | S3b-2 |
| `src/mcp/daemon/coordinator_failure_tests.rs`, `coordinator_order_tests.rs`, `connection_parity_tests.rs` | The four S3a tests that reach a fatal event: a `FatalLog` and the events they expect | Modify (Task 5) | S3b-2 |
| `src/mcp/daemon/connection_panic_tests.rs` | Where a panic is caught; spec test 12 in a child process, which exits when its standard input closes | Create (Task 6) | S3b-2 |
| `CLAUDE.md` | MCP Adapter, the shared-daemon core paragraph | Modify (Tasks 3, 7) | both |
| `src/mcp/transport.rs`, `transport_tests.rs`, `owner_transport_tests.rs`, `session.rs`, `lazy.rs`, `framing.rs`, `output.rs`, `settings.rs`, `src/mcp/shared/*`, `src/bin/`, `tests/`, `tests/fixtures/mcp/golden/*` | The standalone contract and earlier slices' code | **Must not change** | |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file at that point of the plan, then "with this:" and its replacement.
- "Append to `path`:" is followed by text to add at the end of the file.

The replacements were generated from the validated branch's per-task commits, so each carries a few lines of unchanged context around its change; the context is part of the anchor and is not edited.

---

# Part one: S3b-1, concurrent dispatch and refresh visibility (Tasks 1–3)

### Task 1: The automatic refresh's trigger, checked again under exclusive access; refresh visibility

Under an automatic refresh policy, `run_tool_call` asks the read's question again once it holds the write lock (D34). The tests hold two calls at a test seam between their reads and their exclusive access (D38), and spec §9 test 6 runs beside them.

**Files:**
- Modify: `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/connection.rs`
- Create: `src/mcp/daemon/connection_refresh_tests.rs`

**Interfaces:**
- Consumes (all `src/mcp/transport.rs`, unchanged): `pub(super) fn dispatch_tool_read(call: &ToolCall<'_>, view: &impl SessionRuntime, settings: &ConnectionSettings) -> ReadDispatch`; `pub(super) enum ReadDispatch { Response(Value), NeedsAutoRefresh { initial_report: Option<FreshnessReport> } }`; `pub(super) fn dispatch_tool_auto_refresh(call: &ToolCall<'_>, provider: &mut impl SessionRuntime, initial_report: Option<FreshnessReport>, settings: &ConnectionSettings) -> Value`; `impl SessionRuntime for SessionProvider` (so `&*provider` is the read-only facade inside `write`). From S3a: `DaemonState::{read, write, status}`, `Parts::production`, `TestHooks`, `test_support::{client_settings, fixture, scripted, status, wait_until, Gate}`, `wire_support::{close, connect, Client, CALLEES, CALLERS, REFRESH}`, `SessionProvider::force_next_verification_for_tests` (`#[cfg(test)]`), `FreshnessReport::from_changed_paths`.
- Produces: `TestHooks::exclusive: Option<Arc<dyn Fn() + Send + Sync>>` and `#[cfg(test)] pub(super) fn DaemonState::before_exclusive(&self)`; `run_tool_call`'s new last statement. No signature changes.

- [ ] **Step 1: Write the tests**

Create `src/mcp/daemon/connection_refresh_tests.rs`:

````rust
//! Refresh visibility (spec §9 test 6) and the automatic refresh's trigger, checked again once
//! exclusive access is held (spec §4.2): two clients of one daemon, each on its own connection.
use super::super::coordinator::{DaemonState, Parts};
use super::super::test_support::{client_settings, fixture, scripted, status, wait_until, Gate};
use super::super::wire_support::{close, connect, Client, CALLEES, CALLERS, REFRESH};
use crate::mcp::freshness::FreshnessReport;
use crate::mcp::{RefreshPolicy, RefreshVerification, StartupMode};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// What the edit below adds: a second caller of `helper`. A response that names it was
/// computed on a snapshot taken after the edit.
const NEW_CALLER: &str = "extra";

fn add_a_caller(root: &Path) {
    std::fs::write(
        root.join("util.py"),
        "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
    )
    .expect("edit util.py");
}

fn names_the_new_caller(response: &Value) -> bool {
    response["result"].to_string().contains(NEW_CALLER)
}

/// Holds each call that reaches the step between its read and its exclusive access at a gate
/// of its own, in the order the calls arrive there.
struct Turnstile {
    arrived: Mutex<usize>,
    gates: Vec<Arc<Gate>>,
}

impl Turnstile {
    fn new(calls: usize) -> Arc<Self> {
        Arc::new(Self {
            arrived: Mutex::new(0),
            gates: (0..calls).map(|_| Gate::closed()).collect(),
        })
    }

    fn arrived(&self) -> usize {
        *self.arrived.lock().expect("turnstile")
    }

    fn release(&self, call: usize) {
        self.gates[call].open();
    }
}

/// A lazy daemon whose calls stop at `turnstile` before they ask for exclusive access.
fn daemon_with_turnstile(
    cfg: &crate::mcp::ServerConfig,
    turnstile: &Arc<Turnstile>,
) -> Arc<DaemonState> {
    let (builder, _builds) = scripted(cfg, vec![]);
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let turnstile = Arc::clone(turnstile);
    parts.hooks.exclusive = Some(Arc::new(move || {
        let call = {
            let mut arrived = turnstile.arrived.lock().expect("turnstile");
            *arrived += 1;
            *arrived - 1
        };
        if let Some(gate) = turnstile.gates.get(call) {
            let _opened = gate.pass();
        }
    }));
    DaemonState::from_parts(parts).expect("a lazy daemon state")
}

/// Both clients send a query on a stale index, and both get as far as asking for exclusive
/// access before either has it. Returns the two clients with their queries in flight.
fn two_stale_queries_waiting_for_exclusive_access(
    a: &mut Client,
    b: &mut Client,
    turnstile: &Turnstile,
) {
    a.send(CALLERS);
    wait_until("A's query to reach exclusive access", || {
        turnstile.arrived() == 1
    });
    b.send(CALLERS);
    wait_until("B's query to reach exclusive access", || {
        turnstile.arrived() == 2
    });
}

/// Spec §9 test 6: once A's refresh has completed, B's next query is answered from the
/// refreshed index (the changed name, no stale warning) and the daemon reports the new
/// generation. Under a warn-only policy an in-process server of B's own would have answered
/// with the old names and a stale warning; a shared index has one state (spec §5 "Changed").
#[test]
fn a_refresh_by_one_client_is_what_the_next_query_of_another_client_sees() {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();

    let before = b.request(CALLERS);
    assert_eq!(before["result"]["isError"], false, "{before}");
    assert!(!names_the_new_caller(&before), "{before}");
    assert_eq!(status(&state).generation, 0);

    add_a_caller(dir.path());
    let stale = b.request(CALLERS);
    assert_eq!(
        stale["result"]["_meta"]["prism/index_freshness"], "stale",
        "{stale}"
    );
    assert!(!names_the_new_caller(&stale), "{stale}");

    let refreshed = a.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "refreshed",
        "{refreshed}"
    );
    assert_eq!(refreshed["result"]["structuredContent"]["generation"], 1);

    let after = b.request(CALLERS);
    assert_eq!(after["result"]["isError"], false, "{after}");
    assert!(names_the_new_caller(&after), "{after}");
    assert!(
        after["result"]["_meta"]
            .get("prism/index_freshness")
            .is_none(),
        "{after}"
    );
    assert_eq!(status(&state).generation, 1);

    close(a, a_worker);
    close(b, b_worker);
}

/// Spec §4.2: automatic refresh re-checks its trigger after acquiring exclusive access. A and B
/// both find the index stale under shared access; A refreshes first. B, once it has exclusive
/// access, finds the index fresh: it is answered from A's refresh, with no refresh of its own
/// and no stale warning, and the daemon is at generation 1, not 2. Without the re-check B
/// rebuilds an index that is already current.
#[test]
fn a_client_that_finds_the_index_refreshed_once_it_has_exclusive_access_does_not_refresh_again() {
    let (dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let turnstile = Turnstile::new(2);
    let state = daemon_with_turnstile(&cfg, &turnstile);
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    add_a_caller(dir.path());
    two_stale_queries_waiting_for_exclusive_access(&mut a, &mut b, &turnstile);

    turnstile.release(0);
    let from_a = a.recv();
    assert_eq!(
        from_a["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "{from_a}"
    );
    assert_eq!(from_a["result"]["_meta"]["prism/refresh_generation"], 1);
    assert!(names_the_new_caller(&from_a), "{from_a}");

    turnstile.release(1);
    let from_b = b.recv();
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert!(names_the_new_caller(&from_b), "{from_b}");
    assert!(
        from_b["result"]["_meta"]
            .get("prism/auto_refresh")
            .is_none(),
        "B's query found the index current and did not refresh it: {from_b}"
    );
    assert!(
        from_b["result"]["_meta"]
            .get("prism/index_freshness")
            .is_none(),
        "{from_b}"
    );
    assert_eq!(status(&state).generation, 1, "one refresh, not two");

    close(a, a_worker);
    close(b, b_worker);
}

/// The re-check asks the question the read asked (is the index stale, with the refresh's
/// sticky evidence first), not whether another refresh has happened. A's refresh verifies
/// `Diverged`, so the index is still known stale, and B, once it has exclusive access,
/// refreshes again (spec §9 test 14, "auto-refresh retries"). Rules out a re-check that
/// skips the refresh because the generation moved.
#[test]
fn a_client_that_finds_the_index_still_known_stale_once_it_has_exclusive_access_refreshes_it() {
    let (dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let turnstile = Turnstile::new(2);
    let state = daemon_with_turnstile(&cfg, &turnstile);
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    state
        .write(|provider| {
            provider.force_next_verification_for_tests(RefreshVerification::Diverged(
                FreshnessReport::from_changed_paths(["main.py".to_string()]),
            ));
        })
        .expect("the provider lock is not poisoned")
        .expect("an index is published");
    add_a_caller(dir.path());
    two_stale_queries_waiting_for_exclusive_access(&mut a, &mut b, &turnstile);

    turnstile.release(0);
    let from_a = a.recv();
    assert_eq!(
        from_a["result"]["_meta"]["prism/auto_refresh"], "raced_stale",
        "{from_a}"
    );
    assert_eq!(status(&state).generation, 1);

    turnstile.release(1);
    let from_b = b.recv();
    assert_eq!(
        from_b["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "{from_b}"
    );
    assert_eq!(from_b["result"]["_meta"]["prism/refresh_generation"], 2);
    assert!(names_the_new_caller(&from_b), "{from_b}");
    assert_eq!(status(&state).generation, 2);

    close(a, a_worker);
    close(b, b_worker);
}
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
#[path = "connection_ready_tests.rs"]
mod ready_tests;
````

with this:

````rust
#[path = "connection_ready_tests.rs"]
mod ready_tests;

#[cfg(test)]
#[path = "connection_refresh_tests.rs"]
mod refresh_tests;
````

- [ ] **Step 2: Run them to see them fail**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t1-red.log 2>&1; grep -E '^error|^test result' /tmp/s3b-t1-red.log
```

Expected: one compile error, `error[E0609]: no field `exclusive` on type `TestHooks``. (Observed.)

- [ ] **Step 3: Add the test seam**

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// After an attempt's outcome is recorded, before the parked calls are notified.
    pub(super) settled: Option<AttemptHook>,
}

````

with this:

````rust
    /// After an attempt's outcome is recorded, before the parked calls are notified.
    pub(super) settled: Option<AttemptHook>,
    /// On a connection's thread, after a query found the index stale under an automatic
    /// refresh policy and before the call asks for exclusive access.
    pub(super) exclusive: Option<Arc<dyn Fn() + Send + Sync>>,
}

````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    }

    /// Wake every parked call so that it looks at the clock again.
    ///
````

with this:

````rust
    }

    /// The test seam between a call's read and its exclusive access (`TestHooks::exclusive`).
    /// Called with no lock held.
    #[cfg(test)]
    pub(super) fn before_exclusive(&self) {
        if let Some(hook) = &self.hooks.exclusive {
            hook();
        }
    }

    /// Wake every parked call so that it looks at the clock again.
    ///
````

- [ ] **Step 4: Ask the read's question again under exclusive access**

In `src/mcp/daemon/connection.rs`, replace this:

````rust
//! server's `warming` or `build_failed`, from the same `readiness_response`.
//!
//! What this slice leaves to S3b: under an automatic refresh policy the trigger is not
//! re-checked after exclusive access is acquired, so two clients that both find the index stale
//! each refresh it; a query that follows a refresh runs while the slot is still held
//! exclusively; and there is no panic boundary.
use super::budget::ConnectionBudget;
use super::coordinator::{Answer, DaemonState};
````

with this:

````rust
//! server's `warming` or `build_failed`, from the same `readiness_response`.
//!
//! Under an automatic refresh policy a query that finds the index stale asks for exclusive
//! access and checks the trigger again once it has it, so two clients that both found the
//! index stale refresh it once. The query that follows a refresh runs under the same exclusive
//! access, so its response and the refresh's generation are one snapshot's.
//!
//! What this slice leaves to S3b-2: the panic boundaries.
use super::budget::ConnectionBudget;
use super::coordinator::{Answer, DaemonState};
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
    // precedence over a fresh stamp probe exactly as it does in process.
    let read = state.read(|provider| dispatch_tool_read(call, provider, settings))?;
    let initial_report = match published(read)? {
        ReadDispatch::Response(response) => return Ok(response),
        ReadDispatch::NeedsAutoRefresh { initial_report } => initial_report,
    };
    // The read lock is released: `read` has returned. Exclusive access is taken afresh.
    let refreshed = state
        .write(|provider| dispatch_tool_auto_refresh(call, provider, initial_report, settings))?;
    published(refreshed)
}

````

with this:

````rust
    // precedence over a fresh stamp probe exactly as it does in process.
    let read = state.read(|provider| dispatch_tool_read(call, provider, settings))?;
    if let ReadDispatch::Response(response) = published(read)? {
        return Ok(response);
    }
    // The index was stale under an automatic policy. The read lock is released (`read` has
    // returned) and exclusive access is taken afresh, so another client may have refreshed in
    // between. The trigger is checked again under exclusive access (spec §4.2) by asking the
    // read's question again: a query that now finds the index current is answered as a read,
    // and one that still finds it stale refreshes from the report found under this access,
    // not the one found before it. Either way the response comes from the one generation this
    // access saw.
    #[cfg(test)]
    state.before_exclusive();
    let answered =
        state.write(
            |provider| match dispatch_tool_read(call, &*provider, settings) {
                ReadDispatch::Response(response) => response,
                ReadDispatch::NeedsAutoRefresh { initial_report } => {
                    dispatch_tool_auto_refresh(call, provider, initial_report, settings)
                }
            },
        )?;
    published(answered)
}

````

- [ ] **Step 5: Run the module**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t1.log 2>&1; grep -E '^test result|FAILED|^warning' /tmp/s3b-t1.log
```

Expected: `69 passed; 0 failed` (66 + 3), no warning. (Observed.) `a_refresh_by_one_client_is_what_the_next_query_of_another_client_sees` and `a_client_that_finds_the_index_still_known_stale_once_it_has_exclusive_access_refreshes_it` pass on the code before Step 4 too: the first is spec test 6, which S3a's single slot already satisfies, and the second guards the re-check against over-reaching. With the seam of Step 3 and without Step 4, the third failed in validation as it should, B's answer carrying `"prism/auto_refresh":"refreshed","prism/refresh_generation":2` and `"prism/stale_index_total_before_refresh":0`: a rebuild of an index nothing had changed since A refreshed it.

- [ ] **Step 6: Commit**

```bash
git add src/mcp/daemon/coordinator.rs src/mcp/daemon/connection.rs src/mcp/daemon/connection_refresh_tests.rs
git commit -m "feat(mcp): re-check the automatic refresh trigger under exclusive access"
```

- [ ] **Step 7: Prove the tests bite**

Each mutant is applied alone to the committed code, the module is run, and the file is restored with `git checkout -- <file>`.

| Mutant | Edit | Must fail (observed in validation) |
|---|---|---|
| A: refresh whatever the re-check finds (what the code before Step 4 does) | in `connection.rs`, replace `ReadDispatch::Response(response) => response,` with `ReadDispatch::Response(_) => dispatch_tool_auto_refresh(call, provider, None, settings),` | `a_client_that_finds_the_index_refreshed_once_it_has_exclusive_access_does_not_refresh_again` (1 of 69) |
| B: exclusive access does not copy the generation | in `coordinator.rs`, in `write`, delete the two lines `let generation = provider.generation();` and `self.record_generation(&slot, generation)?;` | `a_refresh_by_one_client_is_what_the_next_query_of_another_client_sees`, both other Task 1 tests, and five S3a tests (8 of 69) |

### Task 2: Many clients at once: a response write held, two queries inside dispatch together, one client not reading, another refreshing

Spec §9 tests 7 and 9 (D37). Three tests. The first two put threads exactly where the property is decided, with gates rather than socket buffers: a connection served through `serve_client` over a test transport whose response write is held at a gate while another client refreshes and queries; and a test tool whose handler returns only once two calls are inside it at once. The third is the smoke test under load, over real sockets, and needs two client helpers: one that writes without reading until its socket is full, and one that finishes, from a thread of its own, a frame that the full socket cut off (written from the reading thread, the rest of the frame would deadlock both ends: the server takes it only once it can write, which needs the client to read).

**Files:**
- Modify: `src/mcp/daemon/wire_support.rs`, `src/mcp/daemon/connection.rs`
- Create: `src/mcp/daemon/connection_concurrency_tests.rs`, `src/mcp/daemon/connection_isolation_tests.rs`

**Interfaces:**
- Consumes: `connection::serve_client(state, registry, transport: &mut impl Transport, client) -> anyhow::Result<()>` (private; the test module is a child of `connection`); `crate::mcp::transport::{serve_connection, InMemoryTransport, ReadOutcome, Transport}` (`pub(super)` / `#[cfg(test)] pub use` / `pub use`); `ToolRegistry::register`, `ToolDescriptor { name, description, input_schema, annotations, runtime_behavior, handler }`, `ToolAnnotations::read_only`, `ToolContext`, `McpToolResult { content_text, structured, is_error, meta }` (public); `ConnectionSettings { result_cap, structured_content, concise_shape }` (public fields); `ClientSettings::new(output, first_call_wait_secs, warm_at_startup) -> Result<Self, SettingsError>`; `SessionProvider::bootstrap`; `test_support::{Gate, HANG}`; `std::sync::{Barrier, Condvar}`.
- Produces (test code only): `Client::flood_until_full(&mut self, line: &str) -> (usize, Vec<u8>)`, `Client::send_raw_in_background(&self, bytes: Vec<u8>) -> JoinHandle<()>`, `wire_support::connect_with(state, registry: &Arc<ToolRegistry>, client) -> (Client, Worker)`.

- [ ] **Step 1: Write the client helpers and the tests**

Append to `src/mcp/daemon/wire_support.rs`:

````rust

impl Client {
    /// Send `line` again and again without reading anything, until the socket takes no more:
    /// the server has stopped reading this connection because it cannot write its answers.
    /// Returns how many requests went out whole and the unsent rest of the one that was cut
    /// off (empty if none was). The socket is blocking again when this returns.
    pub(super) fn flood_until_full(&mut self, line: &str) -> (usize, Vec<u8>) {
        let frame = format!("{line}\n").into_bytes();
        self.writer
            .set_nonblocking(true)
            .expect("make the client's socket non-blocking");
        let mut whole = 0usize;
        let rest = 'flood: loop {
            assert!(
                whole < 1_000_000,
                "the server kept reading a client that reads nothing"
            );
            let mut unsent: &[u8] = &frame;
            while !unsent.is_empty() {
                match self.writer.write(unsent) {
                    Ok(written) => unsent = &unsent[written..],
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        let cut = if unsent.len() == frame.len() {
                            Vec::new()
                        } else {
                            unsent.to_vec()
                        };
                        break 'flood cut;
                    }
                    Err(error) => panic!("write a frame: {error}"),
                }
            }
            whole += 1;
        };
        self.writer
            .set_nonblocking(false)
            .expect("make the client's socket blocking again");
        (whole, rest)
    }

    /// Send bytes that complete a frame `flood_until_full` cut off, from a thread of its own.
    /// The server takes them only once it can write again, which needs this client to read, so
    /// the caller reads while they go out and then joins the thread.
    pub(super) fn send_raw_in_background(&self, bytes: Vec<u8>) -> JoinHandle<()> {
        let mut writer = self.writer.try_clone().expect("clone the client socket");
        std::thread::spawn(move || writer.write_all(&bytes).expect("finish a frame"))
    }
}

/// Open a connection to `state` as a client with these settings, served with `registry`.
pub(super) fn connect_with(
    state: &Arc<DaemonState>,
    registry: &Arc<ToolRegistry>,
    client: ClientSettings,
) -> (Client, Worker) {
    let (near, far) = UnixStream::pair().expect("a connected socket pair");
    let state = Arc::clone(state);
    let registry = Arc::clone(registry);
    let worker = std::thread::spawn(move || serve_admitted(&state, &registry, far, &client));
    (Client::new(near), worker)
}
````

Create `src/mcp/daemon/connection_isolation_tests.rs`:

````rust
//! Spec §4.2, "Reads and writes happen outside every lock", shown without depending on how
//! large a socket's buffers are: one connection's response write is held at a gate while
//! another client refreshes and queries, and two queries are inside dispatch at one moment.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    client_settings, fixture, scripted, status, wait_until, Gate, HANG,
};
use super::super::wire_support::{
    close, connect, connect_with, CALLEES, CALLERS, INIT, INITED, REFRESH,
};
use super::serve_client;
use crate::mcp::output::McpToolResult;
use crate::mcp::registry::{ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry};
use crate::mcp::transport::{InMemoryTransport, ReadOutcome, Transport};
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// A transport whose second write (the first is the answer to `initialize`) stops at a gate:
/// the connection's thread is then inside `write_message` with a tool call's response in hand,
/// which is where a socket write to a client that reads nothing blocks.
struct HeldWrite {
    inner: InMemoryTransport,
    writes: usize,
    entered: Arc<AtomicBool>,
    release: Arc<Gate>,
}

impl Transport for HeldWrite {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        self.inner.read_message()
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        self.writes += 1;
        if self.writes == 2 {
            self.entered.store(true, Ordering::SeqCst);
            let _opened = self.release.pass();
        }
        self.inner.write_message(v)
    }
}

/// Spec §9 test 7's "one blocked writer", made deterministic: a connection is held inside the
/// write of a query's response, and meanwhile another client's refresh (which needs the
/// provider's write lock) and its next query complete. Rules out any daemon lock, the provider
/// slot's included, held across a response write, on any host, whatever its socket buffers.
#[test]
fn a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut other, other_worker) = connect(&state, client_settings(600));
    other.handshake();
    let first = other.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    let entered = Arc::new(AtomicBool::new(false));
    let release = Gate::closed();
    let held = {
        let state = Arc::clone(&state);
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        std::thread::spawn(move || {
            let mut transport = HeldWrite {
                inner: InMemoryTransport::new(vec![INIT, INITED, CALLEES]),
                writes: 0,
                entered,
                release,
            };
            let served = serve_client(
                &state,
                &ToolRegistry::all_v1(),
                &mut transport,
                &client_settings(600),
            );
            (served, transport.inner.responses().to_vec())
        })
    };
    wait_until(
        "the held connection to be inside its response write",
        || entered.load(Ordering::SeqCst),
    );

    let refreshed = other.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "refreshed",
        "the refresh completes while a response write is held: {refreshed}"
    );
    let after = other.request(CALLERS);
    assert_eq!(after["result"]["isError"], false, "{after}");
    assert_eq!(status(&state).generation, 1);

    release.open();
    let (served, responses) = held.join().expect("the held connection's thread ends");
    served.expect("the held connection ends cleanly at end of input");
    assert_eq!(responses.len(), 2, "{responses:?}");
    assert_eq!(responses[1]["result"]["isError"], false, "{responses:?}");
    close(other, other_worker);
    assert_eq!(builds.load(Ordering::SeqCst), 1);
}

const MEETING_TOOL: &str = "meet_another_reader";
const MEET: &str = r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"meet_another_reader","arguments":{}}}"#;

/// Counts the calls inside the meeting tool's handler.
#[derive(Default)]
struct Rendezvous {
    inside: Mutex<u64>,
    changed: Condvar,
}

/// Every tool, and one more whose handler returns only once two calls are inside it at the same
/// time (or reports that it was alone, after [`HANG`]). A handler runs under the provider
/// slot's read lock, so two calls inside it at once are two readers holding the lock at once.
fn registry_with_a_meeting_tool(rendezvous: &Arc<Rendezvous>) -> Arc<ToolRegistry> {
    let rendezvous = Arc::clone(rendezvous);
    let mut registry = ToolRegistry::all_v1();
    registry.register(ToolDescriptor {
        name: MEETING_TOOL,
        description: String::new(),
        input_schema: json!({ "type": "object" }),
        annotations: ToolAnnotations::read_only("meets another reader"),
        runtime_behavior: None,
        handler: Box::new(
            move |_context: &ToolContext<'_>, _arguments: &Value| -> McpToolResult {
                let mut inside = rendezvous.inside.lock().expect("rendezvous");
                *inside += 1;
                rendezvous.changed.notify_all();
                let (inside, _timed_out) = rendezvous
                    .changed
                    .wait_timeout_while(inside, HANG, |inside| *inside < 2)
                    .expect("rendezvous");
                let met = *inside >= 2;
                McpToolResult {
                    content_text: if met { "met" } else { "alone" }.to_string(),
                    structured: None,
                    is_error: !met,
                    meta: serde_json::Map::new(),
                }
            },
        ),
    });
    Arc::new(registry)
}

/// Concurrent dispatch (spec §12 row S3b): two clients' queries are inside dispatch at the same
/// moment. Each call of the meeting tool waits for the other inside its handler; both answer
/// `met` only if the two ran at once. Rules out queries served one at a time (a lock that
/// excludes readers from one another), which barrier-released clients alone do not.
#[test]
fn two_clients_queries_are_inside_dispatch_at_the_same_time() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let rendezvous = Arc::new(Rendezvous::default());
    let registry = registry_with_a_meeting_tool(&rendezvous);
    let (mut a, a_worker) = connect_with(&state, &registry, client_settings(600));
    let (mut b, b_worker) = connect_with(&state, &registry, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    a.send(MEET);
    b.send(MEET);
    let from_a = a.recv();
    let from_b = b.recv();
    for answer in [&from_a, &from_b] {
        assert_eq!(answer["result"]["isError"], false, "{answer}");
        assert_eq!(answer["result"]["content"][0]["text"], "met", "{answer}");
    }
    close(a, a_worker);
    close(b, b_worker);
}
````

Create `src/mcp/daemon/connection_concurrency_tests.rs`:

````rust
//! Spec §9 tests 7 and 9 as far as they can run before the accept loop and the shim exist:
//! many clients of one daemon at once, each with its own output settings, while one client
//! stops reading its answers and another refreshes the index.
use super::super::coordinator::DaemonState;
use super::super::test_support::{client_settings, fixture, scripted, status};
use super::super::wire_support::{close, connect, CALLEES, CALLERS, INIT, INITED, LIST, REFRESH};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::transport::{serve_connection, InMemoryTransport};
use crate::mcp::{RefreshPolicy, ServerConfig, SessionProvider, StartupMode};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Barrier};

/// How many clients query at once. The bridge's fan-out gate (spec §6.2) uses eight.
const CLIENTS: usize = 8;

/// Four combinations of the output settings a client chooses for itself (spec §5
/// "Connection-local"), one of them with a smaller result cap.
fn variants() -> [ConnectionSettings; 4] {
    [
        ConnectionSettings {
            result_cap: 80_000,
            structured_content: StructuredContentMode::Always,
            concise_shape: ConciseShapeMode::Slim,
        },
        ConnectionSettings {
            result_cap: 80_000,
            structured_content: StructuredContentMode::OmitDefaultPath,
            concise_shape: ConciseShapeMode::Slim,
        },
        ConnectionSettings {
            result_cap: 80_000,
            structured_content: StructuredContentMode::Always,
            concise_shape: ConciseShapeMode::Legacy,
        },
        ConnectionSettings {
            result_cap: 20_000,
            structured_content: StructuredContentMode::OmitDefaultPath,
            concise_shape: ConciseShapeMode::Legacy,
        },
    ]
}

fn with_id(line: &str, id: u64) -> String {
    let mut message: Value = serde_json::from_str(line).expect("a request constant parses");
    message["id"] = json!(id);
    message.to_string()
}

/// Client `client`'s requests: two queries and a listing, each with an id no other client
/// uses, so that a response delivered to the wrong client or out of order is seen.
fn requests(client: usize) -> Vec<String> {
    let base = 1_000 * (client as u64 + 1);
    vec![
        with_id(CALLEES, base + 1),
        with_id(CALLERS, base + 2),
        with_id(LIST, base + 3),
    ]
}

/// What the in-process server answers `requests` with, for a client with `settings`.
fn in_process_answers(
    cfg: &ServerConfig,
    settings: &ConnectionSettings,
    requests: &[String],
) -> Vec<Value> {
    let mut provider = SessionProvider::bootstrap(cfg).expect("an in-process index");
    let mut inputs = vec![INIT, INITED];
    inputs.extend(requests.iter().map(String::as_str));
    let mut transport = InMemoryTransport::new(inputs);
    serve_connection(
        &mut provider,
        &ToolRegistry::all_v1(),
        &mut transport,
        settings,
    )
    .expect("the in-process server ends at end of input");
    transport.responses()[1..].to_vec()
}

/// Spec §9 test 7, with test 9's settings folded in: eight clients, two of each of four output
/// settings, are released together by a barrier and pipeline three requests each with ids of
/// their own. Meanwhile one client has stopped reading, so its connection's thread is blocked
/// writing an answer, and another client refreshes the index. Every client gets every answer,
/// whole, in order, and byte for byte what the in-process server answers a client with its
/// settings (spec §5); the refresh completes. Rules out a lock held while an answer is written
/// (the refresh would then wait on the stalled client for ever) and output settings that are
/// not the connection's own.
#[test]
fn many_clients_get_their_own_answers_while_one_stops_reading_and_another_refreshes() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let all = variants();
    let expected: Vec<Vec<Value>> = (0..CLIENTS)
        .map(|client| in_process_answers(&cfg, &all[client % all.len()], &requests(client)))
        .collect();

    let (builder, builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut refresher, refresher_worker) = connect(&state, client_settings(600));
    refresher.handshake();
    let first = refresher.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    // A client that sends and never reads, until its connection's thread can write no more.
    let (mut stalled, stalled_worker) = connect(&state, client_settings(600));
    stalled.handshake();
    let (whole, cut) = stalled.flood_until_full(CALLEES);

    let barrier = Arc::new(Barrier::new(CLIENTS + 1));
    let clients: Vec<_> = (0..CLIENTS)
        .map(|client| {
            let settings = ClientSettings::new(all[client % all.len()], 600, false)
                .expect("settings within the accepted bounds");
            let (mut conn, worker) = connect(&state, settings);
            conn.handshake();
            let barrier = Arc::clone(&barrier);
            let requests = requests(client);
            std::thread::spawn(move || {
                barrier.wait();
                for request in &requests {
                    conn.send(request);
                }
                let answers: Vec<Value> = requests.iter().map(|_| conn.recv()).collect();
                close(conn, worker);
                answers
            })
        })
        .collect();

    barrier.wait();
    let refreshed = refresher.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "refreshed",
        "the refresh completes while a client is not reading: {refreshed}"
    );

    for (client, handle) in clients.into_iter().enumerate() {
        let answers = handle.join().expect("a client thread ends without a panic");
        assert_eq!(answers.len(), expected[client].len(), "client {client}");
        for (index, (got, want)) in answers.iter().zip(&expected[client]).enumerate() {
            assert_eq!(got, want, "client {client}, answer {}", index + 1);
        }
    }

    // The stalled client reads at last: every request it sent was answered, whole. A request
    // the full socket cut off is finished while the answers are read.
    let sent = whole + usize::from(!cut.is_empty());
    let finishing = stalled.send_raw_in_background(cut);
    for index in 0..sent {
        let answer = stalled.recv();
        assert_eq!(answer["id"], 7, "answer {}: {answer}", index + 1);
        assert_eq!(answer["result"]["isError"], false, "answer {}", index + 1);
    }
    finishing.join().expect("the cut-off request is sent");
    close(stalled, stalled_worker);
    close(refresher, refresher_worker);

    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "one build for every client"
    );
    assert_eq!(status(&state).generation, 1);
}
````

Append to `src/mcp/daemon/connection.rs`:

````rust

#[cfg(test)]
#[path = "connection_concurrency_tests.rs"]
mod concurrency_tests;

#[cfg(test)]
#[path = "connection_isolation_tests.rs"]
mod isolation_tests;
````

- [ ] **Step 2: Run the module**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t2.log 2>&1; grep -E '^test result|FAILED|^warning' /tmp/s3b-t2.log
```

Expected: `72 passed; 0 failed` (69 + 3), no warning. (Observed.) The tests pass on Task 1's code as it stands: they prove properties S3a's construction already has (an owned response, a shared read lock, a connection's own settings) against the ways of losing them below. On macOS the smoke test's stalled client sent 58–63 whole frames before its socket was full; with larger buffers it sends more (the loop allows up to 1,000,000), and whether its connection's thread is inside `write` when the refresh runs is not determined, which is why the held-write test exists.

- [ ] **Step 3: Commit**

```bash
git add src/mcp/daemon/wire_support.rs src/mcp/daemon/connection.rs src/mcp/daemon/connection_concurrency_tests.rs src/mcp/daemon/connection_isolation_tests.rs
git commit -m "test(mcp): many clients, a held response write, overlapping reads, a stalled reader and a refresh"
```

- [ ] **Step 4: Prove the tests bite**

Mutant C makes every S3a test that takes the provider's write lock from the test thread wait for ever (a connection's thread holds the read guard between frames), so it is run with the filter `mcp::daemon::connection::isolation_tests`; the others run on the whole module. To run C at Linux-sized buffers, also call, on both ends of each socket pair in `wire_support::connect` and `connect_with`, a helper that sets `SO_SNDBUF` and `SO_RCVBUF` to 212,992 with `libc::setsockopt` and asserts it returned 0 (a probe; not part of the plan's code).

| Mutant | Edit | Must fail (observed in validation) |
|---|---|---|
| C: the provider slot's read guard held across the response write | in `coordinator.rs`, add `pub(super) fn pin_for_mutant(&self) -> std::sync::RwLockReadGuard<'_, Option<SessionProvider>> { self.provider.read().unwrap_or_else(\|p\| p.into_inner()) }`; in `serve_client`, declare `let mut held: Option<std::sync::RwLockReadGuard<'_, Option<crate::mcp::SessionProvider>>> = None;` before `serve_loop`, make `held = None;` the closure's first statement, and in the `Control::ToolCall` arm keep `run_tool_call`'s result, then `held = Some(state.pin_for_mutant());` before returning it (the guard taken after a call is let go only when the next frame arrives, so it is held while the response is written) | `a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held`, at the 30 s read limit (1 of 2 with the filter), at Task 2's code and on the code after Task 6, with default buffers and with 212,992-byte buffers alike. The smoke test, under C with 212,992-byte buffers, failed in the one validation run and passed in three of three runs of review round 1: its verdict depends on scheduling |
| D: queries served one at a time | in `coordinator.rs`, in `read`, replace `.read()` on `self.provider` with `.write()` | `two_clients_queries_are_inside_dispatch_at_the_same_time` (each handler waits 30 s alone, then reports `alone`) |
| A: a global lock held across the response write | in `connection.rs`, `serve_client`, replace `    serve_loop(transport, \|message, lifecycle\| {` and the `match handle_control` line after it with a `static SERIAL: std::sync::Mutex<()>`, a `let mut held: Option<std::sync::MutexGuard<'static, ()>> = None;`, and, as the closure's first two statements, `held = None; held = Some(SERIAL.lock().unwrap_or_else(\|p\| p.into_inner()));` | both new tests and the smoke test, among 28 of 83 on the code after Task 6 (the static also serializes the module's other socket tests) |
| B: output settings from the daemon's environment | in `serve_client`, replace `let settings: ConnectionSettings = *client.output();` with `let settings: ConnectionSettings = ConnectionSettings::from_env();` | the smoke test, the held-write test and 16 others (18 of 72 at Task 2's code; 18 of 83 after Task 6) |

### Task 3: S3b-1 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces: the S3b-1 pull request.

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
`Superseded` call gets the daemon's own retryable `indexing` status (retry in
`INDEXING_RETRY_AFTER_SECS`; ignore it if a retry already succeeded), which the in-process
server never sends. Not there yet (the next slice): the re-check of an automatic refresh after
exclusive access is acquired, and panic boundaries.

Algorithms fall into two categories:
````

with this:

````markdown
`Superseded` call gets the daemon's own retryable `indexing` status (retry in
`INDEXING_RETRY_AFTER_SECS`; ignore it if a retry already succeeded), which the in-process
server never sends. Under an automatic refresh policy a query that found the index stale asks
the read's question again once it holds the write lock: one that now finds the index current
(another client refreshed in between) is answered as a read, so clients that race on one stale
index refresh it once, and one that still finds it stale (sticky evidence included) refreshes.
The query after a refresh runs under the same write lock, so a response and the refresh
generation it reports are one snapshot's. No lock is held while a response is written: a client
that stops reading stalls only its own connection. Not there yet (the next slice): panic
boundaries.

Algorithms fall into two categories:
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check
cargo test --features mcp --lib mcp:: > /tmp/s3b1-lib.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3b1-lib.log
env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > /tmp/s3b1-mcp.log 2>&1; grep -E '^test result|FAILED|differs|golden' /tmp/s3b1-mcp.log
cargo check > /tmp/s3b1-default.log 2>&1; tail -1 /tmp/s3b1-default.log
cargo check --features mcp > /tmp/s3b1-feature.log 2>&1; grep -c '^warning' /tmp/s3b1-feature.log
cargo check --tests --features mcp,detached-owner-audit > /tmp/s3b1-owner.log 2>&1; tail -1 /tmp/s3b1-owner.log
cargo clippy --all-targets --features mcp -- -W clippy::all > /tmp/s3b1-clippy.log 2>&1
grep -E -A8 '^warning|^error' /tmp/s3b1-clippy.log | grep -E 'src/mcp/daemon/' | sort -u
```

Expected: `cargo fmt --check` prints nothing; the lib line shows `458 passed; 0 failed` (452 + 6); the `mcp` target `23 passed` with the golden transcripts' test `ok`; both `cargo check` runs end with `Finished` and the feature build has no warning; the last command prints nothing. (Observed on the S3b-1 head, `scratch/s3b-tasks3` `c4c57cdd`: `458 passed; 0 failed`. Clippy there: no line names `src/mcp/daemon/`. The `mcp` target and the three `cargo check` builds were run on the S3b-2 head only (Task 7), where each was as stated; `transport.rs` and the goldens are untouched by both parts.)

- [ ] **Step 3: State the invariants**

```bash
BASE=$(git merge-base origin/main HEAD)
git diff --stat "$BASE" -- src/mcp/transport.rs src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs tests/fixtures/mcp/golden | wc -l
git diff --name-only "$BASE" | grep -v -E '^src/mcp/daemon/|^CLAUDE\.md$' | wc -l
git diff --stat "$BASE" | tail -1
wc -l src/mcp/daemon/*.rs | sort -n | tail -3
```

Expected: `0`; `0`; 7 files changed (six under `src/mcp/daemon/` and `CLAUDE.md`), 702 insertions and 14 deletions; no file over 600 lines.

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
cargo test --features mcp > /tmp/s3b1-full.log 2>&1; grep -E '^test result|FAILED' /tmp/s3b1-full.log
cargo test --doc --features mcp > /tmp/s3b1-doc.log 2>&1; grep -E '^test result' /tmp/s3b1-doc.log
for i in $(seq 1 30); do cargo test --features mcp --lib mcp::daemon > /tmp/s3b1-repeat-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3b1-repeat-$i.log; done | sort | uniq -c
for i in $(seq 1 30); do cargo test --features mcp --lib mcp::daemon -- --test-threads=1 > /tmp/s3b1-repeat1-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3b1-repeat1-$i.log; done | sort | uniq -c
python3 scripts/mutgate/mutgate.py > /tmp/s3b1-mutgate.log 2>&1; tail -5 /tmp/s3b1-mutgate.log
```

Expected: every `test result` line shows `0 failed` (sum the `passed` counts and report the total); the doc tests pass (none added); each repeat prints one line, `30 test result: ok. 72 passed; 0 failed`; the mutation gate exits 0, reported as "full gate green; it builds without `--features mcp`, so it selects no code from this slice". A failure outside this slice's files is reported, not fixed or re-baselined here.

- [ ] **Step 5: Commit and open the PR**

Write the PR body to `/tmp/s3b1-pr-body.md`. It states, in this order:

1. **Goal:** the lane goal, and that S3b-1 makes refresh visible to every client, refreshes once when clients race on a stale index, and proves concurrent service with a stalled reader (spec §9 tests 6, 7, 9).
2. **Invariants:** the Step 3 output; the golden transcripts pass unblessed.
3. **Decisions:** D34–D38 and SI-9, SI-11, verbatim or by link. D35 reverses an item of S3a's D14 and says why.
4. **Claims:** the S3b-1 rows of "Claims and the tests that hold them", with what no test holds.
5. **Totals:** Steps 2 and 4 next to the base (V1), and the mutants of Tasks 1 and 2 as observed.
6. **Not verified:** Linux; tests 6, 7 and 9 through real shims (S5a).
7. **Tier-A harness:** not run; this slice touches none of `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`.

Then:

```bash
git add CLAUDE.md
git commit -m "docs(mcp): the shared daemon's refresh re-check and concurrent service"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): concurrent dispatch and refresh visibility (shared daemon S3b-1)" --body-file /tmp/s3b1-pr-body.md
```

Review brief (lane plan): Opus 5.5 and gpt-6.1-sol, same commit, WRONG/SMELL and MATERIAL/IMMATERIAL tags against the lane goal, a recommended fix plus alternatives per finding, and when each finding would not apply. Cap: two rounds.

---

# Part two: S3b-2, panic boundaries (Tasks 4–7)

Based on S3b-1 as merged. If S3b-1 changed in review, re-run Task 4's base count first and add the difference to every count below.

### Task 4: What ends the daemon

`fatal.rs`: the domains, the fatal event, the injected handler, the production abort and the guard (D39, D40).

**Files:**
- Create: `src/mcp/daemon/fatal.rs`, `src/mcp/daemon/fatal_tests.rs`
- Modify: `src/mcp/daemon/mod.rs`

**Interfaces:**
- Consumes: nothing of this crate.
- Produces (in `crate::mcp::daemon::fatal`, all `pub(super)`): `enum Domain { Dispatch, Build, Control }` (`Display`); `enum Fatal { Panicked(Domain), Poisoned { lock: &'static str } }` (`Clone, Copy, Debug, PartialEq, Eq, Display`); `type OnFatal = Arc<dyn Fn(Fatal) + Send + Sync>`; `fn abort_daemon(fatal: Fatal)`; `struct FatalOnUnwind<'a>` with `fn new(on_fatal: &'a OnFatal, domain: Domain) -> Self` and a `Drop` that calls `on_fatal(Fatal::Panicked(domain))` when the thread unwinds through it and was not already unwinding when it was made.

- [ ] **Step 1: Write the tests**

Create `src/mcp/daemon/fatal_tests.rs`:

````rust
//! The guard that turns a panic in a domain into the daemon's end.
use super::*;
use std::sync::Mutex;

fn recorder() -> (OnFatal, Arc<Mutex<Vec<Fatal>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let on_fatal: OnFatal = Arc::new(move |fatal| sink.lock().expect("log").push(fatal));
    (on_fatal, log)
}

fn events(log: &Mutex<Vec<Fatal>>) -> Vec<Fatal> {
    log.lock().expect("log").clone()
}

#[test]
fn code_that_returns_is_not_fatal() {
    let (on_fatal, log) = recorder();
    {
        let _fatal = FatalOnUnwind::new(&on_fatal, Domain::Dispatch);
    }
    assert_eq!(events(&log), vec![]);
}

#[test]
fn a_panic_while_the_guard_is_alive_is_fatal_in_its_domain() {
    let (on_fatal, log) = recorder();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _fatal = FatalOnUnwind::new(&on_fatal, Domain::Control);
        panic!("injected panic in the control plane");
    }));
    assert!(unwound.is_err());
    assert_eq!(events(&log), vec![Fatal::Panicked(Domain::Control)]);
}

/// A guard made in a destructor that runs during another panic does not claim that panic.
/// Rules out a guard that looks only at `thread::panicking()` when it is dropped.
#[test]
fn a_guard_made_while_the_thread_is_already_unwinding_does_not_fire() {
    struct MakesAGuardWhenDropped {
        on_fatal: OnFatal,
    }
    impl Drop for MakesAGuardWhenDropped {
        fn drop(&mut self) {
            let _fatal = FatalOnUnwind::new(&self.on_fatal, Domain::Build);
        }
    }
    let (on_fatal, log) = recorder();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _cleanup = MakesAGuardWhenDropped { on_fatal };
        panic!("a panic outside every domain");
    }));
    assert!(unwound.is_err());
    assert_eq!(events(&log), vec![]);
}

#[test]
fn the_daemon_s_last_line_names_where_it_failed() {
    assert_eq!(
        Fatal::Panicked(Domain::Dispatch).to_string(),
        "prism-mcp daemon: a panic in provider dispatch; the daemon ends so that no client is \
         served from what the panic left"
    );
    assert_eq!(
        Fatal::Poisoned { lock: "control" }.to_string(),
        "prism-mcp daemon: the control lock is poisoned; the daemon ends so that no client is \
         served from what the panic left"
    );
}
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod control;
mod coordinator;
#[cfg(test)]
mod test_support;
````

with this:

````rust
mod control;
mod coordinator;
mod fatal;
#[cfg(test)]
mod test_support;
````

- [ ] **Step 2: Run them to see them fail**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t4-red.log 2>&1; grep -E '^error' /tmp/s3b-t4-red.log | sort | uniq -c
```

Expected: one compile error, `error[E0583]: file not found for module `fatal``. (Observed.)

- [ ] **Step 3: Write `fatal.rs`**

Create `src/mcp/daemon/fatal.rs`:

````rust
//! What ends a shared daemon (spec §3.1 "Fatal", §4.2 "Panic policy", §10).
//!
//! A panic in provider dispatch (a query or a refresh, run under the provider slot's lock), on
//! the build thread, or in the control plane (under the control lock) ends the daemon, and so
//! does a lock found poisoned. The process aborts: every attached client sees its connection
//! end, as a client of a crashed in-process server does, and the next client starts a new
//! daemon. Nothing is served from state a panic may have left half-changed, and no lock is read
//! through past its poisoning.
//!
//! A panic anywhere else on a connection's thread (framing, the readiness answers, writing a
//! response) ends that connection only; that boundary is in `connection.rs`.
use std::fmt;
use std::io::Write;
use std::sync::Arc;

/// Where a panic that ends the daemon was raised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Domain {
    /// A query or a refresh, under the provider slot's read or write lock.
    Dispatch,
    /// The build thread: the builder, then publication.
    Build,
    /// The readiness coordinator, under the control lock.
    Control,
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dispatch => "provider dispatch",
            Self::Build => "the index build",
            Self::Control => "the control plane",
        })
    }
}

/// Why the daemon ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fatal {
    /// A thread panicked in this domain.
    Panicked(Domain),
    /// Code that needed this lock found it poisoned: a thread panicked while holding it.
    Poisoned { lock: &'static str },
}

impl fmt::Display for Fatal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panicked(domain) => write!(f, "prism-mcp daemon: a panic in {domain}"),
            Self::Poisoned { lock } => {
                write!(f, "prism-mcp daemon: the {lock} lock is poisoned")
            }
        }?;
        f.write_str("; the daemon ends so that no client is served from what the panic left")
    }
}

/// What the daemon does when it cannot go on. Production: [`abort_daemon`]. A test replaces it
/// with a record and then sees the code after the call run, which a running daemon never does.
pub(super) type OnFatal = Arc<dyn Fn(Fatal) + Send + Sync>;

/// One line on standard error (the daemon's log; the panic message, if there was one, is the
/// line before it), then abort.
pub(super) fn abort_daemon(fatal: Fatal) {
    // A failed write must not panic here: this runs while a thread is unwinding.
    let _ = writeln!(std::io::stderr(), "{fatal}");
    std::process::abort();
}

/// Calls `on_fatal` with `Fatal::Panicked(domain)` if the thread unwinds while this is alive.
///
/// Made at the entry of each piece of code in a domain and dropped at its exit. It does nothing
/// if the thread was already unwinding when it was made (code that runs in a destructor during
/// another panic belongs to that panic, which has its own boundary).
pub(super) struct FatalOnUnwind<'a> {
    on_fatal: &'a OnFatal,
    domain: Domain,
    armed: bool,
}

impl<'a> FatalOnUnwind<'a> {
    pub(super) fn new(on_fatal: &'a OnFatal, domain: Domain) -> Self {
        Self {
            on_fatal,
            domain,
            armed: !std::thread::panicking(),
        }
    }
}

impl Drop for FatalOnUnwind<'_> {
    fn drop(&mut self) {
        if self.armed && std::thread::panicking() {
            (self.on_fatal)(Fatal::Panicked(self.domain));
        }
    }
}

#[cfg(test)]
#[path = "fatal_tests.rs"]
mod tests;
````

- [ ] **Step 4: Run the module**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t4.log 2>&1; grep -E '^test result|FAILED|^warning' /tmp/s3b-t4.log
```

Expected: `76 passed; 0 failed` (72 + 4), and one warning, `function `abort_daemon` is never used`: nothing calls it until Task 5 makes it the production handler. (Observed, both.) The default build has no warning: the module's `cfg_attr(not(test), allow(dead_code))` covers it.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/daemon/fatal.rs src/mcp/daemon/fatal_tests.rs src/mcp/daemon/mod.rs
git commit -m "feat(mcp): what ends the shared daemon, and the guard that marks a domain"
```

- [ ] **Step 6: Prove the tests bite**

| Mutant | Edit | Must fail (observed in validation) |
|---|---|---|
| A: the guard is armed whatever the thread is doing | in `fatal.rs`, replace `armed: !std::thread::panicking(),` with `armed: true,` | `a_guard_made_while_the_thread_is_already_unwinding_does_not_fire` (1 of 83, on the code after Task 6) |

### Task 5: The coordinator's domains

`Parts::on_fatal`, a guard at each entry point (D40, D46), `poisoned` (D41), the fatal calls in `publish`, `settle` and `PanicGuard` (D42), and the four S3a tests that reach a fatal event, which now install a `FatalLog` and expect its events. Their S3a expectations stay, as what happens after a test's handler returns.

**Files:**
- Modify: `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/test_support.rs`, `src/mcp/daemon/coordinator_failure_tests.rs`, `src/mcp/daemon/coordinator_order_tests.rs`, `src/mcp/daemon/connection_parity_tests.rs`

**Interfaces:**
- Consumes: Task 4.
- Produces: `Parts::on_fatal: OnFatal` (production `Arc::new(abort_daemon)`); private `DaemonState::{guard, poisoned}`; `test_support::{FatalLog { install(&mut Parts) -> FatalLog, events() -> Vec<Fatal> }, recording_state(builder) -> (Arc<DaemonState>, FatalLog)}`. `read`, `write`, `ensure_ready` and `status` keep their signatures; every `Poisoned` they return has first been reported to `on_fatal`.

Also one new test (D47), `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held`: each guard is made right after its lock is taken, so `on_fatal` runs while the lock is still held; the test's handler checks that another thread's attempt to take the lock would block. And one stale comment in `publish` is corrected (review round 1, Opus F4).

- [ ] **Step 1: Write the test support, the new test, and change the four tests**

In `src/mcp/daemon/test_support.rs`, replace this:

````rust
use super::clock::Clock;
use super::coordinator::{Answer, BuildStatus, DaemonState, Parts, Poisoned};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::lazy::SessionBuilder;
````

with this:

````rust
use super::clock::Clock;
use super::coordinator::{Answer, BuildStatus, DaemonState, Parts, Poisoned};
use super::fatal::Fatal;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::lazy::SessionBuilder;
````

In `src/mcp/daemon/test_support.rs`, replace this:

````rust
            .expect("the call returns within the hang limit")
    }
}
````

with this:

````rust
            .expect("the call returns within the hang limit")
    }
}

/// What would have ended the daemon, in order. A test installs it in place of the abort to see
/// where a panic or a poisoned lock is caught; the code after the call then runs, which a
/// running daemon never does.
#[derive(Clone, Default)]
pub(super) struct FatalLog(Arc<Mutex<Vec<Fatal>>>);

impl FatalLog {
    pub(super) fn install(parts: &mut Parts) -> Self {
        let log = Self::default();
        let sink = log.clone();
        parts.on_fatal = Arc::new(move |fatal| sink.0.lock().expect("fatal log").push(fatal));
        log
    }

    pub(super) fn events(&self) -> Vec<Fatal> {
        self.0.lock().expect("fatal log").clone()
    }
}

/// A lazy daemon on the system clock that records what would end it instead of aborting.
pub(super) fn recording_state(builder: SessionBuilder) -> (Arc<DaemonState>, FatalLog) {
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    (state, log)
}
````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
//! Tests for the ways an attempt can end without a build: the thread cannot be started, the
//! builder panics, a lock is poisoned. In each, every parked call must get an answer.
use super::super::test_support::{
    build_after, fail_now, fixture, panic_now, parts_with, scripted, state_with, status,
    wait_until, Conn, Gate, ManualClock, Outcome,
};
use super::*;
````

with this:

````rust
//! Tests for the ways an attempt can end without a build: the thread cannot be started, the
//! builder panics, a lock is poisoned. In each, every parked call must get an answer.
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    build_after, fail_now, fixture, panic_now, parts_with, scripted, state_with, status,
    wait_until, Conn, FatalLog, Gate, ManualClock, Outcome,
};
use super::*;
````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
}

/// S3a's interim behavior (slice S3b turns a builder panic into the daemon-fatal abort of spec
/// §3.1). Rules out a panic that leaves the phase `Building` forever.
#[test]
fn a_builder_panic_is_a_failed_attempt_every_waiter_hears_of() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![panic_now()]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

````

with this:

````rust
}

/// Spec §3.1: a panic on the build thread ends the daemon. Under a recording `on_fatal`, which
/// returns where the abort would not, the attempt still ends: rules out a panic that leaves the
/// phase `Building` forever in a test, and a builder panic reported as a failed build only.
#[test]
fn a_builder_panic_is_fatal() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![panic_now()]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
        Outcome::Failed("index build panicked".to_string())
    );
    assert_eq!(status(&state).state, BuildState::Failed);
    assert_eq!(status(&state).waiting, 0);
````

with this:

````rust
        Outcome::Failed("index build panicked".to_string())
    );
    assert_eq!(log.events(), vec![Fatal::Panicked(Domain::Build)]);
    assert_eq!(status(&state).state, BuildState::Failed);
    assert_eq!(status(&state).waiting, 0);
````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let parked = Conn::open(&state, 600);

````

with this:

````rust
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let parked = Conn::open(&state, 600);

````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
    state.wake();

    assert_eq!(parked.outcome(), Outcome::Poisoned, "a parked call returns");
    assert_eq!(state.status(), Err(Poisoned { lock: "control" }));
    let later = Conn::open(&state, 600);
    later.call();
    assert_eq!(later.outcome(), Outcome::Poisoned, "and so does a new one");

    // The build thread still ends: it publishes, finds the control lock poisoned and stops.
    gate.open();
    wait_until("the build thread to publish", || {
        state.read(|_| ()) == Ok(Some(()))
    });
}

````

with this:

````rust
    state.wake();

    // Spec §4.2: each find of the poisoned lock ends the daemon, one at a time; nothing is
    // read through it.
    let poisoned = Fatal::Poisoned { lock: "control" };
    assert_eq!(parked.outcome(), Outcome::Poisoned, "a parked call returns");
    assert_eq!(log.events(), vec![poisoned; 1]);
    assert_eq!(state.status(), Err(Poisoned { lock: "control" }));
    assert_eq!(log.events(), vec![poisoned; 2]);
    let later = Conn::open(&state, 600);
    later.call();
    assert_eq!(later.outcome(), Outcome::Poisoned, "and so does a new one");
    assert_eq!(log.events(), vec![poisoned; 3]);

    // The build thread still ends: it publishes, finds the control lock poisoned when it
    // records the generation and again when it records the outcome, and stops.
    gate.open();
    wait_until("the build thread to publish", || {
        state.read(|_| ()) == Ok(Some(()))
    });
    wait_until("the build thread to find the lock poisoned twice", || {
        log.events().len() == 5
    });
    assert_eq!(log.events(), vec![poisoned; 5]);
}

````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
//! Tests for the two steps of the build thread at which another actor can interleave, each
//! played through a test hook, and for exclusive access to the published provider.
use super::super::test_support::{
    build_after, fail_after, fixture, parts_with, scripted, state_with, status, wait_until, Conn,
    Gate, ManualClock, Outcome,
};
use super::*;
````

with this:

````rust
//! Tests for the two steps of the build thread at which another actor can interleave, each
//! played through a test hook, and for exclusive access to the published provider.
use super::super::fatal::Fatal;
use super::super::test_support::{
    build_after, fail_after, fixture, parts_with, scripted, state_with, status, wait_until, Conn,
    FatalLog, Gate, ManualClock, Outcome,
};
use super::*;
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

````

with this:

````rust
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
    assert_eq!(state.read(|_| ()), Err(Poisoned { lock: "provider" }));
    assert_eq!(state.write(|_| ()), Err(Poisoned { lock: "provider" }));
}

````

with this:

````rust
    assert_eq!(state.read(|_| ()), Err(Poisoned { lock: "provider" }));
    assert_eq!(state.write(|_| ()), Err(Poisoned { lock: "provider" }));
    // Spec §4.2: publication, the read and the write each found the slot poisoned and each
    // ended the daemon.
    assert_eq!(log.events(), vec![Fatal::Poisoned { lock: "provider" }; 3]);
}

````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));
}
````

with this:

````rust
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));
}

/// A clock that panics once it is armed. The coordinator reads it under the control lock.
struct ArmedClock {
    armed: std::sync::atomic::AtomicBool,
}

impl Clock for ArmedClock {
    fn now(&self) -> Instant {
        assert!(
            !self.armed.load(Ordering::SeqCst),
            "injected panic under the control lock"
        );
        Instant::now()
    }
}

/// What a test's `on_fatal` saw: the event, and whether the lock the panic happened under was
/// still held at that moment (another thread's attempt to take it would block, rather than
/// succeed or find it poisoned).
type Seen = Arc<Mutex<Vec<(Fatal, bool)>>>;

/// A daemon whose `on_fatal` records, for each event, whether the provider slot (for a panic in
/// dispatch) or the control lock (for a panic in the control plane) was still held.
fn watching_lock_state(mut parts: Parts) -> (Arc<DaemonState>, Seen) {
    let seen: Seen = Arc::default();
    let target: Arc<std::sync::OnceLock<std::sync::Weak<DaemonState>>> = Arc::default();
    let sink = Arc::clone(&seen);
    let watched = Arc::clone(&target);
    parts.on_fatal = Arc::new(move |fatal| {
        let held = watched
            .get()
            .and_then(std::sync::Weak::upgrade)
            .is_some_and(|state| match fatal {
                Fatal::Panicked(super::super::fatal::Domain::Control) => matches!(
                    state.control.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ),
                _ => matches!(
                    state.provider.try_write(),
                    Err(std::sync::TryLockError::WouldBlock)
                ),
            });
        sink.lock().expect("seen").push((fatal, held));
    });
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let _ = target.set(Arc::downgrade(&state));
    (state, seen)
}

/// Spec §3.1: a panic under one of the daemon's locks ends the daemon while that lock is still
/// held, so that in production no other call takes it between the panic and the abort (a
/// refresh waiting on a reader that panicked would otherwise start first). Rules out a guard
/// made before its lock, which, dropped last, fires only after the lock is let go.
#[test]
fn a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held() {
    use super::super::fatal::Domain;

    // Under the provider slot's read lock, then under its write lock.
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let (state, seen) = watching_lock_state(parts_with(builder, &clock));
    let conn = Conn::open(&state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    for exclusive in [false, true] {
        let panicking = Arc::clone(&state);
        let unwound = std::thread::spawn(move || {
            if exclusive {
                let _ =
                    panicking.write(|_| -> () { panic!("injected panic under the write lock") });
            } else {
                let _ = panicking.read(|_| -> () { panic!("injected panic under the read lock") });
            }
        })
        .join();
        assert!(unwound.is_err());
    }
    assert_eq!(
        *seen.lock().expect("seen"),
        vec![
            (Fatal::Panicked(Domain::Dispatch), true),
            (Fatal::Panicked(Domain::Dispatch), true)
        ]
    );

    // Under the control lock: the clock is read there when an attempt starts.
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = Arc::new(ArmedClock {
        armed: std::sync::atomic::AtomicBool::new(true),
    });
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    parts.clock = clock;
    let (state, seen) = watching_lock_state(parts);
    let panicking = Arc::clone(&state);
    let unwound = std::thread::spawn(move || {
        let mut budget = ConnectionBudget::new(&super::super::test_support::client_settings(600));
        let _ = panicking.ensure_ready(&mut budget);
    })
    .join();
    assert!(unwound.is_err());
    assert_eq!(
        *seen.lock().expect("seen"),
        vec![(Fatal::Panicked(Domain::Control), true)]
    );
}
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
//! test 14), and that a broken daemon ends a connection with an error and not a panic.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    build_after, client_settings, fail_after, fixture, scripted, status, wait_until, Gate,
};
use super::super::wire_support::{
````

with this:

````rust
//! test 14), and that a broken daemon ends a connection with an error and not a panic.
use super::super::coordinator::DaemonState;
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    build_after, client_settings, fail_after, fixture, recording_state, scripted, status,
    wait_until, Gate,
};
use super::super::wire_support::{
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
}

/// In S3a a panic under the write lock poisons the slot (slice S3b makes it fatal). Until
/// then a client's call must end its connection with an error, and the control path, which
/// takes no lock, must keep answering.
#[test]
fn a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
````

with this:

````rust
}

/// A panic under the write lock ends the daemon (spec §3.1), and so does every later find of
/// the slot it poisoned. Under a recording `on_fatal`, which returns where the abort would
/// not: a client's call ends its connection with an error, and the control path, which takes
/// no lock, keeps answering.
#[test]
fn a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let (state, log) = recording_state(builder);
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    close(b, b_worker);
}

````

with this:

````rust
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    close(b, b_worker);
    assert_eq!(
        log.events(),
        vec![
            Fatal::Panicked(Domain::Dispatch),
            Fatal::Poisoned { lock: "provider" }
        ]
    );
}

````

- [ ] **Step 2: Run them to see them fail**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t5-red.log 2>&1; grep -E '^error' /tmp/s3b-t5-red.log | sort | uniq -c
```

Expected: two compile errors, `error[E0609]: no field `on_fatal` on type `&mut coordinator::Parts`` (from `FatalLog::install`) and `error[E0609]: no field `on_fatal` on type `coordinator::Parts`` (from the new test). (Observed.) With the guards first made before their locks, the new test fails with `left: [(Panicked(Dispatch), false), (Panicked(Dispatch), false)]`: the lock had been let go when `on_fatal` ran (observed in validation, on the code as it stood before review round 1).

- [ ] **Step 3: Mark the coordinator's domains**

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
//! holds the slot may take the control lock.
//!
//! Not here yet: a panic boundary. A poisoned lock is reported to the caller as [`Poisoned`]
//! and nothing is read through it; a panic in the builder is recorded as a failed attempt.
//! Turning both into the daemon-fatal abort of spec §3.1 is slice S3b.
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use super::control::answer_for;
pub(super) use super::control::{Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use crate::mcp::lazy::SessionBuilder;
// The tests in this file's child modules name `BuildState` through `use super::*`.
````

with this:

````rust
//! holds the slot may take the control lock.
//!
//! Panics (spec §3.1 "Fatal", §4.2 "Panic policy"): a panic in a query or a refresh run through
//! [`DaemonState::read`] or [`DaemonState::write`], on the build thread, or in the control plane
//! ends the daemon through `on_fatal`, and so does a lock found poisoned (`fatal.rs`). Nothing
//! is read through a poisoned lock.
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use super::control::answer_for;
pub(super) use super::control::{Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use super::fatal::{abort_daemon, Domain, Fatal, FatalOnUnwind, OnFatal};
use crate::mcp::lazy::SessionBuilder;
// The tests in this file's child modules name `BuildState` through `use super::*`.
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust

/// A lock of the daemon is poisoned: a thread panicked while holding it. The state behind it
/// is not read again. Until slice S3b makes this the daemon-fatal abort of spec §3.1, the
/// caller ends its connection with this error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Poisoned {
````

with this:

````rust

/// A lock of the daemon is poisoned: a thread panicked while holding it. The state behind it
/// is not read again. Every place that makes one has first ended the daemon (`poisoned`), so a
/// caller sees this error only when a test's `on_fatal` returns; it then ends its connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Poisoned {
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    pub(super) clock: Arc<dyn Clock>,
    pub(super) spawner: Spawner,
    #[cfg(test)]
    pub(super) hooks: TestHooks,
````

with this:

````rust
    pub(super) clock: Arc<dyn Clock>,
    pub(super) spawner: Spawner,
    /// What ends the daemon. Production: [`abort_daemon`].
    pub(super) on_fatal: OnFatal,
    #[cfg(test)]
    pub(super) hooks: TestHooks,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
            clock: Arc::new(SystemClock),
            spawner: Box::new(spawn_build_thread),
            #[cfg(test)]
            hooks: TestHooks::default(),
````

with this:

````rust
            clock: Arc::new(SystemClock),
            spawner: Box::new(spawn_build_thread),
            on_fatal: Arc::new(abort_daemon),
            #[cfg(test)]
            hooks: TestHooks::default(),
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    clock: Arc<dyn Clock>,
    spawner: Spawner,
    #[cfg(test)]
    hooks: TestHooks,
````

with this:

````rust
    clock: Arc<dyn Clock>,
    spawner: Spawner,
    on_fatal: OnFatal,
    #[cfg(test)]
    hooks: TestHooks,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
            clock: parts.clock,
            spawner: parts.spawner,
            #[cfg(test)]
            hooks: parts.hooks,
````

with this:

````rust
            clock: parts.clock,
            spawner: parts.spawner,
            on_fatal: parts.on_fatal,
            #[cfg(test)]
            hooks: parts.hooks,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
        if state.startup == StartupMode::Background {
            let mut control = state.lock_control()?;
            let _started = state.start_attempt(&mut control);
        }
````

with this:

````rust
        if state.startup == StartupMode::Background {
            let mut control = state.lock_control()?;
            let _fatal = state.guard(Domain::Control);
            let _started = state.start_attempt(&mut control);
        }
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    ) -> Result<Answer, Poisoned> {
        let mut control = self.lock_control()?;

        // A connection's next call first hears its own attempt's unheard failure, before the
````

with this:

````rust
    ) -> Result<Answer, Poisoned> {
        let mut control = self.lock_control()?;
        let _fatal = self.guard(Domain::Control);

        // A connection's next call first hears its own attempt's unheard failure, before the
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
                Err(_) => return Err(Poisoned { lock: "control" }),
            };
            control.waiting = control.waiting.saturating_sub(1);
````

with this:

````rust
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
                Err(_) => return Err(self.poisoned("control")),
            };
            control.waiting = control.waiting.saturating_sub(1);
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        let control = self.lock_control()?;
        Ok(BuildStatus {
            state: control.build.state(),
````

with this:

````rust
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        let control = self.lock_control()?;
        let _fatal = self.guard(Domain::Control);
        Ok(BuildStatus {
            state: control.build.state(),
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// `query` must not call `read` or `write` again: the slot's lock is not re-entrant, and a
    /// nested call can deadlock. `status` is safe to call from it.
    pub(super) fn read<R>(
        &self,
````

with this:

````rust
    /// `query` must not call `read` or `write` again: the slot's lock is not re-entrant, and a
    /// nested call can deadlock. `status` is safe to call from it.
    ///
    /// A panic in `query` ends the daemon (provider dispatch, spec §3.1).
    pub(super) fn read<R>(
        &self,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
            .provider
            .read()
            .map_err(|_| Poisoned { lock: "provider" })?;
        let answer = slot.as_ref().map(query);
        Ok(answer)
````

with this:

````rust
            .provider
            .read()
            .map_err(|_| self.poisoned("provider"))?;
        let _fatal = self.guard(Domain::Dispatch);
        let answer = slot.as_ref().map(query);
        Ok(answer)
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// `change` must not call `read` or `write` again: the slot's lock is not re-entrant, and
    /// a nested call deadlocks. `status` is safe to call from it.
    pub(super) fn write<R>(
        &self,
````

with this:

````rust
    /// `change` must not call `read` or `write` again: the slot's lock is not re-entrant, and
    /// a nested call deadlocks. `status` is safe to call from it.
    ///
    /// A panic in `change` ends the daemon (provider dispatch, spec §3.1): a refresh that
    /// unwinds may have replaced part of the provider and not the rest.
    pub(super) fn write<R>(
        &self,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
            .provider
            .write()
            .map_err(|_| Poisoned { lock: "provider" })?;
        let Some(provider) = slot.as_mut() else {
            return Ok(None);
````

with this:

````rust
            .provider
            .write()
            .map_err(|_| self.poisoned("provider"))?;
        let _fatal = self.guard(Domain::Dispatch);
        let Some(provider) = slot.as_mut() else {
            return Ok(None);
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust

    fn lock_control(&self) -> Result<MutexGuard<'_, ControlState>, Poisoned> {
        self.control
            .lock()
            .map_err(|_| Poisoned { lock: "control" })
    }

````

with this:

````rust

    fn lock_control(&self) -> Result<MutexGuard<'_, ControlState>, Poisoned> {
        self.control.lock().map_err(|_| self.poisoned("control"))
    }

    /// Ends the daemon if this thread unwinds before the guard is dropped (`fatal.rs`).
    ///
    /// Each caller makes it right after it has taken its lock, never before: locals are dropped
    /// in reverse order, so the guard then fires while the lock is still held, and in
    /// production the process aborts before another thread can take the lock (a refresh
    /// waiting on a reader that panicked, say). Taking the lock cannot panic; a poisoned lock
    /// is reported by `poisoned`.
    fn guard(&self, domain: Domain) -> FatalOnUnwind<'_> {
        FatalOnUnwind::new(&self.on_fatal, domain)
    }

    /// `lock` was found poisoned: end the daemon (spec §4.2, "no lock is ever unwrapped past
    /// poisoning"). The error returned reaches the caller only when a test's `on_fatal`
    /// returns.
    fn poisoned(&self, lock: &'static str) -> Poisoned {
        (self.on_fatal)(Fatal::Poisoned { lock });
        Poisoned { lock }
    }

````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
            Ok(mut slot) => {
                *slot = Some(provider);
                // A poisoned control lock is left alone here as it is in `settle`.
                let _recorded = self.record_generation(&slot, generation);
                true
            }
            Err(_) => false,
        };
        if !published {
````

with this:

````rust
            Ok(mut slot) => {
                *slot = Some(provider);
                // A poisoned control lock ends the daemon (`lock_control`); under a test's
                // `on_fatal`, which returns, publication goes on and settles the attempt.
                let _recorded = self.record_generation(&slot, generation);
                true
            }
            Err(_) => {
                (self.on_fatal)(Fatal::Poisoned { lock: "provider" });
                false
            }
        };
        if !published {
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// Record how `attempt` ended, in its own outcome and in the phase, and tell every parked
    /// call. Nothing is recorded unless the phase is still `Building` that attempt, so an
    /// attempt ends once and `Ready` is not left. A poisoned control lock is left alone; the
    /// parked calls find it when they wake.
    fn settle(&self, attempt: u64, ended: Result<(), String>) {
        if let Ok(mut control) = self.control.lock() {
            let building = match &control.build {
                BuildPhase::Building {
````

with this:

````rust
    /// Record how `attempt` ended, in its own outcome and in the phase, and tell every parked
    /// call. Nothing is recorded unless the phase is still `Building` that attempt, so an
    /// attempt ends once and `Ready` is not left. A poisoned control lock ends the daemon; under
    /// a test's `on_fatal` the parked calls find it when they wake.
    fn settle(&self, attempt: u64, ended: Result<(), String>) {
        let control = match self.control.lock() {
            Ok(control) => Some(control),
            Err(_) => {
                (self.on_fatal)(Fatal::Poisoned { lock: "control" });
                None
            }
        };
        if let Some(mut control) = control {
            let building = match &control.build {
                BuildPhase::Building {
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
}

/// Ends the attempt as failed if the build thread unwinds, so that a panic in the builder
/// cannot leave the phase `Building` with nobody left to end it.
struct PanicGuard<'a> {
    state: &'a DaemonState,
````

with this:

````rust
}

/// Ends the daemon if the build thread unwinds (spec §3.1: a panic on the build thread is
/// fatal). Under a test's `on_fatal`, which returns, it then ends the attempt as failed, so
/// that the phase is not left `Building` with nobody to end it.
struct PanicGuard<'a> {
    state: &'a DaemonState,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.state
                .settle(self.attempt, Err(BUILD_PANICKED.to_string()));
````

with this:

````rust
    fn drop(&mut self) {
        if std::thread::panicking() {
            (self.state.on_fatal)(Fatal::Panicked(Domain::Build));
            self.state
                .settle(self.attempt, Err(BUILD_PANICKED.to_string()));
````

- [ ] **Step 4: Run the module**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t5.log 2>&1; grep -E '^test result|FAILED|^warning' /tmp/s3b-t5.log
```

Expected: `77 passed; 0 failed` (76 + 1; the four S3a tests are changed), no warning. (Observed.)

- [ ] **Step 5: Commit**

```bash
git add src/mcp/daemon/coordinator.rs src/mcp/daemon/test_support.rs src/mcp/daemon/coordinator_failure_tests.rs src/mcp/daemon/coordinator_order_tests.rs src/mcp/daemon/connection_parity_tests.rs
git commit -m "feat(mcp): a panic or a poisoned lock in the daemon's coordinator ends the daemon"
```

- [ ] **Step 6: Prove the tests bite**

| Mutant | Edit | Must fail (observed in validation, on the code after Task 6) |
|---|---|---|
| A: no guard in `write` | in `coordinator.rs`, delete the line `let _fatal = self.guard(Domain::Dispatch);` in `write` | `a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on`, `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` (2 of 83) |
| B: a builder panic only fails the attempt (S3a's `PanicGuard`) | in `PanicGuard::drop`, delete `(self.state.on_fatal)(Fatal::Panicked(Domain::Build));` | `a_builder_panic_is_fatal` (1 of 83) |
| C: a poisoned control lock found without ending the daemon | in `lock_control`, replace `self.control.lock().map_err(\|_\| self.poisoned("control"))` with `self.control.lock().map_err(\|_\| Poisoned { lock: "control" })` | `a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang` (1 of 83). In validation this mutant first **survived**: the test checked only that every recorded event named the control lock, and the build thread's own finds kept the list non-empty. The test now counts each find, and the mutant is killed |
| D: a guard made before its lock | in `read`, move `let _fatal = self.guard(Domain::Dispatch);` above `let slot = self` | `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` (1 of 83) |

### Task 6: A connection's boundary, and where each panic is caught

`serve_bounded` (D43), the guards' tests over sockets, and spec §9 test 12 in a child process (D44, D45).

**Files:**
- Modify: `src/mcp/daemon/connection.rs`
- Create: `src/mcp/daemon/connection_panic_tests.rs`

**Interfaces:**
- Consumes: Tasks 4 and 5; `ToolRegistry::register`, `ToolDescriptor { name, description, input_schema, annotations, runtime_behavior, handler }`, `ToolAnnotations::read_only`, `ToolContext`, `McpToolResult` (all public); `crate::mcp::transport::{InMemoryTransport, ReadOutcome, Transport}`; `clock::Clock`; `wire_support::connect_with` (Task 2); `std::process::{Command, Child, ExitStatus}`, `std::os::unix::process::ExitStatusExt::signal`, `libc::SIGABRT`.
- Produces: private `fn serve_bounded(state: &Arc<DaemonState>, registry: &ToolRegistry, transport: &mut impl Transport, client: &ClientSettings) -> anyhow::Result<()>`, which `serve_admitted` now calls. Test code: `DaemonProcess` (kills and reaps its child on drop), `start_daemon_process(socket) -> (DaemonProcess, JoinHandle<String>)` (stdin piped and held open; stderr drained on a thread from the start), `ended_process`. The child process exits when its standard input reaches end of file (review round 1, Opus F5). `serve_admitted`'s signature is unchanged; it no longer unwinds.

- [ ] **Step 1: Write the tests**

Create `src/mcp/daemon/connection_panic_tests.rs`:

````rust
//! The panic boundaries of a daemon connection (spec §3.1 "Fatal", §4.2 "Panic policy", §10):
//! where a panic is caught, and what it ends. Spec §9 test 12 runs the daemon in a process of
//! its own, because what it checks is that the process ends.
use super::super::clock::Clock;
use super::super::coordinator::{DaemonState, Parts};
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    client_settings, fixture, recording_state, scripted, wait_until, FatalLog,
};
use super::super::wire_support::{
    close, connect, connect_with, ended, Client, CALLEES, INIT, INITED, LIST, PING,
};
use super::{serve_admitted, serve_bounded};
use crate::mcp::output::McpToolResult;
use crate::mcp::registry::{ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry};
use crate::mcp::transport::{InMemoryTransport, ReadOutcome, Transport};
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::{json, Value};
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

const PANICKING_TOOL: &str = "panic_in_dispatch";
const PANIC_CALL: &str = r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"panic_in_dispatch","arguments":{}}}"#;
const HANDLER_PANIC: &str = "injected panic in a tool handler";

/// Every tool, and one more whose handler panics. A tool handler runs under the provider
/// slot's read lock, so its panic is a panic in provider dispatch.
fn registry_with_a_panicking_tool() -> Arc<ToolRegistry> {
    let mut registry = ToolRegistry::all_v1();
    registry.register(ToolDescriptor {
        name: PANICKING_TOOL,
        description: String::new(),
        input_schema: json!({ "type": "object" }),
        annotations: ToolAnnotations::read_only("panics"),
        runtime_behavior: None,
        handler: Box::new(
            |_context: &ToolContext<'_>, _arguments: &Value| -> McpToolResult {
                panic!("{HANDLER_PANIC}")
            },
        ),
    });
    Arc::new(registry)
}

/// A panic in a tool handler is caught where it is raised, in provider dispatch, and ends the
/// daemon; under a recording `on_fatal` the call's connection then ends. Rules out a handler
/// panic that ends only its own connection (the boundary of `serve_bounded`, which a running
/// daemon must not reach for a panic under a lock).
#[test]
fn a_panic_in_a_query_ends_the_daemon() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let (state, log) = recording_state(builder);
    let registry = registry_with_a_panicking_tool();
    let (mut a, a_worker) = connect_with(&state, &registry, client_settings(600));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    a.send(PANIC_CALL);
    a.expect_closed();
    assert!(ended(a_worker).is_err());
    assert_eq!(log.events(), vec![Fatal::Panicked(Domain::Dispatch)]);
}

/// A clock that panics once it is armed. The coordinator reads the clock under the control
/// lock when it starts an attempt.
struct ArmedClock {
    armed: AtomicBool,
}

impl Clock for ArmedClock {
    fn now(&self) -> Instant {
        assert!(
            !self.armed.load(Ordering::SeqCst),
            "injected panic under the control lock"
        );
        Instant::now()
    }
}

/// A panic under the control lock ends the daemon. Rules out a control plane without a
/// boundary of its own, whose panic would close one connection and leave the lock poisoned
/// for every other client.
#[test]
fn a_panic_in_the_control_plane_ends_the_daemon() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let clock = Arc::new(ArmedClock {
        armed: AtomicBool::new(false),
    });
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let shared: Arc<dyn Clock> = Arc::clone(&clock) as Arc<dyn Clock>;
    parts.clock = shared;
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();

    clock.armed.store(true, Ordering::SeqCst);
    a.send(CALLEES);
    a.expect_closed();
    assert!(ended(a_worker).is_err());
    assert_eq!(log.events(), vec![Fatal::Panicked(Domain::Control)]);
}

/// A transport that panics while it writes its second response.
struct PanicsOnTheSecondWrite {
    inner: InMemoryTransport,
    writes: usize,
}

impl Transport for PanicsOnTheSecondWrite {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        self.inner.read_message()
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        self.writes += 1;
        assert!(
            self.writes != 2,
            "injected panic while a response is written"
        );
        self.inner.write_message(v)
    }
}

/// Spec §4.2: a panic in a connection's framing or output, outside both domains, closes that
/// connection only. The connection's serve loop returns an error instead of unwinding into the
/// accept loop's worker, nothing ends the daemon, and the next client is served. Rules out a
/// connection without a boundary (its panic would reach whatever runs the worker) and a
/// boundary that ends the daemon for a panic that touched none of its state.
#[test]
fn a_panic_outside_both_domains_closes_that_connection_only() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let (state, log) = recording_state(builder);
    let mut transport = PanicsOnTheSecondWrite {
        inner: InMemoryTransport::new(vec![INIT, INITED, PING]),
        writes: 0,
    };

    let served = serve_bounded(
        &state,
        &ToolRegistry::all_v1(),
        &mut transport,
        &client_settings(600),
    );
    let error = served.expect_err("the connection ends with an error");
    assert!(error.to_string().contains("the daemon goes on"), "{error}");
    assert_eq!(log.events(), vec![]);

    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let answer = b.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    close(b, b_worker);
    assert_eq!(log.events(), vec![]);
}

/// Where the daemon process of the test below listens.
const DAEMON_SOCKET: &str = "PRISM_MCP_TEST_PANIC_DAEMON_SOCKET";

/// Not a test of its own: the daemon process of
/// `a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end`,
/// started as a child with `DAEMON_SOCKET` set. In an ordinary run the variable is unset and
/// this returns at once. It serves every connection it accepts on a thread of its own with the
/// production `on_fatal`, the abort. It exits when its standard input reaches end of file, so
/// that it ends with the test that started it even when that test is killed and cannot reap it.
#[test]
fn a_daemon_process_for_the_panic_test() {
    let Some(socket) = std::env::var_os(DAEMON_SOCKET) else {
        return;
    };
    std::thread::spawn(|| {
        let mut ignored = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut ignored);
        std::process::exit(0);
    });
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let registry = registry_with_a_panicking_tool();
    let listener = UnixListener::bind(&socket).expect("bind the test socket");
    for stream in listener.incoming() {
        let stream = stream.expect("accept a connection");
        let state = Arc::clone(&state);
        let registry = Arc::clone(&registry);
        std::thread::spawn(move || {
            serve_admitted(&state, &registry, stream, &client_settings(600))
        });
    }
}

/// The daemon process. It is killed and reaped when the test ends, however the test ends, so
/// that a run in which the daemon does not abort (a defect, or a mutant) leaves no process
/// behind holding an index.
struct DaemonProcess(Child);

impl Drop for DaemonProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Start the daemon process listening at `socket`, with a pipe to its standard input that it
/// holds open, and a thread that reads its standard error from the start (so that the child
/// never blocks on a full pipe) and returns it when the child ends.
fn start_daemon_process(socket: &Path) -> (DaemonProcess, JoinHandle<String>) {
    let (_crate, module) = module_path!()
        .split_once("::")
        .expect("a module path below the crate");
    let daemon_test = format!("{module}::a_daemon_process_for_the_panic_test");
    let mut daemon = DaemonProcess(
        Command::new(std::env::current_exe().expect("the test binary"))
            .args([
                daemon_test.as_str(),
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(DAEMON_SOCKET, socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start the daemon process"),
    );
    let mut stderr = daemon.0.stderr.take().expect("the daemon's standard error");
    let drained = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    (daemon, drained)
}

/// Wait for the daemon process to end and return how it ended.
fn ended_process(daemon: &mut DaemonProcess) -> ExitStatus {
    let mut ended_with = None;
    wait_until("the daemon process to end", || {
        ended_with = daemon.0.try_wait().expect("the daemon process's status");
        ended_with.is_some()
    });
    ended_with.expect("the daemon process ended")
}

/// The daemon process ends by itself when the test that started it is gone: its standard input
/// reaches end of file. Rules out a child that, when its parent is killed without unwinding
/// (and so without `DaemonProcess`'s kill), goes on serving for ever.
#[test]
fn the_daemon_process_ends_when_its_standard_input_is_closed() {
    let dir = tempfile::tempdir().expect("a directory for the socket");
    let socket = dir.path().join("daemon.sock");
    let (mut daemon, drained) = start_daemon_process(&socket);
    let a = attach(&socket);

    drop(daemon.0.stdin.take());
    let ended_with = ended_process(&mut daemon);
    assert!(ended_with.success(), "{ended_with:?}");
    drop(a);
    let _ = drained.join();
}

fn attach(socket: &Path) -> Client {
    let mut stream = None;
    wait_until("the daemon process to listen", || {
        stream = UnixStream::connect(socket).ok();
        stream.is_some()
    });
    let mut client = Client::new(stream.expect("a connection to the daemon process"));
    client.handshake();
    client
}

/// Spec §9 test 12, as far as it can run before the shim exists: an injected panic in
/// dispatch aborts the daemon process, and every attached client sees its connection end,
/// the one whose call panicked and one that was idle. (That the next shim starts a new daemon
/// is slice S5a's.) Rules out a production `on_fatal` that does not end the process, and a
/// panic in dispatch caught by the connection's own boundary.
#[test]
fn a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end() {
    let dir = tempfile::tempdir().expect("a directory for the socket");
    let socket = dir.path().join("daemon.sock");
    let (mut daemon, drained) = start_daemon_process(&socket);

    let mut a = attach(&socket);
    let mut b = attach(&socket);
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");
    let listed = b.request(LIST);
    assert!(listed["result"]["tools"].is_array(), "{listed}");

    a.send(PANIC_CALL);
    a.expect_closed();
    b.expect_closed();

    let ended_with = ended_process(&mut daemon);
    assert_eq!(ended_with.signal(), Some(libc::SIGABRT), "{ended_with:?}");
    let stderr = drained.join().expect("the daemon's standard error is read");
    assert!(stderr.contains(HANDLER_PANIC), "{stderr}");
    assert!(
        stderr.contains("prism-mcp daemon: a panic in provider dispatch"),
        "{stderr}"
    );
}
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
#[path = "connection_isolation_tests.rs"]
mod isolation_tests;
````

with this:

````rust
#[path = "connection_isolation_tests.rs"]
mod isolation_tests;

#[cfg(test)]
#[path = "connection_panic_tests.rs"]
mod panic_tests;
````

- [ ] **Step 2: Run them to see them fail**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t6-red.log 2>&1; grep -E '^error' /tmp/s3b-t6-red.log | sort | uniq -c
```

Expected: one compile error, `error[E0432]: unresolved import `super::serve_bounded``. (Observed.)

- [ ] **Step 3: Add the connection's boundary**

In `src/mcp/daemon/connection.rs`, replace this:

````rust
//! access, so its response and the refresh's generation are one snapshot's.
//!
//! What this slice leaves to S3b-2: the panic boundaries.
use super::budget::ConnectionBudget;
use super::coordinator::{Answer, DaemonState};
````

with this:

````rust
//! access, so its response and the refresh's generation are one snapshot's.
//!
//! Panics: a panic in a query, a refresh, the build or the control plane ends the daemon
//! (`fatal.rs`); one anywhere else on this connection's thread ends this connection only
//! (`serve_bounded`).
use super::budget::ConnectionBudget;
use super::coordinator::{Answer, DaemonState};
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
use serde_json::{json, Value};
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// How many seconds a client that is told `indexing` is asked to wait before it retries. Five
````

with this:

````rust
use serde_json::{json, Value};
use std::os::unix::net::UnixStream;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

/// What a connection's serve loop returns when a panic on its thread was raised outside the
/// daemon's two domains.
const CONNECTION_PANICKED: &str =
    "a panic on this connection's thread outside the index and the control plane; the \
     connection is closed and the daemon goes on";

/// How many seconds a client that is told `indexing` is asked to wait before it retries. Five
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
    stream.set_read_timeout(None)?;
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_client(state, registry, &mut transport, client)
}

````

with this:

````rust
    stream.set_read_timeout(None)?;
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_bounded(state, registry, &mut transport, client)
}

/// The connection's panic boundary (spec §4.2 "Panic policy", §10). A panic that reaches it
/// was raised outside both of the daemon's domains, in framing, in a readiness answer or in
/// writing a response: one inside them ends the daemon before it unwinds this far. It ends
/// this connection only, with an error; the caller drops the socket and the client sees its
/// connection end. Nothing the closure borrowed is used after a panic but `state`, whose shared
/// state changes only inside its own domains.
fn serve_bounded(
    state: &Arc<DaemonState>,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let served = std::panic::catch_unwind(AssertUnwindSafe(|| {
        serve_client(state, registry, transport, client)
    }));
    match served {
        Ok(result) => result,
        Err(_panic) => Err(anyhow::anyhow!(CONNECTION_PANICKED)),
    }
}

````

- [ ] **Step 4: Run the module**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3b-t6.log 2>&1; grep -E '^test result|FAILED|^warning' /tmp/s3b-t6.log
```

Expected: `83 passed; 0 failed` (77 + 6, one of them the child-process test that returns at once), no warning. The test kills and reaps the child however it ends (`DaemonProcess`), so a failing run, or a mutant run in Step 6, leaves no process behind; check with `pgrep -f a_daemon_process_for_the_panic_test` after Step 6, which must print nothing. The run starts one child process, which aborts; on macOS it leaves one crash report in `~/Library/Logs/DiagnosticReports/` (`prism-<hash>-<time>.ips`, about 80 KB). (Observed: `79 passed; 0 failed`, no warning, one report.)

- [ ] **Step 5: Commit**

```bash
git add src/mcp/daemon/connection.rs src/mcp/daemon/connection_panic_tests.rs
git commit -m "feat(mcp): a connection's own panic boundary; a panic in dispatch aborts the daemon"
```

- [ ] **Step 6: Prove the tests bite**

| Mutant | Edit | Must fail (observed in validation) |
|---|---|---|
| A: no guard in `read` | in `coordinator.rs`, delete the line `let _fatal = self.guard(Domain::Dispatch);` in `read` | `a_panic_in_a_query_ends_the_daemon`, and the process test at the 30 s limit (B's connection stays open: the panic ended only A's), and the lock-held test (3 of 83) |
| B: no guard in `ensure_ready` | delete the line `let _fatal = self.guard(Domain::Control);` in `ensure_ready` | `a_panic_in_the_control_plane_ends_the_daemon`, the lock-held test (2 of 83) |
| C: no connection boundary | in `serve_bounded`, replace the `catch_unwind` statement with `let served: std::thread::Result<anyhow::Result<()>> = Ok(serve_client(state, registry, transport, client)); let _ = AssertUnwindSafe(());` | `a_panic_outside_both_domains_closes_that_connection_only`, `a_panic_in_a_query_ends_the_daemon`, `a_panic_in_the_control_plane_ends_the_daemon` (3 of 83: the last two because, under a returning handler, their connection's worker now panics) |
| D: the production handler returns | in `fatal.rs`, replace `std::process::abort();` with `let _ = 0;` | the process test, at the 30 s limit (1 of 83) |
| E: the child outlives a parent that cannot reap it | in `a_daemon_process_for_the_panic_test`, delete `std::process::exit(0);` | `the_daemon_process_ends_when_its_standard_input_is_closed`, at the 30 s limit (1 of 83) |

### Task 7: S3b-2 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
before it announces `Ready`. The generation is recorded under the slot's write guard at
publication and after every exclusive access, so it is the slot's whatever the phase. Lock order: code holding the control lock
does not touch the slot; code holding the slot may take the control lock. A poisoned lock is the
`Poisoned` error and is never unwrapped; there is no panic boundary yet. Time is the injected
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
`control.rs` holds the types the control lock guards (`BuildPhase`, `ControlState`,
````

with this:

````markdown
before it announces `Ready`. The generation is recorded under the slot's write guard at
publication and after every exclusive access, so it is the slot's whatever the phase. Lock order: code holding the control lock
does not touch the slot; code holding the slot may take the control lock. A panic in a query or
a refresh (`read`, `write`), on the build thread, or in the control plane (`ensure_ready`,
`status`), and a lock found poisoned, end the daemon through `Parts::on_fatal` (`fatal.rs`:
a `FatalOnUnwind` guard at each entry; production `abort_daemon` writes one line to stderr and
aborts, so every attached client sees its connection end); nothing is read through a poisoned
lock, and a test installs a `FatalLog` instead of the abort. Time is the injected
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
`control.rs` holds the types the control lock guards (`BuildPhase`, `ControlState`,
````

In `CLAUDE.md`, replace this:

````markdown
The query after a refresh runs under the same write lock, so a response and the refresh
generation it reports are one snapshot's. No lock is held while a response is written: a client
that stops reading stalls only its own connection. Not there yet (the next slice): panic
boundaries.

Algorithms fall into two categories:
````

with this:

````markdown
The query after a refresh runs under the same write lock, so a response and the refresh
generation it reports are one snapshot's. No lock is held while a response is written: a client
that stops reading stalls only its own connection. A panic anywhere else on a connection's
thread (framing, the readiness answers, writing a response) ends that connection only:
`serve_bounded` turns it into an error and the daemon goes on.

Algorithms fall into two categories:
````

- [ ] **Step 2: Run the MCP surface and the checks**

The commands of Task 3 Step 2, with `s3b2` in the log names. Expected: `cargo fmt --check` prints nothing; the lib line `469 passed; 0 failed` (452 + 17); the `mcp` target `23 passed` with the golden transcripts' test `ok`; the `cargo check` runs `Finished`, no warning from the feature build; no clippy line names `src/mcp/daemon/`. (Observed at the S3b-2 head, `scratch/s3b-tasks3` `d332af28`: `cargo fmt --check` silent; `469 passed; 0 failed`; `23 passed`, `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; `cargo check`, `cargo check --features mcp` (0 warnings) and the owner-audit build `Finished`; clippy 257 `warning` lines in the whole log, none naming `src/mcp/daemon/`.)

- [ ] **Step 3: State the invariants**

The commands of Task 3 Step 3. Expected: `0`; `0`; against S3b-1's merge, 11 files changed (ten under `src/mcp/daemon/` and `CLAUDE.md`), 778 insertions and 38 deletions; no file over 600 lines (`coordinator.rs` is the largest this slice edits, 548; `connection_tests.rs`, 577, is not edited).

- [ ] **Step 4: Run the remaining pre-PR checks**

The commands of Task 3 Step 4, with `s3b2` in the log names. Expected as there, with `82 passed` in the repeat lines (83 less the skipped process test). Each of the 60 repeat runs starts the child process of Task 6 and, on macOS, leaves one crash report; add `-- --skip a_panic_in_dispatch_ends_the_daemon_process` to the repeat loops to keep them out, and run that test alone five times instead:

```bash
for i in $(seq 1 5); do cargo test --features mcp --lib mcp::daemon::connection::panic_tests > /tmp/s3b2-process-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3b2-process-$i.log; done | sort | uniq -c
```

Expected: `5 test result: ok. 6 passed; 0 failed`.

- [ ] **Step 5: Commit and open the PR**

The PR body states, in this order: (1) **Goal:** the lane goal; S3b-2 makes the spec's panic policy true, and says what a client sees for each kind of panic (the table "Panic boundaries"). (2) **Invariants:** Step 3. (3) **Decisions:** D39–D46, SI-10, SI-12, SI-13. D42 and SI-10 are the ones the owner may want to see. (4) **Claims:** the S3b-2 rows, with what no test holds. (5) **Totals:** Steps 2 and 4 next to the base, and the mutants of Tasks 4–6 as observed, including Task 5 C's first survival and the test change that killed it. (6) **Not verified:** Linux; the restart by the next shim (S5a); a panic injected into a `--warm-at-startup` start. (7) **Tier-A:** not run (no file it watches is touched).

```bash
git add CLAUDE.md
git commit -m "docs(mcp): the shared daemon's panic boundaries"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): panic boundaries (shared daemon S3b-2)" --body-file /tmp/s3b2-pr-body.md
```

Review brief and cap as in Task 3.

---

## Interfaces for the next slices

Everything below is in `crate::mcp::daemon` and `pub(super)` unless a path says otherwise.

**S4 (ownership, handshake stage, admission and retirement, Draining) consumes and must keep:**

- `Parts::on_fatal: OnFatal` and `fatal::abort_daemon`. `DaemonState::new` uses the abort; a test that reaches a fatal event installs `test_support::FatalLog` (or uses `recording_state`) and must never run with the production handler, or it ends the test binary.
- **Every new entry point into the control plane takes the guard first.** S4's admission, retirement and status-probe functions run under the control lock (spec §4.2 "one critical section"); each starts with `let _fatal = self.guard(Domain::Control);` and gets its lock through `lock_control`, so that a panic or a poisoned lock in it ends the daemon. `guard` and `poisoned` are private to `coordinator.rs`; S4 adds its functions there (as S3a's interface already suggests for `control` and `changed`) or widens them to `pub(super)`. A function that only takes the lock and notifies, as `wake` does, needs none.
- **The accept loop and the handshake stage are outside both domains**, like a connection's framing: a panic there should end what it was doing, not the daemon. A panic in the accept loop itself ends accepting, which is the daemon's end in all but name; S4 decides whether that is `on_fatal(Panicked(Control))` or a retirement.
- `connection::serve_admitted` no longer unwinds: it returns `Err` for a panic outside the domains (fixed text, `CONNECTION_PANICKED`), as for a read or write error. The worker thread that calls it logs the error (bounded) and drops the socket; it needs no `catch_unwind` of its own. Admission's decrement (spec §4.2) belongs after `serve_admitted` returns, on every path.
- After an abort the socket file, the pid file and the start lock's file remain; the owner lock is released by the kernel. The start protocol (§4.1 step 6, S5a) replaces them. S4's test of a dead daemon (§9 test 3) can use Task 6's child-process pattern for a daemon that dies by abort.
- `TestHooks::exclusive` / `before_exclusive`: test seam between a call's read and its exclusive access.
- `test_support::{FatalLog, recording_state}`, `wire_support::{connect_with, Client::flood_until_full, Client::send_raw_in_background}`, and `connection_panic_tests`' child-process pattern (`a_daemon_process_for_the_panic_test`, started with `--exact` and an environment variable, held in a `DaemonProcess` that kills and reaps it on drop): the way to test, before S5a, anything that must end a process. A test that starts a process must kill it on every path; validation found two daemon processes left running by failed runs before the guard existed.
- Still open from S3a, unchanged here: a builder that never returns (no attempt end, so "no build in progress" never holds), SI-5 (a parked call of a departed client), the `Background` startup parity requirement, and bounding `BuildPhase::Failed { error }` where it is logged or sent.

**S5a (the shim) owes:** spec §9 tests 6, 7, 9 and 12 through real shims and a real daemon process; for test 12, "the next shim restarts it" (SI-13): after the abort, the next shim finds the owner lock free, replaces the stale socket and starts a daemon that serves it. **And a bound on that restart (SI-10, D42; review round 1, Opus F6):** a builder panic that recurs aborts every daemon on its first tool call, so without a bound each shim start launches and loses another daemon (and on macOS writes another crash report), indefinitely. The start protocol must notice that the key's last daemon died by abort shortly after it started (for example a marker beside the pid file, written by the daemon before its first build and removed when the build publishes, which a starting shim finds with the owner lock free) and then fail closed with a diagnostic naming the log, or back off, rather than start another at once. The spec states no such rule; S5a's plan decides its shape.

**C2-M1 and every candidate that ships through the fan-out gate:** SI-11. The gate's "one concurrent `refresh_index`" measures the write lock held across a rebuild; read its p95/p99 with that in mind before attributing a regression to the candidate.

**S6 (docs)** owes, besides S3a's list: what a client sees when a shared daemon panics (its connection ends; the next start makes a new daemon), and the crash report macOS writes for each abort.

## Self-review record

- **Spec coverage (§12 row S3b).** Concurrent dispatch → Task 2 (eight clients, a stalled reader, a refresh) on S3a's per-connection threads and the slot's `RwLock`. Refresh visibility → Task 1 `a_refresh_by_one_client_is_what_the_next_query_of_another_client_sees` (test 6). Auto-refresh recheck → Task 1 (D34), both directions. Panic boundaries → Tasks 4–6, the table "Panic boundaries" (spec §3.1, §4.2, §10), test 12 in a child process. Tests 7 and 9 → Task 2 (D37).
- **Spec §4.2 sentences checked against the code:** "queries take the read lock and operate on a read-only facade" (`read`, unchanged); "`refresh_index` and automatic refresh take the write lock" (`write`, unchanged); "automatic refresh re-checks its trigger after acquiring exclusive access" (Task 1); "every response is bound to the generation it actually queried" (D36: one lock acquisition per response); "reads and writes happen outside every lock" (Task 2); "a panic anywhere in provider dispatch, the build thread or the control plane aborts the daemon; no lock is ever unwrapped past poisoning" (Tasks 4–6, D41); "panics in a connection's framing or output shaping close that connection only; the implementation marks these boundaries explicitly" (`serve_bounded`, D43, SI-12).
- **S3a's notes for S3b, one by one.** "`ReadDispatch::NeedsAutoRefresh` is the hand-off point for the re-check" → Task 1, inside `run_tool_call`'s last statement as S3a's interface said. "`dispatch_tool_auto_refresh` runs the query while still holding `&mut provider` … it splits mechanically" → not split, D35. "`ReadDispatch::NeedsAutoRefresh` carries no generation … add a generation accessor" → S3a added `SessionProvider::generation()`; D36 needs no more. "`PanicGuard`'s `Drop`: the body to replace with the abort" → D42 (the abort is called there, and the S3a body stays for a test's returning handler). "`Poisoned`: every place it is constructed is a place the abort belongs" → D41, `poisoned`. "`TestHooks` and `AttemptHook`: the pattern for an injected panic in dispatch" → a panicking tool in a test registry instead (D45): it injects the panic where dispatch really runs, with no production seam. "`record_generation` should read the generation from the guard it is handed when S3b or S4 next changes it" → not changed: S3b does not change `record_generation`; it remains S4's.
- **The lane's review lessons.** (1) Every stated property has a test and a mutant, or is listed as not held: "Claims". One mutant survived in validation (Task 5 C) and the test was strengthened, not the mutant waived. (2) What each mechanism does not cover: its section. (3) Interleavings: the race the re-check exists for is played at a seam (Task 1); the stalled reader is made by filling a socket, not by timing (Task 2). (4) No test asserts an upper bound on a duration or sleeps to order events; socket pairs need no path except Task 6's child, whose socket is in a fresh temporary directory. (5) Nothing new is logged but `abort_daemon`'s fixed line and `serve_bounded`'s fixed error; neither carries peer bytes or a panic payload. (6) No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests in the new code.
- **Size.** From the validated branch: S3b-1 `src/` 6 files, 694 insertions, 12 deletions; S3b-2 `src/` 10 files, 769 insertions, 34 deletions; `CLAUDE.md` +8 −2 and +9 −4. Code (non-test) lines: S3b-1 about 45 (`connection.rs`, `coordinator.rs`), S3b-2 about 210 (`fatal.rs` 101, `coordinator.rs` +65, `connection.rs` +36); the rest is tests and test support.

## Validation record

Built on 2026-10-14 on macOS (arm64), in `~/code/prism-s3b-validate`, from `main` `d93d9047`. Every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, with its output in a file under the session's scratch directory, read from the file. Nothing was pushed. Three local branches:

- `scratch/s3b-validate`, where the code was first written, test first: `28020592` (Task 1), `ab807290` (Task 2), `db39880a` (Tasks 4–6 in one commit), `f32d7881` (the change to `a_poisoned_control_lock_…` after its mutant survived), `08b9cd74` (the two test corrections below), then review round 1's fold: `e3d437a9` (the held-write and overlapping-read tests), `abaf4dc9` (D47 and the `publish` comment), and `c079829d` (the child's lifetime). The mutants of every task ran here, the round-1 ones at `c079829d`.
- `scratch/s3b-tasks3`, the same source cut into this plan's tasks (each commit a checkout of the files the task names, plus the two `CLAUDE.md` edits): `0fc0cd34` (Task 1), `4b66b2ca` (Task 2), `c4c57cdd` (Task 3), `1436c5fd` (Task 4), `0445b5e6` (Task 5), `52712e15` (Task 6), `d332af28` (Task 7). Its source at `52712e15` is byte-identical with `c079829d`. The red and green steps of Tasks 2–6 as listed below, the per-task counts and the final checks ran here.
- `scratch/s3b-tasks` and `scratch/s3b-tasks2`, earlier cuts (before the two test corrections, and before review round 1); superseded, kept for the record.

The plan's directives were generated from `scratch/s3b-tasks3` (each task's commit against the one before it) and replayed by a script, in the plan's order, onto the files of `d93d9047`: every anchor matched exactly once at its point in the sequence, and all 15 files the plan touches came out byte-identical with `d332af28`. Every mutant's anchor matches once in the file at the task's commit, except the two guards named "in `write`" and "in `ensure_ready`", whose line occurs twice in `coordinator.rs` and is identified by the function.

| Step | Command | Observed |
|---|---|---|
| Base | `--lib mcp::daemon`; `--lib mcp::`; `--test mcp` (unblessed), at `d93d9047` | `66 passed`; `452 passed`; `23 passed`, the goldens' test `ok` (build 2 m 21 s) |
| T1 red | the tests and the module line only | `error[E0609]: no field `exclusive` on type `TestHooks`` |
| T1 seam only (Step 3 without Step 4) | `--lib mcp::daemon::connection::refresh_tests` | `2 passed; 1 failed`: `a_client_that_finds_the_index_refreshed_once_…`, B's answer `"prism/auto_refresh":"refreshed","prism/refresh_generation":2`, `"prism/stale_index_total_before_refresh":0` |
| T1 green | `--lib mcp::daemon` | `69 passed; 0 failed`, no warning |
| T1 mutants | A, B | A: 1 failed of 69. B: 8 failed of 69 (the three Task 1 tests, both sticky-evidence tests, the byte-parity session, both generation tests) |
| T2 green | `--lib mcp::daemon` at `4b66b2ca` | `72 passed; 0 failed`, no warning; the smoke test's stalled client sent 63 whole frames before `WouldBlock`, at a frame boundary (a temporary print, run once on the first cut, removed) |
| T2 mutants | A, B, C, D | at Task 2's code (`4b66b2ca`): B 18 of 72; C 1 of 2 with the `isolation_tests` filter (the held-write test, at 30 s), with default buffers and again with 212,992-byte buffers; D 1 of 2 (the overlapping-read test, at 30 s). On the final code: A 28 of 83 (60.24 s), B 18 of 83, C as at Task 2 (and the smoke test under C with 212,992-byte buffers failed, 1 of 1, in this one run), D as at Task 2. The correct code with 212,992-byte buffers: the two isolation tests and the smoke test pass |
| T3 | `--lib mcp::` and clippy at the S3b-1 head `c4c57cdd` | `458 passed; 0 failed`; no clippy line names `src/mcp/daemon/` |
| T4 red | the tests and `mod fatal;` only | `error[E0583]: file not found for module `fatal`` |
| T4 green | `--lib mcp::daemon` at `1436c5fd` | `76 passed; 0 failed`; one warning, `function `abort_daemon` is never used` (gone after Task 5) |
| T5 red | the test support, the new test and the four changed tests only | two `error[E0609]`: no field `on_fatal` on `&mut coordinator::Parts` and on `coordinator::Parts` |
| T5 D47 red | the lock-held test against the guards made before their locks (`f32d7881`'s placement) | `35 passed; 1 failed` in `mcp::daemon::coordinator`: `left: [(Panicked(Dispatch), false), (Panicked(Dispatch), false)]` |
| T5 green | `--lib mcp::daemon` at `0445b5e6` | `77 passed; 0 failed`, no warning |
| T6 red | the tests, `connect_with` and the module line only | `error[E0432]: unresolved import `super::serve_bounded`` |
| T6 green | `--lib mcp::daemon` at `52712e15` | `83 passed; 0 failed`, no warning; one crash report per run |
| S3b-2 mutants (final code, `c079829d`) | Task 4 A; Task 5 A–D; Task 6 A–E | 4A 1 of 83; 5A 2; 5B 1; 5C 1 (it had **survived**, `79 passed`, against the test as first written, before the test counted each find); 5D 1; 6A 3 (at the 30 s limit); 6B 2; 6C 3; 6D 1 (30 s); 6E 1 (30 s). After every run `pgrep -f a_daemon_process_for_the_panic_test` printed nothing |
| T1 mutants on the final code | A, B | A 1 of 83; B 10 of 83 |
| V4–V7 | `cargo fmt --check`; `--lib mcp::`; `--test mcp`; `cargo check`; `cargo check --features mcp`; `cargo check --tests --features mcp,detached-owner-audit`; clippy | at `d332af28`: silent; `469 passed; 0 failed`; `23 passed`, `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; `Finished`; `Finished`, 0 warnings; `Finished` (the base's warnings in `tests/`); 257 `warning` lines in the whole log, none naming `src/mcp/daemon/`. At the S3b-1 head `c4c57cdd`: `--lib mcp::` `458 passed`, clippy the same 257 lines and none naming `src/mcp/daemon/` |
| V9 | 30 × `--lib mcp::daemon -- --skip a_panic_in_dispatch_ends_the_daemon_process`, default threads and `--test-threads=1`; 5 × `mcp::daemon::connection::panic_tests`, at `d332af28` | `30 test result: ok. 82 passed; 0 failed` in each mode, at `d332af28` (the 83rd test, the process test, skipped; test time 1.25–1.29 s with default threads, 1.78–2.04 s with one), load average about 22–24; the process tests alone, 5 times: `5 test result: ok. 6 passed; 0 failed`; afterwards `pgrep -f a_daemon_process_for_the_panic_test` printed nothing. |
| V10 | `git diff --name-only d93d9047 d332af28` | 15 paths: 14 under `src/mcp/daemon/` and `CLAUDE.md`; nothing else |
| Full suite | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp`; `cargo test --doc --features mcp`, at `d332af28` | `cargo test --features mcp`: 31 `test result` lines, **5,762 passed, 0 failed, 1 ignored** (the library 1,942, including the process tests; the goldens' test `ok`); `cargo test --doc --features mcp`: `2 passed; 0 failed`. Load averages 22–24. |

**Corrections made to the plan by the validation.**

| # | Where | What was first written | What was wrong | What it says now |
|---|---|---|---|---|
| 1 | Task 5, `a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang` | assert that the `FatalLog` is not empty and every event is `Poisoned { lock: "control" }` | Mutant 5C (`lock_control` without `on_fatal`) survived: the build thread's own finds of the lock kept the list non-empty | the test counts the events after each find (1, 2, 3, then 5 once the build thread has found the lock twice) |
| 2 | Task 2, the stalled client | `send_raw(&mut self, bytes)`, a blocking write of the cut-off rest of a frame from the test's thread, before it reads | When the full socket cuts a frame (it did not on macOS, where the cut fell on a frame boundary; Linux buffers make a cut likely), the server can take the rest only once it can write, and it can write only once the client reads: both ends block, with no read timeout to end it | `send_raw_in_background`: the rest goes out from a thread of its own while the test reads, and the test joins it. The partial-frame path itself was not exercised on this host |
| 3 | Task 6, the process test | the child held as a bare `Child` | A failing run (two mutant runs in validation) returned before reaping the child, and the child, which had not aborted, went on listening with a built index: two such processes were found running afterwards and were killed | `DaemonProcess` kills and reaps the child on drop, on every path; checked with `pgrep` after the mutant runs |
| 4 | Task 2 | `connect_with` added with the flood helpers | unused until Task 6: a dead-code warning in the test build | moved to Task 6 |
| 5 | Task 4 Step 4 | "no warning" | `abort_daemon` is unused in the test build until Task 5 | names the one warning |

**Review round 1 (PR #384 at `7a68854e`; gpt-6.1-sol APPROVE, Opus 5.5 FIX), folded.**

| Finding | What changed | Evidence |
|---|---|---|
| Opus F1 WRONG/MATERIAL, sol F1 SMELL/MATERIAL: test 7 did not prove that a blocked response writer stalls nobody (a provider guard held across the write passed it at Linux-sized buffers), and barrier-released readers could pass serialized | Task 2 gains `a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held` (a test transport holds the write at a gate; the test waits until it is entered, then requires another client's refresh and query to complete) and `two_clients_queries_are_inside_dispatch_at_the_same_time` (a test tool whose handler waits for a second reader). The socket test is kept as a smoke test and no claim rests on it. Mutants C (provider guard across the write) and D (queries take the write lock) added. "Either way" in the not-held list removed; D37, Review Focus 2, the claims table and "What each mechanism covers" corrected | C killed at default and 212,992-byte buffers, at Task 2's code and at the end; D killed; correct code passes at 212,992-byte buffers |
| Opus F2 SMELL: each guard fired after its lock was let go; the panic table's "no response to calls in flight" was too strong | **Both:** the guards are made right after their locks (D47), so `on_fatal` runs while the lock is held, and the table now says a call already running under a shared lock on another connection may finish first. Chosen over text alone because the move is one line per entry point and closes the window in which a waiting refresh, or a racing poisoned-lock report, gets in before the abort | `a_panic_under_a_lock_ends_the_daemon_while_the_lock_is_still_held` failed on the old placement and passes; mutant 5D killed |
| Opus F3 SMELL: no test holds `status`'s guard | Listed under "Stated and not held", with the reason (nothing in `status` can be made to panic without a production seam) | — |
| Opus F4 SMELL: stale comment in `publish` | Corrected in Task 5 | — |
| Opus F5 SMELL: the child process outlives a parent that cannot reap it; stderr read only after exit | The child exits on standard-input end of file (the parent holds the pipe); stderr is drained on a thread from spawn | `the_daemon_process_ends_when_its_standard_input_is_closed`; mutant 6E killed |
| Opus F6 SMELL: SI-10's restart loop not handed to S5a | SI-10 and "Interfaces for the next slices" (S5a) state that the start protocol must bound it | — |

One accident during the fold, recorded: a hung mutant run was killed and the worktree restored with `git checkout -- src`, which discarded an uncommitted module line; it was re-added and committed before any further experiment (the plan's own "commit before any experiment" rule, broken once). Mutant C on the whole module makes S3a tests that take the write lock from the test thread wait for ever; it is run filtered.

**Not run in this validation:** the mutation gate (it selects no MCP code); anything on Linux; a release build; the Tier-A harness (no file it watches is touched).
