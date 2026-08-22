# prism-mcp lazy handshake — design (roadmap #11)

Date: 2026-08-22 · Status: **v3 — A+C; sol round-1 and round-2 findings folded; for sol round 3 (declared cap)** · Owner-approved 2026-08-22: approach **A+C**
(background bootstrap at spawn, instant handshake, bounded wait on `tools/call` then a structured *warming* result; `--eager` keeps
today's synchronous startup). Record: `docs/analysis/2026-08-21-tier-c-partd-readout.md` §Caveats; roadmap row 11;
ledger 2026-08-21 "RECONCILED PLAN (5)" (dual-model: a blocking lazy build just moves a 263 s cold build to the first call, which dies at
codex's default 60 s per-tool timeout → the first call must return *warming, retry*, not block).

## 1. Problem (measured)
`prism::mcp::run` (`src/mcp/mod.rs`) calls `SessionProvider::bootstrap` (`session.rs` → `build_state`: `load_repo` + `NavigationIndex::build*`)
**before** `serve_stdio` reads stdin; the client's `initialize` waits for the whole index build (TS warm 17–19 s; ruff cold 263 s; prometheus
cold 24.5 s). codex 0.147 silently drops any MCP server not answering within ~10 s regardless of `startup_timeout_sec`; Claude Code has
short startup limits too. codex's per-tool-call default is 60 s (harness arms raise it to 600 s; Claude arms set `MCP_TOOL_TIMEOUT=600000`).
`initialize`, `ping`, `notifications/*`, `tools/list` (static from `ToolRegistry`) need nothing from the index; only the `tools/call` path
touches `SessionRuntime::session()/freshness()/known_stale_after_refresh()/refresh_index()` (sol reachability audit: no hidden path).

Fact (compile-probed 2026-08-22): `SessionProvider`, `NavigationSession`, `NavigationIndex`, `LoadedRepo` are `Send` (`Index`/`LoadedRepo`
also `Sync`); the `#[allow(clippy::arc_with_non_send_sync)]` in `session.rs` is stale. A build thread + `JoinHandle`/channel is enough — no
actor transport.

## 2. Design (A+C)
### 2.1 Startup mode
`ServerConfig` gains two fields, both copied through `canonical_config`: `startup: StartupMode::{Lazy (default), Eager}` and
`first_call_wait: Duration` (default 20 s; `Duration::ZERO` = never block, answer *warming* immediately). `src/bin/prism-mcp.rs` gains
`--eager` and `--first-call-wait <secs>` (accepted and ignored under `--eager`; documented as such). `ServerConfig::new` defaults both. `mod.rs::run`: Eager → today's path unchanged (`SessionProvider::bootstrap` then
`serve_stdio`); Lazy → `LazySessionProvider::new(&cfg)` then `serve_stdio`. `serve_stdio` generalizes to `&mut impl SessionRuntime`.

### 2.2 `src/mcp/lazy.rs` (new module; legacy files get call-site edits only)
```
pub struct LazySessionProvider { cfg: ServerConfig, state: LazyState, wait: Duration, last_error: Option<String>,
                                 attempts: usize /* incremented SYNCHRONOUSLY before each spawn */,
                                 builder: Arc<dyn Fn() -> anyhow::Result<SessionProvider> + Send + Sync> }
enum LazyState { Building { rx: Receiver<anyhow::Result<SessionProvider>>, started: Instant, deadline: Option<Instant> },
                 Ready(SessionProvider), Failed { error: String, at: Instant } }
pub enum Readiness { Ready, Warming { elapsed: Duration }, Failed { error: String } }
```
- `new(cfg)`: `canonical_config` (fail fast on a bad `--repo`, as today; `canonical_config` becomes `pub(super)`) then `spawn_build()`:
  `attempts += 1` (synchronously, so `attempts == 1` is observable right after construction) and `std::thread::spawn` running the
  `builder` (production: `SessionProvider::bootstrap(&cfg)`), sending the result over an mpsc channel; `deadline = None` until the first
  valid call. The build therefore starts at server startup and overlaps the handshake + the agent's first think time. The snapshot is
  whatever `load_repo` sees when the background thread reaches it (after spawn) — edits between spawn and loading are included.
  The thread is detached (no join on shutdown).
