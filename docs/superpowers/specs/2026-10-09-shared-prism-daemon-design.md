# Shared per-repo prism daemon — design

Date: 2026-10-09 · Status: **v3 for owner review** (two gpt-6-astra rounds at the declared cap; round 2 converged to
closed corrections, all folded; see §13) · Owner decision 2026-10-09: approach **A** (per-repo daemon + stdio shim)
with the **C** memory measurement folded in as a first-class step. Predecessor: #356 (merged, main cb0eeec0; this
branch is rebased on it) made `StartupMode::Lazy` defer the index build to the first valid `tools/call`;
`--warm-at-startup` restores the at-spawn build.

## 1. Problem and goal

Every MCP client process spawns its own `prism-mcp`, and every `prism-mcp` that answers a tool call holds the whole
repository index in memory. On this machine the clients multiply:

- codex app-server starts the project's MCP servers **per thread**; codex-acp adds an ephemeral title-generation
  thread per new session; the a2a-bridge fans out reviewers and implementers.
- Claude Code starts one per session through the plugin.
- Standalone `codex exec` runs in the repo start one each.

Measured on the slicing repo (1,232 files, 420k CPG nodes, 672k edges):

| Measurement | Value |
|---|---|
| Resident set of one idle `prism-mcp` after the index is built | 1,168 MB |
| Peak during a warm cache load (`--eager`) | 1,620 MB, 1.6 s wall |
| Handshake-only process after #356 (default) / with `--warm-at-startup` | 3.6 MB / 902 MB at 20 s |
| On-disk nav cache for the same repo | 304 MB CPG + 21 MB call-edge index |
| 2026-10-03 bridge incident | 29 `prism-mcp` under one codex app-server, 34 GiB compressed, swap full |

Goal, in the sense of the proportionality rule: let the bridge run its reviewer and implementer fan-out on this
host without swap exhaustion. Success is **one resident index per compatible (repository checkout, prism build,
cache configuration, refresh policy)**, shared by every agent that navigates it, with no regression in query
latency under fan-out, and a smaller index where measurement says it is cheap to get.

#356 already removes the cost for clients that never call a tool. This design removes the duplication for clients
that do.

## 2. Non-goals

- Serving several worktrees of one repository from one process. One canonical checkout root = one daemon.
  (Worktrees are the accepted compromise; a shared-common-dir design can come later.)
- A network or HTTP listener. The daemon speaks a private preamble followed by newline-delimited JSON-RPC over a
  Unix socket only. An HTTP listener can be added to the same daemon later; nothing here blocks it.
- Changing the experimental owner runtime (`--owner-*`). It stays in-process (`--standalone` is implied).
- Pinned per-client snapshots. The index is one repository state; refresh is visible to every client (§5).
- Isolation against a hostile same-uid process. The socket is same-user; it is not a security boundary.
- Sharing across mount namespaces or containers. The key is host- and mount-scoped (§4.3).
- Reducing the on-disk cache format. Measurement (§6) decides what the in-memory reductions are.

## 3. Architecture

```
codex thread A ──stdio──▶ prism-mcp --shared (shim) ──┐
codex thread B ──stdio──▶ prism-mcp --shared (shim) ──┼─unix socket─▶ prism-mcp daemon ──▶ one NavigationSession
claude session ──stdio──▶ prism-mcp --shared (shim) ──┘   (preamble, then MCP)   (per identity key)
```

- The **shim** is the existing `prism-mcp --repo X` binary in shared mode. After a private readiness handshake
  (§4.4) it is a byte pipe: stdin to the socket, socket to stdout. It holds no index and costs a few MB.
- The **daemon** is the same binary started as `prism-mcp daemon ...`. It owns one lazily built index, serves each
  admitted connection on its own thread with its own MCP `Lifecycle` and its own output settings, and retires after
  an idle TTL.
- The **identity key** (§4.3) names the socket, locks, pid and log files. It covers everything that changes the
  index content or the analyzer's answers; it deliberately excludes per-connection settings, which travel in the
  preamble.

### 3.1 Daemon state machine

```
Starting ──owner lock, bind, accept loop──▶ Ready ──idle TTL, no admitted or pending connections──▶ Draining ──▶ Stopped
    │ (lock/bind failure)                     │ (fatal panic, root identity lost, SIGTERM)             │
    └─────────────────────────────────────────┴───────────────────────────────────────────────────────┘ files removed only by the owner
```

