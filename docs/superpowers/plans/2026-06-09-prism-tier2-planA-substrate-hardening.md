# Prism Tier 2 — Plan A (Substrate Hardening) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the three gating substrate slices (A3 + A4 + A7) that intraprocedural `taint_reaches` v1 consumes, additively, with zero change to diff-review or nav output.

**Architecture:** A3 adds a single inline-CFG-filtered predecessor BFS (`taint_trace`) on the production `CodePropertyGraph` petgraph engine, returning a `Trace { frontier, parents, boundary }` whose witness is dead-end-free by construction. A4 exposes one `pub(crate)` reasoning adapter over the *already-wired* sanitizer recognizers. A7 creates the always-compiled `src/reasoning/` module (Evidence vocabulary + the `shape.rs` output seam + `seeds.rs` type defs) that Plan B fills.

**Tech Stack:** Rust, `petgraph` (`DiGraph<CpgNode, CpgEdge>`), `serde`.

**Source specs:** `docs/superpowers/specs/2026-06-09-prism-tier2-planA-substrate-hardening-design.md` and `…-taint-reaches-design.md`. Revised per `docs/prism-query-layer/planA-plan-review-MCP-2026-06-09.md` (folds blockers B1–B7 + majors M1–M6 + minors m1–m6).

> **GLOBAL OPTION-C PROOF SET (run ALL of these at every task's commit boundary — B6):**
> ```
> cargo test --test cli_nav_compat   # nav + LeftFlow diff-review goldens, byte-unchanged
> cargo build && cargo build --features mcp
> cargo test && cargo test --features mcp
> cargo fmt --check
> ```
> **Never** assert byte-identity through the aggregate `review` preset — it is non-deterministic (`tests/cli/nav_compat_test.rs:17-22`).

**Verified API surface (`file:line`):** `CodePropertyGraph.graph: DiGraph<CpgNode,CpgEdge>` (`build.rs:29`); `CodePropertyGraph::build(&BTreeMap<String,ParsedFile>) -> CodePropertyGraph` (the test harness, `src/cpg/tests.rs:98-102`); `CpgNode::Variable { path: AccessPath, file, function, line, access: VarAccess }` (`types.rs:12`); `VarAccess::{Def,Use}`; `CpgEdge::{DataFlow,ControlFlow,Call,Return,Contains,FieldOf}` (`types.rs:78`); `nodes_at(file,line)->Vec<NodeIndex>` (`query.rs:27`); `to_var_location(NodeIndex)->Option<VarLocation>` (`query.rs:256`); `location_index: BTreeMap<(String,usize),Vec<NodeIndex>>` (pub(crate), `build.rs`); `cfg_reachable_lines(file,line)->BTreeSet<(String,usize)>` **excludes the start line** (`cfg_queries.rs:105`); `cfg_reachable_including_continuation(file,line,&set)->bool` (`cfg_queries.rs:63`, **private — Task 1 makes it `pub(crate)`**); `has_cfg_edges()->bool` (`cfg_queries.rs:17`); `taint_forward_cfg` pure-taint fallback when `!has_cfg_edges()` (`cfg_queries.rs:133-135`); `VarLocation { file, function, line, path: AccessPath, kind: VarAccessKind }` (`data_flow.rs:13`); `VarAccessKind::{Def,Use}`; `AccessPath: Display` (`access_path.rs:256`); arg→param DataFlow edges land on the **callee param Def at the function-start line** (`build.rs:387-400`, `data_flow.rs:204-249`); `function_body_cleansed_for(&ParsedFile, line, SanitizerCategory)->bool` private (`taint.rs:10581`); `is_js_ts_language(Language)->bool`, `active_recognizers()->impl Iterator<&'static SanitizerRecognizer>` (`sanitizers/mod.rs:19`); `SanitizerCategory::{Xss,Sqli,Ssrf,Deserialization,OsCommand,PathTraversal}` derives `Copy,Ord,Eq,Hash` (`frameworks/mod.rs:27`); `Evidence { query, items, truncated, warnings, graph: Option<GraphPayload> }` (`navigation/types.rs:119`); `Location { file, start_line, end_line }` (`navigation/types.rs:4`); `GraphNode { symbol: Option<SymbolRef>, location: Location }`, `GraphEdge { from: usize, to: usize, kind: String }` (`:99`); `Reason` + `WarningKind` (`:42,:79`) — `render()` has a **catch-all `Reason` arm at `output/navigation.rs:73`** and renders `WarningKind` via `{:?}` at `:80`, so additive variants are byte-safe **without new render arms**; `error_text()` (`output/navigation.rs:12`) is an exhaustive 5-arm `QueryError` match (no new `QueryError` variant); `SymbolRef::Variable { file, function, line, path: String, access: String, ordinal }` (`:10`); `lib.rs` module list (`:32-56`, no `reasoning`); `NavigationSession { repo, index }`, `index.cpg` (`navigation/mod.rs:15-27`); **`Evidence {…}` is constructed in `src/navigation/queries.rs`, `src/navigation/module_graph.rs`, `src/output/navigation.rs`, `src/mcp/output.rs` (inside a `#[cfg(test)]` module), and `tests/navigation/types_test.rs`** (B5).

---

## Task 0: Shared test harness helper

Every in-crate test below builds a CPG from source. Define the helper **once per test module** that uses it (Tasks 1 and 5; Task 7 is a separate crate and gets its own via the public path). It is a 3-line wrapper of the proven pattern at `src/cpg/tests.rs:98-102`.

```rust
fn build_python_cpg(src: &str) -> CodePropertyGraph {
    let parsed = crate::ast::ParsedFile::parse("test.py", src, crate::languages::Language::Python).unwrap();
    let mut files = std::collections::BTreeMap::new();
    files.insert("test.py".to_string(), parsed);
    CodePropertyGraph::build(&files)
}
```
> If a Python fixture yields too few CFG/DataFlow edges for an assertion, mirror an existing **Python** CPG test in the suite (or switch that one fixture to C like `src/cpg/tests.rs:88`). The helper itself is identical; only `Language::Python` → `Language::C` and the `.py`→`.c` path change.

---

## Task 1: A3 — `Trace` types + the inline-CFG-filtered BFS

**Files:** Create `src/cpg/trace.rs`; Modify `src/cpg.rs` (`mod trace;` + re-exports), `src/cpg/cfg_queries.rs:63` (`pub(crate)`); Test: append to `src/cpg/tests.rs`.

- [ ] **Step 1: Make the continuation helper reachable.** In `src/cpg/cfg_queries.rs:63`, change `fn cfg_reachable_including_continuation` → `pub(crate) fn cfg_reachable_including_continuation`.

- [ ] **Step 2: Add the `build_python_cpg` helper (Task 0) to `src/cpg/tests.rs`** (top of the file, after the existing `use` lines).

- [ ] **Step 3: Write the failing straight-line frontier test.**
```rust
#[test]
fn test_taint_trace_straight_line_frontier() {
    let src = "def f():\n    user = input()\n    x = user\n    sink(x)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let lines: std::collections::BTreeSet<usize> = trace.frontier.iter()
        .filter_map(|&i| cpg.to_var_location(i).map(|l| l.line)).collect();
    assert!(lines.contains(&3) && lines.contains(&4), "x (l3) and the use in sink(x) (l4): {lines:?}");
    assert!(trace.boundary.is_empty());
}
```
Run: `cargo test --lib test_taint_trace_straight_line_frontier` — Expected: FAIL (`no method taint_trace`).

- [ ] **Step 4: Implement `src/cpg/trace.rs`.**
```rust
//! A3: single inline-CFG-filtered predecessor BFS over the production petgraph.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use crate::cpg::build::CodePropertyGraph;
use crate::cpg::types::{CpgEdge, CpgNode, VarAccess};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation { DataFlow, AssignmentPropagation }

/// A def-use edge crossing a `(file,function)` boundary — recorded, never traversed in v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryEdge { pub root: NodeIndex, pub from: NodeIndex, pub to: NodeIndex }

#[derive(Debug, Clone, Default)]
pub struct Trace {
    pub frontier: BTreeSet<NodeIndex>,
    pub parents: BTreeMap<NodeIndex, (NodeIndex, Relation)>,
    pub boundary: Vec<BoundaryEdge>,
}

impl CodePropertyGraph {
    /// Single inline-CFG-filtered predecessor BFS. Every frontier member is reachable via an
    /// all-CFG-valid intraprocedural path, so the parent walk-back never dead-ends. Determinism:
    /// neighbors sorted by NodeIndex; first enqueue wins the parent slot; DataFlow beats same-line.
    pub fn taint_trace(&self, sources: &[(String, usize)]) -> Trace {
        let has_cfg = self.has_cfg_edges();
        let mut trace = Trace::default();
        let mut enqueued: BTreeSet<NodeIndex> = BTreeSet::new();
        for (file, line) in sources {
            let cfg_set = if has_cfg { self.cfg_reachable_lines(file, *line) } else { BTreeSet::new() };
            for &root in &self.nodes_at(file, *line) {
                if !matches!(self.graph[root], CpgNode::Variable { .. }) { continue; }
                let src_fn = self.node_file_fn(root);
                let mut queue = VecDeque::new();
                if enqueued.insert(root) { trace.frontier.insert(root); queue.push_back(root); }
                while let Some(node) = queue.pop_front() {
                    for (next, rel) in self.taint_neighbors(node) {
                        if self.node_file_fn(next) != src_fn {
                            trace.boundary.push(BoundaryEdge { root, from: node, to: next });
                            continue;
                        }
                        if !self.cfg_valid(&src_fn.0, *line, &cfg_set, has_cfg, next) { continue; }
                        if enqueued.insert(next) {
                            trace.frontier.insert(next);
                            trace.parents.insert(next, (node, rel));
                            queue.push_back(next);
                        }
                    }
                }
            }
        }
        trace
    }

    fn node_file_fn(&self, idx: NodeIndex) -> (String, String) {
        match &self.graph[idx] {
            CpgNode::Variable { file, function, .. } => (file.clone(), function.clone()),
            _ => (String::new(), String::new()),
        }
    }

    fn taint_neighbors(&self, node: NodeIndex) -> Vec<(NodeIndex, Relation)> {
        let mut out = Vec::new();
        let mut df: Vec<NodeIndex> = self.graph.edges(node)
            .filter(|e| matches!(e.weight(), CpgEdge::DataFlow)).map(|e| e.target()).collect();
        df.sort_by_key(|i| i.index());
        out.extend(df.into_iter().map(|t| (t, Relation::DataFlow)));
        if let CpgNode::Variable { access: VarAccess::Use, file, line, .. } = &self.graph[node] {
            if let Some(at) = self.location_index.get(&(file.clone(), *line)) {
                let mut same: Vec<NodeIndex> = at.iter().copied()
                    .filter(|&o| matches!(self.graph[o], CpgNode::Variable { access: VarAccess::Def, .. }))
                    .collect();
                same.sort_by_key(|i| i.index());
                out.extend(same.into_iter().map(|t| (t, Relation::AssignmentPropagation)));
            }
        }
        out
    }

    /// Per-node CFG validity (over-approximation; per-node, not pairwise). No CFG => pure-taint
    /// fallback (always valid). Same source line admitted explicitly (cfg_reachable_lines EXCLUDES
    /// the start line); else membership or multi-line continuation.
    fn cfg_valid(&self, src_file: &str, src_line: usize, cfg_set: &BTreeSet<(String, usize)>,
                 has_cfg: bool, target: NodeIndex) -> bool {
        if !has_cfg { return true; }
        let (tfile, tline) = match &self.graph[target] {
            CpgNode::Variable { file, line, .. } => (file.clone(), *line),
            _ => return false,
        };
        if tfile == src_file && tline == src_line { return true; }      // same source line (start-line landmine)
        cfg_set.contains(&(tfile.clone(), tline))
            || self.cfg_reachable_including_continuation(&tfile, tline, cfg_set)
    }
}
```

- [ ] **Step 5: Register in `src/cpg.rs`.** Add `mod trace;` and `pub use trace::{BoundaryEdge, Relation, Trace};`.

- [ ] **Step 6: Run the straight-line test — PASS.** `cargo test --lib test_taint_trace_straight_line_frontier`.

- [ ] **Step 7: Same-line propagation test (spec must-pass — M1).**
```rust
#[test]
fn test_taint_trace_same_line_assignment() {
    let src = "def f():\n    x = source(); y = x\n    sink(y)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let lines: std::collections::BTreeSet<usize> =
        trace.frontier.iter().filter_map(|&i| cpg.to_var_location(i).map(|l| l.line)).collect();
    assert!(lines.contains(&3), "y (def on l2) must reach the use in sink(y) on l3: {lines:?}");
}
```
Run it — Expected: PASS (the `tline == src_line` admission in `cfg_valid` handles the start-line exclusion).

- [ ] **Step 8: No-dead-end-witness invariant test (the load-bearing one — M3 strengthened).** Build the seeded-root set, assert every frontier member's walk-back terminates **in that root set**:
```rust
#[test]
fn test_taint_trace_no_dead_end_witness() {
    let src = "def f():\n    a = input()\n    b = a\n    c = b\n    sink(c)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let roots: std::collections::BTreeSet<_> = cpg.nodes_at("test.py", 2).into_iter().collect();
    for &node in &trace.frontier {
        let mut cur = node; let mut g = 0;
        while let Some((p, _)) = trace.parents.get(&cur) { cur = *p; g += 1; assert!(g < 1000); }
        assert!(roots.contains(&cur), "witness walk-back must end at a seeded source root");
    }
}
```

- [ ] **Step 9: No-path + absent-CFG fallback tests (spec must-pass — M2).**
```rust
#[test]
fn test_taint_trace_no_path_empty_downstream() {
    let src = "def f():\n    a = input()\n    b = 1\n    sink(b)\n"; // b is not tainted by a
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let l4: bool = trace.frontier.iter().any(|&i| cpg.to_var_location(i).map_or(false, |l| l.line == 4));
    assert!(!l4, "b on line 4 is independent of a; must not be in frontier");
}
#[test]
fn test_taint_trace_no_cfg_falls_back_to_pure_taint() {
    // A language/fixture with no CFG edges still returns DFG-reachable frontier (cfg_valid !has_cfg => true).
    // Mirror an existing fixture in the suite where has_cfg_edges() is false; assert the downstream def is present.
    // (If all Python fixtures build CFG edges, construct a minimal CPG via the same harness for a snippet that
    //  produces DataFlow but no ControlFlow, and assert frontier is non-empty.)
}
```
> Fill the `no_cfg` body against a fixture where `cpg.has_cfg_edges()` is false (assert it, then assert a DataFlow-downstream node is in `frontier`). If every fixture has CFG edges, assert the fallback path directly is unnecessary and delete this test, noting why.

- [ ] **Step 10: DataFlow-wins-same-line tie determinism test (spec §3 — M2).** Construct a fixture where a target is reachable both by a DataFlow edge and same-line propagation; assert `trace.parents[&target].1 == Relation::DataFlow`. (If no fixture produces both for one node, document that `taint_neighbors` emits DataFlow first so first-enqueue guarantees it, and assert the ordering in `taint_neighbors` via a focused unit test.)

- [ ] **Step 11: Boundary test (B4 — boundary target is the callee param Def at the function-start line).**
```rust
#[test]
fn test_taint_trace_records_boundary_at_param_def() {
    let src = "def g(p):\n    sink(p)\n\ndef f():\n    u = input()\n    g(u)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]); // u = input()
    assert!(!trace.boundary.is_empty(), "u flows into g's param across a function boundary");
    for be in &trace.boundary {
        assert!(!trace.frontier.contains(&be.to), "cross-function target not traversed");
        // the boundary target resolves to g's param def — line 1 (def g(p)), NOT line 2 (sink(p)).
        if let Some(loc) = cpg.to_var_location(be.to) {
            assert_eq!(loc.function, "g", "boundary target is g's parameter");
        }
    }
}
```
> If the CPG does not build an arg→param DataFlow edge for this fixture, mirror an interprocedural fixture from `src/cpg/tests.rs` / `tests/ast/dfg_test.rs` that does, and keep the function-equals-`g` assertion.

- [ ] **Step 12: Full Option-C proof + commit.**
Run the **global proof set**. Expected: all green (A3 is purely additive).
```bash
git add src/cpg/trace.rs src/cpg.rs src/cpg/cfg_queries.rs src/cpg/tests.rs
git commit -m "feat(cpg): add taint_trace inline-CFG-filtered predecessor BFS (A3)"
```

---

## Task 2: A4 — the `cleansed_categories_for_source` adapter

**Files:** Modify `src/algorithms/taint.rs`; Test: inline in `taint.rs`.

> **M4 (signature divergence, resolved):** the specs write `cleansed_categories_for_source(source: VarLocation)`. `function_body_cleansed_for` needs the `&ParsedFile`, which the reasoning caller resolves from the session's `files` map. **Pinned signature:** `pub(crate) fn cleansed_categories_for_source(files: &BTreeMap<String, ParsedFile>, source: &VarLocation) -> Vec<String>` — keeps the `VarLocation` source the spec names, and is what Plan B calls (`&session.repo.files`, `&source_loc`). This realizes spec §4; not a silent change.

- [ ] **Step 1: Write the failing test.**
```rust
#[test]
fn test_cleansed_categories_for_source_python_xss() {
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use crate::data_flow::{VarLocation, VarAccessKind};
    use crate::access_path::AccessPath;
    let src = "def f(u):\n    safe = html.escape(u)\n    return safe\n";
    let mut files = std::collections::BTreeMap::new();
    files.insert("t.py".to_string(), ParsedFile::parse("t.py", src, Language::Python).unwrap());
    let source = VarLocation { file: "t.py".into(), function: "f".into(), line: 2,
        path: AccessPath { base: "u".into(), fields: vec![] }, kind: VarAccessKind::Use };
    let cats = cleansed_categories_for_source(&files, &source);
    assert!(cats.iter().any(|c| c == "xss"), "html.escape ⇒ xss: {cats:?}");
}
```
(Mirror the exact `ParsedFile::parse`/`AccessPath` constructors the existing taint tests use if they differ.)

- [ ] **Step 2: Run — FAIL** (`cannot find function cleansed_categories_for_source`).

- [ ] **Step 3: Implement in `src/algorithms/taint.rs`.**
```rust
/// A4: the single reasoning-facing cleansing adapter. Returns the sanitizer categories present
/// in the SOURCE FUNCTION BODY (NOT path-proof) as lowercase strings. Gated to Go/Python/JS-TS
/// exactly like apply_cleansers; honest-empty otherwise.
pub(crate) fn cleansed_categories_for_source(
    files: &std::collections::BTreeMap<String, ParsedFile>,
    source: &crate::data_flow::VarLocation,
) -> Vec<String> {
    let parsed = match files.get(&source.file) { Some(p) => p, None => return Vec::new() };
    if !matches!(parsed.language, Language::Go | Language::Python) && !is_js_ts_language(parsed.language) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let cats: std::collections::BTreeSet<crate::frameworks::SanitizerCategory> =
        crate::sanitizers::active_recognizers().map(|r| r.category).collect();
    for category in cats {
        if function_body_cleansed_for(parsed, source.line, category) {
            out.push(sanitizer_category_str(category).to_string());
        }
    }
    out
}

fn sanitizer_category_str(c: crate::frameworks::SanitizerCategory) -> &'static str {
    use crate::frameworks::SanitizerCategory::*;
    match c {
        Xss => "xss", Sqli => "sqli", Ssrf => "ssrf",
        Deserialization => "deserialization", OsCommand => "os_command", PathTraversal => "path_traversal",
    }
}
```

- [ ] **Step 4: Run — PASS.** `cargo test --lib test_cleansed_categories_for_source_python_xss`.

- [ ] **Step 5: Honest-empty test (Rust → empty).**
```rust
#[test]
fn test_cleansed_categories_for_source_rust_empty() {
    use crate::ast::ParsedFile; use crate::languages::Language;
    use crate::data_flow::{VarLocation, VarAccessKind}; use crate::access_path::AccessPath;
    let mut files = std::collections::BTreeMap::new();
    files.insert("t.rs".to_string(), ParsedFile::parse("t.rs", "fn f(u: &str) -> String { u.to_string() }", Language::Rust).unwrap());
    let s = VarLocation { file: "t.rs".into(), function: "f".into(), line: 1,
        path: AccessPath { base: "u".into(), fields: vec![] }, kind: VarAccessKind::Use };
    assert!(cleansed_categories_for_source(&files, &s).is_empty());
}
```

- [ ] **Step 6: Add the dated A4→`src/sanitizers/` relocation obligation (M6).** Add a comment above the adapter:
```rust
// [→Phase-IP / A2] TEMPORARY layering inversion: reasoning reaches into taint.rs. Relocate
// cleansed_categories_for_source + function_body_cleansed_for into src/sanitizers/ when A2 lands.
// Tracked: docs/superpowers/specs/2026-06-09-prism-tier2-planA-substrate-hardening-design.md §9.
```
(If the repo uses a tracker, file an issue and cite it here instead.)

- [ ] **Step 7: Proof + commit (M5 — correct sanitizer surface).**
Run the **global proof set** PLUS `cargo test --test algo_taxonomy_sanitizers && cargo test --test algo_taxonomy_sanitizers_python`.
Expected: green — sanitizer-taxonomy fixtures byte-unchanged (only a new `pub(crate)` fn added).
```bash
git add src/algorithms/taint.rs
git commit -m "feat(taint): add cleansed_categories_for_source reasoning adapter (A4)"
```

---

## Task 3: A7.1 — create the `src/reasoning/` module

**Files:** Create `src/reasoning/{mod,seeds,shape}.rs`; Modify `src/lib.rs`.

- [ ] **Step 1:** Create `src/reasoning/mod.rs`:
```rust
//! Tier-2 reasoning layer (always compiled; only MCP tool registration is `mcp`-gated).
//! Ephemeral, read-only computation over the production CPG — no overlay data structure.
pub mod seeds;
pub mod shape;
```
- [ ] **Step 2:** In `src/lib.rs`, insert (alphabetically between `react_hooks` and `repo_loader`): `pub mod reasoning;`.
- [ ] **Step 3:** Create placeholders so the crate compiles: `src/reasoning/seeds.rs` and `src/reasoning/shape.rs` each with a `//!` doc comment.
- [ ] **Step 4: Full Option-C proof set + commit.**
```bash
git add src/lib.rs src/reasoning/
git commit -m "feat(reasoning): scaffold always-compiled src/reasoning/ module (A7)"
```

---

## Task 4: A7.2 — Evidence vocabulary (additive, byte-safe)

**Files:** Modify `src/navigation/types.rs`; add `reasoning: None` to **every** `Evidence` constructor (B5: `src/navigation/queries.rs`, `src/navigation/module_graph.rs`, `src/output/navigation.rs`, `src/mcp/output.rs` incl. its `#[cfg(test)]`, `tests/navigation/types_test.rs`); Test: inline in `types.rs`. **No `render()` change needed** (m1: the catch-all `Reason` arm at `output/navigation.rs:73` + `{:?}` `WarningKind` rendering already cover additive variants byte-safely).

- [ ] **Step 1: Failing serde byte-safety test** (append to the `types.rs` test module):
```rust
#[test]
fn test_evidence_reasoning_omitted_when_none() {
    let e = Evidence { query: "q".into(), items: vec![], truncated: false, warnings: vec![], graph: None, reasoning: None };
    assert!(!serde_json::to_string(&e).unwrap().contains("reasoning"));
}
```
- [ ] **Step 2: Run — FAIL** (no field `reasoning`).
- [ ] **Step 3: Add the types + the additive field** to `src/navigation/types.rs`:
```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ReasoningReason {
    TaintedBy { source: SymbolRef, sanitizers_present_in_source_fn: Vec<String>, path_proven: bool },
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ReasoningWarning {
    SeedUnresolved { seed: String },
    InterproceduralBoundary { sink: String },
    Cleansed { source_function: String },
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Reachability { Reached, NotReached, BoundaryExited }
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SinkResult { pub sink: SymbolRef, pub reachability: Reachability, pub graph_node: Option<usize> }
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReasoningSummary {
    pub reachability: Option<Reachability>, pub per_sink: Vec<SinkResult>,
    pub source_count: usize, pub frontier_count: usize,
}
```
Add `Reasoning(ReasoningReason),` to `enum Reason`; `Reasoning(ReasoningWarning),` to `enum WarningKind` (place new arms **before** the `render()` catch-all if you ever add explicit arms — but none are needed); and to `struct Evidence` after `graph`:
```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningSummary>,
```
- [ ] **Step 4: Add `reasoning: None` to every `Evidence` constructor.** Run `cargo build` and `cargo build --features mcp` — the compiler lists each site (the `mcp/output.rs` one is `#[cfg(test)]`, so a feature *build* alone won't flag it; also run `cargo test --features mcp --no-run`). Add `reasoning: None,` to all listed sites (all nav/test sites ⇒ output stays byte-identical).
- [ ] **Step 5: Run the test + full proof + the navigation_types target (B5) + commit.**
Run the **global proof set** PLUS `cargo test --test navigation_types` PLUS `cargo test --features mcp`.
Expected: PASS; nav goldens byte-unchanged.
```bash
git add src/navigation/types.rs src/navigation/queries.rs src/navigation/module_graph.rs src/output/navigation.rs src/mcp/output.rs tests/navigation/types_test.rs
git commit -m "feat(reasoning): additive Evidence reasoning vocabulary, byte-safe (A7)"
```

---

## Task 5: A7.3 — `shape.rs` shaper functions

**Files:** Modify `src/reasoning/shape.rs`; Test: inline.

- [ ] **Step 1: Failing tri-state test** (define `build_python_cpg` in this test module too — Task 0):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpg::CodePropertyGraph;
    use crate::navigation::types::Reachability;
    fn build_python_cpg(src: &str) -> CodePropertyGraph { /* Task 0 body */ }
    #[test]
    fn test_reachability_tri_state() {
        let src = "def g(p):\n    sink(p)\n\ndef f():\n    u = input()\n    v = u\n    g(u)\n";
        let cpg = build_python_cpg(src);
        let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]);
        assert_eq!(reachability_at(&cpg, &trace, "test.py", 6), Reachability::Reached);       // v = u
        assert_eq!(reachability_at(&cpg, &trace, "test.py", 2), Reachability::BoundaryExited); // sink(p) inside g
        assert_eq!(reachability_at(&cpg, &trace, "test.py", 1), Reachability::NotReached);
    }
}
```
- [ ] **Step 2: Run — FAIL** (`reachability_at` not found).
- [ ] **Step 3: Implement `src/reasoning/shape.rs`** (B2 imports via re-exports; B3 `Location` fields; B4 boundary-by-function; m2 access from kind; m4 no `Verbosity` enum):
```rust
use std::collections::BTreeMap;
use petgraph::graph::NodeIndex;
use crate::cpg::{CodePropertyGraph, Relation, Trace};
use crate::navigation::types::{GraphEdge, GraphNode, GraphPayload, Location, Reachability, SymbolRef};

