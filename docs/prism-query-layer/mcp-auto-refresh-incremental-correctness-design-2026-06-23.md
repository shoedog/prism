# MCP Auto-Refresh and Incremental Rebuild Correctness - Architecture Spec

Status: A2A reviewed; Slice A implemented in `codex/mcp-auto-full-refresh`; Slices B/C future
Date: 2026-06-23
Branch: `main` at spec time
Scope: three coordinated follow-on slices after MCP staleness honesty and manual `refresh_index`

## Review History

- Codex GPT-5.5 xhigh task `3d361266-e51e-44ae-8293-9b99651f04d6` returned
  `review_required`. Valid findings folded:
  - AutoFull response-cap handling must define an explicit combined reserve before shaping.
  - Incremental parity must compare every resolution-affecting `CallSite` field and all resolved-edge/drop
    behavior, not a narrow callsite tuple.
  - AutoIncremental must never use bounded freshness display paths as the changed-file set.
- Claude Opus task `3dd41369-b5f7-456e-b42d-2756eeb134b1` failed with bridge idle timeout before returning
  findings. Later Claude default review attempts are recorded below.
- Codex GPT-5.5 xhigh re-review task `afac2084-97db-4e88-8147-173443dc6348` returned
  `review_required`. Valid findings folded:
  - AutoFull must verify that the loaded snapshot matches current disk before claiming a clean refresh; a
    post-refresh `FreshnessProbe::check` alone misses load-to-baseline races.
  - Auto-refresh metadata behavior for tool error results must be exact and tested.
  - Array-dispatch unchanged-caller stale-edge fixtures must match current same-file semantics.
- Claude default re-review task `97345cb3-4ba2-45b9-809a-31be8bef57c0` returned `input_required` with
  `review_required` artifacts. Valid findings folded:
  - Incremental whole-graph recompute order should match full build order.
  - Prefer additive `CallSiteOrigin` over a new `CallKind` variant.
  - AutoFull clean-success metadata must be `_meta`-only and reset `anthropic/maxResultSizeChars` to the
    original cap.
  - Note whole-repo indirect recompute cost, type-registry/live-types oracle scope, and the currently infallible
    `build_incremental` error surface.
- Codex GPT-5.5 xhigh final review task `508e5a4a-f467-4e76-9e4d-d4255b2859a8` returned
  `review_required`. Valid findings folded:
  - AutoFull cannot verify after calling today's `SessionProvider::refresh`, because that API has already
    published the rebuilt session and freshness baseline. Slice A needs a pending build/verify/commit refresh
    path.
  - Low-cap raced-stale and refresh-failure paths need a deterministic floor-cap contract; combined reserves
    must not saturate the tool budget to zero at the minimum accepted production cap.
- Codex GPT-5.5 xhigh focused re-review task `41bd14bb-3693-4cff-8ba0-b084e330c0bb` returned
  `review_required`. Valid findings folded:
  - A committed diverged candidate must remain session-level known-stale until a clean refresh; otherwise the
    next request can see the new baseline as clean and return a stale answer without warning.
  - `SessionRuntime` must expose an explicit auto-refresh candidate operation; dispatch must not downcast or call
    the forbidden published refresh path.
- Claude default focused re-review task `0f959759-b094-498d-98e2-22a4badc4303` returned `completed`. Non-blocking
  clarifications folded:
  - Direct+synthetic `CallSite` collision tests should target `callers`, because `calls: BTreeSet<CallSite>`
    cannot hold two sites with the same `cmp_key` when `origin` is excluded from `Ord`.
  - The parity oracle should pin incremental output to the canonical full-build order, since Slice B changes
    already-shipped CLI incremental recompute ordering.
  - Raising `MAX_RESULT_CHARS_FLOOR` changes sub-floor cap behavior; constant names should map to existing
    `transport::ENVELOPE_RESERVE`.
- Codex GPT-5.5 xhigh closure review task `8d008444-9131-441d-97c9-0c33061fde25` returned `completed` with no
  findings.
- Claude default closure review task `25dc2ace-fb8f-4b56-a31e-1fe32fcb1a2b` returned `completed`. Non-blocking
  clarifications folded:
  - Unpublished candidates may still write content-addressed CPG cache entries to disk; the unpublished part is
    runtime session publication.
  - Manual `refresh_index` must migrate onto the verifying candidate path to preserve sticky known-stale
    semantics.
  - Commit-as-`Diverged` happens only after the retry budget is exhausted, and Slice B's recompute-order change
    should be called out as a CLI incremental behavior change.

## 1. Goal

Make long-lived `prism-mcp` sessions safer after repository edits without trading an honest stale warning for a
fresh-looking but partially stale graph.

This spec covers three related slices:

1. **Auto-full MCP refresh on drift**: opt-in request-time refresh using the existing provider-backed full
   `NavigationSession` rebuild path.
2. **Incremental CPG correctness gate**: full-vs-incremental parity tests plus a fix for the known C/C++
   indirect-call recomputation gap.
3. **Future auto-incremental MCP refresh**: use the incremental path only after the parity gate proves that
   incremental rebuilds are behavior-equivalent to full rebuilds for the supported cases.

The main design rule is:

> MCP may automatically refresh only through a rebuild path that is as correct as a full rebuild for the final
> repository state.

