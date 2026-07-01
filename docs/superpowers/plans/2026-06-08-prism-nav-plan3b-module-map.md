# Prism Navigation Layer — Plan 3b: Module/Repo Map + Ego Envelope Alignment

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `module-deps`/`repo-map` navigation queries (call-derived file→file dependency edges + labeled unresolved imports) and align `ego-graph` to the standard `Evidence` envelope, completing build-order steps 7 and 10 of the Tier-1 spec (steps 8–9 shipped in Plan 2 / Plan 3a).

**Architecture:** Purely additive over the existing nav layer (Option C — zero CPG-core edits). A shared `graph: Option<GraphPayload>` field on `Evidence` carries node/edge payloads for the two graph-shaped queries (`ego`, `repo-map`); `serde(skip_serializing_if = "Option::is_none")` keeps the existing `nodes-at`/`callers`/`callees` JSON byte-identical. A new `src/navigation/module_graph.rs` derives file→file edges by resolving each `CallSite` through the existing `CallGraph::resolve_callees_qualified`, and labels extracted-but-unresolved imports (`source: HeuristicImport`) per the §10 per-language precision tiers. The diff-review compat goldens are untouched; only the nav-owned `ego` golden is re-baselined (intentional re-shape).

**Tech Stack:** Rust, `serde`/`serde_json`, `petgraph`, `clap`, the existing `prism::navigation` + `prism::call_graph` modules.

**Spec:** `docs/superpowers/specs/2026-06-07-prism-navigation-layer-design.md` §5 (skipped-file contract), §8 (Evidence/graph envelope), §10 (module/repo map + precision tiers), §12 (CLI seam), §16 (named goldens), §17 (build order steps 7, 10). **Roadmap:** `docs/archive/review-artifacts/prism-query-layer/roadmap.md` (Plan 3 scope, minus the MCP adapter which is Plan 3c).

---

## v2 — plan-review disposition (round 1: codex + claude, in-depth vs spec)

Round-1 verdict was "needs rework"; every finding is dispositioned here and folded into the tasks below:

