use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "a5dd5e5316dc9c00d6e370673260a1ba35fa8d422e6150debf5dea527aae7130";
pub(super) const CENSUS_DIGEST: &str =
    "91c8f55a260fcffbb2914c32fbafef2787c4cc87be67e602fcb4c321c8e72df6";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Go,
    "E1b",
    [
        ("assignment_statement", "e0b-go-assignment_statement"),
        ("communication_case", "e0b-go-communication_case"),
        ("default_case", "e0b-go-default_case"),
        ("expression_case", "e0b-go-expression_case"),
        ("field_declaration", "e0b-go-field_declaration"),
        ("field_declaration_list", "e0b-go-field_declaration_list"),
        ("for_clause", "e0b-go-for_clause"),
        ("func_literal", "e0b-go-func_literal"),
        ("function_declaration", "e0b-go-function_declaration"),
        ("function_type", "e0b-go-function_type"),
        ("import_declaration", "e0b-go-import_declaration"),
        ("import_spec", "e0b-go-import_spec"),
        ("import_spec_list", "e0b-go-import_spec_list"),
        ("method_declaration", "e0b-go-method_declaration"),
        ("method_elem", "e0b-go-method_elem"),
        ("parameter_declaration", "e0b-go-parameter_declaration"),
        ("range_clause", "e0b-go-range_clause"),
        ("type_case", "e0b-go-type_case"),
        ("type_declaration", "e0b-go-type_declaration"),
        (
            "type_parameter_declaration",
            "e0b-go-type_parameter_declaration"
        ),
        ("type_parameter_list", "e0b-go-type_parameter_list"),
        (
            "variadic_parameter_declaration",
            "e0b-go-variadic_parameter_declaration"
        ),
    ]
);
