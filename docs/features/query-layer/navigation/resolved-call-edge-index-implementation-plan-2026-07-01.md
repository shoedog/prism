# Resolved Navigation Call-Edge Index Implementation Plan

Status: ready to implement after A2A review
Date: 2026-07-01
Source design: `resolved-call-edge-index-design-2026-07-01.md`

## Goal

Speed repeated navigation queries by lazily building resolved call-edge indexes inside `NavigationIndex`, with
no resolver behavior change and no CPG cache shape change.

## Non-Goals

- Do not change `CallGraph::resolve_call_site` semantics.
- Do not add new language recall or precision behavior.
- Do not serialize the new index into the CPG cache.
- Do not change the diff/review CLI behavior.
- Do not rework `nav_ego_graph` traversal beyond collision-warning lookup.
- Do not make `nav_nodes_at`, `call_stats`, or interface-manifest commands pay whole-repo call-edge indexing.

## Task 0: Branch, Baseline, and Timing Harness

- [ ] Start from current `origin/main` after the design branch has merged.
- [ ] Record baseline status:

```bash
git status --short --branch
cargo test --test cli nav_compat_test::nav_callers_json_on_fixture
cargo test --test navigation
```

- [ ] Build release binary and collect baseline timing:

```bash
cargo build --release
rm -rf /tmp/prism-nav-speed-cache
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache callers --repo . --symbol build_session --file src/main.rs --depth 2 --format json >/tmp/prism-nav-callers-main.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache callees --repo . --symbol build_session --file src/main.rs --depth 2 --format json >/tmp/prism-nav-callees-main.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache repo-map --repo . --format json >/tmp/prism-nav-repomap-main.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache nodes-at --repo . --location src/main.rs:495 --format json >/tmp/prism-nav-nodes-at-main.json
/usr/bin/time -p target/release/prism nav --cache-dir /tmp/prism-nav-speed-cache call-stats --repo . >/tmp/prism-nav-call-stats-main.json
```

- [ ] Repeat the warm commands once with the populated cache and record the timings separately from cache-miss
  timings.

## Task 1: Add Lazy Navigation Index Types and Coherence Boundary

Files:

- `src/navigation/mod.rs`
- `src/navigation/call_resolve.rs` if small helper types belong there
- `src/main.rs` for CLI read-only accessor migration

Steps:

- [ ] Add owned `NavigationCallEdgeIndex`, `IndexedOutgoingCallSite`, `IndexedResolvedTarget`, and
  `IndexedIncomingCall` types.
- [ ] Add a private `OnceLock<NavigationCallEdgeIndex>` field to `NavigationIndex`.
- [ ] Add read-only accessors for direct callers, direct callees, collision drops, and all resolved outgoing edges.
- [ ] Close the public-mutation coherence hole before caching:
  - make `NavigationIndex.cpg` private or `pub(crate)` with read-only accessors;
  - add read-only accessors for existing consumers such as CLI `call_stats` and `interface_manifest`;
  - add explicit test helpers or constructors for tests that currently mutate `index.cpg.call_graph`;
  - if a mutation API remains, make it the only mutable path and have it clear or rebuild the resolved edge index.
    Do not leave `cpg` publicly mutable with caller-managed invalidation.
- [ ] Initialize the lazy field in all constructors.
- [ ] Keep the types navigation-local. Do not modify `CallGraph` serialized fields.

Validation:

```bash
cargo test --test navigation
```

## Task 2: Build the Lazy Index With Correct Sources

Files:

- `src/navigation/mod.rs`
- `src/navigation/call_resolve.rs`

Steps:

- [ ] Add a private builder called only by the `OnceLock` accessor.
- [ ] Build outgoing records from `self.cpg.call_graph.calls`, preserving unresolved sites.
- [ ] Build incoming records from `self.cpg.call_graph.callers`, not by reversing `calls`; this preserves duplicate
  indirect/callback sites that `calls` can deduplicate.
- [ ] Insert an incoming record only when the site matches the same candidate predicate as
  `scoped_caller_sites(target.name)`. Prefer extracting a shared helper such as
  `scoped_caller_site_matches(cg, bucket_key, site, target_name)` and using it from both the legacy query helper
  and the index builder.
