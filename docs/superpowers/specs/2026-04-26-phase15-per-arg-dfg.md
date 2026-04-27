# Phase 1.5 — Per-arg DFG: honor `tainted_arg_indices`

**Status:** Approved 2026-04-26 (post 4-iteration external review). Implementation tracked in `docs/superpowers/plans/2026-04-26-phase15-per-arg-dfg.md`.

**Goal:** Make the line-matching engine consult `SinkPattern.tainted_arg_indices` against per-argument taint state at the call site, eliminating the variable-bound-to-literal false-positive shape that Path C currently misses, and retiring Path C scaffolding as a side benefit.

**Source:** Eval-team Phase 1 acceptance reply (`~/code/agent-eval/analysis/RE-prism-cwe-phase1-status-20260426.md`) prioritized this as item #1 of the Phase 1.5+ queue: *"only remaining item that changes what Prism detects on fixtures."*

**Context — what PR #73 left behind.** PR #73 introduced a `SinkMatchOutcome { Match, SemanticallyExcluded, NoMatch }` enum and a `go_sink_outcome` aggregate function, then attempted to extend flat-pattern catch-all suppression to the `SemanticallyExcluded` outcome. The suppression extension was rolled back (P1: silently drops coverage on unmodeled shells like `pwsh`; P2: per-line scoping hides unrelated sinks sharing a line). Post-rollback, the enum + aggregate function remain as cleaner-than-`Option<&SinkPattern>` scaffolding, but `SemanticallyExcluded` is currently a runtime no-op — treated the same as `NoMatch` for engine behavior. Per-arg DFG slots into this chassis without re-introducing the rolled-back suppression: returning `SemanticallyExcluded` for "no relevant arg tainted" is a no-op, which is exactly the right post-rollback behavior (preserves flat fallback).

---

## 1. Problem statement

After Phase 1 (`b3e61a6`) + Path C (`b1240b8`) + the rollback in PR #73, the structured Go sink registry still treats the `tainted_arg_indices: &'static [usize]` field as forward-compat scaffolding — its doc-comment in `src/frameworks/mod.rs` says explicitly:

> **Phase 1 status:** captured for forward compatibility but NOT consulted by the current engine — taint matching is line-granularity (see `taint.rs::line_matches_structured_sink`). Per-argument precision is a Phase 2/3 concern.

Path C semantic_checks (`check_command_taintable_binary`, `check_commandcontext_taintable_binary`) work around this by syntactically gating the tainted-binary `SinkPattern` entries on arg[0]/[1] being non-literal. This catches:

- ✅ `exec.Command("ffmpeg", "-i", taintedInput)` — arg[0] literal, structured tainted-binary suppressed.

But misses:

- ❌ `bin := "ffmpeg"; exec.Command(bin, "-i", taintedInput)` — arg[0] is a variable identifier (non-literal), Path C semantic_check passes, structured tainted-binary fires (false positive).

Eval team confirmed 2026-04-26: *"Constant-folding would catch it but that's the DFG work Path B/1.5 brings. Not common in real Go style — agree it's deferrable."* — and queued this as item #1.

**What this PR does and does NOT close.** Per-arg DFG fixes the structured-layer false positive on the variable-bound-to-literal shape (eliminates a class of spurious tainted-binary `taint_sink` findings). It does **not** change the flat-pattern catch-all behavior on the literal-binary case — `exec.Command("ffmpeg", "-i", taintedInput)` will continue to produce a flat-layer `taint_sink` finding via the `Command` substring pattern, just like today. Closing that flat fallback false-positive class requires further flat-catch-all policy work; PowerShell shell-wrapper coverage is handled by a separate Phase 1.5 shell-list follow-up. Eval team is aware: per their 2026-04-26 message, they're waiting for either per-arg DFG OR Phase 2 STATUS — both of which warrant their re-validation cycle. The variable-bound-to-literal probe shape is the specific eval-side win this PR delivers.

## 2. Design

### 2.1 Per-arg taint resolution at the sink call site

`FlowPath.edges: Vec<FlowEdge>` already tracks variable-level provenance. Each `FlowEdge { from: VarLocation, to: VarLocation }` carries `var_name()`. The engine can resolve "is the identifier at call_node arg `i` tainted along this path?" by walking the path's edges.

