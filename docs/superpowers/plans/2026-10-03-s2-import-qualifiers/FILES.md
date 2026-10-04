# Controller commit set

No Git writes were performed. The prototype has open WRONG S2-W1 and must not be adopted; these are controller custody commits on the planning branch, not a production-ready feature recommendation. Suggested controller commits, preserving the existing artifact:

1. `fix(eval): count unjoinable S2 oracle sites and pin controller replay`
2. `wip(resolution): preserve S2 prototype pending qualifier write-identity proof`
3. `docs(plan): specify corrected S2 admission and dispatch evidence`

The first commit owns CONTROLLER-s2.sh and probes/census.cjs, controls.py, reference-binaries.json. Keep the base binary hashes unchanged; the updated manifest records the predecessor oracle hash and the corrected oracle hash.

The product prototype commit owns:

- New `src/ast/js_import_qualifiers.rs`, `src/js_import_qualifiers.rs` and `tests/integration/js_import_qualifiers_test.rs`.
- Wiring in `src/ast.rs`, `src/lib.rs`, `src/js_exports.rs`, `src/call_graph.rs`, `src/js_paths.rs`, `src/resolution.rs`, `src/resolution_js_namespace.rs` and `tests/integration/main.rs`.
- Cache version updates in `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs` (107/63).
- `mutants/lane-s2-import-qualifiers.json`.
- Both complete new fixture directories `eval/fixtures/{javascript,typescript}/s2_import_qualifiers/` (app, provider, tsconfig, expected.toml).

The plan/evidence commit owns root `VERIFICATION.md` and this packet's README, SPEC, MEASUREMENTS, IMPLEMENTOR, OQ-s2, PROBES, FILES, HANDOFF, BUILD-MANIFEST.json and OWNED-FILES.json. It also owns probes/build-facts.py, dump_imports.rs, compare-head.py, prototype-controls.py, synthetic-controls.json, s1b-parity.py and e5-alias-boundary.py. Existing public.py and supplied-reference-binaries.json remain unchanged. Cargo manifests, Cargo.lock, vendor source and existing Tier-A baselines remain unchanged.

OWNED-FILES.json is the exact relative-path/hash inventory of this continuation, excluding the self-referential manifest/inventory bytes. The two manifests and final source snapshot also receive archive hashes in `target/s2-plan/final-custody.json`.

Keep `target/s2-plan/bin/`, the original S2-0 `evidence.tar.gz`, `packet-snapshot.tar.gz`, `evidence-index.json`, and the new `s2-final-source.tar.gz`, `s2-final-evidence.tar.gz`, `final-custody.json`. Retained raw evidence is compressed after verification. Generated Cargo debug/release intermediates and duplicate exploratory streams may be removed only after the frozen tools, receipts and snapshot are secured; the final cleanup receipt identifies literal removed paths.

A commit changes CLI revision metadata. Rebind any rebuilt CLI and replay the complete public streams before transferring native certificates. Commit/adoption is not a private F result, an independent review, a push or a merge.
