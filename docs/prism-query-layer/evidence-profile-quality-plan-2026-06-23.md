# Evidence Profile Quality Plan

Status: A2A re-review approved after minor findings folded; ready for implementation
Date: 2026-06-23
Branch: `evidence-profile-quality`, based on `origin/main`

## Goal

The first evidence-shaping slice added the MCP extension contract:

- default MCP navigation output stays canonical
- `structuredContent` remains capped canonical `Evidence`
- `format: "agent_markdown"` and `format: "agent_json"` opt into an `EvidenceView`
- `profile`, `snippets`, `group_by`, and `max_view_bytes` are parsed and schema-advertised

This slice makes the `profile` knob materially useful without changing navigation query behavior. It should
improve LLM precision and recall at the presentation layer by changing ranking, grouping, reason labels,
trust signals, and next-query hints inside opt-in agent views only.

## Non-Goals

- Do not change `src/navigation/queries.rs`, call resolution, CPG construction, cache versions, or canonical
  `Evidence` serialization.
- Do not add new snippet window/body extraction yet. Keep snippet policies to the already supported
  `none`, `line`, and `symbol_header`.
- Do not make `profile` change default `content_text` or `structuredContent`.
- Do not add semantic summaries or LLM-generated prose. Views remain deterministic, source-grounded packets.

## Files

Expected implementation files:

- `src/mcp/evidence_view.rs`
- `src/mcp/tools.rs` for schema descriptions/tests and runtime view-kind wiring if needed
- `src/mcp/input.rs` only if profile validation/defaults need local tightening
- `docs/prism-query-layer/evidence-profile-quality-plan-2026-06-23.md`

Expected test locations:

- `src/mcp/tools.rs` unit tests
- optional `src/mcp/evidence_view.rs` unit tests if helper behavior becomes easier to test directly
- `tests/mcp/smoke_test.rs` for one JSON-RPC compatibility check

## A2A Round-1 Findings Folded

Codex and Claude both returned `NEEDS CHANGES` on the first draft. The valid findings are folded into
this revision:

- distinguish omitted `group_by` from explicit `group_by: "none"` before applying profile defaults
- derive trust from reason/source/fallback fields, not from `score < 1.0` or `symbol == None` alone
- keep `nav_nodes_at` hints file/line-safe by never combining a target definition file with a caller-side
  call-site line
- add a structured `reason` field to each next-query hint and make hints parser-valid in tests
- bump the agent-view schema version because view summary, trust, and next-query shapes change
- promote one compatibility assertion to an MCP JSON-RPC integration test

## Design

### 1. Add View Intent

Introduce internal profile policies in `src/mcp/evidence_view.rs`. `ViewOptions` already carries
`EvidenceProfile`; this slice should derive a `ProfilePolicy` that controls:

- default grouping if the caller did not explicitly set `group_by`
- result-set summary fields
- reason-label emphasis
- next-query hint selection
- warning/trust signal placement

The policy is internal to MCP view rendering. It must not leak into canonical `Evidence`.

Suggested policy shape:

```rust
struct ProfilePolicy {
    default_group_by: GroupPolicy,
    show_trust: bool,
    show_graph_summary: bool,
    next_query_mode: NextQueryMode,
    reason_mode: ReasonMode,
}
```

Do not add this exact struct if a smaller helper table is clearer. The important part is that behavior is
profile-driven instead of scattered `match` blocks in render functions.

This also requires the parsed options to preserve whether `group_by` was supplied. The change must be
purely additive to the current `ViewOptions`; keep the existing `requested` flag because it gates agent-view
rendering and preserves canonical default output. Keep `max_view_bytes` as the existing concrete `usize`.
The lowest-churn implementation is:

```rust
pub struct ViewOptions {
    pub format: ViewFormat,
    pub profile: EvidenceProfile,
    pub snippets: SnippetPolicy,
    pub group_by: GroupPolicy,
    pub group_by_explicit: bool,
    pub max_view_bytes: usize,
    pub requested: bool,
}
```

`parse_view_options` sets `group_by_explicit = obj.contains_key("group_by")`. Agent view rendering then uses
`options.group_by` when `group_by_explicit` is true, including explicit `GroupPolicy::None`.

### 2. Preserve Explicit Caller Choices

The caller's explicit `group_by` always wins. If `group_by` is omitted, the profile may choose a better
default only for agent views. Explicit `group_by: "none"` must suppress profile grouping.

