# S2 Implementation Plan — Codex Plan-Review (xhigh)

**Date:** 2026-06-13 · **Plan under review:** rev 1 (commit `19d6848`) →
**outcome: rev 2.** Spec: rev 4 → **rev 5** (§7.3 wording + §5 audit-cite fixes
folded here too).

**Reviewer:** codex (gpt-5.5, **xhigh**) via a2a-bridge
`run-workflow plan-review-codex` (`examples/a2a-bridge.slicing-plan-review-codex.toml`):
5 nodes — exec-readiness draft + coverage draft → each refined → synth. Prism MCP
attached (rebuilt from S3-merged main, host cache cleared+warmed). Raw:
`/tmp/s2-plan-review-codex.md`.

**Verdict: not executable as-is.** Core decomposition sound; ~12 BLOCKERS + ~10 MAJORS,
almost all **compile-surface completeness** ("enumerate every caller/literal" — rev 1's
"compiler-guided" hand-waving) plus three substantive gaps. A full-codebase inventory
(read-only Explore sweep) backs every enumeration folded into rev 2.

## The three substantive gaps (not just enumeration)

1. **DFG `defs`/`uses` re-key omitted.** Spec §4 requires the maps re-key to include
   `function_start_line`; rev 1 never operationalized it. Without it the interprocedural
   edge builder (`data_flow.rs:555`) re-merges what the CPG `var_index` just de-conflated.
   → rev 2 **Task 3** now migrates `var_index` AND DFG `defs`/`uses` together (type +12
   inserts +2 retains +2 iter-destructures, all enumerated).
2. **Tuples instead of named records.** rev 1 widened extractor return tuples; spec §4
   mandates named records. → rev 2 **Task 4** introduces `PathSpan`/`StatementSpan` (ast
   layer) lifted into `VarLocation` (which *is* the spec's `VarOccurrence`), via
   **sibling** methods so line-only callers (`cfg.rs`, `left_flow.rs`, `queries.rs`,
   algorithms, `import_test.rs`) stay intact.
3. **Task 5 test backwards + spec wording bug.** In `q = p`, `q` (lhs) is at a *smaller*
   byte than `p` (rhs), so ascending `start_byte` orders **def-q before use-p** — the
   opposite of rev 1's assertion and of spec §7.3's "use-of-y precedes def-of-x." The sort
   is for *determinism + source fidelity*; data-flow direction is the edge label. → rev 2
   Task 5 asserts the raw `nodes_at` byte order; **spec rev 5** corrects §3/§7.3.

## BLOCKERS → disposition (rev 2)

| # | Blocker | Disposition |
|---|---|---|
| 1 | `VarAccessKind` lacks `Hash` → `identity_key().hash()` won't compile | Task 1/S3 derives `Hash`. |
| 2 | New fields only partially propagated (taint.rs, `src/cpg/tests.rs` literals) | Task 1 enumerates all 28 `tests.rs` + 9 `taint.rs` literals. |
| 3 | `func_index` direct users missed (`function_at`/`callers_of`/`callees_of`/`callers_of_in_file`) | Task 2 enumerates all (name retained in key / routed via `name_index`). |
| 4 | `var_index` direct users missed (`all_defs_of`) | Task 3 enumerates. |
| 5 | DFG `defs`/`uses` re-key not implemented | Task 3 (gap #1 above). |
| 6 | Task 4 widening breaks public callers | Sibling APIs (gap #2). |
| 7 | Task 5 ordering test contradictory | Corrected + spec rev 5 (gap #3). |
| 8 | `Location`/`SymbolRef` shared wire — field add breaks beyond nav queries | Task 6 enumerates 19 `Location` + 12 `SymbolRef` literals (GraphNode unaffected — threads existing values). |
| 9 | `function_calls_on_lines` non-call-graph callers | Task 8 sibling `…_with_spans` method. |
| 10 | Cache tests wrong API (write_cache/JSON) | Task 9 uses real `save_cache`/`load_cache`/bincode in the in-crate `#[cfg(test)]` module (models `wrong_grammar_fingerprint_misses`). |
| 11 | Task 10 placeholder dumps | Reframed as capture-then-freeze (generate→eyeball→paste); not a blocker, made explicit. |
| 12 | Cross-task helpers non-existent (`build_rust_cpg`, …) | **Pre-flight** task adds them to `tests/ast/common`. |

## MAJORS → disposition

Named records (T4) · real statement spans (T4) · anchor coverage incl. destructuring /
multiline-param / augmented-assign Def+Use / per-language (T4) · exact `(file,name,start_line)`
function lookup not ambiguous `function_node` (T6) · **`SCHEMA_VERSION` decision = bump
0.1→0.2** (T6; MCP wire gained byte fields; no test pins it) · `.function` audit widened to
`cfg_queries.rs:244` `!=` boundary + DFG name-scope (T7) · **byte-aware interprocedural
arg-binding explicitly deferred** (T8 + spec §9 — `call_argument_texts` stays line-keyed; a
pre-existing limitation the `CallSite` byte makes an additive follow-up) · assert raw
`nodes_at` order not test-side sort (T5).

## MINORS → disposition

`name_index` buckets sorted by `start_line` so `function_node` returns the lowest-line
overload (T2) · helper wording made precise (pre-flight).

## Key facts the inventory established (used by rev 2)

- DFG `defs`/`uses` key is `(file, function, AccessPath)` — **no line today**; re-key adds
  `function_start_line` only (line stays out, aggregation-by-function preserved).
- `SCHEMA_VERSION` (mcp/output.rs:11 = "0.1") is the MCP wire version, **separate** from
  `CACHE_VERSION` (cpg_cache.rs:45 = 4). Byte-on-wire bumps the former; node-schema bumps
  the latter.
- `AccessPath` and `VarAccess` already derive `Hash`; only `VarAccessKind` lacked it.
- Cache is **bincode**; `CacheResult::{Hit, PartialHit{…}, Miss}`; `build_incremental`
  reuses `assemble_graph`, so the Task-5 `location_index` byte-sort covers the PartialHit
  path for free.

**Nothing rejected. No redesign.** Every finding is an enumeration tightening or one of the
three substantive folds. rev 2 is executable; next is subagent-driven execution (codex
implementers + per-task spec/quality review), then codex full-branch review pre-merge.
