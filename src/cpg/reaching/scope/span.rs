use super::*;

pub(super) fn function_scope(_parsed: &ParsedFile, func_node: &Node<'_>) -> ScopeSpan {
    func_node
        .child_by_field_name("body")
        .map(scope_span)
        .unwrap_or_else(|| scope_span(*func_node))
}

pub(in crate::cpg::reaching) fn scope_span(node: Node<'_>) -> ScopeSpan {
    if let Some(span) = rust_scope::let_header_scope(node) {
        return span;
    }
    ScopeSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        end_line: node.end_position().row + 1,
    }
}
