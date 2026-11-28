use super::source::children;
use prism::ast::ParsedFile;
use tree_sitter::Node;

pub(in crate::targets::inventory) fn alias_name(node: Node<'_>) -> Option<Node<'_>> {
    let mut name = children(node.child_by_field_name("left")?).next()?;
    if name.kind() == "generic_type" {
        name = children(name).next()?;
    }
    (name.kind() == "identifier").then_some(name)
}

pub(in crate::targets::inventory) fn has_ambiguous_type_syntax(parsed: &ParsedFile) -> bool {
    // Inspect the whole original tree. The receiver walk skips alias operands
    // and may replace or refuse annotations before it sees them.
    let mut cursor = parsed.tree.walk();
    let mut nodes = vec![parsed.tree.root_node()];
    while let Some(node) = nodes.pop() {
        let ambiguous = match node.kind() {
            "type_alias_statement" => alias_name(node).is_none(),
            "constrained_type" => children(node)
                .next()
                .and_then(|left| children(left).next())
                .is_none_or(|operand| operand.kind() != "identifier"),
            "ERROR" => parsed.node_text(&node).starts_with("type("),
            _ => false,
        };
        if ambiguous {
            return true;
        }
        nodes.extend(node.named_children(&mut cursor));
    }
    false
}
