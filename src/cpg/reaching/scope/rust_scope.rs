//! Rust-specific lexical scope derivation.

use super::{scope_span, DefSite, ScopeSpan};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) const LET_ELSE_CFG_REVISIT: &str = "future:rust-let-else-cfg";

pub(super) fn pattern_excludes_def(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    if parsed.language != Language::Rust {
        return false;
    }
    let pattern = if node.kind() == "let_declaration" {
        node.child_by_field_name("pattern").filter(|pattern| {
            super::super::grammar_lint::contains_scope_bounded_rust_pattern(*pattern)
        })
    } else {
        super::super::grammar_lint::contains_scope_bounded_rust_pattern(node).then_some(node)
    };
    pattern.is_some_and(|pattern| {
        !super::super::grammar_lint::rust_pattern_introduces_name(parsed, pattern, name)
    })
}

pub(super) fn pattern_declaration_scope(
    parsed: &ParsedFile,
    pattern: Node<'_>,
) -> Option<ScopeSpan> {
    use super::super::grammar_lint::{visibility_span, SpanKind};

    if parsed.language != Language::Rust
        || !super::super::grammar_lint::contains_scope_bounded_rust_pattern(pattern)
    {
        return None;
    }
    if let Some(scope) = enclosing_let_declaration_scope(pattern) {
        return Some(scope);
    }
    match visibility_span(parsed, pattern) {
        SpanKind::Known(spans) => {
            let start_byte = spans.iter().map(|span| span.start).min()?;
            let end_byte = spans.iter().map(|span| span.end).max()?;
            Some(ScopeSpan {
                start_byte,
                end_byte,
                end_line: parsed.line_for_byte(end_byte.saturating_sub(1)),
            })
        }
        SpanKind::Empty => Some(ScopeSpan {
            start_byte: pattern.end_byte(),
            end_byte: pattern.end_byte(),
            end_line: pattern.end_position().row + 1,
        }),
        SpanKind::Unknown => None,
    }
}

pub(super) fn pattern_visible_from(node: Node<'_>) -> usize {
    enclosing_let_declaration(node).map_or(node.end_byte(), |declaration| declaration.end_byte())
}

fn enclosing_let_declaration_scope(node: Node<'_>) -> Option<ScopeSpan> {
    let declaration = enclosing_let_declaration(node)?;
    let mut current = declaration;
    while let Some(parent) = current.parent() {
        if matches!(
            parent.kind(),
            "block" | "unsafe_block" | "const_block" | "try_block" | "match_arm"
        ) {
            return Some(scope_span(parent));
        }
        current = parent;
    }
    None
}

fn enclosing_let_declaration(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        let parent = node.parent()?;
        if parent.kind() == "let_declaration" {
            return Some(parent);
        }
        node = parent;
    }
}

pub(super) fn let_header_scope(node: Node<'_>) -> Option<ScopeSpan> {
    if !matches!(node.kind(), "let_condition" | "let_chain") {
        return None;
    }
    let start_byte = node.end_byte();
    let mut current = node;
    while let Some(parent) = current.parent() {
        let body = match parent.kind() {
            "match_arm" => parent.child_by_field_name("value"),
            "if_expression" => parent.child_by_field_name("consequence"),
            "while_expression" => parent.child_by_field_name("body"),
            _ => {
                current = parent;
                continue;
            }
        }?;
        return Some(ScopeSpan {
            start_byte,
            end_byte: body.end_byte(),
            end_line: body.end_position().row + 1,
        });
    }
    None
}

pub(super) fn closure_parameter_scope(parsed: &ParsedFile, node: Node<'_>) -> Option<ScopeSpan> {
    (parsed.language == Language::Rust && node.kind() == "closure_parameters")
        .then(|| node.parent())
        .flatten()
        .filter(|parent| parent.kind() == "closure_expression")
        .and_then(|closure| closure.child_by_field_name("body"))
        .map(scope_span)
}

pub(in crate::cpg::reaching) fn let_else_cfg_gap(
    parsed: &ParsedFile,
    edge: &FlowEdge,
    defs: &[DefSite],
) -> bool {
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
                    && (def_inside
                        || (edge.from.start_byte < node.start_byte()
                            && use_byte >= node.end_byte()));
                if crosses && alternative_assigns_name(alternative, name, defs) {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    false
}

fn alternative_assigns_name(alternative: Node<'_>, name: &str, defs: &[DefSite]) -> bool {
    defs.iter()
        .any(|def| def.path.base == name && contains(alternative, def.start_byte))
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
