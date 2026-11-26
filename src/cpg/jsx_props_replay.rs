//! Replay JSX pairing against the observed pre-binding graph and masked lookup.

use super::build::PendingEdge;
use super::var_index::VarIndex;
use super::{CodePropertyGraph, CpgEdge, CpgNode};
use crate::ast::ParsedFile;
use petgraph::graph::{DiGraph, NodeIndex};
use std::ops::Range;

pub(super) fn replay(
    file: &ParsedFile,
    opening: Range<usize>,
    cpg: &CodePropertyGraph,
    graph: &DiGraph<CpgNode, CpgEdge>,
    lookup: &VarIndex,
    target: NodeIndex,
    fall_through_on_full_conflict: bool,
) -> Vec<PendingEdge> {
    cpg.replay_jsx_props(
        file,
        opening,
        graph,
        lookup,
        target,
        fall_through_on_full_conflict,
    )
}