/// Tri-state reachability of a (file,line) sink. BoundaryExited when the sink sits in a function
/// that a boundary edge flows into (taint exits toward it) — distinct from NotReached.
pub fn reachability_at(cpg: &CodePropertyGraph, trace: &Trace, file: &str, line: usize) -> Reachability {
    let nodes = cpg.nodes_at(file, line);
    if nodes.iter().any(|n| trace.frontier.contains(n)) { return Reachability::Reached; }
    let sink_fns: Vec<(String, String)> = nodes.iter().map(|&n| node_file_fn(cpg, n)).collect();
    let boundary_fns: Vec<(String, String)> = trace.boundary.iter().map(|b| node_file_fn(cpg, b.to)).collect();
    if sink_fns.iter().any(|sf| sf.1 != *"" && boundary_fns.contains(sf)) {
        return Reachability::BoundaryExited;
    }
    Reachability::NotReached
}

fn node_file_fn(cpg: &CodePropertyGraph, n: NodeIndex) -> (String, String) {
    cpg.to_var_location(n).map(|l| (l.file, l.function)).unwrap_or_default()
}

/// Witness GraphPayload for a reached sink: parent walk-back, relation-named edges, full node
/// identity. NodeIndex space → Location only here, at the shape boundary.
pub fn witness_graph(cpg: &CodePropertyGraph, trace: &Trace, file: &str, line: usize) -> GraphPayload {
    let mut chain = Vec::new();
    if let Some(&sink) = cpg.nodes_at(file, line).iter().find(|n| trace.frontier.contains(n)) {
        let mut cur = sink; chain.push(cur);
        while let Some((p, _)) = trace.parents.get(&cur) { cur = *p; chain.push(cur); }
        chain.reverse();
    }
    let mut idx_of: BTreeMap<NodeIndex, usize> = BTreeMap::new();
    let mut nodes = Vec::new();
    for &n in &chain {
        idx_of.entry(n).or_insert_with(|| { nodes.push(node_of(cpg, n)); nodes.len() - 1 });
    }
    let mut edges = Vec::new();
    for w in chain.windows(2) {
        let (from, to) = (w[0], w[1]);
        if from == to { continue; }
        let kind = match trace.parents.get(&to) {
            Some((_, Relation::AssignmentPropagation)) => "AssignmentPropagation",
            _ => "DataFlow",
        };
        edges.push(GraphEdge { from: idx_of[&from], to: idx_of[&to], kind: kind.to_string() });
    }
    GraphPayload { nodes, edges }
}

