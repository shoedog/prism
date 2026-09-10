# Bounded asserted-member proof, before production normalization

Base: PR306 merge58998926af2eb6ad7750e5a92a19f7b198993216, freshly fetched.
Branch: docs/typescript-asserted-member-proof. Formal review cap: two rounds.
Owner approved the preceding recommendation: bounded compiler/native proof for
asserted receiver paths. No production normalization or authority expansion here.

## Discriminating evidence

Hypothesis: whole-member text serialization loses field identity under erased
receiver wrappers. Alternative: an opaque fallback merely represents unsupported
syntax and another collector still supplies the necessary dependency.

The initial no-write probe cannot discriminate lost flow: existing parameter field
isolation produces no Def for plain or asserted reads. The second probe adds an
explicit runtime.X write at line2. Plain and parenthesized reads retain the
runtime.X Def@2 -> Use@3 edge; as-any/import-type reads do not. Both controls and
cases parse without errors. Evidence: field-write-probe.log under the root below.
Compiler validity/emission will independently bind the fixtures before publication.

## Architecture and acceptance

Keep three different objects distinct: raw source/byte spans; a proposed bounded
syntactic path description; and existing resolution/reaching/owner authority.
The proof observer reads synthetic fixtures only. The native example exposes
current public API names/paths/spans/calls/raw returns and full/subset DFG records;
it does not normalize, infer policy or create new edges. Compiler evidence binds
exact input bytes, native binary identity, diagnostic validity and emitted syntax.
A detached syntax-candidate classifier tests the proposed allowlist and refusals;
its result is not a production proof object or executable-owner authority.

Initial allowlist: one nonoptional literal ASCII dot-member; receiver is a literal
ASCII identifier after at most8 parenthesis/as/satisfies wrappers. No calls,
computed or optional members, assignment/comma/conditional receivers, non-null or
angle assertions, member chains, escapes or malformed/unresolved fixture types.
The budget and nonliteral-name refusals have explicit negative fixtures. Emission
equivalence alone does not admit unsupported source syntax.

Before any subsequent repair, source/native proof must cover collect_path_refs as
well as rvalue path/span collectors. The reference matcher also serializes raw
member text. Normalizing only rvalues cannot establish a repaired DFG edge.
Retain original member span and exact receiver-token span separately. Never use a
synthetic normalized string to compute source offsets or erase reference identity.
Raw returns/calls, field isolation, selected-line behavior, duplicate/write/cache
barriers and confidence labels remain separate existing contracts.

## Gates and stop conditions

Compile/run native observer protocol tests and the pinned compiler fixture audit;
enumerate the entire fixed population and negative dispositions, no first-error
retry loop. Capture the desired dependency assertion failing on unchanged merged
code; label it an unfixed RED, not a regression introduced here. Full project suites
run for new diagnostic tooling, with skips/warnings/exclusions recorded. No src/
runtime edits means Tier-A is not triggered by this slice. No rebaseline.
Independent review, maximum2 rounds, then commit/push/PR. Stop if proof depends on
new authority, a non-enumerable repair expansion or private sources.

Evidence root: /private/tmp/prism-asserted-member-proof-HFzXTL.
No real-source execution, installs, live model/full multi-corpus runs, closure or
React.FC expansion. react-scripts remains unresolved. No real-site recall claim.

## Final checkpoint

Reviewed tooling b27f1f4dd1338e99d4e4588aa94cf9e6e15925e2 is pushed; runtime remains
identical to58998926. Final54 observations:49 valid,5 expected invalid,0 observation
failures. Repair-required mode remains exactly12 UNFIXED failures. Native7/policy7/
CLI negatives7 passed. Full Rust4,095/4,288/4,311; observers726/helpers18/authority40;
Python940+one deliberate skip; membership example12. Clippy completes with warnings.
Full runner starts/ends at the same clean HEAD. Independent round1/2 approved.
No Tier-A trigger or production fix. Final readout and next recommendation:
docs/eval/receiver-closure/2026-09-10-typescript-asserted-member-proof.md.
Earlier future-tense gate steps above are historical executed plan, not pending.
Published with closeout adf07389 as PR307:
https://github.com/shoedog/prism/pull/307. No automatic merge.
