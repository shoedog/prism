Looking at these two designs, I'll synthesize them now — reading specific code points only where the two architects diverge on factual claims.The key divergence is confirmed. Now I have enough to synthesize.

---

# SYNTHESIZED DESIGN: Prism Reasoning Substrate (DFG · CFG · CPG)

---

## Convergent Spine

Both architects independently reached the same conclusions on these points — treat them as high-confidence ground truth:

1. **Split `src/cpg.rs` first.** 3306 lines; prerequisite for everything else. Mechanical, no behavior change.
2. **Star-paths are P0 for Tier-2 v1.** Both `DataFlowGraph::taint_forward()` and `CpgContext::taint_forward_cfg()` emit source→each-target as independent `FlowEdge`s. An Evidence graph built on star-paths cannot show taint chains, cannot place sanitizer markers on intermediate steps, and is structurally misleading to an agent.
3. **Sanitizer fix is two phases, not one.** Every framework's `sanitizers: &[]`. Wiring traversal to consult empty recognizers is vacuously correct. Phase A (populate) and Phase B (wire) must ship together.
4. **Return-value DFG edges are absent.** `cpg.rs:862–940` Step 5b adds arg→param only. The cross-function bypass in `taint_forward_cfg` (`cpg.rs:2043–2045`) is effectively dead for return-value propagation.
5. **Rust `?` early-return CFG edge is missing.** `cfg.rs:527–532` documents the omission explicitly. High value, low effort.
6. **`src/reasoning/` is zero lines of code.** The spec assumes it exists; it does not. All substrate hardening is a prerequisite.
7. **`DataFlowGraph` has no unit tests** (`data_flow.rs` — confirmed 0 `#[cfg(test)]` occurrences). The D4 fix requires changing `forward_reachable`'s return type; tests must accompany it.
8. **`CpgEdge::FieldOf` is defined but never added.** `cpg.rs:476` + `cpg.rs:2178`. Either add edges or delete the variant.
9. **`ToolRegistry::nav_v1` lists exactly 6 tools** and must not be extended for Tier-2. Both architects independently concluded: add a separate registry.

---

## Component/File Boundaries

### Immediate: Split `src/cpg.rs`

```
src/cpg/types.rs       — CpgNode, CpgEdge, StmtKind, VarAccess (~165 lines)
src/cpg/build.rs       — assemble_graph(), Steps 1–9 (~600 lines)
src/cpg/query.rs       — traversal/query methods (~500 lines)
src/cpg/context.rs     — CpgContext, CpgScope, build_registry (~330 lines)
src/cpg/cfg_queries.rs — taint_forward_cfg, dfg_cfg_chop, cfg_reachable_lines (~200 lines)
src/cpg.rs             — re-export façade only
```

### New: `src/reasoning/` (created after substrate hardening)

```
src/reasoning/mod.rs
src/reasoning/options.rs
src/reasoning/flow.rs      — FlowStep, FlowTrace, chain BFS
src/reasoning/interproc.rs — return-to-call-result, receiver-to-self
src/reasoning/cfg.rs       — typed CFG edge kinds
src/reasoning/mcp.rs       — ToolRegistry::reasoning_v1()
```

### CFG typed edges (additive alongside existing)

```rust
// New — does NOT change existing build_cfg_edges() or CpgEdge::ControlFlow
pub enum CfgEdgeKind {
    Sequential, BranchTrue, BranchFalse, LoopBack,
    Exception, Finally, Goto, Fallthrough, EarlyReturn, Defer,
}

pub fn build_cfg_edges_typed(parsed: &ParsedFile) -> Vec<TypedCfgEdge>;
// Old function delegates to new, strips kind field — no CPG behavior change.
```

---

## Key Interfaces/Types

### `src/reasoning/options.rs`

```rust
pub struct ReasoningOptions {
    pub max_call_depth: usize,
    pub max_paths: usize,
    pub include_arg_param: bool,
    pub include_return_to_call_result: bool,
    pub include_receiver_to_self: bool,
    pub use_type_dispatch: bool,
    pub exact_same_line_assignments: bool,
    pub include_cfg_edge_kinds: bool,
}
```

### `src/reasoning/flow.rs`

```rust
pub enum ReasoningEdgeKind {
    LocalDefUse,
    ExactAssignment,
    LegacySameLineAssignment,               // for compatibility/debugging
    ArgumentToParameter { call_line: usize, arg_index: usize },
    ReturnToCallResult { call_line: usize, return_line: usize },
    ReceiverToSelf { call_line: usize },
    Control(CfgEdgeKind),
    Sanitizer { category: SanitizerCategory },
}

pub struct FlowStep {
    pub from: VarLocation,
    pub to: VarLocation,
    pub kind: ReasoningEdgeKind,
    pub confidence: FlowConfidence,
}

pub struct FlowTrace {
    pub steps: Vec<FlowStep>,              // actual chain, not star
    pub cleansed_for: BTreeSet<SanitizerCategory>,
    pub warnings: Vec<String>,
    pub truncated: bool,
}
```

