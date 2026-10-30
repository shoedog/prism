# Exact caller-read post-design final Rust receipt

**Date:** 2026-09-19  
**Frozen checkpoint:** `14e86083c2c754ed412d440742bd5763be388e42`  
**Frozen tree:** `a0411db6d858fe34806376ce41d2621853848d8b`  
**Source manifest:**
`docs/eval/post317-exact-caller-reads/source-manifest.md`, SHA-256
`310b5152d598800a7520181fe53be58cb3677d3ca7421cc900c8005e78e92aa7`

The worktree stayed source-clean relative to this checkpoint throughout these
gates. The only source delta against planning base remains the manifest-bound
798 production / 1,662 production-plus-test changed lines within the amended
850/1,800 budget.

## Environment

- Cargo ran offline from `/private/tmp/prism-post316-slice1` using the shared
  frozen worktree target, with no concurrent worker Cargo process.
- Widest and examples supplied
  `PRISM_TYPESCRIPT=/private/tmp/prism-post316-orchestration/public-inputs/typescript-5.9.3/package/package/lib/typescript.js`
  and
  `PRISM_CALLABLE_PROFILES=/private/tmp/prism-post316-orchestration/public-inputs/callable-authority-profiles/profiles`
  on their first attempt.
- The three full populations each contain the standing ignored test
  `resolution_test::slice_elem_variant_reserved` only.

## Gates

| Command | Result | Log SHA-256 |
|---|---:|---|
| `cargo test --offline --no-fail-fast` | 29 suites; **4,559 pass /0 fail /1 ignored** | `41fea2e440c0dc522935113b61ba2f29ef748eecaf84f0ae07ddca778c2d06eb` |
| `cargo test --offline --no-fail-fast --features mcp` | 31 suites; **4,752 pass /0 fail /1 ignored** | `f8e4a21802c977e61d1ac89a3d3242a5915a0574a141600a844bffd9cd64f6d4` |
| pinned `cargo test --offline --no-fail-fast --features 'mcp detached-owner-audit'` | 31 suites; **4,775 pass /0 fail /1 ignored** | `b1b9ae887b08745944e7bd965c872170dad0911eb1923b66c0b4c85ef9d8a597` |
| pinned `cargo test --offline --no-fail-fast --examples --features 'mcp detached-owner-audit'` | 4 suites; **32 pass /0 fail /0 ignored** | `8efd943285e7a505f66194bc4f7b54d39dc1b098a80a4c0d545a1cfa39b23379` |
| `cargo clippy --offline --all-targets --features mcp -- -W clippy::all` | exit 0; 258 `warning:` lines including summaries | `6b2fd3fb08c76a0b4514f25d71ef92dd93d4d402aafac5454a89ad3f31a20295` |
| `cargo fmt --all -- --check` | exit 0; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `git diff --check` | exit 0; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The Clippy warning population is reported descriptively. No unchanged-base
Clippy comparison ran, so no inherited/new attribution is claimed. No failure
required a same-environment base test control. An initial post-processing regex
was over-escaped and selected zero terminal records; it was classified
inadmissible and corrected before these totals were computed.

## Boundary

These logs supersede the full-Rust completion claims tied to historical
checkpoints `3a2fbfa` and `1a8354b2`; those remain historical evidence for
their exact bytes only. This receipt does not claim R07 cache/cost, Node,
compiler-authority, Python, Tier-A, final review, publication, merge, release,
rebaseline or adoption completion.
