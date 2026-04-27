# Phase 1.5 Follow-up - CFG-aware paired-check direction

## 1. Goal

Fix the remaining PathTraversal sanitizer correctness gap before Phase 2: `filepath.Clean` / `filepath.Rel` paired checks currently suppress by textual co-occurrence of `strings.HasPrefix` anywhere in the function body. That suppresses real bugs when the guard direction is inverted, for example:

```go
rel, _ := filepath.Rel("/safe", name)
if strings.HasPrefix(rel, "..") {
    _, _ = os.ReadFile(rel) // bug: current textual heuristic suppresses this
}
```

The desired behavior is:

- Suppress when the sink is control-flow reachable only through the safe side of the check.
- Do not suppress when the sink is on the reject/bug side of the check.
- Do not suppress when `strings.HasPrefix` checks an unrelated variable.
- Preserve the current 10/10 sanitizer-suite suppression rate for correctly guarded fixtures.

## 2. Important design correction

Do not implement this as a simple replacement for `paired_check_satisfied(function_text, "strings.HasPrefix")`.

Current cleansing is marked at `FlowPath` level by `apply_cleansers -> function_body_cleansed_for`, but guard-direction correctness is sink-specific. A function can contain one validated path use and one unvalidated path sink. A category-wide `path.cleansed_for.insert(PathTraversal)` can suppress both.

Recommended change: make Go PathTraversal sanitizer suppression sink-time:

```rust
path_cleansed_for_sink(parsed, path, sink_line, category, cpg) -> bool
```

For `SanitizerCategory::PathTraversal` in Go, this helper should perform CFG/AST guard validation for the specific sink line. For other categories, or when CFG is unavailable, keep the existing `path.cleansed_for.contains(&category)` fallback.

## 3. Proposed mechanism

### 3.1 Extract sanitizer bindings

Within the enclosing Go function for `sink_line`, collect path sanitizer result bindings:

```rust
struct PathSanitizerBinding {
    kind: PathSanitizerKind, // Clean or Rel
    result_var: String,
    call_line: usize,
}
```

Supported MVP shapes:

- `cleaned := filepath.Clean(name)`
- `cleaned = filepath.Clean(name)`
- `rel, err := filepath.Rel(base, name)`
- `rel, err = filepath.Rel(base, name)`

For `filepath.Rel`, the result var is the first LHS identifier. The error var is useful for future checks but not required for the first pass.

### 3.2 Couple binding to the sink

Only consider a sanitizer binding for a sink if the sanitized result var is actually used at the sink line.

Primary path-aware check:

```rust
path.edges.any(|e| {
    e.to.file == parsed.path &&
    e.to.line == sink_line &&
    e.to.var_name() == binding.result_var
})
```

No-`FlowPath` fallback, used only by the source==sink loop, can scan structured sink calls on `sink_line` and collect identifiers inside the pattern's `tainted_arg_indices`. If the binding result var is not among those identifiers, do not cleanse.

This variable coupling prevents an unrelated `strings.HasPrefix(other, ...)` or an unrelated sanitized path from suppressing the sink.

### 3.3 Classify guard direction from AST

Find `if_statement` nodes in the same function whose condition contains `strings.HasPrefix(binding.result_var, ...)`.

Classify supported safe guard shapes:

For `filepath.Clean`:

- Reject-on-fail: `if !strings.HasPrefix(cleaned, allowed) { return }`, sink after the `if`.
- Allow-on-pass: `if strings.HasPrefix(cleaned, allowed) { sink }`, sink inside the consequence.

For `filepath.Rel`:

- Reject-on-bad-prefix: `if strings.HasPrefix(rel, "..") { return }`, sink after the `if`.
- Allow-on-not-bad-prefix: `if !strings.HasPrefix(rel, "..") { sink }`, sink inside the consequence.

For boolean combinations, support only the safe MVP shapes:

- Bare `strings.HasPrefix(...)`.
- Unary negation, `!strings.HasPrefix(...)`.
- Pure OR-disjunctions where `strings.HasPrefix(...)` appears positively in a
  reject branch whose consequence terminates.

Support the common fixture shape:

```go
if err != nil || strings.HasPrefix(rel, "..") {
    return
}
```

The HasPrefix term is still a reject-on-bad-prefix term because it appears positively inside an OR whose consequence terminates.

Do not treat AND-conjunctions as safe in the MVP. This is not sufficient:

```go
if err != nil && strings.HasPrefix(rel, "..") {
    return
}
```

The consequence only fires when both conditions hold, so a bad-prefix `rel` with
`err == nil` reaches the sink. AND shapes, mixed AND/OR expressions, nested
boolean expressions outside the pure-OR form, and any other ambiguous boolean
shape must return `false` and let the sink fire.

MVP termination recognition should be conservative: require the relevant consequence branch to end in `return`. Do not infer safety from non-returning helper calls, panics, logging, or comments in the first pass.

### 3.4 Use CFG for branch-specific reachability

The existing CPG CFG edges are unlabeled, so do not rely on edge order to mean true/false. Use AST to identify branch roles, then use CFG to prove reachability from the relevant branch entry.

Useful existing APIs:

- `CodePropertyGraph::statement_at(file, line)`
- `CodePropertyGraph::cfg_successors(idx)`
- `CodePropertyGraph::reachable_forward(idx, |e| matches!(e, CpgEdge::ControlFlow))`
- `CpgNode::line()`

For reject-on-fail / reject-on-bad-prefix:

1. AST identifies the reject branch as the `if` consequence.
2. The consequence must terminate with `return`.
3. The sink must be CFG-reachable from the safe successor, usually the statement after the `if`.
4. The sink must not be CFG-reachable from the reject branch entry.

For allow-on-pass / allow-on-not-bad-prefix:

1. AST identifies the consequence as the safe branch.
2. The sink must be inside the consequence range or CFG-reachable from the consequence entry before leaving the branch.
3. If there is an `else`, do not treat it as safe unless it terminates or is unreachable from the sink.

If any proof step is ambiguous, return `false` and let the sink fire. This is intentionally false-positive-biased.

Pre-flight result: verified on 2026-04-26 with a temporary focused Go unit test
for `if cond { return }; sink()`. `cargo test
test_cpg_cfg_go_if_return_does_not_reach_fallthrough --lib` passed: the
`return_statement` had no CFG successor to the sink line, while the `if` node had
successors to both the return branch and fallthrough sink. The temporary test was
removed after verification.

## 4. Integration points

### 4.1 Taint sink suppression

Current forward-flow suppression uses:

```rust
path.cleansed_for.contains(&p.category)
```

Replace category checks in Go sink evaluation with:

```rust
path_cleansed_for_sink(parsed, path, edge.to.line, p.category, &ctx.cpg)
```

Affected sites:

- Structured sink suppression in the forward-flow loop.
- `cleansed_structured_sink_call_ranges`, because flat fallback suppression inside cleansed structured calls must use the same sink-specific answer.
- Source==sink fallback branches currently calling `function_body_cleansed_for`.

Sketch the signature changes explicitly so this does not become a hidden global:

```rust
fn cleansed_structured_sink_call_ranges(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)>

fn push_cleansed_structured_sink_range(
    ranges: &mut Vec<(usize, usize)>,
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    call: &Node<'_>,
    actual: &str,
    sink_pat: &'static SinkPattern,
    path: &FlowPath,
) -> bool
```

Walking the same line's call set once per pattern is acceptable for Phase 1.5
fixtures; tree-walk caching remains out of scope.

### 4.2 Preserve legacy fallback outside Go PathTraversal

Keep `apply_cleansers` and `FlowPath.cleansed_for` for now. For this follow-up:

- Go `PathTraversal`: sink-time CFG-aware helper is authoritative.
- Other categories: existing `cleansed_for` behavior remains.
- No CFG available: fail closed for Go `PathTraversal`. Do not fall back to the textual `paired_check_satisfied` heuristic, because that is the false-negative class this follow-up is removing.
- Leave `apply_cleansers` and `FlowPath.cleansed_for` intact for compatibility. The sink-time helper is authoritative for Go `PathTraversal` suppression; auditing/removing category-wide insertion can be a follow-up if a real downstream consumer requires it.

### 4.3 Update docs

Update:

- `src/sanitizers/mod.rs` and `src/sanitizers/path.rs` comments.
- `STATUS-prism-cwe-phase1.md` item 3.
- `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` section 3.8.

## 5. Test plan

Add to `tests/algo/taxonomy/sanitizers_test.rs`.

Positive suppression tests that must remain suppressed:

- Existing `test_path_clean_with_hasprefix_suppresses`.
- Existing `test_path_rel_with_hasprefix_suppresses`.
- Add `test_path_clean_positive_guard_branch_suppresses`:
  `if strings.HasPrefix(cleaned, "/safe") { os.ReadFile(cleaned) }`.
- Add `test_path_rel_negative_prefix_guard_branch_suppresses`:
  `if !strings.HasPrefix(rel, "..") { os.ReadFile(rel) }`.

Negative tests that must now fire:

- `test_path_clean_inverted_guard_does_not_suppress`:
  `if !strings.HasPrefix(cleaned, "/safe") { os.ReadFile(cleaned) }`.
- `test_path_rel_inverted_guard_does_not_suppress`:
  `if strings.HasPrefix(rel, "..") { os.ReadFile(rel) }`.
- `test_path_clean_unrelated_hasprefix_does_not_suppress`:
  `cleaned := filepath.Clean(name); if !strings.HasPrefix(other, "/safe") { return }; os.ReadFile(cleaned)`.
- `test_path_clean_guard_after_sink_does_not_suppress`:
  `os.ReadFile(cleaned)` appears before the guard.

`test_path_clean_unrelated_hasprefix_does_not_suppress` is the most diagnostic
test for the variable-coupling logic. Today's textual heuristic suppresses it;
if variable coupling silently regresses, this test should fail even if the
directional guard tests still pass.

Regression tests that must stay green:

- `test_path_clean_same_line_unrelated_flat_sink_still_fires`.
- `test_category_isolation_path_cleanse_does_not_suppress_oscommand`.
- `tests/integration/cwe_phase1_suppression_test.rs` remains 10/10 sanitized and 10/10 unsanitized.

## 6. Implementation tasks

1. Add `CodePropertyGraph` threading into sink-time cleanser checks.
   - Change helper signatures instead of using globals.
   - Keep non-Go / non-PathTraversal fallback unchanged.

2. Add sanitizer binding extraction helpers in `taint.rs`.
   - `collect_path_sanitizer_bindings(parsed, func_node)`.
   - `assignment_lhs_identifiers_for_call(parsed, call_node)`.
   - Keep helpers private until the shape stabilizes.

3. Add HasPrefix guard extraction/classification.
   - `find_hasprefix_guards_for_var(parsed, func_node, var_name)`.
   - `guard_safely_controls_sink(parsed, cpg, guard, binding_kind, sink_line)`.
   - Support only the safe shapes listed in section 3.3.

4. Replace PathTraversal suppression checks with `path_cleansed_for_sink`.
   - Forward-flow structured suppression.
   - Cleansed structured call ranges for flat fallback.
   - Source==sink fallback.

5. Add tests.
   - Start with negative inverted-direction tests before implementation to confirm the current failure.
   - Then add positive branch-form tests.

6. Update docs/status.
   - Mark CFG-aware paired-check direction closed only after integration and suppression suite pass.

Implementation branch and PR shape:

- Branch: `claude/phase15-cfg-aware-paired-check`.
- Open a two-commit PR and pause for review before push/merge.

- Commit 1: engine helper + sink-time integration + regression tests.
- Commit 2: docs/status updates.

## 7. Out of scope

- Cross-function sanitizer validation.
- Non-Go sanitizer CFG direction.
- Non-returning helper recognition (`abort`, `panic`, custom error helpers).
- Full boolean SAT over arbitrary guard expressions.
- Tree-walk caching. Do this later only if Phase 2 profiling shows registry scans are expensive.

## 8. Decisions From Re-Review

1. Go `PathTraversal` suppressions fail closed when CFG is unavailable.
2. MVP branch set is return-terminating reject branch plus sink-inside-safe-branch.
3. `panic` stays out of first-pass termination recognition to avoid accidental suppression.
4. Leave `apply_cleansers` alone for now; sink-time validation is authoritative for Go `PathTraversal`.

## 9. Acceptance criteria

- Inverted Clean and Rel guards now produce `taint_sink`.
- Correct Clean and Rel guards still suppress.
- Unrelated `HasPrefix` checks no longer suppress.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets` passes.
- `cargo test --test algo_taxonomy_sanitizers -- --nocapture` passes.
- `cargo test --test integration_cwe_phase1_suppression -- --nocapture` reports 10/10 sanitized and 10/10 unsanitized.
- `cargo test --test algo_taint_sink_lang` passes.
- Full `cargo test` passes before merge.
