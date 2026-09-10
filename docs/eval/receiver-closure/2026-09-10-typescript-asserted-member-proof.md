# Asserted-member proof closeout — production defect remains unfixed

Historical PR307 proof-only result. The [subsequent bounded production repair](2026-09-10-typescript-asserted-member-repair.md)
supersedes its unfixed/current-work status; the evidence below remains the base control.

Runtime base: PR306 merge `58998926af2eb6ad7750e5a92a19f7b198993216`.
Reviewed/tested tooling: `b27f1f4dd1338e99d4e4588aa94cf9e6e15925e2`.
No production src/, manifest/lock, root build script or vendor changes. Later
closeout commits are documentation-only. This is a proof/tooling increment, not
the production normalization repair.

## Demonstrated defect and corrected assumptions

**WRONG:** the explicit `runtime.X` write at line2 reaches the plain and
parenthesized return read at line3, but not the asserted read in each of12
TS/TSX candidate cases. Definitions remain present; the field-reference edge is
missing. The pinned compiler validates the source and emits the same runtime
member identity. Minimal comparison:

```typescript
function owner(runtime: any, value: any) {
  runtime.X = value;
  return (runtime as any).X; // control: runtime.X
}
```

The first probe had no explicit field write and produced no Def for either
plain or asserted reads under existing parameter field isolation. It could not
prove lost flow. That output is retained as nondiscriminating representation
evidence. An opaque AccessPath spelling alone is not another WRONG: the generic
string parser has an intentional fallback. The explicit-write comparison supplies
the behavioral evidence. No real-site recall or taint-reachability claim follows.

Another corrected assumption: rvalue names/paths do not collect return-only lines;
the span sibling supplies these return uses. This API difference was not changed.
A whole runtime-member span can enclose erased type syntax; it is not therefore
an erased identifier use. Keep that source envelope distinct from the exact
receiver-token span and from a normalized logical path.

## Fixed proof and refusal population

27 definitions × TS/TSX =54 observations:49 fully compiler-valid and5 expected
invalid (unknown type and malformed type in both dialects; angle assertion in
TSX). All expected dispositions pass, no skips. Full/subset DFG records, including
endpoint bytes and labels, match exactly for each case.

24 detached syntax candidates consist of4 plain/parenthesized controls,8 different
receiver/field controls, and12 asserted repair cases. The latter cover as-any,
as-import, satisfies-import, nested wrappers, Unicode/comments and the exact
8-wrapper limit. All12 currently lose the field edge. Different receiver/field
controls do not gain an edge from runtime.X.

The proposed syntax boundary admits only one nonoptional literal ASCII dot-member
whose receiver is a literal ASCII identifier after at most8 parentheses/as/satisfies
wrappers. Refusals cover9 wrappers, computed/literal-computed keys, optional access,
calls, conditional/comma/assignment receivers, non-null/angle assertions, member
chains, escaped tokens and invalid fixture types. Several refused forms emit a
simple member: emission equivalence is necessary evidence, not sufficient scope
authorization. No compiler candidate is an executable-owner or binding proof.

The default observer run has0 failures; `--require-repaired` has exactly12 UNFIXED
failures and no others. Both are retained. The latter is a necessary future
dependency check, not a complete production acceptance gate. It does not establish
that every refused native observation stayed unchanged after a hypothetical repair.
The future implementation must add those before/after refusal controls.

Tooling hardening also has captured RED: removing an obligation class was initially
accepted (6 policy tests passed,1 failed). All5 missing classes were enumerated;
mandatory class presence now gives7 policy passes. No product code was changed.

## Architecture and next recommendation

Prism navigation returned stale/truncated evidence; current-source inspection
confirmed the consumers. This tracing established why a collector-only fix is
insufficient: `collect_path_refs` also serializes the entire member expression
and compares field paths before DFG edges are created. Reaching labels do not
reconstruct absent edges. CPG parallel/serial argument and return binding also
construct paths independently and need explicit compatibility checks.

Next recommended implementation: a bounded AST-native member descriptor, followed
by coordinated rvalue and scoped-reference matching, with CPG argument/return
compatibility and refusal tests before enabling the changed identity. Keep raw
source/returns/calls and true occurrence bytes; do not regex-strip assertions in
generic `AccessPath::from_expr`. Compiler validity here is admission of synthetic
evidence, not authorization to add a runtime compiler dependency or infer class
identity from annotations.

Argument/RHS/LHS, erased outer contexts, same-line duplicates, shadow/write sequences,
non-TS controls and interprocedural binding remain implementation acceptance work.
No closure/receiver authority, React.FC or unresolved react-scripts policy expansion.
The production repair is a separate owner-approved increment.

## Verification

| Gate | Measured result |
|---|---|
| Full Rust default / MCP / MCP+detached-owner-audit | 4,095 /4,288 /4,311 passed; one ignored each |
| Observers / receiver helpers / authority controls | 726 /18 /40 passed, no skips |
| Python | 940 passed; one deliberate live-adoption skip |
| Native observer / policy / executable CLI negatives | 7 /7 /7 passed |
| Membership example | 12 passed |
| Format / diff / Clippy | pass /pass /completed with warnings |
| Independent review | round1 of cap2 APPROVE proof tooling; WRONG0/actionable SMELL0 |

The eight-gate runner passed with identical clean HEAD at start and finish.
Rust ignore: `resolution_test::slice_elem_variant_reserved`. Python skip requires
explicit `PRISM_RUN_LIVE_EVALS=1`; it was not enabled. Tier-A is not triggered by
this proof-only diff; no earlier invalid Tier-A result is recast as a pass. Full
multi-corpus/live-model runs and real receiver-population measurements were not run.
Only compiler/native tooling executed, never fixture/emitted JavaScript. No private
source reads, dependency installation, source cleanup or automatic merge.

## Custody and reproducibility

[Harness and commands](asserted-member/README.md) ·
[machine verification](2026-09-10-typescript-asserted-member-verification.json).
Raw evidence: `/private/tmp/prism-asserted-member-proof-HFzXTL`, including the
SHA-bound native observer executable, exact compiler/native receipts, diagnostic
probes, source audit, review, all gate logs and pre-review source snapshot.

Archive: `/private/tmp/prism-asserted-member-b27f1f4d-evidence.tgz`,8,577,563 bytes,
mode0600, gzip integrity checked; SHA256
`cbf2e00c0cc04c0a275c149e83fbacebf3cae07b71bd332f5a33d761fc6a1f4e`.
The raw archive remains local, not a GitHub attachment. Committed tooling, fixtures,
readout and receipt provide remote custody and reproducible commands.
