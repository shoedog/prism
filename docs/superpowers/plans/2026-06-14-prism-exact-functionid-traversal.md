# Exact-`FunctionId` / Confidence-Aware Call Traversal — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the slice algorithms and `prism nav` exact, confidence-filtered caller/callee traversal on S2's node identity, so name-collision callers (e.g. the 19 petgraph `EdgeRef::target()` FPs on `fn target`) stop being reported while recall stays intact.

**Architecture:** Tag CPG `Call`/`Return` edges with `ResolutionConfidence` at build (B1); add node-seeded BFS (`callers_of_node`/`callees_of_node`) + a `ConfidenceFilter`; add a `nav --confidence` flag on the independent `NavCallEdge` path; migrate the precision-biased slices to `ExactOnly` and recover their call-site lines through a confidence-aware `CallGraph` helper. One cache bump v5→v6. External-receiver NameOnly precision is deferred to Phase-IP (clean additive seam).

**Tech Stack:** Rust (tree-sitter, petgraph, serde), the prism CPG/CallGraph/resolution layers; Python Tier-A harness (`uv run tier-a`).

**Spec:** `docs/superpowers/specs/2026-06-14-prism-exact-functionid-traversal-design.md` (rev 3). Second-round review record: `docs/archive/review-artifacts/prism-query-layer/eft-spec-review-2026-06-14.md`.

---

## Pre-flight (execution setup — do once before Task 1)

The spec/plan live on branch `exact-functionid-traversal`; the Tier-A re-anchor (the baseline + `eval/tools/hydrate_pending.py` + flip doc) is on **main** (`3fc96f9`). Before implementing, **rebase the feature branch onto main** so the code is built against the re-anchored baseline and the flip doc isn't duplicated:

```bash
cd /Users/wesleyjinks/code/slicing
git checkout exact-functionid-traversal
git rebase main
# Expected: one add/add on docs/eval/tier-a/target-c-method-flip-adjudication-2026-06-14.md
# (identical content on both sides). Resolve by keeping either copy:
#   git checkout --theirs docs/eval/tier-a/target-c-method-flip-adjudication-2026-06-14.md
#   git add docs/eval/tier-a/target-c-method-flip-adjudication-2026-06-14.md && git rebase --continue
```

If using subagent-driven-development, create the worktree (superpowers:using-git-worktrees) off the rebased branch. Build the release binary before any Tier-A step: `cargo build --release`.

## File structure (what each task touches)

| Area | Files | Responsibility |
|---|---|---|
| Edge confidence (T1) | `src/cpg/types.rs`, `src/resolution.rs`, `src/cpg/build.rs`, `src/cpg_cache.rs` | `CpgEdge::Call/Return(ResolutionConfidence)`, serde, Step-5 materialization, cache v6 |
| CHA gating (T2) | `src/cpg/build.rs` | Step-9 seed scan + guard + construct, all Exact |
| Node traversal (T3) | `src/cpg/query.rs` | `function_node_for_id`, `ConfidenceFilter`, `callers_of_node`/`callees_of_node` |
| Nav filter (T4) | `src/main.rs`, `src/navigation/queries.rs` | `--confidence` flag, emit+frontier filter |
| Exact helper (T5) | `src/resolution.rs`, `src/call_graph.rs` | `ResolvedCallEdge` + `CallGraph::resolved_caller_edges` |
| Slice migration (T6) | `src/algorithms/{barrier,vertical,threed,spiral,membrane,echo}_slice.rs` | node-seeded traversal + helper site-lines per the Exact/All table |
| byte-arg (T7) | `src/ast.rs`, `src/cpg/build.rs` | `call_argument_texts_at` + Step-5b |
| Folds #10/#12 (T8) | `src/ast.rs`, `src/data_flow.rs` | nested-augmented base; line-collapsed `start==end` |
| Tier-A re-bless (T9) | `eval/tier_a/sut.py`, `eval/tier_a/pinned.py`, `eval/tier_a/cli.py` | thread `--confidence`; supplementary exact metric |
| Acceptance (T10) | (none — verification only) | cache round-trip, full suite, Tier-A |

Test homes: Rust CPG/API/edge tests → `tests/ast/cpg_test.rs` (umbrella `cargo test --test integration`? no — `cargo test --test ast cpg_test::`). Slice tests → `tests/algo/taxonomy/` (umbrella `--test algo_taxonomy`). byte-arg/data-flow → `tests/ast/dfg_test.rs` (`--test ast dfg_test::`). Nav → `tests/cli/` (`--test cli`). Tier-A → `eval/tests/` (`cd eval && uv run pytest`).

---

### Task 1: Confidence-tagged `CpgEdge` + serde + Step-5 materialization + cache v6

**Files:**
- Modify: `src/resolution.rs:8` (serde on `ResolutionConfidence`)
- Modify: `src/cpg/types.rs:154-173` (`CpgEdge` variants), `:267-269` (`is_interprocedural`)
- Modify: `src/cpg/build.rs:369-370` (Step-5 add_edge)
- Modify: `src/cpg_cache.rs:45` (CACHE_VERSION)
- Modify: ~20 `matches!(… CpgEdge::Call | … Return)` sites across `src/cpg/query.rs`, `src/cpg/build.rs`, `src/cpg/cfg_queries.rs`, `src/cpg/trace.rs`, `src/algorithms/circular_slice.rs`, `src/algorithms/gradient_slice.rs`, `src/navigation/queries.rs`
- Test: `tests/ast/cpg_test.rs`

