# Prism Step 7 Parallelization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or executing-plans. Steps use checkbox (`- [ ]`). Execution = the codex implement(high)/review(xhigh) loop (gpt-5.5), TDD.

**Goal:** Parallelize `assemble_graph` Step 7 (statement-node creation) — the residual serial dominator (73–80% of assemble) — **proving a byte-identical graph** (node-insertion order = cache bytes).

**Architecture:** Extract Step 7 into a unit fn; freeze the original as a `#[cfg(test)]` reference + a same-binary parity oracle (the SAFETY NET) BEFORE touching it; then refactor to `ordered_files` → `par_iter` collect `PendingStatement` → serial node-create (the S1 C2 pattern). The oracle proves new == old order; `parallel_equality_test` proves thread-determinism; Tier-A backstops. No `CACHE_VERSION` bump.

**Tech Stack:** Rust, `rayon`, `petgraph`, `BTreeMap`/`BTreeSet`.

**Design of record:** `docs/superpowers/specs/2026-06-19-prism-step7-parallel-design.md` (rev 2, PLAN-READY).

**Verification-scope override (macOS host):** full `cargo test` / `--test cli` / `--test frameworks` stall at `_dyld_start`. Use `cargo test --lib`, `cargo test --test integration <filter>`, `cargo test --test infra <filter>`, `cargo fmt`, `cargo clippy -p prism --lib`, `cargo build --release`, and `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (Python/uv — does NOT stall; AGENTS.md pre-commit gate for `src/cpg/` changes). Orchestrator runs Tier-A + perf.

---

## File structure

- **`src/cpg/build.rs`** — extract `assemble_step7` (Task 1, serial); add `#[cfg(test)] assemble_step7_reference` (frozen); parallelize `assemble_step7` (Task 2) with `PendingStatement` + `collect_pending`. `collect_function_statements` stays (used by the reference).
- **`src/cpg/tests.rs`** — the same-binary parity oracle (Task 1).
- **`tests/infra/parallel_equality_test.rs`** — extend (Task 3: determinism + Step-7-heavy + min-count).
- No other files.

---

## Task 1: Extract Step 7 + the serial-reference parity oracle (SAFETY NET, green)

**Files:** Modify `src/cpg/build.rs` (Step 7 `:565`); Test `src/cpg/tests.rs`.

- [ ] **Step 1: Extract the current (serial) Step 7 into `pub(crate) fn assemble_step7` returning `stmt_index`**

Replace the inline Step-7 block (`:565–579`) in `assemble_graph` with a call:

```rust
        // --- Step 7: Statement nodes for CFG ---
        let stmt_index = Self::assemble_step7(files, &mut graph, &mut location_index);
```

Add the fn (verbatim current logic — no behavior change yet):

```rust
    pub(crate) fn assemble_step7(
        files: &BTreeMap<String, ParsedFile>,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> BTreeMap<(String, usize), NodeIndex> {
        let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
        for (path, parsed) in files {
            let root = parsed.tree.root_node();
            let func_types = parsed.language.function_node_types();
            Self::collect_function_statements(
                root, &func_types, parsed, path, graph, &mut stmt_index, location_index,
            );
        }
        stmt_index
    }
```

Step 8 (`:581`) already uses `stmt_index`; it now reads the returned binding (unchanged).

- [ ] **Step 2: Add the frozen `#[cfg(test)]` reference (a copy of the serial body)**

```rust
    #[cfg(test)]
    pub(crate) fn assemble_step7_reference(
        files: &BTreeMap<String, ParsedFile>,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> BTreeMap<(String, usize), NodeIndex> {
        let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
        for (path, parsed) in files {
            let root = parsed.tree.root_node();
            let func_types = parsed.language.function_node_types();
            Self::collect_function_statements(
                root, &func_types, parsed, path, graph, &mut stmt_index, location_index,
            );
        }
        stmt_index
    }
```

- [ ] **Step 3: Write the parity oracle (same-binary, git_sha-immune) — passes now (serial == serial)**

Add to `src/cpg/tests.rs`. It runs BOTH Step-7 impls on a fresh graph over a discriminating corpus (nested fns, closures, same-line) and asserts the `Statement`-node sequence (debug dumps — `PartialEq` ignores spans), `stmt_index` `(file,line)→NodeIndex` ordinals, and the (unsorted) `location_index` appends are identical.

