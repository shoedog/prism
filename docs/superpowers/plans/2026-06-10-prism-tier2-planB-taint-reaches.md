# Prism Tier 2 Plan B — `taint_reaches` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the first Tier-2 reasoning tool, `taint_reaches`, as a read-only MCP tool + library function that answers "does taint from these sources reach these sinks (witness mode) / where does it flow (frontier mode)" over Plan A's A3 BFS, additively (Option C: `cli_nav_compat` byte-identical; production Taint untouched).

**Architecture:** A new `src/reasoning/taint_reaches.rs` orchestrates: resolve `SeedSpec`s → `VarLocation`s (`seeds.rs`), run Plan A's `taint_trace` over `session.index.cpg`'s petgraph (read-only), classify each sink tri-state via `shape::reachability_for_node`, and shape the result (`shape.rs`) into the additive `Evidence.reasoning: Option<ReasoningSummary>` (frontier mode = scored items; witness mode = `per_sink` + a unioned witness `GraphPayload`). Two foundational substrate items the 10-round review contracted to Plan B land first: **node/location-precise seeding** (`taint_trace_locations`) and **per-root reachability/witness variants** (so multi-source attribution and cleansing-union are expressible). The MCP tool is the only `mcp`-gated part.

**Tech Stack:** Rust, the merged Plan A substrate (`src/cpg/trace.rs` `taint_trace`/`Trace`/`Relation`/`BoundaryEdge`; `src/reasoning/{seeds,shape}.rs`; `src/algorithms/taint.rs::cleansed_categories_for_source`; `src/navigation/types.rs` reasoning vocab), `serde_json`, the existing MCP adapter (`src/mcp/*`, cargo `mcp` feature).

**Source spec:** `docs/superpowers/specs/2026-06-09-prism-tier2-taint-reaches-design.md` (revision 3). Section refs below (§N) point at it. `[→plan]` items from the spec are pinned in this plan.

**Substrate surface this plan consumes (verified on merged `main`, commit `40a1b46`):**
- `CodePropertyGraph::taint_trace(&[(String, usize)]) -> Trace` (`src/cpg/trace.rs`); `Trace { frontier_by_root: BTreeMap<NodeIndex, BTreeSet<NodeIndex>>, parents_by_root: BTreeMap<(NodeIndex,NodeIndex),(NodeIndex,Relation)>, boundary: BTreeSet<BoundaryEdge>, degraded: bool, warnings: Vec<String> }`; `Trace::in_frontier(NodeIndex)`, `Trace::frontier()`; `BoundaryEdge { root, from, to }`; `Relation { DataFlow, AssignmentPropagation, RecoveredDefUse }`.
- `shape::reachability_for_node(cpg, trace, sink) -> Reachability`; `shape::witness_graph_for_node(cpg, trace, sink) -> Option<GraphPayload>`; `shape::reachability_at` (line-level, **lossy** — do not use in the tool).
- `cleansed_categories_for_source(files: &BTreeMap<String, ParsedFile>, source: &VarLocation) -> Vec<String>` (`pub(crate)`, `src/algorithms/taint.rs:10680`).
- `navigation::types`: `Reachability { Reached, NotReached, BoundaryExited }`; `SinkResult { sink: SymbolRef, reachability, graph_node: Option<usize> }`; `ReasoningSummary { reachability: Option<Reachability>, per_sink: Vec<SinkResult>, source_count, frontier_count }`; `ReasoningReason::TaintedBy { source, sanitizers_present_in_source_fn, path_proven }`; `ReasoningWarning { SeedUnresolved, InterproceduralBoundary, Cleansed }`; `Reason::Reasoning(ReasoningReason)`; `WarningKind::Reasoning(ReasoningWarning)`; `Evidence.reasoning: Option<ReasoningSummary>` (`#[serde(skip_serializing_if="Option::is_none")]`).
- `reasoning::seeds`: `SeedSpec { Loc{file,line}, Symbol{name,file:Option<String>} }`, `ResolvedSeed { locations: Vec<VarLocation>, symbol: Option<SymbolRef>, origin: SeedSpec }`, `SeedSet { seeds, warnings }` — **types only, no resolution yet.**
- MCP: `registry::nav_v1()` (`src/mcp/registry.rs:55`, frozen `== 6`); `MAX_RESULTS_DEFAULT`/`MAX_RESULTS_CAP` (`src/mcp/input.rs`); `SeedInput` (`src/mcp/input.rs`, `deny_unknown_fields`); `build_result` (`src/mcp/output.rs:149`); `Transport::write_message` (`src/mcp/transport.rs:428` real, `:462` `InMemoryTransport`).

