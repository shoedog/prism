//! Shared per-file producer and production ordered aggregation.
use super::{
    AccessPath, DataFlowGraph, ExactFlowEdge, FlowConfidence, FlowEdge, ParsedFile, RdFileStats,
    VarLocation,
};
use crate::call_graph::rust::macro_args::RustMacroCensus;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod alias_occurrence;
mod aliases;
#[cfg(test)]
mod arrow_body_cap_tests;
#[cfg(test)]
mod census_cap_tests;
mod exact_reads;
mod fallback_cap;
#[cfg(test)]
mod fallback_cap_tests;
#[cfg(test)]
mod field_recall_fold1_tests;
#[cfg(test)]
mod field_recall_fold2_tests;
#[cfg(test)]
mod field_recall_limits_tests;
#[cfg(test)]
mod field_recall_s2_tests;
#[cfg(test)]
mod field_recall_s2f1_tests;
#[cfg(test)]
mod field_recall_s2f2_cpg_tests;
#[cfg(test)]
mod field_recall_s2f2_tests;
#[cfg(test)]
mod field_recall_s2f3_tests;
#[cfg(test)]
mod field_recall_s2f4_tests;
#[cfg(test)]
mod field_recall_tests;
pub(super) mod file;
#[cfg(test)]
mod giant_owner_tests;
#[cfg(test)]
mod go_blank_tests;
#[cfg(test)]
mod go_literal_defs_tests;
#[cfg(test)]
mod go_literal_fold1_tests;
#[cfg(test)]
mod go_literal_fold2_tests;
#[cfg(test)]
mod go_literal_fold3_tests;
#[cfg(test)]
mod go_nodef_defs_tests;
mod js_prune;
#[cfg(test)]
mod js_prune_tests;
#[cfg(test)]
mod legacy_producer;
mod lexical_prune;
#[cfg(test)]
mod lexical_prune_round2_tests;
#[cfg(test)]
mod lexical_prune_tests;
mod order_prune;
#[cfg(test)]
mod order_prune_tests;
mod owner_labels;
mod param_alias_trace;
#[cfg(test)]
mod prune_cap_tests;
mod python_prune;
#[cfg(test)]
mod python_prune_tests;
mod rust_prune;
#[cfg(test)]
mod rust_prune_fold3_tests;
#[cfg(test)]
mod rust_prune_inline_cases;
#[cfg(test)]
mod rust_prune_tests;
#[cfg(test)]
mod same_line_fold2_tests;
mod scope_params;
mod scope_prune;
#[cfg(test)]
mod scope_prune_tests;
mod scope_split;
#[cfg(test)]
mod split_fold2_tests;

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
    /// incremental cache update to process only changed files. The Rust macro census is
    /// computed from the whole `files` map, so a subset build and a cold build agree.
    pub fn build_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        crate::build_pool::install(|| {
            let census = RustMacroCensus::from_files(files);
            Self::build_subset_with_census(files, only_files, &census)
        })
    }

    /// `build_subset` under a census the caller already computed from the whole `files` map.
    pub(crate) fn build_subset_with_census(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
        census: &RustMacroCensus,
    ) -> Self {
        // Filter to only the requested files and delegate to build.
        let filtered: BTreeMap<String, &ParsedFile> = files
            .iter()
            .filter(|(k, _)| only_files.contains(*k))
            .map(|(k, v)| (k.clone(), v))
            .collect();
        crate::build_pool::install(|| Self::build_from_refs_on_pool(&filtered, census))
    }

    /// Build a data flow graph from parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        Self::build_with_census_files(files, files)
    }

    /// `build` over `files` with the Rust macro census read from `census_files` (the whole
    /// repository when `files` is a diff-scoped subset), so a scoped build refuses exactly
    /// what a full build does.
    pub fn build_with_census_files(
        files: &BTreeMap<String, ParsedFile>,
        census_files: &BTreeMap<String, ParsedFile>,
    ) -> Self {
        let refs: BTreeMap<String, &ParsedFile> =
            files.iter().map(|(k, v)| (k.clone(), v)).collect();
        // The per-file extraction walks syntax recursively; enter the large-stack
        // build pool once (inline when already on it) whatever thread calls.
        crate::build_pool::install(|| {
            let census = RustMacroCensus::from_files(census_files);
            Self::build_from_refs_on_pool(&refs, &census)
        })
    }

    /// `build` without the pool boundary, for tests that observe per-worker
    /// thread-local counters on a pool they own: `build` hands the work to the
    /// build pool, where those counters would not be seen.
    #[cfg(test)]
    pub(crate) fn build_on_current_pool(files: &BTreeMap<String, ParsedFile>) -> Self {
        let refs: BTreeMap<String, &ParsedFile> =
            files.iter().map(|(k, v)| (k.clone(), v)).collect();
        Self::build_from_refs_on_pool(&refs, &RustMacroCensus::from_files(files))
    }

    fn build_from_refs_on_pool(
        files: &BTreeMap<String, &ParsedFile>,
        census: &RustMacroCensus,
    ) -> Self {
        let _profile = crate::profile::Phase::serial("dfg");
        let ordered_files: Vec<(&String, &ParsedFile)> =
            files.iter().map(|(path, parsed)| (path, *parsed)).collect();
        let per_file_refs: Vec<super::compact::CompactDfg> =
            crate::build_pool::per_item(&ordered_files)
                .map(|entry| {
                    let (file_path, parsed) = *entry;
                    Producer::compact_file_refs(file_path, parsed, census)
                })
                .collect();

        let mut storage =
            super::compact::combine(per_file_refs).expect("DFG exceeds u32 location capacity");
        storage.rust_macro_census_digest = census.digest();
        DataFlowGraph { storage }
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
