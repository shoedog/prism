# CPG Phase 6: Control Flow Graph Edges — Analysis & Plan

**Status:** Complete (3 PRs merged)
**Date:** 2026-04-02
**Prerequisite:** Phase 4 (CPG on petgraph) — Done
**Actual effort:** ~1 week across 3 PRs

---

## 1. Motivation

All Prism analysis is currently **path-insensitive**: if a variable is tainted
on any code path, it's considered tainted on all paths. This produces false
positives:

```c
int *p = user_input();
if (p == NULL) {
    return -EINVAL;     // p is NULL here — no taint risk
}
use_dangerous(p);       // p is non-NULL here — real taint
```

Path-insensitive analysis reports `use_dangerous(p)` as tainted regardless of
the NULL check. With CFG edges, we can determine that the taint only reaches
the sink on the non-NULL path, and that the NULL path returns early.

### What CFG edges enable

| Capability | Without CFG | With CFG |
|-----------|-------------|----------|
| Guard detection | Cannot see `if (p == NULL) return` guards | Kill taint on guarded paths |
| Dead code after return | Includes lines after `return`/`break` | Stops at terminators |
| Loop iteration modeling | Treats loop body as always-executed | Back-edge distinguishes first-iteration from steady-state |
| Switch/match exhaustiveness | All cases treated as concurrent | Fall-through vs break semantics |
| Error path separation | `goto cleanup` mixed with normal flow | Distinct cleanup vs normal paths |
| Dominator analysis | Not possible | "Every path to sink passes through this check" |

### Which algorithms benefit

| Algorithm | CFG benefit | Priority |
|-----------|------------|----------|
| **taint** | Kill taint at NULL/error guards — #1 FP source in C firmware | Critical |
| **chop** | Source→sink path only if CFG-reachable | High |
| **conditioned_slice** | Evaluate condition predicate against branch conditions | High |
| **provenance** | Track origin through branches accurately | Medium |
| **barrier_slice** | Depth limiting follows actual call paths | Medium |
| **absence_slice** | `goto cleanup` path modeling for resource release | Medium |
| gradient, circular, echo, membrane | Minor improvements to path accuracy | Low |
| horizontal, angle, symmetry, quantum | No benefit (pattern-based, not path-based) | None |

---

## 2. Architecture

### 2.1 CFG edge semantics

Add `CpgEdge::ControlFlow` edges between `CpgNode::Statement` nodes within
each function. The CFG is **intraprocedural** — interprocedural flow is already
modeled by `Call`/`Return` edges.

```
Statement(line 3, Assignment) --ControlFlow--> Statement(line 4, Call)
Statement(line 4, Call)       --ControlFlow--> Statement(line 5, Branch)
Statement(line 5, Branch)     --ControlFlow--> Statement(line 6, Assignment)  // then
Statement(line 5, Branch)     --ControlFlow--> Statement(line 9, Return)      // else
```

### 2.2 Node requirements

Current `CpgNode::Statement` nodes are created only as part of DFG variable
tracking. Phase 6 needs **all** statements as nodes, not just those involved in
data flow. This is the largest structural change.

**New statement discovery pass:** Walk the tree-sitter AST for each function,
create `CpgNode::Statement` nodes for every statement-level tree-sitter node
(assignments, calls, declarations, returns, branches, loops, gotos, labels).

### 2.3 CFG construction algorithm

For each function in each file:

1. **Collect statement nodes** in source order within the function body
2. **Build a basic block list** — consecutive statements with no branches form a
   basic block. A new block starts at: branch targets, loop headers, labels,
   function entry, catch/except handlers
3. **Connect blocks:**
   - Sequential: last statement of block N → first statement of block N+1
   - Branch: `if` condition → then-block entry, else-block entry (or next block
     if no else)
   - Loop: loop condition → body entry, body exit → condition (back-edge),
     condition-false → next block
   - Switch/match: discriminant → each case entry
   - Goto: goto statement → label target
   - Return/break/continue: terminators — no fall-through edge
   - Try/catch: try-block → catch entry (exceptional), try-block → next
     (normal)

### 2.4 Language-specific CFG patterns