- **Starting:** validate the runtime directory (§4.3), take the **owner lock** (`<key>.owner`, `flock(LOCK_EX|LOCK_NB)`;
  failure = another live daemon owns this key → exit 0 silently), bind the socket, write `<key>.pid`
  (`pid\nstart_time\nbuild_identity\n`), enter the accept loop. The owner lock is held for the daemon's whole life.
- **Ready:** connections are accepted into a bounded handshake stage and admitted on `READY` (§4.2, §4.4). The index
  builds on the first valid `tools/call` from any client (`StartupMode::Lazy`), or at start if the starting shim
  passed `--warm-at-startup`.
- **Draining:** entered only from the admission-and-retirement critical section (§4.2) when **admitted == 0 and
  pending == 0** for the TTL and no build is running. Under that same lock the daemon stops accepting, unlinks its
  socket **while still holding the owner lock**, and answers `RETIRING` to any handshake that races in before the
  unlink. A shim that nevertheless observes EOF, reset or broken pipe during its preamble treats it as transient
  and re-enters arbitration (§4.1 step 6), so retirement can never fail a fresh startup. Then the pid file is
  removed, the owner lock released, exit 0.
- **Fatal:** any panic inside provider dispatch, the build thread or the control plane is daemon-fatal: the process
  aborts, attached clients see EOF (exactly what a crash of an in-process server looks like today), and the next
  shim restarts the daemon through the ordinary start protocol. Loss **or change** of the root's filesystem
  identity (§4.3) and SIGTERM/SIGINT go through Draining.

## 4. Components

### 4.1 Shim (`prism-mcp --repo X --shared ...` ; `--standalone` is today's in-process server)

1. Resolve `ServerConfig` exactly as today (`canonical_config`), plus the **effective cache location**: the cache
   API treats `--cache-dir` as a base and appends a per-repository suffix, so the shim resolves `CacheMode::Default`
   and an explicit base to the same canonical base directory before the final directory exists.
2. Shared mode requires a cache-backed, non-eager, non-owner config. `--eager`, `--no-cache` and `--owner-*` are
   served in-process and `--shared` with any of them is a CLI error, not a silent downgrade.
3. Resolve the **connection settings** from this shim's own environment and flags: `PRISM_MCP_STRUCTURED_CONTENT`,
   `PRISM_MCP_CONCISE_SHAPE`, the result cap, `--first-call-wait`, and `--warm-at-startup`. They are sent in the
   preamble, never inherited from whichever shim started the daemon.
4. Compute the identity key and paths (§4.3). Validate the runtime directory. Check the socket path length
   (≤ 103 bytes); if it does not fit, **fail closed** (step 7). Establish the **one absolute deadline**
   (`--daemon-start-timeout`, default 10 s) that bounds everything from here to `READY`.
5. Try `connect`; on success run the preamble (§4.4) and, on `READY`, go to 8. On `RETIRING`, `ECONNREFUSED`,
   `ENOENT`, **or EOF/reset/broken pipe before `READY`**, go to 6. Preamble-time failures are transient by
   definition: no MCP byte has been forwarded yet, so re-entering arbitration is always safe.
6. Take the **start lock** (`<key>.start`, `flock(LOCK_EX|LOCK_NB)` retried until the deadline). Under it, re-try
   step 5 (another shim may have won). If that fails, take the **owner lock** exclusively and non-blocking:
   - **Acquired:** no daemon owns the key. Holding it, unlink any stale socket and pid file, then **release it** and
     spawn the daemon detached (own session via `setsid`, stdin null, stdout and stderr to `<key>.log`,
     close-on-exec on every lock descriptor so the child never inherits either lock, the shim's environment). The
     child acquires the owner lock itself. If two children exist because an earlier starter died mid-spawn, the one
     that loses the owner lock exits 0 silently; whichever wins is the daemon. The inspection-and-unlink happens
     entirely under the owner lock, so a daemon that is about to bind can never have its live socket unlinked.
   - **Held:** a daemon is alive. It may be Starting (keep retrying step 5 until the deadline) or wedged (preamble
     times out at the deadline → fail closed naming the pid and start time from `<key>.pid`). A socket whose owner
     lock is held is **never** unlinked.
   Keep the start lock until `READY` or the deadline.
7. **Fail closed.** If no daemon reached `READY` by the deadline, the shim prints one actionable line to stderr
   (key, runtime dir, socket path, pid file contents, log path, and the exact `--standalone` invocation) and exits
   non-zero. It never silently builds a private index: under fan-out, N silent fallbacks are the incident this
   design exists to prevent. The only way to get a private index is to ask for one with `--standalone`.
8. Pump bytes both ways. Stdin EOF: half-close the socket, drain pending responses, exit 0. Socket EOF or error
   while the client is still open: exit non-zero (the client sees the server die, as it would today).
9. **Retry boundary.** The preamble is the only point at which the shim retries against a different daemon. Once a
   single byte of MCP traffic has been forwarded in either direction, the shim never replays or reconnects; a lost
   daemon is a lost session.

The handshake latency budget is unaffected: a Ready daemon answers the preamble and `initialize` from a thread that
never takes the provider lock (§4.2), so codex's `startup_timeout_sec` (60 s in the project config, 120 s in the
bridge) is never approached by another client's build or refresh.

