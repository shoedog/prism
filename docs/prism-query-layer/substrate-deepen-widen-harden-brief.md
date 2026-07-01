# Clean-room analysis brief — Deepening / widening / hardening the prism reasoning substrate (DFG · CFG · CPG)

> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

**For codex (clean-room), reading the actual repo at the session cwd.** Produce a prioritized,
concrete, code-grounded analysis. This brief is intentionally tool-agnostic: it is run once **now**
(filesystem only) as a baseline, and **re-run later with `prism-mcp` connected** — the two outputs are
compared to see what the navigation/reasoning tooling adds to your own analysis. Ground every claim in
specific files/functions/line ranges you actually read.

## What the substrate is
Prism is a defect-focused code-slicing + review tool that is growing a whole-repo navigation layer (Tier
1, shipped) and a reasoning layer (Tier 2, starting). The **reasoning substrate** is:
- `src/data_flow.rs` — `DataFlowGraph`: def-use edges, `forward_reachable` / `backward_reachable`,
  `chop`, `taint_forward`, `FlowPath`/`FlowEdge`, sanitizer (`cleansed_for`) modeling, `build` /
  `build_subset`.
- `src/cfg.rs` — control-flow graph construction (intraprocedural).
- `src/cpg.rs` — `CpgContext` / `CodePropertyGraph`: composes AST + DFG + CFG + call graph; build entry
  points (`build`, `build_scoped`, `build_with_cached_cpg`, `build_incremental`); `taint_forward_cfg`
  (CFG-pruned taint), `chop`.
- `src/access_path.rs` — field-sensitive access paths (`x`, `dev->name`, `self.config.timeout`).
- `src/call_graph.rs` — cross-file call graph (forward/reverse, cycles).
- Consumers: `src/algorithms/*` (the 26+ slicing algorithms), and the new `src/navigation/` + `src/mcp/`.

## The question
For each of the three structures — **data-flow graph, control-flow graph, code property graph** — analyze
concrete opportunities along three axes, prioritized by (value to defect-finding / Tier-2 reasoning) ÷
(effort + risk):

- **DEEPEN (precision):** field/path/flow sensitivity gaps; alias & container modeling; sanitizer model
  completeness; implicit/indirect flows; **type-awareness** (today call/edge resolution is by symbol
  *name*, not receiver *type* — quantify where this causes imprecision); CFG fidelity (exceptions,
  short-circuit, loops, early return); CPG edge-kind richness.
- **WIDEN (coverage/scope):** **interprocedural** data flow (the headline gap — param→arg, return→caller,
  bounded depth); per-language DFG/CFG completeness across the 11 grammars (which languages are thin?);
  async/closure/generator flows; whole-program vs per-function tradeoffs; new relationship kinds the CPG
  could carry.
- **HARDEN (correctness / robustness / perf):** known unsound or incomplete spots and their blast radius;
  test-coverage gaps in DFG/CFG/CPG *construction* (not just algorithm outputs); determinism; performance
  and memory of construction (note `cpg.rs` is large; identify hot paths, re-clone/re-serialize, and
  caching seams); robustness on malformed/oversized/low-parse-quality inputs.

## Constraints to respect in every recommendation
- **Option C additivity:** the existing diff-review and nav outputs must stay byte-for-byte stable; favor
  additive changes and call out anything that would force a golden re-baseline.
- The codebase is deterministic (sorted maps/sets), tree-sitter-based, multi-language. Respect the
  established module boundaries and the <600-line-file guideline.
- Tier-2 reasoning tools (`taint_reaches`, `dataflow_between`, `impact_of_change`, `what_missing`) are the
  near-term consumers — weight opportunities by how directly they unlock those.

## Deliverable
A prioritized roadmap, organized by structure (DFG / CFG / CPG) × axis (deepen / widen / harden), where
each item has: the concrete gap (with file:line evidence), why it matters (which defects / which Tier-2
tool it unlocks), rough effort & risk, dependencies/ordering, and the Option-C impact. End with a short
"top 5 highest-leverage moves" and the single biggest soundness risk you found.
