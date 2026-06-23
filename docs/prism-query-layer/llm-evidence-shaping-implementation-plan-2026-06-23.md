# LLM Evidence Shaping Implementation Plan

Status: reviewed implementation brief
Date: 2026-06-23
Branch: `evidence-shaping`, based on `origin/main`

## Scope

Add opt-in MCP evidence views for navigation results. The canonical `Evidence` envelope remains the
source of truth and the default MCP output remains byte-for-byte compatible unless a caller explicitly
requests an agent view format.

This slice is intentionally limited to MCP adapter code:

- add parser/schema support for view controls in `src/mcp/input.rs` and `src/mcp/tools.rs`
- add view composition in `src/mcp/evidence_view.rs`
- keep `src/navigation/queries.rs`, resolver code, CPG construction, and cache versions unchanged
- add MCP-focused tests for compatibility, cap behavior, snippets, metadata, and parser validation

## Reviewed Design Decisions

The A2A plan review from Claude Opus and Codex GPT-5 xhigh produced these constraints.

1. Branch from `origin/main` in a clean worktree:

   ```bash
   git fetch origin
   git worktree add -b evidence-shaping /private/tmp/slicing-evidence-shaping origin/main
   ```

2. Keep slice 1 in `src/mcp/evidence_view.rs`. Do not place MCP view composition in
   `src/navigation/`, because navigation should stay independent of MCP transport concerns.

3. Preserve the canonical path:

   - `structuredContent` is the existing max-results and byte-capped canonical `Evidence`
   - default `content_text` remains the existing pretty JSON rendering
   - agent-oriented text is produced only for explicit `format: "agent_markdown"` or
     `format: "agent_json"`

4. Build agent views from pre-`max_results` evidence where possible, but cap the final MCP result as a
   whole. If the view plus canonical `structuredContent` exceeds the transport budget, clip the view
   first. Do not mutate canonical `Evidence` to fit the view.

5. Add extension knobs now, with conservative validation:

   - `format`: `canonical_json`, `agent_markdown`, `agent_json`
   - `profile`: `orientation`, `seed`, `impact`, `dependencies`, `edit_context`, `graph`, `audit`
   - `snippets`: `none`, `line`, `symbol_header`
   - `group_by`: `none`, `file`, `symbol`
   - `max_view_bytes`: integer byte cap for `content_text`

   View controls other than `format: "canonical_json"` require an agent format, so unsupported
   combinations fail fast instead of silently changing defaults.

6. Snippet safety:

   - `nav_callers` may show call-site line snippets because each item location is the caller function and
     `Reason::CalledBy.call_site_line` identifies the call site in that same file.
   - `nav_callees` may show call-site line snippets only for `depth == 1` and file-known seeds. For
     deeper expansion, unfiled symbol seeds, or target-side-only locations, emit symbol headers or omit
     snippets.

7. Add metadata to shaped results:

   - `_meta["prism/view_schema_version"]`
   - `_meta["prism/content_text_format"]`

## Implementation Steps

1. Add MCP input enums and `ViewOptions`.
2. Extend navigation tool schemas with the opt-in view controls.
3. Add `src/mcp/evidence_view.rs`:
   - `shape_navigation_result` wrapper around existing `shape_result`
   - `EvidenceView` and compact view item/group structs
   - markdown and JSON renderers
   - final-response cap loop that clips view item count before falling back to a short view message
4. Update navigation tool handlers to pass both full and canonical-clipped evidence into the wrapper.
5. Add tests:
   - default output compatibility
   - `max_results=1` canonical `structuredContent` equality with and without an agent view
   - final response stays within a small cap
   - callers line snippets include the call site
   - callees line snippets are omitted when depth is greater than 1
   - parser/schema validation rejects view controls without an agent format

## Validation

Targeted validation for this slice:

```bash
cargo fmt --check
cargo test --features mcp --lib
cargo test --features mcp --test mcp
cargo test --test navigation
git diff --check main..evidence-shaping
```

Because this touches MCP navigation output but not CPG construction or resolver behavior, run the Tier-A
harness only after an immediate release rebuild if this proceeds to review:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```
