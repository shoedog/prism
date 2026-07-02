# LLM Evidence Shaping for Navigation Results

Status: Historical implemented design; updated 2026-07-01
Date: 2026-06-23
Scope: MCP/navigation output ergonomics only; no source changes in this document

> Current status: opt-in MCP `EvidenceView` rendering is implemented in the current codebase, and later slices
> advanced `VIEW_SCHEMA_VERSION` to `0.4`. This file is retained for the original evidence-shaping design.

## Executive Summary

Prism already has a good canonical evidence substrate: precise locations, symbol refs, confidence scores,
warnings, graph payloads, and byte-capped MCP shaping. The missing layer is not more resolver logic. It is a
task-aware presentation layer that turns raw evidence lists into compact, navigable packets an LLM can use
with less guesswork and fewer follow-up reads.

The design goal is to keep `Evidence` as the canonical truth and add an adapter-level `EvidenceView` layer
that can group, rank, annotate, clip, and render the same evidence differently by use case:

- orientation: "what parts of the repo matter?"
- seeding: "what symbol is at this line?"
- impact: "what breaks if I change this?"
- dependency tracing: "what does this function depend on?"
- edit context: "give me the smallest code context needed before editing"
- audit/debug: "show raw confidence, warnings, and resolver reasons"

This should be additive and profile-driven. The first implementation slice should preserve the current MCP
default shape, expose opt-in views, and avoid `CACHE_VERSION` or CPG behavior changes.

## Conflict Boundary

This design intentionally avoids the active `member-visibility-tristate` implementation surface:

- Do not change `src/name_resolution/rust_policy.rs`, `src/name_resolution/types.rs`, or
  `src/name_resolution/glob_stats.rs`.
- Do not touch the protected `call_stats` / `glob_expand` block in `src/navigation/queries.rs`.
- Do not change `src/cpg_cache.rs` or cache versions.
- Do not edit the active Rust glob/member visibility tests.

Safe implementation areas for this design are primarily:

- `src/mcp/input.rs`
- `src/mcp/output.rs`
- `src/mcp/tools.rs`
- `src/output/navigation.rs`
- a new `src/navigation/evidence_shape.rs` or `src/mcp/evidence_view.rs`
- `tests/mcp/`, `src/mcp/*` unit tests, and `tests/navigation/`

The first implementation slice should avoid `src/navigation/queries.rs` entirely. A later canonical-output
cleanup may fix `nodes_at` variable path/access formatting, but that changes default structured output and
should be treated as a separate compatibility decision with golden updates.

## Research Takeaways

The repo already contains a research note at `docs/archive/analysis/prism-query-layer/research-llm-codebase-navigation.md`.
The parts that matter for evidence shaping are:

1. Structure-first navigation is the highest-confidence substrate. The strongest studies and deployed
   systems favor symbol/graph/lexical navigation as the backbone, with semantic retrieval as a fallback.
2. Hierarchical localization works because it controls context flow: file -> symbol -> line/edit site. Raw
   whole-repo context is too broad.
3. Long context and broad auto-generated context can hurt. More text is not the same as more usable evidence.
4. Pure semantic retrieval is weak for symbol-exact code tasks. LLMs need stable identifiers, exact file/line
   handles, and dependency edges.
5. The best navigation outputs are therefore not prose summaries. They are compact, verifiable packets:
   locations, reasons, confidence, warnings, and small code snippets only when the agent is close to action.

These research conclusions have caveats. Much of the published evidence is benchmark- or vendor-weighted, and
many studies are Python/Java-heavy. That is still enough to justify structure-first, compact evidence shaping,
but not enough to flip MCP defaults without Prism-specific Rust/Python/Go fixtures and agent-task measurements.

For Prism, the practical implication is clear: do not replace canonical CPG evidence with generated summaries.
Shape evidence so the model can choose the next tool call or edit target with less token waste.

## What LLMs Need From Navigation Output

LLMs navigating codebases need six classes of information.

1. Grounding handles
   Every result needs a stable locator: file, line range, byte range, symbol name, and enough identity to make a
   follow-up tool call. Prism already has this in `Location` and `SymbolRef`.