```rust
#[test]
fn step7_parallel_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    use super::types::{CpgEdge, CpgNode};
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use petgraph::graph::{DiGraph, NodeIndex};
    use std::collections::BTreeMap;

    // Step-7-heavy + discriminating: nested fns / closures / multi-line stmts / multi-file.
    let src: &[(&str, &str, Language)] = &[
        ("a.rs", "fn outer(){ let x=1; fn inner(){ let y=2; } let z=3; }", Language::Rust),
        ("b.js", "function f(){ items.forEach((x)=>{ use(x); }); return 1; }", Language::JavaScript),
        ("c.py", "def f():\n    a = 1\n    def g():\n        b = 2\n    return a\n", Language::Python),
        ("d.go", "func h(){ for i:=0;i<3;i++ { use(i) }; return }", Language::Go),
    ];
    let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    for (p, s, lang) in src {
        files.insert(p.to_string(), ParsedFile::parse(p, s, *lang).unwrap());
    }

    type Step7Fn = fn(
        &BTreeMap<String, ParsedFile>,
        &mut DiGraph<CpgNode, CpgEdge>,
        &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> BTreeMap<(String, usize), NodeIndex>;
    let run = |f: Step7Fn| {
        let mut g: DiGraph<CpgNode, CpgEdge> = DiGraph::new();
        let mut li: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();
        let si = f(&files, &mut g, &mut li);
        let nodes: Vec<String> = g.node_indices().map(|i| format!("{:?}", g[i])).collect();
        (nodes, si, li)
    };

    let reference = run(CodePropertyGraph::assemble_step7_reference);
    let production = run(CodePropertyGraph::assemble_step7);
    assert_eq!(reference.0, production.0, "Statement node sequence diverged");
    assert_eq!(reference.1, production.1, "stmt_index (file,line)->NodeIndex diverged");
    assert_eq!(reference.2, production.2, "location_index appends diverged");
    assert!(!reference.0.is_empty(), "fixture produced no statement nodes");
}
```

Run: `cargo test --lib step7_parallel_matches_serial_reference`
Expected: PASS (production Step 7 is still the serial code == reference). **The safety net is in place.**

- [ ] **Step 4: fmt + clippy + commit**

```bash
cargo fmt && cargo clippy -p prism --lib && cargo test --lib step7
git add src/cpg/build.rs src/cpg/tests.rs
git commit -m "refactor(step7): extract assemble_step7 + serial-reference parity oracle (inert)"
```

---

## Task 2: Parallelize `assemble_step7` (the refactor — oracle guards it)

**Files:** Modify `src/cpg/build.rs`.

- [ ] **Step 1: Add `PendingStatement` + `collect_pending` (the recursion, mutations removed)**

```rust
struct PendingStatement {
    line: usize,
    kind: StmtKind,
    start_byte: usize,
    end_byte: usize,
}

impl CodePropertyGraph {
    /// Read-only per-file collect: the `collect_function_statements` recursion with
    /// node creation removed. Whole-file `seen` (by line), checked BEFORE classify —
    /// exactly the current `if stmt_index.contains_key(&(file,line)) { continue }`.
    fn collect_pending(
        node: tree_sitter::Node<'_>,
        func_types: &[&str],
        parsed: &ParsedFile,
        seen: &mut std::collections::BTreeSet<usize>,
        out: &mut Vec<PendingStatement>,
    ) {
        if func_types.contains(&node.kind()) {
            for span in parsed.statement_spans_in_function(&node) {
                if !seen.insert(span.line) {
                    continue; // duplicate (file,line) — skip BEFORE classify, as today
                }
                let kind = Self::classify_stmt_kind(&span.kind, parsed, span.line);
                out.push(PendingStatement {
                    line: span.line,
                    kind,
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                });
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_pending(child, func_types, parsed, seen, out);
        }
    }
}
```

- [ ] **Step 2: Re-point `assemble_step7` to parallel-collect → serial-create**

Replace the `assemble_step7` body (NOT the reference) with:

```rust
        use rayon::prelude::*;
        // 1. Ordered files (BTreeMap order — NOT scheduler order).
        let ordered: Vec<(&String, &ParsedFile)> = files.iter().collect();
        // 2. Parallel collect (read-only).
        let per_file: Vec<(&String, Vec<PendingStatement>)> = ordered
            .par_iter()
            .map(|(path, parsed)| {
                let func_types = parsed.language.function_node_types();
                let mut seen: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
                let mut out: Vec<PendingStatement> = Vec::new();
                Self::collect_pending(parsed.tree.root_node(), &func_types, parsed, &mut seen, &mut out);
                (*path, out)
            })
            .collect(); // rayon indexed collect is order-preserving => ordered-files order
        // 3. Serial create (the ONLY mutation, in files-order x walk-order).
        let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
        for (path, stmts) in &per_file {
            for s in stmts {
                let idx = graph.add_node(CpgNode::Statement {
                    file: (*path).clone(),
                    line: s.line,
                    kind: s.kind.clone(),
                    start_byte: s.start_byte,
                    end_byte: s.end_byte,
                });
                stmt_index.insert(((*path).clone(), s.line), idx);
                location_index.entry(((*path).clone(), s.line)).or_default().push(idx);
            }
        }
        stmt_index
```

