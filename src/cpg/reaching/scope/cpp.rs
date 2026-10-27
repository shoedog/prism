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