2. Why this result exists
   The result must explain the edge: call site line, import edge, containment, resolution kind, fallback, or
   uncertainty. Prism already has `Reason`, but concise mode currently clears reasons entirely.

3. Trust state
   Agents need to know whether evidence is exact, heuristic, stale, clipped, skipped, name-only, unresolved,
   or affected by known language gaps. Prism has warnings and scores, but the presentation does not summarize
   trust at the group/result-set level.

4. Progressive disclosure
   The first answer should help choose the next query, not dump every possible detail. Snippets should be
   opt-in or profile-driven: no snippets for orientation, call-site snippets for impact, symbol-window snippets
   for edit context.

5. Diversity and deduplication
   One item per call site is useful for audit, but it is noisy for impact. Impact views should group call sites
   by caller function or file while preserving the underlying call-site lines.

6. Action affordances
   A shaped result should say what to do next in machine-readable form: inspect these files, run
   `nav_nodes_at` on this call site, expand depth, lower depth because truncated, request snippets, or switch to
   raw audit view.

## Current Architecture Observations

Current canonical model:

```rust
pub struct Evidence {
    pub query: String,
    pub items: Vec<EvidenceItem>,
    pub truncated: bool,
    pub warnings: Vec<Warning>,
    pub graph: Option<GraphPayload>,
    pub reasoning: Option<ReasoningSummary>,
}

pub struct EvidenceItem {
    pub symbol: Option<SymbolRef>,
    pub location: Location,
    pub score: f32,
    pub source: Source,
    pub fallback: bool,
    pub why: Vec<Reason>,
    pub snippet: Option<String>,
}
```

The model is a good base, but the adapter still behaves like a raw serializer:

- `snippet` exists but navigation callers/callees/nodes-at currently populate `None`.
- `shape_result` clips by byte cap after query generation, not by task-aware group salience.
- concise mode removes `why`, which saves tokens but also removes the model's explanation of why a result was
  returned.
- graph clipping keeps a prefix of nodes, then drops edges to clipped nodes; this is deterministic but not
  necessarily the most useful subgraph.
- MCP input has `verbosity`, `depth`, `hops`, and `max_results`, but no output profile, snippet policy, group
  policy, token/byte budget, or result format.
- `content_text` is currently pretty JSON, duplicating `structuredContent` rather than giving an agent-oriented
  text packet.
- skipped files are tracked by the loader, but not summarized in repo-map/orientation output.

There is also a small cleanup opportunity outside the protected member-visibility block: variable symbols in
`nodes_at` use debug formatting for paths/accesses, which leaks implementation shape into the output. That is
not part of the default-preserving first slice because it changes canonical `SymbolRef::Variable` serialization.

## Design Principles

1. Canonical evidence remains the source of truth.
   Shaping must not resolve new edges or mutate resolver confidence. It can group, annotate, rank, and render
   evidence, but it must preserve raw evidence for audit.

2. Profiles are data, not separate query implementations.
   Each profile should configure grouping, snippets, rank preferences, and render format. Avoid copying logic
   per tool.

3. Defaults stay stable in the first slice.
   Existing MCP consumers should continue receiving current `Evidence` unless they opt into a view/profile.

4. Every shaped output remains verifiable.
   If a view summarizes 20 call sites into one group, it must retain the call-site lines and enough item IDs or
   locators to reconstruct the raw evidence.

5. Budgeting happens before serialization when possible.
   The current serialized-byte fallback remains the final guardrail, but the new layer should choose what to
   keep by salience before the byte cap forces blind clipping.

6. Warnings are first-class.
   A shaped result that omits details must still preserve truncation, skip, ambiguity, stale-index, parse-quality,
   and language-gap warnings.

## Proposed Architecture

Add an adapter-level evidence shaping layer.

