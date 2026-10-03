# Current P2 spec round-1 file inventory — prepared, awaiting branch custody

[MEASURED] This clone remains plan/tsconfig-paths-p2 @bb4e4743 with a clean tracked tree. Four prototype candidates and ten plan candidates are retained under target/p2-spec-r1; none have been applied to tracked paths. The controller must switch to proto b9fd3775 for prototype edits, then plan bb4e4743 for the plan fold. No worker Git writes or F reads. All earlier inventories below are historical.

Suggested controller commits:

- Prototype: `fix(paths): preserve unresolved non-relative forward claims in legacy stars`
- Plan: `docs(paths): fold Opus P2 spec review and refresh cumulative dispatch`

Prototype set, incremental from reviewed b9fd3775:

- `src/js_exports.rs`
- `src/cpg_cache.rs` (comment only)
- `tests/integration/js_paths_p2_test.rs`
- `tests/integration/fixtures/js_paths_p2_legacy_refusal.json` (new frozen complete main rows)

Plan set, under this packet directory, incremental from bb4e4743:

- `IMPLEMENTOR.md`, `HANDOFF-P2.md`, `SPEC.md`, `P2-MEASUREMENTS.md`, `OQ-paths-p2.md`, `P2-FILES.md`.
- `p2-probes/CONTROLLER-p2.sh`.
- New `p2-probes/legacy-forward-controls.py`, `p2-probes/nonrelative-cache.py`, `p2-probes/spec-r1-mutants.py`.

Fresh checks completed so far: exact-revision304/304 production input hash binding for main and cumulative prototype, eight legacy RED fixtures/16 sites against both binaries in the same environment, two H1 reference fixtures/10 real CPG cache states, controller-wrapper shell syntax. Repaired Rust suite, scoped mutants, fmt/clippy, matrix, S1b-4, complete public streams and controller repaired F parity are pending. Expected F repair impact0 is not measured.

Custody: prepared-owned-snapshot.tar.gz, prepared-source-binding.json, prepared-proto.patch, prepared-plan.patch, INTERIM-HANDOFF.md and hypothesis-probe-result.md in target/p2-spec-r1. Starting implementation remains the final cumulative prototype the controller squashes onto main c50de85a, not a reapplication of the old e80fbf54 increment.

---

# Historical P2 non-relative file inventory

[MEASURED] No Git writes. The physical checkout is plan/tsconfig-paths-p2 @35c481cf. The exact proto parent e80fbf54 is exported to `target/p2-nonrelative/work`; apply its incremental patch only after the controller switches to that prototype branch. The plan files below are edited in the physical plan checkout. No private F input/evidence was opened.

Suggested controller commits:

- Prototype: `feat(paths): prove non-relative export hops with caller project options`
- Plan: `docs(paths): bind P2 non-relative yield, controls and controller audit`

## Prototype set (relative to gitless work / e80fbf54)

- `src/js_paths.rs`, `src/call_graph.rs`, `src/resolution.rs`, `src/repo_loader.rs`, `src/ast/js_module_forwarding.rs`.
- `tests/integration/js_paths_p2_test.rs`, `tests/integration/js_binding_export_state_test.rs` (authorized inert-forward-fact expectation).
- All26 files under `eval/fixtures/{javascript,typescript}/tsconfig_paths_nonrelative_hop{,_refusal}/`.

Measured delta: source5 files **93 added /28 removed** (includes the new loader test), integration tests2 files **212 added /4 removed**, fixtures26 files **78 added**. Full33-path inventory: `target/p2-nonrelative/proto-files.json`. Forecast80–140 source /200–280 tests /70–100 fixture LOC, plus bounded probe runners. Cache106/62 already exists on e80fbf54; no new cache edit.

## Plan set (physical plan checkout / 35c481cf)

- `P2-MEASUREMENTS.md`, `SPEC.md`, `IMPLEMENTOR.md`, `P2-FILES.md`, `HANDOFF-P2.md`, `OQ-paths-p2.md`, `P2-GAP-DIAGNOSIS.md`.
- `p2-probes/CONTROLLER-p2.sh`, `p2-probes/s1b.py`.
- New `p2-probes/hop-audit.cjs`, `hop-audit-controls.py`, `nonrelative-controls.py`, `nonrelative-public.py`, `nonrelative-rebuild-parity.py`, `nonrelative-mutants.py`.

