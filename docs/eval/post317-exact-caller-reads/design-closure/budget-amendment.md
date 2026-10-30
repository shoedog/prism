# Exact caller-read budget amendment

**Date:** 2026-09-19  
**Status:** controller approved; same feature and artifact resumed

The controller-authored planning guardrails are revised from **700 production
changed lines / 1,600 total source-and-test changed lines** to **850 production
changed lines / 1,800 total changed lines**. This amendment supersedes only the
original 700/1,600 estimates. It does not change the accepted functional
contract, review cap, evidence requirements, or publication authority.

The formatted held artifact measures **798 production / 1,662 total changed
lines** against planning base
`b4e421c7cdfa2076f6cfea50fb18d3ee16bdf010`. The original estimate omitted the
full source needed for the accepted default-refuse recursive grammar: named
child and field accounting, the complete anonymous-token inventory, lexical
identifier checks, expression and statement validation, and the unknown-kind
policy seam. Test reorganization reduced the test population by three lines;
the overrun comes from this required production grammar.

An independent read-only packaging assessment found no extraneous mechanism
and recommended retaining one cohesive feature at 850/1,800 rather than
splitting inactive infrastructure from activation or compressing the source.
Its source is
`/private/tmp/prism-post317-implementation/review/BUDGET-ASSESSMENT.md`,
SHA-256
`afbf3e19a88ca42866c4a64eafa4c9c1f0f206ec02dfed6c11796e7f49f846f2`.
The assessment is copied below for durable lane custody.

The revised guardrails leave 52 production and 138 total changed lines of
headroom. They permit completion of the already accepted contract only. The
worker must recount the final formatted diff and stop again if either revised
limit is exceeded. The post-design implementation validation cap remains one
round, and focused green results remain insufficient for acceptance.

---

## Independent packaging assessment (byte content copied 2026-09-19)

# Recursive admission repair — packaging and budget assessment

**Recommendation: retain the existing feature/artifact and explicitly revise the controller guardrails to 850 non-test production changed lines /1,800 total source-and-test changed lines.** Do not split an inactive exact-producer substrate from its admission gate merely to satisfy the earlier estimate. No source golf, restart, scope expansion or acceptance waiver is recommended.

This is a read-only packaging assessment, **not** the declared post-design final implementation review. It consumes no implementation validation round. No tests/builds ran and no shared source was edited.

## Binding / observed size

- Parked predecessor: `1a8354b2d80fdedc3b1ab877f5b63ec50913a09d`.
- Accepted design main SHA256 `7808bee475df710769cc73ea19cede21d0b0a34fc80108d3922fcb6623979d5f`; normative appendix `931005cc7a7f60f5b39e19592b22976028c1fe2642a00266732b6da2cbd3a2b1`.
- Preserved current patch `/private/tmp/prism-post317-implementation/design-repair-overbudget.patch`, independently authenticated SHA256 `c87c5b3c83bacead12cb5362d66f7fab041e7613b207e277c8dd03d5c8f6eec4`.
- Held `src/ast.rs` SHA256 `0fc8a5848b92ddfc117270c2c359799482cf71fed57723421219a0d1cf01d82e`.
- Held `src/cpg/exact_caller_read_tests.rs` SHA256 `c6e0751e5dddd0ac6b8637b8f62b31a835d93ce5b740ac871a36613da2ea8ac3`.
- Only those two source/test files differ from1a835. The accepted exact-state, RD classification, Step4 adapter, cache and lifecycle production paths have no post-design changes.

Independent `git diff --numstat b4e421c7 -- src` confirms:

| Category | Changed lines |
|---|---:|
| AST production | 298 |
| DFG production | 335 |
| CPG build production | 118 |
| RD production | 40 |
| Internal CPG export | 2 |
| Cache production | 5 |
| **Production subtotal** | **798** |
| Exact caller tests | 835 |
| Predecessor same-line tests | 29 |
| **Total** | **1,662** |

The prior freeze was643/1510. The increase is **155 production lines /152 total lines**; test reorganization offsets three lines. The earlier690/1595 projection underestimated the code needed to state and enforce the grammar. The worker correctly stopped when the hard700/1600 boundary was crossed.

## What accounts for the added production code

The AST delta replaces the old approximate75-line gate region with approximately228 lines, plus the small crate-internal supported-kind predicate. Every added mechanism maps to the accepted design:

| Implementation component | Accepted purpose |
|---|---|
| Named-child collection / field claims | Account for every non-trivia named node, including exact erasure boundaries |
| Parent-specific anonymous-token matcher | Reject optional-call, using, definite-assignment and unknown modifiers without a denylist/default-accept traversal |
| Identifier atom check | Preserve ordinary Unicode while refusing escaped/reflective runtime spellings |
| Recursive expression validator | Enforce closed callee, argument, member, assignment, binary, parentheses and non-null productions |
| Statement / declarator validator | Bound executable statements, local names, initializers and terminal return |
| Root / signature validation | Reject extra runtime wrappers and initializers; authenticate required names/parameters/body |
| Supported-kind predicate | Provide the production unknown-kind/default-refuse seam required by the design and sentinel test |

I found **no extraneous analysis mechanism** in this bounded delta: no new RD solve or kill/alias semantics, no public identity/query API, no owner inference, no member/default/loop authority, no additional cache version, no new production module or dependency. A small internal boolean kind predicate is not a public API expansion. Test changes stay within the approved positive/refusal matrix and unknown-kind control.

This scope assessment does **not** establish correctness or adequacy of the current implementation/tests. In particular, nine reported focused passes do not replace final primitive endpoint, refusal, cache, cost or full-suite validation. Those remain the final review's job.

## Why one cohesive feature is the safer packaging

An inactive producer substrate would introduce new primary/serialized state, labels, adapter logic and cache behavior without completing the user-visible missing-flow objective. Its meaningful positive RED→GREEN proof depends on the admission/activation part. A second PR would repeat cache/lifecycle/source binding and distribute one behavioral contract across two acceptance events, while the difficult grammar remains intact in the second part.

The current repair is already isolated to one recursive predicate and its finite test population, with previous production findings closed and retained evidence. Splitting here does not remove a design dependency or establish a useful independent behavior. It mainly moves a self-imposed line count across PR boundaries and risks weakening custody or the fail-first narrative.

## Revised guardrail and retained stops

850/1800 gives **52 production /138 total lines** above the observed held artifact. This is modest practical headroom for formatting and any already-required primitive assertions, not permission to add features. It is preferable to800/1700, which would leave only two production lines and invite compression despite the known proof obligations.

Record the reason for the amendment: *the required default-refuse recursive grammar, anonymous-token inventory and lexical identity checks were under-estimated by the original admission-gate budget*. This is a controller planning correction, not a user limit being overridden.

Retain all other boundaries:

1. Current accepted grammar and feature scope remain fixed; no new domain or production subsystem is authorized.
2. Recount the final **formatted** diff before freeze. If it exceeds850/1800, stop for another explicit packaging decision; do not silently increase it again.
3. Keep the post-design implementation validation cap at **ONE**. This packaging read is not that round. A recurring open-class wrong result still parks/escalates.
4. Require the exact A/R controls, same-environment base evidence, primitive preserved-output vectors, R07 final cache/cost, full Rust/nonRust and Tier-A classifications. No gate or accuracy claim follows from the budget amendment.
5. Preserve the held patch and existing branch; no restart, publication, merge or cleanup is implied.

**Packaging verdict: one cohesive feature, explicit850/1800 guardrail amendment recommended. Implementation verdict: not assessed in this task.**
