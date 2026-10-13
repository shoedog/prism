# JS/TS alias-following — decision analysis

Completed static planning census, 2026-10-13. **The revised join gate passes. No implementation or adoption approval is claimed.** X = Excalidraw; T = TypeScript. All counts below are physical bare-call/JSX sites with a frozen-base **Source** record unless explicitly labeled otherwise. `P / Tst` separates product and test files. Every generated table comes from `step0b/{X,T}/decision-summary.json` (the named section); per-site proofs are in `sites.jsonl`. Evidence root: `/Users/wesleyjinks/prism-evidence/js-alias/step0b`.


## 1a. Headline census

C = correct existing target; W = wrong existing target; M = known in-repo callable with no edge; Ø = correct absence of an in-repo edge; Ue / UØ = unknown with / without existing edges. These six outcomes partition sites. Gain counts a missing **registered callable identity** or a corrected target; it is a static opportunity ceiling, not prototype output. Missing anonymous/unregistered targets are not gains. W includes the preserved earlier E5 adjudications; unknown final value does not downgrade those WRONG findings. Source: `part1a.tables`, `part1a.detailed`.


| Corpus | Binding | Sites P / Tst | C | W | M | Ø | Ue / UØ | Gain | Of gain: correction |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| X | global_ambient_library | 122 / 11 | 0 | 0 | 0 | 133 | 0 / 0 | 0 | 0 |
| X | import | 6,236 / 163 | 2467 | 0 | 22 | 3 | 3,112 / 795 | 16 | 0 |
| X | local | 1,909 / 37 | 1562 | 19 | 2 | 0 | 105 / 258 | 2 | 0 |
| X | parameter | 324 / 6 | 0 | 0 | 0 | 0 | 0 / 330 | 0 | 0 |
| X | same_file | 460 / 5 | 461 | 0 | 0 | 0 | 4 / 0 | 0 | 0 |
| X | unresolved | 875 / 67 | 0 | 0 | 0 | 866 | 1 / 75 | 0 | 0 |
| T | global_ambient_library | 48 / 1 | 0 | 4 | 0 | 45 | 0 / 0 | 0 | 0 |
| T | import | 14,430 / 1,109 | 0 | 3 | 15288 | 0 | 0 / 248 | 15219 | 3 |
| T | local | 719 / 46 | 84 | 195 | 0 | 0 | 235 / 251 | 0 | 0 |
| T | parameter | 931 / 73 | 0 | 0 | 0 | 0 | 0 / 1,004 | 0 | 0 |
| T | same_file | 17,059 / 780 | 17798 | 0 | 41 | 0 | 0 / 0 | 41 | 0 |
| T | unresolved | 10 / 164 | 0 | 0 | 0 | 0 | 12 / 162 | 0 | 0 |

**Alias/import opportunity: X 18, T 15,219.** T also has 41 same-file declaration misses, outside this alias plan. X has 6 known missing endpoints without a usable named FunctionId; T imports have 72 endpoints in an oversized, unregistered source file. They remain M but are excluded from gain. X local WRONG 19 and T local WRONG 195 retain the earlier oracle verdicts even though this value tracer cannot name their eventual callees. T has one additional wrong `free_multi` site (three NameOnly candidate edges), beyond the earlier 201 Exact WRONG E5 sites.


### Recorded join gate

| Corpus | Checker files | All bare sites | Recorded sites | Matched bare Source records | Identified bare Source records | Unidentified Source records | Conservative failure upper bound | Gate |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| X | 631 | 22964 | 10215 | 10312 | 10312 | 6 | 0.0582% | PASS |
| T | 708 | 62768 | 35370 | 35551 | 35551 | 223 | 0.6234% | PASS |

All identified bare Source records join. Multiple caller records can describe one physical site. To avoid hiding unknown syntax, the upper bound counts every unidentified Source span as an eligible failed join: X 6/(10,312+6), T 223/(35,551+223). Both are below 3%. No approximate line/name joins are used. Source: `join_gate`, full residuals in `unidentified-source-records.jsonl`.


### Detailed binding classes and outcomes

Each cell is **product / test**. Zero-count classes are omitted; absence is zero, not unsupported taxonomy. Named versus renamed/default/namespace/import-equals/require/dynamic imports, assignment modes, initializer shapes and parameter defaults are classified separately. Source: `part1a.detailed`; symbols come from the checker, reassignment and initializer/parameter shapes from conservative syntax scans.


#### X

| Class | Sites | C | W | M | Ø | Ue | UØ | Gain |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| global_ambient_library | 122 / 11 | 0 / 0 | 0 / 0 | 0 / 0 | 122 / 11 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.default | 233 / 0 | 88 / 0 | 0 / 0 | 5 / 0 | 3 / 0 | 29 / 0 | 108 / 0 | 1 / 0 |
| import.dynamic_import | 1 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.dynamic_import.destructured | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| import.named | 5,977 / 162 | 2,347 / 30 | 0 / 0 | 11 / 5 | 0 / 0 | 2,989 / 91 | 630 / 36 | 10 / 5 |
| import.renamed | 7 / 1 | 1 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 3 / 0 | 0 / 0 |
| import.require.destructured | 17 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 17 / 0 | 0 / 0 |
| local.const.arrow | 1,537 / 33 | 1,501 / 33 | 0 / 0 | 0 / 0 | 0 / 0 | 36 / 0 | 0 / 0 | 0 / 0 |
| local.const.call_result | 66 / 4 | 13 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 17 / 0 | 35 / 4 | 1 / 0 |
| local.const.destructure.await | 6 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 | 0 / 0 |
| local.const.destructure.call_result | 199 / 0 | 0 / 0 | 8 / 0 | 0 / 0 | 0 / 0 | 15 / 0 | 176 / 0 | 0 / 0 |
| local.const.destructure.identifier | 24 / 0 | 0 / 0 | 11 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 12 / 0 | 0 / 0 |
| local.const.element_member | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.function_expression | 15 / 0 | 15 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.identifier | 5 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 4 / 0 | 1 / 0 |
| local.const.identifier_member | 7 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 6 / 0 | 0 / 0 |
| local.const.none | 5 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 0 | 0 / 0 |
| local.const.other | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.const.other_member | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.let.reassigned.function_expression | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 |
| local.let.reassigned.identifier | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.let.reassigned.none | 6 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 4 / 0 | 2 / 0 | 0 / 0 |
| local.let.reassigned.other | 10 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 9 / 0 | 1 / 0 | 0 / 0 |
| local.var.never_reassigned.other | 16 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 16 / 0 | 0 / 0 | 0 / 0 |
| local.var.reassigned.other | 5 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 0 | 0 / 0 | 0 / 0 |
| parameter.destructured | 201 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 201 / 0 | 0 / 0 |
| parameter.plain | 120 / 6 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 120 / 6 | 0 / 0 |
| parameter.plain.default | 3 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| same_file.function | 460 / 5 | 456 / 5 | 0 / 0 | 0 / 0 | 0 / 0 | 4 / 0 | 0 / 0 | 0 / 0 |
| unresolved | 875 / 67 | 0 / 0 | 0 / 0 | 0 / 0 | 864 / 2 | 1 / 0 | 10 / 65 | 0 / 0 |

