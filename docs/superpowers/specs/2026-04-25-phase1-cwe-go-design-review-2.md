# Review of `2026-04-25-phase1-cwe-go-design.md` — iteration 2

**Date:** 2026-04-25
**Reviewer:** Prism agent (independent pass; did not author the design)
**Subject:** [`2026-04-25-phase1-cwe-go-design.md`](2026-04-25-phase1-cwe-go-design.md)
**Scope:** Verification of §1 + §2 changes from [iteration-1 review](2026-04-25-phase1-cwe-go-design-review.md); first pass on §3 (sink + sanitizer registries). §4 still pending; not reviewed.

---

## Verdict

- **§1 + §2 changes:** All seven feedback items addressed cleanly — verbatim adoption in most cases. Two stale references in §1 Commit 1's bullet to clean up, otherwise locked.
- **§3:** Three real issues to resolve, four small clarity items. Solid architectural skeleton; the issues are mostly "the design doesn't yet specify *how* one piece works" rather than "the design is wrong."

---

## §1 + §2 — feedback addressed ✅

| Item | Where fixed | Note |
|---|---|---|
| Real issue: §1 Commit 1 ↔ §2.5/2.8 inconsistency | line 21 — verbatim adoption of suggested rewrite | ✅ |
| S1: CWE-78 tainted-binary case | lines 29–33 — both shell-wrapped AND tainted-binary as separate `SinkPattern` entries | ✅ |
| S2: `FrameworkRegistry` enum unused | §2.4 rewritten — enum dropped, free function `detect_for`, reasoning explicit | ✅ |
| S3: `tainted_arg` field name | line 94 — renamed to `taints_arg`, doc-comment clarified | ✅ |
| S4: multiple `*http.Request` params | line 169 — bind all matching parameter names; rationale included | ✅ |
| S5: no-framework baseline test | line 225 — added with explicit ACK §3 Q5 reference | ✅ |
| Optional polish: §2.3 framing | line 115 — verbatim adoption of suggested rewrite | ✅ |

### Two small stale references in §1 Commit 1

Line 19 still reads:

> New `src/frameworks/mod.rs` defining `FrameworkSpec`, **`FrameworkRegistry`**, and **the detection trait**.

Both bolded phrases are now stale after §2.4's enum-removal. Suggested fix:

> New `src/frameworks/mod.rs` defining `FrameworkSpec`, `ALL_FRAMEWORKS`, and the `detect_for` dispatch function.

One-line cleanup; not blocking.

---

## §3 — review

### Real issues

#### R1 — §3.5: `TaintValue` struct doesn't exist today; integration with existing types unspecified

The §3.5 sketch:

```rust
pub struct TaintValue {
    // ...existing fields (origin, source_line, ...)...
    pub cleansed_for: BTreeSet<SanitizerCategory>,
}
```

But there's no `TaintValue` struct in `src/algorithms/taint.rs` today. Taint propagation works through `crate::data_flow::FlowPath` (a sequence of `FlowEdge`s) and `VarLocation`. The "tainted value" abstraction is implicit in the DFG.

Adding `cleansed_for` requires picking *where* sanitizer state lives. Three plausible choices:

1. **New `TaintValue` wrapper struct** in `taint.rs` that wraps `VarLocation` with metadata. Existing taint engine threads `TaintValue` instead of bare `VarLocation`. Largest refactor; cleanest result.
2. **Augment `VarLocation`** with `cleansed_for`. Per-location, not per-flow-path — would mean two flows reaching the same use-site couldn't disagree on cleansing status. Possibly wrong granularity.
3. **Side-table** `BTreeMap<VarLocation, BTreeSet<SanitizerCategory>>` keyed by location, threaded through the engine. Smaller refactor; clutters call sites.

The doc's framing is closest to (1) but it doesn't pick. **Pick one and add a paragraph** describing the integration shape. This is the highest-leverage clarification for §3 — the rest of §3.6 / §3.7 dangle on this choice.