**Pinned `[→plan]` decisions:**
- **Frontier score curve (§4.4):** `gradient_slice` convention — sources `1.0`, downstream `0.6^depth` (BFS depth from nearest source), `min_score` not applied (keep all frontier nodes; `max_results` caps). Tie-break `(file, function, line, path, ordinal)`.
- **`max_results` default/cap (§9):** reuse `MAX_RESULTS_DEFAULT` / `MAX_RESULTS_CAP` from `input.rs` (no new constant).
- **Truncation marker (§6):** when a frame exceeds the cap, replace the over-cap string payload with `"…[truncated N bytes]"` (ASCII ellipsis `...` to stay byte-safe in tests) inside a structurally-valid JSON-RPC envelope; add `anthropic/maxResultSizeChars` to error `_meta`.
- **`SeedUnresolved` wire shape (§8):** serialized via `Warning.message` (`"seed <spec> unresolved: <reason>"`) + `Warning.location` (the seed's file/line when known); the typed `ReasoningWarning::SeedUnresolved` carries `{ spec_description: String, reason: String }`.

---

## File Structure

- **Modify** `src/cpg/trace.rs` — add `taint_trace_locations(&[VarLocation]) -> Trace`; make `taint_trace(&[(file,line)])` a thin wrapper resolving each line to its `Variable` `VarLocation`s.
- **Modify** `src/reasoning/shape.rs` — add `reachability_for_node_from(cpg, trace, root, sink)` and `witness_graph_for(cpg, trace, root, sink)`; make the existing union forms wrappers.
- **Modify** `src/reasoning/seeds.rs` — add `resolve(session, &[SeedSpec]) -> Result<SeedSet, QueryError>` + the §7 truth table.
- **Create** `src/reasoning/taint_reaches.rs` — `taint_reaches(session, sources, sinks) -> Result<Evidence, QueryError>`; frontier/witness shaping; warnings.
- **Modify** `src/reasoning/mod.rs` — `pub mod taint_reaches;`.
- **Modify** `src/mcp/registry.rs` — add `reason_v1()`.
- **Modify** `src/mcp/*` (server `run`, dispatch, schema) — register `reason_taint_reaches` (mcp-gated).
- **Modify** `src/mcp/transport.rs` + `src/mcp/error.rs` — Task 0 wire-cap chokepoint.
- **Modify** `Cargo.toml` — `default-run = "prism"` (own commit).
- **Tests:** `src/reasoning/{seeds,taint_reaches}.rs` `#[cfg(test)]`; `tests/reasoning/*`; `tests/mcp/*` (or existing mcp test target); the recurring Option-C proof `cargo test --test cli_nav_compat`.

---

## Phase 0 — Foundation (independent; own commits)

### Task 0.1: `default-run = "prism"`

**Files:** Modify `Cargo.toml`.

- [ ] **Step 1: Write the failing test**

```rust
// tests/cli/default_run_test.rs
#[test]
fn cargo_run_no_bin_resolves_to_prism() {
    // `cargo run -- --help` must not error with "could not determine which binary to run".
    let out = std::process::Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--", "--help"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("could not determine which binary"), "{stderr}");
}
```

- [ ] **Step 2: Run it to verify it fails** — `cargo test --test cli_default_run` → FAIL (ambiguous binary). (Register `[[test]] name="cli_default_run"` in `Cargo.toml`.)
- [ ] **Step 3: Add `default-run`**

```toml
# Cargo.toml [package]
default-run = "prism"
```

- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `git commit -m "build: set default-run = prism (resolve ambiguous cargo run)"`

### Task 0.2: Wire-size chokepoint at `write_message`

**Files:** Modify `src/mcp/transport.rs:428`, `src/mcp/error.rs:162-168`. Test: `tests/mcp/wire_cap_test.rs`.

- [ ] **Step 1: Write the failing test** — an oversized **error** frame stays valid JSON-RPC after truncation, with the marker and `anthropic/maxResultSizeChars` in `_meta`.

```rust
#[test]
fn oversized_error_frame_is_valid_jsonrpc_after_cap() {
    use prism::mcp::transport::{Transport, InMemoryTransport};
    let mut t = InMemoryTransport::new();
    let huge = "x".repeat(2_000_000);
    let frame = serde_json::json!({
        "jsonrpc":"2.0","id":1,
        "error":{"code":-32000,"message":huge,"data":{"_meta":{}}}
    });
    t.write_message(frame).unwrap();
    let written: serde_json::Value = serde_json::from_str(&t.last_written()).expect("valid JSON");
    assert_eq!(written["jsonrpc"], "2.0");
    assert!(written["error"]["message"].as_str().unwrap().contains("...[truncated"));
    assert!(written["error"]["data"]["_meta"]["anthropic/maxResultSizeChars"].is_number());
}
```

- [ ] **Step 2: Run → FAIL** (no cap; `last_written` accessor missing).
- [ ] **Step 3: Implement** the per-frame-class cap in `write_message` (success / tool-`isError` / terminal over-cap / protocol-error), preserving envelope structure; truncate the largest string payload with the marker; stamp `anthropic/maxResultSizeChars`. Add `last_written()` test accessor to `InMemoryTransport` (`:462`).
- [ ] **Step 4: Run → PASS.** Also run `cargo test --test cli_nav_compat` → byte-identical (the cap only triggers above existing frame sizes; verify the nav goldens are well under the cap so behavior is unchanged).
- [ ] **Step 5: Commit** — `git commit -m "feat(mcp): bound every wire frame at write_message with truncation marker + error _meta cap"`

---

## Phase 1 — Substrate surface completion (contracted to Plan B by the review)

### Task 1.1: Node/location-precise seeding — `taint_trace_locations`

**Why:** the spec resolves seeds to specific `VarLocation`s (§3); seeding by `(file,line)` promotes *every* `Variable` on the line to a root (over-seeds Symbol params' siblings, inflates `frontier_count`). Add a node-precise entry; keep the line API as a wrapper.

**Files:** Modify `src/cpg/trace.rs`. Test: `src/cpg/tests.rs`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn taint_trace_locations_seeds_only_named_locations() {
    // `a = src(); b = clean()` on one line — seeding ONLY a's VarLocation must not seed b.
    let cpg = build_python_cpg("def f():\n    a = src(); b = clean()\n    sink(a)\n");
    let a_loc = cpg.to_var_location(
        var_def_node(&cpg, "test.py", 2, "a")).unwrap();
    let trace = cpg.taint_trace_locations(&[a_loc]);
    let b_in = cpg.nodes_at("test.py", 2).into_iter().any(|n| {
        cpg.to_var_location(n).is_some_and(|l| l.path.to_string() == "b") && trace.in_frontier(n)
    });
    assert!(!b_in, "seeding a's location must not seed b");
}
```

- [ ] **Step 2: Run → FAIL** (method missing).
- [ ] **Step 3: Implement** — refactor `taint_trace`'s per-root BFS body into a helper keyed on the seed *node*; `taint_trace_locations` maps each `VarLocation` to its node (via the existing `var_node`/location lookup) and seeds those nodes; `taint_trace(&[(file,line)])` resolves each line to all `Variable` `VarLocation`s on it (preserving today's behavior — `taint_trace`'s existing tests must stay green) and delegates. Keep the multi-function / signature-line degrade and dedup logic on the line path.
- [ ] **Step 4: Run → PASS;** run all `taint_trace*` tests → green (line API behavior unchanged).
- [ ] **Step 5: Commit** — `git commit -m "feat(cpg): node-precise taint_trace_locations; line taint_trace becomes a wrapper"`

### Task 1.2: Per-root reachability + witness variants

**Why:** the spec attributes per source (multi-source cleansing union §4.6; "which source taints this sink"). The union `reachability_for_node`/`witness_graph_for_node` can't. Add root-parameterized forms; the union forms become wrappers.

**Files:** Modify `src/reasoning/shape.rs`. Test: `src/reasoning/shape.rs` `#[cfg(test)]`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn reachability_from_root_is_per_source() {
    // Two sources; sink reached only from source A's root, not B's.
    // assert reachability_for_node_from(.., root_a, sink) == Reached
    // assert reachability_for_node_from(.., root_b, sink) == NotReached
    // and witness_graph_for(.., root_a, sink).is_some() && witness_graph_for(.., root_b, sink).is_none()
}
```

- [ ] **Step 2: Run → FAIL** (methods missing).
- [ ] **Step 3: Implement** `reachability_for_node_from(cpg, trace, root, sink)` (consult only `trace.frontier_by_root[root]` / boundaries with `b.root == root`) and `witness_graph_for(cpg, trace, root, sink)` (walk `parents_by_root` for that root only). Make `reachability_for_node` = "any root" (Reached if any; else BoundaryExited if any; else NotReached) and `witness_graph_for_node` = "first BTreeMap-order root that reaches", both delegating to the per-root forms.
- [ ] **Step 4: Run → PASS;** existing shape.rs tests green.
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): per-root reachability_for_node_from / witness_graph_for"`

---

## Phase 2 — Seed resolution (`seeds.rs`)

### Task 2.1: `Loc` resolution + the 3 `None` cases

**Files:** Modify `src/reasoning/seeds.rs`. Test: same file `#[cfg(test)]`.

- [ ] **Step 1: Write the failing tests** — for `SeedSpec::Loc`: (a) a line with Variables → `ResolvedSeed.locations` = all `Variable` `VarLocation`s on the line; (b) line with zero Variables → `SkippedPath` warning, no locations; (c) the three `enclosing_function == None` cases map to `QueryError::UnsupportedFile` (missing/unindexed file) / `QueryError::LocationOutOfRange` (line past EOF) / empty-but-valid (in-file, outside any function) — assert via `resolve`'s `Result`/warnings. Use `tests/common` fixtures.
- [ ] **Step 2: Run → FAIL** (`resolve` missing).
- [ ] **Step 3: Implement** `resolve(session: &NavigationSession, specs: &[SeedSpec]) -> Result<SeedSet, QueryError>` Loc arm: look up `Variable` locations on the line via the CPG (`nodes_at` → `to_var_location`, filter `Variable`); classify missing-file / out-of-range / outside-function per §7; push per-seed warnings (`WarningKind::Reasoning(SeedUnresolved{..})`). Do NOT use `resolve_fn` (drops the exact line, §3).
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): seeds.rs Loc resolution + None-case truth table"`

