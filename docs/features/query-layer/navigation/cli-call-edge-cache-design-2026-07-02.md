# CLI Call-Edge Performance Cache Design

Status: ready to implement after A2A review
Date: 2026-07-02
Primary priority: `nav callers` / `nav callees` CLI speed without recall or precision drift
Related design: `resolved-call-edge-index-design-2026-07-01.md`

## Table of Contents

- [Decision Summary](#decision-summary)
- [Architecture Analysis](#architecture-analysis)
- [Measurement Gate](#measurement-gate)
- [Design Goals](#design-goals)
- [Option A: Persisted Navigation Call-Edge Cache](#option-a-persisted-navigation-call-edge-cache)
- [Option B: Bucket-Scoped CLI Fast Path](#option-b-bucket-scoped-cli-fast-path)
- [Recommendation](#recommendation)
- [Resolved Owner Decision](#resolved-owner-decision)
- [Implementation Plan](#implementation-plan)
- [Validation Plan](#validation-plan)
- [Risks and Mitigations](#risks-and-mitigations)
- [Review Log](#review-log)

## Decision Summary

Implement a persisted navigation call-edge sidecar cache first.

The July 1 lazy in-memory `NavigationCallEdgeIndex` removed repeated resolver work inside a long-lived
`NavigationSession`, especially MCP sessions and repeated in-process queries. It also moved the whole-repo
resolved-edge build to the first call-edge consumer. That tradeoff is good for MCP, but it can hurt one-shot CLI
commands that load a cached CPG and ask for one caller/callee chain: the CPG is warm, but the resolved call-edge
index is cold and must be rebuilt in every process.

A navigation-owned persisted sidecar cache addresses repeated warm-CPG, one-shot-CLI cost while preserving one
source of truth for call-edge semantics:

- `NavigationIndex` still owns one canonical resolved-edge index shape.
- `nav callers`, `nav callees`, `module-deps`, and `repo-map` keep using the same indexed facts.
- Non-call-edge commands such as `nodes-at`, `functions`, and `call-stats` do not pay the edge-index cost.
- The diff/review CLI path stays unchanged because this remains under `src/navigation/`.

Do not put this cache into `CpgContext` or serialized `CallGraph` in this slice. The cache is navigation-local,
derived, and invalidatable independently. The one cross-cutting cache change in scope is binary-input cache
identity: both the existing CPG cache and the new sidecar should validate against a build identity derived from
Prism binary inputs, not raw repo-wide dirty state. This avoids cache misses from docs/eval/target-repo churn while
still invalidating cache entries when analyzer implementation inputs change.

The pre-implementation measurement gate cleared: `NavigationIndex::build_resolved_call_edges` is a material share
of warm one-shot CLI latency on this repo. Option B remains a fallback if sidecar deserialize time or true
cold-cache caller/callee latency remains unacceptable after Option A.

## Architecture Analysis

### Current Call Chain

The CLI path is:

1. `src/main.rs::run_nav`
2. `build_session(repo, no_cache, cache_dir)`
3. `NavigationIndex::build_cached_under` or `NavigationIndex::build`
4. Query-specific code in `src/navigation/queries.rs` or `src/navigation/module_graph.rs`

`NavigationIndex::build_cached_at` currently wraps only the CPG cache:

- It computes the repo topology key from `repo.file_hashes` and `repo.manifest_hashes`.
- It loads `cpg-cache.bin` with exact file/topology/type-db/cache-build-identity checks.
- On a CPG cache hit, it reconstructs a `CpgContext` and calls `NavigationIndex::from_ctx`.
- On a partial hit or miss, it rebuilds the CPG, saves it, and calls `from_ctx`.

`NavigationIndex::from_ctx` initializes:

- `line_range_index`
- `name_index`
- `resolved_call_edges: OnceLock<NavigationCallEdgeIndex>`

The `OnceLock` is always empty after CPG cache load. The first call to `direct_callers`,
`direct_callees`, `outgoing_call_edges`, or `collision_dropped_sites` calls
`NavigationIndex::build_resolved_call_edges`.

### Current Edge-Index Behavior

`build_resolved_call_edges` derives three structures:

- `incoming_by_target: BTreeMap<FunctionId, Vec<IndexedIncomingCall>>`
- `outgoing_by_caller: BTreeMap<FunctionId, Vec<IndexedOutgoingCallSite>>`
- `multi_owner_collision_sites: BTreeSet<CallSiteKey>`

It resolves each unique call site, then:

- stores outgoing call sites for callee expansion and unresolved-callee evidence,
- stores incoming edges expanded through `scoped_caller_site_match_count`, preserving duplicate incoming evidence
  where import-member bucket matching intentionally yields more than one caller-site occurrence,
- stores multi-owner collision sites for caller warning counts.

That expansion means a persisted cache should store `NavigationCallEdgeIndex` itself, not a narrower list of
resolved targets. Recomputing duplicate incoming edges or collision counts from a compressed cache risks drift.

### Consumers

Call-edge consumers are:

- `queries::callers_with_confidence`
- `queries::callees_with_confidence`
- `queries::ego_graph` warning logic for Call-edge queries via collision warnings
- `module_graph::module_deps`
- `module_graph::repo_map`

Non-call-edge consumers should remain cheap on a CPG cache hit:

- `nodes_at`
- `functions`
- `call_stats`
- `interface_dispatch_manifest`

### Performance Shape

The July 1 lazy index optimizes repeated in-process query workloads. The remaining cost is process-local:

- Warm CPG cache, cold process, one CLI call-edge query: slow because the edge index is rebuilt.
- Warm CPG cache, long-lived MCP process: fast after the first call-edge query.
- Cold CPG cache: still dominated by repo load and CPG construction; a sidecar edge cache cannot help until a valid
  CPG exists.

The design target is therefore warm-cache CLI latency. True cold-cache latency is the only case where a
bucket-scoped fast path could beat the sidecar approach.

## Measurement Gate

Measurement date: 2026-07-02
Worktree: `/private/tmp/prism-nav-cli-call-edge-cache-design`
Binary: `target/release/prism`, temporarily instrumented with an env-gated timer around
`NavigationIndex::build_resolved_call_edges`; the instrumentation was removed after measurement.
Cache dir: `/private/tmp/prism-nav-edge-measure-20260702`

Warm-up command:

```bash
/usr/bin/time -p target/release/prism nav --cache-dir /private/tmp/prism-nav-edge-measure-20260702 \
  call-stats --repo . >/private/tmp/prism-nav-edge-measure-call-stats.json
```

Result:

- CPG cache miss warm-up: `real 16.34s`
- No call-edge consumer was invoked during warm-up.

Warm CPG one-shot samples:

| Query | Edge build | Wall time | Edge share |
|-------|------------|-----------|------------|
| `callers build_session@src/main.rs` | 1.860s | 2.45s | 75.9% |
| `callees run_nav@src/main.rs` | 1.865s | 2.45s | 76.1% |
| `callers build_session@src/main.rs` | 1.855s | 2.47s | 75.1% |
| `callees run_nav@src/main.rs` | 1.886s | 2.46s | 76.7% |

The edge-index build averaged 1.867s inside 2.458s wall time, or about 76% of warm one-shot process latency. The
gate clears: replacing rebuild-per-process with sidecar load is worth designing and implementing before looking
for smaller CLI overheads.

The measured index shape was stable across samples:

- `resolved_sites = 53,491`
- `incoming_edges = 15,870`
- `outgoing_sites = 53,491`
- `collision_sites = 910`

## Design Goals

Must have:

- Preserve `Evidence` output exactly for callers/callees/module graph results.
- Preserve collision warning behavior.
- Avoid resolver semantic forks.
- Keep non-call-edge nav commands from building or loading the resolved-edge index unless they need it.
- Treat corrupt, stale, or version-mismatched edge caches as misses.
- Keep `--no-cache` behavior cache-free for both CPG and edge sidecar files.
- Keep cache writes best-effort: query success must not depend on sidecar persistence.

Should have:

- Reuse the existing navigation cache directory: `<base>/prism/nav/<repo-sha256>/`.
- Use atomic writes, matching `cpg_cache`.
- Record human-readable metadata for debugging benchmark runs.
- Make invalidation explicit enough that future resolver changes have an obvious bump point.

Out of scope:

- New call-resolution rungs.
- `CallGraph` or `CpgContext` serialized schema changes.
- Re-baselining Tier-A expected results.
- MCP protocol changes.

## Option A: Persisted Navigation Call-Edge Cache

### Overview

Add a navigation-local sidecar cache under the existing nav cache directory:

- `resolved-call-edge-index.bin`
- `resolved-call-edge-index-meta.json`

The sidecar stores a serialized `NavigationCallEdgeIndex` plus the exact fingerprint needed to prove it was
derived from the current CPG inputs and resolver behavior.

### Module Boundary

Add a new module:

```rust
src/navigation/call_edge_cache.rs
```

Responsibilities:

- Define `NAV_CALL_EDGE_CACHE_VERSION`.
- Define serialized cache and metadata records.
- Load a cache from a nav cache directory.
- Save a cache atomically.
- Validate version, binary identity, file hashes, topology key, and type-db presence.

Keep CPG cache serialization in `src/cpg_cache.rs`. The navigation sidecar should depend on the same repo inputs
used by `NavigationIndex::build_cached_at`, but it should not be part of the CPG cache blob.

### Serialized Shape

Make the existing navigation index records serializable:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IndexedResolvedTarget {
    pub target: FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IndexedOutgoingCallSite {
    pub callee_name: String,
    pub call_site_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub resolved: Vec<IndexedResolvedTarget>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IndexedIncomingCall {
    pub caller: FunctionId,
    pub callee_name: String,
    pub call_site_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct NavigationCallEdgeIndex {
    outgoing_by_caller: BTreeMap<FunctionId, Vec<IndexedOutgoingCallSite>>,
    incoming_by_target: BTreeMap<FunctionId, Vec<IndexedIncomingCall>>,
    multi_owner_collision_sites: BTreeSet<CallSiteKey>,
}
```

`CallSiteKey` must also derive serde if `multi_owner_collision_sites` remains serialized directly. If a native
field used by `CallSiteKey` does not serialize cleanly, store the already-normalized comparable form used today
instead of weakening the key.

`ResolutionKind` currently crosses the navigation boundary through `IndexedResolvedTarget.kind` and
`IndexedIncomingCall.kind`. The implementation must add `serde::Serialize` / `serde::Deserialize` derives to
`ResolutionKind` in `src/resolution.rs`; `ResolutionConfidence`, `FunctionId`, `CallKind`, and `CallSiteOrigin`
already carry serde support.

The cache envelope should be explicit:

```rust
struct NavigationCallEdgeCache {
    nav_call_edge_cache_version: u32,
    prism_version: String,
    grammar_fingerprint: String,
    skip_policy_version: u32,
    fingerprint: NavigationCallEdgeCacheFingerprint,
    git_sha: String, // metadata only; not the dirty-load/cache-validity gate
    index: NavigationCallEdgeIndex,
}

struct NavigationCallEdgeCacheFingerprint {
    cache_build_identity: String,
    file_hashes_digest: String,
    topology_key_digest: String,
    has_type_db: bool,
}
```

The bincode sidecar envelope is authoritative for validation and contains both fingerprint fields and index bytes.
The JSON metadata file is human-readable diagnostics only; never validate a binary sidecar against JSON metadata.

Use one sidecar version field: `nav_call_edge_cache_version`. It covers the sidecar envelope and derived index
shape. Bump it when resolver behavior, `NavigationCallEdgeIndex`, `CallSiteKey`, or sidecar fingerprint semantics
change. If the CPG cache exposes its `CACHE_VERSION`, include it in the fingerprint as well. If exposing it creates
avoidable churn, the sidecar can rely on exact file/topology digests, type-db presence, cache-build identity, and
its own version; the loaded CPG has already passed CPG cache validation.

### NavigationIndex Changes

Add optional cache write context to `NavigationIndex`:

```rust
struct NavigationCallEdgeCacheStore {
    cache_dir: PathBuf,
    fingerprint: NavigationCallEdgeCacheFingerprint,
    load_allowed: bool,
}

pub struct NavigationIndex {
    ...
    resolved_call_edges: OnceLock<NavigationCallEdgeIndex>,
    call_edge_cache_store: Option<NavigationCallEdgeCacheStore>,
}
```

Store a compact fingerprint rather than cloning full `file_hashes` and `topology_key` maps into every
`NavigationIndex`. Full maps already exist on `LoadedRepo` and are used for the CPG cache hit; the sidecar store
only needs a deterministic current fingerprint to compare against the bincode envelope on first call-edge use.

Add an internal constructor:

```rust
fn from_ctx_with_call_edge_cache(
    ctx: CpgContext,
    store: Option<NavigationCallEdgeCacheStore>,
) -> Self
```

Construction rules:

- `NavigationIndex::build(repo)` uses no store and no initial edge index.
- `NavigationIndex::build_incremental_from_previous(...)` uses no store; incremental indexes must not inherit a
  disk sidecar from the previous session.
- `NavigationIndex::build_cached_at(repo, cache_dir)` computes the store metadata once.
- On CPG cache hit, retain a store with `load_allowed = true`.
- On CPG partial hit or miss, retain a store with `load_allowed = false`; the current process rebuilt the CPG, so
  any old sidecar should be ignored and overwritten only after a fresh edge-index build.
- On `--no-cache`, no sidecar load or save is attempted because `build_session` calls `NavigationIndex::build`.
- `with_modified_cpg_for_testing` must drop the store or set `load_allowed = false` when it clears the `OnceLock`;
  otherwise a test that mutates an index built from cache could reload stale disk edges for the pre-mutation CPG.

Save rules:

```rust
pub(crate) fn resolved_call_edges(&self) -> &NavigationCallEdgeIndex {
    self.resolved_call_edges.get_or_init(|| {
        if let Some(store) = &self.call_edge_cache_store {
            if let Some(idx) = store.load_best_effort() {
                return idx;
            }
        }
        let idx = self.build_resolved_call_edges();
        if let Some(store) = &self.call_edge_cache_store {
            store.save_best_effort(&idx);
        }
        idx
    })
}
```

`load_best_effort` returns `None` when `load_allowed` is false, the sidecar is missing, or validation fails. This
keeps sidecar deserialization lazy: `nodes-at`, `functions`, `call-stats`, and other non-call-edge commands should
not even read the sidecar file.

The save is synchronous and best-effort. This keeps implementation simple and makes benchmark behavior easy to
reason about. If save cost becomes visible, move only the disk write behind an opt-in async/background path later.

### Invalidation Contract

The sidecar is valid only when all of these match:

- `NAV_CALL_EDGE_CACHE_VERSION`
- `CARGO_PKG_VERSION`
- `GRAMMAR_FINGERPRINT`
- `SKIP_POLICY_VERSION`
- `PRISM_CACHE_BUILD_IDENTITY`
- current `repo.file_hashes`
- current topology key from file hashes plus manifest hashes
- current type-db presence

This mirrors the CPG cache's exact-hit posture for committed builds. The sidecar also caches resolver output, which
is a stricter dirty-build hazard than the CPG cache alone: before this sidecar, a warm CPG cache still recomputed
resolver output at query time after a dirty resolver edit. The implementation must make the dirty-build policy
explicit in code and docs; see [Resolved Owner Decision](#resolved-owner-decision).

Do not use raw `GIT_SHA` as the cache-validity identity or sidecar dirty-load gate. `GIT_SHA` dirtiness is
repo-wide and currently ignores untracked files; it can be too broad for docs/eval churn and too narrow for
untracked binary inputs. Add a separate binary-input cache identity and dirty signal for both CPG and sidecar cache
policy.

When changing resolver behavior, `NavigationCallEdgeIndex` shape, `CallSiteKey` semantics, or
`scoped_caller_site_match_count`, bump `NAV_CALL_EDGE_CACHE_VERSION`.

### Why Not CpgContext

`CpgContext` is used outside navigation. Moving the resolved nav edge index there would widen ownership before the
need is proven:

- It would pull navigation-specific evidence expansion into a broader CPG abstraction.
- It would force non-navigation CPG consumers to carry a cache they do not currently use.
- It would increase the risk of accidental cache construction on diff/review workflows.
- It would make serialized CPG cache versioning more contentious.

If later measurements show multiple non-navigation consumers need identical resolved call facts, promote a narrower
resolver-result cache into `CallGraph` or `CpgContext` then. Do not promote the navigation-expanded evidence index.

## Option B: Bucket-Scoped CLI Fast Path

### Overview

Add a CLI-only path for `nav callers` and `nav callees` that resolves only the current BFS frontier instead of
building the whole-repo `NavigationCallEdgeIndex`.

For callers:

- For each frontier target, scan only `scoped_caller_sites(cg, target.name)`.
- Resolve each candidate call site.
- Identity-filter to the target `FunctionId`.
- Count multi-owner collision drops only for the queried target name.

For callees:

- For each frontier caller, scan only `cg.calls[caller]`.
- Resolve those call sites.
- Preserve unresolved call-site evidence exactly as the indexed path does.

### Required Boundary

This must not become a second semantic implementation hidden inside the query layer. If implemented later, create
a small lookup abstraction and test both implementations against each other:

```rust
trait CallEdgeLookup {
    fn direct_callers(&self, target: &FunctionId) -> Vec<IndexedIncomingCall>;
    fn direct_callees(&self, caller: &FunctionId) -> Vec<IndexedOutgoingCallSite>;
    fn collision_dropped_sites(&self, seed_name: &str) -> usize;
}
```

The current global index can implement the trait by cloning indexed records. A scoped resolver can implement it by
resolving only the requested bucket/frontier. Query formatting and sorting should remain shared.

### Benefits

- Improves true cold-cache, one-shot CLI caller/callee latency.
- Avoids writing another cache file.
- Keeps disk cache footprint unchanged.

### Costs

- Reintroduces a second query-time resolution path after the July 1 work consolidated call-edge semantics.
- Only helps `callers` and `callees`; `module-deps` and `repo-map` still need whole-repo call-derived edges.
- Increases parity-test burden because the scoped and global paths must stay identical.
- Makes automatic strategy selection risky: thresholds based on bucket size or frontier size can produce
  unpredictable latency cliffs.

For those reasons, this should remain a fallback after measuring the persisted sidecar. If implemented, start with
an explicit internal mode or benchmark-only flag before making `auto` decisions in the CLI.

## Recommendation

Proceed with Option A first.

The strongest reasons are:

- The measurement gate shows global edge-index construction is about 76% of warm one-shot caller/callee latency on
  this repo.
- A sidecar cache preserves one canonical global edge-index path for callers, callees, collision warnings,
  `module-deps`, and `repo-map`.
- It avoids reintroducing a second query-time resolution path before proving the sidecar is insufficient.

Do not implement Option B in this slice. Use Option B after Option A only if post-implementation benchmarks show
one of these:

- True cold-cache caller/callee latency is still a priority and dominates the user workflow.
- First-and-only caller/callee latency is still unacceptable because sidecar load/write amortization does not fit
  the real usage pattern.
- The sidecar file is too large or too slow to deserialize on realistic repos.
- Cache invalidation churn makes the sidecar miss often enough that warm-cache speed does not improve.

If reviewers still disagree with Option A as the first implementation after the measurement gate and owner
decision, escalate that judgment before implementation. The decision affects correctness-risk posture, not just code
shape.

## Resolved Owner Decision

Owner decision on 2026-07-02: proceed with Option A first after the measurement gate.

Option A is now the planned first implementation:

- persisted sidecar for `NavigationCallEdgeIndex`;
- lazy load inside `NavigationIndex::resolved_call_edges()`;
- best-effort save after first index build;
- exact invalidation using repo hashes, topology, type-db presence, binary identity, grammar fingerprint, and a
  sidecar cache version.

Option B remains the next fallback if Option A does not deliver enough speed for true cold-cache or first-only
caller/callee workflows.

Dirty-build sidecar load policy and cache identity:

- Disable sidecar loads only when the Prism binary's own build-impacting inputs are dirty, while still allowing
  dirty builds to write sidecars. Do not key this decision from arbitrary repo dirtiness.
- Build-impacting inputs are the files that can change the analyzer binary or its embedded cache semantics, such
  as `src/**`, `build.rs`, `Cargo.toml`, and `Cargo.lock`. The implementation should include untracked files under
  those inputs, because an untracked Rust module can still be compiled if a tracked module references it.
- Dirty docs, eval output, benchmark notes, target-corpus changes, or work in another repository such as Hugo must
  not disable sidecar loads unless those files are part of the Prism binary inputs.
- Keep raw `GIT_SHA` for `--version`, reporting, and human diagnostics, but add a cache-specific identity, for
  example `PRISM_CACHE_BUILD_IDENTITY`.
- `PRISM_CACHE_BUILD_IDENTITY` should be a deterministic binary-input content fingerprint in both clean and dirty
  builds, not the repo HEAD SHA. This prevents docs-only commits or eval/corpus commits from churning caches when
  analyzer binary inputs did not change.
- Compute the fingerprint with SHA-256 over sorted `path + NUL + content` records for all build-impacting inputs,
  including relevant tracked files and untracked files under binary-input paths. Include path names so delete/add
  and rename cases cannot collide with identical concatenated contents.
- For dirty binary inputs, the same fingerprint changes when implementation inputs change, so CPG and sidecar
  caches do not reuse resolver output across dirty implementation edits.
- The existing CPG cache should validate against `PRISM_CACHE_BUILD_IDENTITY` rather than raw repo-wide `GIT_SHA`;
  otherwise docs/eval dirtiness can still defeat warm-cache behavior before sidecar logic runs.
- When sidecar loads are disabled for a binary-input-dirty build, print an explicit warning the first time a
  call-edge consumer reaches the sidecar path: Prism is recomputing resolved call edges because the analyzer binary
  was built from dirty implementation inputs, and warm sidecar loads require a clean binary-input state.
- Add a deliberately noisy benchmark/debug override, such as `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY=1`, only for local
  measurement. If used, print a warning that stale resolver output is possible.

## Implementation Plan

1. Add `src/navigation/call_edge_cache.rs`.
   - Define sidecar paths, version, serialized envelope, metadata, `load`, and `save`.
   - Use bincode for the index and JSON for metadata.
   - Use temp-file-plus-rename atomic writes with a unique temp filename, such as pid plus nonce, so concurrent CLI
     or MCP writers do not race on a fixed `resolved-call-edge-index.bin.tmp`.
   - Refuse sidecar loads when the binary-input dirty signal is true, unless the benchmark/debug override is set.
   - Emit a warning on the first disabled dirty-build load attempt and on any override-enabled dirty load.

2. Add serde derives to navigation call-edge index structs.
   - Keep visibility `pub(crate)`.
   - Avoid changing field names or query behavior.

3. Add binary-input cache identity and dirty signals.
   - Add dedicated build-time env vars, for example `PRISM_CACHE_BUILD_IDENTITY` and `PRISM_BINARY_INPUT_DIRTY`,
     rather than overloading `GIT_SHA`.
   - Implement this in `build.rs` with `println!("cargo:rustc-env=...")`.
   - Compute the identity with SHA-256 over sorted `path + NUL + content` records for build-affecting pathspecs
     such as `src/`, `build.rs`, `Cargo.toml`, and `Cargo.lock`.
   - Compute `PRISM_BINARY_INPUT_DIRTY` from a second git status query that includes untracked files, filtered to
     those same binary-input pathspecs.
   - Exclude docs, eval reports, benchmark workspaces, corpora, and unrelated target repositories unless they are
     actual binary inputs.
   - Document that current `build.rs` uses `git status --porcelain -uno` for `GIT_SHA`, so untracked files do not
     affect `GIT_SHA`; the new sidecar dirty signal must include untracked files under binary inputs.
   - Update the existing CPG cache metadata to store and validate `PRISM_CACHE_BUILD_IDENTITY` while preserving raw
     `GIT_SHA` in user-facing version output and optional metadata.
   - Bump `src/cpg_cache.rs::CACHE_VERSION` because the serialized CPG cache envelope changes. Do not rely on
     bincode/serde defaults for old cache compatibility.

4. Add cache store context to `NavigationIndex`.
   - Keep `NavigationIndex::build` cache-free.
   - Add an internal constructor that can attach a sidecar store.
   - Retain the store only for cached builds.
   - Store compact sidecar fingerprint data, not deep clones of full repo hash maps.

5. Wire `NavigationIndex::build_cached_at`.
   - Compute topology/type-db metadata once.
   - On CPG cache hit, attach a sidecar store with loading enabled.
   - On CPG partial hit or miss, attach a sidecar store with loading disabled so stale sidecars cannot be reused.
   - On sidecar miss/corruption during first call-edge use, continue silently except optional diagnostic stderr
     consistent with existing nav cache messages.

6. Save after first edge-index build.
   - Save from inside `resolved_call_edges().get_or_init`.
   - Log save failures as best-effort cache diagnostics.

7. Add tests.
   - `build_cached_writes_cpg_but_not_edge_until_call_edge_query`.
   - `call_edge_cache_hit_matches_uncached_query_output`.
   - `corrupt_call_edge_cache_rebuilds_and_query_succeeds`.
   - `file_or_manifest_change_invalidates_call_edge_cache`.
   - Sidecar version, package version, grammar fingerprint, skip-policy version, cache build identity, and type-db
     presence mismatches invalidate the sidecar.
   - CPG cache uses binary-input cache identity: tracked docs/eval dirtiness does not invalidate, tracked `src`
     dirtiness invalidates, and untracked `src` inputs invalidate or produce a distinct dirty-input identity.
   - CPG cache version pin reflects the cache identity envelope change.
   - `with_modified_cpg_for_testing` disables or drops the store after clearing `OnceLock`.
   - Sidecar serde round-trip preserves `multi_owner_collision_sites` and duplicated `incoming_by_target`
     multiplicity from `scoped_caller_site_match_count`.
   - Sidecar-hit parity covers `callers`, `callees`, collision warnings, `module-deps`, and `repo-map`.
   - A version-pin test fails loudly if `NAV_CALL_EDGE_CACHE_VERSION` changes unexpectedly or an index shape change
     lands without a planned bump.
   - Binary-input dirty policy tests cover: dirty tracked `src` input disables load; untracked `src` input disables
     load; tracked docs/eval changes do not disable load; dirty target repo files do not disable load; override
     allows load and emits the explicit warning.
   - Existing caller/callee/module graph tests remain unchanged.

8. Add benchmark evidence.
   - Warm CPG plus cold edge sidecar.
   - Warm CPG plus warm edge sidecar.
   - Repeated in-process query benchmark to ensure no regression from serialization changes.
   - Non-call-edge `nodes-at` or `functions` smoke timing to confirm no accidental edge build. Do not use
     `call-stats` as the sentinel because it already resolves call sites outside the nav edge index.

## Validation Plan

Required before review:

```bash
cargo fmt --check
cargo test navigation:: --lib
cargo test --test navigation
cargo test --test cli nav_compat_test::
cargo test --features mcp --test mcp
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

The Tier-A commands are required because implementation touches navigation call-edge construction and query
resolution paths.

MCP coverage is mandatory for this slice because MCP creates cached navigation indexes and must not retain stale
edge sidecars across refreshes:

- full cached MCP session startup can load the sidecar lazily on first call-edge query;
- incremental MCP refresh builds an index with no sidecar store and must not reload a pre-refresh disk sidecar;
- stale-index metadata and refresh behavior remain unchanged for non-call-edge tools.

Recommended local benchmark protocol:

```bash
rm -rf /tmp/prism-nav-call-edge-cache-bench
cargo build --release

# First run builds CPG and, after the first call-edge query, writes the edge sidecar.
time target/release/prism nav --cache-dir /tmp/prism-nav-call-edge-cache-bench \
  callers --repo . --symbol build_session --file src/main.rs \
  --format json >/tmp/prism-callers-first.json

# Second run should load both CPG and edge sidecar.
time target/release/prism nav --cache-dir /tmp/prism-nav-call-edge-cache-bench \
  callers --repo . --symbol build_session --file src/main.rs \
  --format json >/tmp/prism-callers-warm.json

time target/release/prism nav --cache-dir /tmp/prism-nav-call-edge-cache-bench \
  callees --repo . --symbol run_nav --file src/main.rs \
  --format json >/tmp/prism-callees-warm.json

time target/release/prism nav --cache-dir /tmp/prism-nav-call-edge-cache-bench \
  repo-map --repo . --format json >/tmp/prism-repo-map-warm.json
```

Acceptance gates:

- Warm sidecar caller/callee output is byte-stable or structurally equal to cold sidecar output after sorting.
- Warm sidecar caller/callee wall time is materially lower than warm CPG / cold sidecar one-shot CLI time.
- Warm sidecar load plus query should be substantially below the measured 2.45-2.47s current warm one-shot time.
- Include at least one larger-repo or larger-fixture timing before claiming the speedup generalizes beyond Prism
  itself.
- `nodes-at` or `functions` remains free of sidecar load/build cost.
- `module-deps` and `repo-map` do not regress.
- No Tier-A regressions or unexplained flip candidates.

## Risks and Mitigations

### Risk: Stale Sidecar After Resolver Change

Mitigation:

- Include `PRISM_CACHE_BUILD_IDENTITY`.
- Maintain `NAV_CALL_EDGE_CACHE_VERSION`.
- Document the bump rule beside the version constant.
- Use a sidecar-specific binary-input dirty signal instead of raw repo-wide `GIT_SHA=-dirty`.
- Disable sidecar loads for binary-input-dirty builds, warn explicitly, and require a noisy override for local
  dirty-build cache-load benchmarking.

### Risk: Cache File Size or Deserialize Time Eats the Win

Mitigation:

- Store only navigation-owned edge facts, not duplicated CPG nodes.
- Capture metadata file size.
- Benchmark warm sidecar load separately from first-build save.

### Risk: Save Cost Slows the First Call-Edge Query

Mitigation:

- Treat first-run sidecar write as an investment for subsequent CLI calls.
- Keep writes best-effort.
- Defer background/asynchronous save unless benchmarks prove the synchronous write is too costly.

### Risk: Non-Call-Edge Commands Accidentally Pay Edge Cache Cost

Mitigation:

- Do not load the sidecar in `NavigationIndex::build` or `build_cached_at`.
- Load the sidecar only inside `resolved_call_edges()`, which is reached by call-edge consumers.
- Add a test or benchmark proving `nodes-at` does not initialize `resolved_call_edges`.
- Store compact sidecar fingerprints so non-call-edge commands do not pay large metadata clone costs when building a
  cached navigation index.

### Risk: Type-Database Content Changes Reuse Stale CPG/Sidecar Data

Mitigation:

- Include a deterministic TypeDatabase content fingerprint in the CPG topology key whenever a TypeDatabase is present.
- The sidecar fingerprint includes the topology digest, so TypeDatabase fact changes invalidate both the CPG cache and
  any previously persisted call-edge sidecar.
- Navigation's normal `load_repo` path currently has no TypeDatabase input, but this protects the shared CPG cache
  contract used by the diff/review CLI and future TypeDatabase-backed navigation callers.

### Risk: Dirty Docs, Eval, or Target Repos Disable the Speedup

Mitigation:

- Scope dirty-load disabling to Prism binary inputs only.
- Ignore docs/eval/corpora/benchmark workspace dirtiness unless those files are actual analyzer binary inputs.
- Include untracked files under binary input paths, because they can affect the binary if referenced by tracked code.

### Risk: Two Cache Layers Become Hard to Reason About

Mitigation:

- Keep ownership in `src/navigation/cache.rs` and `src/navigation/call_edge_cache.rs`.
- Reuse the same topology and type-db inputs as CPG cache validation.
- Make sidecar cache misses non-fatal and never partial.

## Review Log

### 2026-07-02 A2A Design Review, Pass 1

Codex result: clear to implement after minor edits.

- Add MCP refresh/cache coverage to validation.
- Ensure `with_modified_cpg_for_testing` disables the sidecar store.
- Call out the missing `ResolutionKind` serde derive.

Claude result: clear to implement after minor edits, with one owner judgment.

- Sidecar-first is sound and safe, and lazy load inside `resolved_call_edges()` is the right seam.
- The recommendation needed clearer workload framing: Option A is strongest for repeated warm-cache CLI, MCP, and
  whole-repo consumers; Option B is more direct for first-and-only caller/callee queries.
- Add a pre-implementation measurement gate for `build_resolved_call_edges` share of warm one-shot latency.
- Correct the CLI test command and enumerate serialized fields.

### 2026-07-02 Measurement Gate

The measurement gate cleared before Option A was finalized:

- average `build_resolved_call_edges`: 1.867s
- average warm one-shot CLI caller/callee wall time: 2.458s
- edge-index build share: about 76%

The design now proceeds with Option A first and keeps Option B as a fallback after sidecar implementation
benchmarks.

### 2026-07-02 A2A Design Review, Pass 2

Codex result: clear to implement after minor edits.

- Add explicit invalidation tests for every sidecar fingerprint field.
- Make MCP refresh/cache validation mandatory and include incremental refresh after sidecar load.

Claude result: clear to implement after minor edits, with one owner judgment.

- Correctly frame dirty resolver iteration as a new sidecar-specific staleness class, not just the inherited CPG
  cache dirty-build caveat.
- Add `SKIP_POLICY_VERSION` to the sidecar envelope.
- Use unique temp filenames for atomic writes.
- Add tests for mutation-store disabling, serde round-trip preserving collision/drop and multiplicity facts,
  module graph parity, and sidecar version pinning.
- State that incremental navigation builds attach no sidecar store.
- Use `Arc` for store hash maps to reduce non-call-edge metadata clone cost.

### 2026-07-02 Owner Dirty-Build Policy

Owner accepted Option A with caveats:

- dirty-build sidecar warnings must be explicit;
- dirty sidecar-load disabling should be scoped to dirty Prism binary inputs, not docs/eval/measurement work or
  dirty target repositories such as Hugo;
- untracked files should count only when they are under binary-input paths.

The design now uses a dedicated binary-input dirty signal rather than raw `GIT_SHA=-dirty`, and it requires a
warning plus explicit override for dirty-build sidecar loads.

### 2026-07-02 A2A Design Review, Pass 3

Codex result: revise and re-review.

- Finding: the binary-input dirty policy was internally inconsistent if the existing CPG cache continued to validate
  against raw repo-wide `GIT_SHA`, because docs/eval dirtiness could still miss the CPG cache before sidecar logic.
- Resolution: the design now includes a shared cache identity cleanup. Raw `GIT_SHA` remains for version/reporting,
  while CPG cache and sidecar validation use `PRISM_CACHE_BUILD_IDENTITY`, derived from a deterministic binary-input
  content fingerprint for both clean and dirty builds.
- Finding: validation commands needed concrete integration and MCP targets.
- Resolution: validation now includes `cargo test --test navigation` and `cargo test --features mcp --test mcp`.

Claude pass 3 failed in the bridge transport before producing review content; rerun is required after this fold.

### 2026-07-02 A2A Design Review, Pass 4

Codex result: clear to implement after minor edits.

- Clarify that changing the existing CPG cache identity envelope requires a `CACHE_VERSION` bump.
- Use compact sidecar fingerprints instead of claiming `Arc<BTreeMap<...>>` avoids the initial metadata clone.
- State that the bincode sidecar envelope is authoritative and JSON metadata is diagnostics only.
- Use `nodes-at` or `functions`, not `call-stats`, as the non-call-edge benchmark sentinel.
- Document that type-db presence-only invalidation is intentionally inherited for this slice.

Claude pass 4 failed in the bridge transport before producing review content; rerun is required after this fold.

### 2026-07-02 A2A Design Review, Pass 5

Claude result: clear to implement after minor edits.

- Specify SHA-256 over sorted path/content records for the binary-input content fingerprint.
- Collapse the sidecar envelope to one `nav_call_edge_cache_version` field so the bump rule is unambiguous.
- Name `build.rs` as the emission site for `PRISM_CACHE_BUILD_IDENTITY` and `PRISM_BINARY_INPUT_DIRTY`.
- Owner judgment on docs-only commits was resolved toward binary-input content identity, not HEAD SHA, so docs/eval
  commits do not churn caches when analyzer binary inputs are unchanged.
