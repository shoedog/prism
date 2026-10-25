//! Rust-specific lexical scope derivation.

use super::{scope_span, ScopeSpan};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) const LET_ELSE_CFG_REVISIT: &str = "future:rust-let-else-cfg";

pub(super) fn closure_parameter_scope(parsed: &ParsedFile, node: Node<'_>) -> Option<ScopeSpan> {
    (parsed.language == Language::Rust && node.kind() == "closure_parameters")
        .then(|| node.parent())
        .flatten()
        .filter(|parent| parent.kind() == "closure_expression")
        .and_then(|closure| closure.child_by_field_name("body"))
        .map(scope_span)
}

pub(in crate::cpg::reaching) fn let_else_cfg_gap(parsed: &ParsedFile, edge: &FlowEdge) -> bool {
    if parsed.language != Language::Rust {
        return false;
    }
    let Some(use_byte) = edge_use_byte(parsed, edge) else {
        return false;
    };
    let Some(function) = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)
    else {
        return false;
    };
    let _revisit = LET_ELSE_CFG_REVISIT;
    let name = edge.from.path.base.as_str();
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut stack = vec![function];
    while let Some(node) = stack.pop() {
        if node.id() != function.id() && boundaries.contains(&node.kind()) {
            continue;
        }
        if node.kind() == "let_declaration" {
            if let Some(alternative) = node.child_by_field_name("alternative") {
                let use_inside = contains(alternative, use_byte);
                let def_inside = contains(alternative, edge.from.start_byte);
                let crosses = !use_inside
                    && use_byte >= node.end_byte()
                    && (def_inside || edge.from.start_byte < node.start_byte());
                if crosses && alternative_assigns_name(parsed, alternative, name, &boundaries) {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    false
}

fn alternative_assigns_name(
    parsed: &ParsedFile,
    alternative: Node<'_>,
    name: &str,
    boundaries: &[&str],
) -> bool {
    let mut stack = vec![alternative];
    while let Some(node) = stack.pop() {
        if node.id() != alternative.id() && boundaries.contains(&node.kind()) {
            continue;
        }
        if parsed.language.is_assignment_node(node.kind())
            && parsed
                .language
                .assignment_target(&node)
                .is_some_and(|target| target_assigns_name(parsed, target, name))
        {
            return true;
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    false
}

fn target_assigns_name(parsed: &ParsedFile, target: Node<'_>, name: &str) -> bool {
    match target.kind() {
        "identifier" => parsed.node_text(&target) == name,
        "tuple_expression" | "parenthesized_expression" => {
            let mut cursor = target.walk();
            let assigns = target
                .named_children(&mut cursor)
                .any(|child| target_assigns_name(parsed, child, name));
            assigns
        }
        _ => false,
    }
}

fn contains(node: Node<'_>, byte: usize) -> bool {
    node.start_byte() <= byte && byte < node.end_byte()
}

fn edge_use_byte(parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
    if edge.to.start_byte < edge.to.end_byte {
        return Some(edge.to.start_byte);
    }
    let mut identifiers = parsed
        .identifiers_on_line(edge.to.line)
        .into_iter()
        .filter(|node| parsed.node_text(node) == edge.to.path.base);
    let first = identifiers.next()?;
    identifiers.next().is_none().then_some(first.start_byte())
}
