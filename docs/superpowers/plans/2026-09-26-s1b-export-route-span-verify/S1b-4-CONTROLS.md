# S1b-4 controls: complete-row audit, 2026-10-01

MEASURED: 349 scenarios; all site-key sets equal; every function inventory
unchanged; stderr 0. 303 whole sections identical, 46 changed. Original 287:
269 identical and 18 changed sections. New 62: 34 identical and 28 changed
sections. The reference is `probes/S1b-controls-s1b4-r2-proto.txt`.

READ / hand audit: every changed row below was checked against its explicit
qualifier declaration and producer in controls_gen.py. A module imports only
its caller-relative module; nested and other-directory functions are decoys.
For C129/C210, supplying an object with its own Lib/ns.f is a concrete
counterexample to base's Exact namespace authority. The class describes that
wrong authority; it does not prove every runtime target impossible or justify
a zero-synthetic-loss claim. The public zero-loss claim uses complete target
identity comparisons, not this lexical classification.
Renamed/star exports point at the enumerated terminal. `with`/duplicates prove
no namespace authority. Parameter defaults exclude body-only declarations;
computed keys exclude method parameter scope. Wrapped renders cannot be bound
from ordinary calls. E7 demotions keep the eligible target. These conclusions
come from source/value origins, not from the lexical auditor's callable grade.

| Scenario | Same-environment base row | Prototype row | Class |
|---|---|---|---|
| C129_solW6_namespace_with_jsx | `app.js:host L4 'f' drop=None -> ['lib.js:f@1-3 exact/import_qualified']` | `app.js:host L4 'f' drop=UnknownName -> []` | removed_wrong_with_namespace_authority |
| C129_solW6_namespace_with_tsx | `app.ts:host L4 'f' drop=None -> ['lib.ts:f@1-3 exact/import_qualified']` | `app.ts:host L4 'f' drop=UnknownName -> []` | removed_wrong_with_namespace_authority |
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
| C201_ns_duplicate_import_jsx | `app.jsx:run L4 'f' drop=None -> ['other.jsx:f@1-3 exact/import_qualified']` | `app.jsx:run L4 'f' drop=UnknownName -> []` | removed_wrong_duplicate_namespace_authority |
| C201_ns_duplicate_import_tsx | `app.tsx:run L4 'f' drop=None -> ['other.tsx:f@1-3 exact/import_qualified']` | `app.tsx:run L4 'f' drop=UnknownName -> []` | removed_wrong_duplicate_namespace_authority |
| C205_ns_nested_closure_jsx | `app.jsx:inner L3 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:inner L3 'f' drop=None -> ['lib.jsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C205_ns_nested_closure_tsx | `app.tsx:inner L3 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:inner L3 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C206_ns_parameter_default_position_jsx | `app.jsx:run L2 'f' drop=UnknownName -> []` | `app.jsx:run L2 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified']` | added_right_parameter_environment |
| C206_ns_parameter_default_position_tsx | `app.tsx:run L2 'f' drop=UnknownName -> []` | `app.tsx:run L2 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified']` | added_right_parameter_environment |
| C207_ns_inner_write_does_not_reach_import_jsx | `app.jsx:run L4 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L4 'f' drop=None -> ['lib.jsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C207_ns_inner_write_does_not_reach_import_tsx | `app.tsx:run L4 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L4 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C208_ns_computed_key_position_jsx | `app.jsx:run L3 'f' drop=UnknownName -> []` | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified']` | added_right_computed_key_environment |
| C208_ns_computed_key_position_tsx | `app.tsx:run L3 'f' drop=UnknownName -> []` | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified']` | added_right_computed_key_environment |
| C210_ns_with_body_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-3 exact/import_qualified']` | `app.jsx:run L3 'f' drop=UnknownName -> []` | removed_wrong_with_namespace_authority |
| C210_ns_with_body_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-3 exact/import_qualified']` | `app.tsx:run L3 'f' drop=UnknownName -> []` | removed_wrong_with_namespace_authority |
| C214_ns_wrapped_jsx_and_call_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified']` | `app.jsx:run L3 'f' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C214_ns_wrapped_jsx_and_call_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified']` | `app.tsx:run L3 'f' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C215_ns_missing_export_decoy_jsx | `app.jsx:run L3 'f' drop=None -> ['lib.jsx:f@1-1 exact/import_qualified']` | `app.jsx:run L3 'f' drop=ImportExternal -> []` | removed_wrong_nonexport |
| C215_ns_missing_export_decoy_tsx | `app.tsx:run L3 'f' drop=None -> ['lib.tsx:f@1-1 exact/import_qualified']` | `app.tsx:run L3 'f' drop=ImportExternal -> []` | removed_wrong_nonexport |
| C216_ns_nonsibling_stem_jsx | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@2-2 exact/import_qualified', 'other/lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L3 'f' drop=None -> ['other/lib.jsx:f@5-7 name_only/import_qualified']` | removed_wrong_decoy_and_E7_demotion |
| C216_ns_nonsibling_stem_tsx | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@2-2 exact/import_qualified', 'other/lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L3 'f' drop=None -> ['other/lib.tsx:f@5-7 name_only/import_qualified']` | removed_wrong_decoy_and_E7_demotion |
| C217_ns_e7_sibling_jsx | `app.jsx:run L2 'f' drop=None -> ['lib.jsx:f@2-2 exact/import_qualified', 'lib.jsx:f@5-7 exact/import_qualified']` | `app.jsx:run L2 'f' drop=None -> ['lib.jsx:f@5-7 name_only/import_qualified']` | removed_wrong_decoy_and_E7_split |
| C217_ns_e7_sibling_tsx | `app.tsx:run L2 'f' drop=None -> ['lib.tsx:f@2-2 exact/import_qualified', 'lib.tsx:f@5-7 exact/import_qualified']` | `app.tsx:run L2 'f' drop=None -> ['lib.tsx:f@5-7 exact/import_qualified']` | removed_wrong_decoy_and_E7_split |
| C62_S1b_namespace_decoy | `app.tsx:App L3 'Island' drop=None -> ['lib.tsx:Island@3-3 exact/import_qualified', 'lib.tsx:Island@6-8 exact/import_qualified']` | `app.tsx:App L3 'Island' drop=None -> ['lib.tsx:Island@6-8 exact/import_qualified']` | removed_wrong_decoy |
| C80_R3_namespace_function_decoy_jsx | `app.js:run L3 'f' drop=None -> ['lib.js:f@2-2 exact/import_qualified', 'lib.js:f@5-7 exact/import_qualified']` | `app.js:run L3 'f' drop=None -> ['lib.js:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C80_R3_namespace_function_decoy_tsx | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@2-2 exact/import_qualified', 'lib.ts:f@5-7 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C81_R3_namespace_js_extension_decoy_jsx | `app.js:run L3 'f' drop=None -> ['lib.js:f@2-2 exact/import_qualified', 'lib.js:f@5-7 exact/import_qualified']` | `app.js:run L3 'f' drop=None -> ['lib.js:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C81_R3_namespace_js_extension_decoy_tsx | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@2-2 exact/import_qualified', 'lib.ts:f@5-7 exact/import_qualified']` | `app.ts:run L3 'f' drop=None -> ['lib.ts:f@5-7 exact/import_qualified']` | removed_wrong_decoy |
| C82_R3_namespace_wrapped_nonjsx_jsx | `app.jsx:App L3 'Island' drop=None -> ['lib.jsx:Island@2-4 exact/import_qualified']` | `app.jsx:App L3 'Island' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C82_R3_namespace_wrapped_nonjsx_tsx | `app.tsx:App L3 'Island' drop=None -> ['lib.tsx:Island@2-4 exact/import_qualified']` | `app.tsx:App L3 'Island' drop=WrappedExportNonJsx -> []` | removed_wrong_wrapped_non_jsx |
| C83_R3_namespace_reexport_rename_jsx | `app.js:run L3 'f' drop=UnknownName -> []` | `app.js:run L3 'f' drop=None -> ['impl.js:g@1-3 exact/import_qualified']` | added_right_renamed_export |
| C83_R3_namespace_reexport_rename_tsx | `app.ts:run L3 'f' drop=UnknownName -> []` | `app.ts:run L3 'f' drop=None -> ['impl.ts:g@1-3 exact/import_qualified']` | added_right_renamed_export |

