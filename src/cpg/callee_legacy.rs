//! Frozen pre-C1 caller-local path, used only for complete-assembly identity.
use super::*;
use std::cell::Cell;
thread_local! { static LEGACY: Cell<bool> = const { Cell::new(false) }; }
pub(super) fn enabled() -> bool {
    LEGACY.with(Cell::get)
}
pub(super) fn with_legacy<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LEGACY.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(LEGACY.with(|c| c.replace(true)));
    f()
}
pub(super) fn collect(
    cg: &CallGraph,
    index: &BTreeMap<LegacyVarKey, NodeIndex>,
    exact: &ExactVarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<PendingArgumentFlow> {
    cg.calls
        .iter()
        .flat_map(|(caller, sites)| {
            CodePropertyGraph::step5b_caller_legacy(caller, sites, cg, index, exact, graph, files)
        })
        .collect()
}
impl CodePropertyGraph {
    pub(super) fn step5b_caller_legacy(
        caller_id: &FunctionId,
        sites: &BTreeSet<CallSite>,
        cg: &CallGraph,
        var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
        exact_var_index: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<PendingArgumentFlow> {
        let mut out = Vec::new();
        let mut param_cache: BTreeMap<FunctionId, Option<Vec<Option<NodeIndex>>>> = BTreeMap::new();
        for site in sites {
            for resolved in cg.resolve_call_site(site) {
                // P3 (F1): R6MultiOwnerCandidate is an unverified, capped NameOnly
                // maybe-edge (nav-only). DataFlow carries no confidence/kind and
                // taint consumes it directly, so wiring an arg->param edge here
                // would leak taint into an unconfirmed owner. The Call/Return CPG
                // edges (Step 5) are untouched — nav still sees the candidate.
                if resolved.kind == ResolutionKind::R6MultiOwnerCandidate {
                    continue;
                }
                let callee_id = resolved.target;
                let cross_file_free_single_exact = resolved.kind == ResolutionKind::FreeSingle
                    && resolved.confidence == ResolutionConfidence::Exact
                    && callee_id.file != caller_id.file;
                let caller_parsed = match files.get(&caller_id.file) {
                    Some(p) => p,
                    None => continue,
                };
                let args = caller_parsed.call_argument_texts_and_spans_at(
                    site.start_byte,
                    site.effective_source_callee_name(),
                );
                if args.is_empty() {
                    continue;
                }
                let callee_parsed = match files.get(&callee_id.file) {
                    Some(p) => p,
                    None => continue,
                };
                let param_nodes = match param_cache.entry(callee_id.clone()).or_insert_with(|| {
                    compute_param_def_nodes(callee_parsed, callee_id, var_index, graph)
                }) {
                    Some(nodes) => nodes.as_slice(),
                    None => continue,
                };
                for (i, &param_idx) in param_nodes.iter().enumerate() {
                    if i >= args.len() {
                        break;
                    }
                    let (arg_text, arg_span) = &args[i];
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
                    let arg_idxs: Vec<NodeIndex> = arg_paths
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
                        .collect();
                    if let Some(to) = param_idx {
                        for from in arg_idxs {
                            out.push(PendingArgumentFlow {
                                edge: (
                                    from,
                                    to,
                                    CpgEdge::DataFlow(FlowConfidence::from(resolved.confidence)),
                                ),
                                cross_file_free_single_exact,
                            });
                        }
                    }
                }
            }
        }
        out
    }
}