| Language | Key CFG constructs | Notes |
|---------|-------------------|-------|
| C/C++ | `if/else`, `for`, `while`, `do-while`, `switch` with fall-through, `goto`, `return`, `break`, `continue` | `goto` is C/C++-specific; `switch` fall-through is unique |
| Python | `if/elif/else`, `for/else`, `while/else`, `try/except/finally`, `with`, `return`, `break`, `continue`, `raise` | `for/else` and `while/else` have unusual CFG — else runs when loop completes normally |
| Go | `if/else`, `for`, `switch` (no fall-through by default), `select`, `goto`, `return`, `break`, `continue`, `defer` | `defer` runs at function exit — creates edge from all return points to deferred stmts |
| JavaScript/TS | `if/else`, `for`, `while`, `do-while`, `switch` with fall-through, `try/catch/finally`, `return`, `break`, `continue`, `throw` | Same as C for most; `finally` always executes |
| Java | Same as JS plus `synchronized` blocks | `synchronized` is effectively try/finally |
| Rust | `if/else`, `loop`, `while`, `for`, `match` (exhaustive, no fall-through), `return`, `break`, `continue`, `?` operator | `?` is early-return — creates edge to function exit. `match` arms are exclusive |
| Lua | `if/elseif/else`, `for`, `while`, `repeat..until`, `return`, `break` | No `continue`, no `goto` (Lua 5.1). `repeat..until` condition checked after body |

### 2.5 Dominator tree (follow-on, not Phase 6 scope)

Once CFG edges exist, `petgraph::algo::dominators::simple_fast()` computes the
dominator tree. A node D **dominates** node N if every path from function entry
to N passes through D. This enables:

- **Guard detection:** If a NULL check dominates a sink, the sink is guarded
- **Post-dominator analysis:** If a cleanup block post-dominates all paths,
  resource release is guaranteed

Dominator analysis is a **follow-on** to Phase 6, not part of it. Phase 6
builds the CFG edges; dominator queries are consumer algorithms.

---

## 3. Implementation Plan

### Step 1: Statement node expansion (3–4 days)

Add a new pass in `build_impl()` (between current Steps 3 and 4) that walks
each function's AST and creates `CpgNode::Statement` nodes for all
statement-level constructs. Current Variable nodes remain — they model data
flow. Statement nodes model control flow.

**Key design decision:** Statement nodes are distinct from Variable nodes. A
line like `x = foo()` produces:
- `CpgNode::Variable { path: "x", access: Def }` (for data flow)
- `CpgNode::Statement { kind: StmtKind::Assignment }` (for control flow)
- `CpgNode::Statement { kind: StmtKind::Call { callee: "foo" } }` (for call)

These are connected by a new `CpgEdge::SameLine` edge (or reuse `Contains` —
TBD).

**Files changed:**
- `src/cpg.rs` — new `collect_statements()` helper, Step 3.5 in `build_impl()`
- `src/languages/mod.rs` — `statement_node_types()` method per language
- `src/ast.rs` — `statements_in_function()` helper returning `(line, StmtKind)` list

### Step 2: Intraprocedural CFG edge construction (5–7 days)

For each function, build CFG edges between its Statement nodes. This is the
core algorithm from §2.3.

**Basic block builder approach:**
```rust
struct BasicBlock {
    statements: Vec<NodeIndex>,  // Statement node indices in this block
    successors: Vec<NodeIndex>,  // Entry nodes of successor blocks
}
```

Walk the AST in source order. Split into basic blocks at branch/loop/label
boundaries. Connect blocks per the rules in §2.3.

**Files changed:**
- `src/cpg.rs` — new `build_cfg_edges()` method called as Step 8 in `build_impl()`
- New: `src/cfg.rs` — basic block builder, `BasicBlock` type, language-specific
  CFG patterns. ~300-400 lines estimated.

### Step 3: Language-specific handlers (3–4 days)

Each language has unique CFG patterns (§2.4). Implement handlers for:
- C/C++ `goto`/`label` → jump edges (reuse existing `goto_statements()` +
  `label_sections()` from ast.rs)
