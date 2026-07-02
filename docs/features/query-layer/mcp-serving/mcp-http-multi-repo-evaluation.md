# prism-mcp: HTTP transport + multi-repo serving — evaluation hand-off

**From:** the a2a-bridge project (`~/code/a2a-bridge`), 2026-06-09
**To:** prism maintainers — *to evaluate*, not a committed ask.
**Status:** a2a-bridge is shipping the stdio integration now (per-agent `[[agents.mcp]]`, dual-delivery).
This doc records the two prism-side enhancements that would let a2a-bridge drop a pile of glue, so you can
decide if/when they're worth it on prism's own roadmap.

---

## Why a2a-bridge is asking

a2a-bridge runs containerized ACP coding agents (claude / codex / kiro) across **many repos from one
long-lived server** (`a2a-bridge serve`; each inbound session carries its own working dir — "session_cwd",
ADR-0014). It wants to give those agents prism's slicing tools (`nav_nodes_at`, `nav_callers`, …) so reviews
and edits are CPG-aware.

The stdio integration works today, but it hits **two structural mismatches** with prism's current shape. Both
are dissolved by changes that live naturally in prism, not in the bridge.

### Mismatch 1 — three delivery channels instead of one

prism speaks **stdio JSON-RPC**. Of the three agents:

| agent | honors the ACP `mcpServers` param? | how the bridge must wire prism |
|---|---|---|
| claude (`claude-agent-acp`) | **yes** (advertises stdio) | ACP `session/new` param — clean, re-sent per session |
| codex (`codex-acp`) | **no** — advertises `mcpCapabilities:{http:true, sse:false}` | bridge must write `~/.codex/config.toml [mcp_servers]` |
| kiro  (`kiro-cli`)    | **no** — advertises `mcpCapabilities:{http:true}` | bridge must write `/root/.kiro/settings/mcp.json` |

codex and kiro **ignore stdio MCP over the ACP param** but both advertise **`http: true`**. So if prism spoke
**HTTP**, the bridge could hand all three agents the *same* server URL through the one ACP `mcpServers` seam and
delete the per-agent native-config renderers + per-session file mounts entirely.

### Mismatch 2 — one prism process is bound to one repo

`SessionProvider::bootstrap` (`src/mcp/session.rs`) canonicalizes `cfg.repo_root` and builds **one**
`NavigationSession { repo, index }` at startup. Every tool call is implicitly against that one repo. So serving
N repos = N prism processes, each launched with its own `--repo`. Under the bridge's `serve` (one container,
many session cwds, agent process started once) that's unworkable: the container can't know the future cwds at
spawn time, so it can only bake **one** `--repo`. (For the bridge's `run-workflow` / `implement` paths, which
*are* one-repo-per-invocation, the single-repo model is fine — those ship now. It's specifically the
many-repos-one-server case that needs multi-repo prism.)

---

## What prism already has going for it

Both asks are smaller than they sound because the current code is well-factored for them:

1. **`handle_message` is transport-agnostic.** (`src/mcp/transport.rs:104`)
   ```rust
   fn handle_message(message: &Value, session: &NavigationSession,
                     registry: &ToolRegistry, state: &mut Lifecycle) -> Dispatch
   ```
   It's a pure function from a JSON-RPC `Value` to `Dispatch::{Response(Value), NoResponse}`. The stdio loop in
   `serve_stdio` is just a framing wrapper around it. **An HTTP transport is a second wrapper around the same
   function** — no dispatch logic moves.

2. **The repo (`session`) is already a parameter, not a global.** `handle_message` takes `session:
   &NavigationSession` per call. Multi-repo is "**choose which session to pass**," not a rearchitecture of the
   tool layer.

3. **Per-repo caching already exists.** `NavigationIndex::build_cached_under(&repo, base)` (`session.rs`) keys a
   built index under a base dir. The bridge already relies on this (cold ~35 s → warm ~1.3 s on a named volume).
   A multi-repo server caching M repos under one base is the same primitive, M times.

4. **You already scoped the hard part.** The `session.rs` comment notes `NavigationSession` is `!Sync`
   ("MCP server dispatches single-threaded, spec §8; Send+Sync deferred to a future async transport, spec §17").
   So the concurrency constraint below is already on prism's radar.

---

## Ask 1 — an HTTP transport (MCP Streamable HTTP)

