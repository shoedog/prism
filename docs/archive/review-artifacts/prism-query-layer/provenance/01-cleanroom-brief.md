# Problem statement: a whole-repo navigation / architecture layer for Prism

You are designing an extension to **Prism** (this repository — Rust, ~16k LoC of
`src/`), a static-analysis tool that today extracts focused code slices from a
**diff** for LLM code review. Explore the repo read-only and ground your design
in the actual code (cite `path:line`).

## What Prism is today

Given `--repo` + `--diff`, Prism parses the files the diff references, builds a
Code Property Graph (CPG) over them, runs a slicing algorithm, and emits review
context. It supports 11 languages via tree-sitter. The CPG unifies AST, a
call graph, a data-flow graph, and a CFG.

## The goal (this initiative, scoped to ONE layer)

Let an LLM/coding agent **navigate and understand the whole repository's
structure** — not only diff-scoped review. Concretely, this layer must answer
**whole-repo navigation and architecture queries**, for example:

- callers / callees of a symbol (bounded depth),
- a bounded graph neighborhood ("ego graph") around a symbol or `file:line`,
- "what CPG nodes are at this `file:line`",
- "what depends on this module / what does this module depend on",
- a repository module / dependency map.

This is the **first** layer of a larger plan. A later layer (OUT OF SCOPE here)
will add seeded interprocedural *reasoning* (taint / data-flow / change-impact
from a symbol seed). Design this navigation layer so that later layer can sit on
top cleanly, but **do not design the reasoning layer now**.

## Hard requirements / constraints

1. **Preserve the existing diff-review behavior byte-for-byte.** The current
   `--repo --diff -a <algo>` path and all its output must be unchanged. This is
   non-negotiable; treat it as a regression contract.
2. **Library-first.** The navigation logic lives in a clean Rust library API.
   Two THIN adapters consume it: the CLI, and an MCP server (for coding agents).
   Neither consumer is privileged.
3. **Single-repo only.** Cross-repo / org-scale symbol resolution is explicitly
   out of scope (a later SCIP/Glean backend will handle it). Design a resolver
   seam, but do not build cross-repo.
4. **Reuse Prism's existing CPG / call-graph infrastructure.** Do not duplicate
   graph construction or introduce a second, parallel graph representation.
5. **Deterministic, explainable output.** Each result should carry *why* it was
   returned (the edge / containment / call that justifies it).
6. **No vector/embedding RAG, no whole-repo long-context prompting, no learned
   models.** (Evidence-rejected for this goal; see the research doc below.)
7. **Repo conventions:** keep files under ~600 lines; BTreeMap/BTreeSet for
   deterministic ordering; clap for CLI.

## Codebase facts worth knowing (verify in code; cite path:line)

These were non-obvious and bear on the design:

- **The CPG core is already diff-independent.** `CpgContext::build(files:
  &BTreeMap<String, ParsedFile>)` and `CodePropertyGraph::build(files)` take an
  *arbitrary* file map — not a diff. There are also `build_scoped`,
  `build_incremental`, `build_with_cached_cpg`, `build_with_registry` (see
  `src/cpg.rs`). The **diff-coupling lives only in `src/main.rs`** (it parses
  the files the diff references) and in algorithm signatures (each slicing
  algorithm takes a `DiffInput`; see `src/algorithms/mod.rs`).
- **The call/symbol graph is separable from the (expensive) data-flow graph.**
  `CallGraph::build` / `CallGraph::build_direct_subset` vs
  `DataFlowGraph::build` / `build_subset` (`src/call_graph.rs`,
  `src/data_flow.rs`). A navigation layer may be able to avoid building the
  data-flow graph entirely.
- **Caching exists but is constrained** (`src/cpg_cache.rs`): per-file SHA-256
  hashes; the cache is all-or-nothing and returns `Hit` / `PartialHit {
  cached_call_graph, cached_dfg, changed_files }` / `Miss`. **`PartialHit`
  requires the SAME file set** — adding or removing a file forces a full
  rebuild. `build_incremental` removes changed files' CG/DFG, rebuilds only
  those, merges, and reassembles — but `build_direct_subset` resolves only
  **direct** calls; indirect (function-pointer / callback) edges into unchanged
  files rely on cached resolution (an incremental-soundness caveat).
- **Symbol resolution is heuristic + import-aware**, not compiler-accurate:
  name-based matching plus per-file import maps (`alias → module_path`);
  `resolve_callees` / `resolve_callees_qualified` (`src/call_graph.rs`). This is
  the relevant precision ceiling; a future SCIP backend is the intended relief.
- **The CLI is a single flat `clap::Cli` struct** with `--repo` and `--diff`
  both `required_unless_present = "list_algorithms"`; there are no subcommands
  today (`src/main.rs`). Whole-repo queries have no diff, so the CLI must grow a
  no-diff invocation path without breaking the existing one.
- **Output formatting** lives in `src/output/` (`text`, `json`, `paper`,
  `review` formats).
- This repository itself is **Rust**; structural Rust support (call graph, CFG,
  def-use) is solid (see `src/languages/mod.rs`). The layer should be
  dogfoodable on this very repo.

## Reference material (read-only, optional)

In `docs/prism-query-layer/` you may read:

- `research-llm-codebase-navigation.md` — a survey of LLM codebase-navigation
  techniques (why precise graph navigation + lexical search beat vector RAG and
  long-context; SCIP/Glean for cross-repo; agentic search as the baseline).
- `analysis-prism-extension.md` — an earlier high-level analysis of extending
  Prism. Use it as background on goals/requirements only; **form your own
  design** — do not treat its proposals as decided.

## What to produce

A concrete, independent **clean-room design** for this whole-repo navigation /
architecture layer:

- **Approach + component/file boundaries** — where responsibility lives (new
  modules, their seams), respecting the existing architecture.
- **How the whole repo is loaded and the graph constructed** — load modes,
  what's built vs skipped (e.g., call graph without data flow), where this slots
  against the existing `CpgContext` builders.
- **The navigation/architecture query API + output model** — operations,
  signatures, and the explainable result type.
- **Caching / incrementality strategy** given the facts above (same-fileset
  constraint, indirect-call caveat) — what's good enough for v1, what's deferred.
- **How diff-review is preserved** byte-for-byte (the regression contract).
- **Delivery seams** — the library API and the thin CLI + MCP adapters.
- **Decisions + rationale, the main ALTERNATIVES considered + why against**,
  risks tied to concrete future change, and the **smallest shippable slices +
  build order**.

Cite `path:line` for everything you build on. Design it your own way.
