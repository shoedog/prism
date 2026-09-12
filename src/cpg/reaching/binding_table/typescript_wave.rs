use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "c790a733fc756b54d4e54dceeb7d2d51e40d8b57136e70277753a75804cce3e3";
pub(super) const CENSUS_DIGEST: &str =
    "f7642358e1507216b4b9d9bfd30a04aac9690223b72c271ab9e069150345a4f5";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::TypeScript,
    "E2b-1",
    [
        (
            "abstract_class_declaration",
            "e0b-ts-abstract_class_declaration"
        ),
        (
            "abstract_method_signature",
            "e0b-ts-abstract_method_signature"
        ),
        ("ambient_declaration", "e0b-ts-ambient_declaration"),
        ("array_pattern", "e0b-ts-array_pattern"),
        ("assignment_expression", "e0b-ts-assignment_expression"),
        ("assignment_pattern", "e0b-ts-assignment_pattern"),
        (
            "augmented_assignment_expression",
            "e0b-ts-augmented_assignment_expression"
        ),
        ("call_signature", "e0b-ts-call_signature"),
        ("catch_clause", "e0b-ts-catch_clause"),
        ("class", "e0b-ts-class"),
        ("class_heritage", "e0b-ts-class_heritage"),
        ("class_static_block", "e0b-ts-class_static_block"),
        ("construct_signature", "e0b-ts-construct_signature"),
        ("constructor_type", "e0b-ts-constructor_type"),
        ("enum_assignment", "e0b-ts-enum_assignment"),
        ("enum_body", "e0b-ts-enum_body"),
        ("enum_declaration", "e0b-ts-enum_declaration"),
        ("function_signature", "e0b-ts-function_signature"),
        ("function_type", "e0b-ts-function_type"),
        ("generator_function", "e0b-ts-generator_function"),
        (
            "generator_function_declaration",
            "e0b-ts-generator_function_declaration"
        ),
        ("import", "e0b-ts-import"),
        ("import_alias", "e0b-ts-import_alias"),
        ("import_attribute", "e0b-ts-import_attribute"),
        ("import_clause", "e0b-ts-import_clause"),
        ("import_require_clause", "e0b-ts-import_require_clause"),
        ("import_specifier", "e0b-ts-import_specifier"),
        ("import_statement", "e0b-ts-import_statement"),
        ("interface_body", "e0b-ts-interface_body"),
        ("interface_declaration", "e0b-ts-interface_declaration"),
        ("mapped_type_clause", "e0b-ts-mapped_type_clause"),
        ("method_definition", "e0b-ts-method_definition"),
        ("method_signature", "e0b-ts-method_signature"),
        ("named_imports", "e0b-ts-named_imports"),
        ("namespace_import", "e0b-ts-namespace_import"),
        (
            "object_assignment_pattern",
            "e0b-ts-object_assignment_pattern"
        ),
        ("object_pattern", "e0b-ts-object_pattern"),
        ("optional_parameter", "e0b-ts-optional_parameter"),
        ("pair_pattern", "e0b-ts-pair_pattern"),
        ("required_parameter", "e0b-ts-required_parameter"),
        ("rest_pattern", "e0b-ts-rest_pattern"),
        ("switch_body", "e0b-ts-switch_body"),
        ("switch_case", "e0b-ts-switch_case"),
        ("type_alias_declaration", "e0b-ts-type_alias_declaration"),
        ("type_parameter", "e0b-ts-type_parameter"),
        ("type_parameters", "e0b-ts-type_parameters"),
        ("variable_declarator", "e0b-ts-variable_declarator"),
        ("with_statement", "e0b-ts-with_statement"),
        ("regex_pattern", "e0b-ts-regex_pattern"),
        (
            "shorthand_property_identifier_pattern",
            "e0b-ts-shorthand_property_identifier_pattern"
        ),
    ]
);
