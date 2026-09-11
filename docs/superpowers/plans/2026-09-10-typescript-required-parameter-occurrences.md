# Bounded TS/TSX required-parameter definitions

Base: merged PR308, `f369f20cdcd3463bae86b191bcdd8ec9efd8f6fd`.

## Evidence and design

WRONG: `function take(a: any, b: any) { sink(a, b); }` parses cleanly but
`function_parameter_occurrences` and the DFG both omit its parameter definitions.
The positional slot extractor already recognizes these parameters. Base RED:
`/private/tmp/prism-required-params-UgjX70/base-red.log`, 3 failures / 2 controls pass.

Hypothesis: the occurrence extractor omits `required_parameter`. Expected:
clean grammar nodes and empty occurrences/Defs; falsifier: occurrences present.
Observed: expected, including the emitted AST. Alternative: call resolution or
slot refusal loses the edges. Direct AST/DFG tests reproduce before either
interprocedural consumer is involved, distinguishing that alternative.

Pinned TS/TSX node-types and live parse trees are grammar authority. The Prism
caller graph is stale (40 paths); current source confirms DFG and reasoning seed
consumers. JS/TS callback-local binding collection takes its separate path and
does not consume this API; quantum parameter-name consumers are Go-specific.

Add a dedicated non-positional extractor alongside the slot safety helpers.
Only `required_parameter` with a direct `pattern: identifier`, optional type
annotation and comments qualifies. All direct children, including unnamed
tokens, are allowlisted. Preserve exact identifier bytes, including Unicode;
annotations do not confer compiler, class, receiver or executable authority.

Whole-list recovery, duplicate bindings (including unsupported patterns), and
escaped binding spellings refuse all new occurrences. The latter avoids claiming
source spelling equals decoded JS identity. Optional/default/rest/destructured,
decorated, parameter-property and erased `this` forms gain no occurrence. No
unparenthesized-arrow expansion. Names derive from occurrences for TS/TSX.

Supported bindings after an unsupported parameter may be observed for local DFG
and seeds. They are NOT positional slots: the existing slot prefix and duplicate
barriers remain unchanged. Do not zip occurrences to arguments. Preserve existing
field isolation, write/shadow confidence and same-line occurrence-index semantics.
Invalidate CPG caches with version79; no schema or skip-policy change.

## Verification and boundaries

Capture base RED, then exact names/token spans, grammar forms, per-predicate
refusals, multiline definitions, full/subset byte equality, real TS/TSX callees,
plain/asserted argument paths, serial/parallel parity and write/shadow controls.
Run full default/MCP/owner-audit Rust, observer/helper/authority suites, offline
Python and freshly rebuilt Tier-A matrix/quick. Report invalid gates and baseline
failures, do not rebaseline. Review cap: two rounds; open-class findings stop.

No React.FC, closure admission, react-scripts, new receiver authority, optional or
default semantics, generic identifier-span repair, or auto-merge. Follow-up only
after measured value and remaining gaps are classified.
