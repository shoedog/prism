# Prism Navigation Layer — Plan 2: callers / callees / ego-graph (v4, review-hardened ×3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the call-graph navigation queries — `prism nav callers`/`callees` (qualifier-aware, multi-hop, with call-site evidence) and `ego-graph` (bounded CPG-edge neighborhood) — on top of the Plan 1 nav layer.

**Architecture:** Purely additive (no CPG-core logic edits). A shared seed resolver maps `--symbol`/`--location` to a function node (or `QueryError`). `callers`/`callees` traverse the retained `CallGraph` (`cpg.call_graph`) by **resolved `FunctionId` identity** (not bare name) — qualifier-aware via `resolve_callees_qualified` — as a bounded BFS over `depth` hops with `score = 1/(1+hop)`. `ego-graph` BFS-walks `cpg.graph` filtered by `CpgEdge` kind. All three subcommands share one `Err(QueryError) → JSON envelope + exit(3)` renderer.

**Tech Stack:** Rust, the Plan 1 nav layer (`NavigationIndex`, `Evidence`, `NavigationSession`, `seed`-style resolution), `petgraph` (`EdgeRef`, `Direction`), clap, serde_json, assert_cmd.

**Spec:** §7–§8, §12, §16. **Builds on:** Plan 1 (merged to main — `prism nav nodes-at` shipped).

---

## 0. Plan-review disposition (closes the codex-exec + claude-coverage review)

| Finding | Resolution | Where |
|---|---|---|
| B1 test targets run before registered | `[[test]]` registration is **Step 1** of each task, before the first `cargo test` | all tasks |
| B2 `resolve_callees_qualified` arg order | `(callee_name, caller_file, qualifier)` — verified `call_graph.rs:654` | Task 3, helper |
| B3 empty `Callees` clap variant | full field list spelled out | Task 5 |
| B4 `ego` has no error path | shared `render_err(&QueryError)->(String,i32)`; all three subcommands route Err→exit(3) | Task 5 |
| M5 `callers` ignores resolved identity | filter `CallSite`s to those that **resolve to the seed `FunctionId`** (qualifier-aware) | Task 2 |
| M6 `--depth>1` silent no-op | real bounded BFS over hops (callers reverse / callees forward), `score=1/(1+hop)` | Tasks 2–3 |
| M7 `git commit -am` drops new files | `git add -A` in every task | all tasks |
| M8 qualifier path + goldens untested | qualified-call fixture test (Task 3) + tool-generated goldens task (Task 6) | Tasks 3, 6 |
| m9 ego visited-guard ineffective | enqueue only when newly discovered (`idx_of` returns `(i, is_new)`) | Task 4 |
| m10 `EgoEdges` selector undefined | dropped — `--edges` is a parsed comma string → `Vec<&str>` | Task 4/5 |
| m11 ambiguity fixture may not yield 2 | use **two top-level same-named fns across two files** (naming-robust) | Task 1 |
| m12 new-subcommand rejection untested | one `--diff`-rejection + one unknown-`--format` test per subcommand | Task 5 |
| m13 duplicate `CpgNode` import | extend the existing `use crate::cpg::...` line | Task 4 |

**Round 2 closed:** R2-B1 `fid_of` adds `..` (the `ordinal` field); R2-B2 `(**f).clone()` (resolve returns `Vec<&FunctionId>`); degenerate goldens → real multi-file `nav_callgraph` fixture + genuine empty case (Task 6); innermost-enclosing **nested-function** test (Task 1); full m12 rejection matrix across all three subcommands (Task 5); callers emits **per-call-site** (symmetry with callees); text `render` prints `why`; `--max-results`/nonexistent-seed/`render_err`-JSON/`IndirectCallApprox` **deferred-in-writing** (see end).

## Grounding (verified)

- `cpg.call_graph: CallGraph` (`cpg.rs:580`): `calls: BTreeMap<FunctionId, BTreeSet<CallSite>>`, `callers: BTreeMap<String, Vec<CallSite>>`, `functions: BTreeMap<String, Vec<FunctionId>>`.
- `CallSite { caller: FunctionId, callee_name: String, line: usize, qualifier: Option<String> }` (`call_graph.rs:23`).
- `resolve_callees_qualified(callee_name: &str, caller_file: &str, qualifier: Option<&str>) -> Vec<&FunctionId>` (`call_graph.rs:654`). **Confirm the return type at execution and adjust `&`/deref.**
- `FunctionId { name, file, start_line, end_line }` — confirm field names at `call_graph.rs`.
- Plan 1: `NavigationIndex.name_index`, `enclosing_function`, `cpg.node`, `Evidence`/`Reason`/`SymbolRef`/`QueryError` types, `cpg.graph: DiGraph<CpgNode, CpgEdge>`, `CpgEdge = {DataFlow,ControlFlow,Call,Return,Contains,FieldOf}`.

## File Structure

