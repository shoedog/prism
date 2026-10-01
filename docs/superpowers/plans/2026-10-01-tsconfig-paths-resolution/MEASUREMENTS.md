# P0 and prototype receipts — lane P

MEASURED: public roots, unchanged base/head source-site identities, TypeScript 5.9.3 and artifact hashes are bound in BUILD-MANIFEST.md. Raw evidence stays under `target/paths-plan/`; this report is an index to it. No private F input or raw result was opened. Fresh public row, source, build, matrix and script-smoke counts are indexed to the resume receipts below. Earlier suite/control/mutant/cache executions are retained receipts; the resume reads their output and reparses totals, without claiming they were re-executed.

MEASURED: final resumed command `python3 probes/reproduce_public.py <base> <rebuilt-head> <base-facts> <bound-TS> target/paths-plan/resume-public` completed successfully. **Authoritative public artifacts are now `target/paths-plan/resume-public/`**: `FINAL-SUMMARY.json`, all X/R/T P0/changes/classified files, base/head dumps and `source-input-hashes.json`. Unqualified public JSON filenames below refer to this subtree. Earlier public rows and import facts reproduced exactly; 640 X, 53 R and 721 T source/context files are hashed. Final same-environment base/head keys and non-resolution metadata are unchanged.

## Public P0

MEASURED: a low site is a source call or JSX site with an eligible non-relative import fact and no Exact target. TypeScript separately proves the call token denotes that imported binding. All proved low sites here have base `UnknownName` and zero targets; **NameOnly candidates: 0**. Shadowed facts are counted separately, not called alias-caused losses.

| Corpus | All sites | Import-fact associated low | TS-proven imported low | Paths/baseUrl module-resolved low | Callable recoverable |
|---|---:|---:|---:|---:|---:|
| X | 19,219 | 3,769 | 3,754 | 3,156 | 3,131 |
| R | 953 | 168 | 168 | 0 | 0 |
| T | 61,712 | 15 | 15 | 0 | 0 |
| F | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN |

MEASURED: X excludes 15 parameter/local-shadow associations. Recoverability additionally requires a repository-contained TypeScript-resolved module, no config diagnostics, a single callable export terminal and the admitted wrapper/JSX grade. A module match without callable proof is not an edge. The oracle removes `paths` and re-resolves: none of the 3,156 X resolved candidates retains that target in the removal control. R/T have no resolved alias candidates. The earlier X/F projections were unmeasured.

## Mechanisms

MEASURED: each cell is **imported low / module-resolved / callable**. Attributes overlap and are not summed. `references present` records metadata, not reference-following success. `index destination` includes explicit mappings to index files as well as directory-index lookup.

| Mechanism | X | R | T | F |
|---|---:|---:|---:|---|
| paths exact | 3,012 / 3,012 / 2,991 | 0 / 0 / 0 | 0 / 0 / 0 | UNKNOWN |
| paths wildcard | 144 / 144 / 140 | 0 / 0 / 0 | 0 / 0 / 0 | UNKNOWN |
| baseUrl bare | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | UNKNOWN |
| extends chain | 3,393 / 2,892 / 2,875 | 0 / 0 / 0 | 15 / 0 / 0 | UNKNOWN |
| per-package config | 3,418 / 2,892 / 2,875 | 0 / 0 / 0 | 15 / 0 / 0 | UNKNOWN |
| references present | 0 / 0 / 0 | 139 / 0 / 0 | 15 / 0 / 0 | UNKNOWN |
| non-relative .js → .ts | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | UNKNOWN |
| index destination | 3,016 / 3,016 / 2,994 | 0 / 0 / 0 | 0 / 0 / 0 | UNKNOWN |
| package exports/workspace resolved | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | UNKNOWN |

MEASURED: X’s 3,131 callable sites point to 41 module destinations and 436 distinct callable identities. Largest module destinations: `packages/math/src/index.ts` 1,226; `packages/element/src/index.ts` 1,025; `packages/common/src/index.ts` 718; `packages/excalidraw/i18n.ts` 40; `packages/excalidraw/index.tsx` 17. Actual declaration origins can differ through relative barrels. `X-candidates.json` retains every site’s specifier, selected config/chain, resolved module and `(file,name,start_line,end_line)` terminal. `X-imports.json` retains import-level results. R/T equivalents are retained too.

