# Bounded erased-type argument/rvalue traversal

Base: PR305 merge70caaf6d43f9bbd1ecf47279af0db05ce02753f4, freshly fetched.
Owner approved the next recommendation; branch fix/typescript-erased-rvalue-traversal.
Evidence: /private/tmp/prism-erased-rvalue-avuSbL. Review cap: two rounds.

## Defect and discriminating probes

H1: the Calls query walks erased import-type arguments before the guarded name
lookup. The known compiler-valid options object therefore emits `with` as a value
use through identifier/path/span APIs. A standalone type alias isolates this path.
Alternative H2: enclosing assignment/argument/return traversal enters the type
subtree independently; an assigned `as`/`satisfies` expression discriminates it.
Fresh RED must establish each; prior logs are inherited, not this slice's control.

## Design and proof boundary

Share the existing TS/TSX erased-context predicate from call-name extraction.
Keep its supported boundaries (type query/annotation/arguments/parameters/alias,
and only the type side of as/satisfies). Do not change grammar or raw syntax APIs.
Use the predicate at the three shared value-collector entry points, then prune
supported erased boundaries during recursive descent. Check ancestry once per
entry, not once per descendant, to avoid introducing a quadratic ancestor walk.
Retain runtime values, call arguments, object labels and original UTF-8 spans.
The identifier helper also feeds condition variables; that consumer needs a test.
Private manual fallbacks and return-span traversal need explicit coverage.

Prism navigation returns StaleIndex; current source confirms consumers in
src/data_flow.rs and src/cpg/reaching/scope.rs plus the condition helper. This is
source-backed consumer tracing, not an LSP-exhaustive references claim.

## Acceptance and non-goals

Capture RED for all three APIs, assignment/call/return/condition nesting, selected
lines and direct collection roots. Preserve TS/TSX and JS real dynamic-import
options, runtime typeof, mixed same-line source, labels and call/argument identity.
Compare full/subset DFG observations and private manual/query collectors. Validate
fixtures with pinned TS5.9.3; inspect emitted JS, never execute it. Then run full
Rust default/MCP/owner-audit, observers/helpers/authority/Python, fresh Tier-A
matrix+quick, and independent review. Preserve baseline failures and invalid oracle
results without rebaselining. No full multi-corpus or live-model run.

No general TypeScript erasure/DFG soundness claim, receiver or closure admission,
React.FC expansion, react-scripts change, grammar/bootstrap/vendor modification,
private source reads, real-source execution, installs or registry publication.
Cache formats stay unchanged; existing source-byte build identity invalidates
affected persisted artifacts. Keep duplicate/write/epoch barriers unchanged.
