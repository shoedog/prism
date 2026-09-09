# S8 — bounded singleton-wildcard type-source eligibility

Schema19 / producer0.20.0 fixes policy
`prism.semantic-closure/singleton-wildcard-v2`. Extend the [S7 contract](2026-09-08-callable-exact-ambient-admission.md)
only in the additive semantic field. No old lookup/status/reason/closure/Props
field, target, authority flag, compiler option, host operation or runtime edge changes.

Keep semantic-closure.mjs exact-ambient-v1 byte-frozen. A separate dependency-free
v2 wrapper calls it with the same already-authenticated facts and cap. Promote only
a prior unproven/unadmitted_binding row whose actual target is null and existing
singleton wildcard observation is observed. The existing original-source provider,
checker-selected declaration, matching-pattern, duplicate, augmentation and supported
request-context guards supply binding proof; do not reconstruct it from spelling.

Emit singleton_wildcard/null at the same occurrence index. Keep every other row,
including non-Program targets and merged pairs, unchanged. Recompute only the final
resolution_unproven aggregate from all resulting rows; retain the order and values
of every other inherited reason. Thus an eligible row never waives a config, entry,
noResolve, diagnostic, source-reference, outside/refused or unrelated-null barrier.
Cap and input refusal occur in v1 before v2 reads rows. Inputs are never mutated.

Schema18 remains fixed to v1, while schema19 validates/recomputes with v2. Policy is
bound to the known schema/version branch, not selected from arbitrary packet/caller
options. Historical schemas10–17 have no invented semantic field. Full reproduction
continues to authenticate source facts and the actual callback census after parsing.
Both helpers participate in current producer identity; empty/refusal packets use
the current fixed classifier and remain incomplete.

Strict relative singleton-wildcard fixtures demonstrate absent asset, present asset
and query-suffix eligibility without boundary encounters. Asset presence does not
prove type binding or a runtime export; both authority flags remain false. Negative
controls cover duplicate/augmented/competing providers, dynamic context, merged pairs,
unrelated unresolved requests, outside/refused space, absent-Program target, genuine
row substitution, source change, immutable historical policy, cap and portability.
S9's merged pair and S10's production-authority contract remain separate slices.
