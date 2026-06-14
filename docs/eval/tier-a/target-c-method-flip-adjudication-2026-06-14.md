# Tier-A `target-c-method` flip — adjudication (2026-06-14)

Dual analysis (claude opus + codex gpt-5.5 xhigh via a2a-bridge, prism MCP; the rust LSP
`lsp-mcp` was wired but did **not** expose its tools in-session — both fell back to prism +
manual source resolution). Both lenses **agree**; codex added a key edge-materialization
refinement for the follow-on increment.

## The flip
Pin `target-c-method` (`eval/tier_a/pinned.py`) seeds `fn target` = `TaintSeed::target` at
`src/algorithms/taint.rs:1276` (a method whose NAME collides 269× across src/ — petgraph
`.target()`, the `target` field, …). Baseline (2026-06-11/12): `known_fail`, raw P=R=0.
S2 run (2026-06-13): `flip_candidate`, **raw P=0.208, R=1.0, tp=5, fp=19, fn=0**.

## Adjudication — RE-BLESS as a legitimate recall win (not a clean pass)
1. **Recall flip is GENUINE (R 0→1.0).** The 5 oracle sites (taint.rs:1763/4420/4430/4438/
   4450) are all explicit `TaintSeed::target(...)` calls; prism now resolves them with
   `qualified_owner` (they were `oracle_only` in the 2026-06-11 report). This is the
   improvement the pin was planted to detect: **S2 de-conflation made `fn target` a distinct
   `(file,name,start_line)` node, so its real callers resolve.**
2. **The 19 prism-only sites are real `prism_fp` — petgraph `EdgeRef::target` name-collisions**
   (verified via the `use ...EdgeRef` imports at trace.rs:6, build.rs:12, query.rs:9; the
   sampled sites — trace.rs:241, build.rs:542, query.rs:125, circular_slice.rs:118 — are all
   `e.target()`/`edge.target()`, not `TaintSeed::target`).
3. **Root cause (codex):** `CallGraph.callers` is keyed by raw callee NAME
   (call_graph.rs:53); nav gathers `target` candidates by name (call_resolve.rs:39); the **S3
   R6 single-owner NameOnly fallback** then maps unknown-owner `.target()` calls to the sole
   in-repo `target` method (resolution.rs:416). So the FPs are NameOnly/R6 edges.

## Implication for the next increment (exact-`FunctionId` traversal) — IMPORTANT
A naive `callers_of_node(NodeIndex)` reverse-traversal **will NOT** fix the precision on its
own: the CPG builder **materializes both `Exact` and `NameOnly` resolved calls as `Call`/
`Return` edges** (build.rs:348), so the false `.target()→TaintSeed::target` edges are
**already in the graph**. The increment must engage `ResolutionConfidence` — e.g.
`callers_of_node` filters to `Exact` edges, OR CPG `Call` edges carry their confidence so the
exact path can exclude NameOnly/R6. **`target-c-method` is the built-in success metric:**
after the increment, P should rise 0.208→~1.0 (the 19 petgraph FPs drop) with R held at 1.0.

## Baseline re-anchoring — scope (deliberate / human-triggered)
The committed `baseline.md` (2026-06-11/12, pre-S3, pre-S2) is stale. Per `eval/README.md`
(:103/:114) a re-anchor is a **full five-corpus run** (`prism, tokio, caddy, flask, click` —
`eval/corpora.toml`), not the dirty prism `--quick` report. Steps:
1. `cd eval && uv run tier-a --corpus all` (human-triggered).
2. Adjudicate the pending diffs (Rust/Go fully; bounded Python sample) — same protocol as
   S3's 26-diff pass; **do not re-baseline away pending regressions.**
3. Update `baseline.md`: **G1(b)** (no longer "`target` known_fail P=R=0" — record the
   adjudicated flip: P=0.208/R=1.0/tp5/fp19/fn0, FPs = `EdgeRef::target` by-name pollution),
   the corpus M1/M2 metrics, and the G1(a) strata gates; refresh the dated per-corpus reports.
4. **Pair with the still-open S3 re-anchor** — the corpus drifted across S3 *and* S2; re-anchor
   once for both. Also flip the `eval/tier_a/pinned.py` `target-c-method` `expected` from
   `known_fail` to the post-flip state (recall won; precision pending exact-`FunctionId`).
