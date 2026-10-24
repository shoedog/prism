use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;

pub(super) fn use_byte(parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
    if edge.to.start_byte < edge.to.end_byte {
        return Some(edge.to.start_byte);
    }
    let lines = std::collections::BTreeSet::from([edge.to.line]);
    let func_node = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)?;
    let mut spans = parsed
        .rvalue_identifier_spans_on_lines(&func_node, &lines)
        .into_iter()
        .filter(|span| span.line == edge.to.line && span.path == edge.to.path);
    if let Some(first) = spans.next() {
        return spans.next().is_none().then_some(first.start_byte);
    }
    let mut identifiers = parsed
        .identifiers_on_line(edge.to.line)
        .into_iter()
        .filter(|node| parsed.node_text(node) == edge.to.path.base);
    let first = identifiers.next()?;
    identifiers.next().is_none().then_some(first.start_byte())
}
