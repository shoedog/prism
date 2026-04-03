# Plan: E9 (Tree-sitter Query-Based Pattern Matching) & E10 (Incremental CPG Construction)

## E9: Tree-sitter Query-Based Pattern Matching

### Problem

`src/ast.rs` contains ~22 recursive `collect_*` methods (1,750+ lines) that
manually walk the AST using `node.walk()` + cursor iteration, checking
`node.kind()` at every step. This is verbose, error-prone across 11 languages,
and slower than tree-sitter's native query engine which is compiled to a
bytecode VM optimized for structural matching.

### Current State

- **Tree-sitter 0.25** is already a dependency — it ships the `Query` and
  `QueryCursor` APIs, but **zero queries are used today**.
- All traversal follows one pattern:
  ```rust
  fn collect_X(&self, node: Node<'_>, ..., out: &mut Vec<...>) {
      if node.kind() == "some_type" { out.push(...); }
      let mut cursor = node.walk();
      for child in node.children(&mut cursor) {
          self.collect_X(child, ..., out);
      }
  }
  ```
- Language-specific node types are mapped via `src/languages/mod.rs` methods
  like `function_node_types()`, `is_assignment_node()`, etc.
- Algorithms (`absence_slice`, `symmetry_slice`, `horizontal_slice`) don't
  traverse directly — they call `ParsedFile` methods that do the walks.

### Design

#### 1. Query registry per language (`src/queries.rs`, new file)

Create a struct that holds pre-compiled `tree_sitter::Query` objects keyed by
`(Language, QueryKind)`:

```rust
pub enum QueryKind {
    Functions,       // replaces collect_functions
    Assignments,     // replaces collect_assignments + collect_assignment_paths
    Calls,           // replaces collect_calls + collect_call_names_at_lines
    Identifiers,     // replaces collect_all_identifiers + collect_identifiers_at_row
    Returns,         // replaces collect_returns
    Statements,      // replaces collect_statements
    Gotos,           // replaces collect_gotos
    Labels,          // replaces collect_labels
    VariableRefs,    // replaces collect_variable_refs
    RValues,         // replaces collect_rvalues + collect_rvalue_paths
    Callees,         // replaces collect_all_callees
    Aliases,         // replaces collect_alias_assignments
    ConditionVars,   // replaces collect_condition_vars
}
```

Each `(Language, QueryKind)` maps to a tree-sitter query string. Example:

```rust
// Python assignments
(Language::Python, QueryKind::Assignments) =>
    "(assignment left: (_) @lhs right: (_) @rhs) @assign"

// C assignments
(Language::C, QueryKind::Assignments) =>
    "(assignment_expression left: (_) @lhs right: (_) @rhs) @assign"

// Go short variable declarations
(Language::Go, QueryKind::Assignments) =>
    r#"[
        (assignment_statement left: (_) @lhs right: (_) @rhs) @assign
        (short_var_declaration left: (_) @lhs right: (_) @rhs) @assign
    ]"#
```

Queries are compiled once (via `OnceLock` or at `ParsedFile` creation time) and
reused across all calls. Tree-sitter query compilation is ~1ms per query;
execution is significantly faster than manual walks because it uses an optimized
state machine that skips irrelevant subtrees.

#### 2. QueryCursor integration in `ParsedFile`

Add a method to `ParsedFile` (in `src/ast.rs`):

```rust
pub fn run_query(&self, kind: QueryKind) -> QueryMatches {
    let query = QUERY_REGISTRY.get(self.language, kind);
    let mut cursor = QueryCursor::new();
    // Optional: cursor.set_point_range() for row-scoped queries
    cursor.matches(query, self.tree.root_node(), self.source.as_bytes())
}
```

For row-scoped variants (e.g., `collect_identifiers_at_row`), use
`QueryCursor::set_point_range()` to restrict matching to a line range —
tree-sitter handles this natively and avoids visiting nodes outside the range.

#### 3. Incremental migration of `collect_*` methods

Migrate one `collect_*` method at a time. Each migration:
1. Adds query strings for all 11 languages to the registry.
2. Replaces the recursive method body with a `run_query()` call + capture extraction.
3. Keeps the same public API signature (returns same types).
4. Existing tests validate correctness — no new tests needed per migration.

**Priority order** (by call frequency and complexity):

