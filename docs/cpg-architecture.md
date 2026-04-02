# Code Property Graph Architecture

**Status:** Design  
**Date:** 2026-04-01  
**Scope:** Long-term architecture for Prism's analysis infrastructure

---

## 1. Motivation

Prism currently has three separate analysis data structures:

| Structure | File | Used by |
|-----------|------|---------|
| Tree-sitter AST + helpers | `src/ast.rs` | All 26 algorithms |
| `DataFlowGraph` (def-use chains) | `src/data_flow.rs` | taint, chop, thin_slice, delta_slice, circular, gradient, relevant, conditioned |
| `CallGraph` (caller/callee edges) | `src/call_graph.rs` | barrier, spiral, vertical, circular, 3D, membrane, echo |

These structures are built independently, use different node representations
(`Node<'_>` vs `VarLocation` vs `FunctionId`), and require each algorithm to
bridge between them manually. This creates several problems:

1. **Field insensitivity.** `dev->name` and `dev->id` are collapsed to `dev` in
   the DFG. This is the #1 source of false positives in taint analysis for
   firmware C code, where structs are pervasive.

2. **No composability.** Algorithms that need both data flow and call graph
   information (circular, gradient) must query two separate structures and
   correlate results by line number — a fragile join.

3. **Duplicated traversal logic.** Each algorithm re-implements BFS/DFS with
   slight variations. Graph algorithms (SCC, dominators, shortest paths) are
   hand-rolled instead of using proven implementations.

4. **No shared representation.** A "line of code" is represented differently in
   each structure. Extending with new edge types (control flow, type
   relationships) means adding a fourth separate structure.

## 2. Target Architecture

Three layers, built incrementally:

```
┌─────────────────────────────────────────────────┐
│  Layer 3: Type Enrichment (optional)            │
│  compile_commands.json + clang for C/C++        │
│  Annotates CPG nodes with struct defs, typedefs │
└──────────────────────┬──────────────────────────┘
                       │ enriches
┌──────────────────────▼──────────────────────────┐
│  Layer 2: Code Property Graph (petgraph)        │
│  Unified graph: AST + DFG + Call + CFG edges    │
│  Algorithms = graph queries over typed edges    │
└──────────────────────┬──────────────────────────┘
                       │ nodes reference
┌──────────────────────▼──────────────────────────┐
│  Layer 1: AccessPath                            │
│  Structured variable representation             │
│  { base: "dev", fields: ["config", "timeout"] } │
└─────────────────────────────────────────────────┘
```

### Layer 1: AccessPath

A structured type replacing `var_name: String` throughout the analysis:

```rust
/// A structured representation of a variable access.
///
/// Replaces bare string variable names with a base + field chain,
/// enabling field-sensitive analysis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessPath {
    /// The root variable name: "dev", "self", "buf", "ctx"
    pub base: String,
    /// Field access chain, outermost first: ["config", "timeout"]
    /// Empty for plain variables.
    pub fields: Vec<String>,
}
```

**Key operations:**

| Operation | Example | Semantics |
|-----------|---------|-----------|
| `is_prefix_of` | `dev` is prefix of `dev->config` | Whole-struct taint: if `dev` is tainted, all fields are |
| `matches_field_sensitive` | `dev->name` matches `dev->name` only | Field-precise: `dev->name` does NOT match `dev->id` |
| `matches_field_insensitive` | `dev->name` matches `dev->*` | Fallback: collapse to base when precision isn't possible |
| `depth` | `dev->config->timeout` = depth 2 | For k-limiting |
| `truncate(k)` | `dev->a->b->c` at k=2 → `dev->a->b` | Prevents infinite paths through recursive structures |
| `from_expr` | Parse `dev->config->timeout` or `self.field` | Language-aware construction from tree-sitter nodes |

**k-limiting:** Access paths deeper than k (default k=5) are truncated to
depth k. This handles recursive data structures (`node->next->next->...`)
without infinite path enumeration. Follows the precedent set by Facebook Infer
and FlowDroid.

