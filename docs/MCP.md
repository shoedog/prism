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
```

| Flag | Required | Meaning |
|---|---|---|
| `--repo <PATH>` | yes | The repository this server instance navigates. **One repo per process** — pin an absolute path. |
| `--cache-dir <PATH>` | no | Where to store the per-repo navigation CPG cache (default: an OS cache dir, keyed by the canonical repo path). |
| `--no-cache` | no | Don't read/write the nav cache (rebuild every start). Conflicts with `--cache-dir`. |

> **First start warms a cache.** A cold whole-repo CPG build can take ~30 s on a large repo; subsequent
> starts on an unchanged tree are near-instant (the cache is keyed by the canonical repo path + a grammar
> fingerprint, and only re-indexes changed files). If your agent host has a short MCP handshake timeout,
> **pre-warm once**: `prism-mcp --repo <REPO> --cache-dir <DIR> < /dev/null`.

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
is the server name you chose in the `add` command).

### Codex

Codex reads MCP servers from `~/.codex/config.toml`:

```toml
[mcp_servers.prism]
command = "/abs/path/to/prism/target/release/prism-mcp"
args = ["--repo", "/abs/path/to/your/repo"]
```

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

The six navigation tools plus `taint_reaches` are read-only and return a Prism `Evidence` JSON
envelope. `refresh_index` is the exception — it changes local server state (not the repo) and
returns a refresh summary instead of `Evidence`.

| Tool | Answers | Seed |
|---|---|---|
| `nav_repo_map` | The whole-repo module dependency graph. | *(none — call first to orient)* |
| `nav_nodes_at` | What symbols/nodes are at `{file, line}`? | `{file, line}` (**1-indexed; exact line**) |
| `nav_callers` | Who calls this symbol / location? (*what breaks if I change X*) | symbol or location |
| `nav_callees` | What does this symbol / location call? (*what X depends on*) | symbol or location |
| `nav_ego_graph` | The local call/dependency graph around a seed. | symbol or location |
| `nav_module_deps` | Outbound module dependencies for one file. | `{file}` |
| `taint_reaches` | Forward taint reachability from a seed. (read-only, returns `Evidence`) | `sources[]`: symbol or location; optional `sinks[]`: symbol or location |
| `refresh_index` | Re-indexes the repo snapshot for this server session. (local state change, not read-only; returns a refresh summary) | *(none)* |

**Seeding.** Most tools accept either `{kind: "symbol", name: "X"}` (optionally `{file}` to disambiguate)
or a node returned by `nav_nodes_at`. `nav_nodes_at` is **exact-line** — if it returns empty, aim at the
symbol's *definition* or *call* line, not a blank/comment line.

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
- **Cold first call.** If you didn't pre-warm and the first tool call stalls, the server is building the
  whole-repo CPG (~30 s on a large repo). It's fast after that.

---

## Environment variables (experimental)

| Variable | Default | Meaning |
|---|---|---|
| `PRISM_MCP_MAX_RESULT_CHARS` | `80000` | Wire byte cap per tool result (floor `12000`). |
| `PRISM_MCP_STRUCTURED_CONTENT` | `omit-default-path` | `omit-default-path` (the default) drops `structuredContent` from the wire on the default (`canonical_json`) path — `content[0].text` already carries the identical JSON, so nothing is lost, only a redundant second copy (~31% of the result). Agent views (`format: agent_markdown` / `agent_json`) always keep `structuredContent`; it is their only canonical-Evidence carrier once `content_text` has been rewritten into prose. `always` opts back into repeating canonical Evidence in both `content[0].text` and `structuredContent` (the pre-2026-07-03 shape) for clients that read only `structuredContent`. |
| `PRISM_MCP_CONCISE_SHAPE` | `slim` | Shape of items in `Verbosity::Concise` results (Concise is the MCP default when a tool call omits `verbosity`). `slim` (the default) drops each item's `symbol` byte-offset/`ordinal` fields, drops the separate `location` field when it duplicates the symbol's file/line span, and omits `snippet` when null (instead of serializing `"snippet": null`). `Verbosity::Detailed` and agent views are never affected. `legacy` opts back into the pre-2026-07-03 item shape, byte-identical to historical output. |

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
