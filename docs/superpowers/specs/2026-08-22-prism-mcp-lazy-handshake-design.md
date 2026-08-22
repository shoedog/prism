# prism-mcp lazy handshake — design (roadmap #11)

Date: 2026-08-22 · Status: **v2 — A+C, sol round-1 findings folded; for sol round 2** · Owner-approved 2026-08-22: approach **A+C**
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
`ServerConfig.startup: StartupMode::{Lazy (default), Eager}`; `src/bin/prism-mcp.rs` gains `--eager` and `--first-call-wait <secs>`
(default 20; `0` = never block, answer *warming* immediately). `mod.rs::run`: Eager → today's path unchanged (`SessionProvider::bootstrap` then
`serve_stdio`); Lazy → `LazySessionProvider::new(&cfg)` then `serve_stdio`. `serve_stdio` generalizes to `&mut impl SessionRuntime`.

### 2.2 `src/mcp/lazy.rs` (new module; legacy files get call-site edits only)
```
pub struct LazySessionProvider { cfg: ServerConfig, state: LazyState, wait: Duration, last_error: Option<String>,
                                 builds: Arc<AtomicUsize> /* test hook: number of build attempts */ }
enum LazyState { Building { rx: Receiver<anyhow::Result<SessionProvider>>, started: Instant },
                 Ready(SessionProvider), Failed { error: String, at: Instant } }
pub enum Readiness { Ready, Warming { elapsed: Duration }, Failed { error: String } }
```
- `new(cfg)`: `canonical_config` (fail fast on a bad `--repo`, as today) then `spawn_build()`: `std::thread::spawn(move || SessionProvider::bootstrap(&cfg))`
  sending the result over an mpsc channel (`builds += 1`). The build therefore starts at **spawn** and overlaps the handshake + the
  agent's first think time; the snapshot boundary is spawn time, the same as eager (ms apart).
- `ensure_ready(&mut self) -> Readiness`: `Ready` → `Ready`; `Building` → `rx.recv_timeout(remaining budget)` where budget = `wait`
  (a call that arrives mid-build waits at most `wait`): `Ok(Ok(p))` → `Ready(p)`, `last_error = None` → `Ready`; `Ok(Err(e))` →
  `Failed{e}` → `Failed`; `Err(Timeout)` → `Warming{elapsed}`; (`Disconnected` = builder panicked → `Failed` with a fixed message);
  `Failed` → **respawn** the build (retry; `builds += 1`) then behave as `Building` (wait up to budget). Protocol-invalid calls never reach
  `ensure_ready` (2.3), so they never retry.
- `impl SessionRuntime for LazySessionProvider`: `ensure_ready` as above; every other trait method delegates to the `Ready` provider and is
  only reachable after `ensure_ready` returned `Ready` (2.3); `startup_mode() = Lazy`. Test-only constructor
  `with_builder(cfg, wait, Box<dyn FnOnce() -> anyhow::Result<SessionProvider> + Send>)` (injected slow / failing / succeeding builders;
  deterministic `builds` count).

### 2.3 `transport.rs` changes (call-site only)
- `SessionRuntime` gains `fn ensure_ready(&mut self) -> Readiness` (`StaticRuntime`, `SessionProvider` → `Ready`) and
  `fn startup_mode(&self) -> StartupMode` (`Eager` for both existing impls).
- `call_tool_response_with_cap_and_mode`: **keep every existing validation first** (request shape `-32602`, arguments object, registry
  membership / unknown-tool result, `refresh_index` argument check). Then, immediately before the first runtime-dependent operation
  (`refresh_index()`, freshness/stale report, `session()`): `match runtime.ensure_ready()` → `Warming{elapsed}` → `warming_result(elapsed)`;
  `Failed{error}` → `build_failure_result(&error)`; `Ready` → continue unchanged (auto-refresh, evidence, caps, modes all untouched).
- `warming_result`: `McpToolResult { is_error: true, content_text: "prism-mcp is still building the repository index (elapsed Ns). Retry this
  call shortly — no other action is needed; later calls are fast.", structured: {"status":"warming","elapsed_secs":N},
  meta: prism/schema_version + "prism/index_state":"warming" }`. `build_failure_result`: `is_error: true`, text "prism-mcp index build
  failed: <clamped cause>. The server keeps running; the next tool call retries the build.", meta `prism/index_state: "failed"`.
- `refresh_index` before readiness: `ensure_ready` (bounded wait) → `Ready` → delegate to `SessionProvider::refresh()` unchanged (generation
  1, verification + one divergence retry, `raced_stale` preserved — the background build plus this refresh is two builds on this rare path;
  correctness over optimization, per sol). `Warming` → warming result (no refresh performed).
- `initialize_response(obj, id, mode)` / `server_instructions(mode)`: `Eager` → the exact current string; `Lazy` → a truthful snapshot
  sentence replacing `SNAPSHOT_NOTICE`'s "loaded when prism-mcp started" ("The repository snapshot is taken when prism-mcp starts and its
  index is built in the background; until it is ready, tool calls return an `index warming` result — retry shortly. Freshness warnings
  compare against that startup snapshot.") + `VIEW_NOTICE`. `handle_message` obtains the mode from `runtime.startup_mode()`.

### 2.4 Error handling
Bad `--repo`: fail at spawn (as today). Build error: `Failed`, reported on the next valid `tools/call` (cause in text), retried on the
following valid call; `last_error` cleared on success. Builder panic: `Failed` ("index build panicked"), same retry. Budget `0`: never blocks.
Shutdown while building: stdin EOF ends the loop; the process exits and the detached build thread dies with it (as a pre-warm with
`--eager < /dev/null` is the supported way to build-and-exit).