- [ ] **Step 1: Write the failing test** — append to `tests/ast/cpg_test.rs`:

```rust
#[test]
fn call_edge_carries_resolution_confidence() {
    // Two same-named methods; an exact (qualified/typed) call → Call(Exact),
    // a name-only (R6 single-owner) call → Call(NameOnly).
    use prism::cpg::types::CpgEdge;
    use prism::resolution::ResolutionConfidence;
    let files = common::make_rust_test(
        "src/lib.rs",
        r#"
struct A; struct B;
impl A { fn run(&self) {} }
impl B { fn run(&self) {} }
fn exact_caller(a: A) { a.run(); }      // typed receiver → Exact
"#,
    );
    let ctx = common::build_ctx(&files);
    let confs: Vec<ResolutionConfidence> = ctx
        .cpg
        .graph
        .edge_weights()
        .filter_map(|w| match w {
            CpgEdge::Call(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert!(
        confs.contains(&ResolutionConfidence::Exact),
        "expected at least one Call(Exact) edge, got {confs:?}"
    );
}
```

(If `common::build_ctx`/`make_rust_test` differ, use the existing helpers in `tests/common/mod.rs`; the assertion — a `Call(Exact)` weight exists — is the contract.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test ast cpg_test::call_edge_carries_resolution_confidence`
Expected: FAIL to compile — `CpgEdge::Call` takes no argument yet.

- [ ] **Step 3: Add serde to `ResolutionConfidence`** — `src/resolution.rs:8`:

```rust
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
    serde::Serialize, serde::Deserialize,
)]
pub enum ResolutionConfidence {
    Exact,
    NameOnly,
}
```

- [ ] **Step 4: Change the `CpgEdge` variants** — `src/cpg/types.rs:154-173`, add the import and payloads:

```rust
use crate::resolution::ResolutionConfidence;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CpgEdge {
    DataFlow,
    ControlFlow,
    /// Call: a call site invokes a callee, tagged with resolution confidence.
    Call(ResolutionConfidence),
    /// Return: the callee→caller back-edge, same confidence as its Call.
    Return(ResolutionConfidence),
    Contains,
    FieldOf,
}
```

And `is_interprocedural` at `src/cpg/types.rs:267-269`:

```rust
    pub fn is_interprocedural(&self) -> bool {
        matches!(self, CpgEdge::Call(_) | CpgEdge::Return(_))
    }
```

- [ ] **Step 5: Materialize confidence at Step-5** — `src/cpg/build.rs:369-370`:

```rust
                        graph.add_edge(caller_idx, callee_idx, CpgEdge::Call(resolved.confidence));
                        graph.add_edge(callee_idx, caller_idx, CpgEdge::Return(resolved.confidence));
```

- [ ] **Step 6: Fix every confidence-agnostic match site (mechanical).** Compile and fix each error by widening the pattern; do NOT change behavior:
  - `matches!(…, CpgEdge::Call)` → `matches!(…, CpgEdge::Call(_))`; same for `Return`.
  - `matches!(…, CpgEdge::Call | CpgEdge::Return)` → `CpgEdge::Call(_) | CpgEdge::Return(_)`.
  - `CpgEdge::Return =>` / `CpgEdge::Call =>` match arms → `CpgEdge::Return(_) =>` etc.
  Known sites (let the compiler find any others): `src/cpg/query.rs:407,451` (callers_of/callees_of); `src/cpg/cfg_queries.rs`; `src/cpg/trace.rs`; `src/cpg/build.rs:541,563` (handled fully in Task 2 — for now widen to `Call(_)` so it compiles); `src/algorithms/circular_slice.rs`; `src/algorithms/gradient_slice.rs`; `src/navigation/queries.rs` (edge-label). Run `cargo build` repeatedly until clean.

- [ ] **Step 7: Bump the cache version** — `src/cpg_cache.rs:45`:

```rust
const CACHE_VERSION: u32 = 6; // EFT: CpgEdge::Call/Return carry ResolutionConfidence.
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test --test ast cpg_test::call_edge_carries_resolution_confidence`
Expected: PASS. Then `cargo build` clean.

- [ ] **Step 9: Commit**

```bash
cargo fmt
git add -A && git commit -m "feat(cpg): tag Call/Return edges with ResolutionConfidence (cache v6)"
```

---

### Task 2: Step-9 CHA — Exact-gate the seed scan AND the guard (R2 BLOCKER)

**Files:**
- Modify: `src/cpg/build.rs:541` (seed scan), `:563` (dup guard), `:565-566` (construct)
- Test: `tests/ast/cpg_test.rs`

- [ ] **Step 1: Write the failing tests (both directions)** — append to `tests/ast/cpg_test.rs`:

```rust
#[test]
fn cha_does_not_launder_nameonly_into_exact() {
    // A NameOnly call edge alone must NOT seed a CHA Exact edge.
    use prism::cpg::types::CpgEdge;
    use prism::resolution::ResolutionConfidence;
    let files = common::make_rust_test(
        "src/lib.rs",
        r#"
trait T { fn m(&self); }
struct S; impl T for S { fn m(&self) {} }
fn caller(x: &dyn T) { x.m(); }   // dyn dispatch; the only basis is name-only
"#,
    );
    let ctx = common::build_ctx(&files);
    // No Call(Exact) edge should target S::m solely because a NameOnly x.m() exists.
    let exacts = ctx.cpg.graph.edge_weights()
        .filter(|w| matches!(w, CpgEdge::Call(ResolutionConfidence::Exact)))
        .count();
    let nameonly = ctx.cpg.graph.edge_weights()
        .filter(|w| matches!(w, CpgEdge::Call(ResolutionConfidence::NameOnly)))
        .count();
    assert!(nameonly >= 1, "expected the name-only dispatch edge");
    assert_eq!(exacts, 0, "a NameOnly seed must not be laundered into Exact CHA edges");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ast cpg_test::cha_does_not_launder_nameonly_into_exact`
Expected: FAIL — `exacts` is nonzero (the seed scan currently expands from `Call(_)`).

- [ ] **Step 3: Exact-gate the seed scan** — `src/cpg/build.rs:539-543`:

```rust
                let callees: Vec<_> = graph
                    .edges(caller_idx)
                    // EFT: only Exact call edges seed CHA expansion — a NameOnly
                    // edge must not launder into freshly-minted Exact CHA edges.
                    .filter(|e| matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact)))
                    .map(|e| e.target())
                    .collect();
