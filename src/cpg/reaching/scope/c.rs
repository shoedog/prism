use super::ScopeSpan;
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn scope_span(parsed: &ParsedFile, node: Node<'_>) -> ScopeSpan {
    let mut span = super::scope_span(node);
    if parsed.language == Language::C
        && node.kind() == "for_statement"
        && node.start_position().row == node.end_position().row
    {
        span.end_line += 1;
    }
    span
}
