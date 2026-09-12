use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "fdfd4b1f3dca1516616a1eb615bb6c1ad3082b8937ad21070d0def5dcfe7e535";
pub(super) const CENSUS_DIGEST: &str =
    "48025b2cb02f0260542533f96b8a365fe9c49c91459261ea41ad97cc83c6e50d";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Cpp,
    "E4c",
    [
        (
            "abstract_array_declarator",
            "e0b-cpp-abstract_array_declarator"
        ),
        (
            "abstract_function_declarator",
            "e0b-cpp-abstract_function_declarator"
        ),
        (
            "abstract_parenthesized_declarator",
            "e0b-cpp-abstract_parenthesized_declarator"
        ),
        (
            "abstract_pointer_declarator",
            "e0b-cpp-abstract_pointer_declarator"
        ),
        (
            "abstract_reference_declarator",
            "e0b-cpp-abstract_reference_declarator"
        ),
        ("alias_declaration", "e0b-cpp-alias_declaration"),
        ("array_declarator", "e0b-cpp-array_declarator"),
        ("assignment_expression", "e0b-cpp-assignment_expression"),
        ("attribute_declaration", "e0b-cpp-attribute_declaration"),
        ("attributed_declarator", "e0b-cpp-attributed_declarator"),
        ("base_class_clause", "e0b-cpp-base_class_clause"),
        ("case_statement", "e0b-cpp-case_statement"),
        ("catch_clause", "e0b-cpp-catch_clause"),
        ("class_specifier", "e0b-cpp-class_specifier"),
        ("declaration", "e0b-cpp-declaration"),
        ("declaration_list", "e0b-cpp-declaration_list"),
        ("delete_expression", "e0b-cpp-delete_expression"),
        ("delete_method_clause", "e0b-cpp-delete_method_clause"),
        (
            "explicit_function_specifier",
            "e0b-cpp-explicit_function_specifier"
        ),
        ("field_declaration", "e0b-cpp-field_declaration"),
        ("field_declaration_list", "e0b-cpp-field_declaration_list"),
        ("for_range_loop", "e0b-cpp-for_range_loop"),
        ("for_statement", "e0b-cpp-for_statement"),
        ("friend_declaration", "e0b-cpp-friend_declaration"),
        ("function_declarator", "e0b-cpp-function_declarator"),
        ("function_definition", "e0b-cpp-function_definition"),
        ("init_declarator", "e0b-cpp-init_declarator"),
        (
            "lambda_capture_initializer",
            "e0b-cpp-lambda_capture_initializer"
        ),
        (
            "lambda_capture_specifier",
            "e0b-cpp-lambda_capture_specifier"
        ),
        ("lambda_default_capture", "e0b-cpp-lambda_default_capture"),
        ("lambda_expression", "e0b-cpp-lambda_expression"),
        ("new_declarator", "e0b-cpp-new_declarator"),
        ("noexcept", "e0b-cpp-noexcept"),
        (
            "optional_parameter_declaration",
            "e0b-cpp-optional_parameter_declaration"
        ),
        (
            "optional_type_parameter_declaration",
            "e0b-cpp-optional_type_parameter_declaration"
        ),
        (
            "parameter_pack_expansion",
            "e0b-cpp-parameter_pack_expansion"
        ),
        (
            "parenthesized_declarator",
            "e0b-cpp-parenthesized_declarator"
        ),
        ("pointer_declarator", "e0b-cpp-pointer_declarator"),
        ("pointer_type_declarator", "e0b-cpp-pointer_type_declarator"),
        ("preproc_function_def", "e0b-cpp-preproc_function_def"),
        ("reference_declarator", "e0b-cpp-reference_declarator"),
        ("requires_expression", "e0b-cpp-requires_expression"),
        ("seh_except_clause", "e0b-cpp-seh_except_clause"),
        (
            "static_assert_declaration",
            "e0b-cpp-static_assert_declaration"
        ),
        ("storage_class_specifier", "e0b-cpp-storage_class_specifier"),
        (
            "structured_binding_declarator",
            "e0b-cpp-structured_binding_declarator"
        ),
        (
            "subscript_range_designator",
            "e0b-cpp-subscript_range_designator"
        ),
        ("template_declaration", "e0b-cpp-template_declaration"),
        ("template_function", "e0b-cpp-template_function"),
        ("template_parameter_list", "e0b-cpp-template_parameter_list"),
        (
            "template_template_parameter_declaration",
            "e0b-cpp-template_template_parameter_declaration"
        ),
        (
            "type_parameter_declaration",
            "e0b-cpp-type_parameter_declaration"
        ),
        ("using_declaration", "e0b-cpp-using_declaration"),
        ("variadic_declarator", "e0b-cpp-variadic_declarator"),
        (
            "variadic_parameter_declaration",
            "e0b-cpp-variadic_parameter_declaration"
        ),
        (
            "variadic_type_parameter_declaration",
            "e0b-cpp-variadic_type_parameter_declaration"
        ),
    ]
);