```

- [ ] **Step 4: Exact-gate the dup guard + tag the new edges Exact** — `src/cpg/build.rs:560-567`:

```rust
            for (from, to) in &virtual_edges {
                // CHA dispatch is type-confirmed = Exact. Guard on an existing
                // Exact edge so a NameOnly pair is *upgraded* (not blocked).
                let already_exists = graph.edges(*from).any(|e| {
                    e.target() == *to
                        && matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact))
                });
                if !already_exists {
                    graph.add_edge(*from, *to, CpgEdge::Call(ResolutionConfidence::Exact));
                    graph.add_edge(*to, *from, CpgEdge::Return(ResolutionConfidence::Exact));
                }
            }
```

Add `use crate::resolution::ResolutionConfidence;` to `build.rs` if not already imported.

- [ ] **Step 5: Add the positive (upgrade) test** — append:

```rust
#[test]
fn cha_upgrades_nameonly_pair_to_exact() {
    // A virtual-dispatch pair that ALSO has an R6 NameOnly edge ends with an Exact CHA edge.
    use prism::cpg::types::CpgEdge;
    use prism::resolution::ResolutionConfidence;
    let files = common::make_rust_test(
        "src/lib.rs",
        common::CHA_OVERRIDE_FIXTURE, // a base/override + a concrete dispatch site
    );
    let ctx = common::build_ctx(&files);
    assert!(
        ctx.cpg.graph.edge_weights()
            .any(|w| matches!(w, CpgEdge::Call(ResolutionConfidence::Exact))),
        "CHA-confirmed dispatch must yield an Exact edge"
    );
}
```

If `common::CHA_OVERRIDE_FIXTURE` doesn't exist, inline a small trait+two-impls+call fixture that currently produces a CHA virtual edge (mirror an existing CHA test in `tests/ast/cpg_test.rs`).

- [ ] **Step 6: Run both tests to verify they pass**

Run: `cargo test --test ast cpg_test::cha_`
Expected: PASS (both `cha_does_not_launder_nameonly_into_exact` and `cha_upgrades_nameonly_pair_to_exact`).

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add -A && git commit -m "fix(cpg): Exact-gate Step-9 CHA seed scan + guard (no NameOnly laundering)"
```

---

### Task 3: Node-seeded exact traversal — `function_node_for_id` + `ConfidenceFilter` + `callers_of_node`/`callees_of_node`

**Files:**
- Modify: `src/cpg/query.rs` (add the enum + 3 methods near `function_node`/`callers_of`)
- Test: `tests/ast/cpg_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn callers_of_node_filters_by_confidence_and_excludes_seed() {
    use prism::cpg::query::ConfidenceFilter;
    let files = common::make_rust_test(
        "src/lib.rs",
        r#"
struct A; struct B;
impl A { fn run(&self) {} }
impl B { fn run(&self) {} }
fn exact_caller(a: A) { a.run(); }       // Exact caller of A::run
fn nameonly_caller(x: &dyn Run) { x.run(); } // name-only caller (not A::run specifically)
trait Run { fn run(&self); }
"#,
    );
    let ctx = common::build_ctx(&files);
    // Seed = A::run by exact identity (file, name, start_line).
    let a_run = ctx.cpg.function_candidates("src/lib.rs", "run")
        .into_iter()
        .find(|&n| ctx.cpg.to_function_id(n).is_some())
        .expect("A::run node");
    let exact = ctx.cpg.callers_of_node(a_run, 2, ConfidenceFilter::ExactOnly);
    let all = ctx.cpg.callers_of_node(a_run, 2, ConfidenceFilter::All);
    assert!(all.len() >= exact.len(), "All ⊇ ExactOnly");
    assert!(!exact.iter().any(|(n, _)| *n == a_run), "seed excluded at depth 0");
    assert!(exact.iter().all(|(_, d)| *d >= 1), "depths are 1-based for callers");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ast cpg_test::callers_of_node_filters_by_confidence_and_excludes_seed`
