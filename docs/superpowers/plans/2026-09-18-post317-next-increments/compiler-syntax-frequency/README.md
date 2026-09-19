# Compiler syntax frequency — accepted narrow slice

**State (2026-09-19):** Planning is accepted and the controller has dispatched
implementation on `feat/parameter-syntax-frequency`; core review is pending.
The user authorized this slice through review, publication, green CI, merge, and
successor planning. The controller owns commits, publication, and merge.

## Scope and gate

The candidate owns only `scripts/parameter-frequency/` and its tests, plus the
compact documentation named by [SPEC.md](SPEC.md). It uses the fixed 414-file
manifest and pinned TypeScript compiler. It does not add Rust, native joins,
Program/type-checking, public parsing before core approval, or parameter-support
claims. Do not run candidate verification or public parsing until controller
core approval and a frozen candidate manifest exist.

## Durable accepted packet

- [SPEC.md](SPEC.md)
- [IMPLEMENTOR.md](IMPLEMENTOR.md)
- [REVIEWER.md](REVIEWER.md)
- [INDEPENDENT-REVIEW-round1.md](INDEPENDENT-REVIEW-round1.md)
- [INDEPENDENT-REVIEW-round2.md](INDEPENDENT-REVIEW-round2.md)
- [artifact-manifest.json](artifact-manifest.json)

The source packet was copied from
`/private/tmp/prism-post317-planning/compiler-syntax-frequency/`. Its source
hashes and byte-exact copy mapping are in the parent [CUSTODY-MAP.md](../CUSTODY-MAP.md).
Only this README is a current custody/status index.

## Source binding

PR #318 merged at `e2d177d5144126a83159f41f1b2ee14e8eeb2b94`, tree
`f21553d63fb99a3c267d44fc39b098c8fe8c7f0b`, matching published head
`85d4b3c0090dfaf7b33f9eb0348fff80cbf784fb`. The reviewed Priority 1 source
remains `14e86083c2c754ed412d440742bd5763be388e42`.
