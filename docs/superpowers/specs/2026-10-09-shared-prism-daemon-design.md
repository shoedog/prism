# Shared per-repo prism daemon — design

Date: 2026-10-09 · Status: **draft v1 for owner review** · Owner decision 2026-10-09: approach **A**
(per-repo daemon + stdio shim) with the **C** memory measurement folded in as a first-class step.
Predecessor: build-on-first-call (`StartupMode::Lazy` now defers the index build to the first valid
`tools/call`; `--warm-at-startup` restores the at-spawn build) — shipped separately on
`worktree-prism-mcp-build-on-first-call`.

## 1. Problem and goal

Every MCP client process spawns its own `prism-mcp`, and every `prism-mcp` that answers a tool call holds
the whole repository index in memory. On this machine the clients multiply:

- codex app-server starts the project's MCP servers **per thread**; codex-acp adds an ephemeral
  title-generation thread per new session; the a2a-bridge fans out reviewers and implementers.
- Claude Code starts one per session through the plugin.
- Standalone `codex exec` runs in the repo start one each.

Measured on the slicing repo (1,232 files, 420k CPG nodes, 672k edges):

| Measurement | Value |
|---|---|
| Resident set of one idle `prism-mcp` after the index is built | 1,168 MB |
| Peak during a warm cache load (`--eager`) | 1,620 MB, 1.6 s wall |
| On-disk nav cache for the same repo | 304 MB CPG + 21 MB call-edge index |
| 2026-10-03 bridge incident | 29 `prism-mcp` under one codex app-server, 34 GiB compressed, swap full |

Goal, in the sense of the proportionality rule: let the bridge run its reviewer and implementer fan-out
on this host without swap exhaustion. Success is **one resident index per repository checkout**, shared by
every agent that navigates it, with no measurable change in query latency, and a smaller index where the
measurement says it is cheap to get.

Build-on-first-call already removes the cost for clients that never call a tool. This design removes
the duplication for clients that do.

## 2. Non-goals

- Serving several worktrees of one repository from one process. One canonical checkout root = one
  daemon. (Worktrees are the accepted compromise; a shared-common-dir design can come later.)
- A network or HTTP listener. The daemon speaks newline-delimited JSON-RPC over a Unix socket only. An
  HTTP listener can be added to the same daemon later for URL-configured clients; nothing here blocks it.
- Changing the experimental owner runtime (`--owner-*`). It stays in-process (`--standalone` is implied).
- Reducing the on-disk cache format. Measurement (§6) decides what the in-memory reductions are; this
  spec does not pre-commit to any of them.

## 3. Architecture

```
codex thread A ──stdio──▶ prism-mcp (shim) ──┐
codex thread B ──stdio──▶ prism-mcp (shim) ──┼─unix socket─▶ prism-mcp daemon ──▶ one NavigationSession
claude session ──stdio──▶ prism-mcp (shim) ──┘                (per canonical repo root)
```

- The **shim** is the existing `prism-mcp --repo X` binary in shared mode. It is a byte pipe: stdin to
  the socket, socket to stdout. It holds no index and costs a few MB.
- The **daemon** is the same binary started as `prism-mcp daemon --repo X ...`. It owns one
  `LazySessionProvider` (so the index still builds on the first valid `tools/call`, from any client) and
  serves each connection on its own thread with its own MCP `Lifecycle`.
- The **identity key** that names the socket, lock and log is a SHA-256 over
  `(canonical repo root, CARGO_PKG_VERSION, GRAMMAR_FINGERPRINT, cache mode + cache dir, refresh policy)`.
  Two clients only share a daemon when every one of those agrees; a new prism build never talks to an old
  daemon, and the old daemon simply idles out.

## 4. Components

### 4.1 Shim (`prism-mcp --repo X [--shared|--standalone]`)

1. Resolve `ServerConfig` exactly as today (`canonical_config`).
2. If `--standalone` (or owner options, or `--eager`, or `--no-cache`): serve in-process as today. Shared
   mode requires a cache-backed, non-eager config; everything else is explicitly local.
3. Compute the identity key and socket path (§4.3). Try `connect`.
4. On `ECONNREFUSED`/`ENOENT`: take the start lock (§4.3). Under the lock, re-try `connect` (another shim
   may have won), else remove a stale socket file and spawn the daemon detached
   (`setsid`/own process group, stdin null, stdout+stderr to the log file, env inherited). Then poll
   `connect` until the socket accepts or `--daemon-start-timeout` (default 10 s) elapses.
