//! #660 S1b-1: the census-free control pass behind `PRISM_RD_CENSUS_TRACE`.
use super::*;

/// Which write-census mechanisms a solver pass applies (#660 S1b-1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pass {
    /// Production: kill-only defs and refusals.
    Census,
    /// Production labels, and D3 records the mechanism of each edge it refuses (the trace's join).
    CensusLogged,
    /// The trace's control: main's rules plus S1a, no census.
    NoCensus,
    /// The trace's second control (#674): production, but recorded defs kill by path equality as on main.
    CensusEqualKills,
}

impl Pass {
    /// Whether the census mechanisms apply (every pass but the control).
    pub(super) fn census(self) -> bool {
        self != Pass::NoCensus
    }

    /// Whether a recorded def of a JS/TS/TSX owner kills by `affects` (#674).
    pub(super) fn recorded_affects(self) -> bool {
        matches!(self, Pass::Census | Pass::CensusLogged)
    }
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
        Pass::CensusLogged,
        &mut facts,
    );
    let RdOutcome::Available(head) = &outcome else {
        return outcome;
    };
    guard_control::trace(
        parsed,
        func_node,
        defs,
        (dfg_edges, exact_edges),
        file_cfg_edges,
        head,
        out,
    );
    // #674: a JS/TS/TSX edge whose label changes when recorded defs kill by path equality only was moved
    // by a recorded `affects` kill (the control keeps the census, so census moves are not claimed).
    // Edges it names are skipped below: one line per moved edge.
    let by_affects = parsed
        .language
        .is_js_family()
        .then(|| {
            reaching_definitions_impl(
                parsed,
                func_node,
                defs,
                dfg_edges,
                exact_edges,
                file_cfg_edges,
                Pass::CensusEqualKills,
                &mut None,
            )
        })
        .and_then(|outcome| match outcome {
            RdOutcome::Available(equal) => Some(equal),
            _ => None,
        });
    let mut named = BTreeSet::new();
    let mut named_exact = BTreeSet::new();
    if let Some(equal) = &by_affects {
        for edge in dfg_edges {
            let key = (edge.from.clone(), edge.to.clone());
            if let (Some(new), old) = (head.labels.get(&key), equal.labels.get(&key)) {
                if old != Some(new) {
                    census_trace::emit(out, census_trace::KILL_AFFECTS, edge);
                    named.insert(key);
                }
            }
        }
        for edge in exact_edges {
            let key = ExactFlowEdge::from_legacy(edge);
            if let (Some(new), old) = (head.exact_labels.get(&key), equal.exact_labels.get(&key)) {
                if old != Some(new) {
                    census_trace::emit(out, census_trace::KILL_AFFECTS, edge);
                    named_exact.insert(key);
                }
            }
        }
    }
    let Some(census) = facts.filter(|c| !c.is_inert()) else {
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
            if old != Some(new) && !named.contains(&key) {
                census_trace::emit(out, &census.mechanism(edge, *new), edge);
            }
        }
    }
    for edge in exact_edges {
        let key = ExactFlowEdge::from_legacy(edge);
        if let (Some(new), old) = (head.exact_labels.get(&key), base.exact_labels.get(&key)) {
            if old != Some(new) && !named_exact.contains(&key) {
                census_trace::emit(out, &census.mechanism(edge, *new), edge);
            }
        }
    }
    outcome
}

#[path = "guard_control.rs"]
mod guard_control;

#[cfg(test)]
#[path = "guard_deleted_tests.rs"]
mod guard_deleted_tests;

#[cfg(test)]
#[path = "param_alias_tests.rs"]
mod param_alias_tests;
