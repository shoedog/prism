# Bounded merged wildcard observations

Approved successor to PR275's source-backed proof requirements; base e45eaef0.
This implements an observation, not asset presence, lookup success, runtime/class
authority, React.FC support or Program closure admission.

## Contract

Schema10 / producer0.11.0 adds `lookup.merged_wildcard`, null unless the existing
wildcard record has multiple source providers or checker declarations. Existing
`lookup` fields and `lookup.wildcard` remain unchanged, including duplicate refusal.
The new non-null record has exactly four fields:

- `status`: observed or unproven; `reason`: null only when observed.
- `declarations`: checker-ordered source entries `{declaration: anchor, shape}`.
- `selected`: the actual checker `symbol.valueDeclaration` as the same entry
  shape, or null when no owned module declaration is available.

Shapes: `empty_block`, `shorthand`, `other`. `other` is evidence, never eligibility.
The old wildcard record supplies pattern/provider/match/augmentation census anchors.
No selected declaration is inferred from order; preserve the checker sequence.

Positive: actual source-owned side-effect import literal, no import clause,
attributes or modifiers; null filesystem target; no exact providers or relevant
augmentations; exactly two matching providers with no competitor; checker identity
equals the two original-source census declarations; top-level non-external `.d.ts`
contributors, one empty block and one shorthand; ValueModule symbol with both
declaration-name symbols equal to the request symbol; selected declaration is one
of the two owned contributors. Either selected shape is allowed, for this request
context only. The pinned compiler probe confirmed both declaration-name symbols
refer to the actual merged request symbol, rather than distinct pre-merge symbols.

Reason priority: unsupported_request, filesystem_target, augmentation,
provider_count, competing_pattern, exact_provider, binding_mismatch,
unsupported_provider, selected_declaration. Reject unsupported contexts and source
shapes even when diagnostics are empty. Keep Program-local census/cache ownership,
source redirects, snapshot/write/refusal/budget barriers unchanged.

Parser rejects malformed fields/anchors/inconsistent positive evidence before
audited-root access; full recomputation rejects well-shaped genuine declaration
substitution, omission and stale epochs. Schema9 packets are not upgraded by trust.
The producer digest includes the new helper. No worker closure-policy edits.

## Plan / verification

1. Capture public-seam RED on unchanged merged main (new field absent).
2. Implement a small helper reusing the configured-Program original-source census;
   wire after the existing wildcard observation, then strict schema/producer bump.
3. Exercise both selected shapes; present/absent assets; context/provider/selection/
   augmentation/redirect/write/cache/forgery negatives. Keep exact-base controls.
4. Reproduce fixed public source before/after: require complete old-field projection
   equality and independent source checks at all84 sites. Measure selected source
   declarations; no positive-count promise before this measurement.
5. Full observer, default/MCP Rust, authority/helper gates. Two SELF-PASS rounds
   (NOT INDEPENDENT), then commit/push/PR and handoff. No cap extension by default.

Stop for unbounded merge semantics or changed closure behavior; do not silently
broaden typed/value imports or discard a partially reviewed artifact.
