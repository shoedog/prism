use super::super::binding_table::{BindingRow, BindingScopeRule, DeclarationKind};
use super::{scope_span, ScopeSpan};
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn header_owner(language: Language, node: Node<'_>) -> Node<'_> {
    if language == Language::Lua
        && matches!(node.kind(), "for_numeric_clause" | "for_generic_clause")
    {
        node.parent()
            .filter(|parent| parent.kind() == "for_statement")
            .unwrap_or(node)
    } else {
        node
    }
}

pub(super) fn selected_declaration(
    language: Language,
    selected_row: Option<&BindingRow>,
    rule: BindingScopeRule,
) -> Option<DeclarationKind> {
    if language == Language::Lua && selected_row.is_some() {
        selected_row.and_then(|row| row.declaration)
    } else {
        selected_row
            .and_then(|row| row.declaration)
            .or(rule.declaration)
    }
}

pub(super) fn lexical_scope(language: Language, parent: Node<'_>) -> ScopeSpan {
    if language == Language::Lua && parent.kind() == "block" {
        if let Some(repeat) = parent
            .parent()
            .filter(|ancestor| ancestor.kind() == "repeat_statement")
        {
            return scope_span(repeat);
        }
    }
    scope_span(parent)
}