| File | Responsibility |
|---|---|
| `src/navigation/seed.rs` (new) | `--symbol`/`--location` → `FunctionId` node; `AmbiguousSymbol`/`SymbolNotFound` |
| `src/navigation/queries.rs` (modify) | `callers`, `callees` (BFS, qualifier-aware), `ego_graph` |
| `src/navigation/types.rs` (modify) | `EgoNode`/`EgoEdge`/`EgoGraph` output |
| `src/output/navigation.rs` (modify) | `render_ego`, shared `render_err` |
| `src/main.rs` (modify) | `NavQuery::{Callers,Callees,Ego}` + dispatch |
| `tests/navigation/*`, `tests/cli/nav_compat_test.rs`, `tests/fixtures/nav_compat/golden/` | tests + tool-generated goldens |

---

## Task 1: Shared seed resolution (`navigation/seed.rs`)

**Files:** Create `src/navigation/seed.rs`, `tests/navigation/seed_test.rs`.

- [ ] **Step 1: Register the test target, then write the failing test**

Add to `Cargo.toml`:
```toml
[[test]]
name = "navigation_seed"
path = "tests/navigation/seed_test.rs"
```
```rust
// tests/navigation/seed_test.rs
use prism::navigation::types::{QueryError, SymbolRef};
use prism::navigation::{seed, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files { std::fs::write(dir.path().join(name), src).unwrap(); }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn resolves_symbol_unique() {
    let s = session(&[("a.py", "def foo():\n    return 1\n")]);
    let r = seed::resolve_fn(&s, Some("foo"), None, None).unwrap();
    assert!(matches!(r.symbol, SymbolRef::Function { ref name, .. } if name == "foo"));
}

#[test]
fn resolves_location_to_enclosing() {
    let s = session(&[("a.py", "def foo():\n    x = 1\n    return x\n")]);
    let r = seed::resolve_fn(&s, None, None, Some("a.py:2")).unwrap();
    assert!(matches!(r.symbol, SymbolRef::Function { ref name, .. } if name == "foo"));
}

#[test]
fn location_resolves_to_innermost_nested_function() {
    // R2 major: the innermost-enclosing property of line_range_index (spec §16/R2-M3).
    // Line 3 is inside `inner`, which is nested in `outer`.
    let s = session(&[("a.py", "def outer():\n    def inner():\n        return 1\n    return inner()\n")]);
    let r = seed::resolve_fn(&s, None, None, Some("a.py:3")).unwrap();
    assert!(matches!(r.symbol, SymbolRef::Function { ref name, .. } if name == "inner"),
        "expected innermost `inner`, got {:?}", r.symbol);
}

#[test]
fn ambiguous_symbol_errors_with_candidates() {
    // Naming-robust: two TOP-LEVEL `dup` functions in two files (m11).
    let s = session(&[("a.py", "def dup():\n    return 1\n"), ("b.py", "def dup():\n    return 2\n")]);
    match seed::resolve_fn(&s, Some("dup"), None, None) {
        Err(QueryError::AmbiguousSymbol { candidates }) => assert_eq!(candidates.len(), 2),
        other => panic!("expected AmbiguousSymbol, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test navigation_seed` → FAIL (`seed` module missing).

- [ ] **Step 3: Implement `navigation/seed.rs`**

```rust
// src/navigation/seed.rs
use crate::cpg::CpgNode;
use crate::navigation::types::{QueryError, SymbolRef};
use crate::navigation::NavigationSession;
use petgraph::graph::NodeIndex;

pub struct ResolvedFn { pub idx: NodeIndex, pub symbol: SymbolRef }

pub(crate) fn fn_symbol(s: &NavigationSession, idx: NodeIndex) -> Option<SymbolRef> {
    match s.index.cpg.node(idx) {
        CpgNode::Function { name, file, start_line, end_line } => Some(SymbolRef::Function {
            file: file.clone(), name: name.clone(), start_line: *start_line, end_line: *end_line, ordinal: 0 }),
        _ => None,
    }
}

/// Resolve a seed to exactly one function node. Precedence: location > symbol.
pub fn resolve_fn(s: &NavigationSession, symbol: Option<&str>, file: Option<&str>, location: Option<&str>)
    -> Result<ResolvedFn, QueryError>
{
    if let Some(loc) = location {
        let (f, line) = loc.rsplit_once(':')
            .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
            .ok_or_else(|| QueryError::SymbolNotFound { seed: format!("loc:{loc}") })?;
        let (idx, _) = s.index.enclosing_function(&f, line)
            .ok_or(QueryError::LocationOutOfRange { file: f.clone(), line })?;
        let symbol = fn_symbol(s, idx).ok_or(QueryError::LocationOutOfRange { file: f, line })?;
        return Ok(ResolvedFn { idx, symbol });
    }
    let name = symbol.ok_or_else(|| QueryError::SymbolNotFound { seed: "<empty>".into() })?;
    let mut hits: Vec<NodeIndex> = s.index.name_index.iter()
        .filter(|((f, n), _)| n == name && file.map_or(true, |ff| ff == f))
        .flat_map(|(_, v)| v.iter().copied()).collect();
    hits.sort_by_key(|i| i.index());
    hits.dedup();
    match hits.len() {
        0 => Err(QueryError::SymbolNotFound { seed: format!("symbol:{name}") }),
        1 => Ok(ResolvedFn { idx: hits[0], symbol: fn_symbol(s, hits[0])
                 .ok_or(QueryError::SymbolNotFound { seed: format!("symbol:{name}") })? }),
        _ => Err(QueryError::AmbiguousSymbol { candidates: hits.iter().filter_map(|&i| fn_symbol(s, i)).collect() }),
    }
}
```
Add `pub mod seed;` to `src/navigation/mod.rs`. `QueryError` must derive `Debug` (Plan 1 already derives it).