#### T

| Class | Sites | C | W | M | Ø | Ue | UØ | Gain |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| global_ambient_library | 48 / 1 | 0 / 0 | 4 / 0 | 0 / 0 | 44 / 1 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.default | 0 / 2 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 2 | 0 / 0 |
| import.equals | 10 / 0 | 0 / 0 | 3 / 0 | 7 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 10 / 0 |
| import.named | 14,391 / 1,105 | 0 / 0 | 0 / 0 | 14,149 / 1,102 | 0 / 0 | 0 / 0 | 242 / 3 | 14,077 / 1,102 |
| import.renamed | 29 / 1 | 0 / 0 | 0 / 0 | 29 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 29 / 1 |
| import.require | 0 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 1 | 0 / 0 |
| local.const.arrow | 78 / 6 | 78 / 6 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.call_result | 210 / 11 | 0 / 0 | 0 / 1 | 0 / 0 | 0 / 0 | 169 / 2 | 41 / 8 | 0 / 0 |
| local.const.conditional | 32 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 30 / 0 | 0 / 0 |
| local.const.destructure.call_result | 36 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 22 / 1 | 14 / 0 | 0 / 0 |
| local.const.destructure.identifier | 130 / 1 | 0 / 0 | 126 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 2 / 1 | 0 / 0 |
| local.const.destructure.none | 4 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 4 / 1 | 0 / 0 |
| local.const.element_member | 3 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 1 | 0 / 0 |
| local.const.identifier | 3 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| local.const.identifier_member | 47 / 5 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 | 41 / 5 | 0 / 0 |
| local.const.none | 3 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 1 | 0 / 0 |
| local.const.other | 7 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 4 / 0 | 2 / 0 | 0 / 0 |
| local.const.other_member | 18 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 18 / 0 | 0 / 0 |
| local.let.never_reassigned.none | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.let.reassigned.arrow | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 |
| local.let.reassigned.call_result | 7 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 7 / 0 | 0 / 0 |
| local.let.reassigned.identifier | 4 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 1 / 0 | 0 / 0 |
| local.let.reassigned.none | 6 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 4 / 0 | 2 / 0 | 0 / 0 |
| local.var.never_reassigned.call_result | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.call_result | 12 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 12 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.conditional | 12 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 12 / 0 | 0 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.identifier | 64 / 0 | 0 / 0 | 17 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 46 / 0 | 0 / 0 |
| local.var.never_reassigned.none | 30 / 19 | 0 / 0 | 30 / 19 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.var.reassigned.identifier | 7 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 7 / 0 | 0 / 0 | 0 / 0 |
| local.var.reassigned.none | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| parameter.destructured | 61 / 7 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 61 / 7 | 0 / 0 |
| parameter.destructured.default | 5 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 1 | 0 / 0 |
| parameter.plain | 844 / 65 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 844 / 65 | 0 / 0 |
| parameter.plain.default | 21 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 21 / 0 | 0 / 0 |
| same_file.function | 17,059 / 780 | 17,018 / 780 | 0 / 0 | 41 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 41 / 0 |
| unresolved | 10 / 164 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 10 | 8 / 154 | 0 / 0 |

### Call-result subclasses

These refine, and do not add to, the local/require/destructure classes above. Each cell is product / test. Import-proven React labels are distinct from spelling-only hook/member/factory labels; a factory name or callable signature does not establish a returned value. Source: `part1a.call_result_tables`; their complete rung/drop/verdict cross-tab is `part1a.call_result_cross_tabs`. Absent memo/forwardRef/styled categories have zero observed sites in that corpus.

#### X

| Initializer / factory subclass | Sites | C | W | M | Ue | UØ | Gain |
| --- | --- | --- | --- | --- | --- | --- | --- |
| import.require.destructured@other_factory | 17 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 17 / 0 | 0 / 0 |
| local.const.call_result@hook_spelling_heuristic | 23 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 21 / 0 | 0 / 0 |
| local.const.call_result@member_factory | 17 / 4 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 | 11 / 4 | 0 / 0 |
| local.const.call_result@other_factory | 11 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 9 / 0 | 2 / 0 | 0 / 0 |
| local.const.call_result@react.memo | 3 / 0 | 2 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| local.const.call_result@react.useCallback | 11 / 0 | 11 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.call_result@react.useContext | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.destructure.call_result@hook_spelling_heuristic | 139 / 0 | 0 / 0 | 8 / 0 | 0 / 0 | 11 / 0 | 120 / 0 | 0 / 0 |
| local.const.destructure.call_result@other_factory | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 |
| local.const.destructure.call_result@react.useState | 59 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 56 / 0 | 0 / 0 |

#### T

| Initializer / factory subclass | Sites | C | W | M | Ue | UØ | Gain |
| --- | --- | --- | --- | --- | --- | --- | --- |
| import.require@other_factory | 0 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 1 | 0 / 0 |
| local.const.call_result@member_factory | 11 / 9 | 0 / 0 | 0 / 1 | 0 / 0 | 5 / 2 | 6 / 6 | 0 / 0 |
| local.const.call_result@other_factory | 199 / 2 | 0 / 0 | 0 / 0 | 0 / 0 | 164 / 0 | 35 / 2 | 0 / 0 |
| local.const.destructure.call_result@member_factory | 12 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 12 / 0 | 0 / 0 |
| local.const.destructure.call_result@other_factory | 24 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 22 / 1 | 2 / 0 | 0 / 0 |
| local.let.reassigned.call_result@other_factory | 7 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 7 / 0 | 0 / 0 |
| local.var.never_reassigned.call_result@other_factory | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.call_result@member_factory | 12 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 12 / 0 | 0 / 0 |

