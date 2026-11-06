//! Production compact storage; the wire decoder validates before serving any view.
use super::*;

/// The data flow graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DataFlowGraph {
    pub(super) storage: compact::CompactDfg,
}
impl DataFlowGraph {
    /// Create an empty graph.
    pub fn empty() -> Self {
        Self {
            storage: compact::combine(Vec::new()).expect("empty DFG"),
        }
    }
    /// Remove primary state at either endpoint, reclaim rows and rebuild adjacency.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        self.storage.remove_files(exclude);
    }
    /// Append primary state in order and rebuild adjacency.
    pub fn merge(&mut self, other: Self) {
        self.storage.merge(other.storage);
    }
}

impl DataFlowGraph {
    /// SCRATCH (measure/mem-census2): destructive per-field census.
    pub fn census_drop(mut self) {
        use crate::census::drop_measure;
        let s = &mut self.storage;
        drop_measure("dfg.edges", std::mem::take(&mut s.edges));
        drop_measure("dfg.defs", std::mem::take(&mut s.defs));
        drop_measure("dfg.uses", std::mem::take(&mut s.uses));
        drop_measure("dfg.forward", std::mem::take(&mut s.forward));
        drop_measure("dfg.backward", std::mem::take(&mut s.backward));
        drop_measure("dfg.labels", std::mem::take(&mut s.labels));
        drop_measure("dfg.exact_labels", std::mem::take(&mut s.exact_labels));
        drop_measure("dfg.rd_function_stats", std::mem::take(&mut s.rd_function_stats));
        drop_measure("dfg.rest(arena)", self);
    }
}
