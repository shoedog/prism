pub mod cache;
pub mod call_resolve;
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
        let ctx = CpgContext::build(&repo.files, repo.type_db.as_ref());
        Self::from_ctx(ctx)
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
