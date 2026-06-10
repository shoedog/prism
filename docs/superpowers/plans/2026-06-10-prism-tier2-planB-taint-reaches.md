# Prism Tier 2 Plan B — `taint_reaches` Implementation Plan

> **Status:** Revision 2 (2026-06-10) — folded the prism-wired plan-review (codex/gpt-5.5 exec-readiness + claude/fable coverage; `docs/prism-query-layer/planB-plan-review-MCP-2026-06-10.md`). All 11 BLOCKERs + 4 MAJORs + 6 MINORs resolved against the merged tree; architecture/sequencing/grounding were judged sound and are unchanged.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the first Tier-2 reasoning tool, `taint_reaches`, as a read-only MCP tool + library function that answers "does taint from these sources reach these sinks (witness mode) / where does it flow (frontier mode)" over Plan A's A3 BFS, additively (Option C: `cli_nav_compat` byte-identical; production Taint untouched).

**Architecture:** A new `src/reasoning/taint_reaches.rs` orchestrates: resolve `SeedSpec`s → `VarLocation`s (`seeds.rs`), run Plan A's `taint_trace_locations` over `session.repo`'s CPG petgraph (read-only), classify each sink tri-state via `shape::reachability_for_node`, and shape the result (`shape.rs`) into the additive `Evidence.reasoning: Option<ReasoningSummary>` (frontier mode = scored items; witness mode = `per_sink` + a unioned witness `GraphPayload`). **The library returns a pre-cap `Evidence`; capping + `graph_node` repair happen in the MCP `shape_result` seam** (matching nav). Two foundational substrate items the 10-round review contracted to Plan B land first: **node/location-precise seeding** (`taint_trace_locations`) and **per-root reachability/witness variants**. The MCP tool is the only `mcp`-gated part.

**Tech Stack:** Rust, the merged Plan A substrate, `serde_json`, the existing MCP adapter (`src/mcp/*`, cargo `mcp` feature).

**Source spec:** `docs/superpowers/specs/2026-06-09-prism-tier2-taint-reaches-design.md` (rev 3). §N refs point there.

