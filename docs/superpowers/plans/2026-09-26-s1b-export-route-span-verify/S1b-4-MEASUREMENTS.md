# S1b-4 r4 measurements — final spec-review r2 fold, 2026-10-01

MEASURED: plan/s1b-4 @922f00df, prototype proto/s1b-4 @beec4a23 plus
final owned fold. Base915fca43. No Git writes, F opens, installs, providers or
network acquisition. Opus2/2 is converging; final targeted fold, no new plan
round, restart or E5 exception. All r2/r3 references are historical.

## Custody and cumulative starting point

MEASURED: BUILD-MANIFEST.json binds18 owned files,270 crate inputs, dirty
build identity and cumulative/fold patches. Controller squashes
`915fca43..<final proto>` into one commit on `proto/s1b-4-final`, fills its SHA
in IMPLEMENTOR/manifest, and dispatches that one cumulative starting point.
No incremental child is the adoption artifact. COMMIT-FILES lists exact paths.

| Artifact | Identity | SHA256 |
|---|---|---|
| base/prism | slicing 3.1.2 (915fca43d84e) | d3fc31233253ddfec36f9f623d780c1cc6d9e376f31806ff516965a141974859 |
| head/prism-r4-final | slicing 3.1.2 (beec4a23cdfa-dirty) | 03c5e6ce67a01e625afa0687f91aeb2ba7695f39347ed747ff44d9efe2426e07 |
| cumulative patch | proto-r4.diff | 0bb693a10d4ccb427b52151bc55c8cc618ed28b7edf0e61c3061c0f4ab080be4 |
| final fold | proto-fold-r2.diff | be4955b45c3f3ab922475f169c13a089ea7bb6cba8e76ab022459680d703d97a |

MEASURED: CONTROLLER-S1b4.sh selects the r4 binary, beec4a23 source identity
and r4 round. Syntax and public hash preflight pass; private commands were
never executed. BUILD-MANIFEST-r3.json retains the original r3 manifest.
Prototype cache pins106/62 are retained as controller-scoped iteration pins;
landed cache bump remains once103/59 ->104/60. No cache claim is transferred
from these dirty prototype bytes to a clean implementation commit.

## Fresh X/R/T row diffs

MEASURED: freshly rerun base and head direct dumps,1200s bound per command;
complete unique keys, empty stderr, binary-hash/time receipts retained under
base/dumps-r4 and head/dumps-r4. rowdiff plus independent valueflow guards
retain complete target multisets and site keys.

| Corpus | Base/head sites | Changed | Removed/retargeted/added/demoted/relabeled/accepted-cost | Lost IDs | Missing keys |
|---|---:|---:|---|---:|---:|
| X |19219/19219|0|0/0/0/0/0/0|0|0|
| R |953/953|0|0/0/0/0/0/0|0|0|
| T |61712/61712|0|0/0/0/0/0/0|0|0|
| F r4 |not run, controller-only|unknown|unknown|unknown|unknown|

MEASURED: all public diffs equal retained r3[]; no r4 expected files created.
No changed public row needs a new lexical audit. Equality does not certify the
retained base edges as right. Historical F aggregates do not certify r4.

## Controls, replay and accounting

MEASURED:411 controls,358 identical/53 changed against same-environment base,
complete keys/inventories and empty stderr. Original397 whole sections equal
r3, and722 old generated source files remain byte-identical. All14 new
C245-C251 twins equal whole base sections. Twelve new namespace rows differ
from r3; C245's escape twins already kept base. Every changed base-to-head row
is individually explained in CONTROLS; no changed class is added by this fold.
Preregistered columns checked in head/registered-controls-r4.json; pre-change
r3 and final/base new namespace rows retained in final-fold-control-rows-r4.json.
RP46 head sections equal r3; original base-to-head RP2-c four changes remain
unchanged. Export-counter maps on413 repositories (411 controls +X/R) agree,
including all original js_export_* counters. T aggregate counters were not
remeasured; complete T site dumps were.

## Full mutant table

MEASURED:39 executable variants,37 KILLED/2 SURVIVED/0 INADMISSIBLE.
One mutation at a time, one180s attempt each. Failed assertion output and
selected permanent tests were inspected; no compile/setup/zero-test failure
counted as a kill. Restoration receipt binds18 owned/270 crate inputs exactly.
Results, patches and logs: head/mutants-r4. X1 fails d15's non-private premise;
E5 independently preserves its escape row, so row-only assertion would mask
that premise mutant. X3 also has premise failures; table lists actual failed
permanent tests. A projection-map absence assertion is the immediate D-M12
kill; no compilation failure is involved.

