use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "23e819ef1eefd357bb6eba844f47f8492f6a88da3e5046b06ec4acd4da8a9fd6";
pub(super) const CENSUS_DIGEST: &str =
    "916cc92c419ca6bda615454e81a730f8d4ca0731d4513261aa2ca48f155763fa";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::C,
    "E4b",
    [
        (
            "abstract_array_declarator",
            "e0b-c-abstract_array_declarator"
        ),
        (
            "abstract_function_declarator",
            "e0b-c-abstract_function_declarator"
        ),
        (
            "abstract_parenthesized_declarator",
            "e0b-c-abstract_parenthesized_declarator"
        ),
        (
            "abstract_pointer_declarator",
            "e0b-c-abstract_pointer_declarator"
        ),
        ("array_declarator", "e0b-c-array_declarator"),
        ("assignment_expression", "e0b-c-assignment_expression"),
        ("attribute_declaration", "e0b-c-attribute_declaration"),
        ("attributed_declarator", "e0b-c-attributed_declarator"),
        ("case_statement", "e0b-c-case_statement"),
        ("declaration", "e0b-c-declaration"),
        ("declaration_list", "e0b-c-declaration_list"),
        ("field_declaration", "e0b-c-field_declaration"),
        ("field_declaration_list", "e0b-c-field_declaration_list"),
        ("for_statement", "e0b-c-for_statement"),
        ("function_declarator", "e0b-c-function_declarator"),
        ("function_definition", "e0b-c-function_definition"),
        ("init_declarator", "e0b-c-init_declarator"),
        ("parenthesized_declarator", "e0b-c-parenthesized_declarator"),
        ("pointer_declarator", "e0b-c-pointer_declarator"),
        ("preproc_function_def", "e0b-c-preproc_function_def"),
        ("seh_except_clause", "e0b-c-seh_except_clause"),
        ("storage_class_specifier", "e0b-c-storage_class_specifier"),
        (
            "subscript_range_designator",
            "e0b-c-subscript_range_designator"
        ),
        ("variadic_parameter", "e0b-c-variadic_parameter"),
    ]
);
