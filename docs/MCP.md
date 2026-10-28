# Prism MCP server — whole-repo navigation for coding agents

`prism-mcp` is a local **stdio MCP server** that exposes whole-repo code navigation (read-only
except `refresh_index`) to MCP-capable coding agents (Claude Code, Codex, Kiro, …). It serves a
Code Property Graph of **one** repository and answers structural questions an agent would
otherwise grep for — who calls a symbol, what it calls, what breaks if you change it, the
module dependency graph.

This is separate from the `slicing` diff-CLI (see the [README](../README.md)). Same engine, different
surface: the CLI slices a *diff*; the MCP server navigates a *whole repo*.

---

## Build

The MCP server lives behind the cargo `mcp` feature (the default build excludes it):

```bash
cargo build --release --bin prism-mcp --features mcp
# binary: target/release/prism-mcp
```

Verify the MCP smoke tests through the umbrella test target:

```bash
cargo test --features mcp --test mcp
```

## Install as a Claude Code plugin (recommended)

Prism ships a Claude Code **plugin** that pre-wires the MCP server to your current project and installs
both skills in one step — no manual `claude mcp add`, no skill symlinks.

```text
# 1. Build the server (once, from a prism checkout):
cargo build --release --bin prism-mcp --features mcp

# 2. In Claude Code, add this repo as a marketplace and install the plugin:
/plugin marketplace add shoedog/prism        # or: /plugin marketplace add /abs/path/to/prism
/plugin install prism@prism-dev
```