#### R2 — §3.9: Shell-escape cleanser appears redundant

`exec.Command` has both:

- **Sinks** in `GO_CWE78_SINKS` with `semantic_check` excluding the safe `Command(literal_non_shell, args...)` form.
- **Cleansers** in `SHELL_RECOGNIZERS` with `check_not_shell_wrapper` matching the same safe form.

For `exec.Command("ls", tainted)`:

- Tainted-binary sink: `arg[0] == "ls"` (not tainted) → no fire.
- Shell-wrapper sink: `arg[0] == "ls"` (not in shell list) → `semantic_check` fails → no fire.
- Result: no sink fires. No cleansing needed for the suppression check.

The cleanser sets `cleansed_for(OsCommand)` on the *result* (`*exec.Cmd`), but no downstream sink in the Phase 1 design consumes the `*exec.Cmd` and checks for OsCommand cleansing. So the cleanser is a no-op in practice.

Compare to path-validation, where the cleanser is genuinely necessary:

```go
cleaned := filepath.Clean(tainted)         // cleanser fires; result marked cleansed_for(PathTraversal)
data, _ := os.ReadFile(cleaned)            // sink consumes `cleaned`; suppression check passes
```

Here the cleanser changes outcome.

**Two options for §3.9:**

- **Drop the shell cleanser** — rely solely on `semantic_check` exclusion at the sink. Simpler design, fewer moving parts, less work at runtime.
- **Keep it** for symmetry / future-proofing (e.g., when `(*exec.Cmd).Output()` becomes a sink in a later phase) — but document explicitly that it's a no-op today and explain the forward-looking justification.

Either is defensible; the design should pick. As written, the doc presents the cleanser as if it does something today, which is misleading.

#### R3 — §3.2: `syscall.Exec` slice-element taint behavior unspecified

```rust
SinkPattern {
    call_path: "syscall.Exec",
    category: SanitizerCategory::OsCommand,
    tainted_arg_indices: &[0, 1],  // 1 is the slice; element-level taint via slice analysis
    semantic_check: None,
},
```

The comment defers to "slice analysis" without specifying which: Prism's existing DFG treats slices conservatively (any tainted element taints the slice as a whole; taint flows through `slice[i]` reads). Is that the intended behavior here?

If yes: state it. "`tainted_arg_indices: &[0, 1]` means: arg 0 (`argv0`) is checked directly; arg 1 (`argv` slice) fires the sink if the slice itself is taint-flagged in the DFG, which captures the case where any element was assigned from a tainted source." If the design intends per-element tracking, that's a bigger change — Prism's DFG isn't currently element-aware for slices, so the spec should say so.

One-paragraph clarification fixes this.

### Small clarity items

#### S1 — §3.8: Rel-form paired_check direction ambiguity

```rust
SanitizerRecognizer {
    call_path: "filepath.Rel",
    category: SanitizerCategory::PathTraversal,
    semantic_check: None,
    paired_check: Some("strings.HasPrefix"),  // negated check on result starting with "..": same matcher
},
```

The comment notes the check is *negated* (caller wants the result to NOT start with `..`), but `paired_check` is textual co-occurrence in the function body — it doesn't distinguish positive from negative use. Both of these would suppress the finding:

```go
// CORRECT use — guard rejects bad path:
if strings.HasPrefix(rel, "..") { return error }
data, _ := os.ReadFile(filepath.Join(base, rel))
```

```go
// INCORRECT use — guard ACCEPTS bad path:
if strings.HasPrefix(rel, "..") { data, _ := os.ReadFile(filepath.Join(base, rel)) }
```

Both contain the literal `strings.HasPrefix(rel, "..")` in the function body, so both would suppress. Document this explicitly: "Rel + `strings.HasPrefix(rel, '..')` paired check fires regardless of whether the prefix check is used as a positive or negative guard. False-positive suppression possible if the guard is logically inverted; CFG-aware enhancement deferred to Phase 1.5." Already mentioned in §3.8 as a future improvement, but the *Rel-specific* version of the limitation isn't called out.

