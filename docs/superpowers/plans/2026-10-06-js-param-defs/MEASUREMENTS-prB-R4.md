**R5b supersession (2026-10-06):** Original R5 questions are resolved by prB-repair-r5b-brief.md. Current candidate is STOP on35 actual non-formal outside-certificate rows; read MEASUREMENTS-prB-R5.md/HANDOFF-repair-prB-r5.md. Historical receipts remain unchanged below.

# R4 retained repairs and design STOP — loop 1 of cap 3

**STOP, not acceptance.** Six DESIGN-CHANGE cells remain. The final frozen head omits **18 ES-proven main-correct rows** in the named-function invoked-default controls. The required LOST CORRECT=0 guarantee is false in the synthetic diagnostic population; no corpus count is inferred from it. ADDED WRONG=0 has not been established. The packet is ready for Fable; no delivery route is available, so it has not been sent.

Entry branch `plan/js-param-defs-prB`, docs HEAD `a3dda01097ef010f39f60e6e65831f90e0e9af7f`. Entry source/test/mutant census:724/724 exactly equal source base `1af4301f`. Main product reference: `da0604b3`. The supplied frozen main byte dumper was freshly run on these synthetic fixtures in this host; its source provenance is inherited rather than rebuilt in R4. No git writes, corpus execution, installs or frontend-portal access occurred. No independent model-metadata or review claim is made.

## Matrix and scope

763 semantic cells / 2289 JS/TS/TSX language cells: **{'OK': 264, 'OUT-OF-MODEL': 373, 'FIXED-here': 120, 'DESIGN-CHANGE': 6}**. Each cell retains source, explicit supply combinations, Node reads/results/errors, binding description, complete main/head rows and labels. All 384 OK/FIXED cells have executable graph seam assertions in `classify_matrix.py`; `validate_node.py` asserts exact result/read values for all763 semantic cells. Rust tests additionally bind the changed paths to source-bound RED/GREEN and negative controls.

Every OUT-OF-MODEL and DESIGN-CHANGE ID is individually listed in `MATRIX-exceptions.md` (379 IDs), with full per-language rows in `MATRIX-param-env.md` and `matrix-cell-records.jsonl`. OUT-OF-MODEL families: {'destructured parameters': 334, 'legacy arguments object': 20, 'direct eval': 12, 'anonymous generator callbacks': 6, 'Legacy duplicate-formal ordinal/value-flow precision; JS/TS admission differs, with main rows retained per language.': 1}. Main-row parity is101 PASS /272 FAIL; the failures are unresolved violations of the requested fail-closed policy, not waived exclusions. Anonymous generators remain byte-identical (six semantic cells); duplicate-simple formal precision is disclosed with per-language main parity.

The enumeration covers8 parameter forms ×11 declaration forms ×2 owners ×3 strictness modes, plus expression effects,12 callable kinds, early-error and lexical-capture controls. It derives the binding/copy seam from ES2025 FDI, IteratorBindingInitialization, early errors and AnnexB. The ordinary rest/destructuring forms without expressions share the parameter binding; defaults/computed expressions create the body environment. AnnexB block functions sharing a formal name do not introduce or overwrite its binding. Arrow defaults capture arguments/this/new.target; the synthetic oracle retains those exact observations.

**Exhaustiveness is not discharged:** this is a finite grammar/effect partition, not an exhaustive certificate for arbitrary initializer calls, exceptions, property effects or legacy value-flow precision. The discovered callable-side-effect design class remains open. The brief's complete acceptance target is therefore not achieved.

## Bounded fixes retained

- Ordered default transfer preserves formal and earlier sources on each undefined-argument skip arm, replacing syntactically-last selection.
- Writes within one selected default compose in execution order: the last must-write replaces intermediates; inner may-writes join; the entire default's skip arm restores incoming sources. Nested assignments order by write completion, not lvalue start bytes.
- Literal logical/ternary branches exclude never-run writes; unknown inner conditions retain both normal-completion alternatives. Nested closures are excluded from this syntactic own-scope transfer; their invocation is the design STOP.
- Completed same-line RHS writes dominate later reads, covering `var f=(f=2,f)` without retaining the overwritten entry value.
- Semantic early-error refusal covers body lexical/class collisions (including decoded escapes and patterns), non-simple body use-strict, strict arguments/eval formals, duplicate BoundNames in restricted function kinds, and the enumerated catch-pattern var conflict. Comments do not terminate directive prologues or change parameter simplicity; property keys and initializer reads are not BoundNames.
- S1's exact sentence is in CLAUDE.md and SPEC-prB §0. Copy rows use the ordinary RD grade; the changed R3 label assertions and obsolete default-always-executes expectation are reconciled.

Only6 source files differ from1af4301f: `src/ast_callback_identity.rs`, `src/data_flow.rs`, `src/cpg/reaching.rs`, `src/cpg.rs`, `src/cpg/callback_identity_tests.rs`, `src/cpg/js_param_defs_tests.rs`. Cache111 and old mutation anchors are retained; their coupling has not been completed and cannot support adoption.

## WRONG R4-DESIGN-1 — default invocation loses a captured write

```javascript
function h(f,d=(()=>{f=5;})()) {
 var f;
 use(f);
 return f;
}
```