| Variant | Actual failing permanent tests | Result |
|---|---|---|
| D-M1 | d1_direct_and_directory_decoys | KILLED |
| D-M2 | d4_scope_write_recovery_and_positions | KILLED |
| D-M3 | d5_authoritative_missing_member_and_fallback | KILLED |
| D-M4 | d6_non_namespace_imports_keep_base | KILLED |
| D-M5-import | d9_written_import_and_export_keep_base | KILLED |
| D-M5-kind | d9_written_import_and_export_keep_base | KILLED |
| D-M6-span | d1_direct_and_directory_decoys | KILLED |
| D-M6-wrapped | d2_wrapped_call_and_jsx | KILLED |
| D-M7 | d3_rename_named_and_star_barrels | KILLED |
| D-M8 | d4_scope_write_recovery_and_positions | KILLED |
| D-M9 | d4_scope_write_recovery_and_positions | KILLED |
| D-M10 | d4_scope_write_recovery_and_positions | KILLED |
| D-M11 | d5_authoritative_missing_member_and_fallback | KILLED |
| D-M12-named | d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| D-M12-star | d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| D-M13 | d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| D-M14-site | d8_serde_cache_and_incremental_epochs | KILLED |
| D-M14-cache | navigation::call_edge_cache::tests::sidecar_version_is_pinned_for_receiver_authority, cpg_cache::tests::cache_versions_are_pinned_for_cpg_semantics | KILLED |
| R3-jsx-sibling | d14_jsx_specifier_tsx_sibling | KILLED |
| R5-opaque-fallback-skipped | d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| R7-skipped-maycall-not-opaque | d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| W1-incomplete-absence-final | d11_incomplete_exports_and_depth_keep_base | KILLED |
| W2-pattern-opacity-omitted | d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| W3-all-unproven-refused | d13_b0_and_nonproving_refusals_keep_base | KILLED |
| S1-barrel-decoy-kept | d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| S1-opaque-cell-is-not-function-origin | d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| R1-namespace-refused-off | d13_b0_and_nonproving_refusals_keep_base | KILLED |
| R2-recovered-type-admitted | d6_non_namespace_imports_keep_base | KILLED |
| R4-wrapped-fallback-final | d5_authoritative_missing_member_and_fallback | KILLED |
| R11-resolved-opaque-dropped | d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| X1_private_barrel_any_stmt | d15_executable_barrel_escape_keeps_exact | KILLED |
| X2_private_barrel_ignores_local_opaque | 22 namespace tests passed | SURVIVED |
| X3_private_barrel_allows_local_export | d16_e5c_star_written_keeps_base, d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| X4_cycle_not_final | d10_barrel_conflict_cycle_final_depth_keeps_base, d11_incomplete_exports_and_depth_keep_base | KILLED |
| X5_no_depth_cut | d10_barrel_conflict_cycle_final_depth_keeps_base, d11_incomplete_exports_and_depth_keep_base | KILLED |
| X6_export_decl_allowed | 22 namespace tests passed | SURVIVED |
| E5-origin-omitted | d16_e5b_bare_written_keeps_base, d16_e5e_named_maycall_keeps_base, d16_e5c_star_written_keeps_base, d16_e5a_bare_maycall_keeps_base | KILLED |
| E5-origin-bypassed | d16_e5b_bare_written_keeps_base, d16_e5e_named_maycall_keeps_base, d16_e5c_star_written_keeps_base, d16_e5a_bare_maycall_keeps_base | KILLED |
| E5-written-kind-projection-omitted | d17_written_kinds_keep_base | KILLED |

READ: X2 is a redundant conjunct survivor: top-level export declarations/
values and executable CJS expressions cannot be private; local named opacity
claims also fail the all-ReExport/ImportForward kind guard. Removing only
the opaque-map emptiness conjunct cannot admit those forms. X6 is a disclosed
coverage survivor, not equivalent: export namespace/enum bodies plus a cycle
can expose a private function if export declarations are admitted. No cheap
runtime test row was added for S7. All prior30 mutants remain killed.
Historical non-executable/equivalent variants R6/R8/R9/R10 and removed dead R12
remain as recorded in r3; no new execution claim for those variants.

## Suites and harnesses

