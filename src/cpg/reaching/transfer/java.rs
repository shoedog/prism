use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn collect_introduced_identifiers<'a>(
    language: Language,
    node: Node<'a>,
    identifiers: &mut Vec<Node<'a>>,
) -> bool {
    if language != Language::Java {
        return false;
    }
    match node.kind() {
        "variable_declarator" | "resource" => {
            identifiers.extend(node.child_by_field_name("name"));
            true
        }
        "resource_specification" => {
            let mut cursor = node.walk();
            identifiers.extend(
                node.named_children(&mut cursor)
                    .filter_map(|resource| resource.child_by_field_name("name")),
            );
            true
        }
        _ => false,
    }
}
