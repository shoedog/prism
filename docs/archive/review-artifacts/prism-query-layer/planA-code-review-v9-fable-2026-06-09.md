# Plan A substrate — in-depth code review round 9 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`,
FULL branch diff vs `main`. Convergence-verification round; merge bar: a concrete in-contract unsafe
false negative in ordinary code, Option-C-safe.

## Outcome: refuted convergence — two distinct ordinary-code false negatives

Both lenses confirmed the round-8 continuation-cap fix handles long plain continuations with no
cross-function over-reach, then each found a distinct missed-taint input the convergence sweep missed.

## Disposition

- **BLOCKER 1 (real) — nested-callback continuation under-reach:** a tainted argument after an inline
  callback in a multi-line call was `NotReached` (the walk-back stopped at the callback body statement).
  Fixed with a `ContinuationScan` policy — reasoning path fails open (unbounded, any reachable preceding
  statement). Regression test added.
- **BLOCKER 2 (real) — loop-carried field-path false negative:** field paths filter refs to lines after
  the def, so a loop-carried `o.data def@N → use@M (M<N)` edge never exists (simple vars get it). Added a
  field-path recovery arm in `taint_neighbors`; the back-edge-aware `cfg_valid` keeps the loop-carried
  use. Corrected the lying `ast.rs` comment. Regression test added; production DFG byte-stable.
- **MAJOR 1 (reverted — false premise):** the proposed "DataFlow connects only Variables" tripwire
  contradicts `test_taint_trace_skips_non_variable_dataflow_neighbors`, which shows the skip is
  intentional/tested. Replaced with a clarifying comment.
- **MAJOR 2, MINOR 2/3/4 (contracted):** continuation walk-back function-boundary stop (couples with
  Plan B's multi-function `cfg_set`, would break the nested-callback fix today); lossy `reachability_at`
  affordance; `CfgScope` enum; `src/cpg/tests.rs` split (>600-line rule).

Both lenses confirmed Option C holds (production `cfg_reachable_including_continuation` keeps
`Production` mode; the field-path DFG path is unchanged; the recovery arm is reasoning-only).

## Trajectory note (carried to the merge decision)
Rounds 6–9 each surfaced a real intra-function false negative in the line-granular CFG-scoping /
DFG-ref-visibility machinery — each fixed Option-C-safely, but the family is deep (line granularity can't
fully resolve continuations, shared lines, nested functions, or field-path ref visibility). The
"`NotReached` is not proven absence" contract already disclaims these as precision limits, and Plan B's
node-precise seeding + interprocedural chase is the architectural resolution for the whole family. This is
the diminishing-returns frontier: each further round costs a full review cycle to find one more exotic
edge case that Plan B will subsume.
