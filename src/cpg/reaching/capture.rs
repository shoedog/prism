use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::reaching::binding_table::select_capture_row;
use crate::data_flow::FlowEdge;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) struct CaptureFacts {
    ranges: Vec<(usize, usize)>,
    references: BTreeSet<(AccessPath, usize)>,
}

pub(super) fn capture_facts(parsed: &ParsedFile, func_node: Node<'_>) -> CaptureFacts {
    fn visit(parsed: &ParsedFile, node: Node<'_>, facts: &mut CaptureFacts) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if is_nested_callable_kind(parsed, child) {
                let body = child.child_by_field_name("body").unwrap_or(child);
                facts.ranges.push((body.start_byte(), body.end_byte()));
                collect_reference_identities(parsed, body, &mut facts.references);
            } else {
                visit(parsed, child, facts);
            }
        }
    }

    let mut facts = CaptureFacts {
        ranges: Vec::new(),
        references: BTreeSet::new(),
    };
    visit(parsed, func_node, &mut facts);
    facts
}

pub(super) fn is_capture(edge: &FlowEdge, facts: &CaptureFacts) -> bool {
    facts
        .references
        .contains(&(edge.to.path.clone(), edge.to.line))
        && !facts
            .ranges
            .iter()
            .any(|(start, end)| *start <= edge.from.start_byte && edge.from.start_byte < *end)
}

fn collect_reference_identities(
    parsed: &ParsedFile,
    node: Node<'_>,
    references: &mut BTreeSet<(AccessPath, usize)>,
) {
    if is_field_access_kind(node.kind()) {
        references.insert((
            AccessPath::from_expr(parsed.node_text(&node)),
            node.start_position().row + 1,
        ));
        if let Some(base) = leftmost_receiver_identifier(parsed, node) {
            references.insert((
                AccessPath::simple(parsed.node_text(&base)),
                base.start_position().row + 1,
            ));
        }
        return;
    }

    if parsed.language.is_identifier_node(node.kind()) {
        references.insert((
            AccessPath::simple(parsed.node_text(&node)),
            node.start_position().row + 1,
        ));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_reference_identities(parsed, child, references);
    }
}

fn leftmost_receiver_identifier<'a>(parsed: &ParsedFile, mut node: Node<'a>) -> Option<Node<'a>> {
    loop {
        let receiver = node
            .child_by_field_name("argument")
            .or_else(|| node.child_by_field_name("object"))
            .or_else(|| node.child_by_field_name("operand"))
            .or_else(|| node.named_child(0))?;
        if is_field_access_kind(receiver.kind()) {
            node = receiver;
            continue;
        }
        return parsed
            .language
            .is_identifier_node(receiver.kind())
            .then_some(receiver);
    }
}

fn is_field_access_kind(kind: &str) -> bool {
    matches!(
        kind,
        "field_expression"
            | "member_expression"
            | "selector_expression"
            | "attribute"
            | "field_access"
            | "dot_index_expression"
            | "method_index_expression"
    )
}

fn is_nested_callable_kind(parsed: &ParsedFile, node: Node<'_>) -> bool {
    select_capture_row(parsed, node).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_classifier_uses_the_selected_boundary_row() {
        let parsed = ParsedFile::parse(
            "test.py",
            "def outer():\n    def inner():\n        pass\n",
            crate::languages::Language::Python,
        )
        .unwrap();
        let inner = parsed
            .all_functions()
            .into_iter()
            .find(|node| {
                parsed
                    .language
                    .function_name(node)
                    .is_some_and(|name| parsed.node_text(&name) == "inner")
            })
            .expect("nested function");
        assert!(is_nested_callable_kind(&parsed, inner));
        assert!(!is_nested_callable_kind(
            &parsed,
            inner.child_by_field_name("body").expect("function body")
        ));
    }
}
