# Handoff — post-PR317 next increments

**Written:** 2026-09-18 America/Denver
**State:** Priority 1 architecture accepted; implementation dispatch ready
**Workspace:** `/private/tmp/prism-post316-slice1`
**Branch:** `plan/post317-next-increments`

## Binding and publication receipt

- PR [#317](https://github.com/shoedog/prism/pull/317) merged after all five CI
  checks were green.
- Accepted PR head: `6e6e978d06e2e157aa9ee4d44a3bc034e32d44e7`.
- Merge commit: `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`.
- Merge time: `2026-09-19T05:57:22Z`.
- PR head and merge commit have the identical tree
  `9b4de2b021d0a53fe4d07e50599f97d6c685e9ee`; the pre-merge source analysis is
  source-equivalent to merged `origin/main`.
- The five-green-CI and merge state were supplied by the controller. This planning
  lane verified the local full commit/tree identities and did not rerun CI.

## Durable planning artifacts

- Candidate priorities:
  `docs/superpowers/plans/2026-09-18-post317-next-increments/candidate-brief.md`
- Independent architecture review prompt:
  `docs/superpowers/plans/2026-09-18-post317-next-increments/fable-second-opinion.md`
- Priority 1 implementation contract:
  `docs/superpowers/plans/2026-09-18-post317-next-increments/specs/01-exact-caller-read-occurrences.md`
- Implementor and reviewer dispatches:
  `docs/superpowers/plans/2026-09-18-post317-next-increments/implementor/01-exact-caller-read-occurrences.md`
  and
  `docs/superpowers/plans/2026-09-18-post317-next-increments/reviewer/01-exact-caller-read-occurrences.md`
- Final architecture acceptance:
  `docs/superpowers/plans/2026-09-18-post317-next-increments/reviews/architecture-final-acceptance.md`

The brief ranks a bounded straight-line caller-RD design first, an input-gated WS2
cohort measurement second, and a non-authorizing WS4 ownership packet third. It keeps
loop/member provenance, destructuring, rest, arrow owners, and compiler ownership as
separate contracts. Historical real-site/private inputs are not assumed available;
failed custody must report `input-blocked`.

## Current gate

The user waived the previously requested unavailable reviewer; it is no longer a blocker.
The saved neutral prompt remains historical planning custody and does not claim that reviewer
was consulted. The controller designated an available independent architecture reviewer with
a two-round cap.

Round 1 reviewed source read-only and returned `REVISE BEFORE DISPATCH`, WRONG 0 / SMELL 3.
Receipt `/private/tmp/prism-post317-planning/architecture-review.md` has SHA-256
`1685ce12ceb48632c47a8792916d5199d8c017cbe33d8d30e0119e276c3937d3`.
The finite decisions are folded into the brief/spec/prompts: internal byte-keyed exact
producer facts with their own pre-reduction labels; unchanged global `VarLocation` and all
legacy DFG/query views; precise simple-binding admission; complete lifecycle/cache96→97;
seven proof groups; current Priority 2/3 populations `input-blocked`.

Round 2 returned `REVISE BEFORE DISPATCH`, WRONG 2 / SMELL 2. Receipt
`/private/tmp/prism-post317-planning/architecture-review-round2.md` has SHA-256
`4b9a9a180e9014b4905f0b0dfb7810032b34c5e0817c1acb9533ed13eec0581c`.
The controller authorized one finite planning supplement. It corrects the exact compact O01
fixture, replaces the invalid one-line Exact oracle with authenticated multiline parameter and
local fixtures, stores only supplemental exact pairs after classification/overlap checking,
adds representation-aware missing/conflict and direct `remove_files` controls, stages public
base-compatible RED before internal-seam tests, sets 700 production / 1,600 total changed-line
guardrails, and binds Tier-A with `--sut-bin` plus immutable quick-corpus custody.

The genuine merged-base CPG96 cache receipt is
`/private/tmp/prism-post317-planning/base-cache/RECEIPT.md`, SHA-256
`dbee9183f888dbecc6a9e593dc09ab4517fa33d734fad218789edb537bf4adbd`; its immutable cache
binary SHA-256 is `2bdf18dc1f485a4d00efa81033a3c90fee0e7c2a43daf373d31ed528b5859fb2`.
The bounded R02 same-environment characterization receipt is
`/private/tmp/prism-post317-planning/base-cache/r02-straight-line-receipt.md`, SHA-256
`2312e8ea236bfb0de6366e93f4d39b1b5299da9e485b40e4295da7171a0b7870`.
Its existing first-use Exact rows and missing later producer rows are characterization, not RED.

The bounded supplement is accepted: `APPROVE PRIORITIES AS WRITTEN`, WRONG 0 / SMELL 0
remaining. The copied final receipt has SHA-256
`e13a136d5e6f2a73c67aabd1888f445dc1059741323254d1dfe6959cf6b2fe9f`; it binds the accepted
Priority 1 spec SHA-256
`52f8648fa0c4c0f07817dc9985a379dabe9943600486cfcc8c1e270c22485b50` and the other frozen
planning hashes. This was finite verification of W1/W2/S1/S2, not a broad third review.
Priorities 2 and 3 still require authenticated input inventory/acquisition before any
measurement claim.

## Resume order

1. Commit the exact accepted planning paths and this status-only handoff update.
2. Dispatch Priority 1 from the accepted spec in an isolated implementation worktree with a
   separate two-round implementation review cap.
3. If implementation requires global identity, public-query, same-line-write, kill/alias,
   loop/member or owner expansion, park for design instead of widening the accepted slice.
4. Use one PR per accepted increment. Do not start Priority 2/3 while inputs are blocked.

No production source was edited and no implementation was authorized. The only new execution
was bounded same-environment characterization of the two fixed R02 fixtures and genuine base96
cache custody; neither is represented as behavioral RED or candidate acceptance.