The original gap diagnostic source binding remains historical/e80fbf54 and must not be pointed at new source without rebuilding/rebinding it. The new standalone hop audit consumes its retained private row partition plus live native inputs; it never assumes old boolean projection tables match the new caller-config tables. Root VERIFICATION.md is local/ignored; retained in the owned snapshot.

Local snapshot/patch hashes and final commands are in HANDOFF-P2. Controller owns branch switching, committing and external custody; no push/merge requested.

---

# Historical P2 file inventory and controller commits (completed)

MEASURED: Entry HEAD8bd3c2dad641bf209f82073017b1549a8f377dff; no planner Git writes. Prototype is in this clone's working tree. Controller commits the **plan** set to the plan branch and the **prototype** set to proto/tsconfig-paths-p2 after checking the frozen owned-file hashes. Do not apply the prototype twice or rewrite it. Local source/binary evidence is target/p2-plan; actual F yield and independent review remain open.

Suggested controller commit messages:

- Plan: `docs(paths): specify and measure P2 relative JS export hops`
- Prototype: `feat(paths): prove local Node10 absence for JS export hops`

## Plan files

```text
VERIFICATION.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF-P2.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/IMPLEMENTOR.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/OQ-paths-p2.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/P2-FILES.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/P2-MEASUREMENTS.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/CONTROLLER-p2.sh
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/cache.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/compare.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/controls.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/mutants.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/public_head.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/s1b.py
docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/verify.py
```

## Prototype files

```text
eval/fixtures/javascript/tsconfig_paths_js_export_hop/app.jsx
eval/fixtures/javascript/tsconfig_paths_js_export_hop/expected.toml
eval/fixtures/javascript/tsconfig_paths_js_export_hop/src/barrel.ts
eval/fixtures/javascript/tsconfig_paths_js_export_hop/src/middle.js
eval/fixtures/javascript/tsconfig_paths_js_export_hop/src/real.jsx
eval/fixtures/javascript/tsconfig_paths_js_export_hop/tsconfig.json
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/app.jsx
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/expected.toml
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/src/barrel.ts
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/src/middle.js
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/src/real.d.ts
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/src/real.jsx
eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal/tsconfig.json
eval/fixtures/typescript/tsconfig_paths_js_export_hop/app.tsx
eval/fixtures/typescript/tsconfig_paths_js_export_hop/expected.toml
eval/fixtures/typescript/tsconfig_paths_js_export_hop/src/barrel.ts
eval/fixtures/typescript/tsconfig_paths_js_export_hop/src/middle.js
eval/fixtures/typescript/tsconfig_paths_js_export_hop/src/real.jsx
eval/fixtures/typescript/tsconfig_paths_js_export_hop/tsconfig.json
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/app.tsx
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/expected.toml
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/src/barrel.ts
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/src/middle.js
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/src/real.d.ts
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/src/real.jsx
eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal/tsconfig.json
src/cpg_cache.rs
src/js_paths.rs
src/js_paths_first_pass.rs
src/navigation/call_edge_cache.rs
tests/integration/js_paths_cap_test.rs
tests/integration/js_paths_common.rs
tests/integration/js_paths_p2_test.rs
tests/integration/js_paths_repair_test.rs
tests/integration/main.rs
```

## Private measurement command

With controller-held CORPUS_F_ROOT, a new PRIVATE_EVIDENCE_ROOT and TS_JS already set:

```bash
bash docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/CONTROLLER-p2.sh \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r1/base/dump_imports \
  /Users/wesleyjinks/code/prism-paths-p2-plan/target/p2-plan/bin/prism-p2
```

Return aggregate stdout only. P2 SHA2563310ec8621dc82d4a93bd1b51f1697d0e3bde925a84f1553e4f523ba1f344ca7. Every changed row must be certified; unsupported/uncertified output rejects acceptance. Raw private evidence stays private. The optional third argument retains P2-0 census-only compatibility.

Custody: p2-build-binding.json covers304 production inputs and the binary; p2-file-split.json enumerates the two sets; p2-owned-snapshot.tar.gz and final source hashes retain local bytes. No external-backup or controller-commit claim is made. Rust build caches are reproducible intermediates, not evidence; only lane-owned completed intermediates may be removed. Preserve the immutable P2 binary, compressed complete public streams, native/suite/cache/mutant receipts and owned snapshot.
