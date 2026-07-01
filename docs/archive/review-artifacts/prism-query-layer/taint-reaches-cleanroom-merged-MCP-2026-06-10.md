I'll merge the two designs. The architects converge on most of the spine; before writing the synthesis I need to adjudicate three factual disagreements against the tree: (1) whether the round-6 ordering gate is mandatory before witness serialization (claude includes a whole component for it; codex omits it), (2) whether function identity must be fixed pre-tool (codex) or documented-and-deferred (claude), and (3) the current `SinkResult` shape and what the followups contract for per-source attribution.Both adjudications are settled by the followups doc itself. Here is the merged design.

---

# Merged design — `taint_reaches` v1 (Tier 2 Plan B)

## Adjudications (verified against the tree)

Three real disagreements; the doc decides all three:

1. **Round-6 ordering gate — claude wins; codex omitted a mandatory item.** `planA-followups.md:270–277` is explicit: the byte/column ordering fix is "**mandatory before any `reasoning` witness is serialized for Plan B**, synthesizing def→use edges only to same-statement-or-later uses." Codex's design serializes witnesses with no gate — that ships the round-6 corruption. Claude's gate component (occurrence oracle + CFG-cycle carve-out, scoped to the `RecoveredDefUse` def→use arm only) is in. The doc's own wording ("the uniform `Def → same-line-Use` arm") also confirms claude's G2 scoping — the `AssignmentPropagation` Use→Def arm is NOT gated (its cross-statement leak is the separately-deferred MINOR 7, `planA-followups.md:26–32`).
2. **`SinkResult` source attribution — codex wins on direction.** Claude treated reshaping as a constraint-2 violation; the doc contracts the opposite: "Plumb `root` into `SinkResult`/witness selection **before the shape freezes** (Plan B)" (#10, `planA-followups.md:72–74`), reinforced by round 8: "leave room for **multi-source** attribution before the shape freezes" (`planA-followups.md:352–354`). Codex also verified no `SinkResult` is constructed anywhere yet, so the reshape is emission-safe. Exact shape is an owner decision (below).
3. **Function-node identity — codex wins on timing.** Claude carried it as a documented risk deferred to the interprocedural phase; the doc says "Plan B **must** key `BoundaryEdge`/`Trace` attribution on CPG Function *node* identity … **before more consumers accrete**" (`planA-followups.md:278–284`, re-raised round 7 at 316–321). `taint_reaches` is the first accreting consumer and it serializes witnesses; the conflation failure mode is a false `Reached` with a witness spanning two unrelated functions. It goes in the foundation. (Mitigating context, per round 7: the parameter-binding boundary fix already catches arg→param conflation name-independently; the residual is direct cross-function same-name DataFlow edges — real but narrower than codex implied.)

## Convergent spine (both chose independently — high confidence)

- Pure orchestration over the four merged seams: **seed resolution → node-precise A3 trace → per-root A7 classification/witness → `Evidence` + shaper**. No taint reimplementation, no overlay state; `Trace` is the only intermediate.
- **Node-precise trace entry** added to `trace.rs`; the existing line API becomes/remains a thin wrapper with byte-identical behavior (all A3 tests stay green untouched).
- **Per-root** `reachability_for_node_from` / `witness_graph_for` in `shape.rs`; existing unioned forms become delegating wrappers (exactly `planA-followups.md:116–121`).
- **Seed resolver** in `src/reasoning/seeds.rs`: `Loc` and `Symbol` kinds; symbol → `resolve_fn`-style scan → parameter `Def`s at the function start line (sound for multi-line signatures per the round-8 pin); partial failures → soft `SeedUnresolved` warnings; `SymbolNotFound`/`AmbiguousSymbol` → hard `QueryError`; explicit `sinks` that all fail → hard error; `sinks: []` invalid; omitted sinks = frontier mode; unresolved sink → **no** `SinkResult` (absent ≠ `NotReached`).
- **Sink resolution** reuses the existing `sink_nodes_at` heuristic, promoted out of `shape.rs` into `cpg/query.rs` (never the lossy `reachability_at` line wrapper — both architects independently honored `planA-followups.md:48–55, 391–393`).
- **Witness mode:** per sink × per root classification; sink verdict = worst-of per the `types.rs:157–162` rule; `ReasoningSummary::aggregate` across sinks; one **union witness graph** deduped by NodeIndex / `(from,to,kind)`, with `Evidence.graph` *being* the witness graph for reasoning-bearing results (resolves MAJOR 4's open question — this tool never emits an ego graph). **Frontier mode:** scored items, `reasoning.reachability = None`, `per_sink = []`, counts pre-cap.
- **A4 sanitizers:** presence-only per source root, carried on `TaintedBy`; never downgrades `Reached`.
- **Boundary-only roots** → sink-located `InterproceduralBoundary` warnings; never a silent false negative.
- **MCP:** one new read-only tool on `prism-mcp`; `nav_v1()` and its six-tool test stay byte-identical; served registry asserts seven; existing `max_results`/verbosity/clamp conventions reused.
- **`graph_node` clip repair is a first-class shaping invariant** (MAJOR 4): both found independently that `build_result`/`shape_result` clips graph nodes without repairing `reasoning.per_sink[*].graph_node`.
- **Option-C proof:** `cli_nav_compat` byte-for-byte + `algo_taint_cve` + the untouched six-tool registry test, run after every slice.

## Component boundaries (merged layout)

```
src/reasoning/
  types.rs         NEW   vocabulary relocates here (#7); navigation/types.rs `pub use`s it
                         (Evidence.reasoning FIELD stays in navigation — byte-compat);
                         SinkResult reshape (per-source attribution, owner decision A);
                         ReasoningSummary::aggregate + repair_after_clip(kept)
  seeds.rs         FILL  resolve_seeds(index, specs, role) -> SeedSet; ResolvedSeed gains nodes: Vec<NodeIndex>
  order.rs         NEW   SameLineOrder oracle over ParsedFile (occurrence↔node rank matching,
                         conservative-keep + warning on count mismatch)
  shape.rs         EXT   per-root variants; unioned forms delegate; union_witness_graph;
                         lazy per-Trace boundary-closure memo (contracted, planA-followups.md:138-140)
  taint_reaches.rs NEW   the query: (index, files, input) -> Result<Evidence, QueryError>  (pure)
src/cpg/
  trace.rs         EXT   trace_root() extraction; taint_trace_nodes(&[NodeIndex], Option<&dyn SameLineOrderView>);
                         line API delegates with None; BoundaryEdge.kind { CrossFunction, SelfFunctionParam };
                         function identity: (file,name) comparison → Function-node identity by containing range
  query.rs         EXT   sink_candidate_nodes_at() promoted from shape.rs;
                         var_node_for_location(&VarLocation) -> Option<NodeIndex>
                         (+ the named VarAccessKind→VarAccess conversion — codex's executable detail)
  cfg_queries.rs   EXT   line_on_cfg_cycle() — NEW fn; cfg_reachable_lines self-membership does NOT work
                         (reachable_forward never re-enqueues the start, query.rs:100-106)
src/mcp/
  tools_reasoning.rs NEW schema + parse + handler + register_reasoning()  [mcp-gated with the rest]
  output.rs        EXT   build_result calls reasoning.repair_after_clip(kept) on EVERY invocation
                         (it runs inside the byte-budget binary search — claude's G7)
  mod.rs           EDIT  run(): nav_v1() then register_reasoning(&mut r)
```

Registry mechanism: claude's additive `register_reasoning` over codex's parallel `mcp_v1()` constructor — avoids a second monolithic constructor and keeps the `mcp` gating local — but adopt codex's negative assertion too: the smoke test asserts seven served AND that the reasoning tool is absent from `nav_v1()`.

## Key interfaces

```rust
// trace.rs — node entry recomputes line-level degrade guards over ALL Variable
// nodes at each root's line (NOT over the passed roots) — claude's G1; codex's
// taint_trace_roots as drafted would resurrect the round-6 shared-line false NotReached.
pub fn taint_trace_nodes(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>) -> Trace;

// query.rs — codex's executable bridge (VarLocation.kind is data_flow::VarAccessKind,
// CPG lookup needs cpg::VarAccess; the conversion is named, not hand-waved)
pub fn var_node_for_location(&self, loc: &VarLocation) -> Option<NodeIndex>;

// shape.rs — per-root (contracted round-3 MAJOR)
pub fn reachability_for_node_from(cpg, trace, root, sink) -> Reachability;
pub fn witness_graph_for(cpg, trace, root, sink) -> Option<GraphPayload>;

// reasoning/taint_reaches.rs
pub fn taint_reaches(session, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>) -> Result<Evidence, QueryError>;
```

MCP input (codex's concrete schema): `sources` required non-empty, `sinks` optional (omitted = frontier, `[]` = invalid), `max_results`, `verbosity`, existing clamp/default conventions from `mcp/input.rs`.

## Decided semantics (merged, with sources)

- **Ordering gate** (claude, mandated): admit a same-line `RecoveredDefUse` edge iff the use's occurrence byte ≥ the def's (oracle), **or** the line is on a CFG cycle (`line_on_cfg_cycle` — preserves the round-9 loop-carried fix for one-lined loop bodies). Gate **only the node-precise BFS** (claude's G5): witnesses serialize only gated parent edges, so order-infeasible witness edges are structurally impossible; the ungated classifier can only over-fire `BoundaryExited` (safe, documented asymmetry). Oracle mismatch → conservative-keep + warning (over-report, never a false `NotReached`).
- **Symbol-sink honesty** (claude's G6): a cross-function symbol sink is structurally `BoundaryExited`-only in v1 (param Defs hit the boundary `continue` before enqueue) — stated in the tool description so it reads as contract, not defect.
- **`path_proven: false` everywhere in v1** (claude's G11) — every path may contain line-granular hops while MINOR 7 is open; documented "reserved."
- **Frontier scoring** `1/(1+depth)`, sort score-desc then file/line (existing `queries.rs` precedents); one `TaintedBy` per reaching root straight from `frontier_by_root`.
- **Witness root** = lowest reaching root in BTreeMap order — deterministic, and now *attributed* via the reshaped `SinkResult`.
- **Boundary warnings** worded by `BoundaryEdge.kind` so intra-function pseudo-boundaries don't masquerade as interprocedural exits (claude's G9; contracted at `planA-followups.md:344–351`). Frontier mode fills the awkward `InterproceduralBoundary { sink }` field with the boundary target's `file:line:path` (constraint 2 forbids renaming; noted in docs).
- **JSON discriminant**: keep the externally-tagged `{"Reasoning":{...}}` nesting, ratified by snapshot tests in slice 1 (claude's G8 — "settle by freezing"; flattening would either unfreeze nav enums or require fragile hand-written `Serialize`).
- **All sources unresolved → `QueryError`** (codex): an empty success is too easily read by an LLM consumer as "analyzed and clean." (Claude's alternative — success-shaped with `source_count: 0` + warnings — is noted; see decision C.)
- **Memoization lands with witness mode**, not "if profiling bites" (claude's G10; contracted at `planA-followups.md:138–140`): lazy `BTreeMap<NodeIndex, BTreeSet<NodeIndex>>` keyed on distinct boundary targets, per `Trace`, shared by per-root and wrapper paths.
- **Do NOT add the continuation-scan function-boundary stop** — explicitly coupled to the multi-function `cfg_set` and contracted to land together (`planA-followups.md:384–390`); adding it alone breaks the nested-callback fix.

## Risks

- **Function-identity change touches trace internals** with pinned tests — the costliest foundation item; sequence it as its own commit inside slice 3 so a bisect isolates it.
- **Oracle's registration-order assumption** (NodeIndex rank ↔ byte rank) is the one *new* invariant this design adds — dedicated pinning test; failure mode is conservative-keep, not unsoundness.
- **Items/graph clip in lockstep** in `build_result` (one `retained` counter, `output.rs:142–174`); witness mode is the first Evidence carrying both — slice 8 must test the both-populated byte-pressure case.
- Deferred, safe-direction: strong-update/kill, MAJOR 2 CFG body-entry for param seeds, interprocedural chase, MINOR 7 full byte-range scoping.

## Slices + build order

1. **Wire freeze:** vocabulary relocation (#7) + `Evidence::new` (#8, 11 sites) + `SinkResult` reshape (#10, per decision A) + `aggregate` + `repair_after_clip` + discriminant snapshot tests. Option-C proof.
2. **Seed resolution** + truth-table matrix (symbol/loc × source/sink × found/empty/ambiguous/zero-param).
3. **Trace foundation:** `trace_root` extraction; `taint_trace_nodes` with line-derived degrade-guard parity tests (named regression: single-node seed on the shared-line fixture must still degrade); `var_node_for_location`; `BoundaryEdge.kind`; function-node identity (own commit).
4. **Per-root accessors** + wrapper-delegation parity + multi-seed attribution test + boundary-closure memo.
5. **Ordering oracle:** `order.rs` + `line_on_cfg_cycle` + the round-6 counterexample / round-9 loop-carried / registration-order test trio.
6. **Frontier mode** (shippable alone — satisfies the frontier half of the contract).
7. **Witness mode:** sink-candidate promotion, per-sink per-source verdicts, aggregate, union graph + `graph_node`.
8. **MCP surface:** schema/handler/registration; clip-repair tests incl. both-populated byte-pressure; seven-served + reasoning-absent-from-nav_v1 transport tests; `planA-followups.md` disposition update naming what closed (per-root API, node-precise seeding, ordering gate, MAJOR 4, #7, #8, #10, `SinkUnresolved` analog, `BoundaryEdge.kind`, function-node identity, memoization) and what remains.

Slices 2–5 are pure foundation reusable by `dataflow_between` / `impact_of_change` / `what_missing`.

## DECISIONS FOR THE OWNER

- **A. `SinkResult` attribution shape.** (a) Codex's nested `sources: Vec<SinkSourceResult>` (per-source reachability + graph_node + sanitizers) vs (b) minimal `source: SymbolRef` naming the witness root. The doc mandates attribution before freeze either way. **Recommend (a):** round 8 explicitly asks for *multi-source* room, the type has zero emissions today, and (b) would re-freeze an underpowered shape — the exact mistake #10 warns about. Cost: per-source clip repair must recurse one level.
- **B. Tool name.** `taint_reaches` (contract name; future siblings `dataflow_between` etc. read naturally unprefixed) vs `reason_taint_reaches` (namespaces the catalog like `nav_*` as the reasoning family grows). **Recommend `taint_reaches`** — descriptions and read-only annotations already group the family, and the shorter name is cheaper for LLM consumers; but this is pure taste and now-or-never (renaming after first emission breaks consumers).
- **C. All-sources-soft-failed semantics.** Hard `QueryError` (codex; unmissable) vs success-shaped Evidence with `source_count: 0` + `SeedUnresolved` warnings (claude; lets a batch caller distinguish "bad seeds" from transport failure). **Recommend hard error** — the false-"clean" misread is the costlier failure for a defect finder — but this is wire-visible and should be snapshot-frozen with the rest.

**Readiness verdict:** ready to plan after deciding A–C (A is the only one that shapes slice 1; B and C can be decided as late as slices 7–8).