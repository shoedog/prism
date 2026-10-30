# Public parameter syntax-frequency readout

## Bound artifact and population

- Tool commit/tree: `f79bb95485940bcb1418a2510cd9ec73a65ca37f` / `c5c48a54f2b4ba989c9c3ad7d2a4b4f36a5aca29`
- Final repair source manifest: `/private/tmp/prism-post318-parameter-frequency/repair-freeze/source-manifest.md`, SHA-256 `2eb75cd65acf7220279ef264540a3b45b5b47f6cbcf07fe8c7b8bf8eb2b23740`
- Core review: APPROVE, 0 WRONG / 0 SMELL; receipt SHA-256 `69cfb7d4695c99899d2bd6c2bb910ce7534f7e9cd104c8901a2dd9ab73bbd676`
- TypeScript: 5.9.3, entry SHA-256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`
- Source: Excalidraw commit `0642e72cfa2d9a71198200e52f37399384610ee3`, tree `709e9146b0fbd78c3ebf0d77e67143b2fbc43e4a`
- Input manifest SHA-256: `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696`
- Fixed population: 414 files / 5,036,151 bytes / 5 declaration files / 62 test-path files
- Public output: `/private/tmp/prism-post317-parameter-frequency-verification/final-f79bb954/cold.json`, SHA-256 `e953121bbcdda492bb651e4ef8f27b9af3de7a0221ee2a98fcda611645a7896d`, 1,677,371 bytes
- Cold/repeat result: byte-identical; `repeat.json` has the same SHA-256 and byte length
- Independent recomputation: PASS with exact equality for all five frequency-table rows; `recomputation.json` SHA-256 `6f379b664c857041effe613a129656da56b57e986fbacadaad4874fd554d5e02`

## Coverage and diagnostics

| Metric | Observed |
|---|---:|
| Files | 414 |
| Clean files | 414 |
| Files with syntax diagnostics | 0 |
| Syntax diagnostics | 0 |
| Excluded bodyless/signature nodes | 437 |
| Body-bearing callable syntax nodes | 5,494 |
| Parameters | 4,600 |

## Cohort frequencies

| Script kind | Declaration | Test path | Parse state | Files | Diagnostics | Excluded bodyless | Body-bearing callables | Parameters | Object patterns | Array patterns | Nested patterns | Nested defaults | Rest | Parenthesized arrows | Unparenthesized arrows | Destructured before later optional params | Callables with that relationship |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Tsx | false | false | clean | 195 | 0 | 220 | 2,175 | 2,154 | 296 | 8 | 2 | 61 | 2 | 2,090 | 0 | 0 | 0 |
| Tsx | false | true | clean | 46 | 0 | 3 | 1,245 | 203 | 4 | 0 | 0 | 0 | 2 | 1,239 | 0 | 0 | 0 |
| TypeScript | false | false | clean | 152 | 0 | 202 | 1,638 | 2,113 | 54 | 15 | 0 | 10 | 9 | 1,077 | 2 | 0 | 0 |
| TypeScript | false | true | clean | 16 | 0 | 0 | 436 | 130 | 1 | 0 | 0 | 0 | 0 | 433 | 0 | 0 | 0 |
| TypeScript | true | false | clean | 5 | 0 | 12 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Direct observations

- Across 4,600 parameter rows, the parser emitted 355 object-pattern parameters, 23 array-pattern parameters, 2 parameters with nested patterns, 71 with nested defaults, and 13 rest parameters.
- The 62 test-path files contained 1,681 body-bearing callable syntax nodes and 333 parameters; the 347 non-test, nondeclaration files contained 3,813 body-bearing callable syntax nodes and 4,267 parameters.
- The fixed population contained 4,839 parenthesized arrow syntax nodes and 2 unparenthesized arrow syntax nodes in the reported body-bearing categories.
- No emitted parameter row in these five strata matched the tool's destructured-before-later-optional relationship. This is zero observed syntax in this population, not proof of general absence or native behavior.

## Interpretation limits

This is a syntax-only observation from TypeScript 5.9.3 `createSourceFile` over one fixed public source population. It does not use a TypeScript Program or type checker, Prism, native parameter slots, owner/call resolution, CPG/dataflow, or runtime reachability. Bodyless signatures are tallied separately and have no parameter rows. Diagnostic-bearing files remain in their own strata. Counts do not establish semantic validity, native support, production readiness, or demand outside this population. A zero count means only that no matching syntax was observed in the named cohort.
