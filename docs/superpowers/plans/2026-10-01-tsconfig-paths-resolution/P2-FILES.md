# Current P2 diagnostic inventory

MEASURED: checkout `proto/tsconfig-paths-p2` at `e80fbf541d53ace5c547d5f26c9135e46e7be12b`; no Git writes. The prototype and original plan are already committed by the controller. INHERITED: controller reports +32 certified JS-hop rows, so P2 is not material. Current work is diagnosis only; no production adoption/dispatch is authorized by this packet.

New packet files: `P2-GAP-DIAGNOSIS.md`; `p2-probes/CONTROLLER-p2-gap.sh`, `gap.cjs`, `gap-class-catalog.json`, `gap-source-binding.json`, `gap-driver.rs`, `gap-kernel.rs`, `build-gap-driver.py`, `gap-controls.py`, `gap-projection.cjs`, `gap-guard-controls.cjs`, `gap-kernel-controls.py`. Updated current-state docs: this file, `P2-MEASUREMENTS.md`, `OQ-paths-p2.md`, `SPEC.md`, `IMPLEMENTOR.md`, `HANDOFF-P2.md`. Root `VERIFICATION.md` is an ignored local receipt, retained in the snapshot; do not lose it when transferring the packet. The controller command and local snapshot/verification receipts are in P2-GAP-DIAGNOSIS and HANDOFF-P2. Controller owns committing this exact diagnostic set; no push/merge claim.

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
