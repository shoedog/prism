# Prism EvidenceView Contract Hardening - Design (2026-06-23, rev 2)

Status: A2A spec review findings folded; ready for implementation
Branch: `evidence-view-spec`, based on `origin/main`

> Next MCP/query-layer slice after the merged evidence-shaping and evidence-profile-quality work.
> This is not a resolver, CPG, or navigation-query slice. It tightens the opt-in `EvidenceView` contract so
> LLM consumers can use navigation results with better grounding, recall, and next-step precision while
> canonical `Evidence` remains stable.
>
> Rev 2 folds the A2A review findings from Claude Opus and Codex GPT-5.5 xhigh: item/node locations stay
> byte-complete, next-query provenance uses a separate line-capable `ViewSourceLocation`, `Reason::Reasoning`
> and symbol-bearing reasons get deterministic mappings, canonical-vs-view schema versions are distinguished,
> `symbol_ref` is fixed to canonical `SymbolRef` reuse, and the test plan now covers the edge cases reviewers
> flagged.

## 1. Premise correction

The early design draft framed EvidenceView as a new layer to build. That layer now exists on `main`.

Committed inputs:

- `docs/prism-query-layer/llm-evidence-shaping-implementation-plan-2026-06-23.md`
- `docs/prism-query-layer/evidence-profile-quality-plan-2026-06-23.md`
- `docs/prism-query-layer/evidence-profile-quality-implementation-handoff-2026-06-23.md`
- `src/mcp/evidence_view.rs`
- `src/mcp/input.rs`
- `src/mcp/tools.rs`
- `tests/mcp/smoke_test.rs`

Current implementation facts:

- Default MCP navigation output remains canonical.
- `structuredContent` remains capped canonical `Evidence`.
- `format: "agent_markdown"` and `format: "agent_json"` opt into an `EvidenceView`.
- `profile`, `snippets`, `group_by`, and `max_view_bytes` are parsed and schema-advertised.
- `VIEW_SCHEMA_VERSION` is `0.2`.
- Profiles already drive grouping, summary trust counts, reason labels, snippets, and next-query hints.
- Next-query hints are capped, deduped, and parser-validated in tests.

The remaining gap is not "make profiles exist." The gap is that the agent-facing view is still a compact
presentation of canonical evidence, not yet a strong stable contract for LLM tool-chaining. In particular,
agent JSON currently carries mostly display strings (`loc`, `symbol`, `reason`) where follow-on agents need
normalized handles, reason arrays, and group-level summaries.

## 2. Goal

Make opt-in EvidenceViews good enough to serve as the default LLM-facing navigation packet without changing
the canonical `Evidence` data model or navigation behavior.

Success means:

- An LLM can cite and re-query exact file/line/span handles from `agent_json` without parsing display text.
- A human or LLM reading `agent_markdown` sees compact grounding, trust, and next-query affordances.
- Groups explain coverage and trust mix instead of merely nesting items.
- Audit/debug views can preserve raw reason evidence while concise profiles stay compact.
- All behavior remains deterministic, source-grounded, byte-bounded, and opt-in.

## 3. Why this matters for LLM navigation

LLM navigation failures usually come from one of six presentation problems, even when the underlying graph is
good:

1. **Weak grounding handles.** If a result only says `foo@src/a.rs:12`, the model must parse text before it
   can call `nav_nodes_at`, quote the location, or compare two results. Structured file, line, byte span, and
   symbol handles reduce that failure mode.
2. **Unclear result intent.** Callers, callees, repo maps, and file dependencies answer different questions.
   The view should label why an item is in the result, not just where it is.
3. **Trust ambiguity.** Exact, heuristic, fallback, unresolved, and stale evidence should not read the same.
   The model needs this to avoid overclaiming.
4. **Context budget pressure.** Full evidence is useful for audit, but most agent turns need a ranked,
   clipped, progressively disclosed packet with next queries.