5. Pump bytes both ways until stdin EOF or the socket closes. Stdin EOF: half-close the socket, drain
   pending responses, exit 0. Socket EOF/error while the client is still open: exit non-zero (the client
   sees the server die, as it would today if the process crashed).
6. **Fallback.** Any failure in steps 3–4 (lock timeout, daemon failed to come up, socket error before the
   first byte) logs one line to stderr and serves **in-process**, identical to today. Shared mode can
   therefore never make a client worse off than standalone mode; it can only save memory.

The handshake latency budget is unchanged: the daemon is lazy, so `initialize` and `tools/list` are served
from the idle daemon in milliseconds; codex's `startup_timeout_sec` (60 s in the project config, 120 s in
the bridge) is never approached.

### 4.2 Daemon (`prism-mcp daemon --repo X --socket P --idle-ttl S ...`)

- Binds `P` (mode 0600, parent dir 0700), writes `<key>.pid`, then enters the accept loop.
- Owns `Arc<Mutex<LazySessionProvider>>` plus the `ToolRegistry`. Startup mode inside the daemon is
  `Lazy` by default and `Background` if the first shim passed `--warm-at-startup`.
- Per connection: one thread, one `UnixStreamTransport` (same framing as `StdioTransport`), one
  `Lifecycle`. The loop is the existing `serve_runtime` split so that reading and writing happen
  outside the lock and `handle_message` runs under it:

  ```
  loop { msg = transport.read_message()?;           // no lock
         resp = { let g = provider.lock(); handle_message(&msg, &mut *g, registry, &mut lifecycle) };
         transport.write_message(resp)? }            // no lock
  ```

  Tool calls are serialized across clients. Navigation queries are milliseconds, so this is not a
  latency change in practice; a `refresh_index` (seconds) blocks other clients for its duration, which
  is the same wall-clock they would have spent rebuilding their own index. If measurement shows
  contention (§9 soak), the mutex becomes an `RwLock` with queries as readers; the trait surface does not
  change.
- **Idle exit.** A connection counter and a last-activity timestamp. When the counter is 0 for
  `--idle-ttl` (default 15 min; the bridge retires adapters 5 min after session end, so 15 min keeps the
  index warm across a review round) the daemon removes its socket and pid files and exits 0. It also
  exits when the repo root disappears or when it receives SIGTERM/SIGINT (removing its files first).
- **Refusal.** A daemon serves exactly the config it was started with. The shim connects only to the
  socket named by the identical key, so a mismatch is impossible by construction; the daemon still
  validates `--repo` on start and refuses to bind a socket for a different key as defense in depth.

### 4.3 Socket, lock and log layout

- Runtime dir: `dirs::runtime_dir()` (Linux `$XDG_RUNTIME_DIR`) else `std::env::temp_dir()` (macOS
  `$TMPDIR`, per-user, ~50 chars), joined with `prism-mcpd/`. Created 0700.
- Files: `<key16>.sock`, `<key16>.lock`, `<key16>.pid`, `<key16>.log` where `key16` is the first 16 hex
  chars of the identity key. macOS caps `sun_path` at 104 bytes; `$TMPDIR` + `prism-mcpd/` + 21 chars
  fits with margin, and the shim asserts the length and falls back to standalone if it does not.
- Start lock: `flock(LOCK_EX)` on `<key16>.lock` held by the shim only while checking/starting; the daemon
  never holds it. A crashed shim releases it automatically.
- Log: daemon stderr. Rotated by truncation when it exceeds 1 MB at daemon start.

### 4.4 Transport refactor

`serve_runtime(runtime, registry, transport)` keeps its signature for stdio and tests. A new
`serve_shared(provider: &Mutex<LazySessionProvider>, registry, transport)` implements the §4.2 loop.
`UnixStreamTransport` wraps `std::os::unix::net::UnixStream` with the same newline framing, EOF and
bad-UTF-8 handling as `StdioTransport` (shared code path, not a copy). No change to `handle_message`,
`Lifecycle`, tools or output.

### 4.5 Lifecycle summary

