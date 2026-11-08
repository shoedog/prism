//! #759: the trace's control for an arrow whose expression body is a callable or a ternary.
//!
//! Such an owner's statement universe used to be the located statements (the nested callable's, or the
//! calls of the ternary's arms); it is now the one synthetic body node
//! (`cfg::js_single::expression_body_statement`). With `PRISM_RD_CENSUS_TRACE=1` the owner is solved a second time under main's universe (`with_main_arrow_universe`), so a
//! measurement can name every edge whose label the rule moved (`arrow-body-node`) and the owners
//! whose availability flipped. Nothing here runs with the trace off.
use super::super::{FlowConfidence, FlowDoubt};
use super::{
    census_pass::Pass, census_trace, reaching_definitions_impl, DefSite, FileCfgEdgeIndex,
    RdOutcome, RdUnavailable,
};
use crate::ast::ParsedFile;
use crate::data_flow::{ExactFlowEdge, FlowEdge, VarLocation};
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) const MECHANISM: &str = "arrow-body-node";

/// Edges the rule moved: named once, so no later mechanism names them again.
#[derive(Default)]
pub(super) struct Named {
    pub(super) labels: BTreeSet<(VarLocation, VarLocation)>,
    pub(super) exact: BTreeSet<ExactFlowEdge>,
}

fn availability(outcome: &RdOutcome) -> &'static str {
    match outcome {
        RdOutcome::Available(_) => "ok",
        RdOutcome::Unavailable(RdUnavailable::NoCfgEdges) => "no-cfg",
        RdOutcome::Unavailable(RdUnavailable::DefinitionsCapExceeded { .. }) => "def-cap",
        RdOutcome::Unavailable(RdUnavailable::StatementLinesCapExceeded { .. }) => "line-cap",
    }
}

/// Whether the rule changed this owner's universe: an expression-bodied arrow whose body is a callable or
/// a ternary and whose body held more than one located statement (one or none was already the single node).
/// Counted in deduplicated lines; an owner whose inner statements share one line was one RD node on main
/// already and is not claimed.
pub(super) fn universe_moved(parsed: &ParsedFile, owner: &Node<'_>) -> bool {
    crate::cfg::js_single::arrow_body_takes_one_node(parsed, owner)
        && parsed.statements_in_function(owner).len() > 1
}

/// `rd-census-owner\tarrow-body-node\t<file>\t<owner>\t<line>\t<control>><head>` once per owner whose
/// universe moved (availability `ok`, `no-cfg`, `def-cap` or `line-cap` on each side, for the counter check),
/// and `arrow-body-node` for every edge whose label differs between main's universe and the head's.
/// An unavailable side labels every edge `NameOnly(CfgIncomplete)`, the label a refusal gives.
pub(super) fn trace(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    (dfg_edges, exact_edges): (&[FlowEdge], &[FlowEdge]),
    cfg: &FileCfgEdgeIndex<'_>,
    head: &RdOutcome,
    out: &mut impl std::io::Write,
) -> Named {
    let mut named = Named::default();
    if !universe_moved(parsed, owner) {
        return named;
    }
    let control = crate::cfg::js_single::with_main_arrow_universe(|| {
        reaching_definitions_impl(
            parsed,
            owner,
            defs,
            dfg_edges,
            exact_edges,
            cfg,
            Pass::ArrowControl,
            &mut None,
        )
    });
    let _ = writeln!(
        out,
        "rd-census-owner\t{MECHANISM}\t{}\t{}\t{}\t{}>{}",
        parsed.path,
        parsed.dfg_owner_name(owner).unwrap_or_default(),
        owner.start_position().row + 1,
        availability(&control),
        availability(head)
    );
    let refused = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    let label = |outcome: &RdOutcome, key: &(VarLocation, VarLocation)| match outcome {
        RdOutcome::Available(r) => r.labels.get(key).copied(),
        RdOutcome::Unavailable(_) => Some(refused),
    };
    let exact_label = |outcome: &RdOutcome, key: &ExactFlowEdge| match outcome {
        RdOutcome::Available(r) => r.exact_labels.get(key).copied(),
        RdOutcome::Unavailable(_) => Some(refused),
    };
    for edge in dfg_edges {
        let key = (edge.from.clone(), edge.to.clone());
        let (new, old) = (label(head, &key), label(&control, &key));
        if new.is_some() && old.is_some() && new != old && named.labels.insert(key) {
            census_trace::emit(out, MECHANISM, edge);
        }
    }
    for edge in exact_edges {
        let key = ExactFlowEdge::from_legacy(edge);
        let (new, old) = (exact_label(head, &key), exact_label(&control, &key));
        if new.is_some() && old.is_some() && new != old && named.exact.insert(key) {
            census_trace::emit(out, MECHANISM, edge);
        }
    }
    named
}