**Normalization:** `(*dev).field` and `dev->field` produce the same AccessPath.
Language-specific parsing in `ast.rs` handles the normalization at construction
time so downstream code never sees syntax variants.

### Layer 2: Code Property Graph

A unified graph where all code entities are nodes and all relationships are
typed edges, built on `petgraph`:

```rust
/// Node types in the Code Property Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpgNode {
    /// A function definition.
    Function {
        name: String,
        file: String,
        start_line: usize,
        end_line: usize,
    },
    /// A statement or expression at a specific location.
    Statement {
        file: String,
        line: usize,
        kind: StmtKind,
    },
    /// A variable access (def or use) with structured path.
    Variable {
        path: AccessPath,
        file: String,
        function: String,
        line: usize,
        access: VarAccessKind,
    },
}

/// Statement kinds relevant for analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Assignment,
    Call { callee: String },
    Return,
    Branch,       // if/switch/match
    Loop,         // for/while/loop
    Goto,
    Label { name: String },
    Declaration,
    Other,
}

/// Edge types in the Code Property Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpgEdge {
    /// Data flow: definition reaches this use.
    DataFlow,
    /// Control flow: execution proceeds from source to target.
    ControlFlow,
    /// Call: call site invokes callee function.
    Call,
    /// Return: function returns to call site.
    Return,
    /// Contains: function contains this statement/variable.
    Contains,
    /// FieldOf: variable is a field access on another variable.
    FieldOf,
}
```

**What petgraph gives us for free:**

| Algorithm | petgraph API | Currently hand-rolled in |
|-----------|-------------|--------------------------|
| SCC (strongly connected components) | `tarjan_scc()` | `circular_slice.rs` |
| Dominators | `dominators::simple_fast()` | Not implemented (needed for CFG) |
| Topological sort | `toposort()` | `call_graph.rs` cycle detection |
| Shortest/all paths | `dijkstra()`, `all_simple_paths()` | `gradient_slice.rs` |
| Reachability | `has_path_connecting()` | `data_flow.rs` forward/backward_reachable |
| Subgraph views | `EdgeFiltered`, `NodeFiltered` | Manual in every algorithm |

**Edge-filtered queries:** Algorithms select which edge types to traverse.
Taint analysis traverses only `DataFlow` edges. Call graph analysis traverses
only `Call`/`Return` edges. Membrane slice traverses `Call` edges then checks
for `DataFlow` edges at call sites. This replaces the current pattern of
building separate structures and joining by line number.

### Layer 3: Type Enrichment (Optional)

For C/C++ firmware projects that have a build system:

```
compile_commands.json → clang -fsyntax-only -Xclang -ast-dump=json
                      → extract struct definitions, field types, typedefs
                      → annotate CPG Variable nodes with type info
```

**What type information enables:**

| Capability | Without types | With types |
|-----------|--------------|------------|
| Whole-struct detection | Heuristic (assignment to base name) | Precise (`memcpy` size matches struct size) |
| Field enumeration | Unknown (can only see accessed fields) | Complete (all fields from struct definition) |
| Typedef resolution | `my_handle_t` is opaque | `my_handle_t` = `struct device *` |
| Virtual dispatch | Name-matched | Vtable-resolved via class hierarchy |
| Union field overlap | Treated as separate | Detected as aliasing |

**Dependency:** `compile_commands.json` (generated by CMake, Bear, or
`intercept-build`). This is standard in C/C++ toolchains and most firmware
projects already produce it. Prism works without it (falls back to
field-insensitive) but benefits significantly when it's available.

**Integration:** A `TypeDatabase` struct built from clang's JSON AST dump,
queried during CPG construction to annotate Variable nodes. This is a separate
enrichment pass — the CPG builds correctly without it.

## 3. Migration Strategy

### Phase 1: AccessPath Type — **Done**

**Scope:** Introduce `AccessPath`, migrate `data_flow.rs` to use it, update
`ast.rs` extraction helpers. All algorithms continue to work unchanged —
`AccessPath` provides backward-compatible `base` matching that's equivalent to
today's string matching.

