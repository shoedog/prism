# Clean-room constraints brief — Tier 2 Plan B: `taint_reaches` (on the MERGED substrate)

> **Status:** Archived query-layer note. See `docs/features/query-layer/README.md` for current docs and the local archive README for routing.

**For codex + claude (clean-room, prism-equipped).** This is a *constraints* brief, not a design. Produce
your **own** analysis → architecture → component design → implementation-plan outline for `taint_reaches`
v1, reading the **actual merged repo** at the session cwd via prism (`mcp__prism__nav_*`) + read-only tools.
Where this brief states a *requirement* (contract, additivity, scope), honor it; everything else (module
layout, how seeds resolve, how the witness/frontier maps into the output, capping seam, tool naming, what
foundation work to fold in, task decomposition/ordering) is **yours to design**. **A separate owner
implementation plan exists and is deliberately withheld so your pass is independent; the two will be
diffed and folded.** Do not try to reverse-engineer it — design from the substrate + the contract.

## What is already MERGED on `main` (the substrate — Plan A, commit `40a1b46`)
Tier 1 shipped a whole-repo **navigation** layer + a local stdio MCP server (`prism-mcp`, cargo feature
`mcp`) returning a JSON `Evidence` envelope. Tier 2 (the **reasoning** layer) just merged its **substrate
gate (A3+A4+A7)** after a 10-round review. `taint_reaches` is the first tool that consumes it. **Explore
these via prism — do not take this list as exhaustive; verify signatures/behavior against the tree:**
- **A3 taint BFS:** `CodePropertyGraph::taint_trace(&[(file,line)]) -> Trace` (`src/cpg/trace.rs`) — a
  single inline-CFG-filtered predecessor BFS over the production petgraph. `Trace` carries per-root
  frontier + parents + a `boundary` set of cross-`(file,function)` `BoundaryEdge`s + `degraded` + warnings;
  `Relation { DataFlow, AssignmentPropagation, RecoveredDefUse }`. **Known v1 precision limit (documented):**
  `reachability` is tri-state and `NotReached` means "not reached within the seed function ∪ first-hop
  boundaries," NOT proven absence (line-granular CFG scoping). Read `docs/archive/plans/prism-query-layer/planA-followups.md`.
- **A4 cleansing:** a `pub(crate)` adapter that, given a source location, returns the sanitizer categories
  present in the source's function (presence keyed on the source, **not** path-proof). Find it.
- **A7 reasoning scaffold:** `src/reasoning/{mod,seeds,shape}.rs`. `shape.rs` already has node-level
  reachability + witness-graph builders. `seeds.rs` has the seed *types* (`SeedSpec`/`ResolvedSeed`/`SeedSet`)
  but **no resolution logic yet**. The additive `Evidence.reasoning: Option<ReasoningSummary>` vocabulary
  (`Reachability`, `SinkResult`, `ReasoningSummary`, `ReasoningReason::TaintedBy`, `ReasoningWarning`) is
  merged in `src/navigation/types.rs`, serde-skipped when `None`.
- **Contracted-to-Plan-B items** the review deferred (decide which `taint_reaches` v1 needs): node/location-
  precise seeding (today's `taint_trace` seeds by line); per-root reachability/witness variants (for
  per-source attribution / multi-source cleansing union). See `planA-followups.md`.

## Required contract (requirements — honor)
`taint_reaches(sources, sinks?)`:
- `sources` and optional `sinks` are **sets of seeds**, each a program point (by source location, or a
  symbol resolving to parameters). The seed-set abstraction must be **reusable** by the later three tools
  (`dataflow_between`, `impact_of_change`, `what_missing`).
- **Sinks given → witness mode:** per-sink **tri-state** reachability (`Reached`/`NotReached`/`BoundaryExited`)
  + a witness path per reached sink. **Sinks omitted → frontier mode:** the full tainted frontier (scored items).
- **Taint semantics**, including the substrate's sanitizer presence — distinct from a future generic
  taint-agnostic chop.

## Hard constraints (requirements)
1. **Option C — purely additive / byte-identical.** `cargo test --test cli_nav_compat` must stay
   byte-for-byte unchanged (NOT the non-deterministic `review` preset). The production Taint slice
   (`taint_forward_cfg`) and nav output must not move. New reasoning code is always-compiled; only the MCP
   *tool registration* is `mcp`-gated. Verify how the substrate kept this.
2. **Reuse the `Evidence` envelope + the merged `reasoning` vocabulary** — do not invent parallel types or
   redefine the merged ones.
3. **Build on the merged substrate** (A3 BFS, A4 adapter, A7 shaper) — do not reimplement taint or the witness.
4. **Reach scope v1 = intraprocedural.** Cross-function taint → a `BoundaryExited` + a visible boundary
   marker, never a silent false negative. Interprocedural = a later phase.
5. **Register on the existing `prism-mcp` adapter**, matching its read-only tool conventions (input schema,
   output shaping/capping seam, registry). Keep the existing nav tool count/test frozen.

## What to produce
1. **Analysis** of the merged substrate's fit for `taint_reaches` (what's ready, what gaps must close first,
   which contracted-to-Plan-B items v1 actually needs, the biggest soundness/Option-C risk).
2. **Architecture** — module layout, the seed-resolution + query + shaping flow, where capping/truncation
   lives, the MCP surface, and how the frontier/witness map into the merged `Evidence.reasoning`.
3. **Component design** for the non-obvious pieces (seed resolution incl. the error/empty truth table;
   frontier scoring; multi-source attribution; the witness union graph; the boundary marker).
4. **Implementation-plan outline** — phased, TDD-shaped, each phase independently testable, with the
   Option-C proof named. Foundation work worth folding in is yours to identify.

## Notes
- prism (`mcp__prism__nav_*`) is wired: `nav_repo_map` to orient; `nav_nodes_at({file,line})` /
  `nav_callers` / `nav_callees` / `nav_ego_graph` / `nav_module_deps` to verify the substrate surface
  (signatures, call sites, the Evidence/shaping seam) against the real tree.
- Cite `file:line`. Flag any place the merged substrate's API differs from what this brief implies.
