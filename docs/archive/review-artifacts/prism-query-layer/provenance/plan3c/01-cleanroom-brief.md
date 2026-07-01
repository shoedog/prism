# Problem statement: an MCP server adapter for Prism's navigation layer

You are designing an extension to **Prism** (this repository — Rust), a static-analysis tool. Explore
the repo **read-only** and ground your design in the actual code — cite `path:line`. This is a
**clean-room** design: design it your own way.

## What Prism is today

Prism began as a diff-review slicer: given `--repo` + `--diff`, it parses the referenced files, builds
a Code Property Graph (CPG: AST + call graph + data-flow + CFG over tree-sitter, 11 languages), runs a
slicing algorithm, and emits review context. **That diff-review path must stay byte-for-byte unchanged
(a regression contract).**

On top of that, a **whole-repo navigation layer** has shipped (Plans 1, 2, 3a, 3b, and an in-flight
3b.5), living in `src/navigation/`. It answers structural queries over the whole repo and is
**library-first** with one **thin CLI adapter** (`prism nav <query>`, dispatched in `src/main.rs`
`run_nav`). Study these before designing:

- `src/navigation/mod.rs` — `NavigationIndex` + `NavigationSession { repo: Arc<LoadedRepo>, index: Arc<NavigationIndex> }` (the owned, whole-repo CPG + nav-local indexes).
- `src/navigation/queries.rs` — `nodes_at`, `callers`, `callees`, `ego_graph`.
- `src/navigation/module_graph.rs` — `module_deps`, `repo_map`.
- `src/navigation/types.rs` — the serde output contract: `Evidence { query, items, truncated, warnings, graph: Option<GraphPayload> }`, `EvidenceItem`, `SymbolRef`, `Source`, `Reason` (incl. `UnresolvedImport` AND `ResolvedImport`), `Warning`/`WarningKind`, `QueryError`.
- `src/navigation/cache.rs` — the per-repo on-disk nav cache (prism-owned XDG store; `build_cached`).
- `src/output/navigation.rs` — how the CLI renders `Evidence` to text/json.
- `src/main.rs` — `Cli`/`Command`/`NavArgs`/`NavQuery` and `run_nav` (the existing adapter to mirror), and `build_session`.
- Spec reference: `docs/superpowers/specs/2026-06-07-prism-navigation-layer-design.md` §13 (MCP adapter intent), §8 (query/output model), §14 (resolver seam), §15 (evaluation seam).

## The goal (this initiative — Plan 3c, scoped to the MCP adapter ONLY)

A **thin MCP server** (a new binary, e.g. `src/bin/prism-mcp.rs`) that exposes the existing navigation
queries as MCP **tools** to coding agents, returning the existing `Evidence` JSON. It is a **second
thin adapter** beside the CLI — neither consumer is privileged, and it must **not** reimplement or fork
nav logic (reuse `src/navigation` + the `Evidence` types).

Tools to expose (the five nav queries already implemented): `nodes_at`, `callers`, `callees`,
`ego_graph`, `module_deps`/`repo_map`.

## Hard requirements / constraints

1. **Preserve diff-review + the CLI byte-for-byte (additive, "Option C").** New binary only; the
   existing `prism` binary, `prism nav …`, and all diff-review output unchanged. No CPG-core edits.
2. **Library-first / thin adapter.** Reuse the `src/navigation` query functions and the `Evidence`
   serde types verbatim. The MCP layer maps `request → existing query → Evidence JSON`. Zero nav-logic
   duplication.
3. **SDK decision via a spike.** Prefer the `rmcp` crate (Rust MCP SDK) over stdio transport; if it
   doesn't fit cleanly, fall back to a minimal hand-rolled stdio JSON-RPC. Specify the spike's
   accept/reject criteria and the transport (stdio).
4. **Forward-compatibility for a LATER Tier-2 reasoning layer (design for it; do NOT build it).** This
   same MCP server will later host reasoning tools — `taint-reaches`, `dataflow-between`,
   `impact-of-change`, `what's-missing` — seeded by a richer **`FocusSet`** (symbol / location /
   source→sink / a diff, all as seeds). Therefore:
   - **Tool registration must be extensible** — adding a new tool must not require restructuring the
     server (a registry/descriptor pattern, not five hardwired handlers).
   - **The seed-input convention must anticipate `FocusSet`** — Tier-1 tools take simple seeds
     (`symbol`, `loc:file:line`); design the input shape so a richer seed slots in later **without
     breaking the Tier-1 tool signatures**.
5. **The `Evidence` JSON is the agent-facing contract — assess its stability.** Once agents depend on
   it, changing it is a breaking change. Specifically: `module_deps` items overload `location.file`
   (it is the *target* file for a call-derived edge but the *source* file for an `UnresolvedImport`
   item), and graph-vs-flat queries return two shapes. **Decide:** stabilize the
   dependency-direction/target contract now (e.g. explicit target modeling) **before** exposing it,
   OR expose v1 explicitly as "experimental/unstable" so a later break is acceptable. Justify the call.
6. **Session lifecycle / caching.** An MCP server is long-lived and the whole-repo index build is
   expensive; the nav cache is per-repo (`NavigationSession`, `Arc`-owned, on-disk cache). Decide:
   one repo bound per server process, or a `repo` argument per request? Cache/reuse `NavigationSession`
   across requests? Concurrency (are queries `&self`-safe over a shared `Arc<NavigationSession>`)?
7. **Type-enrichment config (knob, not schema).** Nav builds with `type_db = None` today; a later
   precision pass needs `compile_commands` enrichment. Allow the server's session construction to
   accept type-enrichment config so the same tools sharpen later — without changing the tool schemas.
8. **Structured, evaluable output.** Keep output as structured `Evidence` (not free text) so a later
   evaluation harness (A/B vs an agentic-search baseline, spec §15) can measure localization
   precision/recall and token cost.
9. **Errors + determinism.** Map `QueryError` (`AmbiguousSymbol`, `SymbolNotFound`,
   `LocationOutOfRange`, `UnsupportedFile`, …) to MCP tool errors; a valid-empty `Evidence`
   (`items: []` + warnings) is a SUCCESS, not an error. Output must stay deterministic.
10. **Tests.** An MCP smoke/integration test (server answers `tools/list` and a `tools/call` returning
    `Evidence`) plus a dogfood on this repo. Nav test files are registered as explicit `[[test]]`
    targets and are NOT in the coverage matrix scanners.

## Out of scope (do NOT design now)

- The Tier-2 reasoning tools/algorithms themselves (taint/impact/dataflow) — only make the adapter
  ready for them.
- Cross-repo / multi-repo / org-scale.
- Import-resolution / method-resolution precision upgrades (separate work).

## What to produce

A concrete design + architecture for the MCP adapter: approach + component/file boundaries; the
interfaces/seams (tool-registry, seed convention, session lifecycle, `Evidence` serialization);
control/data flow per request; key decisions with the ALTERNATIVES considered and why; risks
(esp. the Tier-2 forward-compat seams and the `Evidence`-contract-stability call); and the smallest
shippable slices + build order. Explicitly answer: the `rmcp`-vs-stdio spike criteria, the
session-lifecycle decision, and the `Evidence`-contract-stability decision.
