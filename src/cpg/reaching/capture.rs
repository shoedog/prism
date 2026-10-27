use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::reaching::binding_table::{select_capture_row, Timing};
use crate::data_flow::FlowEdge;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) struct CaptureFacts {
    ranges: Vec<(usize, usize, Timing)>,
    capture_ranges: Vec<(usize, usize)>,
    references: BTreeSet<(AccessPath, usize)>,
    timing_references: BTreeSet<(AccessPath, usize, usize, usize)>,
}

pub(super) fn capture_facts(parsed: &ParsedFile, func_node: Node<'_>) -> CaptureFacts {
    fn visit_node(
        parsed: &ParsedFile,
        node: Node<'_>,
        inside_boundary: bool,
        facts: &mut CaptureFacts,
    ) {
        if let Some(row) = select_capture_row(parsed, node) {
            let body = node.child_by_field_name("body").unwrap_or(node);
            let range = (body.start_byte(), body.end_byte());
            facts.ranges.push((range.0, range.1, row.timing));
            collect_reference_identities(
                parsed,
                body,
                range,
                !inside_boundary,
                &mut facts.references,
                &mut facts.timing_references,
            );
            if !inside_boundary {
                facts.capture_ranges.push(range);
            }
            if body.id() != node.id() {
                visit_node(parsed, body, true, facts);
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            visit_node(parsed, child, inside_boundary, facts);
        }
    }

    let mut facts = CaptureFacts {
        ranges: Vec::new(),
        capture_ranges: Vec::new(),
        references: BTreeSet::new(),
        timing_references: BTreeSet::new(),
    };
    let mut cursor = func_node.walk();
    for child in func_node.children(&mut cursor) {
        visit_node(parsed, child, false, &mut facts);
    }
    facts
}

pub(super) fn is_capture(edge: &FlowEdge, facts: &CaptureFacts) -> Option<Timing> {
    if !facts
        .references
        .contains(&(edge.to.path.clone(), edge.to.line))
        || facts
            .capture_ranges
            .iter()
            .any(|(start, end)| *start <= edge.from.start_byte && edge.from.start_byte < *end)
    {
        return None;
    }
    facts
        .ranges
        .iter()
        .filter(|(start, end, _)| {
            facts
                .timing_references
                .contains(&(edge.to.path.clone(), edge.to.line, *start, *end))
        })
        .min_by_key(|(start, end, _)| end - start)
        .map(|(_, _, timing)| *timing)
}

fn collect_reference_identities(
    parsed: &ParsedFile,
    node: Node<'_>,
    range: (usize, usize),
    capture_reference: bool,
    references: &mut BTreeSet<(AccessPath, usize)>,
    timing_references: &mut BTreeSet<(AccessPath, usize, usize, usize)>,
) {
    if is_field_access_kind(node.kind()) {
        let identity = (
            AccessPath::from_expr(parsed.node_text(&node)),
            node.start_position().row + 1,
        );
        if capture_reference {
            references.insert(identity.clone());
        }
        timing_references.insert((identity.0, identity.1, range.0, range.1));
        if let Some(base) = leftmost_receiver_identifier(parsed, node) {
            let identity = (
                AccessPath::simple(parsed.node_text(&base)),
                base.start_position().row + 1,
            );
            if capture_reference {
                references.insert(identity.clone());
            }
            timing_references.insert((identity.0, identity.1, range.0, range.1));
        }
        return;
    }

    if parsed.language.is_identifier_node(node.kind()) {
        let identity = (
            AccessPath::simple(parsed.node_text(&node)),
            node.start_position().row + 1,
        );
        if capture_reference {
            references.insert(identity.clone());
        }
        timing_references.insert((identity.0, identity.1, range.0, range.1));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_reference_identities(
            parsed,
            child,
            range,
            capture_reference,
            references,
            timing_references,
        );
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
        assert!(select_capture_row(&parsed, inner).is_some());
        assert!(select_capture_row(
            &parsed,
            inner.child_by_field_name("body").expect("function body")
        )
        .is_none());
    }
}