### Task 2.2: `Symbol` resolution (parameters only)

**Files:** Modify `src/reasoning/seeds.rs`.

- [ ] **Step 1: Write the failing tests** — `SeedSpec::Symbol{name}` → the function's PARAMETERS (each param's `Def` at the function-start line, via `function_parameter_names`); field-only params skipped (`has_bare_references`); edge cases: zero-param → empty+warning; all params field-only/no-`Def` → empty+warning (matches "outside any function"); some unresolved → partial `SkippedPath` naming them.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** the Symbol arm: resolve the function, enumerate `function_parameter_names`, map each to its `Def` `VarLocation` (function-start line), skip field-only via `has_bare_references` (`data_flow.rs:208-218`); apply the edge-case rules.
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): seeds.rs Symbol=parameters resolution + edge cases"`

### Task 2.3: All-fail precedence + warning ordering

**Files:** Modify `src/reasoning/seeds.rs`.

- [ ] **Step 1: Write the failing test** — when **all** sources fail, `resolve` returns `QueryError` of the **first** seed-input-order failure's kind; warnings are in seed-input order, deduped. (Never add a `QueryError` variant — reuse `UnsupportedFile`/`LocationOutOfRange`; `error_text` is an exhaustive 5-arm match, `navigation.rs:13-23`.)
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** precedence-by-seed-input-order; dedup + order warnings.
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): seed-resolution failure precedence + warning ordering"`

