# PR-A R2 final measurements

Source is committed `bd4c30ffbe0a843391ca5c44e68c69cd8ae3b4d5` plus R2-src.patch; docs base `a0e8504e87e383536f6be7e1a90ee463e93d76c7`. Base is `c8de720b36c24ae8a7ab274ec10c994f258eb336`. Evidence: `/Users/wesleyjinks/prism-evidence/js-param-defs/repair-r2`. All source/test/mutant files are frozen by final-source-binding.json. The committed byte adjudicator and rowdiff remain unchanged. Every base/R1/head capture below was freshly run; no reused captures.

## Byte rows — all requested corpora

| Corpus | ADDED | CORRECT | WRONG | LOST | UNDECIDED |
|---|---:|---:|---:|---:|---:|
| X | 36 | 36 | 0 | 0 | 0 |
| Xi installed | 36 | 36 | 0 | 0 | 0 |
| T | 1,057 | 1,056 | 1 (existing E7) | 0 | 0 |
| SecBench (574/583 admitted) | 827 | 826 | 1 (existing E7) | 0 | 0 |

T and SecBench each retain the exact same ADDED E7 WRONG byte/owner record as R1, checked by full-record set equality. Four T and three SecBench inherited E1 WRONG bindings retain safer Exact→NameOnly(SameLine) relabels. Every admitted actual call-site output is byte-identical. X/Xi each have25 Exact/11 CfgIncomplete gains; T has402 Exact/641 CfgIncomplete/9 Killed/5 SameLine gains; SecBench has405 Exact/351 CfgIncomplete/61 Killed/10 SameLine gains (503 rest/324 bare-arrow). SecBench base/head rows2,127,895/2,128,722. All574 successful triples are also R1→R2 byte-identical; cost_excluded is empty. Two bounded EXACT_PRIOR_WRITE hits remain; `[INHERITED]` R1's full-package plain-formal controls document their E6 parity, and R2 changes neither row.

## S2 — all four gains recovered

The precise cross-file Exact free_single predicate adds exactly the four rows forfeited by R1, with full byte/owner set equality against R1's lost-row receipt. `testRunner/unittests/tscWatch/watchApi.ts`, `afterProgramCreate`, Def `program@479 [23730,23737]`:

| Use line | Record bytes | Checker Use bytes | Verdict / label |
|---:|---|---|---|
| 480 | [23813,23820] | [23813,23820] | CORRECT / Exact |
| 483 | [24015,24015] (collapsed) | [24031,24038] (unanimous) | CORRECT / Exact |
| 487 | [24385,24392] | [24385,24392] | CORRECT / Exact |
| 490 | [24496,24503] | [24496,24503] | CORRECT / Exact |

All four also receive EXACT_OK from the bounded syntax probe; binding CORRECT is not a complete flow proof. Evidence: S2-recovery.json and public/adjudication/T.cost.details.jsonl. X/Xi R1→R2 cost is0; T gains4 with no LOST or UNDECIDED.

## Controls and gates

Python black20,539, Go caddy72,681 and pinned Rust Prism54,150 DFG byte rows are identical to base, as are actual call-site bytes. TS/JS comparison-frame quick is VALID for both TS and Node with unchanged R1 TP/FP/FN counts (12 symbols per stratum). Supplementary pinned Rust quick is also VALID (3 per stratum), with installed rust-analyzer settings disabling build scripts, procedural macros and checks on save before launch, and Cargo offline/locked. The default unconfigured Rust quick is not run because it would permit corpus code execution.

Six w2m/w2d JS/TS/TSX regressions fail on bd4c30ff after Def and Exact FreeSingle assertions pass. Final37 js_param_defs tests pass; frozen CLI controls show base NotReached → R1 Reached → R2 NotReached, with unchanged call sites. NameOnly recovery has an additional bd4c30ff RED; imported/same-file/NameOnly controls pass. E7 JSX attributes and pair keys are checker-WRONG on both plain base and bare head. JS with-body NameOnly rows are plain-base parity. No parity defect is downgraded to SMELL.

Full nextest:5,174 passed/1 skipped; doctests2/2; required_parameter23/23; nested_execution_owner11/11. Scoped advisory mutations30/30 admissible KILLED; serial authoritative PR-A29/29. Fmt clean; same-environment clippy371 warning records each side, added0/removed0. Matrix178/178. Gate receipts and frozen binaries are backed up in gates-snapshot.tgz.

