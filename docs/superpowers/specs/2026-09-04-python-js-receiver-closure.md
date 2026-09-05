# Python/JS receiver measurement and correctness closure

Authority: owner approved item 1 (measurement/correctness) after merging #240.
Base control `862166d` (pre-#240); merged candidate `f3bf88e`. Three-round cap.
Published evidence: `f5d766b`, [PR #241](https://github.com/shoedog/prism/pull/241) merged at `1886907`. No production source change in that PR; the subsequent authorized repair is tracked by the rust-binary-library spec/handoff.
No default-import/reexport feature expansion, baseline rewrite or full multicorpus.

## Plan and evidence contract

1. Bind clean tracked source and both release binaries in the same environment.
2. Archive unchanged fixed samples: Black b74b23013fe6 (blib2to3 pgen2 package and
   parent initializer), Excalidraw 0642e72cfa2d (Scene/align/distribute/dragElements),
   and two Excalidraw JavaScript build scripts as preservation controls.
3. Compare cache-bypassed production CLI site dumps using exact source spans and
   target identities, not aggregate counts alone. Enumerate every changed Exact
   edge and inspect its declaration/receiver proof. Samples are partial source
   universes; no whole-corpus precision/recall claim.
4. Recheck pinned Rust probes against one fixed source universe with both binaries
   and the same live oracle observations. Separate literal-address drift from
   algorithm regressions and broad NameOnly versus Exact results. Retain pins.
5. Only demonstrated defects authorize bounded repairs: RED on unchanged artifact,
   alternative-cause discriminator, negative/preservation coverage, full default/MCP
   suites and rebuilt matrix/quick if production resolver code changes.
6. Record findings, source/binary custody and exclusions; reconcile merge status;
   commit/push/open the scoped PR. Review converges within three rounds or escalates.

Initial hypotheses: explicit-relative Parser construction and named type-only Scene
parameters should gain defining-file Exact edges. Alternative is that unmodeled
real declaration syntax blocks recovery; actual site dumps discriminate. JavaScript
value-binding outcomes should remain unchanged. Pinned feature-gated missing sites
may be literal line drift, not a resolution change; current source and paired edge
sets must discriminate before attributing. Stale MCP index is orientation only;
current CLI/source controls are authoritative.

Execution complete within the three-round cap: 548-site paired comparison, 11
source-checked additions and served bidirectional checks; all four live pins
identical; full default 3732/0/1 and MCP 3922/0/1, matrix 104/104. Corrected the
predecessor documentation's runtime-binding claim. No demonstrated Python/JS
production repair; inherited Rust qualified-call miss remains explicitly separate.
See the receiver-closure readout for evidence and excluded verification.