- [ ] **Step 4: Run to verify it passes** — `cargo test --test navigation_seed` → PASS.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(nav): shared seed resolution (symbol/location, ambiguity)"`

---

## Task 2: `callers` query (qualifier-aware, BFS)

**Files:** Modify `src/navigation/queries.rs`; create `tests/navigation/callers_test.rs`.

- [ ] **Step 1: Register the target, then write the failing test**

`Cargo.toml`: `[[test]] name = "navigation_callers"` / `path = "tests/navigation/callers_test.rs"`.
```rust
// tests/navigation/callers_test.rs
use prism::navigation::types::{Reason, SymbolRef};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files { std::fs::write(dir.path().join(name), src).unwrap(); }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn callers_reports_caller_and_call_site_line() {
    let s = session(&[("a.py", "def target():\n    return 1\n\ndef caller():\n    return target()\n")]);
    let ev = queries::callers(&s, Some("target"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| matches!(&i.symbol,
        Some(SymbolRef::Function { name, .. }) if name == "caller")));
    assert!(ev.items.iter().any(|i| i.why.iter().any(|r|
        matches!(r, Reason::CalledBy { caller, call_site_line } if caller == "caller" && *call_site_line == 5))));
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test navigation_callers` → FAIL.

- [ ] **Step 3: Implement `callers` (+ a shared per-hop helper)**

```rust
// add to src/navigation/queries.rs
use crate::call_graph::FunctionId;
use crate::navigation::seed;

fn fid_of(sym: &SymbolRef) -> FunctionId {
    match sym {
        // `..` — SymbolRef::Function also has `ordinal` (R2-B1).
        SymbolRef::Function { name, file, start_line, end_line, .. } =>
            FunctionId { name: name.clone(), file: file.clone(), start_line: *start_line, end_line: *end_line },
        _ => unreachable!("seed resolves to a Function"),
    }
}

/// Direct callers of `target` (qualifier-aware identity filter): callers index is keyed by name,
/// so each candidate CallSite is resolved from its caller's file and kept only if it reaches THIS target.
fn direct_callers(s: &NavigationSession, target: &FunctionId) -> Vec<(FunctionId, usize)> {
    let mut out = Vec::new();
    if let Some(sites) = s.index.cpg.call_graph.callers.get(&target.name) {
        for site in sites {
            let resolved = s.index.cpg.call_graph
                .resolve_callees_qualified(&site.callee_name, &site.caller.file, site.qualifier.as_deref());
            if resolved.iter().any(|f| **f == *target) {
                out.push((site.caller.clone(), site.line));
            }
        }
    }
    out
}

pub fn callers(s: &NavigationSession, symbol: Option<&str>, file: Option<&str>, location: Option<&str>, depth: usize)
    -> Result<Evidence, QueryError>
{
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let target = fid_of(&resolved.symbol);
    let query = format!("callers:{}@{}", target.name, target.file);   // @file identity (R3-M4)
    let mut items = Vec::new();
    let mut visited: std::collections::BTreeSet<FunctionId> = std::collections::BTreeSet::new();
    visited.insert(target.clone());
    let mut frontier = vec![target];
    for hop in 0..depth.max(1) {   // hop 0 = direct hit → score 1.0 (R3-M2); depth=2 → hops 0,1
        let mut next = Vec::new();
        for fid in &frontier {
            for (caller, line) in direct_callers(s, fid) {
                // One item PER CALL SITE (m7 symmetry with callees); `visited` only gates BFS recursion.
                items.push(EvidenceItem {
                    symbol: Some(SymbolRef::Function { file: caller.file.clone(), name: caller.name.clone(),
                        start_line: caller.start_line, end_line: caller.end_line, ordinal: 0 }),
                    location: Location { file: caller.file.clone(), start_line: caller.start_line, end_line: caller.end_line },
                    score: 1.0 / (1.0 + hop as f32), source: Source::PrismCpg, fallback: false,
                    why: vec![Reason::CalledBy { caller: caller.name.clone(), call_site_line: line }], snippet: None });
                if visited.insert(caller.clone()) { next.push(caller); }
            }
        }
        frontier = next;
    }
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap()
        .then(a.location.file.cmp(&b.location.file)).then(a.location.start_line.cmp(&b.location.start_line)));
    Ok(Evidence { query, items, truncated: false, warnings: vec![] })
}
```
> `FunctionId` must be `Ord` for the `BTreeSet` (confirm; it derives `Eq`/`Ord` in `call_graph.rs` — if not, key the set on `(file,name,start_line)`). Confirm `resolve_callees_qualified` returns `Vec<&FunctionId>` (adjust `**f`).

