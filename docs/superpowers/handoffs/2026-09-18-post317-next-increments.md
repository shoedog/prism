# Handoff — post-PR317 next increments

**Written:** 2026-09-18 America/Denver
**State:** Priority 1 locally accepted; P2 planning accepted and ready for separate dispatch;
P3 parked
**Workspace:** `/private/tmp/prism-post316-slice1`
**Branch:** `feat/post317-exact-caller-reads`

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
- Public Excalidraw source-custody receipt:
  `docs/eval/post317-input-custody.md`
- Accepted Priority 2 source-only census contract, prompts, input manifest and review:
  `docs/superpowers/plans/2026-09-18-post317-next-increments/specs/02-fixed-source-parameter-census.md`,
  `docs/superpowers/plans/2026-09-18-post317-next-increments/implementor/02-fixed-source-parameter-census.md`,
  `docs/superpowers/plans/2026-09-18-post317-next-increments/reviewer/02-fixed-source-parameter-census.md`,
  `docs/superpowers/plans/2026-09-18-post317-next-increments/inputs/02-excalidraw-input-hash-manifest.json`, and
  `docs/superpowers/plans/2026-09-18-post317-next-increments/reviews/02-fixed-source-parameter-census-acceptance.md`.

The brief ranks a bounded straight-line caller-RD design first, an input-gated WS2
cohort measurement second, and a non-authorizing WS4 ownership packet third. It keeps
loop/member provenance, destructuring, rest, arrow owners, and compiler ownership as
separate contracts. As of 2026-09-19, the exact public Excalidraw source/config/lock
archive is available; historical real-site data, an authenticated dependency root, compiler
Program closure, and private inputs remain unavailable. Each unavailable required input must
report `input-blocked` rather than become an empty population.

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
seven proof groups; Priority 2/3 historical inputs then `input-blocked`.

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
### P2/P3 source-custody update — 2026-09-19

The controller-authorized public source acquisition resolved Excalidraw
`0642e72cfa2d9a71198200e52f37399384610ee3` and tree
`709e9146b0fbd78c3ebf0d77e67143b2fbc43e4a` through the official GitHub API. Its
35,599,041-byte archive passed safe extraction (1,229 regular files, no links), and the
historical root package/config/lock hashes match. Compact durable receipt:
`docs/eval/post317-input-custody.md`; external custody root:
`/private/tmp/prism-post317-measurement-inputs/`.

This is public source custody only. It does not provide `node_modules`, dependency closure,
a compiler Program, historical real-site data, private worker inputs, an observer result, or
permission to install/run effects. The historical installed-root admission refusal remains
context, not a current measurement. P2/P3 may resume only under their separate input/effect
authorities and must report any remaining required input as `input-blocked`.

### Current execution status — 2026-09-19

Priority 1 is frozen and locally accepted at `14e86083c2c754ed412d440742bd5763be388e42`
(tree `a0411db6d858fe34806376ce41d2621853848d8b`). Final evidence review returned
`APPROVE`, WRONG 0 / one nonblocking coverage SMELL. Final Rust, R07,
compiler/grammar/Node/Python and Tier-A matrix gates are complete; compact durable receipts
are under `docs/eval/post317-exact-caller-reads/receipts/`. Tier-A quick is terminal
`INVALID` because of corpus-pin drift and the C-method `4/6` probes, so it supplies no
accuracy or flip claim. The final acceptance receipt is
`docs/eval/post317-exact-caller-reads/receipts/final-acceptance.md`, copied from source
SHA-256 `25ec6a4bf341b952c3bc0becedb2009886f8ea94b4b363981a01e1c8d8884209`.

Priority 2 planning is independently accepted with WRONG 0 / SMELL 0. The accepted external
manifest SHA-256 is `b0197e015b6cfe09294a9cdcfe89e23ef9ecf827e733dbadf0deae7227459ee1`
and accepted spec SHA-256 is `e6e1bb3e8a928943149e59cfa29598c1e15142d10468c32fe5704a70132c1f71`;
the durable prompt copies point to the durable spec path. Execution still requires a separate
controller dispatch.

Priority 3 is parked: its proposed source-only worker-outcome pilot did not add sufficiently
distinct evidence while dependency custody and compiler Program closure remain absent. No P3
execution is authorized; the dependency-custody decision is deferred.

## Resume order

1. Checkpoint the Priority 1 acceptance receipt, final gate receipts, accepted Priority 2 plan,
   and these handoffs without changing the frozen source manifest.
2. Dispatch the accepted Priority 2 source-only census separately; its observer may use only the
   authenticated 414-file population and the accepted fail-first/mutation contract.
3. Keep Priority 3 parked until a separate dependency-custody decision supplies distinct evidence.
4. Use one PR per accepted increment. Publication, merge, rebaseline and adoption remain separate
   controller decisions.

At initial architecture planning acceptance, no production source was edited and no implementation
was authorized. The only planning execution then was bounded same-environment characterization of
the two fixed R02 fixtures and genuine base96 cache custody; neither is represented as behavioral
RED or candidate acceptance. The later 2026-09-19 public source-custody acquisition above is
separate, does not measure P2/P3, and does not change those historical assertions.
