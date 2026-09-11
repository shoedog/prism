# JS/TS/TSX loop-header definition repair

Both loop-header lvalue extractors now inspect only the grammar's `left` field.
For `for (const item of items)`, `item` remains a Def and the iterable read
`items` no longer becomes one. Identifier/destructuring handling is retained;
missing or unsupported left forms do not trigger a sibling fallback. Real
assignments nested inside RHS expressions and loop bodies still produce Defs.
CPG cache version advances from 79 to 80; skip-policy and navigation call-edge
cache versions remain unchanged. Call resolution and parameter binding are not
expanded.

## Fixed-population value checkpoint

The unchanged PR310 native observer/comparator was replayed against the same
immutable source copies. The prior candidate's production sources are identical
to merged base `e8e4c77e7f634077f2090bf57e59f665ac0aacab`; candidate production is
`c54846f8` (verification checkpoint `344c3848` differs only in documentation).
File hashes, skips, parse status, function inventory and parameter/slot shapes
match. The comparator rejects population changes. A separate multiset check
proves every removed flow is exactly a previously source-classified loop-read
target, with no added flow. Private source and graph rows remain local.

| Observation | Public Excalidraw before → after | Authorized private frontend before → after |
|---|---:|---:|
| Loaded files / skipped files | 628 / 584 unchanged | 1,122 / 140 unchanged |
| Functions | 8,614 unchanged | 12,713 unchanged |
| Parameter-token DFG and CPG Defs | 3,892 unchanged | 1,676 unchanged |
| Slot-token-matched Use→Def flows | 6,016 unchanged | 2,420 unchanged |
| Unmatched Use→Def flows | 9 → 3 | 68 → 61 |
| All observed Use→Def flows | 6,025 → 6,019 | 2,488 → 2,481 |
| Added / removed flows | 0 / 6 | 0 / 7 |

These are graph observations, not unique source calls, compiler-verified recall,
or new receiver/executable authority. Remaining unmatched observations are the
previously classified local-declaration targets (3 public, 2 private) and later
assignment targets (59 private). Their callee-body name-search defect remains
for the separate exact supported parameter-token binding slice.

## RED-first contract and corrected assumptions

Seven AST tests cover both path/span and query/manual routes across JS/TS/TSX,
in/of/await, bare/declaration/destructured lefts, missing/unsupported left,
multiline Unicode, complex/member RHS, empty line selections, and same-name
left/RHS tokens. Two CPG tests retain supported line-anchored Uses and reject
false RHS Defs and their argument-flow targets. Python's existing body-assignment
behavior is a non-JS control. An existing cache pin test fails at 79 and passes 80.

The final nine-test pre-repair focused run gave 4 passed / 5 failed; the later
same-name regression separately failed 1/1. After repair the combined focused
selection (nine new tests plus one existing lexical-loop test) passed 10; cache
passed 1. Controls that already passed are not represented as RED regressions.

| Hypothesis / alternative | Probe and observed result |
|---|---|
| All-child loop extraction invents an iterable Def; alternative: downstream aliasing | Direct AST path/span query/manual matrices fail before DFG construction. Both helpers include RHS identifier siblings; left-only repair makes the same matrices pass. Aliasing is not required. |
| Python has a loop-left Def to preserve | Initial control fails on unchanged base. That assumption is inadmissible; corrected ordinary body-assignment/no-iterable-Def control passes before repair. No Python production change. |
| RHS Uses have exact token spans; alternative: existing line anchors | The assignment query excludes `for_in_statement`; supported plain parameters generate line-anchored Uses through parameter-reference jobs. Exact-span filtering rejected legitimate anchors. Tests now preserve that existing contract; member/exact read provenance stays deferred. |
| Removing a name is sufficient | `for (item of item)` RED proves a name filter would lose the legitimate left token. Exact left span survives after repair, RHS span does not. |
| Suppressing the loop helper might lose actual assignments | Nested RHS `items = load()` and body `output = item` remain Defs through recursive assignment collection before and after repair. |

## Verification

Full runner started and ended on clean, unchanged `344c384825db44507841ca53e1b53c5c7956315e`; subsequent closeout changes are documentation only.

| Gate | Result |
|---|---|
| Rust default / MCP / MCP+detached-owner-audit | 4,132 / 4,325 / 4,348 passed; one existing ignore each |
| Observers / receiver helpers / authority controls | 726 / 18 / 40 passed |
| Formatting / diff | passed |

Rust ignore: `resolution_test::slice_elem_variant_reserved`.

Python passed 940 with one intentional live-adoption skip, all native examples
passed 32, and comparator passed 10. Clippy completed successfully with warnings;
this is not a warning-free claim, nor an attribution that every warning predates
the change.

Tier-A matrix: 159 ok, 0 regressions after an immediate fresh release build.
Same-environment base and candidate quick runs are both INVALID: corpus SHA
drift from pinned `20c8490591a3`, C-method/C-name/U-method each 4/6 successful
probes, and oracle error rate 0.20 above 0.10. SUT error rate 0 on both. Candidate
ran from clean `344c3848`; base had two test-only untracked source files.
The oracle samples are not a matched accuracy comparison: pending rows 115→64
must not be claimed as improvement. Neither run changes an adjudication/baseline.

Identical pinned outcomes in both runs: `target-c-method` is a flip candidate;
`module-deps-feature-gated` and `load-repo-feature-gated` are missing expected
oracle-miss sites; `ambiguous-symbol-contract` is ok. Full raw diagnostics stay
in local custody and compact pinned differences accompany the verification
receipt and PR. No full multi-corpus, live-model or compiler-Program accuracy
evaluation was run.

Independent review round 1 of cap 2 approved implementation/checkpoint with
0 WRONG and 0 SMELL. Full gates were pending at review and are recorded separately
above. Round 2 independently approved the documentation-only closeout at
`2b8436d836440bb41f87caa3279607a3b1f1cade`, again 0 WRONG / 0 SMELL.
The subsequent review/publication-status record is self-checked, not a third round.
See the [verification receipt](2026-09-10-js-ts-loop-header-defs-verification.json).

Publication is blocked locally: `git push` requires approval while this session
has AskForApproval set to Never. No PR was opened; the owner-run push command and
resume steps are in the successor handoff. No alternative publication path was used.

Prism navigation was used to trace affected consumers, but reported 41 changed
paths; current source establishes the DFG-only scope. No React.FC, optional or
default parameter expansion, closure admission, or unresolved react-scripts
decision changes.
