# Bounded asserted-member path repair

Base: PR307 merge `c2737e06d0c5a65e8f5b11bf3c10cb78f88ebd36`.
Owner authorization: “merged - proceed to next”. Production repair, tests,
commit/push/PR are in scope; automatic merge and authority expansion are not.

## Contract and architecture

Recognize a parser-clean TS/TSX `member_expression` with a literal ASCII identifier
property and receiver, separated by at most eight parentheses/as/satisfies nodes.
Reject optional/computed/private/escaped members and other receiver forms.
Select assertion operands using operator positions, ignoring comments. Never
interpret assertion types, resolve imports, or modify generic `AccessPath::from_expr`.
An unresolved but syntactically valid annotation is not a runtime identity barrier:
the compiler-validity gate in PR307 admits proof fixtures, not production types.

Keep the logical path, original member envelope and real receiver-token span
separate. Use the descriptor for rvalue path/span collection, scoped field
reference matching, and supported lvalue path/span collection. Names retain
their existing token-traversal contract. Coordinate CPG argument (parallel and
serial) and direct member-return lookup using exact AST span lookup, retaining
legacy fallback on refusal. Do not normalize enclosing arbitrary expressions.
For arguments only, transparent outer parentheses share the same eight-wrapper
budget with the receiver. Return lookup remains exact-member-only. This closes
the draft's observed commented-member argument regression; outer assertions,
comma/array/spread expressions are not traversed by argument path lookup.

The CPG cache version must advance because graph contents change; schema and
navigation call authority are unchanged. No React.FC, closure policy, receiver
ownership or unresolved react-scripts policy changes.

## RED and acceptance

1. Rebuild merged base observer; capture PR307's exact twelve missing edges.
2. Add RED regression tests for path/span/reference and write consistency,
   argument/return consumer compatibility and byte-containing duplicate handling.
3. Implement the bounded descriptor and coordinated consumers.
4. Require the 54-observation compiler/native proof repair gate; compare refused
   native observations against the same-environment base binary. Preserve raw
   source/tree/calls/returns and full/subset exact records and labels.
5. Run focused tests, full Rust default/MCP/owner-audit, observer/helper/authority,
   Python, formatting/Clippy, and rebuilt Tier-A matrix and quick gates. Report
   inherited failures via same-environment base controls, never rebaseline.
6. Independent review cap: two rounds; closed bounded findings get targeted
   fixes, open-class findings stop for design. Publish a clean reviewed branch.

## Hypothesis log

- H1: raw field reference/path disagreement drops the asserted read edge.
  Expected: same-environment base has twelve asserted failures and passing plain
  controls. Falsifier: absent definitions/control edges or unrelated failures.
  Alternative: parser rejection or reaching-label behavior. Compiler/native
  parse checks and retained definitions distinguish parser rejection; exact
  reference matching inspection distinguishes absent edges from later labels.
- Navigation: Prism caller result is stale; current-source reads confirm the
  scoped collector and CPG consumers. No exhaustive semantic-reference claim.
- H1 result: base-red.json reproduces exactly12 expected failures; initial AST
  repair gives54 observations/0 failures and26 byte-identical refusals.
- H2: TS argument fixtures initially lacked even plain callee edges. Parameter
  slot refusal was falsified by source: slots accept required_parameter. The
  separate occurrence extractor omits required_parameter, confirmed by frozen
  base native output (typed TS/TSX defs[], JS defs[a,b], all parse-clean).
  Tests now use a resolved JS callee, isolating the intended binding repair.
  This pre-existing parameter defect is documented, not repaired here.
- H3: an outer argument parenthesis might break path compatibility. Initial
  asserted-envelope probes had no baseline edges either, so no regression claim.
  Expanded15-case base/candidate probe confirmed exactly one loss:
  `(runtime /* c */ .X)` bound on base but not draft. Independent review round1
  rejected with1 WRONG/0 SMELL. Captured envelope-red.log, then bounded argument
  grouping repair: focused-final-green.log17 passed. A numeric-type compile error
  during this repair was inadmissible behavioral evidence, not a test regression.

Review round2 APPROVE0 WRONG/0 SMELL. Final proof54/0 failures,26 unchanged
refusals; focused17pass. Python initially938+3skip without binary variables,
then940+1live skip with fresh binaries. Tier-A matrix159pass. A preliminary
quick run was stopped because its mutable SUT path was rebuilt during the run;
it is inadmissible, not regression evidence. Final quick uses frozen binary bytes.

Full Rust default/MCP/owner-audit4,112/4,305/4,328 passed, one ignored each;
observers726 and authority40 passed. Original helper gate15/18 failed because
all606 required files were absent from its retained archive; base reproduces.
Reconstructed exact pinned source locally. An incomplete-env recheck is invalid;
complete-env base/candidate rechecks both18/18. No audit code changed.
Final frozen Tier-A quick is INVALID (pin drift and oracle6/30), SUT0 errors;
base-artifact pin checker also refuses base. Do not infer an accuracy pass.

Status: implementation/review and runnable gates complete; documentation/publication next. Evidence root:
`/private/tmp/prism-asserted-repair-byXvBe`.