- `ensure_ready(&mut self) -> Readiness`: `Ready` → `Ready`. `Building` → the wait budget is **cumulative per build attempt** (sol r2
  WRONG-1: the transport is serial, so a fresh per-call budget multiplies under head-of-line blocking — four pipelined calls would answer at
  ~20/40/60/80 s): the FIRST valid call sets `deadline = now + wait` (if `deadline` is `None`); every call then does
  `rx.recv_timeout(deadline.saturating_duration_since(now))` (zero when the deadline has passed → an immediate `try_recv`):
  `Ok(Ok(p))` → `Ready(p)`, `last_error = None` → `Ready`; `Ok(Err(e))` → `Failed{e}` → `Failed`; `Err(Timeout)` → `Warming{elapsed}`
  (immediate once the deadline has passed); `Err(Disconnected)` (builder thread panicked) → `Failed{"index build panicked"}`.
  `Failed` → **respawn** via the `builder` (retry; `attempts += 1`; a NEW deadline = now + wait) then behave as `Building`. Total added
  latency across any number of queued calls during one build attempt ≈ one `wait`, never `N × wait`. `ping`/`tools/list` never wait.
  Protocol-invalid calls never reach `ensure_ready` (2.3), so they never wait or retry.
- `impl SessionRuntime for LazySessionProvider` lives in `transport.rs` (the trait is private there; sol r2 SMELL-9): `ensure_ready` as
  above; every other trait method delegates to the `Ready` provider and is only reachable after `ensure_ready` returned `Ready` (2.3);
  `startup_mode() = Lazy`. Test-only: `with_builder(cfg, wait, Arc<dyn Fn() -> anyhow::Result<SessionProvider> + Send + Sync>)` (a
  factory, callable once per attempt — tests use a shared `Mutex<VecDeque<Outcome>>` queue for fail/fail/succeed sequences and a
  channel-released blocking builder that tests release in teardown); test-only accessors `attempts()`, `state_kind()`, `last_error()`.

### 2.3 `transport.rs` changes (call-site only)
- `SessionRuntime` gains `fn ensure_ready(&mut self) -> Readiness` (`StaticRuntime`, `SessionProvider` → `Ready`) and
  `fn startup_mode(&self) -> StartupMode` (`Eager` for both existing impls).
- `call_tool_response_with_cap_and_mode`: **keep every existing validation first** (request shape `-32602`, arguments object, registry
  membership / unknown-tool result, `refresh_index` argument check). Then, immediately before the first runtime-dependent operation
  (`refresh_index()`, freshness/stale report, `session()`): `match runtime.ensure_ready()` → `Warming{elapsed}` → `warming_result(elapsed)`;
  `Failed{error}` → `build_failure_result(&error)`; `Ready` → continue unchanged (auto-refresh, evidence, caps, modes all untouched).