- [ ] Add a `CallSiteKey` or equivalent memo key to avoid repeated resolution work while still pushing one incoming
  record per `callers` vector entry.
- [ ] Ensure any whole-index `CallSiteKey` includes full caller `FunctionId` plus every resolver-affecting call-site
  field, including receiver recovery/materialization and arity fields. Do not reuse bare `CallSite::cmp_key`
  across caller buckets.
- [ ] If full resolver-affecting equality is awkward for any field, skip cross-entry memoization instead of using a
  narrower unsafe key.
- [ ] If `CallSiteKey` is stored in `BTreeSet`/`BTreeMap`, represent resolver-affecting fields as owned comparable
  values. Do not omit non-`Ord` fields just to make the key compile.
- [ ] Store a `multi_owner_collision_sites` drop set, not a precomputed count per seed name. Populate it during
  the `cg.callers` pass from `resolve_call_site_full` or an equivalent full-outcome API so drop reasons are
  available.
- [ ] Keep stable `BTreeMap` iteration and deterministic vector order.

Tests:

- [ ] Add a unit test that compares outgoing records against direct `resolve_site_nav` output for a small Python
  fixture.
- [ ] Add a unit test that compares incoming records against `scoped_caller_sites` plus direct resolution.
- [ ] Add a regression proving indirect/callback duplicate caller sites from `cg.callers` are preserved even when
  `cg.calls` would deduplicate them.
- [ ] Add a regression with same-named caller functions in different files and identical call syntax resolving
  differently; memo/drop-set lookup must not conflate them.
- [ ] Add or identify a receiver/arity-sensitive regression proving memo/drop-set lookup does not conflate
  resolver-distinct call sites that share `CallSite::cmp_key` fields.
- [ ] Add a Rust `use m::secret as r; r()` or re-export alias regression proving the speed slice does not widen
  `nav_callers(secret)` beyond today's `scoped_caller_sites("secret")` candidate scope.
- [ ] Add a collision-warning parity test for same-name receiver collisions.
- [ ] Add an alias/member-import warning parity test if an existing fixture can cover it cheaply.

Validation:

```bash
cargo test --test navigation
cargo test --test name_resolution build_wiring_test
```

## Task 3: Route `nav_callers` and `nav_callees` Through the Index

Files:

- `src/navigation/queries.rs`
- `src/navigation/call_resolve.rs`

Steps:

- [ ] Replace `direct_callers` with an index lookup.
- [ ] Replace `direct_callees` with an index lookup.
- [ ] Preserve `exact_only`, unresolved callee output, BFS visited behavior, score calculation, reasons, and final
  sort.
- [ ] Make callers/callees Evidence sort total by adding call-site line, resolution kind, and stable name/qualifier
  tie-breakers after the existing score/file/function-line keys.
- [ ] Carry call-site `start_byte`/`end_byte` in indexed incoming/outgoing records and include those byte spans in
  the total sort tie-breaker.
- [ ] Keep legacy helper functions under `#[cfg(test)]` until parity tests pass, then either remove them or leave
  them as test-only oracles if they are compact.

Tests:

- [ ] Add parity tests comparing full `Evidence` from old and new paths for direct and depth-2 callers/callees.
- [ ] Add or update fixtures for unresolved callee output and NameOnly/ExactOnly filtering if current tests do not
  cover them.

Validation:

```bash
cargo test --test cli nav_compat_test::nav_callers_json_on_fixture
cargo test --test cli confidence_test
cargo test --test navigation callers_test
cargo test --test navigation callees_test
```

## Task 4: Route Module Graph Call Reasons Through the Index

Files:

- `src/navigation/module_graph.rs`
- `src/navigation/mod.rs` if an iterator accessor is useful

Steps:

- [ ] Replace `collect_module_edges` direct resolver calls with iteration over indexed resolved edges.
- [ ] Keep Rust scope-graph import dependency handling unchanged.
- [ ] Preserve `ModuleCallReason` ordering and final Evidence sorting.
- [ ] Do not cache grouped module edges in this slice unless measurements show resolver removal is insufficient.

Tests:

- [ ] Add a parity test that compares old grouped module call edges to the index-backed grouping.
- [ ] Keep existing `module_deps_consumes_scope_graph_but_resolution_output_stays_inert` green.