```rust
pub struct EvidenceShapeOptions {
    pub profile: EvidenceProfile,
    pub format: EvidenceFormat,
    pub detail: EvidenceDetail,
    pub snippets: SnippetPolicy,
    pub group_by: GroupPolicy,
    pub budget: EvidenceBudget,
    pub include_next_queries: bool,
}

pub enum EvidenceProfile {
    Orientation,
    Seed,
    Impact,
    Dependencies,
    EditContext,
    Graph,
    Audit,
}

pub enum EvidenceFormat {
    CanonicalJson,
    AgentMarkdown,
    AgentJson,
    JsonLines,
}

pub enum SnippetPolicy {
    None,
    Line,
    Window { before: usize, after: usize },
    SymbolHeader,
    SymbolBody { max_bytes: usize },
}

pub enum GroupPolicy {
    None,
    ByFile,
    BySymbol,
    ByCaller,
    ByCallee,
    ByModuleEdge,
}
```

The output of the shaping layer is a view, not a replacement resolver result:

```rust
pub struct EvidenceView {
    pub query: String,
    pub profile: EvidenceProfile,
    pub summary: EvidenceSummary,
    pub groups: Vec<EvidenceGroup>,
    pub loose_items: Vec<ViewItem>,
    pub graph: Option<ViewGraph>,
    pub warnings: Vec<Warning>,
    pub next_queries: Vec<NextQuery>,
    pub meta: ShapeMeta,
}
```

`EvidenceView` is derived from `Evidence` plus `NavigationSession` only for enrichment such as snippets, source
line extraction, skipped-file summaries, and stale-index checks. It should not call resolver functions again.

### Placement

Prefer one of these placements:

1. `src/mcp/evidence_view.rs` if the view is initially MCP-only.
2. `src/navigation/evidence_shape.rs` if CLI navigation output should use the same profiles soon.

The better long-term location is `src/navigation/evidence_shape.rs`, with MCP-specific schema parsing/rendering
in `src/mcp/input.rs` and `src/mcp/output.rs`. That keeps the profile model reusable by CLI and future HTTP MCP.

### Compatibility Strategy

Phase 1 should keep defaults unchanged:

- existing inputs with no `profile` or `format` return the current canonical Evidence JSON
- `structuredContent` always remains canonical `Evidence`, even when an opt-in view is requested
- `content_text` may become a shaped markdown/text packet only when `format: "agent_markdown"` is requested
- `format: "agent_json"` means the shaped `EvidenceView` is serialized into `content_text` as JSON, not that
  `structuredContent` changes shape
- schema version can remain the existing MCP output schema for default behavior

The hard rule is: `format` must not silently change the `structuredContent` schema. A future breaking MCP major
version can revisit this, but it is out of scope for this design. Opt-in views should advertise themselves via
metadata such as `_meta["prism/view_schema_version"] = "0.1"` and `_meta["prism/content_text_format"]`.

Only after consumer validation should the default MCP text content move from pretty JSON to `agent_markdown`.
Until then, the trade-off is deliberate: opt-in views deliver value to agents that ask for them, while default
calls remain byte-compatible for existing clients.

## Complementary Formats By Use Case

### 1. Orientation Packet

Use for `nav_repo_map` and broad `nav_module_deps`.

Purpose: help the model decide where to look next.

Shape:

- compact file-level graph
- top modules by inbound/outbound degree or score
- grouped edges with a small sample of reasons
- skipped-file summary by reason
- warnings about truncation and repo coverage
- no snippets by default

Good output format: `agent_markdown` for humans/LLMs, `agent_json` for downstream agents.

### 2. Seed Packet

Use for `nav_nodes_at`.

Purpose: convert a file/line into exact follow-up handles.

Shape:

- symbols at line
- enclosing function
- variables and access paths as currently serialized; view-only display labels may normalize them without
  changing canonical `SymbolRef::Variable`
- one-line or small window snippet
- next-query hints: callers/callees for functions, taint source/sink for variables

Good output format: compact JSON or markdown.

### 3. Impact Packet

Use for `nav_callers`.

Purpose: answer "what breaks if I change this?"

Shape:

- group by caller function, then file
- retain all call-site lines per group
- rank exact/direct callers above name-only/indirect/fallback callers
- preserve resolution kind and confidence
- include call-site snippets only for top groups or when requested
- include test/prod/generated labels later

Good output format: markdown for agent planning; canonical raw for audit.

### 4. Dependency Packet

Use for `nav_callees`.

Purpose: answer "what does this depend on?"

Shape:

- group by resolved callee
- separately group unresolved call sites
- show direct dependencies first, then transitive hops
- include call-site line snippets when the source-side anchor is known; otherwise prefer target symbol headers
- next-query hints to inspect unresolved calls or expand exact-only mode

Good output format: markdown or agent JSON.

### 5. Edit Context Packet

Use when a model is about to modify code.

Purpose: provide the smallest code context that makes the edit safe.

Shape:

- small snippets around the seed, top callers, and top callees
- function signatures/headers where full bodies are unnecessary
- warnings if snippets were omitted due to cap
- raw locators for immediate file reads

Good output format: markdown text, because the model can read it directly.

### 6. Graph Packet

Use for `nav_ego_graph` and future graph traversals.

Purpose: preserve topology without overwhelming the model.

Shape:

- central seed node
- adjacency lists grouped by edge kind
- small node table with file/line/symbol
- edge counts for clipped nodes
- preserve enough node IDs to request raw graph

Good output format: agent JSON for machine traversal; markdown adjacency for direct LLM use.

### 7. Audit Packet

Use when debugging Prism or reviewing precision/recall.

Purpose: expose every reason and confidence detail.

Shape:

- no grouping unless explicitly requested
- raw items, raw reasons, source, fallback, confidence score
- all warnings
- deterministic order

Good output format: canonical JSON.

### 8. JSONL Rows

Future format for very large result sets.

Purpose: allow streaming or external post-processing.

Shape:

- one evidence row per item or group
- header row for metadata/warnings
- stable row schema

This is useful for batch eval and non-interactive workflows, but not needed for the first implementation slice.

## Ranking And Grouping

Current sorting is mostly score, file, line. That is deterministic but not always useful for LLM workflows.

Add a view-level ranker that computes a view score from existing evidence fields:

```text
view_score =
  confidence_weight
  + directness_weight
  + seed_locality_weight
  + source_rank_weight
  + exact_symbol_match_weight
  + group_diversity_weight
  - hop_penalty
  - fallback_penalty
```

Rules:

- Never hide the raw `score`; view score is presentation-only.
- Group before clipping for impact/dependency views.
- Keep diversity: for broad callers, prefer top groups across different files before showing ten repeated
  call sites from the same helper.
- Preserve all grouped call-site lines, even when only the group summary is shown.
- Put unresolved/fallback groups in their own section rather than mixing them with exact hits.

## Snippet Policy

Prism can derive snippets from `ParsedFile.source` and existing byte/line ranges.

Rules:

- snippets are bounded by bytes and line count
- snippets are optional by default
- call results should prefer call-site snippets, not whole callee bodies
- seed/edit context can include symbol headers or small windows
- snippets must identify their origin: `source_file`, `snippet_start_line`, `snippet_end_line`, and
  `anchor_kind`
- if snippets are clipped or omitted, emit a warning or shape meta entry
- slice-1 snippets live only in `EvidenceView`; do not populate canonical `EvidenceItem.snippet`

Recommended first policies:

- `none`: current behavior
- `line`: the exact source line at the selected snippet anchor
- `window`: one line before and after
- `symbol_header`: first non-empty line of a function/symbol span

Defer full function bodies. They are expensive and often recreate the long-context problem this design is trying
to avoid.

### Snippet Anchors

Do not assume `EvidenceItem.location` is the snippet source. For call queries it often describes the function
definition, not the call site. The view layer needs an explicit source-side anchor:

```rust
pub struct SnippetRef {
    pub source_file: String,
    pub snippet_start_line: usize,
    pub snippet_end_line: usize,
    pub anchor_kind: SnippetAnchorKind,
}

pub enum SnippetAnchorKind {
    EvidenceLocation,
    CallSite,
    SymbolHeader,
}
```

Anchor matrix:

| Tool/profile | Preferred snippet anchor | Notes |
|---|---|---|
| `nav_nodes_at` / seed | `EvidenceLocation` | Use queried file/line or item location. |
| `nav_callers` / impact | `CallSite` | `Reason::CalledBy.call_site_line` is in the caller file, which is also the item function file. |
| `nav_callees` / dependencies | `CallSite` only when source file is known | For direct callees, the source file is the seed function file. For transitive callees, canonical `Evidence` currently drops the frontier caller file, so slice 1 must omit call-site snippets or fall back to `SymbolHeader` rather than guess. |
| `nav_module_deps` / orientation | `CallSite` samples | Source file is the queried module file; target file is only the dependency. |
| `nav_repo_map` / orientation | none | Prefer graph summaries and skipped-file coverage. |
| `nav_ego_graph` / graph | `EvidenceLocation` or none | Keep graph views compact; snippets are optional. |

