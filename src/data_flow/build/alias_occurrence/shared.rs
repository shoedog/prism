//! Helpers shared by the Go, Rust and Python occurrence classifiers.
use crate::ast::ParsedFile;
use crate::ast::PathSpan;
use tree_sitter::Node;

/// The largest node whose byte range is exactly the span's (`u` in `u := o` spans the identifier and
/// its single-element `expression_list` alike). `None` when no node has the range.
pub(super) fn span_node<'t>(parsed: &'t ParsedFile, span: &PathSpan) -> Option<Node<'t>> {
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(span.start_byte, span.end_byte)?;
    while let Some(parent) = node.parent() {
        if parent.start_byte() == span.start_byte && parent.end_byte() == span.end_byte {
            node = parent;
        } else {
            break;
        }
    }
    (node.start_byte() == span.start_byte && node.end_byte() == span.end_byte).then_some(node)
}

/// The field name `node` occupies in its parent, if any.
pub(super) fn field_in_parent(node: Node<'_>) -> Option<&'static str> {
    let parent = node.parent()?;
    let mut cursor = parent.walk();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        if cursor.node().id() == node.id() {
            return cursor.field_name();
        }
        if !cursor.goto_next_sibling() {
            return None;
        }
    }
}

/// A plain ASCII-or-Unicode identifier: the only spelling a name occurrence may have.
pub(super) fn is_plain_ident(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Every node of the owner (pre-order, owner included).
pub(super) fn walk_owner<'t>(owner: &Node<'t>, mut visit: impl FnMut(Node<'t>)) {
    let mut stack = vec![*owner];
    while let Some(node) = stack.pop() {
        visit(node);
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
}