---

## Phase 3 — `taint_reaches` query core

### Task 3.1: Orchestration — resolve → BFS → per-sink tri-state

**Files:** Create `src/reasoning/taint_reaches.rs`; modify `src/reasoning/mod.rs` (`pub mod taint_reaches;`). Test: same file `#[cfg(test)]` + `tests/reasoning/taint_reaches_test.rs`.

- [ ] **Step 1: Write the failing tests** — intraprocedural `S → sink` Reached; `S`, unrelated sink NotReached; the **`S → I → K` interior-node invariant** (sink Reached *and* witness never dead-ends); a cross-`(file,function)` sink → **`BoundaryExited`** + a sink-located `InterproceduralBoundary` warning.
- [ ] **Step 2: Run → FAIL** (fn missing).
- [ ] **Step 3: Implement** `taint_reaches(session, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>) -> Result<Evidence, QueryError>`: resolve sources (+sinks) via `seeds::resolve`; collect source `VarLocation`s; `cpg.taint_trace_locations(&source_locs)`; for each sink location classify via `shape::reachability_for_node` (Reached/BoundaryExited/NotReached). Build a minimal `Evidence` with `reasoning: Some(ReasoningSummary{..})` (shaping detail in Phase 4). Register `[[test]] name="reasoning_taint_reaches"`.
- [ ] **Step 4: Run → PASS;** `cargo test --test cli_nav_compat` byte-identical.
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): taint_reaches orchestration + tri-state sink classification"`

