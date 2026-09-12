use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "19c46facc653381c337ff6cad75dd8b052524179a366c80825d6d0010520eef2";
pub(super) const CENSUS_DIGEST: &str =
    "5b60e3d97f4fb735a58c0752d4499d0dea07007d5faaaee58896a9cdbb7f3f67";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Java,
    "E4a-1",
    [
        ("annotation_type_body", "e0b-java-annotation_type_body"),
        (
            "annotation_type_declaration",
            "e0b-java-annotation_type_declaration"
        ),
        (
            "annotation_type_element_declaration",
            "e0b-java-annotation_type_element_declaration"
        ),
        ("assignment_expression", "e0b-java-assignment_expression"),
        ("block", "e0b-java-block"),
        ("catch_clause", "e0b-java-catch_clause"),
        ("catch_formal_parameter", "e0b-java-catch_formal_parameter"),
        ("catch_type", "e0b-java-catch_type"),
        ("class_body", "e0b-java-class_body"),
        ("class_declaration", "e0b-java-class_declaration"),
        ("class_literal", "e0b-java-class_literal"),
        (
            "compact_constructor_declaration",
            "e0b-java-compact_constructor_declaration"
        ),
        ("constant_declaration", "e0b-java-constant_declaration"),
        ("constructor_body", "e0b-java-constructor_body"),
        (
            "constructor_declaration",
            "e0b-java-constructor_declaration"
        ),
        ("enhanced_for_statement", "e0b-java-enhanced_for_statement"),
        ("enum_body", "e0b-java-enum_body"),
        ("enum_body_declarations", "e0b-java-enum_body_declarations"),
        ("enum_declaration", "e0b-java-enum_declaration"),
        ("field_declaration", "e0b-java-field_declaration"),
        ("for_statement", "e0b-java-for_statement"),
        ("formal_parameter", "e0b-java-formal_parameter"),
        ("import_declaration", "e0b-java-import_declaration"),
        ("inferred_parameters", "e0b-java-inferred_parameters"),
        ("instanceof_expression", "e0b-java-instanceof_expression"),
        ("interface_body", "e0b-java-interface_body"),
        ("interface_declaration", "e0b-java-interface_declaration"),
        ("lambda_expression", "e0b-java-lambda_expression"),
        (
            "local_variable_declaration",
            "e0b-java-local_variable_declaration"
        ),
        ("method_declaration", "e0b-java-method_declaration"),
        ("module_body", "e0b-java-module_body"),
        ("module_declaration", "e0b-java-module_declaration"),
        ("package_declaration", "e0b-java-package_declaration"),
        ("receiver_parameter", "e0b-java-receiver_parameter"),
        ("record_declaration", "e0b-java-record_declaration"),
        ("record_pattern", "e0b-java-record_pattern"),
        ("record_pattern_body", "e0b-java-record_pattern_body"),
        (
            "record_pattern_component",
            "e0b-java-record_pattern_component"
        ),
        ("superclass", "e0b-java-superclass"),
        ("switch_block", "e0b-java-switch_block"),
        (
            "switch_block_statement_group",
            "e0b-java-switch_block_statement_group"
        ),
        (
            "try_with_resources_statement",
            "e0b-java-try_with_resources_statement"
        ),
        ("type_parameter", "e0b-java-type_parameter"),
        ("type_parameters", "e0b-java-type_parameters"),
        ("type_pattern", "e0b-java-type_pattern"),
        ("variable_declarator", "e0b-java-variable_declarator"),
        ("block_comment", "e0b-java-block_comment"),
        ("underscore_pattern", "e0b-java-underscore_pattern"),
    ]
);
