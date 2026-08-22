# prism-mcp lazy handshake — design (roadmap #11)

Date: 2026-08-22 · Status: DRAFT for sol spec review · Owner-approved approach: **A (lazy-on-first-call, single-threaded) + `--eager`**
Record: `docs/analysis/2026-08-21-tier-c-partd-readout.md` §Caveats (probe), `docs/analysis/prism-post-plan-roadmap.md` row 11.

## 1. Problem (measured)

`prism::mcp::run` (`src/mcp/mod.rs`) calls `SessionProvider::bootstrap` (`src/mcp/session.rs::bootstrap` → `build_state`: `load_repo` +
`NavigationIndex::build*`) **before** `serve_stdio` reads a single byte of stdin. The client's `initialize` therefore waits for the whole
index build. Warm loads are 17–19 s on the TypeScript bench repo and longer cold; codex 0.147 silently drops any MCP server that has not
answered within ~10 s regardless of `startup_timeout_sec`/`startup_timeout_ms` (probe-proven; it voided a Part-D slate and leaves every
TypeScript cell unmeasurable). Claude Code also has short MCP startup limits. Today's documented mitigation is "pre-warm once".

Observation that makes the fix small: `initialize`, `ping`, `notifications/*` and `tools/list` (`transport.rs::list_tools` — static from
`ToolRegistry`) need nothing from the index. Only `tools/call` (and therefore `refresh_index`, auto-refresh and the freshness probe)
touch `SessionRuntime::session()` / `freshness()`.

## 2. Design (approach A)

### 2.1 Components
- `session.rs`: new `pub struct LazySessionProvider { cfg: ServerConfig, inner: Option<SessionProvider>, last_error: Option<String> }`
  with `pub fn new(cfg) -> anyhow::Result<Self>` (runs `canonical_config` only — cheap, fail-fast on a bad `--repo`), and
  `pub fn ensure_ready(&mut self) -> anyhow::Result<&mut SessionProvider>` (bootstraps exactly once; on error stores `last_error`, returns
  `Err`; a later call retries). `pub fn is_ready(&self) -> bool` for tests/telemetry.
- `transport.rs::SessionRuntime` gains `fn ensure_ready(&mut self) -> anyhow::Result<()>`; `StaticRuntime` and `SessionProvider` return
  `Ok(())`; `LazySessionProvider` implements the trait by delegating every existing method to `inner` **after** `ensure_ready`
  (`session()`/`freshness()`/`known_stale_after_refresh()` keep their `&self` signatures; they are only reachable after `ensure_ready`
  — see 2.2 — and `freshness()` returns `None` when not built, which the existing code already tolerates).
- `transport.rs::call_tool_response_with_cap_and_mode`: first statement `if let Err(e) = runtime.ensure_ready() { return tool error result
  "prism-mcp index build failed: <clamped cause> (the server keeps running; the next tool call retries the build)" with
  `is_error: true`, standard `prism/schema_version` meta }`. Nothing else in the call path changes.
- `refresh_index` when not yet built: `ensure_ready` IS the build — return a `RefreshSummary { status: "refreshed", strategy: "full",
  fallback_reason: None, generation: 0, indexed_files/tracked_paths from the fresh session, stale_before_refresh: false, … }` (no double
  build). When built: delegate unchanged.
- `transport.rs::server_instructions()` (used by `initialize_response`): in lazy mode append `LAZY_NOTICE` — one sentence: "The first
  tool call builds the repository index (tens of seconds on a large repo, seconds when the cache is warm); later calls are fast." Eager
  mode keeps today's instructions byte-for-byte.
- `src/bin/prism-mcp.rs` + `ServerConfig`: new `--eager` flag (`startup: StartupMode::{Lazy, Eager}`, default Lazy). `mod.rs::run`:
  Eager → today's path unchanged (`SessionProvider::bootstrap` then `serve_stdio`); Lazy → `LazySessionProvider::new` then
  `serve_stdio` over it.

### 2.2 Data flow (lazy)
spawn → read stdin immediately → `initialize` answered (<10 ms) with instructions incl. LAZY_NOTICE → `notifications/initialized` →
`tools/list` answered from the registry → first `tools/call` → `ensure_ready` builds (blocking this one call; the client is waiting on a
tool result, governed by its per-call timeout, not its startup timeout) → dispatch as today → subsequent calls as today. `ping`,
`tools/list`, unknown methods, malformed input: never trigger a build.

### 2.3 Error handling
- Bad `--repo` / un-canonicalizable config: fail at spawn as today (`LazySessionProvider::new` runs `canonical_config`).
- Index build failure (I/O, parse, cache dir unwritable…): the tool call returns an `is_error` result with the cause; the server stays
  alive; the next `tools/call` retries (no backoff — builds are seconds; an agent rarely retries more than a few times). `last_error` is
  reported in the result text.
