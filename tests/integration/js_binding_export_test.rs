//! S1b-2b (S1b SPEC §3.2, §3.3, §7 B-1–B-3, B-7, B-9, B-16): the ESM local export routes
//! bind through the module-scope binding. Every row runs in the JSX and TSX grammars and
//! names the exact `(file, name, span)` target, with a same-named decoy where one matters.
use super::js_wrapped_export_test::{app_sites, graph};

/// `@D` is a same-named function nested in another function: the decoy base's `Local(f)`
/// binds by name (lines 1–4, so the exported callable starts at line 5).
const DECOY: &str = "function outer() {\n  function f() { return 99; }\n  return f();\n}\n";
const F: &str = "import { f } from './lib';\nexport function run() {\n  f();\n}\n";
const DF: &str = "import f from './lib';\nexport function run() {\n  f();\n}\n";
const H: &str = "export function run() {\n  h();\n}\n";
const ISLAND: &str = "import { forwardRef } from 'react';\nconst Island = forwardRef((props, \
    ref) => {\n  return <div ref={ref}/>;\n});\n";
const APP_I: &str = "import { Island } from './lib';\nexport function App() {\n  Island({});\n  \
    return <Island/>;\n}\n";
const FN: &str = "function f() {\n  return 1;\n}\n";

const X57: &[&str] = &["L3 f: Exact import_member lib:f@5-7"];
const DROP: &[&str] = &["L3 f: drop UnknownName"];
const I24: &[&str] = &[
    "L3 Island: drop WrappedExportNonJsx",
    "L4 Island: Exact import_member lib:Island@2-4",
];
const N55: &[&str] = &["L3 f: NameOnly import_member lib:f@2-2, NameOnly import_member lib:f@5-5"];
const N57: &[&str] = &["L3 f: NameOnly import_member lib:f@2-2, NameOnly import_member lib:f@5-7"];
const N68: &[&str] = &["L3 f: NameOnly import_member lib:f@2-2, NameOnly import_member lib:f@6-8"];
const H13: &[&str] = &["L3 h: Exact import_member lib:f@1-3"];
const G46: &[&str] = &["L3 h: Exact import_member lib:g@4-6"];
const DROP_H: &[&str] = &["L3 h: drop UnknownName"];
const C126: &str = "export function f() {\n  return 1;\n}\nexport const g = f => {\n  f = 2;\n};\n";

/// `(control id, lib, app, expected app-site outcomes)`; in `lib`, `@D` is [`DECOY`], `@F`
/// is [`FN`], `@I` is [`ISLAND`] and `@C` is [`C126`]; an app starting `{` imports for [`H`].
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