### 4.2 Daemon (`prism-mcp daemon --repo X --cache-dir D --refresh-policy P --idle-ttl S`)

Two independent synchronization domains. Nothing in the control plane ever waits on the provider.

- **Control plane** (`Mutex<Control>` + `Condvar`): pending-handshake count, admitted-connection count,
  last-activity instant, retirement decision, and the **readiness coordinator** for the index build
  (`Idle | Building{attempt, started} | Ready{generation} | Failed{attempt, error}`). The build runs on one thread
  and publishes the provider into the provider slot **before** announcing `Ready`. Lock ordering: a thread holding
  Control never waits on the provider; a thread holding the provider may briefly take Control to read state.
- **Readiness semantics (per connection and per build attempt):** a connection's deadline for attempt *n* is set
  when its first readiness-requiring call arrives during attempt *n* and is cumulative for that attempt: every later
  call during the same attempt waits only the remaining budget and then returns `index warming` promptly. A
  warming response is a wait outcome for that connection; it neither cancels the build nor changes any other
  connection's deadline, and it is not a state transition. When attempt *n* terminates, its outcome (Ready or
  Failed) is delivered to every connection waiting on it. After Failed, the next readiness-requiring call from any
  connection starts exactly one retry (attempt *n+1*) with fresh per-connection budgets. Worked example: A and B
  join attempt 1 with budgets 20 s and 60 s; A expires at 20 s and gets warming; B keeps waiting; publication at
  35 s serves B directly and A's next call immediately.
- **Provider** (`RwLock<Option<SessionProvider>>`, empty until the first publication): queries take the read lock
  and operate on a **read-only facade** of the whole `SessionProvider`, not a narrower tuple, so they see the
  session, the freshness probe, the generation, **the sticky `known_stale_after_refresh` evidence** and the refresh
  policy with exactly today's precedence (stored verification evidence over a fresh stamp probe). `refresh_index`
  and automatic refresh take the write lock; automatic refresh re-checks its trigger after acquiring exclusive
  access, and every response is bound to the generation it actually queried. `handle_message` is split (§4.5) so
  lifecycle and control methods run with **no provider lock**, read-only tool dispatch runs under the read lock,
  and refresh runs under the write lock. The in-process server keeps `LazySessionProvider` and `SessionRuntime`
  unchanged; the daemon's coordinator is a new type introduced in S3a.
- **Per connection:** accepted sockets enter a **bounded handshake stage** (non-blocking, preamble line ≤ 4 KiB,
  preamble timeout); a worker thread, `UnixStreamTransport` (same framing as `StdioTransport`, 1 MiB frame limit
  retained), `Lifecycle` and `ConnectionSettings` are allocated only after admission. `--max-clients` (default 64)
  bounds **pending + admitted**; beyond it, new connections are answered `BUSY` without allocating a worker. Reads
  and writes happen outside every lock; a slow reader can hold at most one bounded response in flight.
- **Admission and retirement:** one critical section. Admission increments the count before `READY` is written and
  decrements it if the write fails. Retirement requires admitted == 0 **and** pending == 0 for `--idle-ttl`
  (default 15 min; see §11) and no build in progress; it stops accepting and unlinks the socket inside the same
  critical section, so an admission and a retirement cannot interleave, and any handshake that was pending is
  answered `RETIRING` before exit.
- **Refusal:** a daemon serves exactly the identity it was started with; the preamble compares the full identity,
  not the 16-hex locator (§4.4).
- **Panic policy:** a panic anywhere in provider dispatch, the build thread or the control plane aborts the daemon
  (§3.1 Fatal); no lock is ever unwrapped past poisoning. Panics in a connection's framing or output shaping
  (outside both domains) close that connection only; the implementation marks these boundaries explicitly.

### 4.3 Identity key, runtime directory and files

