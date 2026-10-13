# S3a — Readiness Coordinator and Provider Slot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **This plan's code has not been compiled.** It was written on 2026-10-13 while a memory measurement held the host, so no `cargo` command and no `prism` binary was run: not on the base, not on the plan's code. Every code block, every expected test count and every "this mutant fails that test" line below is reasoning from the source at `main` `c785fa4f`, not an observation. What was done instead: the plan's file directives were applied as text, in order, to copies of the affected files outside the repository, and the result was parsed with `rustfmt` after each task (self-review record, "What was checked without a compiler"). That shows the code blocks are syntactically valid and that every edit applies where the plan says; it shows nothing about types, borrows, lints or behavior. The earlier slices' plans were replayed with a compiler on a scratch branch before hand-off; this one still has to be. The list directly below says what to run and what each command must show. Treat a compile error in a code block as a defect of the plan to fix here, not as something for the implementer to work around.

## Validate before dispatch (nothing below was run)

Replay the plan on a scratch branch off `main` `c785fa4f`, as was done for S1, C1 and S2: perform every "Create / Replace / In … replace / Append" directive in order and run every `bash` block as written (skip `git push` and `gh pr create`).

| # | Command | Must show |
|---|---|---|
| V1 | `cargo test --features mcp --lib mcp:: ` and `cargo test --features mcp --test mcp` on the untouched base | The two base totals. The plan states every later total as "base + N"; write the two numbers into Task 1 Step 1. |
| V2 | Each task's red step | A compile failure (the tests name items that do not exist yet). A red step that compiles is a plan defect. |
| V3 | Each task's green step | Exactly the count the step states: Task 1 `1`, Task 2 `5`, Task 3 `12`, Task 4 `16` for the coordinator module, Task 6 `5`, Task 7 `7`, Task 8 `5`. |
| V4 | `cargo test --features mcp --lib mcp::` after Task 4 and after Task 8 | base + 22, then base + 39; `0 failed`. |
| V5 | `env -u PRISM_MCP_BLESS_GOLDEN cargo test --features mcp --test mcp` after Task 6 and after Task 8 | The base number unchanged, with `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`. This is the standalone byte-for-byte proof for the one edit to the serving path (Task 6). |
| V6 | `cargo check` (default features) and `cargo check --tests --features mcp,detached-owner-audit` | `Finished` for both. The second compiles `owner_transport_tests.rs` against the edited `transport.rs`. |
| V7 | `cargo clippy --all-targets --features mcp -- -W clippy::all`, filtered to the touched files (Task 5 Step 2, Task 9 Step 2) | No line. In particular no `dead_code` warning from `src/mcp/daemon/` in the non-test build; see "Unsure of" item 1. |
| V8 | Every "prove the tests bite" step | The named test fails under the named mutant. Five mutants (Task 3 C, D and E; Task 4 B; Task 7 A) make a call wait where it should return; those tests must fail at the 30 s hang limit, not hang the run. |
| V9 | The 30-run repeat loop of Task 5 Step 4 and Task 9 Step 4: once as written, once with `-- --test-threads=1` appended to the `cargo test` command, and once while another `cargo build` loads the host | One line each time: 30 × `ok`, the same count, `0 failed`. The tests wait on conditions with a 30 s limit and assert no upper bound on any duration, so load must not change a verdict. |
| V10 | The three zero-diff checks (Task 5 Step 3, Task 9 Step 3) | `0` for `transport_tests.rs` + `owner_transport_tests.rs`, `0` for the goldens, and the listed set of edited files, nothing else. |
| V11 | CI on Linux, on the scratch branch pushed as a draft | Green. Nothing here was reasoned about per platform except where a test says so. |

**Unsure of, in the order I would check them:**

1. **Dead-code analysis of the new module.** `src/mcp/daemon/` has no production caller until S4, so the module carries `#![cfg_attr(not(test), allow(dead_code))]`. I believe rustc treats items under that allowance as live roots, so that what they call in `transport.rs`, `framing.rs` and `session.rs` is not reported either. If V7 shows a `dead_code` warning anyway, the fix is local (the allowance on the reported item); it does not change the design.
2. **Closure and coercion inference** in code that could not be compiled: the `serve_loop` handler closures (Task 6), `Option::map(query)` with a higher-ranked `FnOnce(&SessionProvider) -> R` in `DaemonState::read` (Task 3), and trait-object coercions in test helpers (written through `let x: Type = …` on purpose).
3. **Whether `cargo fmt` in the repository lays the code out as standalone `rustfmt --edition 2021` did.** There is no `rustfmt.toml`, so it should. If it does not, an "In … replace" anchor in Task 4, 7 or 8, or a mutant's anchor, may need its whitespace adjusted; the anchors were checked against the standalone layout.
4. **Clippy on the new code.** `clippy::all` was not run; `type` aliases are used where a signature would otherwise trip `type_complexity`.
5. **The base totals.** Not measured at `c785fa4f`.
6. **One existing test with a 100 ms wall-clock assertion** (`src/mcp/transport_tests.rs:774`) is untouched and is not this plan's; it can fail on a loaded host independently of this slice.

## Decisions owed before the pull request is opened

No spec issue blocks **building** S3a. Two change what the slice's row in spec §12 delivers, and a reviewer will measure the PR against that row, so they should be confirmed (by the owner, or by the controller if it falls under the standing authority) before the PR is opened:

- **SI-1.** Spec §9 tests 1 and 14 are described as integration tests under `tests/mcp/` with shims and real sockets in a runtime directory. S3a has no accept loop (S4) and no shim (S5a), and test 14 needs a refresh forced to verify `Diverged`, which exists only as a `#[cfg(test)]` method of the library. This plan runs both as in-crate tests over connected Unix socket pairs.
- **SI-2.** Spec §4.7 puts "a test-only status probe over the same control exchange" in S3a. The control exchange is the preamble, whose first line is read by S4's handshake stage. This plan ships the probe's content (`DaemonState::status()`) and leaves its wire verb to S4.

Both are in "Spec issues found" with their options.

---

**Goal:** Ship, as tested library code with no accept loop and no shim yet, the two things a shared daemon needs before it can serve a second client: the readiness coordinator (one index build, joined by every client that needs it, each waiting no longer than its own budget) and the provider slot the built index is published into, with one admitted connection served through both. The in-process server's behavior does not change.

**Architecture:** A new Unix-only module `src/mcp/daemon/` holds `DaemonState`: a control plane (`Mutex<ControlState>` plus one `Condvar`: the build phase `Idle | Building{attempt, started} | Ready{generation} | Failed{attempt, error}` and a count of waiting calls) and a provider slot (`RwLock<Option<SessionProvider>>`, empty until the build thread publishes into it, which it does before announcing `Ready`). `ensure_ready` decides once per call which attempt the call belongs to and then waits for that attempt only, up to a per-connection, per-attempt deadline held in `ConnectionBudget`. `serve_admitted` serves one connection: S1's `handle_control` for everything that needs no index, `ensure_ready`, then S1's `dispatch_tool_read` under the read lock or `dispatch_tool_refresh` / `dispatch_tool_auto_refresh` under the write lock. The frame loop is extracted from `serve_connection` into `serve_loop` so both servers share it. Time is an injected `Clock`; the build thread is started through an injected `Spawner`.

**Tech Stack:** Rust 2021, `std::sync::{Mutex, Condvar, RwLock}`, `std::os::unix::net::UnixStream`, `serde_json`, `anyhow`; `tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` as revised 2026-10-13 (§3.1, §4.2, §4.5, §5, §9 tests 1 and 14 and the unit list, §10, §12 row S3a). Lane plan: `docs/superpowers/plans/2026-10-12-shared-prism-daemon/README.md` ("Notes for the planners of later slices", S3a and S3b). Built on S2b as merged: `main` `c785fa4f`.

