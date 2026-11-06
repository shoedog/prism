//! Entry points around `assemble_graph_in`: the plain assembly and the
//! diff-scoped subset build (#639 S2 fold 2).
use super::*;

impl CodePropertyGraph {
    /// Assemble from a complete file set (every package fact is in `files`).
    pub(in crate::cpg) fn assemble_graph(
        cg: CallGraph,
        dfg: DataFlowGraph,
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
    ) -> Self {
        let r = Self::assemble_graph_in(cg, dfg, files, type_db, None);
        crate::census::mark("assemble_graph_returned");
        r
    }

    /// A CPG over a diff-scoped subset whose Go package facts (func-typed
    /// fields declared in out-of-scope siblings) come from the whole
    /// repository.
    pub(crate) fn build_scoped_subset(
        subset: &BTreeMap<String, ParsedFile>,
        all_files: &BTreeMap<String, ParsedFile>,
        type_db: Option<&TypeDatabase>,
    ) -> Self {
        let type_db = type_db.cloned();
        crate::build_pool::install(|| {
            let dfg = DataFlowGraph::build(subset);
            let cg = CallGraph::build_with_scope_graph_inputs(subset, None);
            Self::assemble_graph_in(cg, dfg, subset, type_db, Some(all_files))
        })
    }
}
