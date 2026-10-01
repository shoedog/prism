# S1b-4 r3 measurements — spec-review r1 fold, 2026-10-01

MEASURED: the prototype and packet fold W1–W4 / S1–S5 in place. No Git writes,
F reads, installs, providers or network acquisition were performed. No owner
question remains. Opus spec r1 consumed round1 of2; no independent post-fold
review has been dispatched. Local verification stayed within its declared
three-attempt cap; the third pass narrowed a closed opaque-cell origin defect.
R1's bounded follow-up closed an enumerated coverage gap; no restart occurred.

## Custody and build

MEASURED: plan/s1b-4 @564ebc8c, prototype proto/s1b-4 @fceb0b4e plus the owned
fold; source base915fca43d84ea1730959453091fbf8ae97763af8. Eight owned files are
dirty relative to fceb0b4e; the cumulative body owns17 files. Controller freezes
a cumulative source commit for adoption off main; Sonnet starts from those owned
files and completes tests/RED/verification/handback. COMMIT-FILES-s1b4.md names
all18 packet paths, eight fold paths and17 cumulative prototype paths.

| Artifact | Version | SHA256 |
|---|---|---|
| base/prism | slicing 3.1.2 (915fca43d84e) | d3fc31233253ddfec36f9f623d780c1cc6d9e376f31806ff516965a141974859 |
| head/prism-r3-final | slicing 3.1.2 (fceb0b4e75a8-dirty) | c4f3d25a7095a11fcb4b70716f38291dc4923954755cfd20ed6c53b4cc1a7371 |

MEASURED: BUILD-MANIFEST.json binds every owned file and tracked crate input,
actual dirty build identity, cumulative patchSHA **4c913ab7b14688b50861af010f9a0b6915548b2814dbeb16690acefab06dd18b**
and fold patchSHA **abf87287315e7527c83e4bb153bf06819f477f5711001d7b2bbbd6adf035aa26**. The final rebuild has identical
binary bytes to the final binary used for r3 corpus/control measurements. The
final private-barrel correction is source-bound separately from preceding stages. Its actual command is head/build-r3-restored.log.
BUILD-MANIFEST-r2.json and r2 references remain unchanged. Earlier r3 stages
are retained as BUILD-MANIFEST-r3-before-using.json and
BUILD-MANIFEST-r3-before-opacity-proof.json with their patches.
The unbound copied-inode SIGKILL is inadmissible; the fresh bound inode runs.
Cache versions are106/62. There are no stray reports/snapshots in the proto tree.
Final source/packet snapshots and SHA inventory are under target/plan-s1b4.

## Fresh public row diffs and classes

MEASURED: freshly run base and head dumps, full unique populations, empty stderr;
rowdiff, independent target multiset/key guard and audit outputs under
base/dumps-r3, head/dumps-r3 (X/R), head/dumps-r3-direct (T) and head/{X,R,T}-{rowdiff,audit,valueflow}-r3.*.

| Corpus | Base / head sites | Changed | Removed / retargeted / added / demoted / relabeled / accepted-cost | Lost IDs | Missing keys |
|---|---:|---:|---|---:|---:|
| X | 19,219 /19,219 |0|0/0/0/0/0/0|0|0|
| R |953 /953|0|0/0/0/0/0/0|0|0|
| T |61,712 /61,712|0|0/0/0/0/0/0|0|0|
| F r3 | controller-only, not measured | unknown | unknown | unknown | unknown |

MEASURED: probes/expected/S1b-4-r3-{X,R,T}.json are fresh[]; r2 retained.
Zero lost identities proves zero public target IDs lost without certifying that
retained base targets are right. No uncertified changed public row remains. The combined dump wrapper
reached300 seconds on T and is inadmissible; direct T retry finished
in255.57 seconds with a1200-second bound, returncode0 and complete rows.
An auditor invocation under Python3.12 lacked tree_sitter and was inadmissible;
the existing system Python dependencies produced the accepted audit. Neither
setup failure updated resolver beliefs or was attributed to the change.
Audit cannot follow alias returns, parameters, object members or runtime writes;
it never certifies a removed edge from qualifier non-callability alone.