### Endpoint and chain distributions

Source: `part1a.tables`, `part1a.chain_depths`. Depth is the measurement tracer’s value-following step count; checker import aliases can collapse several export hops. It is **not** the re-export depth budget. The separate import-route census below supplies that distinction.

| Corpus | Class | Repo callable | Outside repo | No callable | Unknown | depth:sites |
| --- | --- | --- | --- | --- | --- | --- |
| X | global_ambient_library | 0 | 133 | 0 | 0 | 0:133 |
| X | import | 2489 | 3 | 0 | 3907 | 0:3807, 1:147, 2:2283, 3:152, 4:10 |
| X | local | 1564 | 0 | 0 | 382 | 0:165, 1:1758, 2:22, 3:1 |
| X | parameter | 0 | 0 | 0 | 330 | 0:330 |
| X | same_file | 461 | 0 | 0 | 4 | 0:465 |
| X | unresolved | 0 | 0 | 866 | 76 | 0:942 |
| T | global_ambient_library | 0 | 49 | 0 | 0 | 0:49 |
| T | import | 15291 | 0 | 0 | 248 | 0:10, 1:15289, 2:66, 3:171, 4:3 |
| T | local | 84 | 0 | 0 | 681 | 0:89, 1:560, 2:116 |
| T | parameter | 0 | 0 | 0 | 1004 | 0:1004 |
| T | same_file | 17839 | 0 | 0 | 0 | 0:17839 |
| T | unresolved | 0 | 0 | 0 | 174 | 0:174 |

Longest proved value trace is 4 in both corpora; the census permits 32 and reports cycle/depth refusal. Call-result labels in `part1a.call_results` distinguish import-proven React wrappers from spelling-only hook/styled/factory categories. Hook names and callable return signatures never establish the returned function’s identity. Detailed endpoint refusal reasons and product/test splits are retained in `part1a.endpoints`.

For each detailed binding class, `part1a.detailed` also carries repo-callable / outside-repo / no-callable / unknown endpoint counts, while `part1a.binding_chain_depths` cross-tabulates class × product/test × endpoint × depth. These controller-safe aggregates retain the fine-grained chain distribution without conflating checker alias collapse with re-export traversal.


### Current resolution rung / drop cross-tab

Every row is a binding class and exact action signature; each outcome cell is **product / test**. Multiple entries in an action signature are multiple existing targets at one physical site, not additional sites. This preserves `Exact` rung, `NameOnly` rung, and drop reason without treating all unknowns as missing. Source: `part1a.cross_tabs`.

#### X

| Class | Action | C | W | M | Ø | Ue | UØ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| global_ambient_library | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 122 / 11 | 0 / 0 | 0 / 0 |
| import.default | drop:UnknownName | 0 / 0 | 0 / 0 | 5 / 0 | 3 / 0 | 0 / 0 | 108 / 0 |
| import.default | exact:import_member | 88 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 29 / 0 | 0 / 0 |
| import.dynamic_import | drop:UnknownName | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.dynamic_import.destructured | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| import.named | drop:UnknownName | 0 / 0 | 0 / 0 | 11 / 5 | 0 / 0 | 0 / 0 | 630 / 36 |
| import.named | exact:import_member | 2,347 / 30 | 0 / 0 | 0 / 0 | 0 / 0 | 2,989 / 91 | 0 / 0 |
| import.renamed | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 |
| import.renamed | exact:import_member | 1 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| import.require.destructured | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 17 / 0 |
| local.const.arrow | exact:local_def | 1,501 / 33 | 0 / 0 | 0 / 0 | 0 / 0 | 36 / 0 | 0 / 0 |
| local.const.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 1 / 0 | 35 / 4 |
| local.const.call_result | exact:local_def | 13 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 15 / 0 | 0 / 0 |
| local.const.call_result | exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.call_result | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.destructure.await | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 |
| local.const.destructure.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 176 / 0 |
| local.const.destructure.call_result | exact:free_single | 0 / 0 | 8 / 0 | 0 / 0 | 0 / 0 | 11 / 0 | 0 / 0 |
| local.const.destructure.call_result | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| local.const.destructure.call_result | name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.destructure.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 12 / 0 |
| local.const.destructure.identifier | exact:free_single | 0 / 0 | 11 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.destructure.identifier | name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.element_member | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| local.const.function_expression | exact:local_def | 15 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 4 / 0 |
| local.const.identifier_member | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 |
| local.const.identifier_member | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.none | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.const.none | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 |
| local.const.other | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.const.other_member | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.let.reassigned.function_expression | exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.let.reassigned.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.let.reassigned.none | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.let.reassigned.none | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| local.let.reassigned.none | exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.let.reassigned.other | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| local.let.reassigned.other | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 9 / 0 | 0 / 0 |
| local.var.never_reassigned.other | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 16 / 0 | 0 / 0 |
| local.var.reassigned.other | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| local.var.reassigned.other | exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| parameter.destructured | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 37 / 0 |
| parameter.destructured | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 164 / 0 |
| parameter.plain | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 23 / 0 |
| parameter.plain | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 99 / 6 |
| parameter.plain.default | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 |
| same_file.function | exact:local_def | 456 / 5 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| same_file.function | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| same_file.function | exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| unresolved | drop:JsxIntrinsic | 0 / 0 | 0 / 0 | 0 / 0 | 864 / 2 | 0 / 0 | 0 / 0 |
| unresolved | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 10 / 65 |
| unresolved | exact:free_single | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |

#### T