MEASURED: RP replay has 46 equal-population sections. RP2-c's js/jsx/ts/tsx
rows change UnknownName to Exact/import_qualified targeting lib:f@1-1; the other
42 sections are byte-identical. Reference: `probes/S1b-replay-s1b4-r2-proto.txt`.

READ: C128, C130, C84/C85, all type-only/import-equals/named/default pins remain
at base. C197/C198/C199 and C218/C219/C220 also retain their base edges under
E5/Option K/Alias opacity; the new qualified MayCall counters are intentional.
C202/C203 recovery twins already had the correct no-namespace result at base;
C204's sealed error keeps Exact. C194's namespace object gains no callable
projection, and nested ns.inner.f remains unchecked. C195/C196 parameter
receivers keep the existing receiver ladder. C200 literal shadow already refused.
C209 with-object keeps its outer namespace proof. C211's .jsx type-only syntax
is a recovery twin; its ERROR(type) must not grant namespace authority.

READ: d10 tests additionally cover direct/list Alias terminals, named and star
barrels with base-row preservation decoys, eligible ImportForward, and a D4
named-import consumer that stays unresolved. A forwarding producer rejected by
its existing whole-file competitor guard is outside that extension; no producer
contract is relaxed. C220's independently evaluated returned callable is still
f@2 in each grammar; its base/head dump rows are identical.
