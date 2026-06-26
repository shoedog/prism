---
name: prism-code-navigation
description: >-
  Navigate a codebase structurally with the prism-mcp tools (nav_repo_map, nav_nodes_at, nav_callers, nav_callees, nav_ego_graph, nav_module_deps) instead of grepping. ALWAYS use these — never grep/rg/Bash — for any question that names a symbol or file and asks who or what connects to it, INCLUDING quick ones that look grep-able: "who calls X", "list/find the call sites of X", "who uses X", "what does X call", "what functions does X call", "what does this file import / depend on", "what breaks if I change X", "the call/dependency graph around X", "the module structure". Grep is WRONG for these — it matches the definition itself, comments, strings, and same-named symbols in other files, and MISSES aliased / re-exported / renamed-import calls — whereas nav_* returns the *resolved* call/import edges. Reach for the tool even when a single grep looks sufficient. Requires the prism MCP server (tools named mcp__prism__nav_* for Claude/Codex, bare nav_* for Kiro).
metadata:
  project: prism
  surface: mcp
---

# Navigating code with prism

Prism's MCP server answers **structural** questions over a Code Property Graph of the repo. Reach for it
instead of grep whenever the question is "who/what is connected to this symbol" rather than "where does
this text appear." It resolves calls and imports across files — grep can't.

**Default to nav_*, not grep or Read, the moment a request names a symbol or file and asks who/what
connects to it — or what symbol is defined at a specific `file:line` — even a quick one-line lookup.**
These look one-grep-or-Read-able, but those give a wrong or shallow answer: grep matches the definition
line, comments, doc-strings, and same-named symbols in unrelated files, and silently misses calls made
through an alias, a re-export, or a renamed import; Read just shows you the raw text at a line. The nav
tools resolve the *actual* graph instead — `nav_callers` / `nav_callees` / `nav_module_deps` return the
resolved call/import edges, and **`nav_nodes_at({file, line})` resolves the real symbol/node at a line**
(use it, not Read, for "what's defined / what's called at `file:line`"). The cost is one tool call; the
payoff is a correct, complete answer — so don't grep-or-Read first and fall back to nav, start with nav.

## When to use it (vs grep / reading files)

| Question (including quick lookups) | Tool | Don't grep because |
|---|---|---|
| "Where am I? what's the module structure?" | `nav_repo_map` | grep can't build a dependency graph |
| "What's defined / called at this line?" | `nav_nodes_at` | resolves the actual symbol, not the text |
| "Who calls `X`? list/find its call sites? who uses it? what breaks if I change it?" | `nav_callers` | grep hits the definition, comments, strings, and same-named symbols elsewhere — and misses aliased/re-exported calls; this returns resolved call *sites* |
| "What does `X` call? what functions does it call/depend on?" | `nav_callees` | follows resolved call edges, not string hits in the body |
| "Show the local graph around `X`." | `nav_ego_graph` | one call gives the neighborhood |
| "What does this *file* import / depend on?" | `nav_module_deps` | resolves module edges (incl. re-exports & aliased imports), not import-line text |

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
