# Implementation Plan — Extended Slicing Algorithms

## Status Key

- [ ] Not started
- [~] In progress
- [x] Complete
- [!] Blocked / needs investigation

---

## Phase 1: Section 4 — Established Taxonomy (Practical)

### 1.1 Thin Slice
**Data deps only, no control deps. More focused than LeftFlow.**

- [x] Create `src/algorithms/thin_slice.rs`
- [x] Trace only assignment data flow (L-value → definition → other uses)
- [x] Explicitly exclude control flow conditions, branch boundaries, return statements
- [x] No enclosing `if`/`for`/`while` context — just the data chain
- [x] Add `ThinSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: ThinSlice is strict subset of LeftFlow, has data deps

### 1.2 Barrier Slice
**Interprocedural with explicit depth/boundary controls.**

- [x] Create `src/algorithms/barrier_slice.rs`
- [x] Add `BarrierConfig`: max call depth, barrier symbols, barrier modules
- [x] Implement depth-limited caller analysis via call graph
- [x] Implement depth-limited callee analysis via call graph
- [x] Respect barriers — stop tracing when hitting a barrier symbol or module
- [x] CLI flags: `--barrier-depth`, `--barrier-symbols`
- [x] Tests: basic barrier slice

### 1.3 Chopping
**All paths between a source location and a sink location.**

- [x] Create `src/algorithms/chop.rs`
- [x] Build intraprocedural def-use graph via `DataFlowGraph`
- [x] Given source and sink, find all statements on data-flow paths between them
- [x] Handle transitive flow via `forward_reachable` / `backward_reachable` intersection
- [x] CLI: `--chop-source file:line --chop-sink file:line`

### 1.4 Taint Analysis
**Forward trace of values from specified taint sources through the program.**

- [x] Create `src/algorithms/taint.rs`
- [x] Forward propagation via `DataFlowGraph.taint_forward()`
- [x] Built-in sink patterns: exec/eval, SQL, file ops, HTTP responses
- [x] Auto-taint from diff lines when no explicit sources given
- [x] CLI: `--taint-source file:line`
- [x] Tests: taint from diff

### 1.5 Relevant Slice
**Backward slice + potential branch flips — "one flip away from a bug."**

- [x] Create `src/algorithms/relevant_slice.rs`
- [x] Start with LeftFlow output as base
- [x] For each control flow branch in the slice, include alternate paths (else, elif, default)
- [x] Include first N lines of alternate paths
- [x] Detect missing else clauses
- [x] Tests: RelevantSlice >= LeftFlow in line count

### 1.6 Conditioned Slice
**Slice under a specific assumption about a variable's value.**

- [x] Create `src/algorithms/conditioned_slice.rs`
- [x] Parse condition predicates: `var==value`, `var!=null`, `var>0`, etc.
- [x] LeftFlow base with branch pruning for unreachable paths under the condition
- [x] CLI: `--condition "var==value"`
- [x] Tests: condition parsing

### 1.7 Delta Slice
**Minimal set of changes causing behavioral difference between two program versions.**

- [x] Create `src/algorithms/delta_slice.rs`
- [x] Parse both old and new versions of changed files
- [x] Build def-use graphs for both versions
- [x] Diff the edge sets to find changed data flow paths
- [x] CLI: `--old-repo path`

---

## Phase 2: Section 5 — Theoretical Extensions (Research)

### 2.1 Spiral Slice — Adaptive-Depth Review Context
**Progressive widening through concentric rings.**

- [x] Create `src/algorithms/spiral_slice.rs`
- [x] Ring 1: OriginalDiff
- [x] Ring 2: ParentFunction (enclosing function)
- [x] Ring 3: LeftFlow + direct caller/callee signatures
- [x] Ring 4: Depth-2 callers/callees
- [x] Ring 5: Test files referencing changed functions
- [x] Ring 6: Shared utilities imported by multiple changed files
- [x] Auto-stop when ring delta is below threshold
- [x] CLI: `--spiral-max-ring N`
- [x] Tests: ring containment (spiral >= originaldiff)

### 2.2 Circular Slice — Data Flow Cycle Detection
**Detect cycles in cross-function data flow.**

- [x] Create `src/algorithms/circular_slice.rs`
- [x] Call graph cycle detection (DFS with back-edge)
- [x] Data flow cycle detection within functions
- [x] Focus on diff-reachable cycles
- [x] Tests: mutual recursion detection

### 2.3 Quantum Slice — Concurrent State Enumeration
**Enumerate possible states considering async/concurrent interleavings.**

- [x] Create `src/algorithms/quantum_slice.rs`
- [x] Async pattern detection per language (await, go, CompletableFuture, etc.)
- [x] Variable assignment enumeration within async functions
- [x] Model possible orderings around async boundaries
- [x] CLI: `--quantum-var varname`
- [x] Tests: async JS parsing

### 2.4 Horizontal Slice — Peer Pattern Consistency
**Slice to all constructs at the same abstraction level.**

- [x] Create `src/algorithms/horizontal_slice.rs`
- [x] Auto-detect peer pattern (decorators, naming, directory siblings)
- [x] Find all peer functions across the repository
- [x] Include changed function + peer previews
- [x] CLI: `--peer-pattern "decorator:@app.route"`
- [x] Tests: handler peers found

### 2.5 Vertical Slice — End-to-End Feature Path
**Trace the complete path from input to output across architectural layers.**

- [x] Create `src/algorithms/vertical_slice.rs`
- [x] Heuristic layer detection from directory paths (20 patterns)
- [x] Bidirectional tracing: callers up, callees down
- [x] Manual layer specification support
- [x] CLI: `--vertical`, `--layers "routes,services,models,db"`
- [x] Tests: basic vertical trace

### 2.6 Angle Slice — Cross-Cutting Concern Trace
**Follow a specific concern diagonally across the architecture.**

- [x] Create `src/algorithms/angle_slice.rs`
- [x] Built-in concerns: error handling, logging, auth, caching
- [x] Custom concerns via keyword lists
- [x] Trace concern patterns across all files with enclosing function context
- [x] CLI: `--concern "error_handling"`
- [x] Tests: error handling trace

### 2.7 3D Slice — Temporal-Structural Integration
**Combine structural analysis with git temporal data.**

- [x] Create `src/algorithms/threed_slice.rs`
- [x] Structural coupling via call graph (callers + callees count)
- [x] Temporal activity via `git log` churn data
- [x] Risk scoring: `structural_coupling * temporal_activity * change_complexity`
- [x] Output sorted by risk score descending
- [x] CLI: `--3d`, `--temporal-days N`

---

## Phase 2 Extended: Novel Extensions

### 2.8 Absence Slice — Missing Counterpart Detection
**Detect obligations code hasn't fulfilled: open without close, lock without unlock.**

- [x] Create `src/algorithms/absence_slice.rs`
- [x] Detect open/close, lock/unlock, subscribe/unsubscribe, alloc/free patterns
- [x] Flag instances where one side is present without the other
- [x] Add `AbsenceSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: absence detection for resource pairs

