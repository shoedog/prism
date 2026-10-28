#[path = "java/task20.rs"]
mod task20;

pub(super) const CASES: &[super::case::Case] = task20::CASES;
pub(super) const CURATED: &[(&str, &str)] = &[
    ("block", "java-block-shadow"),
    ("catch_clause", "java-catch-clause-scope"),
    ("catch_formal_parameter", "java-catch-param-masks"),
    ("enhanced_for_statement", "java-enhanced-for-header"),
    ("for_statement", "java-for-init-scope"),
    ("formal_parameters", "java-formal-parameter-masks"),
    (
        "local_variable_declaration",
        "java-local-without-initializer",
    ),
    ("spread_parameter", "java-spread-parameter-masks"),
    ("switch_block", "java-switch-block-scope"),
    ("switch_block_statement_group", "java-switch-group-scope"),
    ("try_with_resources_statement", "java-try-with-resources"),
];

#[test]
fn task20_java_binding_cases() {
    for case in CASES {
        super::case::check_case(case);
    }
}

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "constant_declaration" => "interface C { int VALUE = 1; }",
        "type_pattern" => "class C { void f(Object value) { switch (value) { case String text -> {} default -> {} } } }",
        "underscore_pattern" => "class C { void f(Object value) { switch (value) { case Pair(_, var right) -> {} default -> {} } } } record Pair(Object left, Object right) {}",
        _ => super::case::source(crate::languages::Language::Java),
    }
}
