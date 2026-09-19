# Post-PR317 candidate brief for second-opinion architecture review

Planning-only input, written 2026-09-18 and rebound after the PR317 merge. This is
not implementation authority or final architecture approval. Production was read
only; no tests or builds were run.

## Binding and selection rule

- Merged base: PR317 head `6e6e978d06e2e157aa9ee4d44a3bc034e32d44e7` merged as
  `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090` on 2026-09-19T05:57:22Z.
  Both commits resolve to tree `9b4de2b021d0a53fe4d07e50599f97d6c685e9ee`, so the
  pre-merge candidate analysis is source-equivalent to `origin/main`. This planning
  branch began clean at the merge commit.
- Accepted predecessor state includes the three PR317 slices: nested caller ownership,
  exact CPG occurrence materialization, and optional identifiers beside validated inert
  defaults. The next increment must preserve those contracts rather than reopen them.
- Ranking rule: first address the shared producer limitation that now has exact downstream
  consumers; then measure the remaining WS2 syntax/owner cohorts before choosing one; only
  then attempt a compiler-owned project boundary. A measured zero-yield/refusal is valid.
- Every implementation candidate needs a same-environment base control and a fail-first
  behavioral oracle. Measurement candidates need fixed inputs, exact populations, and a
  refusal result when their authority predicates are not met.
- Input availability is currently unverified. The prior run explicitly excluded three
  historical source-custody tests because `real-sites.jsonl` and the required private
  `PRISM_AUDIT_*` inputs were unavailable; no reusable real JS corpus is established locally.
  Prior paths and counts are provenance, not proof that their bytes still exist.

## Priority 1 — byte-distinct caller-side RD for straight-line simple identifiers

**Why first.** Slice 2 intentionally repaired CPG materialization and argument lookup while
leaving the DFG producer unchanged. `src/data_flow.rs:455-470` records every raw rvalue span
but chooses only the first `(AccessPath,line)` in `preferred_use_locs`; `get_use` returns that
representative to all three edge-producing branches (`:481-584`). `VarLocation` ordering and
hash identity also omit bytes (`src/data_flow.rs:53-79`). Consequently later same-line Uses
can now exist in the CPG and bind across a call, yet caller reaching-definition edges still
target the first same-line occurrence. The accepted design decision names byte-distinct RD
identity as the successor (`docs/eval/post316-slice2/design-decision.md:12-29`).

**Bounded behavior.** Start with straight-line, simple-identifier references only. For the
authenticated form below, retain each actual Use span and connect the preceding Def to each
semantically referenced occurrence without first/last/nearest guessing:

```js
import {item} from './origin';
function outer(value){sink(value);return item(value);}
```

This is the exact no-trailing-newline `app.js` fixture, SHA-256
`5457bf7b1248b193da83bd293adc26e7ecdddb6018d5b2c075f48fae057583f1`;
its paired compact `origin.js` is pinned in the Slice 1 spec.

The accepted fixed fixture observes both byte-distinct Use spans (with a duplicate raw row
possible for the argument span), but only a caller label to the first preferred Use.
Re-authenticate offsets on the frozen source fixture rather than copying historical numbers. The candidate oracle is the
exact multiset of `(Def bytes, Use bytes, path, owner, label)` plus the already accepted
later-argument-to-parameter edge. It must show no all-to-all fanout, duplicate amplification,
or label upgrade. Mutating only the later identifier bytes or source order must change only
the corresponding endpoint.

**Seams.** `src/ast.rs:7388` already emits byte-bearing rvalue spans; change the reference
producer/API that currently reduces references to `BTreeSet<usize>` at
`find_path_references_scoped` (`src/ast.rs:8507`) and the selection in
`src/data_flow.rs:455-584`. Review label keying in `src/cpg/reaching.rs:220-249` and Step 4
materialization in `src/cpg/build.rs:901-950`. Preserve the exact CPG index, Contains edges,
Step5b, cache fresh/warm parity, and legacy query behavior established by Slice 2.

**Fail-first population.** JS/TS/TSX: two reads on one line; two call arguments; same-name
owners; Unicode/comment byte shifts; full/subset/incremental/warm cache. Negatives: forged
bytes, zero-width anchors, nested-owner capture, and a path not present at the target span.
Run a saved-base mutation control so the RED proves endpoint identity, not merely extra edge
count.

**Explicit boundary.** Do not include loop-carried or field/member repair in this first
implementation. Simple paths currently admit earlier-line references while field paths are
forward-only (`src/ast.rs:8517-8522`), and reaching classification separately records
backward Exact edges (`src/cpg/reaching.rs:223-245`). Members also require full/base path
policy. Carry one loop and one member fixture as preservation/refusal controls; plan their
producer semantics only after the straight-line endpoint contract passes. Do not alter
`FunctionId`, public query APIs, or synthesize edges from the downstream CPG index.

**Stop condition.** Park for design if exact spans cannot be produced before RD without a
source-wide per-edge scan, if byte identity changes kill/alias equivalence, or if legacy
line-key consumers cannot be kept deterministic through an adapter.

