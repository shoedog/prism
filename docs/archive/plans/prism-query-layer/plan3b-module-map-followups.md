# Plan 3b (module/repo map + ego envelope) — deferred follow-ups

> **Status:** Archived query-layer note. See `docs/features/query-layer/README.md` for current docs and the local archive README for routing.

Non-blocking findings intentionally deferred from Plan 3b. The in-scope fixes
(the two holistic-review MAJORs — shared edge collector + correct call-site text)
were applied in-branch. Items below need a larger change or carry real risk;
bucketed by the follow-on slice that should own them.

## Headline deferred theme — typed module-dependency edge schema (→ before resolved-imports work)

The holistic code-review's architecture lens (inline-diff round) raised four findings that are
one architectural seam, **explicitly gated by the verdict**: *"ship after fixing the 2 MAJOR
architecture seams **if Plan 3b is meant to absorb resolved imports soon; otherwise ship with
known schema debt** plus the test-helper fix."* Resolved imports are NOT imminent (Plan 3c is the
MCP adapter; filesystem-resolved imports are a deferred reasoning/precision item, #5 below), so
this ships as **known, documented schema debt**. The two low-risk items (test-helper `create_dir_all`,
NaN-safe `sort_items`) were fixed in-branch.

The seam, to address when resolved imports land:
- **Model dependencies as typed edges with explicit targets** — a `module-deps`/`repo-map` edge is
  conceptually `(source_file) --kind--> (target: FileTarget(path) | UnresolvedModule(name))` plus
  reason metadata. v1 flattens this into `EvidenceItem` where `location.file` is the source for an
  `UnresolvedImport` item but the target for a `PrismCpg` call edge — so a naive client grouping by
  `location.file` misreads an unresolved import as a self-dependency. (Mitigated today: items are
  distinguishable by `source` + `why`; the JSON is self-describing.)
- **One collector returning typed edges** — `collect_module_edges` centralizes only call-derived
  edges; imports are labeled separately in both `module_deps` and `repo_map`. When imports become
  resolvable, return typed dependency edges (with reason/source) from a single collector and project
  both queries (and the rendering/`GraphNode`-kind) from it, instead of re-splitting graph logic.
- **Explicit graph/node + reason kinds** — replaces `GraphNode.symbol: None` as the file-node
  discriminator (#2) and makes `Reason::Calls` text rendering endpoint-aware (currently source-side
  generic). Re-baseline the two graph goldens via an adapter to preserve current JSON if desired.

## From the holistic code-review (Plan 3b PR)

### 1. Import evidence has no source location — **Low** (→ reasoning / nav-polish)
- **Why deferred:** `ParsedFile::extract_imports` exposes only `alias -> module`
  (`ast.rs:289`), not the import statement's line. Surfacing a real location
  requires extending import extraction (a parsing-layer change with its own tests).
- **Impact:** `module_deps` anchors `UnresolvedImport` items at `{file, line 1}`,
  so a future UI jump / suppression / agent workflow can't land on the actual
  import line.
- **Fix sketch:** have `extract_imports` (or a sibling) also return the import
  node's line; thread it into the `HeuristicImport` item `location`.

### 2. `GraphPayload`/`GraphNode` carry no explicit graph/node kind — **Low** (→ 3c / first-class-nodes follow-up)
- **Why deferred:** overlaps spec §19 follow-up #4 (first-class file/module/class
  CPG nodes). v1 encodes "file node" as `symbol: None` and leaves graph semantics
  implicit in the query name (`ego` vs `repo-map`). Adding kind enums changes the
  serialized shape (golden re-baseline) and is cleaner to do alongside real
  file/module nodes.
- **Impact:** consumers infer node semantics from `symbol` nullability + the query
  string; brittle if more graph-shaped queries are added or a query mixes node kinds.
- **Fix sketch:** add a `GraphKind`/`NodeKind` enum (e.g. `SymbolNode` vs `FileNode`)
  to the payload, or split into typed graph variants; re-baseline the two graph goldens.

## Carried from the plan's own Deferred section

### 3. Scoped-call resolution follow-ups — **Important for navigation precision** (relates to spec §19 #2)
- **Delivered in Plan 3b.5:** nav-local `::`-scoped `mod::fn` / `Ns::func`
  resolution for Rust and C++ when the module or namespace segment matches the
  target file stem. This makes scoped-dispatch files like `src/algorithms/mod.rs`
  produce cross-file `callees`, `callers`, `module-deps`, and `repo-map` edges.
- **Remaining: `Type::method` associated functions.** Calls where the type name
  differs from the target file stem need a type-to-file map before they can be
  resolved without over-reporting.
- **Remaining: `ego` scoped edges.** `ego` walks CPG call edges materialized in
  the nav index, so scoped-call edges need nav-index CPG augmentation before they
  appear there. Warning: `ego` now returns a strictly smaller neighborhood than
  `callees` plus `callers` for scoped-dispatch symbols.
- **Remaining: language-agnostic method/receiver resolution.** Cross-file method
  or receiver calls need `type_db`-backed type information, independent of source
  language.

### 4. Per-edge `reason` on `GraphEdge` — **Low** (→ 3c / nav-polish)
- Spec §8's ego golden shows `edges[].reason`; v1 keeps `{from,to,kind}`. The call
  evidence lives in the richer `module-deps` `why`. Add per-edge reasons when the
  MCP/ego consumers need them.

### 5. Filesystem-resolved imports (`ResolvedImport`) — **Important** (→ reasoning/precision)
- The "Exact (imports+calls)" tier for Python/JS/TS/Go; v1 labels every extracted
  import `UnresolvedImport`. Resolving module strings to repo files (then emitting
  `ResolvedImport{module,target_file}`) is the precision upgrade.

### 6. Spec §10 prose/tier-table contradiction — **doc-sync**
- The §10 tier table says Rust "imports surface as UnresolvedImport", but the prose
  ("imports extracted only for Python/JS/TS/TSX/Go") + the implemented behavior is
  "no items, no warning" for Rust. Amend §10 to match.

### 7. MCP adapter — **Plan 3c**
- Expose `module_deps`/`repo_map` (and the other three queries) as MCP tools
  returning `Evidence` JSON. This is the next plan.
