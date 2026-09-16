# Slice 3 custody manifest — pending final G2 hash

- Predecessor: `f1df12e5cf6b1fc113ed6549967586b12e71651f`, tree `f74d6c8e11eb0a771a233af972ace30cff33e0cf`; production-equivalent to reviewed `7fc89c98`.
- Production candidate committed at `fcb497327d941b1ec76f6a291b78fffdca84f885`; CPG cache `96`, navigation cache `53`.
- Test-only reclassification committed at `34db2e3536a67533779672bcf1b879403609e97b`; old `[a,c]` expectation now `[a,b,c]`, with same-environment receipt `../slice3/base-default-control/RECEIPT.md`.
- Pending: Astra-owned final `src/cpg/optional_parameter_tests.rs` hash, final test-only checkpoint, round 2, and final Rust gates. Do not treat this as a frozen release manifest.

## Evidence

- Initial full-default one-failure historical log: `.../implementation/logs/full-default-final.log`; it is superseded by the controlled reclassification, not final green evidence.
- G1 exact candidate: `.../implementation/logs/g1-exact-candidate.log`; 18-row base/candidate custody: `.../slice3/g1-rows/RECEIPT.md`; refusal inventory: `.../slice3/g1-rows/REFUSAL-INVENTORY.md`.
- G3 incremental caller/callee parity: `.../implementation/logs/round1-incremental-control.log`.
- G4 runtime semantic control: `.../implementation/logs/round1-runtime-control.log`.
- P15 genuine cache and v95→v96 evidence: `.../slice3/final-gates/nonrust/p15-receipt.md`.
- Non-Rust receipt: `.../slice3/final-gates/nonrust/nonrust-gates-receipt.md`, SHA-256 `fa8eff36af18302779ceff849db6c88b4ef114cd9b9dd86503630820350e1eb3`. Node/authority/Python/matrix pass; quick timed out at the authorized 1200 seconds with no report and mutable corpus, so it is incomplete and makes no accuracy or SUT-error claim.

## Limits

Value evidence is synthetic-only. No fixed-source census is claimed. Full Tier-A corpus, live adoption, rebaseline, publication, and merge are excluded.

## Final source binding before Rust gates

- `src/parameter_slots.rs` `11020db81866c0e7b9ce55bf16052d7f11fa933788f92de5bcbed0347455fb4a`
- `src/cpg_cache.rs` `c8ac2991a358ace0ef303d7a060b8282663a1c3ae99466ba95124902ceb69559`
- `src/ast_required_parameter_tests.rs` `797bd9ac001ca5ff3c938f2de86ae6449b72286488496eae751bd1d5a9e18dcd`
- `src/ast_inert_default_parameter_tests.rs` `d931e441abfdfea100774a99afe7fc6b0216157e04194d2e772278033e682d4c`
- `src/cpg/optional_parameter_tests.rs` `6454d29d1bc700603d60779225835cb40568baffbaffb5471d9ec9c132a86e50`

G2 focused candidate is 26/0; pristine base is 18 pass / 8 behavioral RED (`/private/tmp/prism-post316-orchestration/slice3/g2/`). Rust final gates begin against these bytes.
