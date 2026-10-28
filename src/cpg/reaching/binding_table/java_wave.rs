use super::{
    provisional_rows, BindingRow, DeclarationKind, GuardSpanRow, Role, RoleSet, Ruling, Visibility,
};
use crate::languages::Language;
use tree_sitter::Node;

pub(super) const DIGEST: &str = "19c46facc653381c337ff6cad75dd8b052524179a366c80825d6d0010520eef2";
pub(super) const CENSUS_DIGEST: &str =
    "4c20a0721a3376c50bc43e6b43cca087573c477b447dcada659c387141ece7c8";

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

pub(super) static CURATED_ROWS: &[BindingRow] = &[
    scope_row("block", "java-block-shadow", Ruling::Classified),
    scope_row(
        "catch_clause",
        "java-catch-clause-scope",
        Ruling::Classified,
    ),
    BindingRow {
        language: Language::Java,
        kind: "catch_formal_parameter",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["name"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::Header,
        ruling: Ruling::Classified,
        regression: "java-catch-param-masks",
    },
    header_scope_row(
        "enhanced_for_statement",
        &["name"],
        "java-enhanced-for-header",
    ),
    scope_row("for_statement", "java-for-init-scope", Ruling::Classified),
    BindingRow {
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::AfterIntroduction,
        ruling: Ruling::Classified,
        regression: "java-local-without-initializer",
    },
    scope_row(
        "switch_block",
        "java-switch-block-scope",
        Ruling::Classified,
    ),
    scope_row(
        "switch_block_statement_group",
        "java-switch-group-scope",
        Ruling::Uncertain {
            reason: "Java switch statement groups share the enclosing switch block scope",
            revisit: "future:java-switch-scope",
        },
    ),
    header_scope_row(
        "try_with_resources_statement",
        &["resources"],
        "java-try-with-resources",
    ),
];

const fn scope_row(kind: &'static str, regression: &'static str, ruling: Ruling) -> BindingRow {
    BindingRow {
        language: Language::Java,
        kind,
        variant: None,
        roles: RoleSet::of(&[Role::Scope]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling,
        regression,
    }
}

const fn header_scope_row(
    kind: &'static str,
    fields: &'static [&'static str],
    regression: &'static str,
) -> BindingRow {
    BindingRow {
        language: Language::Java,
        kind,
        variant: None,
        roles: RoleSet::of(&[Role::Scope, Role::Binding]),
        fields,
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::Header,
        ruling: Ruling::Classified,
        regression,
    }
}

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Java,
    "future:java-residual",
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
        ("enum_body", "e0b-java-enum_body"),
        ("enum_body_declarations", "e0b-java-enum_body_declarations"),
        ("enum_declaration", "e0b-java-enum_declaration"),
        ("field_declaration", "e0b-java-field_declaration"),
        ("formal_parameter", "e0b-java-formal_parameter"),
        ("import_declaration", "e0b-java-import_declaration"),
        ("inferred_parameters", "e0b-java-inferred_parameters"),
        ("instanceof_expression", "e0b-java-instanceof_expression"),
        ("interface_body", "e0b-java-interface_body"),
        ("interface_declaration", "e0b-java-interface_declaration"),
        ("lambda_expression", "e0b-java-lambda_expression"),
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
        ("type_parameter", "e0b-java-type_parameter"),
        ("type_parameters", "e0b-java-type_parameters"),
        ("type_pattern", "e0b-java-type_pattern"),
        ("variable_declarator", "e0b-java-variable_declarator"),
        ("block_comment", "e0b-java-block_comment"),
        ("underscore_pattern", "e0b-java-underscore_pattern"),
    ]
);
