# Merged wildcard proof audit — 2026-09-07

The remaining **84 bindings are side-effect imports of `*.scss`**, all bound
to one Vite empty module block plus one project shorthand declaration. They
are not interchangeable typed providers. No observation or closure is promoted
in this slice; schema9 / producer0.10.0 remain unchanged.

## Source custody and measurements

- Merged main: `d66328d1ad5b11ef937cd0823fac093f5e8c01bd` (PR274).
- Fixed public Excalidraw source: `0642e72cfa2d9a71198200e52f37399384610ee3`.
- Pinned compiler: TypeScript5.9.3, hash recorded in adjacent evidence/spec.
- Fresh packet SHA256: `f8b266d6bf7de4124d968d31187c82cd32fd94d8e3aae786baf89afe3c509fa5`,
  byte-identical to PR274, including every old observation and refusal.
- 1,445 original Program source files hash-checked and independently parsed;
  all84 request/declaration/matching-provider anchors checked.
- 80 request files;75 distinct relative asset inventory candidates, all present
  with checked bytes. No competing pattern, exact provider or relevant augmentation.

These are occurrences, files and asset paths respectively, not 84 unique assets.
The evidence is for this fixed public copy, not a new frontend-portal measurement.
No installs, source/config/lock changes or runtime analyzer changes were made.

## Hypothesis / probe / result log

1. **Replay:** unchanged producer should reproduce the prior packet; different
   bytes would falsify stability. Result: exact SHA equality, not merely matching
   counts. Alternative “same counts, changed evidence” is excluded by byte equality.
2. **Provider semantics:** empty versus shorthand may depend on selected value
   declaration. Reversing the same pair should alter value-import results if
   selection matters; unchanged selection/results would falsify that mechanism.
   Result: first selected declaration changes named/default acceptance; typed
   `value:string` becomes `any` when shorthand is first. The alternative that
   missing assets cause these diagnostics is excluded by keeping the same absent
   asset while only provider order changes. Pinned source explains selection.
3. **Real contexts:** if these are purely side-effect requests, each parsed parent
   must be an import with no clause. Result:84/84. This does not prove selected
   valueDeclaration identity; the public packet lacks that field, explicitly
   left unmeasured. Minimal compiler controls are not relabeled public observations.
4. **Hidden barriers:** the current duplicate reason could conceal additional
   matching patterns or augmentations. Full original-source Program census finds
   none at these84 sites. This is stronger than trusting the prioritized reason.
5. **Asset separation:** otherwise-identical present/absent fixture assets should
   preserve source binding and closure refusal. Result: both retain two providers,
   null filesystem target and unproven closure with clean diagnostics. Inventory
   membership differs; therefore binding and clean diagnostics do not prove presence.

No production WRONG found. The order-sensitive behavior refutes a proposed
equivalence assumption, not the existing fail-closed implementation.

## Verification and reproduction

Task root: `/private/tmp/prism-merged-wildcard-d18kes`.

```bash
PRISM_TYPESCRIPT=/path/to/pinned/typescript.js node --test scripts/callable-observations/merged-wildcard-proof.test.mjs
node docs/eval/receiver-closure/audit-merged-wildcard.mjs /path/to/pinned/typescript.js /path/to/fixed/source /path/to/packet.json
```

The audit script checks the expected packet hash and all source hashes before
asserting the fixed population; it is not a generic resolver or packet validator.
Use the producer's `validate` command separately for full recomputation.

Characterization:9/9 locally; exact-base control and full gates are in progress.
These tests pass pre-change production; no behavioral RED is claimed for an
audit-only slice. No Tier-A run is triggered: `src/ast.rs`, `src/call_graph.rs`,
`src/navigation/` and `src/cpg/` are unchanged; human-triggered full multi-corpus
evaluation was not run. No LSP tools were exposed, so the LSP-navigation skill
used pinned compiler/source fixtures as the fallback.

## Next

Implement only the [bounded proof requirements](../../superpowers/specs/2026-09-07-callable-merged-wildcard-proof.md)
for a separate source-pair observation on side-effect imports. Retain legacy
duplicate refusal, asset independence and closure/class barriers. Defer typed
merges, value imports, React.FC and all closure-policy changes.
