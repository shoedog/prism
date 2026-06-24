# MCP Auto-Incremental Refresh - Slice C Design

Status: A2A review findings folded; ready for implementation review
Date: 2026-06-24
Branch: `codex/mcp-auto-incremental-refresh`, based on `origin/main` at `ea04a89`
Parent spec: `docs/prism-query-layer/mcp-auto-refresh-incremental-correctness-design-2026-06-23.md`

## Review History

- Codex GPT-5.5 xhigh task `5d7c187d-00a8-4d84-a44f-05ae2e65066b` returned a bridge schema failure because
  its otherwise usable review included an extra JSON key. Valid findings folded:
  - Type database drift is not safely represented by `has_type_db`; this slice now disallows incremental when
    either active or fresh repo has a type DB.
  - File-set fallback reasons must win before topology reasons because the current topology key includes source
    path presence.
  - The test plan must prove the implementation uses an unbounded changed-file set, not capped
    `FreshnessReport.changed_paths`.
- Claude Opus task `736559ad-1cb2-4b59-b299-575cb89e8fc1` returned `review_required`. Valid findings folded:
  - The fallback taxonomy now uses independent comparisons and treats topology as a residual future reason.
  - Incremental candidates are in-memory only in this slice; they are not persisted to the shared nav CPG cache.
  - The test plan now includes chained incremental parity and dependency-changed cases.
  - Candidate retry fully replans on every attempt.
  - CLI wiring and additive AutoFull/refresh summary shape updates are explicitly called out.

## 1. Goal

Add an opt-in `AutoIncremental` MCP refresh policy that refreshes long-lived `prism-mcp` sessions with the CPG
incremental rebuild path only when strict preconditions prove that the edit is a source-content-only change in
the already indexed file set.

When those preconditions do not hold, `AutoIncremental` must fall back to the verified full refresh path shipped
by Slice A. The user-facing contract remains honesty-first:

- never use display-capped stale paths as rebuild input
- never return a fresh-looking answer unless the loaded candidate was verified against current disk
- never fail a navigation call only because incremental refresh is inapplicable
- never persist an incremental refresh product to the shared nav CPG cache in this slice
- keep `WarnOnly` and `AutoFull` behavior unchanged except for additive strategy metadata

## 2. Current Merged State

Slice A is merged:

- `RefreshPolicy` currently has `WarnOnly` and `AutoFull`.
- `SessionProvider::refresh_verified` builds an unpublished candidate, verifies the candidate snapshot against a
  fresh loader snapshot, retries once on divergence, and only then commits.
- a diverged committed candidate stays session-level stale through `known_stale_after_refresh` until a clean
  verified refresh clears it.
- `refresh_index` uses the verified candidate path.
- `AutoFull` attaches bounded `_meta` refresh metadata and preserves stale warnings on raced or failed refresh.

Slice B is merged:

- `CallSiteOrigin` distinguishes source call sites from synthetic C/C++ indirect-resolution sites.
- `CodePropertyGraph::build_incremental_with_scope_graph_inputs` recomputes C/C++ indirect calls over the
  merged whole call graph, then rebuilds scope/Rust receiver state, Go embedding, and Go interface dispatch in
  full-build order.
- full-vs-incremental CPG parity tests cover C/C++ indirect recall/stale-edge cases, Rust receiver
  rematerialization, Go derived state, import bindings, and mixed ordering.

The remaining implementation gap is navigation-level plumbing:

- `NavigationIndex::build_cached_at` intentionally treats `CacheResult::PartialHit` as a miss.
- `SessionProvider` owns only the active `NavigationSession` plus freshness state; it does not store the active
  `SnapshotFingerprint`.
- there is no explicit `NavigationIndex` constructor that takes the active in-memory CPG, a fresh `LoadedRepo`,
  and an unbounded changed-file set.

## 3. Non-Goals

- Do not change the default policy. `WarnOnly` remains default.
- Do not change `AutoFull` semantics.
- Do not change `NavigationIndex::build_cached_at` exact-hit-only partial-hit behavior.
- Do not add file watchers, debounce timers, async transport, or HTTP.
- Do not add a public manual incremental refresh argument to `refresh_index`.
- Do not change resolver, CPG, or navigation query semantics.
- Do not bump `CACHE_VERSION`; Slice B already handled the serialized CPG shape change.

## 4. User-Visible Contract

Extend the CLI/config policy:

```rust
pub enum RefreshPolicy {
    WarnOnly,
    AutoFull,
    AutoIncremental,
}
```

`prism-mcp --refresh-policy auto-incremental` means:

