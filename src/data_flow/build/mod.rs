//! Shared per-file producer and production ordered aggregation.
use super::{
    AccessPath, DataFlowGraph, ExactFlowEdge, FlowConfidence, FlowEdge, ParsedFile, RdFileStats,
    VarLocation,
};
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod alias_occurrence;
mod aliases;
#[cfg(test)]
mod arrow_body_cap_tests;
mod c_file_parens;
#[cfg(test)]
mod c_file_parens_tests;
#[cfg(test)]
mod c_macro_census_tests;
mod c_phantom;
mod c_prune;
#[cfg(test)]
mod c_prune_brackets_r1_tests;
#[cfg(test)]
mod c_prune_brackets_r2_loader_tests;
#[cfg(test)]
mod c_prune_brackets_r2_tests;
#[cfg(test)]
mod c_prune_brackets_r3_depth_tests;
#[cfg(test)]
mod c_prune_brackets_r3_enclosing_tests;
#[cfg(test)]
mod c_prune_brackets_r3_lexical_tests;
#[cfg(test)]
mod c_prune_brackets_r3_loader_tests;
#[cfg(test)]
mod c_prune_brackets_r3_support;
#[cfg(test)]
mod c_prune_brackets_tests;
#[cfg(test)]
mod c_prune_brackets_v2_gate_tests;
#[cfg(test)]
mod c_prune_brackets_v2_tests;
#[cfg(test)]
mod c_prune_fold2_tests;
#[cfg(test)]
mod c_prune_fold7_tests;
#[cfg(test)]
mod c_prune_generated_rows;
#[cfg(test)]
mod c_prune_inline_cases;
#[cfg(test)]
mod c_prune_oracle_rows;
#[cfg(test)]
mod c_prune_perf_tests;
#[cfg(test)]
mod c_prune_raw_splice_tests;
mod c_prune_resolve;
#[cfg(test)]
mod c_prune_tests;
mod c_prune_write;
#[cfg(test)]
mod census_cap_tests;
mod cpp_prune;
#[cfg(test)]
mod cpp_prune_inline_cases;
mod cpp_prune_names;
mod cpp_prune_operand;
#[cfg(test)]
mod cpp_prune_tests;
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
#[cfg(test)]
mod loop_refusal_cap_tests;
mod order_prune;
#[cfg(test)]
mod order_prune_tests;
mod owner_labels;
mod param_alias_trace;
mod pattern_caps;
mod pattern_execution;
mod pattern_formals;
mod pattern_reads;
pub(crate) use pattern_formals::is_signature_pair;
#[cfg(test)]
mod pattern_formals_tests;
mod pattern_values;
#[cfg(test)]
mod prune_cap_tests;
mod prune_roots;
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
    /// Run the file producer uncached and collect the S1 attempt ledger in file order.
    pub fn collect_s1_evidence(
        files: &BTreeMap<String, ParsedFile>,
        source_revision: &str,
    ) -> super::s1_evidence::Evidence {
        crate::build_pool::install(|| {
            let census = RustMacroCensus::from_files(files);
            let c_census = CMacroCensus::from_files(files);
            let files = files
                .par_iter()
                .map(|(path, parsed)| {
                    let evidence = if parsed.language.is_js_family() {
                        Producer::file_s1_evidence(
                            path,
                            parsed,
                            &census,
                            &c_census,
                            fallback_cap::FALLBACK_EDGE_CAP,
                        )
                    } else {
                        super::s1_evidence::FileEvidence::new(parsed)
                    };
                    (path.clone(), evidence)
                })
                .collect();
            super::s1_evidence::Evidence {
                source_revision: source_revision.to_owned(),
                files,
            }
        })
    }

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
            let c_census = CMacroCensus::from_files(files);
            Self::build_subset_with_census(files, only_files, &census, &c_census)
        })
    }

    /// `build_subset` under a census the caller already computed from the whole `files` map.
    pub(crate) fn build_subset_with_census(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
        census: &RustMacroCensus,
        c_census: &CMacroCensus,
    ) -> Self {
        // Filter to only the requested files and delegate to build.
        let filtered: BTreeMap<String, &ParsedFile> = files
            .iter()
            .filter(|(k, _)| only_files.contains(*k))
            .map(|(k, v)| (k.clone(), v))
            .collect();
        crate::build_pool::install(|| Self::build_from_refs_on_pool(&filtered, census, c_census))
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
            let c_census = CMacroCensus::from_files(census_files);
            Self::build_from_refs_on_pool(&refs, &census, &c_census)
        })
    }

    /// One `build_with_census_files(&{file}, census_files)` result per file of `files`, in path
    /// order, with the Rust and C/C++ macro censuses read from `census_files` ONCE. Calling
    /// `build_with_census_files` in a loop recomputes both censuses over the whole repository for
    /// every file (quadratic in the file count); the graphs are the same.
    pub fn build_each_file_with_census_files(
        files: &BTreeMap<String, ParsedFile>,
        census_files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<(String, Self)> {
        crate::build_pool::install(|| {
            let census = RustMacroCensus::from_files(census_files);
            let c_census = CMacroCensus::from_files(census_files);
            files
                .iter()
                .map(|(path, parsed)| {
                    let one = BTreeMap::from([(path.clone(), parsed)]);
                    (
                        path.clone(),
                        Self::build_from_refs_on_pool(&one, &census, &c_census),
                    )
                })
                .collect()
        })
    }

    /// `build` without the pool boundary, for tests that observe per-worker
    /// thread-local counters on a pool they own: `build` hands the work to the
    /// build pool, where those counters would not be seen.
    #[cfg(test)]
    pub(crate) fn build_on_current_pool(files: &BTreeMap<String, ParsedFile>) -> Self {
        let refs: BTreeMap<String, &ParsedFile> =
            files.iter().map(|(k, v)| (k.clone(), v)).collect();
        Self::build_from_refs_on_pool(
            &refs,
            &RustMacroCensus::from_files(files),
            &CMacroCensus::from_files(files),
        )
    }

    fn build_from_refs_on_pool(
        files: &BTreeMap<String, &ParsedFile>,
        census: &RustMacroCensus,
        c_census: &CMacroCensus,
    ) -> Self {
        let _profile = crate::profile::Phase::serial("dfg");
        let ordered_files: Vec<(&String, &ParsedFile)> =
            files.iter().map(|(path, parsed)| (path, *parsed)).collect();
        let per_file_refs: Vec<super::compact::CompactDfg> =
            crate::build_pool::per_item(&ordered_files)
                .map(|entry| {
                    let (file_path, parsed) = *entry;
                    Producer::compact_file_refs(file_path, parsed, census, c_census)
                })
                .collect();

        let mut storage =
            super::compact::combine(per_file_refs).expect("DFG exceeds u32 location capacity");
        storage.rust_macro_census_digest = census.digest();
        storage.c_macro_census_digest = c_census.digest();
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
