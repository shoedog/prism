# Clean-room constraints brief — Tier 2 Plan A: `taint_reaches`

> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

**For codex (clean-room).** This is a *constraints* brief, not a design. Produce your **own** analysis →
architecture → component design → implementation-plan outline for `taint_reaches` v1, reading the actual
repo at the session cwd. Do not assume any particular internal solution — where this brief states a
*requirement* (contract, additivity, scope), honor it; everything else (how paths are represented in the
output, module layout, how the data-flow graph is obtained, tool naming, what foundation work to fold in)
is **yours to design**. A separate owner design exists and is deliberately withheld so your pass is
independent; the two will be compared and folded.

## Problem
Prism Tier 1 shipped a whole-repo **navigation** layer with a local stdio MCP server (`prism-mcp`,
behind cargo feature `mcp`) exposing read-only nav tools that return a JSON `Evidence` envelope. Tier 2
adds a **reasoning** layer. Plan A is its first tool, built as an end-to-end vertical slice that proves
the pattern the other three reasoning tools (`dataflow_between`, `impact_of_change`, `what_missing`) will
reuse.

## Required contract (a requirement — honor it)
`taint_reaches(sources, sinks?)`:
- `sources` and optional `sinks` are **sets of seeds**, each seed addressing a program point (by source
  location, or by a symbol that resolves to one). Design the seed-set abstraction (it must be reusable by
  the later three tools — treat it as a shared input primitive).
- **Sinks given:** propagate taint from the sources and answer "does taint reach the sinks?", returning
  the witnessing path(s).
- **Sinks omitted:** return the full tainted frontier (everything the sources taint).
- Uses **taint-propagation** semantics (including any sanitizer/cleansing the substrate models) — it is
  distinct from a future generic, taint-agnostic source→sink data-flow chop.

## Hard constraints (requirements)
1. **Option C — purely additive.** The default `cargo build` / `cargo test` must stay byte-for-byte
   unchanged (the diff-review and nav golden outputs must not move). New reasoning code is always-compiled
   and additive; only the MCP *tool registration* may be feature-gated. Verify how Tier 1 achieved this
   (search for the additive-accessor / feature-gating pattern) and follow it.
2. **Reuse the `Evidence` envelope** the nav layer already returns — do not invent a parallel output type.
   (How a taint path/frontier maps into that envelope is yours to design.)
3. **Build on the existing data-flow / CPG primitives** — do not reimplement taint propagation. Find them
   and wrap them.
4. **Reach scope v1 = intraprocedural.** Cross-function (interprocedural) taint is explicitly deferred to a
   follow-up; v1 must make the boundary **visible** rather than silently producing a false negative.
5. **Register on the existing `prism-mcp` adapter**, matching its read-only tool conventions (input schema,
   annotations, bounded/size-capped results, deterministic ordering).
6. Deterministic output (the codebase uses sorted maps/sets throughout) and read-only behavior.

## Substrate to read (session cwd is the repo)
- `src/data_flow.rs` — the data-flow graph and its reachability/taint/chop primitives.
- `src/cpg.rs` — the Code Property Graph context that composes AST + data flow + CFG + call graph, and its
  build entry points (note there are scoped/subset builders).
- `src/cfg.rs`, `src/access_path.rs`, `src/call_graph.rs` — control flow, field-sensitive paths, call graph.
- `src/navigation/` — the `Evidence` envelope, seed resolution, and the session/index types.
- `src/mcp/` — the adapter: tool registry, handlers, input schema, output size-capping/error bounding.

## Known facts (from dogfooding `prism nav` on this repo)
- The data-flow reachability is intraprocedural (its same-line propagation is gated on same-function;
  the CFG is intraprocedural). Cross-function taint exists only in heavier algorithm code.
- Nav resolves calls by method **name**, not receiver **type** — so reasoning should traverse the
  data-flow/CPG edges directly, not the nav call graph.
- The nav index builds a full CPG context (including the data-flow graph) but retains only the property
  graph; the loaded repo retains the parsed files, and scoped CPG/DFG builders exist.

## Deliverables (your independent pass)
1. **Analysis:** how the existing primitives map to the required contract; the precise integration seam;
   the cheapest correct way to obtain the data-flow graph for the seed(s) under the intraprocedural scope.
2. **Architecture:** module placement, the seed-set primitive, how the taint path/frontier is represented
   in the `Evidence` envelope, the MCP tool surface, and what (if any) foundation/refactor work you would
   fold into this slice and why.
3. **Implementation-plan outline:** TDD task breakdown with the failing-test-first shape, fixtures, and the
   Option-C verification.
4. **Risks / unknowns / questions** you would want answered before implementing.

When the MCP integration is available, this same brief is re-run with `prism-mcp` connected, to compare
what the tooling adds to your analysis.
