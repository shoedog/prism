//! Serial twin of Step-5b used as the par==serial reference by the parallelization oracles.
//! Moved out of `build.rs` unchanged (test-only).
use super::*;

impl CodePropertyGraph {
    pub(crate) fn collect_step5b_edges_reference(
        cg: &CallGraph,
        var_index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<PendingEdge> {
        // Serial twin of Step-5b (global lazy param_cache, no prewarm) — the par==serial
        // reference for the oracle. The arg→param binding mirrors the production helper's
        // field-sensitive logic so the oracle proves the par_iter restructure is faithful
        // (it is no longer a frozen pre-edge-steps original; that byte-identity is
        // intentionally superseded by the field-sensitivity change).
        let mut out: Vec<PendingEdge> = Vec::new();
        let exact_var_index = Self::exact_var_index_from_graph(graph);
        let mut param_cache: BTreeMap<FunctionId, Option<Vec<Option<NodeIndex>>>> = BTreeMap::new();
        for (caller_id, sites) in &cg.calls {
            for site in sites {
                let resolved_site = cg.resolve_call_site(site);
                let literal_ok = object_literal_args::exact_singleton(&resolved_site);
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
                    let param_nodes =
                        match param_cache.entry(callee_id.clone()).or_insert_with(|| {
                            compute_param_def_nodes(callee_parsed, callee_id, var_index, graph)
                        }) {
                            Some(nodes) => nodes.as_slice(),
                            None => continue,
                        };
                    // #594: the object-literal edges, at the position the production loop
                    // emits them (before the positional edges of the same call site).
                    if literal_ok {
                        let inputs = object_literal_args::SiteInputs {
                            caller_id,
                            caller_parsed,
                            callee_parsed,
                            args: &args,
                            var_index,
                            exact_var_index: &exact_var_index,
                            graph,
                            cross_file_free_single_exact: false,
                        };
                        let mut literal = Vec::new();
                        object_literal_args::collect(&mut literal, &inputs, param_nodes, |&n| {
                            Some(n)
                        });
                        out.extend(literal.into_iter().map(|flow| flow.edge));
                    }
                    for (i, &param_idx) in param_nodes.iter().enumerate() {
                        if i >= args.len() {
                            break;
                        }
                        let (arg_text, arg_span) = &args[i];
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
                        if let Some(to) = param_idx {
                            for from in arg_idxs {
                                out.push((
                                    from,
                                    to,
                                    CpgEdge::DataFlow(FlowConfidence::from(resolved.confidence)),
                                ));
                            }
                        }
                    }
                }
            }
        }
        out
    }
}