**Files changed:**
- New: `src/access_path.rs` — AccessPath type with construction, matching, display
- Modified: `src/data_flow.rs` — `VarLocation.var_name: String` → `VarLocation.path: AccessPath`
- Modified: `src/ast.rs` — `extract_lvalue_names` → `extract_lvalue_paths`, rvalue collection returns AccessPaths
- Modified: All algorithm files that read `var_name` from DFG results (grep for `.var_name`)

**Behavior change:** None in Phase 1. AccessPath stores field info but matching
uses `base` only. This is a pure refactor with zero behavior delta — all 204+
tests must continue to pass.

**Estimated effort:** 3-4 days

### Phase 2: Field-Sensitive Matching — **Done**

**Scope:** Change DFG edge construction to match on full AccessPath instead of
base name only. `dev->name` def connects only to `dev->name` uses, not
`dev->id` uses.

**Key design decisions:**
- Whole-struct operations (`dev = ...`, `memcpy(&dev, ...)`) create edges to ALL
  known fields of `dev`. "Known fields" = any field seen accessed on the same
  base in the current function.
- Pointer dereference (`*ptr`) where `ptr = &dev->field` is handled by local
  must-alias (Phase 3), not by field matching.
- k-limiting: paths deeper than k=5 truncate and fall back to field-insensitive.

**Behavior change:** Reduced false positives in taint, chop, thin_slice for
struct-heavy code. Potential false negative regression if reference tracking
misses field accesses — extensive testing required.

**Estimated effort:** 1-2 days (small change on top of Phase 1)

### Phase 3: Local Must-Alias — **Done**

**Scope:** Within each function, track simple pointer aliases:
`ptr = dev` → `ptr->field` resolves to `dev->field`. `ptr = &dev->field` →
`*ptr` resolves to `dev->field`.

**Implementation:** An alias map built during DFG construction. Before creating
an AccessPath for a variable, check if the base has a known alias and
substitute. This is intraprocedural only — no interprocedural alias analysis.

**Estimated effort:** 2-3 days

### Phase 4: CPG on petgraph — **Done**

**Scope:** Introduce `CpgNode`, `CpgEdge`, build unified graph from
tree-sitter + existing DFG/call graph logic. Migrate algorithms one at a time
from direct ast.rs/data_flow.rs/call_graph.rs queries to CPG queries.

**Migration approach:** Algorithms are migrated incrementally. Both old and new
paths coexist during migration. Each algorithm migration is a separate PR with
its own test verification.

**Algorithm migration priority (by CPG benefit):**
1. `circular_slice` — needs SCC (petgraph `tarjan_scc`)
2. `gradient_slice` — needs weighted paths (petgraph `dijkstra`)
3. `taint` / `chop` / `thin_slice` — DFG reachability
4. `barrier_slice` / `spiral_slice` — call graph + DFG combined
5. `membrane_slice` / `echo_slice` — cross-file call + error handling
6. Pattern-based algorithms (horizontal, angle, absence, symmetry) — least
   benefit since they're primarily AST pattern matching, not graph traversal

**Estimated effort:** 2-3 weeks (incremental, one algorithm at a time)

### Phase 5: Type Enrichment — **Done**

**Scope:** Optional `compile_commands.json` integration for C/C++. Build
`TypeDatabase` from clang AST dump. Annotate CPG Variable nodes with type info.
Enable precise whole-struct detection and field enumeration.

**Estimated effort:** 1-2 weeks

### Phase 6: Control Flow Graph — **Done**

Added CFG edges to the CPG. `cfg.rs` builds intraprocedural CFG edges from
tree-sitter AST. `taint_forward_cfg()` and `dfg_cfg_chop()` filter DFG results
by CFG reachability, pruning dead-code paths. Multi-language handlers for
Python for/else, Go defer/select, Rust match, JS/Java try/catch/finally,
C switch fall-through. See `docs/cpg-phase6-cfg-plan.md` for full details.

## 4. Open Questions & Uncertainties

### Resolved

