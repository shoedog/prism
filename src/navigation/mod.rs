pub mod cache;
pub mod call_resolve;
pub mod code_context;
pub mod inventory;
pub mod module_graph;
pub mod queries;
pub mod seed;
pub mod types;

use crate::cpg::{CodePropertyGraph, CpgContext, CpgNode};
use crate::repo_loader::LoadedRepo;
use crate::type_provider::TypeRegistry;
use petgraph::graph::NodeIndex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub struct NavigationIndex {
    pub cpg: CodePropertyGraph,
    pub types: TypeRegistry,
    pub live_types: BTreeSet<String>,
    // nav-local, derived from CpgNode::Function (spec section 3):
    pub line_range_index: BTreeMap<String, Vec<(usize, usize, NodeIndex)>>,
    pub name_index: BTreeMap<(String, String), Vec<NodeIndex>>,
}

pub struct NavigationSession {
    pub repo: Arc<LoadedRepo>,
    pub index: Arc<NavigationIndex>,
}

impl NavigationIndex {
    /// Build a whole-repo index. Uses `CpgContext::build` (scope == None), never
    /// `build_scoped` (spec section 17 Step 3 / R3-M5). Moves the owned cpg/types/live_types
    /// out of the borrowing context so the index owns them.
    pub fn build(repo: &LoadedRepo) -> Self {
        let ctx = CpgContext::build_with_scope_graph_inputs(
            &repo.files,
            repo.type_db.as_ref(),
            repo.scope_graph_inputs.as_ref(),
        );
        Self::from_ctx(ctx)
    }

