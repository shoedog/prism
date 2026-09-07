# Bounded UMD bridge implementation contract

Owner approved the next slice after PR270. Base9a34ef62. This implements the
[audit's seven proof requirements](2026-09-07-callable-umd-qualifier-proof.md),
not runtime receiver resolution or general ambient/merged namespace support.

## Architecture and plan

1. Preserve the dirty predecessor in local recovery custody; establish a clean
   main-based branch. No unique unfinished source was found; recovery is separate.
2. Convert audit singleton expectations to acceptance, retaining the negative
   populations. Capture failures on unchanged main before production edits.
3. Pass the exact configured Program to the provenance walker. Lazily census its
   global exports and global declaration names once per Program in a WeakMap.
   This scan shares the existing acquisition/time/heap bounds; the existing
   provenance-step counter charges alias hops and the source-module traversal.
4. For a NamespaceExportDeclaration, collect same-domain providers instead of
   mixing in module-local declarations. Require compiler globalExports ownership,
   top-level unmodified declaration syntax in an external .d.ts, canonical Program
   membership, unique owning source-module declaration and unique global provider.
   Refuse same-name script/global-augmentation bindings even under skipLibCheck.
5. Require exactly one export assignment, export= of a direct identifier naming
   one non-alias module-local namespace with a ModuleBlock in the same source.
   Verify immediate compiler alias and module export-table identity. Preserve
   existing import/shadow routes, local duplicate checks and star/merge refusals.
6. Retain source module, assignment and binding anchors using existing alias
   fields. Schema7/producer0.8.0 rejects pre-bridge packets before root access;
   full recomputation remains required. No persistent positive identity cache.
7. Run full observer and Rust gates, authority controls and fixed public replay.
   Preserve options/source bytes and incomplete-Program barriers. Class anchors
   can be retained while status remains program_unproven; no runtime recall claim.

## Tests and review

29 UMD tests include same-name and renamed singleton targets, pinned React18/19,
duplicate providers/assignments, module/global augmentation, explicit import/local
shadow controls, invalid/missing/imported/qualified targets, type/value option
semantics, budgets, epoch changes, tampering, receiver writes and Program closure.
Initial RED:10 failures/19 passes on unchanged base; first implementation:29 passes.
The negatives are controls, not separate implementation RED claims.

Two SELF-PASS rounds, NOT INDEPENDENT. Stop on an open-class identity gap; do not
expand to general UMD, imports as local targets, merged globals or runtime authority.
No LSP tools were exposed; pinned TypeScript5.9.3 source and executable compiler
fixtures supply the exact semantic fallback described in the predecessor audit.