1. **petgraph vs custom graph?**
   → petgraph. Implemented in Phase 4.

2. **Where does AccessPath live?**
   → `src/access_path.rs`. Implemented in Phase 1.

3. **How to handle array indices in AccessPath?**
   → Option (a): ignore index value, keep `[]` as a sentinel field. Array-insensitive.
   Revisit if false positives from array collapsing become a problem.

4. **Should the CPG store source text on nodes?**
   → No. Algorithms query back to `ParsedFile` for source text.

6. **How to handle languages without field access?**
   → AccessPath stores field names only (no operators). All languages use dot
   access; C/C++ `->` normalized at parse time. Implemented and tested across
   all 9 languages.

7. **clang dependency weight for Layer 3?**
   → Option (a): shell out to clang, parse JSON output. Implemented in Phase 5.
   Tree-sitter struct extraction fallback planned — see `docs/cpg-improvements.md` §4.

8. **Virtual dispatch: part of CPG or separate?**
   → Part of CPG via CHA (class hierarchy analysis). Implemented in Phase 5.
   RTA refinement planned — see `docs/cpg-improvements.md` §5.

### Open

5. **Incremental CPG construction?**
   Deferred. Batch construction is fine for code review. Revisit if/when LSP
   integration is on the roadmap.

9. **JS/TS destructuring alias tracking?**
   `const { name } = device` creates an untracked alias. Taint blind spot for
   idiomatic JS/TS. Planned — see `docs/cpg-improvements.md` §1.

10. **Build CPG once, share across algorithms?**
    12 algorithms redundantly rebuild identical CPG. `CpgContext` bundle type
    planned — see `docs/cpg-improvements.md` §2-3.

## 5. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Phase 1 refactor breaks existing tests | Low | High | **Mitigated.** Phase 1 done, zero regressions. |
| Field sensitivity introduces false negatives | Medium | Medium | **Mitigated.** Dual matching implemented; field isolation verified across all 9 languages. |
| petgraph performance on large firmware repos | Low | Medium | Not yet tested at scale. Profile before optimizing. |
| Algorithm migration introduces subtle bugs | Medium | High | **Mitigated.** All 12 algorithms migrated, 489 tests pass. |
| clang dependency reduces portability | Low | Low | Layer 3 is optional. Tree-sitter fallback planned (`docs/cpg-improvements.md` §4). |
| Scope creep — "while we're refactoring..." | High | Medium | **Mitigated.** Strict phase boundaries maintained across all 6 phases. |

## 6. Non-Goals

- **Full points-to analysis.** Andersen/Steensgaard-style analysis requires
  type information and whole-program analysis. Out of scope — local must-alias
  (Phase 3) is sufficient for the code review use case.

- **Full path sensitivity.** Phase 6 added CFG-reachability filtering (prunes
  dead code after return/break), but full path-sensitive analysis ("this taint
  only reaches the sink if branch X is taken") requires dominator analysis +
  symbolic condition evaluation. Dominator infrastructure is ready
  (`petgraph::algo::dominators`) as a follow-on.

- **IDE / LSP integration.** The CPG is built in batch mode for code review.
  Incremental construction for real-time IDE feedback is a future direction,
  not a current goal.

- **Sound analysis.** Prism is a code review assistant, not a verification
  tool. False negatives are acceptable. False positives should be minimized but
  some are inevitable without type information.

## 7. References

- Joern CPG schema: https://cpg.joern.io — open-source CPG for C/C++/Java,
  good reference for node/edge types
- Facebook Infer access paths: "Compositional Analysis with Demand-Driven
  Infer" (OOPSLA 2019) — k-limited access paths for scalable analysis
- FlowDroid: "FlowDroid: Precise Context, Flow, Field, Object-sensitive and
  Lifecycle-aware Taint Analysis for Android Apps" (PLDI 2014) — access path
  design for taint analysis
- petgraph: https://docs.rs/petgraph — Rust graph library
- tree-sitter field_expression: the C grammar represents `dev->field` as
  `field_expression { argument: "dev", field: "field", operator: "->" }`
