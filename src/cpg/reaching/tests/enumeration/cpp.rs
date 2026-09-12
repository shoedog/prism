pub(super) const CASES: &[super::case::Case] = &[];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("parameter_list", "e0a-x-cpp-parameter_list"),
    ("parameter_declaration", "e0a-x-cpp-parameter_declaration"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "abstract_array_declarator" => "void f(int []);",
        "abstract_parenthesized_declarator"
        | "abstract_pointer_declarator"
        | "abstract_reference_declarator" => "void f(int (*)(int), int (&)[2]);",
        "attribute_declaration" => "[[deprecated]];",
        "attributed_declarator" => "int value [[deprecated]];",
        "declaration_list" => "extern \"C\" { int value; }",
        "delete_expression" => "void f(int *value) { delete value; }",
        "optional_type_parameter_declaration" => "template <typename T = int> struct C {};",
        "parenthesized_declarator" => "int (function)(int);",
        "pointer_type_declarator" => "struct C {}; using Pointer = int C::*;",
        "template_function" => "void f() { auto pointer = function<int>; }",
        _ => super::case::source(crate::languages::Language::Cpp),
    }
}