1. On a clean freshness probe, serve the requested tool as usual.
2. On stale drift, load a fresh repo snapshot and choose a refresh strategy.
3. If incremental preconditions pass, rebuild the navigation index incrementally from the active in-memory CPG.
4. If preconditions fail, run a verified full refresh and include fallback metadata.
5. Run the requested tool against the committed refreshed session.
6. If the candidate verification diverged after retry, attach the same `StaleIndex` warning and sticky
   known-stale state used by `AutoFull`.
7. If both incremental preparation and full fallback fail, preserve current `AutoFull` failure behavior: old
   session answer plus stale warning and bounded refresh-failure metadata.

Add strategy metadata to refresh summaries and auto-refresh `_meta`:

```text
strategy = "full" | "incremental"
fallback_reason = null | "type_db_present" | "file_set_changed" | "manifest_changed" |
                  "topology_changed" | "no_semantic_change" | "incremental_error"
```

For successful auto refresh, add:

```text
prism/refresh_strategy = "full" | "incremental"
prism/refresh_fallback_reason = <string>   # only when strategy == "full" because AutoIncremental fell back
```

`AutoFull` summaries should report `strategy = "full"` and no fallback reason. This is additive metadata, not a
behavior change.

## 5. Strategy Gate

`AutoIncremental` may use incremental rebuild only when all of these are true for active snapshot `old` and
freshly loaded candidate snapshot `new`:

- source file key sets are identical
- `changed_files = { file | old.file_hashes[file] != new.file_hashes[file] }` is non-empty
- every changed file is in the active indexed source-file set
- manifest hash maps are identical
- topology keys are identical
- neither the active repo nor the freshly loaded repo has a type database

Otherwise fall back to full refresh.

The changed-file set must come from an unbounded snapshot comparison, not from
`FreshnessReport.changed_paths`. `FreshnessReport.changed_paths` is a bounded display surface and may omit paths.

Recommended helper shape:

```rust
enum RefreshStrategy {
    Full,
    Incremental,
}

struct RefreshPlan {
    strategy: RefreshStrategy,
    changed_files: BTreeSet<String>,
    fallback_reason: Option<&'static str>,
}
```

Planning rules:

| Condition | Strategy |
|---|---|
| same files, same manifests, same topology, no type DB on either side, non-empty changed source set | Incremental |
| active or fresh repo has a type DB | Full uncached, `fallback_reason = "type_db_present"` |
| no semantic snapshot change despite stale freshness signal | Full, `fallback_reason = "no_semantic_change"` |
| source file set changed | Full, `fallback_reason = "file_set_changed"` |
| manifest hashes changed | Full, `fallback_reason = "manifest_changed"` |
| residual topology key changed after file-set and manifest changes are ruled out | Full, `fallback_reason = "topology_changed"` |

Use a deterministic priority order for fallback reasons:

1. type DB present on either side
2. source file set changed
3. manifest hashes changed
4. residual topology key changed after source-presence and manifest entries are ruled out
5. no semantic change

This keeps tests stable when multiple preconditions fail. The current `compute_topology_key` includes
`source:<path>` presence and `manifest:<path>` hashes, so a naive topology-first comparison would make
`file_set_changed` unreachable. In this slice, `topology_changed` is a residual future-proof reason: apply it
only after direct source file-set and manifest comparisons are equal and a remaining topology key difference is
observed.

## 6. Session State

Store the active verified snapshot on `SessionProvider`:

```rust
pub struct SessionProvider {
    cfg: ServerConfig,
    session: NavigationSession,
    freshness: FreshnessProbe,
    snapshot: SnapshotFingerprint,
    known_stale_after_refresh: Option<FreshnessReport>,
    generation: u64,
}
```

`SnapshotFingerprint` should record type database availability, but this is a disqualifying incremental
precondition rather than a sufficient fingerprint:

```rust
struct SnapshotFingerprint {
    file_hashes: BTreeMap<String, String>,
    manifest_hashes: BTreeMap<String, String>,
    topology_key: BTreeMap<String, String>,
    has_type_db: bool,
}
```

`verify_snapshot` compares the candidate snapshot to a freshly loaded current snapshot, including `has_type_db`.
`diff_report` should include a bounded display marker for type-db availability drift, for example `"type_db"`.

Do not treat `has_type_db == true` on both sides as incremental-safe. The current cache and snapshot metadata do
not fingerprint type database content or all compile-command/header inputs. Until that exists, `AutoIncremental`
must fall back to an uncached full rebuild whenever either side has a type database. This is deliberately stricter
than the existing `AutoFull` and nav cache path.

On commit:

- replace `session`
- replace `freshness`
- replace `snapshot`
- increment `generation`
- set or clear `known_stale_after_refresh` based on verification

## 7. Navigation Incremental Builder

Add an explicit builder; do not alter exact-hit-only nav cache behavior:

```rust
impl NavigationIndex {
    pub(crate) fn build_incremental_from_previous(
        previous: &NavigationIndex,
        repo: &LoadedRepo,
        changed_files: &BTreeSet<String>,
    ) -> Self {
        let cpg = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
            previous.cpg.call_graph.clone(),
            previous.cpg.dfg.clone(),
            changed_files,
            &repo.files,
            repo.type_db.clone(),
            repo.scope_graph_inputs.as_ref(),
        );
        Self::from_ctx(CpgContext::build_with_cached_cpg(
            &repo.files,
            cpg,
            repo.type_db.as_ref(),
        ))
    }
}
```

Rationale:

- the active `NavigationIndex` owns the old CPG, including the retained `CallGraph` and `DataFlowGraph`
- `CpgContext::build_with_cached_cpg` rebuilds `TypeRegistry` and `live_types` from the fresh files, matching
  current cache-hit behavior
- Slice B made the CPG incremental path behavior-equivalent for the supported cases
- `build_cached_at` stays conservative for normal CLI/MCP nav cache users

Incremental candidates are in-memory only in this slice. Do not save an incremental refresh product to the nav CPG
cache, even for `CacheMode::Default` or `CacheMode::Dir`. The shared cache may still be read or written by the
full fallback path through existing `NavigationIndex::build_cached_at`, but an incrementally produced CPG must
not survive process restart until chained incremental-vs-full parity has more coverage.

## 8. Verified Candidate Flow

Generalize the candidate builder rather than adding a second publish path:

```text
refresh_verified(strategy_policy):
  build candidate against active snapshot/session by running the full load -> snapshot -> plan -> build sequence
  verify candidate snapshot against current disk
  if diverged and retry remains:
    build another candidate against the same active snapshot/session by rerunning load -> snapshot -> plan -> build
    verify again
  commit latest candidate only if build succeeded and verification returned Clean or Diverged
```

For `AutoFull`, `strategy_policy = FullOnly`.

For `AutoIncremental`, `strategy_policy = PreferIncremental`.

Candidate construction:

```text
load_repo(cfg.repo_root) -> fresh_repo
fresh_snapshot = SnapshotFingerprint::from_repo(fresh_repo)
if policy == PreferIncremental:
  plan = plan_refresh(active_snapshot, fresh_snapshot, active_session)
else:
  plan = Full

if plan.strategy == Incremental:
  index = NavigationIndex::build_incremental_from_previous(active_session.index, fresh_repo, plan.changed_files)
else if plan.fallback_reason == "type_db_present":
  index = build full index without the nav CPG cache
else:
  index = build full index with existing cache behavior

candidate = SessionState { session, freshness, snapshot, strategy, fallback_reason }
```

If incremental construction unexpectedly errors in a future fallible implementation, build one full candidate
and set `fallback_reason = "incremental_error"`. Today the CPG incremental builder is infallible once inputs are
available, so tests can cover this through a test-only hook rather than production error plumbing.

Do not use an incremental candidate after it has been committed as the base for a second retry in the same
refresh operation. Each retry should compare the still-active snapshot to the newest loaded repo. This avoids
publishing partial state and keeps changed-file computation simple.

The refresh plan is per-attempt state. Never compute `plan_refresh` once and reuse it after a verifier divergence:
a second load may introduce a file-set, manifest, or topology change that invalidates the first attempt's
incremental eligibility.

Verification only proves candidate freshness relative to disk. It does not prove incremental-vs-full graph
equivalence. The implementation must rely on the parity tests below for that property and must keep the
incremental product in memory only.

## 9. Transport Dispatch

Change the stale auto-refresh dispatch predicate from:

```rust
stale && runtime.refresh_policy() == RefreshPolicy::AutoFull
```

to:

```rust
stale && matches!(
    runtime.refresh_policy(),
    RefreshPolicy::AutoFull | RefreshPolicy::AutoIncremental
)
```

The existing `auto_full_tool_response` helper can be renamed to `auto_refresh_tool_response`; its response-cap
reserve logic remains unchanged. Strategy/fallback metadata fits inside the existing `AUTO_REFRESH_RESERVE_BYTES`
budget and should be included in the low-cap tests.

Tool error rule remains unchanged: if refresh succeeds but the requested tool returns `isError`, return the tool
error shape without auto-refresh or stale metadata.

Update `src/bin/prism-mcp.rs` so the `--refresh-policy` value parser and match arm accept `auto-incremental`.

## 10. Manual Refresh

Keep `refresh_index` full-refresh-only in this slice. It is explicit and conservative, and it keeps the tool's
current mental model: "rebuild the snapshot."

The manual `RefreshSummary` still gains additive strategy fields:

```json
{
  "status": "refreshed",
  "strategy": "full",
  "fallback_reason": null
}
```

A future tool argument can opt into manual incremental refresh if users need it. This slice should not expand
`refresh_index` arguments because doing so would enlarge the review surface and require more protocol tests.

## 11. Metadata and Result Shape

Update structs:

```rust
pub struct RefreshSummary {
    pub status: &'static str,
    pub strategy: &'static str,
    pub fallback_reason: Option<&'static str>,
    ...
}

pub(crate) struct AutoRefreshSummary {
    pub strategy: &'static str,
    pub fallback_reason: Option<&'static str>,
    ...
}
```

Update `_meta` only on non-error tool results after successful auto refresh:

- `prism/refresh_strategy`
- `prism/refresh_fallback_reason`, only when `Some`

Do not add the fallback reason to `structuredContent` for normal nav tool results. `refresh_index` returns the
summary as structured content because that tool is itself a refresh report.

Existing AutoFull tests that assert `_meta` or `RefreshSummary` exact shape must be updated for the additive
`strategy` fields. Keep `fallback_reason` as `Option<&'static str>` so metadata remains bounded and fits inside
the existing reserve.

## 12. Test Plan

Session-level tests:

- `auto_incremental_policy_uses_incremental_for_indexed_file_edit`
- `auto_incremental_falls_back_to_full_on_file_addition`
- `auto_incremental_falls_back_to_full_on_file_deletion`
- `auto_incremental_falls_back_to_full_on_manifest_change`
- `auto_incremental_falls_back_to_full_on_topology_change`
- `auto_incremental_falls_back_to_full_when_type_db_present`
- `auto_incremental_no_semantic_change_falls_back_to_full_and_clears_stale_probe`
- `auto_incremental_raced_stale_remains_known_stale_until_clean_refresh`
- `auto_incremental_retry_replans_to_full_on_file_set_change`
- `auto_incremental_uses_unbounded_changed_files_not_capped_freshness_paths`
- `manual_refresh_index_reports_full_strategy`

Transport/MCP tests:

- `auto_incremental_refresh_uses_new_callers_after_edit`
- `auto_incremental_metadata_reports_incremental_strategy`
- `auto_incremental_full_fallback_metadata_reports_reason`
- `auto_incremental_under_floor_cap_preserves_strategy_metadata`
- `auto_incremental_tool_error_preserves_tool_error_shape`
- `auto_full_existing_metadata_tests_accept_strategy_fields`

Navigation/CPG integration tests:

- `navigation_incremental_builder_matches_full_for_python_callers`
- `navigation_incremental_builder_matches_full_for_c_indirect_callback`
- `navigation_incremental_builder_rebuilds_type_registry_and_live_types`
- `navigation_incremental_builder_chained_two_edits_matches_full`
- `navigation_incremental_builder_dependency_changed_go_embedding_or_interface_matches_full`
- `navigation_incremental_builder_dependency_changed_rust_receiver_matches_full`
- `navigation_incremental_builder_dependency_changed_python_import_or_inheritance_matches_full`
- `navigation_incremental_builder_dependency_changed_c_indirect_callback_matches_full`

At least one MCP test must use a C indirect-call fixture from Slice B so the public auto-incremental path proves
the same callers/callees result after an edit that would have been stale before the Slice B fix. Prefer a
dependency-changed fixture where the queried caller/dependent file is unchanged and only the callback target or
assignment file changes.

The unbounded changed-files test should edit more than the freshness display path cap (`MAX_STALE_PATHS`) across
indexed files, then assert through a test hook or strategy-planner unit that the incremental changed-file set
includes every changed source file. An implementation that accidentally uses `FreshnessReport.changed_paths` must
fail this test.

The chained parity test must build from a full v1 index, apply at least two consecutive incremental refreshes,
and compare the final navigation index to an independently built full index for the final files. Compare both
public query output and internal CG/DFG-normalized behavior where practical, because the next incremental refresh
uses retained `CallGraph` and `DataFlowGraph` state.

## 13. Validation

Targeted:

```bash
cargo fmt --check
cargo test --features mcp --lib mcp::session
cargo test --features mcp --lib mcp::transport
cargo test --features mcp --test mcp
cargo test --test ast cpg_cache
cargo test --test navigation
git diff --check main..codex/mcp-auto-incremental-refresh
```

Because this slice touches MCP and the navigation index, run the repo accuracy harness before PR:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

Do not rebaseline Tier-A in this slice. Put any regressions or flip candidates in the PR description.

## 14. Reviewer Checklist

Reviewers should look for:

- any path that uses capped `FreshnessReport.changed_paths` as the changed-file set
- any source-file addition/deletion, manifest, topology, or type-db drift that incorrectly uses incremental
- any candidate publication before verification completes
- any raced-stale path that clears `known_stale_after_refresh` too early
- any change to `NavigationIndex::build_cached_at` partial-hit behavior
- any missing strategy/fallback metadata on successful auto refresh
- any response-cap regression from the new metadata
- any mismatch between incremental nav output and full nav output for C indirect-call and ordinary direct-call
  fixtures
