# Bounded imported object-alias identity

Approved successor to PR256, merged e08af858. Branch feat/imported-object-alias-identity.
Three-round SELF-PASS cap; no subagents. No React.FC, hook inference, imported
interfaces, barrels, namespace/query imports, generic aliases or callable expansion.

## Contract

Admit an explicitly annotated required destructured parameter whose type is one
plain named ESM import (including type-only and local renaming) from one unambiguous
relative source module. The target must directly export one non-generic object
type alias. Retain the existing own-property shape and receiver write/duplicate/
shadow barriers. Resolve the selected class reference in the alias declaration's
file, never in the consumer. Existing class export/member barriers remain terminal.
Contextual annotation expansion is deferred in this first imported slice.

Persist a source-span-keyed proof carrying alias declaration and property type
positions plus defining-file identity. Recompute it from the complete supplied
ParsedFile map on full and direct-subset builds; incremental merge REPLACES it.
No old positive proof survives a new missing/ambiguous/unsupported input. CPG and
navigation versions advance together for the new persisted proof and edge authority.

This is the existing literal-relative-module, supplied-source-snapshot contract,
not TypeScript project resolution. No tsconfig paths/package exports/dependency
closure is claimed. Any observed TypeScript parse error or ambient declaration
(including module/global augmentation and included d.ts declarations) blocks the
new route conservatively across that snapshot. Missing or ambiguous source modules
fail closed. Inputs outside the supplied file universe remain outside the guarantee;
the prior P5 audit must not be relabeled a closed compiler-program proof.
Prototype access (including reads that could escape an alias) and observed Object/
Reflect mutator member access also block the new route across the supplied JS/TS
snapshot. This conservative fence does not change existing local receiver routes
and does not claim arbitrary dynamic reflection or general runtime alias analysis.

## Plan and verification

1. Pin compiler5.9.3; rerun the24 shared audit cases. Diagnose stale Prism lookup
   independently using identical before/after-refresh queries.
2. Capture RED for direct imported alias ownership against a consumer decoy,
   then implement declaration-backed proof and exact direct-method consult.
3. Test positive TS/TSX forms and negative imports, shapes, shadows, writes and
   augmentation. Exercise add/delete/change transitions in both directions,
   full/direct-subset/incremental and disk/navigation cache parity.
4. Full default/MCP suites; fmt/clippy; immediate release builds before matrix/quick.
   Report quick exclusions/invalid baselines without rebaseline; no full multicorpus.
5. Re-measure fixed real sites: no gain expected (four React.FC,two hook producers).
   Archive evidence, reconcile docs, commit/push/PR. No merge authority.

## Diagnostic log

Prism hypothesis: stale snapshot versus name-only lookup failure. Symbol query
missed; location3532 returned an older function and StaleIndex90 paths. Refresh
reported stale_before_refresh=true; identical symbol query then returned the
current caller without warnings. Staleness explains the reproduced miss; no
persistent lookup defect demonstrated. LSP tools unavailable in this session.
Pinned TypeScript5.9.3 compiler probes:24/24 pass (same outcomes as6.0.3).

Round1 RED: two new integration tests fail on e08af858 runtime with0 Exact instead
of1; logs red.log. Initial implementation's49-case negative population enumerated
three admitted exclusions (extra export clause,star export,prototype patch).
Captured round1-population.log; targeted proof-boundary fixes pass in round1-fixed.log.
Round2: disk-cache24 bidirectional transitions and four navigation-sidecar states
pass. Existing inline-prop tests and shared audit guards pass; only the approved
imported_alias_declaration_scope guard is promoted to Exact. No rebaseline.