**Independent architecture decision.** Round 1 of 2 (`REVISE BEFORE DISPATCH`, WRONG 0 /
SMELL 3) selected an additive internal byte-keyed endpoint/edge identity with its own
pre-reduction RD confidence. Global `VarLocation`, `FlowEdge`, legacy edges/labels/adjacency,
public queries, kill/alias rules, primitive byte payloads and multiplicities remain unchanged.
All admitted pairs are independently classified and checked for overlap consistency; only
supplemental pairs absent from the legacy exact endpoint population are persisted. That state
participates in empty/build/subset/remove/merge/incremental/cache lifecycle. Step 4 consumes a
deduplicated union; a same-exact-pair label conflict refuses and diagnoses the target binding
expansion while preserving legacy output and an unrelated eligible binding.

The admitted domain is narrower than “straight-line”: existing named JS/TS/TSX owners;
plain parameters or a unique non-aliased earlier-line local; real simple-identifier Uses; no
writes/redeclarations/shadows/aliases/captures, control-flow branches, loops, members,
recovery/reflection or ambiguous owner. The original one-line parameter fixture is admitted
and gains its independently classified `NameOnly(CfgIncomplete)` later edge; a separate
multiline parameter/local fixture must prove Exact. CPG cache 96→97; navigation remains 53.
The implementation slice is capped at 700 non-test production changed lines and 1,600 total
source/test changed lines; exceeding either stops for re-slicing before review.

The fixed multiline Exact characterization uses no-trailing-newline fixtures. The parameter
fixture SHA-256 is `286faa2b12b01b5c06f5435f13576754c418ef21140571b3536f04f9ff25ab10`:
Def `46–51`→earlier Use `70–75` is Exact and the later Use `90–95` is absent. The local fixture
SHA-256 is `5c98fba3895da5d92840cd02ddcd7d0d8f6725f537529da653a7e37980416250`:
Def `24–29`→earlier Use `56–61` is Exact and the later Use `76–81` is absent. Those PASS rows
characterize the base gap; only a new missing-later-edge assertion is behavioral RED.

## Priority 2 — fixed-source decision census for excluded parameter/owner forms

**Why second.** This should be a measurement increment, not a blanket admission patch. The
accepted WS2 implementation deliberately supports simple optional identifiers and inert
defaults. It still refuses destructuring/rest/default-expression expansion and new owner
inventory (`src/parameter_slots.rs:111-119,240-248`). The shared plan records
unparenthesized-arrow syntax counts of 4 public and 2,966 private, while warning that they are
not recoverable named-owner flows; destructured/rest parameters have no measured reachable
demand or approved slot/alias semantics. Those populations answer different questions and
must remain separate.

**Input gate.** Before measuring, inventory the required compiler, observer, corpus roots,
file manifests and source pins without substituting historical paths. Acquire or restore
them only under explicit owner authority, authenticate their complete hashes/populations,
and freeze custody. If the public or private population cannot be authenticated, report
`input-blocked`; do not report zero yield, reuse old aggregates as current results, or replace
the missing population with toy fixtures.

**Three ledgers, no combined numerator.** Once that gate closes, use the pinned parameter-site
observer and authenticated fixed inputs to produce:

1. **Destructuring before a later optional identifier**, for example
   `function take({x}: X, value?: unknown) { sink(value) }`. Current TypeScript slot
   extraction breaks at object/array/rest patterns (`src/parameter_slots.rs:451-459`), so a
   later optional occurrence cannot be assumed to retain ordinal 1. Measure compiler-valid
   syntax, native owner, exact occurrence, slot/hole shape, entry Def, and constructible
   call-site ordinal separately.
2. **Rest siblings**, including the already-preserved
   `function take(value?: unknown, ...rest: unknown[])`. Existing tests require `value` to
   survive before the unsupported rest sibling (`src/ast_required_parameter_tests.rs:345-354`).
   Measure rest-token/alias demand separately; never turn the rest barrier into positional
   compression.
3. **Arrow owner inventory.** Separate supported parenthesized owners such as
   `const take = (value?: unknown) => sink(value)` from the historical unparenthesized
   `value => sink(value)` cohort. The latter is an owner/occurrence inventory question, not
   optional-marker syntax. Arrow-field receiver work already showed that membership alone
   did not change the measured real sites (`docs/eval/receiver-closure/2026-09-04-arrow-members-readout.md:49-61`).

**Measurement oracle.** For every fixed file, bind compiler version/hash, file hash, syntax
diagnostics, UTF-8 parameter bytes, native callable owner, slot vector including explicit
holes, entry Def presence, exact call argument ordinal, and refusal reason. Report unique
constructible call spans and added/removed flows, not syntax-entry counts. Include duplicate,
escape, recovery, omitted argument, nested destructuring default, and body-Def-fallback
negatives already represented in `src/cpg/optional_parameter_tests.rs:153-245`.

**Decision rule.** Promote only one cohort to an implementation spec if it has nonzero
constructible demand and a complete token-owner-slot contract. Destructuring requires a slot
representation with holes and explicit alias semantics; rest requires a non-compressing
variadic policy; arrows require stable callable ownership. Complex/effectful defaults,
optional-with-default, decorators/properties/erased `this`, TDZ/default-value propagation,
and new receiver authority remain excluded regardless of counts.

