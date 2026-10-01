# S1b-4 r5 implementation repair measurements — 2026-10-01

MEASURED in /Users/wesleyjinks/code/prism-s1b-4-impl, branch feat/s1b-4-namespace,
starting8796dc55 vs main5048f443. Controller positive-proof-only re-scope is
normative; r4 content below is historical. No Git writes, F reads, delegation,
installs or network acquisition. Evidence is target/repair-r1/.

## Binary and source custody

| Artifact | SHA256 |
|---|---|
| Base, built at915fca43 (src verified equal to main5048f443) | d3fc31233253ddfec36f9f623d780c1cc6d9e376f31806ff516965a141974859 |
| Frozen old8796dc55 head, review-owned artifact | a35db4cffa39c5e607283b48690663e2064bfdd611937f8427fd340726e35721 |
| Repaired head8796dc55-dirty | eebe8054eccae7c5a3d07353de9518e1e2da4efc141dc5acf112f5e873daf025 |

MEASURED: BUILD-MANIFEST.json binds270 retained crate inputs plus the repair
verification files. The final release rebuild equals the frozen repaired head
byte-for-byte. CONTROLLER-S1b4.sh selects target/repair-r1/prism-head, checks
104/60 and input/binary hashes; public preflight and shell syntax pass. Private
commands were never executed. Controller must retain ignored evidence and
commit the explicit file list; no clean committed repair binary is claimed.

## Complete corpus and synthetic populations

| Corpus | Base/head sites | Changed | Lost targets | Missing keys |
|---|---:|---:|---:|---:|
| X | 19219/19219 | 0 | 0 | 0 |
| R | 953/953 | 0 | 0 | 0 |
| T | 61712/61712 | 0 | 0 | 0 |
| F | not opened | unknown | unknown | unknown |

MEASURED: direct fresh no-cache dumps on both binaries, complete unique keys
and empty stderr. Site row diffs and independent full target-multiset guards
all show zero changes/losses on X/R/T. No accuracy/performance conclusion is
transferred to private F. The historical four F demotions remain a forecast;
rule7 still preserves all identities, but removed origin grading exceptions
may change its demotion population. The controller reruns the bound new head.

MEASURED: controls411,381 unchanged/30 changed;15 positive identity filters
and15 E7 demotions with edges kept. No new drop/addition. Every changed site is
source-audited in controls-audit.json and enumerated in CONTROLS. Twenty-seven
r4 changes revert to base; four new C246/C247 demotions are disclosed. C62/C80
remain positive filters. Export counters agree on all411 controls. RP46 has
complete keys/inventories, empty stderr and zero changed sections vs base.
The four former RP2-c gains are cut by rule4.

MEASURED:62 both-grammar CLI regression repositories all GREEN;45 behavioral
RED outcomes on frozen old head. These include every WRONG input class from
both implementation reviews, plus positive filters and core cycle pins.
JSX TS-only cases are recovery pins. Test source preservation compares actual
base R3 behavior, not a lexical value-flow guess. Tier-A fixture fresh base has
f@2 andf@5 Exact; repaired head has onlyf@5. Matrix166/166, zero regressions.

## Review closure and precision cuts

| Review finding | Rules | Disposition |
|---|---|---|
| Opus W1(a-c), absence holes | 2/3/6 | CUT-to-base: HOC/default, parenthesized/asserted/assignment callable, import-equals rows preserved |
| Opus W2, file revisit hides depth | 2/3 | CUT-to-base: bounded member walk treats cut/cycle as unknown |
| Opus W3, synthesized candidates | 4/5 | CLOSED: actual base R3 candidates only; skipped sites and escaped specifiers unchanged |
| Opus W4, E7 sibling filtered | 7 | CLOSED: all base targets kept, grade only |
| Sol W1, omitted star competition | 2/4 | CLOSED: unknown/non-callable claim prevents positive unique proof; no minted identity |
| Sol W2, depth competitor | 2/4 | CLOSED: unknown supplying branch invalidates uniqueness |
| Sol W3, written D6/ambient inventory | 3/6 | CUT-to-base: Callable-only terminal gate; separate E5 origin inventory removed |
| Sol W4, class poison cycle | 3/6 | CUT-to-base: no absence authority or non-Callable terminal filtering |
| Sol W5/X6, namespace cycle | 5 | CUT-to-base: private-barrel scan/filter and X6 deleted; both-grammar ordered-cycle row preserves base |
| Opus S1, cache comments | 104/60 | CLOSED: history comments updated, literal pins retained |
| Opus S2, X2/X6 survivors | 5 | CUT: obsolete mutants removed with their mechanisms |

Rule1 qualifier proof is unchanged; all retained qualifier mutants die. Direct
named precedence remains valid because stars cannot supply an explicitly named
member. A whole-module skipped/unrecorded export conservatively keeps base.
Self and mutual initializer rows complete; the landed wrapper-provenance walk
never re-enters classification. No TS merge policy, value-flow lane or shared
D4 resolver/counter behavior was expanded.

## Mutants

MEASURED: final25/25 KILLED,0 SURVIVED,0 INADMISSIBLE. One variant at a time,
source bytes restored in finally. The first pass had23 kills, one depth
survivor and one inadmissible cache setup; both reruns are retained separately.
The named-chain depth twin kills the real depth guard, independently of the
star name-inventory bound. Two stale test-call arguments were corrected before
the cache-pins mutant produced actual failing pinned assertions. No compilation
or empty-test result counts as a kill. Evidence: mutants-final.json and both
mutant directories, with patch/command/log/failed assertions for every variant.

