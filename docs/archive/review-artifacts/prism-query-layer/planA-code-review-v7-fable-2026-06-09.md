# Plan A substrate — in-depth code review round 7 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`,
FULL branch diff vs `main`. Framed as a convergence check: the merge-blocker bar was set at "a
concrete unsafe false negative in non-minified code that is Option-C-safe to fix here."

## Outcome: two in-contract false negatives — one common (recursion), one non-reproducing

Both lenses confirmed the convergence ground (shared `taint_neighbors` primitive is the right seam,
no constructible trace/classifier divergence; additive `reasoning: None` build-safe). They then found
two BLOCKERs the prior rounds missed.

## Disposition

- **BLOCKER (real, common) — recursion drops the parameter boundary:** a recursive self-call's
  arg→param edge has `next_fn == src_fn`, so the name-keyed boundary check missed it and `sink(u)`
  classified `NotReached` (verified: `boundary_len=0`). Recursion is a first-hop boundary inside the v1
  contract — this mattered. Fix: a parameter binding (Variable `Def` on a function's signature line, only
  ever written by an arg→param edge) is a boundary regardless of name match. Now `BoundaryExited`.
  Regression test added.
- **BLOCKER (as filed) — nested named function param scope: did NOT reproduce.** Probed
  `function outer(){ return function inner(p){`… — already degrades (`statement_at` is `None` on the
  signature line, not the encloser's `return` as the report assumed) and reaches correctly. The report
  over-assumed the statement indexing. Hardened anyway: a function-signature seed line now degrades to
  pure taint explicitly, so no variant slips through. Regression test added.
- **MAJOR (now-or-never) — non-exhaustive `Relation`→wire-string match:** made exhaustive over
  `Option<Relation>` (a new variant is now a compile error, not a silent `"DataFlow"` mislabel).
- **MAJOR — boundary/`cfg_valid` ordering enforced only by prose:** partially addressed — the boundary
  condition now pairs the function check with the parameter-binding check in one decision; the full
  `classify_neighbor` refactor is contracted.
- **MINORs:** `ReasoningSummary.reachability` aggregation rule documented; `cfg_reachable_lines_unioned`
  documented as inert on real CPGs (the multi-function/signature degrade is the load-bearing defense);
  `forward_reachable_in_function` memoization seam and `sink_nodes_at` callee-shadowing limit noted.

## Note on the loop
Rounds 4–7 each found a real issue, but the *nature* shifted: rounds 4–6 were the minified/line-granular
family (now closed by the uniform same-line arm + the multi-function/signature degrade), and round 7's
real finding was recursion — a distinct, common pattern now fixed. The non-reproducing BLOCKER and the
remaining items are contract/safe-direction. Convergence is close; round 8 is the verification of the
recursion fix and the signature-line degrade.
