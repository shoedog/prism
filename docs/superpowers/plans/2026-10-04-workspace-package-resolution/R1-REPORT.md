# R1 repair report — bounded repair, S2 remains parked

Source base: `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318`. Packet base: `c89bc5b74df05e1f8746792c0cfad2805336b0f9`. Main comparison: `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. No Git writes, network/install, private F access, source restart or S2 adoption.

## WRONG repairs first

| Findings | Concrete repaired mechanism | Evidence |
|---|---|---|
| F1 / W1 | Exact TS exports no longer select absent-target sibling extensions; valid JS substitution remains | pkg_r1_exact_ts_exports; sol exports-ts-substitution witnesses |
| W2 | Literal target stars remain literal; only selected patterns substitute captures | pkg_r1_literal_star_and_pattern |
| F2 | Missing legacy .d.ts field tries same-stem .ts after its exact declaration probe | pkg_r1_missing_legacy_declaration_substitutes_source |
| F4 / W3 | Node ESM type:module dependencies do not extend extensionless fields; native index.js fallback is supported | pkg_r1_esm_legacy_field_and_index |
| F3 | Writer/barrel extension determines usage format before package type | pkg_r1_writer_extension_mode |
| F5 | Retained outer package scope determines format; opaque/ambiguous scope declines | pkg_r1_outer_package_scope; persisted outer-scope checks |
| F6 | Self-name exports remain active when ordinary resolvePackageJsonExports=false | pkg_r1_self_name_with_exports_disabled |
| W4 | Node ESM export hops use caller options and barrel mode, without extension/index guessing; explicit JS substitution is supported | pkg_r1_esm_export_hop |
| W5 | #imports is Unsupported and cannot bind the unrelated node_modules shadow | pkg_r1_imports_shadow_is_unsupported; live shadow witness |
| W6 | Null is a known no-result, top-level null uses legacy, and proven inner misses continue outward; literal null never retries a lower same-map pattern | pkg_r1_null_continuation_and_outer_exports |
| F7 / C28 | Paths precede URI classification; matched unproved paths remains Unsupported | pkg_r1_colon_paths_precede_scheme; both C28 modes |
| F11 / W7 | Dispatch names committed source plus R1 and separate packet base; verification links use retained local reports | IMPLEMENTOR, FILES, BUILD-MANIFEST and VERIFICATION |
| Additional R1 WRONG | Newly supported canonical legacy subpaths could bypass root typesVersions and bind feature.ts instead of native other.ts. Guard moved into root metadata inspection before file probing | typesversions-subpath-probe; pkg_r1_typesversions_subpath_is_unsupported; 144-case differential witness population |

The additional typesVersions WRONG was introduced by R1's canonical-subpath admission: same-environment committed-prototype and main facts both have no proof, while the early R1 artifact had feature.ts. It is corrected in final R1. The failed placement produced 288 observations (144 bindings + 144 callable edges), all in that one class; the full population was enumerated before retry. No inherited WRONG was downgraded to SMELL.

## Structural and SMELL disposition

F8 / S1: Resolution is Bound, ProvenUnresolved or Unsupported(reason). Only native absence can justify S2-O9 exclusion. Declaration/external/opaque winners and omitted native features remain Unsupported. Production admits only Bound; the public resolver reports status and actual selected owner. #imports remains the brief-authorized minimum Unsupported implementation.

F9: linked legacy subpaths, bundler and Node ESM index fallbacks, colon paths, top-level null and outer continuation are implemented. C8 failed paths fallback remains an explicit authority barrier in OQ. Lower-risk C9/C10/C13 are implemented. JS secondary, arrays, versioned conditions/typesVersions, directory-valued fields and broad ownership/language/options stay explicitly gated.

F10: generated oracle differential is the rerunnable acceptance gate. Sol S2: persisted CPG and nav sidecar invalidation is tested with 12 edit/warm/cold checks, including metadata, config, links, new declaration winners and outer scope bytes. F12: package TSX without JSX declines; inherited ordinary Lane-P TSX behavior is recorded for follow-up. F13: classify is documented as lexical, without a production classification consumer.

## Differential totals

**5096 cases: 2242 Bound, 861 ProvenUnresolved, 1993 Unsupported. Zero wrong modules, false Exact callable edges, false native-absence claims or native expected-target mismatches. TS binds 1651 Unsupported cases.**

The full product is 4 modes × 7 writer extensions × 3 writer/dependency package types × 15 entry shapes × 4 workspace link/self shapes = 5,040 cases. Types are coupled in the product; separate ESM field and outer-scope witnesses discriminate writer versus dependency type. The 56 additional witnesses include all 30 sol probes, every defined Opus appendix case, C6b/C28 variants and two scope-precedence controls. Real TS ProjectService owner and usage mode are retained; Bound must match production module facts and native owner, and every Exact must match the full native declaration span.

The Opus review supplies no definitions for C3, C15, C17-C21, C23, C24 or C26. A clarification was requested; without a public fixture source those ten IDs remain unverified. No invented repros are counted. C16b's inherited ordinary Lane-P JSX behavior is outside the repaired package gate and remains a follow-up.

| Feature | Unsupported reason where TS binds | Cases |
|---|---|---:|
| declaration | declaration, unindexed, or opaque winner | 180 |
| declaration | unindexed writer extension | 90 |
| exports-array | export arrays | 180 |
| exports-array | unindexed writer extension | 90 |
| exports-conditions | unindexed writer extension | 90 |
| exports-null | unindexed writer extension | 72 |
| exports-pattern | unindexed writer extension | 90 |
| exports-string | unindexed writer extension | 90 |
| exports-subpath | unindexed writer extension | 90 |
| index-fallback | unindexed writer extension | 72 |
| legacy-main | unindexed writer extension | 72 |
| legacy-module | unindexed writer extension | 72 |
| legacy-types | unindexed writer extension | 72 |
| literal-star | unindexed writer extension | 18 |
| missing-target | unindexed writer extension | 18 |
| opus-C16 | JSX compiler option | 1 |
| opus-C27 | declaration, unindexed, or opaque winner | 1 |
| opus-C7 | package imports | 1 |
| opus-C8 | paths authority | 1 |
| opus-outer-self | outside-root self-reference | 1 |
| sol-array | export arrays | 1 |
| sol-closer-js-outer-ts | JS secondary priority pass | 1 |
| sol-modern-paths-win | paths authority | 1 |
| sol-package-imports-shadow | package imports | 1 |
| sol-paths-missing-fallback | paths authority | 1 |
| sol-types-directory | directory-valued package field | 1 |
| sol-typesVersions | typesVersions | 1 |
| sol-versioned-types | exports syntax or depth | 1 |
| ts-sibling-only | unindexed writer extension | 90 |
| typesversions-subpath | typesVersions | 180 |
| typesversions-subpath | unindexed writer extension | 72 |

Machine report: `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/differential-r1-final/summary.json`; all per-case native/Prism outputs and complete rejected predecessors are retained alongside it. These totals describe the generated population, not exhaustive TS conformance or S2 adoption.

## Regression evidence and gates

Ten graph-path R1 tests are behavioral RED on committed 92c1d0bc in this same environment (0 passed / 10 failed), and GREEN in final R1. Eight live prechange false-edge witnesses plus null misses independently bind owner/mode/native result. New status APIs have no pre-change callable API: no compile failure is counted as behavioral RED. Final package tests: 24/24 pass. The inherited missing() helper was repaired to examine resolved_targets rather than the always-null exact_target convenience field, making negative assertions behavioral.

- Requested single nextest: 5,160 passed / 0 failed / 1 skipped; precedes the final typesVersions guard and is explicitly limited to its source.
- Final-source full MCP cargo suite: **5163 passed / 0 failed / 1 ignored**, 31 result records including two doctests. No environment exclusion or out-of-scope failure was re-baselined.
- Final differential: 5,096 cases, zero wrong/false-absence observations.
- Persisted cache: 12 checks, both artifacts truly warm, metadata edits invalidate, complete warm/cold outputs match.
- Advisory scoped mutation gate: 26 selected / 26 admissible / 26 killed. It remains advisory; registry intent revisions explain three repaired witness defects.
- fmt passes; clippy passes with 182 lib-test warnings, including a current boolean-simplification warning in js_paths.rs. No warning cleanup was folded into this repair.
- Tier-A matrix: 178 OK / 0 regression / 0 skipped after immediate same-worktree release rebuild; quick is explicitly skipped. The installed slicing eval Python environment executes this checkout's tier_a module from eval/, without installs; wrong-module launcher output was inadmissible and not used.
- S1b-4: 411 scenarios / 1,234 output files byte-identical to main; complete call-site keys agree.

## Yield, custody and limits

Public yield/module preservation and fresh added-proof certificates are in MEASUREMENTS and public-r1-module-audit.json. Main denominators are X 19,219, installed-X 19,219, R 953 and T 61,712; X snapshots are not summed as independent corpora. Final source and binary hashes are in BUILD-MANIFEST. Patches are relative to the distinct committed bases, with proposed commit messages in FILES.

Diagnostic cap was two rounds. Bounded cache-probe corrections and the one-class root guard/placement corrections were disclosed before extensions; no artifact restart occurred. Failed/setup/wrong-module/old-source receipts remain historical and cannot certify final source.

Not verified: ten undefined Opus IDs, ordinary Lane-P JSX follow-up, private F/controller execution, S2 scratch execution/adoption, independent review, performance/RSS budgets, off-machine custody, full TS conformance, detached all-features suites, Tier-A quick/full multi-corpus. Final public audit passed with zero wrong bindings, lost edges or changed main module proofs; no public STOP occurred.
