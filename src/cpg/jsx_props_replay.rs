//! Compile-only entry point for selector replay during the S3 base overlay.

use super::build::PendingEdge;
use super::var_index::VarIndex;
use super::{CodePropertyGraph, CpgEdge, CpgNode};
use crate::ast::ParsedFile;
use petgraph::graph::{DiGraph, NodeIndex};
use std::ops::Range;

// S3-STUB: Replace with the real collector using the supplied masked lookup.
// The mutant changes only full-conflict fall-through in that same selector.
pub(super) fn replay(
    _file: &ParsedFile,
    _opening: Range<usize>,
    _cpg: &CodePropertyGraph,
    _graph: &DiGraph<CpgNode, CpgEdge>,
    _lookup: &VarIndex,
    _target: NodeIndex,
    _fall_through_on_full_conflict: bool,
) -> Vec<PendingEdge> {
    Vec::new()
}
