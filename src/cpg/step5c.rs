//! Return flow: owner-local read selection, with the original serial mutations.
use super::*;

#[cfg(test)]
#[path = "step5c_tests.rs"]
pub(super) mod tests;
#[path = "step5c_uses.rs"]
mod uses;

impl CodePropertyGraph {
    pub(super) fn assemble_step5c_return_flow(
        cg: &CallGraph,
        var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
        func_index: &BTreeMap<(String, String, usize), NodeIndex>,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> ReturnFlowStats {
        #[cfg(test)]
        if step5c_reference::enabled() {
            return Self::assemble_step5c_return_flow_reference(
                cg,
                var_index,
                func_index,
                graph,
                location_index,
                files,
            );
        }
        // Step 5c adds only ReturnValue nodes: Variable Use identities stay fixed.
        let uses = uses::UseIndex::new(graph);
        let shared_returns = callee_returns::ReturnCache::new(files);
        let mut stats = ReturnFlowStats::default();
        let mut return_value_index: BTreeMap<
            (String, String, usize, usize, usize, usize),
            NodeIndex,
        > = BTreeMap::new();

        for (caller_id, sites) in &cg.calls {
            let Some(caller_parsed) = files.get(&caller_id.file) else {
                continue;
            };
            for site in sites {
                let Some(assignment) =
                    caller_parsed.assignment_for_exact_call(site.start_byte, site.end_byte)
                else {
                    continue;
                };

                let outcome = cg.resolve_call_site_full(site);
                let Some(resolved) = singleton_exact(&outcome) else {
                    let has_exact = outcome
                        .resolved
                        .iter()
                        .any(|item| item.confidence == ResolutionConfidence::Exact);
                    let has_nameonly = outcome
                        .resolved
                        .iter()
                        .any(|item| item.confidence == ResolutionConfidence::NameOnly);
                    if has_exact && has_nameonly {
                        stats.return_flow_skipped_mixed += 1;
                    } else if has_exact && outcome.resolved.len() > 1 {
                        stats.return_flow_skipped_multi += 1;
                    } else {
                        stats.return_flow_skipped_nameonly += 1;
                    }
                    continue;
                };

                if assignment.lvalues.is_empty()
                    || assignment
                        .lvalues
                        .iter()
                        .any(|lhs| !lhs.path.fields.is_empty())
                {
                    stats.return_flow_skipped_non_simple_lhs += 1;
                    continue;
                }

                let Some(callee_parsed) = files.get(&resolved.target.file) else {
                    continue;
                };
                let cell = shared_returns.get(resolved.target);
                let Some(returns) = cell.get().expect("initialized return facts").as_ref() else {
                    continue;
                };

                let targets: Vec<NodeIndex> = assignment
                    .lvalues
                    .iter()
                    .filter_map(|lhs| {
                        var_index
                            .get(&(
                                caller_id.file.clone(),
                                caller_id.name.clone(),
                                caller_id.start_line,
                                lhs.line,
                                lhs.path.clone(),
                                VarAccess::Def,
                            ))
                            .copied()
                            .filter(|&idx| {
                                matches!(
                                    &graph[idx],
                                    CpgNode::Variable { start_byte, end_byte, .. }
                                        if *start_byte == lhs.start_byte && *end_byte == lhs.end_byte
                                )
                            })
                    })
                    .collect();
                if targets.len() != assignment.lvalues.len() {
                    stats.return_flow_skipped_non_simple_lhs += 1;
                    continue;
                }

                let mut modeled: Vec<(NodeIndex, NodeIndex)> = Vec::new();
                let mut complete_returns = !returns.is_empty();
                for ret in returns {
                    if ret.values.is_empty() {
                        stats.return_flow_skipped_named_return += 1;
                        complete_returns = false;
                        continue;
                    }
                    if targets.len() > 1
                        && ret.values.len() == 1
                        && callee_parsed
                            .language
                            .is_call_node(ret.values[0].kind.as_str())
                    {
                        stats.return_flow_skipped_forwarded_return += 1;
                        complete_returns = false;
                        continue;
                    }
                    if ret.values.len() != targets.len() {
                        stats.return_flow_skipped_arity_mismatch += 1;
                        complete_returns = false;
                        continue;
                    }

                    for (value, &target) in ret.values.iter().zip(&targets) {
                        let simple_path = matches!(
                            value.kind.as_str(),
                            "identifier"
                                | "field_expression"
                                | "member_expression"
                                | "selector_expression"
                                | "attribute"
                                | "field_access"
                                | "dot_index_expression"
                                | "method_index_expression"
                        )
                        .then(|| {
                            callee_parsed
                                .bounded_member_path_at(value.start_byte, value.end_byte)
                                .unwrap_or_else(|| AccessPath::from_expr(&value.text))
                        });

                        let endpoint = simple_path.and_then(|path| {
                            var_index
                                .get(&(
                                    resolved.target.file.clone(),
                                    resolved.target.name.clone(),
                                    resolved.target.start_line,
                                    value.line,
                                    path,
                                    VarAccess::Use,
                                ))
                                .copied()
                                .filter(|&idx| {
                                    matches!(
                                        &graph[idx],
                                        CpgNode::Variable { start_byte, end_byte, .. }
                                            if value.start_byte <= *start_byte
                                                && *end_byte <= value.end_byte
                                    )
                                })
                        });

                        let endpoint = if let Some(endpoint) = endpoint {
                            endpoint
                        } else {
                            let identity = (
                                resolved.target.file.clone(),
                                resolved.target.name.clone(),
                                resolved.target.start_line,
                                ret.start_byte,
                                ret.end_byte,
                                value.slot,
                            );
                            if let Some(&synthetic) = return_value_index.get(&identity) {
                                synthetic
                            } else {
                                let synthetic = graph.add_node(CpgNode::ReturnValue {
                                    file: resolved.target.file.clone(),
                                    function: resolved.target.name.clone(),
                                    function_start_line: resolved.target.start_line,
                                    line: value.line,
                                    return_start_byte: ret.start_byte,
                                    return_end_byte: ret.end_byte,
                                    child_slot: value.slot,
                                    start_byte: value.start_byte,
                                    end_byte: value.end_byte,
                                });
                                return_value_index.insert(identity, synthetic);
                                location_index
                                    .entry((resolved.target.file.clone(), value.line))
                                    .or_default()
                                    .push(synthetic);
                                if let Some(&function) = func_index.get(&(
                                    resolved.target.file.clone(),
                                    resolved.target.name.clone(),
                                    resolved.target.start_line,
                                )) {
                                    graph.add_edge(function, synthetic, CpgEdge::Contains);
                                }

                                let mut inputs = uses.within(
                                    resolved.target,
                                    value.start_byte,
                                    value.end_byte,
                                    callee_parsed,
                                );
                                inputs.sort_by_key(|idx| idx.index());
                                for input in inputs {
                                    graph.add_edge(input, synthetic, CpgEdge::ReturnInput);
                                    stats.return_input_edges += 1;
                                }
                                synthetic
                            }
                        };
                        modeled.push((endpoint, target));
                    }
                }

                let rhs_uses =
                    uses.within(caller_id, site.start_byte, site.end_byte, caller_parsed);
                let all_rhs_uses_bound = rhs_uses.iter().all(|&use_idx| {
                    graph.edges(use_idx).any(|edge| {
                        matches!(edge.weight(), CpgEdge::DataFlow(_))
                            && matches!(
                                &graph[edge.target()],
                                CpgNode::Variable {
                                    file,
                                    function,
                                    function_start_line,
                                    access: VarAccess::Def,
                                    ..
                                } if file == &resolved.target.file
                                    && function == &resolved.target.name
                                    && *function_start_line == resolved.target.start_line
                            )
                    })
                });
                let certified = complete_returns && all_rhs_uses_bound && !modeled.is_empty();
                if certified {
                    stats.return_flow_suppression_certified += 1;
                } else {
                    if !complete_returns {
                        stats.return_flow_suppression_void_incomplete_returns += 1;
                    }
                    if !all_rhs_uses_bound {
                        stats.return_flow_suppression_void_unbound_uses += 1;
                    }
                }
                for (from, to) in modeled {
                    graph.add_edge(
                        from,
                        to,
                        CpgEdge::ReturnFlow {
                            suppress_shortcut: certified,
                        },
                    );
                    stats.return_flow_edges += 1;
                }
            }
        }
        stats
    }
}
