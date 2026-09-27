"""Re-plan synthetic fixtures RP1/RP2 (EXPECTATIONS-pre-run.md). Each scenario is its own tiny repo; JS and TS
grammar twins where the syntax exists. Usage: python3 gen_rp.py <out_dir>"""
import os, sys
S = {}
def both(name, files, js=True, ts=True):
    if js:
        S[name + '_js'] = {k.replace('.EXT', '.jsx' if 'x' in k else '.js'): v for k, v in files.items()}
    if ts:
        S[name + '_ts'] = {k.replace('.EXT', '.tsx' if 'x' in k else '.ts'): v for k, v in files.items()}

both('RP1a_computed_method_inner', {'lib.EXT':
    "export class C {\n  [f()]() {\n    function f() { return 1; }\n    return f();\n  }\n}\n"})
both('RP1b_computed_method_outer', {'lib.EXT':
    "function f() { return 2; }\nexport class C {\n  [f()]() {\n    function f() { return 1; }\n    return f();\n  }\n}\n"})
both('RP1c_namespace_merge', {'lib.EXT':
    "function f() { return 1; }\nnamespace N {\n  export function f() { return 2; }\n}\nnamespace N {\n  export function run() { return f(); }\n}\n"}, js=False)
both('RP1d_enum_member', {'lib.EXT':
    "function f() { return 1; }\nexport enum E {\n  f = 1,\n  g = f(),\n}\n"}, js=False)
both('RP1e_with_object', {'lib.EXT':
    "function f() { return {}; }\nexport function run() {\n  with (f()) {\n    return 1;\n  }\n}\n"}, ts=False)
both('RP1f_nested_namespace', {'lib.EXT':
    "namespace A {\n  export function f() { return 1; }\n}\nnamespace A.B {\n  export function g() { return f(); }\n}\n"}, js=False)
both('RP1g_computed_object_method', {'lib.EXT':
    "function f() { return 2; }\nexport const o = {\n  [f()]() {\n    function f() { return 1; }\n    return f();\n  },\n};\n"})
both('RP1h_heritage', {'lib.EXT':
    "function f() { return class {}; }\nexport class f extends f() {}\n"})
both('RP1i_param_default', {'lib.EXT':
    "function g() { return 1; }\nexport function h(g, a = g()) { return a; }\n"})
both('RP2a_header_error', {'lib.EXT':
    "function f() { return 1; }\nfunction run() { let f = 3 ) ` const g = () => { return 2; }; f(); }\n"})
both('RP2b_string_brace', {'lib.EXT':
    "function f() { return 1; }\nfunction broken() {\n  const x = \"{\" \"y\";\n  return x;\n}\nexport function run() {\n  return f();\n}\n"})
both('RP2c_string_export_name', {'lib.EXT': "function f() { return 1; }\nexport { f as \"g\" };\n",
    'app.EXT': "import * as ns from \"./lib\";\nexport function run() {\n  return ns.g();\n}\n"})

out = sys.argv[1]
for name, files in S.items():
    d = os.path.join(out, name)
    os.makedirs(d, exist_ok=True)
    for f, src in files.items():
        open(os.path.join(d, f), 'w').write(src)
print(len(S), 'scenarios')
