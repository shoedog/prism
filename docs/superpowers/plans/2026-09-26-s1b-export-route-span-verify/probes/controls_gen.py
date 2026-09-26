# Negative/positive control fixtures for wrapped-export planning probes (P3).
# Each scenario is its own tiny repo so resolution cannot cross-contaminate.
import os, json
S = {}
APP_ISLAND = "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"
S["C01_plain_arrow"] = {
 "lib.tsx": "export const A = (props: any) => <div/>;\n",
 "app.tsx": "import { A } from './lib';\nexport function App() {\n  return <A/>;\n}\n"}
S["C02_forwardref_named"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C03_forwardref_samefile"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\nexport function Host() {\n  return <Island/>;\n}\n"}
S["C04_forwardref_exportlist"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nconst Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\nexport { Island };\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C05_forwardref_default_ident"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nconst Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\nexport default Island;\n",
 "app.tsx": "import Island from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C06_F4_hazard_exportlist"] = {
 "util.ts": "function outer(): number {\n  function f(): number { return 99; }\n  return f();\n}\nconst a = 1;\nconst b = 2;\nconst f = a > 0 ? a : b;\nexport { f };\n",
 "app.ts": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S["C07_F4_hazard_declarator"] = {
 "util.ts": "function outer(): number {\n  function f(): number { return 99; }\n  return f();\n}\nconst a = 1;\nconst b = 2;\nexport const f = a > 0 ? a : b;\n",
 "app.ts": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S["C08_memo_forwardref"] = {
 "lib.tsx": "import { memo, forwardRef } from 'react';\nexport const Island = memo(forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n}));\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C09_forwardref_ident_arg"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nfunction IslandInner(props: any, ref: any) {\n  return <div ref={ref}/>;\n}\nexport const Island = forwardRef(IslandInner);\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C10_ident_arg_nested_samename"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nfunction IslandInner(props: any, ref: any) {\n  return <div ref={ref}/>;\n}\nfunction helper() {\n  function Island() { return 1; }\n  return Island();\n}\nexport const Island = forwardRef(IslandInner);\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C11_wrapped_plus_nested_samename"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nfunction helper() {\n  function Island() { return 1; }\n  return Island();\n}\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C12_local_forwardref_impostor"] = {
 "lib.tsx": "function forwardRef(f: any) {\n  return () => null;\n}\nexport const Island = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C13_store_factory"] = {
 "store.ts": "import { create } from 'zustand';\nexport const useStore = create((set: any) => ({\n  count: 0,\n}));\n",
 "app.ts": "import { useStore } from './store';\nexport function run() {\n  return useStore();\n}\n"}
S["C14_reexport_chain"] = {
 "Island.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "index.ts": "export { Island } from './Island';\n",
 "app.tsx": "import { Island } from './index';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C15_react_ns_named_fe"] = {
 "lib.tsx": "import React from 'react';\nexport const Island = React.forwardRef(function Island(props: any, ref: any) {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C16_styled"] = {
 "lib.tsx": "import styled from 'styled-components';\nexport const Box = styled.div`\n  color: red;\n`;\n",
 "app.tsx": "import { Box } from './lib';\nexport function App() {\n  return <Box/>;\n}\n"}
S["C17_multi_declarator"] = {
 "lib.tsx": "import { forwardRef, memo } from 'react';\nexport const A = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n}), B = memo((props: any) => {\n  return <span/>;\n});\n",
 "app.tsx": "import { A, B } from './lib';\nexport function App() {\n  return <div><A/><B/></div>;\n}\n"}
S["C18_importer_shadow"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App(Island: any) {\n  return <Island/>;\n}\n"}
S["C19_observer_hoc"] = {
 "lib.tsx": "import { observer } from 'mobx-react-lite';\nexport const Panel = observer((props: any) => {\n  return <div/>;\n});\n",
 "app.tsx": "import { Panel } from './lib';\nexport function App() {\n  return <Panel/>;\n}\n"}
S["C20_let_reassigned"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport let Island = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\nfunction Other() { return null; }\nIsland = Other as any;\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C21_let_reassigned_plain_arrow"] = {
 "lib.tsx": "export let A = (props: any) => <div/>;\nfunction Other() { return null; }\nA = Other as any;\n",
 "app.tsx": "import { A } from './lib';\nexport function App() {\n  return <A/>;\n}\n"}
S["C22_default_object_alias"] = {
 "Stack.tsx": "const RowStack = (props: any) => <div/>;\nconst ColStack = (props: any) => <div/>;\nexport default { Row: RowStack, Col: ColStack };\n",
 "app.tsx": "import Stack from './Stack';\nexport function App() {\n  return <Stack.Row/>;\n}\n"}
S["C23_debounce_callable"] = {
 "util.ts": "import debounce from 'lodash/debounce';\nexport const save = debounce((x: number) => {\n  return x;\n}, 100);\n",
 "app.ts": "import { save } from './util';\nexport function run() {\n  save(1);\n}\n"}
S["C24_arrow_second_arg"] = {
 "sel.ts": "import { createSelector } from 'reselect';\nconst a = (s: any) => s.a;\nexport const selectA = createSelector(a, (x: any) => {\n  return x;\n});\n",
 "app.ts": "import { selectA } from './sel';\nexport function run(s: any) {\n  return selectA(s);\n}\n"}
S["C25_forwardref_as_cast"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n}) as any;\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C26_react_default_arrow"] = {
 "lib.tsx": "import React from 'react';\nexport const Island = React.forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": APP_ISLAND}
S["C27_react_ns_arrow"] = {
 "lib.tsx": "import * as React from 'react';\nexport const Island = React.forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": APP_ISLAND}
# ---- r2 controls (sol spec round 1) ----
def lib(pre, post=""):
    return pre + "export const Island = React.forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n" + post
S["C28_default_member_write"] = {"lib.tsx": lib("import React from 'react';\nReact.forwardRef = function fake(): any {\n  return function Replacement() { return null; };\n} as any;\n"), "app.tsx": APP_ISLAND}
S["C29_import_parse_recovery"] = {
 "lib.tsx": "import { forwardRef as fr ??? } from 'react';\nexport const Island = fr((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": APP_ISLAND}
S["C30_memo_sameline_comparator"] = {
 "lib.tsx": "import { memo } from 'react';\nexport const Island = memo((p: any) => <div/>, (a: any, b: any) => true);\n",
 "app.tsx": APP_ISLAND}
S["C31_direct_call"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return (Island as any)({});\n}\nexport function App2() {\n  return Island({} as any, null as any);\n}\n"}
S["C32_new_expression"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import { Island } from './lib';\nexport function App() {\n  return new (Island as any)();\n}\nexport function App2() {\n  return <Island/>;\n}\n"}
S["C33_object_assign_escape"] = {"lib.tsx": lib("import React from 'react';\nObject.assign(React, { forwardRef: (f: any) => f });\n"), "app.tsx": APP_ISLAND}
S["C34_alias_escape"] = {"lib.tsx": lib("import React from 'react';\nconst R: any = React;\nR.forwardRef = (f: any) => f;\n"), "app.tsx": APP_ISLAND}
S["C35_namespace_member_write"] = {"lib.tsx": lib("import * as React from 'react';\n(React as any).forwardRef = (f: any) => f;\n"), "app.tsx": APP_ISLAND}
S["C36_require_react_handle"] = {"lib.tsx": lib("import React from 'react';\nrequire('react').forwardRef = (f: any) => f;\n"), "app.tsx": APP_ISLAND}
S["C37_named_form_default_write"] = {
 "lib.tsx": "import React, { forwardRef } from 'react';\n(React as any).forwardRef = (f: any) => f;\nexport const Island = forwardRef((props: any, ref: any) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": APP_ISLAND}
S["C38_benign_react_uses"] = {
 "lib.tsx": "import React from 'react';\ntype P = React.ComponentProps<'div'>;\nconst x: typeof React | null = null;\nexport const Island = React.forwardRef<HTMLDivElement, P>((props, ref) => {\n  const [s] = React.useState(0);\n  return <React.Fragment><div ref={ref}>{s}</div></React.Fragment>;\n});\nIsland.displayName = 'Island';\n",
 "app.tsx": APP_ISLAND}
S["C39_star_barrel_span_conflict"] = {
 "impl.tsx": "import { memo } from 'react';\nexport const A = memo(function Same(p: any) {\n  return <div/>;\n});\nexport const B = memo(function Same(p: any) {\n  return <span/>;\n});\n",
 "a.ts": "export { A as X } from './impl';\n",
 "b.ts": "export { B as X } from './impl';\n",
 "index.ts": "export * from './a';\nexport * from './b';\n",
 "app.tsx": "import { X } from './index';\nexport function App() {\n  return <X/>;\n}\n"}
S["C40_delete_member"] = {"lib.tsx": lib("import React from 'react';\ndelete (React as any).memo;\n"), "app.tsx": APP_ISLAND}
S["C41_logical_assign"] = {"lib.tsx": lib("import React from 'react';\n(React as any).forwardRef ||= (f: any) => f;\nReact.forwardRef ??= React.forwardRef;\n"), "app.tsx": APP_ISLAND}
S["C42_destructuring_write"] = {"lib.tsx": lib("import React from 'react';\nlet fake: any;\n[React.forwardRef] = [fake];\n"), "app.tsx": APP_ISLAND}
S["C43_subscript_write"] = {"lib.tsx": lib("import React from 'react';\n(React as any)['forwardRef'] = (f: any) => f;\n"), "app.tsx": APP_ISLAND}
S["C44_jsx_js_positive"] = {
 "lib.jsx": "import React from 'react';\nexport const Island = React.memo((props) => {\n  return <div/>;\n});\n",
 "app.jsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C45_eval_in_file"] = {"lib.tsx": lib("import React from 'react';\neval('1');\n"), "app.tsx": APP_ISLAND}
S["C46_member_write_other_prop"] = {"lib.tsx": lib("import React from 'react';\n(React as any).customThing = 1;\n"), "app.tsx": APP_ISLAND}
S["C47_update_write"] = {"lib.tsx": lib("import React from 'react';\n(React as any).x = 0;\n") .replace("(React as any).x = 0;", "React.version++;"), "app.tsx": APP_ISLAND}
S["C48_for_of_write"] = {"lib.tsx": lib("import React from 'react';\nfor (React.version of ['1']) {}\n"), "app.tsx": APP_ISLAND}
S["C49_shorthand_escape"] = {"lib.tsx": lib("import React from 'react';\nexport const bag = { React };\n"), "app.tsx": APP_ISLAND}
S["C50_export_clause_escape"] = {"lib.tsx": lib("import React from 'react';\nexport { React };\n"), "app.tsx": APP_ISLAND}
S["C51_escaped_spelling"] = {"lib.tsx": lib("import React from 'react';\nconst v = Re\\u0061ct;\n"), "app.tsx": APP_ISLAND}
S["C52_with_js"] = {
 "lib.jsx": "import React from 'react';\nwith (Math) { max(1, 2); }\nexport const Island = React.memo((props) => {\n  return <div/>;\n});\n",
 "app.jsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
# ---- Branch P controls (owner 2026-09-25; sol r2 W1-W3 as model-boundary MB tests; r2 SMELL folds) ----
S["C53_MB2_named_require_write"] = {
 "lib.jsx": "import { forwardRef } from 'react';\n\nfunction fake() {\n  return function Replacement() { return null; };\n}\n\nrequire('react').forwardRef = fake;\nexport const Island = forwardRef((props, ref) => <div ref={ref} />);\n",
 "app.jsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C54_MB3_define_getter"] = {
 "lib.jsx": "import React from 'react';\n\nfunction fake() {\n  return function Replacement() { return null; };\n}\n\nReact.__defineGetter__('forwardRef', () => fake);\nexport const Island = React.forwardRef((props, ref) => <div ref={ref} />);\n",
 "app.jsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
S["C55_P2_type_only_collision"] = {
 "lib.tsx": "import { memo } from 'react';\nimport type { T as memo } from './types';\nexport const Island = memo((props: any) => {\n  return <div/>;\n});\n",
 "app.tsx": APP_ISLAND}
S["C56_TN18_destructured_require"] = {
 "lib.jsx": "const { forwardRef } = require('react');\nexport const Island = forwardRef((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "app.jsx": "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n"}
# ---- sol r3 folds (W2 hoisted var; W1 recorded as S1b RED characterizations) ----
def memo_lib(pre):
    return "import { memo } from 'react';\n" + pre + "export const Island = memo((props: any) => {\n  return <div/>;\n});\n"
S["C57_TR6P4b_block_var"] = {"lib.tsx": memo_lib("declare const flag: boolean;\ndeclare const fake: any;\nif (flag) {\n  var memo = fake;\n}\n"), "app.tsx": APP_ISLAND}
S["C58_for_of_var"] = {"lib.tsx": memo_lib("for (var memo of [1]) {}\n"), "app.tsx": APP_ISLAND}
S["C59_try_for_init_var"] = {"lib.tsx": memo_lib("try {\n  for (var memo = 0; memo < 1; memo++) {}\n} catch (e) {}\n"), "app.tsx": APP_ISLAND}
S["C60_block_let_positive"] = {"lib.tsx": memo_lib("declare const flag: boolean;\nif (flag) {\n  let memo = 1;\n}\n"), "app.tsx": APP_ISLAND}
S["C61_nested_fn_var_positive"] = {"lib.tsx": memo_lib("function helper() {\n  var memo = 1;\n  return memo;\n}\nclass K { m() { var memo = 2; return memo; } }\n"), "app.tsx": APP_ISLAND}
S["C62_S1b_namespace_decoy"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nfunction helper() {\n  function Island() { return 1; }\n  return Island();\n}\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "app.tsx": "import * as Lib from './lib';\nexport function App() {\n  return <Lib.Island/>;\n}\n"}
S["C63_S1b_producer_local_call"] = {
 "lib.tsx": "import { forwardRef } from 'react';\nexport const Island = forwardRef<HTMLDivElement, any>((props, ref) => {\n  return <div ref={ref}/>;\n});\nexport function Host() {\n  return (Island as any)({}) && Island({} as any, null as any);\n}\n"}
S["C64_TR6P4_top_function"] = {"lib.tsx": memo_lib("function memo(x: any): any { return x; }\n"), "app.tsx": APP_ISLAND}
S["C65_TR6P4_top_class"] = {"lib.tsx": memo_lib("class memo {}\n"), "app.tsx": APP_ISLAND}
S["C66_TR6P4_top_destructure"] = {"lib.tsx": memo_lib("declare const x: any;\nconst { memo } = x;\n"), "app.tsx": APP_ISLAND}
S["C67_TR6P4_component_local_positive"] = {"lib.tsx": memo_lib("export function Other() {\n  const memo = 1;\n  return memo;\n}\n"), "app.tsx": APP_ISLAND}

# ---- S1b scenarios (C68+). Each JS-syntax scenario is emitted twice: `_jsx` (.jsx/.js) and `_tsx` (.tsx/.ts),
# because the two grammars recover differently. `{x}` is the JSX-capable extension, `{s}` the plain one.
S1B = {}
S1B["C68_D4_default_ternary_decoy"] = {
 "util.{s}": "function outer() {\n  function f() { return 99; }\n  return f();\n}\nconst a = 1;\nconst f = a > 0 ? a : 2;\nexport default f;\n",
 "app.{s}": "import f from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C69_D4_export_function_nested_decoy"] = {
 "util.{s}": "function outer() {\n  function f() { return 99; }\n  return f();\n}\nexport function f() {\n  return 1;\n}\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C70_D4_export_const_arrow_nested_decoy"] = {
 "util.{s}": "function outer() {\n  const f = () => 99;\n  return f();\n}\nexport const f = () => {\n  return 1;\n};\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C71_D4_list_arrow_nested_decoy"] = {
 "util.{s}": "function outer() {\n  function f() { return 99; }\n  return f();\n}\nconst f = () => {\n  return 1;\n};\nexport { f };\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C72_D4_list_wrapped_nonjsx_and_jsx"] = {
 "lib.{x}": "import { forwardRef } from 'react';\nconst Island = forwardRef((props, ref) => {\n  return <div ref={ref}/>;\n});\nexport { Island };\n",
 "app.{x}": "import { Island } from './lib';\nexport function App() {\n  Island({});\n  return <Island/>;\n}\n"}
S1B["C73_D4_list_factory_callable"] = {
 "store.{s}": "import { create } from 'zustand';\nconst useStore = create((set) => ({\n  count: 0,\n}));\nexport { useStore };\n",
 "app.{s}": "import { useStore } from './store';\nexport function run() {\n  return useStore();\n}\n"}
S1B["C74_D4_duplicate_var"] = {
 "util.{s}": "var f = () => 1;\nvar f = () => 2;\nexport { f };\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C75_D4_hoisted_var_competitor"] = {
 "util.{s}": "export function f() {\n  return 1;\n}\nif (globalThis.x) {\n  var f = 2;\n}\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C77_D4_default_function_written"] = {
 "util.{s}": "function f() {\n  return 1;\n}\nfunction other() {\n  return 2;\n}\nf = other;\nexport default f;\n",
 "app.{s}": "import f from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C78_D4_list_named_fe_positive"] = {
 "util.{s}": "const f = function g() {\n  return 1;\n};\nexport { f };\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C79_D4_unrelated_parse_error"] = {
 "util.{x}": "export function f() {\n  return 1;\n}\nexport function broken() {\n  return <div>{</div>;\n}\n",
 "app.{s}": "import { f } from './util';\nexport function run() {\n  f();\n}\n"}
S1B["C80_R3_namespace_function_decoy"] = {
 "lib.{s}": "function helper() {\n  function f() { return 99; }\n  return f();\n}\nexport function f() {\n  return 1;\n}\n",
 "app.{s}": "import * as Lib from './lib';\nexport function run() {\n  Lib.f();\n}\n"}
S1B["C81_R3_namespace_js_extension_decoy"] = {
 "lib.{s}": "function helper() {\n  function f() { return 99; }\n  return f();\n}\nexport function f() {\n  return 1;\n}\n",
 "app.{s}": "import * as Lib from './lib.js';\nexport function run() {\n  Lib.f();\n}\n"}
S1B["C82_R3_namespace_wrapped_nonjsx"] = {
 "lib.{x}": "import { forwardRef } from 'react';\nexport const Island = forwardRef((props, ref) => {\n  return <div ref={ref}/>;\n});\n",
 "app.{x}": "import * as Lib from './lib';\nexport function App() {\n  Lib.Island({});\n  return <Lib.Island/>;\n}\n"}
S1B["C83_R3_namespace_reexport_rename"] = {
 "impl.{s}": "export function g() {\n  return 1;\n}\n",
 "lib.{s}": "export { g as f } from './impl';\n",
 "app.{s}": "import * as Lib from './lib';\nexport function run() {\n  Lib.f();\n}\n"}
S1B["C84_R3_named_object_qualifier_unchanged"] = {
 "lib.{s}": "export const obj = {\n  f: () => 1,\n};\n",
 "app.{s}": "import { obj } from './lib';\nexport function run() {\n  obj.f();\n}\n"}
S1B["C85_R3_default_qualifier_decoy_unchanged"] = {
 "lib.{x}": "function helper() {\n  function Island() { return 1; }\n  return Island();\n}\nconst Island = () => <div/>;\nexport default { Island };\n",
 "app.{x}": "import Lib from './lib';\nexport function App() {\n  return <Lib.Island/>;\n}\n"}
S1B["C86_R4_two_nested_same_name"] = {
 "a.{s}": "export function one() {\n  const g = () => 1;\n  return g();\n}\nexport function two() {\n  const g = () => 2;\n  return g();\n}\n"}
S1B["C87_R4_param_shadows"] = {
 "a.{s}": "export function run(f) {\n  return f();\n}\nexport function holder() {\n  const f = () => 1;\n  return f;\n}\n"}
S1B["C88_R4_pattern2_global"] = {
 "a.{s}": "export const sys = {\n  require: (x) => x,\n};\nexport function load() {\n  return require('fs');\n}\n"}
S1B["C89_R4_useCallback"] = {
 "a.{x}": "import { useCallback } from 'react';\nexport function App() {\n  const save = useCallback(() => {\n    return 1;\n  }, []);\n  save();\n  return <div/>;\n}\n"}
S1B["C90_R4_throttle_wrapper"] = {
 "a.{s}": "import throttle from 'lodash/throttle';\nexport function App() {\n  const t = throttle(() => {\n    return 1;\n  }, 10);\n  t();\n}\n"}
S1B["C91_R4_let_assigned_later"] = {
 "a.{s}": "export function run(flag) {\n  let g;\n  if (flag) {\n    g = () => 1;\n  } else {\n    g = () => 2;\n  }\n  return g();\n}\n"}
S1B["C92_R4_shadow_forms"] = {
 "a.{s}": "function h() {\n  return 0;\n}\nexport function viaCatch() {\n  try { h(); } catch (h) { h(); }\n}\nexport function viaFor(xs) {\n  for (const h of xs) { h(); }\n}\nexport function viaClass() {\n  class h {}\n  return h();\n}\nexport function viaWith(o) {\n  with (o) { h(); }\n}\n"}
S1B["C93_R4_block_function"] = {
 "a.{s}": "export function run(flag) {\n  if (flag) {\n    function inner() { return 1; }\n    inner();\n  }\n  return inner();\n}\n"}
S1B["C94_R4_hoisted_var_callable"] = {
 "a.{s}": "export function run(flag) {\n  if (flag) {\n    var h = () => 1;\n  }\n  return h();\n}\n"}
S1B["C95_R4_same_line_collision"] = {
 "a.{s}": "export function a() { const f = () => 1; return f(); } export function b() { const f = () => 2; return f(); }\n"}
S1B["C96_intrinsic_tags"] = {
 "a.{x}": "function div() {\n  return 1;\n}\nfunction island() {\n  return 2;\n}\nexport function App() {\n  return <div><island>x</island><my-el/></div>;\n}\n"}
S1B["C97_Z02b_lowercase_plain_export"] = {
 "lib.{x}": "export const island = () => <span/>;\n",
 "app.{x}": "import { island } from './lib';\nexport function App() {\n  island();\n  return <island/>;\n}\n"}
S1B["C98_namespace_lowercase_member_tag"] = {
 "lib.{x}": "export const island = () => <span/>;\n",
 "app.{x}": "import * as lib from './lib';\nexport function App() {\n  return <lib.island/>;\n}\n"}
S1B["C99_R4_recursion_and_fe_self"] = {
 "a.{s}": "export function f(n) {\n  return n ? f(n - 1) : 0;\n}\nexport const g = function h(n) {\n  return n ? h(n - 1) : 0;\n};\n"}
S1B["C101_R4_arrow_single_param_shadow"] = {
 "a.{s}": "function x() {\n  return 1;\n}\nexport const run = x => x();\n"}
S1B["C105_R4_parse_recovery_in_scope"] = {
 "a.{x}": "function g() {\n  return 1;\n}\nexport function App() {\n  const g = () => 2;\n  g();\n  let x = ;\n  return 1;\n}\n"}
S1B["C108_R4_jsx_recovery_in_scope"] = {
 "a.{x}": "function g() {\n  return 1;\n}\nexport function App() {\n  const g = () => 2;\n  g();\n  return <div><span></div>;\n}\n"}
S1B["C109_R4_recovery_hides_closer_declaration"] = {
 "a.{x}": "function g() {\n  return 1;\n}\nexport function App(flag) {\n  if (flag) {\n    let g = ;\n    g();\n  }\n  return 1;\n}\n"}
S1B["C106_R4_wrapped_samefile_jsx_and_call"] = {
 "a.{x}": "import { memo } from 'react';\nconst Card = memo((props) => {\n  return <div/>;\n});\nexport function App() {\n  Card({});\n  return <Card/>;\n}\n"}
S1B["C107_R4_lazy_loader"] = {
 "a.{x}": "import { lazy } from 'react';\nconst Editor = lazy(() => import('./Editor'));\nexport function App() {\n  return <Editor/>;\n}\n"}
S1B["C110_R4_param_default_scope"] = {
 "a.{s}": "function g() {\n  return 1;\n}\nexport function f(a = g()) {\n  function g() {\n    return 2;\n  }\n  return a;\n}\n"}
S1B["C111_R4_annex_b_block_function"] = {
 "a.{s}": "function inner() {\n  return 0;\n}\nexport function run(flag) {\n  if (flag) {\n    function inner() {\n      return 1;\n    }\n  }\n  return inner();\n}\n"}
S1B["C112_R3_namespace_competed"] = {
 "lib.{s}": "export function f() {\n  return 1;\n}\n",
 "app.{s}": "import * as Lib from './lib';\nvar Lib = { f() { return 2; } };\nexport function run() {\n  Lib.f();\n}\n"}
S1B["C113_R4_implicit_arguments"] = {
 "a.{s}": "function arguments() {\n  return 1;\n}\nexport function f() {\n  return arguments();\n}\nexport const g = () => arguments();\n"}
TS_ONLY = {}
TS_ONLY["C102_R4_ts_overloads"] = {
 "a.ts": "export function f(a: string): void;\nexport function f(a: any) {\n  return a;\n}\nexport function run() {\n  f('x');\n}\n"}
TS_ONLY["C103_R4_ts_declare_function"] = {
 "a.ts": "declare function f(): void;\nexport function holder() {\n  function f() { return 1; }\n  return f;\n}\nexport function run() {\n  f();\n}\n"}
TS_ONLY["C104_R4_ts_namespace_enum_shadow"] = {
 "a.ts": "function E() {\n  return 1;\n}\nfunction N() {\n  return 2;\n}\nexport function run() {\n  enum E { A }\n  return E();\n}\nnamespace N {}\nexport function run2() {\n  return N();\n}\n"}
TS_ONLY["C114_R4_ts_using_declaration"] = {
 "a.ts": "declare function res(): any;\nfunction h() {\n  return 1;\n}\nexport function f() {\n  using h = res();\n  h();\n}\nexport function g() {\n  return h();\n}\n"}
for name, files in S1B.items():
    for tag, x, s_ in (('jsx', 'jsx', 'js'), ('tsx', 'tsx', 'ts')):
        S[name + '_' + tag] = {f.replace('{x}', x).replace('{s}', s_): src for f, src in files.items()}
S.update(TS_ONLY)
for name, files in S.items():
    os.makedirs(name, exist_ok=True)
    for f, src in files.items():
        open(os.path.join(name, f), "w").write(src)
json.dump(sorted(S), open("SCENARIOS.json", "w"), indent=1)
print(len(S))