- [ ] **Step 4: Run to verify it passes** — `cargo test --test navigation_callers` → PASS.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(nav): callers query (qualifier-aware, bounded BFS)"`

---

## Task 3: `callees` query (qualifier-aware, BFS) + qualified-call coverage

**Files:** Modify `src/navigation/queries.rs`; create `tests/navigation/callees_test.rs`.

- [ ] **Step 1: Register the target, then write the failing tests (incl. the qualified path, M8)**

`Cargo.toml`: `[[test]] name = "navigation_callees"` / `path = "tests/navigation/callees_test.rs"`.
```rust
// tests/navigation/callees_test.rs
use prism::navigation::types::Reason;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files { std::fs::write(dir.path().join(name), src).unwrap(); }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn callees_reports_callee_and_line() {
    let s = session(&[("a.py", "def helper():\n    return 1\n\ndef caller():\n    return helper()\n")]);
    let ev = queries::callees(&s, Some("caller"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| i.why.iter().any(|r|
        matches!(r, Reason::Calls { callee, call_site_line, .. } if callee == "helper" && *call_site_line == 5))));
}

#[test]
fn callees_resolves_qualified_import() {
    // qualified call `util.helper()` — exercises resolve_callees_qualified (B2/M8).
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        ("main.py", "import util\n\ndef run():\n    return util.helper()\n"),
    ]);
    let ev = queries::callees(&s, Some("run"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| i.why.iter().any(|r|
        matches!(r, Reason::Calls { callee, qualifier, .. } if callee == "helper" && qualifier.as_deref() == Some("util")))));
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test navigation_callees` → FAIL.

- [ ] **Step 3: Implement `callees`**

```rust
// add to src/navigation/queries.rs
fn direct_callees(s: &NavigationSession, caller: &FunctionId) -> Vec<(Option<FunctionId>, String, usize, Option<String>)> {
    let mut out = Vec::new();
    if let Some(sites) = s.index.cpg.call_graph.calls.get(caller) {
        for site in sites {
            // CORRECT arg order (B2): (callee_name, caller_file, qualifier)
            let resolved = s.index.cpg.call_graph
                .resolve_callees_qualified(&site.callee_name, &site.caller.file, site.qualifier.as_deref());
            // resolve_callees_qualified returns Vec<&FunctionId>, so first() is Option<&&FunctionId> (R2-B2).
            let def = resolved.first().map(|f| (**f).clone());
            out.push((def, site.callee_name.clone(), site.line, site.qualifier.clone()));
        }
    }
    out
}

pub fn callees(s: &NavigationSession, symbol: Option<&str>, file: Option<&str>, location: Option<&str>, depth: usize)
    -> Result<Evidence, QueryError>
{
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let seed_fid = fid_of(&resolved.symbol);
    let query = format!("callees:{}@{}", seed_fid.name, seed_fid.file);   // @file identity (R3-M4)
    let mut items = Vec::new();
    let mut visited: std::collections::BTreeSet<FunctionId> = std::collections::BTreeSet::new();
    visited.insert(seed_fid.clone());
    let mut frontier = vec![seed_fid];
    for hop in 0..depth.max(1) {   // hop 0 = direct hit → score 1.0 (R3-M2); depth=2 → hops 0,1
        let mut next = Vec::new();
        for fid in &frontier {
            for (def, callee_name, line, qualifier) in direct_callees(s, fid) {
                let (sym, loc) = match &def {
                    Some(d) => (Some(SymbolRef::Function { file: d.file.clone(), name: d.name.clone(),
                                  start_line: d.start_line, end_line: d.end_line, ordinal: 0 }),
                                Location { file: d.file.clone(), start_line: d.start_line, end_line: d.end_line }),
                    None => (None, Location { file: fid.file.clone(), start_line: line, end_line: line }),
                };
                items.push(EvidenceItem { symbol: sym, location: loc, score: 1.0 / (1.0 + hop as f32),
                    source: Source::PrismCpg, fallback: false,
                    why: vec![Reason::Calls { callee: callee_name, call_site_line: line, qualifier }], snippet: None });
                if let Some(d) = def { if visited.insert(d.clone()) { next.push(d); } }
            }
        }
        frontier = next;
    }
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap()
        .then(a.location.file.cmp(&b.location.file)).then(a.location.start_line.cmp(&b.location.start_line)));
    Ok(Evidence { query, items, truncated: false, warnings: vec![] })
}
```
> If `resolve_callees_qualified` returns `Vec<FunctionId>` (owned) rather than `Vec<&FunctionId>`, drop the `(*f).clone()` deref accordingly.

