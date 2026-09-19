# Handoff — post-PR317 next increments

**Written:** 2026-09-18 America/Denver  
**State:** planning ready; independent architecture review not yet invoked  
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

The brief ranks a bounded straight-line caller-RD design first, an input-gated WS2
cohort measurement second, and a non-authorizing WS4 ownership packet third. It keeps
loop/member provenance, destructuring, rest, arrow owners, and compiler ownership as
separate contracts. Historical real-site/private inputs are not assumed available;
failed custody must report `input-blocked`.

## Current gate

The user requested a Fable second opinion as reviewer/planner/architect. No callable
Fable model or plugin is exposed in the current environment, and no substitute was used
or described as equivalent. Invocation details remain a pending user/controller question.
The architecture prompt deliberately contains no model name so the exact requested
reviewer can receive it unchanged when available.

Priority 1 affects global `VarLocation` and label identity. No implementation may start
until that independent architecture review approves or revises the identity and legacy
compatibility strategy. Priorities 2 and 3 additionally require authenticated input
inventory/acquisition before any measurement claim.

## Resume order

1. Commit these three documentation files on the planning branch.
2. Resolve how the requested independent reviewer is invoked; do not silently substitute.
3. Run the saved architecture prompt read-only and preserve its complete verdict/findings.
4. Reconcile the candidate brief and prepare a bounded implementation/reviewer dispatch
   only after the architecture verdict. Use one PR per accepted increment.

No production source was edited, no new implementation was authorized, and no tests,
builds, corpus downloads, or measurements were run in this planning lane.

