use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn collect_introduced_identifiers<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    identifiers: &mut Vec<Node<'a>>,
) -> bool {
    let Some(found) = introduced_identifiers(parsed, node) else {
        return false;
    };
    identifiers.extend(found);
    true
}

fn introduced_identifiers<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Vec<Node<'a>>> {
    if parsed.language != Language::Cpp {
        return None;
    }
    let mut identifiers = Vec::new();
    match node.kind() {
        "parameter_list" => {
            let mut cursor = node.walk();
            for parameter in node
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "parameter_declaration")
            {
                collect(parsed, parameter, &mut identifiers);
            }
        }
        "init_declarator"
        | "parameter_declaration"
        | "array_declarator"
        | "attributed_declarator"
        | "parenthesized_declarator"
        | "pointer_declarator"
        | "reference_declarator" => {
            if let Some(declarator) = node.child_by_field_name("declarator").or_else(|| {
                (node.kind() == "reference_declarator")
                    .then(|| node.named_child(0))
                    .flatten()
            }) {
                collect(parsed, declarator, &mut identifiers);
            }
        }
        _ => return None,
    }
    Some(identifiers)
}

fn collect<'a>(parsed: &ParsedFile, node: Node<'a>, identifiers: &mut Vec<Node<'a>>) {
    if parsed.language.is_identifier_node(node.kind()) {
        identifiers.push(node);
    } else if let Some(nested) = introduced_identifiers(parsed, node) {
        identifiers.extend(nested);
    }
}