## 2. Current Source State

### 2.1 MCP already has honesty and manual full refresh

`SessionProvider` owns the current session, freshness baseline, and generation:

- `src/mcp/session.rs`
  - `SessionProvider::bootstrap` builds one session and one freshness baseline.
  - `SessionProvider::refresh` calls `build_state` again, replaces the session, replaces the freshness
    baseline, and increments `generation`.
  - `build_state` runs `load_repo`, constructs `FreshnessProbe`, and builds a `NavigationIndex`.

`src/mcp/transport.rs` has two relevant paths:

- `refresh_index` tool calls `runtime.refresh_index()` and returns a refresh summary.
- normal known tools call `FreshnessProbe::check`; if stale, Prism reserves response bytes and applies a
  `StaleIndex` warning to successful structured results.

This means current MCP behavior is:

- fresh calls: normal result
- stale calls: stale warning, no automatic rebuild
- explicit `refresh_index`: full session rebuild through `SessionProvider::refresh`

### 2.2 Navigation cache is exact-hit-only for partial cache hits

`NavigationIndex::build_cached_at` intentionally treats `CacheResult::PartialHit` as a miss and does a full
whole-repo `CpgContext::build_with_scope_graph_inputs`. This is important: the current MCP refresh path can use
`CacheMode::Default` or `CacheMode::Dir` without invoking `CodePropertyGraph::build_incremental` for partial
hits. It may load exact hits, but partial hits rebuild fully and then save the new cache.

### 2.3 CLI/review CPG cache still uses incremental partial hits

The review CLI path in `src/main.rs` still handles `CacheResult::PartialHit` by calling
`CodePropertyGraph::build_incremental_with_scope_graph_inputs`. This is a separate correctness surface from MCP
navigation caching, but it shares the same CPG incremental code that future MCP auto-incremental would depend on.

### 2.4 Incremental CPG has a documented indirect-call limitation

`CodePropertyGraph::build_incremental_with_scope_graph_inputs` currently does:

1. remove changed files from cached CG/DFG
2. build fresh `CallGraph::build_direct_subset` and `DataFlowGraph::build_subset` for changed files
3. merge fresh into retained cached data
4. recompute Go embedding promotion and Go interface dispatch
5. rebuild the Rust scope graph and rematerialize Rust receiver outcomes
6. assemble the CPG

The issue is step 2: `CallGraph::build_direct_subset` is direct-only. Its comment says callers should run
indirect resolution on the merged result, but `build_incremental_with_scope_graph_inputs` does not currently
rerun the C/C++ Phase-3 indirect-call resolution. Full `CallGraph::build` has the Phase-3 logic inline for:

- local function pointer assignment
- array dispatch tables
- struct field callbacks
- parameter-passed function pointers

Without a recompute, incremental can miss indirect targets for changed callers and can retain stale derived
indirect call sites when an unchanged caller's old derived target becomes invalid because another file changed.

## 3. Non-Goals

- Do not change default MCP behavior in the first auto-refresh slice. `WarnOnly` remains default unless a later
  product decision changes it.
- Do not use file watchers in the first auto-refresh slice. Request-time freshness checks already exist and are
  deterministic in tests.
- Do not use the CPG incremental path for MCP auto-refresh until the incremental correctness gate passes.
- Do not change navigation query semantics, evidence shaping, or canonical `Evidence` schema in Slice A.
- Do not make Tier-A oracle rebaselining part of these slices. Regressions and flip candidates go into PR notes.

## 4. Slice A: Auto-Full MCP Refresh On Drift

### 4.1 User-visible contract

Add an opt-in MCP refresh policy:

```rust
#[derive(Clone, Debug)]
pub enum RefreshPolicy {
    WarnOnly,
    AutoFull,
}
```

`WarnOnly` preserves current behavior:

- stale normal tools return the pre-refresh answer with `StaleIndex` metadata/warnings
- callers may manually invoke `refresh_index`

`AutoFull` changes normal known tool calls:

- if the freshness probe is clean, serve the tool normally
- if the freshness probe is stale, refresh the provider session before running the requested tool
- run the requested tool against the refreshed session
- include refresh metadata so the model can tell the answer came from a refreshed snapshot
- if refresh fails, keep the old session and fall back to current stale-warning behavior

### 4.2 Configuration shape

Extend `ServerConfig`:

```rust
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub repo_root: PathBuf,
    pub cache: CacheMode,
    pub refresh_policy: RefreshPolicy,
}
```

`canonical_config` must preserve the policy.

`src/bin/prism-mcp.rs` should expose a conservative flag, for example:

```text
--refresh-policy warn-only|auto-full
```

Default: `warn-only`.

Test helpers that construct `ServerConfig` can use a `ServerConfig::new(repo_root)` helper or explicit
`RefreshPolicy::WarnOnly` to avoid scattered defaults.

### 4.3 Runtime placement

Extend `SessionRuntime` with policy-aware refresh support:

```rust
trait SessionRuntime {
    fn session(&self) -> &NavigationSession;
    fn freshness(&self) -> Option<&FreshnessProbe>;
    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport>;
    fn refresh_policy(&self) -> RefreshPolicy;
    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary>;
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary>;
}

pub struct AutoRefreshSummary {
    pub generation: u64,
    pub stale_before_refresh: FreshnessReport,
    pub verification: RefreshVerification,
    pub indexed_files: usize,
    pub tracked_paths: usize,
}

pub enum RefreshVerification {
    Clean,
    Diverged(FreshnessReport),
}
```

`StaticRuntime` returns `RefreshPolicy::WarnOnly` because it cannot rebuild. Its `auto_refresh_index()` can return
an unavailable error and should be unreachable in normal dispatch because the policy is `WarnOnly`.

`SessionProvider` returns its configured policy. Manual `refresh_index` can keep its user-visible tool shape, but
once Slice A adds candidate verification it should route through the same build/verify/commit machinery or an
equivalent wrapper. Automatic refresh must not call today's `refresh()` and then verify afterward:
`SessionProvider::refresh` currently replaces the session and freshness baseline before it returns, which is too
late for the load-to-baseline race check below.

Slice A should introduce an atomic candidate refresh path, for example:

```text
build RefreshCandidate { state, loaded_snapshot_fingerprint }
verify candidate.loaded_snapshot_fingerprint against current disk
if verifier reports a retryable divergence and retry budget remains:
  build and verify one more candidate
if verifier returns Clean, or returns Diverged after retry budget is exhausted:
  commit candidate, replace freshness baseline, increment generation
if build fails or verifier returns an IO/internal error:
  drop candidate, keep old session/generation, report refresh failure
```

Only the commit step may replace the runtime session. The verifier's `Diverged` outcome is distinct from verifier
failure: a diverged candidate can be published as the latest known snapshot only with `raced_stale` metadata and a
stale warning, and only after the retry budget has been exhausted; a verifier error must fall back to the old
session because Prism cannot prove what the candidate represents.

"Unpublished" means the candidate is not installed as the active runtime session until verification allows a
commit. It does not require deferring content-addressed on-disk nav/CPG cache writes performed by
`NavigationIndex::build_cached_at`; those writes are keyed by the candidate's file hashes/topology and are benign.

A diverged commit must also set a sticky session-level known-stale report. This is required because the freshness
baseline produced by today's `FreshnessProbe::from_loaded_repo` stamps filesystem state after `load_repo`; a
known-diverged candidate can therefore have a baseline that looks clean on the next request even though its loaded
bytes are stale. The sticky report is cleared only by a later clean verified refresh. Until then:

- `known_stale_after_refresh()` returns the verifier's bounded `FreshnessReport`
- dispatch treats that report as stale even if `freshness.check()` is clean
- `AutoFull` attempts another refresh on the next normal tool call
- `WarnOnly`/fallback paths apply the stale warning from the sticky report

Manual `refresh_index` is in Slice A scope for this reason: it must migrate onto the verifying candidate path, or
an equivalent wrapper, so it cannot clear a sticky known-stale report unless the newly built snapshot is verified
clean.

The dispatch path for normal tools becomes:

```text
resolve tool
if tool is refresh_index:
  existing manual refresh path
else:
  original_cap = resolve_cap()
  initial_report = effective_stale_report(known_stale_after_refresh(), freshness.check())
  if initial_report.stale && policy == AutoFull:
    try runtime.auto_refresh_index()
    if refresh succeeds:
      verification = refresh verification outcome from the committed candidate
      post_report = effective_stale_report(known_stale_after_refresh(), freshness.check()) against the
        committed candidate baseline
      may_apply_stale = verification is Diverged || post_report.stale
      cap = original_cap - auto_refresh_reserve - freshness_reserve_if(may_apply_stale)
      run tool against refreshed session
      attach auto-refresh metadata
      if verification is Diverged || post_report.stale:
        also apply StaleIndex warning because the refreshed snapshot is not proven current
      return
    if refresh fails:
      cap = original_cap - auto_refresh_failure_reserve - freshness_reserve
      run existing stale-warning path against old session, with refresh-error metadata
  else:
    existing WarnOnly path
```

Important: refresh must happen before `ToolContext::new(runtime.session(), cap)` so the handler sees the new
session.

### 4.3.1 Response-cap reserve rule

Auto-refresh mutates a shaped tool result, just like stale-warning application does today. Therefore the reserve
must be decided before constructing `ToolContext`; otherwise auto-refresh metadata can push the final
`CallToolResult` over the MCP response cap.

Add a small dedicated reserve:

```rust
pub(crate) const AUTO_REFRESH_RESERVE_BYTES: usize = 2048;
```

The exact value should be proven by tests and can be adjusted, but the rule is fixed. The reserve calculation
must be paired with an accepted-cap floor invariant:

```text
reserved = 0
if auto-refresh metadata may be attached:
  reserved += AUTO_REFRESH_RESERVE_BYTES
if a StaleIndex warning may be attached after the tool result is shaped:
  reserved += FRESHNESS_RESERVE_BYTES
assert production original_cap >= reserved + MIN_MUTATING_TOOL_CAP_BYTES + JSON_RPC_ENVELOPE_RESERVE
tool_cap = original_cap - reserved
```

`saturating_sub` is not enough for production traffic. At spec-review time, source had
`FRESHNESS_RESERVE_BYTES = 4096` and a `MAX_RESULT_CHARS_FLOOR` of `4000`; adding auto-refresh reserve on top of
freshness reserve would otherwise make the effective tool cap zero at the minimum accepted cap. Slice A must
settle this before adding AutoFull by either:

- raising the accepted production floor so
  `MAX_RESULT_CHARS_FLOOR >= FRESHNESS_RESERVE_BYTES + AUTO_REFRESH_RESERVE_BYTES +
  MIN_MUTATING_TOOL_CAP_BYTES + JSON_RPC_ENVELOPE_RESERVE`, with a test/const assertion documenting the
  relationship; or
- introducing a bounded degraded stale result that deliberately omits the normal tool payload but still carries
  the required stale/auto-refresh metadata under the old floor.

The preferred implementation is the first option: raise the production floor and keep normal tool shaping for all
accepted production caps. Sub-floor caps used only in tests may still exercise the existing terminal `isError`
path, but accepted production caps must leave enough room for a minimal shaped tool result plus any post-shape
metadata that the dispatch path has promised to attach.

Slice A implements this by raising `MAX_RESULT_CHARS_FLOOR` to `12000` and asserting the relationship against
`FRESHNESS_RESERVE_BYTES`, `AUTO_REFRESH_RESERVE_BYTES`, `MIN_MUTATING_TOOL_CAP_BYTES`, and
`transport::ENVELOPE_RESERVE`. The minimum accepted cap is also exercised through deterministic transport unit
tests that inject the cap directly, avoiding global environment mutation in parallel tests.

Compatibility note: `resolve_cap_from` currently snaps any env value below `MAX_RESULT_CHARS_FLOOR` to the default
`MAX_RESULT_CHARS` value, not to the floor. If Slice A raises the floor, clients setting
`PRISM_MCP_MAX_RESULT_CHARS` between the old and new floor will receive the default larger cap. Call this out in
the PR/release notes, or choose the bounded degraded stale-result option if preserving those small caps matters
more than keeping one normal shaping path.

Name mapping: `JSON_RPC_ENVELOPE_RESERVE` in the invariant refers to today's `transport::ENVELOPE_RESERVE`
constant, currently `512` bytes. The implementation should use the real constant name or a shared helper rather
than adding a second divergent value.

`MIN_MUTATING_TOOL_CAP_BYTES` must be assigned during implementation from the smallest non-error shaped result
that the mutating-warning path is willing to return, then locked to the raised floor with a const/test assertion.

Cases:

- `WarnOnly` stale path keeps today's `FRESHNESS_RESERVE_BYTES`.
- `AutoFull` refresh succeeds and `post_report` is clean: reserve only `AUTO_REFRESH_RESERVE_BYTES`.
- `AutoFull` refresh succeeds and `post_report` is stale: reserve
  `AUTO_REFRESH_RESERVE_BYTES + FRESHNESS_RESERVE_BYTES`.
- `AutoFull` refresh fails and the old stale snapshot is used: reserve
  `AUTO_REFRESH_RESERVE_BYTES + FRESHNESS_RESERVE_BYTES`, because the response may include both refresh-failure
  metadata and stale-warning surfaces.

All auto-refresh metadata must be bounded to fit inside `AUTO_REFRESH_RESERVE_BYTES`. Low-cap tests must assert
the final serialized JSON-RPC response, including envelope, remains within the same cap enforced by
`shape_result`/transport today. The low-cap matrix must include the minimum accepted production cap for:

- clean auto-refresh success
- raced-stale auto-refresh success
- refresh failure with old-session stale-warning fallback
- warn-only stale warning, because the same floor/reserve invariant protects today's behavior too

Auto-refresh metadata in the clean-success path is `_meta`-only. It must not mutate `structuredContent` or
`content_text`; this keeps `AUTO_REFRESH_RESERVE_BYTES` small and preserves the model-visible navigation answer.
When attaching auto-refresh metadata, reset `_meta["anthropic/maxResultSizeChars"]` to `original_cap`, matching
the existing stale-warning path. The reduced `tool_cap` is an internal shaping budget, not the caller's requested
cap.

### 4.3.2 Loaded-snapshot verification

`FreshnessProbe::check` after refresh is not enough to prove the newly loaded session is current. The current
refresh path loads repository bytes first and constructs the freshness baseline afterward. If a file changes
after `load_repo` reads it but before `FreshnessProbe::from_loaded_repo` stamps current filesystem metadata, the
new baseline can look clean even though the loaded session is already stale.

Slice A must add a verification step before claiming `AutoFull` success as clean:

```text
load/build refreshed SessionState candidate without publishing it
verify candidate's loaded snapshot against current disk
if verification diverges and retry budget remains:
  load/build a second candidate without publishing it
  verify the second candidate
commit the latest candidate only after verifier returns Clean, or returns Diverged after retry budget is exhausted
```

The verification must be unbounded and semantic, not based on the display-capped `FreshnessReport.changed_paths`.
It may rerun `load_repo` or use a lighter equivalent, but it must compare at least:

- loaded source file set
- loaded source file hashes
- loaded manifest hash set
- loaded manifest hashes
- topology key inputs affected by manifests/workspace membership

Outcomes:

