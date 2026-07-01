# EFT whole-branch final review — 2026-06-14

Final dual review of the completed `exact-functionid-traversal` increment (10 task
commits + 4 docs commits on `main`), after all per-task codex-xhigh diff reviews +
operator merge-gate triage + green acceptance. Two host-RO reviewers, distinct lenses,
both with prism MCP (lsp-mcp not exposed):

- **codex gpt-5.5 xhigh** — correctness / cross-task integration. Verdict: **fix-then-ship**.
- **claude opus 4.8** — architecture / soundness / coherence. Verdict: **ship**.

Both confirmed the load-bearing design end to end: the two independent confidence
surfaces (CPG `Call/Return` edges for slices+node-traversal vs `NavCallEdge.confidence`
for nav, never mixed); the F7 two-substrate join (caller SET from `callers_of_node` ∩ site
LINES from `resolved_caller_edges`, keyed on caller `start_line` + resolved `FunctionId` —
identity-correct because CPG `Function` nodes are built from the same `FunctionId` set);
CHA Exact-gating (seed scan + guard, both directions tested, no laundering); cache v6
(serde + round-trip + v5 invalidation); the Phase-IP seam still clean and untouched; and
that the genuinely risky behaviors are each pinned by a test.

## Findings → disposition (folded in `f7d06f2`)

| # | Finding (reviewer) | Disposition |
|---|---|---|
| F1 | **MAJOR (codex)** — `membrane_slice.rs:91`: caller SET is exact, but the error-handling scan reopened the caller body via `find_function_by_name(name)` (first same-name) → wrong body in same-name files. | Scan the **exact** caller body by `caller_id.start_line..=end_line` (drop the by-name lookup). Also fixes a latent recall miss (by-name lookup could return `None` and skip a real resolved caller). |
| F2 | **MAJOR (codex)** — `spiral_slice.rs:210/242`: spiral's CALL rings are ExactOnly, but its textual reference rings (`find_variable_references` + the name-match shared-utility ring) still match by bare name → same-name leak outside the migrated rings. | These rings are textual (a different mechanism than call resolution) and recall-biased **by design** (outer rings widen the net). **Documented** as recall-biased; the ExactOnly promise is scoped to spiral's call rings. |
| F3 | **MAJOR (opus, perf)** — `resolved_caller_edges(func_id)` (scans every repo call site via `resolve_call_site`, a known hotspot) was called **per caller** in barrier/echo/membrane → latent quadratic on large corpora. | **Hoisted** out of the per-caller loop in all three (results identical; `func_id`/`caller_set` are loop-invariant). |
| F4 | **MINOR (opus)** — `vertical_slice.rs:204`: diff-highlight match keyed on `file+name` — the sibling of the T6 dedup fix at `:147`. | Now keys on `file+name+start_line`. |

**Informational (no fix):** CHA-synthesized virtual callers land in barrier's caller set
(via `Call(Exact)` CHA edges) but have no `resolved_caller_edges` row (CHA edges aren't in
`cg.calls`), so such a caller gets its signature lines but no highlighted call-site line —
correct (there is no literal source call site for a synthesized dispatch edge).

## Acceptance (re-confirmed after the fold)

`cargo fmt` clean · full `cargo test` + `--features mcp` green · Tier-A matrix 29 ok + 4
expected_gap (the Phase-IP set) · `--quick` oracle_err 0.0 with **`target-c-method`
DEFAULT `flip_candidate` P=0.2/R=1.0 + EXACT supplementary P=R=1.0** — the EFT success
metric. (Branch `baseline_invalid` is only `corpus_sha_drift` vs the dd60ed6 pin, expected
on a feature branch.)
