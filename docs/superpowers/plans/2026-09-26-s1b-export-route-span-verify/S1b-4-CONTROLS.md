# S1b-4 r5 implementation repair controls — 2026-10-01

Current authority: SPEC §0, Controller re-scope (impl r1, open-class).
MEASURED: 411 scenarios; **381 identical / 30 changed** versus source-equivalent
base915fca43 / main5048f443, run in this environment. Complete unique site keys,
function inventories and empty stderr are checked. The thirty changed sites are
**15 positive identity filters / 15 E7 demotions**, with no added identities or
new drops. Export counters match on all411 repositories. Current reference is
`probes/S1b-controls-s1b4-r5-impl.txt`; the r4 reference is retained unchanged.
Evidence: target/repair-r1/{controls-comparison,controls-audit,controls-forgone}.json.

## Every changed control section

Each source, complete old/new site tuple and explanation is retained in
controls-audit.json. The following inventory is exhaustive.

| Section | Class | Reason |
|---|---|---|
| C131_E7_bare_namespace_jsx | E7 regrade only | bare specifier unresolved |
| C131_E7_bare_namespace_tsx | E7 regrade only | bare specifier unresolved |
| C132_E7_nonsibling_stem_jsx | E7 regrade only | non-sibling stem unresolved |
| C132_E7_nonsibling_stem_tsx | E7 regrade only | non-sibling stem unresolved |
| C148_E7_js_specifier_sibling | Positive identity filter | exact indexed TS sibling exported f@7-9 |
| C190_ns_direct_decoy_jsx | Positive identity filter | direct exported f@5-7 excludes nested f@2 |
| C190_ns_direct_decoy_tsx | Positive identity filter | direct exported f@5-7 excludes nested f@2 |
| C191_ns_directory_decoy_jsx | Positive identity filter | caller-relative exported lib:f excludes other/lib |
| C191_ns_directory_decoy_tsx | Positive identity filter | caller-relative exported lib:f excludes other/lib |
| C205_ns_nested_closure_jsx | Positive identity filter | closure qualifier still names program import; exported f@5-7 |
| C205_ns_nested_closure_tsx | Positive identity filter | closure qualifier still names program import; exported f@5-7 |
| C207_ns_inner_write_does_not_reach_import_jsx | Positive identity filter | write in a different parameter scope does not reach namespace import |
| C207_ns_inner_write_does_not_reach_import_tsx | Positive identity filter | write in a different parameter scope does not reach namespace import |
| C216_ns_nonsibling_stem_jsx | E7 regrade only | non-sibling stem; both candidates retained |
| C216_ns_nonsibling_stem_tsx | E7 regrade only | non-sibling stem; both candidates retained |
| C217_ns_e7_sibling_jsx | E7 regrade only | JSX .js specifier has no supported sibling; both candidates retained |
| C217_ns_e7_sibling_tsx | Positive identity filter | TSX exact sibling f@5-7; JSX has no admitted .js replacement |
| C232_bare_alias_e7_jsx | E7 regrade only | bare Alias; edge retained |
| C232_bare_alias_e7_tsx | E7 regrade only | bare Alias; edge retained |
| C242_nonsibling_opaque_e7_jsx | E7 regrade only | non-sibling Alias; edge retained |
| C242_nonsibling_opaque_e7_tsx | E7 regrade only | non-sibling Alias; edge retained |
| C246_e5a_bare_maycall_jsx | E7 regrade only | E7 applies to bare MayCall; no origin inventory exception |
| C246_e5a_bare_maycall_tsx | E7 regrade only | E7 applies to bare MayCall; no origin inventory exception |
| C247_e5b_bare_written_jsx | E7 regrade only | E7 applies to bare written export; no origin inventory exception |
| C247_e5b_bare_written_tsx | E7 regrade only | E7 applies to bare written export; no origin inventory exception |
| C62_S1b_namespace_decoy | Positive identity filter | exported Island@6-8 excludes nested Island@3 |
| C80_R3_namespace_function_decoy_jsx | Positive identity filter | exported f@5-7 excludes nested f@2 |
| C80_R3_namespace_function_decoy_tsx | Positive identity filter | exported f@5-7 excludes nested f@2 |
| C81_R3_namespace_js_extension_decoy_jsx | Positive identity filter | relative/exact TS sibling f@5-7 excludes nested f@2 |
| C81_R3_namespace_js_extension_decoy_tsx | Positive identity filter | relative/exact TS sibling f@5-7 excludes nested f@2 |

