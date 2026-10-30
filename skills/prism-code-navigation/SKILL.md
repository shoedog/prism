---
name: prism-code-navigation
description: >-
  Navigate a codebase STRUCTURALLY with the prism MCP nav tools (nav_repo_map, nav_nodes_at,
  nav_symbol_spans, nav_callers, nav_callees, nav_ego_graph, nav_module_deps) instead of grepping. Use when reviewing a change, writing a spec, planning an edit or refactor, or analyzing
  impact and you need to know who calls a symbol, what it calls, what breaks if you change it, the
  call/dependency graph around it, or the repo's module layout — "find callers/callees", "impact /
  blast radius of changing X", "what would a change touch", "what depends on Y", "who uses Z",
  "module dependencies", "trace the call graph". Resolves calls/imports across files that grep can't.
  On a spec / plan task, orient with these tools FIRST, before reading files.
  Structural counterpart to lsp-nav (type-resolved go-to-def/references) and the slicing CLI
  (diff-driven review). Tools (nav_*, taint_reaches, refresh_index) are prefixed
  mcp__plugin_prism_prism__ (Claude Code plugin) or mcp__prism__ (server named "prism"); bare in Kiro.
metadata:
  project: prism
  surface: mcp
---

# Navigating code with prism

Prism's MCP server answers **structural** questions over a Code Property Graph of one repo. Reach for it
when the question is "who/what is connected to this symbol," not "where does this text appear." It
resolves calls and imports across files; grep can't.

## Before your first query

- **Find the tools.** Claude Code may list them as *deferred* tools (names only): load them with
  ToolSearch (`select:mcp__plugin_prism_prism__nav_callers,mcp__plugin_prism_prism__nav_callees,...`)
  before calling. A server named something
  other than `prism` changes the middle segment.
- **Check which checkout the server indexes.** One server indexes the one `--repo` it was started with.
  If you work in a worktree or clone other than that one, its answers describe the other checkout's code:
  `SymbolNotFound` for a symbol you can see in your diff, or lines that don't match your files, is that
  tell. Use a server started for your checkout, or the CLI below.
- **No tools, or the wrong checkout:** the `prism` CLI answers the same questions for any checkout:
  `prism nav callers --repo <checkout> --symbol X --format json` (also `callees`, `ego`, `module-deps`,
  `repo-map`, `nodes-at`, `taint-reaches`). It is built by `cargo build --release` in a prism checkout; if
  `prism` is not on PATH, call it by full path (`<prism checkout>/target/release/prism`).
- **Wait out warming.** The first call on a checkout with no index builds it: seconds to a few minutes
  on large repos. Retry the same call every ~10 s for up to ~3 min before giving up;
  later calls answer in milliseconds while the index is loaded. After 10 minutes without tool
  calls, the index unloads and the next call reloads it; retry warming results then too.
  `--index-idle-secs 0` keeps the index resident.

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

## In a spec or code review

**Spec and plan review:** an "X is only called from Y" or "sole producer" claim → `nav_callers` on X
(depth 1–2) and compare with the claim; a `file:line` anchor → `nav_symbol_spans` / `nav_nodes_at`
confirms it still names that callable; blast radius of a signature or semantic change → `nav_callers`
depth 2–3, and list consumers outside the spec's scope; layering → `nav_module_deps` / `nav_repo_map`.

**Code review (index the PR head):** each changed public or shared function → `nav_callers` and check
every caller still holds the changed contract; new code → `nav_callees` for the helpers it relies on; a
change to a pattern with several sites (fallbacks, guards, rebuilds) → `nav_callees` of the containing
function or `nav_ego_graph`, which finds sibling sites a text search keyed on one token misses; a new
input source near shell/SQL/path sinks → `taint_reaches`. Cite the `file:line` prism returned.

**Skip it** for docs-, test- or config-only diffs, and for a few-line diff whose one caller is in the diff.
**Don't trust it for** runtime mutation or reflection (out of model), Python inheritance, field- or
return-typed receivers, serde/struct-field data paths (not call edges), or C/C++. **An empty caller set
is not proof** that code is unreachable: confirm with a targeted read.

## Pick the tool (vs grep / read / siblings)

| Question | Tool |
|---|---|
| Where am I? what's the module structure? | `nav_repo_map` (no args) |
| What's defined/called at this exact line? | `nav_nodes_at({file, line})` |
| What exact callable coordinates should my editor use? | `nav_symbol_spans({seed})` |
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
4. **Locate an edit only when needed** with `nav_symbol_spans` on the resolved callable. Use its
   snapshot-relative byte offsets in your normal editor; Prism does not apply the edit.
5. **Stop.** The graph is the evidence. Summarize callers/callees as `file:line`, then read the specific
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
- **Framework route handlers now show a caller.** A Flask/FastAPI/Express route handler
  (`@app.route(...)`, `app.get(path, handler)`) surfaces a `nav_callers` edge (kind
  `framework_entry`, name-only) at the registration site — the registration is a real
  entrypoint even though it isn't itself a call. A module-level registration's caller is the
  synthetic `<module>` symbol (not independently navigable); it never appears in `nav_callees`.
- **Lines are 1-indexed; `nav_nodes_at` matches the exact line.** Empty result ⇒ aim at the definition
  or a call line, not a blank/comment/brace line.
- **`NoEnclosingCallable` means no recognized callable encloses the indexed line.** Seed a named
  function's line or symbol. If a function failed to parse, fix the syntax and refresh the index.
  **`NoNamedEnclosingCallable` means navigation has no enclosing named callable in the index,**
  although a callable encloses the line (such as a JS callback or Go func literal). Seed a named
  function's line or symbol, including an enclosing named function if available. For anonymous
  JS/TS/TSX variable flow, use `taint_reaches` on a line with usable variable evidence; anonymous
  symbol seeds are refused. JS/TS/TSX module lines are also taint regions: without variable
  evidence taint returns `SymbolNotFound`, including on comments and anonymous callback lines.
  `LocationOutOfRange` means line zero, past EOF, or a file absent from the navigation index;
  choose an indexed source file and an in-range line. Taint uses `UnsupportedFile` for non-indexed
  files; MCP rejects line zero as bad arguments. These query errors also apply to CLI location seeds.
- **`nav_symbol_spans` is callable-only and coordinate-only.** Offsets are end-exclusive UTF-8 bytes;
  its raw `body_span` may include braces/delimiters, and null fields explain themselves in
  `unavailable`. It never returns body text or edits source.
- **Graphs truncate (50 items by default, raise with `max_results` up to 1000; 80 KB byte cap)** and the
  **first call on a fresh checkout can take seconds to minutes** (building the CPG); fast while loaded. After 10 minutes idle, the next call reloads the index.
- **A retryable answer is not a result.** `_meta` `prism/retryable: true` with `status` `warming` or
  `indexing` means the index is still building: retry the same call (after `retry_after_secs`, when
  given). `build_failed` reports why; the next call retries the build. Under a shared server
  (`--shared`) another agent's `refresh_index` changes what you see too.
- **Read-only.** These tools never edit or run code. Make changes with your normal file tools.
