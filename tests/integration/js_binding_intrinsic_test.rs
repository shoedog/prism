//! S1b-1 (S1b SPEC §3.5, §3.1 F1–F3, §7 A-1…A-3): the JSX intrinsic guard and the
//! shared-collector fixes. Rows run in both grammars with exact `(file, name, span)` targets.
use super::js_wrapped_export_test::{app_sites, check, graph, APP};

const BOTH: &[&str] = &["jsx", "tsx"];
const NO_LIB: &str = "export const unrelated = 1;\n";

#[test]
fn a1_intrinsic_tags_drop_on_every_rung() {
    // C97 (R4c): the JSX tag drops; the plain call of the same import keeps its edge.
    let lib = "export const island = () => <span/>;\n";
    let app = "import { island } from './lib';\nexport function App() {\n  island();\n  \
        return <island/>;\n}\n";
    let c97: &[&str] = &[
        "L3 island: Exact import_member lib:island@1-1",
        "L4 island: drop JsxIntrinsic",
    ];
    // C96 (R4 `local_def`, opening tags) plus the dashed and namespaced spellings.
    let c96 = "function div() {\n  return 1;\n}\nfunction island() {\n  return 2;\n}\n\
        export function App() {\n  return <div><island>x</island><my-el/><svg:rect/></div>;\n}\n";
    let c96_want: &[&str] = &[
        "L8 div: drop JsxIntrinsic",
        "L8 island: drop JsxIntrinsic",
        "L8 my-el: drop JsxIntrinsic",
        "L8 svg:rect: drop JsxIntrinsic",
    ];
    // R5 (the Tier-A fixture's shape): a cross-file free function named like the tag.
    let input = "export function input() {\n  return 1;\n}\n";
    let r5 = "export function App() {\n  return <input/>;\n}\n";
    check(
        &[
            (lib, app, c97),
            (NO_LIB, c96, c96_want),
            (input, r5, &["L2 input: drop JsxIntrinsic"]),
        ],
        BOTH,
    );
}

#[test]
fn a3_f2_f3_parameter_bindings_shadow_cross_file_functions() {
    // F3 (C101 cross-file): a single unparenthesized arrow parameter binds `x`.
    let x = "export function x() {\n  return 1;\n}\n";
    let f3 = "export const run = x => x();\n";
    // F2 (C124, parameter form): `{ f = … }` in a parameter pattern binds `f`.
    let f = "export function f() {\n  return 1;\n}\n";
    let f2 = "export function caller({ f = () => 2 } = {}) {\n  return f();\n}\n";
    check(
        &[
            (x, f3, &["L1 x: drop UnknownName"]),
            (f, f2, &["L2 f: drop UnknownName"]),
        ],
        BOTH,
    );
}

/// A wrapped export whose React wrapper local is preceded by `write`.
fn wrapped_after(write: &str) -> String {
    format!(
        "import {{ forwardRef }} from 'react';\n{write}\nexport const Island = \
         forwardRef((props, ref) => null);\n"
    )
}

#[test]
fn a3_f1_f2_f3_writes_through_the_base_module_scan() {
    // S1's provenance check uses the base module write scan (SPEC §3.2): a write to the
    // wrapper local refuses it (`callee_provenance`), so `<Island/>` has no export fact.
    let refused = ["L3 Island: drop UnknownName"];
    let admitted = ["L3 Island: Exact import_member lib:Island@3-3"];
    let rows: [(&str, &str, &[&str]); 6] = [
        ("tsx", "(forwardRef as any) = null;", &refused), // F1 (C142)
        ("tsx", "(forwardRef satisfies any) = null;", &refused), // F1
        ("tsx", "forwardRef! = null;", &refused),         // F1
        ("ts", "(<any>forwardRef) = null;", &refused),    // F1 (TS only)
        ("tsx", "({ forwardRef = null } = {});", &refused), // F2 (C125 write form)
        // F3 (C126 shape): the arrow's own parameter is written, not the module import.
        (
            "tsx",
            "const g = forwardRef => { forwardRef = 2; };",
            &admitted,
        ),
    ];
    for (ext, write, want) in rows {
        let (name, lib) = (format!("lib.{ext}"), wrapped_after(write));
        let cg = graph(&[(name.as_str(), lib.as_str()), ("app.tsx", APP)]);
        assert_eq!(app_sites(&cg), want, "{write}");
    }
}