- [ ] **Step 3: Run the oracle — it must STAY GREEN (parallel == frozen serial reference)**

Run: `cargo test --lib step7_parallel_matches_serial_reference`
Expected: PASS — the parallel build reproduces the exact serial node sequence + `stmt_index` + `location_index`. If it fails, the refactor broke the order contract (fix before proceeding).

- [ ] **Step 4: Behavior-preserving gate (incl. Tier-A, pre-commit per AGENTS.md)**

```bash
cargo test --lib
cargo test --test integration core_test::
cargo test --test infra            # parallel_equality_test (determinism + cache-byte)
cargo fmt && cargo fmt --check
cargo clippy -p prism --lib
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # 0 regressions — then cd back
```
Expected: all green; **Tier-A 0 regressions**; `parallel_equality_test` green (determinism + cache-byte parity).

- [ ] **Step 5: Commit**

```bash
git add src/cpg/build.rs
git commit -m "perf(step7): parallelize statement-node collection (ordered par_iter -> serial create)"
```

---

## Task 3: Strengthen the determinism gate

**Files:** Modify `tests/infra/parallel_equality_test.rs`.

- [ ] **Step 1: Add Step-7-heavy coverage + min-count assertions**

The existing `cpg_build_parallel_matches_serial_reference_in_order` (default vs 1-thread node/edge dump) and `cache_blob_bytes_identical_serial_vs_parallel` now exercise the parallel Step 7. Add **minimum node/statement-count assertions** so a silent corpus shrink can't mask a divergence, and confirm the corpus contains Statement nodes:

```rust
#[test]
fn step7_corpus_has_statement_nodes_and_is_nontrivial() {
    let repo = corpus(); // src/navigation — many functions/statements
    let cpg = CpgContext::build(&repo.files, None);
    let stmt_count = cpg
        .cpg
        .node_indices()
        .filter(|&i| matches!(cpg.cpg.node(i), Some(prism::cpg::CpgNode::Statement { .. })))
        .count();
    assert!(stmt_count > 200, "corpus too small to surface Step-7 divergence: {stmt_count}");
}
```

Run: `cargo test --test infra`
Expected: PASS (determinism + cache-byte parity + the min-count guard).

- [ ] **Step 2: Commit**

```bash
cargo fmt
git add tests/infra/parallel_equality_test.rs
git commit -m "test(step7): min-count guard + Step-7 statement-node coverage in parallel_equality"
```

---

## Task 4: Verification & perf gate (orchestrator)

- [ ] **Full gate:** `cargo fmt --check`, `cargo clippy -p prism --lib`, `cargo test --lib`, `cargo test --test infra`, `cargo build --release`.
- [ ] **Tier-A `--matrix-only --allow-stale-sut`: 0 regressions** (behavior-preserving backstop).
- [ ] **Perf re-measure:** cold `nav --no-cache call-stats` on prism/tokio/hugo, branch vs `main`. Expect ~hugo −4s / tokio −2.2s (the spike). Record in the PR.
- [ ] **PR** (when the owner asks). Body: the oracle/parity story + the gate + perf. End with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.

---

## Self-review

- **Spec coverage:** §3 design (Task 2 par_iter/collect_pending), §4 proof — the same-binary serial-reference oracle = gate 1 (Task 1 Step 3, compares node sequence + `stmt_index` ordinals + `location_index`, debug dumps), determinism gate (Task 3, `parallel_equality_test` + min-count), Tier-A (Task 2 Step 4 / Task 4). §3 order contract (whole-file `seen` before classify) = `collect_pending`. Covered.
- **TDD note:** the oracle (Task 1) is the SAFETY NET — it passes against the unchanged serial code (serial==serial), then is the hard guard for the Task-2 refactor (parallel must == frozen serial). The "red" is a refactor that breaks order → the oracle fails. This is the behavior-preserving pattern, not new-behavior red/green.
- **No CACHE_VERSION bump.** If the cache bytes (serial-vs-parallel, same build) or the oracle diverge, the refactor is wrong — fix it, don't bump.
- **Type consistency:** `assemble_step7(files, &mut graph, &mut location_index) -> BTreeMap<(String,usize),NodeIndex>`, `PendingStatement{line,kind:StmtKind,start_byte,end_byte}`, `collect_pending(node,func_types,parsed,&mut seen,&mut out)` used consistently in both tasks.
