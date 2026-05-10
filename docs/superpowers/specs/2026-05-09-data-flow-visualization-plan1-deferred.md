# Plan 1 — Deferred follow-ups

**Status:** Recorded 2026-05-09 after final code review of `feat/data-flow-visualization`. These items were surfaced by the whole-branch review (review log lives in the PR thread; the reviewer's "Important" and "Minor" sections). They are deferred — not silently dropped — because they are below the bar for blocking merge and the cost of doing them now is higher than the cost of leaving them for a future implementer who knows the context.

This document exists so future implementors don't have to re-discover any of these issues. Each entry includes priority, the rationale for deferral, what it impacts in production, and the concrete fix sketch.

**Done in the same round (for cross-reference):**
- Critical: `finalize_diagrams` wired into `main.rs::run_algorithm` (commit `d6555b5`)
- Critical: `ReviewOutput` + `MultiReviewOutput` carry `diagrams` and `diagram_warnings` (commit `8c68b27`)
- Critical: `cargo fmt` drift cleared, `escape_label` dead code removed (commit `0566a90`)
- Important: end-to-end CLI test asserting `--format json` includes diagrams for Vertical/Echo/Circular and non-empty mermaid for Taint
- Important: `truncate_to_cap` cycle-shape special case (preserve cycle back-edge endpoints)
- Important: drop early-out gates in per-algorithm diagram tests
- Important: VerticalSlice edge deduplication (matches Echo's `seen_edges` pattern)
- Important: CLI validation tests strengthened to exercise flag values, not `--help` short-circuit

The items below are everything else.

---

## 1. `NodeKind::Callee` is defined but never used by Plan 1

**Priority:** Low.

**Why deferred:** No Plan-1 algorithm needed it. Echo uses `Caller` + `Origin`, Vertical uses `Step`, Circular uses `Step`, Taint uses `Source`/`Sink`/`Step`. Adding a usage requires deciding which algorithm should distinguish downstream functions as `Callee`-kind, which is a Plan-2 design conversation.

**Impact:** Every rendered Mermaid diagram embeds the full `classDef` block including `classDef callee fill:#fff3c4;`. Mermaid silently ignores unused classDefs, so there's no visual artifact. The "noise" is only ~50 bytes per diagram in the JSON `mermaid` field. No correctness impact.

**Context for future implementor:**
- Plan 2's `Membrane` slice (cross-file callers of changed APIs) is the obvious first user — it should distinguish caller-side and callee-side nodes.
- Plan 2's refactored `Vertical` could use `Caller`/`Callee` instead of bare `Step` to convey direction.
- Once any algorithm pushes a `Callee`-kind node, the classDef earns its keep.
- If by end of Plan 2 it's still unused, consider either (a) emit classDefs lazily (only for kinds present in `g.nodes`), or (b) remove the variant.

**Concrete fix when a use case lands:**
```rust
// In e.g. membrane_slice.rs:
let callee_node = GraphNode {
    /* ... */
    kind: NodeKind::Callee,
    /* ... */
};
```

---

## 2. `source_line_text` in `taint.rs` does not truncate before label construction

**Priority:** Low (informational warning noise, not a bug).

**Why deferred:** The renderer's `escape_label_inner` already truncates labels at 80 chars and emits a `LabelTruncated` warning. So long source lines work correctly — they just generate one informational warning per long-labeled node. The bug-class predicate (`is_bug()`) excludes `LabelTruncated`, so this doesn't fail `--strict-diagrams`.

**Impact:** Taint diagrams over real C/Rust/Go code (where 100+ char source lines are common) will have many `LabelTruncated` warnings in the `## Diagnostics` section of `--format mermaid` output. Visually noisy. Doesn't affect the rendered diagram itself — labels are still truncated correctly with `…`.

**Context for future implementor:**
- The label format is `format!("{}:{}\n{}", file, line, text)`. The `file:line` prefix can be ~30 chars on long paths, leaving ~45 chars for source text before truncation kicks in.
- The fix is preventive: cap `text` at e.g. 60 chars in `source_line_text` itself, so the combined label fits under 80 without triggering the warning.
- Same pattern repeats in the other Plan-1 algorithms (`vertical_slice.rs`, `echo_slice.rs`, `circular_slice.rs`) — they all use `format!("{}:{}\n{}", file, line, text)`. A shared helper in `src/output/mermaid.rs` would consolidate the convention.

**Concrete fix:**
```rust
// In a shared helper, e.g. src/output/mermaid.rs:
pub(crate) fn diagram_label(file: &str, line: usize, text: &str) -> String {
    const MAX_TEXT: usize = 60;
    let trimmed: String = text.chars().take(MAX_TEXT).collect();
    let suffix = if text.chars().count() > MAX_TEXT { "…" } else { "" };
    format!("{}:{}\n{}{}", file, line, trimmed, suffix)
}
```

Then refactor each Plan-1 algorithm to call `diagram_label(...)` instead of `format!(...)` directly.

---

## 3. `safe_node_id` does not guarantee cross-file uniqueness for files with same alphanumeric-collapsed name

**Priority:** Low (latent collision risk).

**Why deferred:** Collision requires two file paths that differ only in non-alphanumeric chars — e.g., `src/foo.c` and `src_foo.c` both collapse to slug `src_foo_c`, so on the same line number both produce node id `n_src_foo_c_42`. This is unusual in practice (real codebases rarely have parallel files like this). The renderer's `DuplicateNodeId` validation pass catches the collision, emits a bug-class warning, and drops the duplicate node — so the failure mode is "one node missing from diagram + one warning logged" rather than a crash.

**Impact:** When a collision does happen: the diagram loses one node (the second occurrence is dropped during sanitization), `--strict-diagrams` exits non-zero, and a `DuplicateNodeId` warning fires with a misleading message that suggests the algorithm has a bug when it's actually a `safe_node_id` collision.

**Context for future implementor:**
- The fix has two flavors:
  - **Cheap:** add a doc comment to `safe_node_id` warning about the collision class. Don't change behavior. Document that callers should not rely on `safe_node_id` for cross-file uniqueness.
  - **Robust:** include a hash of the original file path in the slug, or use a base32-style escape that preserves uniqueness (`/` → `_2f_`, `.` → `_2e_`). Trade-off: longer node ids in rendered Mermaid output.
- If Plan 2 starts seeing real collisions in fixtures, lean toward the robust fix.

**Concrete fix (robust):**
```rust
pub(crate) fn safe_node_id(file: &str, line: usize) -> String {
    let slug: String = file
        .chars()
        .map(|c| match c {
            c if c.is_ascii_alphanumeric() => c.to_string(),
            c => format!("_{:02x}_", c as u32),  // unique-per-char escape
        })
        .collect();
    format!("n_{}_{}", slug, line)
}
```

This keeps the result Mermaid-safe (lowercase alphanumeric + underscores) while making the slug a bijection with the file path.

---

## 4. `format_mermaid_report`'s "Finding N" enumerate semantics

**Priority:** Trivial (documentation only).

**Why deferred:** The current behavior is correct: when an algorithm produces 10 findings of which only 3 carry diagrams, the report shows "Finding 1 / Finding 4 / Finding 7" using the original index in `findings`. This is what readers expect when correlating against the JSON output. But the code uses `r.findings.iter().enumerate().filter(...)` and a reviewer reading it might assume the index is into `findings_with_diagrams` (i.e., 1/2/3).

**Impact:** None on behavior. Risk of confusion during future code review of the formatter.

**Context for future implementor:** Add a one-line comment above the `enumerate().filter()` call:

```rust
// `enumerate()` runs over the full `findings` list before `filter` so the
// printed index corresponds to the position in the original findings array
// (matching what a JSON consumer would see).
let findings_with_diagrams: Vec<_> = r
    .findings
    .iter()
    .enumerate()
    .filter(|(_, f)| !f.diagrams.is_empty())
    .collect();
```

---

## 5. `escape_label_inner` called twice per node in `render()`

**Priority:** Trivial (micro-perf, no correctness impact).

**Why deferred:** `render()` calls `escape_label_inner` once during the label-truncation detection pass (to emit `LabelTruncated` warnings) and the shape templates call it again during rendering. Double-walks the label string. Costs ~10ns per node on typical labels. Negligible compared to the algorithm work.

**Impact:** None measurable. The size cap (40 nodes default) bounds the wasted work to at most 40 extra escape passes per diagram.

**Context for future implementor:** The fix has two flavors:

- **Refactor templates** to return both the rendered string and a `Vec<truncation_flag>`, lifting the truncation collection into the template loop. Lots of API churn for trivial savings.
- **Compute truncation in the size-cap pass** (which already walks every node) and stash the flag on the node somehow (extra field? side table?). Adds state. Not worth it.

The cleanest answer: leave it. If profiling ever shows label escaping in the hot path, revisit.

---

## 6. Top-level `MultiSliceResult.diagram_warnings: vec![]` in main.rs is cosmetically misleading

**Priority:** Trivial (cosmetic).

**Why deferred:** The CLI initializes `MultiSliceResult.diagram_warnings: vec![]` even when per-result warnings exist. `aggregate_diagram_warnings()` walks all `SliceResult.diagram_warnings` and collects them, so no information is lost. But a reader inspecting the JSON output might see the empty top-level field and assume no diagram warnings fired — when in fact they did, just nested inside per-result blocks.

**Impact:** Possible confusion for downstream consumers reading raw JSON. No behavioral impact.

**Context for future implementor:** Two ways to fix:

- **Populate top-level on serialization:** in `main.rs`, after constructing the `MultiSliceResult`, explicitly set `multi.diagram_warnings = multi.aggregate_diagram_warnings()`. Then the JSON shows the same warnings twice (top-level + per-result), but the top-level is no longer misleadingly empty.
- **Drop the top-level field entirely:** remove `diagram_warnings` from `MultiSliceResult`. Consumers that want aggregation call `aggregate_diagram_warnings()` themselves. Smaller schema, fewer surprises.

The cleaner answer is the second one (drop the field), but it's a schema change that affects any current consumer. Defer until Plan 2 or a future schema-versioning PR makes the change cheap.

---

## How to consume this list

When you start Plan 2 (or any future cleanup pass), scan this file and decide which items to fold into your work:

- Items 1, 2 are most likely to come up naturally during Plan 2 algorithm work — handle them when you touch the relevant code.
- Items 3, 4, 6 are independent cleanup; do them in a "hygiene" PR if you're feeling generous, or wait until they bite.
- Item 5 stays deferred unless profiling justifies the work.

Mark items as resolved by removing them from this file in the PR that fixes them.
