# Phase 1.5 (item #1) — Per-arg DFG Implementation Plan

> **Status:** Approved 2026-04-26 (post 4-iteration external review). Pairs with `docs/superpowers/specs/2026-04-26-phase15-per-arg-dfg.md`.
>
> **For agentic workers:** plan is small enough (~½ day, 2 commits) to execute in-session without sub-agent dispatch. The execution rhythm is: branch off main → implement Commit 1 → verify → implement Commit 2 → verify → push + PR.

**Goal:** Make the line-matching engine consult `SinkPattern.tainted_arg_indices` against per-argument taint state at the call site. Eliminates the variable-bound-to-literal false-positive shape (eval-team flagged); retires Path C scaffolding.

**Tech stack:** Rust 2021. Tree-sitter Go grammar (already integrated). Existing `FlowPath` / `FlowEdge` / `VarLocation` types in `src/data_flow.rs` carry the per-variable provenance the algorithm needs.

**Source-of-truth references** (read these before starting):
- Design spec: `docs/superpowers/specs/2026-04-26-phase15-per-arg-dfg.md`.
- Phase 1 spec: `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §3.1–§3.4 for sink registry context.
- `src/algorithms/taint.rs` — `SinkMatchOutcome` enum (~line 842), `line_matches_structured_sink` (~line 877), `go_sink_outcome` (~line 934), the two call sites at the forward-flow loop (~line 1140) and source==sink loop (~line 1227).
- `src/data_flow.rs` — `FlowPath`, `FlowEdge`, `VarLocation` shapes.
- PR #73 review feedback context: P1 (pwsh fallback) + P2 (same-line unrelated sink) regression guards already in tests/algo/taxonomy/taint_sink_lang_test.rs.

---

## File Structure

### Modified in Commit 1 — Per-arg taint resolution
| File | Change |
|---|---|
| `src/algorithms/taint.rs` | Add `use crate::data_flow::FlowPath;`. New `arg_is_tainted_in_path` + `arg_node_taints_match` helpers (with `e.to.file == parsed.path` file-scoping guard). New `find_sink_with_inline_framework_source` (returns `Option<&'static SinkPattern>` after exhaustive scan over (call, sink_pat) combinations on the line; reuses the existing function-scoped `collect_request_param_names` + `framework_request_types`; returns None when `request_param_names.is_empty()` to mirror the `detect_framework_sources` empty-function-scope guard at ~L794) + `subtree_has_call_in` helpers for source==sink mixed-line fallback. `line_matches_structured_sink` and `go_sink_outcome` take `Option<&FlowPath>`. Forward-flow loop passes `Some(path)`. Source==sink loop: when `originating.is_empty()`, pass `None` for primary fallback (preserves current source==sink behavior); when non-empty, iterate originating paths combining Match-detection and per-path cleansing in one loop, then attempt secondary inline-source fallback via `find_sink_with_inline_framework_source` to catch mixed same-line shapes (e.g. `other := c.Query(); c.File(c.Param("f")); fmt.Println(other)` all on one line). |
| `tests/algo/taxonomy/taint_sink_lang_test.rs` | 6 new regression tests (variable-bound-to-literal on c.File, os.Rename smoke for multi-index, taint-near-but-not-into-call on c.File, complex-expression conservative, pure source==sink primary fallback, mixed same-line source==sink secondary fallback). |

### Modified in Commit 2 — Path C scaffolding retirement
| File | Change |
|---|---|
| `src/algorithms/taint.rs` | Remove `check_command_taintable_binary`, `check_commandcontext_taintable_binary`, `check_arg_is_non_literal_at` helpers. Restore `semantic_check: None` on the two tainted-binary `SinkPattern` entries. |
| `src/frameworks/mod.rs` | Update `tainted_arg_indices` doc-comment (drop "NOT consulted" language, describe any-tainted semantics). |
| `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` | Update §3.2: remove Path C interim language; replace with per-arg DFG description. |

### NOT changed
- `Cargo.toml` (no new test files — extending `taint_sink_lang_test.rs`).
- `tests/integration/coverage_test.rs` (no new test files registered).
- `src/data_flow.rs` (existing `FlowPath`/`FlowEdge` shapes are sufficient).
- `src/sanitizers/` (cleansing logic unaffected).
- Any fixture file in `tests/fixtures/sanitizer-suite-go/` (sanitizer suppression rate unchanged).

---

## Pre-flight (run once before Task 1)

- [ ] **P1: Confirm branch + base**

```bash
cd /Users/wesleyjinks/code/slicing
git checkout main
git pull
git log -1 --oneline
# expect: a recent commit on main; verify the PR #73 merge is in history (git log --oneline | grep "583d114\|6703735\|e2042fc" — those should appear)
git checkout -b claude/phase15-per-arg-dfg
git branch --show-current
# expect: claude/phase15-per-arg-dfg
```

- [ ] **P2: Confirm baseline tests pass**

```bash
cargo test 2>&1 | grep "test result:" | awk '{sum+=$4} END {print sum}'
# expect: 1438 (matches PR #73's final state; includes the P1/P2 regression guards)
cargo fmt --check
# expect: no output
```

- [ ] **P3: Read design note + relevant `taint.rs` regions**

```bash
sed -n '/^### 2\.1/,/^### 2\.4/p' docs/superpowers/specs/2026-04-26-phase15-per-arg-dfg.md
# Reads the algorithm + path-aware dispatch section.

grep -n "fn line_matches_structured_sink\|fn go_sink_outcome\|enum SinkMatchOutcome" src/algorithms/taint.rs
# Note line numbers for the three definitions.
```

- [ ] **P4: Note the two call sites**

```bash
grep -n "go_sink_outcome(parsed" src/algorithms/taint.rs
# Expect ~2 hits: forward-flow loop + source==sink loop.
```

---

# Commit 1 — Per-arg taint resolution

**Sub-task: this commit lands the per-arg DFG mechanism. Tasks 1–7. Verify `cargo test && cargo fmt --check` green before committing. Commit at end of Task 7 with the verbatim message in 7.3.**

## Task 1: Add `arg_is_tainted_in_path` helper in `taint.rs`

**File:** `src/algorithms/taint.rs`

- [ ] **Step 1.1: Add the `FlowPath` import**