- `impact`: group by symbol if symbols exist, otherwise file
- `dependencies`: group by file
- `orientation`: no item groups; prefer graph/module summary
- `edit_context`: no grouping by default, keep compact actionable items
- `audit`: no grouping by default, preserve raw-ish order
- `seed` and `graph`: no grouping by default

Compute an effective group policy at render time:

```rust
fn effective_group_by(options: &ViewOptions, evidence: &Evidence, policy: &ProfilePolicy) -> GroupPolicy {
    if options.group_by_explicit {
        options.group_by
    } else {
        choose_profile_default(policy.default_group_by, evidence)
    }
}
```

Do not change default MCP output. This only applies after `format` has selected an agent view.

### 3. Improve View Summary

Extend `ViewSummary` for agent views with deterministic, compact fields:

- `visible_items`
- `total_items`
- `canonical_items`
- `truncated`
- `warnings`
- `visible_files`
- `exact`
- `fallback`
- `unresolved`
- `heuristic`

Interpretation:

- `visible_files`: unique files represented by visible view item locations or graph nodes. Do not infer
  source-vs-target dependency semantics unless the evidence kind already carries that meaning.
- `exact`: visible items whose derived trust is `exact`. Exact means the view did not see fallback,
  heuristic, or unresolved evidence for that item; it does not guarantee a unique semantic target.
- `fallback`: visible items where `EvidenceItem::fallback == true`.
- `unresolved`: visible items with `Reason::UnresolvedImport`, or unresolved call evidence that has
  call reasons but no resolution/symbol.
- `heuristic`: visible items whose source is not `Source::PrismCpg`, after unresolved/fallback precedence.

Keep these fields deterministic and cheap. Do not infer resolver facts that are not present in `Evidence`.
Do not treat `score < 1.0` as fallback; scores can be discounted by traversal distance.

### 4. Profile-Specific Reason Labels

Keep `ViewItem.reason` as one concise string but improve labels by profile:

- `impact`: "caller `<name>` at call site line N" for `Reason::CalledBy`
- `dependencies`: "calls `<callee>` at line N" for `Reason::Calls`; label unresolved calls/imports clearly
- `orientation`: use file/module graph language
- `edit_context`: emphasize exact locator and reason needed before editing
- `audit`: include fallback/score/resolution kind when available

Avoid long prose. The goal is to help the LLM decide the next tool call or edit target.

### 5. Next-Query Hints

Replace the current generic "first three `nav_nodes_at`" hints with profile-aware hints. Every emitted hint
must pass the existing MCP input parser in debug/test coverage.

Add a structured reason to the hint:

```rust
struct NextQuery {
    tool: String,
    reason: String,
    arguments: serde_json::Value,
}
```

Use stable reason strings such as `call_site`, `caller_symbol`, `callee_symbol`, `callee_definition`,
`dependency_target`, `inspect_module`, `edit_locator`, and `result_truncated`.

Required hint behavior:

- `impact` on `nav_callers`: suggest `nav_nodes_at` for call sites and `nav_callers` for visible caller
  symbols when `EvidenceItem::symbol` is a structured `SymbolRef::Function { name, file }`.
- `dependencies` on `nav_callees`: suggest `nav_nodes_at` for safe visible locators and `nav_callees` for
  resolved callee symbols when `EvidenceItem::symbol` is structured.
- `orientation` on `nav_repo_map`: suggest `nav_module_deps` for top visible files.
- `orientation` on `nav_module_deps`: no graph is available, so degrade to no grouping, flat dependency
  items, and the normal summary/trust fields.
- `seed` / `edit_context`: suggest `nav_nodes_at` for visible locations.
- `audit`: suggest `nav_nodes_at` only; audit should stay close to raw evidence.

File/line safety invariants:

- `nav_nodes_at` may use `item.location.file` with `item.location.line` because that is the item locator.
- A caller-side call-site line from `Reason::CalledBy`, `Reason::Calls`, or module-call reasons may only be
  paired with the known source/queried file for that navigation request.
- Never pair `item.location.file` with a call-site line unless `item.location.file` is known to be the
  source file for that call-site line.
- For `nav_callees`, call-site hints are allowed only for depth-1 evidence and only when the original seed
  file is carried in `NavigationViewKind`; resolved callee items should otherwise emit `callee_definition`
  or `callee_symbol` hints.
- For `nav_module_deps`, carry the queried source file in `NavigationViewKind::ModuleDeps { file }` if
  module call-site hints are emitted. Otherwise emit only target definition/file hints.