Node observes5 when the default runs and1 when d is supplied. An uncalled-closure control observes1. The main nested-write source reaches both body reads; head omits it. Sloppy/strict/module × JS/TS/TSX ×2 body reads gives18 complete lost rows. `default-invocation-losses.json` retains exact bytes, paths, owners, labels and oracle runs. Confidence100/100 for this bounded population; false endpoint custody or Node observations would collapse it. Frequency beyond this population is unknown. This is a retained own-scope transfer limitation, not an attribution of all PR-B changes to R4.

Six DESIGN IDs: E-invoked-default-write-named-sloppy, E-invoked-default-write-named-strict, E-invoked-default-write-named-module, E-invoked-default-write-callback-sloppy, E-invoked-default-write-callback-strict, E-invoked-default-write-callback-module. A principled repair must distinguish invocation from closure creation, propagate captured side effects before entry copy, and decide how B-D2 own-scope/kill-only sources and B-D5 invocation isolation change. No ad hoc IIFE special case or design-changing repair was made.

## Tables

| Population | ADDED | LOST | RELABELLED | LOST CORRECT | ADDED WRONG |
|---|---:|---:|---:|---:|---:|
| X | NOT RUN | NOT RUN | NOT RUN | UNKNOWN | UNKNOWN |
| Xi | NOT RUN | NOT RUN | NOT RUN | UNKNOWN | UNKNOWN |
| T | NOT RUN | NOT RUN | NOT RUN | UNKNOWN | UNKNOWN |
| SecBench | NOT RUN | NOT RUN | NOT RUN | UNKNOWN | UNKNOWN |
| Synthetic763, raw identity diff | 5133 | 2398 | 1053 | ≥18 proven diagnostic rows | NOT ESTABLISHED |

Raw synthetic rowdiff is not adjudication and includes inherited wrong/token/alias rows. Earlier partial TS adjudications under `initial-*` and `matrix.raw-ts.*` are historical checkpoints; TS is blind at the parameter-env seam and their CORRECT counts are not accepted as ES truth. No current X/Xi/T/SecBench table, non-JS/callsite identity, nav-removal proof, O1 SecBench sweep or performance acceptance was measured after this STOP. No earlier green result is transferred.

## Gates and attribution controls

| Check | Fresh result | Evidence |
|---|---|---|
| Callback suite |55 passed /0 failed |green-bound-names.log |
| Full nextest, final source |Summary [ 204.010s] 5229 tests run: 5229 passed (3 slow), 1 skipped |nextest-stopped-final.log |
| Doctests |2 passed /0 failed |doctests-final.log |
| fmt |PASS |fmt-final.log |
| Release production + byte helper |PASS, offline/locked |build-stopped-final.log, build-bytes-final.log |
| Exact Node predictions |763 passed /0 failed |node-regressions.json/log |
| Graph matrix seam assertions |384 PASS /0 FAIL /379 excluded |matrix-regressions.json |
| Out-of-model fail-closed parity |101 PASS /272 FAIL |matrix-cell-records.jsonl |
| Mutants, clippy235/235, Tier-A matrix, TS/Node quick |NOT RUN — design/zero-loss STOP |Uncompleted gates, no transferred credit |

Earlier R4 full suite checkpoints:5227/5227 passed with1 skipped; a later5228 run had5226 pass/2 transport failures/1 skipped under concurrent release compile; the following5228 full run passed5228/5228 with1 skipped. Both failed transport tests passed2/2 on the entry1af source in this same host (`transport-entry-control.log`). Transport/lazy source bytes were unchanged. The mocked readiness timeout/load explanation remains a hypothesis: the isolated control and later quieter run do not reproduce the original contended-load condition, so they do not prove a cause or justify blaming the parameter repair. The original failures are retained, not re-baselined or silently fixed. The subsequent5229 run had5228PASS/1FAIL/1SKIP because refused_shapes_gain_no_def still expected a plain formal Def from the SyntaxError function twice(m,...m). The entry1af focused control passed1/1; Node rejects the exact duplicate-rest input. That in-scope obsolete assertion was explicitly updated to refuse the entire callable; nextest-before-rest-expectation.log, bound-names-entry-control.log and A-duplicate-rest retain the control and correction.

The one skipped integration check is `SliceElem` reserved-path classifier coverage at `tests/integration/resolution_test.rs:575`, ignored until a future slice. Independent review and controller F verification are not run.

## Binaries and custody-only commits

Frozen production: `bin/prism-head-r4`, sha256 `6da219e9ce4d29f7033fdca3cb71fb6ac3ae2acede2b01fed02b3154ed4f1789`.
Frozen byte probe: `bin/prism-head-r4-bytes`, sha256 `f469d11271a2f3d7208b14f5be9f02fdb88fde615f16f038508bf9ddcfd6a2fe`.
Main byte probe: `prB/bin/prism-base-da0604b3-bytes`, sha256 `184812b37f3f1d536d8a71b4cb49b0a46b22479d3c664452a6cdff0882fb6d83` (fresh execution, inherited build provenance).

`R4-src.patch` is relative to1af4301f; proposed controller commit: **wip(js-param-defs): retain R4 parameter-environment repairs and design STOP**.
`R4-docs.patch` is relative toa3dda010; proposed controller commit: **docs(js-param-defs): record R4 matrix and Fable escalation**.
Source patch construction compares baseline blobs against live bytes, so the two untracked files already present in1af are not falsely represented as deletions by ordinary git diff. Snapshots/manifests and `final-custody-check.json` preserve the stopped artifact. The controller owns Git commits; neither message grants shipping/adoption authority.
