# Prism: Follow-up Tasks

**Updated:** April 6, 2026
**Baseline:** PR 64 merged (1,223 tests, 27 algorithms, 11 languages)

-----

## 1. Current State

|Metric                           |Value                                         |
|---------------------------------|----------------------------------------------|
|Algorithms                       |27                                            |
|Tests                            |1,223 (100% passing, zero warnings)           |
|Source lines                     |~28,000                                       |
|Test lines                       |~37,000                                       |
|Languages with ≥50% algo coverage|11/11                                         |
|Type providers                   |7 (C++, Go, TypeScript, Java, Rust, Python, C)|
|RTA live type collection         |✅ All 7 dispatch languages                    |
|Interprocedural taint            |✅ Crosses function boundaries via DFG         |
|Import-aware resolution          |✅ Python, JS/TS, Go                           |
|Scoped CPG                       |✅ `--scoped-cpg` flag                         |
|CPG DFG integration              |✅ left_flow, full_flow field-sensitive        |
|CFG integration                  |✅ relevant_slice, conditioned_slice           |
|Contract analysis                |Phase 1 (preconditions) shipped               |
|Parse quality reporting          |✅ Per-file and per-finding grades             |
|Struct callback resolution       |✅ Level 4 call graph resolution               |
|Goto error path tracking         |✅ Absence slice handles goto cleanup          |
|CVE pattern test fixtures        |✅ 17 real-world vulnerability patterns        |

-----

## 2. Remaining Work

### 2.1 E13: Serialized CPG for Caching ← HIGHEST PRIORITY

**Impact:** High — production latency blocker
**Effort:** 1–2 weeks
**Depends on:** Nothing (all CPG infrastructure is stable)

See §5 for full implementation plan.

### 2.2 Contract Slice Phase 2: Postcondition Extraction

**Impact:** Medium — completes the contract model
**Effort:** 1 week

Analyze all return statements in a function. Classify return value patterns
(always non-null, consistent type, raises-on-error). Flag when a diff
introduces a new `return None` path or changes the return type on one branch.

### 2.3 Contract Slice Phase 3: Delta Contract Comparison

**Impact:** High — catches contract weakening
**Effort:** 1–2 weeks
**Depends on:** Phase 2 (2.2)

Compare pre-change and post-change contracts using `--old-repo`. Flag cases
where a precondition is weakened or a postcondition is broken.

### 2.4 MCP Server Mode (6.3)

**Impact:** High — enables interactive CPG queries during review
**Effort:** 2–3 weeks

Wrap `CpgContext` in a persistent server process. Expose tools like
`query_callers(function, depth)`, `taint_from(file, line)`,
`slice(algorithm, diff)`, `find_contracts(function)`.

-----

## 3. Follow-on Enhancements

These emerged from the E12 type system work and recent PRs. Not blocking
production but improve precision and unlock new capabilities.

### 3.1 Type-Enriched Finding Descriptions

**Impact:** Medium — improves LLM reviewer’s severity assessment
**Effort:** 1 week
**Depends on:** E12 ✅ (all providers ready)

Findings currently describe code patterns. With type info available, findings
can include type context: “taint flows from user_input (string) through
db.query(sql: string) — no parameterized query enforcement.”

Designed in E12 spec §9. Candidate algorithms: Taint, Contract, Echo,
Absence (suppress RAII findings via Drop detection), Membrane.

### 3.2 CompatibilitySlice Algorithm

**Impact:** Medium — catches version-incompatible code
**Effort:** 1 week (Phase 1), 2–3 weeks (Phase 2)
**Depends on:** E12 ✅ (target_version config wired up)

Phase 1: Cross-reference AST patterns and stdlib calls against a per-language
TOML feature matrix. Flags Python `match` with `--python-version 3.8`, etc.

Phase 2: Type definition delta comparison across library versions. Catches
React 18 `FC<Props>` losing `children`, Express 5 middleware signature changes.

Designed in E12 spec §8.

### 3.3 `.d.ts` Parsing for TypeScript

**Impact:** Medium — resolves library types (React, Express, Hapi)
**Effort:** 1 week

TS provider currently extracts types from source files only. Adding
`node_modules/@types/` parsing gives `React.FC<Props>`, `Request`,
`Response` interfaces. Eager mode (~2–3 seconds for full parse). Uses import
map from E6 to scope packages in scoped mode.

### 3.4 Import Map Improvements

