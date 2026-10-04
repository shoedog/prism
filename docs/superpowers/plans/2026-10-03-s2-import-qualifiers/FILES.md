# Controller commit set — S2 owner fail-closed repair

No Git writes were performed. These exact prototype/packet paths are for controller custody on the planning branch. Adoption still needs F, authoritative mutation coverage, Tier-A quick and two serial independent review rounds. Planning HEAD is `6e4e0ef19de578d4865d7297eb5cf72a5b6a27a6`; immutable product base is `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`.

## Src/tests and validation

Suggested commit: `feat(resolution): add fail-closed JS/TS import qualifier proofs`

The source keeps the complete populated base result, adds positional member/identity/module/span proof on drops, and revokes qualifiers using the closed cross-file lexical whitelist. Cache versions are 108/64. Controls include both grammars, named and dynamic namespace aliases, escapes, member writes, all supported implicit receiver carriers and positive whitelist uses. Registry S2-14 removes the whitelist; four Tier-A fixture directories cover positive/refused cases.

- `eval/fixtures/javascript/s2_import_qualifiers/app.jsx`
- `eval/fixtures/javascript/s2_import_qualifiers/expected.toml`
- `eval/fixtures/javascript/s2_import_qualifiers/lib.jsx`
- `eval/fixtures/javascript/s2_import_qualifiers/tsconfig.json`
- `eval/fixtures/javascript/s2_qualifier_escape/app.jsx`
- `eval/fixtures/javascript/s2_qualifier_escape/expected.toml`
- `eval/fixtures/javascript/s2_qualifier_escape/lib.jsx`
- `eval/fixtures/javascript/s2_qualifier_escape/tsconfig.json`
- `eval/fixtures/typescript/s2_import_qualifiers/app.tsx`
- `eval/fixtures/typescript/s2_import_qualifiers/expected.toml`
- `eval/fixtures/typescript/s2_import_qualifiers/lib.tsx`
- `eval/fixtures/typescript/s2_import_qualifiers/tsconfig.json`
- `eval/fixtures/typescript/s2_qualifier_escape/app.tsx`
- `eval/fixtures/typescript/s2_qualifier_escape/expected.toml`
- `eval/fixtures/typescript/s2_qualifier_escape/lib.tsx`
- `eval/fixtures/typescript/s2_qualifier_escape/tsconfig.json`
- `mutants/lane-s2-import-qualifiers.json`
- `src/ast.rs`
- `src/ast/js_import_qualifiers.rs`
- `src/call_graph.rs`
- `src/cpg_cache.rs`
- `src/js_exports.rs`
- `src/js_import_qualifiers.rs`
- `src/js_paths.rs`
- `src/lib.rs`
- `src/navigation/call_edge_cache.rs`
- `src/resolution.rs`
- `src/resolution_js_namespace.rs`
- `tests/integration/js_import_qualifiers_test.rs`
- `tests/integration/main.rs`

## Docs and reproducible probes

Suggested commit: `docs(plan): finalize S2 fail-closed design, yield and controller replay`

SPEC §0 records S2-O6 and controller interim positions; IMPLEMENTOR preserves the existing artifact and two-round review boundary. CONTROLLER-s2.sh takes base/head/head-facts/TypeScript and verifies changed-row correctness on F. Measurements report +132/+132/0/0 and the exhaustive 179-row refusal partition. Already-committed packet support files are listed for custody even when unchanged relative to HEAD.

- `VERIFICATION.md` (local excluded receipt; snapshot only)
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/CONTROLLER-s2.sh`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/FILES.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/HANDOFF.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/IMPLEMENTOR.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/MEASUREMENTS.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/OQ-s2.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/PROBES.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/README.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/SPEC.md`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/build-facts.py`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/census.cjs`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/compare-head.py`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/controls.py`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/dump_imports.rs`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/e5-alias-boundary.py`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/prototype-controls.py`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/reference-binaries.json`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/s1b-parity.py`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/synthetic-controls.json`
- `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/yield-refusals.cjs`

Metadata: `BUILD-MANIFEST.json`, `OWNED-FILES.json` in this directory. The inventory excludes these self-referential bytes; final-custody.json hashes the source archive including both. No Cargo manifest/lock, vendor source or existing Tier-A baseline changes.

Keep `target/s2-plan/bin/`, original S2-0 evidence/source snapshots, `s2-owner-failclosed-source.tar.gz`, `s2-owner-failclosed-evidence.tar.gz` and `owner-final-custody.json`. After binding snapshots, compress retained new JSON/JSONL streams and remove only generated local Cargo targets/control source trees. The cleanup receipt identifies literal paths. Rebind rebuilt post-commit binary hashes and replay complete public streams before transferring certificates; a commit is not F evidence, review, publication or merge.

`VERIFICATION.md` is excluded by this clone's `.git/info/exclude`; retain that controller-local exclusion. MEASUREMENTS/BUILD-MANIFEST contain its durable packet claims, and the source snapshot retains its bytes.
