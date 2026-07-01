# Merged Plan Review — Prism Tier-2 Plan A (Substrate Hardening)

Both lenses (Executability, Coverage) ran successfully; no lens is missing. Both independently corrected their own first drafts (the trace.rs "private field" breakage and the render-arm "compile blocker" were retracted by both; Coverage fixed a wrong file citation for `Location`). The merged findings below already incorporate those corrections.

---

## BLOCKERS (plan will not compile / a test fails / a commit lands unproven)

**B1 — `build_python_cpg(src)` doesn't exist (Tasks 1 Steps 2/7/8, Task 5, Task 7).**
Issue: every test snippet calls this helper, but it exists only in the plan doc; the real harness is inline `ParsedFile::parse(path, src, Language::Python).unwrap()` → `BTreeMap` insert → `CodePropertyGraph::build(&files)` (src/cpg/tests.rs:98-102). Verbatim execution → `cannot find function build_python_cpg`.
Fix: add an explicit step defining `fn build_python_cpg(src: &str) -> CodePropertyGraph` wrapping that 3-line pattern; reuse across Tasks 1/5; Task 7 (separate crate) needs its own copy.
*Lens split: Executability=BLOCKER, Coverage=MAJOR. BLOCKER is correct — it's a literal compile error, which is Executability's domain.*

**B2 — Task 5 Step 3 imports private modules.**
Issue: `use crate::cpg::build::CodePropertyGraph;` and `crate::cpg::trace::{Trace, Relation}` from `crate::reasoning` (a sibling of `cpg`) hit private `mod build`/`mod trace` → E0603. (Task 1's trace.rs is fine — it's a `cpg` descendant.)
Fix: use the re-exports Task 1 Step 5 adds: `use crate::cpg::{CodePropertyGraph, Trace, Relation};`.

**B3 — Task 5 Step 3 builds `Location { file, line }`; the struct has three fields.**
Issue: `Location` (src/navigation/types.rs:4-8 — the one shape.rs imports; cpg/types.rs has no `Location`) is `{ file, start_line, end_line }`. Compile error.
Fix: `Location { file, start_line: line, end_line: line }`.

**B4 — Task 5 tri-state test asserts `BoundaryExited` at the wrong line.**
Issue: the test asserts `reachability_at(..., line 2) == BoundaryExited` for `sink(p)`, but A3's cross-function edge is arg→parameter, so `BoundaryEdge.to` is the parameter definition at the callee's function-start line (line 1, `def g(p):`), not line 2 (data_flow.rs:204-249, cpg/build.rs:387-400). The assertion yields `NotReached` and fails as written.
Fix: assert the parameter line, or implement boundary classification that recognizes "boundary target would flow to this sink inside the callee" without traversing it. *(Unique to Executability — its strongest single finding.)*

**B5 — Task 4 `Evidence.reasoning` ripple is broader than the staged/proven set.**
Issue: `Evidence { … }` constructors also live in src/navigation/queries.rs, src/navigation/module_graph.rs, tests/navigation/types_test.rs, and src/mcp/output.rs — the last inside a `#[cfg(test)]` module, so even `cargo build --features mcp` won't catch it. The commit stages only types.rs + output/navigation.rs.
Fix: stage every constructor file and prove with `cargo test --test navigation_types` AND `cargo test --features mcp` (a feature *build* is insufficient).

**B6 — Per-task proofs don't honor the global Option-C contract (ordering defect).**
Issue: the plan (line 13) requires `cli_nav_compat` + `fmt` + `cargo build` + `cargo build --features mcp` after every task, but Task 2 omits builds+mcp (:391), Task 3 omits cli_nav_compat (:435), Task 4 omits builds+mcp (:531), Task 5 omits builds+mcp (:670), Task 6 omits cli_nav_compat+default build (:735). Commits could land without compiling all touched feature/test surfaces.
Fix: bring every task's final proof up to the full Option-C set before its commit boundary.

**B7 — Task 7 Step 1 is a placeholder, yet Step 3 expects it green (false-green).**
Issue: the smoke-test body is entirely comments; it will "pass" trivially without exercising anything.
Fix: write a concrete body — public imports, build a CPG via `prism::cpg::CodePropertyGraph::build(&files)` (the cpg field is public but `CpgContext<'a>` is lifetime-bound, so the tests.rs pattern is simpler), call `cpg.taint_trace(...)`, and assert one `Reached` and one `BoundaryExited` — accounting for the B4 boundary-target line.

---

## MAJORS (gaps to close before "done"; not compile blockers)

**M1 — Task 1: same-line handling diverges from design B1, and its must-pass test is missing.**
Issue: `cfg_valid`'s `is_on_a_source_line` hard-returns `false` and never receives `source_line`, contradicting design B1 (`target.line == source.line` must be valid). The spec marks a same-line propagation test must-pass-before-done, and it's absent; worse, the implementer note references a "Step-6 same-line test" that doesn't exist (Step 6 is the straight-line test).
Fix: add the spec-mandated same-line fixture (e.g. `x = source(); y = x` on one line) asserting y's def in `frontier` + a dead-end-free witness. If it passes on seeding alone, vindicate the no-op stub and correct the note; else thread `source_line` into `cfg_valid`.
*Lens split: Coverage=BLOCKER (unmet done-gate), Executability=MAJOR (impl divergence). MAJOR is right: the code likely behaves correctly because `taint_trace` seeds every Variable at the source line (Coverage itself retracted its "ships a bug" framing and could not falsify the seeding argument). The real defects are a missing deliverable + an internally inconsistent note — neither breaks compilation nor existing tests.*