- Build panics: unchanged from today (process exits; the client sees a dead server).

### 2.4 Non-goals (explicit)
- No background/eager-async build (the session state is `!Send` — `#[allow(clippy::arc_with_non_send_sync)]` in `session.rs`; a thread
  would need an actor-pattern transport). Deferred as "approach B" until a measured first-call timeout appears.
- No bounded-wait "warming, retry" result (needs B). No change to tool semantics, wire shapes, cache layout, or freshness/refresh policy.

## 3. Compatibility
- **Pre-warm recipe**: `prism-mcp --repo X --cache-dir D < /dev/null` relied on eager bootstrap; with lazy default, EOF arrives before any
  build. `docs/MCP.md` L67–70 / L161 change to `prism-mcp --repo X --cache-dir D --eager < /dev/null` and mention the equivalent
  `prism nav repo-map --repo X --cache-dir D` (same nav store: `NavigationIndex::build_cached_under`). The "Cold first call" note is
  rewritten to describe lazy behavior.
- **Tier-C harness** (`eval/tier_c/arm_runner.py`): prewarm already uses `prism nav repo-map --cache-dir`; `warm_gate_check` (JSON-RPC
  initialize/tools-list handshake ≤15 s) keeps working and now always passes quickly — it still proves "the server answers the handshake";
  the prewarm still proves "the cache is warm". No harness change required; optional follow-up: a first-call latency probe in the gate.
- **First-call budget**: Claude arms set `MCP_TOOL_TIMEOUT=600000`; codex's per-tool-call timeout is separate from its startup drop
  (believed 60 s default — CONFIRM BY PROBE before declaring TS measurable). TS warm (≤19 s) fits both; cold builds beyond a client's
  per-call timeout remain "pre-warm once" (documented).
- **Wire**: `initialize.instructions` gains one sentence in lazy mode only; everything else byte-identical (existing
  `transport_tests.rs` golden assertions must pass unchanged under `--eager`, and under lazy except for the instructions sentence).

## 4. Tests (TDD, `cargo test --features mcp`; `src/mcp/transport_tests.rs` harness: `InMemoryTransport` + `run_provider`)
1. Lazy: `[INIT, INITED, tools/list]` → all answered, `is_ready() == false` (no build). `initialize.instructions` contains LAZY_NOTICE.
2. Lazy: `[…, tools/call nav_repo_map]` → `is_ready()` flips exactly once; result identical to the eager server's; a second call does not
   rebuild (generation unchanged; build counter == 1 via a test hook or `is_ready` + generation).
3. Lazy: `refresh_index` as the first call → builds once, returns status "refreshed"/strategy "full"/generation 0; a subsequent
   `refresh_index` → generation 1 (existing semantics).
4. Lazy: bootstrap failure (point `--repo` at a dir that is deleted between `new` and the first call, or an unwritable `--cache-dir`
   under `CacheMode::Dir`) → `is_error` tool result with the cause; server still answers `ping`; a later call after the cause is fixed
   succeeds (retry).
5. `ping` / unknown method / malformed line / `tools/list` never build (`is_ready()` stays false).
6. Eager (`--eager`): `initialize.instructions` byte-identical to today's; behavior identical (existing tests re-run under eager).
7. CLI: `prism-mcp --help` shows `--eager`; `--eager` + `--no-cache` and `--eager` + `--cache-dir` accepted.
8. Pre-warm: `prism-mcp --repo <tmp> --cache-dir <d> --eager < /dev/null` exits 0 and populates `<d>` (integration test via `assert_cmd`).

## 5. Acceptance (before merge)
- Full suite + `--features mcp` green; `cargo fmt`; clippy.
- Live probe (controller, not the implementer): codex 0.147 against the **cold** TypeScript bench repo (`~/code/bench-repos/TypeScript`),
  server in lazy mode: server kept (tools exposed), first `nav_*` call succeeds; the read-out's failing probe (eager, same repo) rerun as
  the control. Record both in the PR body. If codex's per-call timeout kills the first call, record the number and open approach B.
- `docs/MCP.md` updated; roadmap row 11 → DONE with PR #.

## 6. Risks / open questions for sol
- Q1: `SessionRuntime` trait change vs a separate `LazyRuntime` wrapper in transport — is the trait method the right seam given
  `StaticRuntime` (tests) and `SessionProvider` both need it?
- Q2: `refresh_index`-before-build summary shape (generation 0, status "refreshed") — acceptable, or should it delegate and report
  generation 1 (double build)?
- Q3: retry-on-every-call for build failures — any pathological case (e.g., a huge repo failing late each time) that argues for a cap?
- Q4: anything in the auto-refresh / freshness path that assumes `freshness()` is `Some` before the first call?
- Q5: lazy as the DEFAULT (breaking the old pre-warm recipe silently) vs eager default + `--lazy` opt-in. Owner-approved: lazy default.
