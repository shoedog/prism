use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "0d80ab597fcf1310efb9694d4276c407655d060b8bbab8f4ffda0223c43e94bf";
pub(super) const CENSUS_DIGEST: &str =
    "eb97d8531425246460338d5bca13d810ae9cf30f8ccfa936922c896706198060";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::JavaScript,
    "E2a",
    [
        ("array_pattern", "e0b-js-array_pattern"),
        ("assignment_expression", "e0b-js-assignment_expression"),
        ("assignment_pattern", "e0b-js-assignment_pattern"),
        (
            "augmented_assignment_expression",
            "e0b-js-augmented_assignment_expression"
        ),
        ("catch_clause", "e0b-js-catch_clause"),
        ("class", "e0b-js-class"),
        ("class_heritage", "e0b-js-class_heritage"),
        ("class_static_block", "e0b-js-class_static_block"),
        ("generator_function", "e0b-js-generator_function"),
        (
            "generator_function_declaration",
            "e0b-js-generator_function_declaration"
        ),
        ("import", "e0b-js-import"),
        ("import_attribute", "e0b-js-import_attribute"),
        ("import_clause", "e0b-js-import_clause"),
        ("import_specifier", "e0b-js-import_specifier"),
        ("import_statement", "e0b-js-import_statement"),
        ("method_definition", "e0b-js-method_definition"),
        ("named_imports", "e0b-js-named_imports"),
        ("namespace_import", "e0b-js-namespace_import"),
        (
            "object_assignment_pattern",
            "e0b-js-object_assignment_pattern"
        ),
        ("object_pattern", "e0b-js-object_pattern"),
        ("pair_pattern", "e0b-js-pair_pattern"),
        ("rest_pattern", "e0b-js-rest_pattern"),
        ("switch_body", "e0b-js-switch_body"),
        ("switch_case", "e0b-js-switch_case"),
        ("variable_declarator", "e0b-js-variable_declarator"),
        ("with_statement", "e0b-js-with_statement"),
        ("regex_pattern", "e0b-js-regex_pattern"),
        (
            "shorthand_property_identifier_pattern",
            "e0b-js-shorthand_property_identifier_pattern"
        ),
    ]
);