- verification clean: run the tool without stale warning, unless a later `FreshnessProbe::check` reports drift
- verification diverged after retry: run the tool against the latest built state, attach
  `prism/auto_refresh = "raced_stale"`, apply a `StaleIndex` warning built from the verification report, and
  persist that report as session-level known-stale until a clean verified refresh
- verifier error, or build failure before a candidate can be safely committed: treat as refresh failure and use
  the old-session stale-warning fallback

This keeps the contract honest: auto-refresh may still race active edits, but it must not return a
fresh-looking result unless the loaded snapshot has been verified against current disk at least once after load.

### 4.4 Metadata

Manual `refresh_index` already returns `prism/refresh_generation` and `prism/refresh_status`. Once it migrates to
the verified candidate path, `prism/refresh_status` may be `"raced_stale"` as well as `"refreshed"`; this is an
honesty improvement rather than a resolver behavior change.

Auto-refresh should add bounded `_meta` keys to the final tool result:

```text
prism/auto_refresh = "refreshed" | "failed" | "raced_stale"
prism/refresh_generation = <u64>       # when refresh succeeded
prism/indexed_files = <usize>          # when refresh succeeded
prism/tracked_paths = <usize>          # when refresh succeeded
prism/stale_index_total_before_refresh = <usize>
prism/stale_index_paths_before_refresh = [<bounded paths>]
```

If refresh succeeds but `post_report.stale` is true, the result should get both:

- auto-refresh metadata showing a refresh happened
- normal `StaleIndex` warning for the new drift after the refreshed baseline

Do not include unbounded error strings in metadata. Use the existing `clamp_user_text` pattern for refresh
failure text.

Metadata attachment rule:

- successful tool result after clean auto-refresh: attach `_meta` auto-refresh keys only; leave
  `structuredContent` and `content_text` byte-identical to a normal fresh result, aside from `_meta`
- successful tool result after raced-stale auto-refresh: attach auto-refresh keys and the normal stale-warning
  surfaces
- requested tool returns `isError` after refresh succeeded: return the tool error shape unchanged and do not
  attach auto-refresh metadata or stale warnings in Slice A
- refresh failed before the requested tool runs: use the explicit refresh-failure fallback below

### 4.5 Failure behavior

On refresh failure:

- do not replace the old session
- run the originally requested tool against the old session
- apply the stale warning to successful structured results
- include a bounded `prism/auto_refresh = "failed"` and a bounded refresh error string

If the tool itself errors, keep the current rule: stale warnings are not applied to tool errors. Refresh-failure
metadata can still be included only if the error-result path is deliberately designed and tested; otherwise omit
it from tool errors to preserve today's error shape.

### 4.6 Tests

Add MCP transport/session tests:

- `warn_only_preserves_stale_warning_without_refresh`
- `auto_full_refresh_rebuilds_before_tool_and_clears_stale_warning`
- `auto_full_refresh_adds_generation_metadata`
- `auto_full_refresh_uses_new_repo_map_after_file_addition`
- `auto_full_refresh_uses_new_callers_after_edit`
- `auto_full_refresh_failure_keeps_old_session_and_stale_warning`
- `auto_full_refresh_race_reports_stale_after_refresh`
- `auto_full_refresh_load_to_baseline_race_reports_stale_or_retries`
- `auto_full_diverged_commit_remains_stale_on_next_request`
- `auto_full_clean_refresh_clears_known_stale_after_refresh`
- `manual_refresh_does_not_clear_known_stale_without_clean_verification`
- `session_runtime_exposes_auto_refresh_without_downcast`
- `auto_full_refresh_verifier_error_keeps_old_session_unpublished`
- `auto_full_refresh_clean_success_keeps_content_text_byte_identical`
- `auto_full_refresh_clean_success_resets_max_result_size_meta_to_original_cap`
- `auto_full_refresh_then_tool_error_preserves_tool_error_shape`
- `static_serve_session_ignores_auto_full_or_reports_unavailable_as_warn_only`
- `accepted_cap_floor_exceeds_max_post_shape_reserve_plus_min_tool_cap`
- `auto_full_raced_stale_under_floor_cap_preserves_warning_metadata`
- `auto_full_refresh_failure_under_floor_cap_preserves_warning_metadata`
- `warn_only_stale_under_floor_cap_preserves_warning_metadata`

For failure injection, avoid making filesystem permissions assumptions. Prefer a test-only `SessionRuntime`
that returns a controlled refresh error. For load-to-baseline race tests, prefer a test-only verifier hook over
filesystem timing assumptions.

### 4.7 Validation

Targeted:

```bash
cargo fmt --check
cargo test --features mcp --lib mcp::session
cargo test --features mcp --lib mcp::transport
cargo test --features mcp --test mcp
cargo test --test navigation
git diff --check main..<branch>
```

Tier-A is required by the repo instruction if this touches `src/navigation/*`; otherwise it is optional but
reasonable before PR because MCP results depend on navigation:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

## 5. Slice B: Incremental CPG Correctness Gate

### 5.1 Correctness invariant

For a given final `files` map and optional `ScopeGraphBuildInputs`, these must be behavior-equivalent:

1. full build: `CodePropertyGraph::build_enriched_with_scope_graph_inputs(files, type_db, inputs)`
2. incremental build: cached old CG/DFG plus `changed_files` through
   `CodePropertyGraph::build_incremental_with_scope_graph_inputs`

