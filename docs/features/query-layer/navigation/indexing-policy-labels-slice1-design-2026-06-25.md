# Indexing Policy Labels - Slice 1 Design

Status: Historical implemented design; updated 2026-07-01
Date: 2026-06-25
Branch: `codex/indexing-policy-labels`, based on `origin/main` at `84cc3d2`
Predecessors:

- `docs/features/query-layer/navigation/follow-on-roadmap-2026-06-23.md`
- `docs/features/query-layer/evidence/llm-evidence-shaping-design-2026-06-23.md`
- `docs/superpowers/specs/2026-06-23-prism-evidence-view-contract-design.md`
- `docs/features/query-layer/mcp-refresh/mcp-auto-incremental-refresh-slice-c-design-2026-06-24.md`

> Current status: production/test evidence labels are implemented in the current codebase via
> `src/navigation/code_context.rs` and `src/mcp/evidence_view.rs`; agent views advertise
> `prism/view_indexing_policy = code_role_v1`. This file is retained for the Slice 1 design.

## 1. Goal

Improve LLM navigation precision by labeling evidence as production or test evidence without changing what Prism
indexes, resolves, caches, or returns by default.

This is the first indexing-policy slice after EvidenceView and MCP auto-refresh. It should answer:

> "Are these callers/dependencies production code, test code, or mixed?"

Success means:

- opt-in `agent_json` and `agent_markdown` views expose deterministic production/test labels;
- summaries and groups report production/test counts;
- tests remain indexed and visible by default;
- canonical `Evidence`, resolver behavior, CPG/cache behavior, and query result ordering remain unchanged;
- the implementation creates a reusable classifier seam for later `.gitignore` / `.prismignore` / `scope`
  filtering slices.

## 2. Why This Matters

Prism's call graph can be correct while still presenting a misleading answer to an LLM. In the Prism repo itself,
many high-fanout callers are tests. For impact analysis, "15 callers" is less useful than "2 production callers,
13 test callers." Test evidence should not be hidden, because it is often the best usage documentation, but it
must be categorically distinct from production evidence.

The label is especially valuable for:

- `nav_callers`: "what breaks if I change this?"
- `nav_callees`: "what does this depend on?"
- `nav_module_deps` and `nav_repo_map`: "is this edge part of product code or test-only structure?"
- Tier-C / agent-value experiments, where overclaiming test-only reachability is a common failure mode.

This slice is deliberately smaller than a full indexing-policy implementation. It labels evidence; it does not
filter it.

## 3. Current State

### 3.1 Loader and index

`repo_loader::load_repo` currently:

- skips built-in directories such as `.git`, `target`, `node_modules`, `vendor`, `dist`, and `build`;
- skips hidden dirs, symlinks, oversized files, unreadable/non-UTF-8 files, and severe parse failures;
- returns `LoadedRepo { files, file_hashes, manifest_hashes, scope_graph_inputs, skipped, type_db }`.

It does not:

- consult `.gitignore`;
- support `.prismignore`;
- label tests, generated files, fixtures, or vendored sources beyond coarse skipped directory names.

### 3.2 Canonical evidence

`src/navigation/types.rs` defines canonical `Evidence`:

- `Evidence.items: Vec<EvidenceItem>`
- `Evidence.graph: Option<GraphPayload>`
- `EvidenceItem.location`
- `EvidenceItem.symbol`
- `EvidenceItem.why`

Canonical evidence has no production/test field today. Many tests assert serialized canonical evidence shapes.

### 3.3 EvidenceView

`src/mcp/evidence_view.rs` already provides opt-in agent views:

- `format: "agent_json"` and `format: "agent_markdown"`
- `VIEW_SCHEMA_VERSION = "0.3"`
- `ViewSummary`, `ViewGroup`, `ViewItem`, `ViewGraph`, and next-query hints
- `structuredContent` remains canonical `Evidence`

This is the right surface for Slice 1 labels because it is LLM-facing, opt-in, and already has summary/group
metadata.

## 4. Non-Goals

- No `.gitignore` support.
- No `.prismignore` support.
- No `scope: prod|test|all` input parameter.
- No exclusion of tests, fixtures, generated files, or vendored code from indexing.
- No canonical `Evidence` schema change in this slice.
- No edits to `src/navigation/queries.rs`.
- No resolver, call graph, CPG, data-flow, parser, or cache-version changes.
- No `CACHE_VERSION` bump.
- No ranking or truncation change based on production/test labels.
- No span-level Rust `#[cfg(test)]` classification in this slice.
- No generated-code labeling in this slice, except through a future-proof schema shape that can add it later.