If future work wants full call-site snippets for transitive `nav_callees`, add an internal view-side
`CallSiteRef { source_file, source_function, line, callee, qualifier }` while building the result, or make an
explicit canonical schema change. Do not reconstruct source files by guessing from callee locations.

## Next Query Hints

A shaped result should include machine-readable next-query hints. These are not prose instructions; they are
ready-to-call suggestions.

```rust
pub struct NextQuery {
    pub tool: String,
    pub arguments: serde_json::Value,
    pub reason: String,
    pub priority: f32,
}
```

Examples:

- from `nav_repo_map`: inspect top hub file with `nav_module_deps`
- from `nav_nodes_at`: call `nav_callers` for the enclosing function
- from truncated `nav_callers`: rerun with lower depth or narrower file seed
- from unresolved `nav_callees`: inspect call site with `nav_nodes_at`
- from skipped-file warning: read the file directly or adjust index policy

These hints should be deterministic and conservative. Avoid speculative chains that create tool spam.

Every emitted hint must round-trip through the corresponding MCP parser before it is returned. This matters
because the input layer rejects unknown fields and uses tagged unions for seeds. When new optional fields are
added, update both the parser allowlists in `src/mcp/input.rs` and the advertised tool schemas/descriptors in
`src/mcp/tools.rs` / `src/mcp/registry.rs`. Tests should parse every emitted `NextQuery.arguments` through the
target tool's `parse_*` function.

## Budgeting Model

Keep the current hard serialized-byte cap as the final safety mechanism, but add pre-serialization budgeting:

1. derive canonical evidence
2. enrich items with cheap metadata and optional snippets
3. group by profile
4. rank groups/items
5. fit group summaries to the requested budget
6. fit snippets inside remaining budget
7. serialize
8. fall back to existing byte-cap binary search if still over cap

Budget state should be visible:

```rust
pub struct ShapeMeta {
    pub profile: String,
    pub format: String,
    pub total_items: usize,
    pub retained_items: usize,
    pub total_groups: usize,
    pub retained_groups: usize,
    pub snippets_requested: bool,
    pub snippets_retained: usize,
    pub clipped_by: Vec<String>,
}
```

This solves a current ambiguity: `truncated: true` says something was clipped, but not whether the loss was
items, graph nodes, snippets, reasons, or source detail.

The shaped path must receive the full canonical result before `max_results` clipping. Today MCP handlers call
`clip_flat` / `clip_graph` before `shape_result`; an opt-in shaped request must instead pass the full `Evidence`
plus `max_results`/view budget into the shaper, then let the shaper group, rank, and clip. The existing
pre-shaping clip path should remain only for canonical default output.

This full-evidence path is for deriving `content_text` views only. Even for opt-in `agent_markdown` /
`agent_json`, `structuredContent` should still be produced from the same canonical `max_results`-clipped
`Evidence` a default call would return with the same arguments. That keeps machine consumers byte-compatible
while allowing the markdown/json view to group and budget from full pre-clip evidence.

`ShapeMeta.clipped_by` is supplemental. It must compose with the existing authoritative `truncated` flag and
`WarningKind::ResultTruncated` machinery rather than producing competing truncation warnings.

## Extension Points

Design the layer around small traits or modules:

```rust
trait EvidenceEnricher {
    fn enrich(&self, session: &NavigationSession, evidence: &mut EvidenceView);
}

trait EvidenceRanker {
    fn rank(&self, group: &EvidenceGroup, profile: EvidenceProfile) -> f32;
}

trait EvidenceRenderer {
    fn render(&self, view: &EvidenceView) -> McpToolResult;
}
```

Initial enrichers:

- snippet enricher
- skipped-file/coverage enricher
- truncation enricher
- follow-up query enricher

Future enrichers:

- stale-index warning
- test/generated/source-set labels
- language-gap warnings
- git churn/co-change hints
- semantic search backfill
- external index provenance, such as SCIP/LSIF/Glean

The important constraint: enrichers may add context and warnings, but they must not silently change resolver
truth.

## MCP Input Design

Add optional fields to the nav tools:

```json
{
  "profile": "impact",
  "format": "agent_markdown",
  "snippets": "line",
  "group_by": "caller",
  "max_results": 50,
  "max_view_bytes": 40000,
  "verbosity": "concise"
}
```

For compatibility:

- all new fields are optional
- unknown values are rejected with clear `BadArguments`
- default profile is derived from the tool, but default format remains canonical JSON in Phase 1
- `verbosity` remains accepted and maps into `detail`
- each new request field must be added to the relevant `object(...)` allowlist in `src/mcp/input.rs` and to the
  advertised schema in `src/mcp/tools.rs`; otherwise the parser will reject the request before shaping runs

Suggested profile defaults when the caller opts into `format: "agent_markdown"`:

- `nav_repo_map`: `orientation`
- `nav_nodes_at`: `seed`
- `nav_callers`: `impact`
- `nav_callees`: `dependencies`
- `nav_ego_graph`: `graph`
- `nav_module_deps`: `orientation`

## Example Impact Packet

```markdown
query: callers:parse_config@src/config.rs
profile: impact
items: 8 callers in 5 files; showing 5 groups
trust: 4 exact, 1 name-only; 0 skipped; not stale

1. src/server.rs:42 run_server
   score=1.00 exact direct
   call sites: 44, 51
   snippet:
     44 | let cfg = parse_config(path)?;
   next: nav_nodes_at {"file":"src/server.rs","line":44}

2. src/cli.rs:18 main
   score=1.00 exact direct
   call sites: 22

warnings:
- showing 5 of 8 groups; raise max_results or narrow the seed
```

The canonical `Evidence` remains available for audit; this packet is only the model-facing view.

## Implementation Plan

### Phase 0: Golden fixtures and no behavior change

- Add tests that snapshot current canonical `Evidence` output for representative `nav_nodes_at`,
  `nav_callers`, `nav_callees`, `nav_repo_map`, and `nav_ego_graph` cases.
- Assert default-path `structuredContent` byte identity where feasible, plus output size and item counts.
- Add small-cap tests that warnings and `ResultTruncated` behavior remain stable before adding shaped views.

### Phase 1: Opt-in view-only snippets

- Add snippet extraction helper using `ParsedFile.source`.
- Populate snippets only in `EvidenceView`, never in canonical `EvidenceItem.snippet`.
- Keep `src/navigation/queries.rs` byte-identical in this slice.
- For `nav_callees`, provide call-site snippets only when the source-side file is known; otherwise use
  `symbol_header` or omit the snippet with explicit shape metadata.
- Add `snippet` tests for line/window policies.
- Keep default output unchanged.

### Phase 2: EvidenceView and profiles

- Add `EvidenceShapeOptions`, `EvidenceView`, groups, meta, and next-query hints.
- Implement `agent_markdown` and `agent_json` renderers.
- Add profile-aware grouping for callers/callees/module deps.
- Preserve canonical `structuredContent` for every opt-in format.
- Route shaped requests around pre-shaping `clip_flat` / `clip_graph` so grouping sees the full result.
- Round-trip every emitted `NextQuery.arguments` through the relevant MCP parser in tests.

### Phase 3: Budget-aware clipping

- Add pre-serialization clipping by group/item/snippet budgets.
- Preserve warning summaries and exact retained/total counts.
- Improve graph clipping for `nav_ego_graph` and `nav_repo_map` by rank/salience rather than prefix only, but
  define a total deterministic order with explicit tie-breakers before changing graph output.

### Phase 4: Coverage and trust metadata

- Add skipped-file summaries to repo-map/orientation views.
- Add parse-quality and stale-index warnings if available.
- Add test/prod/generated labels after index-policy decisions are settled.

### Phase 5: Evaluation

