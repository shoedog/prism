# Gate-input durability — Slice A acceptance receipt (2026-09-24)

- **Source:** `f52a50f72ad80a4f077c6db8b3b5984282421c8e` on `feat/gate-input-durability`. Spec v3 is `docs/superpowers/plans/2026-09-24-gate-input-durability/SPEC.md`.
- **Implementation review, 2 rounds:**
  - Round 1: sol FIX 2W/1S; terra FIX 1W, a duplicate.
  - Round 2: sol **APPROVE** 0W/1S; terra 0W/1S.
  - Both round-2 SMELLs were fixed in `f52a50f7`. The injected TypeError now exits 1, and the root-prefix mutant is killed.
- **Tests:** 28/28 Node tests; behavioral RED admissible; mutants all killed.
- **Lines:** 293/300 helper, 313/350 tests.

## Live acquisition (§7 A1)

Command: `node scripts/gate-inputs/acquire.mjs`, run on the default durable root `~/.local/share/prism/gate-inputs`
(`XDG_DATA_HOME` unset). Exit 0.

| Artifact | Result | Files / dirs |
|---|---|---|
| `typescript-5.9.3.tgz` | verified | 132 / 0 |
| `@types/react-18.3.31.tgz` | verified | 15 / 2 |
| `csstype-3.2.3.tgz` | verified | 5 / 0 |
| `@types/prop-types-15.7.15.tgz` | verified | 4 / 1 |
| `@types/react-19.0.10.tgz` | verified | 24 / 4 |
| `csstype-3.1.3.tgz` | verified | 5 / 0 |
| `upstream.tar.gz`, `javascript-0.23.1.tgz`, `tree-sitter-macos-arm64.gz` | verified (raw) | — |

Install directories, one per input:

- `typescript/7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54`
- `profiles/aa290dd630dcbfea5523bb627e7e3f6008de70c6fa0601c0df82d1d9975ff443`
- `grammar-archives/35ba6cbb8757dd812ab334163cac59ef2f6bdcccb1588b4d97d941e1cf579e5f`

These equal the spec §2 digests.

## Identity and verify (§7 A2–A3)

- **Cold identity.** Acquisition into a fresh durable root yielded the identical three digests.
- **Rerun.** A second run on the default root reported every input as `verified`, with no refetch.
- **`verify`.** Exit 0; it has no fetch path.
- **`env`.** Exit 0. The output evaluates under `sh`, and `PRISM_TYPESCRIPT` hashes to `3ae902c9…7675`, the
  `COMPILER_HASH` pin.

## Unblocked modules (evidence of value)

The 7 modules that the 2026-09-23 gate had to exclude now run against these inputs:

- `verify-callable-authority`
- `callable-observations/{index,module-search,nested,props-class,umd-qualifier}`
- `verify-typescript-grammar`

Result: **112 tests, 110 pass, 0 fail, 2 skipped** (Node v24.15.0 host). The two skips are the grammar verifier's
Node v26.0.0/darwin/arm64 host pin, as expected on a v24 runtime.

The logs are external, at `~/prism-evidence/gate-input-durability/acceptance-A/`.

## Default Rust suite

Recorded by the implementer on this slice: 4,559 passed / 0 failed / 1 ignored. It is unchanged; no Rust path was
touched.
