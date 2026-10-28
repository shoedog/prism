use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;

pub(super) fn use_byte(parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
    if edge.to.start_byte < edge.to.end_byte {
        return Some(edge.to.start_byte);
    }
    let candidates = candidate_bytes(parsed, edge);
    match candidates.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

pub(super) fn candidate_bytes(parsed: &ParsedFile, edge: &FlowEdge) -> Vec<usize> {
    let lines = std::collections::BTreeSet::from([edge.to.line]);
    let Some(func_node) = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)
    else {
        return Vec::new();
    };
    let spans: Vec<_> = parsed
        .rvalue_identifier_spans_on_lines(&func_node, &lines)
        .into_iter()
        .filter(|span| span.line == edge.to.line && span.path == edge.to.path)
        .map(|span| span.start_byte)
        .collect();
    if !spans.is_empty() {
        return spans;
    }
    parsed
        .identifiers_on_line(edge.to.line)
        .into_iter()
        .filter(|node| parsed.node_text(node) == edge.to.path.base)
        .map(|node| node.start_byte())
        .collect()
}