- `warming_result`: `McpToolResult { is_error: true, content_text = canonical JSON of the status object `{"status":"warming",
  "elapsed_secs":N,"message":"prism-mcp is still building the repository index; retry this call shortly — no other action is needed;
  later calls are fast."}` (so the machine-readable status survives `OmitDefaultPath`, which drops transport-generated `structuredContent`
  — `output.rs::to_call_tool_result_value`; sol r2 WRONG-2), structured: the same object, meta: `prism/schema_version`,
  `prism/index_state: "warming"`, `prism/retryable: true` }`. `build_failure_result`: same shape with `status: "build_failed"`,
  `cause: <clamped>`, message "the server keeps running; the next tool call retries the build", meta `prism/index_state: "failed"`,
  `prism/retryable: true`. Both tested under `StructuredContentMode::Always` AND `OmitDefaultPath` (text carries the status in both).
- `refresh_index` before readiness: `ensure_ready` (bounded wait) → `Ready` → delegate to `SessionProvider::refresh()` unchanged (generation
  1, verification + one divergence retry, `raced_stale` preserved — the background build plus this refresh is two builds on this rare path;
  correctness over optimization, per sol). `Warming` → warming result (no refresh performed).
- `initialize_response(obj, id, mode)` / `server_instructions(mode)`: `Eager` → the exact current string; `Lazy` → a truthful snapshot
  sentence replacing `SNAPSHOT_NOTICE`'s "loaded when prism-mcp started": "The repository snapshot is loaded by a background build started
  at server startup; until it completes, tool calls return an `index warming` result — retry shortly. Freshness warnings compare the
  working tree against the most recently completed build or refresh snapshot." + `VIEW_NOTICE`. `handle_message` obtains the mode from
  `runtime.startup_mode()`.
- Visibility / API: `session.rs::canonical_config` → `pub(super)`; `transport.rs` keeps the public eager `serve_stdio(&mut SessionProvider, …)`
  wrapper unchanged and adds a crate-private generic `serve_stdio_runtime(&mut impl SessionRuntime, …)` used by `mod.rs::run` for both
  modes; `run_provider` in `transport_tests.rs` generalizes to `&mut impl SessionRuntime`.

### 2.4 Error handling
Bad `--repo`: fail at spawn (as today). Build error: `Failed`, reported on the next valid `tools/call` (cause in text), retried on the
following valid call; `last_error` cleared on success. Builder panic: `Failed` ("index build panicked"), same retry. Budget `0`: never blocks.
Shutdown while building: stdin EOF ends `serve_runtime` (it returns without joining the worker); in the `prism-mcp` binary the process
then exits and the detached build thread dies with it; an embedding library caller may keep running with the detached thread (documented).
A pre-warm with `--eager < /dev/null` is the supported way to build-and-exit.

### 2.5 Non-goals
No status tool, no cancellation, no progress streaming, no change to tool semantics, evidence shapes, cache layout, cache versions, or
freshness/refresh policy. No change to the nav CLI.

## 3. Compatibility
- **Byte-compat (narrowed):** Eager mode is byte-identical to today (pinned by an exact `initialize` fixture + the existing tool goldens).
  Lazy differs in (a) `initialize.instructions` (exact delta pinned), (b) the possible *warming*/*build failed* results before readiness,
  (c) nothing else for a stable filesystem and a successful build. The snapshot is taken when the background `load_repo` runs (shortly
  after spawn), not at `initialize`; after `refresh_index` the baseline is the refreshed snapshot (as today).
- **Pre-warm recipe** (`docs/MCP.md` L67–70, L161): `prism-mcp --repo X --cache-dir D --eager < /dev/null` (builds then exits on EOF), or
  `prism nav --cache-dir D repo-map --repo X` (`--cache-dir` is a `NavArgs` flag and must precede the subcommand). "Cold first call" note
  rewritten to describe warming results and `--first-call-wait`. New flag rows for `--eager`, `--first-call-wait`.
- **Tier-C harness (required, sol WRONG-1):** `warm_gate_check` (`eval/tier_c/arm_runner.py`) must launch its throwaway server with
  `--eager` so "handshake ≤ 15 s" keeps meaning "the cache is warm" (`_prewarm_cpg` is best-effort and never raises). Update its docstring and
  argv tests; add a pytest that the gate FAILS when the prewarm is absent/mismatched (cold cache) and passes when warm. The agent-facing
  codex/claude configs stay lazy (that is the point).
- **First-call budget:** default 20 s, cumulative per build attempt (never `N × wait` for queued calls) — below codex's 60 s per-tool
  default and Claude's limits; TS warm ≤ 19 s typically returns real results on the first call; cold large repos return *warming* (within
  ms once the deadline has passed) and succeed on retry.

## 4. Tests (TDD; `cargo test --features mcp` + default `cargo test`; harness `InMemoryTransport` + generalized `run_provider(&mut impl
SessionRuntime)`; injected builder factories; test-only accessors `attempts()/state_kind()/last_error()`)
1. Lazy `[INIT, INITED, tools/list]` → answered; `attempts() == 1` immediately after construction; state not ready; `initialize.instructions`
   equals the exact lazy fixture.
2. Lazy + channel-released blocking builder (released in teardown), `wait = 0`: `tools/call nav_repo_map` → warming result (is_error,
   `prism/index_state` warming, `prism/retryable` true, text = canonical status JSON with `status:"warming"`), under BOTH
   `StructuredContentMode::Always` and `OmitDefaultPath`; `ping` still answered; `attempts() == 1`.
3. Cumulative deadline: blocking builder, `wait = 200 ms`, FOUR queued `tools/call`s → all four warming; total elapsed ≈ one `wait` (assert
   < 2 × wait, not ≥ 4 × wait); releasing the builder then a fifth call → real result; `attempts() == 1`.
4. Builder completing after the call starts: `wait = 5 s` → the call returns the real result (identical to eager's); second call does not
   rebuild (`attempts() == 1`).
5. `refresh_index` first-call semantics (two tests): (a) repo edited DURING the initial background build, then stable → first `refresh_index`
   returns generation **1**, `stale_before_refresh: true`, status `refreshed`; (b) forced divergence during the refresh verification
   window (`force_next_verification_for_tests(Diverged)`) → status `raced_stale`, known-stale retained; plus `refresh_index` while warming →
   warming result, no refresh performed.
6. Failure/retry with a queued factory [fail, fail, succeed]: 1st valid call → build-failure result (`status:"build_failed"`, cause,
   `attempts() == 1`); 2nd → retry, failure again (`attempts() == 2`, new deadline observed); 3rd → success (`attempts() == 3`,
   `last_error() == None`). Deleted-then-recreated repo-root variant (real `bootstrap`) as an integration pole.
7. Panicking builder → `Failed{"index build panicked"}` result; next valid call respawns (`attempts() == 2`).
8. EOF during build: `InMemoryTransport` queue empties while the builder is blocked → `serve_runtime` returns `Ok` without joining (bounded
   elapsed); release builder in teardown.
9. Negatives never build/wait/retry: pre-init `tools/call`, invalid `initialize`, malformed `tools/call` (missing params → `-32602`),
   non-object arguments, unknown tool, invalid `refresh_index` arguments → `attempts()` unchanged (== 1) and elapsed ≪ `wait` with a blocking
   builder.
10. Eager: exact `initialize` fixture byte-identical to today's; existing transport_tests pass unchanged; `startup_mode() == Eager`;
    `first_call_wait` ignored.
11. Lazy delegation after readiness across `WarnOnly` / `AutoFull` / `AutoIncremental` (existing provider tests re-run through the lazy wrapper).
12. CLI (`assert_cmd`): default `prism-mcp` answers `initialize` with the exact LAZY instructions (mode proof by content, not timing);
    `--eager` → exact eager instructions; `--first-call-wait 0` accepted (and accepted+ignored with `--eager`);
    `--eager --cache-dir D < /dev/null` exits 0 and populates `D`.
13. Harness pytest: `warm_gate_check` argv contains `--eager`; gate fails on a cold cache, passes warm; docstring updated.

## 5. Acceptance (controller, before merge)
- Full suite + `--features mcp` green; fmt; clippy no new warnings; `git diff --check`.
- Live probe (treatment vs control): codex 0.147 (`codex exec --json`, isolated CODEX_HOME as the harness builds) against the **cold**
  TypeScript bench repo — lazy default: server kept, prism tools exposed, first `nav_*` call returns a result or a *warming* result and a
  retry succeeds; control: `--eager` same repo → dropped (the read-out's failing probe). Record both transcripts in the PR body.
- `docs/MCP.md` updated; roadmap row 11 → DONE (PR #).

## 6. Sol round-2 answers (recorded) and round-3 questions
Recorded: Q1 `is_error: true` + machine-readable retryable status in BOTH text and structured (`prism/retryable: true`); Q2 cumulative
per-build-attempt budget, 20 s; Q3 no retry cap (request-driven, not a loop; record attempts; cooldown only if live evidence shows storms);
Q4 instructions text as in §2.3; Q5 gate uses `--eager`; Q6 no thread-affine state (queries/refresh run on the transport thread after the
`Send` transfer; `OnceLock` + shared Rayon pool).
Round 3: R1 any remaining WRONG in the deadline state machine (§2.2) — e.g. a call arriving exactly at the deadline, or `wait = 0` with a
build that completes between `try_recv` and the response? R2 the warming/failure result shape (canonical status JSON in text + structured)
— consistent with the other transport-generated results (`unknown_tool_result`, refresh errors)? R3 anything the generalized
`serve_stdio_runtime` / `run_provider` seam breaks for existing callers (MCP feature gate, `lib.rs` exports)?
