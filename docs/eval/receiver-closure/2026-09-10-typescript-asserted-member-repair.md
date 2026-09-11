# Bounded TS/TSX asserted-member field-flow repair

Base: PR307 merge `c2737e06d0c5a65e8f5b11bf3c10cb78f88ebd36`.
Reviewed production/test checkpoint: `cef3b902ee8fdf9627917a88b1c05a635c628029`.
Later closeout/publication commits change documentation only.

## Outcome and scope

The twelve compiler-backed TS/TSX cases now retain the exact `runtime.X`
Def@2 → Use@3 edge. A shared AST descriptor recognizes one literal ASCII dot
member with an identifier receiver through at most eight parentheses/as/satisfies
wrappers. It coordinates rvalue paths/spans, scoped reference matching, asserted
writes, and CPG argument/return binding. Names, raw trees, source, calls and return
texts stay unchanged. Member envelopes and receiver-token bytes remain separate.

CPG argument lookup additionally follows only transparent outer parentheses,
sharing the eight-wrapper budget with the receiver; return lookup stays exact
member only. Both serial and parallel argument consumers use the same helper.
Existing same-line occurrence-index collisions and reaching-confidence policy
are preserved, not claimed solved. Cache version77 →78 invalidates old graphs;
the schema and generic AccessPath string parser remain unchanged.

No class/type/import/receiver/executable-owner or closure authority is inferred.
React.FC and unresolved react-scripts decisions stand. Two unknown-type fixtures
remain compiler-invalid but parser-clean: their runtime field identity normalizes
without resolving the annotation. Malformed-type and invalid TSX angle fixtures
do not gain the edge. Compiler validity admits evidence, not runtime type authority.

## RED, compatibility and review

- Rebuilt base: exactly12 missing-edge failures. Final fixed population:54
  observations,49 compiler-valid,5 expected invalid,0 verification failures.
- All26 refused native observations are byte-identical to the same-environment
  base; full/subset graph records and labels match. Compiler byte spans are
  checked against actual native spans. Six mutated-receipt controls are refused.
- Initial focused RED:5 passed/11 failed. Four initial CPG failures also lacked
  plain callee definitions and were not admissible member-regression evidence.
  The corrected JS-callee fixture isolates4 consumer failures (2 controls pass).
- Review round1 found one draft WRONG: `(runtime /* c */ .X)` lost an existing
  argument binding. Fifteen base/candidate envelope probes isolated that loss.
  Captured targeted RED, then the bounded grouping repair and17 focused passes.
- Round2 of cap2: APPROVE,0 WRONG/0 SMELL, prior WRONG closed. Review inspected
  source and root-generated evidence; it did not independently execute tests.
  Prism navigation was stale/unavailable; current-source tracing established
  consumer requirements. No exhaustive semantic-reference claim follows.

## Verification

| Gate | Measured result |
|---|---|
| Full Rust default / MCP / MCP+detached-owner-audit | 4,112 /4,305 /4,328 passed; one ignored each |
| Observers / receiver helpers / authority profiles | 726 /18 /40 passed, no skips |
| Python | 940 passed; one intentional live-evaluation skip |
| Focused Rust / native example / membership example | 17 /7 /12 passed |
| Proof policy / comparison mutation controls | 7 /6 passed |
| Formatting / diff / Clippy | pass /pass /completed with warnings |
| Tier-A matrix | 159 passed against a freshly built frozen binary |
| Tier-A quick | INVALID accuracy result; corpus pin drift and oracle6/30 errors; SUT0 errors |

Rust ignore: `resolution_test::slice_elem_variant_reserved`. Python live skip
requires explicit `PRISM_RUN_LIVE_EVALS=1`; no live/full multi-corpus run occurred.
Clippy is not warning-free, including a test-only single-element-loop warning.
No real receiver-population, recall, taint-reachability or closure-completeness
measurement is claimed.

The eight-gate runner started/finished at identical clean `cef3b902`. It recorded
one failed gate:15/18 receiver helpers, because the retained upstream fixture
had zero of606 required files. The base artifact reproduced all three failures.
We enumerated the complete missing population and reconstructed the pinned
Excalidraw archive from local Git objects, without download/install or modifying
the depleted archive. A first isolated recheck omitted two environment variables
and was inadmissible. With the complete original gate environment and restored
archive, both base and candidate pass18/18. The initial failed runner receipt is
retained; it is not relabeled green. Other seven gates passed unchanged.

Python initially938+3 skips lacked binary variables;940+1 follows fresh binaries.
A preliminary Tier-A quick was stopped after its mutable binary path was rebuilt;
it is not evidence. Final quick used frozen `cef3b902` bytes, recorded clean corpus,
and remains invalid: pin `20c8490591a3` differs from current source and oracle20%
exceeds10%. A same-environment base-artifact replay also rejects base `c2737e06`
for pin drift. Oracle failures are not attributed to this patch; no base oracle
performance comparison or baseline/adjudication change was made.

## Separate pre-existing defect and next recommendation

**Historical PR308 finding, repaired for required identifiers in the
[successor](2026-09-10-typescript-required-parameter-occurrences.md):**
`function take(a: any, b: any) { sink(a, b); }`
parses cleanly in TS/TSX but produces no parameter Defs; JavaScript's equivalent
produces both. The frozen base reproduces this. `function_parameter_slots` accepts
the TS grammar wrapper, while `extract_param_name_node` omits `required_parameter`
and `optional_parameter`; occurrence extraction therefore loses required names.
Even plain argument-to-parameter binding can be absent, independently of assertions.

The approved successor implements bounded TS/TSX occurrence repair for simple
required identifier parameters and explicit typed/untyped/plain-argument controls;
preserve duplicate, write, optional/default/rest and malformed-pattern barriers.
Do not expand type or executable-owner resolution as part of that prerequisite.

## Custody

[Harness](asserted-member/README.md) ·
[Machine receipt](2026-09-10-typescript-asserted-member-repair-verification.json) ·
[Handoff](../../superpowers/handoffs/2026-09-10-typescript-asserted-member-repair.md).
Raw evidence: `/private/tmp/prism-asserted-repair-byXvBe`.
Restored pinned source: `/private/tmp/prism-asserted-upstream-CXalOE` at Excalidraw
`0642e72cfa2d9a71198200e52f37399384610ee3`; no dependencies installed or source run.
Raw evidence archive hash/location are recorded in the machine receipt. Archives
remain local; committed code/docs/receipt provide remote reproducibility custody.
