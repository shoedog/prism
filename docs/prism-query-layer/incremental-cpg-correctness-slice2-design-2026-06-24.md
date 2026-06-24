# Incremental CPG Correctness Gate - Slice 2 Design

Status: Draft for A2A review
Date: 2026-06-24
Branch: `codex/incremental-cpg-correctness-spec`, based on `origin/main` at `042078d`
Predecessor: `docs/prism-query-layer/mcp-auto-refresh-incremental-correctness-design-2026-06-23.md`

## 1. Goal

Make Prism's existing CPG incremental rebuild path behavior-equivalent to a full rebuild for the same final
repository state.

Slice 1 added opt-in MCP `AutoFull` refresh by deliberately avoiding the CPG incremental path. Slice 2 is the
correctness gate that must land before any future MCP `AutoIncremental` policy or broader reliance on partial
CPG cache hits.

Success means:

- `CodePropertyGraph::build_incremental_with_scope_graph_inputs` produces the same normalized call graph, data
  flow, CPG call edges, and selected navigation query outputs as a full build for the final `files` map.
- C/C++ indirect-call recomputation no longer misses new synthetic edges or retains stale synthetic edges.
- Whole-graph recompute ordering in incremental matches full-build semantics.
- Any serialized `CallGraph` shape change is cache-versioned and tested.
- The implementation PR carries Tier-A evidence because this slice touches call resolution/CPG behavior.

## 2. Current Source State

### 2.1 Incremental cache path

The review CLI path in `src/main.rs` uses `CacheResult::PartialHit` this way:

- `cpg_cache::load_cache_with_topology` returns cached `CallGraph`, cached `DataFlowGraph`, and a complete
  `changed_files: BTreeSet<String>`.
- `CodePropertyGraph::build_incremental_with_scope_graph_inputs` receives the cached parts and the final parsed
  `files` map.
- The updated CPG is saved back to cache with the new hashes/topology key.

This path is already in production for review cache partial hits. Navigation cache partial hits remain
conservative elsewhere: `NavigationIndex::build_cached_at` treats partial hits as misses and does a full build.

### 2.2 Incremental build body

`src/cpg/build.rs` currently does:

1. `cached_cg.remove_files(changed_files)` and `cached_dfg.remove_files(changed_files)`.
2. `CallGraph::build_direct_subset(files, changed_files)` and `DataFlowGraph::build_subset(...)`.
3. `cached_cg.merge(fresh_cg)` and `cached_dfg.merge(fresh_dfg)`.
4. `cached_cg.apply_go_embedding_promotion(files)`.
5. `cached_cg.apply_go_interface_dispatch(files)`.
6. `cached_cg.rebuild_scope_graph(files, scope_inputs)`.
7. `CodePropertyGraph::assemble_graph(...)`.

`build_incremental` still documents that indirect call resolution is not rerun and recommends `--no-cache` for
C/C++ reviews with heavy function-pointer usage.

### 2.3 Full build order

`CallGraph::build` currently performs:

1. direct function/call extraction,
2. inline Phase 3 C/C++ indirect-call resolution,
3. construction of the final `CallGraph` with scope graph populated,
4. `refresh_rust_receiver_state(files)`,
5. `apply_go_embedding_promotion(files)`,
6. `apply_go_interface_dispatch(files)`.

The incremental path should converge on this order:

```text
remove changed files
build direct subset for changed files
merge fresh direct subset
recompute C/C++ indirect calls over the merged whole graph
rebuild scope graph and rematerialize Rust receiver state
apply Go embedding promotion
apply Go interface dispatch
assemble CPG
```

### 2.4 Known defect

`CallGraph::build_direct_subset` intentionally skips Phase 3. Its comment says callers should run an indirect
resolver after merging, but no such reusable resolver exists.

This creates two correctness gaps:

- **Recall gap:** changed C/C++ callers can introduce indirect-call patterns that incremental never resolves.
- **Precision gap:** unchanged callers can retain old synthetic indirect targets when a changed assignment,
  callback table, wrapper, or target function invalidates the old edge.

The second gap is the harder one: pruning only changed callers cannot remove stale derived sites that originate
from an unchanged caller but depend on changed data elsewhere.

## 3. Non-Goals