#### S2 — §3.10: C1 fixture validation is external

The test plan covers synthetic minimal examples (~42 tests across Commits 2 and 3) and an in-tree suppression-rate fixture suite (10+10). But Phase 1 acceptance criterion 1 is "taint fires on at least one CWE-78 + CWE-22 example **in C1 fixtures**" — and C1 fixtures live in `~/code/agent-eval/cache/prism-cwe-fixtures/` (per ACK §6), not in this repo.

Add one sentence: "C1 fixture validation runs externally via the eval-team validation cadence (ACK §6 / handoff §9). Phase 1 in-tree tests pin synthetic minimal examples; criterion 1 is validated post-merge by the eval team."

#### S3 — §3.1: Two sink-storage shapes coexist; migration footnote

Existing `SINK_PATTERNS: &[&str]` (string patterns) and new `GO_CWE78_SINKS: &[SinkPattern]` (structured) coexist. The taint engine consults both. Worth a one-line footnote: "Future migration: existing `SINK_PATTERNS` could be promoted to `SinkPattern` shape for uniform handling; deferred to keep this PR scoped." Otherwise readers may wonder whether the two registries should be unified.

#### S4 — §3.4: `paired_check` resolution is textual co-occurrence

The `paired_check: Option<&'static str>` field uses a string name. §3.8 line 418 says implementation walks the function body looking for the literal token, but §3.4 doesn't say so explicitly. Adding "Resolved at suppression time by textual co-occurrence in the same function body — not type-safe binding to a Rust function" makes the binding model unambiguous.

### Stylistic / nits

- **§3.2:** `check_shell_wrapper` and `check_shell_wrapper_ctx` could be one helper `fn check_shell_wrapper_at(call: &CallSite, bin_idx: usize, flag_idx: usize) -> bool`. Minor; saves duplication.
- **§3.2:** Shell binary list (`sh` / `bash` / `cmd.exe` / `/bin/sh` / `/bin/bash`) covers common cases; misses PowerShell (`pwsh` / `powershell.exe`) and exotic absolute paths (`/usr/bin/sh`, `/usr/local/bin/bash`). Worth a one-line scope note: "Common Linux/Windows shells only; exotic paths and PowerShell deferred."
- **§3.3:** Note about `filepath.Join` not being a sink is good. Could add the same note for `path.Join` (lower-level package, same semantics) or leave implicit.

### Strengths

- **Cross-cutting vs framework-gated sink separation** in §3.1 is the right architectural cut. Reusing `SinkPattern` shape for both storage locations keeps the sink-evaluation code path unified.
- **Concrete `SinkPattern` consts with index-layout comments** (`// ctx, sh, -c, X` etc.) make the arg positioning reviewable at a glance — much better than abstract sketches.
- **Dual-role of `exec.Command`** (sink + cleanser, inverse `semantic_check`s) is unusual but well-documented in §3.9 once you accept the redundancy concern (R2).
- **`BTreeSet` not `HashSet`** for `cleansed_for` — explicitly chosen for deterministic test snapshots. Right call.
- **Heuristic limitations accepted explicitly per handoff** (§3.8 paragraph after the recognizer entry). Doesn't overclaim precision.
- **Test plan splits Commit 2 / Commit 3** with clear acceptance-criterion mapping and an integration test pinning the ≥80% suppression rate. Right shape.
- **`filepath.Join` footnote** in §3.3 anticipates a common confusion. Good defensive doc-writing.

---

## Recommendation

Address R1 (`TaintValue` integration shape — pick one of the three options) before §4 lands; this is the load-bearing decision that the rest of §3.5–3.7 dangle on. R2 (shell cleanser redundancy) and R3 (slice-element taint) are smaller — pick a position and add a paragraph each. The four small items can roll into the final pass alongside §4. The two stale §1 references are one-line cleanups whenever convenient.

Then signal and the reviewer will move to §4 (test strategy + PR cadence + branch strategy).
