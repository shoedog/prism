# Build and custody binding

Measured clone: `/Users/wesleyjinks/code/prism-pkgres`, branch `plan/workspace-package-resolution`, base `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, dirty uncommitted prototype. Runtime caches CPG/nav **107/63**. `BUILD-MANIFEST.json` is the machine-readable controller authority for the final pinned binaries and native probes; it contains 547 production/vendor/script/build inputs and 447 test-file hashes. No Git writes.

| Role | Retained path | SHA256 |
|---|---|---|
| base | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/base-prism` | `c5aea1b30cd3534972bcaa6d5d111d5dba8c24c22c97997e163d45a2cfdfb979` |
| head | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/head-v2-prism` | `cba28d1eebc38fed13dbd9b41ae30f65dceb5050c8d2e89143920ebfc79d21fd` |
| basefacts | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/base-facts` | `c722fd8e8759e9ac7e34cefae323a70f774c54e848d8704d3301090759870147` |
| headfacts | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/head-v2-facts` | `1f31b824babf496af4a3e7c0c8309f72d297a5795d0d658faac3b9f112c40b0d` |
| s2base | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/s2-base-prism` | `4a457b80669cd4d27138e8213e8ef6fc0811cd6a05ad205751bb6b0d5c80acf7` |
| s2pkg | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/s2-pkg-v2-prism` | `3da62e4bcd0c6f24ad5e0d36f5d7a4a399e2ff40815ddbcaaccc4e37cb80b32f` |
| s2basefacts | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/s2-base-facts` | `351175b10cfad74eb42d1392bff3613ce2d12a843b3772ddb6b66390ef12d54f` |
| s2pkgfacts | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/s2-pkg-v2-facts` | `4783be7af9dccc06db19519a0df86438acdcf161a3317b30fa8c50d3f6e5e897` |

Oracle TS **5.9.3**, `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`, SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. Rust 1.94.0, clippy 0.1.94, nextest 0.9.146, Node 26.0.0, Python 3.9.6. All builds offline; no install/update/fetch.

Base `planning/base-binding.json` was captured before edits, with 304 source/vendor/build inputs. Its retained `base-source.tar` restores the clean base. `base-auxiliary-binding.json` retrospectively pins 242 tracked script assets to exact HEAD bytes and verifies current parity; initial checkout was clean. This retrospective supplement is not presented as a contemporaneous base-build hash census.

Final `planning/head-v2-binding.json` freezes 547 production inputs before and after release compilation. The facts helper is Cargo-compiled from `probes/dump-facts.rs`, copied to a temporary example, then removed; the probe source hash is part of this manifest. `final-test-binding.json` records final source/test inputs, registry and final full-suite log. Earlier head-final/pre-mode artifacts are historical and do not certify this final candidate.

Reproduction from this exact checkout (temporary example must be removed before fmt/clippy):

```bash
cargo build --offline --release --bin prism
mkdir -p examples
cp docs/superpowers/plans/2026-10-04-workspace-package-resolution/probes/dump-facts.rs examples/pkgres_final_facts.rs
cargo build --offline --release --example pkgres_final_facts
# Copy prism and facts executables to a new immutable evidence directory.
rm examples/pkgres_final_facts.rs
```

S2 is a separate scratch snapshot of parent `61641bdb64c911049c567efc51c29e17ef94ef4f`. It retains all tracked include_bytes script assets. Overlay the production delta (apart from the different lib.rs context), register js_packages, and apply the measurement-only external-builtin branch in Resolver::refusal_module_inner. Scratch runtime caches **121/77**, parent **120/76**. No S2 source was adopted or copied back. `planning/s2-pkg-v2-binding.json` includes its complete source/build/helper bytes and executable hashes. Its full test suite was not run; these binaries certify measurements only.

Local checkpoint archives preserve the pre-design census, prototype, and final candidate under planning. Final packet/source custody and removed build directories are recorded in `planning/final-custody.json` after all producers exit. Local copies are not an off-machine backup. Controller commits the two file groups in FILES.md; neither a snapshot nor a green measurement is adoption authority.