INHERITED: controller's r2 F acceptance (prior committed measurement record):
13,299 equal sites, four Exact→NameOnly/import_qualified changes with edges kept,
zero lost IDs, zero counter differences. It does not certify r3. Updated
CONTROLLER-S1b4.sh selects prism-r3-final, verifies binary and all source/input
hashes, and preserves full command/completeness/audit/identity receipts privately.
Script syntax and public custody preflight pass; private F was never invoked.

## Controls, replay and telemetry

MEASURED:397 generated controls,344 identical sections /53 changed versus
base; original349:309/40; new48:35/13. Pre-fold comparison:358 identical /39
changed. All keys and complete function inventories agree; stderr0. All349
historical generated source repositories remain byte-identical. Every registered
column is asserted by registered-controls-r3.json; CONTROLS enumerates every
changed row and all new preservation columns. C129/C210/C201 are early-SyntaxError
pins, outside reachable behavior. TS-only syntax in JSX is only recovery.

MEASURED: supplementary R1 using probes have both grammars. Valid TSX base
wrong Exact→UnknownName; JSX recovery drops on both. d13 permanently proves
not_callable with both legacy receiver flags false. These two scope probes are
separate from the397 generated repositories. Six more opacity-origin twins
prove renamed resolved Exact preservation, renamed bare E7 NameOnly and cyclic
exported-rootFn preservation. Same-environment base/first-fold/final rows and
independent Node function identity are retained under opacity-origin; d10/d12
RED13/2 then GREEN15/0. The broad cell-file/name filter was refuted in place. No wrong target is accepted here.

MEASURED: RP46 complete sections, four RP2-c twins UnknownName→Exact to the
renamed exported terminal, other42 unchanged. Whole r3 summary equals r2 byte
for byte. All46 full function inventories/keys/stderr agree, including fixed
no-call fixtures; the generic empty-dump comparator failure was inadmissible.
References: probes/S1b-controls-s1b4-r3-proto.txt and S1b-replay-s1b4-r3-proto.txt.

MEASURED: every js_export_* counter agrees on399 repositories (all397 controls
plus X/R); head/export-counters-r3/comparison.json contains complete maps.
T aggregate counters were not re-measured (the r2 base probe hit180 seconds);
complete T site dumps did finish. Qualified MayCall/Position maps remain the
intended accounting; unqualified and D4 facts/counters are preserved.
INHERITED: C220's independent Node source-identity experiment remains r2 evidence;
this turn replays its edge but does not re-execute that experiment.

## Full mutant table

MEASURED: **30 distinct executable variants KILLED** by actual permanent-suite
assertions, not setup/compile/zero-test failures. One mutation at a time,180-second
per-run bound, exact-byte restoration. Final receipts are head/mutants-r3-final/results.json and per-case logs/patches.
All30 ran against the final production/test body. Earlier25 and reviewer
follow-up receipts are retained as history, including first R1 survival. R1 initially survived
and was corrected with the valid using row, never inferred equivalent.
Full suites validate the final semantic body after the using and private-barrel
corrections; afterward one blank line was removed to keep the test file under600.
All final mutants compile that identical semantic body. Source
hash restoration and the final source-bound manifest/snapshots were checked.

