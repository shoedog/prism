# Callback argument-flow ownership audit and proof requirements

Seventh local increment, base7bdf9981, same future MR bundle. Classification and
executable proof requirements only; no production fix or cache/policy expansion.
Two review rounds maximum. Root design/test writer, existing reviewer read-only.
Completed2026-09-12: two closed assertion gaps corrected at round2; full gates
refreshed, exclusions retained. See the readout and handoff for measured results.

Trace resolved identity → source argument spans → exact callee parameter Def →
caller occurrence selection. Compare namespace and named imports, declarations,
compact/multiline function expressions, arrows, property/object-pair/variable
initializers and assignment-head line placement. Inspect overlapping assignment
query captures, raw rvalue spans, DFG occurrences, CPG index selection and edges.

Keep a separately ignored desired-behavior regression with captured RED; the
normal audit tests assert current classified dispositions, not a completed repair.
Test-only reindexing may establish sufficiency of an existing true DFG argument
occurrence, but must not become a production fallback. Preserve byte containment,
exact parameter-token Def, lexical self refusal, unsupported-parameter refusal and
genuine same-line earlier-use collisions. Full/incremental source transitions use
actual changed app sets. Audit nested callable ownership and path/span siblings.

Future proof split: (1) caller-contained query captures, preventing overlapping
ancestor RHS ingestion; (2) nested callable signature/body ownership and genuine
RHS reads, without conflating execution scopes. Review both path/span consumers.
The measured first mechanism is assignment-expression specific: the JS/TS/TSX
assignment-kind gate rejects variable declarators. Simple const function, arrow
and object-pair initializers are supported controls, not affected fixtures.
Nested initializer returns still demonstrate the separate execution-scope issue.
Do not fix the first-per-line occurrence model, loosen argument containment, or
special-case namespace names as part of a bounded first repair. Default/destructured
parameter evaluation and nested captures need explicit scope decisions/negatives.

Run focused observations and explicit ignored RED, full Rust configurations and
examples, helpers, formatting/Clippy, fresh-release Tier-A matrix/quick before final
review. State new known-gap ignore separately from the pre-existing reserved test.
No real-corpus claim, React.FC, closure admission, unknown CJS wildcard or require
snapshot expansion. Static skipped CJS names remain an independent export item.
