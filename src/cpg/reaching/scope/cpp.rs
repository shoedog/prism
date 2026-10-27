use super::{Binding, BindingId, BindingRelation, DeclarationKind};
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn condition_scope_owner(language: Language, parent: Node<'_>) -> Option<Node<'_>> {
    (language == Language::Cpp && parent.kind() == "condition_clause")
        .then(|| parent.parent())
        .flatten()
        .filter(|owner| {
            matches!(
                owner.kind(),
                "if_statement" | "while_statement" | "switch_statement"
            )
        })
}

pub(super) fn capture_body_scope(node: Node<'_>) -> Option<Node<'_>> {
    std::iter::successors(node.parent(), |node| node.parent())
        .find(|node| node.kind() == "lambda_expression")?
        .child_by_field_name("body")
}

pub(super) fn binding_id(kind: DeclarationKind, index: usize) -> BindingId {
    if kind == DeclarationKind::CaptureCopy {
        BindingId::CaptureCopy(index)
    } else {
        BindingId::Declaration(index)
    }
}

pub(super) fn missing_use_relation(definition: &Binding, line_byte: usize) -> BindingRelation {
    if matches!(definition.id, BindingId::CaptureCopy(_)) || definition.scope.contains(line_byte) {
        BindingRelation::Unresolved
    } else {
        BindingRelation::KilledAt(definition.scope.end_line)
    }
}

pub(super) fn outside_scope_relation(definition: &Binding) -> BindingRelation {
    if matches!(definition.id, BindingId::CaptureCopy(_)) {
        BindingRelation::Unresolved
    } else {
        BindingRelation::KilledAt(definition.scope.end_line)
    }
}
