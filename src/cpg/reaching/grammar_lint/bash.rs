use super::{collect_binding_identifiers, dominance_route_lines};
use crate::ast::ParsedFile;
use crate::cpg::reaching::Line;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

fn bare_declaration_names<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() == "variable_name")
        .collect()
}

pub(super) fn introduces_name(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    bare_declaration_names(node)
        .into_iter()
        .any(|identifier| parsed.node_text(&identifier) == name)
}

pub(super) fn collect_bare_declaration_lines(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_function_id: usize,
    lines: &mut BTreeMap<String, BTreeSet<Line>>,
) {
    if parsed.language != Language::Bash || node.kind() != "declaration_command" {
        return;
    }
    let dominance_lines = dominance_route_lines(node, root_function_id);
    for target in bare_declaration_names(node) {
        collect_binding_identifiers(parsed, target, &dominance_lines, lines);
    }
}