- [ ] **Step 4: Run to verify it passes** — `cargo test --test navigation_callees` → PASS (both, incl. the qualified path).
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(nav): callees query (qualifier-aware, bounded BFS)"`

---

## Task 4: `ego-graph` query

**Files:** Modify `src/navigation/types.rs`, `src/navigation/queries.rs`; create `tests/navigation/ego_test.rs`.

- [ ] **Step 1: Add ego types, register target, write failing test**

Add to `src/navigation/types.rs`:
```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EgoNode { pub symbol: SymbolRef, pub location: Location }
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EgoEdge { pub from: usize, pub to: usize, pub kind: String }
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EgoGraph { pub query: String, pub nodes: Vec<EgoNode>, pub edges: Vec<EgoEdge>, pub warnings: Vec<Warning> }
```
`Cargo.toml`: `[[test]] name = "navigation_ego"` / `path = "tests/navigation/ego_test.rs"`.
```rust
// tests/navigation/ego_test.rs
use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(src: &str) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), src).unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn ego_includes_seed_and_call_edge() {
    let s = session("def helper():\n    return 1\n\ndef caller():\n    return helper()\n");
    let g = queries::ego_graph(&s, Some("caller"), None, None, 1, &["Call"]).unwrap();
    assert!(g.nodes.iter().any(|n| matches!(&n.symbol, SymbolRef::Function { name, .. } if name == "caller")));
    assert!(!g.edges.is_empty(), "expected at least one Call edge from the seed");
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test navigation_ego` → FAIL.

- [ ] **Step 3: Implement `ego_graph`** (extend the existing `CpgNode` import — m13)

In `queries.rs`, change the existing `use crate::cpg::CpgNode;` to `use crate::cpg::{CpgEdge, CpgNode};` and add:
```rust
use crate::navigation::types::{EgoEdge, EgoGraph, EgoNode};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{BTreeMap, VecDeque};

fn edge_kind(e: &CpgEdge) -> &'static str {
    match e { CpgEdge::DataFlow => "DataFlow", CpgEdge::ControlFlow => "ControlFlow", CpgEdge::Call => "Call",
        CpgEdge::Return => "Return", CpgEdge::Contains => "Contains", CpgEdge::FieldOf => "FieldOf" }
}

fn node_symbol_loc(s: &NavigationSession, ni: NodeIndex) -> (SymbolRef, Location) {
    match s.index.cpg.node(ni) {
        CpgNode::Function { name, file, start_line, end_line } => (
            SymbolRef::Function { file: file.clone(), name: name.clone(), start_line: *start_line, end_line: *end_line, ordinal: 0 },
            Location { file: file.clone(), start_line: *start_line, end_line: *end_line }),
        CpgNode::Variable { path, file, function, line, access } => (
            SymbolRef::Variable { file: file.clone(), function: function.clone(), line: *line,
                path: format!("{path:?}"), access: format!("{access:?}"), ordinal: 0 },
            Location { file: file.clone(), start_line: *line, end_line: *line }),
        CpgNode::Statement { file, line, kind } => (
            SymbolRef::Statement { file: file.clone(), line: *line, kind: format!("{kind:?}"), ordinal: 0 },
            Location { file: file.clone(), start_line: *line, end_line: *line }),
    }
}

pub fn ego_graph(s: &NavigationSession, symbol: Option<&str>, file: Option<&str>, location: Option<&str>,
                 hops: usize, edges: &[&str]) -> Result<EgoGraph, QueryError>
{
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let ego_fid = fid_of(&resolved.symbol);
    let query = format!("ego:{}@{}", ego_fid.name, ego_fid.file);   // ego symbol-seed form (R3-M4)
    let g = &s.index.cpg.graph;
    let mut order: BTreeMap<NodeIndex, usize> = BTreeMap::new();
    let mut nodes: Vec<EgoNode> = Vec::new();
    let mut ego_edges: Vec<EgoEdge> = Vec::new();
    // returns (index, is_new) so we only enqueue freshly-discovered nodes (m9).
    let mut intern = |s: &NavigationSession, ni: NodeIndex, order: &mut BTreeMap<NodeIndex, usize>, nodes: &mut Vec<EgoNode>| -> (usize, bool) {
        if let Some(&i) = order.get(&ni) { return (i, false); }
        let i = nodes.len();
        let (symbol, location) = node_symbol_loc(s, ni);
        nodes.push(EgoNode { symbol, location });
        order.insert(ni, i);
        (i, true)
    };
    intern(s, resolved.idx, &mut order, &mut nodes);
    let mut q = VecDeque::from([(resolved.idx, 0usize)]);
    while let Some((ni, d)) = q.pop_front() {
        if d >= hops { continue; }
        for dir in [Direction::Outgoing, Direction::Incoming] {
            for er in g.edges_directed(ni, dir) {
                if !edges.contains(&edge_kind(er.weight())) { continue; }
                let other = if er.source() == ni { er.target() } else { er.source() };
                let (from, _) = intern(s, ni, &mut order, &mut nodes);
                let (to, is_new) = intern(s, other, &mut order, &mut nodes);
                let (a, b) = if dir == Direction::Outgoing { (from, to) } else { (to, from) };
                ego_edges.push(EgoEdge { from: a, to: b, kind: edge_kind(er.weight()).into() });
                if is_new { q.push_back((other, d + 1)); }
            }
        }
    }
    ego_edges.sort_by(|x, y| (x.from, x.to, &x.kind).cmp(&(y.from, y.to, &y.kind)));
    ego_edges.dedup();
    Ok(EgoGraph { query, nodes, edges: ego_edges, warnings: vec![] })
}
```
> Note: `Contains` edges are function→variable only (no statement containment, spec R3-M3), so ego over `Contains` surfaces variables, not statements.

