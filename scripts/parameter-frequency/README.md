# Parameter syntax frequency

This opt-in research tool counts TypeScript parser syntax in a fixed, hash-authenticated
source population. It does not build a TypeScript Program, invoke Prism, or authorize a
runtime edge, parameter slot, support decision, or production change.

## Run

Supply the pinned TypeScript 5.9.3 entry and an existing output path's parent directory:

```sh
node scripts/parameter-frequency/index.mjs \
  --root /absolute/source-root \
  --manifest docs/eval/parameter-syntax-frequency/input-manifest.json \
  --manifest-sha256 <sha256> \
  --typescript /absolute/typescript-5.9.3/package/lib/typescript.js \
  --out /absolute/new-output.json
```

The output path must not exist. The tool validates the complete manifest, recursively
checks the selected source population, hashes and strictly decodes every member, verifies
the compiler entry, then parses the retained bytes with `createSourceFile`. It writes one
canonical JSON packet with a trailing newline. Syntax diagnostics are observations and do
not abort later files.

## Bounds and meaning

- 512 source files and 8 MiB of source bytes
- 200,000 file, callable, parameter, and diagnostic rows
- 128 MiB serialized output
- 15-minute owned-child timeout

The packet schema is `prism.parameter-syntax-frequency/1`; it always carries
`authorizes_runtime_edge:false` and `measurement:"compiler_syntax_only"`. Frequencies
cover body-bearing syntax only. Bodyless/signature syntax is counted separately and emits
no parameter row. Counts from diagnostic-bearing files remain in their own strata.

Run the focused tests with the already acquired compiler:

```sh
PRISM_TYPESCRIPT=/absolute/typescript.js \
  node --test scripts/parameter-frequency/index.test.mjs
```
