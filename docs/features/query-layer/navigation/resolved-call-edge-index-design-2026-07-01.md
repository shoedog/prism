# Resolved Navigation Call-Edge Index Design

Status: ready to implement after A2A review
Date: 2026-07-01
Primary priority: query-layer speed without recall or precision drift
Companion plan: `resolved-call-edge-index-implementation-plan-2026-07-01.md`

## Table of Contents

- [Decision Summary](#decision-summary)
- [Architecture Analysis](#architecture-analysis)
- [Proposed Design](#proposed-design)
- [Correctness Contract](#correctness-contract)
- [Benchmark Plan](#benchmark-plan)
- [Rejected Alternatives](#rejected-alternatives)
- [Risks and Mitigations](#risks-and-mitigations)
- [Review Log](#review-log)

## Decision Summary

Add a navigation-local, in-memory resolved call-edge index to `NavigationIndex`.

The first implementation should lazily build resolved outgoing and incoming call-edge indexes the first time a
call-edge navigation consumer needs them, then reuse that index for the rest of the `NavigationSession`. Route
`nav_callers`, `nav_callees`, collision warnings, and call-derived module dependency queries through the lazy
index. Preserve current resolver behavior and Evidence output. This is a performance slice, not a recall or
precision slice.

The index belongs in `src/navigation/`, not in serialized `CallGraph`, for the first slice:

- It avoids a `CACHE_VERSION` bump because navigation indexes are rebuilt from the cached CPG in memory.
- It keeps the diff/review CLI path unchanged.
- It lets us prove the speedup and parity before deciding whether a shared `CallGraph` or `CpgContext`
resolved-edge cache is worth the wider blast radius.

The key tradeoff is moved from build time to first call-edge use. `nav_nodes_at`, `call_stats`, and other
non-call-edge commands should not pay whole-repo resolution just because a `NavigationIndex` exists. Long-lived
MCP sessions and in-process repeated query workflows should pay the call-edge resolution cost once and then
reuse it.

## Architecture Analysis

### Current State

`NavigationIndex` currently owns:

- `cpg: CodePropertyGraph`
- `types: TypeRegistry`
- `live_types: BTreeSet<String>`
- `line_range_index` for location seeds
- `name_index` for symbol seeds

The current query layer then re-derives resolved call facts on demand:

- `src/navigation/queries.rs::direct_callers` scans candidate caller-site buckets from
  `call_resolve::scoped_caller_sites`, calls `resolve_site_nav` for each candidate site, then identity-filters
  to the target `FunctionId`.
- `src/navigation/queries.rs::direct_callees` walks `call_graph.calls[caller]` and calls `resolve_site_nav` for
  every call site each time the caller is expanded.
- `src/navigation/queries.rs::collision_dropped_sites` warning calculation scans candidate sites and calls
  `resolve_call_site_full` again.
- `src/navigation/module_graph.rs::collect_module_edges` scans every caller and every site, then calls
  `resolve_site_nav` to build cross-file dependency reasons for `nav_module_deps` and `nav_repo_map`.
- `nav_ego_graph` already traverses the CPG petgraph edges; the only obvious repeated resolver work in that
  path is the call-collision warning for Call-edge ego queries.

This repeats deterministic resolver work across:

- repeated MCP calls in the same server session,
- breadth/depth expansion within `callers` and `callees`,
- `module_deps` and `repo_map` calls after the same index is already loaded,
- warning paths that re-scan the same candidate sites.

### Why This Is Next

The larger recall/precision slices that motivated earlier planning have landed or are tracked elsewhere:

- JS/TS import-member Tier-A coverage has landed.
- Python inherited receiver resolution has landed.
- `CpgContext` and JS/TS destructuring alias tracking have landed.
- The remaining language-construct gaps are lower priority or architectural.

That leaves query-layer speed as the clearest CPG-adjacent improvement with a measurable acceptance gate and
low semantic risk.

### Scope Boundary

In scope:

- Build resolved nav call-edge indexes lazily from `NavigationIndex` on first call-edge use.
- Route callers/callees and call-derived module graph code through the indexes.
- Preserve unresolved callee output for `nav_callees`.
- Preserve collision warnings.
- Add parity tests against the existing resolver path.
- Add single-shot and in-process repeat-query benchmarks for current main versus the branch.

Out of scope:

- Changing `CallGraph::resolve_call_site` semantics.
- Adding new call-resolution rungs.
- Changing CPG graph edge construction.
- Changing serialized `CallGraph` or CPG cache shape.
- Replacing CPG `Call` edge traversal inside `nav_ego_graph`.
- Moving this cache into `CpgContext` before measurements prove the wider reuse is worth it.

## Proposed Design

### Data Model

Add navigation-owned records with owned, deterministic data. Do not store references into `CallGraph`; the
index must survive as ordinary owned state inside `NavigationIndex`.

```rust
pub struct NavigationCallEdgeIndex {
    outgoing_by_caller: BTreeMap<FunctionId, Vec<IndexedOutgoingCallSite>>,
    incoming_by_target: BTreeMap<FunctionId, Vec<IndexedIncomingCall>>,
    multi_owner_collision_sites: BTreeSet<CallSiteKey>,
}

pub struct IndexedOutgoingCallSite {
    pub callee_name: String,
    pub call_site_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub resolved: Vec<IndexedResolvedTarget>,
}

pub struct IndexedResolvedTarget {
    pub target: FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

pub struct IndexedIncomingCall {
    pub caller: FunctionId,
    pub callee_name: String,
    pub call_site_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}
```

**`ResolutionKind` has grown nav-only synthetic kinds since this design was written**, each surfaced directly from
a dedicated whole-program `CallGraph` table rather than from `resolve_call_site_full` (no CPG `Call`/`Return` edge,
no resolve-time consult path): `callback_registration`/`func_value_field` (P5, Go function-value callbacks),
`property_access` (P7, Python `@property`/`@cached_property` access), and `framework_entry` (P9, Flask/FastAPI/
Express route registrations — see `CallGraph::apply_framework_entries`, `src/framework_entries.rs`). All three
merge into `NavigationCallEdgeIndex` the same way: a loop over the dedicated `BTreeSet` beside the `cg.calls`/
`cg.callers` loops in `NavigationIndex::build_resolved_call_edges`.

`CallSiteKey` must be a whole-index key, not just `CallSite::cmp_key`. `CallSite::cmp_key` is only safe inside a
`calls[FunctionId]` bucket because it includes `caller.name` but not the caller file or span. The navigation
memo/drop-set key must include the full caller `FunctionId` plus every call-site field that can affect resolver
output. `CallSite::cmp_key` fields are necessary but not sufficient because resolver behavior can also depend on
receiver recovery/materialization and arity fields that are excluded from set ordering.

```rust
pub struct CallSiteKey {
    pub caller: FunctionId,
    pub callee_name: String,
    pub line: usize,
    pub kind: CallKind,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub receiver_type: Option<String>,
    pub receiver_recovery: Option<ReceiverRecovery>,
    pub receiver_materialized: bool,
    pub arg_count: Option<usize>,
    pub arg_spread: bool,
    pub receiver_outcome: Option<ReceiverOutcome>,
}
```

An implementation may use a narrower key only inside data already bucketed by full caller identity and only when
the omitted fields are proven irrelevant to the specific cached value. If carrying equality for every
resolver-affecting field is awkward, do not memoize across `cg.callers` entries; correctness beats the small memo
optimization.

The key must be comparable if it is stored in a `BTreeSet`/`BTreeMap`. If a resolver-affecting field's native type
does not implement `Ord`, normalize it into an owned comparable fingerprint for the key or skip memoization for
that path. Do not drop the field merely to satisfy ordering.

Add one lazy field to `NavigationIndex`:

```rust
resolved_call_edges: OnceLock<NavigationCallEdgeIndex>,
```

`outgoing_by_caller` covers direct callees and module graph consumers. `incoming_by_target` covers direct
callers. `multi_owner_collision_sites` stores the sites whose full resolution drops with
`MultiOwnerCollision`; warning code still uses `call_resolve::scoped_caller_sites(seed_name)` and filters that
candidate set by the drop set. This reuses the existing scoped-name and member-import alias semantics instead
of inverting them into a fragile precomputed count.

Keep the new index private and expose small accessors:

- `NavigationIndex::direct_callers(target: &FunctionId)`
- `NavigationIndex::direct_callees(caller: &FunctionId)`
- `NavigationIndex::collision_dropped_sites(seed_name: &str)`
- `NavigationIndex::resolved_call_edges()`

### Coherence Boundary

The lazy index is correct only if the CPG/call graph is immutable after index construction. Today
`NavigationIndex.cpg` is public and tests mutate `index.cpg.call_graph.scope_graph` after build. The
implementation must close that coherence hole before caching resolved edges:

- Make `NavigationIndex`'s new call-edge index private.
- Make `NavigationIndex.cpg` non-publicly mutable before enabling cached call edges. The preferred shape is
  private or `pub(crate)` storage with read-only accessors.
- Add read-only accessors for existing binary/query consumers that currently read
  `session.index.cpg.call_graph`, including `call_stats` and `interface_manifest`.
- For tests that need modified call graphs, add explicit test helpers or constructors that rebuild a
  `NavigationIndex` after the mutation rather than mutating a live index behind the cache.
- A mutation API is acceptable only if it is the sole mutable path and always clears or rebuilds the resolved
  edge index. Leaving `cpg` publicly mutable while relying on callers to remember invalidation is not acceptable.

### Build Algorithm

Build on first access through `NavigationIndex::resolved_call_edges()`.

1. Build outgoing callees by iterating `self.cpg.call_graph.calls` in `BTreeMap` order. This matches today's
   `direct_callees` source and preserves unresolved sites by storing an `IndexedOutgoingCallSite` even when
   `resolved` is empty.
2. Build incoming callers by iterating `self.cpg.call_graph.callers`, not by reversing `calls`. This is
   load-bearing: `calls` is a `BTreeSet` keyed by `CallSite` ordering and can deduplicate indirect/callback
   sites that `callers: Vec<CallSite>` preserves. `nav_callers` currently reads `callers`, so the incoming
   index must preserve that multiplicity.
3. Preserve legacy incoming-candidate scope. An incoming record for a resolved `(site -> target)` edge is inserted
   only if that site would be returned by today's `scoped_caller_sites(target.name)` candidate predicate:
   - caller bucket key equals the target name,
   - or caller bucket key ends with `::{target_name}`,
   - or the site is an eligible Python/JS/TS member-import alias candidate in the caller file with no qualifier.
   This deliberately does not widen Rust alias/re-export recall in a speed-only slice. Prefer refactoring
   `scoped_caller_sites` to share a `scoped_caller_site_matches(cg, bucket_key, site, target_name)` helper so the
   query helper and index builder cannot drift.
4. Use a small memo only if it is keyed by a whole-index `CallSiteKey` that includes the full caller `FunctionId`
   and every resolver-affecting call-site field. This avoids resolving the same logical site more than necessary
   while still pushing one incoming record for every `callers` vector entry. Do not reuse bare `CallSite::cmp_key`
   across caller buckets, and skip cross-entry memoization if the full key is impractical.
5. Populate `multi_owner_collision_sites` during the `cg.callers` pass from full resolution outcomes, using
   `resolve_call_site_full` or an equivalent API that preserves drop reasons. `resolve_call_site` alone is
   insufficient because it discards `DropReason`.
6. Warning queries then call `scoped_caller_sites(seed_name)` and count candidate sites whose key is in the drop
   set.
7. Keep stable iteration order. Query code should also gain total tie-breakers so Evidence order does not depend
   on whether the source collection was a `Vec` or a `BTreeSet`.

### Query Integration

`nav_callers`:

- Replace `direct_callers` with an index lookup by exact `FunctionId`.
- Preserve the `exact_only` filter.
- Preserve one Evidence item per call site.
- Preserve BFS visited behavior: items are per site; `visited` only gates recursive expansion.
- Make final sort order total: score, file, function start line, call-site line, call-site byte span, resolution
  kind, and a stable name/qualifier tie-breaker. This removes hidden dependence on insertion order for tied
  Evidence items.

`nav_callees`:

- Replace `direct_callees` with an `outgoing_by_caller[caller]` lookup.
- Preserve unresolved output when `resolved` is empty.
- Preserve one Evidence item per resolved callee edge.
- Preserve the `exact_only` behavior where unresolved sites are excluded when exact-only is requested.
- Make final sort order total using the same additional call-site, byte-span, and kind tie-breakers.

`nav_module_deps` and `nav_repo_map`:

- Replace direct calls to `resolve_site_nav` with iteration over `NavigationIndex` resolved edges.
- Continue grouping into `(source_file, target_file)` and `ModuleCallReason`.
- Keep resolved import facts from the Rust scope graph unchanged.
- Outgoing/module-graph parity is exact because both the current `collect_module_edges` implementation and the new
  outgoing index consume `cg.calls`.
- If grouping cost remains visible after resolver cost is removed, cache the grouped module-edge map as a
  follow-up. Do not introduce that extra cache in the first implementation unless the benchmark shows it is
  necessary.

`nav_ego_graph`:

- Leave petgraph traversal unchanged.
- Replace only the collision-warning calculation with the drop-set-backed `collision_dropped_sites` accessor.
- Do not try to reinterpret CPG `Call` edges from the new index in this slice; that risks graph-shape drift
  with no clear speed benefit.

### Cache and Incremental Refresh

No CPG `CACHE_VERSION` bump is expected for the first slice because:

- The new lazy field is in `NavigationIndex`, not serialized `CodePropertyGraph` or `CallGraph`.
- `build_cached` loads a cached CPG and then returns a fresh `NavigationIndex` with an empty `OnceLock`.
- `build_incremental_from_previous` already returns through `from_ctx`, so refreshed sessions get a fresh empty
  lazy index that will be built from the current CPG on first use.

If implementation moves any resolved-edge facts into serialized `CallGraph`, the design must be revised and a
cache-version bump becomes mandatory.

### Future Lift Into Shared CPG Context

Several slicing algorithms call `CallGraph::resolved_caller_edges`, which scans all call sites for each target.
This design intentionally does not change those algorithms. The lower-risk sequence is:

1. Prove the data model and parity in navigation.
2. Measure the repeated-query speedup and build overhead.
3. If useful, consider a second slice that lifts the resolved-edge index into a shared, non-serialized
   `CpgContext` service for algorithm use.

That follow-up would have a wider blast radius and should have its own review.

## Correctness Contract

This is a semantics-preserving refactor.

The implementation is acceptable only if:

- Existing fixture outputs for `nav_callers`, `nav_callees`, `nav_module_deps`, `nav_repo_map`, and relevant MCP
  smoke tests are unchanged except for documented ordering-only changes among items that were previously tied on
  score/file/function-line and therefore insertion-order dependent.
- New parity tests prove index-backed direct callers/callees match the legacy resolver-backed helpers for:
  same-name collisions, alias/member imports, scoped callee names, unresolved calls, NameOnly results, and
  ExactOnly filters.
- Collision warning counts match the old `collision_dropped_sites` behavior.
- Incoming caller parity includes indirect/callback duplicate sites that are present in `cg.callers` but
  deduplicated in `cg.calls`.
- Memo/drop-set key parity includes two same-named caller functions in different files with identical call
  syntax resolving differently; they must not share cached resolution.
- Rust alias/re-export callers that resolve through scope graph but are not returned by
  `scoped_caller_sites(target.name)` do not appear in `nav_callers` in this speed-only slice.
- The lazy index is not constructed by `nodes_at`, `call_stats`, or interface-manifest queries.
- Existing incremental navigation tests still pass, proving refreshed indexes reflect changed files.
- Tier-A matrix and quick runs do not show resolver regressions. The intended change is navigation speed, but
  `AGENTS.md` requires Tier-A validation when navigation queries are touched.

## Benchmark Plan

Measure four phases separately:

1. Cold or cache-miss index build time.
2. Cache-hit index build time.
3. Single-shot query time for commands that should not build the call-edge index.
4. In-process repeated query time after the lazy call-edge index exists.

Use at least:

- a small fixture repo for stable CI-smoke timing,
- the Prism repo itself for realistic Rust/Python/JS/TS call graph shape,
- one medium external repo only if available locally and cheap to run.

Recommended local single-shot commands for the implementation branch:

```bash
cargo build --release
rm -rf /tmp/prism-nav-speed-cache
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache nodes-at --repo . --location src/main.rs:495 --format json >/tmp/prism-nav-nodes-at.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache call-stats --repo . >/tmp/prism-nav-call-stats.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache callers --repo . --symbol build_session --file src/main.rs --depth 2 --format json >/tmp/prism-nav-callers.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache callers --repo . --symbol build_session --file src/main.rs --depth 2 --format json >/tmp/prism-nav-callers-warm.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache callees --repo . --symbol build_session --file src/main.rs --depth 2 --format json >/tmp/prism-nav-callees-warm.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache repo-map --repo . --format json >/tmp/prism-nav-repomap-warm.json
```

Add an in-process benchmark or ignored test that builds one `NavigationSession` and then runs a sequence such as:

- 20 `queries::callers` calls across at least two seeds,
- 20 `queries::callees` calls across at least two seeds,
- 5 `module_graph::repo_map` or representative `module_deps` calls.

Acceptance target:

- No material regression in `nodes-at` or `call-stats` single-shot timing.
- No material regression in cache-hit index construction before first call-edge use.
- Repeated `callers`/`callees` query time should improve measurably on the Prism repo. A 2x warm-query speedup
  is a reasonable minimum target; larger gains are expected for high-call-count repos and deeper BFS queries.
- Output files should be byte-identical or structurally equal where stderr cache messages differ.
- Report RSS or resident memory for the benchmark process, since incoming and outgoing indexes intentionally
  retain additional session state.
- Re-check the concrete benchmark seed symbol and line numbers at implementation time; `build_session` and
  `src/main.rs:495` are current as of this design, not a stable public contract.

## Rejected Alternatives

### Store Resolved Edges In Serialized `CallGraph`

Rejected for this slice. It would allow broader reuse but forces cache-version and constructor/incremental path
work. The current goal is a low-risk navigation speed slice.

### Derive Navigation Query Results From CPG Petgraph Call Edges

Rejected. CPG `Call` edges do not carry enough query evidence: call-site line, qualifier, resolution kind, and
unresolved site records are needed by current Evidence output.

### Cache Only `scoped_caller_sites`

Rejected as too shallow. It avoids part of the candidate scan but still resolves the same sites repeatedly and
does nothing for `callees` or module graph queries.

### Eagerly Build The Index In `from_ctx`

Rejected after review. It would make `nodes-at`, `call_stats`, and other non-call-edge commands pay whole-repo
call resolution even when they never consume call edges. Lazy construction gives repeated MCP sessions the same
amortization benefit without pessimizing single-shot non-call-edge commands.

### Skip Module Graph Refactor

Rejected for the plan, though it can be a separate commit. `module_deps` and `repo_map` are whole-repo consumers
of the same resolved call facts. Leaving them on the old path would preserve a large repeated resolver scan in a
common orientation workflow.

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Build-time overhead hides warm-query gains | Measure build and query phases separately; do not claim speedup from mixed timings. |
| `nav_callers` loses indirect/callback duplicate sites | Build incoming indexes from `cg.callers`, not by reversing `cg.calls`; add a duplicate-preservation regression. |
| `nav_callers` widens recall for Rust aliases/re-exports | Insert incoming records only when the site matches the same candidate predicate as `scoped_caller_sites(target.name)`; add a Rust `use ... as` regression. |
| Whole-index memo conflates resolver-distinct call sites | `CallSiteKey` includes full caller `FunctionId` and every resolver-affecting call-site field, or the implementation skips cross-entry memoization; add same-name and receiver/arity regressions. |
| Collision warning counts drift | Store a multi-owner drop set and continue using `scoped_caller_sites`; add parity tests for scoped names and alias imports. |
| Unresolved `nav_callees` entries disappear | Store unresolved call sites in `outgoing_by_caller` and test them. |
| Module graph reason ordering changes | Keep `ModuleCallReason` ordering and final sort unchanged; compare old/new grouped edges in tests before deleting the legacy path. |
| Public CPG mutation makes cached edges stale | Remove public mutable CPG access before enabling lazy cached edges; any remaining mutation API must own invalidation. |
| Incremental refresh produces stale indexes | Fresh sessions get empty lazy indexes; keep existing incremental full-vs-incremental tests green. |
| Wider algorithm consumers still scan | Treat shared `CpgContext` resolved-edge service as a follow-up after navigation measurements. |

## Review Log

| Round | Reviewer | Result | Findings Folded |
|-------|----------|--------|-----------------|
| 1 | Claude + Codex via A2A | Fix then implement | Incoming caller source must be `cg.callers`; lazy build required; CPG mutation coherence required; collision warnings use drop set plus `scoped_caller_sites`; total Evidence sort and corrected validation commands required. |
| 2a | Codex via A2A | Fix then implement | Whole-index `CallSiteKey` must include full caller identity, not only `CallSite::cmp_key`; folded into data model, build algorithm, tests, and risk table. |
| 2b | Codex via A2A | Clear to implement with minors | Added read-only CPG/call-graph accessor requirement and call-site byte spans for total sort tie-breakers. |
| 2c | Claude via A2A | Fix then implement | Incoming index must preserve `scoped_caller_sites(target.name)` candidate scope to avoid Rust alias/re-export recall widening; folded into build algorithm, data model, tests, and risk table. |
| 2d | Claude via A2A | Clear to implement with minors | Pinned drop-set population to the callers pass, clarified tied-order fixture drift, and noted benchmark seed recheck. |
| 2e | Codex via A2A | Fix then implement | Publicly mutable `NavigationIndex.cpg` must be removed before cached edges; invalidation is acceptable only behind a controlled mutation API. |
| 2f | Codex via A2A | Fix then implement | Required validation must explicitly run unit and MCP incremental-refresh tests; folded into the implementation gate. |
| 2g | Codex via A2A | Fix then implement | Whole-index `CallSiteKey` must include resolver-affecting receiver and arity fields, not only `CallSite::cmp_key`; folded into data model, build algorithm, and risk table. |
| 2h | Codex via A2A | Clear to implement with minors | Clarified comparable `CallSiteKey` representation and expected grep matches for retained test oracles. |
