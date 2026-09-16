# Slice 3 final verification manifest

- Final checkpoint: `440c6f4e2dc0bc24521d67b172fa51226d007716`, tree `cce3d1e19192f8b7085ca1145fa4825b287ca0e4`.
- Production: `src/parameter_slots.rs` SHA-256 `11020db81866c0e7b9ce55bf16052d7f11fa933788f92de5bcbed0347455fb4a`; `src/cpg_cache.rs` `c8ac2991a358ace0ef303d7a060b8282663a1c3ae99466ba95124902ceb69559`; CPG96/nav53.
- Tests: AST `dc0b9305f8e80501223322a5d77b945765724dab8f5dae5266c6252d40b37cfe`; CPG `2de50ac75e6cee1dcd58efd334b61728495d745432066ac05fe7c2351dd0af69`.

## Evidence

- G1: `/private/tmp/prism-post316-orchestration/slice3/g1-rows/RECEIPT.md` and `REFUSAL-INVENTORY.md`.
- G2: `/private/tmp/prism-post316-orchestration/slice3/g2/candidate-focused.log`, `base-focused.log`, and `cap-supplement-focused.log`.
- G3/G4: `/private/tmp/prism-post316-orchestration/slice3/implementation/logs/round1-incremental-control.log` and `cap-supplement-ast.log`.
- Non-Rust: `/private/tmp/prism-post316-orchestration/slice3/final-gates/nonrust/nonrust-gates-receipt.md` SHA-256 `fa8eff36af18302779ceff849db6c88b4ef114cd9b9dd86503630820350e1eb3`; quick incomplete after 1200s, no report, mutable corpus.

Rust final logs are under `/private/tmp/prism-post316-orchestration/slice3/implementation/logs/`; default 4550/0/1, MCP 4743/0/1, widest 4766/0/1, examples 32/0/0, clippy exit 0, fmt/diff clean. No full Tier-A corpus, rebaseline, live adoption, publication, merge, or release authority.
