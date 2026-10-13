//! I1 round-1 fold, item 2: the namespace route must not turn a module that
//! is found ONLY by the new normalized fallback into a wrong `Exact`. On
//! main such a module was not found, so the call reached the NameOnly E7
//! outcome; when the positive proof now fails, the branch must return that
//! same E7 outcome, not defer to the legacy stem match.
use super::js_relative_extension_test::{graph, rows};

const NAME_ONLY: &str = "other/x.ts:f@1-1 NameOnly/import_qualified";
const NAME_ONLY_X: &str = "x.ts:f@1-1 NameOnly/import_qualified";
const DECOY: &str = "export function f() { return 2; }\n";

fn ns_rows(lib: &str, spec: &str) -> Vec<String> {
    let app = format!("import * as N from '{spec}';\nexport function g() {{ N.f(); }}\n");
    let cg = graph(&[
        ("x.ts", lib),
        ("other/x.ts", DECOY),
        ("app.ts", app.as_str()),
    ]);
    rows(&cg, "g", "f")
}

#[test]
fn newly_found_module_without_the_member_stays_name_only() {
    // main: module not found -> E7 -> [NAME_ONLY]. Branch before the fold:
    // module found, member absent, `None` -> legacy stem match -> Exact on
    // the unrelated decoy.
    let lib = "export function other() { return 1; }\n";
    for spec in ["./a/../x.js", "./a/../x"] {
        assert_eq!(ns_rows(lib, spec), [NAME_ONLY], "{spec}");
    }
}

#[test]
fn newly_found_module_with_type_only_export_stays_name_only() {
    // main: module not found -> E7 keeps every base candidate as NameOnly,
    // including `x.ts:f` itself. Branch before the fold: Exact on both.
    let lib = "function f() { return 1; }\nexport type { f };\n";
    for spec in ["./a/../x.js", "./a/../x"] {
        assert_eq!(ns_rows(lib, spec), [NAME_ONLY, NAME_ONLY_X], "{spec}");
    }
}

#[test]
fn newly_found_module_with_the_member_is_exact_on_it_alone() {
    // Positive control. main: module not found -> E7 NameOnly on both the
    // decoy and x.ts:f; the branch proves the member and binds only x.ts:f.
    let lib = "export function f() { return 1; }\n";
    for spec in ["./a/../x.js", "./a/../x"] {
        assert_eq!(
            ns_rows(lib, spec),
            ["x.ts:f@1-1 Exact/import_qualified"],
            "{spec}"
        );
    }
}