| Variant | Actual killing permanent test | Result |
|---|---|---|
| D-M1 | js_binding_namespace_test::d1_direct_and_directory_decoys | KILLED |
| D-M2 | js_binding_namespace_test::d4_scope_write_recovery_and_positions | KILLED |
| D-M3 | js_binding_namespace_test::d5_authoritative_missing_member_and_fallback | KILLED |
| D-M4 | js_binding_namespace_test::d6_non_namespace_imports_keep_base | KILLED |
| D-M5-import | js_binding_namespace_test::d9_written_import_and_export_keep_base | KILLED |
| D-M5-kind | js_binding_namespace_test::d9_written_import_and_export_keep_base | KILLED |
| D-M6-span | js_binding_namespace_test::d1_direct_and_directory_decoys | KILLED |
| D-M6-wrapped | js_binding_namespace_test::d2_wrapped_call_and_jsx | KILLED |
| D-M7 | js_binding_namespace_test::d3_rename_named_and_star_barrels | KILLED |
| D-M8 | js_binding_namespace_test::d4_scope_write_recovery_and_positions | KILLED |
| D-M9 | js_binding_namespace_test::d4_scope_write_recovery_and_positions | KILLED |
| D-M10 | js_binding_namespace_test::d4_scope_write_recovery_and_positions | KILLED |
| D-M11 | js_binding_namespace_test::d5_authoritative_missing_member_and_fallback | KILLED |
| D-M12-named | js_binding_namespace_test::d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| D-M12-star | js_binding_namespace_test::d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| D-M13 | js_binding_namespace_test::d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| D-M14-site | js_binding_namespace_test::d8_serde_cache_and_incremental_epochs | KILLED |
| D-M14-cache | cpg_cache::tests::cache_versions_are_pinned_for_cpg_semantics, navigation::call_edge_cache::tests::sidecar_version_is_pinned_for_receiver_authority | KILLED |
| R3-jsx-sibling | js_binding_namespace_test::d14_jsx_specifier_tsx_sibling | KILLED |
| R5-opaque-fallback-skipped | js_binding_namespace_test::d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| R7-skipped-maycall-not-opaque | js_binding_namespace_test::d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| W1-incomplete-absence-final | js_binding_namespace_test::d11_incomplete_exports_and_depth_keep_base | KILLED |
| W2-pattern-opacity-omitted | js_binding_namespace_test::d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |
| W3-all-unproven-refused | js_binding_namespace_test::d13_b0_and_nonproving_refusals_keep_base | KILLED |
| S1-barrel-decoy-kept | js_binding_namespace_test::d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| S1-opaque-cell-is-not-function-origin | js_binding_namespace_test::d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED |
| R1-namespace-refused-off | js_binding_namespace_test::d13_b0_and_nonproving_refusals_keep_base | KILLED |
| R2-recovered-type-admitted | js_binding_namespace_test::d6_non_namespace_imports_keep_base | KILLED |
| R4-wrapped-fallback-final | js_binding_namespace_test::d5_authoritative_missing_member_and_fallback | KILLED |
| R11-resolved-opaque-dropped | js_binding_namespace_test::d12_pattern_alias_and_skipped_maycall_and_bare_alias | KILLED |

| Non-executable or equivalent reviewer variant | Mechanism proof / disposition |
|---|---|
| R6 insert_named opacity poison | Equivalent on valid programs: competing duplicate exports are early errors |
| R8 sibling basename guard | Equivalent on valid indexed inputs: the exact replacement candidate has resolver precedence; unsupported mts/cts are not indexed |
| R9 program scope check | Equivalent: exact declaring-node identity must equal a top-level inventory import |
| R10 duplicate opacity insertion | Equivalent on valid programs: repeated export/binding claims are early errors |
| R12 resolved NameOnly branch | Removed; span projection admits one identity or none, so the branch was dead |

READ: these equivalents are recorded as the supplied controller disposition,
checked against their mechanisms; no failed test or runtime kill is claimed.
.mjs→.mts and .cjs→.cts arms remain unavailable through current indexing.

## Final suites and harnesses

MEASURED: final unmutated prototype commands, after the using guard and private-barrel proof, with actual
behavioral output inspected. All default/mcp tests were rerun on the final body.

| Check | Passed | Failed | Ignored / skipped | Evidence |
|---|---:|---:|---:|---|
| cargo test --offline --no-fail-fast,29 groups |4,765|0|1|head/tests-default-r3-opacity.log|
| cargo test --offline --features mcp --no-fail-fast,31 groups |4,958|0|1|head/tests-mcp-r3-opacity.log|
| Namespace permanent matrix |15|0|0|final integration group|
| Tier-A matrix, immediate preceding rebuild |166|0|0|head/tier-a-matrix-r3-restored.log|
| Node gate |853|0|1|head/node-gate-r3-final/receipt.json and log.txt|

MEASURED: fmt and source/packet diff checks pass. Clippy completes with warnings;
head/clippy-r3-opacity.log is not warning-clean. The opaque helper
uses next_back after the final correction; remaining warnings are reported,
without regression/inheritance attribution to an unrerun base clippy control.
Unrelated warnings were not repaired.
Grammar closure:186 named kinds,76 suspect,0 unclassified;74 E_TABLE positions,
zero missing/extra/different (head/grammar-closure-r3.log).