ASSUMPTION: exact/single-star paths with local inheritance and package-local configs offer the justified P1 cut. BaseUrl is included only as a matched substitution’s origin. Bare lookup, ordered substitutions, .js mapping, references and package routing have no measured public alias gain; park them. X’s 16.24% whole-site gain justifies P1; do not claim a general R/T or F benefit.

## Final prototype row audit

MEASURED: `rowdiff.py` joins base/head by caller, source bytes and callee spelling, rejects duplicate keys and non-resolution metadata changes, and reports all target/drop changes. Both binaries ran in this environment against the same read-only corpus. No key was added or removed.

| Corpus | Changed rows | Transition | CORRECT_STATIC_BINDING | Other classes |
|---|---:|---|---:|---:|
| X | 3,121 | UnknownName → one Exact/import_member | 3,121 | 0 |
| R | 0 | none | 0 | 0 |
| T | 0 | none | 0 | 0 |
| F | UNKNOWN | controller-only | UNKNOWN | UNKNOWN |

MEASURED: the fresh final TypeScript oracle checks each changed call’s import token, module, declaration name and exact callable span. The final classifier also requires config validity, admitted wrapper position, Exact confidence and import_member kind. All 3,121 satisfy these checks (1,864 function-variable, 1,247 function, 10 wrapped-function terminals). “0 wrong” is scoped to these changed rows under OQ2’s root-file selection and the READ static/wrapper model. It does not certify old edges, runtime behavior or private F.

MEASURED: changed-row mechanism attributes are X exact paths **2,984**, wildcard paths **137**, inherited/per-package configs **2,873** and index destinations **2,984**; attributes overlap. R/T have no changed-row mechanism counts. The final summary retains each breakdown.

MEASURED: ten callable X candidates retain base: getSceneVersion ×4; getCommonBounds ×2; FooterCenter, WelcomeScreen, getSelectedElements and convertToExcalidrawElements ×1 each. `X-retained-callable-candidates.json` supplies those exact sites and origins. ASSUMPTION: adjacent producer/graph proof work should stay outside P1; do not widen authority to recover those ten without new evidence.

## Verification

READ retained execution: base `cargo test --offline`: **4,750 passed, 0 failed, 1 ignored**, 29 groups (`base/cargo-test.log`, `base/test-totals.json`). Retained standalone P1 harness on that base library: **8 behavioral RED / 9 preservation pass**, 17 groups (`base/P1-red.log`). The base and prototype executions used the same local Rust/OS environment and fixture source bytes. MEASURED: this resume reparsed all suite result groups and matched their recorded totals (`resume-suite-receipts.json`). The 4,983-pass all-features run is the largest completed suite; it was not repeated because no production or test behavior changed during this receipt-only resume.

READ retained execution: final prototype `cargo test --offline --features mcp`: **4,960 passed, 0 failed, 1 ignored**, 31 groups. Final `PRISM_TYPESCRIPT=<bound offline compiler> cargo test --offline --all-features`: **4,983 passed, 0 failed, 1 ignored**, 31 groups, including all 23 optional compiler-audit tests. Logs and parsed totals are `proto-full-test*` and `proto-all-features-test*`. The one upstream ignored test is `integration::resolution_test::slice_elem_variant_reserved`, annotated as reserved SliceElem behavior; it was not enabled or rebaselined.

MEASURED: the final source-bound release rebuild and repeated Tier-A matrix passed (`resume-build.log`, `resume-tier-a-matrix.log`, `resume-binaries.json`). Tier-A matrix: **169 fixtures passed, 0 failed**, including four new JSX/TSX positive/declaration-refusal fixtures. Immediate release rebuild and matrix ran in the same prototype tree using the installed Python 3.12 interpreter (`resume-tier-a-matrix.log`; earlier `tier-a-matrix.log` agrees). Existing fixture expectations were not changed.

