//! #660 S1b-1: the census-free control pass behind `PRISM_RD_CENSUS_TRACE`.
use super::*;

/// Which write-census mechanisms a solver pass applies (#660 S1b-1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pass {
    /// Production: kill-only defs and refusals.
    Census,
    /// The trace's control: main's rules plus S1a, no census.
    NoCensus,
}

/// The production pass; with the trace on, `traced` to stderr.
pub(super) fn run(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    dfg_edges: &[FlowEdge],
    exact_edges: &[FlowEdge],
    file_cfg_edges: &FileCfgEdgeIndex<'_>,
) -> RdOutcome {
    if census_trace::enabled() {
        let out = &mut std::io::stderr().lock();
        return traced(
            parsed,
            func_node,
            defs,
            dfg_edges,
            exact_edges,
            file_cfg_edges,
            out,
        );
    }
    let mut none = None;
    reaching_definitions_impl(
        parsed,
        func_node,
        defs,
        dfg_edges,
        exact_edges,
        file_cfg_edges,
        Pass::Census,
        &mut none,
    )
}

/// The production pass, plus (trace on only) a control pass without the census whose labels name each
/// edge the census moved. The control never changes the returned outcome.
pub(super) fn traced(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    dfg_edges: &[FlowEdge],
    exact_edges: &[FlowEdge],
    file_cfg_edges: &FileCfgEdgeIndex<'_>,
    out: &mut impl std::io::Write,
) -> RdOutcome {
    let mut facts = None;
    let outcome = reaching_definitions_impl(
        parsed,
        func_node,
        defs,
        dfg_edges,
        exact_edges,
        file_cfg_edges,
        Pass::Census,
        &mut facts,
    );
    let Some(census) = facts.filter(|c| !c.is_inert()) else {
        return outcome;
    };
    let RdOutcome::Available(head) = &outcome else {
        return outcome;
    };
    let RdOutcome::Available(base) = reaching_definitions_impl(
        parsed,
        func_node,
        defs,
        dfg_edges,
        exact_edges,
        file_cfg_edges,
        Pass::NoCensus,
        &mut None,
    ) else {
        return outcome;
    };
    for edge in dfg_edges {
        let key = (edge.from.clone(), edge.to.clone());
        if let (Some(new), old) = (head.labels.get(&key), base.labels.get(&key)) {
            if old != Some(new) {
                census_trace::emit(out, &census.mechanism(edge, *new), edge);
            }
        }
    }
    for edge in exact_edges {
        let key = ExactFlowEdge::from_legacy(edge);
        if let (Some(new), old) = (head.exact_labels.get(&key), base.exact_labels.get(&key)) {
            if old != Some(new) {
                census_trace::emit(out, &census.mechanism(edge, *new), edge);
            }
        }
    }
    outcome
}
