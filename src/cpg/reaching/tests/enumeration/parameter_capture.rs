use super::super::super::binding_table::{
    capture_rows, rows, DeclarationKind, Predicate, Role, Ruling, Timing, Visibility,
};
use super::super::{function, parsed};
use crate::languages::Language;

struct ExpectedParameterRow {
    lookup_language: Language,
    language: Language,
    kind: &'static str,
    variant: Option<Predicate>,
    creates_scope: bool,
    is_binding: bool,
    fields: &'static [&'static str],
    declaration: Option<DeclarationKind>,
    visibility: Visibility,
    ruling: Ruling,
    regression: &'static str,
}

const EXPECTED_PARAMETER_ROWS: &[ExpectedParameterRow] = &[
    parameter_row(
        Language::Python,
        Language::Python,
        "parameters",
        &[],
        "e0a-x-py-parameters",
    ),
    parameter_row(
        Language::JavaScript,
        Language::JavaScript,
        "formal_parameters",
        &[],
        "e0a-x-js-formal_parameters",
    ),
    parameter_row(
        Language::TypeScript,
        Language::JavaScript,
        "formal_parameters",
        &[],
        "e0a-x-js-formal_parameters",
    ),
    parameter_row(
        Language::Tsx,
        Language::JavaScript,
        "formal_parameters",
        &[],
        "e0a-x-js-formal_parameters",
    ),
    parameter_row(
        Language::Go,
        Language::Go,
        "parameter_list",
        &[],
        "e0a-x-go-parameter_list",
    ),
    parameter_row(
        Language::Java,
        Language::Java,
        "formal_parameters",
        &[],
        "e0a-x-java-formal_parameters",
    ),
    parameter_row(
        Language::Java,
        Language::Java,
        "spread_parameter",
        &[],
        "e0a-x-java-spread_parameter",
    ),
    parameter_row(
        Language::C,
        Language::C,
        "parameter_list",
        &[],
        "e0a-x-c-parameter_list",
    ),
    parameter_row(
        Language::C,
        Language::C,
        "parameter_declaration",
        &["declarator"],
        "e0a-x-c-parameter_declaration",
    ),
    parameter_row(
        Language::Cpp,
        Language::Cpp,
        "parameter_list",
        &[],
        "e0a-x-cpp-parameter_list",
    ),
    parameter_row(
        Language::Cpp,
        Language::Cpp,
        "parameter_declaration",
        &["declarator"],
        "e0a-x-cpp-parameter_declaration",
    ),
    parameter_row(
        Language::Rust,
        Language::Rust,
        "parameters",
        &[],
        "e0a-x-rs-parameters",
    ),
    parameter_row(
        Language::Lua,
        Language::Lua,
        "parameters",
        &["name"],
        "e0a-x-lua-parameters",
    ),
];

const fn parameter_row(
    lookup_language: Language,
    language: Language,
    kind: &'static str,
    fields: &'static [&'static str],
    regression: &'static str,
) -> ExpectedParameterRow {
    ExpectedParameterRow {
        lookup_language,
        language,
        kind,
        variant: None,
        creates_scope: false,
        is_binding: true,
        fields,
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression,
    }
}

struct ExpectedCaptureRow {
    language: Language,
    kind: &'static str,
    variant: Option<Predicate>,
    timing: Timing,
    regression: &'static str,
}