**Substrate surface (verified on merged `main` + plan-review):**
- `CodePropertyGraph::taint_trace(&[(String,usize)]) -> Trace` (`src/cpg/trace.rs`); `Trace { frontier_by_root, parents_by_root, boundary: BTreeSet<BoundaryEdge>, degraded, warnings }`; `Trace::in_frontier(NodeIndex)`; `BoundaryEdge { root, from, to }`; `Relation { DataFlow, AssignmentPropagation, RecoveredDefUse }`.
- `shape::{reachability_for_node, witness_graph_for_node}`; `reachability_at` is **lossy** (do not use in the tool).
- `cleansed_categories_for_source(files: &BTreeMap<String,ParsedFile>, source: &VarLocation) -> Vec<String>` — `pub(crate)`, `src/algorithms/taint.rs:10680`.
- `navigation::types` (**use these merged shapes verbatim — do NOT redefine**): `Reachability { Reached, NotReached, BoundaryExited }`; `SinkResult { sink: SymbolRef, reachability, graph_node: Option<usize> }`; `ReasoningSummary { reachability: Option<Reachability>, per_sink: Vec<SinkResult>, source_count, frontier_count }`; `ReasoningReason::TaintedBy { source: SymbolRef, sanitizers_present_in_source_fn: Vec<String>, path_proven: bool }`; **`ReasoningWarning { SeedUnresolved { seed: String }, InterproceduralBoundary { sink: String }, Cleansed { source_function: String } }`** (single-string each — types.rs:130-134); `Reason::Reasoning(..)`/`WarningKind::Reasoning(..)`; `Evidence.reasoning: Option<ReasoningSummary>` (`skip_serializing_if`); `QueryError::AmbiguousSymbol` exists (types.rs:183).
- `reasoning::seeds`: `SeedSpec { Loc{file,line}, Symbol{name,file:Option<String>} }`, `ResolvedSeed { locations: Vec<VarLocation>, symbol, origin }`, `SeedSet { seeds, warnings }` — **types only**.
- `NavigationSession { repo: Arc<LoadedRepo>, .. }` (`navigation/mod.rs:24-25`); `LoadedRepo.files: BTreeMap<String,ParsedFile>` (`repo_loader.rs:33`); the CPG is on the session/repo (reached as `…cpg`).
- `ParsedFile::{find_function_by_name(name) -> Option<Node>` (`ast.rs:2716`), `function_parameter_names(func_node: &Node) -> Vec<String>` (`:2743`), `has_bare_references(func_node: &Node, var_name) -> bool` (`:1780`)}` — **AST methods taking a tree-sitter `Node`**, not data-flow helpers.
- `data_flow::VarAccessKind` (`data_flow.rs:13-21`) vs `cpg::VarAccess` (`cpg/query.rs:44-51`) — **different types; a `VarAccessKind → VarAccess` conversion is required** wherever a `VarLocation` is mapped back to a CPG node.
- MCP: `registry::nav_v1()` (`registry.rs:55`, frozen `== 6`); `MAX_RESULTS_DEFAULT`/`MAX_RESULTS_CAP` (`input.rs`); `SeedInput` (`input.rs:22-27`, `deny_unknown_fields`; **`to_triple()` is nav-lossy — do not route through it**); **`pub fn shape_result(ev, retained, truncated, verbosity, max_results)` (`output.rs:98`)** is the capping seam (`build_result` at `:149` is **private**); handlers call `shape_result` (`tools.rs:80…`); `Verbosity::Concise` clears `item.why` (`output.rs:160-165`); `InMemoryTransport` is `#[cfg(test)]` with `new(inputs: Vec<&str>)` (`transport.rs:444`) storing `serde_json::Value`s; unit tests live in `src/mcp/transport_tests.rs` (`#[path]` at `:468`); `tests/mcp/smoke_test.rs:24` asserts `tools.len() == 6`.