| Class | Action | C | W | M | Ø | Ue | UØ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| global_ambient_library | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 44 / 1 | 0 / 0 | 0 / 0 |
| global_ambient_library | exact:free_single | 0 / 0 | 4 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.default | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 2 |
| import.equals | drop:UnknownName | 0 / 0 | 0 / 0 | 7 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.equals | exact:free_single | 0 / 0 | 2 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.equals | name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.named | drop:UnknownName | 0 / 0 | 0 / 0 | 14,149 / 1,102 | 0 / 0 | 0 / 0 | 242 / 3 |
| import.renamed | drop:UnknownName | 0 / 0 | 0 / 0 | 29 / 1 | 0 / 0 | 0 / 0 | 0 / 0 |
| import.require | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 1 |
| local.const.arrow | exact:local_def | 78 / 6 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 41 / 8 |
| local.const.call_result | exact:free_single | 0 / 0 | 0 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.call_result | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 162 / 0 | 0 / 0 |
| local.const.call_result | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.const.call_result | exact:local_def,exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 2 | 0 / 0 |
| local.const.call_result | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 0 | 0 / 0 |
| local.const.conditional | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 30 / 0 |
| local.const.conditional | exact:free_single | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.conditional | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.const.destructure.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 14 / 0 |
| local.const.destructure.call_result | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 0 | 0 / 0 |
| local.const.destructure.call_result | name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 11 / 0 | 0 / 0 |
| local.const.destructure.call_result | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 1 | 0 / 0 |
| local.const.destructure.call_result | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| local.const.destructure.call_result | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 | 0 / 0 |
| local.const.destructure.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 1 |
| local.const.destructure.identifier | exact:free_single | 0 / 0 | 126 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.destructure.identifier | name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.const.destructure.none | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 4 / 1 |
| local.const.element_member | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 1 |
| local.const.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 0 |
| local.const.identifier_member | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 41 / 5 |
| local.const.identifier_member | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 | 0 / 0 |
| local.const.none | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 1 |
| local.const.none | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.const.other | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.const.other | exact:free_single | 0 / 0 | 1 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.const.other | name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 4 / 0 | 0 / 0 |
| local.const.other_member | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 18 / 0 |
| local.let.never_reassigned.none | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| local.let.reassigned.arrow | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.let.reassigned.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 7 / 0 |
| local.let.reassigned.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| local.let.reassigned.identifier | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.let.reassigned.identifier | exact:local_def,exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.let.reassigned.none | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.let.reassigned.none | exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.let.reassigned.none | exact:local_def,exact:local_def | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| local.var.never_reassigned.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| local.var.never_reassigned.destructure.call_result | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 12 / 0 |
| local.var.never_reassigned.destructure.conditional | name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.conditional | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 6 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.identifier | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 46 / 0 |
| local.var.never_reassigned.destructure.identifier | exact:free_single | 0 / 0 | 17 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.var.never_reassigned.destructure.identifier | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 | 0 / 0 |
| local.var.never_reassigned.none | exact:free_single | 0 / 0 | 30 / 19 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| local.var.reassigned.identifier | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 7 / 0 | 0 / 0 |
| local.var.reassigned.none | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| parameter.destructured | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 34 / 7 |
| parameter.destructured | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 27 / 0 |
| parameter.destructured.default | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 1 |
| parameter.destructured.default | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 0 |
| parameter.plain | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 161 / 9 |
| parameter.plain | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 683 / 56 |
| parameter.plain.default | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| parameter.plain.default | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 20 / 0 |
| same_file.function | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 41 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| same_file.function | exact:local_def | 17,018 / 780 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| unresolved | drop:LocalBindingUnproven | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 7 / 0 |
| unresolved | drop:UnknownName | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 154 |
| unresolved | exact:free_single | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 2 | 0 / 0 |
| unresolved | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 6 | 0 / 0 |
| unresolved | name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi,name_only:free_multi | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 2 | 0 / 0 |

### Parameter: lookup versus following actual arguments

Source: `part1a.parameters`, `parameters.jsonl`. Counts of actual expressions are observations per unique parameter binding and caller occurrence, not distinct functions. Export and escape visibility is a **syntax heuristic**, not a closed-world proof. The unchanged checker Program sees callers with no Prism site record; these are explicitly separated from existing Exact incoming graph edges.

| Corpus | Sites P / Tst | Bindings | Defaults | Exported bindings | Syntax closed | Other open | Named actuals | Inline actuals | Member actuals | Unknown actuals |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| X | 324 / 6 | 182 | 3 | 111 | 22 | 49 | 22 | 124 | 22 | 160 |
| T | 931 / 73 | 678 | 23 | 226 | 245 | 207 | 1872 | 1895 | 108 | 2103 |

| Corpus | Argument-following ceiling | Product / test |
| --- | --- | --- |
| X | Sites with registered observed targets, all source callers | 24 / 0 |
| X | Candidate edges, all source callers | 43 / 0 |
| X | Sites with 1–3 candidates, all source callers | 22 / 0 |
| X | Sites from current Exact incoming callers | 22 / 0 |
| X | Candidate edges from current Exact incoming callers | 37 / 0 |
| X | Sites with 1–3 candidates from current Exact incoming callers | 20 / 0 |
| X | Syntax-closed singleton sites (not an Exact proof) | 6 / 0 |
| T | Sites with registered observed targets, all source callers | 259 / 20 |
| T | Candidate edges, all source callers | 1,360 / 73 |
| T | Sites with 1–3 candidates, all source callers | 204 / 14 |
| T | Sites from current Exact incoming callers | 133 / 5 |
| T | Candidate edges from current Exact incoming callers | 278 / 8 |
| T | Sites with 1–3 candidates from current Exact incoming callers | 116 / 5 |
| T | Syntax-closed singleton sites (not an Exact proof) | 26 / 4 |

Lookup-only identifies the parameter and prevents a later same-name fallback; all 330 X / 1,004 T recorded parameter calls already lack targets, so it yields 0 additions, 0 current WRONG removals and 0 known CORRECT losses here. Argument-following is a different feature. With existing Exact incoming calls, at most 22 X / 138 T parameter-call sites have registered candidates (37 / 286 edges); a ≤3-target cap leaves 20 / 121 sites. All-source ceilings rise to 24 / 279 sites and 43 / 1,433 edges, but some callers have no graph record or need import repair first. Neither set justifies Exact for exported/escaped functions. Even 6 X / 30 T syntax-closed singleton sites lack a demonstrated complete caller proof. Recommend lookup-only first; optional argument-following produces NameOnly candidates with actual-expression provenance, never a guessed Exact edge.


### Disjoint opportunity allocation for the plan

