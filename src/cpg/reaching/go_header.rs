//! Keep a separately anchored Go initializer and its condition/tag together.
//! The keyword is entry-only; header evaluation dispatches from the initializer.
use crate::ast::{ParsedFile, StatementSpan};
use crate::languages::Language;

pub(super) fn anchor(parsed: &ParsedFile, span: &StatementSpan) -> usize {
    if parsed.language != Language::Go
        || !matches!(
            span.kind.as_str(),
            "if_statement" | "expression_switch_statement" | "type_switch_statement"
        )
    {
        return span.line;
    }
    parsed
        .tree
        .root_node()
        .named_descendant_for_byte_range(span.start_byte, span.start_byte + 1)
        .filter(|node| node.kind() == span.kind)
        .and_then(|node| node.child_by_field_name("initializer"))
        .filter(|init| parsed.language.is_statement_node(init.kind()))
        .map_or(span.line, |init| init.start_position().row + 1)
}
