//! Frozen Step 4 from main 2453933f, except Go's cycle-aware telemetry.
use super::*;
use std::cell::Cell;
thread_local! { static REFERENCE: Cell<bool> = const { Cell::new(false) }; }
pub(super) fn enabled() -> bool {
    REFERENCE.with(Cell::get)
}
pub(super) fn with_reference<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REFERENCE.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(REFERENCE.with(|c| c.replace(true)));
    f()
}
impl CodePropertyGraph {
    pub(super) fn assemble_step4_reference(
        dfg: &mut DataFlowGraph,
        files: &BTreeMap<String, ParsedFile>,
        var_index: &BTreeMap<LegacyVarKey, NodeIndex>,
        exact_var_index: &ExactVarIndex,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
    ) -> DfgLabelStats {
        // --- Step 4: DataFlow edges ---
        let mut dfg_label_stats = DfgLabelStats::default();
        let mut go_cycles = super::super::go_loop_cycles::GoLoopCycles::default();
        let mut dfg_rd_functions_without_cfg = BTreeSet::new();
        let mut materialized_dfg_edges = BTreeSet::new();
        let mut legacy_materialized_labels = BTreeMap::new();
        let legacy_exact_labels: BTreeMap<_, _> = dfg
            .edges()
            .iter()
            .map(|edge| {
                let confidence = dfg
                    .label(edge.from, edge.to)
                    .unwrap_or(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));
                (
                    crate::data_flow::ExactFlowEdge::from_refs(edge.from, edge.to),
                    confidence,
                )
            })
            .collect();
        let conflicting_exact_bindings: BTreeSet<_> = dfg
            .exact_labels()
            .filter_map(|(edge, label)| {
                legacy_exact_labels
                    .get(&edge)
                    .is_some_and(|legacy| *legacy != label)
                    .then(|| edge.binding_key())
            })
            .collect();
        if !conflicting_exact_bindings.is_empty() {
            eprintln!(
                "prism: refused {} conflicting exact producer binding(s) during CPG assembly",
                conflicting_exact_bindings.len()
            );
        }
        for edge in dfg.edges().iter() {
            let from_access = match edge.from.kind {
                VarAccessKind::Def => VarAccess::Def,
                VarAccessKind::Use => VarAccess::Use,
            };
            let to_access = match edge.to.kind {
                VarAccessKind::Def => VarAccess::Def,
                VarAccessKind::Use => VarAccess::Use,
            };
            if let (Some(from_idx), Some(to_idx)) = (
                Self::dfg_endpoint_node(
                    &edge.from,
                    from_access,
                    files,
                    &var_index,
                    &exact_var_index,
                    &graph,
                ),
                Self::dfg_endpoint_node(
                    &edge.to,
                    to_access,
                    files,
                    &var_index,
                    &exact_var_index,
                    &graph,
                ),
            ) {
                let confidence = dfg.label(edge.from, edge.to).unwrap_or_else(|| {
                    dfg_rd_functions_without_cfg.insert((
                        edge.from.file.clone(),
                        edge.from.function.clone(),
                        edge.from.function_start_line,
                    ));
                    FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
                });
                if materialized_dfg_edges.insert((from_idx, to_idx, confidence)) {
                    legacy_materialized_labels
                        .entry((from_idx, to_idx))
                        .and_modify(|stored: &mut FlowConfidence| {
                            *stored = stored.worst(confidence)
                        })
                        .or_insert(confidence);
                    dfg_label_stats.record_label(confidence);
                    if confidence.is_exact()
                        && go_cycles.is_loop_carried(
                            files,
                            &edge.from.file,
                            edge.from.function_start_line,
                            edge.from.line,
                            edge.to.line,
                        )
                    {
                        dfg_label_stats.dfg_label_loop_carried += 1;
                    }
                    graph.add_edge(from_idx, to_idx, CpgEdge::DataFlow(confidence));
                }
            }
        }
        for (exact_edge, confidence) in dfg.exact_labels() {
            if conflicting_exact_bindings.contains(&exact_edge.binding_key()) {
                continue;
            }
            let edge = exact_edge.as_legacy();
            if edge.from.kind != VarAccessKind::Def
                || edge.to.kind != VarAccessKind::Use
                || edge.from.file != edge.to.file
                || edge.from.function != edge.to.function
                || edge.from.function_start_line != edge.to.function_start_line
                || edge.from.path != edge.to.path
                || !edge.from.path.is_simple()
                || !Self::exact_occurrence_language(files, &edge.from.file)
            {
                continue;
            }
            let (Some(from_idx), Some(to_idx)) = (
                Self::exact_dfg_endpoint_node(&edge.from, VarAccess::Def, &exact_var_index, &graph),
                Self::exact_dfg_endpoint_node(&edge.to, VarAccess::Use, &exact_var_index, &graph),
            ) else {
                continue;
            };
            if let Some(legacy_confidence) = legacy_materialized_labels.get(&(from_idx, to_idx)) {
                // Same-pair overlap is either an exact duplicate or a conflict.
                // In both cases the legacy row wins unchanged.
                debug_assert_eq!(legacy_confidence, &confidence);
                continue;
            }
            if materialized_dfg_edges.insert((from_idx, to_idx, confidence)) {
                dfg_label_stats.record_label(confidence);
                if confidence.is_exact()
                    && go_cycles.is_loop_carried(
                        files,
                        &edge.from.file,
                        edge.from.function_start_line,
                        edge.from.line,
                        edge.to.line,
                    )
                {
                    dfg_label_stats.dfg_label_loop_carried += 1;
                }
                graph.add_edge(from_idx, to_idx, CpgEdge::DataFlow(confidence));
            }
        }
        for (file, function, start_line) in dfg_rd_functions_without_cfg {
            // The identity set persists through PartialHit. Inserting the same
            // retained fallback again is idempotent, while a missing label in a
            // newly affected function grows the exact per-file union.
            dfg.rd_function_stats_mut()
                .entry(file)
                .or_default()
                .record_without_cfg(function, start_line);
        }
        dfg_label_stats.record_rd_stats(dfg);

        dfg_label_stats
    }
}