### 2.9 Resonance Slice — Change Coupling from Git History
**Files that usually change together — are any missing from this diff?**

- [x] Create `src/algorithms/resonance_slice.rs`
- [x] Analyze git co-change frequency for files in the diff
- [x] Flag files with high co-change frequency that are missing from the diff
- [x] Add `ResonanceSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: co-change detection

### 2.10 Symmetry Slice — Broken Symmetry Detection
**If one side of a symmetric pair changed, is the other side still consistent?**

- [x] Create `src/algorithms/symmetry_slice.rs`
- [x] Detect symmetric pairs: serialize/deserialize, encode/decode, toJSON/fromJSON, etc.
- [x] Flag when one side is changed without the other
- [x] Add `SymmetrySlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: symmetry pair detection

### 2.11 Gradient Slice — Continuous Relevance Scoring
**How relevant is each line to the change, on a continuous scale?**

- [x] Create `src/algorithms/gradient_slice.rs`
- [x] Compute decaying relevance scores instead of binary include/exclude
- [x] Score based on data-flow distance, control-flow distance, and call depth
- [x] Add `GradientSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: gradient score ordering

### 2.12 Provenance Slice — Data Origin Tracing
**Where did this data come from, and does that origin require special handling?**

- [x] Create `src/algorithms/provenance_slice.rs`
- [x] Classify data origins: user_input, config, database, constant, env_var, function_param, external_call
- [x] Trace origin through assignment chains
- [x] Add `ProvenanceSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: origin classification

### 2.13 Phantom Slice — Recently Deleted Code Surfacing
**Is there recently deleted code that this change might unknowingly depend on?**

- [x] Create `src/algorithms/phantom_slice.rs`
- [x] Use git history to find recently deleted functions/variables
- [x] Flag references to deleted code in the current diff
- [x] Add `PhantomSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: deleted code detection

### 2.14 Membrane Slice — Module Boundary Impact
**Who depends on this API, and will they break if its contract changes?**

- [x] Create `src/algorithms/membrane_slice.rs`
- [x] Identify changed functions that are part of a module's public API
- [x] Find all cross-file callers of changed functions
- [x] Add `MembraneSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: cross-module caller detection

### 2.15 Echo Slice — Ripple Effect Modeling
**If this change subtly alters behavior, what downstream code would break?**

- [x] Create `src/algorithms/echo_slice.rs`
- [x] Trace callers of changed functions
- [x] Flag callers missing error handling or null checks for changed return values
- [x] Add `EchoSlice` to `SlicingAlgorithm` enum, wire into dispatcher and CLI
- [x] Tests: ripple effect caller analysis

---

## Phase 3: Infrastructure

### 3.1 Call Graph Builder

- [x] Create `src/call_graph.rs`
- [x] Forward call graph: function → called functions
- [x] Reverse call graph: function → callers
- [x] Cross-file resolution
- [x] Depth-limited BFS traversal (callers_of, callees_of)
- [x] Cycle detection (find_cycles_from)

### 3.2 Data Flow Graph Builder

- [x] Create `src/data_flow.rs`
- [x] Intraprocedural def-use chains
- [x] Forward/backward adjacency maps
- [x] Transitive reachability (forward_reachable, backward_reachable)
- [x] Chopping (source-sink path intersection)
- [x] Taint forward propagation

### 3.3 Extended CLI and Config

- [x] All 26 algorithms accessible via CLI `--algorithm` flag
- [x] Algorithm-specific flags (barrier-depth, chop-source/sink, condition, etc.)
- [x] `--list-algorithms` flag
- [ ] Config file support (`.slicing.toml`) — deferred, not essential
- [ ] Multiple algorithms in a single invocation — deferred

---

## Test Summary

43 tests passing:
- 3 unit tests (diff parsing, original diff)
- 16 original integration tests (4 algorithms × multi-language)
- 24 new integration tests (thin, barrier, taint, relevant, spiral, circular, horizontal, vertical, angle, quantum, conditioned, call graph, data flow, algorithm listing, absence, resonance, symmetry, gradient, provenance, phantom, membrane, echo)
