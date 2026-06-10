# Plan A substrate — in-depth code review round 8 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`,
FULL branch diff vs `main`. Convergence-verification round: merge bar set at "a concrete in-contract
unsafe false negative in ordinary (non-minified, non-pathological) code that is Option-C-safe to fix."

## Outcome: one ordinary-code false negative found; round-7 fixes confirmed correct

Both lenses confirmed the round-7 parameter-binding boundary and signature-line degrade are correct in
direction (only safe `BoundaryExited` false positives, no parameter-binding miss today). Codex found one
in-contract unsafe false negative the convergence sweep otherwise missed.

## Disposition

- **BLOCKER (real, ordinary code) — 20-line continuation-scan cap:** a tainted argument on the 21st+
  continuation line of a multi-line call was CFG-rejected → `NotReached` (verified by probe). Fixed by
  parameterizing the cap: production `taint_forward_cfg` keeps the byte-stable 20, the reasoning path
  scans unbounded. Regression test added; production taint unchanged.
- **MAJOR 2 (fragility) — `is_parameter_binding` rests on an unwritten cross-module invariant:** verified
  multi-line-signature recursion already works (param Defs pinned to the function start line). Pinned with
  a multi-line-signature recursion test and a dependency comment at `data_flow.rs` so a future param-Def
  line change can't silently revive the recursion false negative.
- **MAJOR 3 (contracted) — `BoundaryEdge` has no kind discriminant:** internal `Trace` state, not a frozen
  wire shape; the conflated cases are safe-direction. Plan B adds the `kind` with the structural
  parameter-ness flag (which also resolves MAJOR 2's fragility).
- **MINORs (contracted):** `ReasoningSummary::aggregate` + multi-source attribution; `Evidence::new`
  constructor (#8); `CfgScope` enum (#6); `classify_neighbor` fold — all to land before Plan B edits the
  paths.

Both lenses confirmed Option C holds (the new code is consumed only by reasoning paths; production
`cfg_reachable_including_continuation` keeps the 20-line cap).

## Trend
Round 8's finding was a *pre-existing production-shared* cap (not introduced by this branch), fixed
Option-C-safely in the reasoning path. The round-7 fixes verified correct. Remaining items are all
contracted Plan B surface or safe-direction. The substrate is a v1 gate feeding Plan B, not a finished
taint engine; this is the point of diminishing returns on hardening exotic CFG/identity edge cases vs
letting node-precise seeding (Plan B) subsume the family.
