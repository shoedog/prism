# S3a — Readiness Coordinator and Provider Slot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Validation status (2026-10-13).** Every task of this plan has been replayed with a compiler. **Tasks 1–5 (S3a-1)** were replayed, changed by one review round that found two defects in the coordinator, approved in a second round, and given five more tests by the controller; the plan's text reproduces that branch (`f2d56b1c`, merged to `main` as `e7dc1cc2`) byte for byte. **Tasks 6–14 (S3a-2)** are based on `origin/main` at `e7dc1cc2`, the squash merge of S3a-1 (pull request #374, the same tree as `f2d56b1c`), and were replayed there on the scratch branch `scratch/s3a2-validate`: Tasks 6, 7 and 8 as first written, each compiling and passing at the first attempt; then one test asked for by the S3a-1 reviewers; then Task 9 (a file split they asked for) and Task 10 (an owner decision that replaces D26), both built on the branch, test first, and written into this plan from the edits that were made. The first review of S3a-2 (pull request #377) found one parity break and asked for four smaller things; the fold added Task 11 (a connection's next call first hears its own attempt's unheard failure, D32) and Task 12 (the stream's mode, D33), built on `feat/mcp-daemon-s3a-2` test first. The second review approved (sol) or asked for text only (Opus); with it came the owner's amendment of D26, built as Task 13. The plan's text reproduces that branch byte for byte as well. What was observed is in the five records at the end. Nothing here ran on Linux, and the unfiltered suite, the doc tests and the mutation gate were not run in validation.

## Validate before dispatch

What each check must show, and what it showed. Every cargo command ran alone, niced, with four jobs, at host load averages between 30 and 75.

| # | Command | Must show | S3a-1 (Tasks 1–5) | S3a-2 (Tasks 6–14) |
|---|---|---|---|---|
| V1 | `cargo test --features mcp --lib mcp::` and `cargo test --features mcp --test mcp` on the untouched base | The two base totals; every later total is "base + N". | Lib: **380 passed** at `86756aad`. | Lib 415 at `f2d56b1c` (the controller's number); `--test mcp`: **23 passed** on the S3a-1 head, with the golden transcripts. |
| V2 | Each task's red step | A compile failure. | Observed for Tasks 1–4. | Observed for Tasks 6, 7, 8, 10 and 11 (compile failures) and Tasks 12 and 13 (test failures; Task 12's at the 30 s limit). Task 9 is a move and has none. |
| V3 | Each task's green step | The count the step states. | 1, 5, 12, 16 as first written; 17 and 29 after the two review rounds. | Task 6: 5. Task 7: 7 as first written, 8 with the test added. Task 8: 5. Module after Task 8: 47; after Task 9: 47; after Task 10: 49; after Task 11: 62; after Task 12: 63; after Task 13: 66. |
| V4 | `cargo test --features mcp --lib mcp::` at the end of each part | base + 35, then base + 72; `0 failed`. | 415 (the controller's run). | **452 passed**, 0 failed (435 before the first review fold, 449 before the second). |
| V5 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` after each edit to `transport.rs` | The base number, with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`. | Not applicable: S3a-1 does not edit `transport.rs`. | **23 passed** after Task 6, after Task 10, after Task 12 and after Task 13, golden transcripts unblessed. |
| V6 | `cargo check` (default features), `cargo check --features mcp`, `cargo check --tests --features mcp,detached-owner-audit` | `Finished`; no warning from the feature build. | Observed. | Observed; 0 warnings. |
| V7 | `cargo clippy --all-targets --features mcp -- -W clippy::all`, filtered to the touched files | No line that names a file this slice adds or `transport.rs`. | After one correction: only the `session.rs` warning in code the slice does not edit. | No line names `src/mcp/daemon/`, `transport_loop_tests.rs` or `transport.rs` after the fold's one correction (`empty_line_after_doc_comments` in `control.rs`; the S3a-2 fold record). 258 `warning` lines in the whole log, as on the S3a-1 head, where `transport.rs` has none either. |
| V8 | Every "prove the tests bite" step | The named test fails under the named mutant; a mutant that makes a call wait fails at the 30 s hang limit and the run ends. | All 18 mutants of Tasks 1–4 killed (two of them by the controller's edge tests). | All 17 mutants of Tasks 6, 7, 8 and 10 killed, and the two S3a-1 edge-test mutants, again on the code after Task 12 (Task 10's A–C and Task 4's G re-anchored); the five of Task 11 and the two of Task 12 killed; after Task 13 its mutant, Task 10's five and Task 11's five killed again. None survived; none hung. |
| V9 | The 30-run repeat loop of `mcp::daemon`, default threads and `--test-threads=1`, under host load | 30 × `ok`, the same count, `0 failed`. | 30 of 30 in both modes, at 21 and at 30 tests. | `30 test result: ok. 66 passed; 0 failed` and `30 test result: ok. 66 passed; 0 failed`, at load averages of 38.08–42.12 (after the second fold; 63 tests after the first, 49 before it). |
| V10 | The zero-diff checks | `0` for `transport_tests.rs` + `owner_transport_tests.rs`, `0` for the goldens, and only the listed files edited. | Observed. | Observed (the S3a-2 record). |
| V11 | CI on Linux | Green. | The controller's (S3a-1 is in CI). | **Not run.** |

**Still not verified:** the unfiltered `cargo test --features mcp`, `cargo test --doc --features mcp` and the mutation gate for S3a-2 (the controller ran the full suite on S3a-1); anything on Linux; a release build; the two scenarios through a real shim and a real daemon process (S4, S5a). One existing test with a 100 ms wall-clock assertion (`src/mcp/transport_tests.rs:774`) is not this plan's; it passed in every `--lib mcp::` run of the validation at load averages above 30, which is not evidence that it always will.

## Decisions owed before the pull request is opened

No spec issue blocks **building** S3a. Two change what the slice's row in spec §12 delivers, and a reviewer will measure the PR against that row, so they should be confirmed (by the owner, or by the controller if it falls under the standing authority) before the PR is opened:

- **SI-1.** Spec §9 tests 1 and 14 are described as integration tests under `tests/mcp/` with shims and real sockets in a runtime directory. S3a has no accept loop (S4) and no shim (S5a), and test 14 needs a refresh forced to verify `Diverged`, which exists only as a `#[cfg(test)]` method of the library. This plan runs both as in-crate tests over connected Unix socket pairs.
- **SI-2.** Spec §4.7 puts "a test-only status probe over the same control exchange" in S3a. The control exchange is the preamble, whose first line is read by S4's handshake stage. This plan ships the probe's content (`DaemonState::status()`) and leaves its wire verb to S4.

Both are in "Spec issues found" with their options.

Where these stand after S3a-1 merged: both reviewers of S3a-1 accepted SI-2 as planned (the snapshot now, the wire verb in S4). SI-1 is S3a-2's to carry, because the in-crate versions of spec tests 1 and 14 are in Tasks 7 and 8. The owner decided that a call is told to retry when a retry has superseded its failed attempt (D26), and on 2026-10-13 settled the interval: "5 seconds is right without further evidence on min/max/average or any estimate based on lines of code and files". `INDEXING_RETRY_AFTER_SECS` = 5 is the owner's value; a change needs measured build-time evidence.

From the first review of S3a-2, two more for the owner and one for the spec:

- **Owner question (Opus S2): answered 2026-10-13.** A call whose failed attempt has been superseded by an index that is already ready was told `indexing`. The owner: "on the superseded call finding index ready it [w]ould be better to be answered at once". Built as Task 13 (D26 as amended), on the waiter's path and on D32's.
- **Owner or controller, for the spec (D32, SI-8).** Spec §4.2 says "After Failed, the next readiness-requiring call starts exactly one retry"; §5 keeps the in-process server's warming result and retry contract. Task 11 reads §4.2 per connection. The proposed sentence for §4.2 is in SI-8.
- **Settled elsewhere, recorded so no later slice reopens it:** the result cap below the floor is refused, as merged in S2b.
- **For S6 (docs), Opus S6.** `status: "indexing"`, `retry_after_secs` and the `prism/index_state` values are new on the wire, and neither `docs/MCP.md` nor `skills/` documents `warming` or `build_failed` either. Recorded for S6; nothing in this slice.

---

**Goal:** Ship, as tested library code with no accept loop and no shim yet, the two things a shared daemon needs before it can serve a second client: the readiness coordinator (one index build, joined by every client that needs it, each waiting no longer than its own budget) and the provider slot the built index is published into, with one admitted connection served through both. The in-process server's behavior does not change.

**Architecture:** A new Unix-only module `src/mcp/daemon/` holds `DaemonState`: a control plane (`Mutex<ControlState>` plus one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the generation of the provider in the slot, and a count of waiting calls) and a provider slot (`RwLock<Option<SessionProvider>>`, empty until the build thread publishes into it, which it does before announcing `Ready`). `ensure_ready` decides once per call which attempt the call belongs to, waits for that attempt only, up to a per-connection, per-attempt deadline held in `ConnectionBudget`, and is answered from that attempt's own outcome (`AttemptOutcome`, written once and shared by the calls that joined the attempt), whatever the phase has become since. `serve_admitted` serves one connection: S1's `handle_control` for everything that needs no index, `ensure_ready`, then S1's `dispatch_tool_read` under the read lock or `dispatch_tool_refresh` / `dispatch_tool_auto_refresh` under the write lock. The frame loop is extracted from `serve_connection` into `serve_loop` so both servers share it. Time is an injected `Clock`; the build thread is started through an injected `Spawner`.

**Tech Stack:** Rust 2021, `std::sync::{Mutex, Condvar, RwLock}`, `std::os::unix::net::UnixStream`, `serde_json`, `anyhow`; `tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§3.1, §4.2, §4.5, §5, §9 tests 1 and 14 and the unit list, §10, §12 row S3a). Lane plan: `docs/superpowers/plans/2026-10-12-shared-prism-daemon/README.md` ("Notes for the planners of later slices", S3a and S3b). Built on S2b as merged: `main` `c785fa4f`.

**Ships as two pull requests.** **S3a-1 = Tasks 1–5** (the coordinator and the slot): as built and merged, 14 files, about 2,250 added lines with the tests of both review rounds. **S3a-2 = Tasks 6–14** (the shared frame loop, one connection served through the coordinator with spec tests 1 and 14, the split of the control-plane types into their own file, the owner's decision on superseded failures, and the two fixes of its first review): as built, 18 files, 2127 insertions(+), 171 deletions(-) against `main` `e7dc1cc2`. A reviewer can approve the coordinator and reject how a connection is composed on it, or the reverse. S3a-2 changes S3a-1's files in five ways, each its own task or commit: one module line (Task 7), one test-only method (Task 8), the move (Task 9), `ensure_ready`'s answer with the coordinator tests that expect it (Task 10), and the budget's memory of the attempt it joined with the one coordinator test whose expectation that changes (Task 11).

S3a-1 went over the lane's 1,500-line guide because its tests are what hold its claims. S3a-2 is over it too, after Tasks 9 and 10 (added after it was planned) and Tasks 11 and 12 (its review fold); about three quarters of its added lines are tests and test support. If it is wanted smaller, Tasks 9 and 10 are separable from Tasks 6–8 without rework: they touch nothing Tasks 6–8 add except `run_tool_call`'s first statement; Task 11 depends on Task 10, and Task 12 on Task 7 only.

## Global Constraints

- All work is under `--features mcp`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged, byte for byte.** The proof has three parts, checked in Task 5 and Task 14: the golden transcripts under `tests/fixtures/mcp/golden/` are unmodified and pass (never set `PRISM_MCP_BLESS_GOLDEN`); `src/mcp/transport_tests.rs` and `src/mcp/owner_transport_tests.rs` have a zero-line diff against the base; and the only edits to serving files are (a) Task 1's additions to `src/mcp/session.rs` (one accessor nothing in the standalone path calls, and one comment) (b) Task 6's extraction of the frame loop in `src/mcp/transport.rs`, where `serve_connection` passes the unchanged `handle_message_with` to the extracted loop, and (c) Task 10's one added function in `transport.rs`, `retryable_status_response`, which nothing in the standalone path calls. `src/mcp/lazy.rs`, `framing.rs`, `output.rs`, `owner.rs`, `settings.rs`, `src/mcp/shared/*` and `src/bin/prism-mcp.rs` are not touched.
- The in-process server keeps `LazySessionProvider` and the `SessionRuntime` trait unchanged (spec §4.2). Do not add, remove or rename a `SessionRuntime` method.
- Spec values, copied exactly:
  - Build phases: `Idle | Building{attempt, started} | Ready{generation} | Failed{attempt, error}` (§4.2). Their wire words, already in `shared::preamble::BuildState`: `idle`, `building`, `ready`, `failed` (§4.4). In the code `Ready` carries no generation of its own: the generation is `ControlState::generation`, kept apart from the phase because the slot is filled, and can be written to, before `Ready` is announced (D13).
  - A connection's deadline for attempt *n* is set when its first readiness-requiring call arrives during attempt *n* and is cumulative for that attempt; a warming response is a wait outcome for that connection, not a state transition, and cancels nothing; when attempt *n* ends its outcome is delivered to every connection waiting on it (by the owner's later decision, D26: a failure is reported to the calls that waited on it unless a retry has already superseded it, in which case they are told to retry); after `Failed`, the next readiness-requiring call from any connection starts exactly one retry (attempt *n+1*) with fresh per-connection budgets (§4.2).
  - The worked example: A and B join attempt 1 with budgets 20 s and 60 s; A gets warming at 20 s; B keeps waiting; publication at 35 s serves B directly and A's next call immediately (§4.2).
  - The build publishes the provider into the slot **before** announcing `Ready` (§4.2).
  - Lock order: a thread holding the control lock never waits on the provider; a thread holding the provider may briefly take the control lock (§4.2).
  - A first-call wait is at most `FIRST_CALL_WAIT_MAX` = 600 s and arrives only as a `ClientSettings` (S2b).
  - The index builds on the first valid `tools/call` (`StartupMode::Lazy`) or at daemon start (`StartupMode::Background`); `--eager` is not a shared mode (§3.1, §4.1 step 2).
  - Invalid input on a connection gets the same `-32700` / `-32600` responses as stdio and the connection stays (§10).
  - The frame limit stays 1 MiB (`MAX_REQUEST_BYTES`); `framing.rs` is not edited.
- **No panic on a path a client can reach.** No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests in the new module; a poisoned lock is a typed error (`Poisoned`), never unwrapped. S3a has **no panic boundary**: where a panic can still happen and what it does today is stated in "Panics in S3a" below; the boundaries and the abort are S3b.
- Everything under `src/mcp/daemon/` is `#[cfg(unix)]` through the one `mod` line in `src/mcp/mod.rs`.
- Keep every new file under 600 lines. A module's tests are in sibling `*_tests.rs` files included with `#[path]`. `transport.rs` (1,006 lines) and `session.rs` (927) are already over; this slice adds 21 and 7 lines to them and puts every new test in a new file, except one test appended to the existing `session_stage_tests.rs`.
- **Tests do not depend on the host's speed.** Budgets are moved with a manual clock. A test waits for a condition (a count in `status()`, a response on a socket) up to `HANG` = 30 s and asserts no upper bound on any elapsed time; the one real-clock test asserts a lower bound only. No test sleeps to order two events.
- No new clippy warning in any file this slice adds or edits (the base prints about 259 warning lines; CI does not deny warnings).
- **Commit before any experiment.** The "prove the tests bite" steps edit a source file and restore it with `git checkout -- <file>`; that discards uncommitted work in the file. They are ordered after the task's commit for that reason.
- Logs go to files; read the file. Do not pipe cargo output into `grep` or `tail`.

### What each mechanism covers, and what it does not

- **The first-call budget bounds how long a call waits for the build.** It does not bound how long a query then waits for the provider lock (a query that arrives during another client's `refresh_index` waits for that refresh, as long as it takes) and it does not bound the query itself.
- **A warming answer ends a call's wait, not the build.** Nothing a client does cancels a build or shortens another client's wait; a client that disconnects leaves the build running and its result is kept for the next client.
- **A disconnected client's parked call is not noticed until it wakes.** It wakes when the attempt ends or its own deadline passes (at most 600 s), then fails to write and ends. Its worker thread and, from S4 on, its admitted slot are held that long.
- **`status()` reports the control plane.** Its generation is the provider's as of the last completed exclusive access; during a refresh it is the generation still being served.
- **"Exactly one retry" is exclusion, not a limit.** One retry starts per failed attempt, by the next call. There is no cap on how many attempts a persistently failing build gets and no pause between them (SI-3).

### Panics in S3a (the boundaries are S3b)

| Where | What happens in S3a | What S3b makes of it |
|---|---|---|
| The builder panics on the build thread | The attempt ends `Failed` with the text the in-process server uses, `index build panicked`; every waiter is told; the next call retries. | Abort (spec §3.1). |
| A tool handler panics under the read lock | The calling connection's thread unwinds and its socket closes. `std`'s `RwLock` is not poisoned by a reader, so other connections go on. | Abort. |
| `refresh` or a handler panics under the write lock | The slot is poisoned. Every later tool call on every connection ends its connection with `Poisoned`; handshakes are still answered. | Abort. |
| A thread panics holding the control lock | The control lock is poisoned. Every later `ensure_ready` and `status` returns `Poisoned`; parked calls return `Poisoned` when they wake (each is bounded by its deadline). S3a's own code under that lock is field assignments, checked and saturating arithmetic, one `format!`, string clones, filling an attempt's outcome, `Clock::now` and the `Spawner` call. | Abort. |
| The build thread cannot be started | Not a panic: `std::thread::Builder::spawn` returns an error and the attempt ends `Failed`. | Unchanged. |
| The slot is poisoned when a build is to be published | The attempt ends `Failed`. This cannot happen outside a test: the slot is poisoned only by a panic under its write guard, which needs a provider in it, after which nothing is published again. If it could, every call would start a build that cannot be published. | Abort covers it. |

Nothing ships between S3a and S3b (there is no daemon binary until S4), so no user meets the left column.

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously. The reason is one line; "cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D1 | The daemon core is a new private module `src/mcp/daemon/` (`mod daemon;` in `src/mcp/mod.rs`), not part of `src/mcp/shared/`. Its items are `pub(super)`. | `shared/`'s own doc says nothing there takes a lock or starts a thread. S3a's tests are in-crate, so nothing outside `crate::mcp` needs these names yet. | A visibility edit when S4 adds the public entry point. |
| D2 | The module carries `#![cfg_attr(not(test), allow(dead_code))]`, to be removed in S4. | Nothing calls the coordinator in production until the accept loop exists. | One line. |
| D3 | The control plane is one `Mutex<ControlState>` and one `Condvar`, as spec §4.2 words it; calls park on that condition variable. **Revised by the first review of S3a-1:** each attempt has an outcome (`AttemptOutcome`, a `OnceLock`) that is created with the attempt, shared by `Arc` with every call that starts or joins it, and filled once, under the control mutex, when the attempt ends. A call reads only its own attempt's outcome. As first written the plan answered a waiter from the current phase and one remembered error, so a waiter that ran late was told another attempt's outcome. | The spec's shape, and S4 needs the build phase and its own counters under one lock for the retirement decision. The outcome per attempt is what makes "delivered to every connection waiting on it" true for any number of retries; pairing the remembered error with an attempt number would not have kept the outcome of an attempt two retries back. | One allocation per attempt. The outcome is dropped when the last call that joined the attempt returns; nothing is kept per attempt in the control plane. (Superseded by D32: each connection's budget keeps the outcome of the attempt it last joined until it joins another or the connection ends; still nothing per attempt in the control plane.) |
| D4 | `ensure_ready` picks its attempt once, then only waits for that attempt. A call that is told `Failed` never starts the retry itself. | Spec: "the next readiness-requiring call … starts exactly one retry". A call that restarted on failure could loop for as long as builds keep failing. | A loop edit; Task 3's failed-attempt test pins it. |
| D5 | "Single retry" (spec §12) is read as: one retry per failed attempt, started by the next call. No cap, no back-off. | It is the in-process server's contract ("the next tool call retries the build"), which spec §5 preserves. | SI-3. A cap or a minimum interval is a few lines in `start_attempt`. |
| D6 | A budget is a `ConnectionBudget`, constructible only from a `ClientSettings`; it holds one deadline keyed by attempt number. | `ClientSettings` already bounds the wait at 600 s (S2b), so the bound is carried by the type. The key by attempt is what makes the budget "per attempt". | None foreseen. |
| D7 | If `now + wait` cannot be represented, the deadline is `now` (no wait). | It cannot panic. No `ClientSettings` can reach it; no portable test can either, and the branch is disclosed as untested. | One expression. |
| D8 | Time is an injected `Clock` (`SystemClock` in production). Tests move a `ManualClock` and call `DaemonState::wake()`. | Spec §9 asks for deterministic constructions; S4's idle-exit test needs an injected clock anyway. | S4 can keep or replace the 20-line trait. |
| D9 | The build thread is started through an injected `Spawner`; a spawn error is a `Failed` attempt. | `std::thread::spawn` panics when the thread cannot be created, and the first tool call of any client reaches that line. Injection makes the error branch testable for real. | Fifteen lines. |
| D10 | A builder panic is a `Failed` attempt with the text `index build panicked` (the in-process server's), delivered through a drop guard on the build thread. | Without it a panic would leave the phase `Building` forever and every call would get warming forever. The abort of spec §3.1 is S3b's. | S3b replaces the guard's body with the abort. |
| D11 | A poisoned lock is the typed error `Poisoned`; the connection whose call met it ends with an error. Nothing is read through a poisoned lock (`into_inner` is never used). | Spec §4.2: "no lock is ever unwrapped past poisoning". Ending the process is S3b's boundary. | S3b turns the error into the abort where it is raised. |
| D12 | An attempt's ending is recorded, in its outcome and in the phase, only while the phase is still `Building` that attempt; the outcome itself cannot be written twice. `Ready` is therefore never left by the coordinator. | A stale `Failed` after `Ready` would let the next call rebuild and replace a provider that has since been refreshed. | None foreseen. |
| D13 | The generation of the provider in the slot is a field of the control plane, `ControlState::generation`, written by one function that takes the slot's write guard as an argument: when the provider is stored, and before every exclusive access ends. It does not depend on the phase, and `BuildPhase::Ready` carries no copy of it. **Revised by the first review of S3a-1:** as first written the generation lived in `Ready { generation }` and was recorded only once the phase was `Ready`, so exclusive access between publication and the announcement left a stale generation announced. | S4 writes `READY` from the control plane and must not wait on the provider to learn the generation. Requiring the guard makes "recorded in the order the slot changed" a matter of types; one copy cannot disagree with itself. The reviewers' other fix, holding the slot's write guard through the announcement, would run the publication hook under a lock. | The spec says a thread holding the provider may take the control lock "to read state"; this writes one field (SI-4). `status().generation` is the slot's also while the phase is still `Building`, in the interval after publication. |
| D14 | **Where S3a stops and S3b starts in dispatch.** S3a: queries under the read lock, `refresh_index` and automatic refresh under the write lock, composed from S1's three dispatch functions unchanged. S3b: the re-check after acquiring exclusive access, responses bound to the generation queried, the query outside the write lock, the many-clients-and-a-blocked-writer proof, and panic boundaries. | Spec test 14 is S3a's and needs both refresh paths through the daemon. The rest is S3b's by its row. | Under an automatic policy two clients that both find the index stale each run a refresh in S3a; the second is redundant, not wrong. Nothing ships in between. |
| D15 | The status probe in S3a is `DaemonState::status() -> BuildStatus` (state, attempt, generation, waiting calls), called in process. No wire verb. | SI-2: S4 owns the first line of a connection and the fields a wire reply would need beside these (pid, start time, admitted, pending). | S4 adds the verb, or a follow-up adds a codec here. |
| D16 | `status()` reports `waiting`, the number of calls parked on the build, which spec §4.7's list does not have. | It is how a test knows a call has arrived and parked without sleeping, and how "no waiter is left behind" is asserted. | Drop the field from the wire reply in S4 if it is not wanted there. |
| D17 | Spec tests 1 and 14 run in-crate, over `UnixStream::pair()`, with raw MCP clients standing in for shims. | SI-1. | S5a re-runs them through real shims with the transcript driver, which the lane plan already asks of it. |
| D18 | `serve_loop` takes a handler of a whole message, `FnMut(&Value, &mut Lifecycle) -> anyhow::Result<Option<Value>>`. The lane plan's S3a note sketched a handler of a validated tool call instead. | `serve_connection` then passes the unchanged `handle_message_with`, so the standalone path runs the same calls in the same order and does not depend on `startup_mode()` being constant; and the daemon's handler can fail (a poisoned lock ends the connection). | The daemon's handler repeats a three-arm `match` on `handle_control`'s result; a connection that skipped `handle_control` would be visible in a ten-line function. Changing the seam is about twenty lines. |
| D19 | `Readiness` and `SessionBuilder` are reused from `src/mcp/lazy.rs`; `lazy.rs` is not edited. | `readiness_response` already takes `Readiness`, so the warming and failure results are the in-process server's by construction. | None. |
| D20 | `SessionProvider::generation()` is `pub`, beside `session()`, `freshness()` and `refresh_policy()`. | The lane plan asks for the accessor (S3b note). `pub` cannot raise a dead-code warning on any target. | A visibility edit. `SessionProvider` is not part of the `prism::api` facade. |
| D21 | The tool registry is an argument of `serve_admitted`, as it is of `serve_connection`; `DaemonState` does not own one. | S4 owns the accept loop and can hold one registry for all workers; the coordinator has no use for it. | A field. |
| D22 | `Ready` announced with an empty slot (which D12 and the publication order rule out) ends the connection with an error instead of an `expect`. | A panic in dispatch is fatal to every client once S3b lands. | S3b may make it the abort. |
| D23 | A call parked for a client that has disconnected is left parked until its attempt ends or its deadline passes. | Noticing sooner needs the wait to poll the socket, and S4 owns the stream's mode. | SI-5. |
| D24 | The warming answer's `elapsed` is measured from the start of the attempt on the daemon's clock, for every client. | It is what the in-process server reports (time since the build started), and it is the same number for everyone waiting on one build. | One expression. |
| D26 | **The owner's decision (2026-10-13), replacing this plan's first D26.** A failed attempt is reported to the calls that waited on it unless a retry has already superseded it; then they are told to retry. Precisely: a call whose own attempt failed is answered `build_failed`, with that attempt's own text, while the phase is still a failure (its own, or a later one with nothing building); if a later attempt is building or the index is ready when the call runs again, it is answered with a retryable `indexing` status ("retry in about N seconds; if you already retried and that call succeeded, ignore this message") and is given no failure text. Built in Task 10. **Amended by the owner on 2026-10-13:** "on the superseded call finding index ready it [w]ould be better to be answered at once". A call whose own attempt failed and which finds the index ready is served, on the waiter's path and on D32's; `indexing` remains for a later attempt that is building. Built in Task 13. | The owner does not want a call told of a stale failure ("Building...retry in <retry interval seconds> or ignore if you retried and call succeeded"; "could use indexing in place of building; this is what IDEs do"). The first D26 gave such a call `build_failed` for an index that was in fact ready. | The status is one the in-process server never sends, so a client of a shared daemon can see a status a standalone client cannot. `prism/index_state` is `warming` so that clients keyed on it treat it as warming. The interval, `INDEXING_RETRY_AFTER_SECS` = 5, is the owner's value (2026-10-13; it was the controller's placeholder until then); a change needs measured build-time evidence. A failure whose waiters are all superseded is reported to nobody; the next section says where else that already happens. |
| D27 | Attempt numbers are never reused. When the counter cannot go on (`u64::MAX` attempts), no build is started and the call is told `the index build attempt counter is exhausted`; so is every later call. | A connection's budget is kept by attempt number; a number used twice would hand a later attempt an earlier one's deadline. | Unreachable in any process lifetime; the branch exists so that the arithmetic is checked, and a test drives it. |
| D28 | The build thread is named `prism-idx-build`. | Linux keeps 15 bytes of a thread name. | A constant. |
| D29 | `ensure_ready` returns the daemon's own `Answer { Ready, Warming, Failed, Superseded }`; `lazy.rs`'s `Readiness` is unchanged and the connection maps the first three onto it. A call reads its own attempt's outcome first; only when that is a failure does it look at the phase, once, under the control lock it holds, and only to learn whether the failure has been superseded. | The owner's decision needs a fourth answer and `Readiness` is the in-process server's type. The one look at the phase cannot bring back the first review's defect: it returns no outcome and no text of another attempt, only "superseded". | Task 10's mutants B and C are the two ways to get it wrong (every failed waiter told to retry; the latest failure's text), and each fails named tests. |
| D30 | The `indexing` result's envelope is built by one function added to `transport.rs`, `retryable_status_response`, from the same private helpers as `warming` and `build_failed`; its status text and constants live in `daemon/connection.rs`. | `ToolCall`'s id and the envelope helpers are private to `transport.rs`; building the envelope elsewhere would copy them. | `transport.rs` gains 17 lines that the standalone path never calls; the golden transcripts and the transport test corpus are unchanged. |
| D31 | The control-plane types (`AttemptOutcome`, `BuildPhase`, `ControlState`, `BuildStatus`, and `Answer`) are in `daemon/control.rs`; `coordinator.rs` holds the lock and re-exports them. Task 9 is the move alone. | Both reviewers of S3a-1 asked for the split before `coordinator.rs` (540 of its 600 lines) was extended. | None. `record_generation` still takes the generation from its caller; a reviewer asked that it read it from the guard it is handed when the file is next changed there. Not done: it is a change of behavior in an emptied slot, outside a pure move. **`control` stays a sibling of `coordinator`** (the first review of S3a-2, Opus S5, suggested a child module so that `AttemptOutcome::fill` and `ended` are visible to the coordinator only): `budget.rs` now reads `AttemptOutcome::ended` (Task 11), and `connection.rs` and the tests name `Answer` and `BuildPhase` through `coordinator`'s re-export, which a private child module would need widened item by item. The widening is to the `daemon` module, whose only other files are the connection, the budget and tests, and nothing outside `coordinator.rs` can reach an `AttemptOutcome` that a call did not join. |
| D32 | **A connection's next call first hears its own attempt's unheard failure** (the first review of S3a-2, Opus F1; decided by the controller under the owner's D26 principle). `ConnectionBudget` keeps the outcome of the attempt the connection last joined and whether a call of the connection was told how it ended. Before the shortcut to `Ready` and before an attempt is started or joined, a failure no call of the connection was told of is answered by the D26 rule (`answer_for`): `Failed` with its own text when nothing has superseded it, `Superseded` when a later attempt is building or the index is ready. That call starts nothing; the connection is then told. Since the owner's amendment of D26 (Task 13), a failure superseded by a ready index is answered `Ready` on this path too and the call is served. An attempt that ended ready needs no such step. Built in Task 11. | The in-process server answers a client that stopped waiting `warming`, then `build_failed` with the cause, then retries on the call after. Without D32 the daemon retried on the call after the failure: the cause reached no client whose wait was shorter than the build, and a slow failing build was rebuilt on every call. Spec §4.2 and §5 disagree on this case (SI-8). | A client whose wait is shorter than the build gets one more answer (the failure) before the retry starts, as in process. The budget keeps one `Arc<AttemptOutcome>` (an error string at most) per connection until the connection joins another attempt or ends. |
| D33 | `serve_admitted` sets the stream blocking with no read timeout before serving it. Built in Task 12. | S4's handshake stage reads the preamble from a non-blocking socket and may set a timeout; a stream left that way ends the connection as a read error at the first quiet moment (the first review of S3a-2, Opus S4). | Two system calls per connection. |
| D25 | Two test seams on the build thread, compiled only under `#[cfg(test)]` and called with no lock held: after publication and before the announcement; after an outcome is recorded and before waiters are notified. | They are the two steps where another actor can interleave ("publication before Ready", "a waiter that wakes late"). | Removed with their tests. |

## Spec issues found (not resolved by this plan)

- **SI-1 (§9 tests 1 and 14, §12 row S3a; needs confirming).** The tests are specified with shims, real sockets under a temporary `PRISM_MCP_RUNTIME_DIR`, and `tests/mcp/`. At S3a there is no listener and no shim, and `tests/mcp/` links the library without `cfg(test)`, so it cannot reach `SessionProvider::force_next_verification_for_tests` (`src/mcp/session.rs:203`), the only deterministic way to make a refresh verify `Diverged`. Options: (a) in-crate tests over socket pairs now, and the same two scenarios again through real shims in S5a (this plan, D17); (b) move tests 1 and 14 to S5a and let S3a ship with unit tests only; (c) add a production-reachable test hook (an environment variable that forces `Diverged`, a builder barrier in a file) so that process-level tests can exist from S4 on. (c) will be needed for process-level versions of tests 1, 8 and 14 whenever they are written; it is not needed to build S3a.
- **SI-2 (§4.7, §12 row S3a; needs confirming).** "The integration tests use a test-only status probe over the same control exchange (S3a)". The control exchange is the preamble; S2b's `Hello::decode` accepts only `HELLO`, and the code that reads a connection's first line is S4's bounded handshake stage. A wire probe written now would guess S4's reply fields, and by S2's own rule a new verb is a wire change that bumps `SHARED_PROTOCOL_VERSION` (the lane plan records that question as open until S6). Options: (a) S3a ships the snapshot the probe reports, S4 adds the verb with its own fields (this plan, D15); (b) S3a adds a `STATUS` request and reply codec to `src/mcp/shared/` with only the build fields, and S4 extends it. "Test-only" also needs a meaning for a binary that integration tests start as a child: a hidden verb that is always compiled in, or one behind an environment variable. That is S4's to decide with SI-1(c).
- **SI-3 (§4.2, §12 "single retry").** The text gives each failed attempt exactly one retry, started by the next call, and says nothing of a limit or a pause. A build that fails after thirty seconds of work is then started again by the next call of any of N clients, for as long as they keep calling. That is today's behavior for one client. Options: as written (this plan, D5); a minimum interval between attempts; a cap after which `Failed` is sticky until a refresh or a restart.
- **SI-4 (§4.2, wording).** "A thread holding the provider may briefly take Control to read state." Keeping the control plane's generation true after a refresh, and at publication, needs a write to the control plane while the provider is held. The lock order is the permitted one; the word "read" is narrower than what the design needs. This plan writes the one field (D13).
- **SI-5 (§4.6 "Client exits: connection drops; admitted count decrements").** With D23 the decrement can lag the disconnect by up to the client's first-call wait (at most 600 s) when the client leaves while its call is parked on a build. The idle TTL is 900 s, so retirement is delayed, not prevented. Options: accept; have the parked call poll its socket for a hang-up; cap the wait of a parked call independently of the client's setting.
- **SI-6 (§3.1, §4.2 against §5).** Spec §3.1 makes a panic on the build thread daemon-fatal; spec §5 preserves "the warming result and retry contract", under which the in-process server reports a builder panic as a failed build and retries. S3a implements the second and S3b the first. The two readings differ only once S3b lands; recorded so S3b's plan decides it on purpose.
- **SI-8 (§4.2 against §5; proposed wording for the controller to apply).** §4.2: "After Failed, the next readiness-requiring call … starts exactly one retry". §5: the in-process server's warming result and retry contract is preserved, and there a client that stopped waiting hears the failure on its next call and only the call after that retries. D32 reads §4.2 per connection. Proposed sentence for §4.2, after "starts exactly one retry": "A connection's next call first hears its own attempt's unheard outcome: a call from a connection that was answered warming for an attempt that has since failed is answered with that failure (or told to retry, when the failure has been superseded) and starts nothing; the retry is started by the next call from a connection with no unheard failure." (The clause first proposed here, "by a call that has heard it", was wrong: a connection that never joined the failed attempt starts the retry too; both reviewers of round 2.) Under `Background` startup the sentence does not yet hold for the startup attempt, which no connection joins (S4's requirement under "Interfaces for the next slices").
- **SI-7 (§4.2, cost of the read path).** The lane plan notes that the first query on a fresh snapshot builds the call-edge index and writes its cache sidecar. In the daemon that happens under the read lock: other readers wait on that `OnceLock`, not on the provider, but a `refresh_index` requested meanwhile waits for the disk write. Not pursued.

## Review Focus

Six ways this code is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code.

1. **A call that is still parked when the retry starts, or ends.** Attempt 1 fails; before its waiter runs again, another client's call starts attempt 2, which may still be building, may have succeeded, may have failed for another reason, or may itself have been followed by attempt 3. The waiter must never be given another attempt's outcome or text, must not start an attempt, and must not join one with the budget it had for attempt 1. By the owner's decision (D26, as amended) it is told to retry when a later attempt is building, is served when the index is ready, and is told of attempt 1's failure, with attempt 1's text, when every later attempt has failed too. → Task 4, Task 10 and Task 13: `a_waiter_whose_failed_attempt_was_superseded_by_a_building_retry_is_told_to_retry`, `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served` (Task 13), `a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_failed_differently`, `waiters_of_failed_attempts_are_told_to_retry_while_a_third_attempt_is_building`, `waiters_of_failed_attempts_hear_their_own_failure_when_every_later_attempt_failed_too`; over a socket, `a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served` (Task 13). Each plays the other clients between the recording of a failure and its notification. The first review of S3a-1 found three of these cases wrong in the plan as first written.
2. **A client that disconnects while its call is parked.** The build must go on and be kept, no waiter may remain counted, and the next client must be served from that build, not a second one. → Task 7, `a_client_that_leaves_while_its_call_waits_leaves_no_waiter_and_the_build_is_kept`, `a_departed_client_s_call_is_parked_no_longer_than_its_budget`.
3. **A build that cannot start, or dies.** No thread can be created; or the builder panics. Either way every parked call gets an answer and the next call can retry. → Task 3, `a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries`, `a_builder_panic_is_a_failed_attempt_every_waiter_hears_of`.
4. **A lock poisoned while calls are parked.** They must return an error when they wake, not hang and not panic; later calls and `status` the same. → Task 3, `a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang`; Task 4, `a_poisoned_provider_slot_fails_the_attempt_and_is_an_error_for_every_reader`; Task 8, `a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on`.
5. **A whole session, not one call.** Garbage frames, calls before `initialize`, an unknown tool, a bad argument, an edit that makes the index stale, a refresh, a re-query: a daemon connection must answer every one with the bytes the in-process server writes, under a warn-only and an automatic policy. → Task 8, `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server`.
6. **A client that stopped waiting before its build failed.** It was told `warming`; the build fails; it calls again. It must hear that failure, with its cause, exactly once and before anything is started, as the in-process server's client does; if another client has meanwhile started a retry or the index is ready, it is told to retry instead; it never hears a later attempt's text; an attempt that published while it was not waiting serves it. → Task 11: the parity leg `a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server`, `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile`, and the four coordinator tests named `a_connection_…`. The first review of S3a-2 found this case wrong.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or comments state, with the test that fails when it breaks and one wrong implementation that test rules out. The mutant in the last column was applied in validation and the test failed, with two exceptions marked in the table. Tests named for Task 4 that Task 10 renames or re-expects are listed under Task 10.

| Claim | Test | A mutant it kills |
|---|---|---|
| Nothing is built before the first readiness-requiring call. | T3 `nothing_is_built_until_the_first_readiness_requiring_call`; T7 `two_clients_share_one_build` (status is idle after two handshakes) | start the build in `from_parts` for `Lazy` |
| A connection's deadline for an attempt is set once and kept. | T2 `the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it`; T3 `later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget`; T7 `a_connection_s_budget_spans_its_calls_within_one_attempt` | a fresh deadline per call; a `ConnectionBudget` made per message |
| Each connection has its own deadline; one client's warming changes nothing for another. | T2 `each_connection_has_its_own_deadline_for_the_same_attempt`; T3 `the_spec_s_worked_example_two_budgets_on_one_attempt` | one deadline per attempt shared by all waiters (not applied in validation: it is not a one-line change) |
| A new attempt gives every connection a fresh budget. | T2 `a_new_attempt_gets_a_fresh_full_budget`; T3 `a_retry_gives_a_connection_a_fresh_full_budget` | keep the first deadline for the connection's life |
| An attempt's outcome reaches every call waiting on it. | T3 `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself`; T4 the four late-waiter tests | `notify_one` instead of `notify_all` |
| A call is never given another attempt's outcome or error. | T10 `a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_failed_differently`, `waiters_of_failed_attempts_hear_their_own_failure_when_every_later_attempt_failed_too`, `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served` (served from the index retry 2 published; it is never given retry 2's outcome as its own) | report a failure with the latest attempt's text (Task 10 C); before Task 10, answer from the current phase (Task 4 B) |
| A failure is reported to the calls that waited on it unless a retry has superseded it; then they are told to retry while it builds, and served once it has published. | T10 `a_waiter_whose_failed_attempt_was_superseded_by_a_building_retry_is_told_to_retry`, `waiters_of_failed_attempts_are_told_to_retry_while_a_third_attempt_is_building`; T13 `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served`, `a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served`, `a_connection_that_stopped_waiting_is_served_once_another_s_retry_has_published`, `a_client_that_stopped_waiting_is_served_once_another_client_s_retry_has_published`, the two rule tests; and for "unless", T3 `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself` | still report a superseded failure (Task 10 A); tell every waiter of a failed attempt to retry (Task 10 B); tell a call to retry when the index is ready (Task 13 A) |
| The `indexing` answer is retryable, carries the index state `warming`, and carries no failure text. | T11/T13 `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile` (the envelope checks moved there from the Task 10 socket test when that case became a served call) | answer a superseded call as warming (Task 10 D); another index state (Task 10 E) |
| A connection's next call hears its own attempt's failure, which no call of the connection was told of, before anything is started, and hears it once. | T11 `a_connection_that_stopped_waiting_hears_its_attempt_s_failure_before_any_retry`, `a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server` (byte parity), `a_failure_no_call_was_told_of_is_reported_once`; T3 `a_retry_gives_a_connection_a_fresh_full_budget` | the code before Task 11 (Task 11 A); mark it told on joining (B); report it on every call (D); not mark a waited-for ending told (E) |
| That call is answered from its own outcome only: told to retry when superseded, its own text otherwise. | T11 `a_connection_that_stopped_waiting_is_told_to_retry_when_another_retried_meanwhile`, `a_connection_hears_its_own_attempt_s_failure_although_a_later_attempt_failed_too`, and over sockets `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile` | answer from the latest attempt's text (Task 11 C) |
| An attempt that published while the connection was not waiting serves the connection's next call. | T11 `a_connection_whose_attempt_published_while_it_was_not_waiting_is_served`, `an_attempt_that_published_the_index_is_never_an_unheard_failure` | none applied: it guards against a D32 that over-reaches (reports any ending, or treats `Ok` as unheard) |
| The D26 rule holds in every phase, with no threads. | T11 the three table tests in `control_tests.rs` | Task 10 A, B and C, on `answer_for` |
| An admitted connection is served blocking and with no read timeout, whatever mode it was handed in. | T12 `a_connection_handed_over_non_blocking_and_with_a_read_timeout_is_served_blocking` | leave the stream non-blocking (Task 12 A); leave the read timeout (Task 12 B) |
| Nothing is served from the slot before `ensure_ready` has answered ready. | T7 `a_call_is_not_served_from_the_slot_before_ready_is_announced` | serve from whatever is in the slot (Task 7 E) |
| An attempt's outcome is written once. | T3 `an_attempt_s_outcome_keeps_the_first_ending_it_is_given` | none: a `OnceLock` cannot be written twice |
| A connection's deadline is counted from its own first call during the attempt. | T3 `a_connection_that_joins_a_running_build_gets_its_whole_budget` | count the deadline from the attempt's start |
| A call reads its attempt's outcome before it looks at its deadline, so an ending is heard by a call whose budget has just run out, and by a client that waits zero seconds. | T4 `a_budget_that_ends_as_the_attempt_publishes_is_answered_ready`, `a_budget_that_ends_as_the_attempt_fails_is_answered_failed`, `a_zero_wait_client_hears_a_failure_that_is_already_recorded` | check the deadline before the outcome |
| A provider's generation is recorded when it is stored, whatever it is. | T4 `the_generation_of_a_published_provider_is_recorded_at_publication` | no record at publication |
| An attempt number is used once. | T3 `an_exhausted_attempt_counter_is_a_failure_that_starts_no_build` | a saturating counter |
| The generation the control plane reports is the slot's, before `Ready` is announced too. | T4 `exclusive_access_between_publication_and_ready_keeps_the_generation_current` | record the generation only once the phase is `Ready`; announce the generation read before publication |
| A call told `Failed` does not retry by itself. | same T3 test (the builder is called once) | restart from the wait loop when the attempt failed |
| After `Failed`, exactly one retry starts however many calls arrive. | T3 `the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together` | start an attempt from `Building` too |
| The provider is in the slot before `Ready` is announced. | T4 `the_index_is_in_the_slot_before_ready_is_announced` | announce first, publish second |
| An attempt ends once; `Ready` is never left. | T3 `an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left` | record without checking which attempt is building |
| A call never waits past its deadline, also when nobody wakes it. | T3 `on_the_real_clock_a_call_returns_warming_when_its_budget_ends_unprompted` | wait without a timeout |
| No waiter stays counted after its call returns. | T3 (every test that ends with `waiting: 0`); T7 the two departed-client tests | drop the decrement |
| A build thread that cannot start is a failed attempt, not a panic, at construction of a warm daemon too. | T3 `a_build_thread_that_cannot_be_started_…`, `a_warm_daemon_whose_build_thread_cannot_be_started_is_built_by_the_first_call` | ignore the spawner's error |
| A poisoned lock is an error to the caller, never a panic, never a hang. | Review Focus 4's three tests | `unwrap` the lock; `into_inner` |
| Exclusive access ends with the provider's generation in the control plane. | T4 `exclusive_access_records_the_provider_s_generation_in_the_control_plane`; T8 the two sticky-evidence tests | drop the copy |
| The frame loop answers a frame that is not JSON itself and keeps the connection. | T6 `a_frame_that_is_not_a_json_value_is_answered_by_the_loop_and_never_reaches_the_handler` | hand malformed frames to the handler; end the loop on them |
| One lifecycle state lasts the whole connection. | T6 `one_lifecycle_state_lasts_the_whole_connection_and_starts_before_initialize` | a new `Lifecycle` per frame |
| A handler error ends the loop. | T6 `a_handler_error_ends_the_loop_and_nothing_after_it_is_read_or_answered` | log and continue |
| A connection's output settings are its own. | T7 `two_clients_share_one_build` (one client gets `structuredContent`, the other does not) | `ConnectionSettings::from_env()` in the daemon |
| The handshake and listing need neither the build nor the provider. | T7 `the_handshake_listing_and_ping_are_answered_while_the_build_is_blocked`; T8 the poisoned-slot test | call `ensure_ready` before `handle_control` |
| Sticky stale evidence outranks a clean stamp probe for every client. | T8 `sticky_stale_evidence_reaches_another_client_although_the_stamp_probe_is_clean`, `under_an_automatic_policy_sticky_stale_evidence_makes_the_next_client_s_query_refresh_again` | drop the `known_stale_after_refresh` precedence in `effective_stale_report` |
| `Ready` with nothing published is an error, not a panic. | T8 `ready_with_nothing_published_ends_the_connection_with_an_error_not_a_panic` | `expect` in `published` |

Stated and **not** held by a test, or held only under an assumption: two tests hold waiters behind withheld notifications (`waiters_of_failed_attempts_are_told_to_retry_while_a_third_attempt_is_building`, and over sockets `a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served`) and assume no spurious wake-up in that short window; a spurious wake-up would make them fail loudly, never pass wrongly, and the rule they exercise is held without threads by Task 11's table tests (both reviewers of S3a-2, Opus S3 and sol S1). `wake()`'s claim that a call which has read the old time and not yet parked is woken too (it holds because `wake` takes the control lock before notifying; no test can place a call in that interval deterministically); that an injected `Clock` makes progress and an injected `Spawner` returns promptly and does not run the job inline (contracts on the two types, stated in their docs; production uses `SystemClock` and `std::thread`); D7's overflow branch (no `ClientSettings` reaches it); that no S3a code panics while holding the control lock (by inspection of the five things it does there); that a response is written with no lock held (by construction: `run_tool_call` returns an owned `Value`, so no guard can outlive it; the blocked-writer test is S3b's, spec §9 test 7).

## File Structure

| File | Responsibility | Change | PR |
|---|---|---|---|
| `src/mcp/session.rs` | adds `SessionProvider::generation()`; corrects the comment above `build_state` | Modify (Task 1) | S3a-1 |
| `src/mcp/session_stage_tests.rs` | one test for the accessor | Modify (Task 1): append | S3a-1 |
| `src/mcp/mod.rs` | Module list | Modify (Task 2): `#[cfg(unix)] mod daemon;` | S3a-1 |
| `src/mcp/daemon/mod.rs` | Module list of the daemon core; compile-time `Send + Sync` assertions | Create (Task 2), replace (Task 3), extend (Task 7) | both |
| `src/mcp/daemon/budget.rs` | `ConnectionBudget`; from Task 11, the outcome of the attempt it last joined | Create (Task 2), modify (Task 11) | both |
| `src/mcp/daemon/budget_tests.rs` | Its tests | Create (Task 2), replace (Task 11) | both |
| `src/mcp/daemon/clock.rs` | `Clock`, `SystemClock` | Create (Task 3) | S3a-1 |
| `src/mcp/daemon/coordinator.rs` | `DaemonState`, `ControlState`, `BuildPhase`, `BuildStatus`, `Poisoned`, `Parts`, `Spawner`, test seams | Create (Task 3), extend (Task 4: `write`; Task 8: one test-only method) | both |
| `src/mcp/daemon/coordinator_tests.rs` | States and budgets | Create (Task 3), modify (Task 11: one expectation) | both |
| `src/mcp/daemon/coordinator_failure_tests.rs` | Builds that cannot start or die; poisoning; write-once outcome | Create (Task 3) | S3a-1 |
| `src/mcp/daemon/coordinator_order_tests.rs` | Publication before `Ready`; the late waiter; the generation copy | Create (Task 4), modify (Tasks 10, 13) | both |
| `src/mcp/daemon/coordinator_edge_tests.rs` | Edges the second review round found untested: the third-attempt waiter, a published generation other than 0, a budget that ends as its attempt ends, a zero-wait client; from Task 11, a connection that stopped waiting | Create (Task 4), modify (Tasks 10, 11, 13) | both |
| `src/mcp/daemon/control.rs` | `AttemptOutcome`, `BuildPhase`, `ControlState`, `BuildStatus`; from Task 10, `Answer`; from Task 11, the rule `answer_for` | Create (Task 9, moved out of `coordinator.rs`), modify (Tasks 10, 11, 13) | S3a-2 |
| `src/mcp/daemon/control_tests.rs` | Table tests of `answer_for` | Create (Task 11), modify (Task 13) | S3a-2 |
| `src/mcp/daemon/test_support.rs` | Manual clock, gates, scripted builder, test connections | Create (Task 2), replace (Task 3) | S3a-1 |
| `src/mcp/transport.rs` | adds `serve_loop`, which `serve_connection` calls; adds `retryable_status_response` for the daemon's `indexing` status | Modify (Tasks 6, 10) | S3a-2 |
| `src/mcp/transport_loop_tests.rs` | Tests for `serve_loop` | Create (Task 6) | S3a-2 |
| `src/mcp/daemon/connection.rs` | `serve_admitted`, the tool-call step under the two locks; from Task 10, the `indexing` status; from Task 12, the stream's mode | Create (Task 7), modify (Tasks 10, 12, 13) | S3a-2 |
| `src/mcp/daemon/wire_support.rs` | A raw MCP client on a socket pair, for tests | Create (Task 7), extend (Task 8) | S3a-2 |
| `src/mcp/daemon/connection_tests.rs` | Spec test 1; warming, failure and departure over a socket; nothing served before `Ready`; a superseded failure; a client that stopped waiting; the stream's mode | Create (Task 7), modify (Tasks 10, 11, 12, 13) | S3a-2 |
| `src/mcp/daemon/connection_parity_tests.rs` | Byte parity with the in-process server, also for a client that stopped waiting before its build failed; spec test 14; the two error endings | Create (Task 8), modify (Tasks 11, 13) | S3a-2 |
| `src/mcp/daemon/connection_ready_tests.rs` | A client whose failed attempt a published retry superseded is served | Create (Task 13) | S3a-2 |
| `CLAUDE.md` | MCP Adapter notes | Modify (Tasks 5, 14) | both |
| `src/mcp/transport_tests.rs`, `src/mcp/owner_transport_tests.rs`, `tests/fixtures/mcp/golden/*`, `src/mcp/lazy.rs`, `src/mcp/framing.rs`, `src/mcp/shared/*` | The standalone contract and S1/S2's code | **Must not change** | |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "Replace the whole of `path` with:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file, then "with this:" and its replacement.
- "Append to `path`:" is followed by text to add at the end of the file.
- "In `path`, delete this:" is followed by text that occurs exactly once in the file; remove it together with the one blank line that follows it.

Every test step that expects a failure expects a **compile** failure: the tests are written first against items that do not exist yet. Tasks 8 and 12 are the exceptions and say so; Task 9 is a move and has no red step.

---

# Part one: S3a-1, the coordinator (Tasks 1–5)

### Task 1: The provider's generation, readable

The coordinator reports the generation of the index it serves (`Ready{generation}`, spec §4.2; `READY`'s `generation`, §4.4). `SessionProvider` counts it and has no accessor. This task also corrects a comment the lane plan measured to be false.

**Files:**
- Modify: `src/mcp/session.rs`
- Modify: `src/mcp/session_stage_tests.rs` (append one test)

**Interfaces:**
- Consumes (all `src/mcp/session.rs`):
  - `pub struct SessionProvider { …, generation: u64, … }` (line 100; the field at line 106)
  - `pub fn bootstrap(cfg: &ServerConfig) -> anyhow::Result<Self>` (line 163)
  - `pub fn refresh(&mut self) -> anyhow::Result<RefreshSummary>` (line 212)
  - `pub(crate) fn auto_refresh(&mut self) -> anyhow::Result<AutoRefreshSummary>` (line 232)
  - `#[cfg(test)] pub(crate) fn force_next_refresh_error_for_tests(&mut self, message: &'static str)` (line 208)
  - in `src/mcp/session_stage_tests.rs`: `fn config(dir: &tempfile::TempDir) -> ServerConfig` (line 3), which writes `a.py` and returns an eager, cache-free, warn-only configuration.
- Produces: `pub fn generation(&self) -> u64` on `SessionProvider`.

- [ ] **Step 1: Record the base test totals**

```bash
cargo test --features mcp --lib mcp:: > /tmp/s3a-base-lib.log 2>&1; grep -E '^test result' /tmp/s3a-base-lib.log
cargo test --features mcp --test mcp > /tmp/s3a-base-mcp.log 2>&1; grep -E '^test result' /tmp/s3a-base-mcp.log
```

Write down the two `passed` numbers. Observed in validation at `main` `86756aad`: `380 passed; 0 failed` for the first. The second was not measured there (it starts `prism-mcp` binaries). Every later expectation is "base + N" against these two.

- [ ] **Step 2: Write the test**

Append to `src/mcp/session_stage_tests.rs`:

````rust

/// The shared daemon reports this number to every client; it must count committed refreshes
/// and nothing else. Rules out an accessor that returns a constant or counts attempts.
#[test]
fn generation_counts_committed_refreshes_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let mut provider = SessionProvider::bootstrap(&config(&dir)).unwrap();
    assert_eq!(provider.generation(), 0, "a freshly built index");

    provider.refresh().unwrap();
    assert_eq!(provider.generation(), 1);

    provider.force_next_refresh_error_for_tests("refused");
    assert!(provider.refresh().is_err());
    assert_eq!(provider.generation(), 1, "a failed refresh commits nothing");

    provider.auto_refresh().unwrap();
    assert_eq!(
        provider.generation(),
        2,
        "an automatic refresh is a refresh"
    );
}
````

- [ ] **Step 3: Run it and see it fail to compile**

```bash
cargo test --features mcp --lib mcp::session::stage_tests > /tmp/s3a-t1-red.log 2>&1; grep -E '^error' /tmp/s3a-t1-red.log | sort -u
```

Expected: `error[E0599]: no method named `generation` found for struct `SessionProvider``.

- [ ] **Step 4: Implement**

In `src/mcp/session.rs`, replace this:

````rust
    pub(crate) fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        self.known_stale_after_refresh.as_ref()
    }
````

with this:

````rust
    pub(crate) fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        self.known_stale_after_refresh.as_ref()
    }

    /// How many refreshes this provider has committed: 0 for a freshly built index. A shared
    /// daemon reports it to its clients as the generation of the index it serves.
    pub fn generation(&self) -> u64 {
        self.generation
    }
````

In `src/mcp/session.rs`, replace this:

````rust
// `NavigationSession` holds `Arc<LoadedRepo>`/`Arc<NavigationIndex>` (the nav layer's shape);
// `LoadedRepo` is not `Sync`, but the MCP server dispatches single-threaded (spec §8) — `Send+Sync`
// is deferred to a future async transport (spec §17). Same `Arc<LoadedRepo>` pattern as `build_session`.
````

with this:

````rust
// `NavigationSession` holds `Arc<LoadedRepo>`/`Arc<NavigationIndex>` (the nav layer's shape).
// `SessionProvider` is `Send + Sync`; the shared daemon queries one provider from many
// connection threads and asserts that at compile time in `src/mcp/daemon/mod.rs`. The lint
// allowance below is older than that fact and is kept so that this change alters no lint outcome.
````

The `#[allow(clippy::arc_with_non_send_sync)]` line under the comment stays as it is.

- [ ] **Step 5: Run the test**

```bash
cargo fmt
cargo test --features mcp --lib mcp::session::stage_tests::generation > /tmp/s3a-t1-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t1-green.log
```

Expected: `test result: ok. 1 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git add src/mcp/session.rs src/mcp/session_stage_tests.rs
git commit -m "feat(mcp): expose the session provider's refresh generation"
```

- [ ] **Step 7: Prove the test bites (after the commit)**

Temporarily, in `src/mcp/session.rs`, replace this:

````rust
    pub fn generation(&self) -> u64 {
        self.generation
    }
````

with this:

````rust
    pub fn generation(&self) -> u64 {
        0
    }
````

```bash
cargo test --features mcp --lib mcp::session::stage_tests::generation > /tmp/s3a-t1-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t1-bite.log
git checkout -- src/mcp/session.rs
```

Expected: `generation_counts_committed_refreshes_and_nothing_else ... FAILED` and `0 passed; 1 failed`. Confirm the restore with `git status --short` (no output).

---

### Task 2: The daemon module and a connection's budget

One connection's first-call wait, counted per build attempt (spec §4.2 "Readiness semantics"). Pure: the clock is an argument. This task also creates the module the rest of the slice fills.

**Files:**
- Modify: `src/mcp/mod.rs`
- Create: `src/mcp/daemon/mod.rs`, `src/mcp/daemon/budget.rs`, `src/mcp/daemon/budget_tests.rs`, `src/mcp/daemon/test_support.rs`

**Interfaces:**
- Consumes:
  - `pub struct ClientSettings` with `pub fn new(output: ConnectionSettings, first_call_wait_secs: u64, warm_at_startup: bool) -> Result<Self, SettingsError>` (`src/mcp/settings.rs:80`) and `pub fn first_call_wait(&self) -> Duration` (`src/mcp/settings.rs:111`). `new` refuses a wait above `FIRST_CALL_WAIT_MAX`.
  - `pub struct ConnectionSettings { pub result_cap: usize, pub structured_content: StructuredContentMode, pub concise_shape: ConciseShapeMode }` (`src/mcp/settings.rs:13`).
  - `pub const FIRST_CALL_WAIT_MAX: Duration = Duration::from_secs(600)` (`src/mcp/session.rs:10`, re-exported as `crate::mcp::FIRST_CALL_WAIT_MAX`).
- Produces (in `crate::mcp::daemon::budget`):
  - `pub(super) struct ConnectionBudget`
  - `pub(super) fn new(client: &ClientSettings) -> Self`
  - `pub(super) fn deadline_for(&mut self, attempt: u64, now: Instant) -> Instant`
- Produces (in `crate::mcp::daemon::test_support`, tests only): `client_settings(wait_secs: u64) -> ClientSettings`, `client_settings_with(wait_secs: u64, structured_content: StructuredContentMode) -> ClientSettings`.

- [ ] **Step 1: Declare the module**

In `src/mcp/mod.rs`, replace this:

````rust
pub mod concise_shape;
````

with this:

````rust
pub mod concise_shape;
#[cfg(unix)]
mod daemon;
````

Create `src/mcp/daemon/mod.rs`:

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

mod budget;
#[cfg(test)]
mod test_support;
````

Create `src/mcp/daemon/test_support.rs`:

````rust
//! Pieces the daemon's tests share.
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};

/// The settings of a client that waits `wait_secs` for the index and always gets
/// `structuredContent`.
pub(super) fn client_settings(wait_secs: u64) -> ClientSettings {
    client_settings_with(wait_secs, StructuredContentMode::Always)
}

pub(super) fn client_settings_with(
    wait_secs: u64,
    structured_content: StructuredContentMode,
) -> ClientSettings {
    let output = ConnectionSettings {
        result_cap: 80_000,
        structured_content,
        concise_shape: ConciseShapeMode::Slim,
    };
    ClientSettings::new(output, wait_secs, false).expect("settings within the accepted bounds")
}
````

- [ ] **Step 2: Write the tests**

Create `src/mcp/daemon/budget_tests.rs`:

````rust
//! Tests for one connection's first-call budget: one deadline per connection per build attempt
//! (spec §4.2). The clock is an argument, so nothing here waits.
use super::super::test_support::client_settings;
use super::*;

fn budget_of(wait_secs: u64) -> ConnectionBudget {
    ConnectionBudget::new(&client_settings(wait_secs))
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// Rules out a fresh deadline for every call: queued calls would then multiply one attempt's
/// wait.
#[test]
fn the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.deadline_for(1, start), start + secs(20));
    assert_eq!(
        budget.deadline_for(1, start + secs(5)),
        start + secs(20),
        "a later call in the same attempt has only the rest of the budget"
    );
    assert_eq!(
        budget.deadline_for(1, start + secs(500)),
        start + secs(20),
        "and none once it has passed"
    );
}

/// Rules out one deadline for the connection's whole life: after a failed build the retry
/// would be answered with warming at once.
#[test]
fn a_new_attempt_gets_a_fresh_full_budget() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.deadline_for(1, start), start + secs(20));
    assert_eq!(
        budget.deadline_for(2, start + secs(30)),
        start + secs(50),
        "counted from the first call that arrives during the new attempt"
    );
    assert_eq!(budget.deadline_for(2, start + secs(45)), start + secs(50));
}

#[test]
fn each_connection_has_its_own_deadline_for_the_same_attempt() {
    let start = Instant::now();
    let mut a = budget_of(20);
    let mut b = budget_of(60);

    assert_eq!(a.deadline_for(1, start), start + secs(20));
    assert_eq!(b.deadline_for(1, start + secs(10)), start + secs(70));
    assert_eq!(
        a.deadline_for(1, start + secs(10)),
        start + secs(20),
        "B joining the attempt changed nothing for A"
    );
}

#[test]
fn a_wait_of_zero_has_expired_the_moment_it_is_set() {
    let start = Instant::now();
    let mut budget = budget_of(0);

    assert_eq!(budget.deadline_for(1, start), start);
}

#[test]
fn the_longest_wait_a_client_may_ask_for_is_kept_whole() {
    let start = Instant::now();
    let longest = crate::mcp::FIRST_CALL_WAIT_MAX;
    let mut budget = budget_of(longest.as_secs());

    assert_eq!(budget.deadline_for(1, start), start + longest);
}
````

Create `src/mcp/daemon/budget.rs`:

````rust
//! One connection's first-call wait (spec §4.2 "Readiness semantics", §5 "connection-local").

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
````

- [ ] **Step 3: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon::budget > /tmp/s3a-t2-red.log 2>&1; grep -E '^error' /tmp/s3a-t2-red.log | sort -u | head -20
```

Expected: errors naming `ConnectionBudget`, `Duration` and `Instant` as not found in this scope.

- [ ] **Step 4: Implement**

Replace the whole of `src/mcp/daemon/budget.rs` with:

````rust
//! One connection's first-call wait (spec §4.2 "Readiness semantics", §5 "connection-local").
use crate::mcp::settings::ClientSettings;
use std::time::{Duration, Instant};

/// How long one connection may wait for the index, counted separately for each build attempt.
///
/// A connection's deadline for an attempt is set by its first call that arrives during that
/// attempt and is kept for every later call during it, so a client that sends several calls
/// while one build runs waits its budget once, not once per call. A new attempt (a retry after
/// a failed build) starts a new deadline.
///
/// The only way to make one is from a [`ClientSettings`], whose constructor has already
/// refused a wait above `FIRST_CALL_WAIT_MAX`. The budget bounds the wait for the build; it
/// says nothing about how long a query then takes.
pub(super) struct ConnectionBudget {
    wait: Duration,
    /// The attempt this connection last joined, and its deadline for that attempt.
    joined: Option<(u64, Instant)>,
}

impl ConnectionBudget {
    pub(super) fn new(client: &ClientSettings) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

    /// This connection's deadline for `attempt`, for a call that arrives at `now`.
    ///
    /// If `now + wait` cannot be represented the deadline is `now`: the call does not wait.
    /// No `ClientSettings` can ask for a wait that long.
    pub(super) fn deadline_for(&mut self, attempt: u64, now: Instant) -> Instant {
        if let Some((joined, deadline)) = self.joined {
            if joined == attempt {
                return deadline;
            }
        }
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        self.joined = Some((attempt, deadline));
        deadline
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
````

- [ ] **Step 5: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon::budget > /tmp/s3a-t2-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t2-green.log
```

Expected: `test result: ok. 5 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git add src/mcp/mod.rs src/mcp/daemon/mod.rs src/mcp/daemon/budget.rs src/mcp/daemon/budget_tests.rs src/mcp/daemon/test_support.rs
git commit -m "feat(mcp): per-connection, per-attempt first-call budget for the shared daemon"
```

- [ ] **Step 7: Prove the tests bite (after the commit)**

Two mutants, one at a time. After each, run the command and restore the file.

Mutant A (a fresh deadline for every call). Temporarily, in `src/mcp/daemon/budget.rs`, replace this:

````rust
            if joined == attempt {
````

with this:

````rust
            if joined == attempt && deadline < now {
````

Mutant B (one deadline for the connection's life). Temporarily, in `src/mcp/daemon/budget.rs`, replace this:

````rust
            if joined == attempt {
````

with this:

````rust
            if joined <= attempt {
````

```bash
cargo test --features mcp --lib mcp::daemon::budget > /tmp/s3a-t2-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t2-bite.log
git checkout -- src/mcp/daemon/budget.rs
```

Expected for A: `the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it ... FAILED` (and two more: `2 passed; 3 failed`). Expected for B: `a_new_attempt_gets_a_fresh_full_budget ... FAILED`, `4 passed; 1 failed`. Both observed in validation.

---

### Task 3: The readiness coordinator

`DaemonState`: where the one index build is, who waits for it and for how long, and the slot the built index is published into (spec §4.2).

**Files:**
- Create: `src/mcp/daemon/clock.rs`, `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/coordinator_tests.rs`, `src/mcp/daemon/coordinator_failure_tests.rs`
- Replace: `src/mcp/daemon/mod.rs`, `src/mcp/daemon/test_support.rs`

The code of this task and of Task 4 is the code after the first review round of S3a-1 (two reviewers, the same two defects; "Review fold record" at the end of this plan). Three things differ from the plan as first written: a call is answered from the outcome of the attempt it joined (`AttemptOutcome`), not from the current phase; the generation of the provider in the slot is a field of the control plane that does not depend on the phase; and attempt numbers are never reused.

**Interfaces:**
- Consumes:
  - `pub(crate) type SessionBuilder = Arc<dyn Fn() -> anyhow::Result<SessionProvider> + Send + Sync>;` (`src/mcp/lazy.rs:6`)
  - `pub enum Readiness { Ready, Warming { elapsed: Duration }, Failed { error: String } }` (`src/mcp/lazy.rs:8`; it derives nothing, so tests convert it)
  - `pub enum StartupMode { Lazy, Background, Eager }` (`src/mcp/session.rs:13`)
  - `pub fn SessionProvider::bootstrap(cfg: &ServerConfig) -> anyhow::Result<Self>` (`src/mcp/session.rs:163`), `pub fn generation(&self) -> u64` (Task 1)
  - `pub enum BuildState { Idle, Building, Ready, Failed }`, `Clone + Copy + Debug + PartialEq + Eq` (`src/mcp/shared/preamble.rs:148`)
  - `pub struct ToolRegistry` (`src/mcp/registry.rs:114`), only in a compile-time assertion
  - `ConnectionBudget::{new, deadline_for}` (Task 2)
- Produces (in `crate::mcp::daemon::clock`): `pub(super) trait Clock: Send + Sync { fn now(&self) -> Instant; }`, `pub(super) struct SystemClock;`
- Produces (in `crate::mcp::daemon::coordinator`, all `pub(super)`):
  - `struct Poisoned { lock: &'static str }` (`Clone + Copy + Debug + PartialEq + Eq`, `Display`, `Error`)
  - `struct AttemptOutcome` (one attempt's ending, written once; `Debug + Default`)
  - `enum BuildPhase { Idle, Building { attempt: u64, started: Instant, outcome: Arc<AttemptOutcome> }, Ready, Failed { attempt: u64, error: String } }` (`Debug`) with `fn state(&self) -> BuildState`
  - `struct ControlState { build: BuildPhase, attempts: u64, waiting: u64, generation: u64 }`
  - `struct BuildStatus { state: BuildState, attempt: u64, generation: u64, waiting: u64 }` (`Clone + Copy + Debug + PartialEq + Eq`)
  - `type BuildJob = Box<dyn FnOnce() + Send>;`, `type Spawner = Box<dyn Fn(BuildJob) -> std::io::Result<()> + Send + Sync>;`
  - `struct Parts { builder: SessionBuilder, startup: StartupMode, clock: Arc<dyn Clock>, spawner: Spawner, #[cfg(test)] hooks: TestHooks }` with `fn production(builder: SessionBuilder, startup: StartupMode) -> Self`
  - `struct DaemonState` with
    - `fn new(builder: SessionBuilder, startup: StartupMode) -> anyhow::Result<Arc<Self>>`
    - `fn from_parts(parts: Parts) -> anyhow::Result<Arc<Self>>`
    - `fn startup_mode(&self) -> StartupMode`
    - `fn ensure_ready(self: &Arc<Self>, budget: &mut ConnectionBudget) -> Result<Readiness, Poisoned>`
    - `fn status(&self) -> Result<BuildStatus, Poisoned>`
    - `fn read<R>(&self, query: impl FnOnce(&SessionProvider) -> R) -> Result<Option<R>, Poisoned>`
    - `fn wake(&self)`
  - tests only: `type AttemptHook = Arc<dyn Fn(u64) + Send + Sync>;`, `struct TestHooks { published: Option<AttemptHook>, settled: Option<AttemptHook> }`

- [ ] **Step 1: Write the test support**

Replace the whole of `src/mcp/daemon/test_support.rs` with:

````rust
//! Pieces the daemon's tests share: a clock the test moves, gates that hold a build until the
//! test lets it go, a builder that plays a script, and a stand-in for one connection.
//!
//! No test here sleeps to order two events. A test waits for a condition that must become true
//! (`wait_until`) up to [`HANG`], and moves time itself (`advance`).
use super::budget::ConnectionBudget;
use super::clock::Clock;
use super::coordinator::{BuildStatus, DaemonState, Parts, Poisoned};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::lazy::{Readiness, SessionBuilder};
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, SessionProvider, StartupMode};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// How long a test waits for something that must happen before it calls the run hung. It is
/// generous on purpose: a verdict depends on whether the thing happens, never on how soon.
pub(super) const HANG: Duration = Duration::from_secs(30);

/// The settings of a client that waits `wait_secs` for the index and always gets
/// `structuredContent`.
pub(super) fn client_settings(wait_secs: u64) -> ClientSettings {
    client_settings_with(wait_secs, StructuredContentMode::Always)
}

pub(super) fn client_settings_with(
    wait_secs: u64,
    structured_content: StructuredContentMode,
) -> ClientSettings {
    let output = ConnectionSettings {
        result_cap: 80_000,
        structured_content,
        concise_shape: ConciseShapeMode::Slim,
    };
    ClientSettings::new(output, wait_secs, false).expect("settings within the accepted bounds")
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

/// A clock that moves only when a test moves it.
pub(super) struct ManualClock {
    now: Mutex<Instant>,
}

impl ManualClock {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(Instant::now()),
        })
    }

    fn advance(&self, by: Duration) {
        let mut now = self.now.lock().expect("manual clock");
        *now += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.now.lock().expect("manual clock")
    }
}

/// Move `clock` forward and make every parked call look at it. `DaemonState::wake` takes the
/// control lock, so a call that has read the old time and not yet parked is not missed.
pub(super) fn advance(clock: &ManualClock, state: &DaemonState, secs: u64) {
    clock.advance(Duration::from_secs(secs));
    state.wake();
}

/// Something a thread waits at until a test opens it.
pub(super) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub(super) fn closed() -> Arc<Self> {
        Arc::new(Self {
            open: Mutex::new(false),
            changed: Condvar::new(),
        })
    }

    pub(super) fn open(&self) {
        *self.open.lock().expect("gate") = true;
        self.changed.notify_all();
    }

    /// Wait until the gate is open and return `true`. Gives up after four times [`HANG`] and
    /// returns `false`, so a test that fails before it opens its gate does not leave a thread
    /// parked for the rest of the run. A test thread asserts on the result; a build step or a
    /// hook has nobody to tell and goes on.
    pub(super) fn pass(&self) -> bool {
        let open = self.open.lock().expect("gate");
        let (open, _timed_out) = self
            .changed
            .wait_timeout_while(open, HANG * 4, |open| !*open)
            .expect("gate");
        *open
    }
}

/// A two-file Python repository and a cache-free configuration for it.
pub(super) fn fixture(refresh_policy: RefreshPolicy) -> (tempfile::TempDir, ServerConfig) {
    let dir = tempfile::tempdir().expect("temp repository");
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n")
        .expect("write util.py");
    std::fs::write(
        dir.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("write main.py");
    let mut cfg = ServerConfig::new(dir.path().to_path_buf());
    cfg.cache = CacheMode::NoCache;
    cfg.refresh_policy = refresh_policy;
    (dir, cfg)
}

/// What one build attempt does in a test.
pub(super) type BuildStep = Box<dyn FnOnce() -> anyhow::Result<SessionProvider> + Send>;

/// Build the fixture once `gate` is open.
pub(super) fn build_after(gate: &Arc<Gate>, cfg: &ServerConfig) -> BuildStep {
    let gate = Arc::clone(gate);
    let cfg = cfg.clone();
    Box::new(move || -> anyhow::Result<SessionProvider> {
        let _opened = gate.pass();
        SessionProvider::bootstrap(&cfg)
    })
}

/// Fail with `message` once `gate` is open.
pub(super) fn fail_after(gate: &Arc<Gate>, message: &'static str) -> BuildStep {
    let gate = Arc::clone(gate);
    Box::new(move || -> anyhow::Result<SessionProvider> {
        let _opened = gate.pass();
        Err(anyhow::anyhow!(message))
    })
}

/// Fail with `message` at once.
pub(super) fn fail_now(message: &'static str) -> BuildStep {
    Box::new(move || -> anyhow::Result<SessionProvider> { Err(anyhow::anyhow!(message)) })
}

/// Panic on the build thread.
pub(super) fn panic_now() -> BuildStep {
    Box::new(|| -> anyhow::Result<SessionProvider> { panic!("injected builder panic") })
}

/// A builder that plays `steps` in order, one per build attempt, and builds the fixture for
/// every attempt after the last step. Also returns how many times the builder has been called.
pub(super) fn scripted(
    cfg: &ServerConfig,
    steps: Vec<BuildStep>,
) -> (SessionBuilder, Arc<AtomicU64>) {
    let calls = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&calls);
    let steps = Mutex::new(VecDeque::from(steps));
    let cfg = cfg.clone();
    let builder: SessionBuilder = Arc::new(move || {
        counted.fetch_add(1, Ordering::SeqCst);
        // The lock is released at the end of this statement, before the step runs.
        let step = steps.lock().expect("build script").pop_front();
        match step {
            Some(step) => step(),
            None => SessionProvider::bootstrap(&cfg),
        }
    });
    (builder, calls)
}

/// Production parts for a lazy daemon, on a manual clock.
pub(super) fn parts_with(builder: SessionBuilder, clock: &Arc<ManualClock>) -> Parts {
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let concrete: Arc<ManualClock> = Arc::clone(clock);
    let shared: Arc<dyn Clock> = concrete;
    parts.clock = shared;
    parts
}

/// A lazy daemon state on a manual clock.
pub(super) fn state_with(builder: SessionBuilder, clock: &Arc<ManualClock>) -> Arc<DaemonState> {
    DaemonState::from_parts(parts_with(builder, clock)).expect("a lazy daemon state")
}

pub(super) fn status(state: &DaemonState) -> BuildStatus {
    state.status().expect("the control lock is not poisoned")
}

/// What a readiness call returned, in a form a test can compare and print.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Ready,
    Warming { elapsed: Duration },
    Failed(String),
    Poisoned,
}

impl Outcome {
    fn of(result: Result<Readiness, Poisoned>) -> Self {
        match result {
            Ok(Readiness::Ready) => Self::Ready,
            Ok(Readiness::Warming { elapsed }) => Self::Warming { elapsed },
            Ok(Readiness::Failed { error }) => Self::Failed(error),
            Err(_) => Self::Poisoned,
        }
    }
}

/// A stand-in for one connection: a thread that owns one `ConnectionBudget` and makes one
/// readiness call each time the test asks, the way a connection's worker makes them one after
/// another.
pub(super) struct Conn {
    ask: mpsc::Sender<()>,
    answers: mpsc::Receiver<Outcome>,
    early: RefCell<Option<Outcome>>,
}

impl Conn {
    pub(super) fn open(state: &Arc<DaemonState>, wait_secs: u64) -> Self {
        let (ask, asked) = mpsc::channel::<()>();
        let (answer, answers) = mpsc::channel();
        let state = Arc::clone(state);
        std::thread::spawn(move || {
            let mut budget = ConnectionBudget::new(&client_settings(wait_secs));
            while asked.recv().is_ok() {
                let outcome = Outcome::of(state.ensure_ready(&mut budget));
                if answer.send(outcome).is_err() {
                    break;
                }
            }
        });
        Self {
            ask,
            answers,
            early: RefCell::new(None),
        }
    }

    /// Start one readiness call. It runs on the connection's thread; collect it with
    /// [`Conn::outcome`].
    pub(super) fn call(&self) {
        self.ask.send(()).expect("the connection thread is alive");
    }

    /// Whether the call has returned. Does not wait.
    pub(super) fn answered(&self) -> bool {
        let mut early = self.early.borrow_mut();
        if early.is_none() {
            *early = self.answers.try_recv().ok();
        }
        early.is_some()
    }

    /// What the call returned. Panics if it has not returned within [`HANG`].
    pub(super) fn outcome(&self) -> Outcome {
        if let Some(outcome) = self.early.borrow_mut().take() {
            return outcome;
        }
        self.answers
            .recv_timeout(HANG)
            .expect("the call returns within the hang limit")
    }
}
````

- [ ] **Step 2: Write the tests for states and budgets**

Create `src/mcp/daemon/coordinator_tests.rs`:

````rust
//! Tests for the readiness coordinator's states and budgets (spec §4.2 "Readiness semantics",
//! §9 unit list: per-connection, per-attempt budgets, terminal-outcome delivery, single retry).
//! Time is a clock the test moves; see `test_support` for why no verdict depends on the host.
use super::super::test_support::{
    advance, build_after, fail_after, fail_now, fixture, parts_with, scripted, state_with, status,
    wait_until, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

fn built(calls: &AtomicU64) -> u64 {
    calls.load(Ordering::SeqCst)
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn status_of(state: BuildState, attempt: u64, waiting: u64) -> BuildStatus {
    BuildStatus {
        state,
        attempt,
        generation: 0,
        waiting,
    }
}

/// Rules out building at construction: a client that only handshakes must cost no index.
#[test]
fn nothing_is_built_until_the_first_readiness_requiring_call() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    assert_eq!(status(&state), status_of(BuildState::Idle, 0, 0));
    assert_eq!(built(&calls), 0);
    assert_eq!(state.read(|_| ()), Ok(None), "nothing is published");

    // A client that waits zero seconds: the call starts the build and is answered at once.
    let conn = Conn::open(&state, 0);
    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Warming {
            elapsed: Duration::ZERO
        }
    );
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });

    // A second call during the same attempt starts nothing.
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    assert_eq!(status(&state).attempt, 1);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 1);
    assert_eq!(state.read(|_| ()), Ok(Some(())), "the index is published");
}

#[test]
fn a_warm_daemon_starts_attempt_one_at_construction_and_eager_is_refused() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    parts.startup = StartupMode::Background;
    let state = DaemonState::from_parts(parts).expect("a warm daemon state");

    assert!(state.startup_mode() == StartupMode::Background);
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });

    // The first call joins the attempt that is already running.
    let conn = Conn::open(&state, 600);
    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    assert_eq!(status(&state).attempt, 1);
    gate.open();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 1);

    // `--eager` is the in-process server's mode; a daemon state refuses it before building.
    let (eager_builder, eager_calls) = scripted(&cfg, vec![]);
    let refused = DaemonState::new(eager_builder, StartupMode::Eager)
        .err()
        .expect("an eager daemon state is refused");
    assert!(refused.to_string().contains("Eager"), "{refused}");
    assert_eq!(built(&eager_calls), 0);
}

/// Spec §4.2, the worked example: A and B join attempt 1 with budgets of 20 s and 60 s; A gets
/// warming at 20 s; B keeps waiting; publication at 35 s serves B directly and A's next call
/// immediately. Rules out one deadline shared by everyone waiting on an attempt.
#[test]
fn the_spec_s_worked_example_two_budgets_on_one_attempt() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let a = Conn::open(&state, 20);
    let b = Conn::open(&state, 60);

    a.call();
    b.call();
    wait_until("both calls to be waiting", || status(&state).waiting == 2);
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 2),
        "B joined the attempt A started"
    );

    advance(&clock, &state, 20);
    assert_eq!(a.outcome(), Outcome::Warming { elapsed: secs(20) });
    wait_until("B to be the only call waiting", || {
        status(&state).waiting == 1
    });
    assert!(!b.answered(), "A's deadline is not B's");
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "a warming answer is not a state change"
    );
    assert_eq!(built(&calls), 1, "and it cancels nothing");

    advance(&clock, &state, 15);
    gate.open();
    assert_eq!(
        b.outcome(),
        Outcome::Ready,
        "B is served by the publication at 35 s"
    );
    a.call();
    assert_eq!(
        a.outcome(),
        Outcome::Ready,
        "A's next call is served at once"
    );
    assert_eq!(status(&state), status_of(BuildState::Ready, 1, 0));
    assert_eq!(built(&calls), 1);
}

/// Rules out a fresh deadline for every call: a client that sends four calls during one build
/// would otherwise wait four budgets.
#[test]
fn later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 20);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });
    advance(&clock, &state, 12);
    advance(&clock, &state, 8);
    assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(20) });

    // The budget for this attempt is spent. Every later call returns without waiting.
    for _ in 0..3 {
        conn.call();
        assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(20) });
    }
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));

    advance(&clock, &state, 100);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(120) });
    assert_eq!(built(&calls), 1);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
}

/// Rules out waking one waiter, and rules out a call that answers a failure by starting the
/// retry itself (a build that keeps failing would then keep one call forever).
#[test]
fn a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![fail_after(&gate, "the repository is gone")]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conns = [
        Conn::open(&state, 600),
        Conn::open(&state, 60),
        Conn::open(&state, 20),
    ];

    for conn in &conns {
        conn.call();
    }
    wait_until("all three calls to be waiting", || {
        status(&state).waiting == 3
    });
    gate.open();

    for conn in &conns {
        assert_eq!(
            conn.outcome(),
            Outcome::Failed("the repository is gone".to_string())
        );
    }
    assert_eq!(status(&state), status_of(BuildState::Failed, 1, 0));
    assert_eq!(
        built(&calls),
        1,
        "a failure is answered; the retry waits for the next call"
    );
}

/// Rules out a retry per caller. Eight calls arrive after a failure; the builder runs once
/// more, not eight times.
#[test]
fn the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let retry_gate = Gate::closed();
    let (builder, calls) = scripted(
        &cfg,
        vec![
            fail_now("first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    let first = Conn::open(&state, 600);
    first.call();
    assert_eq!(
        first.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(status(&state), status_of(BuildState::Failed, 1, 0));

    let conns: Vec<Conn> = (0..8).map(|_| Conn::open(&state, 600)).collect();
    for conn in &conns {
        conn.call();
    }
    wait_until("all eight calls to be waiting on the retry", || {
        status(&state).waiting == 8
    });
    wait_until("the retry's build thread to call the builder", || {
        built(&calls) == 2
    });
    assert_eq!(status(&state), status_of(BuildState::Building, 2, 8));

    retry_gate.open();
    for conn in &conns {
        assert_eq!(conn.outcome(), Outcome::Ready);
    }
    assert_eq!(status(&state), status_of(BuildState::Ready, 2, 0));
    assert_eq!(built(&calls), 2);
}

/// Rules out a deadline kept for the connection's life: its budget for attempt 1 is spent, and
/// the retry must still be waited for in full.
#[test]
fn a_retry_gives_a_connection_a_fresh_full_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let retry_gate = Gate::closed();
    let (builder, calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 20);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    advance(&clock, &state, 20);
    assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(20) });

    first_gate.open();
    wait_until("the first attempt to fail", || {
        status(&state).state == BuildState::Failed
    });

    // This call starts the retry. With a fresh budget it waits; with the old deadline it would
    // be answered at once.
    conn.call();
    wait_until("the call to wait on the retry, or to return", || {
        status(&state).waiting == 1 || conn.answered()
    });
    assert!(
        !conn.answered(),
        "the retry's budget is fresh, so the call waits"
    );
    assert_eq!(status(&state), status_of(BuildState::Building, 2, 1));

    advance(&clock, &state, 20);
    assert_eq!(
        conn.outcome(),
        Outcome::Warming { elapsed: secs(20) },
        "elapsed counts from the retry's start"
    );

    retry_gate.open();
    wait_until("the retry to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 2, "one build per attempt");
}

/// The production clock path: nobody moves a clock and nobody wakes the call. Rules out a wait
/// with no timeout. Asserts a lower bound only, so a slow host cannot fail it.
#[test]
fn on_the_real_clock_a_call_returns_warming_when_its_budget_ends_unprompted() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let conn = Conn::open(&state, 1);

    conn.call();
    match conn.outcome() {
        Outcome::Warming { elapsed } => assert!(
            elapsed >= secs(1),
            "the call returned before its budget ended: {elapsed:?}"
        ),
        other => panic!("expected warming, got {other:?}"),
    }
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));
    gate.open();
}

/// Spec §4.2: a connection's deadline for an attempt is set when its first call arrives during
/// that attempt. A connection that joins a build 100 s old with a 20 s budget waits until
/// 120 s. Rules out counting the deadline from the attempt's start (the joiner would be told
/// warming at once, on every call). Also pins what `elapsed` means for a joiner: the age of
/// the attempt.
#[test]
fn a_connection_that_joins_a_running_build_gets_its_whole_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    let first = Conn::open(&state, 600);
    first.call();
    wait_until("the first call to be waiting", || {
        status(&state).waiting == 1
    });
    advance(&clock, &state, 100);

    let joiner = Conn::open(&state, 20);
    joiner.call();
    wait_until("the joiner to wait, or to return", || {
        status(&state).waiting == 2 || joiner.answered()
    });
    assert!(
        !joiner.answered(),
        "a call that joins a running build waits its own budget"
    );

    advance(&clock, &state, 20);
    assert_eq!(
        joiner.outcome(),
        Outcome::Warming { elapsed: secs(120) },
        "elapsed is the age of the attempt"
    );
    gate.open();
    assert_eq!(first.outcome(), Outcome::Ready);
}
````

- [ ] **Step 3: Write the tests for builds that cannot start or die**

Create `src/mcp/daemon/coordinator_failure_tests.rs`:

````rust
//! Tests for the ways an attempt can end without a build: the thread cannot be started, the
//! builder panics, a lock is poisoned. In each, every parked call must get an answer.
use super::super::test_support::{
    build_after, fail_now, fixture, panic_now, parts_with, scripted, state_with, status,
    wait_until, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};

fn built(calls: &AtomicU64) -> u64 {
    calls.load(Ordering::SeqCst)
}

/// `std::thread::spawn` panics when the thread cannot be created, and the first tool call of
/// any client reaches that line. Rules out ignoring the spawner's error (the call would then
/// wait for a build that never runs).
#[test]
fn a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let refusals_left = AtomicU64::new(1);
    let spawner: Spawner = Box::new(move |job: BuildJob| {
        if refusals_left.swap(0, Ordering::SeqCst) == 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "no more threads",
            ));
        }
        std::thread::Builder::new().spawn(job).map(|_handle| ())
    });
    parts.spawner = spawner;
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("could not start the index build thread: no more threads".to_string())
    );
    assert_eq!(
        status(&state),
        BuildStatus {
            state: BuildState::Failed,
            attempt: 1,
            generation: 0,
            waiting: 0
        }
    );
    assert_eq!(built(&calls), 0, "the builder never ran");
    assert!(
        matches!(
            &state.lock_control().expect("the control lock").build,
            BuildPhase::Failed { attempt: 1, error } if error.ends_with("no more threads")
        ),
        "the phase names the attempt that failed and why"
    );

    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 1);
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

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("index build panicked".to_string())
    );
    assert_eq!(status(&state).state, BuildState::Failed);
    assert_eq!(status(&state).waiting, 0);

    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 2);
}

/// Rules out recording an outcome without looking at the phase: a late `Failed` after `Ready`
/// would let the next call rebuild and replace an index that has since been refreshed.
#[test]
fn an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);

    // An outcome for an attempt that is not the one building is ignored.
    state.settle(9, Ok(()));
    assert_eq!(status(&state).state, BuildState::Building);
    assert_eq!(state.read(|_| ()), Ok(None));

    gate.open();
    assert_eq!(conn.outcome(), Outcome::Ready);

    // So is a second outcome for the attempt that already ended.
    state.settle(1, Err("too late".to_string()));
    assert_eq!(status(&state).state, BuildState::Ready);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 1, "nothing was rebuilt");
}

fn poison_the_control_lock(state: &Arc<DaemonState>) {
    let state = Arc::clone(state);
    let panicked = std::thread::spawn(move || {
        let _control = state.control.lock();
        panic!("poisoning the control lock for a test");
    })
    .join();
    assert!(panicked.is_err());
}

/// Rules out `unwrap` on the lock (a panic on a client's path) and a wait that never looks at
/// the lock again (a hang).
#[test]
fn a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let parked = Conn::open(&state, 600);

    parked.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    poison_the_control_lock(&state);
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

/// An attempt's outcome is written once. Rules out a second ending that replaces the first:
/// calls that share the outcome could then be told two different things.
#[test]
fn an_attempt_s_outcome_keeps_the_first_ending_it_is_given() {
    let outcome = AttemptOutcome::default();
    assert!(outcome.answer().is_none(), "empty while the attempt builds");

    outcome.fill(Err("the first ending".to_string()));
    outcome.fill(Ok(()));
    outcome.fill(Err("a third".to_string()));

    assert!(
        matches!(outcome.answer(), Some(Readiness::Failed { error }) if error == "the first ending")
    );
}

/// A warm daemon (`StartupMode::Background`) starts its first attempt at construction. If the
/// build thread cannot be started then, construction still succeeds, the attempt is failed,
/// and the first call retries. Rules out a daemon that fails to start, or one left `Building`
/// with no thread.
#[test]
fn a_warm_daemon_whose_build_thread_cannot_be_started_is_built_by_the_first_call() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    parts.startup = StartupMode::Background;
    let refusals_left = AtomicU64::new(1);
    let spawner: Spawner = Box::new(move |job: BuildJob| {
        if refusals_left.swap(0, Ordering::SeqCst) == 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "no more threads",
            ));
        }
        std::thread::Builder::new().spawn(job).map(|_handle| ())
    });
    parts.spawner = spawner;
    let state = DaemonState::from_parts(parts).expect("a warm daemon state");

    assert_eq!(
        status(&state),
        BuildStatus {
            state: BuildState::Failed,
            attempt: 1,
            generation: 0,
            waiting: 0
        }
    );
    assert_eq!(built(&calls), 0);

    let conn = Conn::open(&state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 1);
}

/// An attempt's number is what a connection's budget and an attempt's outcome are told apart
/// by, so one is never used twice. When the counter cannot go on, a call is told so and no
/// build is started. Rules out a counter that saturates and reuses its last number.
#[test]
fn an_exhausted_attempt_counter_is_a_failure_that_starts_no_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![fail_now("the last attempt failed")]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    state.lock_control().expect("the control lock").attempts = u64::MAX - 1;
    let conn = Conn::open(&state, 600);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("the last attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, u64::MAX);
    assert_eq!(built(&calls), 1);

    for _ in 0..2 {
        conn.call();
        assert_eq!(
            conn.outcome(),
            Outcome::Failed(ATTEMPTS_EXHAUSTED.to_string())
        );
    }
    assert_eq!(
        built(&calls),
        1,
        "no build was started for a number already used"
    );
    assert_eq!(
        status(&state),
        BuildStatus {
            state: BuildState::Failed,
            attempt: u64::MAX,
            generation: 0,
            waiting: 0
        }
    );
    assert!(matches!(
        &state.lock_control().expect("the control lock").build,
        BuildPhase::Failed { attempt: u64::MAX, error } if error == ATTEMPTS_EXHAUSTED
    ));
}

/// Linux keeps 15 bytes of a thread's name.
#[test]
fn the_build_thread_s_name_fits_in_what_linux_keeps() {
    assert!(BUILD_THREAD_NAME.len() <= 15, "{BUILD_THREAD_NAME}");
}
````

Create `src/mcp/daemon/clock.rs`:

````rust
//! The daemon's clock, injected so that tests can move time instead of waiting for it.
````

Create `src/mcp/daemon/coordinator.rs`:

````rust
//! The readiness coordinator and the provider slot of a shared daemon (spec §4.2).

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "coordinator_failure_tests.rs"]
mod failure_tests;
````

Replace the whole of `src/mcp/daemon/mod.rs` with:

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

mod budget;
mod clock;
mod coordinator;
#[cfg(test)]
mod test_support;

// One provider is queried from many connection threads at once, one registry serves all of
// them, and the coordinator is shared by every one (spec §4.2). A field that stops being
// `Send + Sync` fails here, by name.
const _: fn() = || {
    fn shared_between_threads<T: Send + Sync>() {}
    shared_between_threads::<crate::mcp::SessionProvider>();
    shared_between_threads::<crate::mcp::registry::ToolRegistry>();
    shared_between_threads::<coordinator::DaemonState>();
};
````

- [ ] **Step 4: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t3-red.log 2>&1; grep -cE '^error' /tmp/s3a-t3-red.log; grep -E '^error' /tmp/s3a-t3-red.log | sort -u | head -20
```

Expected: a non-zero count; the errors name `Clock`, `DaemonState`, `Parts`, `BuildStatus`, `Poisoned` and the other coordinator items as unresolved.

- [ ] **Step 5: Implement the clock**

Replace the whole of `src/mcp/daemon/clock.rs` with:

````rust
//! The daemon's clock, injected so that tests can move time instead of waiting for it.
use std::time::Instant;

/// Where the coordinator reads the time: when an attempt started, and whether a call's
/// deadline has passed.
///
/// A clock must make progress. A parked call sleeps for what is left of its budget in real
/// time and then reads this clock again; if the clock has not moved, the call sleeps again,
/// so a clock that stands still keeps a call parked for as long as its attempt runs. A clock
/// that goes backwards is treated as one that has not moved.
pub(super) trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

/// The clock a running daemon uses.
pub(super) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
````

- [ ] **Step 6: Implement the coordinator**

Replace the whole of `src/mcp/daemon/coordinator.rs` with:

````rust
//! The readiness coordinator and the provider slot of a shared daemon (spec §4.2).
//!
//! Two synchronization domains:
//!
//! - the **control plane**, one `Mutex<ControlState>` with one `Condvar`: where the index build
//!   is (`Idle`, `Building`, `Ready`, `Failed`) and how many calls are parked on it. It is held
//!   for a few field reads and writes at a time and while one build thread is started; not
//!   across a build, a query or a refresh.
//! - the **provider slot**, an `RwLock<Option<SessionProvider>>`, empty until a build is
//!   published into it.
//!
//! Lock order: code that holds the control lock does not touch the provider slot. Code that
//! holds the slot may take the control lock.
//!
//! Not here yet: a panic boundary. A poisoned lock is reported to the caller as [`Poisoned`]
//! and nothing is read through it; a panic in the builder is recorded as a failed attempt.
//! Turning both into the daemon-fatal abort of spec §3.1 is slice S3b.
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use crate::mcp::lazy::{Readiness, SessionBuilder};
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, RwLock, RwLockWriteGuard};
use std::time::Instant;

/// What a call is told when the builder panicked. The in-process server uses the same text.
const BUILD_PANICKED: &str = "index build panicked";
/// What a call is told once every attempt number has been used.
const ATTEMPTS_EXHAUSTED: &str = "the index build attempt counter is exhausted";
/// The build thread's name. Linux keeps 15 bytes of it.
const BUILD_THREAD_NAME: &str = "prism-idx-build";
const SLOT_POISONED: &str =
    "the index was built and could not be published: the provider lock is poisoned";

/// A lock of the daemon is poisoned: a thread panicked while holding it. The state behind it
/// is not read again. Until slice S3b makes this the daemon-fatal abort of spec §3.1, the
/// caller ends its connection with this error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Poisoned {
    pub(super) lock: &'static str,
}

impl fmt::Display for Poisoned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the daemon's {} lock is poisoned: a thread panicked while holding it",
            self.lock
        )
    }
}

impl std::error::Error for Poisoned {}

/// How one build attempt ended: empty while it builds, filled when it ends, and not changed
/// after that (a second fill has no effect). The attempt's phase holds one share of it and
/// every call that starts or joins the attempt holds another, so a call reads the outcome of
/// its own attempt however far the phase has moved on by the time the call runs again.
#[derive(Debug, Default)]
pub(super) struct AttemptOutcome {
    ended: OnceLock<Result<(), String>>,
}

impl AttemptOutcome {
    /// Record how the attempt ended. Only the first call has any effect.
    fn fill(&self, ended: Result<(), String>) {
        let _ = self.ended.set(ended);
    }

    /// What a call that belongs to this attempt is told; `None` while it is still building.
    fn answer(&self) -> Option<Readiness> {
        self.ended.get().map(|ended| match ended {
            Ok(()) => Readiness::Ready,
            Err(error) => Readiness::Failed {
                error: error.clone(),
            },
        })
    }
}

/// Where the index build is (spec §4.2).
#[derive(Debug)]
pub(super) enum BuildPhase {
    /// No build has been asked for.
    Idle,
    /// Attempt `attempt` is running since `started`. `outcome` is that attempt's, still empty.
    Building {
        attempt: u64,
        started: Instant,
        outcome: Arc<AttemptOutcome>,
    },
    /// A provider is in the slot and calls have been told so. Its generation is
    /// [`ControlState::generation`], which is kept apart from the phase because the slot is
    /// filled, and can be written to, before this phase is announced.
    Ready,
    /// Attempt `attempt` ended without a provider. The next readiness call starts the retry.
    Failed { attempt: u64, error: String },
}

impl BuildPhase {
    /// The word `READY` and the status probe use for this phase.
    pub(super) fn state(&self) -> BuildState {
        match self {
            Self::Idle => BuildState::Idle,
            Self::Building { .. } => BuildState::Building,
            Self::Ready => BuildState::Ready,
            Self::Failed { .. } => BuildState::Failed,
        }
    }
}

/// What the control lock guards. Slice S4 adds the admission counters and the retirement
/// decision beside the build phase, under the same lock.
pub(super) struct ControlState {
    pub(super) build: BuildPhase,
    /// How many attempts have been started; also the number of the latest one.
    pub(super) attempts: u64,
    /// How many calls are parked on the condition variable right now.
    pub(super) waiting: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty. It is written by
    /// `record_generation` and by nothing else, each time while the slot's write guard is
    /// held: when the provider is stored, and at the end of every exclusive access. It does
    /// not depend on the phase, so it is true in the interval between publication and the
    /// announcement of `Ready` as well.
    pub(super) generation: u64,
}

/// A snapshot of the control plane: what the status probe reports about the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildStatus {
    pub(super) state: BuildState,
    /// The number of the latest attempt; 0 before the first.
    pub(super) attempt: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty.
    pub(super) generation: u64,
    /// Calls parked on the build at the moment of the snapshot.
    pub(super) waiting: u64,
}

/// The work of one build thread.
pub(super) type BuildJob = Box<dyn FnOnce() + Send>;

/// Starts a thread that runs a [`BuildJob`]. It is called with the control lock held, so it
/// must return promptly (every other caller of the control plane waits while it runs, and the
/// calling connection's deadline is not set until it has returned) and must not run the job on
/// the calling thread (the job ends by taking the control lock, which the caller still holds).
/// An error means no thread exists and the job will never run.
pub(super) type Spawner = Box<dyn Fn(BuildJob) -> std::io::Result<()> + Send + Sync>;

fn spawn_build_thread(job: BuildJob) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(BUILD_THREAD_NAME.to_string())
        .spawn(job)
        .map(|_handle| ())
}

/// A step of the build thread at which a test can stand in for another actor. Called with no
/// lock held, with the attempt's number.
#[cfg(test)]
pub(super) type AttemptHook = Arc<dyn Fn(u64) + Send + Sync>;

#[cfg(test)]
#[derive(Clone, Default)]
pub(super) struct TestHooks {
    /// After the provider is in the slot, before `Ready` is announced.
    pub(super) published: Option<AttemptHook>,
    /// After an attempt's outcome is recorded, before the parked calls are notified.
    pub(super) settled: Option<AttemptHook>,
}

/// What a [`DaemonState`] is made of. Production code uses [`Parts::production`]; tests replace
/// the clock, the spawner or the hooks.
pub(super) struct Parts {
    pub(super) builder: SessionBuilder,
    pub(super) startup: StartupMode,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) spawner: Spawner,
    #[cfg(test)]
    pub(super) hooks: TestHooks,
}

impl Parts {
    pub(super) fn production(builder: SessionBuilder, startup: StartupMode) -> Self {
        Self {
            builder,
            startup,
            clock: Arc::new(SystemClock),
            spawner: Box::new(spawn_build_thread),
            #[cfg(test)]
            hooks: TestHooks::default(),
        }
    }
}

/// One daemon's index build and the index it produced, shared by every connection.
pub(super) struct DaemonState {
    control: Mutex<ControlState>,
    changed: Condvar,
    provider: RwLock<Option<SessionProvider>>,
    builder: SessionBuilder,
    startup: StartupMode,
    clock: Arc<dyn Clock>,
    spawner: Spawner,
    #[cfg(test)]
    hooks: TestHooks,
}

impl DaemonState {
    /// A daemon state that builds with `builder`: on the first readiness call for
    /// `StartupMode::Lazy`, at once for `StartupMode::Background`.
    pub(super) fn new(builder: SessionBuilder, startup: StartupMode) -> anyhow::Result<Arc<Self>> {
        Self::from_parts(Parts::production(builder, startup))
    }

    pub(super) fn from_parts(parts: Parts) -> anyhow::Result<Arc<Self>> {
        if parts.startup == StartupMode::Eager {
            anyhow::bail!(
                "a shared daemon builds its index on the first tool call or in the background; \
                 StartupMode::Eager belongs to the in-process server"
            );
        }
        let state = Arc::new(Self {
            control: Mutex::new(ControlState {
                build: BuildPhase::Idle,
                attempts: 0,
                waiting: 0,
                generation: 0,
            }),
            changed: Condvar::new(),
            provider: RwLock::new(None),
            builder: parts.builder,
            startup: parts.startup,
            clock: parts.clock,
            spawner: parts.spawner,
            #[cfg(test)]
            hooks: parts.hooks,
        });
        if state.startup == StartupMode::Background {
            let mut control = state.lock_control()?;
            let _started = state.start_attempt(&mut control);
        }
        Ok(state)
    }

    /// How this daemon's index is started. `initialize` advertises it (spec §5).
    pub(super) fn startup_mode(&self) -> StartupMode {
        self.startup
    }

    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// The call first decides which attempt it belongs to, once: the one that is building, or a
    /// new one if none has been started or the last one failed. From then on it reads only that
    /// attempt's outcome, whatever the phase has become by the time it runs again, and returns
    ///
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived),
    /// - `Failed`, with that attempt's own error, if that attempt ended without an index. The
    ///   call does not start the retry (the next call does), does not join a retry that has
    ///   begun in the meantime, and is told of the failure even if a retry has since succeeded:
    ///   its next call finds the index ready,
    /// - `Warming` when this connection's deadline for that attempt has passed. That is this
    ///   connection's wait ending; the build goes on and nobody else's deadline changes.
    ///
    /// Starting a new attempt and joining a running one happen under one acquisition of the
    /// control lock, so however many calls find the build failed, one of them starts the retry
    /// and the rest join it.
    pub(super) fn ensure_ready(
        self: &Arc<Self>,
        budget: &mut ConnectionBudget,
    ) -> Result<Readiness, Poisoned> {
        let mut control = self.lock_control()?;

        let building = match &control.build {
            BuildPhase::Ready => return Ok(Readiness::Ready),
            BuildPhase::Building {
                attempt,
                started,
                outcome,
            } => Some((*attempt, *started, Arc::clone(outcome))),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
        };
        let (attempt, started, outcome) =
            building.unwrap_or_else(|| self.start_attempt(&mut control));
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            if let Some(answer) = outcome.answer() {
                return Ok(answer);
            }
            let now = self.clock.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Readiness::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
            control.waiting = control.waiting.saturating_add(1);
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
                Err(_) => return Err(Poisoned { lock: "control" }),
            };
            control.waiting = control.waiting.saturating_sub(1);
        }
    }

    /// The control plane's view of the build.
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        let control = self.lock_control()?;
        Ok(BuildStatus {
            state: control.build.state(),
            attempt: control.attempts,
            generation: control.generation,
            waiting: control.waiting,
        })
    }

    /// Run `query` on the provider in the slot with shared access; `None` when the slot is
    /// empty. `query` gets the whole provider, so it sees what the in-process server's read
    /// path sees, the stale evidence a refresh recorded included.
    ///
    /// This looks at the slot as it is. The slot is filled a moment before `Ready` is
    /// announced, so a caller that has not been told `Ready` by `ensure_ready` can find a
    /// provider here while `status` still says `Building`.
    ///
    /// `query` must not call `read` or `write` again: the slot's lock is not re-entrant, and a
    /// nested call can deadlock. `status` is safe to call from it.
    pub(super) fn read<R>(
        &self,
        query: impl FnOnce(&SessionProvider) -> R,
    ) -> Result<Option<R>, Poisoned> {
        let slot = self
            .provider
            .read()
            .map_err(|_| Poisoned { lock: "provider" })?;
        let answer = slot.as_ref().map(query);
        Ok(answer)
    }

    /// Copy the generation of the provider in the slot into the control plane, whatever the
    /// phase. The write guard is an argument so that this cannot be called after the slot has
    /// been let go: the control plane's generation then changes in the order the slot's
    /// content did. This is the permitted lock order (slot, then control).
    fn record_generation(
        &self,
        _held: &RwLockWriteGuard<'_, Option<SessionProvider>>,
        generation: u64,
    ) -> Result<(), Poisoned> {
        let mut control = self.lock_control()?;
        control.generation = generation;
        Ok(())
    }

    /// Wake every parked call so that it looks at the clock again.
    ///
    /// A running daemon does not need this: a parked call wakes by itself when its deadline
    /// passes. Tests that move a manual clock call it. The control lock is taken first, so a
    /// call that has read the time and not yet parked is woken too.
    pub(super) fn wake(&self) {
        let _control = self.control.lock();
        self.changed.notify_all();
    }

    fn lock_control(&self) -> Result<MutexGuard<'_, ControlState>, Poisoned> {
        self.control
            .lock()
            .map_err(|_| Poisoned { lock: "control" })
    }

    /// Start the next attempt and return its number, its start time and its outcome, still
    /// empty. Called with the control lock held, from `Idle` or `Failed` only; that is what
    /// makes it the one place a build thread comes from.
    ///
    /// An attempt's number is what a connection's budget is kept by, so a number is used once.
    /// When the counter cannot go on, no attempt is started: the phase is `Failed` and the
    /// outcome returned is already filled with that failure.
    fn start_attempt(
        self: &Arc<Self>,
        control: &mut ControlState,
    ) -> (u64, Instant, Arc<AttemptOutcome>) {
        let started = self.clock.now();
        let outcome = Arc::new(AttemptOutcome::default());
        let Some(attempt) = control.attempts.checked_add(1) else {
            let attempt = control.attempts;
            outcome.fill(Err(ATTEMPTS_EXHAUSTED.to_string()));
            control.build = BuildPhase::Failed {
                attempt,
                error: ATTEMPTS_EXHAUSTED.to_string(),
            };
            return (attempt, started, outcome);
        };
        control.attempts = attempt;
        control.build = BuildPhase::Building {
            attempt,
            started,
            outcome: Arc::clone(&outcome),
        };

        let state = Arc::clone(self);
        let job: BuildJob = Box::new(move || state.run_build(attempt));
        if let Err(error) = (self.spawner)(job) {
            // No thread, so nothing will ever end this attempt: end it here. Nobody but the
            // caller can have joined it, because the control lock has been held since it began.
            let error = format!("could not start the index build thread: {error}");
            outcome.fill(Err(error.clone()));
            control.build = BuildPhase::Failed { attempt, error };
        }
        (attempt, started, outcome)
    }

    /// The build thread: run the builder with no lock held, then publish or record the failure.
    fn run_build(self: Arc<Self>, attempt: u64) {
        let fail_on_panic = PanicGuard {
            state: &self,
            attempt,
        };
        let built = (self.builder)();
        match built {
            Ok(provider) => self.publish(attempt, provider),
            Err(error) => self.settle(attempt, Err(error.to_string())),
        }
        drop(fail_on_panic);
    }

    /// Put the provider in the slot and record its generation, then announce `Ready`. In that
    /// order: a call that is told `Ready` goes straight to the slot. The generation is recorded
    /// while the guard that stored the provider is still held, so exclusive access that lands
    /// before the announcement finds it recorded and records its own after it.
    fn publish(&self, attempt: u64, provider: SessionProvider) {
        let generation = provider.generation();
        let published = match self.provider.write() {
            Ok(mut slot) => {
                *slot = Some(provider);
                // A poisoned control lock is left alone here as it is in `settle`.
                let _recorded = self.record_generation(&slot, generation);
                true
            }
            Err(_) => false,
        };
        if !published {
            self.settle(attempt, Err(SLOT_POISONED.to_string()));
            return;
        }
        #[cfg(test)]
        if let Some(hook) = &self.hooks.published {
            hook(attempt);
        }
        self.settle(attempt, Ok(()));
    }

    /// Record how `attempt` ended, in its own outcome and in the phase, and tell every parked
    /// call. Nothing is recorded unless the phase is still `Building` that attempt, so an
    /// attempt ends once and `Ready` is not left. A poisoned control lock is left alone; the
    /// parked calls find it when they wake.
    fn settle(&self, attempt: u64, ended: Result<(), String>) {
        if let Ok(mut control) = self.control.lock() {
            let building = match &control.build {
                BuildPhase::Building {
                    attempt: building,
                    outcome,
                    ..
                } if *building == attempt => Some(Arc::clone(outcome)),
                _ => None,
            };
            if let Some(outcome) = building {
                control.build = match &ended {
                    Ok(()) => BuildPhase::Ready,
                    Err(error) => BuildPhase::Failed {
                        attempt,
                        error: error.clone(),
                    },
                };
                outcome.fill(ended);
            }
        }
        #[cfg(test)]
        if let Some(hook) = &self.hooks.settled {
            hook(attempt);
        }
        self.changed.notify_all();
    }
}

/// Ends the attempt as failed if the build thread unwinds, so that a panic in the builder
/// cannot leave the phase `Building` with nobody left to end it.
struct PanicGuard<'a> {
    state: &'a DaemonState,
    attempt: u64,
}

impl Drop for PanicGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.state
                .settle(self.attempt, Err(BUILD_PANICKED.to_string()));
        }
    }
}

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "coordinator_failure_tests.rs"]
mod failure_tests;
````

- [ ] **Step 7: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t3-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t3-green.log
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t3-module.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t3-module.log
cargo check --features mcp > /tmp/s3a-t3-check.log 2>&1; grep -A4 -E '^warning' /tmp/s3a-t3-check.log | grep -E 'src/mcp/(daemon/|session\.rs|mod\.rs)' | sort -u
```

Expected: `test result: ok. 17 passed; 0 failed` (nine in `coordinator::tests`, eight in `coordinator::failure_tests`), then `22 passed; 0 failed` for the module. The run takes a little over a second of real time because of the one real-clock test. Two tests panic a helper thread on purpose (`injected builder panic`, `poisoning the control lock for a test`); the test harness captures that output, so a passing run prints neither message. The third command prints nothing: the non-test build has no warning that names the new module or the two files edited so far.

- [ ] **Step 8: Commit**

```bash
git add src/mcp/daemon
git commit -m "feat(mcp): readiness coordinator and provider slot for the shared daemon"
```

- [ ] **Step 9: Prove the tests bite (after the commit)**

Seven mutants in `src/mcp/daemon/coordinator.rs`, one at a time. After each, run the command below and restore the file. Mutants A, C, D and E make a call wait where it should return, so the named tests fail at the 30 s hang limit; that is the expected way for them to fail.

Mutant A (an attempt per caller). Replace this:

````rust
            BuildPhase::Building {
                attempt,
                started,
                outcome,
            } => Some((*attempt, *started, Arc::clone(outcome))),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
````

with this:

````rust
            BuildPhase::Building { .. } | BuildPhase::Idle | BuildPhase::Failed { .. } => None,
````

Mutant B (an outcome recorded without looking at which attempt is building). Replace this:

````rust
                } if *building == attempt => Some(Arc::clone(outcome)),
````

with this:

````rust
                } if *building == attempt || attempt > 0 => Some(Arc::clone(outcome)),
````

Mutant C (a wait with no timeout). Replace this:

````rust
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
````

with this:

````rust
            control = match self.changed.wait(control) {
                Ok(reacquired) => reacquired,
````

Mutant D (the spawner's error ignored). Replace this:

````rust
        if let Err(error) = (self.spawner)(job) {
````

with this:

````rust
        if let Err(error) = (self.spawner)(job).or(Ok::<(), std::io::Error>(())) {
````

Mutant E (no guard against a builder panic). Replace this:

````rust
        if std::thread::panicking() {
````

with this:

````rust
        if std::thread::panicking() && self.attempt == 0 {
````

Mutant F (a deadline counted from the attempt's start, not from the connection's first call). Replace this:

````rust
budget.deadline_for(attempt, self.clock.now())
````

with this:

````rust
budget.deadline_for(attempt, started)
````

Mutant G (an attempt counter that saturates and reuses its last number). Replace this:

````rust
control.attempts.checked_add(1)
````

with this:

````rust
Some(control.attempts.saturating_add(1))
````

```bash
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t3-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t3-bite.log
git checkout -- src/mcp/daemon/coordinator.rs
```

Expected, and observed on the folded branch (there with Task 4's tests present as well; the tests named here are Task 3's):

| Mutant | Fails |
|---|---|
| A | seven tests, at the 30 s hang limit: `the_spec_s_worked_example_two_budgets_on_one_attempt` (the status shows attempt 2 once B has called), `the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together`, `a_warm_daemon_starts_attempt_one_at_construction_and_eager_is_refused`, `nothing_is_built_until_the_first_readiness_requiring_call`, `later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget`, `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself`, `a_connection_that_joins_a_running_build_gets_its_whole_budget` |
| B | `an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left` |
| C | `on_the_real_clock_a_call_returns_warming_when_its_budget_ends_unprompted` (after 30 s) |
| D | `a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries` and `a_warm_daemon_whose_build_thread_cannot_be_started_is_built_by_the_first_call` (after 30 s) |
| E | `a_builder_panic_is_a_failed_attempt_every_waiter_hears_of` (after 30 s) |
| F | `a_connection_that_joins_a_running_build_gets_its_whole_budget` (before that test existed this mutant passed every test) |
| G | `an_exhausted_attempt_counter_is_a_failure_that_starts_no_build` |

There is no mutant for "an attempt's outcome is filled twice": the outcome is a `OnceLock`, which cannot be written a second time, and `an_attempt_s_outcome_keeps_the_first_ending_it_is_given` states that.

---

### Task 4: Exclusive access, and the two orderings another actor can break

`DaemonState::write`, the only way to the provider's `&mut`, which leaves the provider's generation in the control plane; and the tests that stand on the build thread where another actor can interleave: between publication and the announcement of `Ready` (the slot is filled first, and exclusive access that lands there still leaves a true generation), and between a failed attempt and the notification of its waiter (the waiter hears of its own attempt, whatever the retries did meanwhile).

**Files:**
- Modify: `src/mcp/daemon/coordinator.rs`
- Create: `src/mcp/daemon/coordinator_order_tests.rs`, `src/mcp/daemon/coordinator_edge_tests.rs`

`coordinator_edge_tests.rs` came from the second review round of S3a-1 (both reviewers approved; these are the edges of the contract that round showed were stated and not held by a test): a waiter that first runs when a third attempt is building, with both earlier notifications held; a provider that is published at a generation other than 0; a budget that ends exactly as its attempt ends; and a zero-wait client whose attempt is already over.

**Interfaces:**
- Consumes: Task 3's `DaemonState` (with its private `record_generation`), `Parts`, `TestHooks { published, settled }`, `AttemptHook`, `Poisoned`; `pub fn SessionProvider::refresh(&mut self) -> anyhow::Result<RefreshSummary>` (`src/mcp/session.rs:212`) and `RefreshSummary.generation: u64` (`src/mcp/session.rs:75`).
- Produces: `pub(super) fn write<R>(&self, change: impl FnOnce(&mut SessionProvider) -> R) -> Result<Option<R>, Poisoned>` on `DaemonState`.

- [ ] **Step 1: Write the tests**

Create `src/mcp/daemon/coordinator_order_tests.rs`:

````rust
//! Tests for the two steps of the build thread at which another actor can interleave, each
//! played through a test hook, and for exclusive access to the published provider.
use super::super::test_support::{
    build_after, fail_after, fixture, parts_with, scripted, state_with, status, wait_until, Conn,
    Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};

fn built(calls: &AtomicU64) -> u64 {
    calls.load(Ordering::SeqCst)
}

/// A hook that reports it was reached and then holds the build thread until the test lets it
/// go, for the attempt numbered `only`.
fn stop_at(reached: &Arc<Gate>, resume: &Arc<Gate>, only: u64) -> AttemptHook {
    let reached = Arc::clone(reached);
    let resume = Arc::clone(resume);
    Arc::new(move |attempt: u64| {
        if attempt == only {
            reached.open();
            let _opened = resume.pass();
        }
    })
}

/// "Publication before Ready" (spec §4.2). The test stands on the build thread between the two
/// steps and looks from outside. Rules out announcing first: a call told `Ready` would then
/// find the slot empty.
#[test]
fn the_index_is_in_the_slot_before_ready_is_announced() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let reached = Gate::closed();
    let resume = Gate::closed();
    parts.hooks.published = Some(stop_at(&reached, &resume, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert!(
        reached.pass(),
        "the build thread reaches the publication hook"
    );

    // The build thread is between publishing and announcing.
    assert_eq!(
        state.read(|_| ()),
        Ok(Some(())),
        "the provider is already in the slot"
    );
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "and Ready has not been announced"
    );
    assert!(
        !conn.answered(),
        "so no call has been told the index is ready"
    );

    resume.open();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).state, BuildState::Ready);
}

/// Attempt 1 fails. Before its waiter is notified, another connection calls: it is the next
/// call after the failure, so it starts the retry. The waiter then wakes to find attempt 2
/// building. It must be told that its own attempt failed. Rules out a waiter that joins
/// whatever is building (it would wait on the retry with the budget of another attempt) and
/// one that starts a third attempt.
#[test]
fn a_waiter_that_wakes_after_the_retry_began_still_hears_of_its_own_attempt_s_failure() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let retry_gate = Gate::closed();
    let (builder, calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = Gate::closed();
    let notify = Gate::closed();
    parts.hooks.settled = Some(stop_at(&recorded, &notify, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("the early call to be waiting on attempt 1", || {
        status(&state).waiting == 1
    });

    first_gate.open();
    assert!(recorded.pass(), "attempt 1's failure is recorded");
    // Attempt 1's failure is recorded and its waiter has not been notified. Play the other
    // client now.
    let late = Conn::open(&state, 600);
    late.call();
    wait_until("the retry to be building", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 2
    });

    notify.open();
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string()),
        "the early call is told how its own attempt ended"
    );
    wait_until("the retry's build thread to call the builder", || {
        built(&calls) == 2
    });
    assert_eq!(status(&state).attempt, 2, "and it started no third attempt");

    retry_gate.open();
    assert_eq!(late.outcome(), Outcome::Ready);
    early.call();
    assert_eq!(early.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 2);
}

/// Rules out a control plane that keeps reporting the generation the index was published
/// with: `READY` and the status probe would then name a snapshot nobody is served any more.
#[test]
fn exclusive_access_records_the_provider_s_generation_in_the_control_plane() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    assert_eq!(
        state.write(|_| ()),
        Ok(None),
        "nothing is published, so there is nothing to change"
    );
    assert_eq!(status(&state).state, BuildState::Idle);

    let conn = Conn::open(&state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).generation, 0);

    let refreshed = state
        .write(|provider| provider.refresh().map(|summary| summary.generation))
        .expect("the provider lock is not poisoned")
        .expect("an index is published");
    assert_eq!(refreshed.expect("the refresh succeeds"), 1);
    assert_eq!(
        status(&state).generation,
        1,
        "the control plane reports the refreshed generation"
    );
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));

    // Access that changes nothing leaves the generation where it is.
    assert_eq!(state.write(|_| ()), Ok(Some(())));
    assert_eq!(status(&state).generation, 1);
}

fn poison_the_provider_slot(state: &Arc<DaemonState>) {
    let state = Arc::clone(state);
    let panicked = std::thread::spawn(move || {
        let _slot = state.provider.write();
        panic!("poisoning the provider slot for a test");
    })
    .join();
    assert!(panicked.is_err());
}

/// Rules out announcing `Ready` when the provider could not be put in the slot, and `unwrap`
/// on the slot's lock.
#[test]
fn a_poisoned_provider_slot_fails_the_attempt_and_is_an_error_for_every_reader() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    poison_the_provider_slot(&state);
    gate.open();

    assert_eq!(conn.outcome(), Outcome::Failed(SLOT_POISONED.to_string()));
    assert_eq!(
        status(&state).state,
        BuildState::Failed,
        "Ready was not announced over an empty slot"
    );
    assert_eq!(state.read(|_| ()), Err(Poisoned { lock: "provider" }));
    assert_eq!(state.write(|_| ()), Err(Poisoned { lock: "provider" }));
}

/// What the three tests below share: attempt 1 is parked on by `early`, fails with "first
/// attempt failed", and its build thread is held between recording that and notifying. The
/// caller then lets other attempts come and go before `early` is allowed to hear anything from
/// attempt 1's own notification. Returns the state, the early connection, the gate that
/// releases attempt 1's notification, and the builder's call counter.
fn a_waiter_held_behind_its_failed_attempt(
    cfg: &crate::mcp::ServerConfig,
    later_attempts: Vec<super::super::test_support::BuildStep>,
) -> (Arc<DaemonState>, Conn, Arc<Gate>, Arc<AtomicU64>) {
    let first_gate = Gate::closed();
    let mut steps = vec![fail_after(&first_gate, "first attempt failed")];
    steps.extend(later_attempts);
    let (builder, calls) = scripted(cfg, steps);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = Gate::closed();
    let notify = Gate::closed();
    parts.hooks.settled = Some(stop_at(&recorded, &notify, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("the early call to be waiting on attempt 1", || {
        status(&state).waiting == 1
    });
    first_gate.open();
    assert!(recorded.pass(), "attempt 1's failure is recorded");
    (state, early, notify, calls)
}

/// A call is answered with the outcome of the attempt it joined, whatever has happened since
/// (spec §4.2: "its outcome is delivered to every connection waiting on it"). Here the retry
/// has already succeeded when the waiter of attempt 1 runs. Rules out answering from the
/// current phase: the waiter would be told `Ready`, and attempt 1's failure would reach nobody.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_has_succeeded() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (state, early, notify, calls) = a_waiter_held_behind_its_failed_attempt(&cfg, vec![]);

    // The next call after the failure starts attempt 2, which builds and is announced. That
    // announcement wakes everybody parked, the waiter of attempt 1 included.
    let late = Conn::open(&state, 600);
    late.call();
    assert_eq!(late.outcome(), Outcome::Ready);
    assert_eq!(status(&state).state, BuildState::Ready);

    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string()),
        "the waiter of attempt 1 is told how attempt 1 ended"
    );
    notify.open();
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 2);

    // Its next call finds the index ready.
    early.call();
    assert_eq!(early.outcome(), Outcome::Ready);
}

/// As above, with a retry that failed for another reason. Rules out one remembered error for
/// all attempts: the waiter of attempt 1 would be told attempt 2's.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_failed_differently() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (state, early, notify, _calls) = a_waiter_held_behind_its_failed_attempt(
        &cfg,
        vec![super::super::test_support::fail_now(
            "second attempt failed",
        )],
    );

    let late = Conn::open(&state, 600);
    late.call();
    assert_eq!(
        late.outcome(),
        Outcome::Failed("second attempt failed".to_string()),
        "the call that started attempt 2 hears attempt 2's"
    );

    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string()),
        "the waiter of attempt 1 hears attempt 1's"
    );
    notify.open();
    assert_eq!(status(&state).attempt, 2);
}

// The third-attempt case is in `coordinator_edge_tests.rs`, where both earlier notifications
// are held so that the waiter of attempt 1 cannot run before attempt 3 is building.

/// Exclusive access can land after the provider is in the slot and before `Ready` is
/// announced. The generation the control plane then reports must still be the slot's. Rules
/// out announcing the generation that was read before publication, and recording a generation
/// only once the phase is `Ready`.
#[test]
fn exclusive_access_between_publication_and_ready_keeps_the_generation_current() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let reached = Gate::closed();
    let resume = Gate::closed();
    parts.hooks.published = Some(stop_at(&reached, &resume, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert!(
        reached.pass(),
        "the build thread reaches the publication hook"
    );
    assert_eq!(status(&state).state, BuildState::Building);

    let refreshed = state
        .write(|provider| provider.refresh().map(|summary| summary.generation))
        .expect("the provider lock is not poisoned")
        .expect("an index is in the slot");
    assert_eq!(refreshed.expect("the refresh succeeds"), 1);
    assert_eq!(
        status(&state).generation,
        1,
        "the generation is the slot's before Ready is announced too"
    );

    resume.open();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).state, BuildState::Ready);
    assert_eq!(
        status(&state).generation,
        1,
        "the announcement did not bring back the generation read before publication"
    );
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));
}
````

Create `src/mcp/daemon/coordinator_edge_tests.rs`:

````rust
//! Edges of the coordinator's contract that the second review round showed were stated and
//! not held by a test: each of these passes on the code as it is and fails under a mutant the
//! other tests let through (named in each test's comment).
use super::super::clock::Clock;
use super::super::test_support::{
    build_after, fail_after, fail_now, fixture, parts_with, scripted, status, wait_until,
    BuildStep, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use crate::mcp::ServerConfig;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// A clock that moves without waking anybody (`ManualClock::advance` always wakes).
struct StepClock {
    now: Mutex<Instant>,
}

impl StepClock {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(Instant::now()),
        })
    }
    fn step(&self, secs: u64) {
        *self.now.lock().expect("step clock") += Duration::from_secs(secs);
    }
}

impl Clock for StepClock {
    fn now(&self) -> Instant {
        *self.now.lock().expect("step clock")
    }
}

fn refusing_spawner() -> Spawner {
    Box::new(move |_job: BuildJob| {
        Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "no more threads",
        ))
    })
}

/// The waiter of attempt 1 first runs when attempt 3 is building. The notifications of
/// attempts 1 and 2 are both held, so nothing wakes either earlier waiter until then; each is
/// then told its own attempt's failure, and the caller of attempt 3 keeps waiting. Rules out
/// an error kept with only its attempt number: nothing of attempt 1 would be left.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_a_third_attempt_is_building() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let third_gate = Gate::closed();
    let (builder, _calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            fail_now("second attempt failed"),
            build_after(&third_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = [Gate::closed(), Gate::closed()];
    let notify = [Gate::closed(), Gate::closed()];
    let (hook_recorded, hook_notify) = (recorded.clone(), notify.clone());
    let hook: AttemptHook = Arc::new(move |attempt: u64| {
        if attempt == 1 || attempt == 2 {
            let at = (attempt - 1) as usize;
            hook_recorded[at].open();
            let _opened = hook_notify[at].pass();
        }
    });
    parts.hooks.settled = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("early to wait on attempt 1", || status(&state).waiting == 1);
    first_gate.open();
    assert!(recorded[0].pass(), "attempt 1's failure is recorded");

    let second = Conn::open(&state, 600);
    second.call();
    assert!(recorded[1].pass(), "attempt 2's failure is recorded");
    let third = Conn::open(&state, 600);
    third.call();
    wait_until("attempt 3 to be building with three calls parked", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 3 && now.waiting == 3
    });
    assert!(
        !early.answered(),
        "nothing has woken the waiter of attempt 1"
    );
    assert!(!second.answered(), "nor the waiter of attempt 2");

    notify[0].open();
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(
        second.outcome(),
        Outcome::Failed("second attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, 3);
    assert_eq!(status(&state).state, BuildState::Building);
    assert!(!third.answered(), "the caller of attempt 3 keeps waiting");

    notify[1].open();
    third_gate.open();
    assert_eq!(third.outcome(), Outcome::Ready);
}

/// A builder may hand over a provider whose generation is not 0. The control plane reports
/// it from the moment of publication. Rules out not recording at publication, which every
/// other test lets through because their builders return a provider at generation 0.
#[test]
fn the_generation_of_a_published_provider_is_recorded_at_publication() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let step_cfg = cfg.clone();
    let step: BuildStep = Box::new(move || -> anyhow::Result<SessionProvider> {
        let mut provider = SessionProvider::bootstrap(&step_cfg)?;
        provider.refresh()?;
        Ok(provider)
    });
    let (builder, _calls) = scripted(&cfg, vec![step]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));
    assert_eq!(status(&state).generation, 1);
}

fn one_call_whose_budget_ends_as_its_attempt_ends(
    step_for: impl Fn(&Arc<Gate>, &ServerConfig) -> BuildStep,
) -> Outcome {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![step_for(&gate, &cfg)]);
    let clock = StepClock::new();
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let shared: Arc<dyn Clock> = clock.clone();
    parts.clock = shared;
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 20);

    conn.call();
    wait_until("the call to be parked", || status(&state).waiting == 1);
    // The deadline passes while the call is parked; nobody wakes it. The only wake-up it
    // gets is the attempt's ending.
    clock.step(20);
    gate.open();
    conn.outcome()
}

/// A waiter whose budget has run out by the time its attempt's ending wakes it is told the
/// outcome, not warming. Rules out checking the deadline before the outcome.
#[test]
fn a_budget_that_ends_as_the_attempt_publishes_is_answered_ready() {
    let outcome = one_call_whose_budget_ends_as_its_attempt_ends(build_after);
    assert_eq!(outcome, Outcome::Ready);
}

/// The same with a failure: the waiter hears it although its deadline has passed. Under the
/// same mutant the failure would reach nobody.
#[test]
fn a_budget_that_ends_as_the_attempt_fails_is_answered_failed() {
    let outcome =
        one_call_whose_budget_ends_as_its_attempt_ends(|gate, _cfg| fail_after(gate, "it failed"));
    assert_eq!(outcome, Outcome::Failed("it failed".to_string()));
}

/// A client that waits zero seconds still hears an attempt that ended before the call could
/// wait: a build thread that cannot be started, and an exhausted counter. Under the same
/// mutant it would be told warming for ever.
#[test]
fn a_zero_wait_client_hears_a_failure_that_is_already_recorded() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    parts.spawner = refusing_spawner();
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 0);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("could not start the index build thread: no more threads".to_string())
    );
    state.lock_control().expect("control").attempts = u64::MAX;
    for _ in 0..2 {
        conn.call();
        assert_eq!(
            conn.outcome(),
            Outcome::Failed(ATTEMPTS_EXHAUSTED.to_string())
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
#[cfg(test)]
#[path = "coordinator_failure_tests.rs"]
mod failure_tests;
````

with this:

````rust
#[cfg(test)]
#[path = "coordinator_failure_tests.rs"]
mod failure_tests;

#[cfg(test)]
#[path = "coordinator_order_tests.rs"]
mod order_tests;

#[cfg(test)]
#[path = "coordinator_edge_tests.rs"]
mod edge_tests;
````

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon::coordinator::order_tests > /tmp/s3a-t4-red.log 2>&1; grep -E '^error' /tmp/s3a-t4-red.log | sort -u
```

Expected: `error[E0599]: no method named `write` found`, with `error[E0282]: type annotations needed` on the closures passed to it. The hook fields and the outcome a waiter reads exist since Task 3, so the publication test, the late-waiter tests and four of the five edge tests would already pass; they are shown to bite in Step 6.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// Copy the generation of the provider in the slot into the control plane, whatever the
````

with this:

````rust
    /// Run `change` on the provider in the slot with exclusive access; `None` when the slot is
    /// empty. This is the only way to the provider's `&mut`. Before it lets go of the slot it
    /// copies the provider's generation into the control plane, so `status` reports the
    /// generation of the provider in the slot once the change is over. That holds whether or
    /// not `Ready` has been announced yet: like `read`, this looks at the slot as it is.
    ///
    /// `change` must not call `read` or `write` again: the slot's lock is not re-entrant, and
    /// a nested call deadlocks. `status` is safe to call from it.
    pub(super) fn write<R>(
        &self,
        change: impl FnOnce(&mut SessionProvider) -> R,
    ) -> Result<Option<R>, Poisoned> {
        let mut slot = self
            .provider
            .write()
            .map_err(|_| Poisoned { lock: "provider" })?;
        let Some(provider) = slot.as_mut() else {
            return Ok(None);
        };
        let result = change(&mut *provider);
        let generation = provider.generation();
        self.record_generation(&slot, generation)?;
        Ok(Some(result))
    }

    /// Copy the generation of the provider in the slot into the control plane, whatever the
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t4-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t4-green.log
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t4-module.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t4-module.log
```

Expected: `test result: ok. 29 passed; 0 failed` (Task 3's seventeen, the seven of `coordinator::order_tests` and the five of `coordinator::edge_tests`), then `34 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/daemon/coordinator.rs src/mcp/daemon/coordinator_order_tests.rs src/mcp/daemon/coordinator_edge_tests.rs
git commit -m "feat(mcp): exclusive provider access that keeps the control plane's generation current"
```

- [ ] **Step 6: Prove the tests bite (after the commit)**

Seven mutants in `src/mcp/daemon/coordinator.rs`, one at a time. After each, run the command and restore the file.

Mutant A (announce, then publish). Replace this:

````rust
        let published = match self.provider.write() {
````

with this:

````rust
        self.settle(attempt, Ok(()));
        let published = match self.provider.write() {
````

Mutant B (a call answered from the current phase, not from its own attempt). Replace this:

````rust
            if let Some(answer) = outcome.answer() {
````

with this:

````rust
            let from_the_phase = match &control.build {
                BuildPhase::Ready => Some(Readiness::Ready),
                BuildPhase::Failed { error, .. } => Some(Readiness::Failed {
                    error: error.clone(),
                }),
                _ => None,
            };
            if let Some(answer) = from_the_phase.or_else(|| outcome.answer().filter(|_| false)) {
````

Mutant C (the generation is not copied). Replace this:

````rust
        control.generation = generation;
````

with this:

````rust
        let _ = (&mut control, generation);
````

Mutant D (the generation is recorded only once the phase is `Ready`). Replace this:

````rust
        control.generation = generation;
````

with this:

````rust
        if matches!(control.build, BuildPhase::Ready) {
            control.generation = generation;
        }
````

Mutant E (the generation read before publication is what gets announced). Two replacements. Replace this:

````rust
                let _recorded = self.record_generation(&slot, generation);
````

with this:

````rust
                let _recorded: Result<(), Poisoned> = Ok(());
````

and replace this:

````rust
        self.settle(attempt, Ok(()));
````

with this:

````rust
        if let Ok(mut control) = self.control.lock() {
            control.generation = generation;
        }
        self.settle(attempt, Ok(()));
````

Mutant F (the generation is not recorded when the provider is stored). Replace this:

````rust
                let _recorded = self.record_generation(&slot, generation);
````

with this:

````rust
                let _recorded: Result<(), Poisoned> = Ok(());
````

Mutant G (the deadline is checked before the outcome). Replace this:

````rust
            if let Some(answer) = outcome.answer() {
                return Ok(answer);
            }
            let now = self.clock.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Readiness::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
````

with this:

````rust
            let now = self.clock.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Readiness::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
            if let Some(answer) = outcome.answer() {
                return Ok(answer);
            }
````

```bash
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t4-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t4-bite.log
git checkout -- src/mcp/daemon/coordinator.rs
```

Expected. A to E were observed on the folded branch; F and G were observed by the controller when the edge tests were added, and again in the S3a-2 validation on the code as it is after Task 10:

| Mutant | Fails |
|---|---|
| A | `the_index_is_in_the_slot_before_ready_is_announced` (the status is already `Ready` at the hook), `a_poisoned_provider_slot_fails_the_attempt_and_is_an_error_for_every_reader`, `exclusive_access_between_publication_and_ready_keeps_the_generation_current` |
| B | the four late-waiter tests: `a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_has_succeeded` (it is told `Ready`), `…_although_the_retry_failed_differently` (it is told attempt 2's error), `…_although_a_third_attempt_is_building` and `a_waiter_that_wakes_after_the_retry_began_still_hears_of_its_own_attempt_s_failure` (it waits on a retry it never joined; 30 s) |
| C | `exclusive_access_records_the_provider_s_generation_in_the_control_plane`, `exclusive_access_between_publication_and_ready_keeps_the_generation_current` |
| D | `exclusive_access_between_publication_and_ready_keeps_the_generation_current` |
| E | `exclusive_access_between_publication_and_ready_keeps_the_generation_current` |
| F | `the_generation_of_a_published_provider_is_recorded_at_publication` (before that test existed this mutant passed every test, because every other builder returns a provider at generation 0) |
| G | `a_budget_that_ends_as_the_attempt_publishes_is_answered_ready`, `a_budget_that_ends_as_the_attempt_fails_is_answered_failed`, `a_zero_wait_client_hears_a_failure_that_is_already_recorded` (before those tests existed this mutant passed every test) |

---

### Task 5: S3a-1 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md` (MCP Adapter section)

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces: the S3a-1 pull request, with the invariants and the exclusions stated.

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.
````

with this:

````markdown
wait above 600 s. A change to what goes into the key or to the preamble's wire text bumps
`SHARED_PROTOCOL_VERSION`.

Shared-daemon core (`src/mcp/daemon/`, Unix only; it has no production caller until the accept
loop of a later slice): `coordinator.rs` holds `DaemonState`, which is a control plane
(`Mutex<ControlState>` and one `Condvar`: the build phase `Idle | Building | Ready | Failed`, the
attempt count, the parked calls, the generation of the provider in the slot) and a provider slot
(`RwLock<Option<SessionProvider>>`). `ensure_ready` picks its build attempt once and waits for
that attempt only, up to the connection's `ConnectionBudget` (`budget.rs`: one deadline per
connection per attempt); it is answered from that attempt's own `AttemptOutcome`, written once,
whatever the phase has become since. A failed attempt is retried by the next call, once however
many calls arrive, and the build thread puts the provider in the slot and records its generation
before it announces `Ready`. The generation is recorded under the slot's write guard at
publication and after every exclusive access, so it is the slot's whatever the phase. Lock order: code holding the control lock
does not touch the slot; code holding the slot may take the control lock. A poisoned lock is the
`Poisoned` error and is never unwrapped; there is no panic boundary yet. Time is the injected
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check
cargo test --features mcp --lib mcp:: > /tmp/s3a1-final-lib.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a1-final-lib.log
env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > /tmp/s3a1-final-mcp.log 2>&1; grep -E '^test result|FAILED|differs|^error' /tmp/s3a1-final-mcp.log
cargo check > /tmp/s3a1-final-default.log 2>&1; tail -1 /tmp/s3a1-final-default.log
cargo check --tests --features mcp,detached-owner-audit > /tmp/s3a1-final-owner.log 2>&1; tail -1 /tmp/s3a1-final-owner.log
cargo clippy --all-targets --features mcp -- -W clippy::all > /tmp/s3a1-final-clippy.log 2>&1
grep -E -A8 '^warning|^error' /tmp/s3a1-final-clippy.log | grep -E 'src/mcp/daemon/|src/mcp/session|src/mcp/mod\.rs' | sort -u
```

Expected: `cargo fmt --check` prints nothing; the lib line shows base + 35 passed (1 + 5 + 17 + 12), `0 failed` (observed: `411 passed` on a base of 380 before the edge tests, `415` with them, the second by the controller); the `mcp` target shows the base number unchanged, which includes the golden transcripts (not run in validation); both `cargo check` runs end with `Finished`. The last command prints exactly one line, `--> src/mcp/session.rs:542:10`: clippy's `filter_map_bool_then` in `plan_refresh`, code this slice does not edit and which has the same expression on the base at line 535 (Task 1 adds seven lines above it). No line names `src/mcp/daemon/` or `src/mcp/mod.rs`.

- [ ] **Step 3: State the invariants**

```bash
BASE=$(git merge-base origin/main HEAD)
git diff --stat "$BASE" -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs | wc -l
git diff --stat "$BASE" -- tests/fixtures/mcp/golden | wc -l
git diff --stat "$BASE" -- src/mcp/transport.rs src/mcp/framing.rs src/mcp/output.rs src/mcp/lazy.rs src/mcp/owner.rs src/mcp/settings.rs src/mcp/shared src/bin/prism-mcp.rs tests/mcp | wc -l
git diff --name-only "$BASE" -- src tests | grep -v -E '^src/mcp/daemon/'
git diff "$BASE" -- src/mcp/session.rs | grep -c -E '^-[^-]'
wc -l src/mcp/daemon/*.rs | sort -n | tail -4
```

Expected: three zeros (the test corpus, the goldens, and every serving file but `session.rs` are untouched); the fourth command prints exactly `src/mcp/mod.rs`, `src/mcp/session.rs` and `src/mcp/session_stage_tests.rs`; the fifth prints `3` (the three comment lines Task 1 replaced; no line of code was removed from `session.rs`); the largest new file is under 600 lines (observed: `coordinator.rs`, 500). All as observed in validation.

- [ ] **Step 4: Run the remaining pre-PR checks**

These are the lane plan's "verification before a slice is called done".

```bash
cargo test --features mcp > /tmp/s3a1-full.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/s3a1-full.log
cargo test --doc --features mcp > /tmp/s3a1-doc.log 2>&1; grep -E '^test result' /tmp/s3a1-doc.log
for i in $(seq 1 30); do cargo test --features mcp --lib mcp::daemon > /tmp/s3a1-repeat-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3a1-repeat-$i.log; done | sort | uniq -c
python3 scripts/mutgate/mutgate.py > /tmp/s3a1-mutgate.log 2>&1; tail -5 /tmp/s3a1-mutgate.log
```

Expected: every `test result` line shows `0 failed` (sum the `passed` counts and report the total next to the Task 1 base numbers); the doc tests pass (this slice adds none); the repeat prints one line, `30 test result: ok. 34 passed; 0 failed` (observed with 30 tests, before the edge tests, in validation, also with `-- --test-threads=1`, under host load; the full suite, the doc tests and the mutation gate were not run there); the mutation gate exits 0. Report the gate as: "full gate green; it builds without `--features mcp`, so it selects no code from this slice." A failure outside this slice's files is reported, not fixed or re-baselined here.

- [ ] **Step 5: Commit and open the PR**

Write the PR body to `/tmp/s3a1-pr-body.md`. It states, in this order:

1. **Goal:** the lane goal from the lane plan, and that S3a-1 adds the coordinator as library code nothing in production calls yet.
2. **Invariants:** the Step 3 output, and that the golden transcripts pass unblessed.
3. **Decisions:** this plan's D-table and spec issues, verbatim or by link. D3, D5, D10, D11 and D15 are the ones a reviewer is most likely to want to change; SI-1 and SI-2 say what the slice's row in spec §12 does not deliver yet.
4. **Claims:** the "Claims and the tests that hold them" table, with the three claims no test holds.
5. **Totals:** the Step 2 and Step 4 numbers next to the Task 1 base numbers, and the Step 6 and Step 9 mutant results of Tasks 3 and 4 as observed.
6. **Not verified:** a non-Unix build (the module is `#[cfg(unix)]`); the mutation gate does not cover MCP code; D7's overflow branch; behavior under a real `std::thread` spawn failure (the injected spawner stands in for it).
7. **Tier-A harness:** not run; this slice touches none of `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`.

Then:

```bash
git add CLAUDE.md
git commit -m "docs(mcp): the shared daemon's readiness coordinator"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): readiness coordinator and provider slot (shared daemon S3a-1)" --body-file /tmp/s3a1-pr-body.md
```

Review brief (lane plan): Opus 5.5 and gpt-6.1-sol, same commit, WRONG/SMELL and MATERIAL/IMMATERIAL tags against the lane goal, a recommended fix plus alternatives per finding, and when each finding would not apply. Cap: two rounds.

---

# Part two: S3a-2, one connection served through the coordinator (Tasks 6–14)

**Base: `origin/main` at `e7dc1cc2`**, the squash merge of S3a-1 (pull request #374). Branch S3a-2 from there. Tasks 7 to 11 edit files S3a-1 created (`src/mcp/daemon/mod.rs`, `coordinator.rs` and, in Tasks 10 and 11, its tests; in Task 11, `budget.rs` and its tests).

### Task 6: One frame loop for both servers

`serve_connection` owns the frame loop today and takes a `&mut impl SessionRuntime`, which a provider behind a lock cannot implement (`session(&self) -> &NavigationSession` cannot borrow from a guard). The loop is extracted so that a daemon connection runs the same one, including the `-32700` handling spec §10 requires to match stdio. This is the slice's only edit to the standalone serving path.

**Files:**
- Modify: `src/mcp/transport.rs`
- Create: `src/mcp/transport_loop_tests.rs`

**Interfaces:**
- Consumes (all `src/mcp/transport.rs` unless noted):
  - `pub(super) fn serve_connection(runtime: &mut impl SessionRuntime, registry: &ToolRegistry, transport: &mut impl Transport, settings: &ConnectionSettings) -> anyhow::Result<()>` (line 103)
  - `fn handle_message_with(message: &Value, runtime: &mut impl SessionRuntime, registry: &ToolRegistry, state: &mut Lifecycle, settings: &ConnectionSettings) -> Dispatch` (line 320), and `enum Dispatch { Response(Value), NoResponse }` (line 290)
  - `pub(super) enum Lifecycle { PreInit, InitializeReceived, Initialized }` (line 64; it derives `PartialEq, Eq` only)
  - `fn error_response(id: Value, code: i64, message: &str) -> Value` (line 958, private)
  - `pub(super) fn handle_control<'r>(message: &Value, state: &mut Lifecycle, registry: &'r ToolRegistry, startup_mode: StartupMode, settings: &ConnectionSettings) -> Control<'r>` (line 355), `pub(super) enum Control<'r> { Response(Value), NoResponse, ToolCall(ToolCall<'r>) }` (line 312), in the tests
  - `pub trait Transport { fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>>; fn write_message(&mut self, v: Value) -> anyhow::Result<()>; }` and `pub enum ReadOutcome { Message(String), Malformed }` (`src/mcp/framing.rs:24`, `:19`)
  - `pub(super) struct LineTransport<R, W>` with `pub(super) fn new(reader: R, writer: W) -> Self` (`src/mcp/framing.rs:30`, `:36`), in the tests
- Produces: `pub(super) fn serve_loop(transport: &mut impl Transport, handle: impl FnMut(&Value, &mut Lifecycle) -> anyhow::Result<Option<Value>>) -> anyhow::Result<()>`.

- [ ] **Step 1: Write the tests**

Create `src/mcp/transport_loop_tests.rs`:

````rust
//! Tests for `serve_loop`, the frame loop the in-process server and a daemon connection share:
//! what the loop answers itself, what it leaves to its handler, and when it ends.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

/// Run `input` (one frame per line) through the loop; return how the loop ended and every
/// response it wrote.
fn run(
    input: &[u8],
    handle: impl FnMut(&Value, &mut Lifecycle) -> anyhow::Result<Option<Value>>,
) -> (anyhow::Result<()>, Vec<Value>) {
    let mut written = Vec::new();
    let outcome = {
        let mut transport = LineTransport::new(std::io::Cursor::new(input.to_vec()), &mut written);
        serve_loop(&mut transport, handle)
    };
    let responses = String::from_utf8(written)
        .expect("responses are UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each response is one JSON line"))
        .collect();
    (outcome, responses)
}

/// Spec §10: invalid JSON gets `-32700` and the connection stays. Rules out handing such a
/// frame to the handler, ending the loop on it, and answering it with nothing.
#[test]
fn a_frame_that_is_not_a_json_value_is_answered_by_the_loop_and_never_reaches_the_handler() {
    let mut seen = Vec::new();
    let (outcome, responses) = run(
        b"this is not json\n\xff\xfe\n{\"ok\":1}\n",
        |message, _state| {
            seen.push(message.clone());
            Ok(None)
        },
    );

    outcome.expect("the loop ends cleanly at end of input");
    assert_eq!(seen, [json!({"ok": 1})], "only the JSON frame was handled");
    assert_eq!(responses.len(), 2, "one answer per frame that is not JSON");
    for response in &responses {
        assert_eq!(response["error"]["code"], -32700);
        assert_eq!(response["error"]["message"], "Parse error");
        assert!(response["id"].is_null());
    }
}

#[test]
fn the_handler_decides_whether_a_frame_is_answered() {
    let (outcome, responses) = run(b"1\n2\n3\n4\n", |message, _state| {
        let number = message.as_u64().unwrap_or(0);
        Ok((number % 2 == 1).then(|| json!({ "odd": number })))
    });

    outcome.expect("the loop ends cleanly at end of input");
    assert_eq!(responses, [json!({"odd": 1}), json!({"odd": 3})]);
}

/// Rules out a lifecycle state made per frame: `tools/list` would then be refused for ever.
#[test]
fn one_lifecycle_state_lasts_the_whole_connection_and_starts_before_initialize() {
    let registry = ToolRegistry::all_v1();
    let settings = ConnectionSettings {
        result_cap: 80_000,
        structured_content: StructuredContentMode::Always,
        concise_shape: ConciseShapeMode::Slim,
    };
    let input = format!("{LIST}\n{INIT}\n{INITED}\n{LIST}\n");
    let (outcome, responses) = run(input.as_bytes(), |message, state| {
        let control = handle_control(message, state, &registry, StartupMode::Lazy, &settings);
        Ok(match control {
            Control::Response(response) => Some(response),
            Control::NoResponse | Control::ToolCall(_) => None,
        })
    });

    outcome.expect("the loop ends cleanly at end of input");
    assert_eq!(responses.len(), 3, "the notification is not answered");
    assert_eq!(
        responses[0]["error"]["code"], -32600,
        "tools/list before initialize: the state starts before the handshake"
    );
    assert!(responses[1]["result"]["instructions"].is_string());
    assert_eq!(
        responses[2]["result"]["tools"].as_array().map(Vec::len),
        Some(9),
        "tools/list after the handshake: the state carried over"
    );
}

/// A daemon's handler fails when a lock is poisoned; the connection must then end. Rules out
/// logging the error and reading on.
#[test]
fn a_handler_error_ends_the_loop_and_nothing_after_it_is_read_or_answered() {
    let mut handled = 0;
    let (outcome, responses) = run(b"1\n2\n3\n", |message, _state| {
        handled += 1;
        if message == &json!(2) {
            anyhow::bail!("stop here");
        }
        Ok(Some(json!({ "echo": message })))
    });

    let error = outcome.expect_err("the handler's error is the loop's");
    assert!(error.to_string().contains("stop here"), "{error}");
    assert_eq!(handled, 2, "the third frame was never handled");
    assert_eq!(responses, [json!({"echo": 1})]);
}

#[test]
fn end_of_input_ends_the_loop_without_calling_the_handler() {
    let mut called = false;
    let (outcome, responses) = run(b"", |_message, _state| {
        called = true;
        Ok(None)
    });

    outcome.expect("an empty stream is a clean end");
    assert!(!called);
    assert!(responses.is_empty());
}
````

In `src/mcp/transport.rs`, replace this:

````rust
#[cfg(test)]
#[path = "transport_split_tests.rs"]
mod split_tests;
````

with this:

````rust
#[cfg(test)]
#[path = "transport_split_tests.rs"]
mod split_tests;

#[cfg(test)]
#[path = "transport_loop_tests.rs"]
mod loop_tests;
````

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::transport::loop_tests > /tmp/s3a-t6-red.log 2>&1; grep -E '^error' /tmp/s3a-t6-red.log | sort -u
```

Expected: `error[E0425]: cannot find function `serve_loop` in this scope`.

- [ ] **Step 3: Implement**

Two replacements. The lines between them (the two `-32700` branches) are not touched, so the diff shows them unmoved.

In `src/mcp/transport.rs`, replace this:

````rust
) -> anyhow::Result<()> {
    let mut state = Lifecycle::PreInit;

    while let Some(outcome) = transport.read_message()? {
````

with this:

````rust
) -> anyhow::Result<()> {
    serve_loop(transport, |message, state| {
        let dispatch = handle_message_with(message, runtime, registry, state, settings);
        Ok(match dispatch {
            Dispatch::Response(response) => Some(response),
            Dispatch::NoResponse => None,
        })
    })
}

/// The frame loop of one connection, in process or in a daemon. It reads one frame at a time
/// until end of input. A frame that is not a JSON value is answered here with `-32700` and the
/// connection stays; every other frame goes to `handle` together with this connection's
/// lifecycle state, which starts at `PreInit` and lives as long as the loop. What `handle`
/// returns is written; `None` writes nothing. An error from `handle`, from a read or from a
/// write ends the loop and is returned.
///
/// `handle` returns an owned value, so whatever it locked to compute a response is released
/// before the response is written.
pub(super) fn serve_loop(
    transport: &mut impl Transport,
    mut handle: impl FnMut(&Value, &mut Lifecycle) -> anyhow::Result<Option<Value>>,
) -> anyhow::Result<()> {
    let mut state = Lifecycle::PreInit;

    while let Some(outcome) = transport.read_message()? {
````

In `src/mcp/transport.rs`, replace this:

````rust
        let response = match handle_message_with(&message, runtime, registry, &mut state, settings)
        {
            Dispatch::Response(response) => Some(response),
            Dispatch::NoResponse => None,
        };
        if let Some(response) = response {
            transport.write_message(response)?;
        }
````

with this:

````rust
        if let Some(response) = handle(&message, &mut state)? {
            transport.write_message(response)?;
        }
````

- [ ] **Step 4: Run the tests, the corpus and the goldens**

```bash
cargo fmt
cargo test --features mcp --lib mcp::transport::loop_tests > /tmp/s3a-t6-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t6-green.log
cargo test --features mcp --lib mcp:: > /tmp/s3a-t6-lib.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t6-lib.log
env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > /tmp/s3a-t6-mcp.log 2>&1; grep -E '^test result|FAILED|differs|^error' /tmp/s3a-t6-mcp.log
cargo check --tests --features mcp,detached-owner-audit > /tmp/s3a-t6-owner.log 2>&1; tail -1 /tmp/s3a-t6-owner.log
git diff --stat -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs tests/fixtures/mcp/golden | wc -l
git diff -- src/mcp/transport.rs > /tmp/s3a-t6-transport.diff; grep -c -E '^[-+].*Parse error' /tmp/s3a-t6-transport.diff
```

Expected: `test result: ok. 5 passed; 0 failed`; the lib total is the S3a-1 total + 5 with `0 failed` (every existing transport test passes unedited); the `mcp` target shows the base number, with the golden transcripts passing unblessed; the owner-audit check ends with `Finished`; the corpus diff is `0`; and the last command prints `0`: neither `-32700` line was added or removed, so the diff of the serving file is the new closure in `serve_connection`, the new function's header and comment, the one changed call, and the test module line. Read `/tmp/s3a-t6-transport.diff` to confirm that is all it is.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/transport.rs src/mcp/transport_loop_tests.rs
git commit -m "refactor(mcp): extract the frame loop so a daemon connection can share it"
```

- [ ] **Step 6: Prove the tests bite (after the commit)**

Three mutants in `src/mcp/transport.rs`, one at a time. After each, run the command and restore the file.

Mutant A (a lifecycle state per frame). Replace this:

````rust
        if let Some(response) = handle(&message, &mut state)? {
````

with this:

````rust
        let mut state = Lifecycle::PreInit;
        if let Some(response) = handle(&message, &mut state)? {
````

Mutant B (a handler error swallowed). Replace this:

````rust
        if let Some(response) = handle(&message, &mut state)? {
````

with this:

````rust
        if let Some(response) = handle(&message, &mut state).unwrap_or(None) {
````

Mutant C (a malformed frame is not answered). In the `ReadOutcome::Malformed` arm of `serve_loop`, delete the line `transport.write_message(error_response(Value::Null, -32700, "Parse error"))?;` (the first of the two identical lines).

```bash
cargo test --features mcp --lib mcp::transport::loop_tests > /tmp/s3a-t6-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t6-bite.log
git checkout -- src/mcp/transport.rs
```

Expected, and observed in validation: A fails `one_lifecycle_state_lasts_the_whole_connection_and_starts_before_initialize` (and warns that the outer `state` is unused); B fails `a_handler_error_ends_the_loop_and_nothing_after_it_is_read_or_answered`; C fails `a_frame_that_is_not_a_json_value_is_answered_by_the_loop_and_never_reaches_the_handler`. Each fails exactly that one test of the five. Mutant A would also fail the golden transcripts; that run is not needed here.

---

### Task 7: One admitted connection

`serve_admitted`: the frame loop, S1's control path, the coordinator's readiness, then S1's dispatch functions under the slot's read or write lock. Tested over connected socket pairs with raw MCP clients; the first test is spec §9 test 1 as far as it can run before S4 and S5a.

**Files:**
- Modify: `src/mcp/daemon/mod.rs`
- Create: `src/mcp/daemon/connection.rs`, `src/mcp/daemon/connection_tests.rs`, `src/mcp/daemon/wire_support.rs`

**Interfaces:**
- Consumes (all `src/mcp/transport.rs` unless noted):
  - `serve_loop` (Task 6)
  - `pub(super) fn handle_control<'r>(message: &Value, state: &mut Lifecycle, registry: &'r ToolRegistry, startup_mode: StartupMode, settings: &ConnectionSettings) -> Control<'r>` (line 355)
  - `pub(super) struct ToolCall<'r>` with `pub(super) fn is_refresh(&self) -> bool` (lines 298, 306)
  - `pub(super) fn readiness_response(call: &ToolCall<'_>, readiness: Readiness, admission: impl FnOnce() -> Option<Value>, settings: &ConnectionSettings) -> Option<Value>` (line 627)
  - `pub(super) fn dispatch_tool_read(call: &ToolCall<'_>, view: &impl SessionRuntime, settings: &ConnectionSettings) -> ReadDispatch` (line 673), `pub(super) enum ReadDispatch { Response(Value), NeedsAutoRefresh { initial_report: Option<FreshnessReport> } }` (line 661)
  - `pub(super) fn dispatch_tool_refresh(call: &ToolCall<'_>, provider: &mut impl SessionRuntime, settings: &ConnectionSettings) -> Value` (line 645)
  - `pub(super) fn dispatch_tool_auto_refresh(call: &ToolCall<'_>, provider: &mut impl SessionRuntime, initial_report: Option<FreshnessReport>, settings: &ConnectionSettings) -> Value` (line 716)
  - `impl SessionRuntime for SessionProvider` (line 208): this is what makes `&SessionProvider` the read-only facade and `&mut SessionProvider` the exclusive path
  - `pub(super) type UnixStreamTransport = LineTransport<BufReader<UnixStream>, BufWriter<UnixStream>>` with `pub(super) fn from_stream(stream: UnixStream) -> std::io::Result<Self>` (`src/mcp/framing.rs:140`, `:146`)
  - `pub fn ClientSettings::output(&self) -> &ConnectionSettings` (`src/mcp/settings.rs:106`); `ConnectionSettings` is `Copy`
  - `DaemonState::{ensure_ready, read, write, startup_mode, status}`, `ConnectionBudget::new` (Tasks 2–4)
- Produces (in `crate::mcp::daemon::connection`): `pub(super) fn serve_admitted(state: &Arc<DaemonState>, registry: &ToolRegistry, stream: UnixStream, client: &ClientSettings) -> anyhow::Result<()>`.

- [ ] **Step 1: Write the test client**

Create `src/mcp/daemon/wire_support.rs`:

````rust
//! A raw MCP client on one end of a connected socket pair, and a daemon connection serving the
//! other end on its own thread. This is what stands in for a shim and an admitted connection
//! until the accept loop (S4) and the shim (S5a) exist.
use super::connection::serve_admitted;
use super::coordinator::DaemonState;
use super::test_support::{wait_until, HANG};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::ClientSettings;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::thread::JoinHandle;

pub(super) const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
pub(super) const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
pub(super) const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
pub(super) const PING: &str = r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#;
pub(super) const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;
pub(super) const REFRESH: &str = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#;
pub(super) const NO_SUCH_TOOL: &str = r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#;

/// One client's end of a connection. Every read gives up after [`HANG`], so a server that
/// stops answering fails the test instead of hanging it.
pub(super) struct Client {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    pub(super) fn new(stream: UnixStream) -> Self {
        stream
            .set_read_timeout(Some(HANG))
            .expect("set the client's read timeout");
        let reader = BufReader::new(stream.try_clone().expect("clone the client socket"));
        Self {
            writer: stream,
            reader,
        }
    }

    /// Send one frame and do not wait for an answer.
    pub(super) fn send(&mut self, line: &str) {
        self.writer
            .write_all(line.as_bytes())
            .expect("write a frame");
        self.writer.write_all(b"\n").expect("end the frame");
    }

    /// The next response line, newline included.
    pub(super) fn recv_line(&mut self) -> String {
        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .expect("a response within the hang limit");
        assert!(
            read > 0,
            "the server closed the connection instead of answering"
        );
        line
    }

    pub(super) fn recv(&mut self) -> Value {
        serde_json::from_str(&self.recv_line()).expect("a JSON response")
    }

    pub(super) fn request(&mut self, line: &str) -> Value {
        self.send(line);
        self.recv()
    }

    pub(super) fn handshake(&mut self) {
        let initialized = self.request(INIT);
        assert!(
            initialized["result"].is_object(),
            "initialize failed: {initialized}"
        );
        self.send(INITED);
    }
}

/// The thread serving the other end, and what its serve loop returned.
pub(super) type Worker = JoinHandle<anyhow::Result<()>>;

/// Open a connection to `state` as a client with these settings.
pub(super) fn connect(state: &Arc<DaemonState>, client: ClientSettings) -> (Client, Worker) {
    let (near, far) = UnixStream::pair().expect("a connected socket pair");
    let state = Arc::clone(state);
    let worker =
        std::thread::spawn(move || serve_admitted(&state, &ToolRegistry::all_v1(), far, &client));
    (Client::new(near), worker)
}

/// Wait for a worker to end and return what its serve loop returned. Panics if the worker
/// panicked or has not ended within [`HANG`].
pub(super) fn ended(worker: Worker) -> anyhow::Result<()> {
    wait_until("the connection's worker to end", || worker.is_finished());
    worker.join().expect("the worker must not panic")
}

/// Close the client's end and require the worker to end cleanly at end of input.
pub(super) fn close(client: Client, worker: Worker) {
    drop(client);
    ended(worker).expect("a connection ends cleanly at end of input");
}
````

- [ ] **Step 2: Write the tests**

Create `src/mcp/daemon/connection_tests.rs`:

````rust
//! Tests for one admitted connection served through the coordinator and the provider slot,
//! over a connected pair of Unix sockets with a raw MCP client. `two_clients_share_one_build`
//! is spec §9 test 1 as far as it can run before the accept loop and the shim exist; the
//! status probe it reads is `DaemonState::status`, called in process.
use super::super::coordinator::{AttemptHook, BuildStatus, DaemonState};
use super::super::test_support::{
    advance, build_after, client_settings, client_settings_with, fail_now, fixture, parts_with,
    scripted, state_with, status, wait_until, Gate, ManualClock,
};
use super::super::wire_support::{
    close, connect, ended, CALLEES, INIT, INITED, LIST, NO_SUCH_TOOL, PING, REFRESH,
};
use crate::mcp::output::StructuredContentMode;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const RUN_CALLEES: &str = "callees:run@main.py";

fn built(builds: &AtomicU64) -> u64 {
    builds.load(Ordering::SeqCst)
}

fn status_of(state: BuildState, attempt: u64, waiting: u64) -> BuildStatus {
    BuildStatus {
        state,
        attempt,
        generation: 0,
        waiting,
    }
}

/// Spec §9 test 1: the builder is blocked on a barrier; the status probe shows `building`,
/// attempt 1; release; B's first call is served from that build. Also: each client's output
/// setting is its own. Rules out a build per client and settings read from the daemon's
/// environment.
#[test]
fn two_clients_share_one_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(
        &state,
        client_settings_with(600, StructuredContentMode::Always),
    );
    let (mut b, b_worker) = connect(
        &state,
        client_settings_with(600, StructuredContentMode::OmitDefaultPath),
    );

    a.handshake();
    b.handshake();
    assert_eq!(
        status(&state),
        status_of(BuildState::Idle, 0, 0),
        "two handshakes build nothing"
    );
    assert_eq!(built(&builds), 0);

    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });
    b.send(CALLEES);
    wait_until("B's call to join it", || status(&state).waiting == 2);
    wait_until("the build thread to call the builder", || {
        built(&builds) == 1
    });
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 2),
        "the status probe shows building, attempt 1"
    );

    gate.open();
    let from_a = a.recv();
    let from_b = b.recv();
    assert_eq!(from_a["id"], 7);
    assert_eq!(from_a["result"]["isError"], false, "{from_a}");
    assert_eq!(from_a["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert!(
        from_b["result"].get("structuredContent").is_none(),
        "B asked for no structuredContent on the default path, and B's setting is B's: {from_b}"
    );
    assert_eq!(status(&state), status_of(BuildState::Ready, 1, 0));
    assert_eq!(
        built(&builds),
        1,
        "B was served from the build A's call started"
    );

    // A client that arrives after the build is served from it too.
    let (mut c, c_worker) = connect(&state, client_settings(600));
    c.handshake();
    let from_c = c.request(CALLEES);
    assert_eq!(from_c["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 1);

    close(a, a_worker);
    close(b, b_worker);
    close(c, c_worker);
}

/// Spec §4.1: a client's handshake must not wait on another client's build. Rules out asking
/// for readiness before the control path has had the message.
#[test]
fn the_handshake_listing_and_ping_are_answered_while_the_build_is_blocked() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });

    // B arrives while the build is blocked. Nothing it does here needs the index.
    let (mut b, b_worker) = connect(&state, client_settings(600));
    let initialized = b.request(INIT);
    assert!(
        initialized["result"]["instructions"]
            .as_str()
            .unwrap_or_default()
            .contains("started by the first tool call"),
        "a lazy daemon advertises how its index is started: {initialized}"
    );
    b.send(INITED);
    let listed = b.request(LIST);
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    assert_eq!(b.request(PING)["result"], json!({}));
    let unknown = b.request(NO_SUCH_TOOL);
    assert_eq!(unknown["result"]["isError"], true, "{unknown}");
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 1),
        "none of that waited on the build"
    );

    gate.open();
    assert_eq!(
        a.recv()["result"]["structuredContent"]["query"],
        RUN_CALLEES
    );
    close(a, a_worker);
    close(b, b_worker);
}

/// The warming result is the in-process server's (spec §5), because it comes from the same
/// `readiness_response`.
#[test]
fn a_client_with_no_wait_is_told_the_index_is_warming_and_its_retry_is_served() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();

    let warming = a.request(CALLEES);
    assert_eq!(warming["id"], 7);
    assert_eq!(warming["result"]["isError"], true, "{warming}");
    assert_eq!(warming["result"]["structuredContent"]["status"], "warming");
    assert_eq!(warming["result"]["_meta"]["prism/index_state"], "warming");
    assert_eq!(warming["result"]["_meta"]["prism/retryable"], true);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(
        built(&builds),
        1,
        "the warming answer started no second build"
    );
    close(a, a_worker);
}

/// Rules out a budget made per message: the second call would wait a second full budget.
#[test]
fn a_connection_s_budget_spans_its_calls_within_one_attempt() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (mut a, a_worker) = connect(&state, client_settings(20));
    a.handshake();

    a.send(CALLEES);
    wait_until("the first call to be waiting", || {
        status(&state).waiting == 1
    });
    advance(&clock, &state, 20);
    let first = a.recv();
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    assert_eq!(first["result"]["structuredContent"]["elapsed_secs"], 20);

    // The budget for this attempt is spent: the next call is answered without waiting. With
    // a budget per message it would wait, and the read below would give up.
    let second = a.request(CALLEES);
    assert_eq!(second["result"]["structuredContent"]["status"], "warming");
    assert_eq!(status(&state).waiting, 0);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 1);
    close(a, a_worker);
}

#[test]
fn a_failed_build_is_reported_as_the_in_process_server_reports_it_and_the_next_call_retries() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![fail_now("no such repository")]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();

    let failed = a.request(CALLEES);
    assert_eq!(failed["result"]["isError"], true, "{failed}");
    assert_eq!(
        failed["result"]["structuredContent"]["status"],
        "build_failed"
    );
    assert_eq!(
        failed["result"]["structuredContent"]["cause"],
        "no such repository"
    );
    assert_eq!(failed["result"]["_meta"]["prism/index_state"], "failed");
    assert_eq!(failed["result"]["_meta"]["prism/retryable"], true);
    assert!(failed["result"]["structuredContent"]
        .get("owner_admission")
        .is_none());
    assert_eq!(status(&state), status_of(BuildState::Failed, 1, 0));

    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(status(&state), status_of(BuildState::Ready, 2, 0));
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
}

/// A client that disconnects cancels nothing: the build goes on, its result is kept, and the
/// next client is served from it. Rules out a build tied to the connection that asked for it
/// and a waiter left counted.
#[test]
fn a_client_that_leaves_while_its_call_waits_leaves_no_waiter_and_the_build_is_kept() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });

    drop(a);
    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    b.send(CALLEES);
    wait_until("B's call to join the build A's call started", || {
        status(&state).waiting == 2
    });
    assert_eq!(status(&state).attempt, 1);

    gate.open();
    assert_eq!(
        b.recv()["result"]["structuredContent"]["query"],
        RUN_CALLEES
    );
    // A's worker wakes with the build's outcome, finds its peer gone and ends: with a write
    // error, or cleanly if the kernel took the bytes. Either way it ends and does not panic.
    let _how_it_ended = ended(a_worker);
    assert_eq!(status(&state), status_of(BuildState::Ready, 1, 0));
    assert_eq!(built(&builds), 1);
    close(b, b_worker);
}

/// What "does not leak a waiter" is bounded by: a departed client's call stays parked until
/// its attempt ends or its own deadline passes, whichever is first, and no longer.
#[test]
fn a_departed_client_s_call_is_parked_no_longer_than_its_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (mut a, a_worker) = connect(&state, client_settings(20));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });
    wait_until("the build thread to call the builder", || {
        built(&builds) == 1
    });

    drop(a);
    advance(&clock, &state, 20);
    let _how_it_ended = ended(a_worker);
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 0),
        "the call is gone and the build goes on"
    );
    assert_eq!(built(&builds), 1);
    gate.open();
}

/// Nothing in the library stops a caller of `DaemonState::read` or `write` from looking into
/// the slot before `Ready` is announced; serving a call is what must not do it. Here the
/// build thread is held after the provider is in the slot and before the announcement. A
/// query and a refresh that arrive then are told the index is warming, and the provider is
/// not touched. Rules out serving from whatever is in the slot, and a `write` reached without
/// `ensure_ready` having returned ready.
#[test]
fn a_call_is_not_served_from_the_slot_before_ready_is_announced() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let reached = Gate::closed();
    let resume = Gate::closed();
    let hook: AttemptHook = {
        let reached = Arc::clone(&reached);
        let resume = Arc::clone(&resume);
        Arc::new(move |_attempt: u64| {
            reached.open();
            let _opened = resume.pass();
        })
    };
    parts.hooks.published = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();

    // The first call starts the build; this client waits zero seconds, so it is answered.
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    assert!(
        reached.pass(),
        "the build thread reaches the publication hook"
    );
    assert_eq!(
        state.read(|_| ()),
        Ok(Some(())),
        "the provider is in the slot"
    );
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "and Ready has not been announced"
    );

    let query = a.request(CALLEES);
    assert_eq!(
        query["result"]["structuredContent"]["status"], "warming",
        "a query is not answered from a slot that has not been announced: {query}"
    );
    let refresh = a.request(REFRESH);
    assert_eq!(
        refresh["result"]["structuredContent"]["status"], "warming",
        "a refresh does not run on it either: {refresh}"
    );
    assert_eq!(
        state.read(|provider| provider.generation()),
        Ok(Some(0)),
        "the provider was not refreshed"
    );
    assert_eq!(status(&state).generation, 0);

    resume.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 1);
    close(a, a_worker);
}
````

Create `src/mcp/daemon/connection.rs`:

````rust
//! One admitted connection of the daemon (spec §4.2 "Per connection", §4.5).

#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod clock;
mod coordinator;
#[cfg(test)]
mod test_support;
````

with this:

````rust
mod clock;
mod connection;
mod coordinator;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod wire_support;
````

- [ ] **Step 3: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon::connection > /tmp/s3a-t7-red.log 2>&1; grep -E '^error' /tmp/s3a-t7-red.log | sort -u
```

Expected: `error[E0432]: unresolved import `super::connection::serve_admitted``.

- [ ] **Step 4: Implement**

Replace the whole of `src/mcp/daemon/connection.rs` with:

````rust
//! One admitted connection of the daemon (spec §4.2 "Per connection", §4.5).
//!
//! A connection is the frame loop the in-process server runs (`serve_loop`), the same control
//! path (`handle_control`: lifecycle, `tools/list`, call validation; no index, no lock), and
//! for a validated tool call the coordinator's readiness followed by the same three dispatch
//! functions, under the provider slot's read or write lock.
//!
//! What this slice leaves to S3b: under an automatic refresh policy the trigger is not
//! re-checked after exclusive access is acquired, so two clients that both find the index stale
//! each refresh it; a query that follows a refresh runs while the slot is still held
//! exclusively; and there is no panic boundary.
use super::budget::ConnectionBudget;
use super::coordinator::DaemonState;
use crate::mcp::framing::{Transport, UnixStreamTransport};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::transport::{
    dispatch_tool_auto_refresh, dispatch_tool_read, dispatch_tool_refresh, handle_control,
    readiness_response, serve_loop, Control, ReadDispatch, ToolCall,
};
use serde_json::Value;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// Serve one admitted connection until its peer closes it or it fails.
///
/// `stream` is the connection after the preamble: blocking, with no read timeout (a timeout
/// would end the connection as a read error). `client` is what this client sent in its
/// `HELLO`; its output settings and its first-call wait apply to this connection only.
///
/// Returns `Ok` at end of input. Returns an error when a read or write fails (the peer went
/// away mid-frame or stopped reading) or when a daemon lock is poisoned; the caller closes the
/// socket either way.
pub(super) fn serve_admitted(
    state: &Arc<DaemonState>,
    registry: &ToolRegistry,
    stream: UnixStream,
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_client(state, registry, &mut transport, client)
}

fn serve_client(
    state: &Arc<DaemonState>,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let settings: ConnectionSettings = *client.output();
    let mut budget = ConnectionBudget::new(client);
    let startup = state.startup_mode();
    serve_loop(transport, |message, lifecycle| {
        match handle_control(message, lifecycle, registry, startup, &settings) {
            Control::Response(response) => Ok(Some(response)),
            Control::NoResponse => Ok(None),
            Control::ToolCall(call) => {
                run_tool_call(state, &mut budget, &call, &settings).map(Some)
            }
        }
    })
}

/// Run one validated tool call: wait for the index within this connection's budget, then
/// query it under the read lock, or refresh it under the write lock. The value returned is
/// owned, so every lock taken here is released before the response is written.
fn run_tool_call(
    state: &Arc<DaemonState>,
    budget: &mut ConnectionBudget,
    call: &ToolCall<'_>,
    settings: &ConnectionSettings,
) -> anyhow::Result<Value> {
    let readiness = state.ensure_ready(budget)?;
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
        return Ok(response);
    }

    if call.is_refresh() {
        let refreshed = state.write(|provider| dispatch_tool_refresh(call, provider, settings))?;
        return published(refreshed);
    }

    // The query sees the whole provider, so the stale evidence a refresh recorded takes
    // precedence over a fresh stamp probe exactly as it does in process.
    let read = state.read(|provider| dispatch_tool_read(call, provider, settings))?;
    let initial_report = match published(read)? {
        ReadDispatch::Response(response) => return Ok(response),
        ReadDispatch::NeedsAutoRefresh { initial_report } => initial_report,
    };
    // The read lock is released: `read` has returned. Exclusive access is taken afresh.
    let refreshed = state.write(|provider| {
        dispatch_tool_auto_refresh(call, provider, initial_report, settings)
    })?;
    published(refreshed)
}

/// `Ready` is announced only after publication and is never left, so a ready call always
/// finds a provider. If it does not, that is a defect of the coordinator; the connection ends
/// with this error instead of a panic.
fn published<T>(outcome: Option<T>) -> anyhow::Result<T> {
    match outcome {
        Some(value) => Ok(value),
        None => anyhow::bail!("the index is announced ready and no index is published"),
    }
}

#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
````

- [ ] **Step 5: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon::connection > /tmp/s3a-t7-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t7-green.log
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t7-module.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t7-module.log
cargo check --features mcp > /tmp/s3a-t7-check.log 2>&1; grep -A4 -E '^warning' /tmp/s3a-t7-check.log | grep -E 'src/mcp/(daemon/|transport\.rs|framing\.rs)' | sort -u
```

Expected: `test result: ok. 8 passed; 0 failed`, then `42 passed; 0 failed` for the module (34 + 8). The third command prints nothing.

- [ ] **Step 6: Commit**

```bash
git add src/mcp/daemon
git commit -m "feat(mcp): serve one admitted connection through the daemon's coordinator"
```

- [ ] **Step 7: Prove the tests bite (after the commit)**

Five mutants in `src/mcp/daemon/connection.rs`, one at a time. After each, run the command and restore the file.

Mutant A (a budget per message). In the call to `run_tool_call` inside `serve_client`, replace this:

````rust
&mut budget, &call
````

with this:

````rust
&mut ConnectionBudget::new(client), &call
````

Mutant B (the daemon's own environment decides a client's output). Replace this:

````rust
    let settings: ConnectionSettings = *client.output();
````

with this:

````rust
    let settings: ConnectionSettings = ConnectionSettings::from_env();
````

Mutant C (every daemon advertises an eager index). Replace this:

````rust
    let startup = state.startup_mode();
````

with this:

````rust
    let startup = crate::mcp::StartupMode::Eager;
````

Mutant D (readiness is not answered). Replace this:

````rust
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
        return Ok(response);
    }
````

with this:

````rust
    let _unanswered = readiness_response(call, readiness, || None, settings);
````

Mutant E (a call is served from whatever is in the slot, announced or not). Replace this:

````rust
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
        return Ok(response);
    }
````

with this:

````rust
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
        if state.read(|_| ())?.is_none() {
            return Ok(response);
        }
    }
````

```bash
cargo test --features mcp --lib mcp::daemon::connection > /tmp/s3a-t7-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t7-bite.log
git checkout -- src/mcp/daemon/connection.rs
```

Expected, and observed in validation (on the code after Task 10, where `connection::tests` has nine tests):

| Mutant | Fails |
|---|---|
| A | `a_connection_s_budget_spans_its_calls_within_one_attempt` (at the 30 s hang limit) |
| B | 11 of the tests of `connection`, every one that looks at a result's shape: the environment's default is not what these clients asked for. Among them `two_clients_share_one_build` |
| C | `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server`, `the_handshake_listing_and_ping_are_answered_while_the_build_is_blocked` |
| D | `a_call_is_not_served_from_the_slot_before_ready_is_announced`, `a_client_with_no_wait_is_told_the_index_is_warming_and_its_retry_is_served`, `a_connection_s_budget_spans_its_calls_within_one_attempt`, `a_failed_build_is_reported_as_the_in_process_server_reports_it_and_the_next_call_retries` |
| E | `a_call_is_not_served_from_the_slot_before_ready_is_announced` |

**The test that holds "nothing is served before `ensure_ready` has answered ready".** Nothing in the library stops a caller of `DaemonState::read` or `write` from looking into the slot before `Ready` is announced (both S3a-1 reviewers noted it); `run_tool_call` is the convention that does not. `a_call_is_not_served_from_the_slot_before_ready_is_announced` holds it: it stops the build thread after the provider is in the slot and before the announcement, sends a query and a `refresh_index`, and requires both to be answered `warming` and the provider to be at generation 0 afterwards. Mutant E passes every other test and fails this one; mutant D (which drops the readiness answer altogether) fails it too.

---

### Task 8: Parity with the in-process server, and sticky stale evidence

Proof tests for the connection Task 7 built: a whole session answered byte for byte like the in-process server (spec §5 "Preserved"), spec §9 test 14 under both kinds of refresh policy, and the two ways a connection ends with an error instead of a panic.

The red step of this task is a compile failure for two support items the tests need. The behavior under test already exists after Task 7, so the tests are expected to pass on their first run; Step 6's mutants are what shows each can fail.

**Files:**
- Create: `src/mcp/daemon/connection_parity_tests.rs`
- Modify: `src/mcp/daemon/connection.rs` (one `mod` line), `src/mcp/daemon/wire_support.rs` (append), `src/mcp/daemon/coordinator.rs` (one test-only method)

**Interfaces:**
- Consumes:
  - `serve_admitted`, `DaemonState::{new, read, write, status}`, `wire_support::{Client, connect, close, ended}` (Tasks 3–7)
  - `pub(super) fn serve_connection(runtime: &mut impl SessionRuntime, registry: &ToolRegistry, transport: &mut impl Transport, settings: &ConnectionSettings) -> anyhow::Result<()>` (`src/mcp/transport.rs:103`), with `impl SessionRuntime for LazySessionProvider` (`src/mcp/transport.rs:242`)
  - `#[cfg(test)] pub(crate) fn LazySessionProvider::with_builder(cfg: &ServerConfig, wait: Duration, builder: SessionBuilder) -> anyhow::Result<Self>` (`src/mcp/lazy.rs:69`)
  - `#[cfg(test)] pub(crate) fn SessionProvider::force_next_verification_for_tests(&mut self, verification: RefreshVerification)` (`src/mcp/session.rs:203`); `pub(crate) enum RefreshVerification { Clean, Diverged(FreshnessReport) }` (`src/mcp/session.rs:95`, re-exported as `crate::mcp::RefreshVerification`)
  - `pub(crate) fn FreshnessReport::from_changed_paths<I: IntoIterator<Item = String>>(paths: I) -> Self` (`src/mcp/freshness.rs:96`); `pub fn FreshnessProbe::check(&self) -> FreshnessReport` (`src/mcp/freshness.rs:76`); `pub fn SessionProvider::freshness(&self) -> &FreshnessProbe` (`src/mcp/session.rs:190`)
  - The result metadata keys `prism/index_freshness`, `prism/stale_index_paths` (`src/mcp/freshness.rs:14`, `:16`), `prism/auto_refresh`, `prism/refresh_generation` (`src/mcp/transport.rs:817`, `:821`)
- Produces: tests only; `#[cfg(test)] pub(super) fn unpublish_for_tests(&self)` on `DaemonState`; `Client::expect_closed`; the constant `CALLERS`.

- [ ] **Step 1: Write the tests**

Create `src/mcp/daemon/connection_parity_tests.rs`:

````rust
//! Tests that a daemon connection is the in-process server for one client (spec §5
//! "Preserved"), that the stale evidence a refresh recorded reaches every client (spec §9
//! test 14), and that a broken daemon ends a connection with an error and not a panic.
use super::super::coordinator::DaemonState;
use super::super::test_support::{client_settings, fixture, scripted, status};
use super::super::wire_support::{
    close, connect, ended, Client, CALLEES, CALLERS, INIT, INITED, LIST, REFRESH,
};
use crate::mcp::framing::UnixStreamTransport;
use crate::mcp::freshness::FreshnessReport;
use crate::mcp::lazy::{LazySessionProvider, SessionBuilder};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::transport::serve_connection;
use crate::mcp::{RefreshPolicy, RefreshVerification, SessionProvider, StartupMode};
use serde_json::json;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const RUN_CALLEES: &str = "callees:run@main.py";

enum Step {
    /// Send this line and read exactly one response line.
    Request(&'static str),
    /// Send this line; nothing answers it.
    Notify(&'static str),
    /// Overwrite a fixture file, so the index becomes stale.
    Edit {
        file: &'static str,
        contents: &'static str,
    },
}

/// Garbage, a call before the handshake, the handshake, listing, a query (the first call that
/// needs the index, so it starts the build and waits for it), every kind of refused call, an
/// edit that makes the index stale, a query on the stale index, a refresh, a query again, an
/// unknown method and a request with no id.
fn session() -> Vec<Step> {
    vec![
        Step::Request("this is not json"),
        Step::Request("[]"),
        Step::Request(r#"{"jsonrpc":"2.0","id":90,"method":"tools/list"}"#),
        Step::Request(INIT),
        Step::Notify(INITED),
        Step::Request(r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#),
        Step::Request(LIST),
        Step::Request(CALLEES),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#,
        ),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"nav_callees","arguments":[]}}"#,
        ),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"bogus"}}}}"#,
        ),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"refresh_index","arguments":{"x":1}}}"#,
        ),
        Step::Edit {
            file: "util.py",
            contents: "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
        },
        Step::Request(CALLERS),
        Step::Request(REFRESH),
        Step::Request(CALLERS),
        Step::Request(r#"{"jsonrpc":"2.0","id":21,"method":"no/such/method"}"#),
        Step::Request(r#"{"jsonrpc":"2.0","method":"tools/list"}"#),
    ]
}

/// Play `steps` as one client and return every response line, in order.
fn play(client: &mut Client, root: &Path, steps: &[Step]) -> String {
    let mut transcript = String::new();
    for step in steps {
        match step {
            Step::Request(line) => {
                client.send(line);
                transcript.push_str(&client.recv_line());
            }
            Step::Notify(line) => client.send(line),
            Step::Edit { file, contents } => {
                std::fs::write(root.join(*file), *contents).expect("edit a fixture file");
            }
        }
    }
    transcript
}

/// Spec §5: for the same snapshot, the same connection settings and the same readiness state,
/// a daemon answers what the in-process server answers. Both sides here are lazy, wait up to
/// 60 s for the build their first query starts, and serve a repository with the same content.
/// Rules out any difference in how a daemon connection composes control, readiness and
/// dispatch, down to one byte of one response.
#[test]
fn a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server() {
    for policy in [RefreshPolicy::WarnOnly, RefreshPolicy::AutoFull] {
        let settings = client_settings(60);
        let steps = session();
        let requests = steps
            .iter()
            .filter(|step| matches!(step, Step::Request(_)))
            .count();

        // In process: the lazy provider behind `serve_connection`, over a socket pair.
        let (in_process_dir, in_process_cfg) = fixture(policy);
        let (near, far) = UnixStream::pair().expect("a connected socket pair");
        let output = *settings.output();
        let in_process = std::thread::spawn(move || {
            let builder_cfg = in_process_cfg.clone();
            let builder: SessionBuilder =
                Arc::new(move || SessionProvider::bootstrap(&builder_cfg));
            let mut provider =
                LazySessionProvider::with_builder(&in_process_cfg, Duration::from_secs(60), builder)
                    .expect("a lazy provider");
            let mut transport = UnixStreamTransport::from_stream(far).expect("a transport");
            serve_connection(
                &mut provider,
                &ToolRegistry::all_v1(),
                &mut transport,
                &output,
            )
        });
        let mut client = Client::new(near);
        let expected = play(&mut client, in_process_dir.path(), &steps);
        drop(client);
        ended(in_process).expect("the in-process server ends cleanly at end of input");

        // The daemon: the same session on a connection served through the coordinator.
        let (daemon_dir, daemon_cfg) = fixture(policy);
        let (builder, builds) = scripted(&daemon_cfg, vec![]);
        let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
        let (mut client, worker) = connect(&state, settings);
        let actual = play(&mut client, daemon_dir.path(), &steps);
        close(client, worker);

        assert_eq!(
            expected.lines().count(),
            requests,
            "policy {policy:?}: one response per request"
        );
        for (index, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
            assert_eq!(
                got,
                want,
                "policy {policy:?}: response {} differs",
                index + 1
            );
        }
        assert_eq!(actual.len(), expected.len(), "policy {policy:?}");
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "policy {policy:?}: one build for the whole session"
        );
        // Warn-only: the explicit refresh. Automatic: the stale query refreshed first.
        let refreshes = if policy == RefreshPolicy::WarnOnly { 1 } else { 2 };
        assert_eq!(status(&state).generation, refreshes, "policy {policy:?}");
    }
}

fn force_the_next_refresh_to_verify_diverged(state: &DaemonState) {
    state
        .write(|provider| {
            provider.force_next_verification_for_tests(RefreshVerification::Diverged(
                FreshnessReport::from_changed_paths(["main.py".to_string()]),
            ));
        })
        .expect("the provider lock is not poisoned")
        .expect("an index is published");
}

/// Spec §9 test 14, first half: a refresh that verifies `Diverged`, followed by a clean stamp
/// probe; B's next query still carries the stale warning. Rules out a read view narrower than
/// the whole provider (the review finding N2 of the spec: a view of the session and the
/// freshness probe alone drops the evidence).
#[test]
fn sticky_stale_evidence_reaches_another_client_although_the_stamp_probe_is_clean() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    // A's refresh verifies Diverged: the tree moved while the snapshot was being built.
    force_the_next_refresh_to_verify_diverged(&state);
    let refreshed = a.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "raced_stale",
        "{refreshed}"
    );
    assert_eq!(refreshed["result"]["structuredContent"]["generation"], 1);
    assert_eq!(status(&state).generation, 1);

    // The stamp probe is clean: nothing on disk differs from what the new snapshot recorded.
    assert_eq!(
        state.read(|provider| provider.freshness().check().stale),
        Ok(Some(false))
    );

    // B refreshed nothing and asked for nothing yet. Its next query carries the warning, and
    // the warning is the refresh's evidence (a clean probe has no path to name).
    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(
        from_b["result"]["_meta"]["prism/index_freshness"], "stale",
        "{from_b}"
    );
    assert_eq!(
        from_b["result"]["_meta"]["prism/stale_index_paths"],
        json!(["main.py"]),
        "{from_b}"
    );

    // It stays until a refresh verifies clean, for every client.
    let again = a.request(CALLEES);
    assert_eq!(again["result"]["_meta"]["prism/index_freshness"], "stale");

    close(a, a_worker);
    close(b, b_worker);
}

/// Spec §9 test 14, second half: "and auto-refresh retries". Under an automatic policy the
/// sticky evidence is what makes B's query refresh, although the stamp probe is clean.
#[test]
fn under_an_automatic_policy_sticky_stale_evidence_makes_the_next_client_s_query_refresh_again() {
    let (_dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert!(
        first["result"]["_meta"].get("prism/auto_refresh").is_none(),
        "a fresh index is not refreshed: {first}"
    );

    force_the_next_refresh_to_verify_diverged(&state);
    let refreshed = a.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "raced_stale",
        "{refreshed}"
    );
    assert_eq!(status(&state).generation, 1);
    assert_eq!(
        state.read(|provider| provider.freshness().check().stale),
        Ok(Some(false))
    );

    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(
        from_b["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "{from_b}"
    );
    assert_eq!(from_b["result"]["_meta"]["prism/refresh_generation"], 2);
    assert_eq!(status(&state).generation, 2);

    // That refresh verified clean, so the evidence is gone for every client.
    let again = a.request(CALLEES);
    assert!(
        again["result"]["_meta"].get("prism/auto_refresh").is_none(),
        "{again}"
    );
    assert!(
        again["result"]["_meta"]
            .get("prism/index_freshness")
            .is_none(),
        "{again}"
    );
    assert_eq!(status(&state).generation, 2);

    close(a, a_worker);
    close(b, b_worker);
}

/// The coordinator never announces `Ready` over an empty slot. If it ever did, a client's call
/// must not panic the daemon. Rules out `expect` on the slot.
#[test]
fn ready_with_nothing_published_ends_the_connection_with_an_error_not_a_panic() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert_eq!(status(&state).state, BuildState::Ready);

    state.unpublish_for_tests();
    a.send(CALLEES);
    a.expect_closed();
    let error = ended(a_worker).expect_err("the connection ends with an error");
    assert!(
        error.to_string().contains("no index is published"),
        "{error}"
    );
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
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    let poisoner = Arc::clone(&state);
    let panicked = std::thread::spawn(move || {
        let _ = poisoner.write(|_provider| -> () {
            panic!("poisoning the provider slot for a test");
        });
    })
    .join();
    assert!(panicked.is_err());

    a.send(CALLEES);
    a.expect_closed();
    let error = ended(a_worker).expect_err("the connection ends with an error");
    assert!(
        error.to_string().contains("provider lock is poisoned"),
        "{error}"
    );

    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let listed = b.request(LIST);
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    close(b, b_worker);
}
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
````

with this:

````rust
#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "connection_parity_tests.rs"]
mod parity_tests;
````

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon::connection::parity_tests > /tmp/s3a-t8-red.log 2>&1; grep -E '^error' /tmp/s3a-t8-red.log | sort -u
```

Expected: an unresolved import `CALLERS`, and no method named `unpublish_for_tests` or `expect_closed`.

- [ ] **Step 3: Add the two support items**

Append to `src/mcp/daemon/wire_support.rs`:

````rust

pub(super) const CALLERS: &str = r#"{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"helper","file":"util.py"}}}}"#;

impl Client {
    /// Require that the server closes the connection without sending anything more.
    pub(super) fn expect_closed(&mut self) {
        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .expect("end of stream within the hang limit");
        assert_eq!(read, 0, "expected the connection to be closed, got: {line}");
    }
}
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// Wake every parked call so that it looks at the clock again.
````

with this:

````rust
    /// Empty the provider slot and leave the phase as it is. Only for the test of what a
    /// connection does if `Ready` were ever announced over an empty slot.
    #[cfg(test)]
    pub(super) fn unpublish_for_tests(&self) {
        if let Ok(mut slot) = self.provider.write() {
            *slot = None;
        }
    }

    /// Wake every parked call so that it looks at the clock again.
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon::connection::parity_tests > /tmp/s3a-t8-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t8-green.log
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t8-module.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t8-module.log
```

Expected: `test result: ok. 5 passed; 0 failed`, then `47 passed; 0 failed` for the module (42 + 5). If `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server` fails, the message names the policy and the first response that differs; that is a defect in Task 7's composition, not in the test. Do not relax the comparison.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/daemon
git commit -m "test(mcp): a daemon connection matches the in-process server and keeps sticky stale evidence"
```

- [ ] **Step 6: Prove the tests bite (after the commit)**

Four mutants, one at a time. After each, run the command and restore the file named.

Mutant A (sticky evidence loses to the stamp probe), in `src/mcp/transport.rs`. Replace this:

````rust
    runtime
        .known_stale_after_refresh()
        .cloned()
        .or_else(|| runtime.freshness().map(FreshnessProbe::check))
````

with this:

````rust
    runtime.freshness().map(FreshnessProbe::check)
````

Mutant B (`expect` on the slot), in `src/mcp/daemon/connection.rs`. Replace this:

````rust
        None => anyhow::bail!("the index is announced ready and no index is published"),
````

with this:

````rust
        None => panic!("the index is announced ready and no index is published"),
````

Mutant C (every daemon advertises an eager index), in `src/mcp/daemon/connection.rs`: Task 7's mutant C.

Mutant D (the generation is not copied), in `src/mcp/daemon/coordinator.rs`: Task 4's mutant C.

```bash
cargo test --features mcp --lib mcp::daemon::connection::parity_tests > /tmp/s3a-t8-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t8-bite.log
git checkout -- src/mcp/transport.rs src/mcp/daemon/connection.rs src/mcp/daemon/coordinator.rs
```

Expected, and observed in validation (on the code after Task 10):

| Mutant | Fails |
|---|---|
| A | `sticky_stale_evidence_reaches_another_client_although_the_stamp_probe_is_clean`, `under_an_automatic_policy_sticky_stale_evidence_makes_the_next_client_s_query_refresh_again`. It is a mutant of S1's code; it shows the daemon's read path consumes that precedence and nothing else supplies it |
| B | `ready_with_nothing_published_ends_the_connection_with_an_error_not_a_panic` (the worker panics) |
| C | `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server`, at response 4, `initialize` |
| D | `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server`, `sticky_stale_evidence_reaches_another_client_although_the_stamp_probe_is_clean`, `under_an_automatic_policy_sticky_stale_evidence_makes_the_next_client_s_query_refresh_again` |

`unpublish_for_tests` empties the slot without recording a generation, so in that one test the control plane keeps the last generation for an empty slot, against the field's own doc. It is test-only and the state it makes is one the coordinator rules out.

---

### Task 9: Move the control-plane types into their own file

`coordinator.rs` is 549 lines after Task 8 and Task 10 adds to it; both reviewers of S3a-1 asked that the outcome, phase and status types be split from `DaemonState` before the file is extended. This task is that split and nothing else: `AttemptOutcome`, `BuildPhase`, `ControlState` and `BuildStatus` move to `src/mcp/daemon/control.rs` with their comments, unchanged except that `AttemptOutcome::fill` and `answer` become `pub(super)` (they are now called from another file). `coordinator.rs` re-exports the four names, so no path in any other file changes, and imports `BuildState` only for its tests, which name it through `use super::*`.

**Files:**
- Create: `src/mcp/daemon/control.rs`
- Modify: `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/mod.rs`

**Interfaces:**
- Consumes: Task 3's types as they are.
- Produces: the same types at `crate::mcp::daemon::control`, re-exported from `crate::mcp::daemon::coordinator`.

- [ ] **Step 1: Move the types**

Create `src/mcp/daemon/control.rs`:

````rust
//! What the control lock guards, and how an attempt's ending is kept for the calls that joined
//! it (spec §4.2). These are the types only: `coordinator.rs` holds the lock and makes every
//! change to them.
use crate::mcp::lazy::Readiness;
use crate::mcp::shared::preamble::BuildState;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

/// How one build attempt ended: empty while it builds, filled when it ends, and not changed
/// after that (a second fill has no effect). The attempt's phase holds one share of it and
/// every call that starts or joins the attempt holds another, so a call reads the outcome of
/// its own attempt however far the phase has moved on by the time the call runs again.
#[derive(Debug, Default)]
pub(super) struct AttemptOutcome {
    ended: OnceLock<Result<(), String>>,
}

impl AttemptOutcome {
    /// Record how the attempt ended. Only the first call has any effect.
    pub(super) fn fill(&self, ended: Result<(), String>) {
        let _ = self.ended.set(ended);
    }

    /// What a call that belongs to this attempt is told; `None` while it is still building.
    pub(super) fn answer(&self) -> Option<Readiness> {
        self.ended.get().map(|ended| match ended {
            Ok(()) => Readiness::Ready,
            Err(error) => Readiness::Failed {
                error: error.clone(),
            },
        })
    }
}

/// Where the index build is (spec §4.2).
#[derive(Debug)]
pub(super) enum BuildPhase {
    /// No build has been asked for.
    Idle,
    /// Attempt `attempt` is running since `started`. `outcome` is that attempt's, still empty.
    Building {
        attempt: u64,
        started: Instant,
        outcome: Arc<AttemptOutcome>,
    },
    /// A provider is in the slot and calls have been told so. Its generation is
    /// [`ControlState::generation`], which is kept apart from the phase because the slot is
    /// filled, and can be written to, before this phase is announced.
    Ready,
    /// Attempt `attempt` ended without a provider. The next readiness call starts the retry.
    Failed { attempt: u64, error: String },
}

impl BuildPhase {
    /// The word `READY` and the status probe use for this phase.
    pub(super) fn state(&self) -> BuildState {
        match self {
            Self::Idle => BuildState::Idle,
            Self::Building { .. } => BuildState::Building,
            Self::Ready => BuildState::Ready,
            Self::Failed { .. } => BuildState::Failed,
        }
    }
}

/// What the control lock guards. Slice S4 adds the admission counters and the retirement
/// decision beside the build phase, under the same lock.
pub(super) struct ControlState {
    pub(super) build: BuildPhase,
    /// How many attempts have been started; also the number of the latest one.
    pub(super) attempts: u64,
    /// How many calls are parked on the condition variable right now.
    pub(super) waiting: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty. It is written by
    /// `record_generation` and by nothing else, each time while the slot's write guard is
    /// held: when the provider is stored, and at the end of every exclusive access. It does
    /// not depend on the phase, so it is true in the interval between publication and the
    /// announcement of `Ready` as well.
    pub(super) generation: u64,
}

/// A snapshot of the control plane: what the status probe reports about the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildStatus {
    pub(super) state: BuildState,
    /// The number of the latest attempt; 0 before the first.
    pub(super) attempt: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty.
    pub(super) generation: u64,
    /// Calls parked on the build at the moment of the snapshot.
    pub(super) waiting: u64,
}
````

In `src/mcp/daemon/coordinator.rs`, delete this:

````rust
/// How one build attempt ended: empty while it builds, filled when it ends, and not changed
/// after that (a second fill has no effect). The attempt's phase holds one share of it and
/// every call that starts or joins the attempt holds another, so a call reads the outcome of
/// its own attempt however far the phase has moved on by the time the call runs again.
#[derive(Debug, Default)]
pub(super) struct AttemptOutcome {
    ended: OnceLock<Result<(), String>>,
}

impl AttemptOutcome {
    /// Record how the attempt ended. Only the first call has any effect.
    fn fill(&self, ended: Result<(), String>) {
        let _ = self.ended.set(ended);
    }

    /// What a call that belongs to this attempt is told; `None` while it is still building.
    fn answer(&self) -> Option<Readiness> {
        self.ended.get().map(|ended| match ended {
            Ok(()) => Readiness::Ready,
            Err(error) => Readiness::Failed {
                error: error.clone(),
            },
        })
    }
}

/// Where the index build is (spec §4.2).
#[derive(Debug)]
pub(super) enum BuildPhase {
    /// No build has been asked for.
    Idle,
    /// Attempt `attempt` is running since `started`. `outcome` is that attempt's, still empty.
    Building {
        attempt: u64,
        started: Instant,
        outcome: Arc<AttemptOutcome>,
    },
    /// A provider is in the slot and calls have been told so. Its generation is
    /// [`ControlState::generation`], which is kept apart from the phase because the slot is
    /// filled, and can be written to, before this phase is announced.
    Ready,
    /// Attempt `attempt` ended without a provider. The next readiness call starts the retry.
    Failed { attempt: u64, error: String },
}

impl BuildPhase {
    /// The word `READY` and the status probe use for this phase.
    pub(super) fn state(&self) -> BuildState {
        match self {
            Self::Idle => BuildState::Idle,
            Self::Building { .. } => BuildState::Building,
            Self::Ready => BuildState::Ready,
            Self::Failed { .. } => BuildState::Failed,
        }
    }
}

/// What the control lock guards. Slice S4 adds the admission counters and the retirement
/// decision beside the build phase, under the same lock.
pub(super) struct ControlState {
    pub(super) build: BuildPhase,
    /// How many attempts have been started; also the number of the latest one.
    pub(super) attempts: u64,
    /// How many calls are parked on the condition variable right now.
    pub(super) waiting: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty. It is written by
    /// `record_generation` and by nothing else, each time while the slot's write guard is
    /// held: when the provider is stored, and at the end of every exclusive access. It does
    /// not depend on the phase, so it is true in the interval between publication and the
    /// announcement of `Ready` as well.
    pub(super) generation: u64,
}

/// A snapshot of the control plane: what the status probe reports about the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildStatus {
    pub(super) state: BuildState,
    /// The number of the latest attempt; 0 before the first.
    pub(super) attempt: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty.
    pub(super) generation: u64,
    /// Calls parked on the build at the moment of the snapshot.
    pub(super) waiting: u64,
}
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
use super::clock::{Clock, SystemClock};
use crate::mcp::lazy::{Readiness, SessionBuilder};
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, RwLock, RwLockWriteGuard};
````

with this:

````rust
use super::clock::{Clock, SystemClock};
pub(super) use super::control::{AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use crate::mcp::lazy::{Readiness, SessionBuilder};
// The tests in this file's child modules name `BuildState` through `use super::*`.
#[cfg(test)]
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
````

In `src/mcp/daemon/mod.rs`, replace this:

````rust
mod connection;
mod coordinator;
````

with this:

````rust
mod connection;
mod control;
mod coordinator;
````

- [ ] **Step 2: Run the tests; nothing else changed**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t9-move.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t9-move.log
cargo check --features mcp > /tmp/s3a-t9-check.log 2>&1; grep -c '^warning' /tmp/s3a-t9-check.log
git status --short
wc -l src/mcp/daemon/coordinator.rs src/mcp/daemon/control.rs
```

Expected: `47 passed; 0 failed`, the same 47 as after Task 8; `0` warnings; `git status` lists `coordinator.rs`, `mod.rs` and the new `control.rs` and no test file; `coordinator.rs` is 467 lines and `control.rs` 92. There is no "prove the tests bite" step: no behavior and no test changed.

- [ ] **Step 3: Commit**

```bash
git add src/mcp/daemon
git commit -m "refactor(mcp): move the control-plane types out of coordinator.rs"
```

---

### Task 10: A call is told to retry when a retry has superseded its failed attempt

The owner's decision, which replaces D26 as first written. A call whose own build attempt failed is told so only while that failure is still the last word. If, by the time the call runs again, a later attempt is building or the index is ready, the call is not answered `build_failed` with a stale cause: it is told that the repository is being indexed, to retry shortly, and to ignore the message if a retry has already succeeded. In process there is one client and this cannot happen, so the in-process server's results do not change by a byte: `warming_result` and `build_failure_result` are untouched and the new status is produced on the daemon's path only.

How it is built. `ensure_ready` now returns the daemon's own `Answer` (`Ready`, `Warming`, `Failed`, `Superseded`) instead of `lazy.rs`'s `Readiness`, which stays as it is. A call still reads its own attempt's outcome first and nothing else. Only when that outcome is a failure does it look at the phase, once, under the control lock it already holds, and only to learn whether something has superseded the failure. Nothing of another attempt can reach the call through that look: not its outcome and not its error. That is the difference from the defect the first review found, where a waiter was answered from the phase.

**Files:**
- Modify: `src/mcp/daemon/control.rs`, `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/connection.rs`, `src/mcp/transport.rs` (one added function)
- Modify (tests): `src/mcp/daemon/test_support.rs`, `coordinator_order_tests.rs`, `coordinator_edge_tests.rs`, `coordinator_failure_tests.rs`, `connection_tests.rs`

**Interfaces:**
- Consumes (all `src/mcp/transport.rs`): `fn retryable_status_result(status: Value, index_state: &'static str) -> McpToolResult` (private; it builds the `warming` and `build_failed` results, `is_error: true`, `prism/retryable: true`, `prism/index_state`); `fn success_response(id: Value, result: Value) -> Value` (private); `ToolCall`'s private `id`. They are why the envelope for the new status is built by one added function in `transport.rs` and not in the daemon.
- Produces:
  - `pub(super) enum Answer { Ready, Warming { elapsed: Duration }, Failed { error: String }, Superseded }` in `daemon::control` (`Clone + Debug + PartialEq + Eq`), re-exported from `daemon::coordinator`
  - `AttemptOutcome::ended(&self) -> Option<&Result<(), String>>` in place of `answer`
  - `DaemonState::ensure_ready(self: &Arc<Self>, budget: &mut ConnectionBudget) -> Result<Answer, Poisoned>`
  - `pub(super) fn retryable_status_response(call: &ToolCall<'_>, status: Value, index_state: &'static str, settings: &ConnectionSettings) -> Value` in `transport.rs`
  - in `daemon::connection`: `const INDEXING_RETRY_AFTER_SECS: u64 = 5` (the controller's placeholder; the owner named no number), `const INDEXING_INDEX_STATE: &str = "warming"`, `fn indexing_status() -> Value`

The status on the wire, in the envelope of the other retryable statuses:

```json
{"status":"indexing","retry_after_secs":5,"message":"prism-mcp is indexing the repository; retry this call in about 5 seconds. If you already retried and that call succeeded, ignore this message."}
```

- [ ] **Step 1: Write the tests**

Three coordinator tests change what they expect (two are renamed for it), one is split in two, and one test of a whole connection is added.

In `src/mcp/daemon/test_support.rs`, replace this:

````rust
use super::coordinator::{BuildStatus, DaemonState, Parts, Poisoned};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::lazy::{Readiness, SessionBuilder};
````

with this:

````rust
use super::coordinator::{Answer, BuildStatus, DaemonState, Parts, Poisoned};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::lazy::SessionBuilder;
````

In `src/mcp/daemon/test_support.rs`, replace this:

````rust
    Failed(String),
    Poisoned,
}

impl Outcome {
    fn of(result: Result<Readiness, Poisoned>) -> Self {
        match result {
            Ok(Readiness::Ready) => Self::Ready,
            Ok(Readiness::Warming { elapsed }) => Self::Warming { elapsed },
            Ok(Readiness::Failed { error }) => Self::Failed(error),
            Err(_) => Self::Poisoned,
        }
    }
}
````

with this:

````rust
    Failed(String),
    /// The call's own attempt failed and a retry has superseded it: told to retry.
    Superseded,
    Poisoned,
}

impl Outcome {
    fn of(result: Result<Answer, Poisoned>) -> Self {
        match result {
            Ok(Answer::Ready) => Self::Ready,
            Ok(Answer::Warming { elapsed }) => Self::Warming { elapsed },
            Ok(Answer::Failed { error }) => Self::Failed(error),
            Ok(Answer::Superseded) => Self::Superseded,
            Err(_) => Self::Poisoned,
        }
    }
}
````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
    assert!(outcome.answer().is_none(), "empty while the attempt builds");
````

with this:

````rust
    assert!(outcome.ended().is_none(), "empty while the attempt builds");
````

In `src/mcp/daemon/coordinator_failure_tests.rs`, replace this:

````rust
    assert!(
        matches!(outcome.answer(), Some(Readiness::Failed { error }) if error == "the first ending")
    );
````

with this:

````rust
    assert!(matches!(outcome.ended(), Some(Err(error)) if error == "the first ending"));
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
/// Attempt 1 fails. Before its waiter is notified, another connection calls: it is the next
/// call after the failure, so it starts the retry. The waiter then wakes to find attempt 2
/// building. It must be told that its own attempt failed. Rules out a waiter that joins
/// whatever is building (it would wait on the retry with the budget of another attempt) and
/// one that starts a third attempt.
#[test]
fn a_waiter_that_wakes_after_the_retry_began_still_hears_of_its_own_attempt_s_failure() {
````

with this:

````rust
/// Attempt 1 fails. Before its waiter is notified, another connection calls: it is the next
/// call after the failure, so it starts the retry. The waiter then wakes to find attempt 2
/// building. Its own attempt's failure is stale by then, so it is told to retry (the owner's
/// decision, D26): it is not given that failure, it does not join the retry with the budget
/// of another attempt, and it starts no third attempt. Rules out a waiter that reports a
/// failure a retry has already superseded, and one that waits on whatever is building.
#[test]
fn a_waiter_whose_failed_attempt_was_superseded_by_a_building_retry_is_told_to_retry() {
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
    notify.open();
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string()),
        "the early call is told how its own attempt ended"
    );
    wait_until("the retry's build thread to call the builder", || {
````

with this:

````rust
    notify.open();
    assert_eq!(
        early.outcome(),
        Outcome::Superseded,
        "attempt 1 failed and attempt 2 is building: the early call is told to retry"
    );
    wait_until("the retry's build thread to call the builder", || {
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
/// A call is answered with the outcome of the attempt it joined, whatever has happened since
/// (spec §4.2: "its outcome is delivered to every connection waiting on it"). Here the retry
/// has already succeeded when the waiter of attempt 1 runs. Rules out answering from the
/// current phase: the waiter would be told `Ready`, and attempt 1's failure would reach nobody.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_has_succeeded() {
````

with this:

````rust
/// The retry has already succeeded when the waiter of attempt 1 runs. It is told to retry, and
/// its next call is served (D26). It is not told `Ready` either: a call is answered from its
/// own attempt, and its own attempt built nothing. Rules out reporting the superseded failure,
/// and answering from the current phase.
#[test]
fn a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_told_to_retry() {
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string()),
        "the waiter of attempt 1 is told how attempt 1 ended"
    );
    notify.open();
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 2);

    // Its next call finds the index ready.
````

with this:

````rust
    assert_eq!(
        early.outcome(),
        Outcome::Superseded,
        "attempt 1 failed and the index is ready: the waiter of attempt 1 is told to retry"
    );
    notify.open();
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 2);

    // Its next call finds the index ready.
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
/// As above, with a retry that failed for another reason. Rules out one remembered error for
/// all attempts: the waiter of attempt 1 would be told attempt 2's.
#[test]
````

with this:

````rust
/// Here the retry failed too, for another reason, and nothing is building: no retry has
/// superseded attempt 1's failure, so its waiter is told of it, with attempt 1's own text.
/// Rules out one remembered error for all attempts (the waiter of attempt 1 would be told
/// attempt 2's) and telling every waiter of a failed attempt to retry.
#[test]
````

In `src/mcp/daemon/coordinator_edge_tests.rs`, replace this:

````rust
/// The waiter of attempt 1 first runs when attempt 3 is building. The notifications of
/// attempts 1 and 2 are both held, so nothing wakes either earlier waiter until then; each is
/// then told its own attempt's failure, and the caller of attempt 3 keeps waiting. Rules out
/// an error kept with only its attempt number: nothing of attempt 1 would be left.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_a_third_attempt_is_building() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let third_gate = Gate::closed();
    let (builder, _calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            fail_now("second attempt failed"),
            build_after(&third_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = [Gate::closed(), Gate::closed()];
    let notify = [Gate::closed(), Gate::closed()];
    let (hook_recorded, hook_notify) = (recorded.clone(), notify.clone());
    let hook: AttemptHook = Arc::new(move |attempt: u64| {
        if attempt == 1 || attempt == 2 {
            let at = (attempt - 1) as usize;
            hook_recorded[at].open();
            let _opened = hook_notify[at].pass();
        }
    });
    parts.hooks.settled = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("early to wait on attempt 1", || status(&state).waiting == 1);
    first_gate.open();
    assert!(recorded[0].pass(), "attempt 1's failure is recorded");

    let second = Conn::open(&state, 600);
    second.call();
    assert!(recorded[1].pass(), "attempt 2's failure is recorded");
    let third = Conn::open(&state, 600);
    third.call();
    wait_until("attempt 3 to be building with three calls parked", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 3 && now.waiting == 3
    });
    assert!(
        !early.answered(),
        "nothing has woken the waiter of attempt 1"
    );
    assert!(!second.answered(), "nor the waiter of attempt 2");

    notify[0].open();
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(
        second.outcome(),
        Outcome::Failed("second attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, 3);
    assert_eq!(status(&state).state, BuildState::Building);
    assert!(!third.answered(), "the caller of attempt 3 keeps waiting");

    notify[1].open();
    third_gate.open();
    assert_eq!(third.outcome(), Outcome::Ready);
}
````

with this:

````rust
/// Attempts 1 and 2 fail and the notifications of both are held, so nothing wakes either
/// waiter; a third connection then starts attempt 3, which plays `third`. Returns the state,
/// the three connections in the order of their attempts, and the two held notifications.
fn two_waiters_held_behind_their_failed_attempts(
    cfg: &ServerConfig,
    third: BuildStep,
) -> (Arc<DaemonState>, [Conn; 3], [Arc<Gate>; 2]) {
    let first_gate = Gate::closed();
    let (builder, _calls) = scripted(
        cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            fail_now("second attempt failed"),
            third,
        ],
    );
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = [Gate::closed(), Gate::closed()];
    let notify = [Gate::closed(), Gate::closed()];
    let (hook_recorded, hook_notify) = (recorded.clone(), notify.clone());
    let hook: AttemptHook = Arc::new(move |attempt: u64| {
        if attempt == 1 || attempt == 2 {
            let at = (attempt - 1) as usize;
            hook_recorded[at].open();
            let _opened = hook_notify[at].pass();
        }
    });
    parts.hooks.settled = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("early to wait on attempt 1", || status(&state).waiting == 1);
    first_gate.open();
    assert!(recorded[0].pass(), "attempt 1's failure is recorded");

    let second = Conn::open(&state, 600);
    second.call();
    assert!(recorded[1].pass(), "attempt 2's failure is recorded");
    let third = Conn::open(&state, 600);
    third.call();
    (state, [early, second, third], notify)
}

/// The waiters of attempts 1 and 2 first run when attempt 3 is building. Each is told to
/// retry (D26): a later attempt has superseded its failure. Neither joins attempt 3, and the
/// caller of attempt 3 keeps waiting.
#[test]
fn waiters_of_failed_attempts_are_told_to_retry_while_a_third_attempt_is_building() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let third_gate = Gate::closed();
    let (state, [early, second, third], notify) =
        two_waiters_held_behind_their_failed_attempts(&cfg, build_after(&third_gate, &cfg));
    wait_until("attempt 3 to be building with three calls parked", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 3 && now.waiting == 3
    });
    assert!(
        !early.answered(),
        "nothing has woken the waiter of attempt 1"
    );
    assert!(!second.answered(), "nor the waiter of attempt 2");

    notify[0].open();
    assert_eq!(early.outcome(), Outcome::Superseded);
    assert_eq!(second.outcome(), Outcome::Superseded);
    assert_eq!(status(&state).attempt, 3);
    assert_eq!(status(&state).state, BuildState::Building);
    assert!(!third.answered(), "the caller of attempt 3 keeps waiting");

    notify[1].open();
    third_gate.open();
    assert_eq!(third.outcome(), Outcome::Ready);
}

/// The same, with a third attempt that fails as well, so that nothing is building when the
/// earlier waiters run: no retry has superseded their failures. Each is told of its own
/// attempt's, with its own text, two and one attempts later. Rules out an error kept with
/// only its attempt number (nothing of attempt 1 would be left), and answering with the
/// latest failure's text.
#[test]
fn waiters_of_failed_attempts_hear_their_own_failure_when_every_later_attempt_failed_too() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (state, [early, second, third], notify) =
        two_waiters_held_behind_their_failed_attempts(&cfg, fail_now("third attempt failed"));

    // Attempt 3's ending is not held: it wakes everybody parked.
    assert_eq!(
        third.outcome(),
        Outcome::Failed("third attempt failed".to_string())
    );
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(
        second.outcome(),
        Outcome::Failed("second attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, 3);
    assert_eq!(status(&state).state, BuildState::Failed);
    notify[0].open();
    notify[1].open();
}
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
    advance, build_after, client_settings, client_settings_with, fail_now, fixture, parts_with,
    scripted, state_with, status, wait_until, Gate, ManualClock,
};
````

with this:

````rust
    advance, build_after, client_settings, client_settings_with, fail_after, fail_now, fixture,
    parts_with, scripted, state_with, status, wait_until, Gate, ManualClock,
};
````

Append to `src/mcp/daemon/connection_tests.rs`:

````rust

/// The owner's decision (D26): a call is not told of a failure that a retry has already
/// superseded. A's call waits on attempt 1, which fails; before A's call runs again, B's call
/// has started the retry and been served from it. A is then told that the repository is being
/// indexed and to retry, in the envelope of the other retryable statuses, and its retry is
/// served. Rules out answering `build_failed` with a stale cause, and a status that clients
/// keyed on the index state would not treat as warming.
#[test]
fn a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let (builder, builds) = scripted(
        &cfg,
        vec![fail_after(&first_gate, "first attempt failed")],
    );
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = Gate::closed();
    let notify = Gate::closed();
    let hook: AttemptHook = {
        let recorded = Arc::clone(&recorded);
        let notify = Arc::clone(&notify);
        Arc::new(move |attempt: u64| {
            if attempt == 1 {
                recorded.open();
                let _opened = notify.pass();
            }
        })
    };
    parts.hooks.settled = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on attempt 1", || {
        status(&state).waiting == 1
    });
    first_gate.open();
    assert!(recorded.pass(), "attempt 1's failure is recorded");

    // B's call is the next call after the failure: it starts the retry and is served from it.
    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);

    // The retry's announcement woke A's call. Its own attempt failed; the index is ready.
    let told = a.recv();
    assert_eq!(told["id"], 7);
    assert_eq!(told["result"]["isError"], true, "{told}");
    assert_eq!(
        told["result"]["structuredContent"],
        json!({
            "status": "indexing",
            "retry_after_secs": 5,
            "message": "prism-mcp is indexing the repository; retry this call in about 5 seconds. If you already retried and that call succeeded, ignore this message."
        }),
        "{told}"
    );
    let text = told["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text).ok().as_ref(),
        Some(&told["result"]["structuredContent"]),
        "the text view carries the same status"
    );
    assert_eq!(told["result"]["_meta"]["prism/index_state"], "warming");
    assert_eq!(told["result"]["_meta"]["prism/retryable"], true);
    assert!(
        !told.to_string().contains("first attempt failed"),
        "the superseded failure is not sent: {told}"
    );
    notify.open();

    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
    close(b, b_worker);
}
````

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t10-red.log 2>&1; grep -E '^error' /tmp/s3a-t10-red.log | sort -u
```

Expected: `error[E0432]: unresolved import `super::coordinator::Answer`` and `error[E0599]: no method named `ended` found`.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/control.rs`, replace this:

````rust
use crate::mcp::lazy::Readiness;
use crate::mcp::shared::preamble::BuildState;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
````

with this:

````rust
use crate::mcp::shared::preamble::BuildState;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// What a readiness call is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// The index is published.
    Ready,
    /// This connection's deadline for its attempt has passed and the attempt is still
    /// building. `elapsed` is the attempt's age.
    Warming { elapsed: Duration },
    /// The call's own attempt ended without an index, with this error, and nothing has
    /// superseded that: no later attempt is building and the index is not ready.
    Failed { error: String },
    /// The call's own attempt ended without an index, and the daemon has since moved on: a
    /// later attempt is building, or the index is ready. The failure is stale, so the call is
    /// not given it; it is told to retry. It is never given another attempt's outcome.
    Superseded,
}
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    /// What a call that belongs to this attempt is told; `None` while it is still building.
    pub(super) fn answer(&self) -> Option<Readiness> {
        self.ended.get().map(|ended| match ended {
            Ok(()) => Readiness::Ready,
            Err(error) => Readiness::Failed {
                error: error.clone(),
            },
        })
    }
````

with this:

````rust
    /// How the attempt ended: `Ok` if it published the index, its error if it did not; `None`
    /// while it is still building.
    pub(super) fn ended(&self) -> Option<&Result<(), String>> {
        self.ended.get()
    }
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
pub(super) use super::control::{AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use crate::mcp::lazy::{Readiness, SessionBuilder};
````

with this:

````rust
pub(super) use super::control::{Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use crate::mcp::lazy::SessionBuilder;
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// The call first decides which attempt it belongs to, once: the one that is building, or a
    /// new one if none has been started or the last one failed. From then on it reads only that
    /// attempt's outcome, whatever the phase has become by the time it runs again, and returns
    ///
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived),
    /// - `Failed`, with that attempt's own error, if that attempt ended without an index. The
    ///   call does not start the retry (the next call does), does not join a retry that has
    ///   begun in the meantime, and is told of the failure even if a retry has since succeeded:
    ///   its next call finds the index ready,
    /// - `Warming` when this connection's deadline for that attempt has passed. That is this
    ///   connection's wait ending; the build goes on and nobody else's deadline changes.
    ///
    /// Starting a new attempt and joining a running one happen under one acquisition of the
    /// control lock, so however many calls find the build failed, one of them starts the retry
    /// and the rest join it.
    pub(super) fn ensure_ready(
        self: &Arc<Self>,
        budget: &mut ConnectionBudget,
    ) -> Result<Readiness, Poisoned> {
        let mut control = self.lock_control()?;

        let building = match &control.build {
            BuildPhase::Ready => return Ok(Readiness::Ready),
            BuildPhase::Building {
                attempt,
                started,
                outcome,
            } => Some((*attempt, *started, Arc::clone(outcome))),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
        };
        let (attempt, started, outcome) =
            building.unwrap_or_else(|| self.start_attempt(&mut control));
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            if let Some(answer) = outcome.answer() {
                return Ok(answer);
            }
            let now = self.clock.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Readiness::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
            control.waiting = control.waiting.saturating_add(1);
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
                Err(_) => return Err(Poisoned { lock: "control" }),
            };
            control.waiting = control.waiting.saturating_sub(1);
        }
    }
````

with this:

````rust
    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// The call first decides which attempt it belongs to, once: the one that is building, or a
    /// new one if none has been started or the last one failed. From then on it reads only that
    /// attempt's outcome, and returns
    ///
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived),
    /// - `Failed`, with that attempt's own error, if that attempt ended without an index and
    ///   nothing has superseded it. The call does not start the retry; the next call does,
    /// - `Superseded` if that attempt ended without an index and, by the time the call runs
    ///   again, a later attempt is building or the index is ready. The failure is stale then,
    ///   and the call is told to retry instead of being given it,
    /// - `Warming` when this connection's deadline for that attempt has passed. That is this
    ///   connection's wait ending; the build goes on and nobody else's deadline changes.
    ///
    /// Starting a new attempt and joining a running one happen under one acquisition of the
    /// control lock, so however many calls find the build failed, one of them starts the retry
    /// and the rest join it.
    pub(super) fn ensure_ready(
        self: &Arc<Self>,
        budget: &mut ConnectionBudget,
    ) -> Result<Answer, Poisoned> {
        let mut control = self.lock_control()?;

        let building = match &control.build {
            BuildPhase::Ready => return Ok(Answer::Ready),
            BuildPhase::Building {
                attempt,
                started,
                outcome,
            } => Some((*attempt, *started, Arc::clone(outcome))),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
        };
        let (attempt, started, outcome) =
            building.unwrap_or_else(|| self.start_attempt(&mut control));
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            match outcome.ended() {
                Some(Ok(())) => return Ok(Answer::Ready),
                // The call's own attempt failed. One look at the phase, under the lock this
                // call holds, decides whether that failure is still the last word. The look
                // takes nothing from the phase but that: no outcome and no error of another
                // attempt can reach this call through it, which is what answering from the
                // phase once got wrong. `Building` here is always a later attempt, because
                // this call's own has ended.
                Some(Err(error)) => {
                    return Ok(match &control.build {
                        BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Superseded,
                        BuildPhase::Failed { .. } | BuildPhase::Idle => Answer::Failed {
                            error: error.clone(),
                        },
                    });
                }
                None => {}
            }
            let now = self.clock.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Answer::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
            control.waiting = control.waiting.saturating_add(1);
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
                Err(_) => return Err(Poisoned { lock: "control" }),
            };
            control.waiting = control.waiting.saturating_sub(1);
        }
    }
````

In `src/mcp/transport.rs`, replace this:

````rust
/// `refresh_index`: rebuild the snapshot. Needs exclusive access to the provider.
````

with this:

````rust
/// A retryable status for a call that cannot run yet, in the envelope `readiness_response`
/// gives the warming and build-failed statuses. It is for a status that only a shared daemon
/// reports; the in-process server reports those two, through `readiness_response`, and
/// nothing else.
pub(super) fn retryable_status_response(
    call: &ToolCall<'_>,
    status: Value,
    index_state: &'static str,
    settings: &ConnectionSettings,
) -> Value {
    success_response(
        call.id.clone(),
        retryable_status_result(status, index_state)
            .to_call_tool_result_value(settings.structured_content),
    )
}

/// `refresh_index`: rebuild the snapshot. Needs exclusive access to the provider.
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
//! functions, under the provider slot's read or write lock.
//!
````

with this:

````rust
//! functions, under the provider slot's read or write lock.
//!
//! One answer here is the daemon's alone. A call whose own build attempt failed, and which
//! runs again only after a retry has superseded that failure, is told that the repository is
//! being indexed and to retry (`indexing_status`); the in-process server has one client and no
//! such case. Everything else a call is told before the index is ready is the in-process
//! server's `warming` or `build_failed`, from the same `readiness_response`.
//!
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
use super::coordinator::DaemonState;
use crate::mcp::framing::{Transport, UnixStreamTransport};
use crate::mcp::registry::ToolRegistry;
````

with this:

````rust
use super::coordinator::{Answer, DaemonState};
use crate::mcp::framing::{Transport, UnixStreamTransport};
use crate::mcp::lazy::Readiness;
use crate::mcp::registry::ToolRegistry;
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
    readiness_response, serve_loop, Control, ReadDispatch, ToolCall,
};
use serde_json::Value;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
````

with this:

````rust
    readiness_response, retryable_status_response, serve_loop, Control, ReadDispatch, ToolCall,
};
use serde_json::{json, Value};
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// How many seconds a client that is told `indexing` is asked to wait before it retries. The
/// owner asked for "retry in <retry interval seconds>" and named no number; five is a
/// placeholder to be changed here.
const INDEXING_RETRY_AFTER_SECS: u64 = 5;

/// The `prism/index_state` an `indexing` answer carries: the same as a warming answer's, so
/// that a client that keys on the index state treats the two alike.
const INDEXING_INDEX_STATE: &str = "warming";

/// What a call is told when its own build attempt failed and a retry has already superseded
/// that failure (a later attempt is building, or the index is ready): retry shortly, and
/// ignore this if a retry has already succeeded. The stale failure's text is not in it.
fn indexing_status() -> Value {
    json!({
        "status": "indexing",
        "retry_after_secs": INDEXING_RETRY_AFTER_SECS,
        "message": format!(
            "prism-mcp is indexing the repository; retry this call in about \
             {INDEXING_RETRY_AFTER_SECS} seconds. If you already retried and that call \
             succeeded, ignore this message."
        ),
    })
}
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
    let readiness = state.ensure_ready(budget)?;
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
````

with this:

````rust
    let readiness = match state.ensure_ready(budget)? {
        Answer::Ready => Readiness::Ready,
        Answer::Warming { elapsed } => Readiness::Warming { elapsed },
        Answer::Failed { error } => Readiness::Failed { error },
        Answer::Superseded => {
            return Ok(retryable_status_response(
                call,
                indexing_status(),
                INDEXING_INDEX_STATE,
                settings,
            ));
        }
    };
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
````

- [ ] **Step 4: Run the tests and the standalone contract**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t10-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t10-green.log
cargo test --features mcp --lib mcp:: > /tmp/s3a-t10-lib.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t10-lib.log
env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > /tmp/s3a-t10-mcp.log 2>&1; grep -E '^test result|FAILED|differs|^error' /tmp/s3a-t10-mcp.log
git diff --stat -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs tests/fixtures/mcp/golden | wc -l
git diff -- src/mcp/transport.rs | grep -c -E '^-[^-]'
```

Expected: `49 passed; 0 failed` for the module (47, one more edge test, one more connection test); the lib total with `0 failed`; the `mcp` target at the base number with the golden transcripts passing unblessed; `0`; and `0` again: the edit to `transport.rs` removes no line. The byte-parity sessions of Task 8 pass unchanged; they have one client per server and never meet a superseded failure. (They also never meet a client that stops waiting before its build fails, a single-client case where the daemon as built here differs from the in-process server; the first review of S3a-2 found it, and Task 11 fixes it with a parity leg.)

- [ ] **Step 5: Commit**

```bash
git add src/mcp
git commit -m "feat(mcp): tell a call to retry when a retry has superseded its failed build attempt"
```

- [ ] **Step 6: Prove the tests bite (after the commit)**

Five mutants, one at a time. After each, run the command and restore the file named.

Mutant A (a superseded failure is still reported), in `src/mcp/daemon/coordinator.rs`. Replace this:

````rust
                        BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Superseded,
````

with this:

````rust
                        BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Failed {
                            error: error.clone(),
                        },
````

Mutant B (every waiter of a failed attempt is told to retry), in `src/mcp/daemon/coordinator.rs`. Replace this:

````rust
                        BuildPhase::Failed { .. } | BuildPhase::Idle => Answer::Failed {
                            error: error.clone(),
                        },
````

with this:

````rust
                        BuildPhase::Failed { .. } | BuildPhase::Idle => Answer::Superseded,
````

Mutant C (a failure is reported with the latest attempt's text: the defect of the first review, through the new look at the phase), in `src/mcp/daemon/coordinator.rs`. Replace the same three lines as in B with this:

````rust
                        BuildPhase::Failed { error: latest, .. } => Answer::Failed {
                            error: latest.clone(),
                        },
                        BuildPhase::Idle => Answer::Failed {
                            error: error.clone(),
                        },
````

Mutant D (a superseded call is answered as warming), in `src/mcp/daemon/connection.rs`. Replace this:

````rust
        Answer::Superseded => {
            return Ok(retryable_status_response(
                call,
                indexing_status(),
                INDEXING_INDEX_STATE,
                settings,
            ));
        }
````

with this:

````rust
        Answer::Superseded => Readiness::Warming {
            elapsed: std::time::Duration::ZERO,
        },
````

Mutant E (the indexing answer carries an index state clients do not treat as warming), in `src/mcp/daemon/connection.rs`. Replace this:

````rust
const INDEXING_INDEX_STATE: &str = "warming";
````

with this:

````rust
const INDEXING_INDEX_STATE: &str = "failed";
````

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t10-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t10-bite.log
git checkout -- src/mcp/daemon/coordinator.rs src/mcp/daemon/connection.rs
```

Expected, and observed in validation:

| Mutant | Fails |
|---|---|
| A | `a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served`, `a_waiter_whose_failed_attempt_was_superseded_by_a_building_retry_is_told_to_retry`, `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_told_to_retry`, `waiters_of_failed_attempts_are_told_to_retry_while_a_third_attempt_is_building` |
| B | 11 tests, every one in which a call hears of a failure nothing has superseded; among them `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself` and `a_failed_build_is_reported_as_the_in_process_server_reports_it_and_the_next_call_retries` |
| C | `a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_failed_differently`, `waiters_of_failed_attempts_hear_their_own_failure_when_every_later_attempt_failed_too` |
| D | `a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served` |
| E | `a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served` |

---

### Task 11: A connection's next call first hears its own attempt's unheard failure

Found by the first review of S3a-2 (Opus, F1), and decided by the controller under the owner's D26 principle (D32). A client whose wait ran out is answered `warming`. If the build then fails, the in-process server answers that client's next call `build_failed` with the cause, and only the call after that starts the retry. The daemon as Tasks 7–10 build it found the phase `Failed` on that next call and started the retry at once, answering `warming` again: the failure reached nobody, and a slow build that keeps failing was rebuilt on every call while the client was told "no other action is needed". Spec §4.2 ("after `Failed`, the next readiness-requiring call starts exactly one retry") and §5 (the warming result and retry contract is preserved) disagree on this case; D32 reads §4.2 per connection, and SI-8 proposes the sentence for the spec.

How it is built. `ConnectionBudget` now keeps, beside its deadline, the outcome of the attempt the connection last joined (`join` replaces `deadline_for` and takes the attempt's `Arc<AttemptOutcome>`) and whether a call of the connection has been told how it ended. `ensure_ready` asks the budget first, before the shortcut to `Ready` and before starting or joining anything: an unheard failure is answered by the D26 rule and starts nothing. The rule is now one pure function, `answer_for(own, phase)` in `control.rs`, which the wait loop uses too, and which is table-tested with no threads. The only inputs are the connection's own outcome and the phase's discriminant, so no other attempt's outcome or text can reach the call. An attempt that ended ready needs no such step: the call proceeds as before.

`a_retry_gives_a_connection_a_fresh_full_budget` pinned the old behavior (the call after a failure started the retry) and changes its expectation: it gains the call that hears the failure first.

**Files:**
- Modify: `src/mcp/daemon/budget.rs`, `src/mcp/daemon/control.rs`, `src/mcp/daemon/coordinator.rs`
- Create (tests): `src/mcp/daemon/control_tests.rs`
- Modify (tests): `budget_tests.rs`, `coordinator_tests.rs`, `coordinator_edge_tests.rs`, `connection_tests.rs`, `connection_parity_tests.rs`

**Interfaces:**
- Consumes: `AttemptOutcome::ended` (Task 10), `BuildPhase`, `Answer`.
- Produces:
  - `ConnectionBudget::join(&mut self, attempt: u64, outcome: &Arc<AttemptOutcome>, now: Instant) -> Instant`, in place of `deadline_for`
  - `ConnectionBudget::told(&mut self)` and `ConnectionBudget::unheard_failure(&mut self) -> Option<String>` (it records the failure as told)
  - `pub(super) fn answer_for(own: &Result<(), String>, phase: &BuildPhase) -> Answer` in `daemon::control`

- [ ] **Step 1: Write the tests**

The budget's tests move from `deadline_for` to `join` and gain four; three table tests of `answer_for`; four coordinator tests (one of them the over-reach guard: an attempt that published while the connection was not waiting is served); one byte-parity leg against the in-process server; one two-client test over sockets. Two existing tests that hold waiters behind withheld notifications say in their docs what they assume of spurious wake-ups.

Replace the whole of `src/mcp/daemon/budget_tests.rs` with:

````rust
//! Tests for one connection's first-call budget: one deadline per connection per build attempt
//! (spec §4.2), and the ending of the attempt the connection last joined, which its next call
//! hears if no call of its own has. The clock is an argument, so nothing here waits.
use super::super::test_support::client_settings;
use super::*;

fn budget_of(wait_secs: u64) -> ConnectionBudget {
    ConnectionBudget::new(&client_settings(wait_secs))
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// A fresh, empty outcome for an attempt. What a budget keeps of it is tested below; the
/// deadline does not depend on it.
fn cell() -> Arc<AttemptOutcome> {
    Arc::new(AttemptOutcome::default())
}

/// Rules out a fresh deadline for every call: queued calls would then multiply one attempt's
/// wait.
#[test]
fn the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.join(1, &cell(), start), start + secs(20));
    assert_eq!(
        budget.join(1, &cell(), start + secs(5)),
        start + secs(20),
        "a later call in the same attempt has only the rest of the budget"
    );
    assert_eq!(
        budget.join(1, &cell(), start + secs(500)),
        start + secs(20),
        "and none once it has passed"
    );
}

/// Rules out one deadline for the connection's whole life: after a failed build the retry
/// would be answered with warming at once.
#[test]
fn a_new_attempt_gets_a_fresh_full_budget() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.join(1, &cell(), start), start + secs(20));
    assert_eq!(
        budget.join(2, &cell(), start + secs(30)),
        start + secs(50),
        "counted from the first call that arrives during the new attempt"
    );
    assert_eq!(budget.join(2, &cell(), start + secs(45)), start + secs(50));
}

#[test]
fn each_connection_has_its_own_deadline_for_the_same_attempt() {
    let start = Instant::now();
    let mut a = budget_of(20);
    let mut b = budget_of(60);

    assert_eq!(a.join(1, &cell(), start), start + secs(20));
    assert_eq!(b.join(1, &cell(), start + secs(10)), start + secs(70));
    assert_eq!(
        a.join(1, &cell(), start + secs(10)),
        start + secs(20),
        "B joining the attempt changed nothing for A"
    );
}

#[test]
fn a_wait_of_zero_has_expired_the_moment_it_is_set() {
    let start = Instant::now();
    let mut budget = budget_of(0);

    assert_eq!(budget.join(1, &cell(), start), start);
}

#[test]
fn the_longest_wait_a_client_may_ask_for_is_kept_whole() {
    let start = Instant::now();
    let longest = crate::mcp::FIRST_CALL_WAIT_MAX;
    let mut budget = budget_of(longest.as_secs());

    assert_eq!(budget.join(1, &cell(), start), start + longest);
}

/// A connection whose calls for an attempt were all answered warming hears how it ended on its
/// next call, once. Rules out a budget that forgets the attempt it joined, and one that
/// reports the same failure on every call.
#[test]
fn a_failure_no_call_was_told_of_is_reported_once() {
    let mut budget = budget_of(0);
    let attempt = cell();
    budget.join(1, &attempt, Instant::now());
    assert_eq!(budget.unheard_failure(), None, "the attempt is still building");

    attempt.fill(Err("it failed".to_string()));
    assert_eq!(budget.unheard_failure(), Some("it failed".to_string()));
    assert_eq!(budget.unheard_failure(), None, "and only once");
}

/// A call that was told how its attempt ended has heard it: nothing is reported again.
#[test]
fn a_failure_a_call_was_told_of_is_not_reported_again() {
    let mut budget = budget_of(600);
    let attempt = cell();
    budget.join(1, &attempt, Instant::now());
    attempt.fill(Err("it failed".to_string()));
    budget.told();

    assert_eq!(budget.unheard_failure(), None);
}

#[test]
fn an_attempt_that_published_the_index_is_never_an_unheard_failure() {
    let mut budget = budget_of(0);
    let attempt = cell();
    budget.join(1, &attempt, Instant::now());
    attempt.fill(Ok(()));

    assert_eq!(budget.unheard_failure(), None);
}

/// What is remembered is the attempt joined last; joining a new one starts it unheard.
#[test]
fn joining_a_new_attempt_remembers_that_attempt_and_not_the_old_one() {
    let mut budget = budget_of(0);
    let first = cell();
    budget.join(1, &first, Instant::now());
    first.fill(Err("first".to_string()));
    budget.told();

    let second = cell();
    budget.join(2, &second, Instant::now());
    second.fill(Err("second".to_string()));
    assert_eq!(budget.unheard_failure(), Some("second".to_string()));
}
````

In `src/mcp/daemon/coordinator_tests.rs`, replace this:

````rust
    first_gate.open();
    wait_until("the first attempt to fail", || {
        status(&state).state == BuildState::Failed
    });

    // This call starts the retry. With a fresh budget it waits; with the old deadline it would
    // be answered at once.
    conn.call();
````

with this:

````rust
    first_gate.open();
    wait_until("the first attempt to fail", || {
        status(&state).state == BuildState::Failed
    });

    // The connection was told only that the build was warming; its next call hears the
    // failure and starts nothing (spec §5's warming-then-failed sequence, D32).
    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, 1);

    // This call starts the retry. With a fresh budget it waits; with the old deadline it would
    // be answered at once.
    conn.call();
````

In `src/mcp/daemon/coordinator_edge_tests.rs`, replace this:

````rust
/// The waiters of attempts 1 and 2 first run when attempt 3 is building. Each is told to
/// retry (D26): a later attempt has superseded its failure. Neither joins attempt 3, and the
/// caller of attempt 3 keeps waiting.
````

with this:

````rust
/// The waiters of attempts 1 and 2 first run when attempt 3 is building. Each is told to
/// retry (D26): a later attempt has superseded its failure. Neither joins attempt 3, and the
/// caller of attempt 3 keeps waiting. The rule itself is table-tested without threads in
/// `control_tests.rs`; this test assumes that neither waiter wakes spuriously while its
/// notification is held. A spurious wake-up cannot make it pass wrongly: the waiter would
/// answer early and the assertions on it would fail.
````

Append to `src/mcp/daemon/coordinator_edge_tests.rs`:

````rust

/// A connection whose call was answered warming hears, on its next call, that its attempt
/// failed, as an in-process client does (spec §5), and only the call after that starts the
/// retry (D32). Rules out a connection that starts the retry at once and never hears the
/// failure; with a build that keeps failing slowly, that would rebuild on every call while the
/// client is told nothing is wrong.
#[test]
fn a_connection_that_stopped_waiting_hears_its_attempt_s_failure_before_any_retry() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![fail_after(&gate, "first attempt failed")]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let conn = Conn::open(&state, 0);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Warming {
            elapsed: Duration::ZERO
        }
    );
    gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, 1, "and no retry was started");
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Warming {
            elapsed: Duration::ZERO
        },
        "the call after that starts the retry"
    );
    assert_eq!(status(&state).attempt, 2);
}

/// The same, when another connection has retried in the meantime: the first connection's
/// next call is told to retry (D26), not given the stale failure, and starts nothing; the call
/// after that joins the retry.
#[test]
fn a_connection_that_stopped_waiting_is_told_to_retry_when_another_retried_meanwhile() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let retry_gate = Gate::closed();
    let (builder, _calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let quick = Conn::open(&state, 0);
    let patient = Conn::open(&state, 600);

    quick.call();
    assert!(matches!(quick.outcome(), Outcome::Warming { .. }));
    first_gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });
    patient.call();
    wait_until("the retry to be building, with one call waiting", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 2 && now.waiting == 1
    });

    quick.call();
    assert_eq!(quick.outcome(), Outcome::Superseded);
    quick.call();
    assert!(
        matches!(quick.outcome(), Outcome::Warming { .. }),
        "the call after that joins the retry"
    );
    assert_eq!(status(&state).attempt, 2, "and starts no third attempt");

    retry_gate.open();
    assert_eq!(patient.outcome(), Outcome::Ready);
}

/// The connection hears its own attempt's failure, with that attempt's text, although a later
/// attempt has failed for another reason since. Rules out answering an unheard failure from
/// the latest attempt.
#[test]
fn a_connection_hears_its_own_attempt_s_failure_although_a_later_attempt_failed_too() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let (builder, _calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            fail_now("second attempt failed"),
        ],
    );
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let quick = Conn::open(&state, 0);
    let patient = Conn::open(&state, 600);

    quick.call();
    assert!(matches!(quick.outcome(), Outcome::Warming { .. }));
    first_gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });
    patient.call();
    assert_eq!(
        patient.outcome(),
        Outcome::Failed("second attempt failed".to_string())
    );

    quick.call();
    assert_eq!(
        quick.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
}

/// An attempt that published the index while the connection was not waiting needs no extra
/// step: the next call is served.
#[test]
fn a_connection_whose_attempt_published_while_it_was_not_waiting_is_served() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let conn = Conn::open(&state, 0);

    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
````

Append to `src/mcp/daemon/control.rs`:

````rust

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
````

Create `src/mcp/daemon/control_tests.rs`:

````rust
//! The rule that decides what a call is told about its own attempt's ending (D26, D32),
//! tested for every ending and every phase, without threads.
use super::*;

fn building() -> BuildPhase {
    BuildPhase::Building {
        attempt: 2,
        started: Instant::now(),
        outcome: Arc::new(AttemptOutcome::default()),
    }
}

fn failed(error: &str) -> BuildPhase {
    BuildPhase::Failed {
        attempt: 2,
        error: error.to_string(),
    }
}

fn every_phase() -> Vec<BuildPhase> {
    vec![
        BuildPhase::Idle,
        building(),
        BuildPhase::Ready,
        failed("a later attempt's error"),
    ]
}

/// An attempt that published the index is answered `Ready`, whatever the phase says now.
#[test]
fn an_attempt_that_published_is_answered_ready_in_every_phase() {
    for phase in every_phase() {
        assert_eq!(answer_for(&Ok(()), &phase), Answer::Ready, "{phase:?}");
    }
}

/// A failed attempt is superseded when a later attempt is building or the index is ready.
#[test]
fn a_failed_attempt_is_superseded_by_a_building_retry_and_by_a_ready_index() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(answer_for(&own, &building()), Answer::Superseded);
    assert_eq!(answer_for(&own, &BuildPhase::Ready), Answer::Superseded);
}

/// Otherwise the call is told its own attempt's failure, with its own text: never the text of
/// the attempt the phase names.
#[test]
fn a_failed_attempt_nothing_has_superseded_is_answered_with_its_own_text() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(
        answer_for(&own, &failed("a later attempt's error")),
        Answer::Failed {
            error: "own error".to_string()
        }
    );
    assert_eq!(
        answer_for(&own, &BuildPhase::Idle),
        Answer::Failed {
            error: "own error".to_string()
        }
    );
}
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
use super::super::test_support::{client_settings, fixture, scripted, status};
````

with this:

````rust
use super::super::test_support::{
    build_after, client_settings, fail_after, fixture, scripted, status, wait_until, Gate,
};
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
use crate::mcp::lazy::{LazySessionProvider, SessionBuilder};
````

with this:

````rust
use crate::mcp::lazy::{LazySessionProvider, LazyTestHooks, SessionBuilder};
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
use std::sync::atomic::Ordering;
````

with this:

````rust
use std::sync::atomic::{AtomicU64, Ordering};
````

Append to `src/mcp/daemon/connection_parity_tests.rs`:

````rust

/// One client of each server, waiting zero seconds: a call, then the build fails, then two
/// calls. Returns every response line in order.
fn play_a_client_that_stopped_waiting(
    client: &mut Client,
    gate: &Gate,
    mut failed: impl FnMut() -> bool,
) -> String {
    let mut transcript = String::new();
    client.send(INIT);
    transcript.push_str(&client.recv_line());
    client.send(INITED);
    // The build is held, so this call is told the index is warming.
    client.send(CALLEES);
    transcript.push_str(&client.recv_line());
    gate.open();
    wait_until("the first build attempt to fail", &mut failed);
    // The next call hears the failure, with its cause, and starts nothing.
    client.send(CALLEES);
    transcript.push_str(&client.recv_line());
    // The call after that starts the retry, which is held: warming again.
    client.send(CALLEES);
    transcript.push_str(&client.recv_line());
    transcript
}

/// Spec §5 keeps "the warming result and retry contract". A client that stopped waiting is told
/// the index is warming; the build then fails; its next call is told `build_failed` with the
/// cause, and only the call after that starts the retry. The daemon must say the same bytes
/// (D32). Rules out a daemon whose next call after a failure starts the retry at once: the
/// client would never hear the failure.
#[test]
fn a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server() {
    let settings = client_settings(0);

    // In process.
    let (_in_dir, in_cfg) = fixture(RefreshPolicy::WarnOnly);
    let in_gate = Gate::closed();
    let in_retry = Gate::closed();
    let (in_builder, in_builds) = scripted(
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
    let in_process = std::thread::spawn(move || {
        let mut provider =
            LazySessionProvider::with_builder_and_hooks(&in_cfg, Duration::ZERO, in_builder, hooks)
                .expect("a lazy provider");
        let mut transport = UnixStreamTransport::from_stream(far).expect("a transport");
        serve_connection(
            &mut provider,
            &ToolRegistry::all_v1(),
            &mut transport,
            &output,
        )
    });
    let mut client = Client::new(near);
    let expected = play_a_client_that_stopped_waiting(&mut client, &in_gate, || {
        delivered.load(Ordering::SeqCst) == 1
    });
    drop(client);
    in_retry.open();
    ended(in_process).expect("the in-process server ends cleanly at end of input");

    // The daemon.
    let (_daemon_dir, daemon_cfg) = fixture(RefreshPolicy::WarnOnly);
    let daemon_gate = Gate::closed();
    let daemon_retry = Gate::closed();
    let (daemon_builder, daemon_builds) = scripted(
        &daemon_cfg,
        vec![
            fail_after(&daemon_gate, "the repository could not be loaded"),
            build_after(&daemon_retry, &daemon_cfg),
        ],
    );
    let state = DaemonState::new(daemon_builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut client, worker) = connect(&state, settings);
    let actual = play_a_client_that_stopped_waiting(&mut client, &daemon_gate, || {
        status(&state).state == BuildState::Failed
    });
    close(client, worker);
    daemon_retry.open();

    assert_eq!(expected.lines().count(), 4, "initialize and three calls");
    for (index, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(got, want, "response {} differs", index + 1);
    }
    assert_eq!(actual.len(), expected.len());
    assert!(
        expected.contains("the repository could not be loaded"),
        "the failure's cause is in the transcript: {expected}"
    );
    wait_until("both servers to have started the retry", || {
        in_builds.load(Ordering::SeqCst) == 2 && daemon_builds.load(Ordering::SeqCst) == 2
    });
}
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
/// The owner's decision (D26): a call is not told of a failure that a retry has already
/// superseded.
````

with this:

````rust
/// The owner's decision (D26): a call is not told of a failure that a retry has already
/// superseded. The thread-free table of the rule is `control_tests.rs`; this test assumes that
/// A's call does not wake spuriously while attempt 1's notification is held, and would fail
/// loudly, not pass wrongly, if it did.
````

Append to `src/mcp/daemon/connection_tests.rs`:

````rust

/// A client that stopped waiting (zero seconds) on an attempt that then failed, while another
/// client has started the retry: the first client's next call is told `indexing`, not the
/// stale failure (D26, D32), and its call after that joins the retry.
#[test]
fn a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let retry_gate = Gate::closed();
    let (builder, builds) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    first_gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });

    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    b.send(CALLEES);
    wait_until("B's call to wait on the retry", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 2 && now.waiting == 1
    });

    let told = a.request(CALLEES);
    assert_eq!(
        told["result"]["structuredContent"]["status"], "indexing",
        "{told}"
    );
    assert!(
        !told.to_string().contains("first attempt failed"),
        "{told}"
    );
    let joined = a.request(CALLEES);
    assert_eq!(
        joined["result"]["structuredContent"]["status"], "warming",
        "{joined}"
    );

    retry_gate.open();
    assert_eq!(b.recv()["result"]["structuredContent"]["query"], RUN_CALLEES);
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
    close(b, b_worker);
}
````

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t11-red.log 2>&1; grep -E '^error' /tmp/s3a-t11-red.log | sort | uniq -c
```

Expected (observed): 33 errors: `E0599` no method named `join` (16), `unheard_failure` (6) and `told` (2) on `ConnectionBudget`; `E0425` cannot find function `answer_for` (5); and the unresolved `Arc` and `AttemptOutcome` in `budget_tests.rs`, which reaches them through `budget.rs`'s imports (4).

- [ ] **Step 3: Implement**

In `src/mcp/daemon/budget.rs`, replace this:

````rust
//! One connection's first-call wait (spec §4.2 "Readiness semantics", §5 "connection-local").
use crate::mcp::settings::ClientSettings;
use std::time::{Duration, Instant};

/// How long one connection may wait for the index, counted separately for each build attempt.
///
/// A connection's deadline for an attempt is set by its first call that arrives during that
/// attempt and is kept for every later call during it, so a client that sends several calls
/// while one build runs waits its budget once, not once per call. A new attempt (a retry after
/// a failed build) starts a new deadline.
///
/// The only way to make one is from a [`ClientSettings`], whose constructor has already
/// refused a wait above `FIRST_CALL_WAIT_MAX`. The budget bounds the wait for the build; it
/// says nothing about how long a query then takes.
pub(super) struct ConnectionBudget {
    wait: Duration,
    /// The attempt this connection last joined, and its deadline for that attempt.
    joined: Option<(u64, Instant)>,
}

impl ConnectionBudget {
    pub(super) fn new(client: &ClientSettings) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

    /// This connection's deadline for `attempt`, for a call that arrives at `now`.
    ///
    /// If `now + wait` cannot be represented the deadline is `now`: the call does not wait.
    /// No `ClientSettings` can ask for a wait that long.
    pub(super) fn deadline_for(&mut self, attempt: u64, now: Instant) -> Instant {
        if let Some((joined, deadline)) = self.joined {
            if joined == attempt {
                return deadline;
            }
        }
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        self.joined = Some((attempt, deadline));
        deadline
    }
}
````

with this:

````rust
//! One connection's first-call wait (spec §4.2 "Readiness semantics", §5 "connection-local"),
//! and what the connection has heard of the build attempt it last joined.
use super::coordinator::AttemptOutcome;
use crate::mcp::settings::ClientSettings;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long one connection may wait for the index, counted separately for each build attempt,
/// and whether the connection has heard how that attempt ended.
///
/// A connection's deadline for an attempt is set by its first call that arrives during that
/// attempt and is kept for every later call during it, so a client that sends several calls
/// while one build runs waits its budget once, not once per call. A new attempt (a retry after
/// a failed build) starts a new deadline.
///
/// The budget also keeps the outcome of the attempt the connection last joined. If every call
/// of the connection for that attempt was answered warming and the attempt then failed, the
/// connection's next call hears that failure before anything is started (D32); that is the
/// in-process server's warming, then failed, then retry.
///
/// The only way to make one is from a [`ClientSettings`], whose constructor has already
/// refused a wait above `FIRST_CALL_WAIT_MAX`. The budget bounds the wait for the build; it
/// says nothing about how long a query then takes.
pub(super) struct ConnectionBudget {
    wait: Duration,
    joined: Option<Joined>,
}

/// The attempt a connection last joined.
struct Joined {
    attempt: u64,
    /// This connection's deadline for the attempt.
    deadline: Instant,
    /// The attempt's outcome, shared with the coordinator and every call that joined it.
    outcome: Arc<AttemptOutcome>,
    /// Whether a call of this connection has been told how the attempt ended.
    told: bool,
}

impl ConnectionBudget {
    pub(super) fn new(client: &ClientSettings) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

    /// Join `attempt`, whose outcome is `outcome`, for a call that arrives at `now`, and return
    /// this connection's deadline for it. A call during the attempt the connection already
    /// joined keeps that deadline; a call during another attempt starts a new one.
    ///
    /// If `now + wait` cannot be represented the deadline is `now`: the call does not wait.
    /// No `ClientSettings` can ask for a wait that long.
    pub(super) fn join(
        &mut self,
        attempt: u64,
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
        deadline
    }

    /// Record that a call of this connection has been told how the attempt it joined ended.
    pub(super) fn told(&mut self) {
        if let Some(joined) = &mut self.joined {
            joined.told = true;
        }
    }

    /// The error of the attempt this connection last joined, if that attempt failed and no
    /// call of this connection has been told so. Records it as told: a connection hears its
    /// attempt's failure once.
    pub(super) fn unheard_failure(&mut self) -> Option<String> {
        let joined = self.joined.as_mut()?;
        if joined.told {
            return None;
        }
        match joined.outcome.ended() {
            Some(Err(error)) => {
                joined.told = true;
                Some(error.clone())
            }
            Some(Ok(())) | None => None,
        }
    }
}
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
/// How one build attempt ended: empty while it builds
````

with this:

````rust
/// What a call is told about its own attempt's ending `own`, given the phase now (D26). An
/// attempt that published the index is `Ready`. A failed attempt is `Superseded` when a later
/// attempt is building or the index is ready, and otherwise `Failed` with its own text. The
/// phase contributes only which of those it is: no other attempt's outcome or text.
pub(super) fn answer_for(own: &Result<(), String>, phase: &BuildPhase) -> Answer {
    match own {
        Ok(()) => Answer::Ready,
        Err(error) => match phase {
            BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Superseded,
            BuildPhase::Failed { .. } | BuildPhase::Idle => Answer::Failed {
                error: error.clone(),
            },
        },
    }
}

/// How one build attempt ended: empty while it builds
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
pub(super) use super::control::{Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState};
````

with this:

````rust
use super::control::answer_for;
pub(super) use super::control::{Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState};
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// The call first decides which attempt it belongs to, once:
````

with this:

````rust
    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// If the attempt this connection last joined has failed and no call of the connection has
    /// been told so (each was answered `Warming`), this call is answered about that attempt
    /// and starts nothing: `Failed` with its text, or `Superseded` (D32). Otherwise the call
    /// first decides which attempt it belongs to, once:
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
        let mut control = self.lock_control()?;

        let building = match &control.build {
            BuildPhase::Ready => return Ok(Answer::Ready),
````

with this:

````rust
        let mut control = self.lock_control()?;

        // A connection's next call first hears its own attempt's unheard failure, before the
        // shortcut to `Ready` and before a retry is started or joined (D32). It is answered by
        // the same rule as any other ending.
        if let Some(error) = budget.unheard_failure() {
            return Ok(answer_for(&Err(error), &control.build));
        }

        let building = match &control.build {
            BuildPhase::Ready => return Ok(Answer::Ready),
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            match outcome.ended() {
                Some(Ok(())) => return Ok(Answer::Ready),
                // The call's own attempt failed. One look at the phase, under the lock this
                // call holds, decides whether that failure is still the last word. The look
                // takes nothing from the phase but that: no outcome and no error of another
                // attempt can reach this call through it, which is what answering from the
                // phase once got wrong. `Building` here is always a later attempt, because
                // this call's own has ended.
                Some(Err(error)) => {
                    return Ok(match &control.build {
                        BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Superseded,
                        BuildPhase::Failed { .. } | BuildPhase::Idle => Answer::Failed {
                            error: error.clone(),
                        },
                    });
                }
                None => {}
            }
````

with this:

````rust
        let deadline = budget.join(attempt, &outcome, self.clock.now());

        loop {
            // The call's own attempt has ended. If it failed, one look at the phase, under the
            // lock this call holds, decides whether that failure is still the last word
            // (`answer_for`). The look takes nothing from the phase but that: no outcome and
            // no error of another attempt can reach this call through it, which is what
            // answering from the phase once got wrong. `Building` there is always a later
            // attempt, because this call's own has ended.
            if let Some(ended) = outcome.ended() {
                budget.told();
                return Ok(answer_for(ended, &control.build));
            }
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t11-green.log 2>&1; grep -E '^test result|FAILED|^error|^warning' /tmp/s3a-t11-green.log
```

Expected (observed): `62 passed; 0 failed` (49, then 4 budget, 3 rule, 4 coordinator, 1 parity and 1 connection test), no warning.

- [ ] **Step 5: Commit**

```bash
git add src/mcp
git commit -m "fix(mcp): a connection's next call first hears its own attempt's unheard failure"
```

The commit message says that `a_retry_gives_a_connection_a_fresh_full_budget` changes its expectation, and why.

- [ ] **Step 6: Prove the tests bite (after the commit)**

Five mutants, one at a time. After each, run the command and restore the file named. Mutant A is the code before this task.

Mutant A (the remembered outcome is never consulted), in `src/mcp/daemon/coordinator.rs`. Replace this:

````rust
        if let Some(error) = budget.unheard_failure() {
            return Ok(answer_for(&Err(error), &control.build));
        }
````

with nothing.

Mutant B (marked told before anything was answered), in `src/mcp/daemon/budget.rs`. Replace this:

````rust
            told: false,
````

with this:

````rust
            told: true,
````

Mutant C (the unheard failure is answered with the latest attempt's text), in `src/mcp/daemon/coordinator.rs`. Replace this:

````rust
            return Ok(answer_for(&Err(error), &control.build));
````

with this:

````rust
            let error = match &control.build { BuildPhase::Failed { error, .. } => error.clone(), _ => error }; return Ok(answer_for(&Err(error), &control.build));
````

Mutant D (an unheard failure is reported on every call), in `src/mcp/daemon/budget.rs`. Replace this:

````rust
                joined.told = true;
                Some(error.clone())
````

with this:

````rust
                Some(error.clone())
````

Mutant E (a call that waited and heard its attempt's ending is not marked told), in `src/mcp/daemon/coordinator.rs`. Replace this:

````rust
                budget.told();
````

with nothing.

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t11-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t11-bite.log
git checkout -- src/mcp/daemon/coordinator.rs src/mcp/daemon/budget.rs
```

Expected, and observed in validation:

| Mutant | Fails |
|---|---|
| A | `a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server`, `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile`, `a_connection_hears_its_own_attempt_s_failure_although_a_later_attempt_failed_too`, `a_connection_that_stopped_waiting_hears_its_attempt_s_failure_before_any_retry`, `a_connection_that_stopped_waiting_is_told_to_retry_when_another_retried_meanwhile`, `a_retry_gives_a_connection_a_fresh_full_budget` (at the 30 s hang limit) |
| B | `a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server`, `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile`, `a_connection_hears_its_own_attempt_s_failure_although_a_later_attempt_failed_too`, `a_connection_that_stopped_waiting_hears_its_attempt_s_failure_before_any_retry`, `a_connection_that_stopped_waiting_is_told_to_retry_when_another_retried_meanwhile`, `a_failure_no_call_was_told_of_is_reported_once`, `a_retry_gives_a_connection_a_fresh_full_budget`, `joining_a_new_attempt_remembers_that_attempt_and_not_the_old_one` (at the 30 s hang limit) |
| C | `a_connection_hears_its_own_attempt_s_failure_although_a_later_attempt_failed_too` |
| D | `a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server`, `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile`, `a_connection_that_stopped_waiting_hears_its_attempt_s_failure_before_any_retry`, `a_connection_that_stopped_waiting_is_told_to_retry_when_another_retried_meanwhile`, `a_failure_no_call_was_told_of_is_reported_once`, `a_retry_gives_a_connection_a_fresh_full_budget` |
| E | `a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries`, `a_builder_panic_is_a_failed_attempt_every_waiter_hears_of`, `a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served`, `a_failed_build_is_reported_as_the_in_process_server_reports_it_and_the_next_call_retries`, `a_waiter_whose_failed_attempt_was_superseded_by_a_building_retry_is_told_to_retry`, `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_told_to_retry`, `a_zero_wait_client_hears_a_failure_that_is_already_recorded`, `an_exhausted_attempt_counter_is_a_failure_that_starts_no_build` |

Task 10's mutants A, B and C now apply to `answer_for` in `src/mcp/daemon/control.rs`, the same lines at 12 spaces of indentation instead of 24, and Task 4's mutant G to `if let Some(ended) = outcome.ended() {` in place of `match outcome.ended() {`. Re-run on the code after Task 12, each fails every test it failed before; A, B and C also fail a table test of `answer_for` and some of this task's tests, and G fails the same three edge tests as before. The S3a-2 fold record has the counts.

---

### Task 12: An admitted connection is served blocking and with no read timeout

Found by the first review of S3a-2 (Opus, S4). S4's handshake stage reads the preamble from a non-blocking socket and may set a read timeout. `serve_admitted` documented that it needs a blocking stream with no read timeout and left it to the caller; a stream handed over as the handshake left it would end the connection as a read error at the first quiet moment. `serve_admitted` now sets the mode itself (D33).

**Files:**
- Modify: `src/mcp/daemon/connection.rs`
- Modify (tests): `src/mcp/daemon/connection_tests.rs`

**Interfaces:**
- Consumes: `serve_admitted` (Task 7); `libc` (already a dependency) for the test's `fcntl` probe.
- Produces: nothing new; `serve_admitted`'s contract no longer asks anything of the stream's mode.

- [ ] **Step 1: Write the test**

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
use crate::mcp::output::StructuredContentMode;
use crate::mcp::shared::preamble::BuildState;
````

with this:

````rust
use crate::mcp::output::StructuredContentMode;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::preamble::BuildState;
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
use super::super::coordinator::{AttemptHook, BuildStatus, DaemonState};
````

with this:

````rust
use super::super::coordinator::{AttemptHook, BuildStatus, DaemonState};
use super::serve_admitted;
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
use std::sync::Arc;
````

with this:

````rust
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;
````

Append to `src/mcp/daemon/connection_tests.rs`:

````rust

/// The handshake stage of a later slice reads the preamble from a non-blocking socket, and may
/// set a read timeout. A connection is served blocking and with no read timeout, whatever it
/// was handed in. Rules out leaving the mode to the caller: the first quiet moment on the
/// socket would end the connection as a read error.
#[test]
fn a_connection_handed_over_non_blocking_and_with_a_read_timeout_is_served_blocking() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (near, far) = UnixStream::pair().expect("a connected socket pair");
    far.set_nonblocking(true).expect("make the daemon's end non-blocking");
    far.set_read_timeout(Some(Duration::from_millis(1)))
        .expect("give the daemon's end a read timeout");
    let probe = far.try_clone().expect("a second handle on the daemon's end");
    let settings = client_settings(600);
    let worker = {
        let state = Arc::clone(&state);
        std::thread::spawn(move || serve_admitted(&state, &ToolRegistry::all_v1(), far, &settings))
    };
    let mut a = super::super::wire_support::Client::new(near);

    a.handshake();
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);

    assert_eq!(probe.read_timeout().expect("the read timeout"), None);
    // SAFETY: `probe` owns an open descriptor for the whole call, and F_GETFL only reads the
    // descriptor's flags.
    let flags = unsafe { libc::fcntl(probe.as_raw_fd(), libc::F_GETFL) };
    assert!(flags >= 0, "fcntl failed");
    assert_eq!(flags & libc::O_NONBLOCK, 0, "the socket is blocking");
    close(a, worker);
}
````

- [ ] **Step 2: Run it and see it fail**

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t12-red.log 2>&1; grep -E 'FAILED$|^test result|panicked' -A1 /tmp/s3a-t12-red.log
```

Expected (observed): this red step is a test failure, not a compile failure; the test uses only items that exist. `a_connection_handed_over_non_blocking_and_with_a_read_timeout_is_served_blocking ... FAILED` at the 30 s hang limit, `a response within the hang limit: ... WouldBlock`: the daemon's read on the non-blocking socket failed and ended its side, and the test's second handle on that socket keeps it open, so the client waits out its own read timeout. `62 passed; 1 failed`.

- [ ] **Step 3: Implement**

In `src/mcp/daemon/connection.rs`, replace this:

````rust
/// `stream` is the connection after the preamble: blocking, with no read timeout (a timeout
/// would end the connection as a read error).
````

with this:

````rust
/// `stream` is the connection after the preamble, in whatever mode the handshake left it: it
/// is set blocking and with no read timeout here, because a timeout or a would-block read
/// would end the connection as a read error.
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_client(state, registry, &mut transport, client)
````

with this:

````rust
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(None)?;
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_client(state, registry, &mut transport, client)
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t12-green.log 2>&1; grep -E '^test result|FAILED|^error|^warning' /tmp/s3a-t12-green.log
```

Expected (observed): `63 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/mcp
git commit -m "fix(mcp): serve an admitted connection blocking and with no read timeout"
```

- [ ] **Step 6: Prove the test bites (after the commit)**

Two mutants in `src/mcp/daemon/connection.rs`, one at a time. After each, run the command and restore the file.

Mutant A (the stream is left non-blocking). Replace this:

````rust
    stream.set_nonblocking(false)?;
````

with nothing.

Mutant B (the read timeout is left in place). Replace this:

````rust
    stream.set_read_timeout(None)?;
````

with nothing.

```bash
cargo test --features mcp --lib mcp::daemon::connection::tests::a_connection_handed_over > /tmp/s3a-t12-bite.log 2>&1; grep -E 'FAILED$|^test result|panicked' -A1 /tmp/s3a-t12-bite.log
git checkout -- src/mcp/daemon/connection.rs
```

Expected, and observed in validation: each fails `a_connection_handed_over_non_blocking_and_with_a_read_timeout_is_served_blocking`; A at the 30 s hang limit (`WouldBlock`), B at its assertion that the read timeout is `None` (the 1 ms timeout did not end the connection during the test's one exchange, which is why the assertion is there).

---

### Task 13: A call whose failed attempt a published retry has superseded is served

The owner's decision of 2026-10-13, amending D26: "on the superseded call finding index ready it [w]ould be better to be answered at once". A call whose own attempt failed, and which finds that a later attempt has already published the index, is served like any call that finds the index ready; it is not told `indexing`. `Superseded` (`indexing`) remains only for a later attempt that is still building. This holds on both paths that answer from an outcome: a parked waiter that wakes to its own failed attempt, and a connection's unheard failure (D32), which is then marked told, since nothing about the superseded failure is owed. Both paths go through `answer_for`, so the change is one arm of it.

**Files:**
- Modify: `src/mcp/daemon/control.rs`, `src/mcp/daemon/coordinator.rs` (docs), `src/mcp/daemon/connection.rs` (docs and one test-module line)
- Create (tests): `src/mcp/daemon/connection_ready_tests.rs` (its own file to keep `connection_tests.rs` under 600 lines)
- Modify (tests): `control_tests.rs`, `coordinator_order_tests.rs`, `coordinator_edge_tests.rs`, `connection_tests.rs`, `connection_parity_tests.rs`

**Interfaces:**
- Consumes: `answer_for` (Task 11).
- Produces: `answer_for(&Err(_), &BuildPhase::Ready) == Answer::Ready`; `Answer::Superseded` now means "a later attempt is building".

- [ ] **Step 1: Write the tests**

The rule's table splits its ready row out. The order test and the socket test whose retry has published now expect the call to be served (each renamed); the socket test of the `indexing` envelope moves to the building case, where it still applies. A coordinator test and a socket test cover the D32 path with the index ready. The parity leg also asserts that the call that hears the failure starts no build (the second review of S3a-2, Opus S2); that assertion passes before this task too.

In `src/mcp/daemon/control_tests.rs`, replace this:

````rust
/// A failed attempt is superseded when a later attempt is building or the index is ready.
#[test]
fn a_failed_attempt_is_superseded_by_a_building_retry_and_by_a_ready_index() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(answer_for(&own, &building()), Answer::Superseded);
    assert_eq!(answer_for(&own, &BuildPhase::Ready), Answer::Superseded);
}
````

with this:

````rust
/// A failed attempt is superseded when a later attempt is building: the call is told to retry.
#[test]
fn a_failed_attempt_is_superseded_by_a_building_retry() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(answer_for(&own, &building()), Answer::Superseded);
}

/// When a later attempt has published the index, a call whose own attempt failed is served at
/// once (the owner's decision of 2026-10-13, amending D26). Rules out telling it to retry with
/// nothing building.
#[test]
fn a_failed_attempt_whose_retry_published_the_index_is_answered_ready() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(answer_for(&own, &BuildPhase::Ready), Answer::Ready);
}
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
/// The retry has already succeeded when the waiter of attempt 1 runs. It is told to retry, and
/// its next call is served (D26). It is not told `Ready` either: a call is answered from its
/// own attempt, and its own attempt built nothing. Rules out reporting the superseded failure,
/// and answering from the current phase.
#[test]
fn a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_told_to_retry() {
````

with this:

````rust
/// The retry has already succeeded when the waiter of attempt 1 runs. Its own attempt failed,
/// and a later one has published the index since, so it is answered `Ready` and served at once
/// (the owner's decision of 2026-10-13, amending D26), and so is its next call. Rules out
/// reporting the superseded failure, and telling the call to retry with nothing building.
#[test]
fn a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served() {
````

In `src/mcp/daemon/coordinator_order_tests.rs`, replace this:

````rust
    assert_eq!(
        early.outcome(),
        Outcome::Superseded,
        "attempt 1 failed and the index is ready: the waiter of attempt 1 is told to retry"
    );
````

with this:

````rust
    assert_eq!(
        early.outcome(),
        Outcome::Ready,
        "attempt 1 failed and the index is ready: the waiter of attempt 1 is served"
    );
````

Append to `src/mcp/daemon/coordinator_edge_tests.rs`:

````rust

/// The owner's decision of 2026-10-13 on the path of D32: a connection that stopped waiting on
/// an attempt that then failed, while another connection's retry has published the index, is
/// answered `Ready` on its next call, and on the call after it. Rules out `Superseded` with
/// nothing building, and a failure reported after the connection was served.
#[test]
fn a_connection_that_stopped_waiting_is_served_once_another_s_retry_has_published() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![fail_after(&first_gate, "first attempt failed")]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let quick = Conn::open(&state, 0);
    let patient = Conn::open(&state, 600);

    quick.call();
    assert!(matches!(quick.outcome(), Outcome::Warming { .. }));
    first_gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });
    patient.call();
    assert_eq!(patient.outcome(), Outcome::Ready, "the retry published");

    quick.call();
    assert_eq!(quick.outcome(), Outcome::Ready);
    quick.call();
    assert_eq!(quick.outcome(), Outcome::Ready, "and nothing is owed after it");
    assert_eq!(status(&state).attempt, 2);
}
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
/// The owner's decision (D26): a call is not told of a failure that a retry has already
/// superseded. The thread-free table of the rule is `control_tests.rs`; this test assumes that
/// A's call does not wake spuriously while attempt 1's notification is held, and would fail
/// loudly, not pass wrongly, if it did. A's call waits on attempt 1, which fails; before A's call runs again, B's call
/// has started the retry and been served from it. A is then told that the repository is being
/// indexed and to retry, in the envelope of the other retryable statuses, and its retry is
/// served. Rules out answering `build_failed` with a stale cause, and a status that clients
/// keyed on the index state would not treat as warming.
#[test]
fn a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served() {
````

with this:

````rust
/// The owner's decisions (D26, amended 2026-10-13): a call is not told of a failure that a
/// retry has already superseded, and when that retry has published the index the call is
/// served at once. The thread-free table of the rule is `control_tests.rs`; this test assumes
/// that A's call does not wake spuriously while attempt 1's notification is held, and would
/// fail loudly, not pass wrongly, if it did. A's call waits on attempt 1, which fails; before
/// A's call runs again, B's call has started the retry and been served from it. A's call is
/// then served from the index. Rules out answering `build_failed` with a stale cause, and
/// telling A to retry when there is nothing to wait for.
#[test]
fn a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served() {
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
    // The retry's announcement woke A's call. Its own attempt failed; the index is ready.
    let told = a.recv();
    assert_eq!(told["id"], 7);
    assert_eq!(told["result"]["isError"], true, "{told}");
    assert_eq!(
        told["result"]["structuredContent"],
        json!({
            "status": "indexing",
            "retry_after_secs": 5,
            "message": "prism-mcp is indexing the repository; retry this call in about 5 seconds. If you already retried and that call succeeded, ignore this message."
        }),
        "{told}"
    );
    let text = told["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .as_ref(),
        Some(&told["result"]["structuredContent"]),
        "the text view carries the same status"
    );
    assert_eq!(told["result"]["_meta"]["prism/index_state"], "warming");
    assert_eq!(told["result"]["_meta"]["prism/retryable"], true);
    assert!(
        !told.to_string().contains("first attempt failed"),
        "the superseded failure is not sent: {told}"
    );
    notify.open();
````

with this:

````rust
    // The retry's announcement woke A's call. Its own attempt failed; the index is ready.
    let served = a.recv();
    assert_eq!(served["id"], 7);
    assert_eq!(
        served["result"]["structuredContent"]["query"], RUN_CALLEES,
        "{served}"
    );
    assert!(
        !served.to_string().contains("first attempt failed"),
        "the superseded failure is not sent: {served}"
    );
    notify.open();
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
/// A client that stopped waiting (zero seconds) on an attempt that then failed, while another
/// client has started the retry: the first client's next call is told `indexing`, not the
/// stale failure (D26, D32), and its call after that joins the retry.
````

with this:

````rust
/// A client that stopped waiting (zero seconds) on an attempt that then failed, while another
/// client has started the retry: the first client's next call is told that the repository is
/// being indexed and to retry, in the envelope of the other retryable statuses, not the stale
/// failure (D26, D32), and its call after that joins the retry. Rules out answering
/// `build_failed` with a stale cause, and a status that clients keyed on the index state would
/// not treat as warming.
````

In `src/mcp/daemon/connection_tests.rs`, replace this:

````rust
    let told = a.request(CALLEES);
    assert_eq!(
        told["result"]["structuredContent"]["status"], "indexing",
        "{told}"
    );
    assert!(!told.to_string().contains("first attempt failed"), "{told}");
````

with this:

````rust
    let told = a.request(CALLEES);
    assert_eq!(told["result"]["isError"], true, "{told}");
    assert_eq!(
        told["result"]["structuredContent"],
        json!({
            "status": "indexing",
            "retry_after_secs": 5,
            "message": "prism-mcp is indexing the repository; retry this call in about 5 seconds. If you already retried and that call succeeded, ignore this message."
        }),
        "{told}"
    );
    let text = told["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .as_ref(),
        Some(&told["result"]["structuredContent"]),
        "the text view carries the same status"
    );
    assert_eq!(told["result"]["_meta"]["prism/index_state"], "warming");
    assert_eq!(told["result"]["_meta"]["prism/retryable"], true);
    assert!(!told.to_string().contains("first attempt failed"), "{told}");
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
fn play_a_client_that_stopped_waiting(
    client: &mut Client,
    gate: &Gate,
    mut failed: impl FnMut() -> bool,
) -> String {
````

with this:

````rust
fn play_a_client_that_stopped_waiting(
    client: &mut Client,
    gate: &Gate,
    builds: &AtomicU64,
    mut failed: impl FnMut() -> bool,
) -> String {
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
    // The next call hears the failure, with its cause, and starts nothing.
    client.send(CALLEES);
    transcript.push_str(&client.recv_line());
````

with this:

````rust
    // The next call hears the failure, with its cause, and starts nothing.
    client.send(CALLEES);
    transcript.push_str(&client.recv_line());
    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "the call that hears the failure starts no build"
    );
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
play_a_client_that_stopped_waiting(&mut client, &in_gate, || {
````

with this:

````rust
play_a_client_that_stopped_waiting(&mut client, &in_gate, &in_builds, || {
````

In `src/mcp/daemon/connection_parity_tests.rs`, replace this:

````rust
play_a_client_that_stopped_waiting(&mut client, &daemon_gate, || {
````

with this:

````rust
play_a_client_that_stopped_waiting(&mut client, &daemon_gate, &daemon_builds, || {
````

Create `src/mcp/daemon/connection_ready_tests.rs`:

````rust
//! A daemon connection whose own build attempt failed, served once another client's retry has
//! published the index (the owner's decision of 2026-10-13, amending D26). Kept apart from
//! `connection_tests.rs` to keep both files under 600 lines.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    client_settings, fail_after, fixture, scripted, status, wait_until, Gate,
};
use super::super::wire_support::{close, connect, CALLEES};
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, StartupMode};
use std::sync::atomic::{AtomicU64, Ordering};

const RUN_CALLEES: &str = "callees:run@main.py";

fn built(builds: &AtomicU64) -> u64 {
    builds.load(Ordering::SeqCst)
}

/// The owner's decision of 2026-10-13 on the path of D32: a client that stopped waiting on an
/// attempt that then failed, while another client's retry has since published the index, is
/// served on its next call, and so is the call after it. Rules out telling it to retry
/// (`indexing`) with nothing building, and reporting the stale failure later.
#[test]
fn a_client_that_stopped_waiting_is_served_once_another_client_s_retry_has_published() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![fail_after(&first_gate, "first attempt failed")]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    first_gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });
    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);

    for _ in 0..2 {
        let served = a.request(CALLEES);
        assert_eq!(
            served["result"]["structuredContent"]["query"], RUN_CALLEES,
            "{served}"
        );
        assert!(
            !served.to_string().contains("first attempt failed"),
            "{served}"
        );
    }
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
    close(b, b_worker);
}
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
#[cfg(test)]
#[path = "connection_parity_tests.rs"]
mod parity_tests;
````

with this:

````rust
#[cfg(test)]
#[path = "connection_parity_tests.rs"]
mod parity_tests;

#[cfg(test)]
#[path = "connection_ready_tests.rs"]
mod ready_tests;
````

- [ ] **Step 2: Run them and see them fail**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t13-red.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t13-red.log
```

Expected (observed): a test failure, not a compile failure (no new item is needed). `61 passed; 5 failed`: `a_failed_attempt_whose_retry_published_the_index_is_answered_ready`, `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served` and `a_connection_that_stopped_waiting_is_served_once_another_s_retry_has_published` (each `Superseded` for `Ready`), and the two socket tests `a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served` and `a_client_that_stopped_waiting_is_served_once_another_client_s_retry_has_published` (each answered `indexing`, so no `query` in the result).

- [ ] **Step 3: Implement**

In `src/mcp/daemon/control.rs`, replace this:

````rust
    /// The index is published.
    Ready,
````

with this:

````rust
    /// The index is published: by the call's own attempt, or by a later attempt after the
    /// call's own failed (the owner's decision of 2026-10-13: such a call is served at once).
    Ready,
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
    /// The call's own attempt ended without an index, and the daemon has since moved on: a
    /// later attempt is building, or the index is ready. The failure is stale, so the call is
    /// not given it; it is told to retry. It is never given another attempt's outcome.
    Superseded,
````

with this:

````rust
    /// The call's own attempt ended without an index, and a later attempt is building. The
    /// failure is stale, so the call is not given it; it is told to retry. It is never given
    /// another attempt's outcome.
    Superseded,
````

In `src/mcp/daemon/control.rs`, replace this:

````rust
/// What a call is told about its own attempt's ending `own`, given the phase now (D26). An
/// attempt that published the index is `Ready`. A failed attempt is `Superseded` when a later
/// attempt is building or the index is ready, and otherwise `Failed` with its own text. The
/// phase contributes only which of those it is: no other attempt's outcome or text.
pub(super) fn answer_for(own: &Result<(), String>, phase: &BuildPhase) -> Answer {
    match own {
        Ok(()) => Answer::Ready,
        Err(error) => match phase {
            BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Superseded,
````

with this:

````rust
/// What a call is told about its own attempt's ending `own`, given the phase now (D26, as the
/// owner amended it on 2026-10-13). An attempt that published the index is `Ready`. A failed
/// attempt is `Ready` when a later attempt has published the index since (the call is served
/// at once), `Superseded` when a later attempt is building, and otherwise `Failed` with its
/// own text. The phase contributes only which of those it is: no other attempt's outcome or
/// text.
pub(super) fn answer_for(own: &Result<(), String>, phase: &BuildPhase) -> Answer {
    match own {
        Ok(()) => Answer::Ready,
        Err(error) => match phase {
            BuildPhase::Ready => Answer::Ready,
            BuildPhase::Building { .. } => Answer::Superseded,
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// been told so (each was answered `Warming`), this call is answered about that attempt
    /// and starts nothing: `Failed` with its text, or `Superseded` (D32). Otherwise the call
````

with this:

````rust
    /// been told so (each was answered `Warming`), this call is answered about that attempt
    /// and starts nothing: `Failed` with its text, `Superseded`, or `Ready` when a later attempt
    /// has published the index (D32). Otherwise the call
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived),
````

with this:

````rust
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived), and also when that attempt failed and, by the time the call
    ///   runs again, a later attempt has published the index (the owner's decision of
    ///   2026-10-13),
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// - `Superseded` if that attempt ended without an index and, by the time the call runs
    ///   again, a later attempt is building or the index is ready. The failure is stale then,
    ///   and the call is told to retry instead of being given it,
````

with this:

````rust
    /// - `Superseded` if that attempt ended without an index and, by the time the call runs
    ///   again, a later attempt is building. The failure is stale then, and the call is told
    ///   to retry instead of being given it,
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
//! One answer here is the daemon's alone. A call whose own build attempt failed, and which
//! runs again only after a retry has superseded that failure, is told that the repository is
//! being indexed and to retry (`indexing_status`); the in-process server has one client and no
//! such case.
````

with this:

````rust
//! One answer here is the daemon's alone. A call whose own build attempt failed, and which
//! runs again only while a later attempt is building, is told that the repository is being
//! indexed and to retry (`indexing_status`); the in-process server has one client and no such
//! case. If the later attempt has already published the index, the call is served.
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
/// How many seconds a client that is told `indexing` is asked to wait before it retries. The
/// owner asked for "retry in <retry interval seconds>" and named no number; five is a
/// placeholder to be changed here.
````

with this:

````rust
/// How many seconds a client that is told `indexing` is asked to wait before it retries. Five
/// is the owner's value (2026-10-13); a change needs measured build-time evidence (how long
/// builds take, by repository size), not an estimate.
````

In `src/mcp/daemon/connection.rs`, replace this:

````rust
/// What a call is told when its own build attempt failed and a retry has already superseded
/// that failure (a later attempt is building, or the index is ready): retry shortly, and
/// ignore this if a retry has already succeeded.
````

with this:

````rust
/// What a call is told when its own build attempt failed and a retry has already superseded
/// that failure (a later attempt is building): retry shortly, and ignore this if a retry has
/// already succeeded.
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t13-green.log 2>&1; grep -E '^test result|FAILED|^error|^warning' /tmp/s3a-t13-green.log
```

Expected (observed): `66 passed; 0 failed` (63, then one more rule test, one more coordinator test, one more socket test).

- [ ] **Step 5: Commit**

```bash
git add src/mcp
git commit -m "fix(mcp): serve a call whose failed attempt a published retry has superseded"
```

- [ ] **Step 6: Prove the tests bite (after the commit)**

Mutant A (a failure superseded by a ready index is told to retry: the code before this task), in `src/mcp/daemon/control.rs`. Replace this:

````rust
            BuildPhase::Ready => Answer::Ready,
````

with this:

````rust
            BuildPhase::Ready => Answer::Superseded,
````

```bash
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t13-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t13-bite.log
git checkout -- src/mcp/daemon/control.rs
```

Expected, and observed in validation: `a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served`, `a_client_that_stopped_waiting_is_served_once_another_client_s_retry_has_published`, `a_connection_that_stopped_waiting_is_served_once_another_s_retry_has_published`, `a_failed_attempt_whose_retry_published_the_index_is_answered_ready`, `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served`.

Task 10's mutant A now replaces `BuildPhase::Building { .. } => Answer::Superseded,` (the `Ready` arm is its own); B and C are unchanged. Re-run on the code after this task with Task 11's five mutants, each is still killed; the S3a-2 fold-2 record has the counts.

---

### Task 14: S3a-2 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md` (MCP Adapter section)

**Interfaces:**
- Consumes: Tasks 6–14.
- Produces: the S3a-2 pull request.

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
that attempt only, up to the connection's `ConnectionBudget` (`budget.rs`: one deadline per
connection per attempt); it is answered from that attempt's own `AttemptOutcome`, written once,
whatever the phase has become since. A failed attempt is retried by the next call, once however
many calls arrive, and the build thread puts the provider in the slot and records its generation
````

with this:

````markdown
that attempt only, up to the connection's `ConnectionBudget` (`budget.rs`: one deadline per
connection per attempt, and the outcome of the attempt it last joined); it is answered from that
attempt's own `AttemptOutcome`, written once, whatever the phase has become since. A connection
that stopped waiting before its attempt failed hears that failure on its next call before
anything is started (the in-process server's `warming`, `build_failed`, retry). A failed attempt
is retried by the next call from a connection with no unheard failure, once however many calls
arrive, and the build thread puts the provider in the slot and records its generation
````

In `CLAUDE.md`, replace this:

````markdown
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
````

with this:

````markdown
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
`control.rs` holds the types the control lock guards (`BuildPhase`, `ControlState`,
`AttemptOutcome`, `BuildStatus`) and `Answer`, what `ensure_ready` tells a call: `Ready`,
`Warming`, `Failed` with the call's own attempt's error, or `Superseded` when that attempt
failed and a later attempt is already building; when a later attempt has already published the
index, a call whose attempt failed is answered `Ready` and served (the pure rule `answer_for`).
`connection.rs` serves one admitted connection (`serve_admitted`, which first sets the stream
blocking with no read timeout): `transport::serve_loop`, the
frame loop the in-process server also runs, then `handle_control` with no lock, then for a tool
call `ensure_ready` followed by `dispatch_tool_read` under the slot's read lock, or
`dispatch_tool_refresh` / `dispatch_tool_auto_refresh` under its write lock. Nothing is served
from the slot before `ensure_ready` has answered `Ready`. A query sees the whole provider, so
stale evidence a refresh recorded outranks a clean stamp probe for every client. Before the
index is ready a call gets the in-process server's `warming` or `build_failed` result; a
`Superseded` call gets the daemon's own retryable `indexing` status (retry in
`INDEXING_RETRY_AFTER_SECS`; ignore it if a retry already succeeded), which the in-process
server never sends. Not there yet (the next slice): the re-check of an automatic refresh after
exclusive access is acquired, and panic boundaries.
````

- [ ] **Step 2: Run the MCP surface and the checks**

```bash
cargo fmt --check
cargo test --features mcp --lib mcp:: > /tmp/s3a2-final-lib.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a2-final-lib.log
env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp > /tmp/s3a2-final-mcp.log 2>&1; grep -E '^test result|FAILED|differs|^error' /tmp/s3a2-final-mcp.log
cargo check > /tmp/s3a2-final-default.log 2>&1; tail -1 /tmp/s3a2-final-default.log
cargo check --tests --features mcp,detached-owner-audit > /tmp/s3a2-final-owner.log 2>&1; tail -1 /tmp/s3a2-final-owner.log
cargo clippy --all-targets --features mcp -- -W clippy::all > /tmp/s3a2-final-clippy.log 2>&1
grep -E -A8 '^warning|^error' /tmp/s3a2-final-clippy.log | grep -E 'src/mcp/daemon/|src/mcp/transport_loop_tests\.rs' | sort -u
grep -E -A8 '^warning|^error' /tmp/s3a2-final-clippy.log | grep -E 'src/mcp/transport\.rs' | sort -u > /tmp/s3a2-clippy-transport.txt; wc -l < /tmp/s3a2-clippy-transport.txt
```

Expected: `cargo fmt --check` prints nothing; the lib line shows the Task 1 base + 69 passed (35 from S3a-1, then 5 + 8 + 5 + 2 + 13 + 1 + 3; observed: `452 passed` on a base of 380), `0 failed`; the `mcp` target shows the base number unchanged, including `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; both `cargo check` runs end with `Finished`. The first clippy filter prints nothing (no warning in a file this slice adds). The second counts the lines that name `transport.rs`, which this slice edits: it prints `0`, as it does on the base (observed on both).

- [ ] **Step 3: State the invariants**

```bash
BASE=$(git merge-base origin/main HEAD)
git diff --stat "$BASE" -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs | wc -l
git diff --stat "$BASE" -- tests/fixtures/mcp/golden | wc -l
git diff --stat "$BASE" -- src/mcp/framing.rs src/mcp/output.rs src/mcp/lazy.rs src/mcp/owner.rs src/mcp/settings.rs src/mcp/session.rs src/mcp/shared src/bin/prism-mcp.rs tests/mcp | wc -l
git diff --name-only "$BASE" -- src tests | grep -v -E '^src/mcp/daemon/'
git diff "$BASE" -- src/mcp/transport.rs | grep -c -E '^[-+].*(Parse error|-32600|-32601|-32602)'
wc -l src/mcp/daemon/*.rs src/mcp/transport_loop_tests.rs | sort -n | tail -4
```

Expected, with S3a-1 already on `main`: three zeros; the fourth command prints exactly `src/mcp/transport.rs` and `src/mcp/transport_loop_tests.rs`; the fifth prints `0` (no line that produces a protocol error was added or removed in the serving file); the largest file is under 600 lines (observed: `connection_tests.rs`, 574 lines; `coordinator.rs` 486).

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
cargo test --features mcp > /tmp/s3a2-full.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/s3a2-full.log
cargo test --doc --features mcp > /tmp/s3a2-doc.log 2>&1; grep -E '^test result' /tmp/s3a2-doc.log
for i in $(seq 1 30); do cargo test --features mcp --lib mcp::daemon > /tmp/s3a2-repeat-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3a2-repeat-$i.log; done | sort | uniq -c
python3 scripts/mutgate/mutgate.py > /tmp/s3a2-mutgate.log 2>&1; tail -5 /tmp/s3a2-mutgate.log
```

Expected: every `test result` line shows `0 failed`; the doc tests pass; the repeat prints one line, `30 test result: ok. 66 passed; 0 failed`; the mutation gate exits 0, reported with the same sentence as in Task 5. A failure outside this slice's files is reported, not fixed or re-baselined here.

- [ ] **Step 5: Commit and open the PR**

Write the PR body to `/tmp/s3a2-pr-body.md`. It states, in this order:

1. **Goal:** the lane goal, and that S3a-2 serves one admitted connection through S3a-1's coordinator; still no accept loop, so still no production caller.
2. **Invariants:** the Step 3 output; that the golden transcripts pass unblessed after the one edit to the serving path; and Task 6 Step 4's diff of `transport.rs`. **Parity, as claimed:** a daemon connection writes the in-process server's bytes for a whole session under a warn-only and an automatic policy (Task 8), and for a client that stops waiting before its build fails: `warming`, then `build_failed` with the cause, then the retry (Task 11). Not claimed: a superseded failure, which only a second client can cause and which is answered with the daemon's own `indexing` status while the retry builds, or served when it has published (D26 as amended); one `StructuredContentMode`; the other tools; cache-backed startup; and `Background` startup, because a startup attempt that fails before the first call is joined by no connection, so the first call starts the retry instead of reporting the failure as the in-process server does (the second review of S3a-2, Opus F1; S4 must restore it, see "Interfaces for the next slices").
3. **Decisions:** D14, D17, D18, D21, D22, D23, the owner's D26, D29, D30, D32, D33 and SI-1, SI-2, SI-5, SI-8, verbatim or by link; the owner question on a superseded call when the index is ready.
4. **What S3b inherits:** the three things `connection.rs`'s module comment lists, and the "Panics in S3a" table.
5. **Totals:** the Step 2 and Step 4 numbers next to the Task 1 base numbers, and the mutant results of Tasks 6, 7, 8, 10, 11, 12 and 13 as observed.
6. **Not verified:** a non-Unix build; the mutation gate does not cover MCP code; the blocked-writer case (spec §9 test 7, S3b); the scenarios through a real shim and a real daemon process (S4, S5a).
7. **Tier-A harness:** not run; this slice touches none of `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`.

Then:

```bash
git add CLAUDE.md
git commit -m "docs(mcp): a daemon connection, the control-plane types and the indexing answer"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): one connection served through the readiness coordinator (shared daemon S3a-2)" --body-file /tmp/s3a2-pr-body.md
```

Review brief and cap as in Task 5.

---

## Interfaces for the next slices

Everything below is in `crate::mcp::daemon` and `pub(super)` unless a path says otherwise. Field and method names are the ones this plan's code uses.

**S3b (concurrent dispatch, refresh visibility, auto-refresh re-check, panic boundaries) consumes and changes:**

- `DaemonState::ensure_ready` returns `control::Answer { Ready, Warming { elapsed }, Failed { error }, Superseded }`, not `lazy::Readiness`, chosen by `control::answer_for` from the call's own attempt's outcome and the phase's discriminant. A connection's next call first hears its own attempt's unheard failure (D32), so a `ConnectionBudget` holds state, not only a deadline: one per connection, kept for the connection's life, never shared. `Superseded` is answered with the daemon's `indexing` status and has no counterpart in process (D26, the owner's). `Failed` means the call's own attempt failed and nothing has superseded it; the `build_failed` text's "the next tool call retries the build" is then true.
- `connection::run_tool_call(state: &Arc<DaemonState>, budget: &mut ConnectionBudget, call: &ToolCall<'_>, settings: &ConnectionSettings) -> anyhow::Result<Value>` (private). It is the one function that takes the slot's locks for a call: the re-check after `state.write` is acquired goes into its last statement, where `ReadDispatch::NeedsAutoRefresh { initial_report }` has been taken out of the read lock.
- `DaemonState::read<R>(&self, impl FnOnce(&SessionProvider) -> R) -> Result<Option<R>, Poisoned>` and `DaemonState::write<R>(&self, impl FnOnce(&mut SessionProvider) -> R) -> Result<Option<R>, Poisoned>`. A panic boundary around a handler wraps the closure passed to these two. The closure must not call `read` or `write` again (the slot's lock is not re-entrant; a nested call can deadlock); `status()` is safe from inside it. Both look at the slot as it is, which is filled a moment before `Ready` is announced. `write` already leaves the generation in the control plane; `SessionProvider::generation()` (now `pub`) is what a response is bound to.
- `Poisoned { lock: &'static str }`: every place it is constructed (`coordinator.rs`: `lock_control`, `ensure_ready`'s wait, `read`, `write`) is a place the abort belongs.
- `coordinator::PanicGuard`'s `Drop`: the body to replace with the abort.
- `TestHooks { published, settled }` and `AttemptHook`: the pattern for an injected panic in dispatch (spec §9 test 12).
- For its tests: `test_support::{ManualClock, Gate, scripted, Conn, wait_until, advance, HANG}`, `wire_support::{Client, connect, close, ended}`, and `connection_parity_tests::session` as the model of a whole session.

**S4 (ownership, handshake stage, admission and retirement, Draining) consumes:**

- `DaemonState::new(builder: SessionBuilder, startup: StartupMode) -> anyhow::Result<Arc<DaemonState>>`. `StartupMode::Background` is `--warm-at-startup` on the daemon's command line (spec §11 item 7); `StartupMode::Eager` is refused. `SessionBuilder` is `crate::mcp::lazy::SessionBuilder`; the in-process equivalent is `Arc::new(move || SessionProvider::bootstrap(&cfg))`.
- `connection::serve_admitted(state: &Arc<DaemonState>, registry: &ToolRegistry, stream: UnixStream, client: &ClientSettings) -> anyhow::Result<()>`, called on the worker thread after `READY` is written. It sets the stream blocking with no read timeout itself (D33), whatever the handshake stage left. `client` is the `ClientSettings` from the decoded `Hello`; `serve_admitted` reads its output settings and its first-call wait and nothing else. One `ToolRegistry::all_v1()` can be shared by all workers (it is asserted `Send + Sync`). An `Err` return is an ordinary event (the peer left mid-frame); its text is an OS error, the fixed `Poisoned` text, or the fixed "no index is published" text, so it carries no peer bytes, but bound it anyway where it is logged.
- `DaemonState::status() -> Result<BuildStatus, Poisoned>` with `BuildStatus { state: BuildState, attempt: u64, generation: u64, waiting: u64 }`. `state` and `generation` are `READY`'s `build` and `generation` (`shared::preamble::Ready`); it takes only the control lock, so admission never waits on the provider. It is also the content of the status probe (SI-2): S4 adds the verb and its own fields. A shim must not infer readiness from the generation: in the interval after publication `status()` reports `building` with the generation of the provider already in the slot, which need not be 0.
- `ControlState { build: BuildPhase, attempts, waiting, generation }`: the struct to add the pending and admitted counts, the last-activity instant and the retirement decision to, under the same mutex (spec §4.2). `DaemonState`'s `control` and `changed` fields are private to `coordinator.rs`; S4 either adds its admission functions to that file or widens them to `pub(super)`. Retirement's "no build in progress" is `!matches!(control.build, BuildPhase::Building { .. })`. `BuildPhase::Ready` has no payload; the generation is `control.generation`. The types are in `daemon/control.rs` (Task 9); `record_generation` still takes the number from its caller and should read it from the guard it is handed when S3b or S4 next changes it (a reviewer's request, not done here).
- `clock::{Clock, SystemClock}`, `Parts { clock, .. }` and `DaemonState::wake()` for the idle-TTL test with an injected clock (spec §9 test 2).
- From the first review of S3a-1, for S4 to decide or keep: **an attempt has no end if the builder never returns** (a FIFO in the repository, a dead network mount). Calls stay bounded, each by its budget, but the phase stays `Building`, so a retirement rule of "no build in progress" never fires and every later client is told warming until the process is killed; S4 decides whether a build older than some bound stops blocking retirement or `daemon-shutdown` (a watchdog that fails the attempt would let a second builder run beside the hung one, which breaks "one build"). **Any new kind of waiter on the control plane's one condition variable must keep `notify_all` and re-check its own predicate in a loop**; a `notify_one`, or a waiter that trusts a wake-up, steals wake-ups from readiness calls. **`BuildPhase::Failed { error }` and an attempt's outcome hold the builder's error whole**, cloned once per waiter under the control lock; nothing bounds it before S4 does. `BuildPhase::Failed { attempt, error }` has no reader outside tests since calls are answered from their own outcome; it is kept for S4's status reply and should be removed if S4 does not use it. **A failure can go unreported in two cases.** By design, when every call that waited on the attempt has been superseded (D26). And, as built here and **not** as in process, under `Background` startup (`--warm-at-startup`) when the startup attempt fails before any connection's first call: no connection joined attempt 1, so the first call starts the retry and is answered `warming`, where the in-process server answers `build_failed` with the cause and retries on the next call (the second review of S3a-2, Opus F1, with a probe). With a wait at least as long as the retry takes to fail, and a deterministic failure, the cause arrives one build later through D32. A client whose wait had run out on an attempt it joined hears the failure on its next call (D32), as in process; one that never calls again does not, as in process.
- **S4 requirement: parity for a failed startup build under `Background`.** S4 wires `--warm-at-startup`, so S4 restores parity for the case above. One way (Opus's): `DaemonState` keeps attempt 1's `Arc<AttemptOutcome>` from `from_parts`; a connection admitted while `attempts == 1` has its `ConnectionBudget` seeded with it as if it had joined; `Joined.deadline` becomes `Option<Instant>`, set by the connection's first call, so the seed does not start the budget. A connection's first call then hears attempt 1's failure once (`Failed` with its text, `Superseded`, or served when ready), as each in-process `--warm-at-startup` client hears its own build fail. Parity test sketch (the reviewer's probe, which fails on this slice's code; run each side's startup build to failure before the client's first call):

  ````rust
  #[test]
  fn a_warm_daemon_whose_startup_build_failed_before_the_first_call_reports_it_like_the_in_process_server() {
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
          wait_until("the startup build to fail", || delivered2.load(Ordering::SeqCst) == 1);
          let mut transport = UnixStreamTransport::from_stream(far).expect("a transport");
          serve_connection(&mut provider, &ToolRegistry::all_v1(), &mut transport, &output)
      });
      let mut client = Client::new(near);
      let mut expected = String::new();
      client.send(INIT);
      expected.push_str(&client.recv_line());
      client.send(INITED);
      client.send(CALLEES);
      expected.push_str(&client.recv_line()); // build_failed, with the cause
      client.send(CALLEES);
      expected.push_str(&client.recv_line()); // the retry: warming
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
      wait_until("the startup build to fail", || status(&state).state == BuildState::Failed);
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
- **S4 notes on D33 (the second review of S3a-2, Opus S3).** `serve_admitted` clears the read timeout and the non-blocking flag but keeps any **write** timeout the handshake set; S4 decides whether that is what it wants for slow readers. `serve_admitted` takes the raw `UnixStream`, so S4's preamble reader must not read past the `HELLO` line (anything it buffered would be lost); the shim sends no MCP byte before `READY`, so a line-exact reader is enough.
- To do there: remove `#![cfg_attr(not(test), allow(dead_code))]` from `daemon/mod.rs`; decide SI-5 (a parked call of a departed client holds its admitted slot for up to its first-call wait); and bound and strip control characters from `BuildPhase::Failed { error }` wherever it is logged or sent in a status reply (it is the repository loader's text, stored whole so that the tool result's `cause` is the in-process server's).

**S6 (docs)** owes the wire vocabulary of readiness: `warming`, `build_failed`, and the daemon's `indexing` with `retry_after_secs`, and the `prism/index_state` values (the first review of S3a-2, Opus S6).

**S5a (the shim) consumes nothing here by name.** It owes the lane two things this slice could only do in process: spec §9 tests 1 and 14 through real shims (SI-1), and the byte parity of Task 8 through a shim against `--standalone`, with `tests/mcp/transcript.rs`. **S5a design item:** the shim checks the daemon's peer uid on the socket it connects to. The S2a reviews concluded that the guarantee between users is the peer-uid check on both ends; the spec has only the daemon's side. Not an owner question.

## Self-review record

- **Spec coverage (§12 row S3a).** Readiness coordinator → Tasks 3 and 4 (`DaemonState`). Per-connection, per-attempt budgets → Task 2 (`ConnectionBudget`) and Task 3 (`ensure_ready`). Publication before `Ready` → Task 3 (`publish`), held by Task 4's hook test. Single retry → Task 3 (`start_attempt` under the lock that read the phase), in the reading D5 states. `RwLock<Option<SessionProvider>>` → Task 3 (the slot, `read`), Task 4 (`write`), Task 7 (dispatch through both). Test-only status probe → `DaemonState::status`, in process (D15, SI-2). Test 1 → Task 7 `two_clients_share_one_build`. Test 14 → Task 8, two tests. Both in-crate (D17, SI-1).
- **Spec §9 unit list, S3a's item:** "readiness coordinator: per-connection, per-attempt budgets, terminal-outcome delivery, single retry" → Task 2's five tests; Task 3 `the_spec_s_worked_example_two_budgets_on_one_attempt`, `later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget`, `a_retry_gives_a_connection_a_fresh_full_budget`, `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself`, `the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together`; Task 4's late-waiter test.
- **Spec §10 rows S3a's code answers:** "Client sends invalid JSON → same `-32700`/`-32600` responses as stdio; connection stays" → Task 6 (the loop is the same code) and Task 8's parity session (garbage, an array, a call before `initialize`, a request with no id). "Panic in provider dispatch, build, or control plane → abort" and "panic in a connection's framing/output → that connection closes" → not S3a; the "Panics in S3a" table says what happens meanwhile.
- **Spec §4.2 sentences checked against the code:** "the build runs on one thread" (one `start_attempt`, from `Idle` or `Failed` only); "publishes the provider into the provider slot before announcing Ready" (`publish`); "a thread holding Control never waits on the provider" (`coordinator.rs` takes the slot in `publish`, `read`, `write` and the test-only `unpublish_for_tests`, none of them with the control lock held; `record_generation` takes control while holding the slot, the permitted order); "queries … operate on a read-only facade of the whole `SessionProvider`" (`read` hands out `&SessionProvider`, which is S1's facade through `impl SessionRuntime for SessionProvider`); "lifecycle and control methods run with no provider lock" (`serve_client` calls `handle_control` before anything else); "the in-process server keeps `LazySessionProvider` and `SessionRuntime` unchanged" (`lazy.rs` untouched; no `SessionRuntime` method added).
- **The lane's review lessons, one by one.** (1) Every stated property has a test and a mutant, or is listed as not held by a test: the "Claims" table. (2) What each mechanism does not cover: "What each mechanism covers, and what it does not". (3) Interleavings: publication before `Ready` and the late waiter are played through hooks (Task 4); one retry among many callers and one build among many clients are excluded by one lock acquisition and demonstrated with eight and with two callers; a response is written with no lock held by construction, and its many-clients test is S3b's. (4) No test asserts an upper bound on a duration or sleeps to order events; socket pairs need no path, so nothing depends on a temp-directory layout; the one real-clock test takes one second and asserts a lower bound. (5) S3a writes no log line and puts no peer bytes into an error; what leaves `serve_admitted` as an error is listed under S4's interface. (6) No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests in `src/mcp/daemon/`. The `expect`s that remain on a daemon client's path are in code this slice does not edit (`transport.rs`, `tools_refresh.rs`, `freshness.rs`: each serializes a value the server itself built) and in the tool handlers, which this plan did not audit; what a panic there does today is in the "Panics in S3a" table, and what it should do is S3b's boundary. (7) Every wait in `ensure_ready` has a deadline from a `ConnectionBudget`, which only a `ClientSettings` can make (at most 600 s); the control lock is held for constant work; a departed client's call is bounded by its deadline and its build is the daemon's, not the client's.
- **Deliberate differences from the lane plan's S3a note:** the seam of `serve_loop` (D18).
- **Size.** S3a-1's counts are from the validated branch. S3a-2's are line counts of this plan's code blocks after `rustfmt` (see "What was checked without a compiler" below), not counts from a built tree.

  | PR | File | Lines |
  |---|---|---|
  | S3a-1 | `session.rs` +7, `session_stage_tests.rs` +23, `src/mcp/mod.rs` +2 | 32 |
  | | `daemon/mod.rs` | 25 |
  | | `daemon/budget.rs` 47, `budget_tests.rs` 80 | 127 |
  | | `daemon/clock.rs` | 22 |
  | | `daemon/coordinator.rs` | 536 |
  | | `daemon/test_support.rs` | 280 |
  | | `daemon/coordinator_tests.rs` | 398 |
  | | `daemon/coordinator_failure_tests.rs` | 268 |
  | | `daemon/coordinator_order_tests.rs` | 374 |
  | | `CLAUDE.md` | 15 |
  | | **S3a-1 total** | **2,080 insertions, 3 deletions, as built after the review fold** (1,705 before it; code 640, tests 1,420, docs 15) |
  | S3a-2 | `transport.rs` +44 −6, `transport_loop_tests.rs` 125 | 169 |
  | | `daemon/connection.rs` | 155 |
  | | `daemon/control.rs` (104, of which 85 moved from `coordinator.rs`) | 104 |
  | | `daemon/coordinator.rs` +40 −97 (483 lines after Task 10) | 40 |
  | | `daemon/wire_support.rs` | 120 |
  | | `daemon/connection_tests.rs` | 473 |
  | | `daemon/connection_parity_tests.rs` | 354 |
  | | S3a-1's tests and support, changed by Task 10: +82 −41 | 82 |
  | | `daemon/mod.rs` +4, `CLAUDE.md` +15 | 19 |
  | | **S3a-2 total before its review** | **1,516 insertions, 144 deletions, as built in validation** (code about 330 including the 85 moved lines, tests about 1,170, docs 15) |
  | | Tasks 11 and 12 and their docs: `budget.rs`, `control.rs`, `coordinator.rs`, `connection.rs`; `control_tests.rs` (new), and the budget, coordinator, edge, connection and parity tests | 637 insertions(+), 53 deletions(-) |
  | | **S3a-2 total** | **2127 insertions(+), 171 deletions(-)**, 18 files |

  `coordinator.rs` reached 549 lines after Task 8; Task 9 moves the control-plane types out, and it is 483 after Task 10 and 486 after Task 11. The largest files are test files: `connection_tests.rs` 574, `coordinator.rs` 486, `connection_parity_tests.rs` 466.
- **What was checked without a compiler.** Every file directive of this plan ("Create", "Replace the whole of", "In … replace", "Append") was applied in order by a script to a copy of the eight files involved, with `rustfmt --edition 2021` run on the result after each task. Every anchor matched exactly once, at the point in the sequence where the plan uses it and after formatting; every mutant's anchor matches exactly once in the final formatted tree; `rustfmt` parsed every file, so the code blocks are syntactically valid Rust; the resulting diff of `transport.rs` is +27 −6 with both `-32700` branches untouched; and `transport_tests.rs`, `owner_transport_tests.rs` and `transport_split_tests.rs` are byte-identical. That is a check of the plan's text. It says nothing about types, borrows, lints or behavior, and has since been superseded for every task by the replays with a compiler recorded below. The same text replay is still what shows that the plan and the two branches agree byte for byte.
- **Not verified, beyond the list at the top:** that the fixture repository's responses are identical across two temporary directories for this plan's session (the lane plan measured it for a 25-response script that contains every request used here); the wording of `std`'s error for a custom `io::Error` in `a_build_thread_that_cannot_be_started_…` (the test expects the message passed to `io::Error::new`); that `SO_RCVTIMEO` set on the client's socket also bounds reads through its `try_clone` (it is one socket; if not, a hung server would hang a test instead of failing it at 30 s); Linux behavior of a write to a socket pair whose peer has closed (the two departed-client tests accept either outcome of that write).

## Validation record (S3a-1, Tasks 1–5)

Replayed on 2026-10-13 on macOS (arm64, rustc 1.94.0), branch `scratch/s3a-validate` in `~/code/prism-s3a`, from `main` `86756aad`. Every directive was applied by a script from this file's text; every cargo command ran alone, as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo …`, while a memory measurement and other builds held the host (load average 33–60 throughout). Not pushed.

| Task | Commit |
|---|---|
| 1 | `804458d8` feat(mcp): expose the session provider's refresh generation |
| 2 | `60f81d4d` feat(mcp): per-connection, per-attempt first-call budget for the shared daemon |
| 3 | `2c32107d` feat(mcp): readiness coordinator and provider slot for the shared daemon |
| 4 | `a54831cd` feat(mcp): exclusive provider access that keeps the control plane's generation current |
| (correction to Task 3) | `4e14ba4d` fix(mcp): drop an explicit deref clippy flags in the build thread's panic guard |
| 5 | `430acad5` docs(mcp): the shared daemon's readiness coordinator |

The correction is its own commit because the branch's rule was one commit per task and no amending. This plan's Task 3 now contains the corrected line, so a fresh replay produces the same tree without that commit.

| Step | Command | Observed |
|---|---|---|
| T1 S1 | `cargo test --features mcp --lib mcp::` on the base | `380 passed; 0 failed; 0 ignored; 0 measured; 1426 filtered out` |
| T1 S3 / S5 | red, then `mcp::session::stage_tests::generation` | `E0599` no method `generation`; then `1 passed; 0 failed` |
| T1 S7 | mutant: accessor returns 0 | `generation_counts_committed_refreshes_and_nothing_else ... FAILED`; `0 passed; 1 failed` |
| T2 S3 / S5 | red, then `mcp::daemon::budget` | 9 compile errors; then `5 passed; 0 failed` |
| T2 S7 | mutants A, B | A: `2 passed; 3 failed`; B: `a_new_attempt_gets_a_fresh_full_budget ... FAILED`, `4 passed; 1 failed` |
| T3 S4 / S7 | red, then `mcp::daemon::coordinator` | 44 compile errors; then `12 passed; 0 failed` on the first compile of the implementation, in 1.02 s |
| T3 S7 | `mcp::daemon`; `cargo check --features mcp` | `17 passed; 0 failed`; `Finished` with no warning of any kind |
| T3 S9 | mutants A–E | A: 6 tests failed (30 s); B: `an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left`; C: `on_the_real_clock_…` (30 s); D: `a_build_thread_that_cannot_be_started_…` (30 s); E: `a_builder_panic_…` (30 s). Each run ended; none hung. |
| T4 S2 / S4 | red, then `mcp::daemon::coordinator`; `mcp::daemon` | `E0599` no method `write` (and `E0282`); then `16 passed; 0 failed`; `21 passed; 0 failed` |
| T4 S6 | mutants A–C | A: the publication test and the poisoned-slot test; B: the late-waiter test (30 s); C: the generation test |
| extra | four mutants from the "Claims" table, on the Task 4 tree | Build at construction for a lazy daemon: 4 tests failed, among them `nothing_is_built_until_the_first_readiness_requiring_call`. `notify_one` when an attempt ends: `a_failed_attempt_is_delivered_to_every_call_…` and `the_next_call_after_a_failure_…` (30 s). No decrement of `waiting`: 7 tests failed. A call that answers `Failed` by starting the retry: 5 tests failed, among them `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself`. |
| T5 S2 | `cargo fmt --check`; `--lib mcp::`; `cargo check`; `cargo check --tests --features mcp,detached-owner-audit` | no output; `402 passed; 0 failed`; `Finished`; `Finished` |
| T5 S2 | clippy, filtered to this slice's files | First run: one new warning, `clippy::explicit_auto_deref` at `state: &*self` in `run_build` (the correction below). After the correction: only `--> src/mcp/session.rs:542:10`, the warning in `plan_refresh` that this slice does not touch; 258 `warning` lines in the whole log. |
| T5 S3 | the invariants | `0`, `0`, `0`; `src/mcp/mod.rs`, `src/mcp/session.rs`, `src/mcp/session_stage_tests.rs`; `3`; largest file `coordinator.rs` 500 lines; 13 files changed, 1,705 insertions, 3 deletions |
| T5 S4 | 30 × `mcp::daemon`, default threads | `30 test result: ok. 21 passed; 0 failed`; test time 1.00–1.15 s per run; load average 40–57 |
| T5 S4 | 30 × `mcp::daemon -- --test-threads=1` | `30 test result: ok. 21 passed; 0 failed`; test time 1.38–2.37 s per run; load average 35–56 |

**Corrections made to this plan by the validation.**

| # | Where | What the plan said | What was wrong | What it says now |
|---|---|---|---|---|
| 1 | Task 3 Step 6, `run_build` | `state: &*self,` | Compiles and passes, but clippy (`explicit_auto_deref`, in `clippy::all`) flags it: a new warning in a file this slice adds, which the plan's own constraint forbids. | `state: &self,` |
| 2 | Task 5 Step 2, clippy filter | "the last command prints nothing" | `session.rs` already has one clippy warning on the base (`filter_map_bool_then` in `plan_refresh`), so the filter prints one line. | The expectation names that line and why it is not this slice's. |
| 3 | Task 3 Step 7 | "Two tests print a panic message from a helper thread" | The harness captures it; a passing run prints nothing. | Says so. |
| 4 | Task 3 Step 9, mutant A | three failing tests | Six fail. | Lists the six. |
| 5 | Task 4 Step 2 | one expected error | `E0282` accompanies `E0599`. | Names both. |
| 6 | Task 4 Step 6, mutant A | one failing test | The poisoned-slot test fails too. | Names both. |
| 7 | Self-review, size | `CLAUDE.md` 13 lines; S3a-1 "about 1,700" | 12 lines; 1,705 insertions as built. | Corrected. |

Nothing in Tasks 1–5 failed to hold as designed: no test needed a change, no stated count was wrong, no mutant survived, and no run of 60 under load differed.

**Not run in this validation:** `cargo test --features mcp --test mcp` (so neither its base total nor the golden transcripts), the full `cargo test --features mcp` suite, `cargo test --doc --features mcp`, the mutation gate, a release build, the Tier-A harness, anything on Linux (S3a-2 has its own record below). Clippy was not run on the base in the same environment; that the one `session.rs` warning is not this slice's rests on the expression it flags being outside this slice's diff.

## Review fold record (S3a-1, review round 1)

Two reviewers (Opus 5.5 and gpt-6.1-sol) reviewed the S3a-1 branch at `430acad5`. Both confirmed the lock discipline, the state machine, the wait bounds and that standalone is unchanged, both accepted SI-2 and D4/D5, and both returned FIX on the same two defects. The fold was made on the S3a-1 branch (`feat/mcp-daemon-s3a-1`, worktree `~/code/prism-s3a`), test first, in three commits; not pushed by the folder.

| Commit | What |
|---|---|
| `be6841d9` | fix(mcp): answer a waiting call with the outcome of the attempt it joined |
| `a2da5edf` | fix(mcp): keep the control plane's generation true before Ready is announced |
| `52a45bc3` | fix(mcp): checked attempt numbers, a 15-byte build thread name, and the contracts of the injected clock and spawner |

**The two defects, each with its regression seen to fail before the fix and pass after.**

| Finding | Regression test | On `430acad5` | After |
|---|---|---|---|
| A late waiter is told another attempt's outcome (sol F1, opus W2) | `a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_has_succeeded` | failed: told `Ready` | passes |
| | `…_although_the_retry_failed_differently` | failed: told `second attempt failed` | passes |
| | `…_although_a_third_attempt_is_building` | failed: told `second attempt failed` | passes |
| Exclusive access between publication and `Ready` leaves a stale generation announced (sol F2, opus W1) | `exclusive_access_between_publication_and_ready_keeps_the_generation_current` | failed: `status().generation` 0, provider at 1 | passes |

The existing late-waiter test (`a_waiter_that_wakes_after_the_retry_began_still_hears_of_its_own_attempt_s_failure`) and the publication test passed before and pass after, unchanged apart from asserting on `Gate::pass`.

**The smaller items.**

| Item | What was done |
|---|---|
| opus S1: no test pinned where a deadline is counted from | `a_connection_that_joins_a_running_build_gets_its_whole_budget` added (it passes on `430acad5` too; the mutant it exists for is Task 3's mutant F, which passed all 21 earlier tests) |
| opus S3: `read`/`write` closures can re-enter the slot | stated in both docs; also what a caller may assume before `ensure_ready` has returned ready |
| sol S1: injected `Clock` and `Spawner` under-specified | stated in both docs |
| sol S2: attempt exhaustion had no outcome | checked add; `an_exhausted_attempt_counter_is_a_failure_that_starts_no_build` (failed before: a build was started for a number already used). The test sets the private counter from a child test module; no production seam was added |
| opus S4: thread name of 16 bytes | `prism-idx-build`; `the_build_thread_s_name_fits_in_what_linux_keeps` (failed before) |
| opus S4: `Gate::pass` returned silently after its limit | returns `bool`; the tests assert on it |
| opus S4: background start with a spawn failure had no test | `a_warm_daemon_whose_build_thread_cannot_be_started_is_built_by_the_first_call` added (passes before and after; the brief asked only that it be listed as not held by a test) |
| opus S4: `wake()`'s claim | listed under "stated and not held by a test" |
| opus S2, S4 and sol: a builder that never returns; one condition variable; unbounded `Failed.error` | recorded for S4 under "Interfaces for the next slices" |

**What ran on the folded branch (`52a45bc3`), each cargo command alone, niced, under host load.**

| Command | Observed |
|---|---|
| `cargo fmt --check` | no output |
| `cargo test --features mcp --lib mcp::daemon` | `30 passed; 0 failed` (21 before, plus 9) |
| `cargo test --features mcp --lib mcp::session` | `19 passed; 0 failed` |
| `cargo test --features mcp --lib mcp::` | `411 passed; 0 failed` (380 + 31) |
| `cargo check --features mcp` | `Finished`, no warning |
| clippy, filtered to this slice's files | only `--> src/mcp/session.rs:542:10`, as before; 258 `warning` lines in the whole log |
| Task 3's seven mutants, Task 4's five, and four more from the "Claims" table | all sixteen killed; those that make a call wait ended at the 30 s limit. Results are in the two tasks' tables |
| 30 × `mcp::daemon`, default threads | `30 test result: ok. 30 passed; 0 failed`; test time 1.24–2.21 s; load average 31–41 |
| 30 × `mcp::daemon -- --test-threads=1` | `30 test result: ok. 30 passed; 0 failed`; test time 1.37–2.52 s; load average 30–39 |
| The plan's state after Task 3 (the final coordinator without `write` and without the order tests) | `22 passed; 0 failed`, no warning |
| Text replay of this plan's directives, `rustfmt` after each task, compared with the branch | every S3a-1 file byte-identical |

**Departures from the fold brief, and one thing for the owner.**

- The brief asked that announcing `Ready` take the generation from the new field. The fold goes one step further: `BuildPhase::Ready` no longer carries a generation at all, so there is one copy and `status()` reads it whatever the phase. During the interval between publication and the announcement `status().generation` is therefore already the slot's while the state is still `building`. No interleaving was found in which it differs from the provider's generation once both locks are released. Two things it does not cover: a closure passed to `write` that panics leaves the slot poisoned and the field unwritten (the slot can then not be read at all), and a poisoned control lock leaves it unwritten (`status()` then returns the error).
- The brief asked for a mutant "fill a cell twice". With a `OnceLock` that mutant cannot be written; a unit test states the property instead.
- D26, as the brief decided: a waiter of a failed attempt is told of that failure even when the index is ready by the time it runs. That is spec §4.2 read literally, and the client's next call succeeds. The cost is one `build_failed` answer, carrying "the next tool call retries the build", for an index that is in fact ready. The alternative (answer `Ready` whenever the index is ready, before reading the attempt's outcome) is one line and is never wrong for the client; it would leave that attempt's failure unreported to that call. Not changed; the owner may prefer it.

**Not run for the fold:** the `--test mcp` binary tests, the full suite, the doc tests, the mutation gate, anything on Linux. The earlier validation record's counts (12, 16, 17, 21, 402) describe the branch before the fold.

**Review round 2 (2026-10-13).** Both reviewers approved `52a45bc3` with no WRONG finding. Their test notes were folded by the controller as one test-only commit, `f2d56b1c` (`coordinator_edge_tests.rs`, five tests; the third-attempt test moved there from `coordinator_order_tests.rs`), which this plan's Task 4 now contains; `--lib mcp::daemon` is 34 tests and `--lib mcp::` 415 there. Their plan-text notes are in: the `SLOT_POISONED` row of "Panics in S3a"; under "Interfaces for the next slices", that a shim must not infer readiness from the generation, that `BuildPhase::Failed`'s fields have no reader outside tests, and where a failure goes unreported; D31 for the file split, done in Task 9, and for the change to `record_generation` that a reviewer tied to it, not done. S3a-1 was squash-merged to `main` as `e7dc1cc2` (pull request #374). The decision the fold recorded as D26 was then replaced by the owner's; Task 10 builds it.

## Validation record (S3a-2, Tasks 6–10 and the first docs)

Replayed on 2026-10-13 on macOS (arm64, rustc 1.94.0), branch `scratch/s3a2-validate` in `~/code/prism-s3a2`. It was first replayed on S3a-1 as it was before its first review (`430acad5`), rebased onto the review fold (`52a45bc3`), then onto the approved head with the controller's edge tests (`f2d56b1c`), and finally onto `main` at `e7dc1cc2`, the squash merge of S3a-1 (pull request #374), whose tree is `f2d56b1c`'s. **The base of Tasks 6–11 is `origin/main` at `e7dc1cc2`.** Every rebase applied without a conflict; the last changed no file, so what was run on `f2d56b1c` is what is on `main`. Not pushed.

| Task (numbered as in this plan now) | Commit (on `main` `e7dc1cc2`) |
|---|---|
| 6 | `9be8cecf` refactor(mcp): extract the frame loop so a daemon connection can share it |
| 7 | `37256c4f` feat(mcp): serve one admitted connection through the daemon's coordinator |
| 8 | `360f06bd` test(mcp): a daemon connection matches the in-process server and keeps sticky stale evidence |
| 13 (docs as first written) | `b000b343` docs(mcp): a daemon connection and the shared frame loop |
| 7 (the test the S3a-1 reviewers asked for) | `f31a39e4` test(mcp): a call is not served from the slot before Ready is announced |
| 9 | `53788958` refactor(mcp): move the control-plane types out of coordinator.rs |
| 10 | `12a09171` feat(mcp): tell a call to retry when a retry has superseded its failed build attempt |
| 13 (docs for Tasks 9 and 10) | `5e58e646` docs(mcp): the control-plane types and the daemon's indexing answer |

The branch has more commits than the plan has tasks: the test added to Task 7 and the docs commits were made after the tasks they belong to and nothing was amended. The fold's commits are in the next record.

**As first written (Tasks 6, 7, 8), on `430acad5`.** Every red step was a compile failure with the expected error; every green step gave the stated count at the first compile: Task 6 `5 passed` (lib 407 = 402 + 5), Task 7 `7 passed` (module 28), Task 8 `5 passed` (module 33), lib 419. The byte-parity test passed for both refresh policies at the first run, and both sticky-evidence tests passed. `--test mcp` gave `23 passed` before and after Task 6's edit, the golden transcripts unblessed; the diff of `transport.rs` was +27 −6 with both `-32700` branches untouched; `cargo check --tests --features mcp,detached-owner-audit` finished. All 11 mutants were killed.

**On the review fold (`52a45bc3`), with the test added to Task 7.** Lib `429 passed`; `--test mcp` `23 passed`; all 12 mutants killed; 30 of 30 `ok. 43 passed; 0 failed` with default threads and with `--test-threads=1`, at load averages of 46–74.

**Final, after Tasks 9 and 10, on the tree of `main` `e7dc1cc2` (run when the branch sat on `f2d56b1c`, the same tree).**

| Command | Observed |
|---|---|
| `cargo fmt --check` | no output |
| `cargo test --features mcp --lib mcp::daemon` after Task 9 | `47 passed; 0 failed`; `cargo check --features mcp` with 0 warnings |
| Task 10 red | `E0432` unresolved import `Answer`; `E0599` no method `ended` |
| `cargo test --features mcp --lib mcp::daemon` after Task 10 | `49 passed; 0 failed` at the first compile |
| `cargo test --features mcp --lib mcp::` | `435 passed; 0 failed` (380 + 55) |
| `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `23 passed; 0 failed`, with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok` |
| `cargo check`; `cargo check --features mcp`; `cargo check --tests --features mcp,detached-owner-audit` | `Finished` each; 0 warnings from the feature build |
| clippy, filtered to `src/mcp/daemon/`, `transport_loop_tests.rs` and `transport.rs` | no line (258 `warning` lines in the whole log) |
| 30 × `mcp::daemon`, default threads | `30 test result: ok. 49 passed; 0 failed`; test time 1.27–1.61 s |
| 30 × `mcp::daemon -- --test-threads=1` | `30 test result: ok. 49 passed; 0 failed`; test time 1.55–3.38 s |
| Zero-diff checks against `origin/main` | `transport_tests.rs`, `owner_transport_tests.rs`, the goldens, `framing.rs`, `output.rs`, `lazy.rs`, `owner.rs`, `settings.rs`, `session.rs`, `shared/`, `src/bin`, `tests/`: no change. Outside `src/mcp/daemon/` the branch edits `src/mcp/transport.rs` (+44 −6) and adds `src/mcp/transport_loop_tests.rs`. 14 files changed, 1516 insertions(+), 144 deletions(-). Largest file: `coordinator.rs`, 483 lines |
| Text replay of this plan, `rustfmt` after each task, compared with the branch | every file byte-identical; the tree after Task 5 byte-identical to `main` `e7dc1cc2` |

**Mutants, on the final code.** Every one was killed; those that make a call wait ended at the 30 s limit.

| Mutant | Result |
|---|---|
| T6A a lifecycle state per frame | 1 failed of 5 |
| T6B a handler error swallowed | 1 failed of 5 |
| T6C a malformed frame is not answered | 1 failed of 5 |
| T7A a budget per message | 1 failed of 14, at 30 s |
| T7B the daemon environment decides output | 11 failed of 14 |
| T7C every daemon advertises eager | 2 failed of 14 |
| T7D readiness is not answered | 4 failed of 14 |
| T7E a call is served from whatever is in the slot | 1 failed of 14 |
| T8A sticky evidence loses to the stamp probe | 2 failed of 5 |
| T8B panic on an empty slot | 1 failed of 5 |
| T8C every daemon advertises eager | 1 failed of 5 |
| T8D the generation is not copied | 3 failed of 5 |
| T10A a superseded failure is still reported | 4 failed of 49 |
| T10B every waiter of a failed attempt is told to retry | 11 failed of 49 |
| T10C a failure is reported with the latest attempt text | 2 failed of 49 |
| T10D a superseded call is answered as warming | 1 failed of 49 |
| T10E the indexing answer carries another index state | 1 failed of 49 |
| S1F no record of the generation at publication | 1 failed of 49 |
| S1G the deadline is checked before the outcome | 3 failed of 49 |

**Corrections and additions made to Tasks 6–11 by the validation.**

| # | Where | What |
|---|---|---|
| 1 | Tasks 6, 7, 8 | No code correction was needed: every block compiled and every count was right as first written. Three mutants killed more tests than predicted (Task 7 B and D, Task 4 A); the tables now list what failed. |
| 2 | Task 7 | One test added, `a_call_is_not_served_from_the_slot_before_ready_is_announced`, with mutant E. Both S3a-1 reviewers noted that nothing in the library forces a caller of `read` or `write` through `ensure_ready`; no earlier test failed if serving skipped that. `REFRESH` moved from Task 8's support to Task 7's for it. |
| 3 | Task 8 | Its mutant D re-anchored to the generation field of the review fold. |
| 4 | Task 9 (new) | The split of the control-plane types, a pure move. |
| 5 | Task 10 (new) | The owner's decision on superseded failures: `Answer`, the `indexing` status, one function in `transport.rs`; two coordinator tests renamed and re-expected, one split in two, one connection test added; five mutants. |
| 6 | Task 13 (was 9, then 11) | Counts, the docs paragraph, and the clippy expectation for `transport.rs` (0 lines on the base and on the branch). |

**Not run in this validation:** the unfiltered `cargo test --features mcp`, `cargo test --doc --features mcp`, the mutation gate, a release build, the Tier-A harness, anything on Linux. The byte-parity sessions have one client per server and so never reach a superseded failure; parity for that case is not claimed, because the in-process server has no such case. **Superseded by the review fold below:** this record claimed that the superseded case was the only one outside parity; the first review of S3a-2 found a single-client case (a client that stops waiting before its build fails), now Task 11, with a parity leg.

## Review fold record (S3a-2, review round 1)

Two reviewers reviewed pull request #377 at `5e58e646`: gpt-6.1-sol approved with one test note (a held notification is not a held waiter); Opus 5.5 returned FIX on one demonstrated parity break (F1) and five SMELLs. The fold was made on `feat/mcp-daemon-s3a-2` (worktree `~/code/prism-s3a2`), test first, new commits only, not pushed by the folder, every cargo command alone, niced, with four jobs.

| Commit | What |
|---|---|
| `71c600f0` | fix(mcp): a connection's next call first hears its own attempt's unheard failure |
| `74f29035` | fix(mcp): serve an admitted connection blocking and with no read timeout |
| `7bfb393e` | docs(mcp): an attempt's unheard failure and the stream mode in the daemon notes |
| `15f67d84` | fix(mcp): keep BuildPhase's doc comment on BuildPhase |

The correction is its own commit because nothing is amended: Task 11's first edit put `control.rs`'s test-module declaration between `BuildPhase` and its doc comment (clippy's `empty_line_after_doc_comments`, a new warning in a file this slice adds, and `BuildPhase` lost its doc). This plan's Task 11 has the declaration at the end of the file, so a fresh replay produces the corrected tree without that commit.

**Each regression, failing before and passing after.** "Before" is Task 11's mutant A, which removes the one check Task 11 adds to `ensure_ready` and so is the code at `5e58e646` with the new budget in place (the new tests do not compile against `5e58e646` itself). Task 12's test was run red against `5e58e646`'s `connection.rs`.

| Finding | Regression | Before | After |
|---|---|---|---|
| Opus F1: a client that stopped waiting never hears its build failed | `a_client_that_stopped_waiting_hears_the_failure_and_then_the_retry_like_the_in_process_server` (byte parity) | failed: response 3 is `warming`, `prism/index_state` `warming`, where the in-process server wrote `build_failed` with `cause: "the repository could not be loaded"` | passes |
| | `a_connection_that_stopped_waiting_hears_its_attempt_s_failure_before_any_retry` | failed: `Warming { elapsed: 0ns }` for `Failed("first attempt failed")` | passes |
| | `a_connection_that_stopped_waiting_is_told_to_retry_when_another_retried_meanwhile` | failed: `Warming` for `Superseded` | passes |
| | `a_connection_hears_its_own_attempt_s_failure_although_a_later_attempt_failed_too` | failed: `Warming` for `Failed("first attempt failed")` | passes |
| | `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile` (sockets) | failed: `status` `warming` for `indexing` | passes |
| | `a_retry_gives_a_connection_a_fresh_full_budget` (changed expectation) | failed at the 30 s limit: the call that should hear the failure started the retry and waited on it | passes |
| | `a_connection_whose_attempt_published_while_it_was_not_waiting_is_served` (over-reach guard) | passes, as it should | passes |
| Opus S3, sol S1: the D26 rule held only through threads | the three table tests of `answer_for` | (new function) | pass; Task 10 A, B, C each fail them |
| Opus S4: the stream's mode | `a_connection_handed_over_non_blocking_and_with_a_read_timeout_is_served_blocking` | failed at the 30 s limit: `WouldBlock` | passes |

With Task 11 A applied, `--lib mcp::daemon` gave `56 passed; 6 failed` (the six rows above that fail).

**Mutants, on the code after Task 12.** Every one was killed; those that make a call wait ended at the 30 s limit. Task 10's A, B and C now apply to `answer_for` in `control.rs` and Task 4's G (S1G) to `if let Some(ended) = outcome.ended() {`; each was re-anchored, not re-designed.

| Mutant | Result |
|---|---|
| T6A a lifecycle state per frame | 1 failed of 5 |
| T6B a handler error swallowed | 1 failed of 5 |
| T6C a malformed frame is not answered | 1 failed of 5 |
| T7A a budget per message | 3 failed of 17, at 30 s |
| T7B the daemon environment decides output | 14 failed of 17 |
| T7C every daemon advertises eager | 3 failed of 17 |
| T7D readiness is not answered | 6 failed of 17 |
| T7E a call is served from whatever is in the slot | 1 failed of 17 |
| T8A sticky evidence loses to the stamp probe | 2 failed of 6 |
| T8B panic on an empty slot | 1 failed of 6 |
| T8C every daemon advertises eager | 2 failed of 6 |
| T8D the generation is not copied | 3 failed of 6 |
| T10A a superseded failure is still reported | 7 failed of 63 |
| T10B every waiter of a failed attempt is told to retry | 16 failed of 63 |
| T10C a failure is reported with the latest attempt text | 4 failed of 63 |
| T10D a superseded call is answered as warming | 2 failed of 63 |
| T10E the indexing answer carries another index state | 1 failed of 63 |
| S1F no record of the generation at publication | 1 failed of 63 |
| S1G the deadline is checked before the outcome | 3 failed of 63 |
| Task 11 A: the remembered outcome is never consulted (the code before Task 11) | 6 failed of 63, at 30 s |
| Task 11 B: marked told on joining | 8 failed of 63, at 30 s |
| Task 11 C: the unheard failure answered with the latest attempt's text | 1 failed of 63 |
| Task 11 D: an unheard failure reported on every call | 6 failed of 63 |
| Task 11 E: a waited-for ending not marked told | 8 failed of 63 |
| Task 12 A: the stream left non-blocking | 1 failed of 63, at 30 s |
| Task 12 B: the read timeout left in place | 1 failed of 63 |

The Task 7 and Task 8 mutants fail more tests than before the fold (the module has the new connection and parity tests); none fails fewer.

**What ran on the folded branch.** The mutants, the repeat loops, `--lib mcp::transport`, `--test mcp` and the three `cargo check` builds ran on `7bfb393e`, before the correction, which moves one module declaration and changes no code; after it, `cargo fmt --check`, `--lib mcp::daemon` (63), `--lib mcp::` (449) and clippy ran again on `15f67d84`.

| Command | Observed |
|---|---|
| `cargo fmt --check` | no output |
| Task 11 red | 33 compile errors: `E0599` `join` (16), `unheard_failure` (6), `told` (2); `E0425` `answer_for` (5); `Arc`, `AttemptOutcome` (4) |
| `cargo test --features mcp --lib mcp::daemon` after Task 11 | `62 passed; 0 failed`, no warning |
| Task 12 red | `62 passed; 1 failed`, at the 30 s limit |
| `cargo test --features mcp --lib mcp::daemon` | `63 passed; 0 failed` |
| `cargo test --features mcp --lib mcp::transport` | `94 passed; 0 failed` |
| `cargo test --features mcp --lib mcp::` | `449 passed; 0 failed` (380 + 69) |
| `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `23 passed; 0 failed`, with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok` |
| `cargo check`; `cargo check --features mcp`; `cargo check --tests --features mcp,detached-owner-audit` | `Finished` each; 0 warnings from the feature build (the owner-audit build prints the base's warnings in `tests/algo/novel/contract_delta_test.rs` and `tests/ast/cpg_cache_test.rs`, not this slice's) |
| clippy, filtered to `src/mcp/daemon/`, `transport_loop_tests.rs` and `transport.rs` | before the correction: one line, `src/mcp/daemon/control.rs:63:1` (`empty_line_after_doc_comments`); after it: no line (258 `warning` lines in the whole log) |
| 30 × `mcp::daemon`, default threads | `30 test result: ok. 63 passed; 0 failed`; test time 1.25–1.67 s |
| 30 × `mcp::daemon -- --test-threads=1` | `30 test result: ok. 63 passed; 0 failed`; test time 1.60–3.64 s; load averages 26.61–37.21 across both loops |
| Zero-diff checks against `origin/main` | as before the fold; the fold edits only `src/mcp/daemon/` and `CLAUDE.md`. 18 files changed, 2127 insertions(+), 171 deletions(-) |
| Text replay of this plan, `rustfmt` after each task, compared with the branch | 86 directives, every anchor matched once and every mutant anchor once at its point in the sequence; all 25 files the plan touches byte-identical with `15f67d84`, `CLAUDE.md` included; the tree after Task 5 (18 files) byte-identical to `main` `e7dc1cc2` |

**What was not folded, and why.**

- Opus S5 (make `control` a child module of `coordinator`): left as a sibling; D31 says why.
- Opus S2 (serve a superseded call when the index is ready): the owner's D26 is kept; recorded as an owner question under "Decisions owed".
- Opus S6 (the wire vocabulary): recorded for S6.
- sol S1's preferred form (a test-only gate where a waiter resumes): not built. The brief chose the table tests of an extracted rule (Opus S3's form) and a note in the two thread tests' docs; the gate would be more machinery for a rule that now has thread-free tests.

**Deviations from the fold brief, accepted by the controller before they were built.**

- The brief's third F1 test, "a connection whose attempt ended ready proceeds normally", is built as an over-reach guard (`a_connection_whose_attempt_published_while_it_was_not_waiting_is_served`): it passes before the fix too, because the case it guards was never wrong; it fails a D32 that reports any ending.
- `a_retry_gives_a_connection_a_fresh_full_budget` pinned the old behavior and changes its expectation (stated in the Task 11 commit message).
- The brief's mutant "mark told before answering" is built as Task 11 B (told on joining, which is before any answer) and two more were added (D: reported on every call; E: a waited-for ending not marked told).

**Two statements in the brief that needed care.** "Answered `warming` for it and not yet told": the code does not record that a call was answered `warming`; it records only whether a call of the connection was told the ending, which is equivalent because a call that joined an attempt and returned without hearing its ending was answered `warming` (or ended its connection with an error, after which there is no next call). "The second attempt held open" in the parity leg: the in-process side's retry must also be held, or its fourth answer could be `Ready` if the retry built within the zero wait; both sides hold it.

**Not run for the fold:** the unfiltered suite, the doc tests, the mutation gate, a release build, anything on Linux.

## Review fold record (S3a-2, review round 2)

Both reviewers reviewed `15f67d84`. gpt-6.1-sol approved, with two text notes (the sentence on who starts a retry; D3's old cost line) and one inherited test note (held notifications, needing nothing). Opus 5.5 found no code defect in the fold and two text defects: F1, a failed startup build under `Background` is not reported as in process, while the plan said a failure goes unreported in one case only (a new instance of round 1's class, which Opus asked to route to S4 as text, not to a third code round); F2, the same sentence as sol's first note. Its S1 named D32's path for the owner question on a ready index, S2 a stronger parity assertion, S3 two S4 notes. With them came the owner's two decisions of 2026-10-13: a superseded call that finds the index ready is served at once, and the retry interval is 5 s. The fold was made test first on `wip/s3a2-fold2` (worktree `~/code/prism-s3a2-f2`), from `15f67d84`, new commits only, not pushed by the folder.

| Commit | What |
|---|---|
| `c5fd83cd` | fix(mcp): serve a call whose failed attempt a published retry has superseded |
| `248da0ca` | docs(mcp): a superseded call finding the index ready is served; who starts a retry |

**Each new test, failing before and passing after** (before: the test commit's tests on `15f67d84`'s code, `61 passed; 5 failed`; after: `66 passed; 0 failed`).

| Test | Before | After |
|---|---|---|
| `a_failed_attempt_whose_retry_published_the_index_is_answered_ready` | `Superseded` for `Ready` | passes |
| `a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served` (renamed from `…_is_told_to_retry`) | `Superseded` for `Ready` | passes |
| `a_connection_that_stopped_waiting_is_served_once_another_s_retry_has_published` | `Superseded` for `Ready` | passes |
| `a_call_whose_failed_attempt_was_superseded_by_a_published_retry_is_served` (renamed from `…_is_told_to_retry_and_its_retry_is_served`) | answered `indexing`: no `query` in the result | passes |
| `a_client_that_stopped_waiting_is_served_once_another_client_s_retry_has_published` | answered `indexing`: no `query` in the result | passes |
| the building case still answers `indexing`: `a_failed_attempt_is_superseded_by_a_building_retry`, `a_client_that_stopped_waiting_is_told_to_retry_when_another_client_retried_meanwhile` (now with the whole envelope) and the Task 10 building tests | pass | pass |
| the parity leg's new assertion (one build after the call that hears the failure) | passes | passes |

**Mutants, on `248da0ca` (the code of `c5fd83cd`; the later commit is docs).** Every one was killed.

| Mutant | Result |
|---|---|
| N1 a failure superseded by a ready index is told to retry | 5 failed of 66 |
| T10A a superseded failure is still reported | 5 failed of 66 |
| T10B every waiter of a failed attempt is told to retry | 16 failed of 66 |
| T10C a failure is reported with the latest attempt text | 4 failed of 66 |
| T10D a superseded call is answered as warming | 1 failed of 66 |
| T10E the indexing answer carries another index state | 1 failed of 66 |
| M1 no unheard failure is heard | 6 failed of 66, at 30 s |
| M2 told before answering | 8 failed of 66, at 30 s |
| M3 the unheard failure is answered from the latest attempt | 1 failed of 66 |
| M4 an unheard failure is heard again | 6 failed of 66 |
| M5 a waited-for outcome is not marked told | 5 failed of 66 |

N1 is Task 13's mutant A; T10A is re-anchored to `BuildPhase::Building { .. } => Answer::Superseded,`; M1–M5 are Task 11's A–E. T10D and T10E are now killed by the building-case socket test, which carries the `indexing` envelope checks.

**What ran.**

| Command | Observed |
|---|---|
| `cargo fmt --check` | no output |
| `cargo test --features mcp --lib mcp::daemon` | `66 passed; 0 failed` |
| `cargo test --features mcp --lib mcp::transport` | `94 passed; 0 failed` |
| `cargo test --features mcp --lib mcp::` | `452 passed; 0 failed` |
| `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` | `23 passed; 0 failed`, goldens unblessed |
| `cargo check --features mcp` | `Finished`, 0 warnings |
| clippy, filtered to `src/mcp/daemon/`, `transport_loop_tests.rs` and `transport.rs` | no line (258 `warning` lines in the whole log) |
| 30 × `mcp::daemon`, default threads | `30 test result: ok. 66 passed; 0 failed`; test time 1.27–1.91 s |
| 30 × `mcp::daemon -- --test-threads=1` | `30 test result: ok. 66 passed; 0 failed`; test time 1.63–4.79 s; load averages 38.08–42.12 |
| Text replay of this plan, compared with `248da0ca` | 109 directives, every anchor and every mutant anchor matched once; all 26 files the plan touches byte-identical, `CLAUDE.md` included |

**Plan text changed with it:** D26 amended (the owner's words and date) and the retry interval recorded as the owner's; the owner question marked answered; SI-8's last clause corrected (Opus F2, sol S1); D3's cost line marked superseded by D32 (sol S2); D32 notes the ready case; the Background case in "Interfaces for the next slices" with an S4 requirement and the reviewer's probe as its test sketch, and the parity claim's reason for not claiming `Background` (Opus F1); two S4 notes on D33 (Opus S3); the shim-side peer-uid check as an S5a design item; the sub-floor result cap recorded as settled (refused, S2b); Claims and Review Focus follow the renamed tests.

**In the brief, and what needed care.** The brief asked for "a waiter whose attempt failed and whose retry already published is served (real tool result)": the existing socket test of that sequence was the one that asserted the `indexing` envelope, so it now asserts a served query, and the envelope checks moved to the building-case socket test, so Task 10's mutants D and E stay killed. The new socket test is in a new file because `connection_tests.rs` would have reached 616 lines. Nothing in the brief was wrong.

**Not run for this fold:** the unfiltered suite, the doc tests, the mutation gate, `cargo check` without features and the owner-audit build (the fold changes no feature-gated or default-build code), anything on Linux.