**Stop condition.** A compiler-clean syntax row without native owner, exact entry token, or
unambiguous ordinal is a measured refusal, not a candidate flow. Do not infer owner authority
from `all_functions()` membership or from the compiler syntax census.

**Architecture disposition.** One observer-only slice may measure all three cohorts only
under one authenticated corpus manifest, observer version and deterministic row budget, with
separate denominators, outputs and promotion decisions. Promote at most one constructible
cohort. Current historical public and private populations remain independently
`input-blocked`; no acquisition is authorized here.

## Priority 3 — WS4 compiler-owned input packet for one public project boundary

**Why third.** Compiler syntax has been useful as an independent observer, but it is not yet
production ownership. Existing source-backed measurements keep native directory inputs,
configured roots, repository Program files, dependencies, compiler files, and the whole
Program as distinct sets. All seven public packets were semantically closure-incomplete;
the complete root failed `input_budget`, while smaller selections exposed parse/config/
closure refusals (`docs/eval/receiver-closure/2026-09-09-project-boundary-feasibility.md:30-53,78-105`).
WS4 should therefore close one ownership packet before any cap increase or compiler-driven
admission.

**Input gate — updated 2026-09-19.** The exact public Excalidraw source archive is now
authenticated at commit `0642e72cfa2d9a71198200e52f37399384610ee3`; its root
`package.json`, `tsconfig.json`, and `yarn.lock` match the historical control hashes.
This closes public **source** custody only. The historical installed root, complete dependency
bytes, compiler Program closure, and all private worker inputs remain unavailable. The prior
missing `real-sites.jsonl`/private `PRISM_AUDIT_*` custody inputs and lack of an authenticated
public dependency root still prohibit assuming measurement reuse. Do not run a compiler-feasibility
measurement or infer a zero result until those separate inputs are authorized and authenticated;
otherwise close the affected increment as `input-blocked`.

**Bounded increment.** Select one existing public config/root from the pinned Excalidraw
population and generate a non-authorizing candidate packet containing:

- exact configured roots, compiler Program membership, repository/dependency/compiler
  partitions, source hashes, resolution/effect obligations, and all excluded inputs;
- native loader correspondence and explicit representations for JSON, `.mts`, and `.cts`;
- deterministic budget accounting for files, bytes, packet output, and worker termination;
- evidence that the supplied Prism inputs, detached sources, and project Program members
  satisfy the existing equality/closure policy, or the exact first refusal if they do not.

The implementation seams are policy gates, not the language producers:
`src/executable_owner.rs:122-241`, `src/executable_owner/acquisition.rs`, and the detached
compiler worker. Existing policy requires input budgets, a non-authorizing detached schema,
and complete semantic closure. Do not route a compiler Program directly into CPG ownership.

**Measurement oracle.** Freeze compiler 5.9.3, config and repository commit, then run
cold/repeat/mutated/restored packets. A valid packet reproduces byte-for-byte, changes when
one member/config/dependency is changed, restores exactly, and refuses missing dependencies,
outside lookup, duplicate identity, unsupported representation, budget overflow, and worker
failure. Compare complete sets; counts alone are insufficient. The private worker remains a
separate classification and cannot be recorded as an empty Program.

**Stop condition.** If no existing public config satisfies closure and representation within
current caps, close this increment as a source-backed refusal with an options brief. Do not
raise 512, drop inconvenient files, flatten inherited configs, install dependencies, treat
JSON as executable, or equate compiler membership with runtime-edge authority.

**Architecture disposition.** Conditional on complete original-root custody, use the
historical Excalidraw `fractional-indexing` config as the bounded packet falsification target:
historically 2 repository + 216 dependency + 86 compiler files = 304 Program files. This is
below the other configs' observed Program sizes, but it had zero historical real receiver
sites and supports no value claim. The 2026-09-19 public-source archive makes the original
source/config side available, but does not change these historical installed-root counts or
close dependency/program/production-budget predicates. Keep the full installed root/inherited
configs; do not reroot or construct an artificial equality subset. The native root historically
contained 628 files, so config selection does not itself bypass the production input budget.

## Recommended sequence and second-opinion questions

1. Dispatch Priority 1 only after the controller-designated independent architecture review
   approves the bounded spec and internal exact/legacy compatibility strategy at round 2.
2. Run Priority 2 as an observer/measurement slice. Use its constructible-flow results to
   choose exactly one later syntax/owner implementation; authenticated zero yield ends the
   branch cleanly, while unavailable inputs end as `input-blocked`.
3. Run Priority 3 as an architecture/feasibility slice. Production admission remains a
   successor even if one packet closes.

Settled architecture questions:

- Use a new internal exact occurrence/edge type; do not change `VarLocation` identity.
- One Priority 2 observer slice is acceptable only with separate cohort ledgers and one
  authenticated manifest; it authorizes no syntax implementation.
- The conditional WS4 falsification target is `fractional-indexing`; complete-root custody,
  closure and production-budget predicates remain load-bearing.