- [ ] **Step 4: Run to verify it passes** — `cargo test --test navigation_ego` → PASS.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(nav): bounded ego-graph over CPG edges"`

---

## Task 5: CLI wiring + shared error rendering

**Files:** Modify `src/main.rs`, `src/output/navigation.rs`; extend `tests/cli/nav_compat_test.rs`.

- [ ] **Step 1: Write failing dogfood + rejection tests (m12)**

```rust
// append to tests/cli/nav_compat_test.rs
#[test]
fn nav_callers_json_on_self() {
    let out = bin().args(["nav","callers","--repo",".","--symbol","build_scoped","--format","json"]).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["query"].as_str().unwrap().starts_with("callers:build_scoped"));
    assert!(v["items"].is_array());
}

#[test]
fn nav_subcommands_reject_review_diff_flag() {           // m12: one --diff rejection per subcommand
    for sub in ["callers", "callees", "ego"] {
        let out = bin().args(["nav", sub, "--repo", ".", "--symbol", "x", "--diff", "d.json"]).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "`nav {sub} --diff` must be a clap parse error");
    }
}

#[test]
fn nav_subcommands_reject_unknown_format() {              // m12: one --format rejection per subcommand
    for sub in ["callers", "callees", "ego"] {
        let out = bin().args(["nav", sub, "--repo", ".", "--symbol", "x", "--format", "jsn"]).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "`nav {sub} --format jsn` must be a clap parse error");
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test cli_nav_compat -- nav_callers_json_on_self nav_subcommands_reject_review_diff_flag nav_subcommands_reject_unknown_format nav_ambiguous_seed_exits_3` → FAIL (R3-B1: filter names must match the test fn names above).

- [ ] **Step 3: Add NavQuery variants (full fields — B3), shared err renderer (B4), dispatch**

In `src/main.rs`, extend `enum NavQuery` — **every field spelled out** (no `/* identical */`). For `Callers` and `Callees` use identical bodies:
```rust
Callers { #[arg(long)] repo: std::path::PathBuf, #[arg(long)] symbol: Option<String>,
          #[arg(long)] file: Option<String>, #[arg(long)] location: Option<String>,
          #[arg(long, default_value_t = 1)] depth: usize,
          #[arg(long, default_value = "text", value_parser = ["text","json"])] format: String },
Callees { #[arg(long)] repo: std::path::PathBuf, #[arg(long)] symbol: Option<String>,
          #[arg(long)] file: Option<String>, #[arg(long)] location: Option<String>,
          #[arg(long, default_value_t = 1)] depth: usize,
          #[arg(long, default_value = "text", value_parser = ["text","json"])] format: String },
Ego { #[arg(long)] repo: std::path::PathBuf, #[arg(long)] symbol: Option<String>,
      #[arg(long)] file: Option<String>, #[arg(long)] location: Option<String>,
      #[arg(long, default_value_t = 1)] hops: usize,
      #[arg(long, default_value = "Call,Return,DataFlow,Contains")] edges: String,
      #[arg(long, default_value = "text", value_parser = ["text","json"])] format: String },
```
> Match the `value_parser` form to Plan 1's `--format` fix (so `nav_..._rejects_unknown_format` exit-2 stays consistent).