## 5. Design Principle

Label, do not filter.

The first production/test label should be:

- deterministic;
- derived only from repository-relative paths and language/file naming conventions;
- stable for a file across all evidence items and graph nodes;
- surfaced in agent views only;
- stored behind a reusable module so later filtering can use the same classifier.

## 6. Classifier Contract

Add a small reusable classifier module, tentatively:

```rust
// src/navigation/code_context.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeRole {
    Production,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CodeContext {
    pub role: CodeRole,
    pub is_test: bool,
    pub reasons: Vec<&'static str>,
}

pub fn classify_file(path: &str) -> CodeContext;
```

Rules:

- `is_test` is redundant with `role == Test`, but intentionally present for cheap downstream consumption.
- `reasons` must be deterministic, low-cardinality strings suitable for tests and docs.
- No filesystem reads. Classification takes a repository-relative path string only.
- If no rule matches, return production with an empty reason list.
- The classifier is file-level only in this slice.

### 6.1 Initial test heuristics

The first rule set should be conservative enough to avoid surprising production mislabels while catching common
test paths:

Authoritative path component rules:

- component equals `test`
- component equals `tests`
- component equals `__tests__`
- component equals `testdata`

Ancillary path component rules:

- component equals `spec`
- component equals `specs`
- component equals `fixtures`
- component equals `fixture`

Ancillary components are not sufficient by themselves. They are reason-only signals that should be retained when an
authoritative path component or filename rule already classifies a file as test. This deliberately avoids false
positives such as `docs/superpowers/specs/*` while still preserving auditable reasons for paths like
`tests/fixtures/foo.rs` or `pkg/spec/foo.spec.ts`.

Filename rules:

- Python: `test_*.py`, `*_test.py`
- Go: `*_test.go`
- Rust: `*_test.rs`
- Ruby/RSpec: `*_spec.rb`
- Java/JVM: `*Test.java`, `*Tests.java`, `*IT.java`
- JavaScript/TypeScript/TSX/JSX: `*.test.js`, `*.spec.js`, `*.test.ts`, `*.spec.ts`,
  `*.test.tsx`, `*.spec.tsx`, `*.test.jsx`, `*.spec.jsx`
- Generic fallback: basename contains `.test.` or `.spec.`

Reason strings should identify the first matching rule family, for example:

- `path_component:tests`
- `path_component:testdata`
- `filename_prefix:test_`
- `filename_suffix:_test.go`
- `filename_suffix:Test.java`
- `filename_infix:.spec.`

If multiple rules match, keep all reasons in deterministic rule order. This is useful for debugging and does not
affect role assignment.

The role decision is:

- `Test` when any authoritative path component rule matches;
- `Test` when any filename rule matches;
- otherwise `Production`.

The `reasons` vector may include ancillary matches even when they do not independently determine the role, but tests
should cover that `docs/superpowers/specs/foo.rs` remains production.

### 6.2 Known limitations

This slice will not label these precisely:

- Rust unit tests inside `src/*.rs` under `#[cfg(test)] mod tests`.
- Python tests in unconventional names without path or filename signals.
- Generated files such as `*_pb2.py`, `*.gen.go`, checked-in `*.d.ts` bundles.
- Vendor code outside existing built-in skipped directories.

Those limitations should be explicit in the spec and tests. They are acceptable because this slice adds a useful
positive signal without hiding evidence.

## 7. EvidenceView Contract

Bump `VIEW_SCHEMA_VERSION` from `0.3` to `0.4`.

Do not change `_meta["prism/schema_version"]`, canonical `Evidence`, or default MCP content.

### 7.1 View item shape

Add `code` to `ViewItem`:

```rust
struct ViewItem {
    // existing fields...
    code: ViewCodeContext,
}

struct ViewCodeContext {
    role: &'static str,       // "production" | "test"
    is_test: bool,
    reasons: Vec<&'static str>,
}
```

`code` is an extensible object. Slice 1 only defines `role`, `is_test`, and `reasons`; future slices should add
fields, not redefine existing fields. `role` values are append-only for a given `view_schema_version`. Generated,
vendor, and ignored state should become additive flags/fields rather than replacing `role`.

