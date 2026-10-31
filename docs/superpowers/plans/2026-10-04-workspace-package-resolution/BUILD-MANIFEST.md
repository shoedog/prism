# R1 build and custody binding

Measured clone `/Users/wesleyjinks/code/prism-pkgres`, branch proto/workspace-package-resolution. Committed source base `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318` plus R1 source patch; separate docs base `c89bc5b74df05e1f8746792c0cfad2805336b0f9`; main comparison `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Runtime cache epochs **108/64**. No Git writes.

BUILD-MANIFEST.json is the controller authority for immutable binaries and probe hashes. It rehashes 547 production/vendor/script/build inputs and 447 test files, plus the mutant registry, all packet probes and the controller script. The final census was captured after the final full suite and frozen across custody rebuilds; its timing is explicit rather than represented as a pre-test census. Final source gates are listed in VERIFICATION.

| Role | Immutable path | SHA256 |
|---|---|---|
| base | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/base-prism` | `c5aea1b30cd3534972bcaa6d5d111d5dba8c24c22c97997e163d45a2cfdfb979` |
| basefacts | `/Users/wesleyjinks/prism-evidence/pkgres/planning/bin/base-facts` | `c722fd8e8759e9ac7e34cefae323a70f774c54e848d8704d3301090759870147` |
| head | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/bin/r1-prism` | `afa9c1bca61297a2ac953c7ec85bf0b5895a4db347b3b03fd53a396be31a0701` |
| headfacts | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/bin/r1-facts` | `f5006383ed6c47ef36152086bf37de7a6b5f731cdb9d26eac2b30c63e81b7dd0` |
| headresolution | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/bin/r1-resolution` | `14555a8aa9f4b46ab06238fb32029d8d0b0899708552be36823be95dd5ec3da3` |

TS oracle **5.9.3**: `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`, SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. Toolchain: Rust 1.94.0, clippy 0.1.94, nextest 0.9.146, Node 26.0.0, Python 3.9.6. All builds offline; no installs/fetch/update.

Original main binding and auxiliary script supplement remain under planning; their retrospective timing is unchanged. R1 helper executables were compiled from probes/dump-facts.rs and dump-resolution.rs as temporary Cargo examples and copied into immutable evidence roles, then temporary examples removed. The corrected root-typesVersions source was rebuilt before final differential/public/cache/S1b measurements (build-r1-final.log). Measured r1-prism is byte-identical to the retained compiled MCP dependency artifact prism-3ba4b1893b0b8e1d. A subsequent same-source Cargo rebuild selects a different top-level artifact after mixed feature/transitive dependency builds; this investigation does not prove why Cargo selects it. Its bytes were never substituted into measurements. Tier-A used its immediate separately rebuilt same-source CLI. final-source-binding.json records both hashes and unchanged inputs; do not claim Tier-A binary identity with the measured MCP CLI.

Reproduction from an applied R1 source tree; temporary examples must be removed before fmt/clippy:

```bash
CARGO_INCREMENTAL=0 cargo build --offline --release --features mcp --bins
cp docs/superpowers/plans/2026-10-04-workspace-package-resolution/probes/dump-facts.rs examples/pkgres_r1_facts.rs
cp docs/superpowers/plans/2026-10-04-workspace-package-resolution/probes/dump-resolution.rs examples/pkgres_r1_resolution.rs
CARGO_INCREMENTAL=0 cargo build --offline --release --features mcp --example pkgres_r1_facts --example pkgres_r1_resolution
# Retain Cargo's compiled executables in a new evidence directory and bind their hashes.
rm examples/pkgres_r1_facts.rs examples/pkgres_r1_resolution.rs
```

Keep main base/basefacts roles distinct from the committed-prototype repair base. CONTROLLER-pkg.sh requires current base/head/basefacts/headfacts hashes plus census/compare probe hashes. Before private F, the controller must rebind source and oracle inputs too. This worker did not execute F.

The old eight-role prototype/S2 manifest is archived in repair-r1/historical-packet, with the earlier operational documents. S2 parent/overlay binaries and 0/132 receipts under planning are historical only; no R1 S2 rerun or source adoption. CENSUS, PROBES and PROBE-LOG are explicitly marked historical where retained.

R1-src.patch applies to 92c1d0bc; R1-docs.patch applies to c89bc5b7. final-custody.json records clean patch application, exact reconstructed owned bytes and final local source/packet snapshots. These copies do not claim off-machine backup. Controller owns Git custody and the two FILES commit groups; no green receipt is independent acceptance or S2 adoption.
