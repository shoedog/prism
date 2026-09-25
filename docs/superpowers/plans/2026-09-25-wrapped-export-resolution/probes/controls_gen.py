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
for name, files in S.items():
    os.makedirs(name, exist_ok=True)
    for f, src in files.items():
        open(os.path.join(name, f), "w").write(src)
json.dump(sorted(S), open("SCENARIOS.json", "w"), indent=1)
print(len(S))