MEASURED: Node uses existing validated cached inputs, no network acquisition.
Its skip is RED adapter exposes compressed alias binding ordinal; excluded file
is docs/eval/receiver-closure/audit-imported-props-source.test.mjs, reason
inputs-not-reconstructible. Node gate was rerun after the final source corrections and mutant restoration.

MEASURED: Tier-A quick has three300-second attempts: two on preceding r3
bodies and one source-bound attempt on the final106/62 body. All ended
 deadline_no_complete_result, returncode-2, without a complete artifact; none
is GREEN. head/quick-r3{,-retry,-final}/receipt.json retains them. The final
receipt binds the binary/patch/cache hashes. After the final production change,
one last attempt was necessary to avoid borrowing old-source verification;
at the declared third quick attempt the repeated incomplete harness is parked.
The final attempt ran before the Node gate and after the other heavy checks.
Resource contention is not established, and no regression/inheritance attribution
is made without a same-environment base quick control. uv was not retried after
its known sandbox refusal; installed Python3.12 invokes tier_a.cli.main with
normal queries/oracle/grades and only a writable nav cache. Generated inventories
are under evidence, never adopted as snapshots or docs/eval baselines.

MEASURED: d8 covers graph serde/defaults, disk cold/full/partial CPG cache and
incremental per-file export/qualifier/write changes, plus two positions in one
file;106/62 pins pass. NOT VERIFIED: whole-process parent103/59→r3 106/62 clean
warm-sidecar transition, since the authorized body remains dirty and Git writes
are controller-only. Prior r2 transition receipts are historical, not r3 credit.
The ignored Rust row is resolution_test::slice_elem_variant_reserved, a reserved
SliceElem path. No real-corpus lost target or wrong Exact was inferred from
suite exit status. The new Tier-A fixture has direct same-environment RED/GREEN:
base includes wrong util:f@2 and right util:f@5; final head keeps only f@5
(head/tier-a-fixture-red-green-r3/receipt.json).

## Size and dispositions

MEASURED: nonblank non-// diff against915fca43, after rustfmt; test helpers,
tests/** and fixtures counted separately. Source **486 added /
19 removed /467 net**; tests/fixtures **629
added /6 removed /623 net**. Full per-file
size-r3.json. Namespace test file599 physical lines. Generated repositories,
packet scripts and evidence excluded. Forecast:486–560 added src and
629–750 added tests, not caps; no restart or new slice proposed.


| Review item | Disposition |
|---|---|
| W1 WRONG | Fixed: ESM-complete/untruncated absence; CJS/TS/depth controls preserve base; true ESM absence/conflict/cycle remains final |
| W2 WRONG | Fixed: every pattern name gets namespace-only opacity, both object/array controls |
| W3 WRONG | Fixed: only proved non-import bindings refuse base R3; valid B0 and named twin preserve; using row kills guard deletion |
| W4 WRONG | Fixed: permanent bare-Alias and skipped-MayCall rows kill R5/R7, both in dispatch |
| S1 SMELL | Applied existing rules: non-escaping private-barrel decoy removed; opaque binding cell never proves origin/name; original fallback edges kept at E7 NameOnly |
| S2 SMELL | Relabelled C129/C210/C201 early SyntaxErrors outside runtime argument |
| S3 SMELL | JSX→TSX test kills R3; dead NameOnly removed; R6/R8/R9/R10 equivalents, R12 removed |
| S4 SMELL | Current fceb0b4e plus fold and cumulative17-file adoption; implementer starts prototype body |
| S5 SMELL | Recorded as inherited D4 function/namespace merge policy; no classifier change |

READ: no semantic owner question remains. Controller commits packet/prototype,
preserves ignored evidence and runs private F r3; spec round2 may then review
the revision-bound artifact. No post-fold independent approval is claimed.
NOT VERIFIED: private F r3, full human-triggered multi-corpus Tier-A, complete
quick/oracle acceptance, a same-environment base quick attribution control,
T aggregate telemetry, clean warm-sidecar transition, and Git publication/adoption.
No buildable implementation step is parked; excluded controller/harness work is
explicit. All three quick attempts are retained as exclusions, never silently skipped.
