pub(super) const CASES: &[super::case::Case] = &[];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("parameter_list", "e0a-x-c-parameter_list"),
    ("parameter_declaration", "e0a-x-c-parameter_declaration"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "abstract_function_declarator" => "void f(int (int));",
        "abstract_parenthesized_declarator" | "abstract_pointer_declarator" => {
            "void f(int (*)(int));"
        }
        "attributed_declarator" => "int value [[deprecated]];",
        "declaration_list" => "extern \"C\" { int value; }",
        _ => super::case::source(crate::languages::Language::C),
    }
}