---

## The Flow (Build Sequence)

1. Build `CpgContext` exactly as today — no changes to `DataFlowGraph::build`, `CodePropertyGraph::build`, or `CpgEdge`.
2. Construct `ReasoningGraph` from `ctx.files`, `ctx.cpg.call_graph`, `ctx.cpg.dfg`, and `ctx.live_types`. Do **not** mutate `ctx.cpg.graph`.
3. Store reasoning edge metadata in `ReasoningGraph`, not in `CpgEdge`.
4. `taint_reaches` calls `ReasoningGraph::trace_forward(source, options)` → `FlowTrace` → maps to `GraphPayload` for Evidence.

---

## Decisions + Rationale per Axis

### DATA FLOW GRAPH

**D4 — Star-paths → Chain BFS (P0)**

Replace `DataFlowGraph::forward_reachable()` with a BFS tracking `(node, Option<predecessor>)`. Build a parent map; reconstruct concrete source→sink chains. Apply to both `taint_forward()` (`data_flow.rs:593`) and `taint_forward_cfg()` (`cpg.rs:2058`). The `FlowPath.edges` field already has the right shape — the constructor changes from "all reachable" to "chain BFS."

*Option-C impact:* No golden tests check `FlowPath.edges` shape directly; algorithm tests check reachable line *sets* only. Safe to change; add chain-shape assertions in the same PR.

**D1 — Sanitizer model (two-phase)**

- Phase A: Add at least one `SanitizerRecognizer` entry per language family: Go `html.EscapeString` (XSS), Python `html.escape`, JS `DOMPurify.sanitize`. Both `taint_forward()` and `taint_forward_cfg()` have `cleansed_for: BTreeSet::new()` — both need Phase B.
- Phase B: During traversal, when a visited node's source line matches a recognizer's `call_path`, accumulate `category` into the path's `cleansed_for`.
- Both phases must ship together. Phase B alone is vacuously correct on real code.

**D2 — Same-line propagation (document, fix only in reasoning layer)**

`data_flow.rs:498–504` iterates all defs on a visited use's line, not just the L-value of the containing assignment. Acceptable over-approximation for legacy algorithms. In `ReasoningOptions`, the `exact_same_line_assignments` flag enables precise extraction; legacy behavior is preserved as `LegacySameLineAssignment` with lower confidence for debugging.

**W1 — Return-value DFG edges**

Extend `cpg.rs:862–940` to: find callee return expressions via `ParsedFile::return_value_nodes`; map returned identifiers to caller assignment LHS on the call line; emit `ReturnToCallResult` edges. Additive, Option-C safe. Required for Plan A.5 (interprocedural `taint_reaches`); not in v1 scope (intraprocedural only, with `ReachesFunctionBoundary` marker).

**P3 / W3 — Arg stripping and closure/container coverage**

`cpg.rs:902–903` strips both `.` and `->`. Taint on `dev->name` appears as taint on `dev`. Document as known over-taint for v1. Container operations (`Vec::push`, `HashMap::insert`) untracked. Both acceptable for v1 scope.

**H1 — DFG unit tests**

Required in the same PR as the D4 chain-BFS fix, since `forward_reachable`'s return type changes. Required fixtures: arg-param, return-to-call-result, receiver-to-self, same-line unrelated assignments, multi-call same line, field arg `a.b`, parse-degraded confidence.

**H3 — `reachable_forward` queue duplicates (corrected characterization)**

The two implementations differ:
- `DataFlowGraph::reachable_forward()` (`data_flow.rs:484–507`): NO push-time check. Nodes can be enqueued multiple times with no guard. Fix: add `if !visited.contains(&next)` before pushing.
- `CpgContext::reachable_forward()` (`cpg.rs:1263–1265`): HAS a push-time `!visited.contains()` check, but `visited` is updated at pop-time only — so a queued-but-not-yet-popped node passes the check and is pushed again by a second parent. Bounded, harmless (discarded at pop), lower priority.

Fix DFG version first; CPG version is a minor optimization.

---

### CONTROL FLOW GRAPH

**C1 — Rust `?` early-return edge (high value, low effort)**

