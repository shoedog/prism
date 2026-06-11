---
name: prism-code-navigation
description: Navigate a codebase structurally with the prism-mcp tools (nav_repo_map, nav_nodes_at, nav_callers, nav_callees, nav_ego_graph, nav_module_deps) instead of grepping. Use when you need to know who calls a symbol, what a symbol calls, what breaks if you change it, the call/dependency graph around it, or the repo's module structure — i.e. "find callers/callees", "impact of changing X", "what depends on Y", "who uses Z", "module dependencies", "trace the call graph". Requires the prism MCP server (tools named mcp__prism__nav_* for Claude/Codex, bare nav_* for Kiro).
metadata:
  project: prism
  surface: mcp
---

# Navigating code with prism

Prism's MCP server answers **structural** questions over a Code Property Graph of the repo. Reach for it
instead of grep whenever the question is "who/what is connected to this symbol" rather than "where does
this text appear." It resolves calls and imports across files — grep can't.

## When to use it (vs grep / reading files)

| Question | Tool | Don't grep because |
|---|---|---|
| "Where am I? what's the module structure?" | `nav_repo_map` | grep can't build a dependency graph |
| "What's defined / called at this line?" | `nav_nodes_at` | resolves the actual symbol, not the text |
| "Who calls `X`? what breaks if I change it?" | `nav_callers` | finds call *sites* across files, not name matches |
| "What does `X` call / depend on?" | `nav_callees` | follows resolved edges, not string hits |
| "Show the local graph around `X`." | `nav_ego_graph` | one call gives the neighborhood |
| "What does this *file* import/depend on?" | `nav_module_deps` | module edges, not import-line text |

Use grep/Read when you want literal text, comments, strings, config values, or a symbol prism can't
resolve (see Gotchas).

## Workflow: orient → seed → expand → stop

1. **Orient** with `nav_repo_map` (no args) once, if you don't already know the layout.
2. **Seed** the symbol you care about. Two ways:
   - By symbol: `{kind: "symbol", name: "process_order"}` (add `{file}` to disambiguate a common name).
   - By location: `nav_nodes_at({file, line})` → take a node from the result and seed the graph tools with it.
3. **Expand** with exactly the tool that answers the question — `nav_callers` for impact-of-change,
   `nav_callees` for dependencies, `nav_ego_graph` for the neighborhood. One hop is usually enough.
4. **Stop.** The graph is the evidence; don't keep re-querying. Summarize the callers/callees you found
   with their `file:line`, then act (read the specific sites, make the change, etc.).

This is a *sharpening* step, not a goal. A couple of calls should answer most structural questions.

## Seeding mechanics

- **Lines are 1-indexed.** `nav_nodes_at` matches the **exact** line. Empty result ⇒ you're a line or two
  off — aim at the symbol's *definition* line or a *call* line, not a blank/comment/brace line.
- Prefer **symbol seeds** for "who calls this function"; prefer **location seeds** (`nav_nodes_at` first)
  for a specific variable or an overloaded/ambiguous name.
- Every tool returns a Prism **`Evidence`** JSON envelope: a list of evidence items, each with a
  `location` (`file`, `start_line`, `end_line`) and a symbol/reason. Read `location` to jump to the site.

## Gotchas

- **One repo only.** The server knows *only* the repo it was launched with (`--repo`). It cannot see
  sibling repos, third-party dependencies, or the standard library. Callers/callees outside the tree
  simply aren't there — that's not "no callers," it's "out of scope."
- **Graphs truncate at ~`max_results` (~200 nodes).** A returned `nav_ego_graph`/`nav_repo_map` may be a
  partial view. If it looks clipped, narrow the seed rather than trusting it as complete.
- **Call resolution is name-based, not type-based — treat it as high-recall, not precise for dispatch.**
  Prism resolves dot/`::`-qualified, `use`-imported, and free-function calls. The known gaps:
  `Type::method` where the type name differs from the file stem, and cross-file method/receiver calls
  (`obj.method()` where `obj`'s type lives elsewhere). For those, `callers`/`callees` may be incomplete
  or point at a same-named method on the wrong type. **Verify a method-dispatch edge by reading the site**
  before relying on it for a refactor.
- **Read-only.** These tools never modify or execute code. Make edits with your normal file tools.
- **Cold first call (~30 s).** If the first tool call stalls, the server is building the whole-repo CPG.
  It's near-instant afterward. (The operator can pre-warm; see `docs/MCP.md`.)
- **Don't ask it to slice a diff.** Diff-driven slicing is the *other* surface — the `slicing` CLI (see
  the `prism-code-slicing` skill). The MCP server navigates a whole repo; it does not take a patch.

## Example

> "What breaks if I change the signature of `build_cfg_edges`?"

1. `nav_callers({kind: "symbol", name: "build_cfg_edges"})`
2. Read the returned `file:line` call sites; each is a caller that must be updated.
3. If a caller is a method on a type whose file stem differs from the type name, open the site to
   confirm the edge (name-based resolution caveat) before counting it.
