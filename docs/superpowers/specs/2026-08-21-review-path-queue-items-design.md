# Review-path follow-up queue — four bounded items (design for spec review)

Date: 2026-08-21. Base: `main` @ `47e21ae` (PR #168 merged). Source of the queue:
`docs/analysis/prism-post-plan-roadmap.md` §1 items #4, #4b, #6, #7 and
`docs/superpowers/pipeline-lessons.md` §follow-up. Owner focus: code-review usage on
Go / Java / Rust / JS-TS-Node / Python; precision/accuracy and noise reduction over recall-at-any-cost.

Standing constraints (apply to all four): consumer-visibility doctrine (nothing below Exact feeds an
asserted finding); precision floor for Rust/Go (drop-not-fanout); ONE cache transition per PR; safe
failure direction stated per item and verified by tests; full suite green + `cd eval && uv run tier-a
--matrix-only --allow-stale-sut` 0 regressions for anything touching `src/cpg/`, `src/resolution.rs`,
`src/call_graph.rs`, `src/ast.rs`, or taint. Each item ships as its own branch/PR.

---

## Item A (#4) — Multi-line-call Step-5b arg→param edge gap

**Problem / signal.** `g(\n    user\n)` produces no arg→param DataFlow edge, so interprocedural taint
(P14 descent, `taint_reaches`, the taint algorithm) silently reports NotReached through the callee.
Pinned today by `src/cpg/tests.rs::test_multi_line_call_shape_is_currently_not_descended` (line ~872,
asserting the *absence* of descent as an out-of-scope Stage-A pin). This is a review false negative.

**Mechanism (grounded at HEAD).** `src/cpg/build.rs` (production helper, ~L880–947) calls
`caller_parsed.call_argument_texts_at(site.start_byte, &site.callee_name)` → `Vec<String>` of arg
texts, then for each arg builds `arg_key = (caller.file, caller.name, caller.start_line, site.line,
arg_path, VarAccess::Use)` (fallback `Def`) and looks it up in `var_index`. Var nodes are keyed by the
line the occurrence sits on; for a multi-line call the argument's identifier is on a later line than
`site.line`, so the lookup misses and no `CpgEdge::DataFlow` is pushed. A second, serial copy of the
same loop exists ~L1000–1060 ("par==serial twin for the parallelization oracle") — both must change.

**Design.**
1. `src/ast.rs`: add `call_argument_texts_and_lines_at(start_byte, callee_name) -> Vec<(String, usize)>`
   (1-indexed start line of each argument node; same argument enumeration as the existing helper).
   Keep `call_argument_texts_at` as a thin wrapper (callers elsewhere unchanged).
2. `src/cpg/build.rs`, BOTH copies: for each `(arg_text, arg_line)`, look up the arg var node at
   `arg_line` first; if absent, fall back to `site.line` (recall-preserving: any edge resolvable today
   is still resolvable). Lookup order per line stays Use → Def. Same for the field path AND the base
   path (the PR #113 supplement-not-replace behavior is unchanged).
3. One `CACHE_VERSION` bump (DataFlow edges are persisted in the CPG cache).

**Safe failure direction.** Under-recording keeps today's NotReached (status quo); over-recording would
bind an unrelated same-named variable. The line-scoped lookup cannot bind across statements: the key still
requires the exact `(caller fn, line, access path)`; an argument node's own line can only hold that
argument's occurrence(s) of the name. A same-named variable on a *different* line inside the call span
is not reachable because the key line is the argument's own start line.

**Tests (TDD).** Flip the pinned test to assert descent (rename to `..._is_descended`). Add fixtures:
Python, Go, Rust, JavaScript/TypeScript, Java — shapes: one-arg-per-line, trailing comma, first arg on
the call line + second on the next line (mixed), and a nested multi-line call as an argument. Negative:
a same-named local assigned on another line within the call's span must not bind. Keep all P14 Stage-A
tests green.

**Acceptance.** `cargo test` full; `tier-a --matrix-only` 0 regressions; `prism nav call-stats` on
ripgrep/caddy byte-identical (no resolution change; only DFG edges); the new edge count on a
self-host `taint_reaches` smoke is >0 on a multi-line fixture.

**Non-goals.** Return-flow taint (queue #2); recursion descent; depth-lock relaxation (queue #8).

**Reviewer questions.** (a) Is the arg-line-first/site-line-fallback order the right precision call, or
should site-line fallback be removed once arg lines exist? (b) Any language where the argument node's
start line ≠ the identifier's line (decorators/labels/spread) that would still miss?

---

## Item B (#7) — Sanitizer advisory tier cross-language matching (noise + false suppression)

**Problem / signal.** P10 (#159) added `SanitizerRecognizer.languages` and gated ONLY
`taint::sanitizer_call_site` (the verdict-path matcher). The advisory tier —
`function_body_cleansed_for` (taint.rs ~L10626) and `cleansed_categories_for_source` (~L10725),
which feed `sanitizers_present_in_source_fn` / the `Cleansed` warning AND the CWE sink-suppression
engine (`cleansed_for` marks) — iterates `crate::sanitizers::active_recognizers()` unfiltered by
language (deliberately left "byte-for-byte" by P10; documented as a plausible follow-up at ~L10786–10800).
Effect: a bare `escape` recognizer (registered by BOTH JS_TS and PYTHON tables) matches a Go
`escape(...)` call; `html.escape` (Python) matches a JS `html.escape(...)`; etc. Two consequences:
(1) advisory noise on polyglot repos; (2) a **false CWE sink suppression** — a hidden finding. (2) is
the unsafe direction per engineering doctrine 7.

**Mechanism (grounded at HEAD).** Tables: `SHELL_RECOGNIZERS` is EMPTY; `PATH_RECOGNIZERS` are Go-only
(`languages: &[Language::Go]`, paired-check family); `JS_TS_RECOGNIZERS` JS/TS/Tsx; `PYTHON_RECOGNIZERS`
Python. `sanitizer_supported(language)` (sanitizers/mod.rs L34) is a hand-maintained second source of
truth = {Go, Python, JavaScript, TypeScript, Tsx} and gates the advisory entry points at taint.rs
L10701/L10733 and `reasoning/sanitizer_walk.rs` L154 — but only by FILE language, not by recognizer.

**Design.**
1. In `function_body_cleansed_for` and `cleansed_categories_for_source`, skip recognizers where
   `!recognizer.languages.contains(&parsed.language)` (identical predicate to `sanitizer_call_site`).
2. Derive `sanitizer_supported(language)` from the tables:
   `active_recognizers().any(|r| r.languages.contains(&language))` — provably equal to the current set
   (Go via PATH, Python, JS/TS/Tsx) and removes the documented second source of truth.
3. Update the P10 doc comment at ~L10786–10800 (the "advisory tier stays unfiltered" paragraph).

**Safe failure direction.** Removing a cross-language match can only (a) remove a false `Cleansed`
advisory and (b) un-suppress a sink finding that was suppressed by a wrong-language recognizer. It cannot
suppress anything new. A same-language match is unchanged.

**Tests (TDD).** Go file with a user-defined `escape(x)` feeding an XSS-category sink → NOT cleansed,
no `Cleansed` warning, finding present (today: suppressed/advised — red first). JS file calling Python's
`markupsafe.escape` path → not cleansed. Python `html.escape` positive unchanged. A test asserting the
derived `sanitizer_supported` equals the previous hardcoded set for every `Language` variant. Run
`cargo test --test algo_taxonomy taint_cve_test::` and the full suite; enumerate and justify EVERY
fixture/expected.toml flip (expected class: only removed false suppressions) — no blind re-baseline.

**Acceptance.** Full suite green; `tier-a --matrix-only` 0 regr; the taint CVE suite green with each
flip explained in the PR body.

**Reviewer questions.** (a) Is there any legitimate cross-language recognizer use (e.g. a Go project
calling a JS sanitizer name via cgo/wasm) that this would lose? (b) Should the paired-check family stay
advisory-only as today (yes, unchanged) — confirm no interaction.

---

## Item C (#6) — `--review-no-diagrams`

**Problem / signal.** P1 (#149) collapsed `--format review`; the residual is that diagram payloads
(`diagrams: Vec<SliceGraph>` at result level — `CompactReviewOutput.diagrams`, review_compact.rs L99 —
and per-finding `SliceFinding.diagrams`, slice.rs ~L42) dominate the compacted output (552 KB post-P1,
"diagrams are most of it"). Review agents pay that in tokens.

**Design.** Add `--review-no-diagrams` (clap, `--format review` only, documented alongside
`--review-min-severity` / `--review-full-slices`; ignored with a stderr note for other formats, matching
how the other review-only flags behave). When set: `to_compact_review_output` emits `diagrams: vec![]`
and strips `diagrams` from each retained finding; `diagram_warnings` are kept (small; `--strict-diagrams`
exit-code semantics unchanged); skip `finalize_diagrams`' Mermaid rendering when set (pure compute save;
warnings that are produced during finalize — check — must still be produced; if finalize is the sole
source of `DiagramWarning`s, do NOT skip it, only strip the payload). Both fields already carry
`skip_serializing_if = "Vec::is_empty"` so the keys vanish; without the flag output stays byte-identical.

**Safe failure direction.** Omitting diagrams loses only visualization, never findings/slices.

**Tests.** CLI validation test (`tests/cli/validation_test.rs` style): with the flag, no `diagrams` key
anywhere in the review JSON (results + findings), `diagram_warnings` still present when produced; without
the flag, output byte-identical to today's golden; `--format json` unaffected by the flag.

**Acceptance.** Full suite; a size comparison on one self-host review run recorded in the PR body.

**Reviewer questions.** Keep `diagram_warnings` in the JSON when diagrams are suppressed, or move them to
stderr-only under the flag?

---

## Item D (#4b) — Go dot-import resolution (`. "pkg"` → bare-name calls bind to that package)

**Problem / signal.** Measured recall gap (tier-a baseline.md 2026-07-04): four zap `New(...)` sites in
`package observer_test` that dot-import `go.uber.org/zap/zaptest/observer` were adjudicated `prism_fn`.
Post-P13 same-package partitioning (package clause), `observer_test` ≠ `observer`, so the bare call has no
same-package candidate and the dot-import is the only binding path.

**Mechanism (grounded at HEAD).** `src/ast.rs::extract_go_import_spec` (~L2376–2395) records
`alias → path` but explicitly DROPS `_` and `.` aliases (`if local != "_" && local != "." { insert }`),
so dot-imports leave no trace. Import-qualified `pkg.f()` resolution narrows by the import path's last
segment vs the package directory (`resolution.rs` ~L247–278, `resolve_go_owner_identity` and the R4 Go
package-directory narrowing). The unqualified-call rungs consider local free defs → same-package
(clause-partitioned, P13) → cross-file free functions (non-Go) / drop.

**Design.**
1. `ParsedFile`: new `go_dot_imports() -> Vec<String>` (import paths with a `.` alias), collected by the
   same walker; the alias map stays unchanged (do NOT insert `"."`). Plumb into `CallGraph` as a per-file
   list next to `imports`.
2. Resolution: in the Go unqualified-call path, AFTER local free def and same-package (clause) candidates
   fail, for each dot-imported path resolve the package directory with the SAME import-path→dir convention
   R4 uses; candidates = exported free functions named N declared in files whose package clause is that
   directory's non-test package (respecting P13 partitions and build constraints). Emit Exact only when
   exactly ONE directory matches the import path AND exactly ONE function matches; otherwise drop with
   the existing drop attribution (precision floor: never fan out; never NameOnly for Go).
3. Go spec guarantees no collision between dot-imported names and package-block names in a compiling
   program, so local/same-package-first ordering is both correct and the conservative choice.
4. Non-Go languages: byte-identical. One `CACHE_VERSION` bump (call edges are cached).
5. `prism nav call-stats`: count the new kind under an explicit `go_dot_import` resolution-kind/counter so
   the win side is measurable (doctrine 10: a new rung whose success counter never fires is presumed
   broken until a positive corpus case is reproduced).

**Safe failure direction.** Drop on any ambiguity (two candidate dirs with the same last segment; two
functions; shadowing by a local/same-package def). A missed binding is today's behavior; a false Exact
would be a precision regression — the tests below cover both poles.

**Tests (TDD).** (1) zap shape: `pkg/observer/observer.go` (`package observer`, `func New()`),
`pkg/observer/observer_test.go` (`package observer_test`, `import . "mod/pkg/observer"`, bare `New()`)
→ Exact edge to `observer.New`, kind `go_dot_import`. (2) Two directories `a/observer` and `b/observer`
both exporting `New`, dot-import of one → must still bind (path narrows) — and if the convention cannot
disambiguate, drop (assert drop, not fan-out). (3) Dot-import plus a same-package `New` defined locally →
local wins (no dot-import edge). (4) Dot-imported package has no `New` → no edge. (5) Non-Go file with a
`.`-like import shape unaffected. (6) A fixture in `eval/fixtures/go/` with `expected.toml` so tier-a
matrix covers it.

**Acceptance.** Full suite; `tier-a --matrix-only` 0 regr; `prism nav call-stats --repo` on caddy and
prometheus: drops byte-flat or reduced, no new Exact outside the `go_dot_import` counter, Rust (ruff or
ripgrep) byte-identical; zap (bench repo if present) shows ≥4 new Exact at the adjudicated sites.

**Reviewer questions.** (a) Should dot-import candidates include exported methods' receivers/types
(e.g. `T{}` composite literals) — proposed NO (calls only; types are not call targets). (b) Is the
module-path→dir convention in `resolution.rs` sufficient when the repo has a `go.mod` module path with a
prefix not mirrored by directories (e.g. `go.uber.org/zap` rooted at repo root) — does `go_build_profile.rs`
expose a module root we should prefer? (c) Interaction with P5 function-value callbacks and P11 receiver
typing — any rung that consults `imports` and would now need `go_dot_imports` too (doctrine 6: second
copies drift — please grep `imports` consumers).