`taint.rs` currently references `FlowPath` only via the fully-qualified `crate::data_flow::FlowPath` path (e.g., `apply_cleansers`'s signature). The new helpers use `FlowPath` as a bare name in their signatures, which requires an explicit `use` declaration.

Add to the top imports of `src/algorithms/taint.rs` (alphabetically among the existing `use crate::*` lines):

```rust
use crate::data_flow::FlowPath;
```

After this, the existing `apply_cleansers` signature can either be left as-is (`crate::data_flow::FlowPath`) or simplified to bare `FlowPath` — leave alone unless touching the line for another reason.

- [ ] **Step 1.2: Locate insertion point for the new helpers**

Place the new helper just above `line_matches_structured_sink` (~line 877). The helper depends only on `FlowPath`, `FlowEdge`, tree-sitter `Node`, and `ParsedFile`.

- [ ] **Step 1.3: Implement the helper**

```rust
/// Returns true if argument `arg_idx` of the call expression is tainted along `path`.
///
/// Resolution rules:
/// - Literal arg (string, int, bool, nil) → always false (literals can't be tainted).
/// - Bare identifier → check if any `FlowEdge` in `path` has this identifier as a `to`
///   location matching `parsed.path` (file scoping prevents cross-file collisions),
///   `call_line`, and `var_name()`. Without the file-scoping guard, an interprocedural
///   FlowEdge ending in another file at the same line/name could falsely register
///   as taint here.
/// - Complex expression (call, selector, binary, ...) → conservative recurse into
///   descendants; if ANY identifier descendant is tainted on the path (with file
///   scoping), the arg is considered tainted. Phase 1.5 keeps this conservative;
///   tightening (e.g., only considering specific positions in a selector chain) is
///   Phase 2+.
///
/// Returns false if the call has fewer than `arg_idx + 1` arguments.
fn arg_is_tainted_in_path(
    parsed: &ParsedFile,
    call: &tree_sitter::Node<'_>,
    arg_idx: usize,
    path: &FlowPath,
) -> bool {
    let arguments = match call.child_by_field_name("arguments") {
        Some(n) => n,
        None => return false,
    };
    let mut cursor = arguments.walk();
    let mut idx = 0usize;
    let mut target_arg: Option<tree_sitter::Node<'_>> = None;
    for child in arguments.named_children(&mut cursor) {
        if idx == arg_idx {
            target_arg = Some(child);
            break;
        }
        idx += 1;
    }
    let arg_node = match target_arg {
        Some(n) => n,
        None => return false,
    };
    let call_line = call.start_position().row + 1;
    arg_node_taints_match(parsed, &arg_node, call_line, path)
}

/// Walk `arg_node` and any descendants for identifiers that are tainted on `path` at
/// `call_line` in `parsed`'s file. Returns true on first hit.
fn arg_node_taints_match(
    parsed: &ParsedFile,
    arg_node: &tree_sitter::Node<'_>,
    call_line: usize,
    path: &FlowPath,
) -> bool {
    match arg_node.kind() {
        // Literal kinds — definitely not tainted.
        "interpreted_string_literal" | "raw_string_literal" | "rune_literal"
        | "int_literal" | "float_literal" | "imaginary_literal"
        | "true" | "false" | "nil" => false,

        // Bare identifier — direct check with file scoping.
        "identifier" => {
            let name = parsed.node_text(arg_node);
            path.edges.iter().any(|e| {
                e.to.file == parsed.path
                    && e.to.line == call_line
                    && e.to.var_name() == name
            })
        }

        // Composite expression — recurse into descendants. Conservative: any
        // tainted identifier within counts.
        _ => {
            let mut cursor = arg_node.walk();
            for child in arg_node.named_children(&mut cursor) {
                if arg_node_taints_match(parsed, &child, call_line, path) {
                    return true;
                }
            }
            false
        }
    }
}
```

Note: `parsed.node_text(node)` exists on `ParsedFile` (used elsewhere in `taint.rs`). Verify by `grep -n "node_text" src/algorithms/taint.rs | head -5` if unsure. `parsed.path` is the `pub path: String` field on `ParsedFile` (`src/ast.rs:46`).

- [ ] **Step 1.4: Compile-check**

```bash
cargo check 2>&1 | tail -10
# Expected: errors about unused `arg_is_tainted_in_path` (we use it in Task 2). No other errors.
```

---

## Task 2: Thread `&FlowPath` through `line_matches_structured_sink`; consult per-arg taint

**File:** `src/algorithms/taint.rs`

- [ ] **Step 2.1: Update signature**

Change:
```rust
fn line_matches_structured_sink(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
) -> SinkMatchOutcome {
```

to:
```rust
fn line_matches_structured_sink(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
```

`Option<&FlowPath>` (not `&FlowPath`) is required so the source==sink loop can pass `None` for the no-originating-path branch — see Task 5 and design note §2.3 for rationale. Forward-flow callers always pass `Some(&path)`.

- [ ] **Step 2.2: Add per-arg taint check inside the call-walk loop**

Find the loop body inside `line_matches_structured_sink` after the `semantic_check` block. The current logic returns `SinkMatchOutcome::Match(sink_pat)` once `call_path` matches and `semantic_check` passes (or is `None`). Insert the per-arg check between `semantic_check` passing and the `Match` return:

```rust
// existing logic for call_path + semantic_check ...
if let Some(check) = sink_pat.semantic_check {
    let cs = CallSite {
        call_node: *call,
        source: parsed.source.as_str(),
    };
    if !check(&cs) {
        continue;
    }
}

// NEW: per-arg taint check — only when a FlowPath is provided.
// `path == None` is the source==sink no-originating-path fallback (see
// design note §2.3); in that case we trust the existing call_path +
// semantic_check gate without per-arg precision, preserving today's
// source==sink behavior for shapes like `c.File(c.Param("f"))`.
if let Some(p) = path {
    let any_arg_tainted = sink_pat
        .tainted_arg_indices
        .iter()
        .any(|&idx| arg_is_tainted_in_path(parsed, call, idx, p));
    if !any_arg_tainted {
        // call_path + semantic_check passed, but the relevant args aren't
        // tainted on this path. Mark as a structural call_path-match-but-
        // not-actually-firing, which propagates as `SemanticallyExcluded`
        // at the aggregate level.
        had_call_path_match = true;
        continue;
    }
}

return SinkMatchOutcome::Match(sink_pat);
```

The continue after `had_call_path_match = true` is intentional: subsequent iterations may find a different `call` node on the same line that DOES have tainted args. Falling through to the bottom of the loop without a `Match` produces `SemanticallyExcluded` (because `had_call_path_match` is set).

- [ ] **Step 2.3: Update doc comment**

Replace the existing per-arg note ("Note: `tainted_arg_indices` is *not* checked here…") with:

```rust
/// Returns the structured-sink outcome for `sink_pat` on `line` of `parsed`,
/// using `path` to resolve per-argument taint via `arg_is_tainted_in_path`.
///
/// Outcome rules:
/// - `Match(sink_pat)` — call_path matches, semantic_check (if any) passes,
///   AND at least one arg in `sink_pat.tainted_arg_indices` is tainted on `path`.
/// - `SemanticallyExcluded` — call_path matches but EITHER `semantic_check`
///   rejects OR no arg in `tainted_arg_indices` is tainted on this path.
/// - `NoMatch` — no call expression on `line` matches `sink_pat.call_path`.
///
/// Caller is responsible for confirming `parsed.language == Language::Go`
/// (the function returns `NoMatch` for non-Go files).
fn line_matches_structured_sink(...) -> SinkMatchOutcome {
```

- [ ] **Step 2.4: Compile-check**

```bash
cargo check 2>&1 | tail -10
# Expected: errors at the call sites (go_sink_outcome and beyond) about missing
# argument. We fix those in Task 3.
```

---

## Task 3: Thread `&FlowPath` through `go_sink_outcome`

**File:** `src/algorithms/taint.rs`

- [ ] **Step 3.1: Update signature**

Change:
```rust
fn go_sink_outcome(parsed: &ParsedFile, line: usize) -> SinkMatchOutcome {
```

to:
```rust
fn go_sink_outcome(parsed: &ParsedFile, line: usize, path: Option<&FlowPath>) -> SinkMatchOutcome {
```

- [ ] **Step 3.2: Pass `path` through to inner calls**

In the body, change every `line_matches_structured_sink(parsed, line, pat)` to `line_matches_structured_sink(parsed, line, pat, path)`. Three call sites inside the body (CWE-78 loop, CWE-22 loop, framework SINKS loop).

- [ ] **Step 3.3: Update doc comment**

Brief addition: mention that `path` is consulted for per-arg taint via `line_matches_structured_sink`.

- [ ] **Step 3.4: Compile-check**

```bash
cargo check 2>&1 | tail -10
# Expected: errors at the two callers of go_sink_outcome (forward-flow loop + source==sink loop).
```

---

## Task 4: Update the forward-flow loop to pass the path

**File:** `src/algorithms/taint.rs` (~line 1173).

- [ ] **Step 4.1: Locate the forward-flow `go_sink_outcome` call**

```bash
grep -n "go_sink_outcome(parsed" src/algorithms/taint.rs
```

The first hit is in the forward-flow loop body. Inside that loop, the iteration variable is `path: &FlowPath` (or similar — verify by reading 5 lines up). The call is:

```rust
let outcome = if parsed.language == Language::Go {
    go_sink_outcome(parsed, edge.to.line)
} else {
    SinkMatchOutcome::NoMatch
};
```

- [ ] **Step 4.2: Pass `Some(path)` through**

Change to:

```rust
let outcome = if parsed.language == Language::Go {
    go_sink_outcome(parsed, edge.to.line, Some(path))
} else {
    SinkMatchOutcome::NoMatch
};
```

The `path` variable is already in scope (the outer for-loop iterates over `&paths`). Forward-flow always has a concrete path → always `Some(path)`.

---

## Task 5: Update the source==sink loop to pass the path

**File:** `src/algorithms/taint.rs` (~line 1227).

- [ ] **Step 5.1: Locate the source==sink loop**

The second `go_sink_outcome` hit (from Step 4.1's grep). Currently:

```rust
let sink_pat = match go_sink_outcome(parsed, *line) {
    SinkMatchOutcome::Match(p) => p,
    SinkMatchOutcome::SemanticallyExcluded | SinkMatchOutcome::NoMatch => continue,
};
```

- [ ] **Step 5.2: Define `find_sink_with_inline_framework_source` helper**

This helper supports the secondary inline-source fallback in Step 5.3 — needed for mixed same-line source==sink cases where another source on the line generates a FlowPath (making `originating` non-empty and the `Option::None` primary fallback inapplicable). The helper scans **all** structured sink calls on the line (not just the first one `go_sink_outcome` would return) and returns the first sink pattern whose tainted_arg subtree contains a descendant `call_expression` matching the active framework's source patterns.

**Why "all sinks, not just first-match-wins":** routing through `go_sink_outcome(.., None)` would only consider the first aggregate Match. For lines like `exec.Command("ls"); c.File(c.Param("f"))`, the first match is `exec.Command` (tainted-binary pattern, since `semantic_check: None` post-Path-C-retirement). The helper would check exec.Command's tainted_arg ("ls" — no inline source), return false, and never get to c.File. Scanning all sink-pat × call combinations independently is the correct shape.

**Why function-scoped request param names** (NOT file-wide): the existing `detect_framework_sources` collects `*http.Request` / `*gin.Context` parameter names from the enclosing function only — so `c.Param` only resolves if `c` is actually bound in that function's signature. The new helper must mirror this scope, otherwise an unrelated handler in the same file with `c *gin.Context` would cause `c.Param` calls in a function whose signature doesn't bind `c` to be wrongly treated as framework sources. The corresponding **empty-function-scope guard** (return None when `request_param_names.is_empty()`) is also load-bearing: without it, non-prefixed sources like `mux.Vars` would still be added to `source_paths` (because `concretize_source_call_path` passes them through unchanged) even in functions with no `*http.Request` parameter — re-creating the same false-positive class through the back door. The existing `detect_framework_sources` carries this guard at taint.rs ~L794; the new helper must too.

Place the helpers near `arg_is_tainted_in_path` (added in Task 1). They share concerns about subtree walking + framework-source recognition.

```rust
/// Returns the first structured sink pattern on `line` whose tainted_arg subtrees
/// contain a descendant call_expression matching the active framework's source
/// patterns (e.g. `c.Param`, `r.URL.Query`). Used as a secondary fallback in the
/// source==sink loop to catch inline source==sink shapes that the per-arg DFG with
/// a real FlowPath cannot resolve — inline framework-source calls don't generate
/// FlowEdges because their results are consumed inline.
///
/// Scanning is exhaustive over (sink_pat, call) on this line, not first-match.
/// First-match would miss inline shapes when an unrelated structured sink earlier
/// on the line shadows the inline-bearing one.
///
/// Request param names are scoped to the enclosing function of `line`, mirroring
/// `detect_framework_sources`. File-wide collection would treat unrelated handlers'
/// receiver names as valid binders here.
///
/// Phase 1.5 limitation: only framework sources are recognized, not IPC sources.
/// IPC source==sink shapes are rare and remain a Phase 1.5.1+ refinement.
fn find_sink_with_inline_framework_source(
    parsed: &ParsedFile,
    line: usize,
) -> Option<&'static SinkPattern> {
    let framework = parsed.framework()?;

    // Function-scoped request param name collection. Mirrors
    // `detect_framework_sources` — only binds receiver names that appear in the
    // enclosing function's signature.
    let func_node = parsed.enclosing_function(line)?;
    let target_types = framework_request_types(framework.name);
    if target_types.is_empty() {
        return None;
    }
    let request_param_names = collect_request_param_names(parsed, &func_node, target_types);
    // Empty-function-scope guard: mirrors the early `continue` in
    // `detect_framework_sources` (taint.rs ~L794). Without it, non-prefixed
    // sources like `mux.Vars` would still be inserted into `source_paths` even
    // when the enclosing function has no `*http.Request` / `*gin.Context`
    // parameter, wrongly recognizing them as framework sources for this line.
    // Returning None here matches the existing detector's "no request-like
    // parameter, no framework source" behavior — the file-scope warning above
    // is the spatial half of this guard, this is the function-scope half.
    if request_param_names.is_empty() {
        return None;
    }

    // Build the set of concrete framework-source call_paths for THIS function.
    let mut source_paths: BTreeSet<String> = BTreeSet::new();
    for src in framework.sources {
        for concrete in concretize_source_call_path(src.call_path, &request_param_names) {
            source_paths.insert(concrete);
        }
    }
    if source_paths.is_empty() {
        return None;
    }

    // Walk all calls on `line`. For each, check against EVERY structured sink
    // pattern (priority order: GO_CWE78_SINKS, GO_CWE22_SINKS, framework SINKS).
    // First (call, sink_pat) pair where the tainted_arg subtree contains an
    // inline framework source returns its sink_pat.
    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };

        let pattern_iter = GO_CWE78_SINKS
            .iter()
            .chain(GO_CWE22_SINKS.iter())
            .chain(framework.sinks.iter());

        for pat in pattern_iter {
            if actual != pat.call_path {
                continue;
            }
            // Apply semantic_check (matches go_sink_outcome's gating for non-
            // None paths). If semantic_check rejects, this pattern doesn't
            // describe THIS call — skip and try the next pattern.
            if let Some(check) = pat.semantic_check {
                let cs = CallSite {
                    call_node: *call,
                    source: parsed.source.as_str(),
                };
                if !check(&cs) {
                    continue;
                }
            }
            let arguments = match call.child_by_field_name("arguments") {
                Some(n) => n,
                None => continue,
            };
            let mut cursor = arguments.walk();
            let mut idx = 0usize;
            for arg in arguments.named_children(&mut cursor) {
                if pat.tainted_arg_indices.contains(&idx) {
                    if subtree_has_call_in(parsed, &arg, &source_paths) {
                        return Some(pat);
                    }
                }
                idx += 1;
            }
        }
    }
    None
}

/// Walk `node` and descendants; returns true if any `call_expression` node has
/// a call_path text in `paths`.
fn subtree_has_call_in(
    parsed: &ParsedFile,
    node: &tree_sitter::Node<'_>,
    paths: &BTreeSet<String>,
) -> bool {
    if node.kind() == "call_expression" {
        if let Some(cp) = go_call_path_text(parsed, node) {
            if paths.contains(&cp) {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if subtree_has_call_in(parsed, &child, paths) {
            return true;
        }
    }
    false
}
```

**Notes on `collect_request_param_names` / `framework_request_types` / `concretize_source_call_path`:**
- `collect_request_param_names(parsed, &func_node, target_types) -> Vec<String>` already exists in `taint.rs` (used by `detect_framework_sources`, see ~L709) and is **function-scoped** by signature — exactly the scope the new helper needs. **Reuse it directly**; no `_in_function` wrapper required. **Do NOT widen to file-scope** — see review feedback P2: a file-wide collection would treat unrelated handlers' receiver names as valid binders here, creating false-positive inline-source detections.
- `framework_request_types(framework_name) -> &[&str]` already exists (~L674). Use to derive `target_types` for `collect_request_param_names`. If empty (framework not in the enum match), return None — matches the existing `detect_framework_sources` guard at ~L788-790.
- **Empty-function-scope guard:** when `request_param_names.is_empty()`, return None *before* populating `source_paths`. Mirrors the `detect_framework_sources` guard at ~L794. Without it, non-prefixed sources like `mux.Vars` would still be inserted into `source_paths` (because `concretize_source_call_path` returns them unchanged), wrongly recognizing them as framework sources for a function that doesn't bind a request-like parameter. The file-scope warning above is the spatial half of this guard; this is the function-scope half.
- `parsed.enclosing_function(line)` already exists on `ParsedFile` (used by `function_body_cleansed_for`). Use it to locate the enclosing function for the line.
- `substitute_prefix` exists in `taint.rs`; `concretize_source_call_path` is a thin wrapper that produces all concrete forms for a given source pattern (one per request_param_name plus the unsubstituted pattern for non-prefixed paths like `mux.Vars`). Inline if trivial.

If any of these helpers requires significant refactoring to be reusable, factor in a separate refactoring commit before this one.

- [ ] **Step 5.3: Replace the entire source==sink body — Option<&FlowPath> + per-path Match-and-cleansing combined loop + secondary inline-source fallback**

This step REPLACES the source==sink loop's existing body (currently spans `~taint.rs:1232-1265`, including both the `match go_sink_outcome` block and the subsequent `path_states` cleansing block). Critical: the existing `path_states.iter().any(|c| !c)` pattern is **wrong** post-DFG — it iterates all originating paths regardless of whether they Match this sink, so a non-matching, non-cleansed path can spuriously fire when a matching path is properly cleansed. We must combine Match-detection and cleansing into a single per-path loop.

Three sub-cases:

- **No originating paths** (pure source==sink shape like `c.File(c.Param("f"))` — only inline source on the line, no FlowEdge): pass `Option<&FlowPath>::None` to `go_sink_outcome`. The engine skips per-arg DFG and falls back to call_path + semantic_check matching. Then use `function_body_cleansed_for` for cleansing, mirroring today's behavior.
- **Originating paths exist, primary check** (`paths.iter().filter(originating)` non-empty): the source has FlowEdges. Walk each originating path; for those that produce `Match`, check if that path's `cleansed_for` contains the matched category. **Fire iff at least one matching path is not cleansed.** Non-matching paths are skipped — their cleansing state is irrelevant because per-arg DFG already says they don't fire this sink.
- **Originating paths exist, secondary inline-source fallback** (only when primary check found no fire): the line may host both a non-inline source (driving the FlowPath) AND an inline source==sink shape. Call `find_sink_with_inline_framework_source(parsed, *line)` directly — it scans **all** (call, sink_pat) combinations on the line independently and returns the first sink pattern whose tainted_arg subtree contains an inline framework-source call. Routing through `go_sink_outcome(.., None)` first would be wrong: its first-match-wins aggregation would shadow inline-bearing sinks behind earlier non-inline matches. If the helper returns `Some(pat)`, fire (modulo `function_body_cleansed_for`). The helper itself is the load-bearing guard — it scopes the secondary fallback to genuinely-inline syntactic shapes only, preventing the variable-bound-to-literal over-fire class from creeping back in.

**DO NOT synthesize an empty FlowPath.** That was the original draft — it has a subtle bug: with empty `path.edges`, every non-literal arg appears not-tainted, including args that should be tainted by virtue of containing a framework-source call. The empty-path approach would silently drop the `c.File(c.Param("f"))` finding. `Option::None` + the inline-source guard is the correct semantic.

Replace the entire body of the `for (file, line) in &taint_sources` block (Go branch) with:

```rust
// Find every path whose source is this (file, line).
let originating: Vec<&FlowPath> = paths
    .iter()
    .filter(|p| {
        p.edges
            .first()
            .map(|e| e.from.file == *file && e.from.line == *line)
            .unwrap_or(false)
    })
    .collect();

if originating.is_empty() {
    // No FlowPath originates — pure source==sink shape (e.g. c.File(c.Param("f"))).
    // Pass None to skip per-arg DFG; engine falls back to call_path +
    // semantic_check matching. Preserves today's source==sink behavior.
    let sink_pat = match go_sink_outcome(parsed, *line, None) {
        SinkMatchOutcome::Match(p) => p,
        SinkMatchOutcome::SemanticallyExcluded | SinkMatchOutcome::NoMatch => continue,
    };
    // No FlowPath cleansing applies. Fall back to function-body scan
    // (mirrors today's behavior at the same site).
    let cleansed = function_body_cleansed_for(parsed, *line, sink_pat.category);
    if !cleansed {
        sink_lines.insert((file.clone(), *line));
    }
} else {
    // Per-arg DFG applies. Walk originating paths; for each that Matches,
    // check whether its FlowPath is cleansed for the matched category.
    // Fire iff AT LEAST ONE matching path is not cleansed.
    //
    // Crucially, we ONLY consult cleansing for paths that actually Match
    // (per-arg DFG: relevant args are tainted on this path). A non-matching
    // path's cleansing state is irrelevant — per-arg DFG already says it
    // doesn't fire this sink, so its cleansing-or-not can't move the
    // decision. Including non-matching paths in the cleansing decision
    // (the pre-fix shape) would cause spurious fires when one path matches
    // and is cleansed, while another path doesn't match but isn't cleansed.
    let mut any_matching_uncleansed = false;
    for p in &originating {
        if let SinkMatchOutcome::Match(pat) = go_sink_outcome(parsed, *line, Some(p)) {
            if !p.cleansed_for.contains(&pat.category) {
                any_matching_uncleansed = true;
                break;
            }
        }
    }

    // Secondary inline-source fallback for mixed same-line shapes. The line
    // may host both a non-inline source (driving an originating FlowPath)
    // AND an inline source==sink shape (e.g. c.File(c.Param("f"))) that the
    // primary per-arg DFG can't recognize because its conservative recursion
    // only checks identifiers against FlowPath edges, and the inline c.Param
    // call generates no FlowEdge.
    //
    // The helper scans ALL (call, sink_pat) combinations on the line and
    // returns the first sink pattern whose tainted_arg subtree contains an
    // inline framework-source call. This is intentionally NOT routed through
    // go_sink_outcome's first-match-wins aggregation — that would only
    // consider the first matching sink and miss inline shapes when an
    // unrelated structured sink earlier on the line shadows them.
    //
    // The helper is the load-bearing guard: it scopes the secondary fallback
    // to genuinely-inline source==sink shapes only, avoiding the variable-
    // bound-to-literal over-fire class that pre-DFG no-path matching would
    // re-introduce.
    if !any_matching_uncleansed {
        if let Some(pat) = find_sink_with_inline_framework_source(parsed, *line) {
            let cleansed = function_body_cleansed_for(parsed, *line, pat.category);
            if !cleansed {
                any_matching_uncleansed = true;
            }
        }
    }

    if any_matching_uncleansed {
        sink_lines.insert((file.clone(), *line));
    }
    // else: every matching path is cleansed (or no path matches AND no inline
    // source==sink shape was detected). Suppress.
}
```

**Why guard the secondary fallback on `find_sink_with_inline_framework_source`?** Without the guard, running `go_sink_outcome(.., None)` unconditionally would re-introduce the variable-bound-to-literal over-fire class (the entire reason Path C exists today, and the reason the dual-layer suppression in PR #73 was rolled back). The guard scopes the secondary fallback to genuinely-inline source==sink shapes only — the same syntactic shape that the §3.5 primary fallback test pins.

- [ ] **Step 5.4: Compile-check**

```bash
cargo check 2>&1 | tail -10
# Expected: clean compile. If errors mention FlowPath not in scope, ensure
# `use crate::data_flow::FlowPath;` is at the top of taint.rs (Task 1's
# import step). The struct itself has pub fields (`edges`, `cleansed_for`)
# already.
```

---

## Task 6: Add 6 regression tests

**File:** `tests/algo/taxonomy/taint_sink_lang_test.rs`

- [ ] **Step 6.1: Add the variable-bound-to-literal test (design note §3.1)**

Use a **structured-only sink** (`c.File`, no flat overlap) so absence-of-finding is genuinely diagnostic. The original draft used `exec.Command(bin, ...)` but the flat `Command` substring fires regardless, and `sink_lines` line-dedupes so a `count <= 1` assertion can't distinguish "structured + flat" from "flat-only." Insert after the existing `// Path C dual-layer regression tests` comment block:

```rust
#[test]
fn test_taint_cwe22_cfile_variable_bound_to_literal_no_finding() {
    // Phase 1.5 (#1) discriminating regression. `bin = "/etc/static.txt"` is a
    // literal-bound variable; pre-DFG, the structured c.File pattern fires
    // because the line is line-tainted (input from c.Query is referenced on
    // the same line via `_ = input`). Post-DFG: arg[0] = bin identifier; bin
    // is not in any FlowEdge — defined as a literal — so arg[0] is not
    // tainted → SemanticallyExcluded → no fire. c.File has no flat-pattern
    // overlap, so absence of any taint_sink on the call line discriminates
    // pre vs post DFG behavior.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	input := c.Query("input")
	bin := "/etc/static.txt"
	_ = c.File(bin); _ = input
}
"#;
    // Source line: 6 (c.Query). Sink line: 8 (c.File).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    let sink_line_finding = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 8);
    assert!(
        !sink_line_finding,
        "c.File(bin) where bin is bound to a literal should NOT fire post-DFG, \
         even when an unrelated tainted variable shares the line"
    );
}
```

- [ ] **Step 6.2: Add a smoke test for `os.Rename` (multi-index sink)**

`os.Rename` is the only multi-index sink in the Phase 1 registry (`tainted_arg_indices: &[0, 1]`). This test confirms it fires when arg[1] is tainted, but it is **not diagnostic for any-tainted semantics specifically** because flat `SINK_PATTERNS` already includes the `Rename` substring (`taint.rs:123`) — the test would pass via flat fallback even if structured per-arg DFG were broken on multi-index. Documented as a smoke test with a TODO for a unit-level any-tainted assertion in Phase 1.5.1.

```rust
#[test]
fn test_taint_cwe22_os_rename_smoke() {
    // Smoke test for os.Rename firing on tainted arg[1].
    //
    // Limitation: not a discriminator for any-tainted semantics on
    // `tainted_arg_indices: &[0, 1]` because flat SINK_PATTERNS already
    // includes "Rename" (taint.rs:123). The test would still pass via flat
    // fallback even if structured per-arg DFG mishandled multi-index.
    // For a discriminating assertion of any-tainted semantics, queue a
    // unit test against `go_sink_outcome` internals — Phase 1.5.1 follow-up.
    let source = r#"package main

import (
	"os"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	dest := c.Query("dest")
	_ = os.Rename("/tmp/static.txt", dest)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "os.Rename should fire when arg[1] is tainted (smoke; not diagnostic \
         for structured any-tainted semantics due to flat-pattern overlap)"
    );
}
```

- [ ] **Step 6.3: Add the taint-near-but-not-into-call test (design note §3.4)**

Use a structured-only sink (`c.File`) for the same discrimination reason as Step 6.1.

```rust
#[test]
fn test_taint_cwe22_cfile_literal_arg_with_unrelated_line_taint_no_finding() {
    // Phase 1.5 negative control on a structured-only sink. Tainted `input` is
    // referenced on the same line as a c.File call whose arg[0] is a literal.
    // Pre-DFG: line-granular over-fire (line tainted via input). Post-DFG:
    // arg[0] = "/etc/static.txt" literal → not tainted → SemanticallyExcluded
    // → no fire. c.File has no flat overlap; absence of taint_sink on the
    // call line discriminates.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	input := c.Query("input")
	_ = c.File("/etc/static.txt"); _ = input
}
"#;
    // Source line: 6 (c.Query). Sink line: 7 (c.File with literal arg).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    let sink_line_finding = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 7);
    assert!(
        !sink_line_finding,
        "c.File with a literal arg should NOT fire post-DFG even when an unrelated \
         tainted variable shares the line"
    );
}
```

- [ ] **Step 6.4: Add the complex-expression conservative test**

```rust
#[test]
fn test_taint_cwe78_complex_arg_expression_fires() {
    // exec.Command takes the result of a method call as the binary arg. The
    // method call itself returns tainted data via DFG. Per-arg conservative
    // recursion: descend into the call_expression for arg[0]; the inner identifier
    // is tainted, so arg[0] is treated as tainted; structured fires.
    let source = r#"package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	bin := c.Query("bin")
	_ = exec.Command(string([]byte(bin)), "--help").Run()
}
"#;
    // Source line: 10 (c.Query). Sink line: 11 (exec.Command with complex expr).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "exec.Command with a complex-expression arg[0] containing a tainted \
         identifier should fire via per-arg conservative recursion"
    );
}
```

- [ ] **Step 6.5: Add source==sink positive regression (design note §3.5)**

Pins the no-originating-path fallback's call_path + semantic_check matching. Without this test, a future change replacing `Option<&FlowPath>` with synthetic-empty-path would silently drop `c.File`/`c.Param` source==sink coverage.

```rust
#[test]
fn test_taint_cwe22_cfile_inline_param_source_still_fires() {
    // Source==sink shape. c.Param is the source AND its return value is
    // c.File's arg[0]. No FlowEdge connects them (no intermediate variable),
    // so paths.iter().filter(originating) is empty. The source==sink loop's
    // no-originating-path branch passes None for the FlowPath argument; the
    // engine falls back to call_path + semantic_check matching (skipping
    // per-arg DFG) and fires.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	c.File(c.Param("f"))
}
"#;
    // Source/sink line: 6 (c.File and c.Param share this line).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    assert!(
        has_taint_sink(&result),
        "c.File(c.Param(\"f\")) source==sink shape must still fire post-DFG \
         via the Option<&FlowPath>::None fallback in the source==sink loop"
    );
}
```

- [ ] **Step 6.6: Add mixed same-line source==sink regression (design note §3.6)**

Pins the secondary inline-source fallback. Without this test, a future change that simplifies the source==sink loop (e.g. dropping `find_sink_with_inline_framework_source`) could silently regress the mixed-line shape that the reviewer flagged in PR #73's iteration.

```rust
#[test]
fn test_taint_cwe22_cfile_inline_param_with_parallel_path_still_fires() {
    // P2 mixed same-line regression. Single-line function body has TWO
    // sources: c.Query (generates a FlowPath via def-use of `other`) AND
    // c.Param (inline inside c.File, no FlowPath). Source 1's FlowPath
    // makes `originating` non-empty for this line, so the primary
    // Option<&FlowPath>::None fallback (gated on originating.is_empty())
    // is skipped. The secondary inline-source fallback (gated on
    // find_sink_with_inline_framework_source) detects c.Param as an
    // inline framework-source call inside c.File's arg[0] and fires the
    // c.File sink (modulo function-body cleansing).
    let source = r#"package main

import (
	"fmt"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	other := c.Query("other"); c.File(c.Param("f")); fmt.Println(other)
}
"#;
    // All three statements share line 10. taint_sources contains (file, 10).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "c.File(c.Param(\"f\")) on a line with a parallel c.Query/FlowPath \
         must still fire post-DFG via the inline-source secondary fallback. \
         If this fails, audit find_sink_with_inline_framework_source's \
         framework-source recognition + concretize_source_call_path's prefix \
         substitution."
    );
}
```

- [ ] **Step 6.7: Run tests for Commit 1 milestone**

```bash
cargo test --test algo_taint_sink_lang 2>&1 | tail -25
# Expected: all 21 tests pass (15 prior + 6 new).
```

---

## Task 7: Full suite + fmt + commit

- [ ] **Step 7.1: Full suite**

```bash
cargo test 2>&1 | grep "test result:" | awk '{sum+=$4} END {print sum}'
# Expected: 1444 (1438 prior + 6 new). If different, audit.
cargo test --test integration_cwe_phase1_suppression -- --nocapture 2>&1 | grep -E "rate|test result"
# Expected: "10/10 (100% — pinned floor: 80%)" — sanitized suppression unchanged.
```

- [ ] **Step 7.2: fmt + clippy**

```bash
cargo fmt --check
# Expected: no output.
cargo clippy --all-targets 2>&1 | grep -E "warning:|error" | grep -v "^Compiling\|^Finished\|never_loop\|format_string_wrappers" | head -10
# Expected: no NEW warnings beyond pre-existing baseline issues.
```

- [ ] **Step 7.3: Commit**

```bash
git add src/algorithms/taint.rs tests/algo/taxonomy/taint_sink_lang_test.rs

git commit -m "$(cat <<'EOF'
Per-arg DFG: honor tainted_arg_indices at sink eval time

Phase 1.5 (item #1) per the eval-team RE finding (2026-04-26). The
existing engine treated `tainted_arg_indices: &'static [usize]` as
forward-compat scaffolding — taint matching was line-granular, so the
structured tainted-binary sink fired whenever any taint reached the
call line, even on shapes like `bin := "ffmpeg"; exec.Command(bin, ...)`
where arg[0] is a non-literal but never carries tainted data (Path C
semantic_check passed because bin is an identifier, not a literal).

This commit makes the engine consult each arg's identifier against
the FlowPath's edges. New helpers `arg_is_tainted_in_path` and
`arg_node_taints_match` walk the arg node:
- Literal arg → never tainted.
- Bare identifier → check `FlowEdge.to` matches `(parsed.path,
  call_line, var_name)`. File scoping is required so an
  interprocedural FlowEdge in another file with a colliding line/
  name doesn't falsely register as taint here.
- Complex expression → conservative recurse into identifier
  descendants.

`line_matches_structured_sink` and `go_sink_outcome` now take
`Option<&FlowPath>`. Forward-flow loop passes `Some(path)`. The
source==sink loop branches: when no FlowPath originates at the source
(canonical `c.File(c.Param("f"))` shape), pass `None` so the engine
falls back to call_path + semantic_check matching, then use
`function_body_cleansed_for` for cleansing — preserves today's
source==sink behavior. When originating paths exist, iterate them
combining Match-detection and cleansing decisions per-path: fire iff
at least one matching path is not cleansed for the matched
category. Non-matching paths are skipped (their cleansing state is
irrelevant because per-arg DFG already says they don't fire this
sink).

Aggregation rule: any-tainted on `tainted_arg_indices: &[i, j, …]` —
sink fires if AT LEAST ONE indexed arg is tainted (matches the
existing line-granular over-approximation and the spec §3.2 whole-slice
behavior for syscall.Exec).

Returning `SinkMatchOutcome::SemanticallyExcluded` for "no arg
tainted" lands cleanly on the post-PR-#73-rollback engine: the flat
catch-all stays active (no-op behavior preserved), so the eval-team
P1/P2 regression guards remain green.

6 new regression tests (using structured-only sinks where applicable
— flat-pattern overlap defeats absence-of-finding assertions because
sink_lines line-dedupes):
- test_taint_cwe22_cfile_variable_bound_to_literal_no_finding
  (eval-team probe shape on c.File, structured-only).
- test_taint_cwe22_os_rename_smoke (multi-index sink — flagged as
  smoke because Rename has flat-pattern overlap; not diagnostic for
  any-tainted semantics specifically).
- test_taint_cwe22_cfile_literal_arg_with_unrelated_line_taint_no_finding
  (taint flows past the call without entering its args).
- test_taint_cwe78_complex_arg_expression_fires (conservative
  recursion into composite expressions).
- test_taint_cwe22_cfile_inline_param_source_still_fires (pure
  source==sink: c.File(c.Param("f")) with no parallel FlowPath; pins
  the Option<&FlowPath>::None primary fallback).
- test_taint_cwe22_cfile_inline_param_with_parallel_path_still_fires
  (mixed same-line: c.File(c.Param("f")) sharing a line with c.Query
  whose FlowPath makes originating non-empty; pins the secondary
  inline-source fallback gated on
  find_sink_with_inline_framework_source).

Signature shape: `Option<&FlowPath>` (not `&FlowPath`) so the
source==sink loop can pass None for the no-path/inline cases and
fall back to call_path + semantic_check matching. Synthetic-empty-
path was rejected — it would silently drop c.File(c.Param("f")).

Source==sink loop has THREE branches:
1. originating.is_empty() → primary Option::None fallback for pure
   source==sink shapes.
2. originating non-empty + per-path Match-and-cleansing combined
   loop → fire if any matching path is uncleansed.
3. originating non-empty + no matching-uncleansed → secondary inline-
   source fallback gated on find_sink_with_inline_framework_source,
   for mixed same-line shapes.

The secondary fallback's guard is critical: running Option::None
unconditionally would re-introduce the variable-bound-to-literal
over-fire class (the entire reason Path C exists, and the reason
PR #73's dual-layer suppression was rolled back).

File scoping: `arg_node_taints_match` requires
`e.to.file == parsed.path` so cross-file FlowEdges with colliding
line/name don't falsely register as taint here.

Test count: 1438 -> 1444. Sanitized suppression rate unchanged at
10/10 (100%). PR #73 P1/P2 regression guards still pass.

Path C scaffolding remains in place for this commit; retirement lands
in the next commit (separate concerns: engine mechanism vs scaffolding
cleanup).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"

git log -1 --oneline
git status --short
```

Expected: working tree clean. HEAD is the new commit.

---

# Commit 2 — Path C scaffolding retirement

**Sub-task: this commit removes Path C's now-redundant `semantic_check` helpers and updates the spec accordingly. Tasks 8–11. Verify tests still pass before committing.**

## Task 8: Remove Path C `semantic_check` helpers + restore `semantic_check: None`

**File:** `src/algorithms/taint.rs`

- [ ] **Step 8.1: Locate Path C helpers**

```bash
grep -n "check_command_taintable_binary\|check_commandcontext_taintable_binary\|check_arg_is_non_literal_at" src/algorithms/taint.rs
```

Expect 3 helper definitions + 2 usages on the `SinkPattern` const entries.

- [ ] **Step 8.2: Remove the helpers**

Delete the `check_arg_is_non_literal_at`, `check_command_taintable_binary`, and `check_commandcontext_taintable_binary` function definitions.

- [ ] **Step 8.3: Restore `semantic_check: None` on the tainted-binary entries**

In `GO_CWE78_SINKS`, change:

```rust
SinkPattern {
    call_path: "exec.Command",
    category: SanitizerCategory::OsCommand,
    tainted_arg_indices: &[0],
    semantic_check: Some(check_command_taintable_binary),
},
SinkPattern {
    call_path: "exec.CommandContext",
    category: SanitizerCategory::OsCommand,
    tainted_arg_indices: &[1],
    semantic_check: Some(check_commandcontext_taintable_binary),
},
```

to:

```rust
SinkPattern {
    call_path: "exec.Command",
    category: SanitizerCategory::OsCommand,
    tainted_arg_indices: &[0],
    semantic_check: None,
},
SinkPattern {
    call_path: "exec.CommandContext",
    category: SanitizerCategory::OsCommand,
    tainted_arg_indices: &[1],
    semantic_check: None,
},
```

- [ ] **Step 8.4: Update the const-block doc comment**

Find the doc-comment block above `pub const GO_CWE78_SINKS`. Update the prose about tainted-binary variants to remove Path C interim language. Replace text describing `check_command_taintable_binary` / `check_commandcontext_taintable_binary` with: *"Tainted-binary variants — first non-ctx arg is the binary path. `semantic_check: None` because per-arg taint resolution at sink-eval time (see `arg_is_tainted_in_path`) is the structural gate: a literal binary has no identifier and is never tainted; a variable bound to a non-tainted source isn't in any FlowPath edge at the call line."*

---

## Task 9: Update `tainted_arg_indices` doc-comment in `src/frameworks/mod.rs`

**File:** `src/frameworks/mod.rs`

- [ ] **Step 9.1: Locate the field**

```bash
grep -n "tainted_arg_indices" src/frameworks/mod.rs
```

- [ ] **Step 9.2: Replace the doc comment**

Current:
```rust
/// 0-indexed argument positions whose taint fires this sink.
///
/// **Phase 1 status:** captured for forward compatibility but NOT
/// consulted by the current engine — taint matching is line-granularity
/// (see `taint.rs::line_matches_structured_sink`). Per-argument
/// precision is a Phase 2/3 concern.
pub tainted_arg_indices: &'static [usize],
```

Replace with:
```rust
/// 0-indexed argument positions whose taint fires this sink. Any-tainted
/// semantics: the sink fires if AT LEAST ONE indexed arg is tainted on
/// the matching FlowPath (see `taint.rs::arg_is_tainted_in_path`).
///
/// Examples:
/// - `&[0]` for `exec.Command(taintedBin, ...)` — fires when arg[0] is tainted.
/// - `&[2]` for `exec.Command("sh", "-c", taintedCmd)` — fires when arg[2] is
///   tainted (paired with `semantic_check` confirming the shell-wrapper shape).
/// - `&[0, 1]` for `os.Rename(old, new)` — fires when EITHER arg is tainted.
/// - `&[0, 1]` for `syscall.Exec(argv0, argv, envv)` — argv slice is treated
///   conservatively (any tainted element taints the whole slice via DFG).
pub tainted_arg_indices: &'static [usize],
```

---

## Task 10: Update spec §3.2

**File:** `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`

- [ ] **Step 10.1: Locate the Path C interim paragraph**

```bash
grep -n "Path C interim\|check_command_taintable_binary\|check_commandcontext_taintable_binary" docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **Step 10.2: Update the §3.2 const example**

Restore the `semantic_check: None` entries on the tainted-binary `SinkPattern` examples.

Remove the `check_command_taintable_binary` / `check_commandcontext_taintable_binary` helper definitions from the prose. They no longer exist in the code.

- [ ] **Step 10.3: Replace the Path C interim explanation**

Find the paragraph beginning *"**Tainted-binary `semantic_check` (Path C interim).**"*. Replace with:

> **Tainted-binary `semantic_check: None` (Phase 1.5+).** Earlier Phase 1 used a Path C `semantic_check` (`check_command_taintable_binary`) to syntactically gate the tainted-binary pattern on arg[0] being non-literal. Phase 1.5 (item #1) replaced this with proper per-arg taint resolution at sink-eval time via `arg_is_tainted_in_path` in `taint.rs`. The structural gate is now: "is arg[0]'s identifier in this FlowPath's tainted set?" A literal binary has no identifier and is never tainted; a variable bound to a non-tainted source isn't reached by any FlowPath edge at the call line. Path C's semantic_check is therefore redundant and removed.

---

## Task 11: Final verify + commit

- [ ] **Step 11.1: Full suite**

```bash
cargo test 2>&1 | grep "test result:" | awk '{sum+=$4} END {print sum}'
# Expected: 1444 (unchanged from Commit 1).
cargo test --test integration_cwe_phase1_suppression -- --nocapture 2>&1 | grep -E "rate|test result"
# Expected: 10/10 unchanged.
```

- [ ] **Step 11.2: fmt + clippy**

```bash
cargo fmt --check
cargo clippy --all-targets 2>&1 | grep -E "warning:|error" | grep -v "^Compiling\|^Finished\|never_loop\|format_string_wrappers" | head -10
```

- [ ] **Step 11.3: Commit**

```bash
git add src/algorithms/taint.rs src/frameworks/mod.rs docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md

git commit -m "$(cat <<'EOF'
Retire Path C scaffolding now that per-arg DFG is the structural gate

The Path C `semantic_check` helpers (`check_command_taintable_binary`,
`check_commandcontext_taintable_binary`, and the underlying
`check_arg_is_non_literal_at`) were introduced in Phase 1 to
syntactically gate the tainted-binary CWE-78 pattern on arg[0] being
non-literal. They were always interim — see PR #72's spec §3.2 prose.

With per-arg DFG (parent commit), a literal arg has no identifier and
is therefore never in any FlowPath's taint set. The Path C
syntactic check is redundant.

This commit:
- Removes the three helper functions.
- Restores `semantic_check: None` on the two tainted-binary
  `SinkPattern` entries in `GO_CWE78_SINKS`.
- Updates the doc-block for `GO_CWE78_SINKS` to describe per-arg
  taint resolution as the structural gate.
- Updates the `tainted_arg_indices` doc-comment in
  `src/frameworks/mod.rs` to drop "NOT consulted" language and
  describe any-tainted semantics with examples.
- Updates `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`
  §3.2 — replaces the Path C interim paragraph with a Phase 1.5+
  description, restores the `SinkPattern` const examples to use
  `semantic_check: None`.

Test count: 1444 unchanged. Sanitized suppression rate: 10/10 unchanged.
PR #73 P1/P2 regression guards still pass.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"

git log -2 --oneline
```

---

# F1: Push + open PR

- [ ] **F1.1: Push**

```bash
git push -u origin claude/phase15-per-arg-dfg
```

- [ ] **F1.2: Open PR**

Title: `Phase 1.5 (item #1): per-arg DFG — honor tainted_arg_indices`

Body: summary of both commits, acceptance criteria checklist, link to design note.

```bash
gh pr create --title "Phase 1.5 (item #1): per-arg DFG — honor tainted_arg_indices" --body "$(cat <<'EOF'
## Summary

Per-arg DFG (Phase 1.5 queue item #1) per the eval-team Phase 1 acceptance reply (`RE-prism-cwe-phase1-status-20260426.md`). Closes the variable-bound-to-literal false-positive class that Path C semantic_checks were missing.

- New `arg_is_tainted_in_path` helper resolves taint at arg-position granularity using the existing `FlowPath`/`FlowEdge` infrastructure.
- `line_matches_structured_sink` and `go_sink_outcome` now take `Option<&FlowPath>` and consult per-arg taint after the existing call_path + semantic_check gates. The forward-flow loop passes `Some(path)`; the source==sink loop passes `Some(path)` when originating paths exist or `None` to fall back to call_path + semantic_check matching for the canonical `c.File(c.Param("f"))` shape.
- Path C `semantic_check` scaffolding (`check_command_taintable_binary`, `check_commandcontext_taintable_binary`) retired; tainted-binary entries restored to `semantic_check: None`. Per-arg DFG is now the structural gate.
- 6 new regression tests (structured-only sinks for genuine discrimination + 1 smoke test): variable-bound-to-literal on c.File, os.Rename smoke for multi-index, taint-near-but-not-into-call on c.File, complex-expression conservative, pure source==sink primary fallback on c.File(c.Param), mixed same-line source==sink secondary fallback (the PR #73 line-scoping class regression guard).

Returning `SinkMatchOutcome::SemanticallyExcluded` for "no arg tainted" lands cleanly on the post-PR-#73-rollback engine — the flat catch-all stays active (no-op behavior preserved), so the eval-team P1/P2 regression guards remain green.

## Test plan

- [x] `cargo test` — 1,444 passing / 0 failing / 0 ignored (1,438 prior + 6 new)
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --all-targets` — no new warnings
- [x] PR #73 P1/P2 regression guards remain green (`test_taint_cwe78_pwsh_shell_wrapper_fires`, `test_taint_cwe78_same_line_unrelated_sink_preserved`)
- [x] Sanitized suppression rate: 10/10 (100% — pinned floor: 80%)
- [x] Variable-bound-to-literal probe shape no longer produces a structured tainted-binary `taint_sink` finding (regression test `test_taint_cwe78_variable_bound_to_literal_no_finding`)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Post-merge follow-ups (NOT in this PR)

- Update STATUS-prism-cwe-phase1.md priority queue to mark item #1 closed (Path A Phase 1.5+ note becomes stale for the variable-bound-to-literal shape; literal-binary flat-layer false-positive class remains, gated on PowerShell expansion + structured CWE-79 sinks + per-call scoping).
- Update memory entry `project_phase15_dual_layer_interaction.md` — note that #1 is closed and the dual-layer concern remains parked behind the other queue items.
- Eval team validates the new build against C1 fixtures; their reply doc lands at `~/code/agent-eval/analysis/RE-*.md`.