READ retained execution: finite synthetic controls: **76 scenarios / 84 sites**, **28 CORRECT_STATIC_BINDING changes + 2 CORRECT_STATIC_REFUSAL changes**, no key/metadata changes and zero complete-row preservation violations (`controls-*`, `controls-verification.log`). The two refusals are ordinary calls of independently proven React wrappers; S1’s existing non-JSX grade is `WrappedExportNonJsx`. Their JSX counterparts bind Exact. Position/Unchecked/MayCall/unbound injection is separately covered in the Rust harness in both grammars.

READ retained execution: ten actual-kernel mutants are killed. Four actual-source integration mutants are killed with admissible single-test assertion failures; their summary/logs are bound separately. Cache CLI controls passed: cross-version rejection, cold/full-hit/sidecar/no-cache equality, config-only left→right caller movement. The Rust incremental/full parity test passed.

READ retained execution: formatter and scoped clippy receipts are bound in BUILD-MANIFEST.md. Clippy reports 139 library warnings; no claim that the tree is warning-free is made.

MEASURED: the F script exists, is executable, passes `bash -n` and an actual public C01 TSX smoke. Its stdout was exactly one aggregate JSON object, with one correct binding, zero added/removed keys and no paths, callable names or raw rows; stderr was empty. Receipt: `controller-public-smoke/receipt.json`. This was public synthetic input, **not a private F run**.

## Exclusions and inadmissible probes

MEASURED: the prototype’s own Tier-A quick run produced **no admissible result**. It inherits main’s Git identity under ignored `target/`, yielding an empty tracked source universe, `oracle_unsupported` and the committed corpus pin mismatch (`5048f44300a7` vs `20c8490591a3`). The receipt is `tier-a-quick-inadmissible.json`. An auxiliary quick control on real main was stopped before it produced a report; it is not a result. READ: a real controller-created worktree is required for the prototype’s tracked-universe run. Current-parent evaluation may use `--allow-drift` for an explicitly labeled fresh-head quick control; it must not be presented as a pinned baseline or rebaseline committed data.

READ: normal uv and py_compile cache destinations were sandbox-refused. MEASURED: direct installed Python ran the matrix; Python probe sources compile with `compile()` without writing bytecode. The position-injection test initially used an unavailable BTreeSet mutable iterator and did not compile; fixed with an owned-set reconstruction. I04’s first mutation pattern missed rustfmt’s line break; that run supplies no I04 evidence. These setup errors are recorded in `PROBE-LOG.md` and `RESUME-PROBE-LOG.md` under the evidence directory and are excluded from RED/kill totals.

READ: private F is prohibited here and remains entirely controller-only. Full multi-corpus Tier-A is human-triggered and was not run. Opus review, owner answers, Git commits/worktree creation, push and merge were not performed. Public P0 is bounded to main’s indexed import/site universe and root-file membership; it does not discover sites the base parser never emitted. Snapshot concurrency/security and performance overhead were not measured; corpora were trusted read-only inputs.

## Reproduction

READ: the exact offline package path and hashes are in BUILD-MANIFEST.md. ASSUMPTION: use the same bound base/head/fact binaries or rebuild them in controller-owned worktrees; never install or fetch to repair the evidence environment.

```bash
python3 probes/reproduce_public.py "$BASE_BIN" "$HEAD_BIN" "$BASE_FACTS_BIN" "$TS_JS" "$OUT"
python3 probes/controls_gen.py "$OUT/controls"
# Dump both control binaries and base import facts; rowdiff, oracle.cjs, verify_controls.py.
python3 probes/mutants.py "$WORKTREE" "$OUT" "$RELEASE_DEPS" "$OUT/controls"
python3 probes/integration_mutants.py "$WORKTREE" "$OUT" "$MUTANT_TARGET_DIR"
python3 probes/cache_probe.py "$BASE_BIN" "$HEAD_BIN" "$OUT/cache"
```

READ: controller-only private invocation (stdout contains aggregates only; all paths/rows/diagnostics stay in PRIVATE_EVIDENCE_ROOT):

```bash
export TS_JS=/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js
# Controller supplies CORPUS_F_ROOT and PRIVATE_EVIDENCE_ROOT privately.
bash probes/CONTROLLER-paths.sh "$BASE_BIN" "$HEAD_BIN" "$BASE_FACTS_BIN"
```
