# Optional/default identifier parameters — local implementation bundle

Base: merged PR312 `d9d1cc9195af240d2f6f0797954eea8e6e3d1c27`.
Branch: `feat/js-ts-optional-default-parameters`. Predecessor status commits
92190cfa/0b48072d carried as683d314c/6ffe7ecc; no source changes in that carry.

## Owner authority and boundaries

Owner approved source-backed proof/value assessment and bounded implementation
as multiple local slice commits, bundled in one eventual MR. Hold publication:
no push, PR creation, merge, or docs-only MR in this run. Primary owns design and
decisions; Claude Code Sonnet is preferred for bounded implementation/advice,
Opus for high-value independent review, Fable only if needed. No unbounded agent
fanout or model fallback hidden from the owner. Review cap: two rounds per slice.

Preserve exact token/owner/positional identity, duplicate/recovery/escape/write
and cache barriers; preserve non-JS behavior, caller field/base supplementation,
and private-source custody. Do not expand React.FC, closure admission, erased
this, parameter properties/decorators, rest/destructuring, inferred callable
owners or default-initializer value propagation. The unresolved react-scripts
decision remains unchanged. Stop if safe support requires those expansions.

## Sequence and value checkpoints

1. Proof/value: inspect all real occurrence consumers; classify optional/default
   source forms in immutable public/private corpora using pinned compiler syntax.
   Separate syntax counts, supported token candidates, existing bare Def uses,
   resolved argument sites and runtime/receiver authority. Establish negatives
   for initializer writes, omitted/undefined arguments, duplicate/escape/recovery,
   nested initializers and positional holes before choosing the supported subset.
2. Bounded occurrence implementation: capture RED, choose exact AST allowlists
   from evidence, implement only approved-safe forms. Optional and default support
   may be distinct local commits if their proof obligations differ. Explicitly
   update old refusal-characterization tests; do not silently rebaseline defects.
3. Integration/hardening: retain Step5b exact-token binding; test DFG/CPG, labels,
   writes, field isolation, full/subset and parallel/serial parity, cache invalidation
   and adjacent previous-slice regressions. Classify every corpus edge delta.
4. Full verification and closeout: Rust default/MCP/owner-audit, all Node tests,
   authority controls, native examples, Python, formatting/diff and Clippy; fresh
   release before Tier-A matrix and quick. Keep INVALID oracle evidence and
   same-environment controls, never rebaseline. Commit docs with implementation;
   prepare a single MR body locally, but do not publish.

## Initial hypothesis / alternatives

Optional/default identifier tokens already occupy slots but do not supply
occurrence authority. Hypothesis: exact occurrence support can recover real
argument/Def paths without changing slot selection. Alternative: initializer
evaluation or scope effects make function-entry Def/Use modeling unsound even
when token identity is correct. Discriminate with runtime/compiler-backed toy
fixtures, current DFG labels and fixed-source census deltas before editing.

Prism navigation reports41 stale paths. Current source, not an exhaustive LSP
claim, establishes consumers. LSP tools are unavailable in this session.

## Executed local slices

- `8d89f98e`: source-proof tool and bounded optional identifiers.
- `01522b2b`: positional comment-trivia repair discovered by source-ordinal replay.
- `5b80828b`: companion argument-count/cache hardening.
- `0bb567cb`: inert-default identifiers, including primary adoption corrections.
- Post-review: source hash normalization and equivalent boolean simplification;
  final gates and review closeout pending. Production review approved the bounded
  contract; observer review identified one bounded defect and two documentation
  limitations, now corrected or explicit. No review-cap extension.

The initial full Rust suites passed 4,184 / 4,377 / 4,400 with one existing ignore
each. Historical helper fixture loss prevents three separate source-custody tests;
candidate and base both reproduce ENOENT, and a bounded 12-archive recovery failed.
Carry those exclusions rather than fabricate expected call-site records. Python's
initial timing failure is retained; isolated base/candidate controls and a full
940-test rerun passed. Final verification will bind the post-review source commit.
