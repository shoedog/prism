use super::super::binding_table::{select_binding_row, Ruling};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn uncertain_node_introduces_name(
    parsed: &ParsedFile,
    node: Node<'_>,
    name: &str,
) -> Option<bool> {
    if parsed.language != Language::Cpp {
        return None;
    }
    let row = select_binding_row(parsed, node)
        .filter(|row| matches!(row.ruling, Ruling::Uncertain { .. }))?;
    Some(
        super::super::transfer::row_introduced_identifiers(parsed, node, row.fields)
            .into_iter()
            .any(|identifier| parsed.node_text(&identifier) == name),
    )
}

pub(super) fn has_byte_scoped_uncertainty(parsed: &ParsedFile, node: Node<'_>) -> bool {
    parsed.language == Language::Cpp
        && node.kind() == "declaration"
        && select_binding_row(parsed, node)
            .is_some_and(|row| matches!(row.ruling, Ruling::Uncertain { .. }))
}
