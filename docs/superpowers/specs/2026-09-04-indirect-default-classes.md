# Indirect local default-class identity

Authority: owner approved indirect local default-class exports with duplicate/write
barriers, followed by real receiver measurement. Base: merged #242, `854d53f`.
Three self-review rounds maximum; no baseline rewrite or full multicorpus run.

## Contract and architecture

Admit only `class C { ... }; export default C;` (also a directly named exported
local class followed by the default identifier expression). The class must be a
unique, undecorated module declaration completed before the export. Preserve
declaration-backed owner identity through existing Class export facts and
clean_class_spans. Root parse errors, competing value bindings/imports, duplicate
exports and visible module writes poison the default export, including writes
after the export or inside escaping functions. Shadow-local writes do not poison
the module binding. This is conservative for export-expression snapshots.

Do not reinterpret a rejected class as an exported function. Non-class function
defaults retain their existing behavior. Exclude local export lists, arbitrary
expressions, alias chains, class expressions, imported-class forwarding, reexports,
decorators and pre-declaration exports. Type-only imports remain erased; no new
receiver shapes, class fields or Python behavior are introduced. CPG/nav versions
advance together because resolved topology changes even without a new field.

## Hypothesis / probe / result

Expected: existing class/constructor recovery works, but export identity stops at
Local rather than Class. Alternative: the constructor itself lacks receiver
metadata. Same-environment RED on untouched base: constructor metadata contains
Alias/ConstructorLocal, but only NameOnly m edges; positive test fails, full
duplicate/write matrix passes (1 pass, 1 fail). This discriminates the export-fact
gap from constructor recovery. prism-nav could not find the new consumer symbol
in its index; current source is authoritative, not that failed index lookup.

## Plan and verification

1. Add the bounded AST class-identity proof; reuse lexical/write proof and poison.
2. Positive and negative JS/TS/TSX matrices; preserve callable defaults, erased
   imports and reexport barriers. Exercise cold/subset, cached incremental A→B,
   duplicate/write toggles and nav sidecar projection. Full default/MCP suites.
3. Pin real Excalidraw source at 0642e72cfa2d9a71198200e52f37399384610ee3.
   Measure Library plus its direct consumers from unchanged archived source,
   before/after with cache-bypassed production binaries. Inspect all changed
   Exact targets and both served directions when additions exist.
4. Immediate rebuild before Tier-A matrix and quick; retain stale-pin/baseline
   caveats. Commit/push/open PR with reconciled predecessor state and handoff.

Initial source census corrects a prior assumption: Library methods are arrow
fields; App uses this.library and menu consumers use destructured props or a
destructured useApp() return. Export
identity alone does not authorize those receiver shapes. Zero measured gain is
an admissible result, not permission to expand the implementation silently.

## Implementation and self-review record

- Round 1 WRONG, corrected: `class Client { m() {} }; import Client from './other';
  export default Client` produced Exact Client.m after the first implementation.
  Alternative was a broken duplicate-class scan; the full enumerable matrix
  passed duplicate classes, functions, variables and writes, isolating imports.
  The lexical candidate collector does not model imports. Added explicit value
  import binding exclusion alongside the separate erased-import check.
- Round 2 survived: cold/subset identity, 23-case negative matrix, terminal class
  poison vs callable defaults, cached good↔bad and A↔B owner replacement, sidecar
  reload. Full suites: 3744/0/1 default, 3934/0/1 MCP. CPG64/nav33.
- Round 3 survived (self-pass, not independent): reread producer/consumer and
  persistence diff; real-source probe confirms Class(Library), no conflict.
  Alternative explanation for zero gain (identity still rejected) is falsified.
  Remaining arrow-field / this.field / destructured-prop barriers are outside
  the approved scope. All 2780 real sample records unchanged; prior Python/JS
  samples byte-identical. No further implementation fixes or round extensions.

Published implementation: `257fb7f`, PR #244 OPEN, not merged. Current custody is
`docs/superpowers/handoffs/2026-09-04-indirect-default-handoff.md`; the publication
follow-up is documentation-only.
