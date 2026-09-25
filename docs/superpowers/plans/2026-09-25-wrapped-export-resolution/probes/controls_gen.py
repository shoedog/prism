# Negative/positive control fixtures for wrapped-export planning probes (P3).
# Each scenario is its own tiny repo so resolution cannot cross-contaminate.
import os, json
S = {}
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
for name, files in S.items():
    os.makedirs(name, exist_ok=True)
    for f, src in files.items():
        open(os.path.join(name, f), "w").write(src)
json.dump(sorted(S), open("SCENARIOS.json", "w"), indent=1)
print(len(S))