Expected: FAIL — `ConfidenceFilter` / `callers_of_node` don't exist.

- [ ] **Step 3: Implement the API** — add to `src/cpg/query.rs` (after `function_candidates`, ~line 31, and near `callers_of`):

```rust
/// Confidence filter for node-seeded traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceFilter {
    /// Only traverse Call/Return edges resolved Exact.
    ExactOnly,
    /// Traverse all Call/Return edges (today's recall behavior).
    All,
}

impl CodePropertyGraph {
    /// Exact identity lookup keyed (file, name, start_line) — NOT first-candidate.
    pub fn function_node_for_id(&self, id: &FunctionId) -> Option<NodeIndex> {
        self.func_index
            .get(&(id.file.clone(), id.name.clone(), id.start_line))
            .copied()
    }

    fn confidence_ok(w: &CpgEdge, f: ConfidenceFilter) -> bool {
        use crate::resolution::ResolutionConfidence::Exact;
        match f {
            ConfidenceFilter::All => true,
            ConfidenceFilter::ExactOnly => {
                matches!(w, CpgEdge::Call(Exact) | CpgEdge::Return(Exact))
            }
        }
    }

    /// BFS over Return edges (callee→caller) from a node seed, confidence-filtered.
    /// Returns (caller node, depth); seed excluded; deduped by node.
    pub fn callers_of_node(
        &self,
        callee: NodeIndex,
        max_depth: usize,
        filter: ConfidenceFilter,
    ) -> Vec<(NodeIndex, usize)> {
        self.bfs_interproc(callee, max_depth, filter, /*forward=*/ false)
    }

    /// BFS over Call edges (caller→callee) from a node seed, confidence-filtered.
    pub fn callees_of_node(
        &self,
        caller: NodeIndex,
        max_depth: usize,
        filter: ConfidenceFilter,
    ) -> Vec<(NodeIndex, usize)> {
        self.bfs_interproc(caller, max_depth, filter, /*forward=*/ true)
    }

    fn bfs_interproc(
        &self,
        seed: NodeIndex,
        max_depth: usize,
        filter: ConfidenceFilter,
        forward: bool,
    ) -> Vec<(NodeIndex, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<NodeIndex> = BTreeSet::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        queue.push_back((seed, 0));
        visited.insert(seed);
        while let Some((node, depth)) = queue.pop_front() {
            if depth > 0 {
                result.push((node, depth));
            }
            if depth >= max_depth {
                continue;
            }
            for edge in self.graph.edges(node) {
                let keep = if forward {
                    matches!(edge.weight(), CpgEdge::Call(_))
                } else {
                    matches!(edge.weight(), CpgEdge::Return(_))
                };
                if keep && Self::confidence_ok(edge.weight(), filter) {
                    let next = edge.target();
                    if visited.insert(next) {
                        queue.push_back((next, depth + 1));
                    }
                }
            }
        }
        result
    }
}
```

