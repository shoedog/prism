use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "23e819ef1eefd357bb6eba844f47f8492f6a88da3e5046b06ec4acd4da8a9fd6";
pub(super) const CENSUS_DIGEST: &str =
    "4eb2f8173ecc13361b740351e0a593947bd2521862f2f073ad280543bdd91fe5";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::C,
    "future:c-residual",
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
        ("attribute_declaration", "e0b-c-attribute_declaration"),
        ("attributed_declarator", "e0b-c-attributed_declarator"),
        ("declaration_list", "e0b-c-declaration_list"),
        ("field_declaration", "e0b-c-field_declaration"),
        ("field_declaration_list", "e0b-c-field_declaration_list"),
        ("function_declarator", "e0b-c-function_declarator"),
        ("function_definition", "e0b-c-function_definition"),
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