**Impact:** Medium — improves cross-file accuracy
**Effort:** 1 week

E6 import map doesn’t handle: re-exports (`export { X } from './other'`),
barrel files (`index.ts`), TypeScript path aliases (`@/utils` via
`tsconfig.json paths`), dynamic imports. Benefits both type resolution and
call graph accuracy.

### 3.5 Serialized Type Cache (E13 Extension)

**Impact:** Low-medium — saves ~2–3 seconds on cached reviews
**Effort:** 3–5 days
**Depends on:** E13 ✅, E12 ✅

Serialize TypeRegistry data alongside the CPG. Per-file content hash
invalidation. Phase 1 of E13 rebuilds TypeRegistry from files (cheap);
this extension eliminates even that cost.

-----

## 4. Prioritized Execution Plan

### Immediate (Next 1–2 Weeks)

|Priority|Task                           |Effort   |
|--------|-------------------------------|---------|
|1       |E13: Serialized CPG for caching|1–2 weeks|

### Short-term (Next Month)

|Priority|Task                              |Effort   |
|--------|----------------------------------|---------|
|1       |Contract Phase 2: Postconditions  |1 week   |
|2       |Type-enriched finding descriptions|1 week   |
|3       |Contract Phase 3: Delta comparison|1–2 weeks|

### Medium-term (Next Quarter)

|Priority|Task                          |Effort   |
|--------|------------------------------|---------|
|1       |MCP Server Mode (6.3)         |2–3 weeks|
|2       |`.d.ts` parsing for TypeScript|1 week   |
|3       |CompatibilitySlice Phase 1    |1 week   |
|4       |Import map improvements       |1 week   |

### Parallelization

```
Track A: E13 (serialized CPG) → E13 extension (type cache)
Track B: Contract Phase 2 → Contract Phase 3
Track C: Type-enriched findings (independent)

E13 has zero conflicts with any other track.
Contract and type-enriched findings can run concurrently.
MCP server mode is standalone — start whenever capacity allows.
```

-----

## 5. E13 Implementation Plan: Serialized CPG for Caching

### 5.1 Problem

Every review rebuilds the full CPG from scratch. For a typical repo (50–200
files), CPG construction takes 5–30 seconds:

- tree-sitter parsing: ~1–2 seconds
- Call graph construction: ~1–2 seconds
- DFG construction: ~2–5 seconds
- CPG graph assembly (9 steps): ~2–10 seconds
- Type provider construction: ~2–3 seconds
- CFG construction: ~1–3 seconds

For incremental reviews where only 3–5 files changed, 95% of this work is
wasted — the other 195 files produce identical graph nodes and edges.

### 5.2 Design

**Cache unit:** One serialized CPG per repository, stored in a configurable
cache directory. The cache contains the full `CodePropertyGraph` (petgraph
graph + indexes) and per-file metadata for invalidation.

**Invalidation:** Per-file content hash (SHA-256 of file contents). When a
file changes, its nodes, edges, and contributions to the call graph / DFG
are invalidated. Unchanged files retain their cached graph fragments.

**TypeRegistry:** NOT cached in Phase 1. Rebuilt from parsed files on every
review (~2–3 seconds). Cached in the follow-on extension (§3.5) once the
provider set is stable.

**CLI:**

```bash
# First review: builds CPG, saves cache
slicing --repo . --diff changes.patch --cache-dir .prism-cache

# Second review: loads cache, rebuilds only changed files
slicing --repo . --diff changes2.patch --cache-dir .prism-cache

# Force full rebuild (ignore cache)
slicing --repo . --diff changes.patch --cache-dir .prism-cache --no-cache
```

### 5.3 Serialization Format

**bincode** (not FlatBuffers). Rationale:

- Prism already has `serde` as a dependency. bincode adds one crate.
- FlatBuffers requires schema files, code generation, and a build step.
  bincode is `#[derive(Serialize, Deserialize)]` on existing types.
- The CPG is a Rust-only internal cache, not a cross-language interchange
  format. bincode’s Rust-native ergonomics are the right tradeoff.
- Performance: bincode serializes/deserializes at ~1 GB/s. A typical CPG
  (50K nodes, 200K edges) is ~10–50 MB serialized → 10–50 ms to load.

### 5.4 What Gets Serialized

**Full serialization (Phase 1 — simpler, slower invalidation):**