The parity oracle should compare normalized behavior, not raw node indices:

- call graph functions by `(file, name, start_line, end_line)`
- call sites by every resolution-affecting `CallSite` field, including:
  - caller fid
  - `callee_name`
  - `line`
  - `kind`
  - `start_byte` / `end_byte`
  - `qualifier`
  - `receiver_type`
  - `receiver_recovery`
  - `receiver_materialized`
  - `arg_count`
  - `arg_spread`
  - `receiver_outcome`
- resolved call behavior for every call site, not only selected seeds:
  - resolved target set
  - dropped/filtered target set if exposed by the resolver result
  - confidence/resolution kind where available
  - over-approximation or ambiguity markers where available
- callers/callees query outputs for selected seeds as an end-to-end query check
- CPG call edges as `(caller fid, callee fid)` plus callsite line when available
- relevant algorithm result counts only as secondary smoke coverage

Do not omit a `CallSite` field merely because it is excluded from the `Ord`/`cmp_key` implementation. Several
fields are resolver inputs or telemetry that affect precision, recall, interface-dispatch output, or later
diagnostics even when they are not part of sorted identity.

The oracle deliberately focuses on CG/DFG/CPG behavior. `TypeRegistry` and `live_types` are rebuilt from the
same final `files` by `CpgContext::build_with_cached_cpg`, so they do not need a separate incremental parity
oracle unless a future change starts caching or incrementally updating them.

### 5.2 Refactor Phase-3 indirect resolution into an idempotent recompute

Move the inline full-build Phase-3 indirect logic from `CallGraph::build` into a reusable method. Suggested
shape:

```rust
impl CallGraph {
    pub(crate) fn recompute_indirect_calls(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_indirect_calls();
        let sites = self.compute_indirect_call_sites(files);
        self.apply_indirect_call_sites(sites);
    }
}
```

The method must be idempotent:

- calling it twice produces the same graph as calling it once
- calling it after a direct-subset merge produces the same indirect sites as full build
- calling it after target removal does not retain old synthetic sites

### 5.3 Marking or clearing synthetic indirect call sites

The current `CallSite` has `kind: CallKind::Call | MacroInvocation`; synthetic indirect sites currently look
like ordinary calls with `kind = Call`.

To make recomputation safe, the implementation needs a way to remove old derived indirect sites before adding
new ones.

Use an additive provenance field:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CallSiteOrigin {
    #[default]
    Source,
    IndirectResolution,
}
```

```rust
pub struct CallSite {
    // existing fields...
    #[serde(default)]
    pub origin: CallSiteOrigin,
}
```

Then `clear_indirect_calls` can remove synthetic sites from both `calls` and `callers` without touching direct
source call sites. Keep `origin` excluded from `cmp_key`/`Ord`, like receiver metadata. Do not add
`CallKind::IndirectResolved`: `CallKind` participates in sorted identity and is matched across resolver,
navigation, arity, and CPG assembly code, so overloading it as provenance would create unnecessary blast radius.

This changes serialized `CallGraph`, so bump `CACHE_VERSION` and update the cache-version test. Exact cache hits
from old binaries must miss.

Add a direct+synthetic collision test: when a source direct call and an indirect synthetic site share the same
logical `cmp_key`, `clear_indirect_calls` removes only `origin = IndirectResolution` and retains the source call.
Because `calls` is a `BTreeSet<CallSite>` and `origin` remains excluded from `Ord`, a source and synthetic site
with the same `cmp_key` cannot coexist there; full build inserts source sites before synthetic sites, so the
source wins and the synthetic insert is a no-op. The collision test should therefore construct the duplicated
state against the per-caller `callers` collection, and idempotency in `calls` should be asserted by the
source-before-synthetic insertion order.

**Fallback: recompute all `calls`/`callers` from source**

This avoids a serialized shape change but is less incremental: rebuild the full direct `CallGraph` state for all
files, not only the `calls` and `callers` maps, then rerun indirect resolution. Rebuilding only `calls`/`callers`
would risk stale resolver inputs in functions, methods, imports, receiver metadata, arity metadata, interface
tables, and other derived indexes. This fallback may still be acceptable for correctness, but it weakens the
value of partial incremental rebuild. If chosen, document the performance tradeoff and still add parity tests.

The preferred option is better because it preserves direct-subset rebuild economics and gives future diagnostics
visibility into derived indirect edges.

### 5.4 Ordering and byte stability

Preserve deterministic ordering:

- collect synthetic sites into `Vec<(FunctionId, CallSite)>`
- sort/dedup by a stable key before applying if any source iteration order is not already BTree-based
- update `calls` first, then `callers`, mirroring the existing full-build application order

`CallSiteOrigin::IndirectResolution` is provenance only. Navigation filtering, callers/callees, and CPG assembly
must continue treating these sites as call edges.

### 5.5 Incremental build placement

After merging fresh direct CG into cached CG, run all whole-graph recomputes together and match full-build
ordering:

```rust
cached_cg.merge(fresh_cg);
cached_dfg.merge(fresh_dfg);