5. **Duplicate or low-diversity evidence.** A long list of adjacent results can crowd out the repo shape. The
   view should support grouping and summary counts that preserve coverage.
6. **No action affordance.** Navigation results are most useful when they carry valid next tool calls, because
   that turns one result into a reliable exploration path.

Those needs are presentation-layer needs. They do not require resolver or CPG changes, which makes this the
best low-conflict follow-on after the MCP work.

## 4. Complementary formats

Keep multiple formats because they serve different consumers:

| Format | Consumer | Contract |
|---|---|---|
| `canonical_json` | Tests, deterministic clients, regressions, compatibility | Existing canonical `Evidence`; default path; unchanged. |
| `agent_json` | LLMs that chain MCP calls, agents that post-process results | Stable view schema with normalized handles, grouped summaries, trust, reasons, and next queries. |
| `agent_markdown` | Human-in-loop review and conversational LLM reading | Compact rendered summary of the same view, not a separate source of truth. |

Future formats can be added behind the same view builder:

- `jsonl` for streaming many evidence items.
- `graph_json` for UI graph rendering.
- `diff_context` for edit-review packets.

This slice should not add those future formats. It should make the existing two agent formats extensible by
using a versioned internal schema and additive fields.

## 5. Non-goals

- No edits to `src/navigation/queries.rs`.
- No resolver, call graph, CPG, parser, or cache-version changes.
- No `CACHE_VERSION` bump.
- No change to default MCP output.
- No change to canonical `Evidence` serialization.
- No LLM-generated summaries.
- No new source snippet policy beyond the existing `none`, `line`, and `symbol_header`.
- No ranking rewrite based on new semantic signals. Keep ordering from canonical evidence for this slice.
- No indexing policy or `is_test` labels yet.

## 6. Conflict boundary

Expected source files:

- `src/mcp/evidence_view.rs`
- `src/mcp/input.rs`, only if schema/version/default parsing needs a small additive change
- `src/mcp/tools.rs`, for tool schema descriptions and tests
- `src/mcp/output.rs`, only if a shared result/meta helper is genuinely needed

Expected tests:

- `src/mcp/tools.rs`
- `src/mcp/evidence_view.rs`, if helper-level tests are clearer
- `tests/mcp/smoke_test.rs`

Avoid:

- `src/navigation/queries.rs`
- `src/navigation/types.rs`
- `src/cpg_cache.rs`
- `src/cpg/`
- `src/call_graph.rs`
- resolver and language policy files
- Tier-A baseline files

## 7. Wire contract

The tool input surface stays as it is today:

```json
{
  "format": "agent_json",
  "profile": "impact",
  "snippets": "line",
  "group_by": "symbol",
  "max_view_bytes": 12000
}
```

Rules:

- `format` defaults to `canonical_json`.
- `profile`, `snippets`, `group_by`, and `max_view_bytes` require an agent format unless only
  `format: "canonical_json"` is supplied.
- Tool-specific profile allowlists stay enforced in `src/mcp/input.rs`.
- Explicit `group_by: "none"` continues to override profile defaults.
- Agent views are only reflected in `content_text` and `_meta`; `structuredContent` remains canonical
  `Evidence`.

## 8. Schema revision

Bump `VIEW_SCHEMA_VERSION` from `0.2` to `0.3` because `agent_json` adds fields that consumers can rely on.
The change is additive for view consumers and invisible to default canonical consumers.

Do not conflate this with the canonical MCP envelope schema version. `_meta["prism/schema_version"]` from
`src/mcp/output.rs` stays at its current canonical value in this slice. Only
`_meta["prism/view_schema_version"]` and `EvidenceView.meta.schema_version` move to `0.3`.

Target `EvidenceView` shape:

```rust
struct EvidenceView {
    query: String,
    profile: String,
    summary: ViewSummary,
    groups: Vec<ViewGroup>,
    items: Vec<ViewItem>,
    graph: Option<ViewGraph>,
    warnings: Vec<Warning>,
    next_queries: Vec<NextQuery>,
    meta: ViewMeta,
}
```