**Pinned `[→plan]` decisions (revised):**
- **Frontier score:** sources `1.0`, downstream `0.6^depth` (min BFS depth over sources; `gradient_slice` convention). No `min_score` filter. Tie-break `(file, function, line, path, ordinal)`. **`TaintedBy.source` is singular → the min-depth root, tie-break `BTreeMap` (NodeIndex) order** (Phase 1's per-root variants make this expressible).
- **Capping:** the library `taint_reaches` returns a **pre-cap** `Evidence`; the MCP dispatch calls `shape_result(.., max_results)` which caps items/graph AND **repairs `reasoning.per_sink[*].graph_node`** (→ `None` if its node was clipped). `max_results` default/cap = `MAX_RESULTS_DEFAULT`/`MAX_RESULTS_CAP`.
- **Wire-cap:** `pub(crate) const MAX_RESULT_SIZE_CHARS: usize = 1_000_000;` in `transport.rs`; over-cap frames truncate the largest string payload with the ASCII marker **`"...[truncated N bytes]"`** in a valid JSON-RPC envelope; `anthropic/maxResultSizeChars` stamped into error `_meta`.
- **Warning shapes (merged):** `SeedUnresolved { seed }` where `seed` = `"<file>:<line>"` (Loc) / `"Symbol(<name>)"` (Symbol) + reason appended (`"<seed> — <reason>"`); `InterproceduralBoundary { sink }` = the sink's display; `Cleansed { source_function }`.
- **`per_sink` granularity:** **one `SinkResult` per resolved sink `VarLocation`** (a sink `Loc` resolving to N locations → N `SinkResult`s), `sink: SymbolRef` built from the `VarLocation`.
- **Ambiguous `Symbol`:** `Symbol{name, file:None}` matching multiple functions → `QueryError::AmbiguousSymbol` only when **all** sources are ambiguous (nav precedent); otherwise warn-and-skip.
- **Concise verbosity:** frontier rationale (`TaintedBy`) rides in `item.why`, which `Concise` strips by design (nav parity); the `reasoning` summary (`source_count`/`frontier_count`/`per_sink`) survives. Tool description steers agents to **`verbosity: detailed`** for frontier rationale; a concise-vs-detailed test pins it.

---

## File Structure

- **Modify** `src/cpg/trace.rs` — add `taint_trace_locations(&[VarLocation]) -> Trace`; `taint_trace(&[(file,line)])` becomes a wrapper. Add a `VarLocation → NodeIndex` resolver (with the `VarAccessKind → VarAccess` conversion).
- **Modify** `src/reasoning/shape.rs` — add `reachability_for_node_from(cpg, trace, root, sink)` / `witness_graph_for(cpg, trace, root, sink)`; union forms become wrappers.
- **Modify** `src/reasoning/seeds.rs` — `resolve(session, &[SeedSpec]) -> Result<SeedSet, QueryError>` + §7 truth table (bridges CPG `Function` → AST `Node` via `repo.files[file].find_function_by_name`).
- **Create** `src/reasoning/taint_reaches.rs` — the query; pre-cap `Evidence`.
- **Modify** `src/reasoning/mod.rs` — `pub mod taint_reaches;`.
- **Modify** `src/mcp/output.rs` (`shape_result`) — clip-aware `graph_node` repair.
- **Modify** `src/mcp/registry.rs` — `reason_v1()`; the server `run` to serve `nav_v1()`+`reason_v1()`.
- **Modify** `src/mcp/{tools,input}.rs` — `reason_taint_reaches` dispatch + `SeedInput → SeedSpec` conversion + schema.
- **Modify** `src/mcp/transport.rs` + `src/mcp/transport_tests.rs` — wire-cap chokepoint + its unit test.
- **Modify** `src/mcp/error.rs` — `anthropic/maxResultSizeChars` in error `_meta`.
- **Modify** `Cargo.toml` — `default-run`; `[[test]]` registrations (`path` + `required-features` where mcp).
- **Modify** `tests/mcp/smoke_test.rs` — 6 → 7 tools. **Modify** `CLAUDE.md` + `Evidence.graph` doc — "seven tools", witness-mode graph (closing docs task).

---

## Phase 0 — Foundation (own commits)

### Task 0.1: `default-run = "prism"`

**Files:** `Cargo.toml`. Test: `tests/cli/default_run_test.rs` (`[[test]] name="cli_default_run" path="tests/cli/default_run_test.rs"`).

- [ ] **Step 1: Write the failing test** — use the repo's `CARGO_BIN_EXE`/`assert_cmd` convention, not a nested `cargo run`:

```rust
#[test]
fn prism_binary_runs_help() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_prism")).arg("--help").output().unwrap();
    assert!(out.status.success());
}
// (default-run itself is verified by `cargo run -- --help` succeeding in CI/dev; the binary test guards the bin exists.)
```

- [ ] **Step 2: Run → confirm `cargo run -- --help` errors today** (two bins, no default-run, `Cargo.toml:11-18`).
- [ ] **Step 3:** add `default-run = "prism"` to `[package]`.
- [ ] **Step 4: Run → PASS;** `cargo run -- --help` resolves.
- [ ] **Step 5: Commit** — `build: default-run = prism`.

### Task 0.2: Wire-size chokepoint at `write_message`

**Files:** `src/mcp/transport.rs` (`write_message` impls `:428`/`:462`; const), `src/mcp/error.rs:162-168`. Test: the existing **`src/mcp/transport_tests.rs`** unit module (`#[path]`-included at `:468`) — NOT a `tests/` file (`InMemoryTransport` is `#[cfg(test)]`).

- [ ] **Step 1: Write the failing test** (in `transport_tests.rs`): build an oversized **error** `Value`, write via `InMemoryTransport::new(vec![])`, read the stored `Value` back, assert envelope intact + ASCII marker + `anthropic/maxResultSizeChars` number in `error.data._meta`.

```rust
#[test]
fn oversized_error_frame_capped_valid_jsonrpc() {
    let mut t = InMemoryTransport::new(vec![]);
    let huge = "x".repeat(2_000_000);
    t.write_message(json!({"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":huge,"data":{"_meta":{}}}})).unwrap();
    let v = t.written().last().unwrap();              // add `written()` accessor returning &[Value]
    assert_eq!(v["jsonrpc"], "2.0");
    assert!(v["error"]["message"].as_str().unwrap().contains("...[truncated"));
    assert!(v["error"]["data"]["_meta"]["anthropic/maxResultSizeChars"].is_number());
}
```

- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** a shared `fn cap_frame(v: &mut Value)` (used by BOTH transport impls) keyed on `MAX_RESULT_SIZE_CHARS = 1_000_000`: if the serialized frame exceeds it, truncate the largest string payload (`error.message` / `result.content[..].text`) with `"...[truncated {n} bytes]"`, keeping the envelope; stamp `anthropic/maxResultSizeChars` into error `_meta`. Call `cap_frame` at the top of each `write_message`. Add `written()` to `InMemoryTransport`.
- [ ] **Step 4: Run → PASS;** `cargo test --test cli_nav_compat` byte-identical (nav frames are well under 1MB — assert one golden's size in a comment).
- [ ] **Step 5: Commit** — `feat(mcp): cap every wire frame at write_message (1MB, truncation marker, error _meta)`.

---

## Phase 1 — Substrate surface completion

### Task 1.1: `taint_trace_locations`

**Files:** `src/cpg/trace.rs`. Test: `src/cpg/tests.rs`.

- [ ] **Step 1: Write the failing test** — define the node lookup inline (only `build_python_cpg` exists, `src/cpg/tests.rs:6`); use `nodes_at` + `to_var_location`:

```rust
#[test]
fn taint_trace_locations_seeds_only_named_locations() {
    let cpg = build_python_cpg("def f():\n    a = src(); b = clean()\n    sink(a)\n");
    let a_loc = cpg.nodes_at("test.py", 2).into_iter()
        .find_map(|n| cpg.to_var_location(n).filter(|l|
            l.path.to_string()=="a" && matches!(l.kind, crate::data_flow::VarAccessKind::Def)))
        .unwrap();
    let trace = cpg.taint_trace_locations(&[a_loc]);
    let b_seeded = cpg.nodes_at("test.py", 2).into_iter().any(|n|
        cpg.to_var_location(n).is_some_and(|l| l.path.to_string()=="b") && trace.in_frontier(n));
    assert!(!b_seeded);
}
```

- [ ] **Step 2: Run → FAIL** (method missing).
- [ ] **Step 3: Implement** — extract the per-seed BFS body into `fn taint_trace_from_nodes(seeds: &[NodeIndex]) -> Trace`. `taint_trace_locations(&[VarLocation])` maps each `VarLocation` to its node via a new `fn var_location_node(&self, loc: &VarLocation) -> Option<NodeIndex>` that **converts `VarAccessKind → VarAccess`** (`Def→Def`, `Use→Use`) and calls the existing `var_node(file, function, line, path, access)`. `taint_trace(&[(file,line)])` resolves each line to all `Variable` `VarLocation`s on it (existing behavior — keep the multi-function/signature degrade + dedup on this path) and delegates to `taint_trace_locations`.
- [ ] **Step 4: Run → PASS; all `taint_trace*` tests green** (line API unchanged).
- [ ] **Step 5: Commit** — `feat(cpg): node-precise taint_trace_locations; line taint_trace becomes a wrapper`.

### Task 1.2: Per-root reachability + witness variants

**Files:** `src/reasoning/shape.rs`. Test: same file `#[cfg(test)]`.

- [ ] **Step 1: Write the failing test** — a real two-source fixture (assertions, not comments):

```rust
#[test]
fn reachability_and_witness_are_per_root() {
    // def f(): a=input(); b=1; x=a; sink(x); y=b; other(y)
    // seed a and b separately as two roots via taint_trace_locations([a_def, b_def]);
    // sink(x)'s x-use is reached from a's root, NOT b's.
    // assert reachability_for_node_from(cpg, &trace, root_a, x_use) == Reached
    // assert reachability_for_node_from(cpg, &trace, root_b, x_use) == NotReached
    // assert witness_graph_for(cpg, &trace, root_a, x_use).is_some()
    // assert witness_graph_for(cpg, &trace, root_b, x_use).is_none()
}
```

- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** `reachability_for_node_from(cpg, trace, root, sink)` (consult `trace.frontier_by_root[&root]` + boundaries with `b.root == root`) and `witness_graph_for(cpg, trace, root, sink)` (walk `parents_by_root` for that root). `reachability_for_node` = any-root fold; `witness_graph_for_node` = first BTreeMap-order reaching root — both delegate.
- [ ] **Step 4: Run → PASS; existing shape.rs tests green.**
- [ ] **Step 5: Commit** — `feat(reasoning): per-root reachability_for_node_from / witness_graph_for`.

---

## Phase 2 — Seed resolution (`seeds.rs`)

### Task 2.1: `Loc` resolution + 3 `None` cases (merged warning shapes)

**Files:** `src/reasoning/seeds.rs`.

- [ ] **Step 1: Write the failing tests** — Loc with Variables → all `Variable` `VarLocation`s; zero-Variable line → `WarningKind::Reasoning(SeedUnresolved{ seed: "test.py:N — no variable nodes" })`, skipped; the three `None` cases → `QueryError::UnsupportedFile` (missing file) / `QueryError::LocationOutOfRange` (line past EOF) / empty-but-valid (in-file outside any function).
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** `resolve(session, specs) -> Result<SeedSet, QueryError>` Loc arm: `session.repo.files.get(file)` → missing → `UnsupportedFile`; line past EOF → `LocationOutOfRange`; else `cpg.nodes_at(file,line)` filtered to `Variable` → `to_var_location`; zero → `SeedUnresolved`. Use the **merged** `SeedUnresolved { seed: String }` shape. Never `resolve_fn`.
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `feat(reasoning): seeds.rs Loc resolution + None-case truth table`.

### Task 2.2: `Symbol` resolution (parameters; CPG→AST bridge)

**Files:** `src/reasoning/seeds.rs`.

- [ ] **Step 1: Write the failing tests** — `Symbol{name}` → parameters (each param's `Def` at the function-start line); field-only params skipped; zero-param / all-field → empty+`SeedUnresolved`; some unresolved → partial; **ambiguous (`file:None`, name in 2 files) → `QueryError::AmbiguousSymbol` when all sources ambiguous, else warn-skip.**
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** — for `Symbol{name, file}`: pick the file(s) from `session.repo.files` (1 = ok; `file:None` matching >1 → ambiguous rule); `parsed.find_function_by_name(name) -> Node` (`ast.rs:2716`); `parsed.function_parameter_names(&node)` (`:2743`); for each param map to its `Def` `VarLocation` at the function start line, skipping field-only via `parsed.has_bare_references(&node, param)` (`:1780`). Apply edge cases.
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `feat(reasoning): seeds.rs Symbol=parameters (CPG→AST bridge) + ambiguity rule`.

### Task 2.3: Failure precedence + warning order

**Files:** `src/reasoning/seeds.rs`.

- [ ] **Step 1: Write the failing test** — **all** sources fail → `QueryError` of the **first** seed-input-order failure (reuse existing variants; never add one — `error_text` exhaustive, `navigation.rs:13-23`); warnings seed-input-ordered, deduped.
- [ ] **Step 2–4:** implement; PASS.
- [ ] **Step 5: Commit** — `feat(reasoning): seed-resolution failure precedence + warning ordering`.

---

## Phase 3 — `taint_reaches` query core

### Task 3.1: Orchestration — tri-state per sink (+ all-sinks-fail)

**Files:** Create `src/reasoning/taint_reaches.rs`; `mod.rs` `pub mod taint_reaches;`. Test: same file + `tests/reasoning/taint_reaches_test.rs` (register `[[test]] name="reasoning_taint_reaches" path="tests/reasoning/taint_reaches_test.rs"`).

- [ ] **Step 1: Write the failing tests** — intraproc Reached; unrelated sink NotReached; the **`S→I→K` interior-node invariant** (Reached + witness never dead-ends); cross-`(file,function)` sink → **`BoundaryExited`** + sink-located `InterproceduralBoundary{sink}`; **all-sinks-fail → `QueryError` (precedence by seed-input order, sources-then-sinks)**; partial-sinks-fail → `SeedUnresolved` warn + proceed.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** `taint_reaches(session, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>) -> Result<Evidence, QueryError>`: `seeds::resolve` sources (+sinks); collect source `VarLocation`s; `cpg.taint_trace_locations(&src_locs)`; per sink `VarLocation` → one `SinkResult` (`sink: SymbolRef` from the location; `reachability_for_node` tri-state). Build pre-cap `Evidence{ reasoning: Some(ReasoningSummary{..}) }`. Pin the all-sinks-fail precedence in this task.
- [ ] **Step 4: Run → PASS; `cli_nav_compat` byte-identical.**
- [ ] **Step 5: Commit** — `feat(reasoning): taint_reaches orchestration + tri-state per-sink + sink-side truth table`.

### Task 3.2: Cleansing (A4) + multi-source union

**Files:** `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing tests** — Go/Python source with a sanitizer → non-empty `sanitizers_present_in_source_fn` + `path_proven:false`; sink witnessed by two sources → **unioned** sanitizers (§4.6).
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** `crate::algorithms::taint::cleansed_categories_for_source(&session.repo.files, &src_loc)` per source; union across sources witnessing a sink. (Clears the two intentional dead-code warnings.)
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `feat(reasoning): wire A4 cleansing into taint_reaches (multi-source union)`.

---

## Phase 4 — Evidence shaping (frontier + witness)

### Task 4.1: Frontier mode — scored items

**Files:** `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing test** — sinks omitted → `items` each `why:[Reason::Reasoning(TaintedBy{ source /* min-depth root, BTreeMap tie-break */, sanitizers_present_in_source_fn, path_proven:false })]`; `score` sources `1.0` / downstream `0.6^depth`; tie-break `(file,function,line,path,ordinal)`; `reasoning.reachability == None`; `per_sink` empty; counts pre-cap; items XOR graph.
- [ ] **Step 2–4:** implement; PASS; Option-C.
- [ ] **Step 5: Commit** — `feat(reasoning): taint_reaches frontier mode (scored TaintedBy items)`.

### Task 4.2: Witness mode — `per_sink` + unioned graph (pre-cap)

**Files:** `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing tests** — `per_sink` one `SinkResult` per resolved sink `VarLocation` (tri-state each); Reached → witness via `witness_graph_for`; all witnesses **unioned** into one `GraphPayload` (dedup by full identity `(file,function,line,path,kind,ordinal)`, self-edges dropped); `graph_node` indexes the sink node in the union graph (pre-cap; never `None` here for Reached); `reasoning.reachability` = aggregate; graph XOR items; edge kinds `DataFlow`/`AssignmentPropagation`/`RecoveredDefUse`, no `ControlFlow`.
- [ ] **Step 2–4:** implement (pre-cap, no clipping here); PASS; Option-C.
- [ ] **Step 5: Commit** — `feat(reasoning): taint_reaches witness mode (per_sink + unioned witness graph, pre-cap)`.

### Task 4.3: Warnings

**Files:** `src/reasoning/taint_reaches.rs`.

- [ ] **Step 1: Write the failing tests** — partial seed failure → `SeedUnresolved{seed}` (seed-input order); each queried-sink `BoundaryEdge` → sink-located `InterproceduralBoundary{sink}`; witness-mode sanitizer presence → `Cleansed{source_function}`. Deduped, ordered.
- [ ] **Step 2–4:** implement from `SeedSet.warnings` + `Trace.boundary` + cleansing; PASS; **`cli_nav_compat` byte-identical** (additive variants byte-safe via catch-all `navigation.rs:73` + `{:?}`).
- [ ] **Step 5: Commit** — `feat(reasoning): taint_reaches warnings (SeedUnresolved/InterproceduralBoundary/Cleansed)`.

---

## Phase 5 — MCP surface + clip-repair

### Task 5.1: `reason_v1()` registry + smoke update

**Files:** `src/mcp/registry.rs`, the server `run`. Tests: `tests/mcp/registry_test.rs` (register `path` + `required-features=["mcp"]`), and **update `tests/mcp/smoke_test.rs:24` 6 → 7**.

- [ ] **Step 1: Write/adjust the failing tests** — `nav_v1()` stays **==6** (frozen unit test untouched); `reason_v1()` exists; combined served registry **==7**; reasoning tools absent from `nav_v1()`; the process **smoke test now asserts 7** tools.
- [ ] **Step 2: Run → FAIL** (smoke still says 6).
- [ ] **Step 3: Implement** `reason_v1()` (`reason_taint_reaches`); serve `nav_v1()`+`reason_v1()`; update smoke to 7. All `#[cfg(feature="mcp")]`.
- [ ] **Step 4: Run → PASS** (`cargo test --features mcp`).
- [ ] **Step 5: Commit** — `feat(mcp): reason_v1 registry + reason_taint_reaches registration (combined ==7)`.

### Task 5.2: Dispatch + schema + `SeedInput→SeedSpec`

**Files:** `src/mcp/{tools,input}.rs`.

- [ ] **Step 1: Write the failing tests** — schema: `sources` required `[SeedInput]` `minItems:1`; `sinks` optional, **empty `sinks:[]` invalid**; `max_results` default `MAX_RESULTS_DEFAULT`; `verbosity` `concise|detailed` default `concise`. Smoke `reason_taint_reaches` exchange returns valid `Evidence`. **Concise-vs-detailed:** default concise frontier items have `why` cleared (score+symbol survive, `reasoning` summary survives); detailed keeps `TaintedBy`. Description states params-only scope + steers to `per_sink` + notes `verbosity:detailed` for frontier rationale.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** — parse `SeedInput` (reuse, `deny_unknown_fields`); add an explicit **`SeedInput → SeedSpec`** conversion (NOT `to_triple`); dispatch to `taint_reaches`; shape via the **public `shape_result(ev, retained, truncated, verbosity, max_results)`**. Reject empty `sinks:[]`.
- [ ] **Step 4: Run → PASS** (`--features mcp`); `cli_nav_compat` byte-identical.
- [ ] **Step 5: Commit** — `feat(mcp): reason_taint_reaches schema, dispatch, SeedInput→SeedSpec`.

### Task 5.2b: `graph_node` clip-repair in `shape_result`

**Files:** `src/mcp/output.rs` (`shape_result`). Test: `src/mcp/output.rs` `#[cfg(test)]`.

- [ ] **Step 1: Write the failing test** — an `Evidence` with `reasoning.per_sink[i].graph_node = Some(k)` and a `graph` of `< k+1` nodes after clipping → after `shape_result` with a small `max_results`, **no `graph_node` points past `graph.nodes.len()`** (clipped → `None`); `per_sink` itself stays complete (the contracted serialization test, `types.rs:144-152`).
- [ ] **Step 2: Run → FAIL** (`shape_result` clips graph but not `graph_node`).
- [ ] **Step 3: Implement** — after `shape_result` clips `graph.nodes`, walk `reasoning.per_sink` and set any `graph_node >= kept` to `None`. Behind `if let Some(reasoning) = &mut shaped.reasoning`.
- [ ] **Step 4: Run → PASS;** `cli_nav_compat` byte-identical (no `reasoning` on nav goldens → untouched).
- [ ] **Step 5: Commit** — `fix(mcp): repair reasoning graph_node indices after shape_result graph clip`.

### Task 5.3: Boundary-bypass regression guard (§11)

**Files:** `tests/reasoning/boundary_contract_test.rs` (register `path`).

- [ ] **Step 1: Write the test** — pin `taint_forward_cfg`'s deliberate cross-`(file,function)` bypass as a contract (load-bearing for the Taint slice AND, inversely, `taint_reaches`/Phase-IP).
- [ ] **Step 2: Run → PASS.**
- [ ] **Step 3: Commit** — `test(reasoning): pin taint_forward_cfg cross-function bypass contract`.

### Task 5.4: Docs (closing)

**Files:** `CLAUDE.md` (six → seven tools, add `reason_taint_reaches`), `src/navigation/types.rs` (the `Evidence.graph` doc comment — note witness mode also produces a graph).

- [ ] **Step 1–2:** edit; `cargo test` green.
- [ ] **Step 3: Commit** — `docs: reason_taint_reaches in CLAUDE.md + Evidence.graph witness-mode note`.

---

## Recurring gate

`cargo test --test cli_nav_compat` **byte-identical** (Option C; NOT the `review` preset, `nav_compat_test.rs:17-22`). `cargo fmt && cargo test` + `cargo test --features mcp` green. Re-warm the prism nav cache before any prism-wired review.

---

## Self-Review

**Spec coverage:** §2 (3.1, 5.2) · §3 (2.1–2.3) · §4 (3.x, 4.x; capping moved to MCP per finding 8) · §5 (Phase 3–4) · §6 (0.1, 0.2) · §7 (2.x, 3.1 sink-side, 4.3) · §8 (consumed verbatim — merged shapes) · §9 (5.1, 5.2) · §10 (per-task tests + 5.2b clipped-witness) · §11 (5.3). Foundational items (node-precise seeding, per-root variants) → Phase 1. `graph_node` truncation (3rd contracted followup) → 5.2b.

**Type consistency:** `taint_trace_locations(&[VarLocation])`; `reachability_for_node_from(.., root, sink)` / `witness_graph_for(.., root, sink)`; `resolve(session, &[SeedSpec]) -> Result<SeedSet, QueryError>`; `taint_reaches(session, &[SeedSpec], Option<&[SeedSpec]>) -> Result<Evidence, QueryError>` (pre-cap); `shape_result(ev, retained, truncated, verbosity, max_results)`. Warning shapes = merged single-string. `&session.repo.files` for the A4 adapter. `SeedInput → SeedSpec` explicit. `VarAccessKind → VarAccess` named in 1.1.

**Placeholder scan:** no comment-only "tests" (1.2 has a real fixture sketch); every new test target gets a `path`/`required-features` registration step.

---

## Execution Handoff

**Plan revised (rev 2) — all plan-review BLOCKERs/MAJORs/MINORs folded.** Next: **containerized codex implement** (`a2a-bridge implement`, TDD, verify=fmt+build+test --locked) OR subagent-driven TDD; then in-depth code-review vs main (codex/gpt-5.5 + claude/fable) to convergence → squash to docs+feat pair → merge. The plan-review record is `docs/prism-query-layer/planB-plan-review-MCP-2026-06-10.md`.