- **Identity key** = SHA-256 over a length-prefixed, lossless encoding of: canonical repo root (bytes), the root's
  filesystem identity (`st_dev`, `st_ino`), `CARGO_PKG_VERSION`, `GRAMMAR_FINGERPRINT`,
  **`PRISM_CACHE_BUILD_IDENTITY`** (the analyzer build identity the CPG cache already checks, `src/cpg_cache.rs`),
  the **shim–daemon protocol version**, the effective cache base directory (canonical, §4.1 step 1), and
  `--refresh-policy`. Excluded by design: `--first-call-wait`, `--warm-at-startup`, output settings (all
  connection-local). The invariant is therefore "one index per compatible configuration and build", which is what
  §1 promises.
- **Scope of the key:** `st_dev` is the mounted device number, not a persistent volume id, so a volume remount
  that renumbers the device yields a new key and a new daemon (acceptable: the old one retires when its root's
  identity no longer matches, see below). A container with a different device view gets its own daemon (§2); a
  sandbox that sees the same path, device, inode and runtime directory shares (intended). The daemon re-checks its
  root's `(st_dev, st_ino)` on every admission and on the TTL tick; loss **or change** enters Draining — existence
  of the pathname alone is not enough.
- **Runtime directory:** `$PRISM_MCP_RUNTIME_DIR` if set (the test and bridge override), else `dirs::runtime_dir()`
  (Linux `$XDG_RUNTIME_DIR`), else `std::env::temp_dir()` (macOS `$TMPDIR`), joined with `prism-mcpd/`. On every
  use the shim and daemon **validate** it: owned by the current uid, mode 0700, a real directory (not a symlink),
  created 0700 if absent. Every launcher that should share a daemon must resolve the same directory; `docs/MCP.md`
  says so and `daemon-status` prints it.
- **Files:** `<key16>.sock`, `<key16>.start` (start arbitration), `<key16>.owner` (daemon lifetime), `<key16>.pid`,
  `<key16>.log`, with `key16` = first 16 hex chars of the key. Lock files are created 0600 and never unlinked
  (stable inodes; unlinking a lock file is how two holders come to exist). The socket path must be ≤ 103 bytes
  (`sun_path` is 104 on Darwin). The log is truncated at daemon start when over 1 MB and additionally capped during
  the daemon's life (the writer stops appending past 8 MB and notes the cap once).

### 4.4 Preamble (private, versioned, one line each way, before any MCP byte)

```
shim  → daemon:  HELLO {"protocol":1,"identity":"<full hex key>","settings":{"structured_content":"always|omit-default-path","concise_shape":...,"result_cap":N,"first_call_wait_secs":N,"warm_at_startup":bool}}
daemon → shim:   READY {"protocol":1,"identity":"<full hex key>","pid":N,"start_time":N,"generation":N,"build":"idle|building|ready|failed"}
             or  RETIRING {}  |  BUSY {"max_clients":N}  |  MISMATCH {"expected":"<key>"}
```

- The daemon admits the connection (increments the count under the control lock) only when it writes `READY`; the
  shim forwards MCP bytes only after `READY`. `connect` succeeding is not readiness evidence; `READY` is.
- `MISMATCH` cannot happen with a correctly computed key, but the check is what makes the 16-hex filename a locator
  rather than proof of equality.
- The daemon checks the peer's uid (`getpeereid`) equals its own and drops the connection otherwise.
- The preamble is bounded by the shim's single deadline (§4.1 step 4) and by the daemon's handshake-stage timeout.
  A daemon that accepts but never answers is "live but wedged" and is reported, never replaced (§4.1 step 6).

### 4.5 Transport refactor

- `UnixStreamTransport` implements the existing `Transport` trait with the stdio framing code shared, not copied.
- `handle_message` is split into `handle_control(message, lifecycle, registry, settings)` (no runtime) and
  `dispatch_tool(message, view, settings)` where `view` is either the **read-only facade** of a `SessionProvider`
  (session, freshness probe, generation, sticky stale evidence, refresh policy, with today's precedence) or the
  exclusive refresh path. The in-process server composes both exactly as today through `SessionRuntime`, so
  standalone behavior is unchanged byte-for-byte against the post-#356 implementation (S1 ships with that proof
  over the existing transport test corpus).
- Output settings become an explicit `ConnectionSettings` argument instead of being read from the process
  environment inside `transport.rs`; the in-process server builds it from its own environment once (today's
  behavior), the daemon builds it per connection from the preamble.

### 4.6 Lifecycle summary