const ROWS: &[Row] = &[
    // B-1 (C06 list, C68 default): a ternary binding is `not_callable`; base bound the decoy.
    (
        "C06",
        "@Dconst a = 1;\nconst f = a > 0 ? a : 2;\nexport { f };\n",
        F,
        DROP,
    ),
    (
        "C68",
        "@Dconst a = 1;\nconst f = a > 0 ? a : 2;\nexport default f;\n",
        DF,
        DROP,
    ),
    // B-2 (C69–C71, default function): base NameOnly ×2.
    ("C69", "@Dexport function f() {\n  return 1;\n}\n", F, X57),
    (
        "C70",
        "@Dexport const f = () => {\n  return 1;\n};\n",
        F,
        X57,
    ),
    (
        "C71",
        "@Dconst f = () => {\n  return 1;\n};\nexport { f };\n",
        F,
        X57,
    ),
    (
        "C69 default",
        "@Dexport default function f() {\n  return 1;\n}\n",
        DF,
        X57,
    ),
    // B-3 (C72, and its default form): a wrapped binding binds from JSX sites only.
    ("C72", "@Iexport { Island };\n", APP_I, I24),
    ("C72 default", "@Iexport default Island;\n", "default", I24),
    // B-7 GUARDS (E5, E5b a): written or call-wrapped bindings keep base `Local(f)` (NameOnly
    // ×2 with the decoy; a proof here would be a wrong Exact). C21 written arrow, C77
    // written function (default export), C73 `create(fn)`.
    (
        "C21",
        "@Dlet f = () => 1;\nfunction o() {}\nf = o;\nexport { f };\n",
        F,
        N55,
    ),
    (
        "C77",
        "@Dfunction f() {\n  return 1;\n}\nf = o;\nfunction o() {}\nexport default f;\n",
        DF,
        N57,
    ),
    (
        "C73",
        "@Dimport { create } from 'zustand';\nconst f = create((set) => ({\n  count: 0,\n}));\n\
        export { f };\n",
        F,
        N68,
    ),
    // B-9 (C126, F3 through D4): `f = 2` writes the arrow's parameter, not the module `f`.
    // The plain control is a GUARD (one `f`: Exact either way); the decoy makes it RED.
    ("C126", "@C", F, &["L3 f: Exact import_member lib:f@1-3"]),
    ("C126 decoy", "@D@C", F, X57),
    // B-16 `ModuleExportName` (sol r2 W3): StringValue on both sides, both quote forms and
    // escapes; each row spells the two sides differently, so raw text never matches.
    (
        "\"g\" / 'g'",
        "@Fexport { f as \"g\" };\n",
        "{ 'g' as h }",
        H13,
    ),
    ("'g' / g", "@Fexport { f as 'g' };\n", "{ g as h }", H13),
    (
        "\\u0067 / 'g'",
        "@Fexport { f as \"\\u0067\" };\n",
        "{ 'g' as h }",
        H13,
    ),
    (
        "h / \\u{68}",
        "@Fexport { f as h };\n",
        "{ '\\u{68}' as h }",
        H13,
    ),
    (
        "\\x67 / \"g\"",
        "@Fexport { f as '\\x67' };\n",
        "{ \"g\" as h }",
        H13,
    ),
    // B-16 surrogates (impl r1 W1): a pair decodes to one scalar, escaped or literal, in either
    // spelling; different pairs never match; an unpaired surrogate is unmatchable on either
    // side (fail closed, and no fallback to the same-named `h`); `"a😀"` next to `"a"` does not
    // collide; `\é` is a NonEscapeCharacter.
    (
        "pair / literal",
        "@Fexport { f as \"\\uD83D\\uDE00\" };\n",
        "{ '😀' as h }",
        H13,
    ),
    (
        "literal / pair",
        "@Fexport { f as '😀' };\n",
        "{ \"\\u{D83D}\\u{DE00}\" as h }",
        H13,
    ),
    (
        "😀 / 😁",
        "@Fexport { f as \"\\uD83D\\uDE00\" };\n",
        "{ '\\uD83D\\uDE01' as h }",
        DROP_H,
    ),
    (
        "a😀 + a / a",
        "@F@Gexport { f as \"a\\uD83D\\uDE00\" };\nexport { g as \"a\" };\n",
        "{ \"a\" as h }",
        G46,
    ),
    (
        "unpaired / unpaired",
        "@Fexport { f as \"\\uD83D\" };\n",
        "{ \"\\uD83D\" as h }",
        DROP_H,
    ),
    (
        "a / a+lone low (h competitor)",
        "@Fexport function h() {}\nexport { f as \"a\" };\n",
        "{ 'a\\uDE00' as h }",
        DROP_H,
    ),
    (
        "\\é / é",
        "@Fexport { f as \"\\é\" };\n",
        "{ 'é' as h }",
        H13,
    ),
    // B-16 impl r2 (sol W1, W2): a legacy octal escape is unmatchable, never NUL; a specifier
    // the parser recovered (`"\xGG"` as identifier `GG`) is unmatchable, with no fallback.
    (
        "\\0 / \\01",
        "@F@Hexport { f as \"\\0\" };\n",
        "{ \"\\01\" as h }",
        DROP_H,
    ),
    (
        "\\0 / \\0",
        "@Fexport { f as \"\\0\" };\n",
        "{ '\\0' as h }",
        H13,
    ),
    (
        "GG / \\xGG",
        "@F@Hexport { f as \"GG\" };\n",
        "{ \"\\xGG\" as h }",
        DROP_H,
    ),
    (
        "ZZZZ / \\uZZZZ",
        "@F@Hexport { f as \"ZZZZ\" };\n",
        "{ \"\\uZZZZ\" as h }",
        DROP_H,
    ),
];

fn expand(lib: &str, app: &str) -> (String, String) {
    let lib = lib
        .replace("@D", DECOY)
        .replace("@F", FN)
        .replace("@G", "function g() {\n  return 2;\n}\n")
        .replace("@H", "export function h() {}\n")
        .replace("@I", ISLAND);
    let app = match app {
        "default" => APP_I.replace("{ Island }", "Island"),
        a if a.starts_with('{') => format!("import {a} from './lib';\n{H}"),
        a => a.to_string(),
    };
    (lib.replace("@C", C126), app)
}

#[test]
fn b_rows_export_routes_bind_through_the_module_binding() {
    let mut wrong = Vec::new();
    for (id, lib, app, want) in ROWS {
        let (lib, app) = expand(lib, app);
        for ext in ["jsx", "tsx"] {
            let (l, a) = (format!("lib.{ext}"), format!("app.{ext}"));
            let got = app_sites(&graph(&[(&l, &lib), (&a, &app)]));
            if got != *want {
                wrong.push(format!("{id} .{ext}: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong outcomes:\n{}", wrong.join("\n"));
}

#[test]
fn b2_verified_export_binds_through_a_star_barrel() {
    let (lib, _) = expand(ROWS[4].1, F);
    for ext in ["jsx", "tsx"] {
        let (l, i, a) = (
            format!("lib.{ext}"),
            format!("index.{ext}"),
            format!("app.{ext}"),
        );
        let app = F.replace("./lib", "./index");
        let files = [
            (&l[..], &lib[..]),
            (&i, "export * from './lib';\n"),
            (&a, &app),
        ];
        assert_eq!(app_sites(&graph(&files)), X57, ".{ext}");
    }
}
