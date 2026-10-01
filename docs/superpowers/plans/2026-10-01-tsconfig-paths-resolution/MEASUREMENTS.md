# Lane-P measurements — jsconfig precedence correction

MEASURED: integrated prototype 304f24b2 plus src/js_paths.rs and tests/integration/js_paths_test.rs; plan HEAD 38519a04; cumulative base e61d52b8. Current receipts target/paths-plan/jsconfig-precedence/evidence. BUILD-MANIFEST binds exact source, patch, executables, probes and logs.

READ controller-found WRONG, corrected in place: **a same-directory jsconfig barrier removed all F yield**. MEASURED generic constructible input: an admitted tsconfig paths alias with a sibling jsconfig returned empty UnknownName instead of one exact/import_member target. The bounded fix gates jsconfig only where that directory has no tsconfig. Strictly-nearer/no-tsconfig and delegated-project barriers remain. Private restoration has not been verified.

MEASURED: TypeScript 5.9.3 ProjectService.forEachConfigFileLocation probes tsconfig then jsconfig per directory ([source](https://github.com/microsoft/TypeScript/blob/v5.9.3/src/server/editorServices.ts#L2566-L2578)); tsc's findConfigFile defaults to tsconfig ([source](https://github.com/microsoft/TypeScript/blob/v5.9.3/src/compiler/program.ts#L327-L331)). The oracle applies that precedence while preserving P1's nearest-including root-file policy. membership-control.json proves both public twins have valid tsconfig membership and resolve independently of prism. This rules out invalid config/excluded membership as the cause; hypothesis-probe-result.log records expectations, alternatives and inadmissible probes.

| Current check | Result |
|---|---|
| X / R / T correct changed bindings | 3,121 / 0 / 0; delta 0 / 0 / 0 |
| Complete-row public comparison | 0 added/removed keys; 0 metadata changes; all changed rows independently correct |
| Full default | 4810 passed / 0 failed / 1 ignored |
| Full MCP | 5003 passed / 0 failed / 1 ignored |
| Tier-A matrix | 170 ok / 0 regressions |
| Controls | 156 scenarios / 170 sites / 57 changes = 55 bindings + 2 refusals; 0 preservation violations |
| Paths Rust tests | 23 passed; pre-change same-directory test assertion RED |
| Kernel mutants | 21/21 killed; M21 killed by C72 JSX and TSX |
| Formatter / diff / clippy | pass; clippy 139 warnings |
| Controller public synthetic smoke | 170 sites / 57 correct changes; actual_F_run=false |

| New control | JSX | TSX | Before-head / oracle | Corrected head / oracle |
|---|---|---|---|---|
| C72 same-directory tsconfig+jsconfig | gain | gain | UnknownName / JSCONFIG_BARRIER | one exact target / recoverable |
| C73 strictly-nearer jsconfig, parent config pair | preserve base | preserve base | base / JSCONFIG_BARRIER | identical base / JSCONFIG_BARRIER |

MEASURED: before-head-controls.json asserts that only the two C72 rows differ across complete old/new rows. All previous controls and solution references remain identical. C72 plus M21 expose the exact lost-yield mechanism in both grammars. Rust additionally tests an invalid ignored sibling, and nearer-jsconfig refusal with and without any tsconfig. M21 changes exactly two resolved kernel requests to None; compile errors are never counted as kills.

MEASURED: Tier-A quick completed in 311.49 seconds. Baseline validity: False; invalid reasons: ['corpus_sha_drift: 304f24b2d2b0 != pinned 20c8490591a3', 'stratum C-method: 4/6 successful probes']. Matrix 170 checks, 0 regressions. No pin, query, grade, oracle or baseline override. Read quick/result.json and quick/receipt.json for full accuracy validity reasons. This quick cannot support an accepted accuracy baseline if meta.baseline_invalid is true. No regression attribution without a same-environment base quick control.

MEASURED: P1.diff SHA-256 **1b238c4d2cc96760c7e4db59b8cbe536835e08d8d383104cfbccd4488aebbc3b**, against e61d52b8; exact Gitless replay of all 38 owned paths. Head SHA-256 **c56c364608b44bb1d6c8ad4660870c257c41c2580f2f88d6868002620b757bbc**. Full source and packet snapshots bind these bytes. No Git writes or private corpus reads.

READ exclusions this turn: private restoration, all-features Cargo, eleven integration mutants, S1b-4 byte-equivalence probe, cache acceptance probe, parent full suites/quick and full multi-corpus Tier-A were not rerun. Existing default/MCP tests exercised their covered paths; historical dedicated receipts below are not current acceptance. Independent round two, owner decisions and P2 remain external/parked.

# Historical rebind-e61 measurements

READ: everything below is retained historical evidence for the previous 9c52a382 integration packet. Its source/binary/hash/count assertions are superseded as current claims by the section above; they are not reruns on the corrected body.

# Lane-P measurements after e61d52b8 rebind

MEASURED: the final prototype source is frozen in `/Users/wesleyjinks/code/prism-paths-proto` at 9c52a382 plus three integration changes, bound by `target/paths-plan/rebind-e61/evidence/source-hashes-frozen.json`. The cumulative P1.diff is byte-exactly replayed from e61d52b8. All current receipts below use `target/paths-plan/rebind-e61/evidence/` unless explicitly labeled historical. BUILD-MANIFEST binds source, binaries, compiler, logs and expected files. No private F corpus or raw private result was opened. No Git write was made.

## Finding dispositions

| Finding | Disposition | Concrete evidence / boundary |
|---|---|---|
| W1 WRONG | Corrected; not downgraded | C39/C40 exclude matches the path or any parent, including dotted directories. C41/C47 gate JS/JSX/MJS/CJS with allowJs. C42 refuses same-stem JS roots; C65 adds declaration sibling priority. C43/C64 refuse package-folder ownership. All run in both grammar groups. |
| W2 WRONG | Corrected; not downgraded | C44 uses `@ext` plus `./a` in both star orders, with relative controls. Alias results reached past missing/unresolved/opaque stars preserve base; relative results preserve base. C55/C57/C59/C60 cover nested, syntax, unknown-expression and requested-name opacity; C56/C61 admit proven-empty and unrelated enumerated declarations. |
| S1 SMELL | Addressed now as Option K | C45/C46/C54 and the oracle classify nearer jsconfig, references and files: [] as barriers. OQ2 retains the possible future building-project ruling. |
| S2 SMELL | Addressed; fresh F still controller-only | Node/Node10-only recoverability; aggregates-only ordered refusal histogram; C49 probes a complete unknown dotted suffix before extensions. X/F comparison is below. No guessed attribution of the private gap. |
| S3 SMELL | Addressed | C41/C47 allowJs-off, C48 outDir and C39/C40/C52 excludes kill M11/M12/M13/M05. OutDir is a conservative boundary control. |
| S4 SMELL | Addressed | Config/extends and package bytes plus relevant source/opaque occupancy fingerprint. Unrelated .txt retains cache bytes/mtime; config, parent, candidate and package edit/add/remove parity is tested. |
| S5 SMELL | Addressed | Trim relative prefix classification/delegation; keep raw alias keys. C51/C58 preserve relative ESM/CJS, C62 refuses borrowed spelling proof and C63 admits distinct exact raw keys. |

READ historical R1: the review's W1 rows (a)–(d) are respectively represented by C39/C40, C41, C42 and C43, in both grammars. Pre-change `old-prism` is the retained d53cacbd prototype binary; same-environment base/head/old rows demonstrate 54 scenario differences (`target/paths-plan/r1/final-evidence/prechange-control-rows.json`). `membership-doubt-probe/*-pre.json` additionally records the TS fileNames and actual ancestor/decoy errors that motivated final barriers. These are behavioral artifacts, not exit-status claims.

READ historical R1 audit: the final audit found literal package-folder, Unicode-byte glob and declaration-priority membership errors. Rather than keep expanding the matcher after the correction cap, wildcard membership now refuses non-ASCII paths/patterns, case disagreements, package folders and case-variant config names. Explicit files remain authoritative, including Unicode. C64–C69 pin these cuts; C70/C71 prove Unicode target/explicit-file admission. These cuts cost zero final public X/R/T changed rows. They are stated P1 scope cuts, not claims of full TypeScript membership emulation. Independent round 2 may trigger the controller's broader refuse-on-any-doubt switch.

## Oracle admissibility

MEASURED: TypeScript 5.9.3 is the retained offline compiler, sha `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. The oracle independently checks the call token's imported binding, selected parsed root-file set, module target, callable export/name/span and fixed React wrapper grade. Recoverability additionally requires explicit effective Node/Node10 and no delegated/jsconfig ownership barrier. Config diagnostics fail closed. Old source-site/import-fact universes remain the census boundary. Delegated-project barriers change context/mechanism counts: X inherited/per-package imported-low attributes each fall by six (3,393→3,387 and 3,418→3,412); R/T reference/extends attributes are no longer counted through a refused owner. Admitted callable/changed counts stay unchanged.

READ / MEASURED blind spot: `checker.getExportsOfModule` can return a first-star-wins symbol even when two different stars export the same name; that symbol alone falsely certified old W2 edges. The revised oracle checks TS2308 semantic barrel diagnostics, including reachable barrels, and treats such exports as ambiguous. C44 verifies both orders; the production resolver independently refuses skipped stars. TS2308 catches the TSX controls; with checkJs disabled, JSX symbol lookup still exhibits the first-wins blind spot and can call unchanged conflicting candidates recoverable. Production P1 refuses them in both grammars. This diagnostic check does not certify all TypeScript semantic diagnostics or runtime export behavior. Refusal codes are independently ordered observed P1 cuts, not instrumented production reasons; `UNCLASSIFIED_P1_PROOF` remains an open explanation rather than a proven scope cut. Only unchanged recoverable rows enter the histogram.

## Final public corpus counts

| Corpus | All sites | Imported low | Module-resolved low | Callable recoverable | Changed | Classes |
|---|---|---|---|---|---|---|
| X | 19219 | 3754 | 3156 | 3131 | 3121 | {'CORRECT_STATIC_BINDING': 3121} |
| R | 953 | 168 | 0 | 0 | 0 | {} |
| T | 61712 | 15 | 0 | 0 | 0 | {} |

MEASURED: each changed X row is empty UnknownName to one Exact/import_member and independently matches TypeScript. All three corpora have zero added/removed site keys and zero non-resolution metadata changes. Count change from 3,121 / 0 / 0: **0 / 0 / 0**. The corrected empty-star handling prevents the intermediate 1,226-site math regression; unsupported dotted suffixes and conservative membership barriers have zero final X cost. Intermediate runs are superseded, not accepted results.

MEASURED: fresh `public/{X,R,T}-expected.json` derives target identities from the TypeScript terminal, then asserts equality to head; `public/FINAL-SUMMARY.json` and `public/source-input-hashes.json` bind counts and source/context inputs. P0 low means a TS-proven imported source call/JSX site without an Exact target; parameter shadows are counted separately. No NameOnly upgrade is in the admitted candidate set. The oracle paths-removal control distinguishes alias causality from coincident resolution. Exact/wildcard/index/inheritance mechanism attributes overlap; they must not be summed.

| Mechanism (overlapping) | X low / resolved / callable | R | T |
|---|---|---|---|
| per_package_config | 3412 / 2892 / 2875 | 0 | 0 |
| paths_exact | 3012 / 3012 / 2991 | 0 | 0 |
| index_file | 3016 / 3016 / 2994 | 0 | 0 |
| paths_wildcard | 144 / 144 / 140 | 0 | 0 |
| extends_chain | 3387 / 2892 / 2875 | 0 | 0 |

MEASURED: X's 3,131 callable alias sites still include ten unchanged callable candidates. Their exact keys and independent terminals are in `public/X-retained-callable-candidates.json`; no permissive export/producer proof was added to recover them. The name-directed independent histogram assigns eight to NONRELATIVE_EXPORT_HOP and leaves Footer/WelcomeScreen (two) UNCLASSIFIED_P1_PROOF; it does not pretend those two have a proven causal refusal reason. Changed-row terminal classes are 1,864 function_variable, 1,247 function and 10 wrapped_function. This is changed-row static correctness under the stated ownership/wrapper contract; it does not certify pre-existing edges or runtime behavior.

## X versus F / OQ1 cost

MEASURED: X recovers **3,121 / 3,131 = 99.68%**, leaving ten static callable candidates. Unknown dotted suffix probes are implemented, so the old dot-in-last-segment cut is removed; C49 is oracle-checked in both grammars. Known JS-family/JSON/MTS/CTS extension substitution, competing candidates, packages, tie/order and unsupported project ownership remain explicit P1 cuts.

READ controller-supplied, prior binary/oracle: F had **13,299 sites**, **2,345 changed rows**, all CORRECT_STATIC_BINDING, **3,102 callable-recoverable** rows and zero added/removed keys. Yield **75.60%**, gap **757**; all 3,102 used wildcard paths and 724 had index destinations. The old denominator admitted module-resolution modes outside P1. Its 757-row gap is not locally attributed or certified by this fold. The controller must rerun the new immutable binary and oracle to obtain fresh F yield, denominator and refusal histogram. No fresh F claim is made here.

MEASURED: CONTROLLER-paths.sh defaults to `target/paths-plan/rebind-e61/head/prism`, emits one aggregates-only JSON object and retains paths/source/rows/diagnostics privately. Public-only script smoke passed on all 152 synthetic scenarios; 55 changes (53 bindings, 2 refusals), zero keys, empty controller stderr. The wrapper receipt explicitly says PUBLIC_SYNTHETIC_CONTROLS_ONLY and actual_F_run=false; the script's fixed `corpus: F` label in that smoke is not F evidence.

## Control table

| Controls (each grammar) | Evidence | Result |
|---|---|---|
| C01–C38 | Exact/wildcard, inheritance/origin, files, JSONC, index/barrel/default; ties, duplicate/cycle, arrays, NodeNext, package/declaration/competition, wrapper/shadow/require | Original admitted and preservation cases pass |
| C39/C40/C41/C42/C43 | Parent/dotted excludes; mjs/cjs allowJs; same-stem JS; bower/jspm | Membership errors corrected or refused |
| C44 | `@ext` + `./a`, both orders; relative import control | Alias and complete base relative rows preserved |
| C45/C46/C47/C48/C54 | jsconfig/delegated/files-empty, allowJs false, outDir | Barriers; TSX allowJs-false positive |
| C49/C50/C51/C52/C53 | Unknown/known dotted suffix; relative whitespace; exclude nonmatch; explicit files priority exemption | Admitted positives and refusals pass |
| C55–C61 | Nested/skipped/proven-empty/syntax/CJS/name-specific stars; relative CJS whitespace | Empty known facts admitted, opacity refused, relative rows preserved |
| C62/C63 | Raw alias spellings | No borrowing; distinct exact key admits |
| C64–C71 | Literal package includes; .d.ts priority; Unicode/case/config barriers; exact Unicode target/files | Conservative negatives and exact positives pass |

MEASURED: **152 scenarios / 166 sites / 55 changes = 53 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL**, zero complete-row preservation violations. The two refusals are existing WrappedExportNonJsx grading for ordinary calls of proven React wrappers; their JSX counterparts bind Exact. Integration fixture data covers the same mechanisms (C44 uses the equivalent `./real` filename); standalone C44 uses the explicitly requested `./a`. Rust tests additionally inject Position/Unchecked/MayCall/unbound states into both graphs and compare the complete result.

## Mutant tables

| Kernel mutant | First killing control | Changed requests | Result |
|---|---|---|---|
| M01-no-exact | C01-exact-jsx | 71 | KILLED |
| M02-shortest-prefix | C04-longest-prefix-jsx | 2 | KILLED |
| M03-child-paths-origin | C08-child-inherits-paths-jsx | 12 | KILLED |
| M04-ignore-include | C08-child-inherits-paths-jsx | 15 | KILLED |
| M05-ignore-exclude | C39-exclude-parent-glob-jsx | 4 | KILLED |
| M06-ignore-declaration-blocker | C21-declaration-blocker-jsx | 2 | KILLED |
| M07-ignore-package-boundary | C23-package-manifest-jsx | 2 | KILLED |
| M08-accept-tied-pattern | C06-tied-prefix-jsx | 2 | KILLED |
| M09-ignore-baseurl-origin | C09-baseurl-origin-jsx | 2 | KILLED |
| M10-unmatched-target-star | C36-unmatched-target-star-jsx | 2 | KILLED |
| M11-no-allowjs | C41-allowjs-off-cjs-jsx | 5 | KILLED |
| M12-no-outdir-barrier | C48-outdir-barrier-jsx | 2 | KILLED |
| M13-exclude-fullpath-only | C39-exclude-parent-glob-jsx | 4 | KILLED |
| M14-no-same-stem-barrier | C42-same-stem-js-jsx | 4 | KILLED |
| M15-include-package-folders | C43-package-folder-bower_components-jsx | 8 | KILLED |
| M16-no-declaration-priority | C65-declaration-priority-jsx | 2 | KILLED |
| M17-byte-unicode-glob | C66-unicode-glob-barrier-jsx | 2 | KILLED |
| M18-no-include-case-barrier | C67-case-include-jsx | 2 | KILLED |
| M19-no-exclude-case-barrier | C68-case-exclude-jsx | 2 | KILLED |
| M20-ignore-case-config | C69-case-config-name-jsx | 2 | KILLED |

| Actual-source integration mutant | Killing assertion test | Result |
|---|---|---|
| I01-no-config-fingerprint | js_paths_config_change_and_incremental_rebuild | KILLED |
| I02-no-post-merge-map | js_paths_config_change_and_incremental_rebuild | KILLED |
| I03-no-span-requirement | js_paths_cjs_terminal_without_span_preserves_base | KILLED |
| I04-no-binding-position-guard | js_paths_require_alias_preserves_base_even_with_esm_same_module | KILLED |
| I05-no-skipped-star-guard | js_paths_r1_skipped_star_preserves_base | KILLED |
| I06-all-file-occupancy | js_paths_unrelated_text_keeps_topology | KILLED |
| I07-no-proven-empty-facts | js_paths_r1_membership_barriers_dot_and_relative | KILLED |
| I08-no-opaque-star-guard | js_paths_r1_skipped_star_preserves_base | KILLED |
| I09-ignore-skipped-declaration-name | js_paths_r1_skipped_star_preserves_base | KILLED |
| I10-trim-alias-cache-key | js_paths_r1_membership_barriers_dot_and_relative | KILLED |
| I11-taint-namespace-terminal | js_paths_s1b_namespace_star_proof_is_reused | KILLED |

MEASURED: all **20 kernel + 11 integration** mutants are killed. Kernel mutants compile the actual three production resolver modules and compare concrete module outputs; integration mutants use copied actual source trees and each compile/select one test/produce an assertion panic. Setup, zero-test and compile failures count as no evidence. Boundary mutants (outDir, case/Unicode cuts, full-file topology) enforce conservative policy rather than independently demonstrate wrong targets. Existing M01–M10 and I01–I04 remain killed; no mutant was quietly removed.

## Suites, matrix and cache

| Final suite | Passed | Failed | Ignored | Result groups |
|---|---|---|---|---|
| default | 4808 | 0 | 1 | 29 |
| mcp | 5001 | 0 | 1 | 31 |
| all-features | 5024 | 0 | 1 | 31 |

MEASURED: release rebuild preceded the matrix in the same prototype tree. Matrix **170 passed / 0 failed** (fresh e61d52b8 base matrix **166 / 0**; four P1 fixtures), including four added positive/declaration-refusal fixtures, no existing expectation rebaseline. Formatter passes. Scoped `cargo clippy --offline --lib --features mcp` passes; 139 library warnings are reported, not hidden.

MEASURED: cache CLI probe passes cross-binary rejection, cold/full-hit/sidecar/no-cache equality, config-only left→right caller movement, extends parent edit, candidate and package add/remove parity, and unrelated .txt add/remove keeping the existing cache bytes/mtime. The empty-source-change incremental versus full graph test passes in the suites. Tests assert actual caller/function artifacts and cache state, not only command status. Fingerprinting excludes ordinary directories and unrelated text/markdown/hidden regular files; source-candidate occupancy, opaque directories, config chains, package bytes and completeness remain inputs. Large-tree performance remains unmeasured.

MEASURED: fresh e61d52b8 standalone base P1 harness **8 behavioral RED / 9 preservation passes**; same standalone harness on head **17 GREEN**, in this environment (`base-harness.log`, `head-harness.log`). READ historical: old 5048f443 base full suite 4,750/0/1 is not a new-parent suite claim; no full base suite was rerun. The ignored upstream test remains `integration::resolution_test::slice_elem_variant_reserved`; no inherited failure was rebaselined or silently fixed.

## Exclusions and inadmissible work

READ: no private F source/raw evidence; fresh aggregates are controller work. Full multi-corpus Tier-A is human-triggered, not run. Independent Opus round 2, owner confirmation, Git commit/push/merge and P2 are not performed here. Snapshot race/security guarantees and large-tree overhead remain unmeasured; public corpora are trusted read-only inputs.

MEASURED: second actual-worktree Tier-A quick **completed in 474.24 seconds** within its 900-second bound, return code 2. Its accuracy baseline is **invalid** for **corpus pin drift (9c52a3824662 != 20c8490591a3)** and **C-method stratum only 4/6 successful probes**. Oracle error rate 0.0667, SUT error rate 0; all 170 matrix outcomes are ok. No pin, query, oracle, grade or baseline override. quick-summary.json and quick-second/generated retain the actual report, raw run and inventory snapshot. No regression attribution is made without a same-environment base quick control; that control was not run.

MEASURED: PROBE-LOG.md separates setup failures (redirect/archive/copy/boundary/premature-finalization and refused process inventory) from admissible behavior. Every final suite hashes the same 38 files before/after. Mutation builds precede suites serially, while binary measurements use copied immutable executables. No integration repair extension was needed beyond round one; no artifact restart or independent review approval is claimed.

## Reproduction

```bash
python3 probes/reproduce_public.py "$BASE_BIN" "$HEAD_BIN" "$BASE_FACTS_BIN" "$TS_JS" "$OUT/public"
python3 probes/final_receipts.py "$OUT/public" "$BASE_BIN" "$HEAD_BIN" "$BASE_FACTS_BIN"
python3 probes/controls_gen.py "$OUT/controls"
# Dump base/head and base facts, then rowdiff.py, oracle.cjs and verify_controls.py.
python3 probes/mutants.py "$WORKTREE" "$OUT" "$RELEASE_DEPS" "$OUT/controls"
python3 probes/integration_mutants.py "$WORKTREE" "$OUT" "$MUTANT_TARGET_DIR"
python3 probes/cache_probe.py "$BASE_BIN" "$HEAD_BIN" "$OUT/cache"
# Controller alone supplies CORPUS_F_ROOT, PRIVATE_EVIDENCE_ROOT and TS_JS:
bash probes/CONTROLLER-paths.sh
```

## Integration and source-bound verification

MEASURED: head versus fresh main has **411 S1b-4 scenarios / 639 sites** with byte-identical complete dumps, function inventories and stderr. Both summaries also match the committed S1b-controls-s1b4-r5-impl.txt reference: SHA-256 b550a2c7466fdbe4d331f93843d44f0bfcfb6c62febf81f115c86a9ab64dd5ca (`s1b-identity.json`).

MEASURED: local namespace Callable terminals initialize via_unresolved_star=false. The unchanged landed namespace_identity returns Err before insertion on any unresolved star branch; explicit named exports override stars. P1 reuses landed extraction, module/position binding classification, relative-module resolution, R4c span/wrapper/unique callable gates, and both export projections. Its alias-only skipped-star provenance preserves the legacy relative row policy; it is not a second namespace resolver. The two namespace resolver functions are byte-identical to main. Landed is_empty already retains complete namespace facts; extra P1 empty facts have namespace_proof_complete=false and cannot grant namespace authority. The added JSX/TSX regression covers clean, unresolved-sibling and explicit-named-override routes; I11 kills a true terminal initializer.

MEASURED: root VERIFICATION.md records the actual commands and exclusions. Full-suite hashes/totals and all 38 source hashes were rechecked (`verification-summary.json`). Historical R1 oracle RED/O01 evidence is retained in r1/final-evidence, not rerun here. Current 152 controls execute their strengthened oracle assertions, and fresh public finalization checks requested-member histogram assertions. No Rust behavior changed after the full suites.

## Controller F acceptance after the jsconfig-precedence fix (private, aggregates only; 2026-10-01)

Run with head `060092b4` (cumulative on `e61d52b8`), the manifest base binary, and the TypeScript 5.9.3 oracle.

**Result:**
- Changed rows: **2,345**, all `CORRECT_STATIC_BINDING`.
- Keys added or removed: 0.
- Callable-recoverable: 3,106. Module-resolved bindings: 5,043.

**Refusal histogram:**
- `NONRELATIVE_EXPORT_HOP`: 619. A candidate for a later P2 slice; re-measure before authorizing it.
- `UNCLASSIFIED_P1_PROOF`: 119.
- `BINDING_OR_SITE_GUARD`: 23.

**Controller-found defect:** before this fix, a same-directory jsconfig barrier (from the round-1 S1 disposition) removed all F yield. The interim rebind run measured 0. Lesson: run the private-corpus acceptance immediately after every fold that adds a barrier.