- Do not implement MCP `AutoIncremental` in this slice.
- Do not change `RefreshPolicy` or MCP transport behavior.
- Do not change the navigation cache policy that treats partial hits as full misses.
- Do not rewrite indirect-call semantics beyond making incremental match the current full build.
- Do not add new language features for C/C++ callback resolution beyond the cases full build already supports.
- Do not rebaseline Tier-A. Regressions or flip candidates belong in PR notes.
- Do not use node/edge count equality as the primary oracle.

## 4. Correctness Invariant

For a fixed final `files: BTreeMap<String, ParsedFile>`, optional `TypeDatabase`, and optional
`ScopeGraphBuildInputs`, these two builds must be behavior-equivalent:

```rust
let full = CodePropertyGraph::build_enriched_with_scope_graph_inputs(
    &files,
    type_db.as_ref(),
    scope_inputs,
);

let incremental = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
    cached_call_graph,
    cached_dfg,
    &changed_files,
    &files,
    type_db,
    scope_inputs,
);
```

The oracle must compare normalized behavior rather than raw `NodeIndex`:

- call graph functions by `(file, name, start_line, end_line)`;
- calls by caller plus every resolution-affecting `CallSite` field:
  - `callee_name`,
  - `line`,
  - `kind`,
  - `start_byte` / `end_byte`,
  - `qualifier`,
  - `receiver_type`,
  - `receiver_recovery`,
  - `receiver_materialized`,
  - `arg_count`,
  - `arg_spread`,
  - `receiver_outcome`,
  - and the new provenance field described below;
- reverse callers entries, including multiplicity and stable order after sorting normalized entries;
- resolved call behavior for all call sites where a resolver result is exposed;
- CPG call edges normalized to caller function, callee function, call-site line/span, and confidence where
  available;
- DFG defs/uses/edges normalized to stable value keys;
- selected navigation query outputs for callers/callees seeds that exercise the changed graph.

Do not omit a `CallSite` field just because it is excluded from `Ord` or `cmp_key`. Several fields are resolver
inputs, telemetry, or future diagnostic surface.

## 5. Architecture

### 5.1 Extract Phase 3 into a reusable recompute

Move the inline C/C++ indirect logic from `CallGraph::build` into an idempotent whole-graph method:

```rust
impl CallGraph {
    pub(crate) fn recompute_indirect_calls(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_indirect_calls();
        let sites = self.compute_indirect_call_sites(files);
        self.apply_indirect_call_sites(sites);
    }
}
```

The method should cover the same levels full build supports today:

- local function pointer assignment,
- array dispatch table,
- struct field callback,
- parameter-passed callback.

Preserve the current Phase 3 level ordering and its `extra_sites`/dedup behavior during extraction. The goal is
one reusable implementation of existing semantics, not a cleanup that changes which indirect sites win when
multiple levels discover the same logical call.

`CallGraph::build` should call the same helper rather than keeping a second copy of Phase 3 logic. A good end
state is:

```text
build direct source graph
construct CallGraph with the same scope-graph population as today
cg.recompute_indirect_calls(files)
cg.refresh_rust_receiver_state(files)
cg.apply_go_embedding_promotion(files)
cg.apply_go_interface_dispatch(files)
```

That keeps full and incremental paths pinned to one implementation. Because the refactor may move indirect
resolution from pre-construction scratch data into a method on the constructed `CallGraph`, the implementation
must prove zero normalized full-build delta before using the helper for incremental.

### 5.2 Synthetic-site provenance

Recompute needs to remove old derived sites without deleting source call sites. Add a serialized provenance
field to `CallSite`:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CallSiteOrigin {
    #[default]
    Source,
    IndirectResolution,
}

pub struct CallSite {
    // existing fields...
    #[serde(default)]
    pub origin: CallSiteOrigin,
}
```

Rules:

- direct source extraction must set `origin = Source` by constructor/defaulting helper or explicitly at struct
  literal sites;
- all Phase 3 synthetic sites must set `origin = IndirectResolution`;
- `origin` must be excluded from `cmp_key`/`Ord`, like receiver telemetry;
- `clear_indirect_calls` removes only `IndirectResolution` sites from both `calls` and `callers`;
- because `origin` is excluded from `Ord`, `clear_indirect_calls` must retain/filter by `origin`; do not use
  `BTreeSet::remove` with a synthetic key to remove existing entries;
- `origin` may be included in parity dumps and debug output.

Do not use `CallKind::MacroInvocation` or add `CallKind::IndirectResolved` for this. `CallKind` participates in
sorted identity and is consumed by resolver/navigation/CPG code as call-site kind, not provenance.

This is a serialized `CallGraph` shape change. Bump `src/cpg_cache.rs` `CACHE_VERSION` and its version test. The
cache-version bump is the compatibility guarantee for old bincode cache blobs: old blobs should become cache
misses rather than relying on tolerant deserialization. Keep `#[serde(default)]` for source-default ergonomics,
non-cache serde seams, and future tolerant fixtures, but do not treat it as the persisted-cache migration
mechanism. `#[serde(default)]` does not initialize ordinary Rust struct literals; direct `CallSite { ... }`
construction still needs `origin: CallSiteOrigin::Source` or a helper constructor.