Keep that top-level shape. Harden the nested structs.

## 9. Normalized location handles

Add a structured location alongside the existing display `loc` string.

```rust
struct ViewLocation {
    file: String,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
    display: String,
}

struct ViewItem {
    loc: String,                 // compatibility/display
    location: ViewLocation,      // machine handle
    symbol: Option<String>,      // compatibility/display
    symbol_ref: Option<SymbolRef>,
    score: f32,
    trust: &'static str,
    reason: String,
    reasons: Vec<ViewReason>,
    snippet: Option<String>,
}
```

Notes:

- `loc` stays for markdown compatibility and compact reading.
- `location.display` should equal `loc` so clients do not need to duplicate formatting.
- `symbol_ref` reuses canonical `SymbolRef` serialization exactly, including its externally tagged enum shape
  and `ordinal` fields. Do not invent a second symbol schema in this slice.
- Add the same `location` object to `ViewNode`.
- Do not add path normalization beyond the existing repo-relative evidence location.
- `ViewLocation` is only for canonical evidence item and graph-node locations, which already carry byte spans.
  Next-query provenance uses `ViewSourceLocation` in section 14 because call-site reasons only carry a line.

Why this matters: next-query hints are useful, but an agent still needs stable handles for citations,
dedupe, comparisons, and manual follow-up queries. It should not have to parse `loc`.

## 10. Reason policy

Keep the current concise `reason` string. Add a deterministic `reasons` array for machine readers.

```rust
struct ViewReason {
    kind: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}
```

Profile behavior:

- `impact`, `dependencies`, `orientation`, `seed`, `edit_context`, and `graph`: include a compact subset,
  usually the salient reason plus any `Resolution` reason needed to explain trust.
- `audit`: include all canonical reasons in stable order.

Reason labels remain deterministic strings, for example:

- `called_by`
- `calls`
- `resolution`
- `enclosing_function`
- `containment`
- `resolved_import`
- `unresolved_import`
- `reasoning`

Mapping rules:

- `Reason::CalledBy { caller, call_site_line }`: `kind: "called_by"`, `label: "called by `<caller>` at line
  N"`, `target: caller`, `line: call_site_line`.
- `Reason::Calls { callee, call_site_line, qualifier }`: `kind: "calls"`, `label: "calls `<callee>` at line
  N"`, `target: callee`, `line: call_site_line`, and `detail: "qualifier=<q>"` only when present.
- `Reason::Resolution { kind }`: `kind: "resolution"`, `label: "resolution <kind>"`, `detail: kind`.
- `Reason::EnclosingFunction { function }`: `kind: "enclosing_function"`,
  `label: "enclosing function <symbol>"`, `target` is the symbol label, and `file`/`line` are extracted from
  the wrapped `SymbolRef`.
- `Reason::Containment { parent }`: `kind: "containment"`, `label: "contained by <symbol>"`, `target` is the
  symbol label, and `file`/`line` are extracted from the wrapped `SymbolRef`.
- `Reason::ResolvedImport { module, target_file }`: `kind: "resolved_import"`, `target: module`,
  `file: target_file`.
- `Reason::UnresolvedImport { module }`: `kind: "unresolved_import"`, `target: module`.
- `Reason::Reasoning(ReasoningReason::TaintedBy { source, sanitizers_present_in_source_fn, path_proven })`:
  `kind: "reasoning"`, `label: "tainted by <source>"`, `target` is the source symbol label, `file`/`line`
  come from the source `SymbolRef`, and `detail` is deterministic text such as
  `path_proven=true sanitizers=2`.

Do not serialize arbitrary debug output. If a future canonical reason cannot be represented cleanly, use
`kind: "other"` with a short deterministic label and no `Debug` formatting.

For `SymbolRef` file/line extraction, use the variant's native location fields:

- `Function`: `file` and `start_line`.
- `Statement`: `file` and `line`.
- `Variable`: `file` and `line`.

## 11. Group summaries

Current groups contain only `key`, `item_count`, and `items`. Add cheap summaries so an LLM can decide whether
to expand or act on a group without reading every item.

```rust
struct ViewGroup {
    key: String,
    item_count: usize,
    file_count: usize,
    trust: TrustCounts,
    representative_locations: Vec<ViewLocation>,
    items: Vec<ViewItem>,
}
```

Rules:

- `representative_locations` is capped at 3.
- Representatives are first visible items in group order, not re-ranked.
- `file_count` uses item locations only.
- `trust` counts items after each item has been classified with the same precedence as item trust:
  unresolved, fallback, heuristic, exact.
- Group summaries are present in `agent_json`.
- `agent_markdown` renders group headers compactly, for example:
  `## foo (items=4 files=2 exact=3 fallback=1)`.
- Markdown group trust buckets render in this order: `exact`, `fallback`, `unresolved`, `heuristic`.
  Zero-count buckets are omitted from markdown but remain present in `agent_json`.

## 12. Trust policy

Preserve the trust semantics from the profile-quality slice:

- `unresolved`: `Reason::UnresolvedImport`, or unresolved call evidence with call reasons but no
  `Reason::Resolution` and no symbol.
- `fallback`: `EvidenceItem::fallback == true`.
- `heuristic`: `source != Source::PrismCpg`.
- `exact`: otherwise.

Precedence: unresolved, fallback, heuristic, exact.

This slice should only move trust counts into reusable structs and group summaries. It should not reinterpret
score, missing symbols, or graph-only nodes.

## 13. Snippet policy

Keep snippet scope intentionally narrow:

- `none`: no snippets.
- `line`: one source line, with the current file/line safety rules.
- `symbol_header`: one source line at the symbol/item start line.

Do not add multi-line windows in this slice. Line snippets are enough to ground call-site or definition
context while keeping the packet bounded and safe. A future `window` policy can reuse `ViewLocation` and the
existing cap loop.

## 14. Next-query policy

Keep existing constraints:

- Maximum 5 hints.
- Deduplicate `(tool, reason, arguments)`.
- Validate hint arguments through the existing parser helpers.
- Build symbol-seeded hints from `EvidenceItem::symbol`, not rendered labels.
- Never pair a target file with a caller-side call-site line.

Add one field:

```rust
struct ViewSourceLocation {
    kind: &'static str,          // item, graph_node, or call_site
    file: String,
    line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_byte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_byte: Option<usize>,
    display: String,
}

struct NextQuery {
    tool: &'static str,
    reason: &'static str,
    arguments: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_location: Option<ViewSourceLocation>,
}
```

`source_location` explains why the hint exists. It is not a second set of arguments. It is intentionally not
`ViewLocation`: call-site hints are derived from `Reason::Calls` / `Reason::CalledBy`, which carry a source
file by view context plus `call_site_line` but no byte span.

Per-hint source rules:

- `call_site`: `kind: "call_site"`, `file` is the known source file for the call site, `line` is
  `call_site_line`, and bytes are `None`.
- `caller_symbol`, `callee_symbol`, `callee_definition`, `dependency_target`, and `edit_locator`:
  `kind: "item"`, derived from the producing `EvidenceItem.location`, with bytes present.
- `inspect_module`: `kind: "graph_node"`, derived from the producing graph node location, with bytes present.
- `result_truncated`: use the first visible item or graph-node anchor, with bytes present when that anchor is
  item/node-derived; omit `source_location` only if there is no visible anchor.

Call-site file safety remains mandatory:

- `nav_callers` call-site anchors use the caller item file.
- Direct `nav_callees` call-site anchors use `NavigationViewKind::Callees.seed_file`.
- `nav_module_deps` call-site anchors use `NavigationViewKind::ModuleDeps.file`.
- Never use a callee/target item file with a caller-side `call_site_line`.

