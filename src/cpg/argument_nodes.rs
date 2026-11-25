use super::{CodePropertyGraph, CpgEdge, CpgNode, ExactVarIndex, VarIndex};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use petgraph::graph::{DiGraph, NodeIndex};

impl CodePropertyGraph {
    pub(super) fn argument_nodes(
        caller_id: &FunctionId,
        caller_parsed: &ParsedFile,
        arg_text: &str,
        arg_span: &std::ops::Range<usize>,
        var_index: &VarIndex,
        exact_var_index: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Vec<NodeIndex> {
        // Field-sensitive arg binding (from PR #113): prefer the full access path
        // (e.g. `o.data`) so interproc taint flows from the specific field, falling
        // back to the base (`o`) — the pre-change behavior — when no field-path var
        // node exists (recall-preserving).
        let full_arg_path = caller_parsed
            .bounded_argument_member_path_at(arg_span.start, arg_span.end)
            .unwrap_or_else(|| AccessPath::from_expr(arg_text));
        let mut arg_paths = vec![full_arg_path.clone()];
        let base_arg_path = AccessPath::simple(&full_arg_path.base);
        if base_arg_path != full_arg_path {
            arg_paths.push(base_arg_path);
        }
        // Supplement, not replace: emit an edge from EACH resolved arg node (the
        // field path AND the base) so both field-rooted and whole-object taint cross
        // the boundary. prism's DFG does not propagate base taint into field nodes, so
        // binding only the field would drop object-level taint the base edge carries.
        arg_paths
            .into_iter()
            .filter_map(|arg_path| {
                Self::argument_var_node_in_span_with_exact(
                    caller_id,
                    caller_parsed,
                    &arg_path,
                    arg_span,
                    var_index,
                    exact_var_index,
                    graph,
                )
            })
            .collect()
    }
}