fn node_of(cpg: &CodePropertyGraph, n: NodeIndex) -> GraphNode {
    let loc = cpg.to_var_location(n);
    let (file, line, function, path, access) = match &loc {
        Some(l) => (l.file.clone(), l.line, l.function.clone(), l.path.to_string(),
                    match l.kind { crate::data_flow::VarAccessKind::Def => "def", crate::data_flow::VarAccessKind::Use => "use" }),
        None => (String::new(), 0, String::new(), String::new(), "use"),
    };
    GraphNode {
        symbol: Some(SymbolRef::Variable { file: file.clone(), function, line, path, access: access.into(), ordinal: 0 }),
        location: Location { file, start_line: line, end_line: line },
    }
}
```
- [ ] **Step 4: Run — PASS.** `cargo test --lib reasoning::shape`. (If `BoundaryExited` fails because the callee param Def shares a line with another node, adjust the fixture so `g`'s body is unambiguous.)
- [ ] **Step 5: Relation-named edge test + full proof + commit.**
```rust
#[test]
fn test_witness_graph_relation_named_edges() {
    let cpg = build_python_cpg("def f():\n    a = input()\n    b = a\n    sink(b)\n");
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let g = witness_graph(&cpg, &trace, "test.py", 4);
    assert!(g.edges.iter().all(|e| e.kind == "DataFlow" || e.kind == "AssignmentPropagation"), "{:?}", g.edges);
}
```
Run the **global proof set**.
```bash
git add src/reasoning/shape.rs
git commit -m "feat(reasoning): shape.rs tri-state + relation-named witness graph (A7)"
```

---

## Task 6: A7.4 — `seeds.rs` shared type definitions

**Files:** Modify `src/reasoning/seeds.rs`; Test: inline.

- [ ] **Step 1: Failing construction test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_seed_types_construct() {
        let spec = SeedSpec::Loc { file: "a.py".into(), line: 3 };
        assert!(matches!(spec, SeedSpec::Loc { line: 3, .. }));
        assert!(SeedSet::default().seeds.is_empty());
    }
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement the type defs (resolution is Plan B):**
```rust
use crate::data_flow::VarLocation;
use crate::navigation::types::{SymbolRef, Warning};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSpec { Loc { file: String, line: usize }, Symbol { name: String, file: Option<String> } }
#[derive(Debug, Clone)]
pub struct ResolvedSeed { pub locations: Vec<VarLocation>, pub symbol: Option<SymbolRef>, pub origin: SeedSpec }
#[derive(Debug, Clone, Default)]
pub struct SeedSet { pub seeds: Vec<ResolvedSeed>, pub warnings: Vec<Warning> }
```
- [ ] **Step 4: Run + full proof + commit.**
```bash
git add src/reasoning/seeds.rs
git commit -m "feat(reasoning): seeds.rs shared SeedSpec/SeedSet type defs (A7)"
```

---

## Task 7: Integration smoke target + CLAUDE.md + final proof

**Files:** Create `tests/reasoning/smoke_test.rs`; Modify `Cargo.toml`, `CLAUDE.md`.

- [ ] **Step 1: Write the concrete smoke test (B7 — no placeholder body; m5 path).**
`tests/reasoning/smoke_test.rs`:
```rust
use prism::cpg::CodePropertyGraph;
use prism::navigation::types::Reachability;
use prism::reasoning::shape::reachability_at;