**Ships as two pull requests.** The slice is about 2,780 added lines (740 of code, 2,020 of tests, 20 of docs; counted from this plan's code blocks after `rustfmt`), above what two review rounds converge on. **S3a-1 = Tasks 1–5** (the coordinator and the slot: about 1,700 lines, 600 of code of which about 100 are comments). **S3a-2 = Tasks 6–9** (the shared frame loop and one connection served through the coordinator, with spec tests 1 and 14: about 1,080 lines, 140 of code). A reviewer can approve the coordinator and reject how a connection is composed on it, or the reverse; S3a-2 edits S3a-1's files in two small places, both named in Task 7 and Task 8.

S3a-1 is about 200 lines over the lane's 1,500-line guide. It is left whole because its tests (1,090 lines) are what hold its claims, and this lane's reviews have turned back properties that arrived without the test that holds them. If the controller wants it at the guide, Tasks 1 and 2 (the generation accessor and the budget: about 190 lines, depending on nothing else in the slice) can go first as a third, small pull request, which leaves Tasks 3–5 at about 1,500; nothing in the plan changes but the branch points.

## Global Constraints

- All work is under `--features mcp`. The default build must still compile (`cargo check`).
- **Standalone behavior is unchanged, byte for byte.** The proof has three parts, checked in Task 5 and Task 9: the golden transcripts under `tests/fixtures/mcp/golden/` are unmodified and pass (never set `PRISM_MCP_BLESS_GOLDEN`); `src/mcp/transport_tests.rs` and `src/mcp/owner_transport_tests.rs` have a zero-line diff against the base; and the only edits to serving files are (a) Task 1's additions to `src/mcp/session.rs` (one accessor nothing in the standalone path calls, and one comment) and (b) Task 6's extraction of the frame loop in `src/mcp/transport.rs`, where `serve_connection` passes the unchanged `handle_message_with` to the extracted loop. `src/mcp/lazy.rs`, `framing.rs`, `output.rs`, `owner.rs`, `settings.rs`, `src/mcp/shared/*` and `src/bin/prism-mcp.rs` are not touched.
- The in-process server keeps `LazySessionProvider` and the `SessionRuntime` trait unchanged (spec §4.2). Do not add, remove or rename a `SessionRuntime` method.
- Spec values, copied exactly:
  - Build phases: `Idle | Building{attempt, started} | Ready{generation} | Failed{attempt, error}` (§4.2). Their wire words, already in `shared::preamble::BuildState`: `idle`, `building`, `ready`, `failed` (§4.4).
  - A connection's deadline for attempt *n* is set when its first readiness-requiring call arrives during attempt *n* and is cumulative for that attempt; a warming response is a wait outcome for that connection, not a state transition, and cancels nothing; when attempt *n* ends its outcome is delivered to every connection waiting on it; after `Failed`, the next readiness-requiring call from any connection starts exactly one retry (attempt *n+1*) with fresh per-connection budgets (§4.2).
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
| A thread panics holding the control lock | The control lock is poisoned. Every later `ensure_ready` and `status` returns `Poisoned`; parked calls return `Poisoned` when they wake (each is bounded by its deadline). S3a's own code under that lock is field assignments, saturating arithmetic, one `format!`, `Clock::now` and the `Spawner` call. | Abort. |
| The build thread cannot be started | Not a panic: `std::thread::Builder::spawn` returns an error and the attempt ends `Failed`. | Unchanged. |

Nothing ships between S3a and S3b (there is no daemon binary until S4), so no user meets the left column.

## Decisions this plan makes

Each is something the spec leaves open or states ambiguously. The reason is one line; "cost if wrong" is what it takes to reverse.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D1 | The daemon core is a new private module `src/mcp/daemon/` (`mod daemon;` in `src/mcp/mod.rs`), not part of `src/mcp/shared/`. Its items are `pub(super)`. | `shared/`'s own doc says nothing there takes a lock or starts a thread. S3a's tests are in-crate, so nothing outside `crate::mcp` needs these names yet. | A visibility edit when S4 adds the public entry point. |
| D2 | The module carries `#![cfg_attr(not(test), allow(dead_code))]`, to be removed in S4. | Nothing calls the coordinator in production until the accept loop exists. | One line. |
| D3 | The control plane is one `Mutex<ControlState>` and one `Condvar`, as spec §4.2 words it; calls park on that condition variable. The alternative, a shared cell per attempt that waiters hold, would make "a waiter gets its own attempt's outcome" a matter of types. | It is the spec's shape and S4 needs the build phase and its own counters under one lock for the retirement decision. | The waiter that wakes after a retry began is then a case the code must handle; it does (`last_error`) and Task 4 plays that interleaving through a hook. Changing to per-attempt cells later is confined to `coordinator.rs`. |
| D4 | `ensure_ready` picks its attempt once, then only waits for that attempt. A call that is told `Failed` never starts the retry itself. | Spec: "the next readiness-requiring call … starts exactly one retry". A call that restarted on failure could loop for as long as builds keep failing. | A loop edit; Task 3's failed-attempt test pins it. |
| D5 | "Single retry" (spec §12) is read as: one retry per failed attempt, started by the next call. No cap, no back-off. | It is the in-process server's contract ("the next tool call retries the build"), which spec §5 preserves. | SI-3. A cap or a minimum interval is a few lines in `start_attempt`. |
| D6 | A budget is a `ConnectionBudget`, constructible only from a `ClientSettings`; it holds one deadline keyed by attempt number. | `ClientSettings` already bounds the wait at 600 s (S2b), so the bound is carried by the type. The key by attempt is what makes the budget "per attempt". | None foreseen. |
| D7 | If `now + wait` cannot be represented, the deadline is `now` (no wait). | It cannot panic. No `ClientSettings` can reach it; no portable test can either, and the branch is disclosed as untested. | One expression. |
| D8 | Time is an injected `Clock` (`SystemClock` in production). Tests move a `ManualClock` and call `DaemonState::wake()`. | Spec §9 asks for deterministic constructions; S4's idle-exit test needs an injected clock anyway. | S4 can keep or replace the 20-line trait. |
| D9 | The build thread is started through an injected `Spawner`; a spawn error is a `Failed` attempt. | `std::thread::spawn` panics when the thread cannot be created, and the first tool call of any client reaches that line. Injection makes the error branch testable for real. | Fifteen lines. |
| D10 | A builder panic is a `Failed` attempt with the text `index build panicked` (the in-process server's), delivered through a drop guard on the build thread. | Without it a panic would leave the phase `Building` forever and every call would get warming forever. The abort of spec §3.1 is S3b's. | S3b replaces the guard's body with the abort. |
| D11 | A poisoned lock is the typed error `Poisoned`; the connection whose call met it ends with an error. Nothing is read through a poisoned lock (`into_inner` is never used). | Spec §4.2: "no lock is ever unwrapped past poisoning". Ending the process is S3b's boundary. | S3b turns the error into the abort where it is raised. |
| D12 | An attempt's outcome is recorded only while the phase is still `Building` that attempt. `Ready` is therefore never left by the coordinator. | A stale `Failed` after `Ready` would let the next call rebuild and replace a provider that has since been refreshed. | None foreseen. |
| D13 | The provider's generation is copied into the control plane before exclusive access to the slot ends; the copy function takes the write guard as an argument. | S4 writes `READY` from the control plane and must not wait on the provider to learn the generation. Requiring the guard makes "recorded in refresh order" a matter of types. | The spec says a thread holding the provider may take the control lock "to read state"; this writes one field (SI-4). |
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
| D25 | Two test seams on the build thread, compiled only under `#[cfg(test)]` and called with no lock held: after publication and before the announcement; after an outcome is recorded and before waiters are notified. | They are the two steps where another actor can interleave ("publication before Ready", "a waiter that wakes late"). | Removed with their tests. |

## Spec issues found (not resolved by this plan)

- **SI-1 (§9 tests 1 and 14, §12 row S3a; needs confirming).** The tests are specified with shims, real sockets under a temporary `PRISM_MCP_RUNTIME_DIR`, and `tests/mcp/`. At S3a there is no listener and no shim, and `tests/mcp/` links the library without `cfg(test)`, so it cannot reach `SessionProvider::force_next_verification_for_tests` (`src/mcp/session.rs:203`), the only deterministic way to make a refresh verify `Diverged`. Options: (a) in-crate tests over socket pairs now, and the same two scenarios again through real shims in S5a (this plan, D17); (b) move tests 1 and 14 to S5a and let S3a ship with unit tests only; (c) add a production-reachable test hook (an environment variable that forces `Diverged`, a builder barrier in a file) so that process-level tests can exist from S4 on. (c) will be needed for process-level versions of tests 1, 8 and 14 whenever they are written; it is not needed to build S3a.
- **SI-2 (§4.7, §12 row S3a; needs confirming).** "The integration tests use a test-only status probe over the same control exchange (S3a)". The control exchange is the preamble; S2b's `Hello::decode` accepts only `HELLO`, and the code that reads a connection's first line is S4's bounded handshake stage. A wire probe written now would guess S4's reply fields, and by S2's own rule a new verb is a wire change that bumps `SHARED_PROTOCOL_VERSION` (the lane plan records that question as open until S6). Options: (a) S3a ships the snapshot the probe reports, S4 adds the verb with its own fields (this plan, D15); (b) S3a adds a `STATUS` request and reply codec to `src/mcp/shared/` with only the build fields, and S4 extends it. "Test-only" also needs a meaning for a binary that integration tests start as a child: a hidden verb that is always compiled in, or one behind an environment variable. That is S4's to decide with SI-1(c).
- **SI-3 (§4.2, §12 "single retry").** The text gives each failed attempt exactly one retry, started by the next call, and says nothing of a limit or a pause. A build that fails after thirty seconds of work is then started again by the next call of any of N clients, for as long as they keep calling. That is today's behavior for one client. Options: as written (this plan, D5); a minimum interval between attempts; a cap after which `Failed` is sticky until a refresh or a restart.
- **SI-4 (§4.2, wording).** "A thread holding the provider may briefly take Control to read state." Keeping `Ready{generation}` true after a refresh needs a write to the control plane while the provider is held. The lock order is the permitted one; the word "read" is narrower than what the design needs. This plan writes the one field (D13).
- **SI-5 (§4.6 "Client exits: connection drops; admitted count decrements").** With D23 the decrement can lag the disconnect by up to the client's first-call wait (at most 600 s) when the client leaves while its call is parked on a build. The idle TTL is 900 s, so retirement is delayed, not prevented. Options: accept; have the parked call poll its socket for a hang-up; cap the wait of a parked call independently of the client's setting.
- **SI-6 (§3.1, §4.2 against §5).** Spec §3.1 makes a panic on the build thread daemon-fatal; spec §5 preserves "the warming result and retry contract", under which the in-process server reports a builder panic as a failed build and retries. S3a implements the second and S3b the first. The two readings differ only once S3b lands; recorded so S3b's plan decides it on purpose.
- **SI-7 (§4.2, cost of the read path).** The lane plan notes that the first query on a fresh snapshot builds the call-edge index and writes its cache sidecar. In the daemon that happens under the read lock: other readers wait on that `OnceLock`, not on the provider, but a `refresh_index` requested meanwhile waits for the disk write. Not pursued.

## Review Focus

Five ways this code is most likely to be wrong in use that a straightforward test list would not exercise. Each has a test in the task that owns the code.

1. **A call that is still parked when the retry starts.** Attempt 1 fails; before its waiter runs again, another client's call starts attempt 2. The waiter must hear of attempt 1's failure, must not start attempt 3, and must not silently join attempt 2 with the budget it had for attempt 1. → Task 4, `a_waiter_that_wakes_after_the_retry_began_still_hears_of_its_own_attempt_s_failure` (plays the other client between the recording and the notification).
2. **A client that disconnects while its call is parked.** The build must go on and be kept, no waiter may remain counted, and the next client must be served from that build, not a second one. → Task 7, `a_client_that_leaves_while_its_call_waits_leaves_no_waiter_and_the_build_is_kept`, `a_departed_client_s_call_is_parked_no_longer_than_its_budget`.
3. **A build that cannot start, or dies.** No thread can be created; or the builder panics. Either way every parked call gets an answer and the next call can retry. → Task 3, `a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries`, `a_builder_panic_is_a_failed_attempt_every_waiter_hears_of`.
4. **A lock poisoned while calls are parked.** They must return an error when they wake, not hang and not panic; later calls and `status` the same. → Task 3, `a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang`; Task 4, `a_poisoned_provider_slot_fails_the_attempt_and_is_an_error_for_every_reader`; Task 8, `a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on`.
5. **A whole session, not one call.** Garbage frames, calls before `initialize`, an unknown tool, a bad argument, an edit that makes the index stale, a refresh, a re-query: a daemon connection must answer every one with the bytes the in-process server writes, under a warn-only and an automatic policy. → Task 8, `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server`.

## Claims and the tests that hold them

Every "never", "always", "exactly" and "only" this plan's code or comments state, with the test that fails when it breaks and one wrong implementation that test rules out. "Predicted" because nothing was run (V8).

| Claim | Test | A mutant it kills (predicted) |
|---|---|---|
| Nothing is built before the first readiness-requiring call. | T3 `nothing_is_built_until_the_first_readiness_requiring_call`; T7 `two_clients_share_one_build` (status is idle after two handshakes) | start the build in `from_parts` for `Lazy` |
| A connection's deadline for an attempt is set once and kept. | T2 `the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it`; T3 `later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget`; T7 `a_connection_s_budget_spans_its_calls_within_one_attempt` | a fresh deadline per call; a `ConnectionBudget` made per message |
| Each connection has its own deadline; one client's warming changes nothing for another. | T2 `each_connection_has_its_own_deadline_for_the_same_attempt`; T3 `the_spec_s_worked_example_two_budgets_on_one_attempt` | one deadline per attempt shared by all waiters |
| A new attempt gives every connection a fresh budget. | T2 `a_new_attempt_gets_a_fresh_full_budget`; T3 `a_retry_gives_a_connection_a_fresh_full_budget` | keep the first deadline for the connection's life |
| An attempt's outcome reaches every call waiting on it. | T3 `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself`; T4 the late-waiter test | `notify_one` instead of `notify_all` |
| A call told `Failed` does not retry by itself. | same T3 test (the builder is called once) | restart from the wait loop when the attempt failed |
| After `Failed`, exactly one retry starts however many calls arrive. | T3 `the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together` | start an attempt from `Building` too |
| The provider is in the slot before `Ready` is announced. | T4 `the_index_is_in_the_slot_before_ready_is_announced` | announce first, publish second |
| An attempt's outcome is recorded once; `Ready` is never left. | T3 `an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left` | record without checking the phase |
| A call never waits past its deadline, also when nobody wakes it. | T3 `on_the_real_clock_a_call_returns_warming_when_its_budget_ends_unprompted` | wait without a timeout |
| No waiter stays counted after its call returns. | T3 (every test that ends with `waiting: 0`); T7 the two departed-client tests | drop the decrement |
| A build thread that cannot start is a failed attempt, not a panic. | T3 `a_build_thread_that_cannot_be_started_…` | ignore the spawner's error |
| A poisoned lock is an error to the caller, never a panic, never a hang. | Review Focus 4's three tests | `unwrap` the lock; `into_inner` |
| Exclusive access ends with the provider's generation in the control plane. | T4 `exclusive_access_records_the_provider_s_generation_in_the_control_plane`; T8 the two sticky-evidence tests | drop the copy |
| The frame loop answers a frame that is not JSON itself and keeps the connection. | T6 `a_frame_that_is_not_a_json_value_is_answered_by_the_loop_and_never_reaches_the_handler` | hand malformed frames to the handler; end the loop on them |
| One lifecycle state lasts the whole connection. | T6 `one_lifecycle_state_lasts_the_whole_connection_and_starts_before_initialize` | a new `Lifecycle` per frame |
| A handler error ends the loop. | T6 `a_handler_error_ends_the_loop_and_nothing_after_it_is_read_or_answered` | log and continue |
| A connection's output settings are its own. | T7 `two_clients_share_one_build` (one client gets `structuredContent`, the other does not) | `ConnectionSettings::from_env()` in the daemon |
| The handshake and listing need neither the build nor the provider. | T7 `the_handshake_listing_and_ping_are_answered_while_the_build_is_blocked`; T8 the poisoned-slot test | call `ensure_ready` before `handle_control` |
| Sticky stale evidence outranks a clean stamp probe for every client. | T8 `sticky_stale_evidence_reaches_another_client_although_the_stamp_probe_is_clean`, `under_an_automatic_policy_sticky_stale_evidence_makes_the_next_client_s_query_refresh_again` | drop the `known_stale_after_refresh` precedence in `effective_stale_report` |
| `Ready` with nothing published is an error, not a panic. | T8 `ready_with_nothing_published_ends_the_connection_with_an_error_not_a_panic` | `expect` in `published` |

Stated and **not** held by a test: D7's overflow branch (no `ClientSettings` reaches it); that no S3a code panics while holding the control lock (by inspection of the five things it does there); that a response is written with no lock held (by construction: `run_tool_call` returns an owned `Value`, so no guard can outlive it; the blocked-writer test is S3b's, spec §9 test 7).

## File Structure

| File | Responsibility | Change | PR |
|---|---|---|---|
| `src/mcp/session.rs` | adds `SessionProvider::generation()`; corrects the comment above `build_state` | Modify (Task 1) | S3a-1 |
| `src/mcp/session_stage_tests.rs` | one test for the accessor | Modify (Task 1): append | S3a-1 |
| `src/mcp/mod.rs` | Module list | Modify (Task 2): `#[cfg(unix)] mod daemon;` | S3a-1 |
| `src/mcp/daemon/mod.rs` | Module list of the daemon core; compile-time `Send + Sync` assertions | Create (Task 2), replace (Task 3), extend (Task 7) | both |
| `src/mcp/daemon/budget.rs` | `ConnectionBudget` | Create (Task 2) | S3a-1 |
| `src/mcp/daemon/budget_tests.rs` | Its tests | Create (Task 2) | S3a-1 |
| `src/mcp/daemon/clock.rs` | `Clock`, `SystemClock` | Create (Task 3) | S3a-1 |
| `src/mcp/daemon/coordinator.rs` | `DaemonState`, `ControlState`, `BuildPhase`, `BuildStatus`, `Poisoned`, `Parts`, `Spawner`, test seams | Create (Task 3), extend (Task 4: `write`; Task 8: one test-only method) | both |
| `src/mcp/daemon/coordinator_tests.rs` | States and budgets | Create (Task 3) | S3a-1 |
| `src/mcp/daemon/coordinator_failure_tests.rs` | Builds that cannot start or die; poisoning; write-once outcome | Create (Task 3) | S3a-1 |
| `src/mcp/daemon/coordinator_order_tests.rs` | Publication before `Ready`; the late waiter; the generation copy | Create (Task 4) | S3a-1 |
| `src/mcp/daemon/test_support.rs` | Manual clock, gates, scripted builder, test connections | Create (Task 2), replace (Task 3) | S3a-1 |
| `src/mcp/transport.rs` | adds `serve_loop`; `serve_connection` calls it | Modify (Task 6) | S3a-2 |
| `src/mcp/transport_loop_tests.rs` | Tests for `serve_loop` | Create (Task 6) | S3a-2 |
| `src/mcp/daemon/connection.rs` | `serve_admitted`, the tool-call step under the two locks | Create (Task 7) | S3a-2 |
| `src/mcp/daemon/wire_support.rs` | A raw MCP client on a socket pair, for tests | Create (Task 7), extend (Task 8) | S3a-2 |
| `src/mcp/daemon/connection_tests.rs` | Spec test 1; warming, failure and departure over a socket | Create (Task 7) | S3a-2 |
| `src/mcp/daemon/connection_parity_tests.rs` | Byte parity with the in-process server; spec test 14; the two error endings | Create (Task 8) | S3a-2 |
| `CLAUDE.md` | MCP Adapter notes | Modify (Tasks 5, 9) | both |
| `src/mcp/transport_tests.rs`, `src/mcp/owner_transport_tests.rs`, `tests/fixtures/mcp/golden/*`, `src/mcp/lazy.rs`, `src/mcp/framing.rs`, `src/mcp/shared/*` | The standalone contract and S1/S2's code | **Must not change** | |

How a step names a file edit, so there is no doubt what to type:

- "Create `path`:" is followed by the file's complete content.
- "Replace the whole of `path` with:" is followed by the file's complete content.
- "In `path`, replace this:" is followed by text that occurs exactly once in the file, then "with this:" and its replacement.
- "Append to `path`:" is followed by text to add at the end of the file.

Every test step that expects a failure expects a **compile** failure: the tests are written first against items that do not exist yet. Task 8 is the exception and says so.

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

Write down the two `passed` numbers. They were not measured when this plan was written (see the top of the plan). Every later expectation is "base + N" against these two.

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

Expected for A: `the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it ... FAILED` (and two more). Expected for B: `a_new_attempt_gets_a_fresh_full_budget ... FAILED`, `4 passed; 1 failed`.

---

### Task 3: The readiness coordinator

`DaemonState`: where the one index build is, who waits for it and for how long, and the slot the built index is published into (spec §4.2).

**Files:**
- Create: `src/mcp/daemon/clock.rs`, `src/mcp/daemon/coordinator.rs`, `src/mcp/daemon/coordinator_tests.rs`, `src/mcp/daemon/coordinator_failure_tests.rs`
- Replace: `src/mcp/daemon/mod.rs`, `src/mcp/daemon/test_support.rs`

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
  - `enum BuildPhase { Idle, Building { attempt: u64, started: Instant }, Ready { generation: u64 }, Failed { attempt: u64, error: String } }` with `fn state(&self) -> BuildState`, `fn generation(&self) -> u64`
  - `struct ControlState { build: BuildPhase, attempts: u64, waiting: u64, last_error: Option<String> }`
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

    /// Wait until the gate is open. Gives up after four times [`HANG`], so a test that fails
    /// before it opens its gate does not leave a thread parked for the rest of the run.
    pub(super) fn pass(&self) {
        let open = self.open.lock().expect("gate");
        let _waited = self
            .changed
            .wait_timeout_while(open, HANG * 4, |open| !*open)
            .expect("gate");
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
        gate.pass();
        SessionProvider::bootstrap(&cfg)
    })
}