    /// Build a whole-repo index by incrementally refreshing the CPG from a
    /// previously published whole-repo navigation index.
    ///
    /// This is intentionally separate from `cache::build_cached_at`, whose
    /// partial-hit behavior remains conservative for normal nav cache users.
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    pub(crate) fn build_incremental_from_previous(
        previous: &NavigationIndex,
        repo: &LoadedRepo,
        changed_files: &BTreeSet<String>,
    ) -> Self {
        let cpg = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
            previous.cpg.call_graph.clone(),
            previous.cpg.dfg.clone(),
            changed_files,
            &repo.files,
            repo.type_db.clone(),
            repo.scope_graph_inputs.as_ref(),
        );
        Self::from_ctx(CpgContext::build_with_cached_cpg(
            &repo.files,
            cpg,
            repo.type_db.as_ref(),
        ))
    }

    pub(crate) fn from_ctx(ctx: CpgContext<'_>) -> Self {
        debug_assert!(ctx.scope.is_none(), "nav index must be whole-repo");
        let (mut line_range_index, mut name_index) = (
            BTreeMap::<String, Vec<(usize, usize, NodeIndex)>>::new(),
            BTreeMap::<(String, String), Vec<NodeIndex>>::new(),
        );
        for idx in ctx.cpg.node_indices() {
            if let CpgNode::Function {
                name,
                file,
                start_line,
                end_line,
                ..
            } = ctx.cpg.node(idx)
            {
                line_range_index.entry(file.clone()).or_default().push((
                    *start_line,
                    *end_line,
                    idx,
                ));
                name_index
                    .entry((file.clone(), name.clone()))
                    .or_default()
                    .push(idx);
            }
        }
        for v in line_range_index.values_mut() {
            v.sort_by_key(|&(s, _, _)| s);
        }
        NavigationIndex {
            cpg: ctx.cpg,
            types: ctx.types,
            live_types: ctx.live_types,
            line_range_index,
            name_index,
        }
    }

    /// Innermost enclosing function (smallest [start,end] containing `line`).
    pub fn enclosing_function(&self, file: &str, line: usize) -> Option<(NodeIndex, String)> {
        let ranges = self.line_range_index.get(file)?;
        ranges
            .iter()
            .filter(|&&(s, e, _)| s <= line && line <= e)
            .min_by_key(|&&(s, e, _)| e - s)
            .map(|&(_, _, idx)| {
                let name = match self.cpg.node(idx) {
                    CpgNode::Function { name, .. } => name.clone(),
                    _ => String::new(),
                };
                (idx, name)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::queries;
    use crate::navigation::types::{Reason, SymbolRef};
    use crate::repo_loader::load_repo;
    use std::path::Path;

    fn write_files(root: &Path, files: &[(&str, &str)]) {
        for (name, source) in files {
            let path = root.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, source).unwrap();
        }
    }

    fn full_session(root: &Path) -> NavigationSession {
        let repo = Arc::new(load_repo(root).unwrap());
        let index = Arc::new(NavigationIndex::build(&repo));
        NavigationSession { repo, index }
    }

    fn incremental_session(
        previous: &NavigationSession,
        root: &Path,
        changed_files: &[&str],
    ) -> NavigationSession {
        let repo = Arc::new(load_repo(root).unwrap());
        let changed_files = changed_files.iter().map(|f| (*f).to_string()).collect();
        let index = Arc::new(NavigationIndex::build_incremental_from_previous(
            previous.index.as_ref(),
            &repo,
            &changed_files,
        ));
        NavigationSession { repo, index }
    }

    fn function_names_for_callers(
        session: &NavigationSession,
        target: &str,
        file: Option<&str>,
        depth: usize,
    ) -> Vec<String> {
        let evidence = queries::callers(session, Some(target), file, None, depth).unwrap();
        evidence
            .items
            .into_iter()
            .filter_map(|item| match item.symbol {
                Some(SymbolRef::Function { name, .. }) => Some(name),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn incremental_from_previous_chained_edits_match_full_queries() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                ("api.py", "def target():\n    return 1\n"),
                (
                    "callers.py",
                    "from api import target\n\ndef first():\n    return target()\n\ndef second():\n    return 0\n",
                ),
                (
                    "outer.py",
                    "from callers import second\n\ndef third():\n    return 0\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "callers.py",
                "from api import target\n\ndef first():\n    return target()\n\ndef second():\n    return target()\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["callers.py"]);

        write_files(
            dir.path(),
            &[(
                "outer.py",
                "from callers import second\n\ndef third():\n    return second()\n",
            )],
        );
        let v3_incremental = incremental_session(&v2_incremental, dir.path(), &["outer.py"]);
        let v3_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v3_full, Some("target"), Some("api.py"), None, 2).unwrap(),
            queries::callers(&v3_incremental, Some("target"), Some("api.py"), None, 2).unwrap()
        );
        assert_eq!(
            queries::callees(&v3_full, Some("third"), Some("outer.py"), None, 2).unwrap(),
            queries::callees(&v3_incremental, Some("third"), Some("outer.py"), None, 2).unwrap()
        );

        let caller_names = function_names_for_callers(&v3_incremental, "target", Some("api.py"), 2);
        assert!(caller_names.contains(&"first".to_string()));
        assert!(caller_names.contains(&"second".to_string()));
        assert!(caller_names.contains(&"third".to_string()));
    }

    #[test]
    fn incremental_from_previous_recomputes_indirect_edges_for_unchanged_caller() {
        let dir = tempfile::tempdir().unwrap();
        let device_src =
            "struct Device { void (*callback)(); };\nvoid run(struct Device *d) { d->callback(); }\n";
        write_files(
            dir.path(),
            &[
                ("device.c", device_src),
                (
                    "setup.c",
                    "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = old_handler; }\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "setup.c",
                "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = new_handler; }\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["setup.c"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("new_handler"), Some("setup.c"), None, 1).unwrap(),
            queries::callers(
                &v2_incremental,
                Some("new_handler"),
                Some("setup.c"),
                None,
                1
            )
            .unwrap()
        );
        assert_eq!(
            queries::callers(&v2_full, Some("old_handler"), Some("setup.c"), None, 1).unwrap(),
            queries::callers(
                &v2_incremental,
                Some("old_handler"),
                Some("setup.c"),
                None,
                1
            )
            .unwrap()
        );

        let new_handler_callers = queries::callers(
            &v2_incremental,
            Some("new_handler"),
            Some("setup.c"),
            None,
            1,
        )
        .unwrap();
        assert!(
            new_handler_callers.items.iter().any(|item| matches!(
                &item.symbol,
                Some(SymbolRef::Function { file, name, .. }) if file == "device.c" && name == "run"
            )),
            "incremental nav index should expose the recomputed indirect caller"
        );
        assert!(
            new_handler_callers
                .items
                .iter()
                .any(|item| item.why.iter().any(|reason| {
                    matches!(
                        reason,
                        Reason::CalledBy {
                            caller,
                            call_site_line
                        } if caller == "run" && *call_site_line == 2
                    )
                })),
            "caller evidence should point at the unchanged callback call site"
        );

        let old_handler_callers =
            function_names_for_callers(&v2_incremental, "old_handler", Some("setup.c"), 1);
        assert!(!old_handler_callers.contains(&"run".to_string()));
    }
}
