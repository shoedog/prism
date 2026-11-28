//! Direct checks of the unique value-export claim counter (lane 3 S2b, codecov patch). Each
//! refusal row is paired with a control that differs in the one fact the guard reads.

use crate::ast::ParsedFile;
use crate::languages::Language;

fn unique(language: Language, source: &str, member: &str, check_cjs: bool) -> bool {
    let name = if language == Language::TypeScript {
        "m.ts"
    } else {
        "m.js"
    };
    ParsedFile::parse(name, source, language)
        .unwrap()
        .js_ts_has_unique_value_export(member, check_cjs)
}

fn js(source: &str, member: &str) -> bool {
    unique(Language::JavaScript, source, member, false)
}

#[test]
fn unique_export_refuses_an_errorful_tree_and_unsafe_cjs_objects() {
    let ok = "function send({a}) {}\nexport { send };\n";
    assert!(js(ok, "send"));
    // A parse error anywhere voids the count, as does an unsafe CJS export object when asked.
    assert!(!js(
        "function send({a}) {}\nexport { send };\nfunction (",
        "send"
    ));
    assert!(unique(Language::JavaScript, ok, "send", true));
    let unsafe_object = "export { send };\nmodule.exports = {};\nexports = {};\n";
    assert!(js(unsafe_object, "send"));
    assert!(!unique(Language::JavaScript, unsafe_object, "send", true));
}

#[test]
fn unique_export_counts_cjs_claims_only_when_asked() {
    let esm = "export function send({a}) {}\n";
    // Statements that are neither exports nor CJS writes do not count.
    assert!(unique(
        Language::JavaScript,
        &format!("const k = 1;\nlet y;\ny = 2;\n{esm}"),
        "send",
        true
    ));
    // A CJS export claim beside the ESM claim leaves the namespace shape undecided.
    let mixed = format!("{esm}module.exports.send = send;\n");
    assert!(!unique(Language::JavaScript, &mixed, "send", true));
    assert!(unique(Language::JavaScript, &mixed, "send", false));
}

#[test]
fn unique_export_counts_clause_specifiers_and_aliases() {
    let body = "function a() {}\nfunction b() {}\n";
    // Names count by their exported spelling: the alias when present.
    assert!(js(&format!("{body}export {{ a, b as c }};"), "c"));
    assert!(!js(&format!("{body}export {{ a, b as c }};"), "b"));
    assert!(js(&format!("{body}export {{ a, b as c }};"), "a"));
    // Two specifiers exporting one name, or a clause plus a declaration, are two claims.
    assert!(!js(&format!("{body}export {{ a, b as a }};"), "a"));
    assert!(!js(
        &format!("{body}export {{ a }};\nexport function a() {{}}"),
        "a"
    ));
    // String export names and escaped identifier spellings are compared by value.
    assert!(js(&format!("{body}export {{ a as \"x y\" }};"), "x y"));
    assert!(js(&format!("{body}export {{ a as \\u0073end }};"), "send"));
    assert!(!js(&format!("{body}export {{ a as \\u0073end }};"), "sen"));
    // A specifier whose exported name cannot be compared (an unpaired surrogate) voids the count
    // rather than being skipped: the unreadable name may be the claimed one.
    assert!(js(&format!("{body}export {{ a as \"ok\", b }};"), "b"));
    assert!(!js(
        &format!("{body}export {{ a as \"\\uD800\", b }};"),
        "b"
    ));
    // A type-only specifier is no value claim.
    assert!(!unique(
        Language::TypeScript,
        "type T = 1;\nexport { type T };",
        "T",
        false
    ));
    assert!(unique(
        Language::TypeScript,
        "const T = 1;\nexport { T };",
        "T",
        false
    ));
}

#[test]
fn unique_export_default_stars_and_type_only_statements() {
    // `export default` claims only `default`.
    assert!(js("export default function () {}", "default"));
    assert!(!js("export default function () {}", "send"));
    // A bare star may supply any value name but `default`, which it never supplies.
    assert!(!js(
        "export * from './x';\nexport function send() {}",
        "default"
    ));
    assert!(!js("export * from './x';", "default"));
    // Type-only export statements claim nothing.
    assert!(!unique(
        Language::TypeScript,
        "export type { T } from './x';",
        "T",
        false
    ));
    assert!(!unique(
        Language::TypeScript,
        "export interface T {}",
        "T",
        false
    ));
    assert!(unique(
        Language::TypeScript,
        "export const T = 1;",
        "T",
        false
    ));
}

#[test]
fn unique_export_dotted_namespace_exports_its_outermost_name() {
    let source = "export namespace A.B { export const k = 1; }";
    assert!(unique(Language::TypeScript, source, "A", false));
    assert!(!unique(Language::TypeScript, source, "B", false));
}
