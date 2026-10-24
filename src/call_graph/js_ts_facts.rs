//! File-local JS/TS facts, computed concurrently and merged in caller order.
use super::{FunctionId, JsTsResolutionFacts};
use crate::ast::ParsedFile;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "js_ts_facts_tests.rs"]
mod tests;

pub(super) fn extract<'a, I>(files: I) -> JsTsResolutionFacts
where
    I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
{
    #[cfg(test)]
    if tests::sequential_enabled() {
        return sequential(files);
    }
    collect(files, extract_file)
}

// Production entry: public constructors can run on an ordinary thread, so the
// recursive per-file walks go onto prism's large-stack pool.
fn collect<'a, I, F>(files: I, work: F) -> JsTsResolutionFacts
where
    I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
    F: Fn(&str, &ParsedFile) -> JsTsResolutionFacts + Sync,
{
    let files: Vec<_> = files.into_iter().collect();
    let work = &work;
    crate::build_pool::install(move || collect_on_current_pool(files, work))
}

fn collect_on_current_pool<'a, I, F>(files: I, work: F) -> JsTsResolutionFacts
where
    I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
    F: Fn(&str, &ParsedFile) -> JsTsResolutionFacts + Sync,
{
    // Freeze the caller's order before dispatch. Indexed parallel collection
    // retains these positions even when later files finish first.
    let files: Vec<_> = files.into_iter().collect();
    let per_file: Vec<_> = files
        .par_iter()
        .map(|(path, parsed)| work(path, parsed))
        .collect();
    let (mut exports, mut locals, mut returns) = JsTsResolutionFacts::default();
    for (file_exports, file_locals, file_returns) in per_file {
        // Later admitted inserts win; an absent per-file entry erases nothing.
        exports.extend(file_exports);
        locals.extend(file_locals);
        returns.extend(file_returns);
    }
    (exports, locals, returns)
}

fn extract_file(file_path: &str, parsed: &ParsedFile) -> JsTsResolutionFacts {
    let mut export_facts = BTreeMap::new();
    let mut function_locals: BTreeMap<FunctionId, BTreeSet<String>> = BTreeMap::new();
    let mut return_facts = BTreeMap::new();
    if !matches!(
        parsed.language,
        crate::languages::Language::JavaScript
            | crate::languages::Language::TypeScript
            | crate::languages::Language::Tsx
    ) {
        return (export_facts, function_locals, return_facts);
    }
    let exports = parsed.extract_js_ts_export_facts();
    // Keep proven empty modules distinct from missing/opaque error branches.
    if !exports.is_empty() || !parsed.tree.root_node().has_error() {
        export_facts.insert(file_path.to_string(), exports);
    }
    let mut binding_cache = crate::ast::JsBindingCache::for_sites();
    for func_node in crate::anon_owner::owner_nodes(parsed) {
        let Some(func_id) = crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
        else {
            continue;
        };
        let locals = crate::anon_owner::shadow_locals(parsed, &func_node);
        if !locals.is_empty() {
            function_locals.insert(func_id.clone(), locals);
        }
        if let Some(facts) = parsed.js_ts_return_facts(func_node, &mut binding_cache) {
            return_facts.insert(func_id, facts);
        }
    }
    (export_facts, function_locals, return_facts)
}

// Frozen pre-parallel implementation: do not route this through extract_file.
#[cfg(test)]
fn sequential<'a, I>(files: I) -> JsTsResolutionFacts
where
    I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
{
    ParsedFile::with_legacy_export_facts(|| sequential_legacy(files))
}

#[cfg(test)]
fn sequential_legacy<'a, I>(files: I) -> JsTsResolutionFacts
where
    I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
{
    tests::record_oracle();
    let mut export_facts: BTreeMap<String, crate::js_exports::JsExportFacts> = BTreeMap::new();
    let mut function_locals: BTreeMap<FunctionId, BTreeSet<String>> = BTreeMap::new();
    let mut return_facts: BTreeMap<FunctionId, crate::js_returns::JsReturnFacts> = BTreeMap::new();

    for (file_path, parsed) in files {
        if !matches!(
            parsed.language,
            crate::languages::Language::JavaScript
                | crate::languages::Language::TypeScript
                | crate::languages::Language::Tsx
        ) {
            continue;
        }

        let exports = parsed.extract_js_ts_export_facts();
        // A successfully parsed value-empty module is a proven empty star
        // branch, distinct from a missing/opaque branch in alias provenance.
        if !exports.is_empty() || !parsed.tree.root_node().has_error() {
            export_facts.insert(file_path.clone(), exports);
        }

        let mut binding_cache = crate::ast::JsBindingCache::for_sites();
        for func_node in crate::anon_owner::owner_nodes(parsed) {
            let Some(func_id) = crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
            else {
                continue;
            };
            let locals = crate::anon_owner::shadow_locals(parsed, &func_node);
            if !locals.is_empty() {
                function_locals.insert(func_id.clone(), locals);
            }
            if let Some(facts) = parsed.js_ts_return_facts(func_node, &mut binding_cache) {
                return_facts.insert(func_id, facts);
            }
        }
    }

    (export_facts, function_locals, return_facts)
}
