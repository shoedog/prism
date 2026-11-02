//! The solver's edge-independent refusal predicates, in their original order.
use super::{
    deduplicate_definitions, DefSite, FileCfgEdgeIndex, RdUnavailable, RD_MAX_DEFS, RD_MAX_LINES,
};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(crate) fn unavailable_before_edges(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    cfg: &FileCfgEdgeIndex<'_>,
) -> Option<RdUnavailable> {
    let count = deduplicate_definitions(defs).len();
    if count > RD_MAX_DEFS {
        return Some(RdUnavailable::DefinitionsCapExceeded { actual: count });
    }
    let statements = parsed.statements_in_function(owner);
    if statements.len() > RD_MAX_LINES {
        return Some(RdUnavailable::StatementLinesCapExceeded {
            actual: statements.len(),
        });
    }
    let lines: BTreeMap<_, _> = statements
        .iter()
        .enumerate()
        .map(|(i, (line, _))| (*line, i + 1))
        .collect();
    // Join classification changes labels only; it never removes selected edges.
    if !cfg.has_owner_edges(&lines) {
        return Some(RdUnavailable::NoCfgEdges);
    }
    None
}
