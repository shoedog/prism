# MCP Staleness Honesty and Freshness Probe Spec

Status: Historical implemented spec; updated 2026-07-01
Date: 2026-06-23
Branch: `mcp-staleness-freshness`, based on `origin/main`

> Current status: MCP freshness warnings and manual refresh support are implemented in the current codebase.
> This file is retained for the original contract and review rationale; use the sibling auto-refresh specs for
> later refresh-policy design history.

## Goal

Make Prism MCP navigation and reasoning answers honest after files change during a long-lived MCP server
session.

Today `prism-mcp` builds a `NavigationSession` once at startup and serves that frozen snapshot for the process
lifetime. That is fast, but an agent can edit a file and then receive pre-edit callers/callees/module evidence
with no warning. This slice adds a cheap per-tool-call freshness probe and visible stale-index signaling. It
does not rebuild the index.

## Non-Goals

- Do not auto-refresh or rebuild the navigation index.
- Do not change resolver behavior, CPG construction, cache behavior, or navigation query semantics.
- Do not change CLI `prism nav` behavior; CLI remains fresh because it builds/loads a session per invocation.
- Do not walk the full repository on every tool call.
- Do not treat unsupported/skipped file changes as precise semantic invalidations. This slice is an honesty
  layer, not a new file-universe tracker.

## Current Code Shape

- `src/mcp/session.rs`
  - `SessionProvider::bootstrap` canonicalizes the repo root, runs `load_repo`, builds a `NavigationIndex`,
    and stores only `NavigationSession`.
- `src/mcp/transport.rs`
  - `serve_stdio` calls `serve_session(p.session(), registry, transport)`.
  - `serve_session` dispatches `tools/call` through `call_tool_response`.
  - Tool handlers receive only `&NavigationSession`.
- `src/mcp/registry.rs`
  - `ToolHandler = dyn Fn(&NavigationSession, &serde_json::Value) -> McpToolResult + Send + Sync`.
- `src/mcp/output.rs`
  - `McpToolResult` owns `content_text`, canonical `structured`, `is_error`, and `_meta`.
- `src/navigation/types.rs`
  - `WarningKind` currently has no stale-index variant.

The implementation keeps navigation/reasoning logic on `&NavigationSession`, but the MCP tool registry needs a
small MCP-only call context so stale results can reserve response bytes before shaping.

## A2A Round-1 Findings Folded

Codex task `c1ae638d-4487-4f2c-a2c6-622a1a49ef31` and Claude task
`84101dff-b393-47d2-ad42-5bb2f998260e` both returned `NEEDS CHANGES`. The valid findings are folded here:

- freshness signaling must be result-size-budget-aware; unbudgeted mutation after `shape_result` can exceed the
  MCP wire cap
- stale content synchronization must define exact behavior for canonical JSON, `agent_json`, and
  `agent_markdown`, including missing warning sections/arrays and agent summary counts
- directory tracking must not overclaim complete addition detection
- the probe must snapshot immediately after `load_repo` and before index construction
- per-call stat cost and mtime/length false negatives must be stated
- stale application should be gated to non-error tool results with structured content
- tests must cover cap safety, `agent_json`, `taint_reaches`, and no byte-parity expectation for stale JSON text

## A2A Re-Review Findings Folded

Codex task `540bb583-5e11-4ab6-9c41-546c1b033613` and Claude task
`f32f2dd8-e61d-4a86-bfeb-37a09c94c7e4` both returned `NEEDS CHANGES (close)`. The valid findings are folded here:

- `agent_json`/`agent_markdown` content synchronization must tolerate the existing clipped fallback where
  `content_text` is a plain bounded notice even though `_meta["prism/content_text_format"]` names an agent format
- freshness reserve sizing must cover the sum of all three freshness surfaces: `_meta`, `structuredContent`, and
  any edited `content_text`
- displayed stale paths are bounded by a total displayed-path byte budget, not by a per-path budget multiplied
  without proof
- directory-addition coverage wording must match the tracked set
- `agent_json.summary.warnings` must be set from the warnings array length for idempotency
- stale `content_text` may exceed an agent request's `max_view_bytes`; the MCP wire cap remains the hard bound

