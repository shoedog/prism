//! Serial twin of Step-5b used as the par==serial reference by the parallelization oracles.
//! Test-only, with a full-assembly path for checking post-collection guards.
use super::*;
use std::cell::Cell;

thread_local! { static SERIAL: Cell<bool> = const { Cell::new(false) }; }

pub(super) fn enabled() -> bool {
    SERIAL.with(Cell::get)
}

pub(super) fn with_serial<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            SERIAL.with(|active| active.set(self.0));
        }
    }
    let _restore = Restore(SERIAL.with(|active| active.replace(true)));
    f()
}

impl CodePropertyGraph {
    pub(crate) fn collect_step5b_edges_reference(
        cg: &CallGraph,
        var_index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<PendingEdge> {
        Self::collect_step5b_flows_reference(cg, var_index, graph, files)
            .into_iter()
            .map(|flow| flow.edge)
            .collect()
    }

    pub(super) fn collect_step5b_flows_reference(
        cg: &CallGraph,
        var_index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<PendingArgumentFlow> {
        // Serial twin of Step-5b (global lazy param_cache, no prewarm) — the par==serial
        // reference for the oracle. The arg→param binding mirrors the production helper's
        // field-sensitive logic so the oracle proves the par_iter restructure is faithful
        // (it is no longer a frozen pre-edge-steps original; that byte-identity is
        // intentionally superseded by the field-sensitivity change).
        let mut out = Vec::new();
        let exact_var_index = Self::exact_var_index_from_graph(graph);
        let mut param_cache: BTreeMap<FunctionId, Option<Vec<Option<callee_params::ParamDef>>>> =
            BTreeMap::new();
        let mut rest_cache = BTreeMap::new();
        for (caller_id, sites) in &cg.calls {
            for site in sites {
                let resolved_site = cg.resolve_call_site(site);
                let exact_singleton = object_literal_args::exact_singleton(&resolved_site);
                let mut call_args = None;
                for resolved in resolved_site {
                    // P3 (F1): mirrors step5b_edges_for_caller's skip so this serial
                    // oracle stays byte-identical to the parallel production path.
                    if resolved.kind == ResolutionKind::R6MultiOwnerCandidate {
                        continue;
                    }
                    let callee_id = resolved.target;
                    let caller_parsed = match files.get(&caller_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let (args, offsets) =
                        call_args.get_or_insert_with(|| spread_args::for_call(caller_parsed, site));
                    if args.is_empty() {
                        continue;
                    }
                    let callee_parsed = match files.get(&callee_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let cross_file_free_single_exact = resolved.kind == ResolutionKind::FreeSingle
                        && resolved.confidence == ResolutionConfidence::Exact
                        && callee_id.file != caller_id.file;
                    let param_nodes = param_cache
                        .entry(callee_id.clone())
                        .or_insert_with(|| {
                            compute_param_def_nodes(callee_parsed, callee_id, var_index, graph)
                        })
                        .as_deref()
                        .unwrap_or_default();
                    let inputs = object_literal_args::SiteInputs {
                        caller_id,
                        caller_parsed,
                        callee_parsed,
                        args,
                        var_index,
                        exact_var_index: &exact_var_index,
                        graph,
                        cross_file_free_single_exact,
                    };
                    let bare = param_nodes
                        .first()
                        .and_then(Option::as_ref)
                        .and_then(|t| t.bare);
                    let admission = bare_param_admission::Admission::new(
                        bare,
                        site,
                        cg,
                        &resolved,
                        exact_singleton,
                        cross_file_free_single_exact,
                    );
                    object_literal_args::collect(
                        &mut out,
                        &inputs,
                        param_nodes,
                        &admission,
                        |t| t.bare,
                        |t| Some(t.node),
                    );
                    for ((arg_text, arg_span), slot) in args.iter().zip(&offsets.slot) {
                        let Some(param) = slot
                            .and_then(|slot| param_nodes.get(slot))
                            .and_then(Option::as_ref)
                            .filter(|target| admission.ordinary(target.bare))
                        else {
                            continue;
                        };
                        // Field-sensitive arg binding — mirrors the production helper so this
                        // serial reference stays the par==serial twin for the parallelization oracle.
                        let full_arg_path = caller_parsed
                            .bounded_argument_member_path_at(arg_span.start, arg_span.end)
                            .unwrap_or_else(|| AccessPath::from_expr(arg_text));
                        let mut arg_paths = vec![full_arg_path.clone()];
                        let base_arg_path = AccessPath::simple(&full_arg_path.base);
                        if base_arg_path != full_arg_path {
                            arg_paths.push(base_arg_path);
                        }
                        // Supplement, not replace (mirrors the production helper) — both the
                        // field path and the base get an arg→param edge so object-level taint
                        // is preserved alongside field-level precision.
                        let arg_idxs: Vec<NodeIndex> = arg_paths
                            .into_iter()
                            .filter_map(|arg_path| {
                                Self::argument_var_node_in_span_with_exact(
                                    caller_id,
                                    caller_parsed,
                                    &arg_path,
                                    arg_span,
                                    var_index,
                                    &exact_var_index,
                                    graph,
                                )
                            })
                            .collect();
                        for from in arg_idxs {
                            out.push(PendingArgumentFlow {
                                edge: (
                                    from,
                                    param.node,
                                    CpgEdge::DataFlow(FlowConfidence::from(resolved.confidence)),
                                ),
                                cross_file_free_single_exact,
                                bare_literal: false,
                            });
                        }
                    }
                    if !rest_params::admitted(&inputs, site, cg, &resolved, exact_singleton) {
                        continue;
                    }
                    let Some(target) = *rest_cache.entry(callee_id.clone()).or_insert_with(|| {
                        rest_params::compute_rest_def_node(
                            callee_parsed,
                            callee_id,
                            var_index,
                            graph,
                        )
                    }) else {
                        continue;
                    };
                    rest_params::collect(
                        &mut out,
                        &inputs,
                        target,
                        offsets,
                        resolved.confidence.into(),
                    );
                }
            }
        }
        out
    }
}