```rust
#[derive(Serialize, Deserialize)]
struct CpgCache {
    /// Cache format version. Invalidate entire cache if mismatched.
    version: u32,
    /// Per-file content hashes at time of cache creation.
    file_hashes: BTreeMap<String, String>,
    /// The serialized CPG graph.
    graph: SerializedCpg,
    /// Prism version string (invalidate on upgrade).
    prism_version: String,
}

#[derive(Serialize, Deserialize)]
struct SerializedCpg {
    /// petgraph node data + edge data (serialized via node/edge list).
    nodes: Vec<CpgNode>,
    edges: Vec<(u32, u32, CpgEdge)>,
    /// Indexes (rebuilt from graph on deserialize, or serialized directly).
    func_index: BTreeMap<(String, String), u32>,
    /// Call graph data.
    call_graph: SerializedCallGraph,
    /// DFG edges and indexes.
    dfg: SerializedDfg,
}
```

petgraph’s `DiGraph` doesn’t implement `Serialize` directly. The standard
approach: serialize as node list + edge list (with node indices as u32),
reconstruct the `DiGraph` on load. This is a well-established pattern in
the petgraph ecosystem.

### 5.5 Types That Need `Serialize + Deserialize`

Add `#[derive(Serialize, Deserialize)]` to:

|Type           |File            |Current Derives                                         |
|---------------|----------------|--------------------------------------------------------|
|`CpgNode`      |`cpg.rs`        |Debug, Clone, PartialEq, Eq                             |
|`CpgEdge`      |`cpg.rs`        |Debug, Clone, PartialEq, Eq                             |
|`VarAccess`    |`cpg.rs`        |Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash|
|`StmtKind`     |`cpg.rs`        |Debug, Clone, PartialEq, Eq                             |
|`AccessPath`   |`access_path.rs`|Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash      |
|`FunctionId`   |`call_graph.rs` |Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash      |
|`CallSite`     |`call_graph.rs` |Debug, Clone, PartialEq, Eq                             |
|`CallGraph`    |`call_graph.rs` |Debug                                                   |
|`VarLocation`  |`data_flow.rs`  |Debug, Clone, PartialEq, Eq, PartialOrd, Ord            |
|`VarAccessKind`|`data_flow.rs`  |Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord      |
|`DataFlowGraph`|`data_flow.rs`  |Debug                                                   |
|`FlowEdge`     |`data_flow.rs`  |(check)                                                 |

All of these are composed of `String`, `usize`, `Vec`, `BTreeMap`, and
`BTreeSet` — all serde-serializable. No custom serialization logic needed.

The `TypeDatabase` inside `CodePropertyGraph` also needs serde derives, but
since TypeRegistry is NOT cached in Phase 1, the `type_db` field can be
`#[serde(skip)]` and rebuilt on load.

### 5.6 Cache Load Path

```
1. Read CpgCache from disk
2. Verify version == current version
3. Verify prism_version == current prism version
4. For each file in the repo:
   a. Compute SHA-256 of file contents
   b. Compare against file_hashes in cache
5. If ALL hashes match:
   → Reconstruct DiGraph from serialized nodes/edges
   → Reconstruct indexes (func_index, var_index, location_index)
   → Rebuild TypeRegistry from parsed files (cheap)
   → Return CpgContext with cached CPG + fresh TypeRegistry
6. If ANY hash mismatches:
   → Full rebuild (Phase 1 — no incremental)
   → Write new cache to disk
```

### 5.7 Phase 2: Incremental Cache Update

Phase 1 does all-or-nothing: if any file changed, full rebuild. Phase 2
adds incremental update:

```
1. Identify changed files (hash mismatch)
2. Remove all nodes/edges originating from changed files
3. Re-parse changed files
4. Re-run DFG, call graph, CFG for changed files
5. Merge new nodes/edges into cached graph
6. Recompute cross-file edges (interprocedural DFG, call edges)
7. Write updated cache
```

This is architecturally harder because CPG construction doesn’t currently
track which nodes/edges came from which file. Phase 2 needs a
`source_file` field on nodes (already present in `CpgNode::Function` and
`CpgNode::Variable` but not on `CpgNode::Statement`) and a way to identify
cross-file edges that need recomputation.

**Recommendation:** Ship Phase 1 (all-or-nothing). Measure the hit rate on
your 6 repos. If most reviews have <5 changed files out of 50+ total, the
cache hit rate for Phase 1 may be low enough to justify Phase 2. If most
reviews touch the same 2–3 hot files, Phase 1’s full-rebuild-on-miss is
fine because the rebuild is fast.

