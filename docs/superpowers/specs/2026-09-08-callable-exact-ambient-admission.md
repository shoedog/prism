# S7 — additive strict exact-ambient semantic closure

Implement the adopted [S6 contract](2026-09-08-callable-semantic-policy.md) as
schema18 / producer0.19.0 `semantic_closure`, policy
`prism.semantic-closure/exact-ambient-v1`. No existing packet field changes except
schema/producer envelope. Old status/reasons/closure and Props/class statuses keep
their prior meaning. Both authority flags remain false.

The accepted dependency-free S6 classifier is wired without changing its bytes.
Its digest now participates in the producer identity. After the final snapshot and
stable legacy resolution sorting, the worker derives its inputs from current packet
facts: compiler verification, stable-snapshot bit, observed config, S5 entry
completeness, original diagnostics/reasons, every outside/refused/boundary count,
Program membership and exact ordered resolution rows. noResolve is true only from
observed config's present noResolve option and the canonical true-value digest.
No second resolver, host call, declaration census or compiler-option rewrite occurs.

Nonnull targets require Program membership for filesystem_selected. A null target
can qualify as exact_ambient only through the pre-existing observed exact binding
lane. Wildcard/merged observed lanes remain unadmitted. Competing provider,
augmentation, unsupported context and unrelated missing rows retain refusals.
The helper's complete result is a conjunction; eligible rows never erase another
global prerequisite. Null targets and old unresolved_module presence must agree.

The parser validates all historical structural facts before recomputing the entire
new field. Row/index/reason/policy/complete substitution is rejected before root I/O.
This checks consistency, not the original callback census; full source reproduction
still authenticates omitted requests and same-shaped source/anchor substitutions.
Empty refusal packets use the same classifier with false prerequisites and empty
facts. They cannot become vacuously complete.

Schemas10–17 remain readable without invented semantic fields. Current production
always emits schema18; full validation rejects a historical packet as current.
Future policy versions must preserve schema18 exact-ambient-v1 semantics, not make
historical packets adopt a newer wildcard/merged rule. There is no caller-selected
policy relaxation and no production consumer of this observation.

Constructible strict positives are node:known/virtual:known exact ambient imports
without baseUrl, and selected local source modules. Plain/scoped package-like exact
bindings still encounter outside lookup space and remain incomplete. This is actual
compiler proof plus retained barriers, never prefix authority. Required negative
fixtures cover all independent reasons, duplicates/augmentation/context, noResolve,
automatic uncertainty, absent-Program targets, same-genuine row swaps and source
change/full reproduction. S8/S9 admission and executable receiver authority remain
separate decisions.