### 2.5 Non-goals
No status tool, no cancellation, no progress streaming, no change to tool semantics, evidence shapes, cache layout, cache versions, or
freshness/refresh policy. No change to the nav CLI.

## 3. Compatibility
- **Byte-compat (narrowed):** Eager mode is byte-identical to today (pinned by an exact `initialize` fixture + the existing tool goldens).
  Lazy differs in (a) `initialize.instructions` (exact delta pinned), (b) the possible *warming*/*build failed* results before readiness,
  (c) nothing else for a stable filesystem and a successful build. Snapshot boundary ≈ eager (build starts at spawn).
- **Pre-warm recipe** (`docs/MCP.md` L67–70, L161): `prism-mcp --repo X --cache-dir D --eager < /dev/null` (builds then exits on EOF), or
  `prism nav --cache-dir D repo-map --repo X` (`--cache-dir` is a `NavArgs` flag and must precede the subcommand). "Cold first call" note
  rewritten to describe warming results and `--first-call-wait`. New flag rows for `--eager`, `--first-call-wait`.
- **Tier-C harness (required, sol WRONG-1):** `warm_gate_check` (`eval/tier_c/arm_runner.py`) must launch its throwaway server with
  `--eager` so "handshake ≤ 15 s" keeps meaning "the cache is warm" (`_prewarm_cpg` is best-effort and never raises). Update its docstring and
  argv tests; add a pytest that the gate FAILS when the prewarm is absent/mismatched (cold cache) and passes when warm. The agent-facing
  codex/claude configs stay lazy (that is the point).
- **First-call budget:** default 20 s wait < codex's 60 s per-tool default and Claude's limits; TS warm ≤ 19 s typically returns real results
  on the first call; cold large repos return *warming* and succeed on retry.

## 4. Tests (TDD; `cargo test --features mcp` + default `cargo test`; harness `InMemoryTransport` + `run_provider`; injected builders)
1. Lazy `[INIT, INITED, tools/list]` → answered; `builds == 1` (spawned at start), state not ready; instructions = exact lazy fixture.
2. Lazy + blocking builder (never completes), `wait = 0`: `tools/call nav_repo_map` → warming result (is_error, `prism/index_state`
   warming, elapsed ≥ 0); `ping` still answered; `builds == 1`.
3. Lazy + builder completing after the call starts (channel-released): with `wait = 5 s` the call returns the real result (identical to eager's);
   second call does not rebuild (`builds == 1`).
4. `refresh_index` as the first call (ready builder): generation **1**, status per existing contract; forced-divergence regression (repo
   changes during the initial build → `raced_stale`, `stale_before_refresh` truthful); `refresh_index` while warming → warming result.
5. Failing builder: first valid call → build-failure result with the cause (`prism/index_state` failed), `builds == 1`; second valid call →
   retry (`builds == 2`), still failing → failure; swap to a succeeding builder (deleted-then-recreated repo-root fixture or injected) →
   third call succeeds, `last_error` cleared (`builds == 3`).
6. Negatives never build/retry: pre-init `tools/call`, invalid `initialize`, malformed `tools/call` (missing params → `-32602`), unknown
   tool, invalid `refresh_index` arguments → `builds` unchanged (== 1, the spawn build) and no wait incurred (assert elapsed < budget with
   a blocking builder).
7. Eager: exact `initialize` fixture byte-identical to today's; existing transport_tests pass unchanged; `startup_mode() == Eager`.
8. Lazy delegation after readiness across `WarnOnly` / `AutoFull` / `AutoIncremental` (existing provider tests re-run through the lazy wrapper).
9. CLI (`assert_cmd`): default selects lazy (handshake answered with no repo read — e.g. `--repo` pointing at a large fixture completes
   `initialize` in < 1 s); `--eager` selects eager; `--first-call-wait 0`; `--eager --cache-dir D < /dev/null` exits 0 and populates `D`.
10. Harness pytest: `warm_gate_check` argv contains `--eager`; gate fails on a cold cache, passes warm.

## 5. Acceptance (controller, before merge)
- Full suite + `--features mcp` green; fmt; clippy no new warnings; `git diff --check`.
- Live probe (treatment vs control): codex 0.147 (`codex exec --json`, isolated CODEX_HOME as the harness builds) against the **cold**
  TypeScript bench repo — lazy default: server kept, prism tools exposed, first `nav_*` call returns a result or a *warming* result and a
  retry succeeds; control: `--eager` same repo → dropped (the read-out's failing probe). Record both transcripts in the PR body.
- `docs/MCP.md` updated; roadmap row 11 → DONE (PR #).

## 6. Open questions for sol (round 2)
- Q1 `is_error: true` on the *warming* result (tool did not execute; text tells the agent to retry) vs `false` with structured status — which
  do agents handle better without misreading as "no results"?
- Q2 `--first-call-wait` default 20 s — too close to codex's 60 s if the client stacks calls? Should the budget be per-call (as specified) or
  per-session (total)?
- Q3 retry-on-failure respawn: any failure class where immediate respawn on the next call is harmful (e.g., OOM on a huge repo)? A cap is
  deliberately not specified.
- Q4 the lazy instructions sentence — precise enough about the snapshot boundary for freshness semantics?
- Q5 harness: `--eager` in the gate vs a representative `tools/call` (which would also measure first-call latency) — start with `--eager`?
- Q6 anything in `auto_refresh`/freshness that assumes it runs on the same thread that built the index (thread-affine state)?
