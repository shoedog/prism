# Prism Tier 2 Plan B — `taint_reaches` Implementation Plan

> **Status:** Revision 4 (2026-06-10) — folded the **rev-3 re-plan-review** (codex/gpt-5.5 + claude/fable, prism-wired, focused on the new foundation slices; `docs/archive/review-artifacts/prism-query-layer/planB-rev3-plan-review-MCP-2026-06-10.md`). It caught that the rev-3 ordering oracle was **unimplementable as node-rank** (CPG Variable nodes carry no byte/ordinal) and would *certify* the round-6 corruption — respecified **AST-based** (B1); plus compile-order gaps (`pub mod types` in 1a / `SameLineOrderView` trait in 3a), the function-identity name+span+fallback (M1), the both-`RecoveredDefUse`-arms gate (M4), the clip-repair-inside-`build_result` site (m2), the one-lined-`while` self-loop cycle (m3), and `SymbolNotFound` for all-sources-fail (m1). Slice order/coverage judged sound. (Rev 3 folded the firewalled clean-room + owner decisions; rev 2 folded the first plan-review.) **Owner decisions locked:** A = nested per-source `SinkResult`; B = `taint_reaches` (unprefixed); C = all-sources-unresolved → hard `QueryError::SymbolNotFound`.
>
> **STATUS: HOLD before implementation** — the containerized `a2a-bridge implement` step is paused while a2a-bridge migrates its container runtime (orbstack → podman) to relieve memory pressure. Planning is complete (writing-plans → plan-review → clean-room → re-plan-review, all folded); resume at implementation when podman is validated.

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use `- [ ]` checkboxes.

**Goal:** Ship `taint_reaches` — a read-only MCP tool + library fn answering, over Plan A's A3 BFS, "does taint from these sources reach these sinks (witness mode, tri-state + witness) / where does it flow (frontier mode, scored items)" — additively (Option C: `cli_nav_compat` byte-identical; production Taint untouched). It is the first Tier-2 reasoning tool and the first thing that **serializes a reasoning witness**, so the wire shape and witness correctness must be settled here.

