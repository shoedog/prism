//! Intraprocedural reaching definitions over the existing line-granular CFG.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::{FlowEdge, VarLocation};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

use super::FlowConfidence;

/// Hard caps from the authorised measurement pass
/// (~/code/tools/logs/item2-census/REPORT.md §2.3, 92,338 functions).
/// RD_MAX_LINES bounds `stmt_lines.len()` — the CFG statement-line universe
/// returned by `ParsedFile::statements_in_function` — NOT the function's line
/// span (`end - start + 1`). Measured worst case: 590 defs, 331 statement
/// lines; 0 of 92,338 functions exceed either cap.
pub(crate) const RD_MAX_DEFS: usize = 2048;
pub(crate) const RD_MAX_LINES: usize = 4096;

pub(crate) type Line = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DefId(pub(crate) u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefSite {
    pub(crate) id: DefId,
    pub(crate) path: AccessPath,
    pub(crate) line: Line,
    pub(crate) start_byte: usize,
    pub(crate) alias_derived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RdUnavailable {
    DefinitionsCapExceeded { actual: usize },
    StatementLinesCapExceeded { actual: usize },
    NoCfgEdges,
}

impl RdUnavailable {
    pub(crate) fn is_def_cap(self) -> bool {
        matches!(self, Self::DefinitionsCapExceeded { .. })
    }

    pub(crate) fn is_line_cap(self) -> bool {
        matches!(self, Self::StatementLinesCapExceeded { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RdOutcome {
    Available(RdResult),
    Unavailable(RdUnavailable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RdResult {
    pub(crate) labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    pub(crate) loop_carried_edges: BTreeSet<(VarLocation, VarLocation)>,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct RdFileStats {
    pub functions_over_cap: usize,
    pub functions_without_cfg: usize,
}

/// STUB — every function is Unavailable with the no-CFG reason and no labels.
/// Conservative (nothing is ever Exact) and deliberately wrong. Replaced after RED.
pub(crate) fn reaching_definitions(
    _parsed: &ParsedFile,
    _func_node: &Node<'_>,
    _defs: &[DefSite],
    _dfg_edges: &[FlowEdge],
) -> RdOutcome {
    RdOutcome::Unavailable(RdUnavailable::NoCfgEdges)
}

#[cfg(test)]
mod tests;