The plugin wires `prism-mcp --repo ${CLAUDE_PROJECT_DIR}` (so it navigates whatever project you're in)
through a launcher (`scripts/prism-mcp-launch.sh`) that finds the binary you built — in the plugin
checkout's `target/release/` or on `$PATH` — and prints a clear "build it" message if you skipped step 1.
The two skills (`prism-code-navigation`, `prism-code-slicing`) load automatically; approve the `prism` MCP
server when prompted.

> The plugin only handles **wiring** — it can't ship a compiled binary cross-platform, so step 1 (build)
> is required. The plugin source is this repo (`source: "./"` in `.claude-plugin/marketplace.json`); the
> manifest is `.claude-plugin/plugin.json`. For Codex/Kiro (no plugin system), use the manual config below.

## Run / arguments

```bash
prism-mcp --repo /abs/path/to/your/repo                              # serve one repo over stdio
prism-mcp --repo /abs/path/to/repo --cache-dir ~/.cache/prism-nav    # pin the nav-cache location
prism-mcp --repo /abs/path/to/repo --no-cache                        # disable the nav cache
prism-mcp --repo /abs/path/to/repo --eager < /dev/null               # build synchronously, then exit
```

| Flag | Required | Meaning |
|---|---|---|
| `--repo <PATH>` | yes | The repository this server instance navigates. **One repo per process** — pin an absolute path. |
| `--cache-dir <PATH>` | no | Where to store the per-repo navigation CPG cache (default: an OS cache dir, keyed by the canonical repo path). |
| `--no-cache` | no | Don't read/write the nav cache (rebuild from source whenever an index build runs). Conflicts with `--cache-dir`. |
| `--eager` | no | Build the repository index before answering `initialize`, preserving the historical synchronous startup. Use for explicit pre-warming; normally leave it off. |
| `--warm-at-startup` | no | Start the background index build when the process starts instead of on the first valid `tools/call`. Trades memory for a warm first call; every process that uses it holds the full index whether or not a tool is ever called. Conflicts with `--eager`. |
| `--first-call-wait <SECS>` | no | Maximum wait for the first valid `tools/call` while the background build finishes (default `20`, range `0`–`600`). `0` returns a warming result immediately. Accepted but ignored with `--eager`. |
| `--shared` | no | Serve through the shared per-repository daemon (a shim of a few MB) instead of an index of this process's own; opt-in. See [Shared daemon](#shared-daemon---shared-opt-in-macos-and-linux). |
| `--standalone` | no | An index of this process's own (the default). |

> **The first index build warms a cache.** A cold whole-repo CPG build can take seconds to a few minutes on a large repo;
> later builds on an unchanged tree are near-instant (the cache is keyed by the canonical repo path + a
> grammar fingerprint, and only re-indexes changed files). By default that build runs on the first valid
> `tools/call`, not at process start. If you want to pre-warm once, use either
> `prism-mcp --repo <REPO> --cache-dir <DIR> --eager < /dev/null` or
> `prism nav --cache-dir <DIR> repo-map --repo <REPO>` (the `--cache-dir` flag must precede `repo-map`).

---

## Add it to your agent manually (without the plugin)

Use this if you're not using the Claude Code plugin above, or you're on Codex/Kiro. Use **absolute
paths** everywhere — the agent launches the server from an arbitrary working directory. **One server
instance navigates one repo;** add another named instance (`prism-acme`, …) with a different `--repo`
to navigate another repo. (Multi-repo / HTTP transport is a roadmap item, not available yet.)

### Claude Code (manual — the plugin above does this for you)

```bash
# claude mcp add [options] <name> -- <command> [args...]
claude mcp add --transport stdio prism \
  -- /abs/path/to/prism/target/release/prism-mcp --repo /abs/path/to/your/repo
```

Verify with `claude mcp list`. The tools appear to the agent as `mcp__prism__nav_*` (the `prism` segment
is the server name you chose in the `add` command). The plugin's tools are named
`mcp__plugin_prism_prism__nav_*` instead. Claude Code may list either as deferred tools, which the agent
loads with ToolSearch before its first call.

### Codex

Codex reads MCP servers from `~/.codex/config.toml`:

```toml
[mcp_servers.prism]
command = "/abs/path/to/prism/target/release/prism-mcp"
args = ["--repo", "/abs/path/to/your/repo"]
```

Codex starts MCP servers in the session's working directory (`codex exec -C <dir>`), so a project config
can index whichever checkout Codex runs in instead of a fixed path. This repo's own `.codex/config.toml`
does that, which gives every worktree and clone its own index:

```toml
[mcp_servers.prism]
command = "/bin/sh"
args = ["-c", 'root=$(git rev-parse --show-toplevel) || exit 1; exec "$PRISM_MCP_BIN" --repo "$root" --shared']
startup_timeout_sec = 60
env = { PRISM_MCP_BIN = "/abs/path/to/prism/target/release/prism-mcp", PRISM_MCP_RUNTIME_DIR = "/tmp" }
```

A fixed `--repo` pointing at another checkout answers about that checkout's code, not the one under
review. Each checkout gets its own shared daemon (and nav cache); a daemon retires 15 minutes after its
last client. To override the server per run, override `command` as well as `args`: an args-only override
would be passed to `/bin/sh`.

### Kiro

Kiro reads MCP servers from its agent config (e.g. `~/.kiro/settings/mcp.json` or a project
`.kiro/settings/mcp.json`):

```json
{
  "mcpServers": {
    "prism": {
      "command": "/abs/path/to/prism/target/release/prism-mcp",
      "args": ["--repo", "/abs/path/to/your/repo"]
    }
  }
}
```

Kiro names the tools **bare** (`nav_repo_map`), not `mcp__prism__*`.

---

## Tools

Anonymous JS/TS callable owners may appear as `<anon@LINE:COL>` in callers/callees results, `taint_reaches` Variable.function and MCP Evidence variable descriptions. The identity uses a 1-based UTF-8 byte column. Symbol seeds reject it; file/line seeds select the enclosing named function. Anonymous invocation edges and contextual display labels remain follow-on work.
The earlier spelling was `<cb@LINE:COL>`.

The seven navigation tools plus `taint_reaches` are read-only. Six graph/evidence queries and
`taint_reaches` return a Prism `Evidence` JSON envelope; `nav_symbol_spans` returns its dedicated
coordinate-only `SymbolSpans` v1 result. `refresh_index` is the exception — it changes local server
state (not the repo) and returns a refresh summary.

| Tool | Answers | Seed |
|---|---|---|
| `nav_repo_map` | The whole-repo module dependency graph. | *(none — call first to orient)* |
| `nav_nodes_at` | What symbols/nodes are at `{file, line}`? | `{file, line}` (**1-indexed; exact line**) |
| `nav_symbol_spans` | What are this callable's exact outer/name/body spans, insertion anchors, and indentation? | symbol or location |
| `nav_callers` | Who calls this symbol / location? (*what breaks if I change X*) | symbol or location |
| `nav_callees` | What does this symbol / location call? (*what X depends on*) | symbol or location |
| `nav_ego_graph` | The local call/dependency graph around a seed. | symbol or location |
| `nav_module_deps` | Outbound module dependencies for one file. | `{file}` |
| `taint_reaches` | Forward taint reachability from a seed. (read-only, returns `Evidence`) | `sources[]`: symbol or location; optional `sinks[]`: symbol or location |
| `refresh_index` | Re-indexes the repo snapshot for this server session. (local state change, not read-only; returns a refresh summary) | *(none)* |

**Seeding.** Most tools accept either `{kind: "symbol", name: "X"}` (optionally `{file}` to disambiguate)
or a node returned by `nav_nodes_at`. `nav_nodes_at` is **exact-line** — if it returns empty, aim at the
symbol's *definition* or *call* line, not a blank/comment line.

**Symbol coordinates.** `nav_symbol_spans` is callable-only and read-only. Its UTF-8 byte offsets are
end-exclusive; lines are 1-indexed and inclusive. `symbol_span` includes a Python decorator wrapper,
while `name_span` and `body_span` point to the inner grammar nodes. `body_span` is the raw grammar body
(including braces/delimiters when that grammar does); null fields have deterministic entries in
`unavailable`. The result contains no source/body text and does not apply edits.

---

## Gotchas (read these — they defy reasonable assumptions)

- **Lines are 1-indexed.** `nav_nodes_at({file, line})` matches the exact line; an empty result usually
  means you aimed a line or two off the definition/call site.
- **One repo per server.** The server knows only the repo it was launched with (`--repo`). It cannot see
  sibling repos, dependencies outside the tree, or the standard library.
- **Graphs truncate.** `nav_ego_graph` / `nav_repo_map` cap at 50 items by default (`max_results`, up
  to 1000), with an 80 KB result byte cap. A truncated graph is a partial view, not the whole story —
  narrow the seed if you need completeness.
- **Scores carry resolution confidence, not certainty.** Scores start from resolution confidence
  (`1.0` exact, `0.6` name-only); callers/callees decay that by hop, so a lower score means
  farther-away exact evidence or weaker name-only evidence. Read the cited site before relying on
  any score below `1.0`. A warning like `N same-name receiver call site(s) with unknown receiver
  type across multiple owner types; not attributed as callers` means real callers may be missing:
  treat "no callers" plus that warning as *unknown*, not *none*.
- **Read-only.** The server never modifies the repo. It also never executes code.
- **Coordinates are snapshot-relative.** Apply `nav_symbol_spans` offsets only to the indexed snapshot;
  stale-session responses carry the same `StaleIndex` warning and metadata as other navigation tools.
- **Cold first call.** `initialize`, `ping`, and `tools/list` answer immediately and do not start the
  repository index build; a process that only handshakes and lists tools (an agent thread that never
  navigates) stays at a few MB instead of holding the whole index. The first valid `tools/call` starts
  the background build and waits up to `--first-call-wait` (20 seconds by default), then returns an
  error-marked `index warming` JSON result if the build is still running. Retry that same call shortly;
  no other action is needed, and later calls are fast. On a mid-size repo a warm nav cache loads in a
  second or two, so the first call completes inside the wait; very large repos can take 15–20 s even
  warm, so raise `--first-call-wait` or use `--warm-at-startup` there. `--warm-at-startup` starts the
  build at process start instead; use `--eager` only when a synchronous build is desirable, such as
  pre-warming a shared cache.

---

## Environment variables (experimental)

| Variable | Default | Meaning |
|---|---|---|
| `PRISM_MCP_MALLOC_SPACE_EFFICIENT` | `1` on macOS | On macOS, when `MallocSpaceEfficient` is absent, this variable being unset, empty or `1` re-execs `prism-mcp` with `MallocSpaceEfficient=1` before subcommand dispatch or threads start. `0` opts out; any other value warns once and keeps the default allocator settings. A caller's `MallocSpaceEfficient` always wins, even when empty. No allocator policy change on other operating systems. |
| `PRISM_MCP_MAX_RESULT_CHARS` | `80000` | Wire byte cap per tool result (floor `12000`). |
| `PRISM_MCP_STRUCTURED_CONTENT` | `omit-default-path` | `omit-default-path` (the default) drops `structuredContent` from the wire on the default (`canonical_json`) path — `content[0].text` already carries the identical JSON, so nothing is lost, only a redundant second copy (~31% of the result). Agent views (`format: agent_markdown` / `agent_json`) always keep `structuredContent`; it is their only canonical-Evidence carrier once `content_text` has been rewritten into prose. `always` opts back into repeating canonical Evidence in both `content[0].text` and `structuredContent` (the pre-2026-07-03 shape) for clients that read only `structuredContent`. |
| `PRISM_MCP_CONCISE_SHAPE` | `slim` | Shape of items in `Verbosity::Concise` results (Concise is the MCP default when a tool call omits `verbosity`). `slim` (the default) drops each item's `symbol` byte-offset/`ordinal` fields, drops the separate `location` field when it duplicates the symbol's file/line span, and omits `snippet` when null (instead of serializing `"snippet": null`). `Verbosity::Detailed` and agent views are never affected. `legacy` opts back into the pre-2026-07-03 item shape, byte-identical to historical output. |
| `PRISM_MCP_RUNTIME_DIR` | the user runtime or temporary directory | Base of the shared daemon's runtime directory (`<base>/prism-mcpd`). Every launcher that should share a daemon must resolve the same one. |
| `PRISM_MCP_SHARED` | unset | Read by the plugin's launcher (`scripts/prism-mcp-launch.sh`), not by `prism-mcp`: `1` adds `--shared`; unset, empty or `0` adds nothing; any other value stops the launcher. |

On macOS this allocator policy lets unload return freed memory and reduces loaded footprint;
one measurement found index loading about 10% slower (n=1). Re-exec preserves the PID, arguments,
environment and stdio; if restart fails, one stderr warning is emitted and startup continues.
Serving startup events report `malloc_space_efficient=1`, `0`, `unset` or `other` from the current
process environment.

The `slim` transform only touches `items`; it never reaches graph-carried results —
`nav_ego_graph` / `nav_repo_map` payloads live under `graph`, untouched by design.

Both trims are **on by default since 2026-07-03**, gated on an owner-approved live-verification
pass: three probes from `eval/adoption/goldens/probes.toml` run through real `claude -p` sessions
against this server — a bare default-path `nav_callers` call under `omit-default-path` (the wire
carried no `structuredContent`; the Claude Code host surfaced `content[0].text` and the model
reported the correct caller), a bare `nav_callees` call under `slim` (the model listed all callees
correctly from the slim items, preserving the Exact-vs-NameOnly distinction), and a `nav_nodes_at`
call with both flips combined (correct answer). Set the `always`/`legacy` values above to restore
the pre-flip wire shapes. The `initialize` response's `instructions` field states the
snapshot-freshness and agent-view notices once for the whole session; each tool description keeps
only a one-line pointer to it.

## Shared daemon (`--shared`, opt-in; macOS and Linux)

Every MCP client process starts its own `prism-mcp`, and every one that answers a tool call holds the
whole index (1–3 GiB on a mid-size repository). Codex starts one per thread and the a2a-bridge fans out
reviewers, so a busy host can hold dozens of copies of one index. With `--shared` the process is
instead a **shim** of a few MB: it connects to one **daemon** per repository checkout (more exactly, per
checkout, prism build, cache directory and refresh policy), starting it if none is running, and passes
the MCP session through byte for byte. The daemon builds the index on the first tool call from any
client, serves every client from it, and exits 15 minutes after its last client has gone.

The default is still today's in-process server (`--standalone`). `--shared` is opt-in until a soak has
shown it holds under real fan-out.

### Bridge fan-out across worktrees

The daemon key includes the canonical worktree root, prism build, cache directory and refresh
policy. N worktrees of one codebase therefore mean N daemons and N indexes; sharing helps clients
in the same worktree. Options for bridge fan-out (no implementation proposed here):

- **Share one worktree for one task.** Agents using the same configuration share one daemon and
  index. Coordinate edits and refreshes: every agent sees that worktree's changes, and concurrent
  writers lose the isolation that separate worktrees provide.
- **Cap concurrent worktrees or stagger tasks.** This bounds the number of resident indexes while
  preserving worktree isolation, at the cost of parallelism and possible cold starts. Release MCP
  clients so idle daemons can retire (default idle TTL: 900 s); use `daemon-shutdown` to reclaim
  earlier after clients finish. An attached client prevents idle retirement; shutdown drains after
  clients leave and keeps admitting until their count reaches zero.
- **Future design: share an index across worktrees at the same commit**, using a content-addressed
  key. This could remove duplicate indexes while retaining separate worktrees, but commit identity
  alone is insufficient: dirty and untracked files, generated files, configuration, dependency/type
  inputs, absolute paths and refresh invalidation can differ. The design would need to prove input
  equivalence, preserve worktree-specific paths and prevent one worktree's refresh from publishing
  another's snapshot before such sharing is safe.

Inspect today's fan-out with `prism-mcp daemon-status` (repository, key, build and client counts),
and the soak report's `max_daemons_per_repo` (the most simultaneous daemons for one canonical
worktree, across builds/configurations). Separate worktrees have separate repository rows; count
those rows too when assessing fan-out across one codebase.

