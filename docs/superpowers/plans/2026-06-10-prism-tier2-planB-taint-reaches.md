# Prism Tier 2 Plan B — `taint_reaches` Implementation Plan

> **Status:** Revision 3 (2026-06-10) — folded the prism-wired **clean-room** (codex/gpt-5.5 + claude/fable, firewalled, on the merged substrate; `docs/prism-query-layer/taint-reaches-cleanroom-merged-MCP-2026-06-10.md`) on top of revision 2 (which folded the plan-review). The clean-room validated the orchestration spine and surfaced items rev 2 deferred that are **now-or-never for the first emitter** — pulled into a foundation-first slice order. **Owner decisions locked:** A = nested per-source `SinkResult`; B = tool name `taint_reaches` (unprefixed); C = all-sources-unresolved → hard `QueryError`.

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use `- [ ]` checkboxes.

**Goal:** Ship `taint_reaches` — a read-only MCP tool + library fn answering, over Plan A's A3 BFS, "does taint from these sources reach these sinks (witness mode, tri-state + witness) / where does it flow (frontier mode, scored items)" — additively (Option C: `cli_nav_compat` byte-identical; production Taint untouched). It is the first Tier-2 reasoning tool and the first thing that **serializes a reasoning witness**, so the wire shape and witness correctness must be settled here.