The existing `reason` vocabulary remains:

- `call_site`
- `caller_symbol`
- `callee_symbol`
- `callee_definition`
- `dependency_target`
- `inspect_module`
- `edit_locator`
- `result_truncated`

## 15. Markdown rendering

`agent_markdown` should remain compact and scan-friendly:

1. Header with query, profile, visible/total counts, files, and trust counts.
2. Warnings, if any.
3. Graph summary, if present.
4. Groups or flat items.
5. Next queries.

Do not render full JSON blobs inline except for next-query `arguments`, because those are meant to be copied
or consumed by an agent. Prefer stable one-line records:

```text
- `src/a.rs:12-18` `foo` score=1.00 trust=exact reason=called by `bar` at line 42
```

## 16. Byte budget and clipping

Preserve current cap behavior:

- Build the canonical result first.
- Agent view text is clipped before canonical `structuredContent`.
- Binary-search visible item count until `content_text.len() <= max_view_bytes` and the full serialized result
  fits the transport budget.
- If even zero items cannot fit, return the bounded notice and set view metadata.

Add tests around the new fields because structured locations and reason arrays increase view size. The
fallback path must still preserve canonical `structuredContent`.

## 17. Adaptability and extension architecture

Avoid scattering profile behavior across rendering functions. Keep or extend the current policy-table shape:

```rust
struct ProfilePolicy {
    default_group_by: GroupPolicy,
    reason_mode: ReasonMode,
    group_summary: bool,
    next_query_mode: NextQueryMode,
}
```

This exact struct is not required, but the design should keep these axes separate:

- Input validation and tool allowlists: `src/mcp/input.rs`
- Evidence-to-view projection: `src/mcp/evidence_view.rs`
- Rendering: markdown vs JSON functions
- Tool descriptions and tests: `src/mcp/tools.rs`
- Transport caps and stale metadata composition: existing MCP output/transport helpers

Future profiles should require adding a policy entry and tests, not rewriting the builder.

## 18. Implementation plan

### Step 1 - Add view schema structs

In `src/mcp/evidence_view.rs`:

- Add `ViewLocation`.
- Add `ViewSourceLocation`.
- Add `ViewReason`.
- Add reusable `TrustCounts` serialization.
- Add `location` and `symbol_ref` to `ViewItem`.
- Add `location` to `ViewNode`.
- Add group summary fields.
- Add `source_location` to `NextQuery`.
- Bump `VIEW_SCHEMA_VERSION` to `0.3`.

Keep all new fields additive and local to the view schema.

### Step 2 - Build normalized handles

Add small helpers:

```rust
fn view_location(location: &Location) -> ViewLocation
fn view_symbol(symbol: &Option<SymbolRef>) -> Option<SymbolRef>
fn view_reasons(item: &EvidenceItem, profile: EvidenceProfile) -> Vec<ViewReason>
fn item_source_location(item: &EvidenceItem) -> ViewSourceLocation
fn graph_source_location(node: &GraphNode) -> ViewSourceLocation
fn call_site_source_location(file: &str, line: usize) -> ViewSourceLocation
```

`view_symbol` clones the canonical symbol for the view. If clone noise becomes large, pass references
internally and serialize owned view structs at the end; do not change canonical types.

### Step 3 - Extend groups and next queries

- Compute `TrustCounts` from grouped `ViewItem`s.
- Compute `file_count` from `ViewItem.location.file`.
- Add up to 3 representative locations.
- Attach `source_location` to next-query hints.
- Keep parser validation tests for every emitted hint shape.

### Step 4 - Update markdown rendering

- Include group summary counts in group headings.
- Keep item lines one-line unless snippet is requested.
- Keep next-query rendering stable.

### Step 5 - Update schema descriptions

In `src/mcp/tools.rs`, describe the `agent_json` additions:

- normalized `location`
- optional `symbol_ref`
- `reasons`
- group summaries
- `source_location` on next queries