### Opting in

- **Claude Code plugin:** set `PRISM_MCP_SHARED=1` in the environment Claude Code starts from (your
  shell profile). The plugin's launcher then adds `--shared`. Unset, empty or `0` changes nothing; any
  other value makes the launcher refuse to start the server, naming the variable, so that a typo never
  decides silently which mode a session gets. An explicit `--shared` or `--standalone` in the
  arguments wins over a valid value; a value the launcher does not accept is refused even then
  (the variable is checked first).
- **Manual Claude Code, Codex, Kiro:** add `"--shared"` to the server's arguments, for example in Codex:

  ```toml
  [mcp_servers.prism]
  command = "/abs/path/to/prism/target/release/prism-mcp"
  args = ["--repo", "/abs/path/to/your/repo", "--shared"]
  env = { PRISM_MCP_RUNTIME_DIR = "/tmp" }
  ```

  (`/tmp` suits a host with one user. On a shared host give each user a directory of their own,
  for example `$XDG_RUNTIME_DIR` or one under the home directory, in this example and the next.)
- **a2a-bridge:** add `"--shared"` to the `[[agents.mcp]]` arguments and set the runtime directory
  explicitly, so that every agent and your own shell agree on it:

  ```toml
  [[agents.mcp]]
  name = "prism"
  command = "/abs/path/to/prism/target/release/prism-mcp"
  args = ["--repo", "{cwd}", "--shared"]
  [[agents.mcp.env]]
  name = "PRISM_MCP_RUNTIME_DIR"
  value = "/tmp"
  ```

  Agents share a daemon only when their `--repo` resolves to the same checkout: each clone or worktree
  an implementer works in has a daemon of its own. A containerized agent has a daemon of its own
  inside its container; `--shared` saves nothing for a container that runs one agent.