- C/C++ `switch` fall-through → sequential edges between cases unless `break`
- Python `for/else`, `while/else` → else-block edge from loop-exit
- Go `defer` → edges from return points to deferred statements
- Rust `?` operator → early-return edge to function exit
- Rust `match` → exclusive arms (no fall-through)

**Files changed:**
- `src/cfg.rs` — language-specific `build_cfg_for_*` functions
- `src/languages/mod.rs` — new helpers: `is_terminator()`, `is_loop_node()`,
  `switch_has_fallthrough()`

### Step 4: Tests (2–3 days)

- Unit tests for CFG edge construction per language
- Verify dominator relationships on simple examples
- Regression: all existing 400+ tests must pass unchanged
- New tests for guard detection patterns (NULL check, error check, type check)

### Step 5: Algorithm integration (2–3 days, can be separate PR)

Update taint, chop, conditioned_slice to optionally use CFG edges for
path-sensitive filtering. This is additive — algorithms that don't use CFG
continue to work identically.

---

## 4. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Statement node explosion increases graph size 3–5x | High | Medium | Only create Statement nodes for functions that contain diff lines (lazy construction). Most algorithms only care about changed functions. |
| CFG construction complexity for all 9 languages | High | High | Start with C/C++ only (primary FP source). Add other languages incrementally. |
| `goto` creates irreducible CFG | Medium | Low | Use `petgraph` SCC on CFG subgraph to handle irreducible regions. Don't require reducible CFG. |
| Performance regression on large files | Medium | Medium | CFG is intraprocedural — cost is O(statements per function), not O(file). Profile before optimizing. |
| Scope creep into full path-sensitive analysis | High | High | Phase 6 is CFG edges only. Path-sensitive taint killing is a separate PR. Dominator analysis is a follow-on. |

---

## 5. Estimated LOC and Timeline

| Component | Estimated LOC | Days |
|-----------|--------------|------|
| Statement node expansion | ~150 | 3–4 |
| CFG edge construction (`src/cfg.rs`) | ~350 | 5–7 |
| Language-specific handlers | ~200 | 3–4 |
| Tests | ~200 | 2–3 |
| Algorithm integration (taint/chop) | ~150 | 2–3 |
| **Total** | **~1050** | **15–21 days** |

Recommended split into 3 PRs:
1. **PR A:** Statement node expansion + CFG edges for C/C++ (~500 LOC)
2. **PR B:** Multi-language CFG handlers + tests (~400 LOC)
3. **PR C:** Algorithm integration (taint/chop path sensitivity) (~300 LOC)

---

## 6. References

- Joern CPG: Uses ShiftLeft's CFG construction from tree-sitter AST. Similar approach.
- Infer: Facebook's intraprocedural CFG is built per-function, same granularity.
- petgraph dominators: `petgraph::algo::dominators::simple_fast()` — Lengauer-Tarjan O(n α(n))
- Cooper & Torczon, "Engineering a Compiler" ch. 8 — standard CFG construction from AST

---

## 7. Completion Summary

All 3 PRs merged. Implementation matches the plan with these deviations:

**Implemented:**
- Statement node expansion in `cpg.rs` + `ast.rs` (PR A)
- `cfg.rs` CFG edge construction: sequential flow, if/else, loops, goto (PR A)
- `languages/mod.rs`: `is_statement_node()`, `is_loop_node()`, `is_terminator()`, `switch_has_fallthrough()` (PR A)
- Python for/else, while/else, try/except/finally (PR B)
- Go defer→return edges, select statement branches (PR B)
- Rust match arm branching (PR B)
- JS/TS/Java try/catch/finally (PR B)
- C/C++/JS/Java switch fall-through (PR B)
- `taint_forward_cfg()` — CFG-constrained taint with DFG∩CFG filtering (PR C)
- `dfg_cfg_chop()` — CFG-constrained chop with triple intersection (PR C)
- Graceful fallback to pure DFG when no CFG edges present (PR C)

**Deferred / Known gaps:**
- Go `fallthrough` keyword — sequential fall-through workaround only
- Lua pcall/xpcall — no exception branching model
- Dominator analysis — CFG edges exist; `petgraph::algo::dominators` ready
- conditioned_slice / provenance / barrier / absence CFG integration

**Tests added:** 21 new tests across PRs A-C
