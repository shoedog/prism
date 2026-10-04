# Measurements — final bounded prototype

Base main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, head dirty prototype pinned in BUILD-MANIFEST.json. Census was written before design and preserved separately in CENSUS.md and planning/census-checkpoint.tar.gz. Oracle actual TS 5.9.3 writer ProjectService, not a guessed nearest config. No private F access.

## Public yield and preservation

| Corpus | Call sites | Main sites with a target | Changed rows | CORRECT_STATIC_BINDING | Unproven | Lost main edges |
|---|---:|---:|---:|---:|---:|---:|
| X | 19219 | 10776 | 0 | 0 | 0 | 0 |
| installed-X | 19219 | 10776 | 0 | 0 | 0 | 0 |
| R | 953 | 216 | 0 | 0 | 0 | 0 |
| T | 61712 | 27452 | 0 | 0 | 0 | 0 |

All four complete output streams, including non-call records, are byte-identical to main. No keys added/removed; no base module proof removed or changed. Installed-X adds exactly one correct module proof: `examples/with-nextjs/src/excalidrawWrapper.tsx` + `@excalidraw/excalidraw` → `packages/excalidraw/index.tsx`, owner `examples/with-nextjs/tsconfig.json`. Canonical native module and owner agree. This does not by itself produce a callable Exact target: the actual public Excalidraw export is React.memo with an identifier first argument, outside the existing wrapper terminal guard. No guard was relaxed.

X's flagged Node10 Next.js and bundler utils imports are native-unresolved; installed-X bundler utils also remains native-unresolved. Generated-output substitution is OQ-1. R's 15 workspace pairs are native-unresolved. The census's main non-Exact potential direct-site counts (1/1/33/0) therefore reduce to native indexed potential 0/1/0/0, and to actual direct Exact yield 0/0/0/0.

Receipts: `planning/public-v2/summary.json`, per-corpus `comparison.json`, `public-v2-preservation.json`. The final refresh reused complete retained clean-main streams, rehashed every original census native/source read (T: 770), then freshly generated final head rows/facts and native comparison. Original fresh-base and earlier refresh receipts are retained; reuse is explicit in each `base-reuse.json`. Measurements are two snapshots of X, not independent additive corpora.

## Native package controls and regression controls

58 retained controls (29 scenarios × TSX/JSX): **42 changed rows, all CORRECT_STATIC_BINDING**, zero lost/unproven; 16 unchanged/refused. This includes modern self-reference without installation, legacy non-self-reference, Node16/Next ESM writer scopes, custom-condition match/nonmatch, CommonJS/preserve bundler emit-mode witnesses, source-order conditions, literal/pattern exports, Node10 legacy entries, missing condition fallthrough, declaration winners, uninstalled discovery and deliberately unimplemented JS/default/unsupported-mode refusals. Every control checks the native module target even if the call row remains unchanged, and each changed row certifies owner plus complete terminal file/name/start/end span. Inputs and source bytes are rehashed after native use.

8 package tests pass. Seven meaningful existing-path tests fail behaviorally on the same-environment clean base (0/7), with the new API-only classifier test excluded from that RED attribution. A new pre-repair mode witness fails with wrong Exact other.ts while native chooses index.ts; the same witness and full final suite pass after repair. No compile/setup failure is counted as RED evidence.

411 S1b scenarios unchanged; all 1,234 generated output files (full dumps/functions/summary/stderr) byte-identical. Existing lane-P public rows are also preserved by the four-corpus full-stream comparisons. Mutgate is advisory and scoped: 2/2 selected killed, 6/8 registry entries outside Git-diff selection. VERIFICATION retains the final complete-suite totals and requested nextest source limitation.

## S2 scratch unblock

Scratch parent `61641bdb64c911049c567efc51c29e17ef94ef4f`, rebuilt offline in this environment with the final PKG overlay and a measurement-only builtin classification branch. No S2 source adopted. Both snapshots contain 19,219 sites and 10,776 main sites with a target.

