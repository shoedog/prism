use super::super::binding_table::{select_binding_row, Role};
use super::super::grammar_lint::{visibility_span, SpanKind};
use super::super::{transfer, FlowEdge};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn direct_block_uncertainty(
    parsed: &ParsedFile,
    binder: Node<'_>,
    use_byte: Option<usize>,
) -> bool {
    parsed.language == Language::Cpp
        && matches!(
            binder.kind(),
            "declaration" | "structured_binding_declarator"
        )
        && binder.parent().is_some_and(|scope| {
            scope.kind() == "compound_statement"
                && use_byte
                    .is_some_and(|byte| binder.start_byte() <= byte && byte < scope.end_byte())
        })
}

pub(super) fn subsequent_definition_in_uncertain_owner(
    parsed: &ParsedFile,
    binder: Node<'_>,
    edge: &FlowEdge,
) -> bool {
    if parsed.language != Language::Cpp
        || edge.from.start_byte <= binder.end_byte()
        || !span_contains(visibility_span(parsed, binder), edge.from.start_byte)
    {
        return false;
    }
    !nested_binding_owns_source_scope(parsed, binder, edge)
}

fn nested_binding_owns_source_scope(
    parsed: &ParsedFile,
    binder: Node<'_>,
    edge: &FlowEdge,
) -> bool {
    fn walk(parsed: &ParsedFile, node: Node<'_>, binder: Node<'_>, edge: &FlowEdge) -> bool {
        let nested = node.id() != binder.id()
            && binder.start_byte() < node.start_byte()
            && select_binding_row(parsed, node).is_some_and(|row| {
                row.roles.has(Role::Binding)
                    && row.declaration.is_some()
                    && transfer::row_introduced_identifiers(parsed, node, row.fields)
                        .into_iter()
                        .any(|identifier| parsed.node_text(&identifier) == edge.from.path.base)
                    && span_contains(visibility_span(parsed, node), edge.from.start_byte)
            });
        if nested {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|child| walk(parsed, child, binder, edge));
        found
    }
    walk(parsed, parsed.tree.root_node(), binder, edge)
}

fn span_contains(span: SpanKind, byte: usize) -> bool {
    match span {
        SpanKind::Known(spans) => spans.into_iter().any(|span| span.contains(byte)),
        SpanKind::Empty | SpanKind::Unknown => false,
    }
}