Final readiness review returned `READY_TO_IMPLEMENT` from Codex task
`8ece822f-a68e-49e9-8f59-4a99a395d5fc` and Claude task
`e20b48e7-51fb-4d56-af39-936069cef910`. Claude noted three non-blocking clarifications, folded below:

- provide a `ToolContext::for_test(session)` helper so existing MCP tool unit tests do not each construct the
  cap-aware context by hand
- directory-addition tests are mtime-bound on some filesystems, so they should set or advance directory mtime
  deterministically rather than relying on sleeps
- construct the stale warning through the typed `Warning`/`WarningKind::StaleIndex` shape, then serialize it for
  raw JSON insertion, so `structuredContent` remains deserializable as `Evidence`

## User-Visible Contract

Tool descriptions should say:

> Results reflect the repository snapshot loaded when `prism-mcp` started. If indexed files change during the
> server session, Prism marks tool results with stale-index metadata and warnings; restart/re-add the MCP server
> or use the CLI for a fresh snapshot.

On a fresh index, tool output is unchanged except for the updated tool descriptions.

On detected drift, each successful known `tools/call` result with structured content gets:

- `_meta["prism/index_freshness"] = "stale"`
- `_meta["prism/stale_index_total"] = <number>`
- `_meta["prism/stale_index_paths"] = [<bounded sorted paths>]`
- a model-visible `StaleIndex` warning in `structuredContent.warnings`
- a matching `content_text` freshness signal, with exact behavior defined below

The warning message should be deterministic and bounded, for example:

```text
MCP index may be stale; 3 tracked paths changed since server startup: src/main.rs, Cargo.toml, src/lib.rs.
Restart/re-add the MCP server or use CLI nav for a fresh snapshot.
```

If more paths changed than the display cap, include the omitted count.

## Freshness Model

Add `src/mcp/freshness.rs`.

Suggested types:

```rust
pub struct FreshnessProbe {
    root: PathBuf,
    tracked: BTreeMap<String, PathStamp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshnessReport {
    pub stale: bool,
    pub total_changed: usize,
    pub changed_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathStamp {
    kind: PathKind,
    len: Option<u64>,
    modified: Option<SystemTime>,
}

enum PathKind {
    File,
    Directory,
    Missing,
    Other,
    Unreadable,
}
```

Tracked paths at bootstrap:

- every parsed/indexed source file in `LoadedRepo.files`
- every tracked manifest path in `LoadedRepo.manifest_hashes`
- the repo root directory
- ancestor directories of the tracked files/manifests, relative to the repo root

Why include directories: file additions, deletions, and renames can invalidate repo-map/call evidence even when
the added file was not in the original parsed file set. Tracking all ancestors gives a cheap coarse signal
without a per-call repository walk. Directory changes are a conservative "may be stale" signal; source-file and
manifest changes are precise stale signals.

Coverage limitation: this does not detect every possible new supported file. In particular, a new supported
file added inside an existing directory that had no indexed source file or manifest descendant at startup may
not change any tracked directory. `LoadedRepo.skipped` paths are not part of the tracked set in this slice.
Full file-universe drift detection needs a loader-level visited-directory set or an ignore-aware repository
walk and is deferred to the indexing-policy slice.

Comparison rule:

- stale if a tracked path's kind, length, or modification timestamp differs
- stale if stat now fails for a path that was readable at bootstrap
- stale if a previously missing/unreadable path becomes readable
- output paths are repo-relative, sorted, and capped

This intentionally uses metadata, not content hashing. Hashing all indexed files per tool call is too expensive
for large repos and unnecessary for an honesty signal.

Cost: the probe is `O(tracked_paths)` stat calls per known tool call. For typical MCP-scale repos this should be
low milliseconds; for multi-thousand-file repos it can be noticeable. Do not cap the tracked set in this slice,
because dropping paths would make stale detection arbitrary. Debouncing or time-based reuse can be a later
optimization if profiling shows this dominates tool latency.

Known false negative: a same-length edit inside the filesystem's modification-time granularity window can evade
metadata comparison. The intended fix would be content hashing or editor/file-notification integration, not this
cheap honesty slice.

