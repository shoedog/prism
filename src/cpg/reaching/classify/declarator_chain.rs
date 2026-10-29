use super::super::binding_table::select_binding_row;
use super::super::grammar_lint::{visibility_span, SpanKind};
use super::super::transfer;
use super::super::DefSite;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn unproved_exclusive_route(
    parsed: &ParsedFile,
    edge: &FlowEdge,
    defs: &[DefSite],
    source_line: usize,
    use_line: usize,
    statements: &mut super::super::statement_lookup::StatementLookup<'_>,
) -> Option<usize> {
    if !matches!(parsed.language, Language::C | Language::Cpp) {
        return None;
    }
    let function = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)?;
    let (chain, span) = chain_owner(parsed, function, edge)?;
    let arms = parsed.statement_arms_in_function(&function);
    if incomparable(&arms, source_line, use_line)
        || defs.iter().any(|def| {
            !def.alias_derived
                && def.path == edge.from.path
                && (def.line, def.start_byte) != (edge.from.line, edge.from.start_byte)
                && span_contains(&span, def.start_byte)
                && statements
                    .get(def.line)
                    .is_some_and(|def_line| incomparable(&arms, def_line, use_line))
        })
    {
        return Some(chain.start_position().row + 1);
    }
    None
}

fn chain_owner<'a>(
    parsed: &ParsedFile,
    function: Node<'a>,
    edge: &FlowEdge,
) -> Option<(Node<'a>, SpanKind)> {
    let mut pending = vec![function];
    while let Some(node) = pending.pop() {
        if node.kind() == "declaration" {
            let mut declarators = node.walk();
            let is_chain = node
                .children_by_field_name("declarator", &mut declarators)
                .nth(1)
                .is_some();
            let row = is_chain.then(|| select_binding_row(parsed, node)).flatten();
            if let Some(row) = row.filter(|row| {
                transfer::row_introduced_identifiers(parsed, node, row.fields)
                    .into_iter()
                    .any(|identifier| parsed.node_text(&identifier) == edge.from.path.base)
            }) {
                let span = visibility_span(parsed, node);
                let source_owned = node.start_byte() <= edge.from.start_byte
                    && edge.from.start_byte < node.end_byte()
                    || span_contains(&span, edge.from.start_byte);
                let use_owned = span_contains(&span, edge.to.start_byte);
                if source_owned && use_owned && row.declaration.is_some() {
                    return Some((node, span));
                }
            }
        }
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    None
}

fn span_contains(span: &SpanKind, byte: usize) -> bool {
    matches!(span, SpanKind::Known(spans) if spans.iter().any(|candidate| candidate.contains(byte)))
}

fn incomparable(arms: &crate::ast::LexicalArms, left: usize, right: usize) -> bool {
    let (Some(left), Some(right)) = (arms.interval_of_line(left), arms.interval_of_line(right))
    else {
        return false;
    };
    let contains = |outer: (u32, u32), inner: (u32, u32)| outer.0 <= inner.0 && inner.0 <= outer.1;
    !contains(left, right) && !contains(right, left)
}

pub(super) fn chain_mate_read(parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
    if !matches!(parsed.language, Language::C | Language::Cpp) {
        return None;
    }
    let mut pending = vec![parsed.tree.root_node()];
    while let Some(node) = pending.pop() {
        let zero_width_line_match = edge.to.start_byte == edge.to.end_byte
            && node.start_position().row < edge.to.line
            && edge.to.line <= node.end_position().row + 1;
        let occurrence_match =
            node.start_byte() <= edge.to.start_byte && edge.to.end_byte <= node.end_byte();
        if node.kind() == "declaration" && (zero_width_line_match || occurrence_match) {
            let mut declarators = node.walk();
            if node
                .children_by_field_name("declarator", &mut declarators)
                .nth(1)
                .is_some()
            {
                let row = select_binding_row(parsed, node)?;
                if transfer::row_introduced_identifiers(parsed, node, row.fields)
                    .into_iter()
                    .any(|identifier| parsed.node_text(&identifier) == edge.to.path.base)
                {
                    return Some(node.start_position().row + 1);
                }
            }
        }
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    None
}
