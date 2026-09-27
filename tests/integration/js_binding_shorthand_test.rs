//! S1b-1b (F4, S1b SPEC §0 OQ8): a `variable_declarator` binds its BoundNames. Shorthand
//! destructuring shadows an import or a cross-file function; a destructuring default's
//! expression binds nothing. Rows run in both grammars with exact `(file, name, span)` targets.
use super::js_wrapped_export_test::{app_sites, check, graph};

const BOTH: &[&str] = &["jsx", "tsx"];
const LIB_F: &str = "export function f() {\n  return 1;\n}\n";
const DROP_L3: &[&str] = &["L3 f: drop UnknownName"];

#[test]
fn b1_shorthand_destructuring_shadows_import_and_free_routes() {
    // `import_member`: the destructured local `f` is not the imported `f`.
    let import = "import { f } from './lib';\nexport function run(o) {\n  const { f } = o;\n  \
        return f();\n}\n";
    // `free_single`: nor is it the cross-file free function `f`.
    let free = "export function run(o) {\n  const { f } = o;\n  return f();\n}\n";
    // A shorthand default (`object_assignment_pattern`) binds `f` too.
    let default_null = "export function run(o) {\n  const { f = null } = o;\n  return f();\n}\n";
    // Module scope (`module_value_bindings`): the file's `f` is the destructured value.
    let module = "const { f } = globalThis.o;\nexport function run() {\n  return f();\n}\n";
    check(
        &[
            (LIB_F, import, &["L4 f: drop UnknownName"]),
            (LIB_F, free, DROP_L3),
            (LIB_F, default_null, DROP_L3),
            (LIB_F, module, DROP_L3),
        ],
        BOTH,
    );
}

#[test]
fn b1_shorthand_destructuring_shadows_free_multi() {
    // `free_multi` (T's `var { enter, exit } = …` shape): two same-name free functions.
    let app = "export function run(o) {\n  var { f } = o;\n  return f();\n}\n";
    for ext in BOTH {
        let names = [
            format!("lib.{ext}"),
            format!("lib2.{ext}"),
            format!("app.{ext}"),
        ];
        let cg = graph(&[(&names[0], LIB_F), (&names[1], LIB_F), (&names[2], app)]);
        assert_eq!(app_sites(&cg), DROP_L3, ".{ext}");
    }
}

#[test]
fn b2_destructuring_default_values_do_not_shadow() {
    // `const [a = b] = …` binds `a` only: `b()` keeps its cross-file edge.
    let lib_b = "export function b() {\n  return 1;\n}\n";
    let array = "export function run(xs) {\n  const [a = b] = xs;\n  a();\n  return b();\n}\n";
    // X's shape: a default that calls an import (`{ x = t('k') }`) keeps the import edge.
    let lib_t = "export function t(k) {\n  return k;\n}\n";
    let object = "import { t } from './lib';\nexport function run(p) {\n  const { x = t('k') } = \
        p;\n  return x;\n}\n";
    check(
        &[
            (
                lib_b,
                array,
                &[
                    "L3 a: drop UnknownName",
                    "L4 b: Exact free_single lib:b@1-3",
                ],
            ),
            (lib_t, object, &["L3 t: Exact import_member lib:t@1-3"]),
        ],
        BOTH,
    );
}