/// Fail with `message` once `gate` is open.
pub(super) fn fail_after(gate: &Arc<Gate>, message: &'static str) -> BuildStep {
    let gate = Arc::clone(gate);
    Box::new(move || -> anyhow::Result<SessionProvider> {
        gate.pass();
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
    assert_eq!(a.outcome(), Outcome::Ready, "A's next call is served at once");
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
        vec![fail_now("first attempt failed"), build_after(&retry_gate, &cfg)],
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
````

- [ ] **Step 3: Write the tests for builds that cannot start or die**

Create `src/mcp/daemon/coordinator_failure_tests.rs`:

````rust
//! Tests for the ways an attempt can end without a build: the thread cannot be started, the
//! builder panics, a lock is poisoned. In each, every parked call must get an answer.
use super::super::test_support::{
    build_after, fixture, panic_now, parts_with, scripted, state_with, status, wait_until, Conn,
    Gate, ManualClock, Outcome,
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
    state.settle(9, BuildPhase::Ready { generation: 0 });
    assert_eq!(status(&state).state, BuildState::Building);
    assert_eq!(state.read(|_| ()), Ok(None));

    gate.open();
    assert_eq!(conn.outcome(), Outcome::Ready);

    // So is a second outcome for the attempt that already ended.
    state.settle(
        1,
        BuildPhase::Failed {
            attempt: 1,
            error: "too late".to_string(),
        },
    );
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
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock};
use std::time::Instant;

/// What a call is told when the builder panicked. The in-process server uses the same text.
const BUILD_PANICKED: &str = "index build panicked";
const SLOT_POISONED: &str = "the index was built and could not be published: the provider lock is poisoned";
/// Used only if an attempt is found to be over with no failure recorded, which the two
/// functions that change the phase do not allow.
const BUILD_FAILED: &str = "index build failed";

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

/// Where the index build is (spec §4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum BuildPhase {
    /// No build has been asked for.
    Idle,
    /// Attempt `attempt` is running since `started`.
    Building { attempt: u64, started: Instant },
    /// A provider is in the slot. `generation` is the provider's, as of the last completed
    /// exclusive access to it.
    Ready { generation: u64 },
    /// Attempt `attempt` ended without a provider. The next readiness call starts the retry.
    Failed { attempt: u64, error: String },
}

impl BuildPhase {
    /// The word `READY` and the status probe use for this phase.
    pub(super) fn state(&self) -> BuildState {
        match self {
            Self::Idle => BuildState::Idle,
            Self::Building { .. } => BuildState::Building,
            Self::Ready { .. } => BuildState::Ready,
            Self::Failed { .. } => BuildState::Failed,
        }
    }

    /// The generation of the published index; 0 while there is none.
    pub(super) fn generation(&self) -> u64 {
        match self {
            Self::Ready { generation } => *generation,
            Self::Idle | Self::Building { .. } | Self::Failed { .. } => 0,
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
    /// The error of the most recent failed attempt. It outlives the `Failed` phase, for a call
    /// that was waiting on that attempt and runs again only after the retry has begun.
    pub(super) last_error: Option<String>,
}

impl ControlState {
    /// What a call that belongs to `attempt` is told now; `None` while that attempt is still
    /// building.
    fn outcome_for(&self, attempt: u64) -> Option<Readiness> {
        match &self.build {
            BuildPhase::Ready { .. } => Some(Readiness::Ready),
            BuildPhase::Building {
                attempt: building, ..
            } if *building == attempt => None,
            BuildPhase::Failed { error, .. } => Some(Readiness::Failed {
                error: error.clone(),
            }),
            // A later attempt is building, so the one this call belongs to failed. The call is
            // told so; it does not join the retry with a budget it was given for another attempt.
            BuildPhase::Building { .. } | BuildPhase::Idle => {
                let error = self.last_error.clone();
                Some(Readiness::Failed {
                    error: error.unwrap_or_else(|| BUILD_FAILED.to_string()),
                })
            }
        }
    }
}

/// A snapshot of the control plane: what the status probe reports about the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildStatus {
    pub(super) state: BuildState,
    /// The number of the latest attempt; 0 before the first.
    pub(super) attempt: u64,
    /// The generation of the published index; 0 while there is none.
    pub(super) generation: u64,
    /// Calls parked on the build at the moment of the snapshot.
    pub(super) waiting: u64,
}

/// The work of one build thread.
pub(super) type BuildJob = Box<dyn FnOnce() + Send>;

/// Starts a thread that runs a [`BuildJob`]. It is called with the control lock held, so it
/// must not run the job on the calling thread.
pub(super) type Spawner = Box<dyn Fn(BuildJob) -> std::io::Result<()> + Send + Sync>;

fn spawn_build_thread(job: BuildJob) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("prism-mcpd-build".to_string())
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
                last_error: None,
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
    /// new one if none has been started or the last one failed. It then waits for that attempt
    /// only and returns
    ///
    /// - `Ready` once the index is published,
    /// - `Failed` if that attempt ended without an index (the call does not start the retry;
    ///   the next call does),
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
            BuildPhase::Ready { .. } => return Ok(Readiness::Ready),
            BuildPhase::Building { attempt, started } => Some((*attempt, *started)),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
        };
        let (attempt, started) = building.unwrap_or_else(|| self.start_attempt(&mut control));
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            if let Some(outcome) = control.outcome_for(attempt) {
                return Ok(outcome);
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
            generation: control.build.generation(),
            waiting: control.waiting,
        })
    }

    /// Run `query` on the published provider with shared access; `None` when nothing is
    /// published. `query` gets the whole provider, so it sees what the in-process server's
    /// read path sees, the stale evidence a refresh recorded included.
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
        self.control.lock().map_err(|_| Poisoned { lock: "control" })
    }

    /// Start the next attempt and return its number and start time. Called with the control
    /// lock held, from `Idle` or `Failed` only; that is what makes it the one place a build
    /// thread comes from.
    fn start_attempt(self: &Arc<Self>, control: &mut ControlState) -> (u64, Instant) {
        let attempt = control.attempts.saturating_add(1);
        let started = self.clock.now();
        control.attempts = attempt;
        control.build = BuildPhase::Building { attempt, started };

        let state = Arc::clone(self);
        let job: BuildJob = Box::new(move || state.run_build(attempt));
        if let Err(error) = (self.spawner)(job) {
            // No thread, so nothing will ever end this attempt: end it here. Nobody can have
            // joined it yet, because the control lock has been held since it began.
            let error = format!("could not start the index build thread: {error}");
            control.last_error = Some(error.clone());
            control.build = BuildPhase::Failed { attempt, error };
        }
        (attempt, started)
    }

    /// The build thread: run the builder with no lock held, then publish or record the failure.
    fn run_build(self: Arc<Self>, attempt: u64) {
        let fail_on_panic = PanicGuard {
            state: &*self,
            attempt,
        };
        let built = (self.builder)();
        match built {
            Ok(provider) => self.publish(attempt, provider),
            Err(error) => self.settle(
                attempt,
                BuildPhase::Failed {
                    attempt,
                    error: error.to_string(),
                },
            ),
        }
        drop(fail_on_panic);
    }

    /// Put the provider in the slot, then announce `Ready`. In that order: a call that is told
    /// `Ready` goes straight to the slot.
    fn publish(&self, attempt: u64, provider: SessionProvider) {
        let generation = provider.generation();
        let published = match self.provider.write() {
            Ok(mut slot) => {
                *slot = Some(provider);
                true
            }
            Err(_) => false,
        };
        if !published {
            self.settle(
                attempt,
                BuildPhase::Failed {
                    attempt,
                    error: SLOT_POISONED.to_string(),
                },
            );
            return;
        }
        #[cfg(test)]
        if let Some(hook) = &self.hooks.published {
            hook(attempt);
        }
        self.settle(attempt, BuildPhase::Ready { generation });
    }

    /// Record how `attempt` ended and tell every parked call. The outcome is recorded only if
    /// the phase is still `Building` that attempt, so an attempt ends once and `Ready` is not
    /// left. A poisoned control lock is left alone; the parked calls find it when they wake.
    fn settle(&self, attempt: u64, outcome: BuildPhase) {
        if let Ok(mut control) = self.control.lock() {
            let still_building = matches!(
                &control.build,
                BuildPhase::Building { attempt: building, .. } if *building == attempt
            );
            if still_building {
                if let BuildPhase::Failed { error, .. } = &outcome {
                    control.last_error = Some(error.clone());
                }
                control.build = outcome;
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
            self.state.settle(
                self.attempt,
                BuildPhase::Failed {
                    attempt: self.attempt,
                    error: BUILD_PANICKED.to_string(),
                },
            );
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

Expected: `test result: ok. 12 passed; 0 failed` (eight in `coordinator::tests`, four in `coordinator::failure_tests`), then `17 passed; 0 failed` for the module. The run takes a little over a second of real time because of the one real-clock test. Two tests print a panic message from a helper thread (`injected builder panic`, `poisoning the control lock for a test`); that is their subject. The third command prints nothing: the non-test build has no warning that names the new module or the two files edited so far.

- [ ] **Step 8: Commit**

```bash
git add src/mcp/daemon
git commit -m "feat(mcp): readiness coordinator and provider slot for the shared daemon"
```

- [ ] **Step 9: Prove the tests bite (after the commit)**

Five mutants in `src/mcp/daemon/coordinator.rs`, one at a time. After each, run the command below and restore the file. Mutants C, D and E make a call wait where it should fail, so the named test fails at the 30 s hang limit; that is the expected way for them to fail.

Mutant A (an attempt per caller). Replace this:

````rust
            BuildPhase::Building { attempt, started } => Some((*attempt, *started)),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
````

with this:

````rust
            BuildPhase::Building { .. } | BuildPhase::Idle | BuildPhase::Failed { .. } => None,
````

Mutant B (an outcome recorded without looking at the phase). Replace this:

````rust
            if still_building {
````

with this:

````rust
            if still_building || attempt > 0 {
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

```bash
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t3-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t3-bite.log
git checkout -- src/mcp/daemon/coordinator.rs
```

Expected (predicted, not observed):

| Mutant | Fails |
|---|---|
| A | `the_spec_s_worked_example_two_budgets_on_one_attempt` (the status shows attempt 2 once B has called), `the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together`, `a_warm_daemon_starts_attempt_one_at_construction_and_eager_is_refused` |
| B | `an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left` |
| C | `on_the_real_clock_a_call_returns_warming_when_its_budget_ends_unprompted` (after 30 s) |
| D | `a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries` (after 30 s) |
| E | `a_builder_panic_is_a_failed_attempt_every_waiter_hears_of` (after 30 s) |

---

### Task 4: Exclusive access, and the two orderings another actor can break

Three things: `DaemonState::write`, the only way to the provider's `&mut`, which leaves the provider's generation in the control plane; a test that stands between publication and the announcement of `Ready`; and a test that plays a second client between a failed attempt and the notification of its waiter.

**Files:**
- Modify: `src/mcp/daemon/coordinator.rs`
- Create: `src/mcp/daemon/coordinator_order_tests.rs`

**Interfaces:**
- Consumes: Task 3's `DaemonState`, `Parts`, `TestHooks { published, settled }`, `AttemptHook`, `BuildPhase`, `Poisoned`; `pub fn SessionProvider::refresh(&mut self) -> anyhow::Result<RefreshSummary>` (`src/mcp/session.rs:212`) and `RefreshSummary.generation: u64` (`src/mcp/session.rs:75`).
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
            resume.pass();
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
    reached.pass();

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
    recorded.pass();
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
    assert_eq!(
        status(&state).attempt,
        2,
        "and it started no third attempt"
    );

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
````

- [ ] **Step 2: Run them and see them fail to compile**

```bash
cargo test --features mcp --lib mcp::daemon::coordinator::order_tests > /tmp/s3a-t4-red.log 2>&1; grep -E '^error' /tmp/s3a-t4-red.log | sort -u
```

Expected: `error[E0599]: no method named `write` found` (the hook fields the first two tests use exist since Task 3; those two tests are shown to bite in Step 6).

- [ ] **Step 3: Implement**

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock};
````

with this:

````rust
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
````

In `src/mcp/daemon/coordinator.rs`, replace this:

````rust
    /// Wake every parked call so that it looks at the clock again.
````

with this:

````rust
    /// Run `change` on the published provider with exclusive access; `None` when nothing is
    /// published. This is the only way to the provider's `&mut`. Before it lets go of the slot
    /// it copies the provider's generation into the control plane, so `status` reports the
    /// generation of the index that is served once the change is over.
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

    /// Copy the provider's generation into the control plane. The write guard is an argument
    /// so that this cannot be called after the slot has been let go: two refreshes are then
    /// recorded in the order they happened. This is the permitted lock order (slot, then
    /// control).
    fn record_generation(
        &self,
        _held: &RwLockWriteGuard<'_, Option<SessionProvider>>,
        generation: u64,
    ) -> Result<(), Poisoned> {
        let mut control = self.lock_control()?;
        if let BuildPhase::Ready {
            generation: recorded,
        } = &mut control.build
        {
            *recorded = generation;
        }
        Ok(())
    }

    /// Wake every parked call so that it looks at the clock again.
````

- [ ] **Step 4: Run the tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t4-green.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t4-green.log
cargo test --features mcp --lib mcp::daemon > /tmp/s3a-t4-module.log 2>&1; grep -E '^test result|FAILED|^error' /tmp/s3a-t4-module.log
```

Expected: `test result: ok. 16 passed; 0 failed`, then `21 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/mcp/daemon/coordinator.rs src/mcp/daemon/coordinator_order_tests.rs
git commit -m "feat(mcp): exclusive provider access that keeps the control plane's generation current"
```

- [ ] **Step 6: Prove the tests bite (after the commit)**

Three mutants in `src/mcp/daemon/coordinator.rs`, one at a time. After each, run the command and restore the file.

Mutant A (announce, then publish). Replace this:

````rust
        let published = match self.provider.write() {
````

with this:

````rust
        self.settle(attempt, BuildPhase::Ready { generation });
        let published = match self.provider.write() {
````

Mutant B (a late waiter joins whatever is building). Replace this:

````rust
            BuildPhase::Building { .. } | BuildPhase::Idle => {
````

with this:

````rust
            BuildPhase::Building { .. } | BuildPhase::Idle if attempt > 0 => None,
            BuildPhase::Building { .. } | BuildPhase::Idle => {
````

Mutant C (the generation is not copied). Replace this:

````rust
            *recorded = generation;
````

with this:

````rust
            let _ = (recorded, generation);
````

```bash
cargo test --features mcp --lib mcp::daemon::coordinator > /tmp/s3a-t4-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t4-bite.log
git checkout -- src/mcp/daemon/coordinator.rs
```

Expected (predicted, not observed): A fails `the_index_is_in_the_slot_before_ready_is_announced` (the status is already `Ready` at the hook). B fails `a_waiter_that_wakes_after_the_retry_began_still_hears_of_its_own_attempt_s_failure` at the 30 s hang limit (the early call is waiting on the retry). C fails `exclusive_access_records_the_provider_s_generation_in_the_control_plane`.

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
attempt count, the parked calls) and a provider slot (`RwLock<Option<SessionProvider>>`).
`ensure_ready` picks its build attempt once and waits for that attempt only, up to the
connection's `ConnectionBudget` (`budget.rs`: one deadline per connection per attempt). A failed
attempt is retried by the next call, once however many calls arrive, and the build thread puts
the provider in the slot before it announces `Ready`. Lock order: code holding the control lock
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

Expected: `cargo fmt --check` prints nothing; the lib line shows base + 22 passed (1 + 5 + 12 + 4), `0 failed`; the `mcp` target shows the base number unchanged, which includes the golden transcripts; both `cargo check` runs end with `Finished`; the last command prints nothing.

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

Expected: three zeros (the test corpus, the goldens, and every serving file but `session.rs` are untouched); the fourth command prints exactly `src/mcp/mod.rs`, `src/mcp/session.rs` and `src/mcp/session_stage_tests.rs`; the fifth prints `3` (the three comment lines Task 1 replaced; no line of code was removed from `session.rs`); the largest new file is under 600 lines.

- [ ] **Step 4: Run the remaining pre-PR checks**

These are the lane plan's "verification before a slice is called done".

```bash
cargo test --features mcp > /tmp/s3a1-full.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/s3a1-full.log
cargo test --doc --features mcp > /tmp/s3a1-doc.log 2>&1; grep -E '^test result' /tmp/s3a1-doc.log
for i in $(seq 1 30); do cargo test --features mcp --lib mcp::daemon > /tmp/s3a1-repeat-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3a1-repeat-$i.log; done | sort | uniq -c
python3 scripts/mutgate/mutgate.py > /tmp/s3a1-mutgate.log 2>&1; tail -5 /tmp/s3a1-mutgate.log
```

Expected: every `test result` line shows `0 failed` (sum the `passed` counts and report the total next to the Task 1 base numbers); the doc tests pass (this slice adds none); the repeat prints one line, `30 test result: ok. 21 passed; 0 failed`; the mutation gate exits 0. Report the gate as: "full gate green; it builds without `--features mcp`, so it selects no code from this slice." A failure outside this slice's files is reported, not fixed or re-baselined here.

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

# Part two: S3a-2, one connection served through the coordinator (Tasks 6–9)

Branch S3a-2 from `main` after S3a-1 has merged. If it is developed before that, branch from S3a-1's head and rebase when S3a-1 merges; Tasks 7 and 8 edit `src/mcp/daemon/mod.rs` and `src/mcp/daemon/coordinator.rs`, which S3a-1 creates.

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

Expected (predicted, not observed): A fails `one_lifecycle_state_lasts_the_whole_connection_and_starts_before_initialize` (and warns that the outer `state` is unused); B fails `a_handler_error_ends_the_loop_and_nothing_after_it_is_read_or_answered`; C fails `a_frame_that_is_not_a_json_value_is_answered_by_the_loop_and_never_reaches_the_handler`. Mutant A would also fail the golden transcripts; that run is not needed here.

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
use super::super::coordinator::{BuildStatus, DaemonState};
use super::super::test_support::{
    advance, build_after, client_settings, client_settings_with, fail_now, fixture, scripted,
    state_with, status, wait_until, Gate, ManualClock,
};
use super::super::wire_support::{
    close, connect, ended, CALLEES, INIT, INITED, LIST, NO_SUCH_TOOL, PING,
};
use crate::mcp::output::StructuredContentMode;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

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
    assert_eq!(a.recv()["result"]["structuredContent"]["query"], RUN_CALLEES);
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
    assert_eq!(built(&builds), 1, "the warming answer started no second build");
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
    assert_eq!(b.recv()["result"]["structuredContent"]["query"], RUN_CALLEES);
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

Expected: `test result: ok. 7 passed; 0 failed`, then `28 passed; 0 failed` for the module (21 + 7). The third command prints nothing.

- [ ] **Step 6: Commit**

```bash
git add src/mcp/daemon
git commit -m "feat(mcp): serve one admitted connection through the daemon's coordinator"
```

- [ ] **Step 7: Prove the tests bite (after the commit)**

Four mutants in `src/mcp/daemon/connection.rs`, one at a time. After each, run the command and restore the file.

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

```bash
cargo test --features mcp --lib mcp::daemon::connection > /tmp/s3a-t7-bite.log 2>&1; grep -E 'FAILED$|^test result' /tmp/s3a-t7-bite.log
git checkout -- src/mcp/daemon/connection.rs
```

Expected (predicted, not observed): A fails `a_connection_s_budget_spans_its_calls_within_one_attempt` at the 30 s hang limit. B fails `two_clients_share_one_build` (one of the two clients gets the other's shape, whichever the environment selects). C fails `the_handshake_listing_and_ping_are_answered_while_the_build_is_blocked`. D fails `a_client_with_no_wait_is_told_the_index_is_warming_and_its_retry_is_served` and `a_failed_build_is_reported_as_the_in_process_server_reports_it_and_the_next_call_retries` (the connection ends where a status result was due).

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
- Produces: tests only; `#[cfg(test)] pub(super) fn unpublish_for_tests(&self)` on `DaemonState`; `Client::expect_closed`; the constants `REFRESH` and `CALLERS`.

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

Expected: unresolved imports `CALLERS` and `REFRESH`, and no method named `unpublish_for_tests` or `expect_closed`.

- [ ] **Step 3: Add the two support items**

Append to `src/mcp/daemon/wire_support.rs`:

````rust

pub(super) const REFRESH: &str = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#;
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

Expected: `test result: ok. 5 passed; 0 failed`, then `33 passed; 0 failed` for the module (28 + 5). If `a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server` fails, the message names the policy and the first response that differs; that is a defect in Task 7's composition, not in the test. Do not relax the comparison.

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

Expected (predicted, not observed): A fails both sticky-evidence tests (it is a mutant of S1's code; it shows the daemon's read path consumes that precedence and nothing else supplies it). B fails `ready_with_nothing_published_ends_the_connection_with_an_error_not_a_panic` (the worker panics). C fails the byte-parity test at response 4, `initialize`. D fails both sticky-evidence tests and the byte-parity test on the generation in `status`.

---

### Task 9: S3a-2 proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md` (MCP Adapter section)

**Interfaces:**
- Consumes: Tasks 6–8.
- Produces: the S3a-2 pull request.

- [ ] **Step 1: Update the architecture notes**

In `CLAUDE.md`, replace this:

````markdown
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
````

with this:

````markdown
`Clock` (`clock.rs`), so tests move a manual clock and never sleep to order events.
`connection.rs` serves one admitted connection (`serve_admitted`): `transport::serve_loop`, the
frame loop the in-process server also runs, then `handle_control` with no lock, then for a tool
call `ensure_ready` followed by `dispatch_tool_read` under the slot's read lock, or
`dispatch_tool_refresh` / `dispatch_tool_auto_refresh` under its write lock. A query sees the
whole provider, so stale evidence a refresh recorded outranks a clean stamp probe for every
client. Not there yet (the next slice): the re-check of an automatic refresh after exclusive
access is acquired, and panic boundaries.
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

Expected: `cargo fmt --check` prints nothing; the lib line shows the Task 1 base + 39 passed (22 from S3a-1, then 5 + 7 + 5), `0 failed`; the `mcp` target shows the base number unchanged, including `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor ... ok`; both `cargo check` runs end with `Finished`. The first clippy filter prints nothing (no warning in a file this slice adds). The second counts the lines that name `transport.rs`, which this slice edits and which may already have warnings on the base: run the same two commands on the merge base in a second worktree and report both counts; they must be equal.

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

Expected, with S3a-1 already on `main`: three zeros; the fourth command prints exactly `src/mcp/transport.rs` and `src/mcp/transport_loop_tests.rs`; the fifth prints `0` (no line that produces a protocol error was added or removed in the serving file); the largest new file is under 600 lines.

- [ ] **Step 4: Run the remaining pre-PR checks**

```bash
cargo test --features mcp > /tmp/s3a2-full.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/s3a2-full.log
cargo test --doc --features mcp > /tmp/s3a2-doc.log 2>&1; grep -E '^test result' /tmp/s3a2-doc.log
for i in $(seq 1 30); do cargo test --features mcp --lib mcp::daemon > /tmp/s3a2-repeat-$i.log 2>&1; grep -oE '^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed' /tmp/s3a2-repeat-$i.log; done | sort | uniq -c
python3 scripts/mutgate/mutgate.py > /tmp/s3a2-mutgate.log 2>&1; tail -5 /tmp/s3a2-mutgate.log
```

Expected: every `test result` line shows `0 failed`; the doc tests pass; the repeat prints one line, `30 test result: ok. 33 passed; 0 failed`; the mutation gate exits 0, reported with the same sentence as in Task 5. A failure outside this slice's files is reported, not fixed or re-baselined here.

- [ ] **Step 5: Commit and open the PR**

Write the PR body to `/tmp/s3a2-pr-body.md`. It states, in this order:

1. **Goal:** the lane goal, and that S3a-2 serves one admitted connection through S3a-1's coordinator; still no accept loop, so still no production caller.
2. **Invariants:** the Step 3 output; that the golden transcripts pass unblessed after the one edit to the serving path; and Task 6 Step 4's diff of `transport.rs`.
3. **Decisions:** D14, D17, D18, D21, D22, D23 and SI-1, SI-2, SI-5, verbatim or by link.
4. **What S3b inherits:** the three things `connection.rs`'s module comment lists, and the "Panics in S3a" table.
5. **Totals:** the Step 2 and Step 4 numbers next to the Task 1 base numbers, and the mutant results of Tasks 6, 7 and 8 as observed.
6. **Not verified:** a non-Unix build; the mutation gate does not cover MCP code; the blocked-writer case (spec §9 test 7, S3b); the scenarios through a real shim and a real daemon process (S4, S5a).
7. **Tier-A harness:** not run; this slice touches none of `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`.

Then:

```bash
git add CLAUDE.md
git commit -m "docs(mcp): a daemon connection and the shared frame loop"
git push -u origin HEAD
gh pr create --draft --title "feat(mcp): one connection served through the readiness coordinator (shared daemon S3a-2)" --body-file /tmp/s3a2-pr-body.md
```

Review brief and cap as in Task 5.

---

## Interfaces for the next slices

Everything below is in `crate::mcp::daemon` and `pub(super)` unless a path says otherwise. Field and method names are the ones this plan's code uses.

**S3b (concurrent dispatch, refresh visibility, auto-refresh re-check, panic boundaries) consumes and changes:**

- `connection::run_tool_call(state: &Arc<DaemonState>, budget: &mut ConnectionBudget, call: &ToolCall<'_>, settings: &ConnectionSettings) -> anyhow::Result<Value>` (private). It is the one function that takes the slot's locks for a call: the re-check after `state.write` is acquired goes into its last statement, where `ReadDispatch::NeedsAutoRefresh { initial_report }` has been taken out of the read lock.
- `DaemonState::read<R>(&self, impl FnOnce(&SessionProvider) -> R) -> Result<Option<R>, Poisoned>` and `DaemonState::write<R>(&self, impl FnOnce(&mut SessionProvider) -> R) -> Result<Option<R>, Poisoned>`. A panic boundary around a handler wraps the closure passed to these two. `write` already leaves the generation in the control plane; `SessionProvider::generation()` (now `pub`) is what a response is bound to.
- `Poisoned { lock: &'static str }`: every place it is constructed (`coordinator.rs`: `lock_control`, `ensure_ready`'s wait, `read`, `write`) is a place the abort belongs.
- `coordinator::PanicGuard`'s `Drop`: the body to replace with the abort.
- `TestHooks { published, settled }` and `AttemptHook`: the pattern for an injected panic in dispatch (spec §9 test 12).
- For its tests: `test_support::{ManualClock, Gate, scripted, Conn, wait_until, advance, HANG}`, `wire_support::{Client, connect, close, ended}`, and `connection_parity_tests::session` as the model of a whole session.

**S4 (ownership, handshake stage, admission and retirement, Draining) consumes:**

- `DaemonState::new(builder: SessionBuilder, startup: StartupMode) -> anyhow::Result<Arc<DaemonState>>`. `StartupMode::Background` is `--warm-at-startup` on the daemon's command line (spec §11 item 7); `StartupMode::Eager` is refused. `SessionBuilder` is `crate::mcp::lazy::SessionBuilder`; the in-process equivalent is `Arc::new(move || SessionProvider::bootstrap(&cfg))`.
- `connection::serve_admitted(state: &Arc<DaemonState>, registry: &ToolRegistry, stream: UnixStream, client: &ClientSettings) -> anyhow::Result<()>`, called on the worker thread after `READY` is written. The stream must be blocking with no read timeout again (the handshake stage makes it non-blocking). `client` is the `ClientSettings` from the decoded `Hello`; `serve_admitted` reads its output settings and its first-call wait and nothing else. One `ToolRegistry::all_v1()` can be shared by all workers (it is asserted `Send + Sync`). An `Err` return is an ordinary event (the peer left mid-frame); its text is an OS error, the fixed `Poisoned` text, or the fixed "no index is published" text, so it carries no peer bytes, but bound it anyway where it is logged.
- `DaemonState::status() -> Result<BuildStatus, Poisoned>` with `BuildStatus { state: BuildState, attempt: u64, generation: u64, waiting: u64 }`. `state` and `generation` are `READY`'s `build` and `generation` (`shared::preamble::Ready`); it takes only the control lock, so admission never waits on the provider. It is also the content of the status probe (SI-2): S4 adds the verb and its own fields.
- `ControlState { build: BuildPhase, attempts, waiting, last_error }`: the struct to add the pending and admitted counts, the last-activity instant and the retirement decision to, under the same mutex (spec §4.2). `DaemonState`'s `control` and `changed` fields are private to `coordinator.rs`; S4 either adds its admission functions to that file or widens them to `pub(super)`. Retirement's "no build in progress" is `!matches!(control.build, BuildPhase::Building { .. })`.
- `clock::{Clock, SystemClock}`, `Parts { clock, .. }` and `DaemonState::wake()` for the idle-TTL test with an injected clock (spec §9 test 2).
- To do there: remove `#![cfg_attr(not(test), allow(dead_code))]` from `daemon/mod.rs`; decide SI-5 (a parked call of a departed client holds its admitted slot for up to its first-call wait); and bound and strip control characters from `BuildPhase::Failed { error }` wherever it is logged or sent in a status reply (it is the repository loader's text, stored whole so that the tool result's `cause` is the in-process server's).

**S5a (the shim) consumes nothing here by name.** It owes the lane two things this slice could only do in process: spec §9 tests 1 and 14 through real shims (SI-1), and the byte parity of Task 8 through a shim against `--standalone`, with `tests/mcp/transcript.rs`.

## Self-review record

- **Spec coverage (§12 row S3a).** Readiness coordinator → Tasks 3 and 4 (`DaemonState`). Per-connection, per-attempt budgets → Task 2 (`ConnectionBudget`) and Task 3 (`ensure_ready`). Publication before `Ready` → Task 3 (`publish`), held by Task 4's hook test. Single retry → Task 3 (`start_attempt` under the lock that read the phase), in the reading D5 states. `RwLock<Option<SessionProvider>>` → Task 3 (the slot, `read`), Task 4 (`write`), Task 7 (dispatch through both). Test-only status probe → `DaemonState::status`, in process (D15, SI-2). Test 1 → Task 7 `two_clients_share_one_build`. Test 14 → Task 8, two tests. Both in-crate (D17, SI-1).
- **Spec §9 unit list, S3a's item:** "readiness coordinator: per-connection, per-attempt budgets, terminal-outcome delivery, single retry" → Task 2's five tests; Task 3 `the_spec_s_worked_example_two_budgets_on_one_attempt`, `later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget`, `a_retry_gives_a_connection_a_fresh_full_budget`, `a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself`, `the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together`; Task 4's late-waiter test.
- **Spec §10 rows S3a's code answers:** "Client sends invalid JSON → same `-32700`/`-32600` responses as stdio; connection stays" → Task 6 (the loop is the same code) and Task 8's parity session (garbage, an array, a call before `initialize`, a request with no id). "Panic in provider dispatch, build, or control plane → abort" and "panic in a connection's framing/output → that connection closes" → not S3a; the "Panics in S3a" table says what happens meanwhile.
- **Spec §4.2 sentences checked against the code:** "the build runs on one thread" (one `start_attempt`, from `Idle` or `Failed` only); "publishes the provider into the provider slot before announcing Ready" (`publish`); "a thread holding Control never waits on the provider" (`coordinator.rs` takes the slot in `publish`, `read`, `write` and the test-only `unpublish_for_tests`, none of them with the control lock held; `record_generation` takes control while holding the slot, the permitted order); "queries … operate on a read-only facade of the whole `SessionProvider`" (`read` hands out `&SessionProvider`, which is S1's facade through `impl SessionRuntime for SessionProvider`); "lifecycle and control methods run with no provider lock" (`serve_client` calls `handle_control` before anything else); "the in-process server keeps `LazySessionProvider` and `SessionRuntime` unchanged" (`lazy.rs` untouched; no `SessionRuntime` method added).
- **The lane's review lessons, one by one.** (1) Every stated property has a test and a mutant, or is listed as not held by a test: the "Claims" table. (2) What each mechanism does not cover: "What each mechanism covers, and what it does not". (3) Interleavings: publication before `Ready` and the late waiter are played through hooks (Task 4); one retry among many callers and one build among many clients are excluded by one lock acquisition and demonstrated with eight and with two callers; a response is written with no lock held by construction, and its many-clients test is S3b's. (4) No test asserts an upper bound on a duration or sleeps to order events; socket pairs need no path, so nothing depends on a temp-directory layout; the one real-clock test takes one second and asserts a lower bound. (5) S3a writes no log line and puts no peer bytes into an error; what leaves `serve_admitted` as an error is listed under S4's interface. (6) No `unwrap`, `expect`, indexing or unchecked arithmetic outside tests in `src/mcp/daemon/`. The `expect`s that remain on a daemon client's path are in code this slice does not edit (`transport.rs`, `tools_refresh.rs`, `freshness.rs`: each serializes a value the server itself built) and in the tool handlers, which this plan did not audit; what a panic there does today is in the "Panics in S3a" table, and what it should do is S3b's boundary. (7) Every wait in `ensure_ready` has a deadline from a `ConnectionBudget`, which only a `ClientSettings` can make (at most 600 s); the control lock is held for constant work; a departed client's call is bounded by its deadline and its build is the daemon's, not the client's.
- **Deliberate differences from the lane plan's S3a note:** the seam of `serve_loop` (D18).
- **Size.** Line counts of this plan's code blocks after `rustfmt` (see "What was checked without a compiler" below); they are not counts from a built tree.

  | PR | File | Lines |
  |---|---|---|
  | S3a-1 | `session.rs` +7, `session_stage_tests.rs` +23, `src/mcp/mod.rs` +2 | 32 |
  | | `daemon/mod.rs` | 25 |
  | | `daemon/budget.rs` 47, `budget_tests.rs` 80 | 127 |
  | | `daemon/clock.rs` | 17 |
  | | `daemon/coordinator.rs` | 500 |
  | | `daemon/test_support.rs` | 277 |
  | | `daemon/coordinator_tests.rs` | 358 |
  | | `daemon/coordinator_failure_tests.rs` | 156 |
  | | `daemon/coordinator_order_tests.rs` | 198 |
  | | `CLAUDE.md` | 13 |
  | | **S3a-1 total** | **about 1,700** (code 600, tests 1,090, docs 13) |
  | S3a-2 | `transport.rs` +27 −6, `transport_loop_tests.rs` 125 | 152 |
  | | `daemon/connection.rs` | 112 |
  | | `daemon/wire_support.rs` | 120 |
  | | `daemon/connection_tests.rs` | 320 |
  | | `daemon/connection_parity_tests.rs` | 354 |
  | | `daemon/coordinator.rs` +9 (test-only), `daemon/mod.rs` +3, `CLAUDE.md` +8 | 20 |
  | | **S3a-2 total** | **about 1,080** (code 140, tests 930, docs 8) |

  `coordinator.rs` is the largest file at about 510 lines once S3a-2's nine are in, 105 of them comments. S3a-1 is over the 1,500-line guide; the header says why it is left whole and how to split off Tasks 1 and 2 if that is wanted. Moving Task 4 to S3a-2 instead would also bring it under, at the cost of shipping "publication before Ready" and the late-waiter rule one pull request ahead of the tests that hold them; this plan does not do that.
- **What was checked without a compiler.** Every file directive of this plan ("Create", "Replace the whole of", "In … replace", "Append") was applied in order by a script to a copy of the eight files involved, with `rustfmt --edition 2021` run on the result after each task. Every anchor matched exactly once, at the point in the sequence where the plan uses it and after formatting; every mutant's anchor matches exactly once in the final formatted tree; `rustfmt` parsed every file, so the code blocks are syntactically valid Rust; the resulting diff of `transport.rs` is +27 −6 with both `-32700` branches untouched; and `transport_tests.rs`, `owner_transport_tests.rs` and `transport_split_tests.rs` are byte-identical. That is a check of the plan's text. It says nothing about types, borrows, lints or behavior: no test in this plan has ever run.
- **Not verified, beyond the list at the top:** that the fixture repository's responses are identical across two temporary directories for this plan's session (the lane plan measured it for a 25-response script that contains every request used here); the wording of `std`'s error for a custom `io::Error` in `a_build_thread_that_cannot_be_started_…` (the test expects the message passed to `io::Error::new`); that `SO_RCVTIMEO` set on the client's socket also bounds reads through its `try_clone` (it is one socket; if not, a hung server would hang a test instead of failing it at 30 s); Linux behavior of a write to a socket pair whose peer has closed (the two departed-client tests accept either outcome of that write).