(If `func_index`'s key type differs, match it; it is `(String, String, usize)` per `build.rs:363-367`.)

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test ast cpg_test::callers_of_node_filters_by_confidence_and_excludes_seed`
Expected: PASS.

- [ ] **Step 5: Add the `function_node_for_id` distinctness test**

```rust
#[test]
fn function_node_for_id_is_start_line_keyed() {
    let files = common::make_rust_test("src/lib.rs",
        "fn dup() {}\nmod a { pub fn dup() {} }\n");
    let ctx = common::build_ctx(&files);
    let ids: Vec<_> = ctx.cpg.call_graph.functions.get("dup").cloned().unwrap_or_default();
    assert!(ids.len() >= 2, "two `dup` defs");
    let n0 = ctx.cpg.function_node_for_id(&ids[0]).unwrap();
    let n1 = ctx.cpg.function_node_for_id(&ids[1]).unwrap();
    assert_ne!(n0, n1, "distinct nodes by start_line, not first-candidate");
}
```

Run: `cargo test --test ast cpg_test::function_node_for_id_is_start_line_keyed` → PASS.

- [ ] **Step 6: Note the deliberately-unmigrated by-name sites (R2/N1).** Add a one-line doc comment above `call_reachable_functions` (query.rs:338) and `callees_of` (query.rs:433): `// Recall/by-name path (uses function_node first-candidate); NOT part of the EFT precision migration.` No behavior change.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo test --test ast cpg_test::
git add -A && git commit -m "feat(cpg): node-seeded confidence-filtered traversal + function_node_for_id"
```

---

### Task 4: Nav `--confidence` filter (emit + frontier)

**Files:**
- Modify: `src/main.rs` (clap `--confidence` on `nav callers`/`callees`)
- Modify: `src/navigation/queries.rs` (callers ~263-314, callees ~420-459: filter emit + enqueue)
- Test: `tests/cli/` (new `confidence_test.rs` or extend an existing nav test) + a default-unchanged golden assertion

- [ ] **Step 1: Write the failing test** — `tests/cli/` (add `mod confidence_test;` to `tests/cli/main.rs`):

```rust
// tests/cli/confidence_test.rs
use assert_cmd::Command;

#[test]
fn nav_callers_confidence_exact_drops_nameonly() {
    let repo = common::fixture_repo_with_name_collision(); // two same-named methods + a typed + an untyped caller
    let all = common::run_nav(&repo, &["callers", "--symbol", "run", "--confidence", "all"]);
    let exact = common::run_nav(&repo, &["callers", "--symbol", "run", "--confidence", "exact"]);
    assert!(exact.evidence.len() <= all.evidence.len());
    assert!(exact.evidence.iter().all(|e| e.score >= 0.99), "exact keeps only score-1.0 edges");
}

#[test]
fn nav_callers_default_is_all() {
    let repo = common::fixture_repo_with_name_collision();
    let default = common::run_nav(&repo, &["callers", "--symbol", "run"]);
    let all = common::run_nav(&repo, &["callers", "--symbol", "run", "--confidence", "all"]);
    assert_eq!(default.evidence, all.evidence, "absent flag == all (byte-for-byte)");
}
```

(Use existing CLI test helpers in `tests/common/mod.rs`; if `run_nav` doesn't exist, invoke `Command::cargo_bin("prism")` with `nav … --format json` and parse, mirroring an existing nav CLI test.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test cli confidence_test::`
Expected: FAIL — `--confidence` is an unknown argument.

- [ ] **Step 3: Add the clap flag** — in `src/main.rs`, on the `nav callers` and `nav callees` subcommands add:

```rust
        /// Confidence filter: `all` (default, today's recall) or `exact`.
        #[arg(long, value_parser = ["all", "exact"], default_value = "all")]
        confidence: String,
```

Thread `confidence` into the nav query call (parse to a bool `exact = confidence == "exact"`).

- [ ] **Step 4: Filter in `queries.rs`** — in the callers path (~`src/navigation/queries.rs:263-314`) and callees path (~`:420-459`), when `exact`: skip any `NavCallEdge` whose `confidence != ResolutionConfidence::Exact` BOTH at emission and before enqueuing the frontier (so a NameOnly edge is neither emitted nor expanded). Keep unresolved-callee items only in `all` mode. Pass `exact: bool` down the relevant query fns (signature change is internal to the nav module).

```rust
// at each emit/enqueue of a resolved edge:
if exact && edge.confidence != ResolutionConfidence::Exact {
    continue;
}
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --test cli confidence_test::`
Expected: PASS (both tests).

- [ ] **Step 6: Guard the byte-for-byte default with the existing nav_compat golden.**

Run: `cargo test --test cli` (the whole CLI suite, incl. any `nav_compat` golden) — Expected: PASS, no golden churn (default output unchanged).

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add -A && git commit -m "feat(nav): --confidence exact|all filter (emit + frontier); default all unchanged"
```

---

### Task 5: Exact call-edge helper on `CallGraph` (the F7 site-line source)

**Files:**
- Modify: `src/resolution.rs` (`ResolvedCallEdge` struct + `impl CallGraph { resolved_caller_edges }`)
- Test: `tests/ast/cpg_test.rs` (or `tests/integration/call_graph_test.rs`)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn resolved_caller_edges_carry_confidence_and_line() {
    let files = common::make_rust_test("src/lib.rs",
        "struct A;\nimpl A { fn run(&self) {} }\nfn c(a: A) {\n    a.run();\n}\n");
    let ctx = common::build_ctx(&files);
    let a_run = ctx.cpg.call_graph.functions.get("run").unwrap()[0].clone();
    let edges = ctx.cpg.call_graph.resolved_caller_edges(&a_run);
    assert!(edges.iter().any(|e|
        e.caller.name == "c"
        && e.call_site_line == 4
        && e.confidence == prism::resolution::ResolutionConfidence::Exact));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ast cpg_test::resolved_caller_edges_carry_confidence_and_line`
Expected: FAIL — `resolved_caller_edges`/`ResolvedCallEdge` don't exist.

- [ ] **Step 3: Implement** — in `src/resolution.rs`:

```rust
/// A resolved caller edge: who calls the seed, with what confidence, at which line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallEdge {
    pub caller: FunctionId,
    pub call_site_line: usize,
    pub confidence: ResolutionConfidence,
}

impl CallGraph {
    /// All call sites that resolve to `callee`, with caller, line, and confidence.
    /// The site-line source for slice witnesses — CPG edges carry no line.
    pub fn resolved_caller_edges(&self, callee: &FunctionId) -> Vec<ResolvedCallEdge> {
        let mut out = Vec::new();
        for sites in self.calls.values() {
            for site in sites {
                for r in self.resolve_call_site(site) {
                    if r.target == callee {
                        out.push(ResolvedCallEdge {
                            caller: site.caller.clone(),
                            call_site_line: site.line,
                            confidence: r.confidence,
                        });
                    }
                }
            }
        }
        out
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test ast cpg_test::resolved_caller_edges_carry_confidence_and_line`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add -A && git commit -m "feat(resolution): resolved_caller_edges — confidence + site line for slice witnesses"
```

---

### Task 6: Slice migration — ExactOnly (barrier/vertical/threed/spiral) + All (membrane/echo)

**Files:**
- Modify: `src/algorithms/barrier_slice.rs:52-122`, `vertical_slice.rs:91-127`, `threed_slice.rs:72-134`, `spiral_slice.rs:107-146`, `membrane_slice.rs:47-203`, `echo_slice.rs:167-182`
- Test: `tests/algo/taxonomy/` (add `mod eft_precision_test;` to `tests/algo/taxonomy/main.rs`)

For each precision slice (barrier/vertical/threed/spiral): they already obtain the seed via `function_at` (returns `(NodeIndex, FunctionId)`) or a `func_id`. Replace the by-name `callers_of_in_file(func_id.name, depth, Some(file))` with node-seeded `callers_of_node(node, depth, ConfidenceFilter::ExactOnly)` (use the `NodeIndex` from `function_at`, or `function_node_for_id(&func_id)`), and `callees_of(name, file, depth)` with `callees_of_node(node, depth, ExactOnly)`. Recover call-site **lines** from `call_graph.resolved_caller_edges(&callee_id)` filtered to `confidence == Exact` and to the caller set returned by `callers_of_node` (join on caller `start_line` + the resolved target `FunctionId`) — replacing the `call_graph.callers.get(func_name)` scans. membrane/echo do the same but with `ConfidenceFilter::All` and keep `resolved_caller_edges` un-filtered by confidence (Exact+NameOnly) so the R6 C-struct-callback caller survives.

- [ ] **Step 1: Write the failing precision test** — `tests/algo/taxonomy/eft_precision_test.rs`:

```rust
#[test]
fn barrier_does_not_cross_report_same_name_methods() {
    // Two structs each with `handle`; a diff touching A::handle must not pull
    // B::handle's callers into the barrier slice.
    let repo = common::write_repo(&[
        ("src/a.rs", "pub struct A;\nimpl A { pub fn handle(&self) {} }\n"),
        ("src/b.rs", "pub struct B;\nimpl B { pub fn handle(&self) {} }\n"),
        ("src/use_b.rs", "use crate::b::B;\nfn calls_b(b: B) { b.handle(); }\n"),
    ]);
    let diff = common::diff_touching(&repo, "src/a.rs", "handle");
    let slice = common::run_barrier(&repo, &diff);
    assert!(!slice.includes_line("src/use_b.rs", 2),
        "B::handle's caller must not appear in A::handle's barrier slice");
}

#[test]
fn membrane_keeps_c_struct_callback_caller() {
    // membrane uses All: the R6 single-owner NameOnly caller must still surface.
    let repo = common::c_struct_callback_repo();
    let diff = common::diff_touching(&repo, "src/dev.c", "probe");
    let slice = common::run_membrane(&repo, &diff);
    assert!(slice.includes_callback_caller(), "membrane (All) must keep the R6 callback caller");
}
```

(Adapt to the actual slice-runner test helpers; the contracts — barrier excludes the cross-name caller, membrane keeps the NameOnly callback — are the point. Reuse an existing C-callback fixture if one exists.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test algo_taxonomy eft_precision_test::`
Expected: barrier test FAILs (currently cross-reports `b.handle()` via the by-name union); membrane test PASSES already (guards against regression).

- [ ] **Step 3: Migrate barrier** — `src/algorithms/barrier_slice.rs`: at `:52` keep the `(idx, func_id)` from `function_at` (don't discard `idx`); replace `:84` `callers_of_in_file(func_id.name, …)` with `callers_of_node(idx, max_depth, ConfidenceFilter::ExactOnly)`; replace the `:110` `call_graph.callers.get(func_name)` site-line scan with `call_graph.resolved_caller_edges(&func_id)` filtered to `Exact` + the caller set; replace `:122` `callees_of(...)` with `callees_of_node(idx, max_depth, ExactOnly)`. Map `(NodeIndex, depth)` results back to `FunctionId` via `to_function_id`.

- [ ] **Step 4: Migrate vertical, threed, spiral the same way (ExactOnly).** vertical `:91/98/127`, threed `:72/82/83/121/133/134`, spiral `:107/117/135/146`. For spiral/threed sites that recover a `func_id` from a name+file, seed via `function_node_for_id`. Keep depths as-is (vertical 10, spiral 1/2, threed 1/2).

- [ ] **Step 5: Migrate membrane + echo (All).** membrane `:47/78/203`, echo `:167/182`: use `callers_of_node(node, depth, ConfidenceFilter::All)` and `resolved_caller_edges` (Exact+NameOnly, no confidence filter) for the site lines, replacing the raw `call_graph.callers.get` scans. This preserves the R6 callback caller.

- [ ] **Step 6: Run to verify both pass + no slice regressions**

Run: `cargo test --test algo_taxonomy eft_precision_test::` → PASS (both).
Run: `cargo test` → all existing slice/lang tests PASS (no recall regression in membrane/echo).

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add -A && git commit -m "feat(slices): node-seeded confidence-aware traversal (barrier/vertical/threed/spiral ExactOnly; membrane/echo All)"
```

---

### Task 7: byte-aware interprocedural arg binding (S2 #9)

**Files:**
- Modify: `src/ast.rs` (add `call_argument_texts_at` near `:4197`)
- Modify: `src/cpg/build.rs` (Step-5b — use the byte variant)
- Test: `tests/ast/dfg_test.rs`

- [ ] **Step 1: Write the failing test** — `tests/ast/dfg_test.rs`:

```rust
#[test]
fn two_calls_one_line_bind_their_own_args() {
    // `f(a); f(b);` on one line → a→param AND b→param edges (not both to a).
    let files = common::make_rust_test("src/lib.rs",
        "fn f(p: i32) {}\nfn c(a: i32, b: i32) { f(a); f(b); }\n");
    let ctx = common::build_ctx(&files);
    let dfg = &ctx.cpg.dfg;
    assert!(common::arg_binds(dfg, "a", "p"), "a → p");
    assert!(common::arg_binds(dfg, "b", "p"), "b → p");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ast dfg_test::two_calls_one_line_bind_their_own_args`
Expected: FAIL — both args bind via the first same-line call (`call_argument_texts` is line-keyed).

- [ ] **Step 3: Add the byte-keyed variant** — `src/ast.rs` after `:4201`:

```rust
    /// Like `call_argument_texts`, but selects the call expression whose start
    /// byte == `start_byte` (disambiguates multiple calls on one line).
    pub fn call_argument_texts_at(&self, start_byte: usize, callee_name: &str) -> Vec<String> {
        let mut args = Vec::new();
        self.collect_call_args_at(self.tree.root_node(), start_byte, callee_name, &mut args);
        args
    }
```

Add a `collect_call_args_at` mirroring `collect_call_args` (`:4203`) but matching `node.start_byte() == start_byte` instead of `node_line == line`.

- [ ] **Step 4: Use it in Step-5b** — in `src/cpg/build.rs` where Step-5b binds args (search `call_argument_texts(`), pass `site.start_byte` via `call_argument_texts_at(site.start_byte, &site.callee_name)`.

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --test ast dfg_test::two_calls_one_line_bind_their_own_args` → PASS.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add -A && git commit -m "feat(dfg): byte-keyed call_argument_texts_at for per-call arg binding"
```

---

### Task 8: Folded low-priority S2 deferrals (#10, #12)

**Files:**
- Modify: `src/ast.rs` (`collect_identifier_path_spans` ~`:2130`)
- Modify: `src/data_flow.rs` (line-collapsed `start==end` ~`:375-376/437-438`)
- Test: `tests/ast/dfg_test.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn nested_augmented_base_peels_to_leftmost() {
    // `o.config.timeout += 1` → a Use(o) base edge (peeled to leftmost identifier).
    let files = common::make_rust_test("src/lib.rs",
        "struct C { config: Cfg }\nstruct Cfg { timeout: i32 }\nfn f(mut o: C) { o.config.timeout += 1; }\n");
    let ctx = common::build_ctx(&files);
    assert!(common::has_use(&ctx.cpg.dfg, "o"), "leftmost base `o` is used");
}

#[test]
fn line_collapsed_reference_start_eq_end() {
    let files = common::make_rust_test("src/lib.rs", "fn f(x: i32) -> i32 { x }\n");
    let ctx = common::build_ctx(&files);
    // every line-collapsed VarLocation has start_byte == end_byte (production path)
    assert!(common::all_line_collapsed_refs_have_zero_width(&ctx.cpg.dfg));
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --test ast dfg_test::nested_augmented_base_peels_to_leftmost dfg_test::line_collapsed_reference_start_eq_end`
Expected: FAIL.

- [ ] **Step 3: Implement #10** — in `src/ast.rs` `collect_identifier_path_spans` (~`:2130`): for an augmented-assignment / nested receiver (`o.config.timeout`), peel nested receivers to the leftmost identifier before the base `Use(o)` fallback.

- [ ] **Step 4: Implement #12** — in `src/data_flow.rs` (~`:375-376` and `:437-438`): ensure a line-collapsed reference uses `line_start_byte` for BOTH `start` and `end` (zero-width anchor); assert `start == end` holds.

- [ ] **Step 5: Run to verify they pass**

Run: `cargo test --test ast dfg_test::nested_augmented_base_peels_to_leftmost dfg_test::line_collapsed_reference_start_eq_end` → PASS.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add -A && git commit -m "feat(dfg): nested-augmented base peel (#10) + line-collapsed start==end (#12)"
```

---

### Task 9: Tier-A re-bless harness wiring (`--confidence` + supplementary exact metric)

**Files:**
- Modify: `eval/tier_a/sut.py:184-198` (`callers`/`callees` gain a `confidence` param → CLI flag)
- Modify: `eval/tier_a/pinned.py` (record supplementary exact metric for `target-c-method`)
- Test: `eval/tests/test_sut.py`, `eval/tests/test_pinned.py`

- [ ] **Step 1: Write the failing test** — `eval/tests/test_sut.py`:

```python
def test_callers_threads_confidence_flag(monkeypatch):
    captured = {}
    def fake_run(self, args):
        captured["args"] = args
        return []
    monkeypatch.setattr(PrismCli, "_run", fake_run)
    sut = PrismCli(".", sut_bin="prism", allow_stale=True)
    sut.callers("/repo", _seed(), confidence="exact")
    assert "--confidence" in captured["args"]
    assert "exact" in captured["args"]
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd eval && uv run pytest tests/test_sut.py::test_callers_threads_confidence_flag -q`
Expected: FAIL — `callers()` has no `confidence` kwarg.

- [ ] **Step 3: Thread `--confidence`** — `eval/tier_a/sut.py:184`:

```python
    def callers(self, corpus_root: str, seed: FunctionDef, confidence: str = "all") -> list[CallEdge]:
        loc = self._loc(seed)
        if loc is None:
            return []
        args = ["callers", "--repo", corpus_root, "--location", loc, "--depth", "1"]
        if confidence != "all":
            args += ["--confidence", confidence]
        ev = self._run(args)
        return self._edges(ev, "callers", seed)
```

(Mirror for `callees`. Keep the default `"all"` so existing call sites are unchanged — byte-for-byte.)

- [ ] **Step 4: Run to verify it passes**

Run: `cd eval && uv run pytest tests/test_sut.py::test_callers_threads_confidence_flag -q` → PASS.

- [ ] **Step 5: Add the supplementary exact metric to the pinned probe** — in `eval/tier_a/pinned.py` `run_pinned`, for `target-c-method` additionally call `sut.callers(corpus_root, pfd, confidence="exact")`, compute P/R against the oracle, and attach it to the result as `out["exact_supplementary"] = {"precision": …, "recall": …}`. Do **not** change `expected` (`known_fail`) or the headline `outcome` (`flip_candidate`) — the default-measurement headline (R=1.0, P=0.208) stays primary. Add a test in `test_pinned.py` asserting the supplementary block exists and `expected`/headline are unchanged.

```python
def test_target_c_method_reports_exact_supplementary():
    probe = next(p for p in PINNED if p["id"] == "target-c-method")
    # default headline unchanged
    out = evaluate_pinned(probe, _prism_edges_default(), _oracle_edges(), False)
    assert out["expected"] == "known_fail"
    assert out["outcome"] == "flip_candidate"
    # supplementary exact P=R=1.0 recorded separately (wired in run_pinned)
```

- [ ] **Step 6: Run the harness tests**

Run: `cd eval && uv run pytest tests/test_sut.py tests/test_pinned.py -q` → PASS.

- [ ] **Step 7: Commit**

```bash
cd /Users/wesleyjinks/code/slicing
git add -A && git commit -m "feat(eval): thread nav --confidence; target-c-method exact supplementary metric"
```

---

### Task 10: Acceptance — cache v6 round-trip + full suite + Tier-A

**Files:** none (verification + a round-trip test in `tests/ast/cpg_test.rs`)

- [ ] **Step 1: Cache round-trip test**

```rust
#[test]
fn cache_v6_round_trips_edge_confidence() {
    use prism::cpg::types::CpgEdge;
    use prism::resolution::ResolutionConfidence;
    let files = common::make_rust_test("src/lib.rs",
        "struct A;\nimpl A { fn run(&self) {} }\nfn c(a: A) { a.run(); }\n");
    let dir = tempfile::tempdir().unwrap();
    let ctx = common::build_ctx_cached(&files, dir.path()); // writes v6 cache
    let reloaded = common::build_ctx_cached(&files, dir.path()); // reads it back
    let count = |ctx: &_| /* count Call(Exact) edges */
        common::count_call_exact(ctx);
    assert_eq!(count(&ctx), count(&reloaded), "Call(Exact) survives cache round-trip");
    assert!(count(&reloaded) >= 1);
}
```

(If a cached-build helper doesn't exist, drive `cpg_cache::save`/`load` directly per `src/cpg_cache.rs`.)

- [ ] **Step 2: Verify v5 invalidation**

Run: `cargo test --test ast cpg_test::cache_v6_round_trips_edge_confidence` → PASS. Confirm a stale v5 file is rejected (the `cache.version != CACHE_VERSION` path at `src/cpg_cache.rs:250`).

- [ ] **Step 3: Full Rust suite + fmt + mcp feature**

```bash
cargo fmt --check
cargo test
cargo test --features mcp
```
Expected: all PASS.

- [ ] **Step 4: Tier-A matrix + quick (per CLAUDE.md)**

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # exit 0; matrix 29 ok + 4 expected_gap
cd eval && uv run tier-a --quick --allow-stale-sut          # corr P/R sane; pinned target-c-method flip_candidate + exact supplementary P=R=1.0
```
Paste the matrix result + the pinned probe block into the PR description. Do NOT re-baseline (`baseline.md` is the deliberate anchor; the full re-run is human-triggered).

- [ ] **Step 5: Commit**

```bash
cd /Users/wesleyjinks/code/slicing
git add -A && git commit -m "test(eft): cache v6 round-trip + acceptance"
```

---

## Self-review (run before handoff)

- **Spec coverage:** §2 → T1+T2; §3 → T3; §4 → T4; §5 → T5+T6; §6 → T7; §7 → T8; §9 Tier-A re-bless → T9; §9 cache/acceptance → T10. §10 (Phase-IP) is out of scope. ✓
- **Type/name consistency:** `ConfidenceFilter::{ExactOnly, All}`, `function_node_for_id`, `callers_of_node`/`callees_of_node`, `ResolvedCallEdge { caller, call_site_line, confidence }`, `resolved_caller_edges`, `call_argument_texts_at` — used consistently T3/T5/T6/T7. `CpgEdge::Call(ResolutionConfidence)` consistent T1/T2/T3/T10.
- **Ordering/dependencies:** T1 is the breaking enum change (all later tasks depend on it); T2 depends on T1; T3 on T1; T5 independent of T1 (nav/resolution path) but grouped; T6 on T3+T5; T4 on the nav path only; T9 on T4. Execute in numeric order.
- **No placeholders:** every code step shows code; every run step shows the command + expected outcome. Fixture-helper names (`common::…`) defer to the real `tests/common/mod.rs` — implementers must use the existing helpers; the asserted contract is concrete in each test.