Validation:

```bash
cargo test --test name_resolution build_wiring_test::module_deps_consumes_scope_graph_but_resolution_output_stays_inert
cargo test --test navigation module_graph_test
cargo test --test cli nav_compat_test
```

## Task 5: Route Collision Warning Consumers Through the Drop Set

Files:

- `src/navigation/queries.rs`
- `src/navigation/call_resolve.rs`
- `src/navigation/mod.rs`

Steps:

- [ ] Replace production `call_resolve::collision_dropped_sites` calls in `callers_with_confidence` and `ego_graph`
  with a `NavigationIndex::collision_dropped_sites(seed_name)` accessor.
- [ ] Implement the accessor by calling `scoped_caller_sites(seed_name)` and filtering candidate sites by the
  precomputed `multi_owner_collision_sites` drop set.
- [ ] Retain `call_resolve::collision_dropped_sites` as a test oracle until parity coverage is complete.
- [ ] Confirm no production query path still re-resolves sites only for warnings.

Validation:

```bash
rg -n "collision_dropped_sites|resolve_site_nav" src/navigation
cargo test --test navigation
```

Expected grep result: production warning paths should no longer call `collision_dropped_sites` or re-resolve only
for warnings; retained legacy helpers and test-oracle references are allowed until parity coverage is complete.

## Task 6: Benchmarks and Acceptance Evidence

Steps:

- [ ] Re-run the baseline commands from Task 0 on the branch.
- [ ] Add or run an in-process benchmark/ignored test that builds one `NavigationSession` and then executes repeated
  `queries::callers`, `queries::callees`, and module graph calls without rebuilding the session.
- [ ] Compare:
  - cache-miss build plus query,
  - cache-hit build plus query,
  - single-shot `nodes-at`,
  - single-shot `call-stats`,
  - repeated warm callers,
  - repeated warm callees,
  - repo-map.
- [ ] Confirm JSON outputs are structurally equal to baseline outputs. If stderr cache messages differ, compare only
  stdout JSON.
- [ ] If existing golden output changes only because the new total sort reorders previously tied items, document
  that as an ordering-only rebless rather than a resolver behavior change.
- [ ] Record timing table in the PR description or implementation handoff.
- [ ] Record RSS/resident memory for the benchmark process.
- [ ] Re-check that benchmark seeds still exist before collecting final timings; the design-time seeds are not
  stable public API.

Minimum expected result:

- [ ] `nodes-at` and `call-stats` do not materially regress.
- [ ] Warm repeated callers/callees improve measurably on the Prism repo.
- [ ] Cache-hit index construction before first call-edge query does not materially regress.

## Task 7: Required Validation Before Review

Because this touches navigation queries, run the `AGENTS.md` Tier-A sequence after an immediate rebuild:

```bash
cargo fmt --check
cargo test --test cli nav_compat_test
cargo test --test cli confidence_test
cargo test --test navigation
cargo test --test name_resolution build_wiring_test
cargo test --lib navigation::tests::incremental_from_previous
cargo test --features mcp --lib mcp::transport::tests::auto_incremental_refresh
cargo build --release
(cd eval && uv run tier-a --matrix-only --allow-stale-sut)
(cd eval && uv run tier-a --quick --allow-stale-sut)
```

If Tier-A quick exits with the known harness/oracle nonzero state, inspect and report `baseline_invalid`,
`sut_error_rate`, and any regressions/flip-candidates rather than re-baselining.

## Task 8: Review and Merge

- [ ] Run full branch review through Claude and Codex.
- [ ] Fold valid correctness, determinism, or measurement findings.
- [ ] Re-review until only minor issues remain or the branch is ready to merge.
- [ ] Submit PR, wait for green checks, merge, and push `main`.

## Open Follow-Ups

- Consider lifting the resolved-edge index into a shared non-serialized `CpgContext` service only after this
  navigation slice proves the model and speedup.
- Consider caching grouped module-edge maps if `repo_map` remains dominated by grouping rather than resolution.
- Consider using the same resolved-edge service for `CallGraph::resolved_caller_edges` algorithm consumers in a
  separate, reviewed performance slice.