| Slice opportunity | X gain P / Tst | X corrected | T gain P / Tst | T corrected |
| --- | --- | --- | --- | --- |
| L1_local_identifier | 1 / 0 | 0 | 0 / 0 | 0 |
| L2_local_to_import_or_member | 0 / 0 | 0 | 0 / 0 | 0 |
| L3_wrapper_identifier | 1 / 0 | 0 | 0 / 0 | 0 |
| I1_module_paths | 7 / 0 | 0 | 14,064 / 1,103 | 0 |
| I2a_export_aliases | 1 / 0 | 0 | 2 / 0 | 0 |
| I2b_export_wrappers | 3 / 5 | 0 | 0 / 0 | 0 |
| I3_import_equals | 0 / 0 | 0 | 10 / 0 | 3 |
| I4_deeper_reexports | 0 / 0 | 0 | 40 / 0 | 0 |
| outside_alias_same_file_binding | 0 / 0 | 0 | 41 / 0 | 0 |

Source: `slice_opportunities`, with each supporting row in `opportunity-rows.jsonl`. Allocation is disjoint to avoid double counting prerequisites: an exported wrapper also needing a path repair belongs to I2b; a three-hop exported alias belongs to I4. I1’s large T number is chiefly explicit `.js`→`.ts` substitution, not alias complexity. X I1 has six `.` specifier sites and one redundant-slash route. I2a has X one exported const-arrow forwarding site and T two namespace-export alias sites. I3 contains T seven missing sites, two wrong Exact sites and one wrong NameOnly site. I4’s 40 product sites require three export hops; all are already missing. No prototype has measured the actually realized gains, runtime, memory, incremental behavior, or loss count. The source-path classification is explanatory syntax, not a complete re-export uniqueness proof.


## 1b. E5 rows re-read

Source: `part1b`, `e5-reread.jsonl`, `history-XT-summary.json` and filtered `step2-XT-adjudications.jsonl`. The earlier WRONG designation is preserved; a different eventual target is a separate question.

| Corpus | Removed binding | Partition | Original verdict | Following ends at | Sites |
| --- | --- | --- | --- | --- | --- |
| X | local.const.destructure.call_result | product | UNDECIDED | not_statically_determined | 11 |
| X | local.const.destructure.call_result | product | WRONG | not_statically_determined | 8 |
| X | local.const.destructure.identifier | product | WRONG | not_statically_determined | 11 |
| X | unresolved | product | INADMISSIBLE | not_statically_determined | 1 |
| T | global_ambient_library | product | WRONG | no_in_repo_callable_proved | 4 |
| T | import.equals | product | WRONG | in_repo_callable | 2 |
| T | local.const.call_result | test | WRONG | not_statically_determined | 1 |
| T | local.const.conditional | product | WRONG | not_statically_determined | 1 |
| T | local.const.destructure.identifier | product | WRONG | not_statically_determined | 126 |
| T | local.const.other | product | WRONG | not_statically_determined | 1 |
| T | local.var.never_reassigned.destructure.identifier | product | WRONG | not_statically_determined | 17 |
| T | local.var.never_reassigned.none | product | WRONG | not_statically_determined | 30 |
| T | local.var.never_reassigned.none | test | WRONG | not_statically_determined | 19 |
| T | unresolved | test | INADMISSIBLE | not_statically_determined | 2 |

Among E5 WRONG removals: **X 19: 0 proved replacements, 0 proved no-in-repo target, 19 undetermined. T 201: 2 replacements, 4 proved no-in-repo callable, 195 undetermined.** The two replacements bind import-equals calls to the exported alias target rather than the same-spelled unrelated function. The four no-edge cases bind ambient `Array`. Unknown destructured/factory values are not evidence that no callable exists. X additionally loses 11 UNDECIDED and 1 INADMISSIBLE target; T loses 2 INADMISSIBLE targets. Those are accepted uncertainty exposures only if the owner chooses the refusal policy; they must not be advertised as verified zero-loss.


E5 retains no `free_single` site in this recorded bare population. For all unchanged sites, including drops, the endpoint split is:

| Corpus | Unchanged binding | Partition | Endpoint | Sites |
| --- | --- | --- | --- | --- |
| X | global_ambient_library | product | no_in_repo_callable_proved | 122 |
| X | global_ambient_library | test | no_in_repo_callable_proved | 11 |
| X | import | product | in_repo_callable | 2453 |
| X | import | product | no_in_repo_callable_proved | 3 |
| X | import | product | not_statically_determined | 3780 |
| X | import | test | in_repo_callable | 36 |
| X | import | test | not_statically_determined | 127 |
| X | local | product | in_repo_callable | 1531 |
| X | local | product | not_statically_determined | 348 |
| X | local | test | in_repo_callable | 33 |
| X | local | test | not_statically_determined | 4 |
| X | parameter | product | not_statically_determined | 324 |
| X | parameter | test | not_statically_determined | 6 |
| X | same_file | product | in_repo_callable | 456 |
| X | same_file | product | not_statically_determined | 4 |
| X | same_file | test | in_repo_callable | 5 |
| X | unresolved | product | no_in_repo_callable_proved | 864 |
| X | unresolved | product | not_statically_determined | 10 |
| X | unresolved | test | no_in_repo_callable_proved | 2 |
| X | unresolved | test | not_statically_determined | 65 |
| T | global_ambient_library | product | no_in_repo_callable_proved | 44 |
| T | global_ambient_library | test | no_in_repo_callable_proved | 1 |
| T | import | product | in_repo_callable | 14186 |
| T | import | product | not_statically_determined | 242 |
| T | import | test | in_repo_callable | 1103 |
| T | import | test | not_statically_determined | 6 |
| T | local | product | in_repo_callable | 78 |
| T | local | product | not_statically_determined | 466 |
| T | local | test | in_repo_callable | 6 |
| T | local | test | not_statically_determined | 20 |
| T | parameter | product | not_statically_determined | 931 |
| T | parameter | test | not_statically_determined | 73 |
| T | same_file | product | in_repo_callable | 17059 |
| T | same_file | test | in_repo_callable | 780 |
| T | unresolved | product | not_statically_determined | 10 |
| T | unresolved | test | not_statically_determined | 162 |

