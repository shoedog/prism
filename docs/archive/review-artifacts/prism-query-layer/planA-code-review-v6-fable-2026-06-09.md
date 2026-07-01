# Plan A substrate — in-depth code review round 6 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`,
FULL branch diff vs `main`. Input tasked the reviewers to adversarially attack the round-5 uniform
same-line arm and verify the other round-5 changes.

## Outcome: the same-line arm held; a different CFG-scope mechanism surfaced

Both lenses confirmed the uniform `Def → same-line-Use` arm closes the seam — the adversarial probes
(augmented assignment, destructuring, chained assignment) pass at the documented granularity, and
witness walk-back terminates. The new BLOCKER is unrelated to the arm: per-`(file,line)` CFG-scope
degradation mis-attributes one function's scope to another's roots on a shared minified line.

## Disposition

- **BLOCKER (real, verified) — shared-line CFG over-prune:** with `has_cfg` true, a minified line
  hosting two functions where only one contributes a statement caused the other function's root to
  inherit the wrong CFG scope and prune its own intra-function flow into a false `NotReached`. Probed
  and confirmed (`int a(){...} int b(int p){` … `sink(q)` in b → NotReached before fix). Fix: degrade a
  multi-function seed line to pure taint (safe over-approximation; Statement nodes carry no function
  field to split the scope). Regression test added.
- **MAJOR (now-or-never) — `Relation::AssignmentPropagation` overloaded:** split out
  `Relation::SameLineDefUse` (distinct witness `kind`) so Plan B's strong-update can distinguish a
  cross-variable assignment from a variable's own def-use chain. Done while the wire shape is unfrozen.
- **MAJOR (contracted, must-land-before-Plan-B-witnesses) — order-insensitive witness edge:** the
  uniform arm has no column ordering, so `sink(y); var y = u;` synthesizes a `y Def → y Use` edge to the
  earlier sink — a safe-direction over-report today (no `reasoning` consumers) but a witness corruption
  that the byte-range-scoping follow-up must fix before any witness is serialized.
- **MAJOR (contracted) — function identity is a `(file, name)` string pair:** two same-named functions
  in one file conflate; Plan B must key on CPG Function node identity. Documented.
- **MINORs:** `cfg_valid` ordering precondition documented; `sanitizer_supported` parallel-table
  pairing documented; `Trace.degraded` per-seed folded into the Plan B per-root API.

Both lenses verified Option C holds (`sanitizer_supported` gate exactly equals the old
`Go|Python|is_js_ts|Tsx` check; `reasoning: None` covers every `Evidence` constructor and stays
byte-compatible; the new code is consumed only by reasoning paths).

## Trend
The same-line def→use family is now closed (round-6 probes pass). The round-6 BLOCKER is a distinct
CFG-scope-attribution mechanism, also minified-only, fixed by the same safe-degradation principle. The
remaining MAJORs are now-or-never wire-shape / identity contract decisions, not fresh correctness holes
in straight-line code — the convergence signal continues.
