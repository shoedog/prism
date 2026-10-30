# Parameter syntax-frequency census

This directory records the fixed public TypeScript-parser census produced by the opt-in
tool in `scripts/parameter-frequency/`. The source implementation is frozen at
`f79bb95485940bcb1418a2510cd9ec73a65ca37f` and has final local approval with
0 WRONG / 0 SMELL. PR #319 passed all five pre-merge checks and merged as
`30e13053c9f3f9940b5526e20af0cb40f2a9fae4`; its tree exactly matches the
published head. The separate main-push CI run completed with all five checks green.

## Custody

- `input-manifest.json` is the complete 414-file Excalidraw population manifest, SHA-256
  `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696`.
- `readout.md` contains all five observed frequency strata and the interpretation limits.
- `receipt.json` binds the source, inputs, output digest, aggregate counts, gates, and
  retained external evidence.
- `reviews/core-acceptance.md` is the independent core review, 0 WRONG / 0 SMELL.
- `reviews/final-acceptance.md` reconciles the fixed public measurement and all local gates.
- `receipts/final-gates.md` records the public repeat/recomputation and final local gates.
- `receipts/pr319-premerge-ci.json` and `receipts/pr319-merge.json` preserve publication
  and merge custody. `receipts/pr319-main-ci.json` is the terminal all-green main-push
  receipt; `receipts/pr319-main-ci-pending.json` preserves the earlier observation.

The canonical 1,677,371-byte packet is retained outside git at
`/private/tmp/prism-post317-parameter-frequency-verification/final-f79bb954/cold.json`,
SHA-256 `e953121bbcdda492bb651e4ef8f27b9af3de7a0221ee2a98fcda611645a7896d`.
An independently produced repeat is byte-identical. The compact tracked readout avoids
committing the large packet while preserving its exact digest and source bindings.

## Meaning

This is a syntax-only observation from TypeScript 5.9.3 `createSourceFile`. It does not
use a TypeScript Program or type checker, Prism, native parameter slots, ownership,
call resolution, CPG/dataflow, or runtime reachability. Counts do not establish support,
production readiness, accuracy, or demand outside this fixed population.
