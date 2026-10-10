# R1 measurements and limits

Source base `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318`; clean-main comparison `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Final retained R1 binaries are in BUILD-MANIFEST.json. Native oracle is TS 5.9.3 with the writer's real ProjectService owner. No private F access.

## Public yield and preservation

| Corpus | Call sites | Main sites with a target | Changed call rows | CORRECT additions | Unproven | Lost main edges | New module proofs |
|---|---:|---:|---:|---:|---:|---:|---:|
| X | 19219 | 10776 | 0 | 0 | 0 | 0 | 0 |
| installed-X | 19219 | 10776 | 0 | 0 | 0 | 0 | 1 |
| R | 953 | 216 | 0 | 0 | 0 | 0 | 0 |
| T | 61712 | 27452 | 0 | 0 | 0 | 0 | 0 |

Every complete call output stream is byte-identical to main, with zero keys added or removed. No main module proof was removed or changed. Installed-X adds `examples/with-nextjs/src/excalidrawWrapper.tsx` + `@excalidraw/excalidraw` → `packages/excalidraw/index.tsx`, owner `examples/with-nextjs/tsconfig.json`. A fresh TS ProjectService certificate confirms canonical target and owner. This adds no Exact callable edge: the existing React.memo terminal guard stays in force.

Evidence root: `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1`. Final receipts are `public-r1-final/summary.json`, per-corpus comparison/base-reuse receipts and `public-r1-module-audit.json`. A fresh main/head public run completed earlier in this turn. The final refresh explicitly reused retained main streams after rehashing all original native/source inputs, then freshly generated final R1 streams/facts. The module audit checks indexed source populations/hashes against retained main facts and freshly verifies every added proof; original and refreshed controls are not counted as independent populations. X and installed-X are two snapshots of the same project, not additive independent corpora.

## Differential acceptance and S2-O9 gap

Final **5,096 cases: 2,242 Bound, 861 ProvenUnresolved, 1,993 Unsupported**. Zero wrong module bindings, false Exact edges, false native-absence claims or expected-native-target mismatches. **1,651 Unsupported cases have a TS binding**; these must fail closed in S2, never be excluded under S2-O9.

| Unsupported feature/reason where TS binds | Cases |
|---|---:|
| Unindexed writer extension (.mts/.cts) | 1098 |
| Declaration/unindexed/opaque winner | 181 |
| Export arrays | 181 |
| typesVersions | 181 |
| Paths authority barrier | 3 |
| Package imports (#) | 2 |
| JSX option barrier | 1 |
| Outside-root self-reference | 1 |
| JS secondary-priority pass | 1 |
| Directory-valued package field | 1 |
| Unsupported exports syntax/versioned condition | 1 |
| Total | 1651 |

[R1-REPORT.md](R1-REPORT.md) retains the complete feature-by-reason table. Machine data: `differential-r1-final/summary.json`, prism/oracle JSONL and empty wrong.json. The generator, ProjectService oracle and runner are in probes/. The product couples writer/dependency package type; separate ESM and outer-scope controls distinguish them. All 30 sol probes and every defined Opus case are included. Ten undefined Opus IDs remain unverified, as listed in OQ and VERIFICATION.

## Regression and cache preservation

Twenty-four package tests pass. Ten existing graph-path regressions fail behaviorally on committed 92c1d0bc in this environment (0/10), then pass on R1. New API-only tests have no executable pre-change API and are not counted as behavioral RED. The additional R1 root-typesVersions defect has a same-environment prototype/main control and a complete enumerated 144-case failing population, then passes the final differential gate. No unrelated baseline was changed.

Persisted CPG and nav sidecar: 12 edit/warm/cold checks pass, including package.json, tsconfig, symlink, declaration and outside-root scope changes. Both artifacts load on genuine full hits without mtime changes; edits invalidate both; complete cached/cold output agrees. The dirty-sidecar override is confined to synthetic subprocesses. S1b-4: 411 unchanged scenarios, 1,234 generated files byte-identical; lane-P public rows are preserved by the full-stream comparisons.

## Historical S2 and controller boundary

The earlier packet's scratch S2 result (0/132 recovered on X and installed-X) is supplied historical evidence under planning/s2-final, not an R1 rerun. No S2 source, policy or refusal cut was adopted. Lexical node:/scheme spelling alone is not an absence proof: paths and self-name precedence now apply first. Only the three-way native-semantics result may justify S2-O9 exclusion.

Private F remains controller-only. CONTROLLER-pkg.sh is syntax-checked and its native checker is exercised through public measurements. Use the four binary roles in current BUILD-MANIFEST.json, including repair-r1/bin/r1-prism and r1-facts. F execution, independent acceptance, performance/cost decisions and S2 adoption are not authorized by these receipts.

No public STOP occurred: zero wrong public bindings and zero lost main edges.
