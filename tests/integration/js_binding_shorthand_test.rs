//! S1b-1b (F4, S1b SPEC §0 OQ8): a `variable_declarator` binds its BoundNames. Shorthand
//! destructuring shadows an import or a cross-file function; a destructuring default's
//! expression binds nothing. Rows run in both grammars with exact `(file, name, span)` targets.
use super::js_wrapped_export_test::{app_sites, check, graph, run};

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
    // Top scope through `export`, and a rest element (`{ ...f }`) binds `f` (sol W2, M7).
    let export = format!("export {module}");
    let rest = free.replace("{ f }", "{ ...f }");
    check(
        &[
            (LIB_F, import, &["L4 f: drop UnknownName"]),
            (LIB_F, &export, DROP_L3),
            (LIB_F, &rest, DROP_L3),
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
    // `const [a = b] = …` binds `a` (sol W2, M8: `lib.a` is the decoy) but not `b`.
    let lib_b = "export function a() {}\nexport function b() {\n  return 1;\n}\n";
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
                    "L4 b: Exact free_single lib:b@2-4",
                ],
            ),
            (lib_t, object, &["L3 t: Exact import_member lib:t@1-3"]),
        ],
        BOTH,
    );
}

#[test]
fn b3_renamed_and_member_initializer_twins_bind_only_the_local() {
    // Guards (base-green): `{ g: f }` binds `f`, not the key `g`; `const f = o.f` binds `f`.
    // The unshadowed `g()` keeps its edge on both routes.
    let lib = "export function f() {\n  return 1;\n}\nexport function g() {\n  return 2;\n}\n";
    let body =
        |decl: &str| format!("export function run(o) {{\n  {decl}\n  f();\n  return g();\n}}\n");
    let import = |decl: &str| format!("import {{ f, g }} from './lib';\n{}", body(decl));
    let (renamed, member) = ("const { g: f } = o;", "const f = o.f;");
    let free_want: &[&str] = &[
        "L3 f: drop UnknownName",
        "L4 g: Exact free_single lib:g@4-6",
    ];
    let import_want: &[&str] = &[
        "L4 f: drop UnknownName",
        "L5 g: Exact import_member lib:g@4-6",
    ];
    let apps = [body(renamed), body(member), import(renamed), import(member)];
    check(
        &[
            (lib, &apps[0], free_want),
            (lib, &apps[1], free_want),
            (lib, &apps[2], import_want),
            (lib, &apps[3], import_want),
        ],
        BOTH,
    );
}

#[test]
fn b4_dollar_named_locals_shadow() {
    // Opus W1 (D12–D16): `$` is a JS identifier character, so a `$`-named local shadows the
    // same-name cross-file function or import.
    let lib = "export function $() {}\nexport function $f() {}\n";
    let app = |decl: &str, call: &str| {
        format!("export function run(o) {{\n  {decl}\n  return {call}();\n}}\n")
    };
    let d12 = app("const $ = o.jq;", "$");
    let d13 = app("const { $ } = o;", "$");
    let d14 = app("const $f = o.f;", "$f");
    let d15 = format!("import {{ $f }} from './lib';\n{d14}");
    let d16 = "const $f = globalThis.o;\nexport function run() {\n  return $f();\n}\n";
    // P1: the shared collector also binds a `$` parameter (base resolved it cross-file).
    let p1 = "export function run($) {\n  return $();\n}\n";
    check(
        &[
            (lib, &d12, &["L3 $: drop UnknownName"]),
            (lib, &d13, &["L3 $: drop UnknownName"]),
            (lib, &d14, &["L3 $f: drop UnknownName"]),
            (lib, &d15, &["L4 $f: drop UnknownName"]),
            (lib, d16, &["L3 $f: drop UnknownName"]),
            (lib, p1, &["L2 $: drop UnknownName"]),
        ],
        BOTH,
    );
}

#[test]
fn b5_nested_declarators_keep_base() {
    // Opus W2 / sol W1 (Option K): a declarator nested in a block keeps base, so its
    // shorthand never shadows `f()` after the block. The in-block call keeps base's Exact
    // until S1b-3's site walk; a nested `var` keeps base too.
    let block = |head: &str, decl: &str| {
        format!("export function run(o) {{\n  {head}{{\n    {decl}\n    f();\n  }}\n  return f();\n}}\n")
    };
    let (bare, in_if, var) = (
        block("", "const { f } = o;"),
        block("if (o) ", "let { f } = o;"),
        block("", "var { f } = o;"),
    );
    let import = format!("import {{ f }} from './lib';\n{in_if}");
    let same_line = "export function run(o) {\n  { const { f } = o; f(); } return f();\n}\n";
    let exact = |l: &str, k: &str| vec![format!("L{l} f: Exact {k} lib:f@1-3")];
    let free = [exact("4", "free_single"), exact("6", "free_single")].concat();
    let import_want = [exact("5", "import_member"), exact("7", "import_member")].concat();
    let line2 = [exact("2", "free_single"), exact("2", "free_single")].concat();
    for (app, want) in [
        (&bare, &free),
        (&in_if, &free),
        (&var, &free),
        (&import, &import_want),
    ] {
        for ext in BOTH {
            assert_eq!(run(LIB_F, app, ext), *want, "{app} .{ext}");
        }
    }
    for ext in BOTH {
        assert_eq!(run(LIB_F, same_line, ext), line2, ".{ext}");
    }
}
