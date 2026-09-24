# Native parameter-slot characterization

This opt-in observer reports current native AST parameter, slot, and binding
observations. It is research-only: it does not build or read CPG/navigation
caches, resolve callees, authorize runtime edges, or add parameter support.

Build the native worker with the locked dependency set:

```sh
cargo build --offline --example parameter_slot_characterization
```

Run the bounded launcher only against an authenticated manifest and source root:

```sh
node scripts/parameter-slot-characterization/index.mjs \
  --root /absolute/source-root \
  --manifest /absolute/site-manifest.json \
  --manifest-sha256 <64-lowercase-hex> \
  --native /absolute/parameter_slot_characterization \
  --native-sha256 <64-lowercase-hex> \
  --out /absolute/new-output.json
```

Every flag is required exactly once. The launcher verifies the manifest,
selected regular source files, native binary, source byte limits, strict UTF-8
and selector boundaries before sending one JSON request to the worker. The
worker reads no repository files. Successful output is canonical JSON and is
published new-only through a parent-owned staging file and atomic link.

The supplied root is checked exactly as written after absolute resolution: the
launcher `lstat`s its filesystem anchor and every component down to each selected
file, refusing every symlink. On macOS, pass a canonical root such as
`fs.realpathSync(os.tmpdir())`; a root reached through `/tmp` or `/var` is refused.

The expected hash proves supplied binary bytes, not their build provenance.
Keep the build command, binary SHA-256, source binding, and gate logs in the
separate receipt/handback before any controller-authorized public run.

Tests require the explicit built binary and fail if it is absent:

```sh
PRISM_NATIVE_PARAMETER_EXAMPLE="$PWD/target/debug/examples/parameter_slot_characterization" \
  node --test scripts/parameter-slot-characterization/index.test.mjs
```
