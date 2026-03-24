## 4. Program Slicing: Established Taxonomy

The agent's context extraction strategy is fundamentally a program slicing problem. This section catalogs all established slicing types and their applicability.

### 4.1 Directional Types

| Type | Definition | Current Status | Value Assessment |
|------|-----------|---------------|-----------------|
| **Backward slice** | What statements affect this point? | LeftFlow ✓ | Core — already proven |
| **Forward slice** | What does this point affect? | FullFlow implemented, not wired in | High — exclusive catches per ICML paper |
| **Thin slice** | Minimal backward slice (data deps only, no control deps) | Not implemented | High — more focused than LeftFlow for LLM attention |
| **Bidirectional/full** | Both backward and forward | FullFlow covers this | Wire in alongside LeftFlow |

### 4.2 Interprocedural Types

| Type | Definition | Current Status | Value Assessment |
|------|-----------|---------------|-----------------|
| **Caller analysis (up)** | Trace up the call graph | jedi/ts-morph depth-1 ✓ | Extend to depth 2+ |
| **Callee analysis (down)** | Trace down the call graph | Partial (signature_changes, new_call_sites) | Medium — "what happens to my return value?" |
| **Barrier slice** | Interprocedural with explicit boundaries | Not implemented | High — solves "how deep?" question semantically |
| **Chopping** | All paths between source and sink | Not implemented | Medium-high — security, data flow bugs |

### 4.3 Execution-Dependent Types

| Type | Definition | Current Status | Value Assessment |
|------|-----------|---------------|-----------------|
| **Static slice** | Analysis without execution | All current slicing | Foundation |
| **Dynamic slice** | Slice from execution trace | Not implemented | Medium — enhances test generation evidence |
| **Conditioned slice** | Slice under specific assumption | Not implemented | Medium — post-guard analysis for invariant verification |
| **Observation-based** | Delete statements, observe behavior change | Covered by semantic diff technique | Already planned under different name |

### 4.4 Specialized Types

| Type | Definition | Current Status | Value Assessment |
|------|-----------|---------------|-----------------|
| **Relevant slice** | Backward + potential paths from branch flips | Not implemented | Medium — "one flip away from a bug" |
| **Delta slice** | Minimal changes causing behavioral difference | Not implemented | Medium — theoretical foundation for the whole agent |
| **Union/simultaneous** | Combined slice across multiple criteria | Not implemented | Low-medium — refinement of LeftFlow |
| **Taint analysis** | Forward trace of untrusted values | bandit/semgrep do limited taint | Medium — full taint through diff changes |
| **Amorphous slice** | Transformed (not subset) program | Not applicable | Low — comprehension tool, not review tool |

### 4.5 Implementation Priority for Rust AST Library

```rust
// Phase 1: Core (extend existing)
fn forward_slice(ast: &AST, diff_hunks: &[Hunk]) -> Vec<Slice>  // Wire in FullFlow
fn thin_slice(ast: &AST, diff_hunks: &[Hunk]) -> Vec<Slice>     // More focused than LeftFlow
fn barrier_slice(ast: &AST, symbol: &str, barriers: &[Barrier]) -> CallGraph  // Bounded depth

// Phase 2: Security and data flow
fn chop(ast: &AST, source: &Location, sink: &Location) -> Vec<Statement>
fn taint_trace(ast: &AST, taint_source: &Location, depth: u32) -> Vec<TaintPath>

// Phase 3: Advanced
fn conditioned_slice(ast: &AST, statement: &Location, condition: &Predicate) -> Vec<Slice>
fn relevant_slice(ast: &AST, statement: &Location) -> Vec<Slice>
fn delta_slice(old_ast: &AST, new_ast: &AST, behavioral_diff: &Diff) -> Vec<Change>
fn circular_deps_dataflow(ast: &AST, changed_nodes: &[Node]) -> Vec<Cycle>  // See §5.2
```

---

## 5. Program Slicing: Theoretical Extensions

The following concepts extend the established slicing taxonomy into areas that map to genuine analytical needs but lack formal treatment in the literature. Several of these represent potentially publishable contributions.

### 5.1 Spiral Slice — Adaptive-Depth Review Context

**Concept:** Start narrow at the change point and progressively widen through concentric rings, each adding one more layer of context. Stop when the analysis at the current ring reveals no new concerns.

**Rings:**
1. Changed statement only
2. Enclosing function body
3. Direct callers and callees (signatures + guards)
4. Callers of callers, callees of callees (depth 2)
5. Test files exercising these paths
6. Related modules and shared utilities

**Why it matters:** This formalizes adaptive depth for the chunking architecture. The primary chunk is ring 2 (the function). Reference context is ring 3 (direct callers/callees). Deeper rings are available via tool read only when the reviewer identifies a concern that requires tracing further. This is more efficient than fixed-depth tracing because the analysis goes exactly as deep as the code demands.

**Connection to existing plans:** The parallel reviewer-per-chunk model gives each reviewer rings 1-2 as primary focus and ring 3 as reference. The spiral formalization suggests the reviewer should be explicitly prompted to request deeper rings when needed, rather than pre-loading all context.

**Novelty assessment:** High. No formal slicing type captures iterative, concern-driven depth expansion. The closest concept is "demand-driven analysis" in static analysis literature, but applied to LLM context packaging it's novel.

### 5.2 Circular Slice — Data Flow Cycle Detection

**Concept:** Follow data flow across function boundaries until the trace cycles back to the origin. Detect cycles in the def-use graph that cross function or module boundaries.