Prefer a narrow source-site constructor/helper, such as `CallSite::source(...)`, if it reduces literal churn
without hiding required fields. Do not add a broad `Default` implementation that makes incomplete call sites easy
to construct silently.

### 5.3 Collision policy

Because `origin` is excluded from `Ord`, a source site and a synthetic site with the same `cmp_key` cannot both
exist in `calls: BTreeSet<CallSite>`. That is acceptable and should be explicit:

- source sites are inserted before synthetic sites in full build;
- if a synthetic site has the same logical identity as a source site, the source site wins in `calls`;
- `callers: BTreeMap<String, Vec<CallSite>>` can still represent duplicated test states, so `clear_indirect_calls`
  should be tested against `callers` directly.

Add tests for both:

- source-before-synthetic insertion leaves the source call in `calls`;
- `clear_indirect_calls` removes only synthetic entries from `callers`.

### 5.4 Idempotency

`recompute_indirect_calls(files)` must be idempotent:

- running it twice gives the same normalized graph as running it once;
- running it after a direct-subset merge gives the same synthetic sites as full build;
- running it after target removal does not retain old synthetic sites.

The easiest proof is a helper that normalizes `calls` and `callers` before and after repeated recompute.

### 5.5 Whole-graph recompute ordering

Change incremental ordering in `src/cpg/build.rs` to match full build:

```rust
cached_cg.merge(fresh_cg);
cached_dfg.merge(fresh_dfg);

cached_cg.recompute_indirect_calls(files);
cached_cg.rebuild_scope_graph(files, scope_inputs);
cached_cg.apply_go_embedding_promotion(files);
cached_cg.apply_go_interface_dispatch(files);

Self::assemble_graph(cached_cg, cached_dfg, files, type_db)
```

This order is intentionally different from today's incremental order, where Go embedding/interface dispatch run
before `rebuild_scope_graph`. The implementation PR must call this out as a behavior-affecting correctness fix,
not a no-op refactor.

The "match full build" claim is semantic, not byte-for-byte call-order identity. Full build constructs the
`CallGraph` with scope graph populated before the reusable indirect helper runs; incremental should run indirect
after the direct-subset merge and then call `rebuild_scope_graph`, which is the incremental equivalent of full
build's scope population plus `refresh_rust_receiver_state`. This asymmetry is expected because C/C++ indirect
recompute reads functions/files, while Rust receiver refresh must run after scope rebuild. The implementation PR
should note this explicitly and rely on the mixed-ordering fixture plus the zero-full-build-delta gate to prevent
accidental divergence.

### 5.6 Direct-subset economics

The preferred design preserves direct-subset economics:

- changed files are reparsed into direct CG/DFG only;
- the whole-graph work is limited to derived recomputes that are already whole-program by nature.

Fallback option, if provenance proves too invasive: rebuild the whole direct `CallGraph` from all files, then
run derived recomputes. That avoids `CallSiteOrigin` but weakens partial-hit value and should be treated as a
reviewed fallback, not the default plan.

Do not rebuild only `calls`/`callers` from all files while retaining other `CallGraph` indexes. That risks stale
functions, methods, imports, receiver metadata, arity tables, import bindings, class bases, and Go/Rust derived
state.

## 6. Parity Oracle Design

Add the parity helper in `tests/ast/cpg_cache_test.rs` by default:

```rust
fn assert_incremental_matches_full(
    v1: BTreeMap<String, ParsedFile>,
    v2: BTreeMap<String, ParsedFile>,
    changed_files: BTreeSet<String>,
    type_db: Option<&TypeDatabase>,
    scope_inputs: Option<&ScopeGraphBuildInputs>,
)
```

The helper should:

