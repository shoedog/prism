use super::{provisional_rows, BindingRow, GuardSpanRow};
use crate::languages::Language;
use tree_sitter::Node;

pub(super) const DIGEST: &str = "19c46facc653381c337ff6cad75dd8b052524179a366c80825d6d0010520eef2";
pub(super) const CENSUS_DIGEST: &str =
    "a8dada12e0a02956df92434752c5fd5878f8835ab6d15d01ebffd83f41a0421c";

pub(super) static PATTERN_GUARD_SPANS: &[GuardSpanRow] = &[
    GuardSpanRow {
        binder_kind: "record_pattern",
        guard_kind: "instanceof_expression",
        owner_kind: "if_statement",
        condition_field: "condition",
        positive_fields: &["consequence"],
        negative_fields: &["alternative"],
        positive_condition_tail: true,
    },
    GuardSpanRow {
        binder_kind: "type_pattern",
        guard_kind: "instanceof_expression",
        owner_kind: "if_statement",
        condition_field: "condition",
        positive_fields: &["consequence"],
        negative_fields: &["alternative"],
        positive_condition_tail: true,
    },
    GuardSpanRow {
        binder_kind: "instanceof_expression",
        guard_kind: "instanceof_expression",
        owner_kind: "if_statement",
        condition_field: "condition",
        positive_fields: &["consequence"],
        negative_fields: &["alternative"],
        positive_condition_tail: true,
    },
];

pub(super) fn binding_identifiers(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "instanceof_expression" => node.child_by_field_name("name").into_iter().collect(),
        "record_pattern" => {
            let mut cursor = node.walk();
            let identifiers = node
                .named_children(&mut cursor)
                .find(|child| child.kind() == "record_pattern_body")
                .map(binding_identifiers)
                .unwrap_or_default();
            identifiers
        }
        "record_pattern_body" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .flat_map(binding_identifiers)
                .collect()
        }
        "record_pattern_component" | "type_pattern" => {
            let mut cursor = node.walk();
            match node.named_children(&mut cursor).last() {
                Some(target) if target.kind() == "identifier" => vec![target],
                Some(target) if target.kind() == "record_pattern" => binding_identifiers(target),
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

pub(super) fn statement_cannot_complete_normally(statement: Option<Node<'_>>) -> bool {
    let Some(statement) = statement else {
        return false;
    };
    if matches!(
        statement.kind(),
        "return_statement" | "throw_statement" | "break_statement" | "continue_statement"
    ) {
        return true;
    }
    if statement.kind() == "if_statement" {
        return statement_cannot_complete_normally(statement.child_by_field_name("consequence"))
            && statement_cannot_complete_normally(statement.child_by_field_name("alternative"));
    }
    if statement.kind() != "block" {
        return false;
    }
    statement
        .named_child(statement.named_child_count().saturating_sub(1))
        .is_some_and(|last| statement_cannot_complete_normally(Some(last)))
}

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