### 5.8 Cache File Layout

```
.prism-cache/
  cpg-cache.bin          # bincode-serialized CpgCache
  cache-meta.json        # human-readable metadata (for debugging)
```

`cache-meta.json` contains: file count, node count, edge count, cache
creation timestamp, prism version, total serialized size. Not used by the
cache logic — purely for debugging and monitoring.

### 5.9 Implementation Phases

#### Phase 1: Serde derives + serialization (3–4 days)

1. Add `bincode` to `Cargo.toml`.
1. Add `Serialize, Deserialize` derives to all types in §5.5.
1. Implement `CpgCache` struct with `save()` and `load()` methods.
1. Serialize: extract node list + edge list from `DiGraph`, serialize
   with bincode.
1. Deserialize: reconstruct `DiGraph` from node/edge lists, rebuild
   indexes by iterating nodes.
1. Tests: round-trip a small CPG (build → serialize → deserialize →
   verify identical query results).

#### Phase 2: CLI integration + cache-dir (2–3 days)

1. Add `--cache-dir` and `--no-cache` CLI flags.
1. In `main.rs`: before `CpgContext::build()`, check for existing cache.
1. Compute file hashes, compare against cached hashes.
1. On hit: load cached CPG, build fresh TypeRegistry, construct CpgContext.
1. On miss: full build, save new cache.
1. Tests: integration test with temp cache dir, verify cache hit on
   second run.

#### Phase 3: `type_db` handling (1 day)

1. Mark `type_db: Option<TypeDatabase>` as `#[serde(skip)]` in
   `CodePropertyGraph`.
1. On cache load, if `--compile-commands` is provided: rebuild
   TypeDatabase from compile_commands.json (already fast) and inject
   into the loaded CPG.
1. If not provided: `type_db` stays `None` (same as non-cached path).

#### Phase 4: Performance validation (1–2 days)

1. Benchmark: full build time vs cache load time on each of your 6 repos.
1. Measure: cache file sizes, serialization time, deserialization time.
1. Measure: hash computation time for 50–200 files.
1. Document results. If cache load is <500ms for unchanged repos, Phase 1
   is sufficient. If cache miss rate is >50% across typical reviews,
   evaluate Phase 2 (incremental).

### 5.10 Risk Assessment

|Risk                                                     |Severity|Mitigation                                                                |
|---------------------------------------------------------|--------|--------------------------------------------------------------------------|
|petgraph NodeIndex stability across serialize/deserialize|Medium  |Serialize as u32, rebuild DiGraph with same insertion order               |
|Cache corruption from interrupted writes                 |Low     |Write to temp file, atomic rename                                         |
|Cache invalidation miss (stale data served)              |Medium  |Hash ALL files, not just changed ones. Version tag catches schema changes.|
|bincode format changes across versions                   |Low     |Pin bincode version. Cache version tag forces rebuild on mismatch.        |
|Large cache files for big repos                          |Low     |Typical CPG is 10–50 MB. Compress with zstd if >100 MB.                   |

### 5.11 Expected Impact

|Scenario                             |Without Cache|With Cache (Phase 1)                   |
|-------------------------------------|-------------|---------------------------------------|
|First review of repo                 |5–30 seconds |5–30 seconds (cache write adds ~100ms) |
|Second review, no changes            |5–30 seconds |~200ms (cache load + hash check)       |
|Review with 3 changed files          |5–30 seconds |5–30 seconds (full rebuild, cache miss)|
|Review with 3 changed files (Phase 2)|5–30 seconds |~1–2 seconds (incremental)             |

Phase 1 gives the biggest win for the most common scenario: developer runs
the review agent on the same repo repeatedly during iteration. The cache
persists across reviews. The first review pays the full build cost; every
subsequent review of an unchanged codebase loads from cache in ~200ms.

-----

## 6. Production Integration Readiness

The CLI path works today:

```bash
slicing --repo . --diff patch.diff --algorithm review --scoped-cpg --format json
```

**Ready:**

- 27-algorithm `review` suite
- Scoped CPG for large repos
- Import-aware cross-file resolution (Python, JS/TS, Go)
- Type-aware dispatch (7 language providers + RTA)
- Interprocedural taint through function arguments
- Field-sensitive DFG in left_flow/full_flow
- CFG-based alternate path and unreachability analysis
- Parse quality annotations on findings
- Struct callback resolution in call graph
- Goto error path tracking in absence detection
- JSON output with blocks + findings