**Shape.** Add `serve_http(provider, registry, addr)` alongside `serve_stdio`, implementing the MCP
**Streamable HTTP** transport: a single endpoint that accepts JSON-RPC over `POST`, returns either a JSON
response or an SSE stream, and carries an `Mcp-Session-Id` header for session continuity. Each request body is
handed to the **existing `handle_message`**; the `Lifecycle` state that's currently a stack local becomes
per-HTTP-session state (keyed by `Mcp-Session-Id`).

**New surface.** An async HTTP server dep (`axum`/`hyper` + `tokio`) — prism's `Cargo.toml` is currently
synchronous (only `serde_json` on the MCP path). That's the main weight of this ask.

**Concurrency.** `handle_message` itself is sync and cheap; the CPG queries behind it run against a `!Sync`
`NavigationSession`. Simplest correct design: a **single-threaded dispatch actor** (one tokio task owning the
sessions, requests delivered over an mpsc channel) so the async server stays multi-threaded while CPG access
stays single-threaded — exactly the "async transport" your spec §17 anticipates. Making the nav layer `Sync`
is the alternative but a bigger change; the actor avoids it.

**Lower-effort alternative (no prism change):** run today's `prism-mcp` stdio behind a generic
**stdio↔HTTP MCP proxy** sidecar. Zero prism code. Downsides: an extra process/container to ship and supervise,
and it does **not** address Ask 2 (still one repo per stdio prism behind the proxy). Useful only if HTTP is
wanted but multi-repo isn't.

---

## Ask 2 — multi-repo serving (the one that actually unblocks `serve`)

**Shape.** Let one prism process answer queries for **any repo, chosen per request**, instead of one repo fixed
at startup. Two sub-decisions:

1. **Where the repo key rides.** Two viable options:
   - **Per-tool-call argument** — add an optional `repo` field to each nav tool's input schema (`src/mcp/input.rs`),
     defaulting to a configured root when absent. Most MCP-native; the client (agent) names the repo per call.
   - **Per-connection** — an HTTP path/query or header (`?repo=/abs/path`) selects the session for that
     connection. Cleaner if Ask 1 lands first; ties multi-repo to HTTP.
   The bridge can drive either; the per-tool-call form also benefits the existing stdio users.

2. **A repo→session registry.** Replace the single `SessionProvider` with a small **bounded cache** of
   `NavigationSession`s keyed by canonical repo root (LRU over, say, K repos), each built via the existing
   `build_cached_under`. `handle_message` already takes `session` per call, so the only change at the dispatch
   site is "resolve the session for this request's repo key, then call `handle_message` as today."

**Safety knobs prism would own** (the bridge can't enforce these from outside, so they belong here): an
**allowlist / root-prefix** for which repo paths may be opened, a **cap** on concurrently-resident CPGs (memory),
and canonicalization + symlink containment on the incoming repo key.

**Concurrency.** Same `!Sync` constraint as Ask 1 — the single-threaded dispatch actor owning the repo→session
cache resolves it cleanly (serialize CPG access; the cache lives inside the actor).

---

## Suggested sequencing (prism's call)

1. **Ask 2 alone, over stdio** — even keeping stdio, a multi-repo prism (repo as a tool arg + a bounded session
   cache) is independently useful and is the change that actually unblocks the bridge's `serve`. Smaller than
   Ask 1 (no new async/HTTP deps).
2. **Ask 1 (HTTP)** — collapses the bridge's three delivery channels to one and is the natural home for the
   single-threaded dispatch actor; do it second, reusing the actor from step 1.

If only one ever happens, **Ask 2 is the higher-leverage one** for a2a-bridge. Neither blocks the bridge's
current stdio shipment.

---

## a2a-bridge-side reference (what these would let the bridge delete)

- The native-config renderers (`render_codex_toml` / `render_kiro_json`) and their per-session file-write +
  `:ro` mount wiring — replaced by one ACP `mcpServers` entry pointing at prism's HTTP URL (needs Ask 1).
- The "capture the pre-spawn cwd into the SpawnFn so the baked `--repo` is the invocation's repo" workaround,
  and the resulting **`serve` limitation** (codex/kiro native MCP is single-repo under `serve`) — replaced by
  passing the per-session repo to a multi-repo prism (needs Ask 2).

Contact the a2a-bridge side (ADR-0028, `docs/superpowers/specs/2026-06-08-per-agent-mcp-design.md`) for the
exact integration points if/when this gets picked up.