| Variant | Failing permanent tests | Result |
|---|---|---|
| D-M1 | d1_direct_and_directory_decoys | KILLED |
| D-M2 | d4_scope_write_recovery_and_positions | KILLED |
| D-M3 | r1_rule7_e7_only_regrades_all_candidates | KILLED |
| D-M4 | d6_non_namespace_imports_keep_base | KILLED |
| D-M5-import | d9_written_import_and_export_keep_base | KILLED |
| D-M5-kind | d9_written_import_and_export_keep_base | KILLED |
| D-M6-span | d1_direct_and_directory_decoys | KILLED |
| D-M6-wrapped | r1_wrapped_nonjsx_keeps_base_jsx_filters | KILLED |
| D-M8 | d4_scope_write_recovery_and_positions | KILLED |
| D-M9 | d4_scope_write_recovery_and_positions | KILLED |
| D-M14-site | d8_serde_cache_and_incremental_epochs | KILLED |
| D-M14-cache | navigation::call_edge_cache::tests::sidecar_version_is_pinned_for_receiver_authority, cpg_cache::tests::cache_versions_are_pinned_for_cpg_semantics | KILLED |
| R3-jsx-sibling | d14_jsx_specifier_tsx_sibling | KILLED |
| R1-namespace-refused-off | d13_b0_and_nonproving_refusals_keep_base | KILLED |
| R2-recovered-type-admitted | d6_non_namespace_imports_keep_base | KILLED |
| W3-all-unproven-refused | d13_b0_and_nonproving_refusals_keep_base | KILLED |
| P2-incomplete-module | r1_rule2_uncertain_and_competing_stars_keep_base | KILLED |
| P2-ignore-unknown-star | r1_rule2_uncertain_and_competing_stars_keep_base | KILLED |
| P2-depth-unbounded | r1_depth_budget_preserves_decoys | KILLED |
| P2-cycle-is-absence | r1_rule2_uncertain_and_competing_stars_keep_base | KILLED |
| P2-competing-star | r1_rule2_uncertain_and_competing_stars_keep_base | KILLED |
| P4-add-nonbase-target | r1_rule4_never_add_target_or_enter_skipped_r3 | KILLED |
| P4-zero-match-drops | r1_rule4_never_add_target_or_enter_skipped_r3 | KILLED |
| P6-noncallable-terminal | r1_rule6_callable_core_only_and_cycles_bounded | KILLED |
| P7-filter-candidates | r1_rule7_e7_only_regrades_all_candidates | KILLED |

READ: obsolete variants removed: D-M7/D-M10/D-M11/D-M12-named/star/D-M13,
R4/R5/R7/R11, W1/W2 opacity/absence, S1 barrel/cell origin, X1-X6 and all
E5-origin variants. Their r4 receipts stay historical; these mechanisms no
longer exist. P2's five uncertainty mutants, P4's two membership mutants,
P6's terminal mutant and P7's candidate filter mutant are new; D-M3 kills an
E7 Exact-grade regression. All are killed by behavioral assertions.

## Suites, cache and exclusions

| Check | Passed | Failed | Ignored |
|---|---:|---:|---:|
| Default,29 groups | 4781 | 0 | 1 |
| MCP,31 groups | 4974 | 0 | 1 |
| Namespace final matrix | 31 | 0 | 0 |
| Tier-A matrix | 166 | 0 | 0 |

MEASURED: fmt, split-test rustfmt, release build and grammar closure pass.
Grammar186 named/76 suspect/0 unclassified; E_TABLE74 rows,0 missing/extra/diff.
Clippy --all-targets --features mcp completes; same-environment base control
has the same37 normalized warning instances in touched files,0 new. It is not
a globally warning-clean claim. The sole initial full-suite failure was the
owned esm_namespace_import pin; its source-equivalent base CLI row is a Gap.
The pin reverts to Gap under rule4; final suites pass. The only ignored Rust
row is resolution_test::slice_elem_variant_reserved. Three helper declarations
were relocated after the suites to keep the umbrella file599 lines; final
namespace matrix31 and formatting pass, with no helper body, fixture or assertion change. The verification-hook then
reran both full suites on the final helper layout:4781/0/1 default and4974/0/1
MCP, with no outside-scope failures. Root VERIFICATION.md records exact commands,
behavioral RED/negative coverage and exclusions; hook-suites.json retains totals.

MEASURED: cross-process CPG103->104 and sidecar59->60 metadata transition;
cold/warm repaired dumps and callers equal fresh no-cache output, onlyf@5.
Old-head104->repaired104 also refreshes a changed private-barrel row and warm
matches fresh. Build identity differs too; no version-only causality claim.
The first cache full comparison was inadmissible (conflicting CLI flags),
corrected and preserved under cache-check-attempt1. cache-check/receipt.json
and cache-same-epoch/receipt.json carry actual metadata and artifact comparisons.

MEASURED exclusion: one300s foreground Tier-A quick attempt after an immediate
release rebuild ended deadline_no_complete_result at300.15s (returncode-2).
No complete quick/oracle artifact, no retry, no GREEN or performance attribution.
The source-bound166-row matrix is the completed accuracy subset. Query/oracle
behavior was unchanged; reports/cache/generated snapshot were routed or moved
to target/repair-r1/quick. A full quick run was not verified. Private F, clean committed
repair binary, full multi-corpus Tier-A and Node gate are not verified locally.
Full multi-corpus is human-triggered; F is explicitly forbidden. Node has no
changed path in this repair; Rust default/MCP are the requested full suites.
The final handoff records quick's actual completion/exclusion before handback.

## Historical r4 measurements (superseded for current implementation)

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
