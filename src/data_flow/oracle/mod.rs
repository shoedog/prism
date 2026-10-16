//! Frozen legacy contract from rebound execution parent cc20841b.
//! Shares only DTOs, the confidence lattice and per-file producer.
use super::{
    AccessPath, ExactFlowEdge, FlowConfidence, FlowEdge, FlowPath, ParsedFile, RdFileStats,
    VarAccessKind, VarLocation,
};
use crate::finding_confidence::EvidenceHop;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
mod aggregate;
mod queries;
mod update;

/// The data flow graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LegacyDataFlowGraph {
    /// All def-use edges.
    pub edges: Vec<FlowEdge>,
    /// Index: (file, function, function_start_line, access_path) → definitions
    pub defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Index: (file, function, function_start_line, access_path) → uses
    pub uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Forward adjacency: def location → use locations it reaches
    pub forward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// Backward adjacency: use location → def locations it comes from
    pub backward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// One label per unique `(from, to)` FlowEdge endpoint pair. Primary state
    /// with the same file-partitioned lifecycle as `edges`.
    pub labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    /// Supplemental byte-distinct producer facts absent from the legacy exact
    /// endpoint population. Confidence is classified before legacy key reduction.
    pub(crate) exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    /// Per-file reaching-definitions availability counters.
    pub rd_function_stats: BTreeMap<String, RdFileStats>,
}

impl LegacyDataFlowGraph {
    /// Create an empty data flow graph with no edges.
    pub fn empty() -> Self {
        LegacyDataFlowGraph {
            edges: Vec::new(),
            defs: BTreeMap::new(),
            uses: BTreeMap::new(),
            forward: BTreeMap::new(),
            backward: BTreeMap::new(),
            labels: BTreeMap::new(),
            exact_labels: BTreeMap::new(),
            rd_function_stats: BTreeMap::new(),
        }
    }
}