- Add a small prompt-level eval: same navigation task with canonical JSON vs shaped packet.
- Measure token count, follow-up tool calls, localization accuracy, and edit success on a small fixture suite.
- Use results to decide whether `agent_markdown` should become the MCP text default.
- Include at least Rust and Python fixtures before considering any default MCP text change; Tier-A is necessary
  for resolver safety but does not measure shaping quality by itself.

## Validation

For implementation touching MCP/navigation output:

```bash
cargo fmt
cargo test --features mcp mcp::output
cargo test --features mcp --test mcp
cargo test --test navigation
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

The Tier-A commands are required because implementation will touch navigation query/output paths. Use
`--allow-stale-sut` only immediately after the release build in the same worktree.

Shaping-specific tests should additionally assert:

- default MCP calls keep canonical `structuredContent` and current `content_text`
- opt-in `agent_markdown` changes only `content_text` and view metadata, not `structuredContent`
- small byte caps preserve warnings and produce one authoritative `ResultTruncated` warning
- snippet anchors choose caller/source files correctly and never infer source from callee definition locations
- every emitted `NextQuery.arguments` parses through the target tool input parser
- unknown `profile`, `format`, `snippets`, and `group_by` values fail with clear `BadArguments`

## Risks

1. Breaking MCP consumers
   Mitigation: keep default canonical output unchanged; add opt-in formats first.

2. Presentation layer hides important resolver warnings
   Mitigation: warnings and clipping metadata are always visible and tested under small byte caps.

3. Snippets recreate long-context overload
   Mitigation: snippets default to none/line/window; full symbol bodies deferred.

4. Grouping reduces auditability
   Mitigation: groups retain raw locators and call-site lines; audit profile remains raw.

5. Feature creep into resolver behavior
   Mitigation: shaping is downstream of canonical `Evidence`; no resolver calls during shaping.

## External Review Integration

This draft was reviewed through A2A Bridge by `claude-opus` and `codex-review-xhigh` (`gpt-5.5` with
`model_reasoning_effort="xhigh"`). Both reviewers agreed the canonical `Evidence` / derived `EvidenceView`
split is the right backbone. The design was revised to incorporate these findings:

- `structuredContent` must remain canonical `Evidence` for every first-slice format.
- shaped requests must receive full evidence before `clip_flat` / `clip_graph` truncation.
- snippets must use explicit source-side anchors; `EvidenceItem.location` is not enough for call-site snippets.
- slice-1 snippets should be view-only to avoid `src/navigation/queries.rs` and the active branch conflict.
- next-query hints must round-trip through MCP parsers and schemas.
- default-path golden tests are required because Tier-A does not measure output shaping regressions.

## Open Questions

Resolved decisions:

- `structuredContent` stays canonical `Evidence` for all first-slice formats.
- `content_text` switches to `agent_markdown` or `agent_json` only when `format` is explicitly requested.
- slice-1 snippets live in `EvidenceView`; canonical `EvidenceItem.snippet` stays unchanged.
- first-slice implementation avoids `src/navigation/queries.rs`.

Remaining questions:

1. Should the first implementation expose `profile`, `format`, and `snippets` on all nav tools, or only on
   `nav_callers` / `nav_callees` where the value is most obvious?
2. Should graph salience use simple degree/seed-distance first, or wait for measured agent tasks?
3. Should next-query hints be included in every view by default, or only in agent-oriented formats?

## Recommended First Slice

Implement the smallest valuable slice:

1. Add `SnippetPolicy` parsing and view-only source-line snippet extraction.
2. Add opt-in `format: "agent_markdown"` for `nav_callers` and `nav_callees`.
3. Group callers by caller function and callees by callee/unresolved call.
4. Keep canonical `structuredContent` and canonical `EvidenceItem.snippet` unchanged.
5. Bypass pre-shaping `clip_flat` / `clip_graph` for opt-in view derivation so grouping sees the full result;
   still build `structuredContent` from canonical `max_results`-clipped evidence.
6. Add retained/total group metadata and parser-validated next-query hints.
7. Add explicit tests for default byte-compatibility, opt-in markdown, snippet anchors, small caps, and
   warning preservation.

This delivers visible LLM value without touching CPG construction, cache versioning, Rust glob/member
resolution, or `src/navigation/queries.rs`.