`--shared` needs the nav cache and a lazy start: with `--no-cache`, `--eager` or `--owner-*` it is a
command-line error, never a quiet fallback.

### Every launcher must agree on the runtime directory, cache directory and refresh policy

The daemon's socket, locks, pid file and log live in `<runtime>/prism-mcpd/`, where `<runtime>` is
`$PRISM_MCP_RUNTIME_DIR` if set, else `$XDG_RUNTIME_DIR` (Linux), else the temporary directory
(`$TMPDIR` on macOS, `/tmp` on Linux). Two clients share a daemon only when they resolve the same
directory **and** pass the same `--cache-dir` (or none) and `--refresh-policy`, with the same prism
build. Claude Code passes its whole environment to the server, so its sessions use `$TMPDIR`; other
clients may start servers with a reduced environment. To be sure, set `PRISM_MCP_RUNTIME_DIR` to one
value in every launcher (and in the shell you run `daemon-status` from). A client that resolves
another directory does not fail: it starts a second daemon, with a second copy of the index.

The directory must be yours and private: owned by you, mode 0700, no access control list, and nothing
on its path that another user could replace. Two traps:

- `/tmp/prism-mcpd` is one name for every user of a Linux host without `$XDG_RUNTIME_DIR`; the second
  user's shims fail closed with a message naming `PRISM_MCP_RUNTIME_DIR`. Set it per user there.