| Event | Behavior |
|---|---|
| First client for a key | Shim starts the daemon under the start lock; others wait on the lock, then connect |
| Starter crashes mid-start | Start lock releases with its descriptor. If it had spawned a child, that child either already owns the key (next shim sees the owner lock held and waits for `READY`) or loses the owner lock to the next starter's child and exits 0; both outcomes leave exactly one daemon |
| Daemon dies between bind and first accept | Connect fails, or succeeds into the backlog and the preamble sees EOF; both re-enter arbitration; the owner lock is free, so the socket is replaced |
| Retirement races a connecting shim | Pending handshakes get `RETIRING`; a shim that sees EOF instead re-enters arbitration within its original deadline |
| Client exits | Connection drops; admitted count decrements; daemon stays up until idle TTL |
| Last client gone + TTL, no build running, nothing pending | Draining → files removed by the owner → exit 0 |
| Daemon crashes | Attached clients see EOF; next shim finds the owner lock free, unlinks the stale socket, restarts |
| New prism build installed | New key, new daemon; the old daemon retires when its last client leaves (overlap is bounded only by client lifetime; `daemon-status` lists every live daemon with its build identity) |
| Repo root renamed / replaced / remounted with a new device number | Filesystem identity is in the key; the old daemon detects the identity change and drains; already attached shims get EOF (they cannot switch an established MCP session) |
| User logs out | Nothing special: the daemon is per-user, not per-login-session; it retires on idle like any other time |
| Socket dir unwritable / path too long / runtime dir fails validation | Shim fails closed with the diagnostic (§4.1 step 7) |

### 4.7 Operator surface

`prism-mcp daemon-status [--repo X] [--runtime-dir D]` prints, for every live daemon in the runtime dir (or the one
for `X`): key, repo root, build identity, pid, start time, admitted and pending clients, build state and attempt,
generation, idle time, and whether its socket answers the preamble. `prism-mcp daemon-shutdown --repo X` asks the
instance verified through the live control exchange (pid + start time echoed in `READY`) to enter Draining when its
admitted count reaches 0; it never disconnects other clients (§11 decision 3). The integration tests use a
test-only status probe over the same control exchange (S3a) and do not depend on the operator tool (S5b).

## 5. Semantics preserved and changed

- **Preserved, for the same indexed snapshot, the same connection settings and the same readiness state:** every
  tool's result schema and content, the warming result and retry contract, `refresh_index` verification and its
  sticky stale evidence, stale-index warnings, and the `initialize` instructions (the daemon advertises Lazy or
  Background according to how its index was started).
- **Changed, by design:** the index, its `generation`, and freshness evidence are per daemon, not per client. A
  `refresh_index` from client A changes what client B sees next (B sees the refreshed names, where an in-process
  warn-only server would have returned the old names with a stale warning). A build started by A's first call is
  the build B waits on. These are the semantics of one repository state, and they are stated rather than hidden
  behind a byte-identical claim.
- **Connection-local, never shared:** output settings, result cap, `--first-call-wait` budget (per connection and
  per build attempt, §4.2).
- **Excluded from shared mode:** `--eager`, `--no-cache`, and the owner runtime (CLI error with `--shared`).

## 6. Memory measurement (C) and reduction candidates

Sharing removes duplication; this step shrinks the one copy. It is measurement-first and the harness is **coarse
triage**: it ranks candidates for a candidate-specific experiment, it does not by itself select one.

### 6.1 Harness

- `PRISM_MCP_MEM_REPORT=1 prism-mcp --repo X --eager --mem-workload W < /dev/null` prints a JSON report to stderr
  with, at each stage, current RSS, `TASK_VM_INFO.phys_footprint`, `TASK_VM_INFO.compressed` (macOS) or
  `/proc/self/status` RSS (Linux), and `ru_maxrss` at the end. Stages: after `load_repo` (sources + trees), after
  the CPG load or build, after the call-edge index, and after the workload `W` (a fixed list of
  `nav_callers`/`nav_callees`/`nav_ego_graph`/`nav_nodes_at` seeds per corpus, run through the in-process
  transport so the report covers query-time allocations, not only load).
- Four runs per corpus: warm disk cache, cold uncached build (`--no-cache`), the workload, and one `refresh_index`
  (for the refresh peak). Each run three times on the same host state; the report records min and median.
- **Component attribution** independent of RSS deltas: total source bytes, tree-sitter node count × per-node size
  (from `ParsedFile.parse_node_count`; an estimate, not an allocation measurement), CPG node/edge counts and the
  string duplication ratio over node payloads (unique bytes / total bytes). These rank M1 against M3; the
  candidate experiment then measures the actual saving before/after.
