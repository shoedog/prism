# Parallel-session kickoff — S4: scope-honesty / unmodeled-construct warnings

> **Survey/intro brief** for a fresh session. You will self-orient — this just points you at the
> authoritative docs, the subsystem, the workflow, and (important) **what another session is actively
> working on so you avoid conflicts.** Keep it scoped; write your own plan once oriented.

## What this task is
Per traversed function, detect **language constructs prism does not model** and emit a
`WarningKind::Reasoning(..)` coverage note, so a reasoning verdict (`NotReached` / a tri-state
`Reachability`) caused by an *unmodeled construct* is distinguishable from a *proven* non-flow. Examples
to name: Rust `?` / closures crossing the flow; Go `go` spawn / channel send-recv / `select`; JS/TS
`await` / `.then()`; Python comprehension bindings / `with`. It is a **per-language node-kind blocklist
+ a warning emission** — additive, no behavior change to existing edges.

## Why it matters
The reasoning layer's value to an LLM is *trustworthy* tri-state answers. A `NotReached` that's really
"we didn't model the `await` chain" must never read as a safety proof. This extends the established
honesty pattern (`path_proven:false`, `BoundaryExited`). Bonus: the warning telemetry is what should
*prioritize* future per-language DFG modeling — so doing S4 first has compounding value. Est. ~1–2 days.

## Where to look (read these first)
- **`docs/features/cpg/substrate-analysis-2026-06-10.md`** — the authoritative source. §3 "Do BEFORE Plan B"
  defines **S4** (its exact intent + cost); **F6/F7/F8** enumerate the per-language unmodeled constructs;
  §5 component verdicts; §2.4 the `Reason::Reasoning`/`WarningKind::Reasoning` quarantine.
- **`docs/eval/tier-a/baseline.md`** — "Next-increment work-lists" item references the honesty gap.
- **`CLAUDE.md`** — the `reasoning/` layer + `Reason`/`Evidence.reasoning` (additive, byte-safe) notes.
- **Source:** `src/reasoning/` (esp. `shape.rs` — the tri-state `Reachability`/`BoundaryExited` is the
  honesty precedent to extend); the `WarningKind` enum + its `Reasoning(..)` variant (grep
  `WarningKind` in `src/navigation/` / `src/reasoning/`); `src/languages/mod.rs` (the per-language
  node-type mappings — your blocklist lives near here); the CFG/DFG traversal (`src/cfg.rs`,
  `src/data_flow.rs`) where per-function constructs are visited.

## What the OTHER (main) session is doing — AVOID these surfaces
The main session is iterating a **Rust (→ general) name-resolution scope-graph spec**
(`docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`, currently HELD at spec stage
under codex re-review). When it reaches implementation it will own these files — **do not touch them**:
`src/resolution.rs`, `src/ast.rs` import/`mod` extraction, `src/navigation/module_graph.rs`,
`src/call_graph.rs` (imports/scope-graph build), the incremental path in `src/cpg/build.rs`,
`src/cpg_cache.rs` (cache version), and a new scope-graph module. **Also avoid any per-language
*name-resolution* work** (Python `from_import_alias`/`inherited_override`, TS namespace merging, JS/Rust
block scoping) — the scope graph subsumes it.
- **Shared touch-point to coordinate (additive only):** `src/languages/mod.rs` node-type mappings — both
  efforts may add to it. Keep your additions additive (new node-kind queries for the blocklist); don't
  refactor existing mappings.
- S4 lives in the **reasoning/honesty** layer — otherwise disjoint from name resolution. Low conflict.

## How to work (project conventions)
- Branch off `main` (don't reuse the main session's `rust-*`/spec branches). One concern per PR.
- Plan-driven: `superpowers:writing-plans` → `superpowers:subagent-driven-development` (TDD); optional
  codex 2nd-opinion via `a2a-bridge` (see `~/code/a2a-bridge` configs) — the main session has been using
  it for plan + code review.
- Before PR: `cargo fmt && cargo test` (+ `--features mcp` if you touch MCP). Reasoning changes are
  additive to `Evidence.reasoning` (byte-compatible — keep it that way). See CLAUDE.md "Before Creating
  a PR".
- The capability matrix (`tests/integration/coverage_test.rs`) + Tier-A harness (`eval/`) are the
  regression nets; a new `WarningKind` variant is additive but run the suite.