## MCP Runtime Placement

Extend `SessionProvider`:

```rust
pub struct SessionProvider {
    session: NavigationSession,
    freshness: FreshnessProbe,
}
```

`SessionProvider::bootstrap` builds `FreshnessProbe::from_loaded_repo(&repo)` immediately after `load_repo` and
before index construction. The ordering must be:

1. canonicalize repo root
2. `load_repo`
3. build `FreshnessProbe` from the loaded repo immediately
4. build/load `NavigationIndex`
5. publish `SessionProvider`

This avoids accepting edits made during a slow index/cache build as the "fresh" baseline for an older loaded
repo snapshot. Startup is still not atomic: an edit racing exactly with `load_repo` can still produce whatever
snapshot the loader saw. This slice detects drift after the loaded snapshot is established.

Transport should keep the public `serve_session` helper available, but the registry handler needs a cap-aware
context so stale calls can reserve response bytes before shaping. Introduce an MCP-only context:

```rust
pub struct ToolContext<'a> {
    pub session: &'a NavigationSession,
    pub cap: usize,
}
```

Change `ToolHandler` from `Fn(&NavigationSession, &Value)` to `Fn(&ToolContext<'_>, &Value)`. Handler logic
still uses `ctx.session` for all navigation/reasoning work, and uses `ctx.cap` instead of calling
`resolve_cap()` internally. This is MCP-only signature churn and avoids resolver/query changes.

Add `ToolContext::for_test(session)` returning `ToolContext { session, cap: resolve_cap() }` so existing tool
unit tests can keep their intent clear while adapting to the new handler signature.

Add a freshness-aware transport path:

```rust
pub fn serve_stdio(p: &SessionProvider, r: &ToolRegistry) -> anyhow::Result<()> {
    serve_session_with_freshness(p.session(), Some(p.freshness()), r, &mut transport)
}

pub fn serve_session(
    session: &NavigationSession,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
) -> anyhow::Result<()> {
    serve_session_with_freshness(session, None, registry, transport)
}
```

`serve_session` remains available for existing unit tests and any in-process callers that do not want freshness.

`handle_message` threads `Option<&FreshnessProbe>` into `call_tool_response`.

`call_tool_response` should:

1. validate protocol params as it does today
2. resolve the known tool
3. if a `FreshnessProbe` is present, check freshness before dispatch
4. choose the shaping cap:
   - fresh or no probe: `cap = resolve_cap()`
   - stale:
     - `original_cap = resolve_cap()`
     - `cap = original_cap.saturating_sub(FRESHNESS_RESERVE_BYTES)`
5. dispatch the tool handler with `ToolContext { session, cap }`
6. if stale and `!result.is_error && result.structured.is_some()`, call
   `apply_freshness_report(&mut result, &report, original_cap)`
7. assert/test that final serialized payload still fits the original transport budget
8. wrap the result with `success_response`

Use a fixed reserve, for example:

```rust
const FRESHNESS_RESERVE_BYTES: usize = 4096;
const MAX_STALE_PATHS: usize = 5;
const MAX_STALE_PATH_BYTES: usize = 360;
```

`MAX_STALE_PATH_BYTES` is the total displayed byte budget across all stale paths after display sanitization, not
a per-path budget. The formatter should add paths in sorted order until either `MAX_STALE_PATHS` or the total
displayed-path byte budget is reached, then report the omitted count. The stale warning message must be bounded
from those already-bounded display paths.

The reserve must cover the full growth from all three freshness surfaces:

- `_meta["prism/index_freshness"]`, `_meta["prism/stale_index_total"]`, and
  `_meta["prism/stale_index_paths"]`
- the `StaleIndex` object inserted into `structuredContent.warnings`
- any stale text inserted into or appended to `content_text`

Pin this with a unit test that applies a maximum stale report to canonical JSON, `agent_json`, and
`agent_markdown` results, then proves the serialized result growth is `<= FRESHNESS_RESERVE_BYTES`. The final
stale result must also be checked against `payload_budget(original_cap)` in tests. If a future change makes the
stale payload exceed the reserve, tests should fail before a cap regression ships.

