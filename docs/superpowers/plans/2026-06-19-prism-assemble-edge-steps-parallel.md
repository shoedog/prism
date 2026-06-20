# Assemble Edge-Steps Parallelization (5 + 5b + 8) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parallelize the three edge-creation steps of `CodePropertyGraph::assemble_graph` (Call/Return = step5, interproc DataFlow = step5b, ControlFlow = step8) with **byte-identical** output, via the proven C2 pattern (ordered units → `par_iter` collect `Vec<PendingEdge>` → serial `add_edge`).

**Architecture:** Each step's inline loop is split into a pure `collect_stepN_edges(...) -> Vec<(NodeIndex, NodeIndex, CpgEdge)>` (internally `par_iter` over ordered units, order-preserving `collect` + flatten) and a trivial serial apply (`for (f,t,w) in edges { graph.add_edge(f,t,w) }`). A `#[cfg(test)]` reference twin per step (the verbatim original loop emitting to a `Vec`) is the **old-order oracle**: production-parallel collect must equal it, fed by a real build's indexes. Determinism + cache-byte parity is the existing `parallel_equality_test`; Tier-A is the behavioral backstop.

**Tech Stack:** Rust, `rayon` (shared global pool, same as Step 7), `petgraph`, the existing `src/cpg/tests.rs` + `tests/infra/parallel_equality_test.rs` harness.

**Design-of-record:** `docs/superpowers/specs/2026-06-19-prism-assemble-edge-steps-parallel-design.md` (rev 2, PLAN-READY).

**Order (risk-ascending):** Step 8 (no resolve/memo) → Step 5 (resolve, no memo) → Step 5b (resolve + prewarm + local memo). Each: inert extract + oracle commit, then parallelize commit.

**Verification scope (macOS):** full `cargo test` stalls at `_dyld_start` — use `cargo test --lib <filter>` (src/cpg/tests.rs are lib tests), `cargo test --test infra <filter>` (parallel_equality), `cargo build -p prism`, `cargo build --release -p prism`, `cargo fmt`, `cargo clippy -p prism --lib`. `cd eval && uv run tier-a --matrix-only --allow-stale-sut` is Python/uv (no stall).

---

## File Structure

- `src/cpg/build.rs` — Modify: extract `collect_step8_edges`, `collect_step5_edges`, `collect_step5b_edges` (production, parallel); replace the three inline loops (`:575`, `:448`, `:480`) with collect-then-serial-apply; add the Step-5b `call_args` prewarm; add three `#[cfg(test)]` reference twins.
- `src/cpg/tests.rs` — Modify: one shared `edge_fixture()` + three old-order collect-parity oracles.
- `tests/infra/parallel_equality_test.rs` — Modify: edge non-vacuity guard.

Type used throughout: `type PendingEdge = (petgraph::graph::NodeIndex, petgraph::graph::NodeIndex, CpgEdge);` (define once near the top of the `impl` region in `build.rs`).

---

## Task 1: Step 8 — inert extraction + shared fixture + old-order oracle

**Files:**
- Modify: `src/cpg/build.rs` (Step 8 loop at `:575`; add `collect_step8_edges` + `#[cfg(test)] collect_step8_edges_reference` + `PendingEdge` alias)
- Modify: `src/cpg/tests.rs` (add `edge_fixture()` + `step8_parallel_edge_collect_matches_serial_reference`)

- [ ] **Step 1: Write the failing oracle test** in `src/cpg/tests.rs` (after the Step-7 oracle, ~`:125`):

