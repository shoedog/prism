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
