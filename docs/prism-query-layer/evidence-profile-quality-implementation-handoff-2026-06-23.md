# Handoff: Evidence Profile Quality Implementation

Status: implemented locally, validated, publish-ready
Date: 2026-06-23
Worktree: `/private/tmp/slicing-evidence-profile-quality`
Branch: `evidence-profile-quality`, based on `origin/main`

## State of the World

This slice makes MCP navigation `profile` controls materially useful for opt-in agent views only.
Default MCP output and canonical `structuredContent` remain canonical `Evidence`.

Current worktree state:

- Modified:
  - `src/mcp/evidence_view.rs`
  - `src/mcp/input.rs`
  - `src/mcp/tools.rs`
  - `tests/mcp/smoke_test.rs`
- Added:
  - `docs/prism-query-layer/evidence-profile-quality-plan-2026-06-23.md`
  - `docs/prism-query-layer/evidence-profile-quality-implementation-handoff-2026-06-23.md`

No `src/navigation/*`, resolver, CPG, or cache-version files were edited.

## Review Gate

The implementation plan went through A2A review before source edits:

- First round:
  - Codex task `3f816162-50c0-4cd0-a1e4-7daa285db3f8`: `NEEDS CHANGES`
  - Claude task `c8fa7862-54e3-4291-855c-617b1808ffa1`: `NEEDS CHANGES`
- Re-review after folding findings:
  - Codex task `bcc9a1bf-22c1-4f16-9f83-b59ca026e7fd`: `READY TO IMPLEMENT`
  - Claude task `336ec610-fd13-4f27-a1b2-b5b9f6c8012b`: design/scope approved, one required plan-text correction and minor clarifications folded

The final plan artifact is:

- `docs/prism-query-layer/evidence-profile-quality-plan-2026-06-23.md`

Implementation review after source edits:

- Codex task `7c78be65-d9a3-4e61-a9ef-5b5d948e52ff`: `APPROVE`
- Claude task `a52368c5-72e9-43b5-9709-270161530ffc`: `NEEDS CHANGES`, test adequacy only
- Folded follow-up tests:
  - `nav_module_deps` call-site hints stay on the queried source file while dependency targets use target locations.
  - Small `max_view_bytes` clips only agent view text while preserving canonical `structuredContent`.

## Implemented Behavior

Parser/state:

- `ViewOptions` now tracks `group_by_explicit`.
- Explicit `group_by: "none"` suppresses profile default grouping.
- The existing `requested` flag remains intact, preserving the canonical-output gate.

Agent view schema:

- `VIEW_SCHEMA_VERSION` is now `0.2`.
- `ViewSummary` includes:
  - `visible_files`
  - `exact`
  - `fallback`
  - `unresolved`
  - `heuristic`

Trust:

- Per-item `trust` is one of `exact`, `fallback`, `unresolved`, `heuristic`.
- `score < 1.0` is not treated as fallback.
- `symbol == None` is not treated as unresolved by itself.
- Resolved module/file evidence with no symbol can still be `exact`.
- Graph-only repo-map nodes do not get per-node trust.

Grouping:

- `impact` defaults to symbol grouping when symbols exist, otherwise file.
- `dependencies` defaults to file grouping.
- `orientation`, `edit_context`, `audit`, `seed`, and `graph` default to no grouping.
- Explicit caller grouping always wins.

Next-query hints:

- Hints now include `tool`, `reason`, and parser-valid `arguments`.
- Hints are bounded to 5 and deduped by `(tool, reason, arguments)`.
- Symbol-seeded hints come from structured `EvidenceItem.symbol`, not rendered labels.
- `nav_callees` call-site hints use the original seed file, only for direct depth-1 evidence.
- `nav_callees` target locations are labeled as `callee_definition`.
- `nav_module_deps` carries the queried source file in `NavigationViewKind::ModuleDeps { file }`.
- `nav_module_deps` call-site hints use the queried source file; dependency targets use item locations.
- `nav_repo_map` orientation hints suggest `nav_module_deps` for visible graph files.

Markdown:

- Header remains short.
- Summary now includes visible file count and trust counts.
- Item lines include `trust=...`.
- Next-query lines include `reason=...` and machine-readable JSON arguments.

## Tests Added or Updated

Unit-level MCP tests in `src/mcp/tools.rs` cover:

- `impact` profile default grouping and explicit `group_by: "none"` override.
- Symbolless resolved module dependency remains `exact`, not unresolved.
- Transitive score discount does not imply fallback.
- `nav_callees` next-query call-site hints stay on the seed/source file.
- `nav_module_deps` next-query call-site hints stay on the queried source file.
- Clipped agent view text preserves canonical `structuredContent`.
- Repo-map orientation emits module-deps hints without graph-node trust.
- Existing schema-version assertion updated to `0.2`.

Parser test in `src/mcp/input.rs` covers:

- Explicit `group_by: "none"` is tracked as explicit.

Integration smoke in `tests/mcp/smoke_test.rs` now covers:

- Default JSON-RPC `nav_callees`.
- Opt-in `agent_markdown` JSON-RPC `nav_callees`.
- Equality of canonical `structuredContent` between default and agent view calls.

## Validation Run

All validation completed in `/private/tmp/slicing-evidence-profile-quality`:

```bash
cargo fmt --check
cargo test --features mcp --lib
cargo test --features mcp --test mcp
cargo test --test navigation
git diff --check
git diff --check origin/main
```

Results:

- `cargo fmt --check`: passed
- `cargo test --features mcp --lib`: passed, 440 tests
- `cargo test --features mcp --test mcp`: passed, 1 test
- `cargo test --test navigation`: passed, 63 tests
- `git diff --check`: passed
- `git diff --check origin/main`: passed

Known unrelated warning:

- `cargo test --features mcp --lib` emits an existing unused-import warning in `src/type_providers/go.rs` for `BTreeSet`.

Tier-A was not run. This slice does not edit `src/navigation/queries.rs`, call resolution, CPG construction, or cache behavior.

## Reviewer-Sensitive Invariants

Preserve these if continuing the branch:

- Do not move view shaping into `src/navigation/*`.
- Do not mutate canonical `Evidence` to improve agent views.
- Do not derive fallback from score.
- Do not derive unresolved from missing symbol alone.
- Never pair a target definition file with a caller-side call-site line in `nav_nodes_at` hints.
- Keep explicit `group_by` stronger than profile defaults.
- Keep JSON-RPC `structuredContent` equality test passing for default vs opt-in agent view.

## Next Actions

Recommended next steps:

1. Review the diff once for ergonomics and naming.
2. Commit the intended files from the isolated worktree only.
3. Push `evidence-profile-quality`.
4. Open a PR against `main`.
5. Include the validation commands above in the PR body.

Suggested commit message:

```text
feat(mcp): improve evidence profile agent views
```
