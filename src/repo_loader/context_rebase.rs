//! Scope inputs for "the original files plus admitted context files", built only from
//! what the review captured: no directory walk, no manifest or configuration re-read, no
//! change to the original review's bookkeeping. Which files the union holds is the
//! caller's business (the targeted reader is the admission authority).
use super::js_priming::prime_js_paths;
use crate::ast::ParsedFile;
use crate::call_graph::ScopeGraphBuildInputs;
use std::collections::BTreeMap;

/// The captured root, manifests, skipped-testdata count and Rust configuration are kept
/// (no Rust file is ever added, so `cfg` and `complete` are not recomputed). The JS
/// snapshot is a detached copy primed over the union, and its topology entries replace the
/// captured ones; every other `manifest_hashes` entry, `c-census:*` included, stays as captured.
#[cfg_attr(not(test), allow(dead_code))] // S4b1: the round driver is the first caller.
pub(crate) fn rebase_scope_inputs(
    captured: &ScopeGraphBuildInputs,
    union_files: &BTreeMap<String, ParsedFile>,
) -> ScopeGraphBuildInputs {
    let js_paths_snapshot = captured.js_paths_snapshot.detached();
    prime_js_paths(union_files, &js_paths_snapshot);
    let mut manifest_hashes = captured.manifest_hashes.clone();
    manifest_hashes.extend(js_paths_snapshot.topology());
    ScopeGraphBuildInputs {
        repo_root: captured.repo_root.clone(),
        all_file_paths: union_files.keys().cloned().collect(),
        manifest_hashes,
        manifest_snapshot: captured.manifest_snapshot.clone(),
        skipped_go_testdata_files: captured.skipped_go_testdata_files,
        js_paths_snapshot,
        cfg: captured.cfg.clone(),
        complete: captured.complete,
    }
}
