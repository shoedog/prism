use super::super::binding_table::select_binding_row;
use super::super::transfer;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;

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