When `original_cap` is near the floor, subtracting the reserve can produce a very small shaping cap. If the tool
shaper returns a terminal `is_error` over-cap result in that case, do not attach stale metadata; stale signaling
is only for successful evidence results in this slice. This is acceptable because the hard contract is that
successful stale responses fit the original MCP wire budget.

Because `shape_result` records the cap it received in `_meta["anthropic/maxResultSizeChars"]`,
`apply_freshness_report` should restore that metadata value to `original_cap` after applying freshness. The
reduced cap is an internal shaping budget, not a changed client-visible limit.

Do not probe freshness for:

- `initialize`
- `ping`
- `tools/list`
- unknown tools
- protocol errors before tool dispatch

Known tool input errors should not receive stale metadata in this slice. Stale signaling is for successful
answers whose evidence may be old, not protocol/input validation failures.

## Warning Shape

Preferred implementation: add `WarningKind::StaleIndex` in `src/navigation/types.rs`.

Reason: stale evidence is a first-class trust signal for the served `Evidence`; keeping it only in `_meta` may
not be visible to the model or to existing JSON consumers reading `warnings`.

Implication: this touches `src/navigation/types.rs`, so run Tier-A per `AGENTS.md`, even though no query,
resolver, or CPG behavior changes.

This enum addition is compile-safe in the current codebase: existing `WarningKind` matches either compare one
known variant or use a wildcard. Tier-A should show matrix-unchanged / zero regressions; do not re-baseline for
this slice.

If reviewers reject the enum change as too broad, fallback is an MCP-only warning object inserted into JSON
values plus `_meta`. That avoids `src/navigation/*` but leaves the Rust `Evidence` type incomplete for stale
MCP results. The enum change is the cleaner contract.

## Content Synchronization

`apply_freshness_report` must keep the three surfaces coherent and remain idempotent:

1. `_meta` always gets freshness keys.
2. `structuredContent` gets a stale warning when it is an object. If `warnings` is absent, insert an array. If
   a `StaleIndex` warning is already present, replace/update it rather than appending a duplicate.
3. `content_text` gets the same visible signal:
   - detect format by `_meta["prism/content_text_format"]`
   - absent `prism/content_text_format` means canonical JSON
   - canonical JSON: serialize the updated structured `Value` with `serde_json::to_string_pretty`
   - `agent_json`: parse `content_text` as JSON when possible, ensure `warnings` is an array, add/replace the
     stale warning, set `summary.warnings` to the resulting warnings array length, and serialize with
     `serde_json::to_string_pretty`
   - `agent_json` parse failure or `_meta["prism/view_clipped"] == true`: leave `content_text` unchanged and
     rely on `_meta` plus `structuredContent.warnings` as the authoritative freshness signal; this covers the
     existing `bounded_notice(max_view_bytes)` fallback, which is plain text despite the declared agent format
   - `agent_markdown`: append a compact `## Freshness` section when absent, or replace the stale line in that
     section when present
   - `agent_markdown` clipped fallback: if `content_text` is the plain bounded notice, leave it unchanged and
     rely on `_meta` plus `structuredContent.warnings`

`structuredContent.warnings` plus `_meta` are the authoritative freshness signal. `content_text` is kept
model-visible and JSON-valid where applicable, but stale JSON text is not required to be byte-identical to the
fresh renderer because boundary code works with `serde_json::Value`, `Evidence` is `Serialize`-only, and this
crate does not enable `serde_json`'s `preserve_order` feature. Tests should assert JSON validity and
`StaleIndex` presence, not byte parity, for stale content text.

Agent-view `max_view_bytes` is an input to the fresh agent-view renderer, not a hard post-processing limit for
freshness text. Stale `content_text` may exceed `max_view_bytes` after freshness annotation, but the final
serialized MCP response must still fit the original transport budget.

Build the stale warning as a real `Warning { kind: WarningKind::StaleIndex, message, location: None }` and
serialize it with `serde_json::to_value` before inserting into raw `serde_json::Value` objects. Do not hand-roll a
different JSON shape; consumers should be able to deserialize `structuredContent` back into `Evidence`.

## Tool Description Updates

Add the snapshot sentence to every registered MCP tool:

