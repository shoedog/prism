//! Resolve each graph import once, then project it to every requested file
//! covered by its scope. Both module-deps and repo-map use this collector.

use crate::name_resolution::consumer::{graph_module_dep_edge, GraphImport, ResolvedImport};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::EK_GLOB;
use crate::name_resolution::types::BindTarget;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct GraphImportDeps {
    pub files: BTreeMap<String, BTreeSet<String>>,
    pub external: BTreeSet<String>,
}

/// Callers supply only authoritative Rust files in the navigation session.
pub(super) fn collect_graph_import_deps<'a>(
    graph: &ScopeGraph,
    files: impl IntoIterator<Item = &'a str>,
) -> BTreeMap<String, GraphImportDeps> {
    collect_with(graph, files, |import| graph_module_dep_edge(graph, import))
}

fn collect_with<'a>(
    graph: &ScopeGraph,
    files: impl IntoIterator<Item = &'a str>,
    mut resolve: impl FnMut(GraphImport<'_>) -> ResolvedImport,
) -> BTreeMap<String, GraphImportDeps> {
    let mut paths_by_id: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for file in files {
        if let Some(&id) = graph.file_paths.get(file) {
            paths_by_id.entry(id).or_default().push(file);
        }
    }
    let files_by_scope: BTreeMap<_, BTreeSet<_>> = graph
        .scopes
        .iter()
        .map(|(&id, scope)| {
            let files = scope
                .extents
                .iter()
                .filter_map(|extent| paths_by_id.get(&extent.file))
                .flatten()
                .copied()
                .collect();
            (id, files)
        })
        .collect();
    let mut deps: BTreeMap<String, GraphImportDeps> = BTreeMap::new();
    let mut add = |scope, import, name: &str| {
        let Some(files) = files_by_scope.get(&scope).filter(|files| !files.is_empty()) else {
            return;
        };
        // Resolution depends on the import's scope, never the projected file.
        let resolved = resolve(import);
        for &file in files {
            match &resolved {
                ResolvedImport::File(target) if target != file => {
                    deps.entry(file.to_owned())
                        .or_default()
                        .files
                        .entry(target.clone())
                        .or_default()
                        .insert(name.to_owned());
                }
                ResolvedImport::External => {
                    deps.entry(file.to_owned())
                        .or_default()
                        .external
                        .insert(name.to_owned());
                }
                ResolvedImport::File(_) | ResolvedImport::Unresolved => {}
            }
        }
    };
    for binding in &graph.bindings {
        if matches!(binding.target, BindTarget::Pending(..)) {
            add(binding.scope, GraphImport::Named(binding), &binding.name);
        }
    }
    for edge in &graph.edges {
        if edge.kind == EK_GLOB {
            add(edge.from, GraphImport::Glob(edge), "*");
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use crate::name_resolution::rust_populator::{populate_rust, RustCrateConfig};
    use crate::name_resolution::types::FileId;

    #[test]
    fn imports_resolve_once_per_scope_not_once_per_extent_or_file() {
        let files = BTreeMap::from([(
            "lib.rs".into(),
            ParsedFile::parse(
                "lib.rs",
                "use missing::item; use missing::*;",
                Language::Rust,
            )
            .unwrap(),
        )]);
        let config = RustCrateConfig {
            crate_roots: vec!["lib.rs".into()],
            ..Default::default()
        };
        let mut graph = populate_rust(&files, &config, None);
        let first = graph.file_paths["lib.rs"];
        graph.file_paths.insert("other.rs".into(), FileId(42));
        let scope = graph
            .scopes
            .values_mut()
            .find(|s| s.extents.iter().any(|e| e.file == first))
            .unwrap();
        let mut extent = scope.extents[0].clone();
        extent.file = FileId(42);
        scope.extents.extend([extent.clone(), extent]);
        let mut count = 0;
        let deps = collect_with(&graph, ["lib.rs", "other.rs", "absent.rs"], |_| {
            count += 1;
            ResolvedImport::File("lib.rs".into())
        });
        assert_eq!(count, 3); // named import in both namespaces + glob
        assert!(!deps.contains_key("lib.rs")); // self-file suppression per projection
        assert_eq!(
            deps["other.rs"].files["lib.rs"],
            BTreeSet::from(["item".into(), "*".into()])
        );
        assert!(!deps.contains_key("absent.rs"));
        let mut count = 0;
        assert!(collect_with(&graph, ["absent.rs"], |_| {
            count += 1;
            ResolvedImport::External
        })
        .is_empty());
        assert_eq!(count, 0); // imports outside the requested files aren't resolved
    }
}