- **BLOCKER 1 (Task 1 commits non-compiling tree)** → FIXED. Task 1 is now **purely additive** (types + field + `graph: None` on existing literals). The `EgoNode/EgoEdge/EgoGraph` deletion + all consumer migration moved into **Task 2** as one green commit.
- **BLOCKER 2 (missed `Evidence` literal in `types_test.rs`)** → FIXED. Task 1 adds `graph: None` to the existing `evidence_serializes_to_expected_shape` literal.
- **BLOCKER 3 (unregistered test target)** → FIXED. The `[[test]] navigation_module_graph` entry is added in **Task 3** (when the file is created), not Task 6.
- **BLOCKER 4 (`mod common` won't compile)** → FIXED. Task 3 uses a **file-local `session(files)` helper** copied verbatim from `tests/navigation/callees_test.rs:6-14` — no `mod common`, no `tests/common` change.
- **BLOCKER 5 (`repo_map` imported before it exists)** → FIXED. Task 3 imports only `module_deps`; Task 4 adds the `repo_map` import.
- **MAJOR 6 (§5 skipped-file hard error)** → FIXED. `module_deps` now returns `Evidence` (not `Result`) and mirrors `nodes_at`: skipped/unknown files → empty `items` + `SkippedPath` warning, never exit-3.
- **MAJOR 7 (`repo_map` edge-induced)** → FIXED. `repo_map` nodes = **all** indexed `s.repo.files` (isolated files included); edges = resolved cross-file calls.
- **MAJOR 8 (import-count unit mismatch)** → FIXED. Both `module_deps` and `repo_map` count **distinct modules** and use the same warning wording.
- **MAJOR 9 (vacuous dogfood test)** → FIXED. Task 5 asserts `!items.is_empty()` + all-`PrismCpg` on a high-fan-out file.
- **MINOR 10 (Rust call-derived-only tier)** → CONFIRMED "no items, no warning" (reviewers resolved: §10 prose + ILLUSTRATIVE-ONLY golden support this; per-import `UnresolvedImport` is unimplementable for Rust without new extraction). A distinct *precision-tier* warning is logged to **Deferred** (optional, non-blocking).
- **MINOR 11 (ego-test migration mis-specified)** → FIXED. Task 2 Step 1 preserves the existing `.any()`/`!is_empty()` predicates, reads from `ev.graph`, and wraps each `n.symbol` pattern in `Some(..)` (`GraphNode.symbol` is `Option<SymbolRef>`).
- **MINOR 12 (invalid test commands)** → FIXED. Commands use target+prefix filters (`cargo test --test navigation_types evidence_`); no `--nocheck`.
- **MINOR 13 (same-stem over-reporting)** → FIXED. A code comment documents the known `resolve_callees_qualified` same-stem behavior.
- **MINOR 14 (incoherent Task 3 commit list)** → FIXED. With the file-local helper, Task 3 commits only `module_graph.rs`, `mod.rs`, `module_graph_test.rs`, `Cargo.toml`.

## v3 — plan-review disposition (round 2: coverage lens; executability lens unavailable — findings empirically verified against the live tool)

Round-2 verdict was "needs-rework, narrowly." Two findings turned on a real CPG limitation I verified end-to-end with the live `prism nav callees` query:

> **Verified fact (load-bearing):** Rust **scoped** calls — `module::func()`, `Type::method()` — do **not** resolve cross-file. `call_function_name` returns the literal `"original_diff::slice"` with `qualifier=None`; `resolve_callees_qualified` does `functions.get("original_diff::slice")` → miss → unresolved. Empirically, `callees run_slicing_inner@src/algorithms/mod.rs` = **41 items, 0 cross-file resolved** (all scoped). Only **unqualified / `use`-imported** Rust calls resolve cross-file by name: `use util::helper; helper()` → resolves to `util.rs` (verified). Python qualified calls (`util.helper()`) resolve via the import map (the `CG` fixture).

- **BLOCKER 1 (dogfood `!items.is_empty()` on `src/algorithms/mod.rs` fails — 0 edges)** → FIXED. The live-repo dogfood is now a **pure smoke** (`status.success()` + valid JSON + `query` + `graph` shape; **no** non-empty assertion). The non-vacuous cross-file-edge guarantees move to **byte goldens on the `CG` Python fixture** (`module_deps_run.json`, `repo_map_run.json`) where resolution is stable.
- **MAJOR 2 (Rust unit test vacuous — `util::helper()` is scoped, resolves to nothing)** → FIXED. The Rust test now uses `use util::helper;` + **unqualified** `helper()` and asserts ≥1 `PrismCpg` item → `util.rs` (verified non-vacuous). The scoped-call limitation is documented in Design-decision #4 and the §10 Deferred amendment.
- **MINOR 3 (coverage-matrix step is inert)** → FIXED. Verified: `coverage_test.rs` has **zero** `tests/navigation` entries and its scanners key on the `fn test_` prefix; `module_deps_*`/`repo_map_*` are not algorithms and lack that prefix, so the step adds no cells. **Dropped** from Task 5. Only the Task 3 `[[test]]` registration matters.
- **MINOR 4 (§16 goldens structural-only)** → FIXED. Task 5 now generates **byte goldens** `module_deps_run.json` + `repo_map_run.json` on the `CG` fixture (mirroring `ego_run.json`/`callees_run.json`), so these two queries have canonical golden artifacts, not just structural asserts.

Round-1 fixes the coverage lens re-confirmed as holding: Task 1 compiles green (4 `queries.rs` literals + `types_test.rs` patched; `EgoGraph` migrates only in Task 2); `repo_map` indexing clean (nodes over `s.repo.files`, edges `filter_map` against `order`, self-edges excluded, deterministic); Python happy-path correct; compat-safe (`skip_serializing_if` → only `ego_run.json` re-shapes); `cache_test.rs` needs no edit.

---

## Design decisions (locked)

These resolve ambiguities the spec leaves to the implementation (§8 goldens are "ILLUSTRATIVE ONLY"; canonical goldens are generated from the live tool):

1. **`graph` is an optional `Evidence` field, not a separate type.** `ego`/`repo-map` return `Evidence { items: [], graph: Some(..) }`; the flat queries return `graph: None`. `#[serde(skip_serializing_if = "Option::is_none")]` → the flat queries serialize **identically** (no `"graph"` key); only the `ego` golden changes. serde treats `Option<SymbolRef>` transparently, so individual node JSON is unchanged.
2. **`module-deps:<file>` returns flat `items`** (matches §8), one per **distinct target file** reached by a resolved cross-file call, `why` = deduped sorted `Calls` reasons. Self-file edges excluded. `source: PrismCpg`, `location = {target_file, 1, 1}`, `symbol: None`. It **never errors**: a skipped/unknown file yields empty `items` + a `SkippedPath` warning (§5), exactly like `nodes_at`.
3. **`repo-map` returns a `graph`** over the **whole repo**: one file node per indexed `s.repo.files` key (isolated files included), `symbol: None`; edges are distinct `ModuleDep` file→file pairs from resolved cross-file calls. `items: []`.
4. **Import labeling (precision tiers, §10).** v1 never filesystem-resolves imports. For files whose language *extracts* imports (Python/JS/TS/TSX/Go — present in `CallGraph::imports`), each **distinct module** is surfaced as an extra `module-deps` item (`source: HeuristicImport`, `why: [UnresolvedImport{module}]`) + one `UnresolvedModule` warning. For call-derived-only languages (Rust/Java/C/C++ — `extract_imports` returns empty) there are **no import items and no warning** (confirmed by round-1 review); the degraded behavior is **encoded by an explicit Rust test** (§16). `repo-map` carries one repo-wide `UnresolvedModule` warning counting distinct module imports.

   **Rust call-resolution caveat (verified round-2):** v1 Rust call-derived edges cover only **unqualified / `use`-imported** calls (`use util::helper; helper()` → resolves by name). **Scoped** calls — `module::func()`, `Type::method()` — are unresolved today (`call_function_name` returns the literal `"module::func"`; no `scoped_identifier` handling). So `module-deps` on a Rust file dominated by scoped dispatch (e.g. `src/algorithms/mod.rs`) can legitimately be **empty**; tests must not assume a Rust file yields edges. Fuller Rust resolution (split scoped paths / resolve associated functions) is a tracked follow-up (§Deferred).
5. **`GraphEdge` stays `{from,to,kind}`** in v1 (no per-edge `reason`; the §8 ego-edge `reason` is illustrative). The call evidence lives in the richer `module-deps` `why`. → Deferred.
6. **Determinism:** `CallGraph::calls` is `BTreeMap<FunctionId, BTreeSet<CallSite>>`; all derivation iterates `BTreeMap`/`BTreeSet`; items are finally sorted `(score desc, file, start_line, source-rank)`. `resolve_callees_qualified` may return multiple defs (documented same-stem behavior, §MINOR-13) — that is deterministic over-reporting, not nondeterminism.

---

## File Structure

**Create:**
- `src/navigation/module_graph.rs` — `module_deps(&NavigationSession, file) -> Evidence` and `repo_map(&NavigationSession) -> Evidence`. (< 200 lines.)
- `tests/navigation/module_graph_test.rs` — library tests with a **file-local `session(files)` helper** (copied from `callees_test.rs`): Python cross-file edge + import labeling; Rust call-derived-only (edges present, **zero** import items, **no** warning); skipped/unknown file (empty + `SkippedPath`); `repo_map` whole-repo nodes + `ModuleDep` edge.

**Modify (additive only — no CPG-core logic edits):**
- `src/navigation/types.rs` — add `GraphNode`/`GraphEdge`/`GraphPayload`; add `graph: Option<GraphPayload>` to `Evidence` (Task 1). Delete `EgoNode`/`EgoEdge`/`EgoGraph` (Task 2).
- `src/navigation/queries.rs` — `nodes_at`/`callers`/`callees` literals add `graph: None` (Task 1); `ego_graph` returns `Evidence` with `graph: Some(..)` (Task 2).
- `src/navigation/mod.rs` — `pub mod module_graph;` (Task 3).
- `src/output/navigation.rs` — `render` prints `ev.graph`; delete `render_ego` (Task 2).
- `src/main.rs` — route `ego` through `render`/`render_err` (Task 2); add `NavQuery::{ModuleDeps, RepoMap}` (Task 5).
- `tests/navigation/types_test.rs` — `graph: None` on the existing literal + two new tests (Task 1).
- `tests/navigation/ego_test.rs` — migrate to the `Evidence` envelope (Task 2).
- `tests/cli/nav_compat_test.rs` — add `module_deps_golden` + `repo_map_golden` (CG fixture) + `module_deps_repo_map_live_smoke` (`--repo .`) (Task 5).
- `tests/fixtures/nav_compat/golden/ego_run.json` — regenerate (Task 2).
- `tests/fixtures/nav_compat/golden/module_deps_run.json`, `repo_map_run.json` — new byte goldens on the CG fixture (Task 5).
- `Cargo.toml` — `[[test]] navigation_module_graph` (Task 3).

**Not modified:** `tests/integration/coverage_test.rs` — its scanners key on the `fn test_` prefix and list only algorithm/lang/integration files; nav-query tests add no matrix cells (round-2 MINOR 3).
- `CLAUDE.md` — doc-sync (Task 5).

---

## Task 1: `Evidence.graph` envelope + graph payload types (purely additive)

**Files:**
- Modify: `src/navigation/types.rs`, `src/navigation/queries.rs`, `tests/navigation/types_test.rs`

Task 1 does **not** touch `EgoNode/EgoEdge/EgoGraph`, `ego_graph`, or `render_ego` — those migrate in Task 2. The tree stays green.

- [ ] **Step 1: Write the failing tests**

In `tests/navigation/types_test.rs`, first add `graph: None,` as the final field of the **existing** `Evidence { .. }` literal in `evidence_serializes_to_expected_shape` (after `warnings: vec![],`). Then append:

```rust
#[test]
fn evidence_without_graph_omits_key() {
    let ev = Evidence {
        query: "nodes-at:a.rs:1".into(),
        items: vec![],
        truncated: false,
        warnings: vec![],
        graph: None,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(
        !json.contains("\"graph\""),
        "graph:None must be omitted so existing nav goldens stay byte-identical: {json}"
    );
}

#[test]
fn evidence_with_graph_serializes_payload() {
    let ev = Evidence {
        query: "repo-map".into(),
        items: vec![],
        truncated: false,
        warnings: vec![],
        graph: Some(GraphPayload {
            nodes: vec![GraphNode {
                symbol: None,
                location: Location { file: "a.rs".into(), start_line: 1, end_line: 1 },
            }],
            edges: vec![GraphEdge { from: 0, to: 0, kind: "ModuleDep".into() }],
        }),
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("\"graph\""));
    assert!(json.contains("\"ModuleDep\""));
}
```

(`types_test.rs` already has `use prism::navigation::types::*;`, so `GraphPayload`/`GraphNode`/`GraphEdge` resolve once defined.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_types evidence_`
Expected: FAIL to compile — `Evidence` has no field `graph`; `GraphPayload`/`GraphNode`/`GraphEdge` undefined.

- [ ] **Step 3: Add the types + field**

In `src/navigation/types.rs`, **add** (do not delete `EgoNode/EgoEdge/EgoGraph` yet):

```rust
/// A node in a graph-shaped result (`ego`, `repo-map`).
/// `symbol` is `None` for file-level nodes (repo-map); `Some` for symbol nodes (ego).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphNode {
    pub symbol: Option<SymbolRef>,
    pub location: Location,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphPayload {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
```

Change `Evidence` to add the field:

```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Evidence {
    pub query: String,
    pub items: Vec<EvidenceItem>,
    pub truncated: bool,
    pub warnings: Vec<Warning>,
    /// Present only for graph-shaped queries (`ego`, `repo-map`); omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<GraphPayload>,
}
```

- [ ] **Step 4: Add `graph: None` to the existing flat-query literals**

In `src/navigation/queries.rs`, add `graph: None,` as the final field to the **four** `Evidence { .. }` literals: the `nodes_at` SkippedPath early-return, the `nodes_at` success return, the `callers` `Ok(Evidence { .. })`, and the `callees` `Ok(Evidence { .. })`. (Leave `item_fn` — it builds `EvidenceItem`.)

- [ ] **Step 5: Run to verify pass**

Run: `cargo test --test navigation_types evidence_`
Expected: PASS (3 tests: the updated `evidence_serializes_to_expected_shape` + the two new ones). The whole crate still builds — `ego_graph`/`render_ego` are untouched and still use `EgoGraph`.

- [ ] **Step 6: Commit**

```bash
git add src/navigation/types.rs src/navigation/queries.rs tests/navigation/types_test.rs
git commit -m "feat(nav): add optional Evidence.graph payload (Plan 3b T1)"
```

---

## Task 2: Migrate `ego-graph` to the `Evidence` envelope (deletes `Ego*`)

**Files:**
- Modify: `src/navigation/queries.rs`, `src/navigation/types.rs` (delete `Ego*`), `src/output/navigation.rs`, `src/main.rs`, `tests/navigation/ego_test.rs`
- Regenerate: `tests/fixtures/nav_compat/golden/ego_run.json`

- [ ] **Step 1: Migrate the ego library test to the new envelope**

In `tests/navigation/ego_test.rs`, the tests bind `let g = queries::ego_graph(..).unwrap();` and read `g.nodes`/`g.edges` with `matches!(&n.symbol, SymbolRef::Function{..})`. Migrate **preserving each existing predicate** (`.any()`, `!is_empty()` — do **not** invent counts): rename `g` → `ev`, read from `ev.graph`, and wrap each `n.symbol` pattern in `Some(..)`. Concretely:

- After each `let ev = queries::ego_graph(..).unwrap();`, add `let g = ev.graph.as_ref().expect("ego returns Some(graph)");` and keep the original `g.nodes`/`g.edges` predicate bodies — but change every `matches!(&n.symbol, SymbolRef::Function { .. })` to `matches!(&n.symbol, Some(SymbolRef::Function { .. }))` and likewise for `SymbolRef::Variable`.
- The two error-path tests (`ego_edges_reject_unknown_name`) already `.unwrap_err()` on a `QueryError` — `ego_graph` still returns `Result`, so those are unchanged.
- Add one assertion in `ego_includes_seed_and_call_edge`: `assert!(ev.items.is_empty(), "ego carries its result in graph, not items");`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_ego`
Expected: FAIL to compile — `ego_graph` still returns `EgoGraph` (no `.graph`/`.items`); `n.symbol` is bare `SymbolRef`, not `Option`.

- [ ] **Step 3: Change `ego_graph` to return `Evidence`**

In `src/navigation/queries.rs`:
- **Delete** the `use crate::navigation::types::{EgoEdge, EgoGraph, EgoNode};` line (line 5). Do **not** add a replacement — `queries.rs:4` already has `use crate::navigation::types::*;`, which covers `GraphEdge`/`GraphNode`/`GraphPayload` (round-3 nit: avoid a redundant explicit `use`).
- Change `ego_graph`'s return type `Result<EgoGraph, QueryError>` → `Result<Evidence, QueryError>`.
- `let mut nodes: Vec<GraphNode> = Vec::new();` and `let mut ego_edges: Vec<GraphEdge> = Vec::new();`.
- In the `intern` closure, `nodes.push(GraphNode { symbol: Some(symbol), location });` (symbol is `Some` for ego).
- Where edges are pushed: `ego_edges.push(GraphEdge { from: a, to: b, kind: edge_kind(er.weight()).into() });`.
- Replace the final `Ok(EgoGraph { .. })` with:

```rust
    Ok(Evidence {
        query: seed.query,
        items: vec![],
        truncated: false,
        warnings: vec![],
        graph: Some(GraphPayload { nodes, edges: ego_edges }),
    })
```

In `src/navigation/types.rs`, now **delete** the `EgoNode`, `EgoEdge`, and `EgoGraph` structs.

- [ ] **Step 4: Update output rendering**

In `src/output/navigation.rs`:
- Change the import to `use crate::navigation::types::{Evidence, QueryError};` (drop `EgoGraph`).
- **Delete** `render_ego`.
- In `render`, after the `for w in &ev.warnings { .. }` loop in the text arm, add:

```rust
            if let Some(graph) = &ev.graph {
                for (i, n) in graph.nodes.iter().enumerate() {
                    s.push_str(&format!("  [{i}] {}:{}\n", n.location.file, n.location.start_line));
                }
                for e in &graph.edges {
                    s.push_str(&format!("  {} --{}--> {}\n", e.from, e.kind, e.to));
                }
            }
```

(The `json` arm is unchanged — `serde_json::to_string_pretty(ev)` serializes `graph`.)

- [ ] **Step 5: Update the CLI ego dispatch**

In `src/main.rs`, the `NavQuery::Ego { .. }` arm: replace `Ok(g) => println!("{}", render_ego(&g, format))` with the `render` path mirroring `callers`:

```rust
                Ok(ev) => {
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
                }
```

- [ ] **Step 6: Run library to verify green**

Run: `cargo test --test navigation_ego`
Expected: PASS — node/edge predicates satisfied via `ev.graph`.

- [ ] **Step 7: Regenerate the ego compat golden**

```bash
cargo build
# CG is the call-graph fixture dir the nav_compat ego_golden test passes as --repo:
CG=$(grep -E 'const CG' tests/cli/nav_compat_test.rs | sed -E 's/.*"([^"]+)".*/\1/')
./target/debug/prism nav ego --repo "$CG" --symbol run --hops 1 --edges Call --format json \
  > tests/fixtures/nav_compat/golden/ego_run.json
```

Verify the new file is `{"query":"ego:run@main.py","items":[],"truncated":false,"warnings":[],"graph":{"nodes":[{"symbol":{"Function":{..}},"location":{..}}, ..],"edges":[{"from":0,"to":1,"kind":"Call"}]}}`. The two node `symbol` objects must still be `{"Function": {..}}` (serde `Option` is transparent), now nested under `graph`.

- [ ] **Step 8: Run the compat suite**

Run: `cargo test --test cli_nav_compat`
Expected: PASS — `ego_golden` matches the regenerated file; **the diff-review goldens (`leftflow`/`thin`/`list`/`parentfunction`) and `callees_run.json` are byte-identical** (Option C; `skip_serializing_if` omits `graph` on flat queries).

- [ ] **Step 9: Commit**

```bash
git add src/navigation/queries.rs src/navigation/types.rs src/output/navigation.rs \
  src/main.rs tests/navigation/ego_test.rs tests/fixtures/nav_compat/golden/ego_run.json
git commit -m "feat(nav): align ego-graph to the Evidence envelope (Plan 3b T2)"
```

---

## Task 3: `module_deps` — call-derived file→file edges + import labeling

**Files:**
- Create: `src/navigation/module_graph.rs`, `tests/navigation/module_graph_test.rs`
- Modify: `src/navigation/mod.rs`, `Cargo.toml`

- [ ] **Step 1: Register the test target + write the failing tests**

In `Cargo.toml`, add next to the other nav targets (after `navigation_cache`):

```toml
[[test]]
name = "navigation_module_graph"
path = "tests/navigation/module_graph_test.rs"
```

Create `tests/navigation/module_graph_test.rs` with the **file-local `session` helper copied verbatim from `tests/navigation/callees_test.rs:6-14`** (do not use `mod common`):

```rust
use prism::navigation::module_graph::module_deps;
use prism::navigation::types::{Source, WarningKind};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        std::fs::write(dir.path().join(name), src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn module_deps_python_cross_file_call_and_import() {
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        ("main.py", "from util import helper\n\ndef run():\n    return helper()\n"),
    ]);
    let ev = module_deps(&s, "main.py");

    // Call-derived cross-file edge main.py -> util.py (PrismCpg).
    let call_item = ev
        .items
        .iter()
        .find(|it| matches!(it.source, Source::PrismCpg))
        .expect("a PrismCpg call-derived dependency");
    assert_eq!(call_item.location.file, "util.py");
    assert!(call_item.why.iter().any(|r| matches!(
        r,
        prism::navigation::types::Reason::Calls { callee, .. } if callee == "helper"
    )));

    // Extracted import labeled UnresolvedImport (HeuristicImport).
    assert!(ev.items.iter().any(|it| matches!(it.source, Source::HeuristicImport)
        && it.why.iter().any(|r| matches!(
            r,
            prism::navigation::types::Reason::UnresolvedImport { module } if module == "util"
        ))));
    assert!(ev.warnings.iter().any(|w| matches!(w.kind, WarningKind::UnresolvedModule)));
    assert!(ev.graph.is_none(), "module-deps is a flat item list, not a graph");
}

#[test]
fn module_deps_rust_is_call_derived_only_no_import_items() {
    // NON-VACUOUS: an UNQUALIFIED use-imported call resolves cross-file by name
    // (`util::helper()` scoped would NOT — see Design-decision #4 Rust caveat).
    let s = session(&[
        ("util.rs", "pub fn helper() -> i32 { 1 }\n"),
        ("main.rs", "mod util;\nuse util::helper;\nfn run() -> i32 { helper() }\n"),
    ]);
    let ev = module_deps(&s, "main.rs");
    // At least one call-derived cross-file edge to util.rs (proves the tier works).
    assert!(
        ev.items.iter().any(|it| matches!(it.source, Source::PrismCpg)
            && it.location.file == "util.rs"),
        "expected a call-derived edge main.rs -> util.rs"
    );
    // Rust extracts no imports: every item is PrismCpg, and no UnresolvedModule warning.
    assert!(
        ev.items.iter().all(|it| matches!(it.source, Source::PrismCpg)),
        "Rust (call-derived-only tier) must surface no HeuristicImport items"
    );
    assert!(
        !ev.warnings.iter().any(|w| matches!(w.kind, WarningKind::UnresolvedModule)),
        "no extracted imports -> no UnresolvedModule warning"
    );
}

#[test]
fn module_deps_skipped_or_unknown_file_warns_not_errors() {
    let s = session(&[("util.py", "def helper():\n    return 1\n")]);
    let ev = module_deps(&s, "nope.py"); // not in the index -> empty + SkippedPath (no panic/error)
    assert!(ev.items.is_empty());
    assert!(ev.warnings.iter().any(|w| matches!(w.kind, WarningKind::SkippedPath)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_module_graph 2>&1 | head`
Expected: FAIL — `prism::navigation::module_graph` does not exist.

- [ ] **Step 3: Implement `module_deps`**

Create `src/navigation/module_graph.rs`:

```rust
use crate::navigation::types::*;
use crate::navigation::NavigationSession;
use std::collections::{BTreeMap, BTreeSet};

/// Stable secondary sort rank when score/file/line tie.
fn source_rank(s: &Source) -> u8 {
    match s {
        Source::PrismCpg => 0,
        Source::HeuristicImport => 1,
        Source::ExternalIndex { .. } => 2,
    }
}

fn sort_items(items: &mut [EvidenceItem]) {
    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap() // scores are 1.0 here — NaN-safe
            .then(a.location.file.cmp(&b.location.file))
            .then(a.location.start_line.cmp(&b.location.start_line))
            .then(source_rank(&a.source).cmp(&source_rank(&b.source)))
    });
}

/// Outbound module dependencies of `file`: distinct target files reached by a
/// resolved cross-file call (`source: PrismCpg`), plus extracted-but-unresolved
/// imports labeled `UnresolvedImport` (`source: HeuristicImport`). Spec §10.
///
/// Mirrors `nodes_at`'s §5 contract: a skipped or unknown file returns empty
/// `items` + a `SkippedPath` warning — never a hard error.
pub fn module_deps(s: &NavigationSession, file: &str) -> Evidence {
    let query = format!("module-deps:{file}");
    if !s.repo.files.contains_key(file) {
        let message = s
            .repo
            .skipped
            .iter()
            .find(|sk| sk.path == file)
            .map(|sk| format!("file excluded: {:?}: {file}", sk.reason))
            .unwrap_or_else(|| format!("file not in nav index: {file}"));
        return Evidence {
            query,
            items: vec![],
            truncated: false,
            warnings: vec![Warning {
                kind: WarningKind::SkippedPath,
                message,
                location: Some(Location { file: file.into(), start_line: 1, end_line: 1 }),
            }],
            graph: None,
        };
    }

    let cg = &s.index.cpg.call_graph;
    // target_file -> distinct (callee, call_site_line, qualifier) reasons.
    // NOTE: resolve_callees_qualified matches any file whose stem == the module
    // qualifier (documented same-stem behavior, call_graph.rs), so a `utils`
    // qualifier can yield edges to both lib/utils.py and src/utils.py. This is
    // deterministic over-reporting, acceptable for the v1 map.
    let mut deps: BTreeMap<String, BTreeSet<(String, usize, Option<String>)>> = BTreeMap::new();
    for (caller, sites) in &cg.calls {
        if caller.file != file {
            continue;
        }
        for site in sites {
            let resolved = cg.resolve_callees_qualified(
                &site.callee_name,
                &site.caller.file,
                site.qualifier.as_deref(),
            );
            for def in resolved {
                if def.file != file {
                    deps.entry(def.file.clone()).or_default().insert((
                        site.callee_name.clone(),
                        site.line,
                        site.qualifier.clone(),
                    ));
                }
            }
        }
    }

    let mut items = Vec::new();
    for (target_file, reasons) in &deps {
        let why = reasons
            .iter()
            .map(|(callee, line, qualifier)| Reason::Calls {
                callee: callee.clone(),
                call_site_line: *line,
                qualifier: qualifier.clone(),
            })
            .collect();
        items.push(EvidenceItem {
            symbol: None,
            location: Location { file: target_file.clone(), start_line: 1, end_line: 1 },
            score: 1.0,
            source: Source::PrismCpg,
            fallback: false,
            why,
            snippet: None,
        });
    }

    // Import labeling: Python/JS/TS/TSX/Go extract imports; Rust/Java/C/C++ do not.
    // NOTE: labeling is unconditional on call resolution, so a module that is BOTH
    // imported and call-resolved (e.g. `import util` + `util.helper()`) appears twice —
    // once as a PrismCpg call edge and once as a HeuristicImport item. Intentional in
    // v1 (filesystem import resolution is deferred, Design-decision #4).
    let mut warnings = Vec::new();
    if let Some(imports) = cg.imports.get(file) {
        let modules: BTreeSet<&String> = imports.values().collect();
        for module in &modules {
            items.push(EvidenceItem {
                symbol: None,
                location: Location { file: file.into(), start_line: 1, end_line: 1 },
                score: 1.0,
                source: Source::HeuristicImport,
                fallback: false,
                why: vec![Reason::UnresolvedImport { module: (*module).clone() }],
                snippet: None,
            });
        }
        if !modules.is_empty() {
            warnings.push(Warning {
                kind: WarningKind::UnresolvedModule,
                message: format!("{} module import(s) not filesystem-resolved (v1)", modules.len()),
                location: None,
            });
        }
    }

    sort_items(&mut items);
    Evidence { query, items, truncated: false, warnings, graph: None }
}
```

In `src/navigation/mod.rs`, add `pub mod module_graph;` with the other `pub mod` lines.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test navigation_module_graph 2>&1 | tail -20`
Expected: PASS — the three `module_deps_*` tests green.

- [ ] **Step 5: Commit**

```bash
git add src/navigation/module_graph.rs src/navigation/mod.rs \
  tests/navigation/module_graph_test.rs Cargo.toml
git commit -m "feat(nav): module-deps call-derived edges + import labeling (Plan 3b T3)"
```

---

## Task 4: `repo_map` — whole-repo file→file graph

**Files:**
- Modify: `src/navigation/module_graph.rs`, `tests/navigation/module_graph_test.rs`

- [ ] **Step 1: Write the failing test**

In `tests/navigation/module_graph_test.rs`, change the top import to `use prism::navigation::module_graph::{module_deps, repo_map};` and append:

```rust
#[test]
fn repo_map_emits_whole_repo_file_graph() {
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        ("main.py", "from util import helper\n\ndef run():\n    return helper()\n"),
        ("lonely.py", "x = 1\n"), // isolated file: must still appear as a node
    ]);
    let ev = repo_map(&s);
    assert_eq!(ev.query, "repo-map");
    assert!(ev.items.is_empty(), "repo-map carries result in graph, not items");
    let g = ev.graph.as_ref().expect("repo-map returns Some(graph)");
    assert!(g.nodes.iter().all(|n| n.symbol.is_none()), "file nodes have no symbol");
    for f in ["main.py", "util.py", "lonely.py"] {
        assert!(g.nodes.iter().any(|n| n.location.file == f), "node for {f}");
    }
    let main_i = g.nodes.iter().position(|n| n.location.file == "main.py").unwrap();
    let util_i = g.nodes.iter().position(|n| n.location.file == "util.py").unwrap();
    assert!(g.edges.iter().any(|e| e.from == main_i && e.to == util_i && e.kind == "ModuleDep"));
    assert!(ev.warnings.iter().any(|w| matches!(w.kind, WarningKind::UnresolvedModule)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_module_graph repo_map_emits_whole_repo_file_graph 2>&1 | head`
Expected: FAIL — `repo_map` not defined.

- [ ] **Step 3: Implement `repo_map`**

Append to `src/navigation/module_graph.rs`:

```rust
/// Whole-repo module graph: one file node per indexed file (isolated files
/// included) + distinct call-derived `ModuleDep` file→file edges. Spec §10.
pub fn repo_map(s: &NavigationSession) -> Evidence {
    let cg = &s.index.cpg.call_graph;
    let mut edge_set: BTreeSet<(String, String)> = BTreeSet::new();
    for (caller, sites) in &cg.calls {
        for site in sites {
            let resolved = cg.resolve_callees_qualified(
                &site.callee_name,
                &site.caller.file,
                site.qualifier.as_deref(),
            );
            for def in resolved {
                if def.file != caller.file {
                    edge_set.insert((caller.file.clone(), def.file.clone()));
                }
            }
        }
    }

    // Whole-repo node set: every indexed file (isolated files included).
    let files: BTreeSet<&String> = s.repo.files.keys().collect();
    let order: BTreeMap<&String, usize> =
        files.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let nodes = files
        .iter()
        .map(|f| GraphNode {
            symbol: None,
            location: Location { file: (*f).clone(), start_line: 1, end_line: 1 },
        })
        .collect();
    // Defensive: only emit an edge when both endpoints are indexed file nodes.
    let edges = edge_set
        .iter()
        .filter_map(|(a, b)| match (order.get(a), order.get(b)) {
            (Some(&from), Some(&to)) => Some(GraphEdge { from, to, kind: "ModuleDep".into() }),
            _ => None,
        })
        .collect();

    // Distinct modules across the WHOLE repo: collect every module into ONE BTreeSet,
    // so a module imported from N files counts once (matches the "distinct module" unit;
    // summing per-file distinct counts would double-count — round-3 MAJOR fix).
    let import_modules: usize = cg
        .imports
        .values()
        .flat_map(|m| m.values())
        .collect::<BTreeSet<_>>()
        .len();
    let mut warnings = Vec::new();
    if import_modules > 0 {
        warnings.push(Warning {
            kind: WarningKind::UnresolvedModule,
            message: format!("{import_modules} module import(s) not filesystem-resolved (v1)"),
            location: None,
        });
    }

    Evidence {
        query: "repo-map".into(),
        items: vec![],
        truncated: false,
        warnings,
        graph: Some(GraphPayload { nodes, edges }),
    }
}
```

(`GraphNode`/`GraphEdge`/`GraphPayload` come via the existing `use ..::types::*` glob.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test navigation_module_graph 2>&1 | tail -20`
Expected: PASS — all four module-graph tests green.

- [ ] **Step 5: Commit**

```bash
git add src/navigation/module_graph.rs tests/navigation/module_graph_test.rs
git commit -m "feat(nav): repo-map whole-repo file graph (Plan 3b T4)"
```

---

## Task 5: CLI subcommands + goldens (CG fixture) + live-repo smoke + docs

**Files:**
- Modify: `src/main.rs`, `tests/cli/nav_compat_test.rs`, `CLAUDE.md`
- Create: `tests/fixtures/nav_compat/golden/module_deps_run.json`, `tests/fixtures/nav_compat/golden/repo_map_run.json`

> **Why two fixtures.** The byte goldens run on the **`CG` Python fixture** (`tests/fixtures/nav_callgraph/`: `main.py` does `import util` + `util.helper()` and also has a `lonely()` function with no cross-file calls; `util.py` defines `helper()`), where cross-file resolution is **stable and non-vacuous** (qualified Python calls resolve via the import map). The `repo_map` golden therefore has **two** file nodes (`main.py`, `util.py`); the isolated-*file* node case is covered separately by Task 4's `lonely.py` library test. A separate **`--repo .` smoke** exercises the real Rust repo path without asserting non-empty edges (Rust scoped calls don't resolve — Design-decision #4). This gives both a canonical golden (§16) and the dogfood smoke (§16).

- [ ] **Step 1: Write the failing CLI tests (goldens use `golden(..)` helper; live smoke is structural)**

Append to `tests/cli/nav_compat_test.rs`. The `golden(name)` helper + `CG` const already exist (used by `ego_golden`/`callees_*`):

```rust
#[test]
fn module_deps_golden() {
    // CG/main.py: util.helper() resolves cross-file to util.py (PrismCpg) AND
    // `import util` is labeled UnresolvedImport (HeuristicImport) + a warning.
    let out = bin()
        .args([
            "nav", "module-deps", "--repo", CG, "--file", "main.py", "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("module_deps_run.json"));
}

#[test]
fn repo_map_golden() {
    let out = bin()
        .args(["nav", "repo-map", "--repo", CG, "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("repo_map_run.json"));
}

#[test]
fn module_deps_repo_map_live_smoke() {
    // Real repo (Rust, scoped-call heavy): the path must run + emit valid JSON.
    // No non-empty edge assertion — Rust scoped calls don't resolve (Design-decision #4).
    let md = bin()
        .args(["nav", "module-deps", "--repo", ".", "--file", "src/main.rs", "--format", "json"])
        .output()
        .unwrap();
    assert!(md.status.success(), "stderr: {}", String::from_utf8_lossy(&md.stderr));
    let v: serde_json::Value = serde_json::from_slice(&md.stdout).unwrap();
    assert_eq!(v["query"], "module-deps:src/main.rs");
    assert!(v["items"].is_array());
    assert!(v.get("graph").is_none(), "module-deps is a flat item list");
    // every item, if any, is call-derived on a Rust file (no spurious imports).
    assert!(v["items"].as_array().unwrap().iter().all(|it| it["source"] == "PrismCpg"));

    let rm = bin()
        .args(["nav", "repo-map", "--repo", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(rm.status.success(), "stderr: {}", String::from_utf8_lossy(&rm.stderr));
    let r: serde_json::Value = serde_json::from_slice(&rm.stdout).unwrap();
    assert_eq!(r["query"], "repo-map");
    assert!(r["graph"]["nodes"].as_array().unwrap().len() > 1, "this repo has many files");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test cli_nav_compat module_deps 2>&1 | head` (one filter; matches both `module_deps_*` tests)
Expected: FAIL — `unrecognized subcommand 'module-deps'`.

- [ ] **Step 3: Add the subcommands + dispatch**

In `src/main.rs`, add to `NavQuery` (after `Ego { .. }`):

```rust
    /// Outbound module dependencies of a file (call-derived + labeled imports).
    ModuleDeps {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long)]
        file: String,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    /// Whole-repo file→file module dependency graph.
    RepoMap {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
```

In `run_nav`, add the arms (both return `Evidence` directly — no error arm, mirroring `nodes_at`):

```rust
        NavQuery::ModuleDeps { repo, file, format } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let ev = prism::navigation::module_graph::module_deps(&session, file);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::RepoMap { repo, format } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let ev = prism::navigation::module_graph::repo_map(&session);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
```

- [ ] **Step 4: Generate the byte goldens from the live tool**

```bash
cargo build
CG=$(grep -E 'const CG' tests/cli/nav_compat_test.rs | sed -E 's/.*"([^"]+)".*/\1/')
./target/debug/prism nav module-deps --repo "$CG" --file main.py --format json \
  > tests/fixtures/nav_compat/golden/module_deps_run.json
./target/debug/prism nav repo-map --repo "$CG" --format json \
  > tests/fixtures/nav_compat/golden/repo_map_run.json
```

Sanity-check the generated files: `module_deps_run.json` must have `"query":"module-deps:main.py"`, one `PrismCpg` item whose `location.file` is `util.py` with a `Calls` reason (`callee:"helper"`, `qualifier:"util"`), one `HeuristicImport` item with `UnresolvedImport{module:"util"}`, and one `UnresolvedModule` warning. `repo_map_run.json` must have `"query":"repo-map"`, file nodes for `main.py` + `util.py`, a `ModuleDep` edge between them, and an `UnresolvedModule` warning. (`graph` absent on `module_deps_run.json`; present on `repo_map_run.json`.)

- [ ] **Step 5: Run to verify pass**

Run: `cargo test --test cli_nav_compat 2>&1 | tail -20`
Expected: PASS — `module_deps_golden`/`repo_map_golden` match the generated files; `module_deps_repo_map_live_smoke` green; **diff-review goldens + `ego_run.json` + `callees_run.json` unchanged**.

> **Note (round-2 MINOR 3):** do **not** touch `tests/integration/coverage_test.rs`. Its scanners key on the `fn test_` prefix and list only algorithm/lang/integration files; `module_deps_*`/`repo_map_*` are nav queries (no `test_` prefix), so registering the file there is inert. The `[[test]]` entry in Task 3 is the only registration needed.

- [ ] **Step 6: Doc-sync CLAUDE.md**

In `CLAUDE.md`, in the navigation documentation, add `module-deps`/`repo-map` to the listed nav queries and note `src/navigation/module_graph.rs` as their home (one or two lines, matching the existing style).

- [ ] **Step 7: Commit**

```bash
git add src/main.rs tests/cli/nav_compat_test.rs CLAUDE.md \
  tests/fixtures/nav_compat/golden/module_deps_run.json \
  tests/fixtures/nav_compat/golden/repo_map_run.json
git commit -m "feat(nav): module-deps/repo-map CLI + CG goldens + live smoke (Plan 3b T5)"
```

---

## Task 6: Full-suite green + fmt + clippy

- [ ] **Step 1: Format**

Run: `cargo fmt` then `cargo fmt --all -- --check`
Expected: clean (exit 0).

- [ ] **Step 2: Full suite**

Run: `cargo test 2>&1 | tail -30`
Expected: all pass. Confirm explicitly: diff-review compat goldens (`leftflow`/`thin`/`list`/`parentfunction`) unchanged; `ego_golden` matches the regenerated file; `navigation_module_graph` green.

- [ ] **Step 3: Clippy**

Run: `cargo clippy --all-targets -- -W clippy::all 2>&1 | tail -20`
Expected: no new warnings in `module_graph.rs`/`queries.rs`/`main.rs`/`output/navigation.rs`.

- [ ] **Step 4: Commit (if fmt produced changes)**

```bash
git add -A
git commit -m "style(nav): cargo fmt (Plan 3b T6)" || echo "nothing to format-commit"
```

---

## Deferred / follow-up (document per the review fix-vs-defer policy)

- **Rust scoped-call resolution** (round-2 BLOCKER 1 / MAJOR 2 root cause). v1 resolves only unqualified / `use`-imported Rust calls cross-file; `module::func()` and `Type::method()` are unresolved (`call_function_name` returns the literal path; no `scoped_identifier` arm). Splitting scoped paths + resolving associated functions would make Rust module-deps materially richer — a CPG-core change, separate slice with its own goldens (relates to spec §19 follow-up #2).
- **Precision-tier warning for call-derived-only languages** (round-1 MINOR 10 optional). A distinct low-severity warning ("imports not extracted for `<lang>`; map is import-incomplete") so an agent knows a Rust/C/C++ map omits import edges — needs the file's `Language` → tier mapping. → nav-polish / 3c.
- **Per-edge `reason` on `GraphEdge`** (spec §8 ego golden) — v1 keeps `{from,to,kind}`. → 3c or nav-polish.
- **Filesystem-resolved imports (`ResolvedImport`)** — the "Exact (imports+calls)" tier for Python/JS/TS/Go; v1 labels every extracted import `UnresolvedImport`. → reasoning/precision follow-up.
- **First-class file/module CPG nodes** — `repo_map` fabricates `GraphNode { symbol: None }` file nodes; spec §19 follow-up #4.
- **Spec §10 prose/tier-table contradiction** — the tier table's "imports surface as UnresolvedImport" for Rust conflicts with the prose ("imports extracted only for Python/JS/TS/TSX/Go"); amend §10 to match the implemented "no items, no warning". → doc-sync.
- **MCP adapter** exposing `module_deps`/`repo_map` → `Evidence` — Plan 3c.

---

## Self-Review

**Spec coverage:** §5 skipped-file → T3 `module_deps` (+test); §8 graph envelope → T1/T2; §10 module/repo map + tiers → T3/T4; §12 CLI seam → T5; §16 named goldens → T2 (`ego_run.json`) + T5 byte goldens (`module_deps_run.json`, `repo_map_run.json` on the CG fixture) + the live-repo dogfood smoke + the non-vacuous Rust/Python library tests in T3/T4; §17 steps 7 (ego) & 10 (module/repo map) → T2/T3/T4.

**Per-task green contract:** T1 is purely additive (no `Ego*` deletion) → compiles green; T2 migrates `ego_graph` + deletes `Ego*` + updates `render`/`main.rs` + tests in one commit → green; T3 registers its `[[test]]` target and uses a file-local `session` helper → runnable; T4 adds the `repo_map` import only when defining it. No task commits a non-compiling tree.

**Type consistency:** `GraphNode { symbol: Option<SymbolRef>, location }`, `GraphEdge { from, to, kind }`, `GraphPayload { nodes, edges }` defined in T1, used identically in T2/T4. `module_deps -> Evidence`, `repo_map -> Evidence` (both no-`Result`, mirroring `nodes_at`); CLI dispatch has no error arm for either. `Reason::{Calls, UnresolvedImport}`, `Source::{PrismCpg, HeuristicImport}`, `WarningKind::{UnresolvedModule, SkippedPath}`, `SkippedFile {path, reason}` all already exist.

**Compat invariants:** diff-review goldens byte-identical (no CPG-core edits); `graph` omitted when `None` (`skip_serializing_if`) so `nodes-at`/`callers`/`callees`/`callees_run.json` unchanged; only nav-owned `ego_run.json` re-baselined. Unit consistency: both `module_deps` and `repo_map` count **distinct modules** with identical warning wording.