**Architecture:** Pure orchestration over four merged seams — **seed resolution (`seeds.rs`) → node-precise A3 trace (`trace.rs`) → per-root A7 classification/witness (`shape.rs`) → `Evidence` + the MCP `shape_result` shaper.** `Trace` is the only intermediate; no overlay state. The library returns a **pre-cap** `Evidence`; capping + nested `graph_node` repair happen in `shape_result`/`build_result`. Because this tool serializes witnesses, the foundation lands the **witness-ordering gate** (so a witness can't contain an order-infeasible same-line def→use hop), the **`SinkResult` multi-source reshape**, **function-node identity** (no same-name cross-function witness), and **`BoundaryEdge.kind`** (honest boundary warnings) — all before any emission.

**Tech Stack:** Rust, the merged Plan A substrate, `serde_json`, the MCP adapter (`src/mcp/*`, cargo `mcp` feature).

**Source spec:** `…/2026-06-09-prism-tier2-taint-reaches-design.md` (rev 3); **clean-room** `…/taint-reaches-cleanroom-merged-MCP-2026-06-10.md`; **plan-review** `…/planB-plan-review-MCP-2026-06-10.md`; **followups** `…/planA-followups.md` (the contracted-to-Plan-B list).

## Substrate surface (verified on merged `main` + plan-review)
*(unchanged from rev 2 — see git history of this file for the full table; key facts repeated where a slice depends on them.)*
- `taint_trace(&[(file,line)]) -> Trace`; `Trace { frontier_by_root, parents_by_root, boundary: BTreeSet<BoundaryEdge>, degraded, warnings }`; `BoundaryEdge { root, from, to }`; `Relation { DataFlow, AssignmentPropagation, RecoveredDefUse }`.
- Merged reasoning vocab in `src/navigation/types.rs` (relocates in Slice 1): `Reachability`, `SinkResult { sink, reachability, graph_node }` (**reshaped in Slice 1**), `ReasoningSummary`, `ReasoningReason::TaintedBy { source, sanitizers_present_in_source_fn, path_proven }`, `ReasoningWarning { SeedUnresolved { seed }, InterproceduralBoundary { sink }, Cleansed { source_function } }` (single-string), `Reason::Reasoning`/`WarningKind::Reasoning`, `Evidence.reasoning: Option<ReasoningSummary>`. `QueryError::{AmbiguousSymbol, UnsupportedFile, LocationOutOfRange}` exist; **never add a variant** (`error_text` exhaustive).
- `cleansed_categories_for_source(&BTreeMap<String,ParsedFile>, &VarLocation) -> Vec<String>` (`pub(crate)`, `taint.rs:10680`).
- `NavigationSession.repo: Arc<LoadedRepo>`; `LoadedRepo.files: BTreeMap<String,ParsedFile>` (`repo_loader.rs:33`).
- `ParsedFile::{find_function_by_name -> Option<Node>, function_parameter_names(&Node) -> Vec<String>, has_bare_references(&Node, &str) -> bool}` (AST methods, tree-sitter `Node`).
- `data_flow::VarAccessKind` vs `cpg::VarAccess` — **a conversion is required** for `VarLocation → NodeIndex`.
- MCP: `nav_v1()` frozen `== 6`; `MAX_RESULTS_DEFAULT`/`CAP` (`input.rs`); `SeedInput` (`to_triple()` is nav-lossy — don't use); **`pub fn shape_result(ev, retained, truncated, verbosity, max_results)`** (`output.rs:98`) is the public capping seam (`build_result :149` private); `Verbosity::Concise` clears `item.why`; `InMemoryTransport` `#[cfg(test)] new(Vec<&str>)` storing `Value`s, unit tests in `transport_tests.rs`; `tests/mcp/smoke_test.rs:24` asserts `== 6`.

## Pinned decisions (rev 3)
- **`SinkResult` (decision A — nested per-source; Slice 1 reshape):**
  ```rust
  pub struct SinkResult { pub sink: SymbolRef, pub reachability: Reachability, // worst-of across sources
      pub sources: Vec<SinkSourceResult> }
  pub struct SinkSourceResult { pub source: SymbolRef, pub reachability: Reachability,
      pub graph_node: Option<usize>, pub sanitizers_present_in_source_fn: Vec<String> }
  ```
  Top-level `reachability` = worst-of (`Reached` > `BoundaryExited` > `NotReached`, the `types.rs:157-162` rule). `graph_node` indexes the union witness `GraphPayload` (= `Evidence.graph`). Clip-repair recurses one level.
- **Tool name (B):** `taint_reaches` (library + MCP tool both `taint_reaches`, unprefixed).
- **All-sources-unresolved (C):** hard `QueryError` (precedence by seed-input order). Partial failure → `SeedUnresolved` warnings + proceed.
- **Witness-ordering gate:** the `RecoveredDefUse` same-line def→use arm is admitted into the **node-precise BFS** only when the use's source occurrence is **at-or-after** the def's (occurrence oracle), OR the line is on a CFG cycle (`line_on_cfg_cycle` — preserves the round-9 loop-carried fix on one-lined loop bodies). Oracle count-mismatch → conservative-keep + warning (over-report, never a false `NotReached`). Gate **only** the node-precise BFS; the unioned line-level path and the ungated classifier may only over-fire `BoundaryExited` (safe). The `AssignmentPropagation` arm is **not** gated (its cross-statement leak is the separately-deferred MINOR 7).
- **Function-node identity:** `taint_trace`'s `next_fn != src_fn` boundary test keys on the **containing `Function` node (by span)**, not the `(file, name)` string — so two same-named functions in one file (Rust `impl A`/`impl B` methods, C++ overloads) don't conflate into a false `Reached` with a cross-function witness.
- **`BoundaryEdge.kind { CrossFunction, SelfFunctionParam }`** — set at the branch point; `InterproceduralBoundary` warnings are worded off it so an intra-function one-line-signature pseudo-boundary isn't reported as an interprocedural exit.
- **`Evidence.graph` IS the witness union graph** for reasoning-bearing results (this tool never emits an ego/repo-map graph) — resolves the `graph_node` referent.
- **Frontier score:** `1/(1+depth)` (min BFS depth over sources; `queries.rs` precedent), sort score-desc then `(file,function,line,path,ordinal)`. One `TaintedBy` per reaching root; `TaintedBy.source` (singular) = the min-depth root, `BTreeMap` tie-break.
- **JSON discriminant:** keep externally-tagged `{"Reasoning":{...}}`; ratify with snapshot tests in Slice 1 (flattening would unfreeze nav enums).
- **Capping:** library returns pre-cap `Evidence`; `shape_result` caps + nested `graph_node` clip-repair; `max_results` default/cap = `MAX_RESULTS_DEFAULT`/`CAP`.
- **Memoization (boundary-closure):** **deferred** (perf/YAGNI; the clean-room mandated it, but it is not correctness and v1 sink counts are small). Add in Slice 7 only if a realistic-fixture witness test shows O(sinks×boundaries) hurting; documented in followups either way.
- **Vocab relocation #7 / `Evidence::new` #8:** Rust-module hygiene (no wire impact — JSON unchanged). #7 done in Slice 1 (cheap `pub use`); #8 deferred (note in followups) unless trivial.

## File Structure
```
src/reasoning/
  types.rs          NEW   reasoning vocab relocates here (#7); navigation/types.rs `pub use`s it
                          (the Evidence.reasoning FIELD stays in navigation for byte-compat);
                          nested SinkResult/SinkSourceResult; ReasoningSummary::aggregate;
                          repair_after_clip(kept) (recurses into per-source)
  seeds.rs          FILL  resolve(session, &[SeedSpec], role) -> Result<SeedSet, QueryError>;
                          ResolvedSeed gains `nodes: Vec<NodeIndex>`
  order.rs          NEW   SameLineOrder oracle over ParsedFile (occurrence↔node-rank) + count-mismatch guard
  shape.rs          EXT   reachability_for_node_from / witness_graph_for (union forms delegate); union_witness_graph
  taint_reaches.rs  NEW   the query (pure) -> Result<Evidence, QueryError>
  mod.rs            EDIT  pub mod {types, order, taint_reaches};
src/cpg/
  trace.rs          EXT   trace_root extraction; taint_trace_nodes(&[NodeIndex], Option<&dyn SameLineOrderView>);
                          line taint_trace delegates (None); BoundaryEdge.kind; function-node-identity boundary test
  query.rs          EXT   var_node_for_location(&VarLocation)->Option<NodeIndex> (+ VarAccessKind→VarAccess);
                          sink_candidate_nodes_at (promoted from shape.rs::sink_nodes_at)
  cfg_queries.rs    EXT   line_on_cfg_cycle(file,line)->bool  (cfg_reachable_lines self-membership doesn't work —
                          reachable_forward never re-enqueues the start, query.rs:100-106)
src/mcp/
  tools_reasoning.rs NEW  taint_reaches schema + parse(SeedInput→SeedSpec) + handler + register_reasoning() [mcp]
  output.rs         EXT   shape_result calls reasoning.repair_after_clip(kept) after the graph clip
  registry.rs/run   EDIT  reason_v1()/register_reasoning; serve nav_v1()+reason_v1()
  transport.rs/error.rs   wire-cap chokepoint (Slice 0)
Cargo.toml          EDIT  default-run; [[test]] path/required-features registrations
CLAUDE.md / types.rs doc  seven tools; Evidence.graph witness-mode note
```

---

## Slice 0 — Foundation (own commits): `default-run` + wire-cap
*(unchanged from rev 2 Tasks 0.1/0.2 — `default-run = "prism"` via `CARGO_BIN_EXE`; the `write_message` chokepoint with `MAX_RESULT_SIZE_CHARS = 1_000_000`, ASCII marker `"...[truncated N bytes]"`, error `_meta` cap, test in `src/mcp/transport_tests.rs` using `InMemoryTransport::new(vec![])` + a `written()` accessor. Option-C check.)*

## Slice 1 — Wire freeze (BEFORE any emission)

**Files:** Create `src/reasoning/types.rs`; modify `src/navigation/types.rs`, `src/mcp/output.rs`. Tests: `src/reasoning/types.rs` `#[cfg(test)]` + discriminant snapshots.

- [ ] **1a — relocate the reasoning vocab (#7).** Move `Reachability`/`SinkResult`/`SinkSourceResult`/`ReasoningSummary`/`ReasoningReason`/`ReasoningWarning` into `src/reasoning/types.rs`; `navigation/types.rs` does `pub use crate::reasoning::types::*;` (the `Evidence.reasoning` FIELD stays in navigation). **Test:** `cli_nav_compat` byte-identical (JSON unchanged — module move only).
- [ ] **1b — reshape `SinkResult` (decision A, #10).** To the nested `{sink, reachability, sources: Vec<SinkSourceResult>}` shape above. Nothing constructs it yet → emission-safe. **Test:** a snapshot of a hand-built nested `SinkResult` serializes as the agreed JSON.
- [ ] **1c — `ReasoningSummary::aggregate(per_sink) -> Option<Reachability>`** (worst-of across sinks; `None` if empty) + **`ReasoningSummary::repair_after_clip(&mut self, kept: usize)`** (set any `SinkSourceResult.graph_node >= kept` to `None`). **Tests:** aggregate worst-of; repair clears out-of-range nested indices.
- [ ] **1d — discriminant snapshot.** Pin `Reason::Reasoning(TaintedBy{..})` → `{"Reasoning":{"TaintedBy":{..}}}` and `WarningKind::Reasoning(SeedUnresolved{seed})` with snapshot tests, ratifying the externally-tagged shape.
- [ ] **Commit** — `feat(reasoning): wire freeze — relocate vocab, nested SinkResult, aggregate/repair, discriminant snapshots`. Option-C check.

## Slice 2 — Seed resolution (`seeds.rs`)
*(rev 2 Tasks 2.1–2.3, with `ResolvedSeed.nodes: Vec<NodeIndex>` added so the trace seeds nodes directly.)*
- [ ] **2a — `Loc`** → all `Variable` `VarLocation`s + their nodes on the line; 3 `None` cases → `UnsupportedFile`/`LocationOutOfRange`/empty-valid; zero-Variable → `SeedUnresolved{seed}` (merged shape). Never `resolve_fn`.
- [ ] **2b — `Symbol`=parameters** via the CPG→AST bridge: `repo.files[file].find_function_by_name(name)` → `function_parameter_names(&node)` → each param's `Def` `VarLocation`+node at the function start line; field-only skipped via `has_bare_references(&node, param)`; zero-param / all-field → empty+`SeedUnresolved`; **ambiguous (`file:None`, >1 match) → `AmbiguousSymbol` only when all sources ambiguous, else warn-skip.**
- [ ] **2c — failure precedence (decision C):** all sources fail → `QueryError` of the first seed-input-order failure (reuse existing variants); warnings seed-input-ordered, deduped.
- [ ] **Commits** per sub-task. Truth-table matrix tests (symbol/loc × source/sink × found/empty/ambiguous/zero-param).

## Slice 3 — Trace foundation
**Files:** `src/cpg/{trace.rs, query.rs}`. Tests: `src/cpg/tests.rs`.
- [ ] **3a — `trace_root` extraction + `taint_trace_nodes(&[NodeIndex], Option<&dyn SameLineOrderView>) -> Trace`.** The node entry **recomputes the line-derived degrade guards over ALL `Variable` nodes at each root's line** (NOT over the passed roots) — else a single-node seed on a shared-line fixture resurrects the round-6 false `NotReached`. **Named regression test:** single-node seed on the `int a(){…} int b(int p){…}` shared-line fixture still degrades. Line `taint_trace` delegates with `order=None` (behavior byte-identical — all A3 tests stay green).
- [ ] **3b — `var_node_for_location` (`query.rs`)** with the named `VarAccessKind → VarAccess` conversion (`Def→Def`, `Use→Use`). Test: round-trips a `VarLocation` to its node.
- [ ] **3c — `BoundaryEdge.kind { CrossFunction, SelfFunctionParam }`** set at the boundary branch (`next_fn != src_fn` → `CrossFunction`; `is_parameter_binding && same fn` → `SelfFunctionParam`). Existing boundary tests updated.
- [ ] **3d — function-node identity (OWN COMMIT, bisect-isolated).** Replace the `(file,name)` boundary comparison with containing-`Function`-node identity (by span). Add `containing_function_node(NodeIndex) -> Option<NodeIndex>` (innermost `Function` whose `[start,end]` contains the var's line; tie-break smallest span). **Test:** two same-named functions in one file (`impl A { fn f }` / `impl B { fn f }`) with a cross-fn DataFlow edge → recorded as a `CrossFunction` boundary, NOT traversed (no false `Reached`). All A3 tests green.
- [ ] **Commits** per sub-task.

## Slice 4 — Per-root accessors (`shape.rs`)
*(rev 2 Task 1.2.)* `reachability_for_node_from(cpg, trace, root, sink)` (consult `frontier_by_root[root]` + `b.root==root` boundaries) and `witness_graph_for(cpg, trace, root, sink)` (walk `parents_by_root` for `root`); union forms delegate. Real two-source fixture with assertions. *(Memoization deferred — see Pinned decisions.)*

## Slice 5 — Ordering oracle (`order.rs` + `line_on_cfg_cycle`)
**Files:** Create `src/reasoning/order.rs`; modify `src/cpg/cfg_queries.rs`, and the `RecoveredDefUse` admission in `taint_trace_nodes`.
- [ ] **5a — `line_on_cfg_cycle(file,line) -> bool`** (`cfg_queries.rs`): is the line's statement on a CFG cycle (reachable back to itself)? `cfg_reachable_lines` self-membership does NOT work (`reachable_forward` never re-enqueues the start). Test on a `while` loop body.
- [ ] **5b — `SameLineOrder` oracle (`order.rs`)**: over a `ParsedFile`, rank same-line `Variable` occurrences by source byte offset; expose `def_before_use(def_node, use_node) -> bool` by matching occurrence rank ↔ node rank. **Count-mismatch (occurrences ≠ nodes) → return `true` (conservative-keep) + a warning** — over-report, never a false `NotReached`.
- [ ] **5c — gate the BFS:** in `taint_trace_nodes`, admit a `RecoveredDefUse` `def→use` edge iff `order.def_before_use(def, use)` **or** `line_on_cfg_cycle(file, line)`. **Tests (the trio):** round-6 counterexample (`sink(y); var y=u;` → the backward witness hop is NOT synthesized → sink `NotReached` *or* `BoundaryExited`, never a corrupt `Reached`-with-backward-witness); round-9 loop-carried (`while: sink(o.data); o.data=input()` still `Reached` via the cycle carve-out); registration-order pin (the oracle's NodeIndex-rank↔byte-rank invariant).
- [ ] **Commit** — `feat(reasoning): same-line def→use ordering gate (occurrence oracle + CFG-cycle carve-out)`.

## Slice 6 — Frontier mode (shippable alone)
*(rev 2 Task 4.1, scoring `1/(1+depth)`, `TaintedBy` per min-depth root, items XOR graph, counts pre-cap, `reasoning.reachability=None`.)* Plus `taint_reaches` skeleton (`src/reasoning/taint_reaches.rs`, `mod.rs`): resolve sources → `taint_trace_nodes(src_nodes, Some(&order))` → frontier shaping. The A4 adapter (`cleansed_categories_for_source(&session.repo.files, &src_loc)`) fills `sanitizers_present_in_source_fn` (clears its dead-code warnings).

## Slice 7 — Witness mode
*(rev 2 Tasks 3.x/4.2/4.3, with nested `SinkResult`.)*
- [ ] Sink resolution via the promoted `sink_candidate_nodes_at` (`query.rs`); one `SinkResult` per resolved sink `VarLocation`; **per sink × per source** verdict via `reachability_for_node_from` → `SinkSourceResult{source, reachability, graph_node, sanitizers}`; top-level `reachability` = worst-of; `ReasoningSummary::aggregate` across sinks.
- [ ] Union witness graph (`Evidence.graph`), dedup by full node identity `(file,function,line,path,kind,ordinal)`, self-edges dropped, edge kinds `DataFlow`/`AssignmentPropagation`/`RecoveredDefUse`; `SinkSourceResult.graph_node` indexes it (pre-cap).
- [ ] Warnings: `SeedUnresolved{seed}` (input order); per queried-sink `BoundaryEdge` → `InterproceduralBoundary{sink}` **worded by `BoundaryEdge.kind`** (a `SelfFunctionParam` boundary is not called interprocedural); `Cleansed{source_function}`. The `S→I→K` interior invariant + cross-function `BoundaryExited` + all-sinks-fail (`QueryError`) tests. Option-C check.

## Slice 8 — MCP surface + clip-repair
*(rev 2 Tasks 5.1/5.2/5.2b/5.3/5.4, tool name `taint_reaches`.)*
- [ ] `reason_v1()`/`register_reasoning`; serve nav+reason; **update `tests/mcp/smoke_test.rs` 6→7** and assert `taint_reaches` is ABSENT from `nav_v1()` (frozen `==6` unit test untouched).
- [ ] `tools_reasoning.rs`: schema (`sources` required `minItems:1`; `sinks` optional, empty `[]` invalid; `max_results`; `verbosity` default `concise`); **`SeedInput → SeedSpec`** conversion (not `to_triple`); dispatch → `taint_reaches` → public `shape_result(.., max_results)`. Description: params-only scope, steer to `per_sink`, **`verbosity:detailed` for frontier rationale** (concise clears `item.why`).
- [ ] **`shape_result` nested clip-repair:** after the graph clip, `reasoning.repair_after_clip(kept)` (recurses into `SinkSourceResult.graph_node`). Test: no `graph_node` (at either level) points past `graph.nodes.len()`; `per_sink` stays complete (the contracted clipped-witness serialization test). Both-populated (items+graph) byte-pressure test.
- [ ] §11 boundary-bypass regression guard; CLAUDE.md seven-tools + `Evidence.graph` witness-mode doc.
- [ ] **followups disposition update** naming what closed (per-root API, node-precise seeding, ordering gate, MAJOR 4 graph_node, #7, #10, `BoundaryEdge.kind`, function-node identity, `SinkUnresolved` analog) and what remains (memoization, #8 `Evidence::new`, strong-update/kill, MINOR 7 full byte-range, interprocedural chase).

---

## Recurring gate
`cargo test --test cli_nav_compat` **byte-identical** (NOT the `review` preset) + `algo_taint_cve` + the frozen six-tool registry test, after every slice. `cargo fmt && cargo test` + `--features mcp` green. Re-warm the prism cache before any prism-wired review.

## Self-Review
**Spec + clean-room + followups coverage:** wire-shape now-or-never (Slice 1) · seed truth table (Slice 2) · node-precise + identity + BoundaryEdge.kind foundation (Slice 3) · per-root (Slice 4) · ordering gate (Slice 5, **the round-6 mandate**) · frontier (Slice 6) · witness (Slice 7) · MCP + clip-repair (Slice 8). Decisions A/B/C threaded. Deferred-with-reason: memoization (perf), #8 `Evidence::new` (hygiene), strong-update/kill, MINOR 7 full byte-range, interprocedural — all safe-direction / non-emitting.

**Type consistency:** `taint_trace_nodes(&[NodeIndex], Option<&dyn SameLineOrderView>)`; `var_node_for_location`; `reachability_for_node_from`/`witness_graph_for`; nested `SinkResult`/`SinkSourceResult` + `aggregate`/`repair_after_clip`; `resolve(session,&[SeedSpec],role)->Result<SeedSet,QueryError>`; `taint_reaches(session,&[SeedSpec],Option<&[SeedSpec]>)->Result<Evidence,QueryError>` (pre-cap); `shape_result(ev,retained,truncated,verbosity,max_results)`.

## Execution Handoff
**Plan rev 3 — clean-room + plan-review + decisions A/B/C folded; foundation-first.** The architecture shifted materially from rev 2 (foundation now does the wire freeze + ordering gate + identity), so the recommended next gate is a **short re-plan-review of rev 3** (a2a-bridge, prism-wired, gpt-5.5/fable) to confirm the new foundation slices are executable, THEN containerized codex implement (TDD, verify=fmt+build+test --locked) / subagent-driven TDD → in-depth code-review vs main → squash to docs+feat pair → merge.