**Architecture:** Pure orchestration over four merged seams — **seed resolution (`seeds.rs`) → node-precise A3 trace (`trace.rs`) → per-root A7 classification/witness (`shape.rs`) → `Evidence` + the MCP `shape_result` shaper.** `Trace` is the only intermediate; no overlay state. The library returns a **pre-cap** `Evidence`; capping + nested `graph_node` repair happen in `shape_result`/`build_result`. Because this tool serializes witnesses, the foundation lands the **witness-ordering gate** (so a witness can't contain an order-infeasible same-line def→use hop), the **`SinkResult` multi-source reshape**, **function-node identity** (no same-name cross-function witness), and **`BoundaryEdge.kind`** (honest boundary warnings) — all before any emission.

**Tech Stack:** Rust, the merged Plan A substrate, `serde_json`, the MCP adapter (`src/mcp/*`, cargo `mcp` feature).

**Source spec:** `…/2026-06-09-prism-tier2-taint-reaches-design.md` (rev 3); **clean-room** `…/taint-reaches-cleanroom-merged-MCP-2026-06-10.md`; **plan-review** `…/planB-plan-review-MCP-2026-06-10.md`; **followups** `…/planA-followups.md` (the contracted-to-Plan-B list).

## Substrate surface (verified on merged `main` + plan-review)
*(unchanged from rev 2 — see git history of this file for the full table; key facts repeated where a slice depends on them.)*
- `taint_trace(&[(file,line)]) -> Trace`; `Trace { frontier_by_root, parents_by_root, boundary: BTreeSet<BoundaryEdge>, degraded, warnings }`; `BoundaryEdge { root, from, to }`; `Relation { DataFlow, AssignmentPropagation, RecoveredDefUse }`.
- Merged reasoning vocab in `src/navigation/types.rs` (relocates in Slice 1): `Reachability`, `SinkResult { sink, reachability, graph_node }` (**reshaped in Slice 1**), `ReasoningSummary`, `ReasoningReason::TaintedBy { source, sanitizers_present_in_source_fn, path_proven }`, `ReasoningWarning { SeedUnresolved { seed }, InterproceduralBoundary { sink }, Cleansed { source_function } }` (single-string), `Reason::Reasoning`/`WarningKind::Reasoning`, `Evidence.reasoning: Option<ReasoningSummary>`. `QueryError::{AmbiguousSymbol, UnsupportedFile, LocationOutOfRange, SymbolNotFound{seed}}` exist (`navigation/types.rs:181-188`); **never add a variant** (`error_text` exhaustive).
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
- **All-sources-unresolved (C):** hard `QueryError` (precedence by seed-input order). Partial failure → `SeedUnresolved` warnings + proceed. **The mapping for all-sources-fail is `QueryError::SymbolNotFound{seed}`** (`navigation/types.rs:181-188`, an EXISTING variant — never add one); also used for all-empty Loc seeds.
- **Witness-ordering gate (AST-based oracle — rev-4 respec, plan-review B1/M2/M3/M4):** ANY `RecoveredDefUse` edge **whose def and use share a line** is admitted into the **node-precise BFS** only when the use's source occurrence is **at-or-after** the def's, OR the line is on a CFG cycle (`line_on_cfg_cycle`, round-9 carve-out). **The oracle is AST-based, NOT node-rank-based:** `CpgNode::Variable` carries no byte/column/ordinal (`cpg/types.rs:28-35`) and node insertion order is access-kind-major + name-sorted (`build.rs:220-271`), so node rank can NEVER recover byte order. Instead, over the `ParsedFile`, classify each same-line occurrence **syntactically** (an identifier in a declarator / assignment-LHS = def-position; all others = use-position; a field path `o.data` matches the path's `base`+`fields` text, not a bare identifier); `def_before_use` = **any** use occurrence at-or-after **any** def occurrence (conservative over the deduped node granularity). The view is **repo-wide** (files + the CPG metadata needed to map a `NodeIndex` → its file/line/path), dispatching by the node's file (sources span files). Count-mismatch / unparseable → **conservative-keep + a warning routed to `Trace.warnings`** (over-report, never a false `NotReached`) — so the gate returns a structured `{ admit: bool, warning: Option<String> }`, not a bare bool. Gate covers BOTH `RecoveredDefUse` sub-arms (the simple-path same-line arm AND the same-line subset of the field-path "any-line" arm — `trace.rs:249-272,:311-380` — since `cfg_valid` admits same-line targets unconditionally, `:439`). Gate **only** the node-precise BFS; the line wrapper / classifier may only over-fire `BoundaryExited` (safe). The `AssignmentPropagation` arm is NOT gated (MINOR 7).
- **Function-node identity (rev-4 refine, plan-review M1):** `taint_trace`'s `next_fn != src_fn` boundary test keys on the **containing `Function` node**, not the `(file, name)` string — so same-named functions in one file (Rust `impl A`/`impl B`, C++ overloads) don't conflate into a false `Reached` with a cross-function witness. `containing_function_node(var) `: among `Function` nodes whose name == the Variable's `function` string, pick the **innermost by span** (the name filter avoids mis-assigning nested-named-function overlapping spans — the round-7 fixture shape — into false `CrossFunction` boundaries / A3 golden drift). **Do NOT route through `func_index`** (keyed `(file,name)`, last-writer-wins, `build.rs:211` — collapses the two `impl::f`s; `function_nodes()` reads it, `query.rs:73-76`). **Fallback when no containing `Function` node exists** (module-level vars, call-graph misses): the current `(file,function)` string comparison.
- **`BoundaryEdge.kind { CrossFunction, SelfFunctionParam }`** — set at the branch point; `InterproceduralBoundary` warnings are worded off it so an intra-function one-line-signature pseudo-boundary isn't reported as an interprocedural exit.
- **`Evidence.graph` IS the witness union graph** for reasoning-bearing results (this tool never emits an ego/repo-map graph) — resolves the `graph_node` referent.
- **Frontier score:** `1/(1+depth)` (min BFS depth over sources; `queries.rs` precedent), sort score-desc then `(file,function,line,path,ordinal)`. One `TaintedBy` per reaching root; `TaintedBy.source` (singular) = the min-depth root, `BTreeMap` tie-break.
- **JSON discriminant:** keep externally-tagged `{"Reasoning":{...}}`; ratify with snapshot tests in Slice 1 (flattening would unfreeze nav enums).
- **Capping (rev-4 correction, plan-review m2):** library returns pre-cap `Evidence`; the nested `graph_node` clip-repair runs **inside private `build_result`** (`output.rs:166-174`) — between the graph clip and `render`, on the shaped clone — NOT after `shape_result` returns (`shape_result` calls `build_result` repeatedly inside a serialize-each-candidate binary search, `:120-137,:192-193`). `max_results` default/cap = `MAX_RESULTS_DEFAULT`/`CAP`.
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
  order.rs          NEW   AST-based same-line occurrence oracle (repo-wide: files + CPG metadata);
                          syntactic def/use-position classification; conservative-keep + Trace.warnings
                          (impls the SameLineOrderView trait declared in cpg/trace.rs Slice 3a)
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

- [ ] **1a — relocate the reasoning vocab (#7).** **First add `pub mod types;` to `src/reasoning/mod.rs`** (it currently exports only `seeds`/`shape`, `mod.rs:4-5` — without this the `pub use` below doesn't compile). Move `Reachability`/`SinkResult`/`SinkSourceResult`/`ReasoningSummary`/`ReasoningReason`/`ReasoningWarning` into `src/reasoning/types.rs`; `navigation/types.rs` does `pub use crate::reasoning::types::*;` (the `Evidence.reasoning` FIELD stays in navigation). **Test:** `cli_nav_compat` byte-identical (JSON unchanged — module move only).
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
- [ ] **3a — define the `SameLineOrderView` trait + `trace_root` extraction + `taint_trace_nodes`.** **Define the one-method trait `pub trait SameLineOrderView { fn admit_same_line_def_use(&self, def: NodeIndex, use_: NodeIndex) -> (bool, Option<String>); }` HERE** (plan-review B3 — a `&dyn SameLineOrderView` parameter type must have the trait defined to compile, even when callers pass `None`; the impl lands in Slice 5). Then `taint_trace_nodes(&[NodeIndex], Option<&dyn SameLineOrderView>) -> Trace`: the node entry **recomputes the line-derived degrade guards over ALL `Variable` nodes at each root's line** (NOT over the passed roots) — else a single-node seed on a shared-line fixture resurrects the round-6 false `NotReached`. **Named regression test:** single-node seed on the `int a(){…} int b(int p){…}` shared-line fixture still degrades. Line `taint_trace` delegates with `order=None` (byte-identical — all A3 tests stay green).
- [ ] **3b — `var_node_for_location` (`query.rs`)** with the named `VarAccessKind → VarAccess` conversion (`Def→Def`, `Use→Use`). Test: round-trips a `VarLocation` to its node.
- [ ] **3c — `BoundaryEdge.kind { CrossFunction, SelfFunctionParam }`** set at the boundary branch (`next_fn != src_fn` → `CrossFunction`; `is_parameter_binding && same fn` → `SelfFunctionParam`). Existing boundary tests updated.
- [ ] **3d — function-node identity (OWN COMMIT, bisect-isolated; plan-review M1).** Replace the `(file,name)` boundary comparison with containing-`Function`-node identity. `containing_function_node(var: NodeIndex) -> Option<NodeIndex>`: among `Function` nodes whose **name == the Variable's `function` string** (the name filter FIRST — pure innermost-span would mis-assign nested-named-function overlapping spans, the round-7 fixture shape, into false `CrossFunction` boundaries / A3 golden drift), pick the **innermost by span**. **Do NOT route through `func_index`** (keyed `(file,name)`, last-writer-wins `build.rs:211`; collapses `impl A::f`/`impl B::f`). **Fallback (no containing `Function` node — module-level vars / call-graph misses): the current `(file,function)` string comparison.** **Tests:** two same-named functions in one file (`impl A { fn f }` / `impl B { fn f }`) with a cross-fn DataFlow edge → `CrossFunction` boundary, NOT traversed (no false `Reached`); a nested-named-function fixture stays intra-function (no false boundary). All A3 tests green. **Doc:** `forward_reachable_in_function` stays name-keyed (safe — over-fires only `BoundaryExited`); update the "BFS and classifier cannot diverge" comments at `trace.rs:286-289` + `shape.rs:56-58` to document the deliberate asymmetry (plan-review m4).
- [ ] **Commits** per sub-task.

## Slice 4 — Per-root accessors (`shape.rs`)
*(rev 2 Task 1.2.)* `reachability_for_node_from(cpg, trace, root, sink)` (consult `frontier_by_root[root]` + `b.root==root` boundaries) and `witness_graph_for(cpg, trace, root, sink)` (walk `parents_by_root` for `root`); union forms delegate. Real two-source fixture with assertions. *(Memoization deferred — see Pinned decisions.)*

## Slice 5 — Ordering oracle (`order.rs` + `line_on_cfg_cycle`) — AST-based (rev-4 respec)
**Files:** Create `src/reasoning/order.rs`; modify `src/cpg/cfg_queries.rs`, and the `RecoveredDefUse` admission in `taint_trace_nodes`. Implements the `SameLineOrderView` trait from Slice 3a.
- [ ] **5a — `line_on_cfg_cycle(file,line) -> bool`** (`cfg_queries.rs`): is the line's Statement node on a CFG cycle (reachable back to itself over ControlFlow edges)? `cfg_reachable_lines` self-membership does NOT work (`reachable_forward` never re-enqueues the start, `query.rs:96-106`). **A self-edge `n→n` MUST count as a cycle** (plan-review m3 — a fully one-lined `while` is a ControlFlow self-loop `from_line==to_line`, `cfg.rs:269-285` / `build.rs:431-441`; it's the only thing keeping round-9 semantics on one-liners after 5c). **Tests:** multi-line `while` body; **one-lined `while` (self-loop)**.
- [ ] **5b — the AST-based occurrence oracle (`order.rs`) — NOT node-rank (plan-review B1).** `CpgNode::Variable` has no byte/column/ordinal and node insertion is access-kind-major + name-sorted, so node rank can't recover byte order. Build a **repo-wide** view (plan-review M3 — sources span files; it needs CPG metadata to map a `NodeIndex` → its file/line/path, plus the `ParsedFile`s). For a given line, walk the `ParsedFile`'s tree-sitter nodes on that line and **classify each occurrence syntactically**: an identifier in a declarator / assignment-LHS position = **def-occurrence**; all others = **use-occurrence**; a field path matches the access path's `base`+`fields` text (not a bare identifier). `admit_same_line_def_use(def, use_) -> (bool, Option<String>)` returns `true` iff **any** use-occurrence is **at-or-after any** def-occurrence (conservative over deduped node granularity). **Unparseable / count-mismatch → `(true, Some(warning))`** (conservative-keep; the warning is **routed to `Trace.warnings`** by the gate, plan-review M2) — over-report, never a false `NotReached`. **Tests:** `var y = u; sink(y)` admits; `sink(y); var y = u` does NOT; field-path `o.x` ranked by base+field.
- [ ] **5c — gate the BFS (BOTH `RecoveredDefUse` arms; plan-review M4):** in `taint_trace_nodes`, for **any `RecoveredDefUse` edge whose def and use share a line** (the simple-path same-line arm AND the same-line subset of the field-path "any-line" arm — `trace.rs:249-272,:311-380`; `cfg_valid` admits same-line targets unconditionally `:439`), admit iff `order.admit_same_line_def_use(def, use).0` **or** `line_on_cfg_cycle(file, line)`; drain the oracle's warning into `Trace.warnings`. **Tests (the trio + field variant):** round-6 (`sink(y); var y=u;` → backward hop NOT synthesized → never a corrupt `Reached`-with-backward-witness); **field-path round-6 (`sink(o.x); o.x=u;`** → same, in field clothing); round-9 loop-carried (`while: sink(o.data); o.data=input()` still `Reached` via the cycle carve-out, incl. the one-lined-`while` self-loop). **Drop the old "registration-order pin" test — the node-rank↔byte-rank invariant it asserted is FALSE.**
- [ ] **Commit** — `feat(reasoning): AST-based same-line def→use ordering gate (occurrence oracle + CFG-cycle carve-out, both arms)`.

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
- [ ] **nested clip-repair INSIDE `build_result` (plan-review m2):** the graph clip lives in private `build_result` (`output.rs:166-174`), which `shape_result` calls repeatedly inside a serialize-each-candidate binary search — so `reasoning.repair_after_clip(kept)` must run **inside `build_result`, on the shaped clone, between the graph clip and `render`**, NOT after `shape_result` returns. Test: no `graph_node` (top-level absent; nested `SinkSourceResult.graph_node`) points past `graph.nodes.len()`; `per_sink` stays complete (the contracted clipped-witness serialization test). Both-populated (items+graph) byte-pressure test.
- [ ] §11 boundary-bypass regression guard; CLAUDE.md seven-tools + `Evidence.graph` witness-mode doc.
- [ ] **followups disposition update** naming what closed (per-root API, node-precise seeding, ordering gate, MAJOR 4 graph_node, #7, #10, `BoundaryEdge.kind`, function-node identity, `SinkUnresolved` analog) and what remains (memoization, #8 `Evidence::new`, strong-update/kill, MINOR 7 full byte-range, interprocedural chase).

---

## Recurring gate
`cargo test --test cli_nav_compat` **byte-identical** (NOT the `review` preset) + `algo_taint_cve` + the frozen six-tool registry test, after every slice. `cargo fmt && cargo test` + `--features mcp` green. Re-warm the prism cache before any prism-wired review.

## Self-Review
**Spec + clean-room + followups coverage:** wire-shape now-or-never (Slice 1) · seed truth table (Slice 2) · node-precise + identity + BoundaryEdge.kind foundation (Slice 3) · per-root (Slice 4) · ordering gate (Slice 5, **the round-6 mandate**) · frontier (Slice 6) · witness (Slice 7) · MCP + clip-repair (Slice 8). Decisions A/B/C threaded. Deferred-with-reason: memoization (perf), #8 `Evidence::new` (hygiene), strong-update/kill, MINOR 7 full byte-range, interprocedural — all safe-direction / non-emitting.

**Type consistency:** `taint_trace_nodes(&[NodeIndex], Option<&dyn SameLineOrderView>)`; `var_node_for_location`; `reachability_for_node_from`/`witness_graph_for`; nested `SinkResult`/`SinkSourceResult` + `aggregate`/`repair_after_clip`; `resolve(session,&[SeedSpec],role)->Result<SeedSet,QueryError>`; `taint_reaches(session,&[SeedSpec],Option<&[SeedSpec]>)->Result<Evidence,QueryError>` (pre-cap); `shape_result(ev,retained,truncated,verbosity,max_results)`.

## Execution Handoff
**Plan rev 4 — planning COMPLETE.** Gates run and folded: writing-plans (rev 1) → plan-review (rev 2) → firewalled clean-room (rev 3) → re-plan-review of the new foundation slices (rev 4). Both re-plan-review lenses' remaining concerns are resolved (AST-based oracle, compile-order, identity name+span+fallback, both-arms gate, clip-repair site). **No further planning gate is needed before implementation.**

**HOLD:** implementation is paused pending a2a-bridge's orbstack → podman container-runtime migration (memory pressure). When podman is validated, resume with **containerized `a2a-bridge implement`** (codex, TDD, verify = `cargo fmt --check` + `cargo build --locked` + `cargo test --locked` + the Option-C `cli_nav_compat` proof, implement-review loop) — OR subagent-driven TDD in-session if a containerless path is preferred. Then in-depth code-review vs main (gpt-5.5/fable) to convergence → squash to docs+feat pair → merge. Slice 6 (frontier mode) is independently shippable if a smaller first cut is wanted.
