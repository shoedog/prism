use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "78b5789145286799a27a0a7ecc36cc1bcb151f94ec7fa631b248459867010c8c";
pub(super) const CENSUS_DIGEST: &str =
    "3b792fad45cce01f413aafae5b4efe90703fa0a539f166081676befbc56515e8";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Tsx,
    "E2b-2",
    [
        (
            "abstract_class_declaration",
            "e0b-tsx-abstract_class_declaration"
        ),
        (
            "abstract_method_signature",
            "e0b-tsx-abstract_method_signature"
        ),
        ("ambient_declaration", "e0b-tsx-ambient_declaration"),
        ("array_pattern", "e0b-tsx-array_pattern"),
        ("assignment_expression", "e0b-tsx-assignment_expression"),
        ("assignment_pattern", "e0b-tsx-assignment_pattern"),
        (
            "augmented_assignment_expression",
            "e0b-tsx-augmented_assignment_expression"
        ),
        ("call_signature", "e0b-tsx-call_signature"),
        ("catch_clause", "e0b-tsx-catch_clause"),
        ("class", "e0b-tsx-class"),
        ("class_heritage", "e0b-tsx-class_heritage"),
        ("class_static_block", "e0b-tsx-class_static_block"),
        ("construct_signature", "e0b-tsx-construct_signature"),
        ("constructor_type", "e0b-tsx-constructor_type"),
        ("enum_assignment", "e0b-tsx-enum_assignment"),
        ("enum_body", "e0b-tsx-enum_body"),
        ("enum_declaration", "e0b-tsx-enum_declaration"),
        ("function_signature", "e0b-tsx-function_signature"),
        ("function_type", "e0b-tsx-function_type"),
        ("generator_function", "e0b-tsx-generator_function"),
        (
            "generator_function_declaration",
            "e0b-tsx-generator_function_declaration"
        ),
        ("import", "e0b-tsx-import"),
        ("import_alias", "e0b-tsx-import_alias"),
        ("import_attribute", "e0b-tsx-import_attribute"),
        ("import_clause", "e0b-tsx-import_clause"),
        ("import_require_clause", "e0b-tsx-import_require_clause"),
        ("import_specifier", "e0b-tsx-import_specifier"),
        ("import_statement", "e0b-tsx-import_statement"),
        ("interface_body", "e0b-tsx-interface_body"),
        ("interface_declaration", "e0b-tsx-interface_declaration"),
        ("mapped_type_clause", "e0b-tsx-mapped_type_clause"),
        ("method_definition", "e0b-tsx-method_definition"),
        ("method_signature", "e0b-tsx-method_signature"),
        ("named_imports", "e0b-tsx-named_imports"),
        ("namespace_import", "e0b-tsx-namespace_import"),
        (
            "object_assignment_pattern",
            "e0b-tsx-object_assignment_pattern"
        ),
        ("object_pattern", "e0b-tsx-object_pattern"),
        ("optional_parameter", "e0b-tsx-optional_parameter"),
        ("pair_pattern", "e0b-tsx-pair_pattern"),
        ("required_parameter", "e0b-tsx-required_parameter"),
        ("rest_pattern", "e0b-tsx-rest_pattern"),
        ("switch_body", "e0b-tsx-switch_body"),
        ("switch_case", "e0b-tsx-switch_case"),
        ("type_alias_declaration", "e0b-tsx-type_alias_declaration"),
        ("type_parameter", "e0b-tsx-type_parameter"),
        ("type_parameters", "e0b-tsx-type_parameters"),
        ("variable_declarator", "e0b-tsx-variable_declarator"),
        ("with_statement", "e0b-tsx-with_statement"),
        ("regex_pattern", "e0b-tsx-regex_pattern"),
        (
            "shorthand_property_identifier_pattern",
            "e0b-tsx-shorthand_property_identifier_pattern"
        ),
    ]
);
