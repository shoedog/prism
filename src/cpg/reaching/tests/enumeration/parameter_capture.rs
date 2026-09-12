use super::super::super::binding_table::{capture_rows, rows, DeclarationKind};
use super::super::{function, parsed};
use crate::languages::Language;

#[test]
fn parameter_rows_exist_exactly_where_the_grammar_has_a_parameters_node() {
    let minimal = [
        (Language::Python, "def f(a):\n    pass\n"),
        (Language::JavaScript, "function f(a) {}\n"),
        (Language::TypeScript, "function f(a: T) {}\n"),
        (Language::Tsx, "function f(a: T) {}\n"),
        (Language::Go, "func f(a int) {}\n"),
        (Language::Java, "class C { void f(int a) {} }\n"),
        (Language::C, "void f(int a) {}\n"),
        (Language::Cpp, "void f(int a) {}\n"),
        (Language::Rust, "fn f(a: i32) {}\n"),
        (Language::Lua, "function f(a) end\n"),
        (Language::Terraform, "resource \"x\" \"y\" {}\n"),
        (Language::Bash, "f() { :; }\n"),
    ];
    for (lang, src) in minimal {
        let p = parsed(src, lang);
        let func = function(&p);
        let has_params = p.find_parameters_node(&func).is_some();
        let has_row = rows(lang)
            .iter()
            .any(|r| r.declaration == Some(DeclarationKind::Parameter));
        assert_eq!(
            has_params, has_row,
            "{lang:?}: parameters node {has_params} vs parameter row {has_row}"
        );
    }
    assert!(!rows(Language::Bash)
        .iter()
        .any(|r| r.declaration == Some(DeclarationKind::Parameter)));
    assert!(!rows(Language::Terraform)
        .iter()
        .any(|r| r.declaration == Some(DeclarationKind::Parameter)));
}

#[test]
fn every_callable_boundary_kind_has_a_residual_capture_row() {
    for lang in Language::all() {
        for kind in lang.callable_boundary_node_types() {
            assert!(
                capture_rows(lang)
                    .iter()
                    .any(|r| r.kind == kind && r.variant.is_none()),
                "{lang:?} {kind}"
            );
        }
    }
}