| Phase | Methods | Lines Replaced | Impact |
|-------|---------|---------------|--------|
| 1 | `collect_functions` | ~20 | Used by every algorithm; simplest query |
| 2 | `collect_assignments`, `collect_assignment_paths` | ~80 | Core DFG input; 11 language variants |
| 3 | `collect_calls`, `collect_call_names_at_lines`, `collect_all_callees` | ~80 | Core call graph input |
| 4 | `collect_all_identifiers`, `collect_identifiers_at_row` | ~30 | High frequency; scoped variant benefits from `set_point_range` |
| 5 | `collect_rvalues`, `collect_rvalue_paths` | ~60 | DFG R-value analysis |
| 6 | `collect_returns`, `collect_statements`, `collect_nested_statements` | ~60 | CFG construction |
| 7 | `collect_gotos`, `collect_labels` | ~25 | Niche but straightforward |
| 8 | `collect_variable_refs`, `collect_variable_refs_scoped`, `collect_path_refs` | ~100 | Most complex; scoped traversal with filtering |
| 9 | `collect_condition_vars`, `collect_aliases_inner` | ~60 | Complex extraction logic |

#### 4. Language node-type mapping consolidation

Currently `src/languages/mod.rs` has ~30 methods returning node type strings.
Many of these become unnecessary once queries encode the types directly. After
migration:

- **Keep**: `Language::from_extension()`, `comment_node_types()`, and methods
  not replaceable by queries (heuristic logic like `assignment_target()`).
- **Remove**: `function_node_types()`, `is_assignment_node()`, `is_call_node()`,
  `is_identifier_node()`, etc. — their logic moves into query strings.
- **New**: `Language::ts_language(&self) -> tree_sitter::Language` method to
  get the grammar needed for query compilation.

### Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Query syntax differences across grammars | Test each query against all 11 language grammars at compile time via `#[test]` that compiles every registered query |
| Capture semantics differ from manual walk (e.g., nested matches) | Run old and new implementations side-by-side in tests during migration |
| Some `collect_*` methods have imperative filtering logic (e.g., `collect_variable_refs_scoped` checks line ranges) | Use `QueryCursor::set_byte_range()` for spatial filtering; keep post-filter for semantic conditions |
| Performance regression if queries are recompiled per call | `OnceLock<HashMap<(Language, QueryKind), Query>>` ensures single compilation |

### Estimated Scope

- **New file**: `src/queries.rs` (~300 lines: registry + 11 languages x ~13 query kinds)
- **Modified**: `src/ast.rs` (net reduction of ~800 lines after full migration)
- **Modified**: `src/languages/mod.rs` (net reduction of ~200 lines)
- **Modified**: `src/lib.rs` (add `pub mod queries;`)
- **No algorithm changes** — the `ParsedFile` public API stays the same.

---

## E10: Incremental CPG Construction

### Problem

`CpgContext::build()` processes **all parsed files** to construct the full CPG
(DataFlowGraph + CallGraph + CFG + nodes/edges/indices). For a typical review
where the diff touches 3-5 files in a 500-file repo, this means ~99% of the
construction work is wasted. The review agent use case calls for CPG coverage of
only changed files plus their direct dependents.

### Current State

- `CpgContext::build()` (`src/cpg.rs:45-54`) calls
  `CodePropertyGraph::build_enriched()` which unconditionally processes all files.
- `CpgContext::without_cpg()` exists for AST-only algorithms — returns an empty
  CPG. This is the only existing optimization.
- `SlicingAlgorithm::needs_cpg()` (`src/slice.rs:163-179`) distinguishes 12
  CPG-needing algorithms from 14 AST-only ones.
- Construction phases and their costs:
  1. **DataFlowGraph::build()** — O(total statements x function scope). Most expensive.
  2. **CallGraph::build()** — O(total AST nodes) for function/call discovery + O(call sites x code size) for indirect resolution.
  3. **CPG node creation** — O(functions + defs + uses). Fast.
  4. **CFG construction** — O(total statements). Moderate.
  5. **Type enrichment** — O(virtual calls x class hierarchy). Moderate, optional.

### Design

#### 1. New `CpgContext::build_scoped()` entry point

```rust
impl<'a> CpgContext<'a> {
    pub fn build_scoped(
        files: &'a BTreeMap<String, ParsedFile>,
        diff: &DiffInput,
        type_db: Option<&'a TypeDatabase>,
    ) -> Self {
        let cpg = CodePropertyGraph::build_scoped(files, diff, type_db);
        CpgContext { cpg, files, type_db }
    }
}
```

The existing `build()` remains unchanged for non-diff contexts (e.g., whole-repo
analysis, tests).

#### 2. Three-tier file scoping