```rust
/// Shared discriminating fixture for the edge-step old-order oracles:
/// multi-file, cross-file calls, a site that resolves to multiple same-name
/// callees, an unresolved call, and branchy bodies (→ CFG edges).
fn edge_fixture() -> std::collections::BTreeMap<String, ParsedFile> {
    let src: &[(&str, &str, Language)] = &[
        // callee.rs: two same-name `helper` fns (multi-callee resolution) + a leaf.
        (
            "callee.rs",
            "pub fn helper(a: i32) -> i32 { if a > 0 { a } else { -a } }\n\
             pub fn helper(a: i64) -> i64 { a + 1 }\n\
             pub fn leaf() -> i32 { 0 }\n",
            Language::Rust,
        ),
        // caller.rs: calls helper (multi), leaf, and an unresolved free fn.
        (
            "caller.rs",
            "use crate::callee::{helper, leaf};\n\
             pub fn run(x: i32) -> i32 {\n\
                 let y = helper(x);\n\
                 let z = leaf();\n\
                 if y > z { unresolved_fn(y) } else { z }\n\
             }\n",
            Language::Rust,
        ),
    ];
    let mut files = std::collections::BTreeMap::new();
    for (p, s, lang) in src {
        files.insert(p.to_string(), ParsedFile::parse(p, s, *lang).unwrap());
    }
    files
}

#[test]
fn step8_parallel_edge_collect_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    use super::types::{CpgEdge, CpgNode};
    use petgraph::graph::{DiGraph, NodeIndex};
    use std::collections::BTreeMap;

    let files = edge_fixture();
    // Step 8 reads stmt_index (built by Step 7). Regenerate it deterministically;
    // par vs serial collect share the same stmt_index → directly comparable.
    let mut g: DiGraph<CpgNode, CpgEdge> = DiGraph::new();
    let mut li: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();
    let stmt_index = CodePropertyGraph::assemble_step7(&files, &mut g, &mut li);

    let par = CodePropertyGraph::collect_step8_edges(&stmt_index, &files);
    let serial = CodePropertyGraph::collect_step8_edges_reference(&stmt_index, &files);
    assert_eq!(par, serial, "Step-8 ControlFlow edge sequence diverged");
    assert!(!par.is_empty(), "fixture produced no CFG edges");
}
```