`cfg.rs:527–532` explicitly documents the omission. In `build_rust_edges()`, detect `?` in expression statement source text; add `TypedCfgEdge { from_line: stmt_line, to_line: function_exit_sentinel, kind: EarlyReturn }`. Prevents taint from flowing through error branches of Rust functions in CFG-pruned mode. 1–2 days, additive.

**C3 — Dominance/post-dominance**

Needed for "is sanitizer guaranteed to execute before sink?" (`what_missing`). Not needed for `taint_reaches` v1. Plan for Plan B/D phase.

**C7 — Silent edge drop on empty bodies**

`cfg.rs:661–697` — `first_statement_line` / `last_statement_line` return `None` silently. Add `log::warn!` when a non-empty-looking block yields `None`. Low effort, improves debuggability.

**C8 — Same-line statement dedup in CPG**

`cpg.rs:1059` — two statements sharing a start line, second's CFG edges dropped. Known limitation of line-based representation. Document; do not attempt to fix in v1.

---

### CODE PROPERTY GRAPH

**P1 — `CpgEdge::FieldOf` dead variant**

Either add FieldOf edges (between Variable nodes sharing a base path) or delete the variant. For `taint_reaches` v1 (intraprocedural), not needed. Decision: delete from `CpgEdge` now to avoid test debt from the dead variant; re-add when Plan A.5 (interprocedural) needs field traversal.

**P4 — Split cpg.rs** — see file boundaries above. This is the prerequisite gate.

**P6 — Step 5b linear param scan**

`cpg.rs:924–933` — O(function_length × param_count × call_sites). Build a per-function param-name index at Step 1. Medium effort, meaningful perf improvement for large files.

**P7 — `build_incremental` C/C++ gap**

Emit `Warning` in CPG result when C/C++ files are in the changed set and no `compile_commands.json` is available.

**P2 — Type-aware dispatch**

`TypeRegistry` providers for Java, Rust, Go, Python, TypeScript exist but `resolve_callees_qualified()` doesn't use them (only C++ virtual dispatch). Use `TypeRegistry::dispatch_for(language)` only in `ReasoningGraph`, not in normal CPG assembly. Supplements candidate ranking; falls back to existing name-based resolution with lowered confidence.

---

### MCP

Do **not** extend `nav_v1`. Add:

```rust
ToolRegistry::reasoning_v1()   // taint_reaches, dataflow_between, impact_of_change, what_missing
ToolRegistry::all_v1()         // nav + reasoning combined, gated by CLI flag
```

Handlers can still receive `NavigationSession` — it owns repo files, CPG, types, and live types through `NavigationIndex`. Build `ReasoningGraph` lazily per call or cache it inside a future `ReasoningIndex`.

**Required regression test:** `nav_v1` still lists exactly 6 tools; reasoning tools appear only in opt-in registry.

---

## Cache / Schema Compatibility

Cache format version (`src/cpg_cache.rs:37–44`) must bump on serialized shape changes. The D4 fix changes `FlowPath` construction logic but not its struct shape — no cache bump required. Return-value edges add new `CpgEdge::DataFlow` entries — cache bump required when that lands. Reasoning metadata should initially avoid mutating cached `CpgEdge` / `DataFlowGraph` shapes; keep all reasoning state in `ReasoningGraph` (not persisted in the nav cache).

---

## Risks

1. **D4 fix breaks caller assumptions about `FlowPath.edges`.** No golden tests currently check edge shape, but algorithm tests that check line *sets* are passing because star-paths happen to reach the right nodes. A chain-BFS may produce a proper subset if the BFS takes a different path than the current all-reachable set. Mitigation: run full algorithm test suite after D4; add chain-shape assertions before merging.