- six navigation tools in `src/mcp/tools.rs`
- `taint_reaches` in `src/mcp/tools_reasoning.rs`

Prefer a small helper constant to avoid seven independent copies drifting.

## Tests

Unit tests:

1. `FreshnessProbe` is fresh immediately after bootstrap.
2. Editing an indexed source file marks stale and includes the relative file path.
3. Deleting an indexed source file marks stale.
4. Editing a tracked manifest marks stale.
5. Adding a file in a tracked directory marks stale through the directory stamp.
6. Changed path list is sorted and capped, with total count preserved.
7. `apply_freshness_report` adds `_meta` keys, a structured warning, and canonical `content_text` visibility.
8. `apply_freshness_report` does not duplicate stale warnings on repeated application.
9. `tools/list` descriptions mention snapshot semantics.
10. Maximum stale metadata/warning/text payload fits inside `FRESHNESS_RESERVE_BYTES` for canonical JSON,
    `agent_json`, and `agent_markdown`.

Transport/integration tests:

11. `serve_session_with_freshness` returns no stale metadata before edits.
12. After bootstrap, mutate an indexed file, then call `nav_nodes_at`; result has:
    - `_meta["prism/index_freshness"] == "stale"`
    - `structuredContent.warnings` includes `StaleIndex`
    - `content[0].text` includes the stale warning on canonical output
13. Same stale signal is visible for `agent_markdown`.
14. Same stale signal is visible for `agent_json`, including creation of a missing `warnings` array and
    `summary.warnings` set to the warnings array length.
15. `taint_reaches` receives stale signaling because it also serves evidence from the frozen
    `NavigationSession`.
16. A stale near-cap result remains within the MCP payload budget for canonical JSON and agent views.
17. Stale + agent view + tiny `max_view_bytes` leaves clipped fallback `content_text` unchanged while `_meta` and
    `structuredContent.warnings` carry the stale signal.
18. `tools/list`, `ping`, unknown tools, protocol errors, and known-tool input errors do not emit stale
    metadata.

Smoke test:

19. Extend `tests/mcp/smoke_test.rs` or add a focused integration test that starts `prism-mcp`, mutates a repo
    file after initialize, then calls a tool and observes stale metadata/warning.

Keep tests deterministic by avoiding sleeps where possible. If filesystem mtime granularity requires it on a
platform, prefer changing file length and directory entries because length/kind changes avoid timestamp-only
flakiness.

Directory-addition detection is inherently directory-mtime-bound on filesystems such as APFS. For the
directory-stamp test, set or advance the directory mtime deterministically where the platform allows it; do not
depend on wall-clock sleeps alone.

## Validation

Minimum validation before implementation review:

```bash
cargo fmt --check
cargo test --features mcp --lib
cargo test --features mcp --test mcp
git diff --check
git diff --check origin/main
```

Because the preferred warning contract adds `WarningKind::StaleIndex` in `src/navigation/types.rs`, also run
Tier-A before PR:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

If implementation avoids `src/navigation/*`, Tier-A is not required by the repo instruction, but a navigation
test pass is still useful:

```bash
cargo test --test navigation
```

## Deferred Follow-Ups

- Auto-refresh/rebuild on drift using incremental rebuild. Defer until the known incremental indirect-edge
  correctness caveat is resolved.
- Full file-universe drift detection using `.gitignore`/`.prismignore` aware walking. This belongs with the
  indexing policy slice.
- User-triggered `refresh_index` MCP tool. Defer until rebuild semantics are defined.
- A richer stale severity model distinguishing `source_changed`, `manifest_changed`, `directory_changed`, and
  `untracked_change`.

## Reviewer Questions

- Is `WarningKind::StaleIndex` worth the `src/navigation/types.rs` touch and Tier-A run, or should this slice
  keep stale signaling MCP-only?
- Should directory stamps be included now to catch additions/deletions, accepting conservative false-positive
  stale warnings?
- Is the `ToolContext` cap-reserve design the right tradeoff versus a post-shape over-cap degradation path?
- Is the content-text synchronization plan now sufficient for canonical JSON, agent JSON, and agent Markdown?