1. build a full v1 CPG;
2. clone the v1 call graph and DFG as cached input;
3. build a full v2 CPG independently;
4. build an incremental v2 CPG from v1 cached parts and `changed_files`;
5. compare normalized dumps;
6. print compact diffs on failure.

The v2 full build must be independent. Do not implement the expected side by calling a shared helper that also
drives incremental ordering, or the test can accidentally bless the new incremental order.

A focused new integration module is acceptable only if the implementation PR adds a concrete runtime validation
command for that target or filter. Do not rely on `cargo test --all-targets --no-run`; it catches compile
coverage, not parity execution.

### 6.1 Normalized dump contents

At minimum, dump:

- `call_graph.functions`;
- `call_graph.calls`;
- `call_graph.callers`, with each callers `Vec` sorted into stable normalized tuples before comparison;
- Go embedding/interface tables and gaps;
- Rust scope graph presence/completeness markers and receiver outcomes;
- CPG call edges;
- DFG defs, uses, and edges;
- optional navigation outputs for seeds listed by a fixture.

Node and edge counts may remain as smoke checks but must not be the primary assertion.

### 6.2 Query-level checks

For fixtures that name seeds, run navigation-style callers/callees checks against `CpgContext::build_with_cached_cpg`
for both full and incremental outputs.

This matters because a normalized internal dump can miss a behavior gap if a later query path filters or ranks
the same data differently. These query-level checks also guard order-dependent behavior that intentionally should
remain visible through navigation output.

## 7. Required Fixture Matrix

### 7.1 C/C++ indirect recall and stale-edge fixtures

Each case should have at least one changed-caller variant and one unchanged-caller stale-derived-edge variant
where current semantics support it.

- Local function pointer assignment:
  - changed caller introduces `fp = target; fp();`;
  - changed caller removes or changes the assignment;
  - changed target file renames/removes target while caller is unchanged.
- Array dispatch table:
  - changed caller/table file changes a `handlers[i]()` dispatch or its table entry;
  - unchanged caller and table lose a stale edge when a target function in another file is renamed or removed,
    if full build resolves that target today;
  - do not claim cross-file table-entry edits unless full build resolves them today.
- Struct field callback:
  - assignment file changes `.callback = target`;
  - caller file changes `obj->callback()`;
  - target removal/rename invalidates old synthetic edge.
- Parameter-passed callback:
  - outer caller passes a different function;
  - callback target file removes or renames a target;
  - unchanged callee that invokes the parameter loses the old stale synthetic edge.
- C++ syntax coverage:
  - include at least one C++ fixture using equivalent function-pointer or callback syntax so the C/C++ claim is
    backed by a C++ parser path, not only C fixtures.

### 7.2 Existing recompute protections

Keep and strengthen existing tests around:

- Rust receiver outcome rematerialization;
- Go embedding promoted alias stale-state cleanup;
- Go interface dispatch stale table cleanup;
- Python/JS/TS import-binding and direct-call partial-hit parity.

### 7.3 Mixed ordering fixture

Add one mixed final `files` map that contains:

- a Rust receiver case requiring `rebuild_scope_graph`/receiver rematerialization;
- a Go embedding/interface dispatch case;
- a C indirect callback case.

The fixture exists to catch accidental incremental recompute order drift. The assertion is parity against the
independent full v2 build.

## 8. Implementation Plan

1. Add `CallSiteOrigin` and `CallSite.origin`.
2. Update all `CallSite` construction sites to rely on default `Source` or explicitly set `Source` where clarity
   helps. Audit direct struct literals in source and tests, not only extraction paths.
3. Bump `CACHE_VERSION` and update the cache-version test/comment.
4. Extract full-build Phase 3 into `compute_indirect_call_sites` and `apply_indirect_call_sites`.
5. Add `clear_indirect_calls` and `recompute_indirect_calls`.
6. Change full build to call `recompute_indirect_calls`.
7. Change incremental build order to `indirect -> scope/Rust receiver -> Go embedding -> Go interface`.
8. Add normalization helpers and the fixture matrix.
9. Run focused tests and Tier-A gates.

Implementation should be a single code PR only if the parity helper and fixtures land with the behavior change.
Do not land the `CallSiteOrigin` shape change without parity coverage.

## 9. Tests

Required new or updated tests:

- `callsite_origin_defaults_to_source_for_non_cache_serde_data` if there is a local serde seam worth testing;
  persisted cache compatibility is covered by the cache-version test.
