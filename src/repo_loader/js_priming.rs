//! The JS/TS alias and export priming the loader runs before it hashes the
//! topology (moved out of `scope_graph_build_inputs_from_snapshot`, text unchanged).
//! Priming consults every production proof/refusal dependency, so each one,
//! absent paths included, is recorded in the snapshot's bookkeeping.
use crate::ast::ParsedFile;
use crate::resolution::js_ts::paths_snapshot::JsPathsSnapshot;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn prime_js_paths(
    files: &BTreeMap<String, ParsedFile>,
    js_paths_snapshot: &JsPathsSnapshot,
) {
    // Prime every production proof/refusal dependency, including absent paths.
    if !js_paths_snapshot.configs.is_empty() {
        let indexed = files.keys().cloned().collect();
        let mut resolver = crate::resolution::js_ts::paths::Resolver::new(&js_paths_snapshot);
        let mut pending = BTreeSet::new();
        for (file, parsed) in files {
            if !matches!(
                parsed.language,
                crate::languages::Language::JavaScript
                    | crate::languages::Language::TypeScript
                    | crate::languages::Language::Tsx
            ) {
                continue;
            }
            for binding in parsed.extract_import_bindings() {
                if let Some(target) = resolver.resolve(file, &binding.module_path, &indexed) {
                    pending.insert((
                        resolver.project(file).expect("resolved caller project"),
                        target,
                    ));
                }
            }
        }
        // Prime only the export closure consulted by admitted alias modules.
        // Visiting every file repeats expensive extraction for unrelated callers.
        // Extraction is a pure per-file function, memoized on the parsed
        // file; each frontier is extracted in parallel and then consulted in
        // sorted order, which reaches the same closure as a one-at-a-time walk.
        let mut visited = BTreeSet::new();
        while !pending.is_empty() {
            let frontier: Vec<(String, String)> = std::mem::take(&mut pending)
                .into_iter()
                .filter(|entry| visited.insert(entry.clone()))
                .collect();
            crate::build_pool::per_item(&frontier)
                .filter_map(|(_, file)| files.get(file))
                .for_each(|parsed| {
                    parsed.extract_js_ts_export_facts();
                });
            for (project, file) in frontier {
                let Some(parsed) = files.get(&file) else {
                    continue;
                };
                let exports = parsed.extract_js_ts_export_facts();
                for module in
                    exports
                        .star_reexports
                        .iter()
                        .chain(exports.named.values().filter_map(|target| match target {
                            crate::ast::js_ts::exports::JsExportTarget::ReExport {
                                module_path,
                                ..
                            }
                            | crate::ast::js_ts::exports::JsExportTarget::ImportForward {
                                module_path,
                                ..
                            } => Some(module_path),
                            _ => None,
                        }))
                {
                    if let Some(target) = resolver.hop(&project, &file, module, &indexed) {
                        pending.insert((project.clone(), target));
                    }
                }
            }
        }
    }
}