2. **Phase A sanitizer entries need expert review.** A false positive sanitizer (marking a function as cleaning when it doesn't) causes missed taint findings. Each entry needs a language author sign-off before merge.

3. **`cpg.rs` split is pure rename risk.** Rust's re-export façade must preserve all public API. Use `pub use` exhaustively; run `cargo test` as the only acceptance criterion.

4. **`FieldOf` deletion.** If any algorithm or test references it outside `cpg.rs`, the build breaks. Grep before deleting.

---

## Smallest Shippable Slices + Build Order

| # | Slice | Gating | Effort | Option-C impact |
|---|-------|--------|--------|-----------------|
| 1 | Split `src/cpg.rs` into 5 submodules | nothing | 2d | none |
| 2 | Chain BFS in `DataFlowGraph::forward_reachable()` + DFG unit tests | 1 | 3–4d | safe (no golden on edge shape) |
| 3 | Sanitizer two-phase (Phase A entries + Phase B wiring in both taint methods) | 1 | 2–3d | `cleansed_for` goes from always-empty to sometimes-populated |
| 4 | Rust `?` early-return `TypedCfgEdge` + `build_cfg_edges_typed()` | 1 | 1–2d | additive |
| 5 | DFG push-time guard fix (DFG version) | 1 | 0.5d | additive |
| 6 | Create `src/reasoning/` with types, options, `FlowTrace` + local construction tests | 1–5 | 2d | additive |
| 7 | Return-to-call-result `DataFlow` edges in CPG Step 5b | 6 | 3–5d | additive, cache bump |
| 8 | `ReasoningGraph` interprocedural arg-param rebuild with richer metadata | 6, 7 | 3d | additive |
| 9 | `taint_reaches` tool + `ToolRegistry::reasoning_v1()` | 6 | 2–3d | additive, new registry |
| 10 | Receiver-to-self, type-dispatch ranking | 8 | 3d | additive |

Slices 1–5 are substrate hardening; they do not add the reasoning layer but make it sound. Slice 6 is the structural gate for the reasoning layer. Slices 7–10 are Tier-2 features.

---

## Top 5 Highest-Leverage Moves

**(1) Split `src/cpg.rs`** — prerequisite for parallel development and reasoning layer imports; purely mechanical; no behavior change.

**(2) Chain BFS (D4)** — structural prerequisite for agent-readable Evidence; without it `taint_reaches` Evidence is misleading even when technically correct; must be done with DFG unit tests.

**(3) Sanitizer two-phase (D1)** — without populated + wired recognizers, `cleansed_for` is always `{}`; the Tier-2 tool's sanitizer marker never fires on real code.

**(4) Return-value DataFlow edges (W1)** — unlocks interprocedural `taint_reaches` (Plan A.5); the cross-function bypass in `taint_forward_cfg` is currently dead without this.

**(5) Rust `?` CFG early-return edge (C1)** — high value for Rust codebase analysis, lowest effort in the list; prevents taint flow through error branches.

---

## Single Biggest Soundness Risk

**Star-paths + empty sanitizers compound into a structurally misleading Evidence graph.**

A 5-hop taint chain emits as 5 independent source→target `FlowEdge`s — the agent cannot read the chain or determine whether taint passes through a sanitizer at step 3. `cleansed_for` is always `{}`. The only accurate output is `ReachesFunctionBoundary`. The tool is technically correct (listed nodes ARE tainted) but the Evidence is structurally unhelpful. D4 (chain BFS) and D1 (Phase A + B) are the minimum viable substrate for `taint_reaches` to justify its existence.

---

## DECISIONS FOR THE OWNER

**DQ1 — `CpgEdge::FieldOf`: delete now vs. stub for Plan A.5**

- Option A (recommended): Delete the variant now. The dead variant adds test debt and no algorithm uses it. Re-add when Plan A.5 needs field traversal — at that point the design will be clearer.
- Option B: Keep it, add a `// never populated; reserved for Plan A.5` comment.
- *Why it's unresolved:* depends on Plan A.5 timeline. If interprocedural is 2–3 months out, delete; if it's next sprint, stub.

**DQ2 — `ReasoningGraph` caching strategy**

- Option A: Build `ReasoningGraph` lazily per MCP call; no persistence. Simple, correct, potentially slow on large repos.
- Option B: Cache `ReasoningGraph` inside a `ReasoningIndex` parallel to `NavigationIndex`. More complex; needed if build time exceeds 2–3s on typical repos.
- *Recommendation:* Start with Option A; profile on a real large repo after slice 9; decide on caching before Plan A.5.

**DQ3 — Same-line propagation fix scope**

- Option A: Keep legacy behavior in all paths; only add precise extraction in `ReasoningOptions.exact_same_line_assignments`. No existing algorithm test changes.
- Option B: Fix globally; update golden baselines. Cleaner long-term, more churn now.
- *Recommendation:* Option A for v1. The legacy algorithms were designed with the overbroad behavior; changing it risks masked bugs in their test suites.

**DQ4 — DFG push-time guard (CPG version)**

- Option A: Fix only the DFG version (no push-time guard at all); leave the CPG version's insufficient guard as-is (bounded, harmless).
- Option B: Fix both — CPG version needs `visited` updated at push-time, not pop-time.
- *Recommendation:* Fix both in the same PR (0.5 days total); the CPG version's guard is wrong-in-principle even if bounded in practice.

---

**Readiness verdict:** Ready to plan after deciding DQ1 (FieldOf: delete or stub) and DQ2 (ReasoningGraph caching); DQ3 and DQ4 have clear recommendations and can be decided by the implementer at slice time.