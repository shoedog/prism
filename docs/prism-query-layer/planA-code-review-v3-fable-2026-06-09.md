# Plan A substrate — in-depth code review round 3 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review` over the prism-wired host config
(`a2a-bridge.slicing-review.toml`), `ANTHROPIC_MODEL=fable`. Input was the FULL branch diff
vs `main` (merge-base `5e35d64`), not a single follow-on commit, per the in-depth re-review
policy. Codex = correctness lens, fable/claude = architecture lens, fable/claude = synth.

## Verdict as delivered
Do not merge until the 1 BLOCKER (same-line callee-parameter false negative) is fixed and the
two Plan B seam MAJORs (per-root API, source seeding) are closed or explicitly contracted.

## Disposition after verification

The BLOCKER did **not reproduce**. Probed empirically (Python and JS): a one-line callee
(`start_line == end_line`) gets **no param `Def` node and no arg→param edge**, so `taint_trace`
records **no boundary** — there is nothing for `data_flow.rs:232`'s `ref_line == start` skip to
drop and nothing for `reachability_for_node` to mis-classify. The reviewer's *symptom* (taint into
a one-line callee is dropped) is real but lives in **CPG construction**, affects the production
Taint slice equally, pre-exists this work, and is not Option-C-safe to fix in the reasoning layer.
The implied reasoning-layer fix (param-binding bridge) was implemented, shown untriggerable and
unable to fix the symptom, and removed. See `planA-followups.md` → "Round 3" for the probe detail.

The genuinely real reasoning-layer defect in that finding-cluster was the co-reported **MAJOR**:
`reachability_for_node` used the un-function-scoped `dfg_forward_reachable`, which leaks same-line
propagation across functions sharing a minified line (probe: reaches `["a:t:Def","b:c:Def"]`).
**Fixed** by routing boundary classification through `forward_reachable_in_function` — the same
function-scoped traversal the BFS uses — so the two engines can't diverge (probe: now
`["a:t:Def"]`). Pinned by `test_forward_reachable_in_function_is_function_scoped`.

Also fixed: `Trace.boundary` `Vec` → `BTreeSet` (dedup; MINOR).

The two seam MAJORs (per-root reachability/witness API; node/location-precise seeding) and the
remaining MINORs (`sink_nodes_at` ownership, `sanitizer_supported` source-of-truth, double-nested
JSON discriminant, boundary-classification memoization, `Trace` pub-field invariant, `node_of`
fallback) are **explicitly contracted** for Plan B in `planA-followups.md` — their final shape is
set by Plan B's `TaintedBy`/`SinkResult` contract, so Plan B's first tasks add them rather than
Plan A guessing the surface now.

Accepted as-is by both lenses: A4 layering inversion (documented relocation plan), additive
`reasoning: None` (test-pinned byte-compat), boundary-before-CFG-check ordering, the intentionally
dead `cleansed_categories_for_source` adapter awaiting its Plan B consumer.
