# R2 controller application set and commit boundaries

Apply/cherry-pick c35719e1 onto the planning checkout, then apply both patches with patch -p1. R2-src.patch is relative to c357. R2-docs.patch compares planning packet files to b8f5b2f3 and the existing mutation registry to c357: exactly the composite tree after that cherry-pick. Source application is not adoption; rebuild all head tools from source, rebind manifest and replay after committing changes the embedded build identity. No worker Git writes.

| Suggested commit | Files / artifact |
|---|---|
| `fix(resolution): retain S2 static-binding guards under S2-O7` | R2-src.patch: src/ast/js_import_qualifiers.rs, src/call_graph.rs, src/cpg_cache.rs, src/js_exports.rs, src/js_import_qualifiers.rs, src/navigation/call_edge_cache.rs, tests/integration/js_import_qualifiers_test.rs. |
| `docs(plan): bind S2-O7 R2 yield, tests and controller replay` | R2-docs.patch: mutants/lane-s2-import-qualifiers.json; packet SPEC, OQ, IMPLEMENTOR, MEASUREMENTS, VERIFICATION, HANDOFF, README, FILES, PROBES, historical REPAIR-R1/R1b, new REPAIR-R2, BUILD-MANIFEST, OWNED-FILES. |

Eight repair-owned product paths are in product-paths.json;762 other initial inputs unchanged. Patches have dry-run/actual exact-byte application receipts. Head tools rebuilt/bound; immutable main untouched. Full source/evidence snapshots and final-custody.json are local custody, not commits. Root VERIFICATION.md stays an excluded local receipt, not in patches. Existing interrupted quick eval snapshot is preserved as an attempt artifact, not adopted baseline. No private F read, review dispatch or adoption/merge by worker.