X has 10,184 unchanged physical sites, T 35,167; neither comparison has a missing head record or an added target. Earlier step1 had X 98 adjudicated sites (27 WRONG, 34 UNDECIDED, 37 INADMISSIBLE), but step2 narrowed this to 31 Source sites; T stays 203. Step1’s broader/indirect population is not imported into this denominator. Current base captures equal the earlier base call-site multisets exactly (X 19,219, T 61,712). Corpus head captures are reused from step2; equivalence to the step2b head is inherited from the supplied packet, not freshly reproduced on X/T. Frozen step2b is exercised directly by the small synthetic controller run.


## 1c. Round-2 loss shapes

| Input shape | Lookup step / proof needed | Disposition in plan |
| --- | --- | --- |
| `const F=N.F; <F/>`, ordinary `const f=N.f; f(x)` | Local L2 selects nearest alias, proves namespace import identity, follows export to file+span; JSX mode travels with terminal | Resolve before final refusal; synthetic coverage required even though corpus gain is 0 |
| `import f=N.renamed`, exported alias of another callable | Import I3 plus I2a exported-value identity, no spelling exemption | Exact only for unique value-backed target; T has 10 opportunities |
| Module calls script-file global inside merged namespace/enum; decorated class Position variant | Neither local nor parameter nor file import establishes the global; Position is unknown scope, not proof of absence | Owner must choose explicit declaration/scope-proved global exception G or accept this known synthetic loss |
| `super(...)` with imported-object-member heritage | Constructor/heritage proof H; `super` is a keyword, outside this bare-Identifier census | Do not apply a bare-name refusal to it. Retain existing behavior until separate H contract; no claim the broad E5 witness is solved |

The owner’s implementation order is feasible. First select the nearest lexical declaration, then dispatch its local/parameter/import category. Searching all local declarations before inspecting a nearer parameter would be wrong; “local first” is not permission to violate shadowing. No reordering to imports-first is necessary: L2 can reuse the existing import/export resolver at its terminal, then the import phase broadens those shared routes.


## 1d. Remaining repo-name fallback

| Corpus | Current free_single | Current groups / partition | After local+parameter+import | Proved script-global correct | Proved accidental remainder | Unknown remainder |
| --- | --- | --- | --- | --- | --- | --- |
| X | 31 | {"local\|product": 30, "unresolved\|product": 1} | 1 | 0 | 0 | 1 |
| T | 203 | {"global_ambient_library\|product": 4, "import\|product": 2, "local\|product": 175, "local\|test": 20, "unresolved\|test": 2} | 6 | 0 | 4 | 2 |

Source: `part1d`. After consuming the three binding families, X retains one unresolved product site. T retains four wrong ambient product calls and two unresolved test calls. The literal no-by-name choice sacrifices **0 proved CORRECT public corpus free_single edges**, but also drops X 12 / T 2 undecided-or-inadmissible baseline targets across the whole rung, and the independently demonstrated script-global synthetic inputs. No absence-of-counterexample claim closes that cost. Recommend removing unsupported repo-name edges, with a separately approved script-global language-binding rule if those valid inputs must stay supported. That exception is not “there is only one function in the repo.”

Removing repo-wide name search also reaches `free_multi`: X 3 unknown local sites / 9 NameOnly edges; T 60 sites / 363 NameOnly edges, consisting of 59 unknown sites / 360 edges plus the one wrong import-equals site / 3 edges corrected by I3. These additional unknown exposures require an explicit owner decision. Do not merely demote arbitrary candidates: NameOnly still inserts wrong topology, so demotion does not preserve E5’s removed-target result. Nor should a new lookup erase every earlier rung when this checker cannot follow it: X has 3,112 existing Exact import sites with unknown oracle endpoints, chiefly nonrelative paths under the unchanged compiler options. Retain the product’s existing positive path/import proofs.

Broader “drop every bound unknown local edge” would affect X 124 sites (19 W + 105 U; 140 Exact and 9 NameOnly edges), T 430 sites (195 W + 235 U; 396 Exact and 296 NameOnly edges), including earlier same-file/local rungs. This plan limits the final negative change to unsupported repo-name fallback; a broader resolver rewrite needs its own adjudication. Source: `policy_exposure`.


## Separate no-record population — not resolution yield

**T has 15,514 non-test sites in other named-callable contexts with no Prism record. This is a distinct source-admission cause, far above a few hundred.** All are in `compiler/checker.ts` (3,151,774 bytes), which exceeds the product loader’s 2 MiB limit (`src/repo_loader.rs:13`, `load_one_file` TooLarge branch near 1075). The unchanged checker Program loads it transitively; Prism’s full capture has zero records from it. The source contributes 16,546 no-record bare sites in total. Source: `PROBE-LOG.md` P4, `part1a.no_record`, `files.json`; counts below are outside the join denominator.

| Corpus | No record | P / Tst | Module P / Tst | Anonymous P / Tst | Other P / Tst |
| --- | --- | --- | --- | --- | --- |
| X | 12749 | 2,895 / 9,854 | 1,286 / 220 | 1,609 / 9,634 | 0 / 0 |
| T | 27398 | 19,037 / 8,361 | 403 / 343 | 3,120 / 8,018 | 15,514 / 0 |

| Corpus | No-record binding | Product | Test |
| --- | --- | --- | --- |
| X | global_ambient_library | 53 | 15 |
| X | import.default | 49 | 9 |
| X | import.named | 1005 | 2796 |
| X | import.renamed | 1 | 0 |
| X | import.require | 3 | 0 |
| X | import.require.destructured | 11 | 0 |
| X | local.const.arrow | 406 | 683 |
| X | local.const.call_result | 58 | 7 |
| X | local.const.destructure.await | 2 | 31 |
| X | local.const.destructure.call_result | 180 | 7 |
| X | local.const.destructure.identifier | 11 | 0 |
| X | local.const.destructure.identifier_member | 1 | 0 |
| X | local.const.element_member | 2 | 0 |
| X | local.let.reassigned.other | 6 | 0 |
| X | parameter.destructured | 157 | 1 |
| X | parameter.destructured.default | 1 | 0 |
| X | parameter.plain | 33 | 35 |
| X | same_file.function | 58 | 119 |
| X | unresolved | 858 | 6151 |
| T | global_ambient_library | 13 | 14 |
| T | import.equals | 0 | 25 |
| T | import.named | 6196 | 2610 |
| T | import.renamed | 11 | 0 |
| T | local.const.arrow | 200 | 9 |
| T | local.const.call_result | 31 | 257 |
| T | local.const.conditional | 11 | 4 |
| T | local.const.destructure.call_result | 2 | 147 |
| T | local.const.destructure.identifier | 4 | 0 |
| T | local.const.destructure.identifier_member | 0 | 7 |
| T | local.const.destructure.none | 1 | 2 |
| T | local.const.destructure.other | 0 | 72 |
| T | local.const.element_member | 1 | 0 |
| T | local.const.none | 3 | 0 |
| T | local.let.reassigned.none | 2 | 24 |
| T | local.var.never_reassigned.call_result | 45 | 0 |
| T | local.var.never_reassigned.none | 0 | 41 |
| T | local.var.reassigned.arrow | 32 | 0 |
| T | local.var.reassigned.none | 2 | 0 |
| T | parameter.destructured | 9 | 11 |
| T | parameter.plain | 232 | 77 |
| T | parameter.plain.default | 2 | 0 |
| T | same_file.function | 12240 | 2864 |
| T | unresolved | 0 | 2197 |