cached_cg.recompute_indirect_calls(files);
cached_cg.rebuild_scope_graph(files, scope_inputs);
cached_cg.apply_go_embedding_promotion(files);
cached_cg.apply_go_interface_dispatch(files);
```

Rationale: full `CallGraph::build` resolves C/C++ indirect calls, then builds the scope graph / receiver state,
then applies Go embedding and interface dispatch. The incremental path should follow the same ordering unless a
test proves a different order is behavior-equivalent. The indirect recompute is whole-graph and can be costly on
C/C++ repositories because it scans all call sites and callback assignment patterns; this is acceptable for Slice
B correctness and is one reason Slice C remains gated.

The parity oracle must pin incremental output to an independently built v2 full graph using the canonical full
build order. Do not implement the oracle in a way that shares the reordered incremental helper on both sides and
therefore bakes the new ordering into the expected result. The mixed-language fixture in Section 5.6 should
exercise this ordering explicitly. Because this changes the already-shipped CLI incremental recompute order for
Go embedding/interface dispatch relative to scope-graph rebuilding, the Slice B PR should call out possible
Go-repo output changes and include the parity evidence rather than presenting the reorder as a no-op refactor.

### 5.6 Parity fixture matrix

Add a helper that builds v1 full, then v2 full and v2 incremental from v1 cached parts:

```rust
fn assert_incremental_matches_full(
    v1: BTreeMap<String, ParsedFile>,
    v2: BTreeMap<String, ParsedFile>,
    changed: BTreeSet<String>,
    inputs: Option<&ScopeGraphBuildInputs>,
)
```

The helper should compare normalized dumps and print concise diffs on failure.

Required fixtures:

- C local function pointer assignment:
  - changed caller introduces/removes `fp = target`
  - changed target file renames/removes target while caller is unchanged
- C array dispatch table:
  - changed same-file table changes entries while dispatch caller is unchanged, if represented in the same file
  - changed caller changes `handlers[i]` call
- C struct field callback:
  - changed assignment file changes `.callback = target`
  - changed caller file changes `obj->callback()`
- C parameter-passed callback:
  - changed outer caller passes a different function
  - changed callback target file removes or renames a target
- Existing non-C recompute protections:
  - Rust receiver outcome rematerialization fixture remains green
  - Go embedding stale alias fixture remains green
  - Go interface dispatch stale table fixture if available, or add one
- Multi-language control:
  - Python/JS/Go direct-call partial-hit parity remains green
- Mixed whole-graph ordering control:
  - one synthetic repository containing Rust receiver recovery, Go embedding/interface dispatch, and C indirect
    callbacks in the same `files` map, to catch accidental changes in whole-graph recompute ordering

These fixtures should include at least one case where `changed_files` does not include the indirect-calling
function, because that is the stale-derived-edge hazard that direct changed-file replacement will not catch.
For current array-dispatch semantics, unchanged-caller stale-edge coverage should come from same-file table,
struct-field, and parameter-passed callback fixtures; cross-file array-table changes are not currently resolved
by full build and should not be counted unless full-build semantics are extended first.

### 5.7 Tests to avoid

Do not rely only on:

- node count equality
- edge count equality
- same parsed files with only hash changes
- algorithm block count equality

Those tests are useful smoke coverage but can miss wrong or stale call targets.

### 5.8 Validation

Because this touches `src/call_graph.rs`, `src/cpg/build.rs`, and cache serialization:

```bash
cargo fmt --check
cargo test --test integration call_graph
cargo test --test integration resolution
cargo test --test ast cpg_cache
cargo test --test navigation
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
git diff --check main..<branch>
```

If `CallSite` serialization changes, also update:

- `src/cpg_cache.rs` `CACHE_VERSION`
- its version test
- any bincode roundtrip tests for `CallSite`/`CallGraph`

## 6. Slice C: Future Auto-Incremental MCP Refresh

### 6.1 Gate

Do not implement `AutoIncremental` until Slice B has merged and has:

- full-vs-incremental parity tests for indirect C/C++ cases
- cache-version handling for any serialized shape changes
- Tier-A matrix and quick validation with no unexamined regressions

### 6.2 Policy shape

After Slice B:

```rust
pub enum RefreshPolicy {
    WarnOnly,
    AutoFull,
    AutoIncremental,
}
```

`AutoIncremental` should still be opt-in at first.

### 6.3 Refresh strategy

Add an internal refresh strategy:

```rust
enum RefreshStrategy {
    Full,
    Incremental,
}
```

`SessionProvider::refresh_with_strategy(strategy)` returns an extended summary:

```rust
pub struct RefreshSummary {
    pub status: &'static str,
    pub strategy: &'static str, // "full" | "incremental"
    pub generation: u64,
    pub indexed_files: usize,
    pub tracked_paths: usize,
    pub stale_before_refresh: bool,
    pub stale_index_total_before_refresh: usize,
    pub stale_index_paths_before_refresh: Vec<String>,
    pub fallback_reason: Option<String>,
}
```

### 6.4 When incremental is allowed

For MCP navigation, use incremental only when all of these hold:

- the file set is unchanged, matching the existing CPG cache partial-hit precondition
- topology key is unchanged
- type database availability is unchanged
- grammar fingerprint, skip policy, git SHA, and cache version checks pass
- changed paths are source files already in the tracked indexed set
- no manifest path changed
- no tracked directory-only addition/deletion/rename signal is present

Otherwise fall back to full rebuild.

Rationale: directory and manifest changes can alter the file universe, module topology, or workspace member
set. Those are better served by a full `load_repo` plus whole-index rebuild.

Hard invariant: `AutoIncremental` must not use `FreshnessReport.changed_paths` as the changed-file set.
`FreshnessReport.changed_paths` is deliberately capped for display. It can trigger a refresh decision, but it is
not complete enough for rebuild correctness. The implementation must compute an unbounded semantic diff from old
and fresh loader/cache state, or fall back to full refresh.

### 6.5 Session state needed for incremental

Current `SessionProvider` does not retain enough data to do incremental refresh directly; it owns only the
current `NavigationSession` and freshness probe.

The implementation can choose either:

1. reuse the existing on-disk nav cache and rely on `NavigationIndex::build_cached_at`; or
2. retain enough in-memory state to call incremental directly.

Option 1 is simpler but currently exact-hit-only for nav partial hits. To support `AutoIncremental`, it would
need a deliberate `NavigationIndex::build_incremental_cached_at` path, separate from exact-hit-only default nav
cache behavior.

Option 2 requires storing or reconstructing:

- previous CPG call graph and DFG
- current file hashes and topology key
- type DB availability
- unbounded changed file set computed from old and fresh `LoadedRepo.file_hashes`
- unbounded manifest/topology diff computed from old and fresh `LoadedRepo.manifest_hashes` and topology keys
- fresh `LoadedRepo`

Recommendation: use an explicit navigation incremental builder rather than changing `build_cached_at` semantics
globally. That keeps normal CLI/MCP nav caching conservative while allowing `AutoIncremental` to be reviewed as
an explicit policy.

### 6.6 Fallback behavior

`AutoIncremental` should never fail the user's navigation call solely because incremental is not applicable.

Decision table:

| Situation | Behavior |
|---|---|
| Incremental preconditions pass | incremental refresh, then run tool |
| Preconditions fail safely | full refresh, then run tool, metadata `fallback_reason` |
| Incremental preparation/precondition gathering errors | full refresh retry; if full succeeds, run tool with fallback metadata |
| Full retry also errors | old-session stale-warning fallback, same as `AutoFull` failure |
| Post-refresh probe is stale | run tool, attach `StaleIndex` warning for raced changes |

Today `CodePropertyGraph::build_incremental_with_scope_graph_inputs` is infallible once inputs are available.
The fallback row is for future fallible preparation around loading, hash/topology comparison, cache
deserialization, or an explicitly fallible incremental navigation builder.

### 6.7 Tests

In addition to Slice A tests:

- `auto_incremental_uses_incremental_when_only_indexed_file_changes`
- `auto_incremental_falls_back_to_full_on_manifest_change`
- `auto_incremental_falls_back_to_full_on_file_addition`
- `auto_incremental_falls_back_to_full_on_file_deletion`
- `auto_incremental_falls_back_to_full_on_topology_change`
- `auto_incremental_retries_full_after_incremental_error`
- `auto_incremental_metadata_reports_strategy_and_fallback`

Add at least one test with a C indirect-call fixture from Slice B so MCP auto-incremental proves the same
callers/callees result as full refresh after an edit.

## 7. Cross-Slice Ordering

Recommended implementation order:

1. Slice A: `RefreshPolicy::AutoFull` only. Low semantic risk and immediate LLM value.
2. Slice B: incremental parity and indirect recompute. Higher risk, requires Tier-A.
3. Slice C: `AutoIncremental`. Requires Slice B as a hard gate.

Do not combine Slice A and Slice B in one PR. They touch overlapping correctness concerns but different risk
surfaces:

- Slice A: `src/mcp/session.rs`, `src/mcp/transport.rs`, `src/bin/prism-mcp.rs`, MCP tests
- Slice B: `src/call_graph.rs`, `src/cpg/build.rs`, `src/cpg_cache.rs`, CPG/cache/integration tests
- Slice C: both surfaces, after Slice B

## 8. Reviewer Questions

1. Should `AutoFull` remain opt-in for one release, or should MCP default to auto-refresh after stale detection?
   This spec recommends opt-in first.
2. Is the additive `CallSiteOrigin` field sufficient for clearing synthetic indirect sites without changing
   call-kind semantics?
3. Is there any reason incremental whole-graph recomputes should not match full-build order
   `indirect -> scope/receiver -> Go embedding -> Go interface`?
4. Should `NavigationIndex::build_cached_at` remain exact-hit-only permanently, with a separate explicit
   incremental nav builder for `AutoIncremental`? This spec recommends yes.
5. Should auto-refresh metadata be applied to tool error results after a successful refresh? This spec
   recommends no for Slice A, to preserve current error shapes.

## 9. Review Checklist

Reviewers should look for:

- any path where `AutoFull` can return a stale answer without either refreshing or warning
- any path where refresh failure can replace the session with partial state
- response-cap regressions from additional auto-refresh metadata
- stale race handling after refresh
- hidden coupling between MCP nav cache and CPG incremental cache
- missing stale-derived-edge cases in the incremental parity matrix
- cache-version or bincode compatibility issues if `CallSite` shape changes
- whether the future `AutoIncremental` preconditions are strict enough to avoid topology/file-universe hazards
