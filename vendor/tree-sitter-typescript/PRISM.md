# Pinned Prism grammar dependency

Upstream: tree-sitter/tree-sitter-typescript 0.23.2, commit
`f975a621f4e7f532fe322e13c4f79495e0a7b2e7`. MIT license retained in LICENSE.
This is a source-vendored Rust path dependency, not a published upstream release.
Ordinary Cargo builds compile the committed C; they do not need Node, network,
the generator, or any npm lifecycle script.

## Reproduction

Verified generator: tree-sitter 0.24.4
(`fc8c1863e2e5724a0c40bb6e6cfc8631bfe5908b`), ABI14, Node v26.0.0,
JavaScript grammar 0.23.1. Both versions match the pinned upstream package-lock.
The bootstrap currently pins the macOS arm64 generator binary; other hosts can
build the committed Rust/C dependency normally, but regeneration needs a separately
verified generator binary pin. Do not silently substitute a newer generator.

From the Prism root:

```sh
node scripts/verify-typescript-grammar.mjs --download
# Or reuse an archive directory containing the three verified input files:
node scripts/verify-typescript-grammar.mjs /absolute/path/to/pinned-archives
```

The script verifies exact archive SHA256s and JavaScript's upstream lockfile SRI,
unpacks only into a fresh temporary directory, and verifies generator identity.
It regenerates the **unchanged** baseline first, compares every src/ byte, then
regenerates with the authored grammar and compares every shipped src/ byte.
It retains a receipt and scratch directory; it never edits this vendor directory.
No global package installation or npm lifecycle script is used.

Published baseline parser SHA256s:

- TypeScript: `74fe453edd70f4eae9af0a1050cbd7943d8971d59165b6aaebbaa0a0b716d1aa`
- TSX: `1902cb53fa7ff5179df89b2eea863165e84c8cc866226419dc26921d8c055885`

## Authored changes and review boundary

- common/define-grammar.js: replace the two ordinary-call/type-query static
  precedence pairs with explicit conflicts, preserving the type interpretation
  until the enclosing context is known. Fixes first-position inline import types.
  A dynamically preferred generic-call alternative retains the expression path at
  `<`, then applies static call precedence only at the argument list/completion.
  This keeps `await` outside the full call without disturbing ordinary calls,
  constructors, comparisons, instantiations or class heritage.
- bindings/rust/build.rs: watch both src trees and common headers for native rebuilds.
- This notice. All other copied non-generated upstream files are unchanged.

The generated C/JSON files are mechanical output, not independently authored code.
Root build.rs binds every regular file in this bounded vendor tree to grammar and
cache identities, including same-version modifications and added/removed files.
Keep bootstrap directories and symlinks outside this tree.

The original regression expectations remain active, not ignored. Focused tests
and all112 upstream corpus cases pass. Final full gates/review are tracked in
the repair handoff; this notice is dependency provenance, not merge approval.
