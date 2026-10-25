//! Public `CallGraph` constructors: one large-stack execution boundary.
//!
//! Every constructor enters the shared build pool once through
//! `build_pool::install` and runs its whole body there, so the per-file
//! `par_iter` sites and the recursive AST walks inside it get large-stack
//! workers whichever thread the embedder calls from. `install` is inline on a
//! build-pool worker and hands off safely from a foreign rayon worker (issue
//! #513). The bodies are the private `*_inner` functions in `call_graph.rs`.
use super::{CallGraph, ParsedFile, ScopeGraphBuildInputs};
use crate::resolution::ReceiverRecoveryConfig;
use std::collections::{BTreeMap, BTreeSet};

impl CallGraph {
    /// Build a lightweight call graph with only direct calls (Phases 1-2).
    ///
    /// Skips Phase 3 (indirect call resolution). Used to scope a dependency
    /// neighborhood before committing to a full CPG build on the subset.
    pub fn build_skeleton(files: &BTreeMap<String, ParsedFile>) -> Self {
        crate::build_pool::install(|| Self::build_skeleton_inner(files))
    }

    /// Build a call graph from all parsed files (default receiver-recovery config).
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        Self::build_with_receiver_config(files, &ReceiverRecoveryConfig::default())
    }

    /// Build a call graph with an explicit receiver-recovery config (spec §2 seam).
    /// The classifier is built once and shared across the rayon extraction loop.
    pub fn build_with_receiver_config(
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &ReceiverRecoveryConfig,
    ) -> Self {
        crate::build_pool::install(|| {
            let inputs = ScopeGraphBuildInputs::from_files_convention(files);
            Self::build_inner(files, receiver_config, Some(&inputs))
        })
    }

    pub fn build_with_scope_graph_inputs(
        files: &BTreeMap<String, ParsedFile>,
        inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        Self::build_with_receiver_config_and_scope_graph_inputs(
            files,
            &ReceiverRecoveryConfig::default(),
            inputs,
        )
    }

    pub fn build_with_receiver_config_and_scope_graph_inputs(
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &ReceiverRecoveryConfig,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        crate::build_pool::install(|| Self::build_inner(files, receiver_config, scope_inputs))
    }

    /// Build direct calls from only the specified files (Phases 1-2).
    /// The caller recomputes indirect calls after merging the subset.
    pub fn build_direct_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        Self::build_direct_subset_with_receiver_config(
            files,
            only_files,
            &ReceiverRecoveryConfig::default(),
        )
    }

    /// `build_direct_subset` with an explicit receiver-recovery config (spec §2 seam).
    pub fn build_direct_subset_with_receiver_config(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
        receiver_config: &ReceiverRecoveryConfig,
    ) -> Self {
        crate::build_pool::install(|| {
            Self::build_direct_subset_inner(files, only_files, receiver_config)
        })
    }
}