- A `sudo -E prism-mcp --shared …` keeps your `PRISM_MCP_RUNTIME_DIR` or `$TMPDIR` and creates
  `prism-mcpd` owned by root; your own shims then fail closed ("owned by uid 0"). Remove that
  directory with `sudo rm -r`.

The socket path must fit in 103 bytes; a long `PRISM_MCP_RUNTIME_DIR` fails closed saying so.

### Flags

| Flag | Default | Meaning |
|---|---|---|
| `--shared` / `--standalone` | `--standalone` | Shim of the shared daemon / an index of this process's own. |
| `--daemon-start-timeout <SECS>` | `10` (1–600) | The one deadline for reaching a ready daemon, starting it included. |
| `--idle-ttl <SECS>` | `900` (1–86400) | Idle time before a daemon retires. **Only for a daemon this shim starts**; a shim that reaches a running daemon ignores it, silently. |
| `--max-clients <N>` | `64` (1–4096) | Connections a daemon admits at once. Same rule as `--idle-ttl`. |
| `--warm-at-startup` | off | Passed to a daemon this shim starts (it builds at once); otherwise ignored. |
| `--index-idle-secs <SECS>` | `0` (off) | Unload the whole index after tool-idle seconds, even with clients connected. Flag overrides `PRISM_MCP_INDEX_IDLE_SECS`, which overrides off. First-spawner policy, outside the daemon key. |
| `--index-idle-pressure-relief` (hidden) | off | Measurement A+C intent, forwarded to the daemon. Effective only with nonzero index idle on macOS; other cases report it unused. |
| `--first-call-wait`, `PRISM_MCP_STRUCTURED_CONTENT`, `PRISM_MCP_CONCISE_SHAPE`, `PRISM_MCP_MAX_RESULT_CHARS` | as standalone | Per connection: each shim's own values apply to its own session. |