| Event | Behavior |
|---|---|
| First client for a key | Shim starts the daemon under the lock; later clients connect directly |
| Client exits | Connection drops; counter decrements; daemon stays up until idle TTL |
| Last client gone + TTL | Daemon removes socket/pid, exits 0 |
| Daemon crashes | Every attached client sees EOF (same as a crash today); next shim removes the stale socket under the lock and starts a fresh daemon |
| New prism binary installed | New key, new daemon; old daemon idles out |
| Repo deleted | Daemon exits; shims fall back to standalone and fail as today |
| Socket dir unwritable / path too long | Shim falls back to standalone with one stderr line |

### 4.6 Fallback contract

Shared mode never changes observable MCP semantics. Every response a client receives from the daemon
is byte-identical to what an in-process server with the same config would return, except for the
`generation` counter and freshness metadata, which now reflect refreshes performed by any client (§5).

## 5. Semantics preserved and changed

- **Preserved:** the `initialize` instructions, the warming result and `--first-call-wait` budget (per
  call, computed from the shared build's start), tool schemas, output shaping, `refresh_index`
  verification, stale-index warnings.
- **Changed, by design:** the index and its `generation` are per repo, not per client. A
  `refresh_index` from one client is visible to all; stale-index warnings compare the working tree
  against the shared snapshot. This is the correct semantics for a repository that is one filesystem
  state.
- **Changed, documented:** `--first-call-wait` is a property of the one shared build, so the daemon uses
  the value passed by the shim that started it; later shims' values are ignored (logged once). It is
  deliberately not part of the identity key, because a different wait budget is not a reason to hold a
  second copy of the index.
- **Excluded from shared mode:** `--eager` (a synchronous pre-warm is a local act; it still writes the
  cache the daemon will load), `--no-cache`, and the owner runtime.

## 6. Memory measurement (C) and reduction candidates

The daemon removes duplication; this step shrinks the one copy. It is measurement-first: no reduction is
implemented until the numbers say where the memory is.

### 6.1 Harness

- `PRISM_MCP_MEM_REPORT=1 prism-mcp --repo X --eager < /dev/null` prints a JSON report to stderr with
  resident-set deltas at: after `load_repo` (sources + tree-sitter trees), after the CPG load or build,
  after the call-edge index, after the first `nav_repo_map`, plus `ru_maxrss` for the peak. RSS comes
  from `libc::getrusage` (and `proc_pidinfo` on macOS for the current value). This is a dev-only flag;
  no output changes without the env var.
- A second run with `PRISM_MCP_MEM_REPORT=1` and `--no-cache` separates cache-load transients from
  steady state.
- Corpora: slicing (self), `~/code/bench-repos/{ruff, excalidraw, prometheus, kubernetes}` — the
  Tier-A corpora already on disk, covering Rust, TypeScript, Go at three sizes.
- Each run records the commit, the cache version, and the binary's grammar fingerprint so the readout
  is reproducible.

### 6.2 Candidates, in the order the numbers are expected to rank them

| Id | Change | Expected effect (to be measured) | Performance risk |
|---|---|---|---|
| M2 | Stream the cache deserialization instead of `fs::read` + `bincode::deserialize` of the whole file | Removes the file-bytes transient (≈300–450 MB peak on slicing); no steady-state change | None; load time may improve |
| M1 | Drop retained tree-sitter `Tree`s after the index is built; reparse a file on demand behind an accessor (`ParsedFile::tree()` as a `OnceCell`) | Steady-state saving proportional to node count; trees are the likely largest non-CPG component | Reparse cost per touched file on a query (sub-ms to a few ms per file); 204 use sites need the accessor, mostly in build-time code |
| M3 | Intern repeated strings in CPG node payloads (paths, symbol names, kinds) | Depends on duplication ratio; typical CPGs save 20–40 % of node memory | None for queries; interning cost at build |

Decision rule: implement a candidate only if the readout shows it saves at least 15 % of steady-state
RSS on two of the four corpora, or at least 30 % of the peak for M2. The gate for each implemented
candidate is **no regression** on `uv run tier-a --quick` and on a nav latency check
(`nav_callers`/`nav_callees`/`nav_ego_graph` p50 over the standard seeds, warm daemon).

### 6.3 Why not just the reductions

Even a 40 % smaller index duplicated across 29 processes is 20 GiB. Sharing is the first-order fix;
shrinking is the second.

## 7. Security and isolation

- Socket and lock live in a 0700 user directory; the socket is 0600. Only the same uid connects.
- No network listener, no authentication beyond filesystem permissions, no code execution; the daemon
  is as read-only as the in-process server.
- The identity key pins the repo root, so a shim can never be routed to another repository's index.
- The daemon inherits the environment of the first shim. Shims from containers (the bridge's
  containerized agents) have their own filesystem and therefore their own daemon; nothing crosses the
  container boundary.

## 8. Configuration and rollout

1. **PR-A (opt-in):** `--shared` enables the shim path; default stays in-process. The project
   `.codex/config.toml`, the Claude plugin launcher and the bridge's `[[agents.mcp]]` entry add `--shared`.
   Soak for one bridge review round and one week of interactive use.
2. **PR-B (default flip):** shared becomes the default; `--standalone` opts out. Docs and the plugin
   launcher drop the explicit flag.
3. `--idle-ttl <SECS>` (default 900), `--daemon-start-timeout <SECS>` (default 10), `--warm-at-startup`
   (forwarded to a daemon this shim starts), `prism-mcp daemon ...` (internal; documented as such),
   `prism-mcp daemon-status --repo X` (prints key, socket, pid, uptime, clients, generation — the
   operator view the bridge incident lacked).

## 9. Testing

- Unit: identity key stability and sensitivity (each input changes it); socket path length check;
  `UnixStreamTransport` framing shares the stdio tests via the `Transport` trait.
- Integration (`tests/mcp/`, `assert_cmd` + real sockets in a temp runtime dir via an env override
  `PRISM_MCP_RUNTIME_DIR`):
  1. Two shims, one daemon: both handshake, the second client's first `tools/call` is served from the
     index the first client built (daemon `attempts() == 1`, observed through `daemon-status`).
  2. Idle exit: with `--idle-ttl 1`, the daemon's socket disappears after the last shim exits.
  3. Stale socket recovery: a dead socket file is replaced and a new daemon started.
  4. Fallback: an unwritable runtime dir makes the shim serve in-process and the responses are
     byte-identical to standalone mode.
  5. Refresh visibility: `refresh_index` from client A raises the generation seen by client B.
  6. Concurrency: N shims issuing calls in parallel all receive valid, complete responses.
  7. Start race: N shims started simultaneously produce exactly one daemon.
- Soak: one bridge review round with `--shared`, recording `prism-mcp` count and resident memory
  before/after (the 10-03 incident's own probe commands), plus Tier-A quick and the nav latency check.

## 10. Error handling matrix

| Failure | Where | Result |
|---|---|---|
| Lock busy beyond start timeout | Shim | Fallback to in-process, one stderr line |
| Daemon spawn fails (exec error) | Shim | Fallback |
| Daemon binds but dies before first byte | Shim | Fallback |
| Socket error after first byte | Shim | Exit non-zero (client sees a dead server) |
| Client sends invalid JSON | Daemon | Same `-32700/-32600` responses as stdio; connection stays |
| `handle_message` panics | Daemon | Thread-level `catch_unwind`; that connection closes; daemon keeps serving others |
| Repo root removed | Daemon | Exit; files removed |

## 11. Open decisions for the owner

1. Idle TTL default: 15 min (proposed) vs. the bridge's 5 min adapter retirement.
2. Rollout: opt-in first (proposed) vs. default-on from PR-A.
3. Whether `daemon-status` should also expose a `shutdown` verb for the bridge's reaper (proposed: yes,
   same-uid only, graceful).

## 12. Slicing

| PR | Content | Depends on |
|---|---|---|
| A1 | Transport refactor (`UnixStreamTransport`, `serve_shared`), identity key, daemon command, idle exit; tests 1,2,5,6 | build-on-first-call merged |
| A2 | Shim: connect/lock/spawn/fallback/pump; `--shared`, `daemon-status`; tests 3,4,7; docs; config and plugin flags | A1 |
| C1 | `PRISM_MCP_MEM_REPORT` harness + the four-corpus readout committed under `docs/analysis/` | none (can run in parallel with A1) |
| C2 | The reductions the readout selects, each with its own tier-a and latency gate | C1 |
| B | Default flip to shared after the soak | A2 soak |