**M2 — Tasks 1/5/7: three more spec-mandated A3 must-pass tests are absent.**
Issue: no-path `Trace`, absent-CFG pure-taint fallback (`cfg_valid !has_cfg => true`, mirroring taint_forward_cfg at cfg_queries.rs:133-135), and DataFlow-wins-same-line tie determinism (spec §3:55, implemented in `taint_neighbors` but never asserted) can all regress silently.
Fix: add a no-CFG fixture test and a tie-break test asserting the DataFlow parent label wins. *(Unique to Coverage.)*

**M3 — Task 1 Step 7: the no-dead-end-witness test doesn't prove the invariant.**
Issue: it only asserts the terminal walkback node is in `frontier`; a frontier node with no parent passes trivially without proving it's an original source root.
Fix: build a root set from the seeded source nodes and assert every frontier member's parent walkback terminates in that set. *(Unique to Executability.)*

**M4 — Task 2: the A4 adapter signature diverges from both specs (silent cross-plan contract change).**
Issue: Plan A §4:84 and taint-reaches §4:55 pin `cleansed_categories_for_source(source: VarLocation) -> Vec<String>` (the name §8 says Plan B consumes by name); the plan implements `(parsed: &ParsedFile, source_line: usize)`. `function_body_cleansed_for` genuinely needs `&ParsedFile` (taint.rs:10581), so the plan's shape is more implementable — but Plan B's documented call won't typecheck.
Fix: surface the divergence and pick a resolution — keep the `VarLocation` public surface and resolve `VarLocation → &ParsedFile` inside the adapter via the session `files` map, or amend both specs. Don't change it silently. *(Executability noted this but left it unranked; Coverage's dual-spec citation is the stronger framing.)*

**M5 — Task 2 Step 6: the proof targets the wrong unchanged surface.**
Issue: it names `algo_taint_cve`, but A4's relevant byte-unchanged surface is the sanitizer-taxonomy fixtures.
Fix: add `cargo test --test algo_taxonomy_sanitizers` and `cargo test --test algo_taxonomy_sanitizers_python`. *(Both lenses converge on sanitizer fixtures as the relevant surface.)*

**M6 — Missing task: no dated/issue obligation for the A4→`src/sanitizers/` relocation.**
Issue: spec §4:91-93 requires the temporary layering inversion (reasoning reaching into taint.rs) be tracked by a dated/issue `[→plan]` obligation landing in A2/Phase-IP; no task records it, so it becomes untracked permanent debt.
Fix: add a step recording the dated TODO/issue paired with A2. *(Unique to Coverage.)*

---

## MINORS

**m1 — Task 4 Step 5: "render()/WarningKind matches are exhaustive" is inaccurate.**
`render()` has a catch-all `Reason` arm (output/navigation.rs:73) and renders `WarningKind` via `{:?}` (:80); taint-reaches §7:108-109 itself confirms additive variants are byte-safe. New arms are optional, and if added must precede the catch-all to avoid an unreachable-pattern warning. Fix: drop the "exhaustive" framing. *(Both lenses.)*

**m2 — Task 5: `node_of` discards Def/Use identity.**
It hardcodes `access: "use"`, `ordinal: 0`, throwing away the `kind` that `to_var_location → VarLocation` carries (data_flow.rs:13-22). taint-reaches §5:63 makes full node identity load-bearing to avoid merging a Def and a Use on one line. MINOR because it's a Plan-A scaffold, but flag so Plan B doesn't inherit the collapse. Fix: derive `access` from `VarLocation.kind`. *(Both lenses.)*

**m3 — Plan "Key APIs" mislabels `SanitizerCategory` as "Debug-only."**
It actually derives `Copy, Ord, …` (frameworks/mod.rs:27), which the adapter's `BTreeSet<SanitizerCategory>` + `.map(|r| r.category)` relies on (so it compiles — but the annotation misleads an implementer). Fix: correct the derive annotation. *(Unique to Coverage.)*

**m4 — Task 5: new public `Verbosity` enum contradicts the design.**
shape.rs introduces `pub enum Verbosity { Concise, Detailed }`, but the design says reuse the existing concise|detailed convention with "no new enum." Widens API surface accidentally. Fix: drop the enum. *(Unique to Executability.)*

**m5 — Task 7: test path `tests/reasoning/smoke_test.rs` doesn't match the spec's `tests/reasoning_*.rs` wording.**
Not a compile blocker once the `[[test]]` path is registered, but an internal contract mismatch. Fix: align path/name with the spec convention. *(Unique to Executability.)*

**m6 — Missing CLAUDE.md update for the new `src/reasoning/` module and additive `Evidence.reasoning` field.**
The authoritative module/MCP-surface enumeration goes stale. Fix: one-line module/field note (here or with Plan B). *(Unique to Coverage.)*

---

**Verdict: not executable as-is.** Fix the 7 blockers first — the four Task-5 hard errors (B2 private imports, B3 `Location` fields, B4 boundary-line assertion) plus the missing `build_python_cpg` helper (B1), the under-staged/under-proven `Evidence.reasoning` ripple (B5), the per-task Option-C omissions (B6), and the Task-7 placeholder (B7) — then close the same-line must-pass test (M1), the A4 cross-plan signature (M4), and the three remaining spec-mandated A3 tests (M2) before building. Decomposition and build ordering are otherwise sound and nearly every cited API checks out.