| Check | Passed | Failed | Ignored | Evidence |
|---|---:|---:|---:|---|
| default,29 groups |4772|0|1|head/default-r4.log|
| mcp,31 groups |4965|0|1|head/mcp-r4.log|
| namespace permanent matrix |22|0|0|head/spec-r2-green.log|
| Tier-A matrix |166|0|0|head/tier-a-matrix-r4.log|
| Node gate |853|0|1|head/node-gate-r4/receipt.json|

MEASURED: source-bound RED on beec4a23: four E5 assertions fail; all primary
E5 cases have both-grammar CLI RED/base controls as well. Written class direct
RED is retained separately. d15 is a mutant guard, not a claimed pre-change
behavior regression. d18 verifies bincode/serde defaulting and incremental
MayCall ->Alias ->MayCall replacement. The first default attempt's own two
new-test expectation/layout failures were corrected in place; it is history,
not final acceptance. Hypothesis/probe/alternatives and inadmissible harness
setup attempts are recorded in PROBE-LOG-r4.md.

MEASURED: source-restored rebuild has the same bound binary bytes; Tier-A
matrix166/0. cargo fmt and separate split-test rustfmt checks pass. Clippy
completes with warnings (not warning-clean); no base clippy control was run, so
no warning regression/inheritance attribution is made. Grammar closure186 named
kinds/76 suspect/0 unclassified,74 E_TABLE positions,0 missing/extra/different.
Node853/0/1 uses existing cached inputs, no acquisition; excluded file:
docs/eval/receiver-closure/audit-imported-props-source.test.mjs, reason
inputs-not-reconstructible (compressed alias binding ordinal). The Rust ignored
row is resolution_test::slice_elem_variant_reserved.

MEASURED exclusion: one fresh source-bound quick attempt ends
deadline_no_complete_result after300.09s (bound300s,returncode-2); no complete
quick/oracle artifact and no retry. Prior three incomplete r3 attempts remain
historical exclusions. No performance regression is attributed without a
same-environment base quick control. Full multi-corpus Tier-A is human-triggered
and not run. T aggregate export counters, clean parent103/59 -> final104/60
whole-process warm cache transition, clean committed binary, private F and a
new independent post-fold review were not verified. CPG serde/incremental and
cold/partial graph-cache checks are covered by permanent tests, not substituted
for whole-process transition acceptance.

MEASURED: final source/packet snapshots, hashes, current manifest/patches and
verification receipts are checked by final_custody_r4.py and FINAL-CHECK-r4.json.
MANIFEST-r4.sha256 inventories the retained evidence. Legacy crate_input_digest
was removed from the current manifest; current canonical crate_input_map_sha256
and every input hash bind all270 inputs. No reports/snapshots remain in proto.

## Size and dispositions

MEASURED: nonblank non-// cumulative diff against915fca43 after rustfmt:
source528 added/19 removed/509 net; tests/fixtures833 added/6 removed/827 net.
size-r4.json includes the untracked split test file. Namespace files600 and209
physical lines. Forecast528-600 added source /833-925 added tests, no caps.
Generated controls, packet scripts and evidence excluded. No new slice/restart.

| Finding | Disposition |
|---|---|
| W5 WRONG MATERIAL | Escape twin d15/C245 both grammars keeps lib:f Exact; explicit non-private premise guards X1 (KILLED) |
| W6 WRONG E5 IMMATERIAL | E5 as written: raw terminal MayCall/write origin, namespace-only projection, full base before E7/private filtering; E5a/b/c/e plus kinds/epochs guards; no exception |
| S6 SMELL IMMATERIAL | One cumulative squash on proto/s1b-4-final; controller fills SHA |
| S7 SMELL IMMATERIAL | X6 export-declaration-admitted disclosed survivor: TS namespace/enum plus cycle; no cheap runtime row added |
| all r1 items | CLOSED per Opus r2; original397 expectations preserved |

Questions the owner owes an answer to: None. Operational controller custody
and private F are pending; no semantic exception is recommended.

## Controller F acceptance r4 (final plan fold; aggregates only; 2026-10-01)

- Prototype: `298006b3` (cumulative `19bbbb1e`) vs `915fca43`. Sites: 13,299 on base and on head.
- Changed rows: **4**, Exact → NameOnly `import_qualified`, edge kept.
- Lost targets: 0. Export counters changed: 0.
- The result is identical to r2 and r3.
- Spec review: Opus r1 FIX 4W/5S folded; r2 (cap) FIX 2W/2S converging, folded by the planner and verified by the controller, with no third plan round.