The field should always be present in `agent_json`. For markdown, render a compact label on each item:

```text
- `src/lib.rs:10-12` run score=1.00; trust=exact; code=production; called by run at line 10
- `tests/foo_test.rs:8-9` test_run score=1.00; trust=exact; code=test; called by test_run at line 8
```

### 7.2 Summary shape

Extend `ViewSummary`:

```rust
struct ViewSummary {
    // existing fields...
    production: usize,
    test: usize,
}
```

Counts use visible view items, not canonical clipped count. For graph-shaped views, counts use visible graph nodes.
If a view has both graph and items, use the same counting strategy that drives `visible_items`: graph nodes when
`graph.is_some()`, otherwise items/groups. Do not mirror trust-count aggregation if that path only sees item/group
evidence; pure graph views such as repo-map must not report `production=0 test=0` just because `items` is empty.

Markdown summary should include:

```text
code: production=2 test=13
```

### 7.3 Group shape

Extend `ViewGroup`:

```rust
struct ViewGroup {
    // existing fields...
    code: CodeCounts,
}

struct CodeCounts {
    production: usize,
    test: usize,
}
```

Markdown group headings should include non-zero code counts:

```text
## build_cfg_edges (items=15 files=6 exact=15 production=1 test=14)
```

This intentionally follows the existing trust-count omit-zero convention for group headings. The summary line always
renders both `production=N` and `test=M`.

### 7.4 Graph nodes

Extend `ViewNode`:

```rust
struct ViewNode {
    // existing fields...
    code: ViewCodeContext,
}
```

This matters for `nav_repo_map`, where file nodes often reveal whether a module edge is test-only. Do not change
`GraphPayload` or canonical graph serialization.

Markdown graph node lines should include the same compact role label as item lines:

```text
- `src/lib.rs:1-40` module::run code=production
- `tests/lib_test.rs:1-20` module::test_run code=test
```

### 7.5 Meta

Add a view-level metadata marker:

```rust
ViewMeta {
    // existing fields...
    indexing_policy: "code_role_v1",
}
```

Also add `_meta["prism/view_indexing_policy"] = "code_role_v1"` for agent views. This lets clients know labels
come from deterministic path heuristics, not compiler truth.

The metadata marker must be present on every agent view response path, including the fully clipped bounded-notice
fallback. The fallback should advertise `prism/view_schema_version`, `prism/content_text_format`,
`prism/view_profile`, and `prism/view_clipped` so clients do not need a separate compatibility branch for tiny
`max_view_bytes` responses.

## 8. Placement and Ownership

Expected source files:

- `src/navigation/code_context.rs` (new)
- `src/navigation/mod.rs` (module export)
- `src/mcp/evidence_view.rs`
- `src/mcp/tools.rs` tests only unless schema descriptions need small updates
- `tests/mcp/smoke_test.rs` for existing `0.3` view-schema pins

Expected tests:

- classifier unit tests in `src/navigation/code_context.rs`;
- MCP/EvidenceView tests in `src/mcp/tools.rs` or `src/mcp/evidence_view.rs`;
- no Tier-A baseline changes.

Avoid:

- `src/repo_loader.rs` for this slice;
- `src/navigation/queries.rs`;
- `src/navigation/types.rs`, unless reviewers decide canonical schema must carry labels now;
- `src/cpg_cache.rs`;
- `src/cpg/`, `src/call_graph.rs`, resolver and language policy files.

## 9. Implementation Plan

### Task 1: Add classifier module

1. Add `src/navigation/code_context.rs`.
2. Export it from `src/navigation/mod.rs`.
3. Implement `classify_file(path: &str) -> CodeContext`.
4. Add table-driven tests for:
   - `tests/foo.py`
   - `src/foo_test.go`
   - `test_bar.py`
   - `foo_test.py`
   - `foo.test.ts`
   - `foo.spec.tsx`
   - `src/FooTest.java`
   - `src/FooIT.java`
   - `src/lib.rs`
   - `src/foo_test.rs`
   - `pkg/testdata/input.go`
   - `docs/superpowers/specs/foo.rs` remains production
   - `src/fixtures/example.rs` remains production without a test filename or authoritative test component
   - `pkg/spec/foo.spec.ts` is test and reports both ancillary path and filename reasons
   - deterministic multiple reasons for `tests/foo_test.go`