## Disclosed precision forgone

27 prior changed sections now equal base. This is an authorized cut, not a
new wrong-output claim: base's decoys and gaps remain deliberately visible.

| Both-grammar controls unless stated | Why restored |
|---|---|
| C133, C215, C241 | rule3: absence never changes a row |
| C192, C193, C83 | rule4: the proven renamed/out-of-stem terminal was outside base candidates |
| C206, C208 | rule4: base skipped R3 in these evaluation positions |
| C214, C82 | rule4/6: wrapped non-JSX keeps base; no drop |
| C234-C236 | rule5: private-barrel exclusion deleted |
| C244 TSX | rule6: refused TS merge terminal keeps base |

C246/C247, four sections, newly regrade Exact to NameOnly under rule7; every
edge stays. The separate E5 origin inventory and its E7 exceptions were cut.
C216 and C217 JSX also retain their nested candidates while regrading, rather
than the prior export-filtered fallback. C62/C80 and exact siblings remain
positive filters where the terminal is already in base.

MEASURED: RP replay46 has complete matching inventories/keys and **0 changed
sections vs base**. Its former RP2-c rename gains are cut by rule4. Sixty-two
fresh both-grammar review/positive repositories are GREEN; 45 differ from the
frozen old head under the new expected rules, providing behavioral RED.
Permanent namespace matrix31 includes uncertainty, exact candidate membership,
Callable-only terminals, E7 all-candidate grading and bounded initializer cycles.

## Historical r4 controls (superseded for current S1b-4)

# S1b-4 r4 controls — final spec-review r2 fold, 2026-10-01

MEASURED: **411 scenarios; 358 identical sections, 53 changed versus base
915fca43**. Every complete site-key set and function inventory agrees; stderr is
empty. 349 historical controls: 309 identical / 40 changed. New C221–C244,
48 both-grammar rows: 35 identical / 13 changed. Against the pre-fold prototype,
358 sections are identical / 39 changed. r2 references are retained; r4 summary
is probes/S1b-controls-s1b4-r4-proto.txt. Complete comparisons and the registered
column assertions are target/plan-s1b4/head/{controls-comparison-r4.json,
registered-controls-r4.json}. Original397 sections equal r3; all14 new sections
equal base. The prior r1 comparison (358 identical/39 changed) is historical.

READ: **C129/C210 with and C201 duplicate import are early-SyntaxError inputs**
in a module, outside the reachable-behavior argument. Their six rows now keep
base, matching non-proving Unproven doctrine; any runtime outcome is irrelevant.
TS-only syntax under JSX (C225/C226/C244) is a parse-recovery pin; TSX carries
the valid TypeScript semantic assertion. No invalid input proves wrong authority.

INHERITED: r3 source audit below checks each actual qualifier declaration,
caller-relative producer and terminal identity. An opaque cell's file/name does
not prove callable origin; only the private-barrel non-escape proof disproves
that barrel's local decoys. Missing facts alone cannot prove absence. Alias/MayCall
and CJS export interop preserve base where unproven, while opaque fallback keeps
edges outside E5 at E7 NameOnly. May-call/written terminals retain full base
rows on every route, whatever kind. These are source-bound claims, not lexical guesses about
value flow. C244 TSX is an inherited-core-policy cost, not a wrong target.

