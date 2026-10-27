use super::super::graph::{path_exists, path_exists_avoiding, BitSet};
use super::super::Line;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use std::collections::BTreeMap;

#[allow(clippy::too_many_arguments)]
pub(super) fn dominant(
    parsed: &ParsedFile,
    edge: &FlowEdge,
    candidates: &[usize],
    use_node: usize,
    mapped_defs: &[Option<usize>],
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    statements: &BTreeMap<Line, usize>,
) -> Option<Line> {
    if parsed.language != Language::Python {
        return None;
    }
    statements
        .iter()
        .filter(|(line, _)| name_on_line_is_in_finally(parsed, edge, **line))
        .filter(|(_, node)| {
            **node != use_node
                && candidates.iter().all(|index| {
                    mapped_defs[*index].is_some_and(|def_node| {
                        kill[**node].contains(*index)
                            && path_exists(successors, def_node, **node, false)
                            && path_exists(successors, **node, use_node, false)
                            && !path_exists_avoiding(successors, def_node, use_node, **node)
                    })
                })
        })
        .map(|(line, _)| *line)
        .min()
}

fn name_on_line_is_in_finally(parsed: &ParsedFile, edge: &FlowEdge, line: Line) -> bool {
    parsed.identifiers_on_line(line).into_iter().any(|node| {
        parsed.node_text(&node) == edge.to.path.base
            && std::iter::successors(Some(node), |node| node.parent())
                .any(|ancestor| ancestor.kind() == "finally_clause")
    })
}