Index idle stays **off by default** in both stdio and shared mode. Measurement runs explicitly
pass `--index-idle-secs 600`; default activation remains an owner decision at a measured readout.
Values must be unsigned integer seconds that fit the monotonic clock; invalid or overflowing
flag/env values fail before a daemon is started or reached. Explicit `0` overrides even an invalid
ambient value. The shim resolves once and always passes `--index-idle-secs S`, including zero,
to its child. Direct `prism-mcp daemon` accepts explicit options only and reads no idle env.

Index idle measures inactivity of tool work, separately from `--idle-ttl` (time with no clients
before the daemon exits). Unload preserves connected MCP sessions; the next tool call reloads
and serves through the daemon's normal build path. Neither setting changes daemon identity.
An existing daemon keeps its first spawner's policy. A differing shim writes exactly one stderr
line, for example:

```text
prism-mcp: shared daemon pid 4242 idle policy mismatch: effective index_idle_timeout_secs=600; requested index_idle_timeout_secs=0; keeping first-spawner policy
```

If relief differs, that same line also names effective and requested
`index_idle_pressure_relief`, comparing the request after relief applicability is applied.
Relief requested with idle off or outside macOS gets the unused note instead of a relief mismatch.
Equal applicable settings produce no mismatch line. This
diagnostic does not restart or kill the daemon, reconnect the session, or replay a request.

### What changes for a client

- **One index for everyone.** A `refresh_index` from one client changes what every client sees next,
  generation included; stale-index warnings and their evidence are the daemon's, not the session's.
- **One build.** The first tool call from any client starts the build, and every client waits on that
  build. While it runs a call is answered with a retryable result, as standalone: `status` `warming`,
  `_meta` `prism/retryable: true` and `prism/index_state: "warming"`; a failed build is
  `status: "build_failed"` with its `cause` and `prism/index_state: "failed"`, and the next call
  retries it. The daemon adds one answer of its own: `status: "indexing"` with `retry_after_secs: 5`,
  for a call whose own build attempt failed while a later attempt is already building; retry after
  that many seconds, or ignore it if a retry already succeeded.
