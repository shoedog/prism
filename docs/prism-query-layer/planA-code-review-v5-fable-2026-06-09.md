# Plan A substrate — in-depth code review round 5 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`,
FULL branch diff vs `main`. Input again tasked the reviewers to adversarially attack the prior fix
(the round-4 param-binding bridge) and verify the other round-4 changes.

## Outcome: the same-line def→use seam ran one hop deeper than the round-4 bridge

Both lenses confirmed the round-4 bridge correct as far as it went, then converged on its
incompleteness from both sides: Claude found the false-negative side (`var q = p; sink(q)` and the
main-BFS `var y = u; sink(y)` — the start-only bridge patched only the param position), Codex found
the false-positive side of the bridge's unproven precondition. The clean resolution turns out to be
the **uniform** `Def → same-line-Use` arm in `taint_neighbors` that Codex recommended back in round 4
and was declined then for the start-only bridge — it fixes the false negatives at every hop AND
dissolves the precondition critique (there is no longer a param-only special path).

## Disposition

- **BLOCKER (real) — same-line def-then-use:** replaced the start-only bridge with a uniform
  function-scoped same-path `Def → same-line-Use` arm in `taint_neighbors`. Fixes both the main BFS
  frontier (`var y = u; sink(y)`) and classification (`var q = p; sink(q)`). Two new regression tests.
  Adding edges only over-approximates, so it cannot introduce a false negative.
- **MAJOR — minified two-function seed line CFG over-prune (unsafe FN):** `cfg_scope_for_seed` now
  unions CFG-reachable lines over every statement at the seed line (`cfg_reachable_lines_unioned`,
  Option-C-safe; production `cfg_reachable_lines` untouched). Corrected the round-4 comment that
  misstated CFG scope as a line invariant. Regression test added.
- **MAJOR — silently-dropped unresolved seeds:** empty-root seed lines now warn. Test added.
- **MAJOR (Finding 2) — bridge precondition / reassignment over-fire:** dissolved by the uniform arm
  (no param-only path remains); the residual `p = clean(); sink(p) → BoundaryExited` is the may-taint
  no-strong-update over-approximation into the *indeterminate* state — safe direction, documented.
- **MINORs:** duplicate `(file,line)` seeds deduped; `node_of` non-Variable fallback `debug_assert!`s;
  `SinkResult.graph_node` referent documented. Double-nested JSON discriminant (#7) stays contracted
  for Plan B's first external emission.

Both lenses verified Option C holds structurally (`reasoning` is `Option` + `skip_serializing_if`,
never constructed on nav paths; `forward_reachable_in_function` is consumed only by classification;
`cfg_reachable_lines` is byte-stable for production).

## Trend
Rounds 4 and 5 both surfaced the same-line def→use seam, each one position deeper than the prior
patch. The uniform arm closes the seam at the source (every hop) rather than per-position, which
should end this finding-family. Convergence is the residual signal — each round's NEW surface shrinks.