```
Tier 0: Changed files     — files in DiffInput
Tier 1: Direct callers    — files containing functions that call into changed functions
Tier 2: Direct callees    — files containing functions called by changed functions
```

The scope is `Tier 0 ∪ Tier 1 ∪ Tier 2`. This is the minimum set needed for:
- Forward taint from changed code (Tier 0 + Tier 2)
- Backward impact on callers (Tier 0 + Tier 1)
- Membrane/echo analysis (Tier 1 specifically)

#### 3. Two-pass construction

**Pass 1: Lightweight call graph (all files, functions only)**

Build a "skeleton" call graph that collects only function definitions and call
sites — no indirect resolution, no DFG. This is cheap because it only runs
Phases 1-2 of `CallGraph::build()`:

```rust
impl CallGraph {
    /// Build a lightweight call graph with only direct calls.
    /// Skips Phase 3 (indirect resolution). Used for scoping.
    pub fn build_skeleton(files: &BTreeMap<String, ParsedFile>) -> Self { ... }
}
```

Cost: O(total AST nodes) but with a small constant — just function/call
discovery, no string scanning or alias resolution.

From this skeleton, compute the scoped file set:

```rust
fn compute_scope(
    skeleton_cg: &CallGraph,
    diff: &DiffInput,
    files: &BTreeMap<String, ParsedFile>,
) -> BTreeSet<String> {
    let mut scope = BTreeSet::new();

    // Tier 0: changed files
    for d in &diff.files {
        scope.insert(d.file_path.clone());
    }

    // Identify changed functions (functions whose line range overlaps diff lines)
    let changed_fns: Vec<FunctionId> = identify_changed_functions(files, diff);

    // Tier 1: files containing direct callers of changed functions
    for func_id in &changed_fns {
        for caller in skeleton_cg.callers_of(&func_id.name) {
            scope.insert(caller.file.clone());
        }
    }

    // Tier 2: files containing direct callees of changed functions
    for func_id in &changed_fns {
        if let Some(call_sites) = skeleton_cg.calls.get(func_id) {
            for site in call_sites {
                for defn in skeleton_cg.resolve_callees_basic(&site.callee_name) {
                    scope.insert(defn.file.clone());
                }
            }
        }
    }

    scope
}
```

**Pass 2: Full CPG on scoped files**

Filter the `files` BTreeMap to only the scoped set, then run the existing
`build_enriched()` logic on this subset:

```rust
fn build_scoped(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    type_db: Option<&TypeDatabase>,
) -> CodePropertyGraph {
    // Pass 1
    let skeleton_cg = CallGraph::build_skeleton(files);
    let scope = compute_scope(&skeleton_cg, diff, files);

    // Pass 2: build full CPG on scoped subset
    let scoped_files: BTreeMap<&String, &ParsedFile> = files
        .iter()
        .filter(|(k, _)| scope.contains(*k))
        .collect();

    // Reuse existing build_impl logic on scoped_files
    Self::build_impl(&scoped_files, type_db)
}
```

#### 4. Scope metadata in CpgContext

Algorithms need to know when they're working with a scoped CPG so they can:
- Avoid false negatives (e.g., "no callers found" might mean "callers not in scope")
- Report scope boundaries in output

```rust
pub struct CpgContext<'a> {
    pub cpg: CodePropertyGraph,
    pub files: &'a BTreeMap<String, ParsedFile>,
    pub type_db: Option<&'a TypeDatabase>,
    pub scope: Option<CpgScope>,  // NEW
}

pub struct CpgScope {
    pub scoped_files: BTreeSet<String>,
    pub changed_files: BTreeSet<String>,
    pub tier: ScopeTier,
}

pub enum ScopeTier {
    Full,              // all files (existing behavior)
    DiffPlusCallers,   // Tier 0 + 1 + 2
}
```

#### 5. Wire into algorithm dispatch

In `src/algorithms/mod.rs`, modify `run_slicing_compat()`:

```rust
pub fn run_slicing_compat(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
    type_db: Option<&TypeDatabase>,
) -> Vec<SliceResult> {
    let any_needs_cpg = config.algorithms.iter().any(|a| a.needs_cpg());

    let ctx = if any_needs_cpg {
        if config.scoped_cpg {  // NEW flag
            CpgContext::build_scoped(files, diff, type_db)
        } else {
            CpgContext::build(files, type_db)
        }
    } else {
        CpgContext::without_cpg(files, type_db)
    };

    run_slicing(&ctx, diff, config)
}
```

Add `--scoped-cpg` CLI flag (default: off initially, flip to default-on after validation).

#### 6. Algorithm compatibility