### Task 3.2: Cleansing (A4 adapter) + multi-source union

**Files:** Modify `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing tests** — a Go/Python source whose function has a sanitizer surfaces `sanitizers_present_in_source_fn` (non-empty) with `path_proven:false`; a sink witnessed by two sources **unions** their `sanitizers_present_in_source_fn` (§4.6), so a sanitized-but-longer source isn't hidden.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** `cleansed_categories_for_source(session.files(), &source_loc)` per source; union across sources that witness a sink. (Adapter is `pub(crate)` in `crate::algorithms::taint` — import it; this also clears its two intentional dead-code warnings.)
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): wire A4 cleansing adapter into taint_reaches (multi-source union)"`

---

## Phase 4 — Evidence shaping (frontier + witness)

### Task 4.1: Frontier mode (no sinks) — scored items

**Files:** Modify `src/reasoning/taint_reaches.rs` (+ helpers in `shape.rs` if needed).

- [ ] **Step 1: Write the failing test** — sinks omitted → `Evidence.items` = frontier nodes, each `why: [Reason::Reasoning(TaintedBy{source, sanitizers_present_in_source_fn, path_proven:false})]`; sources score `1.0`, downstream `0.6^depth`; tie-break `(file,function,line,path,ordinal)`; `reasoning.reachability == None`; `per_sink` empty; `source_count`/`frontier_count` pre-cap; `items` XOR `graph` (items only).
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** frontier shaping: BFS depth per frontier node (min over sources), `score = 0.6^depth`; map nodes → `EvidenceItem`; `max_results` caps items via the existing `retained_count`/`truncated`.
- [ ] **Step 4: Run → PASS;** Option-C check.
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): taint_reaches frontier mode (scored TaintedBy items)"`