- **The shim says which daemon it reached**, on its standard error:
  `prism-mcp: shared daemon pid <pid> (started <epoch seconds>, generation <g>) at <socket>; index <state>; index_idle_timeout_secs=<effective seconds>; index_idle_pressure_relief=<effective bool>`.
  These are the daemon's actual READY fields, including `unloaded`, rather than the shim's request.
- **A session ends with its daemon.** If the daemon stops while a client is attached (a termination
  signal, its repository root removed or replaced, its runtime files replaced, or a crash), the shim
  exits 1 with `prism-mcp: the shared daemon closed the connection (shared daemon pid <pid>); this
  session cannot continue, …`; it never reconnects once MCP bytes have flowed. Restart the client's
  MCP server (in Claude Code, `/mcp`); the next shim starts a fresh daemon.
- **A crash ends every attached session.** A panic in a query, a refresh, the build or the daemon's
  control plane aborts the daemon (macOS writes a crash report,
  `~/Library/Logs/DiagnosticReports/prism-mcp-<date>.ips`, for each). If the daemon died during its
  index build, shims of that configuration fail closed for 60 s from when the first of them noticed,
  rather than start a daemon per shim that would die the same way; the minute is counted on the system
  clock, so a clock set back after the sighting lengthens it.

### When a shim fails closed

A shim that cannot reach a ready daemon by its deadline never builds a private index (under fan-out,
that is the memory incident this mode exists to prevent). It prints one line and exits 1:

`prism-mcp: --shared failed closed, no index was built: <reason>; key <64 hex>; runtime dir <dir>; socket <path>; pid file <contents>; log <path>; to run without the shared daemon: <the --standalone command>`

| Reason begins | What to do |
|---|---|
| `runtime directory … is owned by uid …`, `has mode …`, `could be replaced by another user`, `has an access control list` | Fix the directory as the message says, or set `PRISM_MCP_RUNTIME_DIR` to a directory of your own. |
| `daemon socket path … is … bytes` | A shorter `PRISM_MCP_RUNTIME_DIR` (the path can be at most 103 bytes). |
| `the daemon answered BUSY` | More clients than `--max-clients`. Stop the daemon (`daemon-shutdown` or `kill -TERM`) and let a shim with a higher `--max-clients` start the next. |
| `the daemon answered MISMATCH`, `READY for another identity` | A daemon of another build or configuration answers at this key's socket; see `daemon-status`. |
| `the socket's peer is uid …` | Another user's process holds the socket. Nothing was sent to it. |
| `no daemon answered READY before the deadline`, `another shim held the start lock` | The daemon is slow to start or wedged: read its log (the line names it); raise `--daemon-start-timeout`; a wedged daemon is named by its pid file, stop it with `kill -TERM <pid>`. |
| `the daemon this shim started, pid …, ended before READY`, `cannot start the daemon` | Read the log. After a rebuild of prism, a shim started from the old binary cannot start a daemon (on macOS the new binary computes another key and refuses; on Linux the old file is gone): restart the client. |
| `the last daemon for this key … died during its index build` | Wait out the minute and read the log for the build's failure. |

### Operating daemons: `daemon-status`, `daemon-shutdown`

```bash
prism-mcp daemon-status [--repo R [--cache-dir D] [--refresh-policy P]] [--runtime-dir BASE] [--timeout S]
prism-mcp daemon-shutdown --repo R [--cache-dir D] [--refresh-policy P] [--runtime-dir BASE] [--pid N --start-time T] [--timeout S]
```

`--runtime-dir` is the base, as `PRISM_MCP_RUNTIME_DIR` gives it (the files are in its `prism-mcpd`);
without it both commands resolve the directory as a shim does. `--repo` with `--cache-dir` and
`--refresh-policy` names this build's daemon of that configuration.