const EXPECTED_CAPTURE_ROWS: &[ExpectedCaptureRow] = &[
    capture_row(
        Language::Python,
        "function_definition",
        "e0a-x-capture-py-function_definition",
    ),
    capture_row(
        Language::Python,
        "decorated_definition",
        "e0a-x-capture-py-decorated_definition",
    ),
    capture_row(Language::Python, "lambda", "e0a-x-capture-py-lambda"),
    capture_row(
        Language::JavaScript,
        "function_declaration",
        "e0a-x-capture-js-function_declaration",
    ),
    capture_row(
        Language::JavaScript,
        "method_definition",
        "e0a-x-capture-js-method_definition",
    ),
    capture_row(
        Language::JavaScript,
        "arrow_function",
        "e0a-x-capture-js-arrow_function",
    ),
    capture_row(
        Language::JavaScript,
        "function_expression",
        "e0a-x-capture-js-function_expression",
    ),
    capture_row(
        Language::JavaScript,
        "generator_function_declaration",
        "e0a-x-capture-js-generator_function_declaration",
    ),
    capture_row(
        Language::JavaScript,
        "generator_function",
        "e0a-x-capture-js-generator_function",
    ),
    capture_row(
        Language::TypeScript,
        "function_declaration",
        "e0a-x-capture-ts-function_declaration",
    ),
    capture_row(
        Language::TypeScript,
        "method_definition",
        "e0a-x-capture-ts-method_definition",
    ),
    capture_row(
        Language::TypeScript,
        "arrow_function",
        "e0a-x-capture-ts-arrow_function",
    ),
    capture_row(
        Language::TypeScript,
        "function_expression",
        "e0a-x-capture-ts-function_expression",
    ),
    capture_row(
        Language::TypeScript,
        "generator_function_declaration",
        "e0a-x-capture-ts-generator_function_declaration",
    ),
    capture_row(
        Language::TypeScript,
        "generator_function",
        "e0a-x-capture-ts-generator_function",
    ),
    capture_row(
        Language::Tsx,
        "function_declaration",
        "e0a-x-capture-tsx-function_declaration",
    ),
    capture_row(
        Language::Tsx,
        "method_definition",
        "e0a-x-capture-tsx-method_definition",
    ),
    capture_row(
        Language::Tsx,
        "arrow_function",
        "e0a-x-capture-tsx-arrow_function",
    ),
    capture_row(
        Language::Tsx,
        "function_expression",
        "e0a-x-capture-tsx-function_expression",
    ),
    capture_row(
        Language::Tsx,
        "generator_function_declaration",
        "e0a-x-capture-tsx-generator_function_declaration",
    ),
    capture_row(
        Language::Tsx,
        "generator_function",
        "e0a-x-capture-tsx-generator_function",
    ),
    capture_row(
        Language::Go,
        "function_declaration",
        "e0a-x-capture-go-function_declaration",
    ),
    capture_row(
        Language::Go,
        "method_declaration",
        "e0a-x-capture-go-method_declaration",
    ),
    capture_row(
        Language::Go,
        "func_literal",
        "e0a-x-capture-go-func_literal",
    ),
    capture_row(
        Language::Java,
        "method_declaration",
        "e0a-x-capture-java-method_declaration",
    ),
    capture_row(
        Language::Java,
        "constructor_declaration",
        "e0a-x-capture-java-constructor_declaration",
    ),
    capture_row(
        Language::Java,
        "lambda_expression",
        "e0a-x-capture-java-lambda_expression",
    ),
    capture_row(
        Language::C,
        "function_definition",
        "e0a-x-capture-c-function_definition",
    ),
    capture_row(
        Language::Cpp,
        "function_definition",
        "e0a-x-capture-cpp-function_definition",
    ),
    capture_row(
        Language::Cpp,
        "template_declaration",
        "e0a-x-capture-cpp-template_declaration",
    ),
    capture_row(
        Language::Cpp,
        "lambda_expression",
        "e0a-x-capture-cpp-lambda_expression",
    ),
    capture_row(
        Language::Rust,
        "function_item",
        "e0a-x-capture-rs-function_item",
    ),
    capture_row(
        Language::Rust,
        "closure_expression",
        "e0a-x-capture-rs-closure_expression",
    ),
    capture_row(
        Language::Rust,
        "async_block",
        "e0a-x-capture-rs-async_block",
    ),
    capture_row(Language::Rust, "gen_block", "e0a-x-capture-rs-gen_block"),
    capture_row(
        Language::Lua,
        "function_declaration",
        "e0a-x-capture-lua-function_declaration",
    ),
    capture_row(
        Language::Lua,
        "function_definition",
        "e0a-x-capture-lua-function_definition",
    ),
    capture_row(Language::Terraform, "block", "e0a-x-capture-tf-block"),
    capture_row(
        Language::Bash,
        "function_definition",
        "e0a-x-capture-bash-function_definition",
    ),
];

const fn capture_row(
    language: Language,
    kind: &'static str,
    regression: &'static str,
) -> ExpectedCaptureRow {
    ExpectedCaptureRow {
        language,
        kind,
        variant: None,
        timing: Timing::Deferred,
        regression,
    }
}

#[test]
fn every_parameter_row_matches_the_expected_table() {
    for lookup_language in Language::all() {
        let actual = rows(lookup_language)
            .iter()
            .filter(|row| row.declaration == Some(DeclarationKind::Parameter))
            .collect::<Vec<_>>();
        let expected = EXPECTED_PARAMETER_ROWS
            .iter()
            .filter(|row| row.lookup_language == lookup_language)
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), expected.len(), "{lookup_language:?}");
        for expected in expected {
            let row = actual
                .iter()
                .find(|row| row.kind == expected.kind)
                .unwrap_or_else(|| panic!("{lookup_language:?}: missing row {}", expected.kind));
            assert_eq!(
                row.language, expected.language,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.variant, expected.variant,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.roles.has(Role::Scope),
                expected.creates_scope,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.roles.has(Role::Binding),
                expected.is_binding,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert!(
                !row.roles.has(Role::NotBinding),
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.fields, expected.fields,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.declaration, expected.declaration,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.visibility, expected.visibility,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.ruling, expected.ruling,
                "{lookup_language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.regression, expected.regression,
                "{lookup_language:?} {}",
                expected.kind
            );
        }
    }
}

#[test]
fn every_capture_residual_row_matches_the_expected_table() {
    for language in Language::all() {
        let actual = capture_rows(language);
        let expected = EXPECTED_CAPTURE_ROWS
            .iter()
            .filter(|row| row.language == language)
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), expected.len(), "{language:?}");
        for expected in expected {
            let row = actual
                .iter()
                .find(|row| row.kind == expected.kind)
                .unwrap_or_else(|| panic!("{language:?}: missing row {}", expected.kind));
            assert_eq!(
                row.language, expected.language,
                "{language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.variant, expected.variant,
                "{language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.timing, expected.timing,
                "{language:?} {}",
                expected.kind
            );
            assert_eq!(
                row.regression, expected.regression,
                "{language:?} {}",
                expected.kind
            );
        }
    }
}

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