### Task 4.2: Witness mode (sinks given) — per_sink + unioned graph

**Files:** Modify `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing tests** — sinks given → `per_sink: Vec<SinkResult>` (tri-state each), Reached sinks get a witness via `witness_graph_for`; all witnesses **unioned** into one `GraphPayload` (deduped by full node identity `(file,function,line,path,kind,ordinal)`, self-edges dropped); each `SinkResult.graph_node` indexes the sink's node in the union graph (`None` for NotReached/BoundaryExited/clipped); `reasoning.reachability` = aggregate (any Reached → Reached, else any BoundaryExited → BoundaryExited, else NotReached); `graph` XOR `items` (graph only); edge kinds relation-named (`DataFlow`/`AssignmentPropagation`/`RecoveredDefUse`), no `ControlFlow`.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** witness shaping: per Reached sink, shortest witness via `witness_graph_for(root, sink)`; union + dedup by full node identity; set `graph_node`; `max_results` caps graph nodes (clipped `graph_node` → `None`, `per_sink` stays complete).
- [ ] **Step 4: Run → PASS;** Option-C check.
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): taint_reaches witness mode (per_sink + unioned witness graph)"`

### Task 4.3: Warnings — SeedUnresolved / InterproceduralBoundary / Cleansed

**Files:** Modify `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing tests** — partial-seed-failure → `WarningKind::Reasoning(SeedUnresolved{..})` in seed-input order; each `BoundaryEdge` in the trace whose sink is queried → a **sink-located** `InterproceduralBoundary` naming the dropped source/edge; witness-mode sanitizer presence → `Cleansed`. Warnings deduped, seed-input order.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** warning assembly from the `SeedSet.warnings` + `Trace.boundary` + cleansing.
- [ ] **Step 4: Run → PASS;** **`cargo test --test cli_nav_compat` byte-identical** (additive `Reason`/`WarningKind` variants are byte-safe via the catch-all `navigation.rs:73` + `{:?}` render — confirm no golden moves).
- [ ] **Step 5: Commit** — `git commit -m "feat(reasoning): taint_reaches warnings (SeedUnresolved/InterproceduralBoundary/Cleansed)"`

---

## Phase 5 — MCP surface

### Task 5.1: `reason_v1()` registry + combined registration

**Files:** Modify `src/mcp/registry.rs`, the server `run` (`src/bin/prism_mcp.rs` or wherever `nav_v1()` is served). Test: `tests/mcp/registry_test.rs`.

- [ ] **Step 1: Write the failing tests** — `nav_v1()` stays **== 6** (existing frozen test untouched); a new `reason_v1()` exists; the **combined** registry served by `prism-mcp run` is **== 7**; reasoning tools are **absent** from `nav_v1()`.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** `reason_v1()` with `reason_taint_reaches`; serve `nav_v1()` + `reason_v1()` combined. All `#[cfg(feature = "mcp")]`.
- [ ] **Step 4: Run → PASS** (`cargo test --features mcp`).
- [ ] **Step 5: Commit** — `git commit -m "feat(mcp): reason_v1 registry + reason_taint_reaches registration"`

### Task 5.2: Tool schema + dispatch + description

**Files:** Modify `src/mcp/*` (input parsing, dispatch, `build_result` at `output.rs:149`).