| Scenario | Same-environment base row | Folded prototype row | Class / explanation |
|---|---|---|---|
| C131_E7_bare_namespace_jsx | `app.js:run L3 'f' drop=None -> ['foo.js:f@1-3 exact/import_qualified']` | `app.js:run L3 'f' drop=None -> ['foo.js:f@1-3 name_only/import_qualified']` | exact_to_nameonly_identity_kept_E7 |
| C131_E7_bare_namespace_tsx | `app.ts:run L3 'f' drop=None -> ['foo.ts:f@1-3 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['foo.ts:f@1-3 name_only/import_qualified']` | exact_to_nameonly_identity_kept_E7 |
| C132_E7_nonsibling_stem_jsx | `app.js:run L3 'f' drop=None -> ['b/foo.js:f@1-3 exact/import_qualified']` | `app.js:run L3 'f' drop=None -> ['b/foo.js:f@1-3 name_only/import_qualified']` | exact_to_nameonly_identity_kept_E7 |
| C132_E7_nonsibling_stem_tsx | `app.ts:run L3 'f' drop=None -> ['b/foo.ts:f@1-3 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['b/foo.ts:f@1-3 name_only/import_qualified']` | exact_to_nameonly_identity_kept_E7 |
| C133_E7_sibling_missing_member_jsx | `app.js:run L3 'f' drop=None -> ['b/foo.js:f@1-3 exact/import_qualified']` | `app.js:run L3 'f' drop=ImportExternal -> []` | removed_wrong_other_directory |
| C133_E7_sibling_missing_member_tsx | `app.ts:run L3 'f' drop=None -> ['b/foo.ts:f@1-3 exact/import_qualified']` | `app.ts:run L3 'f' drop=ImportExternal -> []` | removed_wrong_other_directory |
| C148_E7_js_specifier_sibling | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@2-4 exact/import_qualified', 'lib.ts:f@7-9 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@7-9 exact/import_qualified']` | removed_wrong_decoy |
| C190_ns_direct_decoy_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C190_ns_direct_decoy_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C191_ns_directory_decoy_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified', 'other/lib.jsx:f@1-3 exact/import_qualified']` | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified']` | removed_wrong_other_directory |
| C191_ns_directory_decoy_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified', 'other/lib.tsx:f@1-3 exact/import_qualified']` | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified']` | removed_wrong_other_directory |
| C192_ns_string_rename_barrel_jsx | `app.jsx:run L3 'f' drop=UnknownName -> []` | `app.jsx:run L3 'f' drop=None -> ['impl.jsx:actual@1-1 exact/import_qualified']` | added_right_renamed_export |
| C192_ns_string_rename_barrel_tsx | `app.tsx:run L3 'f' drop=UnknownName -> []` | `app.tsx:run L3 'f' drop=None -> ['impl.tsx:actual@1-1 exact/import_qualified']` | added_right_renamed_export |
| C193_ns_star_barrel_jsx | `app.jsx:run L3 'f' drop=ImportExternal -> []` | `app.jsx:run L3 'f' drop=None -> ['impl.jsx:f@1-3 exact/import_qualified']` | added_right_star_export |
| C193_ns_star_barrel_tsx | `app.tsx:run L3 'f' drop=ImportExternal -> []` | `app.tsx:run L3 'f' drop=None -> ['impl.tsx:f@1-3 exact/import_qualified']` | added_right_star_export |
| C205_ns_nested_closure_jsx | `app.jsx:inner L3 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:inner L3 'f' drop=None -> ['lib.jsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C205_ns_nested_closure_tsx | `app.tsx:inner L3 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:inner L3 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C206_ns_parameter_default_position_jsx | `app.jsx:run L2 'f' drop=UnknownName -> []` | `app.jsx:run L2 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified']` | added_right_parameter_environment |
| C206_ns_parameter_default_position_tsx | `app.tsx:run L2 'f' drop=UnknownName -> []` | `app.tsx:run L2 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified']` | added_right_parameter_environment |
| C207_ns_inner_write_does_not_reach_import_jsx | `app.jsx:run L4 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L4 'f' drop=None -> ['lib.jsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C207_ns_inner_write_does_not_reach_import_tsx | `app.tsx:run L4 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L4 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C208_ns_computed_key_position_jsx | `app.jsx:run L3 'f' drop=UnknownName -> []` | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified']` | added_right_computed_key_environment |
| C208_ns_computed_key_position_tsx | `app.tsx:run L3 'f' drop=UnknownName -> []` | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified']` | added_right_computed_key_environment |
| C214_ns_wrapped_jsx_and_call_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified']` | `app.jsx:run L3 'f' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C214_ns_wrapped_jsx_and_call_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified']` | `app.tsx:run L3 'f' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C215_ns_missing_export_decoy_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-1 exact/import_qualified']` | `app.jsx:run L3 'f' drop=ImportExternal -> []` | removed_wrong_nonexport |
| C215_ns_missing_export_decoy_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-1 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | removed_wrong_nonexport |
| C216_ns_nonsibling_stem_jsx | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@2-2 exact/import_qualified', 'other/lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@5-7 name_only/import_qualified']` | removed_wrong_decoy_and_E7_demotion |
| C216_ns_nonsibling_stem_tsx | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@2-2 exact/import_qualified', 'other/lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@5-7 name_only/import_qualified']` | removed_wrong_decoy_and_E7_demotion |
| C217_ns_e7_sibling_jsx | `app.jsx:run L2 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L2 'f' drop=None -> ['lib.jsx:f@5-7 name_only/import_qualified']` | removed_wrong_decoy_and_E7_split |
| C217_ns_e7_sibling_tsx | `app.tsx:run L2 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L2 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy_and_E7_split |
| C232_bare_alias_e7_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified']` | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@2-2 name_only/import_qualified']` | S1/W4 opaque bare fallback E7 NameOnly; edge kept |
| C232_bare_alias_e7_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified']` | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@2-2 name_only/import_qualified']` | S1/W4 opaque bare fallback E7 NameOnly; edge kept |
| C234_opaque_barrel_decoy_removed_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-1 exact/import_qualified']` | `app.jsx:run L3 'f' drop=ImportExternal -> []` | S1 inert named forwarding barrel disproves its private decoy |
| C234_opaque_barrel_decoy_removed_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-1 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | S1 inert named forwarding barrel disproves its private decoy |
| C235_opaque_barrel_decoy_removed_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-1 exact/import_qualified']` | `app.jsx:run L3 'f' drop=ImportExternal -> []` | S1 inert star forwarding barrel disproves its private decoy |
| C235_opaque_barrel_decoy_removed_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-1 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | S1 inert star forwarding barrel disproves its private decoy |
| C236_opaque_barrel_decoy_removed_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-1 exact/import_qualified']` | `app.jsx:run L3 'f' drop=ImportExternal -> []` | S1 inert ImportForward barrel disproves its private decoy |
| C236_opaque_barrel_decoy_removed_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-1 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | S1 inert ImportForward barrel disproves its private decoy |
| C241_star_cycle_final_jsx | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@1-3 exact/import_qualified']` | `app.jsx:run L3 'f' drop=ImportExternal -> []` | W1 complete star cycle final |
| C241_star_cycle_final_tsx | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@1-3 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | W1 complete star cycle final |
| C242_nonsibling_opaque_e7_jsx | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@2-2 exact/import_qualified']` | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@2-2 name_only/import_qualified']` | S1 opaque non-sibling E7 NameOnly; edge kept |
| C242_nonsibling_opaque_e7_tsx | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@2-2 exact/import_qualified']` | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@2-2 name_only/import_qualified']` | S1 opaque non-sibling E7 NameOnly; edge kept |
| C244_ts_function_namespace_inherited_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-1 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | S5 inherited D4 merge refusal in TSX; JSX recovery keeps base |
| C62_S1b_namespace_decoy | `app.tsx:App L3 'Island' drop=None -> ['lib.tsx:Island@3-3 exact/import_qualified', 'lib.tsx:Island@6-8 exact/import_qualified']` | `app.tsx:App L3 'Island' drop=None -> ['lib.tsx:Island@6-8 exact/import_qualified']` | removed_wrong_decoy |
| C80_R3_namespace_function_decoy_jsx | `app.js:run L3 'f' drop=None -> ['lib.js:f@2-2 exact/import_qualified', 'lib.js:f@5-7 exact/import_qualified']` | `app.js:run L3 'f' drop=None -> ['lib.js:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C80_R3_namespace_function_decoy_tsx | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@2-2 exact/import_qualified', 'lib.ts:f@5-7 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C81_R3_namespace_js_extension_decoy_jsx | `app.js:run L3 'f' drop=None -> ['lib.js:f@2-2 exact/import_qualified', 'lib.js:f@5-7 exact/import_qualified']` | `app.js:run L3 'f' drop=None -> ['lib.js:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C81_R3_namespace_js_extension_decoy_tsx | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@2-2 exact/import_qualified', 'lib.ts:f@5-7 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C82_R3_namespace_wrapped_nonjsx_jsx | `app.jsx:App L3 'Island' drop=None -> ['lib.jsx:Island@2-4 exact/import_qualified']` | `app.jsx:App L3 'Island' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C82_R3_namespace_wrapped_nonjsx_tsx | `app.tsx:App L3 'Island' drop=None -> ['lib.tsx:Island@2-4 exact/import_qualified']` | `app.tsx:App L3 'Island' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C83_R3_namespace_reexport_rename_jsx | `app.js:run L3 'f' drop=UnknownName -> []` | `app.js:run L3 'f' drop=None -> ['impl.js:g@1-3 exact/import_qualified']` | added_right_renamed_export |
| C83_R3_namespace_reexport_rename_tsx | `app.ts:run L3 'f' drop=UnknownName -> []` | `app.ts:run L3 'f' drop=None -> ['impl.ts:g@1-3 exact/import_qualified']` | added_right_renamed_export |

## New registered columns (both JSX and TSX)

MEASURED: all 48 new rows reach the columns registered below, including the
preservation rows absent from the changed-row table. Each control's source is
in probes/controls_gen.py; permanent assertions are d11–d14 and revised d10.

| Controls | Required column and mechanism |
|---|---|
| C221, JSX and TSX | W1 CJS spread keeps base |
| C222, JSX and TSX | W1 Object.assign keeps base |
| C223, JSX and TSX | W1 exports expression keeps base |
| C224, JSX and TSX | W1 module.exports expression keeps base |
| C225, JSX and TSX | W1 TS export= keeps base; JSX recovery pin |
| C226, JSX and TSX | W1 TS export import keeps base; JSX recovery pin |
| C227, JSX and TSX | W1 depth truncation keeps base |
| C228, JSX and TSX | W2 object pattern opacity keeps base |
| C229, JSX and TSX | W2 array pattern opacity keeps base |
| C230, JSX and TSX | W3 valid B0 namespace keeps base |
| C231, JSX and TSX | W3 valid B0 named-import twin keeps base |
| C232, JSX and TSX | S1/W4 opaque bare fallback E7 NameOnly; edge kept |
| C233, JSX and TSX | W4 skipped MayCall initializer keeps base |
| C234, JSX and TSX | S1 inert named forwarding barrel disproves its private decoy |
| C235, JSX and TSX | S1 inert star forwarding barrel disproves its private decoy |
| C236, JSX and TSX | S1 inert ImportForward barrel disproves its private decoy |
| C237, JSX and TSX | S3 exact indexed JSX to TSX sibling |
| C238, JSX and TSX | W1 CJS Local preservation guard |
| C239, JSX and TSX | W1 untruncated depth2 exact guard |
| C240, JSX and TSX | W1 complete star conflict final |
| C241, JSX and TSX | W1 complete star cycle final |
| C242, JSX and TSX | S1 opaque non-sibling E7 NameOnly; edge kept |
| C243, JSX and TSX | W1 mixed ESM/CJS facts incomplete; keeps base |
| C244, JSX and TSX | S5 inherited D4 merge refusal in TSX; JSX recovery keeps base |

## Changes attributable to this fold

MEASURED: pre-fold versus folded controls are run in this environment; each of
39 changed sections is enumerated in controls-fold-r3.json. Twenty-two rows
restore correct W1/W2/B0/mixed-CJS base targets, six restore invalid-program
base pins, ten remove known decoys or apply E7 NameOnly, and one restores the
JSX recovery twin of the TS merge. The valid TSX merge refusal remains unchanged
from the pre-fold body. Same-source function inventories and keys agree.

MEASURED: RP replay has 46 complete sections; four RP2-c twins change base
UnknownName→Exact/import_qualified to the exported f@1 through member g. Other
42 match base. Whole r3 replay summary equals r2 byte-for-byte, with all keys
and stderr checked. The generic controls comparator rejected a valid no-call RP
fixture as empty; that setup observation is inadmissible, not a replay failure.
The fixed replay population explicitly permits no-call fixtures and verifies
both complete inventories. No expected row was re-baselined to hide a change.

READ: these controls do not establish zero synthetic value-flow loss. The public
zero-loss statement independently compares full target multisets and complete
keys. A baseline Exact can already be wrong; preserving it is not proof it is
right. Independent C220 source-identity execution is retained as r2 evidence;
this fold replays its edge but does not re-execute that Node identity experiment.

## R1 declaration-shadow guard (additional scope probe)

MEASURED: `target/plan-s1b4/r1-using/` has both grammar probes for
`import * as ns from './lib'; function run(){ using ns=null; ns.f(); }`.
Valid TSX: base gives `lib.tsx:f@1 Exact/import_qualified`, head drops
UnknownName. `using` binds the nearer null resource, so that Exact target is
wrong. The qualifier walk proves not_callable while both legacy receiver flags
are false. d13 now permanently asserts that state and refusal. The JSX input
is a recovery twin: UnknownName on both binaries, not reachable TS semantics.
These two supplementary probes are separate from the397 generated repositories.

R1 initially survived the old15-test matrix. After adding this valid TSX row,
the isolated R1 mutation fails the concrete drop assertion. Its first survival
receipt remains in mutants-r3-reviewer; the final killing receipt is in
mutants-r3-using. No equivalence was inferred from the initial survival.

## Opaque binding cell is not callable origin (six supplementary rows)

MEASURED: target/plan-s1b4/opacity-origin/before.json binds same-environment
base and first-fold rows in both grammars. Renamed cell g exported as f has a
real nested callable f@2: the resolved lib/index→lib/impl route must keep the
base Exact, and bare pkg/lib must keep that same edge at E7 NameOnly. A valid
cyclic ESM alias exports lib's rootFn through impl's g cell and back as lib.f;
the base lib:f@1 Exact is right and must remain. runtime/check.mjs checks real
function identity and result1 (runtime.log). The first broad terminal-file/name
filter dropped all six right edges. That inference is refuted and superseded.

The final proof only removes private functions in an inert forwarding barrel:
no local callable export, top-level executable expression, declaration/value
export or opaque local claim can expose them. d10's pinned decoy still drops;
its renamed-cell, terminal-directory and cyclic-alias negatives keep the right
origins. d12 retains renamed bare-Alias edges at NameOnly. Permanent RED is
13 passed/2 failed in head/opacity-origin-red.log, then15/0 GREEN. Final six
registered supplementary outcomes are opacity-origin/after.json; they are
separate from the397 generated controls. No new value-flow cost is introduced.

## Final r2 review fold preservation controls

MEASURED: C245-C251 have JSX/TSX twins, all identical to same-environment
base, with full inventories/keys and empty stderr. No original397 section
changes, so the 53 individually explained rows above remain the whole
base-to-head changed population. Complete new-control pre-change r3 outputs
are head/reviewer-r3-new-controls; final base/head controls-r4 retain all calls,
including helper/initializer calls, rather than comparing only ns.f.

| Both-grammar control | Final namespace row | Disposition |
|---|---|---|
| C245 escape twin | lib:f@2 Exact | W5: setG(f) can expose private f; d15 also revokes private-barrel premise |
| C246 E5a bare M1 | lib:f@2 Exact | W6: r3 NameOnly -> base Exact; function-argument call opacity origin |
| C247 E5b bare written | lib:f@1 Exact | W6: r3 NameOnly -> base Exact; M2 origin |
| C248 E5c private star -> written | lib:f@1 Exact | W6: r3 drop -> base decoy kept under unconditional E5 |
| C249 E5e private named -> M1 | lib:f@1 Exact | W6: r3 drop -> base decoy kept under unconditional E5 |
| C250 written class | lib:f@1 Exact | r3 drop -> base; E5 whatever-kind guard; namespace-only projection, D4 untouched |
| C251 eligible imported-local -> written | lib:f@1 Exact | r3 drop -> base; E5 forwarding origin; renamed import preserves the producer's competitor rule |

READ: C248/C249 base decoys are preserved by explicit owner E5, despite the
non-escape proof; no exception is proposed. d16 covers M1 through eligible
ImportForward too; d17 covers written class/literal/arrow with complete base
rows (the arrow has two base targets). d18 exercises serde defaulting and
MayCall -> Alias -> MayCall incremental origin replacement. Historical r1
using and independent Node identity receipts below/above are inherited here.