Hints should stay bounded:

- maximum 5 hints
- no duplicate `(tool, reason, arguments)` triples
- no hints with invalid parser arguments
- build symbol-seeded hints from `EvidenceItem::symbol`, not from rendered labels

If the result is truncated, add at most one machine-readable `result_truncated` hint. The hint must still be
parser-valid and should prefer a safe visible locator such as `nav_nodes_at` for the first visible item. Do
not fabricate same-tool narrowing arguments unless the original seed/depth/max-results arguments are carried
in structured view context. For graph-only views where there are no visible items, fall back to the first
visible graph node locator if it has a parser-valid file/line.

Implementation note: `next_queries` should take the view options/profile, `NavigationViewKind`, and the
truncation flag instead of deriving all behavior from `Evidence` alone. Those values are already available
in `build_view`/`compose_view_result`.

### 6. Trust Signals

Agent views should surface trust state without changing canonical warnings:

- add per-item `trust` string: `exact`, `fallback`, `unresolved`, or `heuristic`
- add summary counts as above
- keep existing warnings in `warnings`

`trust` should be derived only from `EvidenceItem` fields and reasons:

- unresolved: `Reason::UnresolvedImport`, or unresolved call evidence that has call reasons but no
  `Reason::Resolution` and no symbol
- fallback: `fallback == true`
- heuristic: `source != Source::PrismCpg`
- exact: otherwise

If multiple labels apply, choose the first in this order: unresolved, fallback, heuristic, exact.

Important non-examples:

- resolved `nav_module_deps` evidence may have `symbol == None`; that is not unresolved by itself
- repo-map file graph nodes are file/module facts, not symbol-resolution facts; do not emit per-node trust
  for graph-only nodes unless an item-level `EvidenceItem` exists
- `score < 1.0` is not fallback because depth and confidence can both affect score

### 7. Rendering

Markdown should become more scannable:

- header stays short
- summary includes trust counts and files
- groups are stable and alphabetically keyed when grouping applies
- item lines include `trust=...`
- next queries remain machine-readable JSON after the tool name

Agent JSON should expose the same fields via `EvidenceView`.

Bump `VIEW_SCHEMA_VERSION` from `0.1` to `0.2` because summary fields, trust semantics, and next-query hint
shape are changing. This is an agent-view schema version only; it is not a CPG cache version.

Do not introduce a snapshot/golden file unless the existing unit assertions become brittle. Prefer semantic
JSON assertions and precise markdown contains/not-contains checks.

## Test Plan

Add tests for:

1. Omitted `group_by` lets `impact` apply profile grouping.
2. Explicit `group_by: "none"` suppresses profile grouping.
3. `dependencies` callee view labels unresolved calls and resolved callees distinctly.
4. Resolved module-deps/file evidence with `symbol == None` is not labeled unresolved.
5. Exact transitive evidence with `score < 1.0` is not labeled fallback unless `fallback == true`.
6. `orientation` repo-map view emits module-deps next-query hints for visible files and no graph-node trust.
7. `edit_context` keeps compact item order and emits safe `nav_nodes_at` hints.
8. `audit` includes trust and resolution detail without grouping by default.
9. Next-query hints include `reason` and parse through the relevant input parsers.
10. `nav_callees` and `nav_module_deps` call-site hints never combine target files with source call-site
    lines.
11. `VIEW_SCHEMA_VERSION` is `0.2`, including updating the existing schema-version assertion from `0.1`.
12. `tests/mcp/smoke_test.rs`: an opt-in agent view through JSON-RPC preserves canonical
    `structuredContent` equality with the default path.
13. Small `max_view_bytes` still clips view text while preserving canonical structured content.

## Validation

Run:

```bash
cargo fmt --check
cargo test --features mcp --lib
cargo test --features mcp --test mcp
cargo test --test navigation
git diff --check main..evidence-profile-quality
```

Run full `cargo test` if implementation touches anything outside `src/mcp/*` or if tests reveal an
unexpected interaction. Tier-A is not expected for this slice because it should not touch call resolution,
navigation queries, or CPG construction.

## Review Questions

Ask reviewers to focus on:

- whether the plan preserves canonical output compatibility
- whether the trust derivation is honest and does not overclaim precision
- whether the file/line invariants for `nav_callees` and `nav_module_deps` remove the cross-file hint risk
- whether the `group_by_explicit` approach is the smallest clean parser change
- whether any proposed test should be promoted to an integration test instead of a unit test