- [ ] **Step 1: Write the failing tests** — schema: `sources` required `[SeedInput]` `minItems:1`; `sinks` optional; **empty `sinks:[]` is invalid** (omit for frontier mode); `max_results` int default `MAX_RESULTS_DEFAULT`; `verbosity` `concise|detailed` default `concise`. A smoke MCP exchange (`reason_taint_reaches` over `InMemoryTransport`) returns valid `Evidence`. Tool description states the **params-only scope** (taint entering F via a local/global/env read → `reached:false` is a scope artifact) and **steers agents to `per_sink`**, not the aggregate.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** the input schema (reuse `SeedInput`, `deny_unknown_fields`), dispatch to `taint_reaches`, shape via `build_result` (`output.rs:149`). Reject empty `sinks:[]`.
- [ ] **Step 4: Run → PASS** (`cargo test --features mcp`); `cargo test --test cli_nav_compat` byte-identical.
- [ ] **Step 5: Commit** — `git commit -m "feat(mcp): reason_taint_reaches schema, dispatch, params-only description"`

### Task 5.3: Boundary-bypass regression guard (§11)

**Files:** Test only — `tests/reasoning/boundary_contract_test.rs`.

- [ ] **Step 1: Write the test** — pin `taint_forward_cfg`'s deliberate cross-`(file,function)` bypass as a contract: a known interprocedural flow stays present in `taint_forward_cfg` output, so a future "make it intraprocedural" change can't silently break the `BoundaryExited` marker (load-bearing for the Taint slice AND, inversely, `taint_reaches`).
- [ ] **Step 2: Run → PASS** (documents current behavior).
- [ ] **Step 3: Commit** — `git commit -m "test(reasoning): pin taint_forward_cfg cross-function bypass as a contract"`

---

## Recurring gate (every task that touches serialization or MCP)

`cargo test --test cli_nav_compat` MUST stay **byte-identical** (Option C); NOT the aggregate `review` preset (non-deterministic, `nav_compat_test.rs:17-22`). `cargo fmt && cargo test` green. `cargo build --features mcp` clean. Re-warm the prism nav cache before any prism-wired review (source changes stale it; ~27s cold).

---

## Self-Review

**Spec coverage:** §2 contract (Task 3.1, 5.2) · §3 SeedSet (Tasks 2.1–2.3) · §4 query flow (3.1, 3.2, 4.1, 4.2) · §5 placement/shaper (Phase 3–4) · §6 foundation (Task 0.2, 0.1) · §7 truth table (2.1–2.3, 4.3) · §8 vocabulary (already merged; consumed 3.x/4.x) · §9 MCP (5.1, 5.2) · §10 testing (each task's tests) · §11 non-goals/regression guard (Task 5.3). Foundational substrate items from `planA-followups.md` (node-precise seeding, per-root variants) → Phase 1.

**Type consistency:** `taint_trace_locations(&[VarLocation])`, `reachability_for_node_from(.., root, sink)`, `witness_graph_for(.., root, sink)`, `resolve(session, &[SeedSpec]) -> Result<SeedSet, QueryError>`, `taint_reaches(session, &[SeedSpec], Option<&[SeedSpec]>) -> Result<Evidence, QueryError>` — names used consistently across tasks. Reasoning vocab (`Reachability`/`SinkResult`/`ReasoningSummary`/`ReasoningReason`/`ReasoningWarning`) is the merged surface, not redefined.

**Open items for plan-review to pin (the `[→plan]` residue + new):** exact truncation marker bytes + cap value; the `function_parameter_names`/`has_bare_references` exact signatures and the `Def`-at-start-line lookup; how `session.files()` is reached from `NavigationSession` for the A4 adapter; whether `reason_v1()` lives in `registry.rs` or a new `reason_registry.rs`; the `SeedUnresolved` `Warning` serialization exact fields.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-06-10-prism-tier2-planB-taint-reaches.md`.**

Per the established process ([[feedback-workflow-preferences]]): **plan-review** (a2a-bridge `run-workflow plan-review`, prism-wired `a2a-bridge.slicing-plan-review.toml`) is the next adversarial gate — it pins the `[→plan]` residue and checks the substrate-surface assumptions against merged `main` — **before** implementation. After folding plan-review: containerized **codex implement** (`a2a-bridge implement`) or subagent-driven TDD, then in-depth code-review vs main to convergence → squash to docs+feat pair → merge.