- Corpora (**five**): slicing (self), `~/code/bench-repos/{ruff, excalidraw, prometheus, kubernetes}` — Rust,
  TypeScript, Go at three sizes. Each run records commit, dirty state, cache version, grammar fingerprint, binary
  build identity, allocator, and host compression/swap state at the time.
- **Experiment parameters are pinned before a candidate experiment runs:** the footprint metric
  (`phys_footprint`), the sampling window (after the workload, after a 5 s settle), repetitions (three, median
  reported with min/max), and noise treatment (a candidate must beat the baseline's max by its own min).

### 6.2 Candidates and what would select each

| Id | Change | Expected effect (to be measured) | What the experiment must show |
|---|---|---|---|
| M2 | Stream the cache deserialization instead of `fs::read` + `bincode::deserialize` of the whole file | Removes the file-bytes transient (≈300–450 MB of the 1,620 MB peak on slicing, i.e. 19–28 %) | Peak reduction on two corpora **and** load time not worse than today (streaming is not automatically free) |
| M1 | Drop retained tree-sitter trees after the index is built; reparse on demand from the **retained snapshot source** (never the live filesystem) behind a `OnceCell` accessor | Steady-state saving proportional to node count | Saving measured **after** the workload, so regrowth of the per-file tree cache is counted; query p50/p95 within the gate |
| M3 | Intern repeated strings in CPG node payloads | Depends on the measured duplication ratio | Saving on two corpora; build time within the gate |

Decision rule (a prioritization and candidate-advancement heuristic, not a shipping decision): advance a candidate
to its experiment if the readout shows ≥ 15 % of steady-state footprint **or** ≥ 200 MB absolute on at least two
of the five corpora; for M2, ≥ 15 % of peak or ≥ 200 MB on at least two corpora. Shipping gate for each candidate:
no regression on `uv run tier-a --quick`, on single-client nav p50, **and** on fan-out latency (8 shims, p95/p99 of
the workload, with one concurrent `refresh_index`), all measured baseline vs. candidate in the same environment.
The fan-out gate needs a working shim, so a candidate may **begin** after C1 but may **ship** only after S5a.

### 6.3 Why not just the reductions

Even a 40 % smaller index duplicated across 29 processes is 20 GiB. Sharing is the first-order fix; shrinking is
the second.

## 7. Security and isolation

- The runtime directory is validated on every use (owner uid, mode 0700, real directory); files are 0600; the
  daemon checks the peer uid on every accepted connection. This closes pre-existing-directory and symlink games by
  other users; it is not a boundary against a hostile same-uid process (non-goal).
- No network listener, no code execution; the daemon is as read-only as the in-process server.
- Bounded resources: `--max-clients` over pending + admitted connections, a bounded non-blocking handshake stage
  with a 4 KiB preamble limit and a timeout, one in-flight response per connection, the existing 1 MiB frame
  limit, and a capped log.
- The daemon inherits the environment of the shim that started it only for things that do not affect answers
  (locale, PATH); every answer-affecting setting is either in the key or in the per-connection preamble.
- Containers and sandboxes with a different filesystem view have their own daemon; nothing crosses that boundary.

## 8. Configuration and rollout

1. **Opt-in:** `--shared` enables the shim path; the default stays in-process. The project `.codex/config.toml`,
   the Claude plugin launcher and the bridge's `[[agents.mcp]]` entry add `--shared`. The bridge also sets
   `PRISM_MCP_RUNTIME_DIR` explicitly so its reviewers and the operator's shells agree on the directory.
2. **Soak acceptance (the gate for the default flip):** over one bridge review round plus one week of interactive
   use, record resident `prism-mcp` count and aggregate footprint (the 10-03 incident's own probe commands), shim
   fail-closed count (target 0), daemon restarts, and fan-out latency p95 vs. standalone. The flip does not wait for
   any memory reduction.
3. **Default flip:** `--shared` becomes the default; `--standalone` opts out. Docs and launchers drop the flag.
4. Flags: `--shared` / `--standalone`, `--idle-ttl <SECS>` (default 900), `--daemon-start-timeout <SECS>` (default
   10), `--max-clients <N>` (default 64), `--warm-at-startup` (forwarded in the preamble; honored only by the daemon
   this shim starts), `PRISM_MCP_RUNTIME_DIR`, `prism-mcp daemon ...` (internal), `daemon-status`, `daemon-shutdown`.

## 9. Testing

Deterministic constructions, not sleeps:

- Unit: identity key stability and sensitivity (each input, including build identity and protocol version,
  changes it; `--first-call-wait` does not); lossless encoding; cache-base normalization (Default vs. explicit
  equivalent, before the directory exists); socket path length; runtime-dir validation (owner/mode/symlink,
  injected `fs` errors); `UnixStreamTransport` shares the stdio framing tests through the `Transport` trait;
  preamble encode/decode and every reply variant; readiness coordinator: per-connection, per-attempt budgets,
  terminal-outcome delivery, single retry.
- Integration (`tests/mcp/`, real sockets under a temp `PRISM_MCP_RUNTIME_DIR`), each with its deterministic hook:
  1. Two shims, one build: builder blocked on a test barrier; the test status probe shows `building` attempt 1;
     release; B's first call is served from that build.
  2. Idle exit: injected clock; observe the disconnect, advance the clock past the TTL, assert Draining; one bounded
     real-clock smoke test alongside.
  3. Stale socket: bind a fixture socket with no owner lock, close its listener, then start the shim → replaced.
  4. Live-but-wedged owner: a fixture holds the owner lock and accepts without answering the preamble → the shim
     fails closed naming the pid, and the socket is **not** unlinked.
  5. Fail-closed: unwritable or symlinked runtime dir (injected) → non-zero exit, diagnostic line, no cache dir
     created anywhere.
  6. Refresh visibility: await A's completed refresh, then B's query returns the changed name and the new generation.
  7. Concurrency: barrier-released requests from N shims with correlated ids and expected payloads, plus one
     blocked writer; every response complete and matched.
  8. Start race: pause the winner at lock/spawn/readiness boundaries and release contenders; count daemons and
     builders, not pid files.
  9. Output settings: two shims with different `PRISM_MCP_STRUCTURED_CONTENT` get their own shapes from one daemon.
  10. Build mismatch: a daemon with a different build identity answers `MISMATCH`; the shim fails closed.
  11. Partial forwarding: daemon killed after one MCP byte → the shim exits non-zero without reconnecting.
  12. Panic policy: an injected panic in dispatch aborts the daemon; all attached shims see EOF; the next shim
      restarts it.
  13. Retirement race: a shim paused after `connect` and before `HELLO` while the daemon retires → the shim sees
      `RETIRING` or EOF, re-enters arbitration, and reaches `READY` on a fresh daemon within its original deadline.
  14. Sticky stale evidence: a refresh that verifies `Diverged` followed by a clean stamp probe → B's next query
      still carries the stale warning, and auto-refresh retries.
  15. Starter death: starter killed between spawning its child and the child taking the owner lock while a second
      shim starts → exactly one daemon survives and both shims reach `READY`.
  16. Pending bound: more than `--max-clients` connections stalled before `HELLO` → `BUSY` without worker allocation.
- Soak: §8 step 2.

## 10. Error handling matrix

| Failure | Where | Result |
|---|---|---|
| Start lock not acquired by the deadline | Shim | Fail closed, diagnostic |
| Daemon spawn fails (exec error) | Shim | Fail closed |
| Owner lock held but preamble times out | Shim | Fail closed naming the pid; socket left alone |
| EOF/reset/broken pipe before `READY` | Shim | Transient: re-enter arbitration within the same deadline |
| `RETIRING` / `BUSY` / `MISMATCH` | Shim | Retry from the lock step (RETIRING); fail closed with the reason (BUSY, MISMATCH) |
| Socket error after the first forwarded byte | Shim | Exit non-zero (dead server) |
| Client sends invalid JSON | Daemon | Same `-32700/-32600` responses as stdio; connection stays |
| Peer uid mismatch | Daemon | Connection dropped before the preamble reply |
| Panic in provider dispatch, build, or control plane | Daemon | Abort (fatal); clients see EOF |
| Panic in a connection's framing/output | Daemon | That connection closes |
| Root identity lost or changed | Daemon | Draining; attached shims see EOF |

## 11. Decisions for the owner (astra's recommendation, confirmed in both rounds, in italics)

1. Idle TTL default: 15 min. *Keep 15; residency after a bridge session ends is about 20 min only when the
   bridge's 5 min adapter retirement is what closes the last connection.*
2. Rollout: opt-in first with the §8 soak acceptance. *Opt-in; the lifecycle protocol needs multi-client evidence.*
3. `daemon-shutdown` verb for the bridge's reaper. *Yes, graceful and instance-bound, validated through the live
   control exchange; it must never disconnect other active clients.*

## 12. Slicing

| PR | Content | Depends on |
|---|---|---|
| S1 | Transport/framing refactor (`UnixStreamTransport`, `handle_control`/`dispatch_tool` split, read-only provider facade, `ConnectionSettings` explicit); standalone byte-for-byte proof against post-#356 | #356 |
| S2 | Identity key (with build identity + protocol version, filesystem identity, normalized cache base), runtime-dir validation, preamble encode/decode, connection-local settings | S1 |
| S3a | Readiness coordinator (per-connection, per-attempt budgets, publication before Ready, single retry), `RwLock<Option<SessionProvider>>`, test-only status probe; tests 1, 14 | S2 |
| S3b | Concurrent dispatch, refresh visibility, auto-refresh recheck, panic boundaries; tests 6, 7, 9, 12 | S3a |
| S4 | Daemon ownership (owner vs. start locks), bounded handshake stage, admission/retirement critical section, Draining, root-identity check; tests 2, 3, 4, 8, 13, 15, 16 | S3b |
| S5a | Shim: connect/arbitration/spawn/fail-closed/pump with the single deadline; tests 5, 10, 11 | S4 |
| S5b | `daemon-status`, `daemon-shutdown` | S5a |
| S6 | Opt-in launcher/config changes (`.codex/config.toml`, plugin launcher, bridge entry, `PRISM_MCP_RUNTIME_DIR`), docs, soak instrumentation | S5b |
| C1 | `PRISM_MCP_MEM_REPORT` harness + attribution counters + the five-corpus readout under `docs/analysis/` (uses the existing in-process transport; independent of S1 for the baseline, not of the provenance it records) | none |
| C2-M2 / C2-M1a / C2-M1b / C2-M3 | One PR per candidate the readout advances: M2 streaming; M1 accessor migration (no eviction yet); M1 eviction + reparse-from-snapshot; M3 interning. Experiment may begin after C1; **ship** only after S5a (fan-out gate) | C1; S5a to ship |
| B | Default flip after the §8 soak acceptance | S6 soak |

## 13. Review record

- **v1 (27be06dd) → round 1, gpt-6-astra, high reasoning, read-only: FIX.** WRONG/MATERIAL: F1 silent fallback
  defeats the goal → fail closed; F2 key omits the analyzer build identity → key + preamble identity check; F3
  env-derived output settings first-starter-wins → `ConnectionSettings` in the preamble, explicit in dispatch; F4
  one mutex lets a build wait block handshakes → two domains, control plane never waits on the provider,
  connection-local budget. WRONG/IMMATERIAL: F5 byte-identical contract → §5 restated. SMELL/MATERIAL: F6 lifecycle
  state machine; F7 panic policy → daemon-fatal; F8 measurement rigor → attribution, footprint metrics, repeated
  runs, workload-after measurement, five corpora, percentage-or-absolute rule. Plus the Q&A folds listed in v2.
- **v2 (33894146) → round 2 (declared cap): FIX, converging** (6 findings vs. 8, all closed and enumerable, none
  repeating). F1–F5, F7, F8 FOLDED; F6 PARTIAL. New: N1 WRONG/MATERIAL retirement can hand a pending preamble an
  EOF the shim did not recover from → §3.1/§4.1 step 5: preamble-time EOF/reset is transient and re-enters
  arbitration within the one deadline; pending counted in retirement; `RETIRING` to pending handshakes; test 13.
  N2 WRONG/MATERIAL the narrowed read view dropped sticky `known_stale_after_refresh` evidence → read-only facade
  of the whole provider with today's precedence; auto-refresh recheck; test 14. N3 SMELL/MATERIAL stale-socket
  cleanup needed continuous owner-lock protection → inspection and unlink under the owner lock, release before
  spawn, either child may win; §4.6 corrected; test 15. N4 SMELL/MATERIAL budget lifetime and failed-attempt
  delivery unspecified → per-connection, per-attempt cumulative deadline, terminal outcomes delivered, one retry,
  worked example. N5 SMELL/MATERIAL `--max-clients` ignored pending handshakes → bounded handshake stage, pending +
  admitted bound, 4 KiB preamble; test 16. N6 SMELL/IMMATERIAL slice dependencies → S3a/S3b, S5a/S5b, test-only
  status probe, C2 begin-vs-ship. Also folded: the coordinator is a new daemon type (standalone keeps
  `LazySessionProvider`), key scope under remount/container/sandbox and root-identity change detection, cache-base
  normalization, pinned experiment parameters, M2 two-corpus rule. Provenance: branch rebased onto main so it
  contains #356.
- **Convergence classification at the cap:** converging (each round fewer, smaller, non-repeating findings; all
  closed corrections). Folded and stopped per the convergence rule; a third round is the owner's call.