| Algorithm | Scoped CPG Safe? | Notes |
|-----------|-----------------|-------|
| BarrierSlice | Yes | Depth-limited by design; scope ≥ barrier depth=1 |
| Chop | Mostly | Source/sink must both be in scope; warn if not |
| Taint | Yes | Forward from diff; callees in scope |
| DeltaSlice | Yes | Compares two versions of same files |
| SpiralSlice | Yes | Composes other algorithms; inherits their safety |
| CircularSlice | Partial | Cross-function cycles may exit scope; document as known limitation |
| VerticalSlice | Partial | End-to-end feature path may span layers outside scope |
| ThreeDSlice | Yes | Temporal component is git-based, not CPG-dependent |
| GradientSlice | Yes | Scoring is local to reachable nodes |
| ProvenanceSlice | Yes | Traces origin within reachable DFG |
| MembraneSlice | Yes | Specifically needs callers of changed APIs (Tier 1) |
| EchoSlice | Yes | Ripple to callers (Tier 1) |

For `CircularSlice` and `VerticalSlice`, the scoped CPG may produce incomplete
results. Options:
1. Fall back to full CPG for these two algorithms (check `needs_full_cpg()`).
2. Accept the limitation and document it — the diff-scoped results are still
   useful for review even if not exhaustive.

Recommend option 2: the review use case prioritizes speed over exhaustiveness.

### Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Skeleton call graph misses indirect calls → incomplete scope | Indirect calls are rare in practice; Phase 3 resolution in skeleton would negate savings. Accept as known limitation. |
| Scoped CPG produces different results than full CPG | Add integration test: run both modes on the same diff, verify scoped results ⊆ full results |
| `build_impl` assumes ownership of files map | Refactor to accept `impl Iterator<Item = (&str, &ParsedFile)>` or use a trait |
| Two-pass construction could be slower for small repos | Short-circuit: if `scope.len() > files.len() * 0.5`, fall back to full build |

### Estimated Scope

- **Modified**: `src/cpg.rs` (~100 lines: `build_scoped`, `CpgScope` struct)
- **Modified**: `src/call_graph.rs` (~40 lines: `build_skeleton` method)
- **Modified**: `src/algorithms/mod.rs` (~15 lines: scoped dispatch)
- **Modified**: `src/slice.rs` (~5 lines: `scoped_cpg` flag in `SliceConfig`)
- **Modified**: `src/main.rs` (~5 lines: `--scoped-cpg` CLI flag)
- **New tests**: `tests/integration/scoped_cpg_test.rs` (~150 lines)

### Performance Expectations

For a 500-file repo with a 5-file diff:
- **Skeleton call graph**: ~10% of full call graph cost (Phases 1-2 only, no Phase 3)
- **Scoped file set**: ~15-25 files (5 changed + ~10-20 callers/callees)
- **Full CPG on scoped set**: ~5% of full CPG cost (25/500 files)
- **Total**: ~15% of current cost — roughly **6-7x speedup**

For single-file diffs (common in review), the speedup is closer to 10-20x.

---

## Implementation Order

**E10 first, then E9.** Rationale:
- E10 is lower risk (additive, behind a flag) and has immediate measurable
  impact on the review agent use case.
- E9 is a large refactor touching `ast.rs` (the most-imported module). Doing it
  second avoids merge conflicts with E10's changes to `cpg.rs`/`call_graph.rs`.
- E9's query-based traversal makes scoped construction even faster (compound benefit).

### Phase 1: E10 — Incremental CPG (3 steps)

1. **`CallGraph::build_skeleton()`** — Add skeleton builder, test against full
   call graph on fixture repos.
2. **`CpgContext::build_scoped()`** — Implement scoping logic, `CpgScope`
   metadata, integration with existing `build_impl`.
3. **CLI + dispatch wiring** — `--scoped-cpg` flag, `run_slicing_compat`
   integration, integration tests comparing scoped vs full results.

### Phase 2: E9 — Query-Based Matching (4 steps)

1. **Query registry** — `src/queries.rs` with `QueryKind` enum, per-language
   query strings, `OnceLock`-based compilation cache, compile-time validation tests.
2. **Phase 1 migration** — `collect_functions`, `collect_calls`,
   `collect_all_callees` (high-impact, simple queries).
3. **Phase 2 migration** — `collect_assignments`, `collect_rvalues`,
   `collect_identifiers` (core DFG inputs).
4. **Phase 3 migration** — remaining `collect_*` methods, cleanup of obsolete
   `languages/mod.rs` methods.