- `clear_indirect_calls_removes_only_synthetic_entries_from_calls_and_callers`.
- `recompute_indirect_calls_is_idempotent_and_post_merge_matches_full_build`.
- `incremental_matches_full_for_c_local_function_pointer_changed_caller`.
- `incremental_matches_full_for_c_local_function_pointer_changed_target`.
- `incremental_matches_full_for_c_array_dispatch_same_file_table_change`.
- `incremental_matches_full_for_c_struct_field_callback_assignment_change`.
- `incremental_matches_full_for_c_struct_field_callback_target_removal`.
- `incremental_matches_full_for_c_parameter_callback_outer_caller_change`.
- `incremental_matches_full_for_c_parameter_callback_target_removal`.
- `incremental_matches_full_for_rust_receiver_rematerialization`.
- `incremental_matches_full_for_go_embedding_and_interface_dispatch`.
- `incremental_matches_full_for_python_js_ts_direct_calls_and_import_bindings`.
- `incremental_matches_full_for_mixed_recompute_ordering`.
- `cache_version_bumped_for_callsite_origin`.

Existing tests that compare only node/edge counts can remain, but they should not be treated as sufficient.

## 10. Validation

This slice touches `src/call_graph.rs`, `src/cpg/build.rs`, and cache serialization. Required validation:

```bash
cargo fmt --check
cargo test --all-targets --no-run
cargo test --test ast cpg_cache
cargo test --test navigation
cargo test --test name_resolution
cargo test --test lang_c
cargo test --test lang_cpp
cargo test --test integration resolution
cargo test --test integration call_graph
cargo test --lib call_graph
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
git diff --check origin/main..<branch>
```

Use `--allow-stale-sut` only immediately after the release build in the same worktree. Do not rebaseline Tier-A
inside this slice.

The `cargo test --all-targets --no-run` compile gate is intentional: adding `CallSite.origin` touches direct
struct literals outside the primary call-graph implementation, including resolution tests, navigation tests,
language typed-receiver tests, and algorithm tests.

The `lang_c` and `lang_cpp` commands are included by default because the fixture matrix requires C coverage and
at least one C++ parser-path fixture.

## 11. Risk Register

| Risk | Mitigation |
|---|---|
| Old synthetic indirect sites remain in unchanged callers | Provenance field plus whole-graph `clear_indirect_calls` before recompute. |
| Source and synthetic sites collide in `BTreeSet` identity | Source-before-synthetic insertion policy plus explicit collision tests. |
| Incremental and full share too much expected-path code in tests | Build independent full v2 expected output and compare normalized dumps. |
| Old cache blobs deserialize into the new `CallSite` layout | Cache-version bump forces a miss for old bincode blobs; `#[serde(default)]` is only a secondary guard for non-cache serde seams. |
| Recompute order changes Go/Rust behavior unexpectedly | Mixed fixture and Tier-A; PR notes call out the order fix. |
| Whole-graph indirect recompute is expensive on large C/C++ repos | Accept for Slice 2 correctness; measure later before AutoIncremental. |
| Parity oracle misses query-visible behavior | Add selected callers/callees query checks over full vs incremental contexts. |

## 12. Reviewer Checklist

Reviewers should answer:

- Does the proposed provenance field remove stale synthetic sites without changing call-kind semantics?
- Is excluding `origin` from `Ord` correct, given the collision policy and tests?
- Does the proposed incremental ordering match current full-build order?
- Does the fixture matrix cover both recall gaps and stale-edge precision gaps?
- Are the normalized dumps comparing all resolver-affecting `CallSite` fields?
- Is the cache-version bump mandatory and correctly scoped?
- Are the Tier-A and focused validation commands sufficient for this behavior change?

## 13. Implementation Handoff

Expected implementation files:

- `src/call_graph.rs`
- `src/cpg/build.rs`
- `src/cpg_cache.rs`
- possible direct `CallSite` literal updates in `src/resolution.rs`, `src/resolution_disproof.rs`, and
  `src/algorithms/taint.rs`
- tests under `tests/ast/`, `tests/navigation/`, `tests/name_resolution/`, `tests/lang/`, and possibly
  `tests/integration/`

Avoid unrelated MCP, evidence-view, and navigation-query changes. Future MCP `AutoIncremental` remains blocked
until this slice is implemented, reviewed, merged, and validated.
