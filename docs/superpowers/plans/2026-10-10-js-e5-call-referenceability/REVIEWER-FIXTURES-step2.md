# Reviewer fixture three-way results

All calls statically analyzed; no fixture packages executed. Full targets/spans/origin/drop values and commands are retained in reviewer-fixtures.json and per-fixture artifacts.

| Fixture | Main/base | Step1 | Step2 |
|---|---|---|---|
| opus/d12s | Source:escape exact free_single callee.js:escape@1 | Source:escape exact free_single callee.js:escape@1 | Source:escape exact free_single callee.js:escape@1 |
| opus/life1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/m1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/m2 | Source:F exact free_single callee.jsx:F@1 | Source:F exact free_single callee.jsx:F@1 | Source:F exact free_single callee.jsx:F@1 |
| opus/m3 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/t01 | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.js:f@1 |
| opus/t02 | Source:f exact free_single globals.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single globals.ts:f@1 |
| opus/t03 | Source:f exact free_single globals.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single globals.js:f@1 |
| opus/t04 | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.ts:f@1 |
| opus/t04b | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.js:f@1 |
| opus/t04c | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.ts:f@1 |
| opus/t05 | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.js:f@1 |
| opus/t06 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/t07 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/t08 | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.ts:f@1 |
| opus/t09 | Source:f drop UnknownName<br>Source:import drop UnknownName | Source:f drop UnknownName<br>Source:import drop UnknownName | Source:f drop UnknownName<br>Source:import drop UnknownName |
| opus/t10 | Source:f exact import_member callee.js:f@1 | Source:f exact import_member callee.js:f@1 | Source:f exact import_member callee.js:f@1 |
| opus/t11 | Source:f exact import_member callee.js:g@1 | Source:f exact import_member callee.js:g@1 | Source:f exact import_member callee.js:g@1 |
| opus/t12 | Source:f exact import_qualified callee.js:f@1 | Source:f exact import_qualified callee.js:f@1 | Source:f exact import_qualified callee.js:f@1 |
| opus/t13 | Source:f exact local_def caller.js:f@2 | Source:f exact local_def caller.js:f@2 | Source:f exact local_def caller.js:f@2 |
| opus/t14 | Source:f drop UnknownName<br>Source:require drop UnknownName | Source:f drop UnknownName<br>Source:require drop UnknownName | Source:f drop UnknownName<br>Source:require drop UnknownName |
| opus/t15 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| opus/t16 | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| opus/t16b | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| opus/t17 | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| opus/t18 | Source:Item drop UnknownName<br>IndirectResolution:MenuItem exact free_single MenuItem.jsx:MenuItem@1 | Source:Item drop UnknownName<br>IndirectResolution:MenuItem drop LocalBindingUnproven | Source:Item drop UnknownName<br>IndirectResolution:MenuItem exact free_single MenuItem.jsx:MenuItem@1 |
| opus/t19 | Source:Item drop UnknownName<br>IndirectResolution:MenuItem exact import_member MenuItem.jsx:MenuItem@1 | Source:Item drop UnknownName<br>IndirectResolution:MenuItem exact import_member MenuItem.jsx:MenuItem@1 | Source:Item drop UnknownName<br>IndirectResolution:MenuItem exact import_member MenuItem.jsx:MenuItem@1 |
| opus/t20 | IndirectResolution:helper exact free_single helper.js:helper@1<br>Source:run drop UnknownName | IndirectResolution:helper drop LocalBindingUnproven<br>Source:run drop UnknownName | IndirectResolution:helper exact free_single helper.js:helper@1<br>Source:run drop UnknownName |
| opus/t21 | no caller/entry/run call rows | no caller/entry/run call rows | no caller/entry/run call rows |
| opus/t22 | no caller/entry/run call rows | no caller/entry/run call rows | no caller/entry/run call rows |
| opus/t23 | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.ts:f@1 |
| opus/t24 | Source:f exact free_single ns.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single ns.ts:f@1 |
| opus/t25 | Source:g drop UnknownName | Source:g drop UnknownName | Source:g drop UnknownName |
| opus/t26 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/t27 | Source:f exact free_single globals.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single globals.js:f@1 |
| opus/t28 | Source:F exact free_single globals.tsx:F@1 | Source:F drop LocalBindingUnproven | Source:F exact free_single globals.tsx:F@1 |
| opus/t29 | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.js:f@1 |
| opus/t29r | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| opus/t30 | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| opus/t31 | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| opus/t32 | Source:f exact free_single globals.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single globals.js:f@1 |
| opus/t33 | Source:f exact free_single globals.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single globals.ts:f@1 |
| opus/w2d1 | Source:escape exact free_single callee.js:escape@1 | Source:escape drop LocalBindingUnproven | Source:escape drop LocalBindingUnproven |
| opus/w2d2 | Source:escape exact free_single callee.js:escape@1 | Source:escape drop LocalBindingUnproven | Source:escape drop LocalBindingUnproven |
| opus/w2d3 | Source:escape exact free_single callee.js:escape@1 | Source:escape drop LocalBindingUnproven | Source:escape drop LocalBindingUnproven |
| opus/w2m1 | Source:Array exact free_single callee.js:Array@1 | Source:Array drop LocalBindingUnproven | Source:Array drop LocalBindingUnproven |
| opus/w2m2 | Source:Array exact free_single callee.js:Array@1 | Source:Array drop LocalBindingUnproven | Source:Array drop LocalBindingUnproven |
| opus/w2m3 | Source:Array exact free_single callee.js:Array@1 | Source:Array drop LocalBindingUnproven | Source:Array drop LocalBindingUnproven |
| sol/ambient | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| sol/arm-a-alias-control | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| sol/b5-bare-jsx | Source:f exact free_single lib.jsx:f@1<br>Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-bare-tsx | Source:f exact free_single lib.tsx:f@1<br>Source:f exact free_single lib.tsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-const | Source:f exact free_single lib.jsx:f@1<br>Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-if-jsx | Source:f exact free_single lib.jsx:f@1<br>Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-if-tsx | Source:f exact free_single lib.tsx:f@1<br>Source:f exact free_single lib.tsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-import-jsx | Source:f exact import_member lib.jsx:f@1<br>Source:f exact import_member lib.jsx:f@1 | Source:f exact import_member lib.jsx:f@1<br>Source:f exact import_member lib.jsx:f@1 | Source:f exact import_member lib.jsx:f@1<br>Source:f exact import_member lib.jsx:f@1 |
| sol/b5-import-tsx | Source:f exact import_member lib.tsx:f@1<br>Source:f exact import_member lib.tsx:f@1 | Source:f exact import_member lib.tsx:f@1<br>Source:f exact import_member lib.tsx:f@1 | Source:f exact import_member lib.tsx:f@1<br>Source:f exact import_member lib.tsx:f@1 |
| sol/b5-module-jsx | Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| sol/b5-module-tsx | Source:f exact free_single lib.tsx:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| sol/b5-module_var-jsx | Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| sol/b5-module_var-tsx | Source:f exact free_single lib.tsx:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| sol/b5-same_line-jsx | Source:f exact free_single lib.jsx:f@1<br>Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-same_line-tsx | Source:f exact free_single lib.tsx:f@1<br>Source:f exact free_single lib.tsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-var-jsx | Source:f exact free_single lib.jsx:f@1<br>Source:f exact free_single lib.jsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/b5-var-tsx | Source:f exact free_single lib.tsx:f@1<br>Source:f exact free_single lib.tsx:f@1 | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven<br>Source:f drop LocalBindingUnproven |
| sol/cjs_extension | Source:f exact free_single global.js:f@1 | Source:f exact free_single global.js:f@1 | Source:f exact free_single global.js:f@1 |
| sol/default | Source:f exact import_member callee.js:f@1 | Source:f exact import_member callee.js:f@1 | Source:f exact import_member callee.js:f@1 |
| sol/dynamic | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.js:f@1 |
| sol/empty_export | Source:f exact free_single global.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single global.js:f@1 |
| sol/export_equals | Source:f exact free_single global.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single global.ts:f@1 |
| sol/import_equals | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.ts:f@1 |
| sol/import_equals_only | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single callee.ts:f@1 |
| sol/indirect_root_control | Source:Box exact import_member box.js:Box@1 | Source:Box exact import_member box.js:Box@1 | Source:Box exact import_member box.js:Box@1 |
| sol/jsx_indirect | Source:Item drop ImportExternal | Source:Item drop ImportExternal | Source:Item drop ImportExternal |
| sol/jsx_indirect_renamed | IndirectResolution:Component exact free_single item.tsx:Component@1<br>Source:Item drop UnknownName | IndirectResolution:Component drop LocalBindingUnproven<br>Source:Item drop UnknownName | IndirectResolution:Component exact free_single item.tsx:Component@1<br>Source:Item drop UnknownName |
| sol/mjs_extension | Source:f exact free_single global.js:f@1 | Source:f exact free_single global.js:f@1 | Source:f exact free_single global.js:f@1 |
| sol/module_global_js | Source:f exact free_single global.js:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single global.js:f@1 |
| sol/module_global_ts | Source:f exact free_single global.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f exact free_single global.ts:f@1 |
| sol/namespace | Source:f exact import_qualified callee.js:f@1 | Source:f exact import_qualified callee.js:f@1 | Source:f exact import_qualified callee.js:f@1 |
| sol/nested_require | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 | Source:f exact free_single callee.js:f@1 |
| sol/ordinary_indirect | Source:Item drop UnknownName<br>IndirectResolution:f exact free_single item.js:f@1 | Source:Item drop UnknownName<br>IndirectResolution:f drop LocalBindingUnproven | Source:Item drop UnknownName<br>IndirectResolution:f exact free_single item.js:f@1 |
| sol/reexport | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| sol/renamed | Source:f exact import_member callee.js:a@1 | Source:f exact import_member callee.js:a@1 | Source:f exact import_member callee.js:a@1 |
| sol/require_peer_control | Source:f exact free_single callee.ts:f@1 | Source:f exact free_single callee.ts:f@1 | Source:f exact free_single callee.ts:f@1 |
| sol/same_file | Source:f exact local_def caller.js:f@1 | Source:f exact local_def caller.js:f@1 | Source:f exact local_def caller.js:f@1 |
| sol/script_global_js | Source:f exact free_single global.js:f@1 | Source:f exact free_single global.js:f@1 | Source:f exact free_single global.js:f@1 |
| sol/type_only | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| step2/import_equals_renamed | Source:renamed drop UnknownName | Source:renamed drop UnknownName | Source:renamed drop UnknownName |
| step2/module_global_local_alias | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| step2/module_global_parameter | Source:f drop UnknownName | Source:f drop UnknownName | Source:f drop UnknownName |
| step2/module_module | Source:f exact free_single callee.js:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |
| step2/source_jsx_alias | Source:F exact free_single callee.tsx:F@1 | Source:F drop LocalBindingUnproven | Source:F drop LocalBindingUnproven |
| step2/unrelated_import_equals | Source:f exact free_single callee.ts:f@1 | Source:f drop LocalBindingUnproven | Source:f drop LocalBindingUnproven |

| Additional R1 terminal-trivia audit | Exact free_single ns.ts:f@1 | NOT RUN after STOP | drop LocalBindingUnproven — WRONG / STOP |
