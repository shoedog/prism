# Native parameter-slot characterization

This opt-in observer reports current native AST parameter, slot, and binding
observations. It is research-only: it does not build or read CPG/navigation
caches, resolve callees, authorize runtime edges, or add parameter support.

Build the native worker with the locked dependency set:

```sh
cargo build --offline --locked --example parameter_slot_characterization
```

Build the request from the pinned manifest, then pipe it to that worker:

```sh
node scripts/parameter-slot-characterization/request.mjs \
  --root /absolute/source-root \
  --manifest /absolute/site-manifest.json \
  --manifest-sha256 <64-lowercase-hex> \
  --native-sha256 <64-lowercase-hex> \
  | target/debug/examples/parameter_slot_characterization > output.json
```

All four flags are required exactly once. The builder verifies the supplied
manifest digest, requires the frozen digest for non-synthetic manifests, and
reads only safe relative member paths. Before each read it requires a regular
file, the declared byte size, and the remaining 256 KiB budget; it then checks
the source digest and strict UTF-8 while preserving a BOM. It writes one
canonical request JSON object and a trailing newline to stdout.

The worker re-validates the full request before parsing and writes its canonical
result to stdout. The controller owns the pipe, output custody, and independent
reconciliation. This intentionally does not add staging, publication, binary
execution, child limits, or hostile same-account TOCTOU protection.

The expected native hash is controller-supplied request metadata; it does not
prove source provenance. Record the build command, binary SHA-256, manifest
digest, builder digest, output digest, and exact command in the separate
receipt before any controller-authorized public run.

Tests require the explicit built binary and fail if it is absent:

```sh
PRISM_NATIVE_PARAMETER_EXAMPLE="$PWD/target/debug/examples/parameter_slot_characterization" \
  node --test scripts/parameter-slot-characterization/request.test.mjs
```