**Gaps for production:**

- **E13 (serialized CPG)** — without it, every review rebuilds the graph
- **MCP or persistent process** — for interactive queries (optional; CLI
  works for batch mode)

-----

## Appendix: Completed Items

### PRs Merged (Full Session)

|PR|Title                                     |Items Closed                                      |
|--|------------------------------------------|--------------------------------------------------|
|42|Fix analysis bugs B1–B10                  |E1, E2, E4, E5, B1–B7, B9, B10                    |
|44|E9 Phase 1 + E10                          |E9 Phase 1, E10                                   |
|45|E3 + E8                                   |E3, E8                                            |
|46|E9 Phases 2–4 + ContractSlice             |E9 Phases 2–4, ContractSlice                      |
|47|Test coverage expansion                   |74 new tests, 7 languages to ≥50%                 |
|48|E6: Import-aware call resolution          |E6                                                |
|49|Follow-up fixes 2.1–2.6                   |TypeCheck warning, Yoda, LeftFlowResult, etc.     |
|50|Behavioral tests + B8 fix                 |B8, TEST_GAPS Tier 1                              |
|51|Complete TEST_GAPS Tier 2–3               |All 16 test gap items                             |
|52|E7 + E11: CPG DFG + CFG integration       |E7, E11                                           |
|53|E12 Phase 1: Trait plumbing               |TypeProvider traits, CppTypeProvider, TypeRegistry|
|54|E12 Phase 2: GoTypeProvider               |Go interface satisfaction, Arc fix                |
|55|E12 Phase 3: TypeScriptTypeProvider       |Structural typing, extends chain fix              |
|56|E12 Phase 4: JavaTypeProvider             |Class hierarchy dispatch                          |
|57|E12 Phase 5: RustTypeProvider             |Trait dispatch, impl block extraction             |
|58|E12 Phase 6: PythonTypeProvider           |PEP 484 annotation extraction                     |
|59|CppTypeProvider test coverage             |30 tests for >95% coverage                        |
|60|E12 Phase 7: Multi-language RTA           |Live type collection across all 7 languages       |
|61|Fix static function disambiguation        |Call graph query correctness for static functions |
|62|Struct callback resolution + parse quality|Level 4 call graph, per-finding quality grades    |
|63|Goto error path tracking                  |AbsenceSlice handles goto-based cleanup paths     |
|64|CVE pattern test fixtures                 |17 real-world vulnerability patterns              |

### Enhancement Scorecard

|ID |Enhancement                              |Status         |
|---|-----------------------------------------|---------------|
|E1 |Extract left_flow_core                   |✅ PR 42        |
|E2 |Fix dead cross-file block                |✅ PR 42        |
|E3 |AST-based absence detection              |✅ PR 45        |
|E4 |Hoist echo_slice allocations             |✅ PR 42        |
|E5 |Fix resonance ratio                      |✅ PR 42        |
|E6 |Import resolution                        |✅ PR 48        |
|E7 |left_flow/full_flow to CPG DFG           |✅ PR 52        |
|E8 |Interprocedural taint                    |✅ PR 45        |
|E9 |Query-based matching (Phases 1–4)        |✅ PR 44, 46    |
|E10|Scoped CPG                               |✅ PR 44        |
|E11|CFG into relevant/conditioned            |✅ PR 52        |
|E12|Multi-language type system (all 7 phases)|✅ PR 53–60     |
|E13|Serialized CPG for caching               |**Not started**|

### Bug Scorecard

All 10 bugs (B1–B10) fixed. See earlier doc version for details.

### Test Growth

|Milestone                                |Tests|
|-----------------------------------------|-----|
|Session start (PR 42 base)               |699  |
|After E9/E10 (PR 44)                     |751  |
|After test coverage expansion (PR 47)    |858  |
|After follow-ups (PR 49)                 |875  |
|After TEST_GAPS complete (PR 51)         |916  |
|After E12 Phase 1 (PR 53)                |916  |
|After E12 Phase 2 Go (PR 54)             |934  |
|After E12 Phase 3 TS (PR 55)             |958  |
|After E12 Phases 4–7 + extras (PRs 56–64)|1,223|