In `src/output/navigation.rs`, **replace** the existing `use crate::navigation::types::Evidence;` (it's already imported — R3-#7) with the wider import, and add the fns:
```rust
use crate::navigation::types::{EgoGraph, Evidence, QueryError};

pub fn render_err(e: &QueryError) -> (String, i32) {
    (serde_json::to_string_pretty(&serde_json::json!({ "error": e })).unwrap_or_else(|_| "{}".into()), 3)
}
pub fn render_ego(g: &EgoGraph, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(g).unwrap_or_else(|_| "{}".into()),
        _ => {
            let mut s = format!("{}\n", g.query);
            for (i, n) in g.nodes.iter().enumerate() { s.push_str(&format!("  [{i}] {}:{}\n", n.location.file, n.location.start_line)); }
            for e in &g.edges { s.push_str(&format!("  {} --{}--> {}\n", e.from, e.kind, e.to)); }
            s
        }
    }
}
```
Also **extend the existing Plan 1 `render`** (used by callers/callees) to print each item's `why` reasons (caller/callee name + `call_site_line`) under the location line — otherwise `--format text` shows no call-site evidence (m8). `render_ego` already prints edges, so ego text is fine. This adds `why` lines to `nodes-at` text too (the `EnclosingFunction` reason), which has no byte-for-byte golden, so it's safe.

In `run_nav`, add arms. Each loads the repo + builds the index (same as `NodesAt`), runs the query, and routes `Ok`→`render`/`render_ego` (exit 0) or `Err(QueryError)`→`render_err` (print + `std::process::exit(3)`):
```rust
NavQuery::Callers { repo, symbol, file, location, depth, format } => {
    let session = build_session(repo)?;                 // helper extracting the NodesAt load+build
    match prism::navigation::queries::callers(&session, symbol.as_deref(), file.as_deref(), location.as_deref(), *depth) {
        Ok(ev) => { println!("{}", prism::output::navigation::render(&ev, format)); Ok(()) }
        Err(e) => { let (s, code) = prism::output::navigation::render_err(&e); println!("{s}"); std::process::exit(code); }
    }
}
// Callees: identical with `callees(...)`. Ego: parse `edges.split(',')` -> Vec<&str>, call `ego_graph(...)`, render_ego / render_err.
```
Refactor the `NodesAt` load+build into a `fn build_session(repo: &std::path::Path) -> anyhow::Result<NavigationSession>` so all four arms share it (DRY).

- [ ] **Step 4: Run to verify it passes** — `cargo test --test cli_nav_compat` → all PASS (incl. unchanged Plan 1 byte-for-byte goldens).
- [ ] **Step 5: Manual dogfood** — `cargo run -- nav callees --repo . --symbol run_slicing --format json`; `cargo run -- nav ego --repo . --symbol build_scoped --hops 1 --format json`.
- [ ] **Step 6: Commit** — `git add -A && git commit -m "feat(nav): prism nav callers/callees/ego CLI + shared error envelope"`

---

## Task 6: Tool-generated goldens on a real call-graph fixture (M8) + doc-sync

**Files:** Create `tests/fixtures/nav_callgraph/` (multi-file, with a qualified import); add goldens under `tests/fixtures/nav_compat/golden/`; extend `tests/cli/nav_compat_test.rs`; touch `roadmap.md`/`CLAUDE.md`.

> Do **not** reuse `tests/fixtures/nav_compat/a.py` — it has one function and zero call sites (R2 major: callees/empty goldens would collapse), and it drives Plan 1's byte-for-byte goldens.

- [ ] **Step 1: Create the fixture**
```bash
mkdir -p tests/fixtures/nav_callgraph
printf 'def helper():\n    return 1\n' > tests/fixtures/nav_callgraph/util.py
printf 'import util\n\ndef run():\n    return util.helper()\n\ndef lonely():\n    return 0\n' > tests/fixtures/nav_callgraph/main.py
```

- [ ] **Step 2: Write failing golden + empty-case tests** (`assert_eq` vs not-yet-captured goldens):
```rust
// append to tests/cli/nav_compat_test.rs
const CG: &str = "tests/fixtures/nav_callgraph";
#[test]
fn callees_golden_qualified() {
    let out = bin().args(["nav","callees","--repo",CG,"--symbol","run","--format","json"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("callees_run.json"));
}
#[test]
fn ego_golden() {
    let out = bin().args(["nav","ego","--repo",CG,"--symbol","run","--hops","1","--edges","Call","--format","json"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("ego_run.json"));
}
#[test]
fn callees_empty_for_leaf() {
    // `lonely` calls nothing -> the genuine empty-result case (NOT the same artifact as a callee golden).
    let out = bin().args(["nav","callees","--repo",CG,"--symbol","lonely","--format","json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["items"].as_array().unwrap().is_empty());
    assert_eq!(v["query"], "callees:lonely");
}
```

- [ ] **Step 3: Run to verify it fails** — `cargo test --test cli_nav_compat -- callees_golden_qualified ego_golden` → FAIL (missing goldens).

- [ ] **Step 4: Generate goldens from the live tool** (never hand-write — Plan 1 R3-M1):
```bash
cargo run -q -- nav callees --repo tests/fixtures/nav_callgraph --symbol run --format json > tests/fixtures/nav_compat/golden/callees_run.json
cargo run -q -- nav ego --repo tests/fixtures/nav_callgraph --symbol run --hops 1 --edges Call --format json > tests/fixtures/nav_compat/golden/ego_run.json
```
Confirm each is deterministic (run 3×, identical) before committing — drop any that aren't (as the `review` aggregate was dropped in Plan 1).

- [ ] **Step 5: Run to verify it passes** — `cargo test --test cli_nav_compat` → all PASS.

- [ ] **Step 6: Doc-sync + commit** — in `docs/archive/review-artifacts/prism-query-layer/roadmap.md`, mark Plan 1 review follow-up #3 (`QueryError` plumbing) **closed by Plan 2**; add a one-line note of the three new subcommands to `LLM.md`. Then `git add -A && git commit -m "test(nav): callees/ego goldens on a real call-graph fixture; doc-sync"`.

---

## Round 3 disposition (close before executing)

**Fixed inline (above):** R3-B1 Step-2 filter names; R3-#7 import is now a *replace*; R3-M2 hop indexing `0..depth.max(1)` (direct hit = 1.0); R3-M4 `@file` on all three query strings.

**Required test additions** — add these to the relevant tasks:

- *(Task 2)* assert the direct-hit score and the multi-hop case (R3-M2, R3-M5):
```rust
#[test]
fn callers_direct_hit_scores_1_0_and_hop2_decays() {
    // A() called by B() called by C() — transitive (R3-M5).
    let s = session(&[("a.py", "def a():\n    return 1\n\ndef b():\n    return a()\n\ndef c():\n    return b()\n")]);
    let ev = queries::callers(&s, Some("a"), None, None, 2).unwrap();
    let b = ev.items.iter().find(|i| matches!(&i.symbol, Some(SymbolRef::Function{name,..}) if name=="b")).unwrap();
    assert_eq!(b.score, 1.0);                                 // direct caller
    let c = ev.items.iter().find(|i| matches!(&i.symbol, Some(SymbolRef::Function{name,..}) if name=="c")).unwrap();
    assert_eq!(c.score, 0.5);                                 // hop-2 caller
}
```
- *(Task 5)* assert the error envelope is exit 3 + parseable JSON (R3-M6):
```rust
#[test]
fn nav_ambiguous_seed_exits_3() {
    // a tiny ambiguous repo: `dup` in two files, passed via a temp dir written by the test.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def dup():\n    return 1\n").unwrap();
    std::fs::write(dir.path().join("b.py"), "def dup():\n    return 2\n").unwrap();
    let out = bin().args(["nav","callers","--repo",dir.path().to_str().unwrap(),"--symbol","dup","--format","json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["error"]["AmbiguousSymbol"].is_object());
}
```

**Accepted deviations (documented, not silent):**
- **R3-M3 ego shape → deferred to Plan 3.** Plan 2 ships the flat `EgoGraph { query, nodes, edges, warnings }` (functional). Plan 3, when the MCP adapter standardizes on the Evidence JSON envelope (spec §13), aligns ego to `{query, items, truncated, warnings, graph:{nodes,edges}}`, adds `reason` to edges, maps `Contains`→`"ContainsVariable"`, and adds `--direction {Out,In,Both}`. Recorded in the roadmap follow-ups.
- **R3-M9 text-`render` format:** print `why` as indented lines — `    calls {callee} @ {file}:{line}` / `    called by {caller} @ line {n}` / `    in {fn}`; add one text assertion. Implementer may tune wording (no golden binds it).
- **R3-M8 doc-sync:** Task 6 updates `roadmap.md` (#3 closed; + the M3 ego-alignment follow-up) and `LLM.md` (3 subcommands). **Do not** touch CLAUDE.md's array count — contested and orthogonal to Plan 2; the new nav tests aren't `fn test_*` so the coverage matrix doesn't scan them.
- **R3-m10/m11 (static-fn + non-Python qualified fixture; `--edges` typo warning):** deferred to a Plan 2.1 coverage pass; noted in the roadmap.

## Done / handoff

After Plan 2: `prism nav {nodes-at,callers,callees,ego}` all run — qualifier-aware, multi-hop, call-site-evidenced, with a uniform `QueryError`→exit(3) envelope (closes roadmap follow-up #3). **Plan 3:** exact-hit nav cache (`build.rs` grammar fingerprint), `module-deps`/`repo-map` (labeled, call-derived), `rmcp` MCP adapter.

## Resolved by round 2 (no longer open)
- `resolve_callees_qualified` returns `Vec<&FunctionId>` (`call_graph.rs:654`) — handled via `(**f).clone()`.
- `FunctionId` derives `Ord`/`Eq` (`call_graph.rs:11-18`) — the BFS `BTreeSet` is fine.

## Deferred (documented, not silently dropped)
- **`--max-results`/`truncated`** (spec §8): not exposed in Plan 2; all three queries set `truncated:false` and ego over default edges is unbounded. Deferred to Plan 3 (add `--max-results` + an ego node/edge cap); don't advertise the flag until then.
- **Nonexistent seed symbol** → `QueryError::SymbolNotFound` → exit 3 (not spec §8's empty+warning, which is internally inconsistent for callers). Intentional for a *seed* (bad input); lock it in the compat matrix.
- **`render_err` is always JSON** even under `--format text` (intentional — machine-readable envelope).
- **`IndirectCallApprox`/`UnresolvedModule`** stay unemitted by these queries (no spec golden requires them); revisit with `module-deps` in Plan 3.

## Match Plan 1 at execution
- `value_parser` array form must match Plan 1's `--format` fix so the `nav_..._rejects_unknown_format` exit-2 contract stays consistent.
- Task 4: `let intern` (drop `mut` unless the borrow checker demands `FnMut`).