The complete373-row eligible sweep has head120/base113 traced, preserving every base trace; payload-specific BFS head114/base107 also preserves every base-specific witness. All seven new credits (six rest plus port-killer) are payload-specific. The nine targets retain7traced/1partial(portprocesses)/1reached_function_only(is-svg). clean-css/natural remain prism_error on both sides; wind-mvc is partial on both current sides (the original R1 raw runs timed out on both), so this is a current measurement difference, not attributed to R2. Receipts: trace-summary.json, trace-exclusions.json, eligible-{base,head}/entries.jsonl and payload-{base,head}.jsonl. Premature clippy/T snapshots and the sandbox-refused process census are explicitly inadmissible in PROBE-LOG.md; no verdict depends on them. F, independent full CFG/call-ladder soundness, all-lanes mutation and multi-corpus Tier-A remain unverified. No git writes or corpus package execution.


## SecBench producer exclusions

All583 roots/1,749 side jobs were freshly attempted. Exclude the union of failed required base/head producers from both row aggregates, retaining successful partial sides as unpaired evidence, with no binding verdict. Exactly nine pairs excluded:

- command-injection/total.js_3.4.6
- prototype-pollution/total.js_3.4.6
- path-traversal/atropa-ide_0.2.2-2
- redos/cejs_2.0.20170212
- redos/clean-css_4.1.10
- redos/natural_5.1.0
- redos/react-native_0.63.0-rc.0
- redos/three_0.122.0
- redos/vant_2.12.11

Budget300s per producer; exact side/operation seconds and exits are in secbench/excluded.json and per/*.status.json. Base bytes timed out for all nine. Head bytes timed out for eight; React Native head bytes succeeded in286.081s (303,707,443 bytes), but its call-site producer timed out in300.065s, so it remains excluded from both paired aggregates. No partial output was admitted. The original failure set is unchanged, with no retries or budget extensions.

## Export and STOP

N1 uses per-emitted-edge cross-file Exact free_single provenance, not target syntax; the six w2 regressions and PD-29 cover shorthand methods and unimported declared exports. N2 widens E7 to pair keys and JSX attribute names and routes it to PR-C. S3 dispatches committed bd4c30ff plus R2-src.patch, docs a0e8504e plus R2-docs.patch. S4 records JS with-body dynamic-scope plain-base parity next to E8. This is the disclosed targeted fold after review2/2, with no third round/restart.

STOP: none on the measured successful-pair set. LOST correct0, new WRONG outside disclosed E7/E8 zero, UNDECIDED0, non-JS non-identity0, call-site changes0 and trace regressions0. Binding CORRECT does not establish full CFG/call-ladder soundness; the nine failed producer pairs remain unverified.

Frozen files under repair-r2/bin/:prism-head-r2 sha256 bb54ae0b7144dc5539771f06ee82b1693e904a8ed5e47a0b3a5ee13eab6d048d; prism-head-r2-bytes sha256 af1eccc231e2bd4b8f909cbf9d88795df019c69c94870492d0153f7f99dd23a5. All consumers explicitly select those frozen binaries. Later standalone target/release/prism is a distinct auxiliary artifact and is not substituted into any capture/gate receipt.

Source export: R2-src.patch (relative bd4c30ff; exactly src/cpg/build.rs, src/cpg/js_param_defs_tests.rs and mutants/js-param-defs.json), message file commit-message-src.txt. Docs export: R2-docs.patch (relative a0e8504e), message file commit-message-docs.txt. Both receive read-only git apply --check on clean pinned archives. results-summary.json, gates.json, verification-binding-final.json and artifact-manifest.json bind the receipts; snapshots retain custody. Controller owns repository commits and F/adoption; no git writes performed here.


Hook stop:5 follow-up: root VERIFICATION.md now contains `## Verified` and `## Not verified`, explicit commands/totals, behavioral pre-change REDs, edge cases and exact exclusions. No source/test behavior changed; full5174/1skip and doctest2 receipts remain bound to the unchanged722-source manifest. The original r2-final-snapshot.tgz is the full pre-hook snapshot; current documentation/patch/manifest overlay is r2-hook-docs-snapshot.tgz.