Tool descriptions should still state snapshot semantics and opt-in behavior.

## 19. Tests

Required targeted tests:

1. Default canonical compatibility:
   - default `nav_callers`, `nav_callees`, and `nav_repo_map` still produce canonical content text and identical
     `structuredContent` versus pre-view behavior.
2. `agent_json` schema:
   - parses as JSON
   - has `meta.schema_version == "0.3"`
   - MCP `_meta["prism/view_schema_version"] == "0.3"`
   - existing `src/mcp/tools.rs` assertions for `prism/view_schema_version == "0.2"` are updated to `0.3`
   - canonical MCP `_meta["prism/schema_version"]` stays at the canonical envelope version and is not bumped
     by this slice
   - item has both `loc` and `location`
   - `location.display == loc`
   - item has `reasons`
   - symbol-backed item has `symbol_ref`
   - symbolless item omits `symbol_ref`
   - graph view node has `location`
3. Group summaries:
   - `impact` callers default grouping includes `file_count`, `trust`, and representative locations.
   - explicit `group_by: "none"` suppresses groups.
   - markdown group trust buckets are ordered and omit zero-count buckets.
4. Trust semantics:
   - symbolless resolved module dependency remains `exact`.
   - transitive score discount does not imply fallback.
   - unresolved import/call evidence is `unresolved`.
5. Next-query source locations:
   - `nav_callers` call-site hint has `source_location.kind == "call_site"` in the caller file, line equals
     `call_site_line`, and bytes are omitted.
   - `nav_callees` direct call-site hint uses the seed file and omits bytes.
   - `nav_module_deps` call-site hint uses the queried source file and omits bytes.
   - target-derived hints use item locations with bytes present and do not mix target file with source
     call-site line.
   - hints with no visible anchor omit `source_location` rather than fabricating one.
6. Snippet safety:
   - `line` snippets remain single-line.
   - depth > 1 `nav_callees` does not show seed call-site snippets.
7. Reason mapping:
   - `Reason::Reasoning` produces deterministic non-debug labels.
   - `EnclosingFunction` and `Containment` extract `target`, `file`, and `line` from wrapped `SymbolRef`.
   - audit profile preserves canonical reason order.
8. Cap behavior:
   - small `max_view_bytes` clips only agent view text.
   - canonical `structuredContent` remains present and capped by existing canonical rules.
9. JSON-RPC smoke:
   - one opt-in `agent_json` navigation call through `tests/mcp/smoke_test.rs`.

Validation:

```bash
cargo fmt --check
cargo test --features mcp --lib
cargo test --features mcp --test mcp
cargo test --test navigation
git diff --check origin/main
```

Tier-A is not required for this docs/spec slice. For the implementation slice, Tier-A is still optional unless
the diff touches `src/navigation/`, `src/cpg/`, `src/call_graph.rs`, or `src/ast.rs`; if it does, follow
`AGENTS.md`.

## 20. Acceptance criteria

- `VIEW_SCHEMA_VERSION` is `0.3`.
- The canonical MCP envelope schema version is not bumped.
- Default MCP navigation behavior remains canonical and compatibility tests pass.
- `structuredContent` equality holds between default and opt-in agent view calls for the same query and
  canonical caps.
- `agent_json` has normalized locations, optional canonical symbol refs, reason arrays, group summaries, and
  source locations on next-query hints.
- All next-query hints remain parser-valid and capped to 5.
- Markdown remains compact and does not expose debug formatting.
- No resolver, CPG, navigation-query, or cache-version files are changed.

## 21. Review questions

Ask reviewers to focus on:

- Whether profile-filtered `reasons` for non-audit views are compact enough while audit remains complete.
- Whether `source_location` on `NextQuery` is enough provenance now that call-site anchors are line-only, or
  whether hints also need an `item_index`.
- Whether bumping the view schema to `0.3` is sufficient for consumer adaptation.
- Whether any proposed test requires touching navigation internals and should be deferred.
