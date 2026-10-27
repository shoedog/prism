use crate::ast::ParsedFile;
use tree_sitter::Node;

pub(super) fn variable_names<'a>(parsed: &ParsedFile, node: Node<'a>) -> Vec<Node<'a>> {
    fn collect<'a>(parsed: &ParsedFile, node: Node<'a>, out: &mut Vec<Node<'a>>) {
        if parsed.language.is_identifier_node(node.kind()) {
            out.push(node);
            return;
        }
        let descend = node.kind() == "variable_list"
            || matches!(
                node.kind(),
                "variable" | "assignment_statement" | "variable_declaration" | "for_generic_clause"
            );
        if descend {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect(parsed, child, out);
            }
        }
    }

    let mut identifiers = Vec::new();
    collect(parsed, node, &mut identifiers);
    identifiers
}