### Task 2: Add EvidenceView code labels

1. Add `ViewCodeContext` and `CodeCounts` structs.
2. Add `code` to `ViewItem`, `ViewGroup`, and `ViewNode`.
3. Add `production` and `test` counts to `ViewSummary`.
4. Add `indexing_policy` to `ViewMeta`.
5. Bump `VIEW_SCHEMA_VERSION` to `0.4`.
6. Add `_meta["prism/view_indexing_policy"]`.
7. Compute labels by calling `classify_file(&item.location.file)` and `classify_file(&node.location.file)`.

### Task 3: Render markdown labels

1. Add summary line: `code: production=N test=M`.
2. Add `production=N test=M` to group headings when non-zero.
3. Add `code=production` or `code=test` to item lines.
4. Add `code=production` or `code=test` to graph node lines.
5. Keep output compact; do not render reasons in markdown by default.

### Task 4: Tests

Add focused tests proving:

1. `agent_json` item includes `code.role`, `code.is_test`, and deterministic `code.reasons`.
2. `agent_json` summary and group code counts distinguish production and test callers.
3. `agent_markdown` includes the summary/group/item code labels.
4. `nav_repo_map` / graph agent JSON nodes include code labels.
5. `nav_repo_map` / graph agent JSON summary code counts equal visible graph-node roles.
6. `agent_markdown` graph node lines include code labels when graph nodes are rendered.
7. Fully clipped agent views still set `_meta["prism/view_schema_version"] = "0.4"`,
   `_meta["prism/view_indexing_policy"] = "code_role_v1"`, `_meta["prism/view_profile"]`,
   and `_meta["prism/content_text_format"]`.
8. Existing tests pinned to view schema `0.3` are intentionally updated to `0.4`, and agent-json structural
   assertions include the now-always-present `code` field.
9. Canonical structured content remains unchanged for an agent view.
10. Default `canonical_json` output does not include view-only `code` fields.
11. `max_view_bytes` clipping remains bounded after adding labels.

## 10. Validation

Run:

```bash
cargo build --release
cargo fmt --check
git diff --check
cargo test --features mcp --lib navigation::code_context -- --nocapture
cargo test --features mcp --lib mcp::tools -- --nocapture
cargo test --features mcp --lib mcp::evidence_view -- --nocapture
cargo test --features mcp --test mcp -- --nocapture
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

This slice deliberately places the reusable classifier in `src/navigation/`, so the repo accuracy-harness rule
requires a release rebuild followed immediately by Tier-A matrix before commit and Tier-A quick before review.
Use `--allow-stale-sut` only with the immediate preceding rebuild in this same worktree. No rebaseline is expected;
paste any regressions or flip candidates into the PR description.

## 11. Review Questions

Ask A2A reviewers to focus on:

1. Is EvidenceView-only labeling enough for Slice 1, or should canonical `EvidenceItem` carry the label now?
2. Are the path heuristics too broad, especially `fixtures` and `spec/specs`?
3. Should graph nodes use file-level labels even when a graph node has a production symbol inside a test file or
   vice versa?
4. Are summary counts clearly defined under clipping and graph-vs-item views?
5. Does the schema need `role: "mixed"` at item level? Proposed answer: no; items are file-level and therefore
   production or test only. Groups can be mixed through counts.

## 12. Future Slices

### Slice 2: Ignore policy

- Respect `.gitignore`.
- Add `.prismignore`.
- Preserve explicit skipped-file summaries in repo-map/orientation views.
- Decide whether ignored files are absent from freshness tracking or reported as skipped.

### Slice 3: Query scope filtering

- Add `scope: "all" | "prod" | "test"` to selected nav MCP inputs.
- Default remains `all`.
- Filtering should run after canonical query construction at first, so resolver behavior is unchanged.
- Later optimize by filtering during query expansion only if needed.

### Slice 4: Generated/vendor labels

- Add `generated` and `vendor` labels once the production/test contract is stable.
- Consider role as an enum plus flags, not a single mutually exclusive enum, because generated test fixtures can
  exist.

### Slice 5: Span-level labels

- Rust `#[cfg(test)]` modules/functions inside production files.
- Language-specific test decorators/annotations where useful.
- This requires AST/scope context and should be its own reviewed slice.