**Algorithm (per-`SinkPattern` evaluation, replacing today's line-granular check):**

```
fn arg_is_tainted_in_path(parsed, call_node, arg_idx, path) -> bool {
    let arg_node = nth_argument(call_node, arg_idx)?;
    let call_line = call_node.start_position().row + 1;
    arg_node_taints_match(parsed, arg_node, call_line, path)
}

fn arg_node_taints_match(parsed, arg_node, call_line, path) -> bool {
    match arg_node.kind() {
        // Literal — not tainted.
        "interpreted_string_literal" | "raw_string_literal" |
        "int_literal" | "float_literal" | "true" | "false" | "nil" => false,

        // Bare identifier — check if any edge in this path has it as a `to`
        // location at the call site. **File scoping is required**:
        // FlowPath edges may have `to` locations across files for
        // interprocedural shapes; without `parsed.path` matching, an unrelated
        // line/name collision in another file could falsely register as taint.
        "identifier" => {
            let name = node_text(arg_node);
            path.edges.iter().any(|e|
                e.to.file == parsed.path
                && e.to.line == call_line
                && e.to.var_name() == name
            )
        }

        // Complex expression (call, selector, binary, …) — conservative: ANY
        // identifier descendant that's tainted in the path → tainted arg.
        // Phase 1.5 keeps this conservative; tightening (e.g., only specific
        // positions in a selector chain) is Phase 2+.
        _ => arg_node.descendants_recursive().any(|child|
            arg_node_taints_match(parsed, child, call_line, path)
        ),
    }
}

fn line_matches_structured_sink(parsed, line, sink_pat, path) -> SinkMatchOutcome
where path: Option<&FlowPath>
{
    // existing call_path + semantic_check logic...

    // NEW: per-arg taint check — only when a FlowPath is provided.
    // `path == None` is the source==sink no-originating-path fallback (see
    // §2.3); in that case we trust the existing call_path + semantic_check
    // gate without per-arg precision, preserving today's source==sink
    // behavior for shapes like `c.File(c.Param("f"))`.
    if let Some(p) = path {
        let any_arg_tainted = sink_pat.tainted_arg_indices.iter()
            .any(|&i| arg_is_tainted_in_path(parsed, call_node, i, p));
        if !any_arg_tainted {
            return SinkMatchOutcome::SemanticallyExcluded;  // call_path matched but no relevant arg tainted on this path
        }
    }

    SinkMatchOutcome::Match(sink_pat)
}
```

**Interaction with the rolled-back PR #73 chassis.** Returning `SinkMatchOutcome::SemanticallyExcluded` for "no relevant arg tainted" lands cleanly on the post-rollback engine because `SemanticallyExcluded` is a runtime no-op — the flat-pattern catch-all keeps firing normally. That is exactly the desired behavior: the structured layer is more precise (no spurious tainted-binary fires when arg isn't tainted), but the flat-pattern fallback continues to provide its (over-permissive) coverage on `Command`/`Sprintf`/`Exec` substrings. P1 (pwsh fallback) and P2 (same-line unrelated sink) regression guards from PR #73 continue to pass under this design — verified explicitly in §3.7. **Engine plumbing additions:** thread `Option<&FlowPath>` through `line_matches_structured_sink` and `go_sink_outcome`; add `find_sink_with_inline_framework_source` for the source==sink secondary fallback; restructure the source==sink loop to combine Match-detection and per-path cleansing in a single per-path loop, then attempt secondary inline-source fallback when that combined loop finds no fire (the existing `path_states.iter().any(!c)` shape is wrong post-DFG because it includes non-matching paths in the cleansing decision — see §2.3 + the implementation plan's Task 5).

### 2.2 Aggregation: `tainted_arg_indices: &[0, 1]` semantics

Spec §3.2 already accepts conservative whole-slice semantics for `syscall.Exec(argv0, argv, …)`: *"any tainted element taints the slice as a whole."* Generalizing to `tainted_arg_indices: &[i, j, …]`: **any-tainted** semantics — sink fires if AT LEAST ONE indexed arg is tainted on this path. This matches both:
- Existing line-granular over-approximation (any taint on the line → fire).
- The intent of `os.Rename(old, new)` with `&[0, 1]` — either path being tainted is a CWE-22 risk.

### 2.3 Path-aware dispatch — interaction with FlowPath

The current `go_sink_outcome(parsed, line)` is path-agnostic. Per-arg DFG is per-path: the same sink call could be tainted via path A but not path B. Two options:

- **(a) Pass `path: Option<&FlowPath>` into `go_sink_outcome`.** Cleanest semantically. The forward-flow loop iterates per-path and passes `Some(path)`. The source==sink loop has two sub-cases: when `paths.iter().filter(originating)` is non-empty, iterate and pass `Some(path)`; when empty (the `c.File(c.Param("f"))` shape — no FlowEdge connects source to sink because they share a line), pass `None` to skip the per-arg check. The `None` branch falls back to today's call_path + semantic_check gate, preserving source==sink behavior. The function-body cleanser scan below the sink-pat resolution handles cleansing as before.
- **(b) Compute per-arg taint into a side-table once per (path, line, arg_idx).** Mid-build memo. Considered for perf if (a) re-walks excessively.

Pick (a). The `Option<&FlowPath>` shape is the natural per-path-or-no-path expression. The call-walk cost is O(args) per sink eval, dominated by the existing `collect_go_calls` walks. Path B (memo) is a Phase 2 hardening if profiles surface a hot spot.

**Why not synthesize an empty FlowPath for the no-originating-path case?** That was the original draft's plan, but it has a subtle bug: with an empty `path.edges`, `arg_is_tainted_in_path` returns false for every non-literal arg, including args that should be tainted by virtue of containing a framework-source call (like `c.Param("f")` inside `c.File(c.Param("f"))`). The empty-path approach would silently drop the `c.File` finding for the canonical source==sink shape. Using `Option<&FlowPath>::None` and falling back to call_path + semantic_check matching is the correct semantic — "we don't have per-arg precision here; trust the structured call_path gate."

**Mixed same-line case — secondary inline-source fallback.** Gating the no-path fallback only on `originating.is_empty()` is insufficient. Consider a line that hosts BOTH:

1. A source whose result is consumed by a non-inline downstream sink — generates a FlowPath whose first edge originates on this line (so `originating` is non-empty for this line).
2. An inline source==sink shape (e.g. `c.File(c.Param("f"))`) — `c.Param`'s result is consumed inline by `c.File`, no FlowEdge generated.

Concrete: `other := c.Query("other"); c.File(c.Param("f")); fmt.Println(other)` — all three on one line. Source 1 (`c.Query` → `other`) generates a FlowPath, making `originating` non-empty. The per-arg DFG branch runs, but its conservative recursion sees `c.File`'s arg[0] = call expression `c.Param("f")` and only checks identifier descendants against FlowPath edges (which track `other`, not `c.Param`). No Match. The inline `c.File`/`c.Param` source==sink is silently dropped — the same line-scoping problem class that caused the PR #73 review issues.

**Fix: secondary inline-source fallback in the source==sink loop.** When the originating-paths branch finds no matching-uncleansed path, call `find_sink_with_inline_framework_source(parsed, line)` directly. The helper scans **all** (call, sink_pat) combinations on the line independently — not first-match-wins like `go_sink_outcome` — and returns the first sink pattern whose tainted_arg subtree contains a descendant call to a known framework source. It walks only the sink's tainted_arg subtrees (not the whole line) and only accepts call_expression descendants whose call_path matches the active framework's source patterns. If `Some(pat)` is returned, treat as inline-tainted and fire (modulo `function_body_cleansed_for`). If `None`, suppress.

**Why exhaustive scan, not first-match-wins?** Routing through `go_sink_outcome(.., None)` (first-match) would shadow inline-bearing sinks behind unrelated structured sinks earlier on the same line. Concrete: `exec.Command("ls"); c.File(c.Param("f"))` — `go_sink_outcome` aggregates first-match-wins and returns `exec.Command`'s tainted-binary pattern. The inline-source guard checks only that pattern's tainted_arg ("ls" — no inline source), returns false, and the inline `c.File(c.Param)` is never considered. The dedicated helper iterates all sink patterns × all calls on the line independently, so c.File's tainted_arg is checked separately and the inline c.Param is detected.

**Why function-scoped request-param collection?** `detect_framework_sources` collects `*http.Request` / `*gin.Context` parameter names from the **enclosing function** of each line. The helper must mirror this scope. A file-wide collection would treat `c.Param` calls in any function as valid framework sources whenever any other handler in the same file binds `c *gin.Context` — a false-positive class. Function-scoped collection ensures `c.Param` is only recognized when `c` is bound in the enclosing handler's signature.

**Empty-function-scope guard.** The function-scope correction must be paired with an empty-collection short-circuit: when `request_param_names.is_empty()`, the helper returns `None` *before* it populates `source_paths`. `detect_framework_sources` already does this (taint.rs ~L794: `if param_names.is_empty() { continue; }`). Without the guard, non-prefixed source patterns like `mux.Vars` would still be added to `source_paths` because `concretize_source_call_path` passes non-prefixed `call_path`s through unchanged. That would re-create the same false-positive class through the back door — `mux.Vars` recognized as a framework source in functions that don't bind a `*http.Request` parameter. The function-scope warning above is the spatial half of this guard; the empty-collection short-circuit is the corresponding "empty function-scope" half.

The dedicated helper avoids the pitfall the reviewer flagged in earlier iterations: simply running the no-path fallback unconditionally would re-introduce the variable-bound-to-literal over-fire class (the entire reason Path C exists today). The inline-source guard ensures the secondary fallback only activates for the genuinely-inline source==sink syntactic shape, scoped to the binding context of the enclosing function.

### 2.4 Path C scaffolding retirement

After per-arg DFG lands:

- `check_command_taintable_binary` / `check_commandcontext_taintable_binary` become structurally redundant: a literal arg has no identifier → not in any taint set → arg-not-tainted check fails → `SemanticallyExcluded`. The Path C semantic_checks would also independently return false (because `literal_arg` returns `Some`), so they're consistent with the new check but redundant.
- **User-visible behavior is unchanged for the literal-binary case** — `exec.Command("ffmpeg", "-i", taintedInput)` already does not produce a structured tainted-binary `taint_sink` finding under Path C, and continues not to under per-arg DFG. The flat-layer `Command` substring fires either way (acknowledged false-positive class).
- The user-visible benefit is the **variable-bound-to-literal case**: `bin := "ffmpeg"; exec.Command(bin, ...)` currently produces a structured tainted-binary finding (Path C passes because `bin` is a non-literal identifier). Post-DFG, `bin` is not in the taint set → no fire. One fewer spurious `taint_sink` per affected line.
- **Recommendation:** remove the helpers + restore `semantic_check: None` on the two tainted-binary `SinkPattern` entries in the same PR. Update spec §3.2 to remove the Path C interim language and replace with: *"`semantic_check: None` because per-arg taint resolution at sink-eval time is the structural gate."*
- Spec amendment for `tainted_arg_indices` doc-comment in `src/frameworks/mod.rs`: drop "NOT consulted" language; replace with a description of the any-tainted semantics.

## 3. Test coverage

### 3.1 Variable-bound-to-literal regression (the eval-team probe shape)

The original draft used `exec.Command(bin, ...)` for this test, but the flat `Command` substring fires regardless of per-arg DFG behavior — and `sink_lines` line-dedupes findings, so a `count <= 1` assertion can't distinguish "structured + flat" from "flat-only." We use a **structured-only sink** (`c.File`, framework-gated for gin, NOT in flat `SINK_PATTERNS`) so absence-of-finding is genuinely diagnostic.

```rust
#[test]
fn test_taint_cwe22_cfile_variable_bound_to_literal_no_finding() {
    // Phase 1.5 (#1) discriminating regression. `bin = "/etc/static.txt"` is a
    // literal-bound variable; pre-DFG, the structured c.File pattern fires
    // because the line is line-tainted (input from c.Query is referenced on
    // the same line). Post-DFG: arg[0] = bin identifier; bin is not in any
    // FlowEdge — defined as a literal — so arg[0] is not tainted →
    // SemanticallyExcluded → no fire. c.File has no flat-pattern overlap,
    // so absence of any taint_sink on the call line discriminates.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	input := c.Query("input")
	bin := "/etc/static.txt"
	_ = c.File(bin); _ = input
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    let sink_line_finding = result.findings.iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 8);
    assert!(
        !sink_line_finding,
        "c.File(bin) where bin is bound to a literal should NOT fire post-DFG, \
         even when an unrelated tainted variable shares the line"
    );
}
```

This is the eval-team probe shape (variable-bound-to-literal) translated to a structured-only sink so the test is genuinely discriminating. `c.File` is framework-gated (gin's SINKS) with `tainted_arg_indices: &[0]` and `semantic_check: None` — exactly the pattern shape per-arg DFG should restrict.

### 3.2 Multi-index sink — one tainted, one not

```rust
#[test]
fn test_taint_cwe22_os_rename_one_tainted_arg_fires() {
    // os.Rename has tainted_arg_indices: &[0, 1]. If only the second is tainted,
    // any-tainted semantics still fires.
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
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([7]));
    assert!(has_taint_sink(&result), "os.Rename with tainted arg[1] should fire even though arg[0] is literal");
}
```

### 3.3 Positive control — bare tainted-binary

Already covered by `test_taint_cwe78_gin_tainted_binary` — tainted `bin` from `c.Query`, `exec.Command(bin, "--help")`. After per-arg DFG, the path's edges include `bin` as a tainted target on the call line; sink fires correctly.

### 3.4 Negative control — taint flows past the call without entering it

```rust
#[test]
fn test_taint_cwe22_cfile_literal_arg_with_unrelated_line_taint_no_finding() {
    // Phase 1.5 (#1) negative control on a structured-only sink. Tainted
    // `input` is referenced on the same line as a c.File call whose arg[0] is
    // a literal. Pre-DFG: line-granular over-fire (line tainted via input).
    // Post-DFG: arg[0] = "/etc/static.txt" literal → not tainted →
    // SemanticallyExcluded → no fire. c.File has no flat overlap; absence of
    // taint_sink on the call line discriminates.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	input := c.Query("input")
	_ = c.File("/etc/static.txt"); _ = input
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    let sink_line_finding = result.findings.iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 7);
    assert!(
        !sink_line_finding,
        "c.File with a literal arg should NOT fire post-DFG even when an unrelated \
         tainted variable shares the line"
    );
}
```

This pins the false-positive class beyond just the variable-bound-to-literal shape — taint that flows *near* the sink but doesn't enter any tracked arg should not fire. The choice of `c.File` (structured-only, no flat overlap) ensures the absence-of-finding assertion is meaningful.

### 3.5 Source==sink positive regression — `c.File(c.Param("f"))`

```rust
#[test]
fn test_taint_cwe22_cfile_inline_param_source_still_fires() {
    // Source==sink shape. c.Param is the source AND its return value is
    // c.File's arg[0]. No FlowEdge connects them (no intermediate variable),
    // so paths.iter().filter(originating) is empty. The source==sink loop's
    // no-originating-path branch passes None for the FlowPath argument; the
    // engine falls back to call_path + semantic_check matching (skipping
    // per-arg DFG) and fires. This pins the no-path fallback semantic.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	c.File(c.Param("f"))
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    assert!(
        has_taint_sink(&result),
        "c.File(c.Param(\"f\")) source==sink shape must still fire post-DFG \
         via the Option<&FlowPath>::None fallback in the source==sink loop"
    );
}
```

Without this test, the source==sink no-path fallback could silently regress to "no-fire" if a future change replaces the `Option<&FlowPath>` plumbing with a synthetic-empty-path approach. The existing `c.File`/`c.Param` source==sink behavior in the current engine (pre-DFG) is what this test pins forward.

### 3.6 Mixed same-line source==sink — inline-source secondary fallback

```rust
#[test]
fn test_taint_cwe22_cfile_inline_param_with_parallel_path_still_fires() {
    // P2 mixed same-line regression. The single-line function body has
    // TWO sources:
    // 1. c.Query("other") — generates a FlowPath via def-use of `other`
    //    (consumed by fmt.Println on the same line).
    // 2. c.Param("f") inline inside c.File(c.Param("f")) — no FlowPath
    //    because its result is consumed inline by c.File.
    //
    // Source 1's FlowPath makes `originating` non-empty for this line, so
    // the basic Option<&FlowPath>::None fallback (gated on
    // originating.is_empty()) is skipped. Without the secondary inline-
    // source fallback (§2.3), the inline c.File(c.Param) source==sink
    // would be silently dropped — the same line-scoping problem class
    // that caused the PR #73 review issues.
    //
    // Post-fix: `find_sink_with_inline_framework_source` detects
    // c.Param as an inline framework source inside c.File's arg[0],
    // triggering the secondary `go_sink_outcome(.., None)` branch which
    // fires c.File (modulo function-body cleansing).
    let source = r#"package main

import (
	"fmt"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	other := c.Query("other"); c.File(c.Param("f")); fmt.Println(other)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "c.File(c.Param(\"f\")) on a line with a parallel c.Query/FlowPath \
         must still fire post-DFG via the inline-source secondary fallback"
    );
}
```

This test is the third leg of the line-scoping correctness triad:
- §3.5 — pure source==sink (no other source on line) → fires via `Option::None` primary fallback.
- §3.6 — mixed same-line (another source has a FlowPath) → fires via secondary inline-source fallback.
- The PR #73 P1/P2 regression guards (in §3.7 below) — non-source==sink shapes → flat fallback preserved, no spurious dual-layer suppression.

If §3.6 fails, the secondary fallback's `find_sink_with_inline_framework_source` guard either isn't being called or isn't recognizing inline framework sources correctly. Audit `concretize_source_call_path`'s prefix substitution against the active framework spec.

### 3.5 Existing test re-validation

All 1,438 tests must continue to pass. Particular attention to:
- `test_taint_cwe78_gin_shell_wrapped` — shell-wrapped + tainted payload at arg[2]. arg[2] is `cmd` (tainted from `c.Query`). Per-arg should pass.
- `test_taint_cwe22_gin_os_readfile` — `os.ReadFile(name)` with tainted `name` at arg[0]. Per-arg passes.
- The 10 sanitized fixtures (`tests/fixtures/sanitizer-suite-go/sanitized/`) — cleansed_for suppression remains the primary mechanism. Per-arg DFG sits before cleanser suppression (if no arg tainted, no further check needed; if arg tainted, then cleanser suppression decides).

### 3.6 Suppression-rate harness

`tests/integration/cwe_phase1_suppression_test.rs` should remain at 10/10 (100%). If it drops, regression in per-arg DFG (probably the conservative complex-expression branch is too narrow). If it goes to 11/10 (over-100%), test setup bug.

### 3.7 PR #73 P1/P2 regression guards remain green

Both regression tests added in `583d114` MUST continue to pass under per-arg DFG:

- `test_taint_cwe78_pwsh_shell_wrapper_fires` — `exec.Command("pwsh", "-c", taintedInput)` should produce a `taint_sink` finding. Public results still have flat `Command` overlap, so structured PowerShell recognition is pinned by the internal `powershell_shell_wrappers_match_structured_registry` unit test.
- `test_taint_cwe78_same_line_unrelated_sink_preserved` — literal-binary `exec.Command(...)` + `db.Exec(taintedQuery)` on the same line: the `db.Exec` finding must not be hidden by the structured exclusion of the `exec.Command`. Per-arg DFG: `exec.Command` returns `SemanticallyExcluded` (literal binary, no relevant arg tainted). `db.Exec` is not in any structured registry → `NoMatch`. Aggregate is `SemanticallyExcluded` (because at least one pattern was call_path-matched). Currently no-op → flat layer fires on `Exec` substring → finding produced. ✓

If either regression test fails after per-arg DFG lands, that's a sign the no-op behavior of `SemanticallyExcluded` was accidentally re-coupled to flat suppression. Stop and audit before continuing.

## 4. Out of scope for Phase 1.5

- **CFG-aware paired_check direction** (queue item #2) — orthogonal; defer to Phase 1.6 or roll into Phase 2 if eval expands fixtures with inverted-guard shape.
- **Tree-walk caching** (queue item #3) — pure perf; defer.
- **Exotic shell paths** — literal-list extension for `/usr/bin/sh`, `/usr/local/bin/bash`, etc.; defer until fixtures exercise it. PowerShell wrappers are covered by the Phase 1.5 shell-list follow-up.
- **Per-arg taint for non-Go languages** — Phase 2 (Python) will introduce framework specs that may want similar precision; the engine helper (`arg_is_tainted_in_path`) should be Go-agnostic by construction.
- **Field-sensitive arg taint** — `req.Body.Read(buf)` where `req.Body` is the tainted field, not `req`. Existing `AccessPath` machinery in `src/access_path.rs` could power this; out of scope for Phase 1.5 but worth flagging.

## 5. Implementation plan (preview — not the full plan)

Anticipated 2-3 commits, ~300-500 LOC delta net:

1. **Commit 1 — Per-arg taint resolution.** Add `arg_is_tainted_in_path` + `arg_node_taints_match` helpers (with `e.to.file == parsed.path` file-scoping guard) in `taint.rs`. Add `find_sink_with_inline_framework_source` + `subtree_has_call_in` helpers for the source==sink mixed-line secondary fallback. Refactor `line_matches_structured_sink` and `go_sink_outcome` to take `Option<&FlowPath>`. Update the two call sites: forward-flow loop passes `Some(path)`. Source==sink loop has three branches: `originating.is_empty()` → primary `Option::None` fallback; non-empty + per-path Match-and-cleansing combined loop → fire if any matching path is uncleansed; non-empty + no matching-uncleansed → secondary inline-source fallback gated on the new helper, for mixed same-line shapes. Add 6 regression tests (§3.1, §3.2, §3.3 [existing positive control], §3.4, §3.5, §3.6).

2. **Commit 2 — Retire Path C scaffolding.** Remove `check_command_taintable_binary` + `check_commandcontext_taintable_binary` + `check_arg_is_non_literal_at` helpers. Restore `semantic_check: None` on the two tainted-binary `SinkPattern` entries. Update spec §3.2 + the doc-comment on `tainted_arg_indices` in `src/frameworks/mod.rs`.

3. **Commit 3 (if scope allows)** — Update STATUS-prism-cwe-phase1.md priority queue to mark item #1 closed; eval team will re-validate against C1 fixtures.

PR title: `Phase 1.5 (item #1): per-arg DFG — honor tainted_arg_indices`.

## 6. Risks / open questions

1. **Path mismatch for source==sink shapes** — addressed by the `Option<&FlowPath>::None` primary fallback (`originating.is_empty()` case) PLUS the secondary inline-source fallback for mixed same-line cases (§2.3). Pure source==sink (no other source on the line) uses the primary fallback. Mixed same-line source==sink (another source on the line has a FlowPath, making `originating` non-empty) uses the secondary fallback gated on `find_sink_with_inline_framework_source`. Cleansing for both fallback paths uses `function_body_cleansed_for`. The secondary helper is bounded and avoids re-introducing the variable-bound-to-literal over-fire class — see §2.3 for the why.

2. **`AccessPath` interaction** — `VarLocation.path: AccessPath` carries field-sensitive info. Phase 1.5 starts with `var_name()` (`path.base`) for matching, ignoring sub-fields. Acceptable; field-sensitive matching is Phase 2+.

3. **Taint via aliasing** — `bin := tainted; alias := bin; exec.Command(alias, ...)`. The DFG should already track this through transitive def-use chains. Verify with a regression test in 3.1.

4. **Arg-extraction edge cases** — variadic args (`fmt.Sprintf(format, args...)`) — `args...` spreads a slice. Currently no Phase 1 sink uses variadic semantics. Note as a Phase 2+ shape but not blocking.

## 7. Acceptance criteria

- The variable-bound-to-literal class (eval-team probe) is suppressed at the structured layer for sinks without flat overlap (`c.File(bin)` where `bin = "/etc/static.txt"` and a tainted variable shares the line — regression test §3.1 passes). For sinks WITH flat overlap (`exec.Command(bin, ...)`), the structured tainted-binary is suppressed but the flat `Command` substring still fires as today; that's expected post-rollback behavior and not in scope for this PR.
- `os.Rename("/tmp/static.txt", taintedDest)` still fires via any-tainted semantics on `tainted_arg_indices: &[0, 1]` (regression test §3.2 passes).
- `c.File(c.Param("f"))` source==sink shape continues to fire via the `Option<&FlowPath>::None` primary fallback (regression test §3.5 passes).
- Mixed same-line source==sink (e.g. `other := c.Query(...); c.File(c.Param("f")); fmt.Println(other)` — three statements, one line) continues to fire via the secondary inline-source fallback gated on `find_sink_with_inline_framework_source` (regression test §3.6 passes). This pins the line-scoping correctness class that PR #73's review iteration repeatedly flagged.
- All Phase 1 positive tests still pass (1,438 unchanged).
- Sanitizer suppression rate still 10/10.
- PR #73 P1/P2 regression guards remain green (`test_taint_cwe78_pwsh_shell_wrapper_fires` and `test_taint_cwe78_same_line_unrelated_sink_preserved` both pass — see §3.7).
- Path C semantic_checks removed; spec §3.2 updated; `tainted_arg_indices` doc-comment in `src/frameworks/mod.rs` updated.
- Eval-team re-runs C1 validation on the new build; the variable-bound-to-literal shape they flagged is now suppressed at the structured layer. Their literal-binary fixed.go fixtures will continue to show flat-layer `taint_sink` until further flat-catch-all policy work lands; PowerShell shell-wrapper coverage is handled separately by the Phase 1.5 shell-list follow-up.
