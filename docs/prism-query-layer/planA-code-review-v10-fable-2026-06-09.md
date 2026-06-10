# Plan A substrate — in-depth code review round 10 (fable + codex, vs main) — MERGE-READY

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`,
FULL branch diff vs `main`. Framed as the merge-readiness review: each lens was required to end with
its own standalone `MERGE VERDICT: APPROVE | BLOCK`, and the synth to report both separately.

## Outcome: ZERO BLOCKERS — both lenses APPROVE

> **PER-LENS VERDICTS**
> - **CORRECTNESS lens (codex): MY MERGE VERDICT: APPROVE** — production path byte-stable, fail-open
>   admits are over-approximations not unsafe misses, residual imprecision is the documented
>   line-granular / Plan B family.
> - **ARCHITECTURE lens (claude): MY MERGE VERDICT: APPROVE** — no unsafe false negative constructible
>   from the seams; one cheap pre-freeze rename recommended (non-gating); remaining residue is contracted
>   Plan B surface.
>
> **Overall verdict: ship — mergeable as the v1 gate as-is; do the one-line
> `Relation::SameLineDefUse → RecoveredDefUse` rename before Plan B freezes the wire shape.**

Codex reported **zero findings**. Both lenses independently verified the round-9 fixes are
safe-direction only: `ContinuationScan::ReasoningFailOpen` can only flip `NotReached → Reached` (witness
edges remain genuine DFG/recovered def-use edges — no fabricated witness hop); the field-path recovery
arm is scoped to same file/function/path then screened by `cfg_valid`. Neither lens could construct a
new, distinct, ordinary-code unsafe false negative outside the documented line-granular family.

## Disposition

- **MAJOR (now-or-never, DONE before merge) — `Relation::SameLineDefUse` name lies for field paths:**
  round 9 widened the variant to carry any-line recovered edges, so a loop-carried `def@5 → use@4` edge
  serialized as `"SameLineDefUse"`. Renamed to `RecoveredDefUse` (variant + witness `kind` string + doc +
  test); the exhaustive `Option<Relation>` match made it mechanically safe.
- **MINORs (contracted, verified on the Plan B list):** `cfg_valid` per-seed-vs-per-hop seam (Plan B
  node-precise rewrite); `same_function_same_path_uses_any_line` O(field-defs × file-nodes) — add a
  `(file,function,path)` index when Plan B's transitive chase multiplies traces; `is_parameter_binding`
  convention dependency (documented + test-pinned, retired by Plan B's structural parameter flag);
  `Trace` pub-field / `CfgScope` enum / `BoundaryEdge` raw-NodeIndex cross-rebuild caching — all contracted.

## Positives both lenses called out
`ContinuationScan` as a named policy enum (new callers must choose a mode); one shared `taint_neighbors`
behind both the BFS and `forward_reachable_in_function` (structurally prevents the round-6 divergence);
the exhaustive `Option<Relation>` match making new variants a compile error at the wire boundary.

## Convergence
Ten rounds. Rounds 1–9 each found and fixed a real issue (all green + Option-C byte-identical, production
taint untouched); round 10 found zero blockers and both lenses approved. The line-granular CFG-scoping /
DFG-ref-visibility family is a documented v1 precision limit (`NotReached` ≠ proven absence), resolved
wholesale by Plan B's node-precise seeding + interprocedural chase. Merged as the A3+A4+A7 v1 gate.
