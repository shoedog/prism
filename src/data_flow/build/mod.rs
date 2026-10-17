//! Shared unchanged per-file producer and production ordered aggregation.
use super::{
    AccessPath, DataFlowGraph, ExactFlowEdge, FlowConfidence, FlowEdge, ParsedFile, RdFileStats,
    VarLocation,
};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod aliases;
mod exact_reads;
pub(super) mod file;

pub(super) struct Producer;
pub(super) struct FileRefs {
    pub(super) file_path: String,
    pub(super) defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    pub(super) uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    pub(super) edges: Vec<FlowEdge>,
    pub(super) labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    pub(super) exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    pub(super) rd_function_stats: RdFileStats,
}

impl Producer {
    fn insert_label(
        labels: &mut BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
        key: (VarLocation, VarLocation),
        label: FlowConfidence,
    ) {
        labels
            .entry(key)
            .and_modify(|stored| *stored = stored.worst(label))
            .or_insert(label);
    }
}

impl DataFlowGraph {
    /// Build a data flow graph from only the specified files.
    ///
    /// Identical to `build()` but skips files not in `only_files`. Used by
    /// incremental cache update to process only changed files.
    pub fn build_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        // Filter to only the requested files and delegate to build.
        let filtered: BTreeMap<String, &ParsedFile> = files
            .iter()
            .filter(|(k, _)| only_files.contains(*k))
            .map(|(k, v)| (k.clone(), v))
            .collect();
        Self::build_from_refs(&filtered)
    }

    /// Build a data flow graph from parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let refs: BTreeMap<String, &ParsedFile> =
            files.iter().map(|(k, v)| (k.clone(), v)).collect();
        Self::build_from_refs(&refs)
    }

    /// Build a DFG from file references (shared implementation for build and build_subset).
    fn build_from_refs(files: &BTreeMap<String, &ParsedFile>) -> Self {
        let ordered_files: Vec<(&String, &ParsedFile)> =
            files.iter().map(|(path, parsed)| (path, *parsed)).collect();
        let per_file_refs: Vec<super::compact::CompactDfg> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                Producer::compact_file_refs(file_path, parsed)
            })
            .collect();

        DataFlowGraph {
            storage: super::compact::combine(per_file_refs)
                .expect("DFG exceeds u32 location capacity"),
        }
    }
}

impl FileRefs {
    fn into_parts(self) -> super::compact::Parts {
        debug_assert!(self
            .defs
            .keys()
            .chain(self.uses.keys())
            .all(|key| key.0 == self.file_path));
        debug_assert!(self
            .labels
            .keys()
            .all(|(f, t)| f.file == self.file_path && t.file == self.file_path));
        debug_assert!(self
            .exact_labels
            .keys()
            .all(|e| e.from.file == self.file_path && e.to.file == self.file_path));
        super::compact::Parts {
            edges: self.edges,
            defs: self.defs,
            uses: self.uses,
            labels: self.labels,
            exact_labels: self.exact_labels,
            rd_function_stats: BTreeMap::from([(self.file_path, self.rd_function_stats)]),
            adjacency: None,
        }
    }
}
