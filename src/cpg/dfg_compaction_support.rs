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
pub use super::dfg_observation::{fieldwise_payload, lookup_indexes, payload};
pub use crate::data_flow::test_support::{from_legacy, legacy_refreshed_dfg};
