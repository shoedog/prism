//! Ordered DataFlow materialization using the DFG's existing occurrence IDs.
use super::*;
use crate::data_flow::{AssemblyId, AssemblyView};

// The refusal identity intentionally omits line and access kind, just like
// ExactFlowEdge::binding_key. Do not substitute an occurrence ID for this key.
fn binding(loc: &VarLocation) -> (&str, &str, usize, &AccessPath, usize, usize) {
    (
        &loc.file,
        &loc.function,
        loc.function_start_line,
        &loc.path,
        loc.start_byte,
        loc.end_byte,
    )
}

struct Endpoints<'a> {
    view: AssemblyView<'a>,
    nodes: Vec<Option<Option<NodeIndex>>>,
}
impl Endpoints<'_> {
    fn get(
        &mut self,
        id: AssemblyId,
        files: &BTreeMap<String, ParsedFile>,
        var_index: &BTreeMap<LegacyVarKey, NodeIndex>,
        exact_var_index: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Option<NodeIndex> {
        *self.nodes[id.slot()].get_or_insert_with(|| {
            let loc = self.view.location(id);
            let access = match loc.kind {
                VarAccessKind::Def => VarAccess::Def,
                VarAccessKind::Use => VarAccess::Use,
            };
            CodePropertyGraph::dfg_endpoint_node(
                loc,
                access,
                files,
                var_index,
                exact_var_index,
                graph,
            )
        })
    }
}

impl CodePropertyGraph {
    pub(super) fn assemble_step4(
        dfg: &mut DataFlowGraph,
        files: &BTreeMap<String, ParsedFile>,
        var_index: &BTreeMap<LegacyVarKey, NodeIndex>,
        exact_var_index: &ExactVarIndex,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
    ) -> DfgLabelStats {
        #[cfg(test)]
        if step4_reference::enabled() {
            return Self::assemble_step4_reference(dfg, files, var_index, exact_var_index, graph);
        }
        let mut stats = DfgLabelStats::default();
        let mut without_cfg = BTreeSet::new();
        {
            let view = dfg.assembly_view();
            // Probe the supplemental map with existing IDs rather than building
            // an owned ExactFlowEdge map over every legacy edge.
            let conflicts: BTreeSet<_> = view
                .edges()
                .filter_map(|(f, t)| {
                    let exact = view.exact_label(f, t)?;
                    let legacy = view
                        .label(f, t)
                        .unwrap_or(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));
                    (legacy != exact).then(|| binding(view.location(f)))
                })
                .collect();
            if !conflicts.is_empty() {
                eprintln!(
                    "prism: refused {} conflicting exact producer binding(s) during CPG assembly",
                    conflicts.len()
                );
            }
            let mut endpoints = Endpoints {
                nodes: vec![None; view.len()],
                view: dfg.assembly_view(),
            };
            let mut materialized = BTreeSet::new();
            let mut legacy_labels = BTreeMap::new();
            for (f, t) in view.edges() {
                if let (Some(from), Some(to)) = (
                    endpoints.get(f, files, var_index, exact_var_index, graph),
                    endpoints.get(t, files, var_index, exact_var_index, graph),
                ) {
                    let from_loc = view.location(f);
                    let confidence = view.label(f, t).unwrap_or_else(|| {
                        without_cfg.insert((
                            from_loc.file.clone(),
                            from_loc.function.clone(),
                            from_loc.function_start_line,
                        ));
                        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
                    });
                    if materialized.insert((from, to, confidence)) {
                        legacy_labels
                            .entry((from, to))
                            .and_modify(|stored: &mut FlowConfidence| {
                                *stored = stored.worst(confidence)
                            })
                            .or_insert(confidence);
                        stats.record_label(confidence);
                        if confidence.is_exact() && view.location(t).line < from_loc.line {
                            stats.dfg_label_loop_carried += 1;
                        }
                        graph.add_edge(from, to, CpgEdge::DataFlow(confidence));
                    }
                }
            }
            for (f, t, confidence) in view.exact_labels() {
                let from_loc = view.location(f);
                let to_loc = view.location(t);
                if conflicts.contains(&binding(from_loc))
                    || from_loc.kind != VarAccessKind::Def
                    || to_loc.kind != VarAccessKind::Use
                    || from_loc.file != to_loc.file
                    || from_loc.function != to_loc.function
                    || from_loc.function_start_line != to_loc.function_start_line
                    || from_loc.path != to_loc.path
                    || !from_loc.path.is_simple()
                    || !Self::exact_occurrence_language(files, &from_loc.file)
                    || from_loc.start_byte >= from_loc.end_byte
                    || to_loc.start_byte >= to_loc.end_byte
                {
                    continue;
                }
                let (Some(from), Some(to)) = (
                    endpoints.get(f, files, var_index, exact_var_index, graph),
                    endpoints.get(t, files, var_index, exact_var_index, graph),
                ) else {
                    continue;
                };
                if let Some(legacy) = legacy_labels.get(&(from, to)) {
                    debug_assert_eq!(legacy, &confidence);
                    continue;
                }
                if materialized.insert((from, to, confidence)) {
                    stats.record_label(confidence);
                    if confidence.is_exact() && to_loc.line < from_loc.line {
                        stats.dfg_label_loop_carried += 1;
                    }
                    graph.add_edge(from, to, CpgEdge::DataFlow(confidence));
                }
            }
        }
        for (file, function, start_line) in without_cfg {
            dfg.rd_function_stats_mut()
                .entry(file)
                .or_default()
                .record_without_cfg(function, start_line);
        }
        stats.record_rd_stats(dfg);
        stats
    }
}
