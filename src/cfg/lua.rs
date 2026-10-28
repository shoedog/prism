use crate::ast::ParsedFile;
use tree_sitter::Node;

pub(super) fn function_has_goto(parsed: &ParsedFile, function: Node<'_>) -> bool {
    contains_goto(parsed, function, function)
}

fn contains_goto(parsed: &ParsedFile, node: Node<'_>, root: Node<'_>) -> bool {
    if node.id() != root.id()
        && parsed
            .language
            .callable_boundary_node_types()
            .contains(&node.kind())
    {
        return false;
    }
    if node.kind() == "goto_statement" {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| contains_goto(parsed, child, root));
    found
}

pub(super) fn body_has_exiting_break(parsed: &ParsedFile, construct: Node<'_>) -> bool {
    construct
        .child_by_field_name("body")
        .or_else(|| construct.child_by_field_name("consequence"))
        .is_some_and(|body| contains_exiting_break(parsed, body))
}

fn contains_exiting_break(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if node.kind() == "break_statement" {
        return true;
    }
    if parsed
        .language
        .callable_boundary_node_types()
        .contains(&node.kind())
        || matches!(
            node.kind(),
            "for_statement" | "for_in_statement" | "while_statement" | "repeat_statement"
        )
    {
        return false;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| contains_exiting_break(parsed, child));
    found
}
