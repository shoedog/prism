# Plan A substrate — in-depth code review round 4 (fable + codex, vs main)

Run via a2a-bridge `run-workflow code-review`, prism-wired host config, `ANTHROPIC_MODEL=fable`.
Input was the FULL branch diff vs `main` (`5e35d64`). The input explicitly tasked the reviewers
with **adversarially breaking the round-3 BLOCKER dismissal** (construct any input where a boundary
to a param `Def` with a same-line use exists), verifying the round-3 over-fire fix, and finding new
defects — while NOT re-litigating the explicitly-contracted Plan B items unless unsound.

## Outcome: the adversarial re-review worked — round-3's dismissal was wrong

Both lenses confirmed the round-3 fix (`reachability_for_node` → `forward_reachable_in_function`) is
correct and function-scoped with no new false negative. Then both independently **broke** the
dismissal with the same better witness:

```js
function g(p) { sink(p);
  log(p); }
function f() { var u = input(); g(u); }
```

A *multi-line* function whose first body statement shares the signature line: the later `log(p)`
registers param `p Def@1`, the `u → p Def@1` boundary IS recorded, and `data_flow.rs`'s
`ref_line == start` skip drops the def→use edge to the same-line `sink(p)` use — so the real sink
classifies `NotReached` for a path taint crosses. Round 3 over-generalized "strictly one-line
functions get no param Def" to "the precondition is unsatisfiable." Verified by probe; fixed.

## Disposition

- **BLOCKER (real) — same-signature-line param use:** restored the param-binding bridge
  (`same_line_same_path_uses`, seeded only from `start` in `forward_reachable_in_function`; param-scoped
  so sound and out of the BFS frontier/witness). Pinned by
  `test_boundary_classification_same_signature_line_body_use`. The reviewers split on fix location
  (codex: `data_flow.rs` edge; claude: trace.rs); claude's is right for Option C, and the bridge is even
  more surgical (classification-only, never touches existing output paths).
- **MAJOR — one-hop boundary closure / `NotReached` is not proven absence:** resolved by honest
  documentation (the reviewer's accepted v1 option). `NotReached` is now documented as "not reached within
  the seed function ∪ first-hop boundaries"; the transitive callee-chain chase IS Plan B's `taint_reaches`.
- **MINOR — duplicate degraded-seed warnings:** `cfg_scope_for_seed` hoisted to per-`(file,line)`. Pinned
  by `test_degraded_seed_line_warns_once_per_line`.
- **Severed-downstream variant (`var x = p` on the signature line):** verified by probe to be a *distinct*
  CPG-construction gap — the declaration-initializer use of `p` on the signature line gets no Use node at
  all (a later-line `var x = p` does register it), so there is nothing for the reasoning layer to bridge.
  Documented as a CPG limitation, out of scope for Option-C-safe Plan A; the call-argument case is fixed.
- **Contracted MINORs re-raised (CfgScope #6, vocab #7, `SinkUnresolved` state, memoization, `Trace`
  hardening):** kept contracted for Plan B; the CfgScope footgun is now spelled out in a comment.

Both lenses confirmed the previously-contracted items behave as contracted and Option C holds (every
`Evidence` site sets `reasoning: None`, `skip_serializing_if` test-pinned, no existing path calls the new
code).

## Lesson recorded
A dismissed finding must be closed by inviting a counterexample, not by asserting non-reproduction from a
single probe. Round 3 asserted; round 4's adversarial framing produced the counterexample. The in-depth
re-review-vs-main policy earned its cost here.