- [ ] **Step 2: Run it to verify it fails to compile** (functions don't exist yet):

Run: `cargo test --lib step8_parallel_edge_collect -- --nocapture`
Expected: FAIL — `no function or associated item named `collect_step8_edges``.

- [ ] **Step 3: Add the `PendingEdge` alias + extract Step 8 (inert serial) + reference twin** in `src/cpg/build.rs`.

Near the top of `impl CodePropertyGraph` (or just above `assemble_graph`), add:

```rust
/// An edge pending insertion: (from, to, weight). Collected in deterministic
/// unit order, then applied by a serial `add_edge` loop (S1 C2 pattern).
pub(crate) type PendingEdge = (NodeIndex, NodeIndex, CpgEdge);
```

Add the production collect (initially **serial** — the inert extraction) + reference twin:

```rust
/// Step 8: statement→statement ControlFlow edges. Collect-then-apply.
/// (Inert in this task — serial `iter`; parallelized in Task 2.)
pub(crate) fn collect_step8_edges(
    stmt_index: &BTreeMap<(String, usize), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    let ordered: Vec<(&String, &ParsedFile)> = files.iter().collect();
    ordered
        .iter()
        .map(|(_path, parsed)| {
            let mut out: Vec<PendingEdge> = Vec::new();
            for edge in cfg::build_cfg_edges(parsed) {
                let from_idx = stmt_index.get(&(edge.file.clone(), edge.from_line));
                let to_idx = stmt_index.get(&(edge.file.clone(), edge.to_line));
                if let (Some(&from), Some(&to)) = (from_idx, to_idx) {
                    out.push((from, to, CpgEdge::ControlFlow));
                }
            }
            out
        })
        .collect::<Vec<Vec<PendingEdge>>>()
        .into_iter()
        .flatten()
        .collect()
}

#[cfg(test)]
pub(crate) fn collect_step8_edges_reference(
    stmt_index: &BTreeMap<(String, usize), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    let mut out: Vec<PendingEdge> = Vec::new();
    for (_path, parsed) in files {
        let cfg_edges = cfg::build_cfg_edges(parsed);
        for edge in cfg_edges {
            let from_idx = stmt_index.get(&(edge.file.clone(), edge.from_line));
            let to_idx = stmt_index.get(&(edge.file.clone(), edge.to_line));
            if let (Some(&from), Some(&to)) = (from_idx, to_idx) {
                out.push((from, to, CpgEdge::ControlFlow));
            }
        }
    }
    out
}
```

Replace the inline Step 8 loop (`:575`–`:585`) with the apply:

```rust
        // --- Step 8: ControlFlow edges ---
        for (from, to, w) in Self::collect_step8_edges(&stmt_index, files) {
            graph.add_edge(from, to, w);
        }
```

- [ ] **Step 4: Run the oracle + build to verify GREEN:**

Run: `cargo test --lib step8_parallel_edge_collect && cargo build -p prism`
Expected: PASS (the extraction is behavior-preserving: serial collect == reference). Build clean.

- [ ] **Step 5: fmt + clippy:**

Run: `cargo fmt && cargo clippy -p prism --lib 2>&1 | rg -i "warning: unused|warning.*collect_step8" || echo CLEAN`
Expected: no new warnings about the added code.

- [ ] **Step 6: Commit:**

```bash
git add src/cpg/build.rs src/cpg/tests.rs
git commit -m "refactor(step8): extract collect_step8_edges + old-order oracle (inert serial)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Step 8 — parallelize

**Files:**
- Modify: `src/cpg/build.rs` (`collect_step8_edges`: `iter` → `par_iter`)

- [ ] **Step 1: Flip the collect to `par_iter`.** In `collect_step8_edges`, add `use rayon::prelude::*;` at the top of the fn and change `ordered.iter()` to `ordered.par_iter()`. The `.collect::<Vec<Vec<PendingEdge>>>()` stays (rayon indexed collect is order-preserving). No other change.

```rust
pub(crate) fn collect_step8_edges(
    stmt_index: &BTreeMap<(String, usize), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    use rayon::prelude::*;
    let ordered: Vec<(&String, &ParsedFile)> = files.iter().collect();
    ordered
        .par_iter()
        .map(|(_path, parsed)| {
            // ... unchanged body ...
        })
        .collect::<Vec<Vec<PendingEdge>>>()
        .into_iter()
        .flatten()
        .collect()
}
```

- [ ] **Step 2: Run the old-order oracle (now discriminating: par vs serial-reference):**

Run: `cargo test --lib step8_parallel_edge_collect`
Expected: PASS — parallel collect reproduces the exact reference edge order.

- [ ] **Step 3: Run the determinism + cache-byte gate:**

Run: `cargo test --test infra cpg_build_parallel_matches_serial_reference_in_order && cargo test --test infra cache_blob_bytes_identical_serial_vs_parallel`
Expected: PASS (default-Rayon vs 1-thread node+edge dumps identical; cache blob bytes identical).

- [ ] **Step 4: fmt + commit:**

```bash
cargo fmt
git add src/cpg/build.rs
git commit -m "perf(step8): parallelize ControlFlow edge collection (par_iter over files)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Step 5 — inert extraction + old-order oracle

**Files:**
- Modify: `src/cpg/build.rs` (Step 5 loop at `:448`; add `collect_step5_edges` + reference twin)
- Modify: `src/cpg/tests.rs` (add `step5_parallel_edge_collect_matches_serial_reference`)

- [ ] **Step 1: Write the failing oracle test** in `src/cpg/tests.rs`:

```rust
#[test]
fn step5_parallel_edge_collect_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    let files = edge_fixture();
    let cpg = CodePropertyGraph::build(&files); // sources cg + func_index
    let par = CodePropertyGraph::collect_step5_edges(&cpg.call_graph, &cpg.func_index);
    let serial =
        CodePropertyGraph::collect_step5_edges_reference(&cpg.call_graph, &cpg.func_index);
    assert_eq!(par, serial, "Step-5 Call/Return edge sequence diverged");
    assert!(!par.is_empty(), "fixture produced no call edges");
}
```

- [ ] **Step 2: Run to verify it fails to compile:**

Run: `cargo test --lib step5_parallel_edge_collect`
Expected: FAIL — `no function ... collect_step5_edges`.

- [ ] **Step 3: Extract Step 5 (inert serial) + reference twin** in `src/cpg/build.rs`.

```rust
/// Step 5: Function→Function Call + Return edges. Collect-then-apply.
/// (Inert here — serial; parallelized in Task 4.)
pub(crate) fn collect_step5_edges(
    cg: &CallGraph,
    func_index: &BTreeMap<(String, String, usize), NodeIndex>,
) -> Vec<PendingEdge> {
    let ordered: Vec<_> = cg.calls.iter().collect();
    ordered
        .iter()
        .map(|(caller_id, sites)| Self::step5_edges_for_caller(caller_id, sites, cg, func_index))
        .collect::<Vec<Vec<PendingEdge>>>()
        .into_iter()
        .flatten()
        .collect()
}

/// The per-caller Step-5 emission — verbatim semantics of the original inline
/// loop (caller-skip on `func_index` miss; Call then Return per resolved callee).
fn step5_edges_for_caller(
    caller_id: &FunctionId,
    sites: &std::collections::BTreeSet<CallSite>,
    cg: &CallGraph,
    func_index: &BTreeMap<(String, String, usize), NodeIndex>,
) -> Vec<PendingEdge> {
    let mut out: Vec<PendingEdge> = Vec::new();
    let caller_key = (
        caller_id.file.clone(),
        caller_id.name.clone(),
        caller_id.start_line,
    );
    let caller_idx = match func_index.get(&caller_key) {
        Some(&idx) => idx,
        None => return out,
    };
    for site in sites {
        for resolved in cg.resolve_call_site(site) {
            let callee_id = resolved.target;
            let callee_key = (
                callee_id.file.clone(),
                callee_id.name.clone(),
                callee_id.start_line,
            );
            if let Some(&callee_idx) = func_index.get(&callee_key) {
                out.push((caller_idx, callee_idx, CpgEdge::Call(resolved.confidence)));
                out.push((callee_idx, caller_idx, CpgEdge::Return(resolved.confidence)));
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn collect_step5_edges_reference(
    cg: &CallGraph,
    func_index: &BTreeMap<(String, String, usize), NodeIndex>,
) -> Vec<PendingEdge> {
    let mut out: Vec<PendingEdge> = Vec::new();
    for (caller_id, sites) in &cg.calls {
        let caller_key = (
            caller_id.file.clone(),
            caller_id.name.clone(),
            caller_id.start_line,
        );
        let caller_idx = match func_index.get(&caller_key) {
            Some(&idx) => idx,
            None => continue,
        };
        for site in sites {
            for resolved in cg.resolve_call_site(site) {
                let callee_id = resolved.target;
                let callee_key = (
                    callee_id.file.clone(),
                    callee_id.name.clone(),
                    callee_id.start_line,
                );
                if let Some(&callee_idx) = func_index.get(&callee_key) {
                    out.push((caller_idx, callee_idx, CpgEdge::Call(resolved.confidence)));
                    out.push((callee_idx, caller_idx, CpgEdge::Return(resolved.confidence)));
                }
            }
        }
    }
    out
}
```

Note: `step5_edges_for_caller` is shared by the production collect only; the reference twin is a standalone verbatim copy (so the oracle compares two independent implementations). **Required import change** (the helper signatures name `CallSite` explicitly, which `build.rs:6` does not currently import): change `use crate::call_graph::{CallGraph, FunctionId, ScopeGraphBuildInputs};` to `use crate::call_graph::{CallGraph, CallSite, FunctionId, ScopeGraphBuildInputs};` (`CallSite` is `src/call_graph.rs:47`).

Replace the inline Step 5 loop (`:448`–`:478`) with the apply:

```rust
        // --- Step 5: Call edges ---
        for (from, to, w) in Self::collect_step5_edges(&cg, &func_index) {
            graph.add_edge(from, to, w);
        }
```

- [ ] **Step 4: Build + oracle GREEN:**

Run: `cargo test --lib step5_parallel_edge_collect && cargo build -p prism`
Expected: PASS (serial collect == reference). Build clean.

- [ ] **Step 5: fmt + commit:**

```bash
cargo fmt
git add src/cpg/build.rs src/cpg/tests.rs
git commit -m "refactor(step5): extract collect_step5_edges + old-order oracle (inert serial)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Step 5 — parallelize

**Files:**
- Modify: `src/cpg/build.rs` (`collect_step5_edges`: `iter` → `par_iter`)

- [ ] **Step 1: Flip to `par_iter`.** In `collect_step5_edges` add `use rayon::prelude::*;` and change `ordered.iter()` → `ordered.par_iter()`. The per-caller closure already delegates to `step5_edges_for_caller` (which only reads `cg` via `resolve_call_site` (`&self`, audited zero-interior-mutability) and the immutable `func_index`), so no body change.

- [ ] **Step 2: Old-order oracle (now par vs serial-reference):**

Run: `cargo test --lib step5_parallel_edge_collect`
Expected: PASS.

- [ ] **Step 3: Determinism + cache-byte gate:**

Run: `cargo test --test infra cpg_build_parallel_matches_serial_reference_in_order && cargo test --test infra cache_blob_bytes_identical_serial_vs_parallel`
Expected: PASS.

- [ ] **Step 4: fmt + commit:**

```bash
cargo fmt
git add src/cpg/build.rs
git commit -m "perf(step5): parallelize Call/Return edge collection (par_iter over callers)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Step 5b — inert extraction + prewarm + old-order oracle

**Files:**
- Modify: `src/cpg/build.rs` (Step 5b loop at `:480`; add `collect_step5b_edges` + reference twin + prewarm)
- Modify: `src/cpg/tests.rs` (add `step5b_parallel_edge_collect_matches_serial_reference`)

- [ ] **Step 1: Write the failing oracle test** in `src/cpg/tests.rs`:

```rust
#[test]
fn step5b_parallel_edge_collect_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    let files = edge_fixture();
    let cpg = CodePropertyGraph::build(&files); // sources cg + var_index (and warms call_args)
    let par = CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &cpg.var_index, &files);
    let serial = CodePropertyGraph::collect_step5b_edges_reference(
        &cpg.call_graph,
        &cpg.var_index,
        &files,
    );
    assert_eq!(par, serial, "Step-5b DataFlow edge sequence diverged");
    // The fixture's `let y = helper(x);` yields an arg→param DataFlow edge.
    assert!(!par.is_empty(), "fixture produced no interproc DataFlow edges");
}
```

- [ ] **Step 2: Run to verify it fails to compile:**

Run: `cargo test --lib step5b_parallel_edge_collect`
Expected: FAIL — `no function ... collect_step5b_edges`.

- [ ] **Step 3: Extract Step 5b (inert serial) + prewarm + reference twin** in `src/cpg/build.rs`.

The production collect: prewarm the `call_args` OnceLock (so the parallel phase in Task 6 is a literal read), then collect per caller with a **per-caller-local** param memo. Initially serial (`iter`).

```rust
/// Step 5b: interprocedural arg→param DataFlow edges. Collect-then-apply.
/// (Inert here — serial; parallelized in Task 6.)
/// Prewarms each file's `call_args` OnceLock so the (eventual) parallel phase is
/// a literal read of an initialized, deterministic index.
pub(crate) fn collect_step5b_edges(
    cg: &CallGraph,
    var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    // Prewarm (serial here; par_iter in Task 6). Idempotent; each file's OnceLock
    // is independent. compute the index now so the collect never inits under load.
    for parsed in files.values() {
        let _ = parsed.call_args_index();
    }
    let ordered: Vec<_> = cg.calls.iter().collect();
    ordered
        .iter()
        .map(|(caller_id, sites)| Self::step5b_edges_for_caller(caller_id, sites, cg, var_index, files))
        .collect::<Vec<Vec<PendingEdge>>>()
        .into_iter()
        .flatten()
        .collect()
}

/// Per-caller Step-5b emission — verbatim semantics of the original inline loop,
/// with a caller-LOCAL param memo (compute_param_names is pure → identical edges
/// to the #112 global memo; only caching scope changes).
fn step5b_edges_for_caller(
    caller_id: &FunctionId,
    sites: &std::collections::BTreeSet<CallSite>,
    cg: &CallGraph,
    var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    let mut out: Vec<PendingEdge> = Vec::new();
    let mut param_cache: BTreeMap<(String, String, usize), Option<Vec<String>>> = BTreeMap::new();
    for site in sites {
        for resolved in cg.resolve_call_site(site) {
            let callee_id = resolved.target;
            let caller_parsed = match files.get(&caller_id.file) {
                Some(p) => p,
                None => continue,
            };
            let arg_texts =
                caller_parsed.call_argument_texts_at(site.start_byte, &site.callee_name);
            if arg_texts.is_empty() {
                continue;
            }
            let callee_parsed = match files.get(&callee_id.file) {
                Some(p) => p,
                None => continue,
            };
            let cache_key = (
                callee_id.file.clone(),
                callee_id.name.clone(),
                callee_id.start_line,
            );
            let param_names: &[String] = match param_cache
                .entry(cache_key)
                .or_insert_with(|| compute_param_names(callee_parsed, callee_id))
            {
                Some(names) => names.as_slice(),
                None => continue,
            };
            for (i, param_name) in param_names.iter().enumerate() {
                if i >= arg_texts.len() {
                    break;
                }
                let arg_text = &arg_texts[i];
                let arg_base = arg_text.split('.').next().unwrap_or(arg_text);
                let arg_base = arg_base.split("->").next().unwrap_or(arg_base);
                let arg_path = AccessPath::simple(arg_base);
                let arg_key = (
                    caller_id.file.clone(),
                    caller_id.name.clone(),
                    caller_id.start_line,
                    site.line,
                    arg_path.clone(),
                    VarAccess::Use,
                );
                let arg_idx = var_index.get(&arg_key).copied().or_else(|| {
                    let def_key = (
                        caller_id.file.clone(),
                        caller_id.name.clone(),
                        caller_id.start_line,
                        site.line,
                        arg_path,
                        VarAccess::Def,
                    );
                    var_index.get(&def_key).copied()
                });
                let param_path = AccessPath::simple(param_name);
                let param_idx = (callee_id.start_line..=callee_id.end_line).find_map(|line| {
                    let key = (
                        callee_id.file.clone(),
                        callee_id.name.clone(),
                        callee_id.start_line,
                        line,
                        param_path.clone(),
                        VarAccess::Def,
                    );
                    var_index.get(&key).copied()
                });
                if let (Some(from), Some(to)) = (arg_idx, param_idx) {
                    out.push((from, to, CpgEdge::DataFlow));
                }
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn collect_step5b_edges_reference(
    cg: &CallGraph,
    var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    // Verbatim original Step-5b: global lazy param_cache, no prewarm. Same edges
    // as production (param names are pure); the differences (prewarm, local memo)
    // are perf-only and invisible to the emitted Vec.
    let mut out: Vec<PendingEdge> = Vec::new();
    let mut param_cache: BTreeMap<(String, String, usize), Option<Vec<String>>> = BTreeMap::new();
    for (caller_id, sites) in &cg.calls {
        for site in sites {
            for resolved in cg.resolve_call_site(site) {
                let callee_id = resolved.target;
                let caller_parsed = match files.get(&caller_id.file) {
                    Some(p) => p,
                    None => continue,
                };
                let arg_texts =
                    caller_parsed.call_argument_texts_at(site.start_byte, &site.callee_name);
                if arg_texts.is_empty() {
                    continue;
                }
                let callee_parsed = match files.get(&callee_id.file) {
                    Some(p) => p,
                    None => continue,
                };
                let cache_key = (
                    callee_id.file.clone(),
                    callee_id.name.clone(),
                    callee_id.start_line,
                );
                let param_names: &[String] = match param_cache
                    .entry(cache_key)
                    .or_insert_with(|| compute_param_names(callee_parsed, callee_id))
                {
                    Some(names) => names.as_slice(),
                    None => continue,
                };
                for (i, param_name) in param_names.iter().enumerate() {
                    if i >= arg_texts.len() {
                        break;
                    }
                    let arg_text = &arg_texts[i];
                    let arg_base = arg_text.split('.').next().unwrap_or(arg_text);
                    let arg_base = arg_base.split("->").next().unwrap_or(arg_base);
                    let arg_path = AccessPath::simple(arg_base);
                    let arg_key = (
                        caller_id.file.clone(),
                        caller_id.name.clone(),
                        caller_id.start_line,
                        site.line,
                        arg_path.clone(),
                        VarAccess::Use,
                    );
                    let arg_idx = var_index.get(&arg_key).copied().or_else(|| {
                        let def_key = (
                            caller_id.file.clone(),
                            caller_id.name.clone(),
                            caller_id.start_line,
                            site.line,
                            arg_path,
                            VarAccess::Def,
                        );
                        var_index.get(&def_key).copied()
                    });
                    let param_path = AccessPath::simple(param_name);
                    let param_idx = (callee_id.start_line..=callee_id.end_line).find_map(|line| {
                        let key = (
                            callee_id.file.clone(),
                            callee_id.name.clone(),
                            callee_id.start_line,
                            line,
                            param_path.clone(),
                            VarAccess::Def,
                        );
                        var_index.get(&key).copied()
                    });
                    if let (Some(from), Some(to)) = (arg_idx, param_idx) {
                        out.push((from, to, CpgEdge::DataFlow));
                    }
                }
            }
        }
    }
    out
}
```

Replace the inline Step 5b loop (`:480`–`:560`, the whole `let mut param_cache ...` block through its closing) with the apply:

```rust
        // --- Step 5b: Interprocedural data flow edges ---
        for (from, to, w) in Self::collect_step5b_edges(&cg, &var_index, files) {
            graph.add_edge(from, to, w);
        }
```

- [ ] **Step 4: Build + oracle GREEN:**

Run: `cargo test --lib step5b_parallel_edge_collect && cargo build -p prism`
Expected: PASS (serial collect with local memo == reference with global memo — identical edges). Build clean. Also run `cargo test --lib step5b_param_binding_first_wins_parity` (the existing #112 parity test must stay green).

- [ ] **Step 5: fmt + commit:**

```bash
cargo fmt
git add src/cpg/build.rs src/cpg/tests.rs
git commit -m "refactor(step5b): extract collect_step5b_edges + prewarm + old-order oracle (inert serial)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Step 5b — parallelize

**Files:**
- Modify: `src/cpg/build.rs` (`collect_step5b_edges`: prewarm + collect → `par_iter`)

- [ ] **Step 1: Parallelize the prewarm + the collect.** In `collect_step5b_edges` add `use rayon::prelude::*;`, change the prewarm loop to `files.par_iter().for_each(|(_, p)| { let _ = p.call_args_index(); });`, and change `ordered.iter()` → `ordered.par_iter()`. `step5b_edges_for_caller` is unchanged (its per-caller-local `param_cache` is owned, no shared mutability; `resolve_call_site`/`call_argument_texts_at`/`compute_param_names`/`var_index` reads are all on now-immutable/initialized state).

```rust
pub(crate) fn collect_step5b_edges(
    cg: &CallGraph,
    var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingEdge> {
    use rayon::prelude::*;
    files.par_iter().for_each(|(_, p)| {
        let _ = p.call_args_index();
    });
    let ordered: Vec<_> = cg.calls.iter().collect();
    ordered
        .par_iter()
        .map(|(caller_id, sites)| Self::step5b_edges_for_caller(caller_id, sites, cg, var_index, files))
        .collect::<Vec<Vec<PendingEdge>>>()
        .into_iter()
        .flatten()
        .collect()
}
```

- [ ] **Step 2: Old-order oracle (par vs serial-reference) + #112 parity:**

Run: `cargo test --lib step5b_parallel_edge_collect && cargo test --lib step5b_param_binding_first_wins_parity`
Expected: PASS both.

- [ ] **Step 3: Determinism + cache-byte gate:**

Run: `cargo test --test infra cpg_build_parallel_matches_serial_reference_in_order && cargo test --test infra cache_blob_bytes_identical_serial_vs_parallel`
Expected: PASS.

- [ ] **Step 4: fmt + commit:**

```bash
cargo fmt
git add src/cpg/build.rs
git commit -m "perf(step5b): parallelize interproc DataFlow edge collection (prewarm + par_iter)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Non-vacuity guard + full gate (Tier-A + perf)

**Files:**
- Modify: `tests/infra/parallel_equality_test.rs` (edge non-vacuity guard)

- [ ] **Step 1: Add an edge non-vacuity guard** so a silent corpus shrink can't hollow out the determinism/cache-byte gate (mirrors the Step-7 statement floor):

Use the established public `edge_dump()` accessor (each entry is
`"{src}->{tgt}:{weight:?}"`, so weights render as `Call(Exact)` / `DataFlow` /
`ControlFlow` after the `:`) — the same accessor `cpg_build_parallel_matches_serial_reference_in_order` already uses, so no new public surface is needed:

```rust
/// Edge non-vacuity guard: the determinism + cache-byte tests only surface a
/// parallel edge-step divergence if the corpus actually has Call, DataFlow, and
/// ControlFlow edges. Floor against a silent corpus shrink.
#[test]
fn edge_steps_corpus_has_call_dataflow_and_controlflow_edges() {
    let repo = corpus(); // src/navigation — cross-file calls + branchy bodies
    let cpg = CpgContext::build(&repo.files, None);
    let dump = cpg.cpg.edge_dump();
    let call = dump.iter().filter(|s| s.contains("Call(")).count();
    let dataflow = dump.iter().filter(|s| s.contains("DataFlow")).count();
    let controlflow = dump.iter().filter(|s| s.contains("ControlFlow")).count();
    assert!(call > 50, "too few Call edges to surface a Step-5 divergence: {call}");
    assert!(dataflow > 50, "too few DataFlow edges to surface Step-5b: {dataflow}");
    assert!(controlflow > 50, "too few ControlFlow edges to surface Step-8: {controlflow}");
}
```

(Counts are weight substrings; node ordinals are numeric so `Call(`/`DataFlow`/`ControlFlow` only match weights. If `src/navigation` happens to floor-fail any category, raise the corpus to `corpus()` + `corpus2()` merged rather than lowering the floor.)

- [ ] **Step 2: Run the guard + the full infra suite:**

Run: `cargo test --test infra`
Expected: PASS (all parallel_equality tests incl. the new guard; counts comfortably above floors).

- [ ] **Step 3: Full lib test sweep (the oracles + CPG suite):**

Run: `cargo test --lib cpg:: && cargo test --lib step5 && cargo test --lib step8`
Expected: PASS — all three oracles + existing CPG tests green.

- [ ] **Step 4: fmt check + clippy:**

Run: `cargo fmt --check && cargo clippy -p prism --lib 2>&1 | rg "warning" | rg -v "10722|sanitizer_category" || echo CLEAN`
Expected: formatting clean; no new clippy warnings (the 2 pre-existing `frameworks` warnings are not ours).

- [ ] **Step 5: Tier-A backstop (0 regressions required):**

Run: `cargo build --release -p prism && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)`
Expected: 0 regressions vs the committed baseline. Paste any flip-candidates into the PR description (do not re-baseline).

- [ ] **Step 6: Perf gate (the reward — report, don't assert):**

```bash
BIN=target/release/prism
for repo in ~/code/bench-repos/hugo ~/code/bench-repos/tokio /Users/wesleyjinks/code/slicing; do
  echo "== $repo =="
  for i in 1 2; do /usr/bin/time -p $BIN nav --no-cache call-stats --repo "$repo" >/dev/null; done
done
```

Compare wall-clock vs `3cb0182` (current main + Step 7). Expected directional: the summed edge band (hugo ~1.06s / tokio ~2.05s / prism ~2.72s) drops materially; **no regression** on any corpus is the gate. Record the numbers for the PR body. (No `CACHE_VERSION` bump — bytes are identical.)

- [ ] **Step 7: Commit the guard:**

```bash
git add tests/infra/parallel_equality_test.rs
git commit -m "test(edge-steps): non-vacuity guard for Call/DataFlow/ControlFlow edge counts

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review notes (cross-checked against spec rev 2)

- **Spec coverage:** §2 scope (5+5b+8) → Tasks 1–6; §4 collect-then-apply + prewarm + own-skip-ladder → Tasks 1/3/5; §5.1 old-order oracle + non-vacuous fixture → the three oracles + `edge_fixture()`; §5.2 determinism/cache-byte → Tasks 2/4/6 step 3 + Task 7; §5.3 Tier-A + §6 perf → Task 7.
- **No placeholders:** every step has exact code/commands.
- **Type consistency:** `PendingEdge` alias defined once (Task 1) and used in all six fns + the apply sites; `collect_step{5,5b,8}_edges` signatures are stable across inert/parallel tasks (only `iter`→`par_iter` changes).
- **Risk isolation:** each step's extraction (faithfulness) and parallelization (concurrency) land in separate commits — clean bisect, mirrors the Step-7 cadence.
- **Edge-case fixture:** `edge_fixture()` carries an unresolved call (`unresolved_fn`), a multi-callee site (`helper` ×2), Call+Return pairs, an arg→param DataFlow edge (`helper(x)`), and CFG edges (the `if/else`). If the dup-`fn helper` does not parse/resolve as intended on Rust, fall back to two distinct same-name free fns across an additional file; verify each oracle's `assert!(!par.is_empty())` holds during Task execution and adjust the fixture (not the asserts) if a category is empty.
