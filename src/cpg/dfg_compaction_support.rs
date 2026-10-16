//! Temporary test/feature-only CPG assembly and payload observation seam.
use super::CodePropertyGraph;
use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::data_flow::{oracle::LegacyDataFlowGraph, DataFlowGraph};
use crate::type_db::TypeDatabase;
use std::collections::BTreeMap;

impl CodePropertyGraph {
    pub fn assemble_with_dfg_for_compaction(
        cg: CallGraph,
        dfg: DataFlowGraph,
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
    ) -> Self {
        Self::assemble_graph(cg, dfg, files, type_db)
    }
}
pub fn assemble_with_dfg(
    files: &BTreeMap<String, ParsedFile>,
    dfg: DataFlowGraph,
) -> CodePropertyGraph {
    CodePropertyGraph::assemble_with_dfg_for_compaction(CallGraph::build(files), dfg, files, None)
}
pub fn legacy_dfg(files: &BTreeMap<String, ParsedFile>) -> DataFlowGraph {
    from_legacy(LegacyDataFlowGraph::build(files))
}
pub fn from_legacy(old: LegacyDataFlowGraph) -> DataFlowGraph {
    DataFlowGraph {
        edges: old.edges,
        defs: old.defs,
        uses: old.uses,
        forward: old.forward,
        backward: old.backward,
        labels: old.labels,
        exact_labels: old.exact_labels,
        rd_function_stats: old.rd_function_stats,
    }
}
/// Refresh the frozen backend before adapting its completed state for current assembly.
pub fn legacy_refreshed_dfg(
    cached: DataFlowGraph,
    files: &BTreeMap<String, ParsedFile>,
) -> DataFlowGraph {
    let mut old = LegacyDataFlowGraph {
        edges: cached.edges,
        defs: cached.defs,
        uses: cached.uses,
        forward: cached.forward,
        backward: cached.backward,
        labels: cached.labels,
        exact_labels: cached.exact_labels,
        rd_function_stats: cached.rd_function_stats,
    };
    let all = files.keys().cloned().collect();
    old.remove_files(&all);
    old.merge(LegacyDataFlowGraph::build(files));
    from_legacy(old)
}
pub use super::dfg_observation::{fieldwise_payload, lookup_indexes, payload};