- **`daemon-status`** prints the runtime directory, then for each daemon its key, repository,
  build, pid, start time, admitted and pending clients, build state, generation and idle time; a
  daemon that does not answer, through its locks ("alive but not answering", "being started"); what a
  dead daemon left; and in one line the keys that hold nothing but lock files and logs. It reads and
  removes nothing, and asking does not keep a daemon of this build alive. It exits 1 when some key
  could not be looked at.
  Index state (`idle`, `building`, `ready`, `failed`, `unloaded`) is separate from client state
  (`connected` or `clientless`). `idle` means no initial index has been built; `unloaded` means
  an index was disposed after tool-idle. The effective policy line shows
  `index_idle_timeout_secs` (zero disables) and `index_idle_pressure_relief` (a boolean).
  Status and shutdown need no idle setting, ignore idle env, and do not reset tool-idle.
  An incompatible/old control reply leaves policy unavailable and asks for the daemon's
  matching binary for status/shutdown; unknown values are never reported as zero/off.
- **`daemon-shutdown`** asks one daemon to drain once it has no client left. It never disconnects a
  client, and the daemon keeps admitting new clients until its count reaches 0. Exit 0: that
  instance no longer serves the key, or it has accepted and will drain after its clients (the
  message says how many), or it drained. Exit 1: it could not ask (alive but not accepting
  connections, at its client limit, silent), the reply to the request was lost (it may have been
  recorded, and cannot be withdrawn: see `daemon-status`), or the drain was not seen within
  `--timeout`. With `--pid` and `--start-time` (from the shim's `shared daemon pid …` line) it asks
  only that instance, and leaves a newer one alone.
- **Daemons of another build** (the key covers the build, so every rebuild of prism starts new
  daemons) are listed by `daemon-status` and retire on their own after their clients and their idle
  time. To stop one now, `kill -TERM <pid>`: it drains at once, ending its attached sessions. A daemon
  built before `daemon-status` existed restarts its idle time at every `daemon-status` that looks at
  it, so polling keeps it alive; stop it with `kill -TERM <pid>`.

**Files.** In `<runtime>/prism-mcpd/`, per key (its first 16 hex digits): `.sock`, `.start` and
`.owner` (locks, never removed while anything may use them), `.pid` (`pid`, start time, build), `.log`
(the daemon's lines: truncated at start when over 1 MiB, capped at 8 MiB) and, while a build runs,
`.build`. Each rebuild of prism leaves the lock files and log of the old key behind; `daemon-status`
counts them. With no `prism-mcp` process running the whole `prism-mcpd` directory can be removed.

**The daemon** is started by shims as `prism-mcp daemon --repo <root> --cache-dir <base>
--refresh-policy <p> --key <64 hex> --index-idle-secs S [--index-idle-pressure-relief]
[--warm-at-startup] [--idle-ttl S] [--max-clients N]`, in a
session of its own, from `/`, with the shim's environment (but not `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`).
It exits 0 when it has drained, or at once and silently when another daemon already owns the key; it
exits non-zero with one `Error:` line in its log when it cannot start. Its log says
`prism-mcp daemon: pid N serves <key16> for <root> at <socket>` and, when it drains,
`pid N stopped: <reason>`, the reason one of: idle for the idle TTL with no client and no build (no
session was attached); an operator's daemon-shutdown, with no client left (none attached); a
termination signal, the repository root was lost or replaced, or its socket or owner lock file was
replaced (attached sessions end, as above). A crash leaves no `stopped` line.

## Skills (the judgment layer)

The MCP server gives an agent the *access* (tools + data); the bundled **skills** give it the *judgment*
(when to use which tool, the orient→seed→expand workflow, output conventions, the gotchas above). They
follow the [Agent Skills standard](https://agentskills.io) and live in [`../skills/`](../skills/):

- **`prism-code-navigation`** — using these MCP nav tools effectively.
- **`prism-code-slicing`** — using the `slicing` diff-CLI effectively.

Install them for your agent:

- **Claude Code:** the [plugin](#install-as-a-claude-code-plugin-recommended) installs both skills (and
  wires the MCP). To install skills without the plugin, copy or symlink a skill directory into
  `~/.claude/skills/` (user-level) or `<repo>/.claude/skills/` (project-level) — e.g.
  `ln -s "$PWD/skills/prism-code-navigation" ~/.claude/skills/`.
- **Other providers:** point your skills loader at `skills/<name>/SKILL.md` (the standard layout).

A skill activates on its `description`; once active, the agent reads `SKILL.md` and applies the workflow.