| Corpus | Partition | Other callable shape | Sites |
| --- | --- | --- | --- |
| T | product | ArrowFunction:inferred_BinaryExpression | 1 |
| T | product | ArrowFunction:inferred_CallExpression | 182 |
| T | product | ArrowFunction:inferred_PropertyAssignment | 166 |
| T | product | ArrowFunction:inferred_VariableDeclaration | 15 |
| T | product | FunctionDeclaration:named | 14998 |
| T | product | FunctionExpression:named | 63 |
| T | product | MethodDeclaration:named | 89 |

Closing this separate gap would require a source-admission decision for oversized files and caller identities for module-level/anonymous code, with downstream graph ownership contracts; binding resolution cannot attach an edge without such a caller. No extraction fix is designed here. The merged PR-B callback names are DFG-only and explicitly cannot be reused as FunctionIds. Test labels are `.test.` or `/tests/`, plus T `testRunner/`; X recomputation on both old and new rows gives 9,926 product / 289 test recorded sites, one different from the ruling’s 9,925 / 290, while its 2,895 / 9,854 no-record split matches exactly. `test-split-reconciliation.json` records that discrepancy without changing the gate.


## 1e. Reusable machinery and missing proof

| Source / function | Already available | Missing / proposed use |
| --- | --- | --- |
| `src/ast/js_binding.rs`: `JsBinding`, `JsTerminal`, `js_ts_classify` | Callable span/name/wrapper; MayCall, Refused, Unchecked, Import, Alias states; own initializers | Alias currently lowers to MayCall("alias"); retain declaration identity and follow a bounded graph |
| `src/ast/js_binding_site.rs`: `js_ts_site_binding`, `js_ts_scope_lookup`; `js_binding_walk.rs` | Nearest declaration/scope walk, explicit environments, Position refusals | Dispatch families after nearest-binding selection; preserve parameter-default/body boundary; never equate Position with unbound |
| `js_binding_checks.rs`, `js_binding_writes.rs` | Grammar/parse gates and scoped assignment evidence | Reuse every conjunct on every followed declaration, including nested writes, duplicate bindings, eval/with uncertainty |
| `js_binding_namespace.rs`: `js_ts_namespace_binding` | Namespace-import identity, type-only/shadow/write checks | Expose reusable value identity to local member/destructure and import-equals aliases |
| `js_binding_values.rs`: `js_ts_classify_call`, wrapper provenance; `js_wrapped_export.rs`: `js_ts_wrapped_export` | Import-proven React wrapper, arity, spread and JSX restrictions, exact inline spans | Permit an identifier first argument only after bounded terminal proof; retain wrapper mode across aliases |
| `src/js_exports.rs`: `JsExportTarget`, `MAX_REEXPORT_DEPTH` | Local, VerifiedLocal, SpannedLocal, ReExport, ImportForward; conflict/cycle/type-only evidence; current cap 2 | Preserve ambiguity evidence; reuse terminal file+span. Separate export-alias admission from optional cap change |
| `js_module_forwarding.rs`: `js_ts_forwarded_import`, `js_ts_forwardable_functions` | Static forwarding of top-level function declarations | Prove const-arrow/alias exports instead of only direct named declarations |
| `js_cjs_terminal.rs`: `js_ts_cjs_local_target`; `js_cjs_export_barriers.rs` | Unique unwritten CJS callable, value and conflict barriers; UnprovenLocal claims | Retain existing CJS route proofs; no blanket require/factory or escaped-object admission |
| `src/call_graph.rs`: `apply_js_path_resolution`, `resolve_js_ts_member_module`, `apply_js_export_resolution`; relative candidates near 6747 | Project path snapshots, module resolution and export summaries | Explicit extension substitution and directory normalization; reuse guarded tsconfig route rather than this oracle’s unresolved status |
| `src/call_graph.rs`: `js_local_binding_at` near 5710; `recompute_indirect_calls` | Source-byte binder integration; separate IndirectResolution records | Keep Source-only scope; add file-bearing terminal identities for cross-file alias targets; do not re-admit Step1 indirect sites |
| `src/resolution.rs`: `resolve_call_site`, `js_ts_local_callable` near 3685, `js_ts_import_member_candidates` near 3790 | Local callable before R4c import-member, then same-file R4, then R5 free_single/free_multi | Positive local/parameter/import outcomes precede repo-name rung. Negative fallback refusal belongs at R5; do not erase established positive routes |
| `src/call_graph_js_referenceability.rs`: `module_binding_evidence`, `free_single_refusals` | Parked E5 negative target/caller evidence, additional-import spelling facts | Syntax exemptions are not callable proof; retire broad second-arm behavior as the replacement contract is adopted |
| `src/cpg_cache.rs` CACHE_VERSION 115; `src/navigation/call_edge_cache.rs` version 62 | Persisted source facts and nav call outcomes | Bump whichever serialized facts/outcomes change; pin exact versions in registry/tests and lifecycle gates |
| `docs/.../2026-10-06-js-param-defs/SPEC-prA.md` D12 | Outgoing new-formal DataFlow refusal for cross-file Exact free_single, independent of syntax | Keep the provenance firewall until separately justified; alias rule names must not silently launder old by-name argument edges |
| `SPEC-prB.md` B-D1/B-D5 | Synthetic callback identity belongs only to DFG; no callable graph endpoint | Argument-following must use actual registered FunctionIds; no anonymous identity expansion in this plan |