**Bug class caught:** State management cycles (React component A updates state → triggers B → B dispatches action → updates A), event cascades (handler emits event → caught by another handler → emits event caught by first handler), and mutual recursion with state mutation.

**Distinction from existing tools:** madge and dependency-cruiser detect circular *imports* (structural). Circular slicing detects circular *data flow* (semantic) — values that propagate through a cycle and may accumulate unintended transformations.

**Novelty assessment:** High. Data flow cycle detection across function boundaries exists in the static analysis literature but hasn't been applied to code review context or LLM input preparation.

### 5.3 Quantum Slice — Concurrent State Superposition

**Concept:** For concurrent/async code, enumerate all possible states a variable could hold at a given program point considering all possible interleavings of concurrent operations.

**Example:** After a `useEffect` with an async fetch and cleanup:
```
deviceList could be:
  State A: fetched array (fetch completed before cleanup)
  State B: undefined (component unmounted, cleanup ran)
  State C: stale previous value (fetch in flight, hasn't resolved)
```

Only State A is handled → finding: "deviceList is in superposition; States B and C are unhandled."

**Connection to survey:** The NACD paper (ECOOP 2025) uses delay injection to expose which concurrent states actually cause failures. Quantum slicing provides the *enumeration* of possible states; delay injection provides the *testing* of each state. Together they form a complete concurrency analysis: enumerate → test → report.

**Novelty assessment:** Very high. Formal models of concurrent state exist in the verification literature (model checking, Petri nets) but framing them as "slices" for LLM review context is novel. The practical version for async JavaScript/Python would be a significant contribution.

### 5.4 Horizontal Slice — Peer Pattern Consistency

**Concept:** Given a change in one code construct, slice to all constructs at the same abstraction level that should follow the same patterns.

**Examples:**
- Change one route handler → horizontal slice to all route handlers
- Change one React component's error handling → slice to sibling components
- Change one test file's setup pattern → slice to peer test files
- Change one API endpoint's validation → slice to all endpoints in the same controller

**Connection to existing plans:** Directly enables omission detection (spider web §1.1). The horizontal slice provides the comparison set: "this handler has validation; these three don't." Also supports the pattern consistency aspect of the behavioral_diff frame.

**Implementation:** Tree-sitter identifies constructs of the same "type" (same decorator pattern, same class hierarchy, same file naming convention, same directory). Repo metadata and rubrics encode which patterns should be consistent across peers.

**Novelty assessment:** Medium. Pattern consistency checking exists in static analysis (coding standard enforcement). Framing it as a "slice" that provides the LLM with the peer comparison set for review is a novel application.

### 5.5 Vertical Slice — End-to-End Feature Path

**Concept:** Trace the complete path from user input to persistent output for the feature being modified. Show every layer a request touches.

**Example for a device configuration change:**
```
HTTP Request → Route handler (validation) → Service layer (business logic) → 
Database layer (persistence) → Event emission → Notification service → 
HTTP Response
```

**Connection to existing plans:** This is the cross-boundary bug detection need formalized. Extends caller/callee analysis to the full architectural depth. Requires repo metadata declaring architectural layers or inference from directory structure.

**Novelty assessment:** Low-medium. Vertical slice architecture is an established software design pattern. Applying it as a formal slicing concept for review context is a modest extension.

### 5.6 Angle Slice — Cross-Cutting Concern Trace

**Concept:** Trace how a specific concern (error handling, logging, authentication, caching) cuts diagonally across the architecture, following neither the call graph vertically nor the peer set horizontally.

**Example:** Error handling for a specific error type:
```
Database layer: throws ConnectionError
Service layer: catches ConnectionError, wraps in ServiceUnavailableError
Middleware: catches ServiceUnavailableError, logs, sets retry header
Route handler: catches ServiceUnavailableError, returns 503
Frontend: receives 503, shows retry UI
```

A change to error handling in any one layer affects the entire diagonal.

**Connection to existing plans:** The cognitive frames are effectively angle slices — contract_audit cuts across layers checking interfaces, boundary_probe cuts across module boundaries. Formalizing angle slicing provides a systematic way to identify which cross-cutting concerns are affected by a diff.

**Novelty assessment:** Medium-high. Aspect-oriented programming addresses cross-cutting concerns at the design level. Extracting them as analysis slices for code review is a novel application.

### 5.7 3D Slice — Temporal-Structural Integration

**Concept:** Combine structural analysis (who calls what, at what depth) with temporal analysis (how recently was this changed, how often does it change, who else is changing it) into a unified three-dimensional model.

**Three axes:**
- **X: Structural breadth** — horizontal (peer) relationships
- **Y: Structural depth** — vertical (caller/callee) relationships  
- **Z: Temporal** — change history, recent modifications, open MRs

**A 3D slice at a point in the codebase shows:**
- Structurally: all callers and callees (existing 2D analysis)
- Temporally: churn rate (code-maat), recent modifications (GitLab API), active MRs touching this area
- Risk score: f(structural_coupling × temporal_activity × change_complexity)

**Connection to existing plans:** This unifies code-maat (temporal axis), caller/callee analysis (structural axes), and temporal MR interaction analysis (future projection) into a single model. The 3D slice provides a natural risk prioritization: code at the intersection of high structural coupling AND high temporal activity AND complex current change gets the deepest review.

**Novelty assessment:** High. Individual axes are well-studied. The three-dimensional integration as a formal slicing concept for review prioritization is novel and potentially publishable.

---
