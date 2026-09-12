# Bounded callable-contained rvalue query captures

Eighth local increment, base0b345b15, same future MR bundle; no publication.
Owner approved the prior bounded repair recommendation. Two review rounds maximum.

Hypothesis: QueryCursor byte ranges admit overlapping ancestor captures; line
filtering then ingests signatures/outside-callable values. Alternative: the true
argument occurrence is absent or parameter binding fails. Reproduce the existing
strict CPG RED and inspect raw string/path/span extraction before implementation.
Enumerate assignment, call and return captures; contain their accepted source nodes
within the queried callable for JS/TS/TSX only. Whole-file roots remain supported.

Apply one shared capture predicate across string/path/span assignment and call
queries plus the span return query if RED establishes those paths. This is source
range containment, not nearest-callable execution ownership. Preserve recursive
walking inside accepted captures, defaults/destructuring refusal, nested ownership
observations, genuine same-line collisions and exact argument/parameter guards.
Keep other languages unchanged with executable controls. Inspect full-flow/DFG/
reaching consumers; bump CPG cache for altered DFG semantics, not the call-only
navigation cache unless evidence shows its stored call records change.

TDD: enable existing ignored desired flow test; update current disposition tests
to desired supported behavior; add assigned arrows, enclosing call/return controls,
unused signature and outside-sibling rvalue negatives, genuine contained reads,
whole-file root and non-JS controls. Preserve historical baseline/readout as base
evidence, never overwrite it. Verify full/incremental flow, cache version fencing,
full Rust configurations/examples, Node/authority/Python helpers, fmt/Clippy and
fresh-release Tier-A matrix/quick. No full multi-corpus or new real-repo census.

## Closeout

Implemented all seven capture gates; enabled the desired-flow regression and
bumped CPG cache to92 while preserving nav52. Identical focused tests measure
5pass/7fail on base →12pass after repair;51of108 synthetic flow observations
restored. Independent review approved W0/S0 within the two-round cap.

Full Rust default4501/MCP4694/detached-owner4717 and examples32passed, zero failures;
one reserved ignore in each full Rust configuration. Node786passed on the one
bounded-concurrency retry after an initial785pass/1budget refusal; initial failure
and both isolated base/candidate controls retained. Authority40/Python940passed,
one intentional Python live skip. Three historical Node tests lack source input.
Fresh-release Tier-A matrix159passed; quick timed out without a verdict. See the
[readout](../../eval/receiver-closure/2026-09-12-contained-rvalue-captures.md) and
adjacent receipt for exact evidence and limits. No publication.
