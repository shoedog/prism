# Parallel-session kickoff — R6: split `taint.rs` + relocate sanitizers

> **Survey/intro brief** for a fresh session. Self-orient from here — this points you at the
> authoritative docs, the subsystem, the workflow, and **what another session is actively working on so
> you avoid conflicts.** Write your own plan once oriented.

## What this task is
`src/algorithms/taint.rs` is **>10k lines** (the codebase's largest file by far; CLAUDE.md asks for
<600-line files). Split it into focused modules and **relocate sanitizer logic into `src/sanitizers/`**
(the directory already exists as the target). This is **mechanical + behavior-preserving**, pinned by
existing fixtures — the goal is structure, not new behavior.

## Why it matters
- **A4 layering inversion (real, tracked debt):** the reasoning layer reaches *into* taint internals
  (e.g. `cleansed_categories_for_source`, ~`taint.rs:10680`); pulling sanitizers out to `src/sanitizers/`
  fixes the inversion and stops per-language sanitizer growth compounding in a monolith.
- It pairs with the A2 `compute_bindings` extraction (already planned). Est. ~2–4 days, low risk.

## Where to look (read these first)
- **`docs/features/cpg/substrate-analysis-2026-06-10.md`** — authoritative. **F11** ("`taint.rs` is >10k lines …
  the A4 layering inversion is real, tracked, paired with A2; `src/sanitizers/` already exists as the
  target"); **§4 R6** (cost/benefit/risk + the mitigation: the `algo_taxonomy_sanitizers*` fixtures are
  byte-pinned — "the proven A4 technique"); the "Valuable but safely AFTER (additive)" list.
- **`CLAUDE.md`** — the taint algorithm map (`taint.rs` → forward taint), the **<600-line / split-by-
  category** rule, and the test structure (`tests/algo/taxonomy/` taint CVE/sink/lang; one umbrella
  `[[test]]` per `tests/` subdir).
- **Source:** `src/algorithms/taint.rs` (the monolith), `src/sanitizers/` (existing target dir),
  `src/reasoning/` (the consumer reaching in — note the boundary you're cleaning). Tests/fixtures:
  `tests/algo/taxonomy/` (the byte-pinned sanitizer/taint fixtures that lock behavior).

## What the OTHER (main) session is doing — AVOID these surfaces
The main session is iterating a **Rust (→ general) name-resolution scope-graph spec**
(`docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`, HELD at spec stage under
codex re-review). At implementation it will own: `src/resolution.rs`, `src/ast.rs` import/`mod`
extraction, `src/navigation/module_graph.rs`, `src/call_graph.rs`, the incremental `src/cpg/build.rs`
path, `src/cpg_cache.rs`, a new scope-graph module — **don't touch these**, and avoid per-language
*name-resolution* work.
- **R6 is in the taint/sanitizer subsystem — fully disjoint from name resolution. No expected conflict.**
  (If you find taint touching `call_graph`/`resolution`, that's a consumer call — don't refactor those
  files; keep your changes inside `taint.rs`/`sanitizers/`/`reasoning` boundaries.)

## How to work (project conventions)
- Branch off `main` (not the main session's `rust-*`/spec branches). One concern per PR.
- **Behavior-preserving is the contract:** keep the byte-pinned fixtures green at every step — they are
  your safety net (move code, re-run `cargo test --test algo_taxonomy`, confirm byte-identical). This is
  refactor-by-extraction, not a rewrite.
- Plan-driven: `superpowers:writing-plans` → `superpowers:subagent-driven-development` (TDD/fixture-
  pinned); optional codex 2nd-opinion via `a2a-bridge` (`~/code/a2a-bridge`).
- Before PR: `cargo fmt && cargo test` (+ `--features mcp`). Mind the **3 copies** of `all_test_files`
  in `tests/integration/coverage_test.rs` if you add/rename test files (CLAUDE.md). Keep each new module
  <600 lines.