fn build_py(src: &str) -> CodePropertyGraph {
    let parsed = prism::ast::ParsedFile::parse("test.py", src, prism::languages::Language::Python).unwrap();
    let mut files = std::collections::BTreeMap::new();
    files.insert("test.py".to_string(), parsed);
    CodePropertyGraph::build(&files)
}

#[test]
fn taint_trace_reachability_end_to_end() {
    let cpg = build_py("def g(p):\n    sink(p)\n\ndef f():\n    u = input()\n    v = u\n    g(u)\n");
    let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]);
    assert_eq!(reachability_at(&cpg, &trace, "test.py", 6), Reachability::Reached);
    assert_eq!(reachability_at(&cpg, &trace, "test.py", 2), Reachability::BoundaryExited);
}
```
(Confirm `prism::ast`/`prism::languages` are the public paths; mirror an existing `tests/` integration import if they differ.)

- [ ] **Step 2: Register the target in `Cargo.toml`** (mirror `cli_nav_compat`):
```toml
[[test]]
name = "reasoning_smoke"
path = "tests/reasoning/smoke_test.rs"
```

- [ ] **Step 3: Update `CLAUDE.md` (m6).** Add `src/reasoning/` to the Core Modules list (one line: "`reasoning/` — Tier-2 reasoning layer: `taint_trace` consumer, `SeedSet`, output shaper") and note the additive `Evidence.reasoning` field.

- [ ] **Step 4: Run the full proof + the new target + commit.**
Run the **global proof set** PLUS `cargo test --test reasoning_smoke`.
Expected: all green; `cli_nav_compat` byte-unchanged (Option-C holds — every change was additive).
```bash
git add Cargo.toml tests/reasoning/ CLAUDE.md
git commit -m "test(reasoning): integration smoke target + docs (A7)"
```

---

## Self-Review

**Plan-review fixes folded:** B1 (Task 0 helper) · B2 (Task 5 re-export imports) · B3 (`Location { start_line, end_line }`) · B4 (Task 11 + `reachability_at` boundary-by-function) · B5 (Task 4 all constructors + `navigation_types`/`--features mcp` proof) · B6 (global proof set, every task) · B7 (Task 7 concrete body) · M1 (Task 1 Step 7 same-line + `cfg_valid` admits `tline==src_line`) · M2 (Steps 9–10 no-path/no-CFG/tie) · M3 (Step 8 root-set) · M4 (Task 2 `(files, &VarLocation)` pinned + documented) · M5 (sanitizer-taxonomy proof) · M6 (Task 2 Step 6 obligation) · m1 (no render arms) · m2 (`access` from kind) · m3 (`SanitizerCategory` derive corrected in API table) · m4 (no `Verbosity` enum) · m5 (path) · m6 (CLAUDE.md).

**Spec coverage:** A3 (Task 1) · A4 (Task 2) · A7 (Tasks 3–6) · integration + docs (Task 7). Deferred correctly: A2/A5/A6 (Phase-IP/hygiene), `resolve_seed_set`/`taint_reaches`/MCP tool (Plan B).

**Execution note for the implementer:** a few fixtures carry "mirror an existing test if the CPG doesn't build the expected edge" fallbacks (Steps 9, 11; Task 7 paths). These are real conditional instructions, not placeholders — follow the named existing tests when the inline fixture is thin.
