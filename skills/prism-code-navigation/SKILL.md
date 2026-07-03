---
name: prism-code-navigation
description: >-
  Navigate a codebase STRUCTURALLY with the prism MCP nav tools (nav_repo_map, nav_nodes_at,
  nav_callers, nav_callees, nav_ego_graph, nav_module_deps) instead of grepping or reading files
  blindly. Use when reviewing a change, writing a spec, planning an edit or refactor, or analyzing
  impact and you need to know who calls a symbol, what it calls, what breaks if you change it, the
  call/dependency graph around it, or the repo's module layout — "find callers/callees", "impact /
  blast radius of changing X", "what would a change touch", "what depends on Y", "who uses Z",
  "module dependencies", "trace the call graph". Resolves calls/imports across files that grep can't.
  On a spec / plan / impact-analysis task, orient with these tools FIRST, before reading files.
  Structural counterpart to lsp-nav (type-resolved go-to-def/references) and the slicing CLI
  (diff-driven review). Tools are mcp__prism__nav_* for Claude/Codex, bare nav_* for Kiro.
metadata:
  project: prism
  surface: mcp
---

# Navigating code with prism

Prism's MCP server answers **structural** questions over a Code Property Graph of one repo. Reach for it
when the question is "who/what is connected to this symbol," not "where does this text appear." It
resolves calls and imports across files; grep can't.

## Orient before you read — especially for a spec / plan / impact analysis

When the task is to **write a spec, plan a change or refactor, or analyze the blast radius** of editing a
symbol, the structural navigation is the *first* step, not an afterthought. Don't open files and grep your
way around for callers — that misses cross-file/aliased call sites and burns the budget. Instead:

1. `nav_repo_map` once for the module layout (skip if you already know it).
2. `nav_callers` / `nav_callees` / `nav_ego_graph` on the symbol(s) the change touches → you now have the
   exact call sites, dependents, and blast radius as `file:line`.
3. THEN read only those specific sites and write the spec/plan grounded in them.

"List every site a change would update", "what breaks if I change X", "plan splitting this module", "what
does a new flag touch" are all *callers/dependency* questions — answer them with nav, then read the sites
it points at. Reading first and grepping for callers gives a slower, incomplete answer.

## Pick the tool (vs grep / read / siblings)

| Question | Tool |
|---|---|
| Where am I? what's the module structure? | `nav_repo_map` (no args) |
| What's defined/called at this exact line? | `nav_nodes_at({file, line})` |
| Who calls `X`? what breaks if I change it? what would a change touch? | `nav_callers` |
| What does `X` call / depend on? | `nav_callees` |
| The local graph around `X`? | `nav_ego_graph` |
| What does this *file* import/depend on? | `nav_module_deps` |

**Use a different tool when:** the question is *type-resolved* (the exact type at a point, trait
implementors, references through generics) → **lsp-nav**. The input is a *diff* you're reviewing → the
prism **`slicing` CLI**. You want *literal* text, comments, strings, or config → grep/read.

## Workflow: orient → seed → expand → stop

1. **Orient** with `nav_repo_map` once, if you don't know the layout.
2. **Seed** the symbol: by name `{kind: "symbol", name: "process_order"}` (add `{file}` to disambiguate),
   or by location `nav_nodes_at({file, line})` → take a node from the result.
3. **Expand** with the *one* tool that answers the question — `nav_callers` for impact-of-change,
   `nav_callees` for dependencies, `nav_ego_graph` for the neighborhood. One hop usually suffices.
4. **Stop.** The graph is the evidence. Summarize callers/callees as `file:line`, then read the specific
   sites and act. Don't re-query in a loop — this is a *sharpening* step, not the goal.

## Gotchas (don't skip)

- **Read the `score` field.** Scores start from resolution confidence (`1.0` exact, `0.6` name-only);
  callers/callees decay that by hop, so a lower score means farther-away exact evidence or weaker
  name-only evidence. Read the cited site before relying on any score below `1.0`. A warning like
  `N same-name receiver call site(s) with unknown receiver type across multiple owner types; not
  attributed as callers` means real callers may be missing: treat "no callers" plus that warning as
  *unknown*, not *none*.
- **Candidate edges are labeled, not certain.** Python/JS/TS/Tsx unknown-receiver calls with a
  same-name method on 2-3 owner classes surface as a caller/callee with kind `r6_multi_owner_candidate`
  at name-only confidence — verify at the cited site; more than 3 same-name owners are still dropped
  and reported in the warning above.
- **One repo only.** The server sees only its `--repo` tree — not sibling repos, dependencies, or std.
  "No callers" outside the tree means *out of scope*, not *none*.
- **Lines are 1-indexed; `nav_nodes_at` matches the exact line.** Empty result ⇒ aim at the definition
  or a call line, not a blank/comment/brace line.
- **Graphs truncate (50 items by default, raise with `max_results` up to 1000; 80 KB byte cap)** and the
  **first call can take ~30 s** (building the CPG); instant after.
- **Read-only.** These tools never edit or run code. Make changes with your normal file tools.