| Snapshot | S2 R3b against main | S2 + final PKG against main | +132 recovered | Lost/unproven |
|---|---:|---:|---:|---:|
| X | 0 changed | 0 changed | **0** | 0/0 |
| installed-X | 0 changed | 0 changed | **0** | 0/0 |

All four S2 complete streams are byte-identical to main. PKG module proof + builtin absence alone does not unblock S2; it remains parked.

| Residual category | Unique joins X / installed-X after PKG | Risk-bearing joins X / installed-X | Referenced gain rows (overlapping) |
|---|---:|---:|---:|
| Bare/paths aliases | 619 / 619 | 0 / 0 | 0 |
| Relative | 408 / 408 | 5 / 5 | 3 |
| Census workspace bucket | 6 / 5 | 3 / 2 | 132 |
| Other scheme | 1 / 1 | 1 / 1 | 132 |
| node: builtin | 1 / 1 | 0 / 0 | 0 |

Categories use the **same main-outcome precedence as CENSUS**: pairs with a captured paths proof belong to bare/paths alias here, even when their spelling resembles a workspace package. Earlier exploratory prefix-group receipts (workspace 27/26, alias 598) are historical and not directly comparable to these final census-bucket counts. They do not represent different call-row yield. All trace join identities remain in residuals.json.

Before PKG, each snapshot has one risk-bearing node:url join referencing all 132 inherited gain rows. The measured builtin absence removes its identity risk. Installed-X removes the Next.js member Excalidraw join after the native package proof; its namespace join still references EditorLocalStorage (3). Browser utils MIME_TYPES and virtual:pwa-register/registerSW still reference all four gain classes (132). The relative joins are FontPicker/Sidebar/Stats imports of Excalidraw, Stats getCommonBounds, and MainMenu DefaultItems; all reference EditorLocalStorage (3). Category rows overlap and are never summed.

**SMELL — diagnostic attribution limit:** the parent emits traces during multiple graph builds without graph IDs. One enumerated Next.js namespace join varies from all four classes to EditorLocalStorage only. The final receipt retains both last observed emission and conservative union across every emission; category union risks equal the table above. These are descriptive risk references, not isolated causal recovery credit or a proven final-graph trace binding. Actual recovery is measured independently by complete changed-row comparison: zero.

Reference +132 and class weights EditorLocalStorage 3, API 52, Keyboard 34, UI 43 are **inherited from owner brief/R3b evidence**; the original pre-R3b gain candidate was not rebuilt. Final native ownership/call populations and R3b parent/overlay binaries are fresh measurements. Receipts: planning/s2-final/{s2-base,s2-pkg}/{X,installed-X}, summary.json and s2-pkg-v2-binding.json. Capture attempt 3 completed at its cap after two enumerated setup/trace-parser corrections, without source restart or a new refusal cut.

## Controller F

Do not execute in this planner session. Controller supplies private roots; stdout contains aggregates only. Immutable base/head and both facts helpers must match BUILD-MANIFEST.json. Unknown/wrong changed rows or lost edges stop with INADMISSIBLE; private details remain in the private output.

```bash
E=/Users/wesleyjinks/prism-evidence/pkgres/planning
CORPUS_F_ROOT="$PRIVATE_F_ROOT" PRIVATE_EVIDENCE_ROOT="$NEW_PRIVATE_OUTPUT" \
  bash docs/superpowers/plans/2026-10-04-workspace-package-resolution/CONTROLLER-pkg.sh \
  "$E/bin/base-prism" "$E/bin/head-v2-prism" \
  "$E/bin/base-facts" "$E/bin/head-v2-facts" \
  /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js
```

Underlying native checker exercised on public controls; controller syntax checked. F itself, independent acceptance and any new model/cost decision remain unverified. See OQ and VERIFICATION. No refusal cut or S2 adoption.
