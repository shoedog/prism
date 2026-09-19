# Independent architecture review

Review this planning artifact read-only. Do not implement, edit production, run broad
test suites, or infer approval from prior local acceptance.

## Binding

- Repository base: `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`
- Tree: `9b4de2b021d0a53fe4d07e50599f97d6c685e9ee`
- Candidate: `docs/superpowers/plans/2026-09-18-post317-next-increments/candidate-brief.md`
- Accepted predecessor decisions:
  `docs/eval/post316-slice2/design-decision.md`,
  `docs/eval/post316-slice2/final-acceptance.md`, and
  `docs/eval/post316-slice3/final-acceptance.md`

## Source seams to inspect

- `src/data_flow.rs`: `VarLocation` identity, `preferred_use_locs`, `get_use`, and
  edge production
- `src/ast.rs`: `rvalue_identifier_spans_on_lines` and
  `find_path_references_scoped`
- `src/cpg/reaching.rs`: label keys, kill/alias equivalence, and loop-carried facts
- `src/cpg/build.rs`: Step 4 endpoints, exact occurrence index, Step5b, and cache parity
- `src/parameter_slots.rs`: occurrences, slot barriers, destructuring/rest, and inert defaults
- `src/executable_owner.rs` and `src/executable_owner/acquisition.rs`: compiler packet,
  closure, identity, and budget gates

## Assumptions to challenge

1. A bounded straight-line simple-identifier repair can carry byte-distinct Use
   endpoints without changing global `VarLocation` equality or destabilizing
   reaching-definition kill, alias, and label maps.
2. A compatibility adapter can preserve deterministic legacy line-key queries and
   warm-cache behavior while exact internal endpoints remain distinct.
3. Loop-carried reads and member paths must remain separate because their reference
   and confidence semantics differ from straight-line simple paths.
4. Destructuring, rest, and arrow-owner populations require separate ledgers. Syntax
   counts do not establish callable ownership, slot ordinals, or constructible flows.
5. Compiler Program membership is observational until input identity, semantic closure,
   dependencies/effects, representations, and current budgets all close.
6. Historical public/private input paths are not currently authenticated. Missing
   inputs must yield `input-blocked`, not zero demand or an inferred compiler refusal.

## Required architecture answers

1. Should byte identity live in a new internal occurrence/edge endpoint type, or can
   `VarLocation` equality safely change? Trace the effect on label maps, kill/alias
   logic, serialization, legacy indexes, cache reconstruction, and public queries.
2. Define the smallest implementable Priority 1 boundary and its compatibility layer.
   State whether straight-line simple identifiers are truly separable from loop/member
   semantics. If not, identify the exact coupling and park the implementation.
3. Specify a fail-first oracle that proves the later caller Use received its own genuine
   producer edge. It must distinguish extra edge count, downstream CPG recovery, forged
   byte lookup, label fallback, and all-to-all fanout.
4. Decide whether the Priority 2 observer should be one reviewable measurement slice or
   separate destructuring/rest and arrow-owner slices. Keep every cohort numerator,
   refusal, and promotion rule separate.
5. Name the strongest single public WS4 falsification target under existing caps and
   the exact packet predicates required before input selection could be proposed.
   If current inputs are unavailable, require an acquisition/custody gate instead.
6. Recommend the next three increments and stop conditions. Do not authorize production
   work whose input, owner, slot, occurrence, or closure authority remains unproven.

## Finding and verdict discipline

- Label every finding `WRONG` or `SMELL` before the explanation.
- `WRONG` requires a constructible input/state, the incorrect output produced by the
  plan, the mechanism, and a bounded repair or design decision.
- `SMELL` covers an unproven risk, missing evidence, ambiguity, or maintainability cost;
  it is never a blocker by itself.
- Put WRONG findings first. Separate source-proven facts, assumptions, measurements,
  and recommendations.
- For every proposed behavior change, require a meaningful pre-change behavioral RED,
  a negative or refusal control, and same-environment base attribution.
- Do not merge caller RD, loop/member provenance, optional/destructured/rest support,
  arrow ownership, or compiler ownership into one acceptance claim.

End with exactly one architecture verdict:

- `APPROVE PRIORITIES AS WRITTEN`
- `APPROVE WITH BOUNDED REORDERING`
- `PARK PRIORITY 1 FOR DESIGN`
- `REVISE BEFORE DISPATCH`