Line references bind the planning checkout `33c9c1d6305dbd9e100f9f925fc8638320b03adc`; it contains the parked candidate, whereas the frozen base measures main behavior. Source inspection is an inventory, not evidence that the proposed additions already exist. CLAUDE.md’s Exact contract remains static binding, not runtime dispatch or execution-order proof.


## Method, reproduction, admissibility and limits

`run-public.sh` runs the copied pinned earlier oracle with its original compiler options, then `census.cjs`, then `summarize.py`, first X then T. The Program uses allowJs/checkJs, types=[], JSX preserve, ESNext target/module, Bundler resolution, skipLibCheck and noEmit. Root source walking keeps the old hidden/dependency/build/symlink/2 MiB rules; enumeration additionally includes already-loaded in-root transitive source files without changing the Program. X has 628 root names and 631 enumerated files, including three transitively loaded JSON files with no bare call sites; T has 707 root names and 708 enumerated files. The oversized declaration-only T lib file is absent from this Program. No corpus package is executed. No tsconfig-mode substitution was used to improve the oracle. Unknown nonrelative imports stay unknown.

`census.cjs` joins physical UTF-8 spans (BOM-safe) with exact Source origin and callee spelling. JSX opening and enclosing-element span conventions are both indexed. Bare CallExpression/JSX identifiers are included; constructors, `super`, property calls and IndirectResolution are outside the measured population. Binding follows checker declarations plus syntax-based immutable aliases, namespace/static-object members, flat object destructuring, transparent TS wrappers and specifically import-proven React wrappers. Writes, parser/binder ambiguity, cycles, unmodeled factories, destructuring defaults, escaped/mutated objects, spreads/accessors and unsupported value forms remain unknown. A signature is not a value identity. Default export expressions without a stable registered function name do not inflate gains.

Current edges are compared with actual target file/name/start/end identities, cross-checked against a unique callable AST identity and the retained frozen named-function inventory. Earlier E5 WRONG facts remain load-bearing for 19 X / 195 T local sites whose eventual value is unknown. Fresh declaration-only mismatches on other rungs are not automatically elevated to WRONG: alias/export/wrapper value tracing can explain them. The corrected values and traces are retained. This is static binding evidence, not runtime mutability proof or a TypeScript checker certification of an invalid program.

The module route diagnostic uses checker module resolution and a shortest bounded syntax path; it counts re-export hops but does not prove all conflicting routes absent. Its disjoint allocation is a ceiling; a product proof may safely realize less. Synthetic frozen-base `.js` versus extensionless imports to the same target discriminate path substitution from a general export failure: the former drops UnknownName, the latter is Exact import_member. This is a same-environment control, not attribution from a green run elsewhere. No claim that every T miss has only that cause is made.

`check-census.cjs` checks 38 synthetic behaviors, including positive alias/member/wrapper/dynamic cases and negative shadow/write/default/factory/escape/accessor/duplicate/type-only cases, plus real base outcomes and partition invariants. Final result: 38 passed, 0 failed. The earlier empty control and failed nullable-summary run are retained and explicitly inadmissible in `PROBE-LOG.md`; the duplicate-binding failure was repaired with a binder-diagnostic guard. These are tests of measurement tooling, not implementation RED/GREEN tests.


### Controller census and parity

`CONTROLLER-census.sh` requires CORPUS_ROOT and OUT_DIR (new or empty, beneath step0b), sets umask 077, verifies both frozen binary hashes, and runs at most two producers concurrently with a separate fresh cache per command. Normal operation captures base sites/functions and head sites, runs the pinned checker and prints only the aggregate 1a/1b/1d tables. File names, identifiers, commands, source text, rows and errors remain in owner-only files beneath OUT_DIR. The controller may invoke its private-capable mode; this planner did not open private sources. Public X parity explicitly reused the authorized X capture plus retained base inventory/head rows; this is not misrepresented as a fresh X capture. Source hashes are checked against the old manifest.

Reproduction (controller chooses an authorized root; do not substitute a private root in this planner):

```sh
CORPUS_ROOT=/path/to/authorized/corpus \
OUT_DIR=/Users/wesleyjinks/prism-evidence/js-alias/step0b/controller-new \
sh docs/superpowers/plans/2026-10-13-js-alias-following/CONTROLLER-census.sh
```

Exact public parity command, script hashes, aggregate hashes and permissions results are recorded in `controller-parity-final.json` and `controller-X-complete/controller.receipt.json`. The default producer path is exercised on `controller-synthetic-complete/`, with fresh base-sites, base-functions and step2b-head caches. The original X capture is reused as requested; fresh T base capture is in `T/base.receipt.json`. Reused inputs and base multisets are bound in `reused-captures.json` and `{X,T}/prior-base-parity.json`.


### What remains unverified

No product code, tests or mutants were edited. No alias resolver prototype, product full suite, build, mutation suite, Tier-A matrix/quick/full-corpus run, private census, third public corpus, browser, package execution, network or Git write was performed. Realized gains/losses, path-conflict outcomes, performance, cache lifecycle and the private corpus’s new positive/unknown loss counts require implementation/controller execution. The private E5 109 WRONG / 0 CORRECT-lost figure and step2→step2b corpus equality are inherited from the brief/required packet. The synthetic script-global and super loss mechanisms remain design obligations, not closed regressions. No new product bug severity is inferred from a failed tooling probe.


## History: the original STOP and controller ruling

Step0 correctly stopped when only 10,215 of X’s 22,964 checker sites joined (12,749 no record, 55.52%). The controller’s Step0b ruling changed Parts 1a/1b/1d to Source-recorded sites and retained the 3% reverse-join STOP. That authorized continuation, not a reinterpretation of the original gate. The old evidence and STOP snapshot remain under step0; these complete documents replace the blocked worktree documents. Both revised corpus gates pass; checker execution uses the prior oracle’s Program construction, and the owner’s local→parameter→import order is implementable with lexical precedence. **No current brief STOP was triggered.** The previously exhausted two-round E5 review cap is not reset by this planning work.

